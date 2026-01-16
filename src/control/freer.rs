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
//!             | Impure { instruction: I, continuation: Box<dyn Any> -> Freer<I, A> }
//!             | FlatMapInternal(ContinuationBox)  -- for stack safety
//! ```
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
use std::fmt::{self, Debug, Display, Formatter};

type TypeErasedContinuation<I, A> = Box<dyn FnOnce(Box<dyn Any>) -> Freer<I, A>>;

trait FreerContinuation<I, A> {
    fn step(self: Box<Self>) -> Freer<I, A>;
}

#[doc(hidden)]
pub struct ContinuationBox<I, A>(Box<dyn FreerContinuation<I, A>>);

impl<I, A> ContinuationBox<I, A> {
    fn new<T: FreerContinuation<I, A> + 'static>(continuation: T) -> Self {
        Self(Box::new(continuation))
    }

    #[inline]
    fn step(self) -> Freer<I, A> {
        self.0.step()
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
/// `FlatMapInternal` variant and loop-based `interpret` implementation.
///
/// # Note
///
/// This type does NOT implement `TypeConstructor` because `Freer<I, A>` requires
/// `A: 'static` for certain operations, which would propagate and limit usability.
pub enum Freer<I, A> {
    /// A pure value, no instructions to execute.
    Pure(A),

    /// An instruction with a type-erased continuation.
    Impure {
        /// The instruction to execute.
        instruction: I,
        /// The continuation to apply after executing the instruction.
        continuation: TypeErasedContinuation<I, A>,
    },

    /// Internal state for stack-safe flat_map composition.
    #[doc(hidden)]
    FlatMapInternal(ContinuationBox<I, A>),
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
            continuation: Box::new(move |result| Freer::Pure(extract(result))),
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
            _ => self.flat_map(move |a| Freer::pure(function(a))),
        }
    }

    /// Chains computations together (Monad `bind`/`>>=`).
    ///
    /// When `Pure`, the function is applied immediately.
    /// Otherwise, `FlatMapInternal` is used for stack safety.
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
            _ => Freer::FlatMapInternal(ContinuationBox::new(FlatMapContinuation {
                freer: self,
                function,
            })),
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
    /// Uses a loop-based approach for stack safety with deep `flat_map` chains.
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
    pub fn interpret<Handler>(self, mut handler: Handler) -> A
    where
        Handler: FnMut(I) -> Box<dyn Any>,
    {
        let mut current = self;

        loop {
            while let Self::FlatMapInternal(continuation_box) = current {
                current = continuation_box.step();
            }

            current = match current {
                Self::Pure(a) => return a,
                Self::Impure {
                    instruction,
                    continuation,
                } => continuation(handler(instruction)),
                Self::FlatMapInternal(_) => {
                    unreachable!("FlatMapInternal should have been expanded")
                }
            };
        }
    }
}

struct FlatMapContinuation<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B>,
{
    freer: Freer<I, A>,
    function: F,
}

#[allow(clippy::use_self)]
impl<I: 'static, A: 'static, B: 'static, F> FreerContinuation<I, B>
    for FlatMapContinuation<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B> + 'static,
{
    fn step(self: Box<Self>) -> Freer<I, B> {
        match self.freer {
            Freer::Pure(a) => (self.function)(a),
            Freer::Impure {
                instruction,
                continuation,
            } => {
                let function = self.function;
                Freer::Impure {
                    instruction,
                    continuation: Box::new(move |result| {
                        Freer::FlatMapInternal(ContinuationBox::new(FlatMapContinuation {
                            freer: continuation(result),
                            function,
                        }))
                    }),
                }
            }
            Freer::FlatMapInternal(inner) => {
                // Use associativity: (m >>= f) >>= g == m >>= (\x -> f x >>= g)
                Freer::FlatMapInternal(ContinuationBox::new(ComposedContinuation {
                    first: inner,
                    second: self.function,
                }))
            }
        }
    }
}

struct ComposedContinuation<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B>,
{
    first: ContinuationBox<I, A>,
    second: F,
}

#[allow(clippy::use_self)]
impl<I: 'static, A: 'static, B: 'static, F> FreerContinuation<I, B>
    for ComposedContinuation<I, A, B, F>
where
    F: FnOnce(A) -> Freer<I, B> + 'static,
{
    fn step(self: Box<Self>) -> Freer<I, B> {
        match self.first.step() {
            Freer::Pure(a) => (self.second)(a),
            Freer::Impure {
                instruction,
                continuation,
            } => {
                let second = self.second;
                Freer::Impure {
                    instruction,
                    continuation: Box::new(move |result| {
                        Freer::FlatMapInternal(ContinuationBox::new(FlatMapContinuation {
                            freer: continuation(result),
                            function: second,
                        }))
                    }),
                }
            }
            Freer::FlatMapInternal(inner) => {
                Freer::FlatMapInternal(ContinuationBox::new(ComposedContinuation {
                    first: inner,
                    second: self.second,
                }))
            }
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
                .field("continuation", &"<continuation>")
                .finish(),
            Self::FlatMapInternal(_) => formatter
                .debug_tuple("FlatMapInternal")
                .field(&"<continuation>")
                .finish(),
        }
    }
}

impl<I: Display, A: Display> Display for Freer<I, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure(a) => write!(formatter, "Pure({a})"),
            Self::Impure { instruction, .. } => write!(formatter, "Impure({instruction})"),
            Self::FlatMapInternal(_) => write!(formatter, "<FlatMap>"),
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
    fn test_map_impure_creates_flatmap_internal() {
        let mapped = test_get().map(|x| x * 2);
        assert!(matches!(mapped, Freer::FlatMapInternal(_)));
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
    fn test_flat_map_impure_creates_flatmap_internal() {
        let chained = test_get().flat_map(|x| Freer::pure(x * 2));
        assert!(matches!(chained, Freer::FlatMapInternal(_)));
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
        assert!(debug_str.contains("<continuation>"));
    }

    #[rstest]
    fn test_debug_flat_map_internal() {
        let freer = test_get().flat_map(|x| Freer::pure(x * 2));
        assert!(format!("{freer:?}").contains("FlatMapInternal"));
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
    fn test_display_flat_map_internal() {
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
        assert_eq!(format!("{freer}"), "<FlatMap>");
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
}
