//! IO Monad - Deferred side effect handling.
//!
//! The `IO` type represents a computation that may perform side effects.
//! Side effects are not executed until `run_unsafe` is called, maintaining
//! referential transparency in pure code.
//!
//! # Design Philosophy
//!
//! IO "describes" side effects but doesn't "execute" them. Execution happens
//! only via `run_unsafe`, which should be called at the program's "edge"
//! (e.g., in the `main` function).
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::IO;
//! use lambars::typeclass::{Functor, Monad};
//!
//! // Create a pure IO action
//! let io = IO::pure(42);
//! assert_eq!(io.run_unsafe(), 42);
//!
//! // Chain IO actions using Functor and Monad traits
//! let io = IO::pure(10)
//!     .fmap(|x| x * 2)
//!     .flat_map(|x| IO::pure(x + 1));
//! assert_eq!(io.run_unsafe(), 21);
//! ```
//!
//! # Side Effect Deferral
//!
//! ```rust
//! use lambars::effect::IO;
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use std::sync::Arc;
//!
//! let executed = Arc::new(AtomicBool::new(false));
//! let executed_clone = executed.clone();
//!
//! let io = IO::new(move || {
//!     executed_clone.store(true, Ordering::SeqCst);
//!     42
//! });
//!
//! // Not executed yet
//! assert!(!executed.load(Ordering::SeqCst));
//!
//! // Execute the IO action
//! let result = io.run_unsafe();
//! assert!(executed.load(Ordering::SeqCst));
//! assert_eq!(result, 42);
//! ```

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

/// A monad representing deferred side effects.
///
/// `IO<A>` wraps a computation that produces a value of type `A` and may
/// perform side effects. The computation is not executed until `run_unsafe`
/// is called.
///
/// # Type Parameters
///
/// - `A`: The type of the value produced by the IO action.
///
/// # Monad Laws
///
/// `IO` satisfies the monad laws:
///
/// 1. **Left Identity**: `IO::pure(a).flat_map(f) == f(a)`
/// 2. **Right Identity**: `m.flat_map(IO::pure) == m`
/// 3. **Associativity**: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
pub struct IO<A> {
    /// The wrapped computation that produces a value of type `A`.
    run_io: Box<dyn FnOnce() -> A>,
}

impl<A: 'static> IO<A> {
    /// Creates a new IO action from a closure.
    ///
    /// The closure will not be executed until `run_unsafe` is called.
    ///
    /// # Arguments
    ///
    /// * `action` - A closure that produces a value of type `A`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::IO;
    ///
    /// let io = IO::new(|| {
    ///     println!("Side effect!");
    ///     42
    /// });
    /// // Nothing is printed yet
    /// let result = io.run_unsafe();
    /// // Now "Side effect!" is printed
    /// assert_eq!(result, 42);
    /// ```
    pub fn new<F>(action: F) -> Self
    where
        F: FnOnce() -> A + 'static,
    {
        Self {
            run_io: Box::new(action),
        }
    }

    /// Wraps a pure value in an IO action.
    ///
    /// This creates an IO action that returns the given value without
    /// performing any side effects.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::IO;
    ///
    /// let io = IO::pure(42);
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    pub fn pure(value: A) -> Self {
        Self::new(move || value)
    }

    /// Executes the IO action and returns the result.
    ///
    /// This is the only way to extract a value from an IO action.
    /// It should be called at the program's "edge" (e.g., in `main`).
    ///
    /// # Safety Note
    ///
    /// This method is named `run_unsafe` to indicate that it executes
    /// side effects. While it's memory-safe, calling it breaks referential
    /// transparency.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::IO;
    ///
    /// let io = IO::pure(42);
    /// let result = io.run_unsafe();
    /// assert_eq!(result, 42);
    /// ```
    #[must_use]
    pub fn run_unsafe(self) -> A {
        (self.run_io)()
    }
}

// Note: fmap, flat_map, and_then, then, map2, product methods are available
// through the Functor, Applicative, and Monad trait implementations.
// Import these traits to use them:
//   use lambars::typeclass::{Functor, Applicative, Monad};

// =============================================================================
// Convenience Constructors
// =============================================================================

