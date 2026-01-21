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
//! # Performance
//!
//! This implementation uses SmallVec-based continuation storage for improved
//! performance. Short chains (8 elements or fewer) are stored inline without
//! heap allocation.
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

use smallvec::SmallVec;
use std::any::Any;
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

// =============================================================================
// Continuation Types
// =============================================================================

/// Type-erased arrow (continuation).
///
/// Converts `A -> Freer<I, B>` to `Box<dyn Any> -> Freer<I, Box<dyn Any>>`.
/// This enables storing heterogeneous continuations in a single queue.
trait TypeErasedArrow<I> {
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>>;
}

struct FlatMapArrow<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B>,
{
    function: F,
    _phantom: PhantomData<fn(A) -> (I, B)>,
}

impl<I: 'static, A: 'static, B: 'static, F> TypeErasedArrow<I> for FlatMapArrow<I, A, B, F>
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
                mut continuation_queue,
                ..
            } => {
                continuation_queue
                    .arrows
                    .push(Box::new(BoxingArrow::<B>(PhantomData)));
                Freer::Impure {
                    instruction,
                    continuation_queue,
                    _result: PhantomData,
                }
            }
        }
    }
}

struct MapArrow<A, B, F>
where
    F: FnOnce(A) -> B,
{
    function: F,
    _phantom: PhantomData<fn(A) -> B>,
}

impl<I: 'static, A: 'static, B: 'static, F> TypeErasedArrow<I> for MapArrow<A, B, F>
where
    F: FnOnce(A) -> B + 'static,
{
    #[inline]
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>> {
        let value = *input
            .downcast::<A>()
            .expect("Type mismatch in map application");
        Freer::Pure(Box::new((self.function)(value)) as Box<dyn Any>)
    }
}

struct BoxingArrow<T>(PhantomData<T>);

impl<I: 'static, T: 'static> TypeErasedArrow<I> for BoxingArrow<T> {
    #[inline]
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
    #[inline]
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Freer<I, Box<dyn Any>> {
        Freer::Pure(Box::new((self.extract)(input)) as Box<dyn Any>)
    }
}

// =============================================================================
// Continuation Queue (SmallVec-based)
// =============================================================================

const CONTINUATION_INLINE_CAPACITY: usize = 8;

/// A queue of type-erased continuations.
///
/// Uses SmallVec for inline storage of up to 8 continuations,
/// avoiding heap allocation for short chains.
#[doc(hidden)]
pub struct ContinuationQueue<I> {
    arrows: SmallVec<[Box<dyn TypeErasedArrow<I>>; CONTINUATION_INLINE_CAPACITY]>,
}

impl<I> ContinuationQueue<I> {
    #[inline]
    fn new() -> Self {
        Self {
            arrows: SmallVec::new(),
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.arrows.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.arrows.len()
    }
}

impl<I: 'static> ContinuationQueue<I> {
    #[inline]
    fn push_flat_map<A: 'static, B: 'static, F>(mut self, function: F) -> Self
    where
        F: FnOnce(A) -> Freer<I, B> + 'static,
    {
        self.arrows.push(Box::new(FlatMapArrow {
            function,
            _phantom: PhantomData,
        }));
        self
    }

    #[inline]
    fn push_map<A: 'static, B: 'static, F>(mut self, function: F) -> Self
    where
        F: FnOnce(A) -> B + 'static,
    {
        self.arrows.push(Box::new(MapArrow {
            function,
            _phantom: PhantomData,
        }));
        self
    }

    #[inline]
    fn push_extract<R: 'static, E>(mut self, extract: E) -> Self
    where
        E: FnOnce(Box<dyn Any>) -> R + 'static,
    {
        self.arrows.push(Box::new(ExtractArrow {
            extract,
            _phantom: PhantomData,
        }));
        self
    }
}

// =============================================================================
// Continuation Stack (for interpretation)
// =============================================================================

struct ContinuationStack<I> {
    current_index: usize,
    current: ContinuationQueue<I>,
    pending: SmallVec<[(usize, ContinuationQueue<I>); CONTINUATION_INLINE_CAPACITY]>,
}

impl<I: 'static> ContinuationStack<I> {
    #[inline]
    fn new(initial: ContinuationQueue<I>) -> Self {
        Self {
            current_index: 0,
            current: initial,
            pending: SmallVec::new(),
        }
    }

    #[inline]
    fn push_queue(&mut self, queue: ContinuationQueue<I>) {
        if self.current_index < self.current.len() {
            let old = std::mem::replace(&mut self.current, queue);
            self.pending.push((self.current_index, old));
        } else {
            self.current = queue;
        }
        self.current_index = 0;
    }

    #[inline]
    fn pop(&mut self) -> Option<Box<dyn TypeErasedArrow<I>>> {
        loop {
            if self.current_index < self.current.arrows.len() {
                let arrow = std::mem::replace(
                    &mut self.current.arrows[self.current_index],
                    Box::new(BoxingArrow::<()>(PhantomData)) as Box<dyn TypeErasedArrow<I>>,
                );
                self.current_index += 1;
                return Some(arrow);
            }
            let (saved_index, saved_queue) = self.pending.pop()?;
            self.current = saved_queue;
            self.current_index = saved_index;
        }
    }
}

