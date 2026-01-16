//! Freer monad for DSL construction.
//!
//! Enables building domain-specific languages (DSLs) from arbitrary instruction sets
//! without requiring Functor constraints.
//!
//! # Motivation
//!
//! The Free monad in Haskell requires `Functor F` to construct `Monad (Free F)`.
//! Freer monad solves this by separating "instruction" from "continuation",
//! eliminating the Functor constraint entirely.
//!
//! # Design
//!
//! ```text
//! Freer<I, A> = Pure(A)
//!             | Impure { instruction: I, queue: ContinuationQueue<I> }
//! ```
//!
//! Uses "Reflection without Remorse" pattern for O(1) `flat_map` and O(n) interpret.
//!
//! # Examples
//!
//! ## State DSL
//!
//! ```rust
//! use lambars::control::Freer;
//!
//! enum StateCommand { Get, Put(i32) }
//!
//! fn get() -> Freer<StateCommand, i32> {
//!     Freer::<StateCommand, ()>::lift_instruction(
//!         StateCommand::Get,
//!         |result| *result.downcast::<i32>().expect("Get must return i32")
//!     )
//! }
//!
//! fn put(value: i32) -> Freer<StateCommand, ()> {
//!     Freer::<StateCommand, ()>::lift_instruction(StateCommand::Put(value), |_| ())
//! }
//!
//! let program = get().flat_map(|x| put(x + 1)).then(get());
//!
//! let mut state = 10;
//! let result: i32 = program.interpret(|command| match command {
//!     StateCommand::Get => Box::new(state),
//!     StateCommand::Put(value) => { state = value; Box::new(()) }
//! });
//!
//! assert_eq!(result, 11);
//! assert_eq!(state, 11);
//! ```

use std::any::Any;
use std::collections::VecDeque;
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

/// Type-erased arrow (continuation).
///
/// Converts `A -> Freer<I, B>` to `Box<dyn Any> -> Freer<I, Box<dyn Any>>`.
/// This enables storing heterogeneous continuations in a single queue.
trait TypeErasedArrow<I> {
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>>;
}

struct Arrow<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B>,
{
    function: F,
    _phantom: PhantomData<fn(A) -> (I, B)>,
}

impl<I: 'static, A: 'static, B: 'static, F> TypeErasedArrow<I> for Arrow<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B> + 'static,
{
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>> {
        let value = *input
            .downcast::<A>()
            .expect("Type mismatch in arrow application");

        match (self.function)(value) {
            Freer::Pure(b) => Freer::Pure(Box::new(b) as Box<dyn Any>),
            Freer::Impure {
                instruction,
                mut queue,
                ..
            } => {
                queue
                    .arrows
                    .push_back(Box::new(BoxingArrow::<B>(PhantomData)));
                Freer::Impure {
                    instruction,
                    queue,
                    _result: PhantomData,
                }
            }
        }
    }
}

struct BoxingArrow<T>(PhantomData<T>);

impl<I: 'static, T: 'static> TypeErasedArrow<I> for BoxingArrow<T> {
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>> {
        Freer::Pure(input)
    }
}

struct ExtractArrow<R, E>
where
    E: FnOnce(Box<dyn Any>) -> R,
{
    extract: E,
    _phantom: PhantomData<R>,
}

impl<I: 'static, R: 'static, E> TypeErasedArrow<I> for ExtractArrow<R, E>
where
    E: FnOnce(Box<dyn Any>) -> R + 'static,
{
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>> {
        Freer::Pure(Box::new((self.extract)(input)) as Box<dyn Any>)
    }
}

#[doc(hidden)]
pub struct ContinuationQueue<I> {
    arrows: VecDeque<Box<dyn TypeErasedArrow<I>>>,
}

impl<I> ContinuationQueue<I> {
    fn new() -> Self {
        Self {
            arrows: VecDeque::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.arrows.is_empty()
    }

    fn pop(&mut self) -> Option<Box<dyn TypeErasedArrow<I>>> {
        self.arrows.pop_front()
    }
}

impl<I: 'static> ContinuationQueue<I> {
    fn push<A: 'static, B: 'static, F>(mut self, function: F) -> Self
    where
        F: FnOnce(A) -> Freer<I, B> + 'static,
    {
        self.arrows.push_back(Box::new(Arrow {
            function,
            _phantom: PhantomData,
        }));
        self
    }