impl IO<()> {
    /// Creates an IO action that prints a line to standard output.
    ///
    /// The output is not printed until `run_unsafe` is called.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to print.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lambars::effect::IO;
    ///
    /// let io = IO::print_line("Hello, World!");
    /// io.run_unsafe(); // Prints "Hello, World!"
    /// ```
    pub fn print_line<S: std::fmt::Display + 'static>(message: S) -> Self {
        Self::new(move || {
            println!("{message}");
        })
    }

    /// Creates an IO action that waits for a specified duration.
    ///
    /// The delay does not occur until `run_unsafe` is called.
    ///
    /// # Arguments
    ///
    /// * `duration` - How long to wait.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::IO;
    /// use std::time::Duration;
    ///
    /// let io = IO::delay(Duration::from_millis(100));
    /// io.run_unsafe(); // Waits for 100ms
    /// ```
    #[must_use]
    pub fn delay(duration: Duration) -> Self {
        Self::new(move || {
            std::thread::sleep(duration);
        })
    }
}

impl IO<std::io::Result<String>> {
    /// Creates an IO action that reads a line from standard input.
    ///
    /// The input is not read until `run_unsafe` is called.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lambars::effect::IO;
    ///
    /// let io = IO::read_line();
    /// let line = io.run_unsafe().expect("Failed to read line");
    /// println!("You entered: {}", line);
    /// ```
    #[must_use]
    pub fn read_line() -> Self {
        Self::new(|| {
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer)?;
            Ok(buffer)
        })
    }
}

impl<A: 'static> IO<A> {
    /// Catches panics in an IO action and converts them to a recovery value.
    ///
    /// If the IO action panics, the handler is called with the panic info
    /// (as a string) and should return a recovery value.
    ///
    /// # Arguments
    ///
    /// * `io` - The IO action that might panic.
    /// * `handler` - A function to handle the panic and return a recovery value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::IO;
    ///
    /// let panicking = IO::new(|| panic!("oops"));
    /// let recovered = IO::catch(panicking, |_| "recovered".to_string());
    /// assert_eq!(recovered.run_unsafe(), "recovered");
    /// ```
    ///
    /// ```rust
    /// use lambars::effect::IO;
    ///
    /// let successful = IO::pure(42);
    /// let with_catch = IO::catch(successful, |_| 0);
    /// assert_eq!(with_catch.run_unsafe(), 42);
    /// ```
    pub fn catch<F>(io: Self, handler: F) -> Self
    where
        F: FnOnce(String) -> A + 'static,
    {
        Self::new(move || {
            let result = catch_unwind(AssertUnwindSafe(|| io.run_unsafe()));
            match result {
                Ok(value) => value,
                Err(panic_info) => {
                    let message = panic_info
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_string())
                        .or_else(|| panic_info.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "Unknown panic".to_string());
                    handler(message)
                }
            }
        })
    }
}

// =============================================================================
// Conversion to AsyncIO (requires async feature)
// =============================================================================

#[cfg(feature = "async")]
impl<A: Send + 'static> IO<A> {
    /// Converts a synchronous IO to an `AsyncIO`.
    ///
    /// # Important: Immediate Execution
    ///
    /// The IO action is executed **immediately** when `to_async` is called,
    /// not when `run_async` is called on the resulting `AsyncIO`. This is
    /// because `IO` is not `Send` (it contains `Box<dyn FnOnce() -> A>`
    /// without a `Send` bound) and cannot be moved to an async context.
    ///
    /// If you need deferred execution in an async context, use
    /// `AsyncIO::new` directly with your computation instead.
    ///
    /// # Example: Understanding Immediate Execution
    ///
    /// ```rust
    /// use lambars::effect::{IO, AsyncIO};
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use std::sync::Arc;
    ///
    /// let executed = Arc::new(AtomicBool::new(false));
    /// let executed_clone = executed.clone();
    ///
    /// let io = IO::new(move || {
    ///     executed_clone.store(true, Ordering::SeqCst);
    ///     42
    /// });
    ///
    /// // The IO action executes HERE, not when run_async is called
    /// let async_io = io.to_async();
    /// assert!(executed.load(Ordering::SeqCst)); // Already executed!
    /// ```
    ///
    /// # Recommended Alternative
    ///
    /// For true deferred execution, use `AsyncIO::new` directly:
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// // This is deferred - executes only when run_async is called
    /// let async_io = AsyncIO::new(|| async {
    ///     println!("This runs when awaited");
    ///     42
    /// });
    /// ```
    #[must_use]
    pub fn to_async(self) -> super::AsyncIO<A> {
        let result = self.run_unsafe();
        super::AsyncIO::pure(result)
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl<A> std::fmt::Display for IO<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "<IO>")
    }
}

