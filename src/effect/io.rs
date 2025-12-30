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
//! use functional_rusty::effect::IO;
//!
//! // Create a pure IO action
//! let io = IO::pure(42);
//! assert_eq!(io.run_unsafe(), 42);
//!
//! // Chain IO actions
//! let io = IO::pure(10)
//!     .fmap(|x| x * 2)
//!     .flat_map(|x| IO::pure(x + 1));
//! assert_eq!(io.run_unsafe(), 21);
//! ```
//!
//! # Side Effect Deferral
//!
//! ```rust
//! use functional_rusty::effect::IO;
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
    /// use functional_rusty::effect::IO;
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
    /// use functional_rusty::effect::IO;
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
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::pure(42);
    /// let result = io.run_unsafe();
    /// assert_eq!(result, 42);
    /// ```
    pub fn run_unsafe(self) -> A {
        (self.run_io)()
    }

    /// Transforms the result of an IO action using a function.
    ///
    /// This is the `fmap` operation from Functor.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::pure(21).fmap(|x| x * 2);
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> IO<B>
    where
        F: FnOnce(A) -> B + 'static,
        B: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            function(a)
        })
    }

    /// Chains IO actions, passing the result of the first to a function
    /// that produces the second.
    ///
    /// This is the `bind` operation from Monad.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and returns a new IO action.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::pure(10).flat_map(|x| IO::pure(x * 2));
    /// assert_eq!(io.run_unsafe(), 20);
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> IO<B>
    where
        F: FnOnce(A) -> IO<B> + 'static,
        B: 'static,
    {
        IO::new(move || {
            let a = self.run_unsafe();
            let io_b = function(a);
            io_b.run_unsafe()
        })
    }

    /// Alias for `flat_map`.
    ///
    /// This is the conventional Rust name for monadic bind.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::pure(10).and_then(|x| IO::pure(x + 5));
    /// assert_eq!(io.run_unsafe(), 15);
    /// ```
    pub fn and_then<B, F>(self, function: F) -> IO<B>
    where
        F: FnOnce(A) -> IO<B> + 'static,
        B: 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two IO actions, discarding the result of the first.
    ///
    /// The first action is still executed for its side effects.
    ///
    /// # Arguments
    ///
    /// * `next` - The IO action to execute after this one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::pure(10).then(IO::pure(20));
    /// assert_eq!(io.run_unsafe(), 20);
    /// ```
    pub fn then<B>(self, next: IO<B>) -> IO<B>
    where
        B: 'static,
    {
        self.flat_map(move |_| next)
    }

    /// Combines two IO actions using a function.
    ///
    /// # Arguments
    ///
    /// * `other` - The second IO action.
    /// * `function` - A function to combine the results.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io1 = IO::pure(10);
    /// let io2 = IO::pure(20);
    /// let io = io1.map2(io2, |a, b| a + b);
    /// assert_eq!(io.run_unsafe(), 30);
    /// ```
    pub fn map2<B, C, F>(self, other: IO<B>, function: F) -> IO<C>
    where
        F: FnOnce(A, B) -> C + 'static,
        B: 'static,
        C: 'static,
    {
        self.flat_map(move |a| other.fmap(move |b| function(a, b)))
    }

    /// Combines two IO actions into a tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - The second IO action.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let io1 = IO::pure(10);
    /// let io2 = IO::pure("hello".to_string());
    /// let io = io1.product(io2);
    /// assert_eq!(io.run_unsafe(), (10, "hello".to_string()));
    /// ```
    pub fn product<B>(self, other: IO<B>) -> IO<(A, B)>
    where
        B: 'static,
    {
        self.map2(other, |a, b| (a, b))
    }
}

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
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::print_line("Hello, World!");
    /// io.run_unsafe(); // Prints "Hello, World!"
    /// ```
    pub fn print_line<S: std::fmt::Display + 'static>(message: S) -> Self {
        IO::new(move || {
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
    /// use functional_rusty::effect::IO;
    /// use std::time::Duration;
    ///
    /// let io = IO::delay(Duration::from_millis(100));
    /// io.run_unsafe(); // Waits for 100ms
    /// ```
    pub fn delay(duration: Duration) -> Self {
        IO::new(move || {
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
    /// use functional_rusty::effect::IO;
    ///
    /// let io = IO::read_line();
    /// let line = io.run_unsafe().expect("Failed to read line");
    /// println!("You entered: {}", line);
    /// ```
    pub fn read_line() -> Self {
        IO::new(|| {
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
    /// use functional_rusty::effect::IO;
    ///
    /// let panicking = IO::new(|| panic!("oops"));
    /// let recovered = IO::catch(panicking, |_| "recovered".to_string());
    /// assert_eq!(recovered.run_unsafe(), "recovered");
    /// ```
    ///
    /// ```rust
    /// use functional_rusty::effect::IO;
    ///
    /// let successful = IO::pure(42);
    /// let with_catch = IO::catch(successful, |_| 0);
    /// assert_eq!(with_catch.run_unsafe(), 42);
    /// ```
    pub fn catch<F>(io: IO<A>, handler: F) -> IO<A>
    where
        F: FnOnce(String) -> A + 'static,
    {
        IO::new(move || {
            let result = catch_unwind(AssertUnwindSafe(|| io.run_unsafe()));
            match result {
                Ok(value) => value,
                Err(panic_info) => {
                    let message = if let Some(string) = panic_info.downcast_ref::<&str>() {
                        (*string).to_string()
                    } else if let Some(string) = panic_info.downcast_ref::<String>() {
                        string.clone()
                    } else {
                        "Unknown panic".to_string()
                    };
                    handler(message)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_pure_and_run() {
        let io = IO::pure(42);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[test]
    fn test_io_new_and_run() {
        let io = IO::new(|| 10 + 20);
        assert_eq!(io.run_unsafe(), 30);
    }

    #[test]
    fn test_io_fmap() {
        let io = IO::pure(21).fmap(|x| x * 2);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[test]
    fn test_io_flat_map() {
        let io = IO::pure(10).flat_map(|x| IO::pure(x * 2));
        assert_eq!(io.run_unsafe(), 20);
    }

    #[test]
    fn test_io_and_then() {
        let io = IO::pure(10).and_then(|x| IO::pure(x + 5));
        assert_eq!(io.run_unsafe(), 15);
    }

    #[test]
    fn test_io_then() {
        let io = IO::pure(10).then(IO::pure(20));
        assert_eq!(io.run_unsafe(), 20);
    }

    #[test]
    fn test_io_map2() {
        let io = IO::pure(10).map2(IO::pure(20), |a, b| a + b);
        assert_eq!(io.run_unsafe(), 30);
    }

    #[test]
    fn test_io_product() {
        let io = IO::pure(10).product(IO::pure(20));
        assert_eq!(io.run_unsafe(), (10, 20));
    }
}