    fn push_extract<R: 'static, E>(mut self, extract: E) -> Self
    where
        E: FnOnce(Box<dyn Any>) -> R + 'static,
    {
        self.arrows.push_back(Box::new(ExtractArrow {
            extract,
            _phantom: PhantomData,
        }));
        self
    }

    /// Note: Provided for future extensions.
    /// The interpret function uses `QueueStack` instead to avoid O(n^2).
    #[allow(dead_code)]
    fn concat(mut self, mut other: Self) -> Self {
        self.arrows.append(&mut other.arrows);
        self
    }
}

/// A stack of continuation queues.
///
/// Used during interpretation to avoid O(n^2) from repeated queue concatenation.
/// Instead of merging queues, we maintain a stack of queues and process them
/// in LIFO order.
struct QueueStack<I> {
    current: ContinuationQueue<I>,
    pending: Vec<ContinuationQueue<I>>,
}

impl<I> QueueStack<I> {
    const fn new(initial: ContinuationQueue<I>) -> Self {
        Self {
            current: initial,
            pending: Vec::new(),
        }
    }

    fn push_queue(&mut self, queue: ContinuationQueue<I>) {
        let old = std::mem::replace(&mut self.current, queue);
        if !old.is_empty() {
            self.pending.push(old);
        }
    }

    fn pop(&mut self) -> Option<Box<dyn TypeErasedArrow<I>>> {
        loop {
            if let Some(arrow) = self.current.pop() {
                return Some(arrow);
            }
            self.current = self.pending.pop()?;
        }
    }
}

/// Freer monad: constructs a Monad from any instruction set without Functor constraint.
///
/// # Type Parameters
///
/// * `I` - The instruction type (typically an enum representing DSL commands)
/// * `A` - The result type of the computation
///
/// # Laws
///
/// Freer satisfies the Monad laws:
///
/// - **Left Identity**: `Freer::pure(a).flat_map(f) == f(a)`
/// - **Right Identity**: `m.flat_map(Freer::pure) == m`
/// - **Associativity**: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
///
/// # Stack Safety
///
/// Deep `flat_map` chains (10,000+ levels) are handled safely through the
/// `ContinuationQueue` and loop-based `interpret` implementation.
///
/// # Performance
///
/// Uses "Reflection without Remorse" pattern:
/// - `flat_map`: O(1) - appends to continuation queue
/// - `interpret`: O(n) - processes n continuations in linear time
///
/// # Note
///
/// This type does NOT implement `TypeConstructor` because `Freer<I, A>` requires
/// `A: 'static` for certain operations, which would propagate and limit usability.
pub enum Freer<I, A> {
    /// A pure value, no instructions to execute.
    Pure(A),

    /// An instruction with a queue of type-erased continuations.
    Impure {
        /// The instruction to execute.
        instruction: I,
        /// The queue of continuations to apply after executing the instruction.
        queue: ContinuationQueue<I>,
        /// Phantom data to preserve the result type A.
        _result: PhantomData<A>,
    },
}

impl<I, A> Freer<I, A> {
    /// Lifts a pure value into Freer (the `return`/`pure` operation of the Monad).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// let freer: Freer<(), i32> = Freer::pure(42);
    /// let result = freer.interpret(|_| Box::new(()));
    /// assert_eq!(result, 42);
    /// ```
    #[inline]
    pub const fn pure(value: A) -> Self {
        Self::Pure(value)
    }
}