// =============================================================================
// TypeConstructor Implementation
// =============================================================================

impl<A> crate::typeclass::TypeConstructor for IO<A> {
    type Inner = A;
    type WithType<B> = IO<B>;
}

// =============================================================================
// Functor Implementation
// =============================================================================

impl<A: 'static> crate::typeclass::Functor for IO<A> {
    fn fmap<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> B + 'static,
        B: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            function(a)
        })
    }

    fn fmap_ref<B, F>(&self, _function: F) -> Self::WithType<B>
    where
        F: FnOnce(&Self::Inner) -> B + 'static,
        B: 'static,
    {
        // IO cannot implement fmap_ref properly because the value is not available
        // until the IO is executed. We would need to execute the IO to get a reference.
        // This is a limitation of IO's deferred execution model.
        unimplemented!(
            "IO::fmap_ref is not available. Use fmap instead, which executes the IO lazily."
        )
    }
}

// =============================================================================
// Applicative Implementation
// =============================================================================

impl<A: 'static> crate::typeclass::Applicative for IO<A> {
    fn pure<B>(value: B) -> Self::WithType<B>
    where
        B: 'static,
    {
        IO::new(move || value)
    }

    fn map2<B, C, F>(self, other: Self::WithType<B>, function: F) -> Self::WithType<C>
    where
        F: FnOnce(A, B) -> C + 'static,
        B: 'static,
        C: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            let b = other.run_unsafe();
            function(a, b)
        })
    }

    fn map3<B, C, D, F>(
        self,
        second: Self::WithType<B>,
        third: Self::WithType<C>,
        function: F,
    ) -> Self::WithType<D>
    where
        F: FnOnce(A, B, C) -> D + 'static,
        B: 'static,
        C: 'static,
        D: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            let b = second.run_unsafe();
            let c = third.run_unsafe();
            function(a, b, c)
        })
    }

    fn apply<B, Output>(self, other: Self::WithType<B>) -> Self::WithType<Output>
    where
        Self: Sized,
        Self::Inner: FnOnce(B) -> Output,
        B: 'static,
        Output: 'static,
    {
        IO::new(move || {
            let function = self.run_unsafe();
            let b = other.run_unsafe();
            function(b)
        })
    }
}

// =============================================================================
// Monad Implementation
// =============================================================================

impl<A: 'static> crate::typeclass::Monad for IO<A> {
    fn flat_map<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> Self::WithType<B> + 'static,
        B: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            let io_b = function(a);
            io_b.run_unsafe()
        })
    }
}

// =============================================================================
// IOLike Implementation
// =============================================================================