// =============================================================================
// InterpretError
// =============================================================================

/// Error type for interpret operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpretError {
    /// Handler returned a value that doesn't match the expected type.
    TypeMismatch {
        /// Description of the context where mismatch occurred.
        context: &'static str,
    },
}

impl Display for InterpretError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch { context } => {
                write!(f, "Type mismatch in interpret: {context}")
            }
        }
    }
}

impl std::error::Error for InterpretError {}

// =============================================================================
// Freer Monad
// =============================================================================

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
/// Additionally, uses SmallVec-based storage for:
/// - Inline storage of up to 8 continuations (no heap allocation)
/// - Reduced allocation overhead for short chains
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
        continuation_queue: ContinuationQueue<I>,
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
            continuation_queue: ContinuationQueue::new().push_extract(extract),
            _result: PhantomData,
        }
    }

    /// Applies a function to the result (Functor `fmap`).
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
    #[inline]
    pub fn map<B: 'static, F>(self, function: F) -> Freer<I, B>
    where
        F: FnOnce(A) -> B + 'static,
    {
        match self {
            Self::Pure(a) => Freer::Pure(function(a)),
            Self::Impure {
                instruction,
                continuation_queue,
                ..
            } => Freer::Impure {
                instruction,
                continuation_queue: continuation_queue.push_map(function),
                _result: PhantomData,
            },
        }
    }

    /// Chains computations together (Monad `bind`/`>>=`).
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
    #[inline]
    pub fn flat_map<B: 'static, F>(self, function: F) -> Freer<I, B>
    where
        F: FnOnce(A) -> Freer<I, B> + 'static,
    {
        match self {
            Self::Pure(a) => function(a),
            Self::Impure {
                instruction,
                continuation_queue,
                ..
            } => Freer::Impure {
                instruction,
                continuation_queue: continuation_queue.push_flat_map(function),
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
    /// Uses a non-recursive loop with `ContinuationStack` for O(n) interpretation.
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
    pub fn interpret<Handler>(self, handler: Handler) -> A
    where
        Handler: FnMut(I) -> Box<dyn Any>,
    {
        self.try_interpret(handler)
            .expect("Final result type mismatch")
    }

    /// Interprets the Freer computation with Result-based error handling.
    ///
    /// Unlike `interpret`, this method returns `Result<A, InterpretError>` instead
    /// of panicking on type mismatch. Use this when you need graceful error handling.
    ///
    /// # Errors
    ///
    /// Returns `Err(InterpretError::TypeMismatch)` if the final result type does not
    /// match the expected type `A`. This indicates a bug in the DSL design or handler
    /// implementation where the handler returns a value of unexpected type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::{Freer, InterpretError};
    ///
    /// let freer: Freer<(), i32> = Freer::pure(42);
    /// let result = freer.try_interpret(|_| Box::new(()));
    /// assert_eq!(result, Ok(42));
    /// ```
    pub fn try_interpret<Handler>(self, mut handler: Handler) -> Result<A, InterpretError>
    where
        Handler: FnMut(I) -> Box<dyn Any>,
    {
        let (instruction, queue) = match self {
            Self::Pure(a) => return Ok(a),
            Self::Impure {
                instruction,
                continuation_queue,
                ..
            } => (instruction, continuation_queue),
        };

        let mut stack = ContinuationStack::new(queue);
        let mut current_value: Box<dyn Any> = handler(instruction);

        loop {
            let Some(arrow) = stack.pop() else {
                return current_value.downcast::<A>().map(|b| *b).map_err(|_| {
                    InterpretError::TypeMismatch {
                        context: "final result",
                    }
                });
            };

            match arrow.apply(current_value) {
                Freer::Pure(value) => current_value = value,
                Freer::Impure {
                    instruction,
                    continuation_queue,
                    ..
                } => {
                    stack.push_queue(continuation_queue);
                    current_value = handler(instruction);
                }
            }
        }
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl<I: Debug, A: Debug> Debug for Freer<I, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure(a) => formatter.debug_tuple("Pure").field(a).finish(),
            Self::Impure { instruction, .. } => formatter
                .debug_struct("Impure")
                .field("instruction", instruction)
                .field("continuation_queue", &"<queue>")
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

// =============================================================================
// Tests
// =============================================================================

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

    // =========================================================================
    // Pure construction tests
    // =========================================================================

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

    // =========================================================================
    // lift_instruction tests
    // =========================================================================

    #[rstest]
    fn test_lift_instruction_creates_impure() {
        let freer = test_get();
        assert!(matches!(freer, Freer::Impure { .. }));
    }

    // =========================================================================
    // map tests (with fast-path)
    // =========================================================================

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

    // =========================================================================
    // flat_map tests (with fast-path)
    // =========================================================================

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

    // =========================================================================
    // and_then and then tests
    // =========================================================================

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

    // =========================================================================
    // interpret tests
    // =========================================================================

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

    // =========================================================================
    // try_interpret tests
    // =========================================================================

    #[rstest]
    fn test_try_interpret_pure() {
        let result = Freer::<(), i32>::pure(42).try_interpret(|()| Box::new(()));
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn test_try_interpret_with_handler() {
        let program = test_get();
        let mut state = 42;
        let result = program.try_interpret(|command| match command {
            TestCommand::Get => Box::new(state),
            TestCommand::Put(value) => {
                state = value;
                Box::new(())
            }
        });
        assert_eq!(result, Ok(42));
    }

    // =========================================================================
    // Stack safety tests
    // =========================================================================

    #[rstest]
    fn test_deep_flat_map_chain_stack_safety() {
        let mut freer: Freer<(), i32> = Freer::pure(0);
        for _ in 0..10_000 {
            freer = freer.flat_map(|x| Freer::pure(x + 1));
        }
        assert_eq!(freer.interpret(|()| Box::new(())), 10_000);
    }

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
    fn test_deep_flat_map_chain_with_instructions_1000() {
        // This test should complete quickly with the optimized implementation
        let mut freer = test_get();
        for _ in 0..1000 {
            freer = freer.flat_map(|x| test_put(x + 1).then(test_get()));
        }
        let (result, state) = run_state_program(freer, 0);
        assert_eq!(result, 1000);
        assert_eq!(state, 1000);
    }

    // =========================================================================
    // Debug and Display tests
    // =========================================================================

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

    // =========================================================================
    // Monad law tests
    // =========================================================================

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

    // =========================================================================
    // Functor law tests
    // =========================================================================

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
    // Internal structure tests
    // =========================================================================

    #[rstest]
    fn test_continuation_queue_new_is_empty() {
        let queue: ContinuationQueue<()> = ContinuationQueue::new();
        assert!(queue.is_empty());
    }

    #[rstest]
    fn test_continuation_queue_push_not_empty() {
        let queue: ContinuationQueue<()> =
            ContinuationQueue::new().push_flat_map(|x: i32| Freer::<(), i32>::pure(x + 1));
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn test_continuation_queue_push_extract() {
        let queue: ContinuationQueue<()> = ContinuationQueue::new().push_extract(|_| 42i32);
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn test_continuation_stack_single_queue() {
        let queue = ContinuationQueue::new().push_flat_map(|x: i32| Freer::<(), i32>::pure(x + 1));
        let mut stack = ContinuationStack::new(queue);

        assert!(stack.pop().is_some());
        assert!(stack.pop().is_none());
    }

    #[rstest]
    fn test_continuation_stack_multiple_queues() {
        let queue1 = ContinuationQueue::new().push_flat_map(|x: i32| Freer::<(), i32>::pure(x + 1));
        let queue2 = ContinuationQueue::new().push_flat_map(|x: i32| Freer::<(), i32>::pure(x * 2));

        let mut stack = ContinuationStack::new(queue1);
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

    // =========================================================================
    // SmallVec inline capacity test
    // =========================================================================

    #[rstest]
    fn test_smallvec_inline_capacity() {
        // Test that short chains stay inline
        let expected_count: i32 = 8; // CONTINUATION_INLINE_CAPACITY
        let mut freer: Freer<(), i32> = Freer::pure(0);
        for _ in 0..expected_count {
            freer = freer.flat_map(|x| Freer::pure(x + 1));
        }
        // This should not cause heap allocation for Pure chains (fast-path)
        assert_eq!(freer.interpret(|()| Box::new(())), expected_count);
    }

    // =========================================================================
    // InterpretError tests
    // =========================================================================

    #[rstest]
    fn test_interpret_error_display() {
        let error = InterpretError::TypeMismatch {
            context: "test context",
        };
        assert_eq!(
            format!("{error}"),
            "Type mismatch in interpret: test context"
        );
    }

    #[rstest]
    fn test_interpret_error_debug() {
        let error = InterpretError::TypeMismatch {
            context: "test context",
        };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("TypeMismatch"));
        assert!(debug_str.contains("test context"));
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