impl<I: 'static, A: 'static> Freer<I, A> {
    /// Lifts an instruction into Freer.
    ///
    /// The `extract` function converts the type-erased result (`Box<dyn Any>`)
    /// back to the expected concrete type. If the handler returns a wrong type,
    /// `downcast` will fail and `extract` should panic (indicating a DSL design bug).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// enum Command { GetValue, SetValue(i32) }
    ///
    /// fn get_value() -> Freer<Command, i32> {
    ///     Freer::<Command, ()>::lift_instruction(
    ///         Command::GetValue,
    ///         |result| *result.downcast::<i32>().expect("GetValue must return i32")
    ///     )
    /// }
    ///
    /// fn set_value(value: i32) -> Freer<Command, ()> {
    ///     Freer::<Command, ()>::lift_instruction(Command::SetValue(value), |_| ())
    /// }
    /// ```
    pub fn lift_instruction<R: 'static>(
        instruction: I,
        extract: impl FnOnce(Box<dyn Any>) -> R + 'static,
    ) -> Freer<I, R> {
        Freer::Impure {
            instruction,
            queue: ContinuationQueue::new().push_extract(extract),
            _result: PhantomData,
        }
    }

    /// Applies a function to the result (Functor `fmap`).
    ///
    /// When `Pure`, the function is applied immediately (assumes `function` is pure).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// let freer: Freer<(), i32> = Freer::pure(21);
    /// let doubled = freer.map(|x| x * 2);
    /// let result = doubled.interpret(|_| Box::new(()));
    /// assert_eq!(result, 42);
    /// ```
    pub fn map<B: 'static, F>(self, function: F) -> Freer<I, B>
    where
        F: FnOnce(A) -> B + 'static,
    {
        match self {
            Self::Pure(a) => Freer::Pure(function(a)),
            Self::Impure { .. } => self.flat_map(move |a| Freer::pure(function(a))),
        }
    }

    /// Chains computations together (Monad `bind`/`>>=`).
    ///
    /// When `Pure`, the function is applied immediately.
    /// Otherwise, the continuation is appended to the queue in O(1) time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// let freer: Freer<(), i32> = Freer::pure(21);
    /// let result = freer.flat_map(|x| Freer::pure(x * 2));
    /// let value = result.interpret(|_| Box::new(()));
    /// assert_eq!(value, 42);
    /// ```
    pub fn flat_map<B: 'static, F>(self, function: F) -> Freer<I, B>
    where
        F: FnOnce(A) -> Freer<I, B> + 'static,
    {
        match self {
            Self::Pure(a) => function(a),
            Self::Impure {
                instruction, queue, ..
            } => Freer::Impure {
                instruction,
                queue: queue.push(function),
                _result: PhantomData,
            },
        }
    }

    /// Alias for `flat_map`.
    #[inline]
    pub fn and_then<B: 'static, F>(self, function: F) -> Freer<I, B>
    where
        F: FnOnce(A) -> Freer<I, B> + 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two Freer computations, discarding the result of the first (Haskell's `>>`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// let first: Freer<(), ()> = Freer::pure(());
    /// let second: Freer<(), i32> = Freer::pure(42);
    /// let result = first.then(second);
    /// let value = result.interpret(|_| Box::new(()));
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    pub fn then<B: 'static>(self, next: Freer<I, B>) -> Freer<I, B> {
        self.flat_map(move |_| next)
    }

    /// Interprets the Freer computation using the given handler.
    ///
    /// The handler is called for each instruction and must return the result as `Box<dyn Any>`.
    /// Uses a loop-based approach with `QueueStack` for O(n) interpretation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Freer;
    ///
    /// enum Command { Inc, Dec }
    ///
    /// fn inc() -> Freer<Command, i32> {
    ///     Freer::<Command, ()>::lift_instruction(Command::Inc, |r| *r.downcast::<i32>().unwrap())
    /// }
    ///
    /// let program = inc().flat_map(|x| Freer::pure(x * 2));
    ///
    /// let result = program.interpret(|command| match command {
    ///     Command::Inc => Box::new(21i32),
    ///     Command::Dec => Box::new(-1i32),
    /// });
    ///
    /// assert_eq!(result, 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the final result type does not match the expected type `A`.
    /// This indicates a bug in the DSL design or handler implementation.
    pub fn interpret<Handler>(self, mut handler: Handler) -> A
    where
        Handler: FnMut(I) -> Box<dyn Any>,
    {
        enum LoopState<I> {
            ExecuteInstruction {
                instruction: I,
                queue_stack: QueueStack<I>,
            },
            ApplyContinuation {
                value: Box<dyn Any>,
                queue_stack: QueueStack<I>,
            },
        }

        let mut state = match self {
            Self::Pure(a) => return a,
            Self::Impure {
                instruction, queue, ..
            } => LoopState::ExecuteInstruction {
                instruction,
                queue_stack: QueueStack::new(queue),
            },
        };

        loop {
            state = match state {
                LoopState::ExecuteInstruction {
                    instruction,
                    queue_stack,
                } => LoopState::ApplyContinuation {
                    value: handler(instruction),
                    queue_stack,
                },
                LoopState::ApplyContinuation {
                    value,
                    mut queue_stack,
                } => match queue_stack.pop() {
                    None => return *value.downcast::<A>().expect("Final result type mismatch"),
                    Some(arrow) => match arrow.apply(value) {
                        Freer::Pure(boxed) => LoopState::ApplyContinuation {
                            value: boxed,
                            queue_stack,
                        },
                        Freer::Impure {
                            instruction, queue, ..
                        } => {
                            queue_stack.push_queue(queue);
                            LoopState::ExecuteInstruction {
                                instruction,
                                queue_stack,
                            }
                        }
                    },
                },
            };
        }
    }
}