impl<A: 'static> crate::typeclass::IOLike for IO<A> {
    type Value = A;

    fn into_io(self) -> Self
    where
        A: 'static,
    {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeclass::{Applicative, Functor, Monad, TypeConstructor};
    use rstest::rstest;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_io() {
        let io = IO::pure(42);
        assert_eq!(format!("{io}"), "<IO>");
    }

    // =========================================================================
    // Original Tests (Updated to use Trait methods)
    // =========================================================================

    #[rstest]
    fn test_io_pure_and_run() {
        let io = IO::pure(42);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[rstest]
    fn test_io_new_and_run() {
        let io = IO::new(|| 10 + 20);
        assert_eq!(io.run_unsafe(), 30);
    }

    #[rstest]
    fn test_io_fmap() {
        let io = Functor::fmap(IO::pure(21), |x| x * 2);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[rstest]
    fn test_io_flat_map() {
        let io = Monad::flat_map(IO::pure(10), |x| IO::pure(x * 2));
        assert_eq!(io.run_unsafe(), 20);
    }

    #[rstest]
    fn test_io_and_then() {
        let io = Monad::and_then(IO::pure(10), |x| IO::pure(x + 5));
        assert_eq!(io.run_unsafe(), 15);
    }

    #[rstest]
    fn test_io_then() {
        let io = IO::pure(10).then(IO::pure(20));
        assert_eq!(io.run_unsafe(), 20);
    }

    #[rstest]
    fn test_io_map2() {
        let io = Applicative::map2(IO::pure(10), IO::pure(20), |a, b| a + b);
        assert_eq!(io.run_unsafe(), 30);
    }

    #[rstest]
    fn test_io_product() {
        let io = Applicative::product(IO::pure(10), IO::pure(20));
        assert_eq!(io.run_unsafe(), (10, 20));
    }

    // =========================================================================
    // TypeConstructor Tests
    // =========================================================================

    #[rstest]
    fn io_type_constructor_inner_type_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = i32>>() {}
        assert_inner::<IO<i32>>();
    }

    #[rstest]
    fn io_type_constructor_with_type_produces_correct_type() {
        // This verifies that IO<i32>::WithType<String> is IO<String>
        fn assert_io_with_type<A: TypeConstructor<WithType<String> = IO<String>>>() {}
        assert_io_with_type::<IO<i32>>();
    }

    // =========================================================================
    // Functor Tests
    // =========================================================================

    #[rstest]
    fn io_functor_fmap() {
        let io = IO::pure(5);
        let result = Functor::fmap(io, |x| x * 2).run_unsafe();
        assert_eq!(result, 10);
    }

    #[rstest]
    fn io_functor_fmap_type_transformation() {
        let io = IO::pure(42);
        let result = Functor::fmap(io, |x| x.to_string()).run_unsafe();
        assert_eq!(result, "42");
    }

    #[rstest]
    fn io_functor_identity_law() {
        let io = IO::pure(42);
        let result = Functor::fmap(io, |x| x).run_unsafe();
        assert_eq!(result, 42);
    }

    #[rstest]
    fn io_functor_composition_law() {
        let function1 = |x: i32| x + 1;
        let function2 = |x: i32| x * 2;

        let io1 = IO::pure(5);
        let io2 = IO::pure(5);

        let result1 = Functor::fmap(Functor::fmap(io1, function1), function2).run_unsafe();
        let result2 = Functor::fmap(io2, move |x| function2(function1(x))).run_unsafe();

        assert_eq!(result1, result2);
        assert_eq!(result1, 12); // (5 + 1) * 2 = 12
    }

    // Note: io_functor_fmap_ref is not tested because IO::fmap_ref is unimplemented
    // due to the deferred execution model of IO (the value is not available until run_unsafe)

    #[rstest]
    fn io_functor_replace() {
        let io = IO::pure(42);
        let result = io.replace("replaced").run_unsafe();
        assert_eq!(result, "replaced");
    }

    #[rstest]
    #[allow(clippy::let_unit_value)]
    fn io_functor_void() {
        let io = IO::pure(42);
        let result: () = io.void().run_unsafe();
        assert_eq!(result, ());
    }

    // =========================================================================
    // Applicative Tests
    // =========================================================================

    #[rstest]
    fn io_applicative_pure() {
        let io: IO<i32> = <IO<i32> as Applicative>::pure(42);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[rstest]
    fn io_applicative_map2() {
        let io1 = IO::pure(2);
        let io2 = IO::pure(3);
        let result = Applicative::map2(io1, io2, |a, b| a + b).run_unsafe();
        assert_eq!(result, 5);
    }

    #[rstest]
    fn io_applicative_map3() {
        let io1 = IO::pure(1);
        let io2 = IO::pure(2);
        let io3 = IO::pure(3);
        let result = Applicative::map3(io1, io2, io3, |a, b, c| a + b + c).run_unsafe();
        assert_eq!(result, 6);
    }

    #[rstest]
    fn io_applicative_product() {
        let io1 = IO::pure(10);
        let io2 = IO::pure("hello");
        let result = Applicative::product(io1, io2).run_unsafe();
        assert_eq!(result, (10, "hello"));
    }

    #[rstest]
    fn io_applicative_product_left() {
        let io1 = IO::pure(10);
        let io2 = IO::pure(20);
        let result = io1.product_left(io2).run_unsafe();
        assert_eq!(result, 10);
    }

    #[rstest]
    fn io_applicative_product_right() {
        let io1 = IO::pure(10);
        let io2 = IO::pure(20);
        let result = io1.product_right(io2).run_unsafe();
        assert_eq!(result, 20);
    }

    #[rstest]
    fn io_applicative_apply() {
        let io_function: IO<fn(i32) -> i32> = IO::pure(|x| x + 1);
        let io_value = IO::pure(5);
        let result = io_function.apply(io_value).run_unsafe();
        assert_eq!(result, 6);
    }

    // =========================================================================
    // Monad Tests
    // =========================================================================

    #[rstest]
    fn io_monad_flat_map() {
        let io = IO::pure(5);
        let result = Monad::flat_map(io, |x| IO::pure(x * 2)).run_unsafe();
        assert_eq!(result, 10);
    }

    #[rstest]
    fn io_monad_and_then() {
        let io = IO::pure(5);
        let result = Monad::and_then(io, |x| IO::pure(x + 3)).run_unsafe();
        assert_eq!(result, 8);
    }

    #[rstest]
    fn io_monad_then() {
        let io1 = IO::pure(10);
        let io2 = IO::pure("hello");
        let result = Monad::then(io1, io2).run_unsafe();
        assert_eq!(result, "hello");
    }

    #[rstest]
    fn io_monad_left_identity_law() {
        let value = 5;
        let function = |x: i32| IO::pure(x * 2);

        let result1 = Monad::flat_map(IO::pure(value), function).run_unsafe();
        let result2 = function(value).run_unsafe();

        assert_eq!(result1, result2);
        assert_eq!(result1, 10);
    }

    #[rstest]
    fn io_monad_right_identity_law() {
        let io = IO::pure(42);
        let result: i32 = Monad::flat_map(io, <IO<i32> as Applicative>::pure).run_unsafe();
        assert_eq!(result, 42);
    }

    #[rstest]
    fn io_monad_associativity_law() {
        let function1 = |x: i32| IO::pure(x + 1);
        let function2 = |x: i32| IO::pure(x * 2);

        let io1 = IO::pure(5);
        let io2 = IO::pure(5);

        let result1 = Monad::flat_map(Monad::flat_map(io1, function1), function2).run_unsafe();
        let result2 =
            Monad::flat_map(io2, move |x| Monad::flat_map(function1(x), function2)).run_unsafe();

        assert_eq!(result1, result2);
        assert_eq!(result1, 12); // (5 + 1) * 2 = 12
    }

    // =========================================================================
    // Side Effect Deferral Tests with Traits
    // =========================================================================

    #[rstest]
    fn io_functor_fmap_defers_execution() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let io = IO::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
            42
        });

        // fmap should not execute the IO
        let mapped = Functor::fmap(io, |x| x * 2);
        assert!(!executed.load(Ordering::SeqCst));

        // run_unsafe should execute
        let result = mapped.run_unsafe();
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(result, 84);
    }

    #[rstest]
    fn io_monad_flat_map_defers_execution() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed1 = Arc::new(AtomicBool::new(false));
        let executed2 = Arc::new(AtomicBool::new(false));
        let executed1_clone = executed1.clone();
        let executed2_clone = executed2.clone();

        let io1 = IO::new(move || {
            executed1_clone.store(true, Ordering::SeqCst);
            10
        });

        let io2 = Monad::flat_map(io1, move |x| {
            IO::new(move || {
                executed2_clone.store(true, Ordering::SeqCst);
                x * 2
            })
        });

        // Neither should be executed yet
        assert!(!executed1.load(Ordering::SeqCst));
        assert!(!executed2.load(Ordering::SeqCst));

        // run_unsafe should execute both
        let result = io2.run_unsafe();
        assert!(executed1.load(Ordering::SeqCst));
        assert!(executed2.load(Ordering::SeqCst));
        assert_eq!(result, 20);
    }

    // =========================================================================
    // to_async Tests (requires async feature)
    // =========================================================================

    #[cfg(feature = "async")]
    #[rstest]
    fn test_to_async_executes_immediately() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let io = IO::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
            42
        });

        // IO is executed at the point of to_async call
        let _async_io = io.to_async();
        assert!(
            executed.load(Ordering::SeqCst),
            "IO should be executed immediately on to_async"
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    #[allow(deprecated)]
    async fn test_to_async_result_is_captured() {
        let io = IO::pure(42);
        let async_io = io.to_async();
        assert_eq!(async_io.run_async().await, 42);
    }
}