impl<I: Debug, A: Debug> Debug for Freer<I, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure(a) => formatter.debug_tuple("Pure").field(a).finish(),
            Self::Impure { instruction, .. } => formatter
                .debug_struct("Impure")
                .field("instruction", instruction)
                .field("queue", &"<queue>")
                .finish(),
        }
    }
}

impl<I: Display, A: Display> Display for Freer<I, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure(a) => write!(formatter, "Pure({a})"),
            Self::Impure { instruction, .. } => write!(formatter, "Impure({instruction})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[derive(Debug)]
    enum TestCommand {
        Get,
        Put(i32),
    }

    fn test_get() -> Freer<TestCommand, i32> {
        Freer::<TestCommand, i32>::lift_instruction(TestCommand::Get, |result| {
            *result.downcast::<i32>().expect("Get must return i32")
        })
    }

    fn test_put(value: i32) -> Freer<TestCommand, ()> {
        Freer::<TestCommand, ()>::lift_instruction(TestCommand::Put(value), |_| ())
    }

    fn run_state_program<A: 'static>(program: Freer<TestCommand, A>, initial: i32) -> (A, i32) {
        let mut state = initial;
        let result = program.interpret(|command| match command {
            TestCommand::Get => Box::new(state),
            TestCommand::Put(value) => {
                state = value;
                Box::new(())
            }
        });
        (result, state)
    }

    #[rstest]
    fn test_pure_construction() {
        let freer: Freer<(), i32> = Freer::pure(42);
        assert!(matches!(freer, Freer::Pure(42)));
    }

    #[rstest]
    fn test_pure_with_string() {
        let freer: Freer<(), String> = Freer::pure("hello".to_string());
        assert!(matches!(freer, Freer::Pure(ref s) if s == "hello"));
    }

    #[rstest]
    fn test_lift_instruction_creates_impure() {
        let freer = test_get();
        assert!(matches!(freer, Freer::Impure { .. }));
    }

    #[rstest]
    fn test_map_pure() {
        let result = Freer::<(), i32>::pure(21).map(|x| x * 2);
        assert!(matches!(result, Freer::Pure(42)));
    }

    #[rstest]
    fn test_map_chain() {
        let result = Freer::<(), i32>::pure(10).map(|x| x + 5).map(|x| x * 2);
        assert!(matches!(result, Freer::Pure(30)));
    }

    #[rstest]
    fn test_map_impure_adds_to_queue() {
        let mapped = test_get().map(|x| x * 2);
        assert!(matches!(mapped, Freer::Impure { .. }));
    }

    #[rstest]
    fn test_flat_map_pure_to_pure() {
        let result = Freer::<(), i32>::pure(21).flat_map(|x| Freer::pure(x * 2));
        assert!(matches!(result, Freer::Pure(42)));
    }

    #[rstest]
    fn test_flat_map_chain() {
        let result = Freer::<(), i32>::pure(10)
            .flat_map(|x| Freer::pure(x + 5))
            .flat_map(|x| Freer::pure(x * 2));
        assert!(matches!(result, Freer::Pure(30)));
    }

    #[rstest]
    fn test_flat_map_impure_adds_to_queue() {
        let chained = test_get().flat_map(|x| Freer::pure(x * 2));
        assert!(matches!(chained, Freer::Impure { .. }));
    }

    #[rstest]
    fn test_and_then_is_flat_map_alias() {
        let result = Freer::<(), i32>::pure(21).and_then(|x| Freer::pure(x * 2));
        assert!(matches!(result, Freer::Pure(42)));
    }

    #[rstest]
    fn test_then_sequences() {
        let result = Freer::<(), ()>::pure(()).then(Freer::pure(42));
        assert!(matches!(result, Freer::Pure(42)));
    }

    #[rstest]
    fn test_interpret_pure() {
        let result: i32 = Freer::<(), i32>::pure(42).interpret(|()| Box::new(()));
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_interpret_pure_chain() {
        let freer = Freer::<(), i32>::pure(10)
            .flat_map(|x| Freer::pure(x + 5))
            .flat_map(|x| Freer::pure(x * 2));
        assert_eq!(freer.interpret(|()| Box::new(())), 30);
    }

    #[rstest]
    fn test_interpret_single_get() {
        let (result, _) = run_state_program(test_get(), 42);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_interpret_get_put_get() {
        let program = test_get().flat_map(|x| test_put(x + 1)).then(test_get());
        let (result, state) = run_state_program(program, 10);
        assert_eq!(result, 11);
        assert_eq!(state, 11);
    }

    #[rstest]
    fn test_interpret_multiple_operations() {
        let program = test_get()
            .flat_map(|x| test_put(x + 10))
            .then(test_get())
            .flat_map(|y| test_put(y * 2))
            .then(test_get());
        let (result, state) = run_state_program(program, 5);
        assert_eq!(result, 30);
        assert_eq!(state, 30);
    }

    #[rstest]
    fn test_deep_flat_map_chain_stack_safety() {
        let mut freer: Freer<(), i32> = Freer::pure(0);
        for _ in 0..10_000 {
            freer = freer.flat_map(|x| Freer::pure(x + 1));
        }
        assert_eq!(freer.interpret(|()| Box::new(())), 10_000);
    }

    // Deep flat_map chains with instructions (10,000+) require "Reflection without Remorse"
    // optimization. See: docs/internal/issues/ for the related issue.
    #[rstest]
    fn test_deep_flat_map_chain_with_instructions() {
        let mut freer = test_get();
        for _ in 0..100 {
            freer = freer.flat_map(|x| test_put(x + 1).then(test_get()));
        }
        let (result, state) = run_state_program(freer, 0);
        assert_eq!(result, 100);
        assert_eq!(state, 100);
    }

    #[rstest]
    fn test_debug_pure() {
        let freer: Freer<(), i32> = Freer::pure(42);
        assert_eq!(format!("{freer:?}"), "Pure(42)");
    }

    #[rstest]
    fn test_debug_impure() {
        let freer = test_get();
        let debug_str = format!("{freer:?}");
        assert!(debug_str.contains("Impure"));
        assert!(debug_str.contains("Get"));
        assert!(debug_str.contains("<queue>"));
    }

    #[rstest]
    fn test_debug_impure_with_continuations() {
        let freer = test_get().flat_map(|x| Freer::pure(x * 2));
        let debug_str = format!("{freer:?}");
        assert!(debug_str.contains("Impure"));
        assert!(debug_str.contains("<queue>"));
    }

    #[rstest]
    fn test_display_pure() {
        let freer: Freer<i32, i32> = Freer::pure(42);
        assert_eq!(format!("{freer}"), "Pure(42)");
    }

    #[rstest]
    fn test_display_impure() {
        #[derive(Debug)]
        enum DisplayCommand {
            Ping,
        }

        impl std::fmt::Display for DisplayCommand {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Ping")
            }
        }

        let freer: Freer<DisplayCommand, i32> =
            Freer::<DisplayCommand, i32>::lift_instruction(DisplayCommand::Ping, |_| 0);
        assert_eq!(format!("{freer}"), "Impure(Ping)");
    }

    #[rstest]
    fn test_display_impure_with_continuations() {
        #[derive(Debug)]
        enum DisplayCommand {
            Get,
        }

        impl std::fmt::Display for DisplayCommand {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Get")
            }
        }

        let freer: Freer<DisplayCommand, i32> =
            Freer::<DisplayCommand, i32>::lift_instruction(DisplayCommand::Get, |r| {
                *r.downcast::<i32>().expect("expected i32")
            })
            .flat_map(|x| Freer::pure(x * 2));
        assert_eq!(format!("{freer}"), "Impure(Get)");
    }

    #[rstest]
    fn test_monad_left_identity() {
        let value = 10;
        let function = |x: i32| Freer::<(), i32>::pure(x * 2);

        let left = Freer::<(), i32>::pure(value).flat_map(function);
        let right = function(value);

        assert_eq!(
            left.interpret(|()| Box::new(())),
            right.interpret(|()| Box::new(()))
        );
    }

    #[rstest]
    fn test_monad_right_identity() {
        let result = Freer::<(), i32>::pure(42).flat_map(Freer::pure);
        assert_eq!(result.interpret(|()| Box::new(())), 42);
    }

    #[rstest]
    fn test_monad_associativity() {
        fn f(x: i32) -> Freer<(), i32> {
            Freer::pure(x + 10)
        }
        fn g(x: i32) -> Freer<(), i32> {
            Freer::pure(x * 2)
        }

        let left = Freer::<(), i32>::pure(5).flat_map(f).flat_map(g);
        let right = Freer::<(), i32>::pure(5).flat_map(|x| f(x).flat_map(g));

        assert_eq!(
            left.interpret(|()| Box::new(())),
            right.interpret(|()| Box::new(()))
        );
    }

    #[rstest]
    fn test_functor_identity() {
        let result = Freer::<(), i32>::pure(42).map(|x| x);
        assert_eq!(result.interpret(|()| Box::new(())), 42);
    }

    #[rstest]
    fn test_functor_composition() {
        fn f(x: i32) -> i32 {
            x + 10
        }
        fn g(x: i32) -> i32 {
            x * 2
        }

        let left = Freer::<(), i32>::pure(5).map(f).map(g);
        let right = Freer::<(), i32>::pure(5).map(|x| g(f(x)));

        assert_eq!(
            left.interpret(|()| Box::new(())),
            right.interpret(|()| Box::new(()))
        );
    }

    // =========================================================================
    // Internal structure tests (ContinuationQueue, QueueStack)
    // =========================================================================

    #[rstest]
    fn test_continuation_queue_new_is_empty() {
        let queue: ContinuationQueue<()> = ContinuationQueue::new();
        assert!(queue.is_empty());
    }

    #[rstest]
    fn test_continuation_queue_push_not_empty() {
        let queue: ContinuationQueue<()> =
            ContinuationQueue::new().push(|x: i32| Freer::<(), i32>::pure(x + 1));
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn test_continuation_queue_push_extract() {
        let queue: ContinuationQueue<()> = ContinuationQueue::new().push_extract(|_| 42i32);
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn test_continuation_queue_pop_fifo_order() {
        let mut queue: ContinuationQueue<()> = ContinuationQueue::new()
            .push(|x: i32| Freer::<(), i32>::pure(x + 1))
            .push(|x: i32| Freer::<(), i32>::pure(x * 2));

        // First pop should give us the +1 arrow
        let arrow1 = queue.pop().expect("Should have first arrow");
        let result1 = arrow1.apply(Box::new(10i32));
        if let Freer::Pure(boxed) = result1 {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 11); // 10 + 1
        } else {
            panic!("Expected Pure");
        }

        // Second pop should give us the *2 arrow
        let arrow2 = queue.pop().expect("Should have second arrow");
        let result2 = arrow2.apply(Box::new(10i32));
        if let Freer::Pure(boxed) = result2 {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 20); // 10 * 2
        } else {
            panic!("Expected Pure");
        }

        // Queue should be empty now
        assert!(queue.pop().is_none());
    }

    #[rstest]
    fn test_queue_stack_single_queue() {
        let queue = ContinuationQueue::new().push(|x: i32| Freer::<(), i32>::pure(x + 1));
        let mut stack = QueueStack::new(queue);

        assert!(stack.pop().is_some());
        assert!(stack.pop().is_none());
    }

    #[rstest]
    fn test_queue_stack_multiple_queues() {
        let queue1 = ContinuationQueue::new().push(|x: i32| Freer::<(), i32>::pure(x + 1));
        let queue2 = ContinuationQueue::new().push(|x: i32| Freer::<(), i32>::pure(x * 2));

        let mut stack = QueueStack::new(queue1);
        stack.push_queue(queue2);

        // queue2 should be processed first (LIFO), then queue1
        let arrow1 = stack.pop().expect("Should have arrow from queue2");
        let result1 = arrow1.apply(Box::new(5i32));
        if let Freer::Pure(boxed) = result1 {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 10); // 5 * 2
        } else {
            panic!("Expected Pure");
        }

        let arrow2 = stack.pop().expect("Should have arrow from queue1");
        let result2 = arrow2.apply(Box::new(5i32));
        if let Freer::Pure(boxed) = result2 {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 6); // 5 + 1
        } else {
            panic!("Expected Pure");
        }

        assert!(stack.pop().is_none());
    }

    #[rstest]
    fn test_arrow_apply_type_conversion() {
        let arrow = Arrow::<(), i32, i32, _> {
            function: |x: i32| Freer::<(), i32>::pure(x * 2),
            _phantom: PhantomData,
        };
        let boxed_arrow: Box<dyn TypeErasedArrow<()>> = Box::new(arrow);
        let result = boxed_arrow.apply(Box::new(21i32));

        if let Freer::Pure(boxed) = result {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 42);
        } else {
            panic!("Expected Pure");
        }
    }

    #[rstest]
    fn test_extract_arrow_apply() {
        let arrow = ExtractArrow::<i32, _> {
            extract: |boxed: Box<dyn Any>| *boxed.downcast::<i32>().unwrap() + 1,
            _phantom: PhantomData,
        };
        let boxed_arrow: Box<dyn TypeErasedArrow<()>> = Box::new(arrow);
        let result = boxed_arrow.apply(Box::new(41i32));

        if let Freer::Pure(boxed) = result {
            let value = *boxed.downcast::<i32>().unwrap();
            assert_eq!(value, 42);
        } else {
            panic!("Expected Pure");
        }
    }

    // =========================================================================
    // Performance tests
    // =========================================================================

    #[rstest]
    fn test_deep_flat_map_chain_with_instructions_1000() {
        // This test should complete quickly with the new implementation
        // (previously would take ~6.72s, now should be ~70ms or less)
        let mut freer = test_get();
        for _ in 0..1000 {
            freer = freer.flat_map(|x| test_put(x + 1).then(test_get()));
        }
        let (result, state) = run_state_program(freer, 0);
        assert_eq!(result, 1000);
        assert_eq!(state, 1000);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_monad_left_identity(value in any::<i32>()) {
            let f = |x: i32| Freer::<(), i32>::pure(x.wrapping_mul(2));

            let left = Freer::<(), i32>::pure(value).flat_map(f);
            let right = f(value);

            prop_assert_eq!(
                left.interpret(|()| Box::new(())),
                right.interpret(|()| Box::new(()))
            );
        }

        #[test]
        fn prop_monad_right_identity(value in any::<i32>()) {
            let result = Freer::<(), i32>::pure(value).flat_map(Freer::pure);
            prop_assert_eq!(result.interpret(|()| Box::new(())), value);
        }

        #[test]
        fn prop_monad_associativity(value in any::<i32>()) {
            fn f(x: i32) -> Freer<(), i32> {
                Freer::pure(x.wrapping_add(10))
            }
            fn g(x: i32) -> Freer<(), i32> {
                Freer::pure(x.wrapping_mul(2))
            }

            let left = Freer::<(), i32>::pure(value).flat_map(f).flat_map(g);
            let right = Freer::<(), i32>::pure(value).flat_map(|x| f(x).flat_map(g));

            prop_assert_eq!(
                left.interpret(|()| Box::new(())),
                right.interpret(|()| Box::new(()))
            );
        }

        #[test]
        fn prop_functor_identity(value in any::<i32>()) {
            let result = Freer::<(), i32>::pure(value).map(|x| x);
            prop_assert_eq!(result.interpret(|()| Box::new(())), value);
        }

        #[test]
        fn prop_functor_composition(value in any::<i32>()) {
            fn f(x: i32) -> i32 {
                x.wrapping_add(10)
            }
            fn g(x: i32) -> i32 {
                x.wrapping_mul(2)
            }

            let left = Freer::<(), i32>::pure(value).map(f).map(g);
            let right = Freer::<(), i32>::pure(value).map(|x| g(f(x)));

            prop_assert_eq!(
                left.interpret(|()| Box::new(())),
                right.interpret(|()| Box::new(()))
            );
        }
    }
}
