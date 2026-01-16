//! Thread-safe lazy evaluation with memoization.
//!
//! This module provides the `ConcurrentLazy<T, F>` type for thread-safe lazy evaluation.
//! Values are computed only when needed and cached for subsequent accesses.
//! Unlike [`Lazy`](super::Lazy), this type can be safely shared between threads.
//!
//! # Examples
//!
//! ```rust
//! use lambars::control::ConcurrentLazy;
//! use std::sync::Arc;
//! use std::thread;
//!
//! let lazy = Arc::new(ConcurrentLazy::new(|| {
//!     println!("Computing...");
//!     42
//! }));
//!
//! // Spawn multiple threads that access the lazy value
//! let handles: Vec<_> = (0..10).map(|_| {
//!     let lazy = Arc::clone(&lazy);
//!     thread::spawn(move || *lazy.force())
//! }).collect();
//!
//! // All threads get the same value, and initialization happens only once
//! for handle in handles {
//!     assert_eq!(handle.join().unwrap(), 42);
//! }
//! ```

use std::fmt;
use std::sync::{Mutex, OnceLock};

/// Error returned when a `ConcurrentLazy` value cannot be initialized.
///
/// This error is returned by [`ConcurrentLazy::into_inner`] when:
/// - The initialization function has already been consumed (e.g., due to a previous panic
///   in another call to `force()` or `into_inner()`)
/// - The internal Mutex is poisoned (rare; only happens if a panic occurs while
///   holding the Mutex lock, which is a very short window)
///
/// Note: [`ConcurrentLazy::force`] panics instead of returning this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrentLazyPoisonedError;

impl fmt::Display for ConcurrentLazyPoisonedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ConcurrentLazy: initializer already consumed or mutex poisoned"
        )
    }
}

impl std::error::Error for ConcurrentLazyPoisonedError {}

/// A thread-safe lazily evaluated value with memoization.
///
/// `ConcurrentLazy<T, F>` defers computation until the value is first accessed via `force()`.
/// Once computed, the value is cached and subsequent calls to `force()` return
/// the cached value without recomputation.
///
/// This type is safe to share between threads. Multiple threads can call `force()`
/// concurrently, but the initialization function will only be executed once.
///
/// # Type Parameters
///
/// * `T` - The type of the computed value
/// * `F` - The type of the initialization function (defaults to `fn() -> T`)
///
/// # Thread Safety
///
/// This type implements `Send` and `Sync` when:
/// - `T: Send + Sync`
/// - `F: Send`
///
/// After initialization, accessing the value is lock-free (uses `OnceLock::get`).
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use lambars::control::ConcurrentLazy;
///
/// let lazy = ConcurrentLazy::new(|| expensive_computation());
///
/// // Computation happens here
/// let value = lazy.force();
///
/// fn expensive_computation() -> i32 {
///     // Simulating expensive work
///     42
/// }
/// ```
///
/// ## Concurrent Access
///
/// ```rust
/// use lambars::control::ConcurrentLazy;
/// use std::sync::Arc;
/// use std::thread;
///
/// let lazy = Arc::new(ConcurrentLazy::new(|| 42));
///
/// let handles: Vec<_> = (0..10).map(|_| {
///     let lazy = Arc::clone(&lazy);
///     thread::spawn(move || *lazy.force())
/// }).collect();
///
/// for handle in handles {
///     assert_eq!(handle.join().unwrap(), 42);
/// }
/// ```
pub struct ConcurrentLazy<T, F = fn() -> T> {
    value: OnceLock<T>,
    initializer: Mutex<Option<F>>,
}

impl<T, F: FnOnce() -> T> ConcurrentLazy<T, F> {
    /// Creates a new thread-safe lazy value with the given initialization function.
    ///
    /// The function will not be called until `force()` is invoked.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| {
    ///     println!("Initializing...");
    ///     42
    /// });
    /// // Nothing printed yet
    /// ```
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(initializer: F) -> Self {
        Self {
            value: OnceLock::new(),
            initializer: Mutex::new(Some(initializer)),
        }
    }

    /// Forces evaluation of the lazy value and returns a reference to it.
    ///
    /// If the value has not been computed yet, the initialization function
    /// is called and the result is cached. Subsequent calls return the
    /// cached value.
    ///
    /// This method is thread-safe. If multiple threads call `force()` concurrently,
    /// only one will execute the initialization function, and all others will
    /// wait for it to complete before returning the value.
    ///
    /// # Panics
    ///
    /// - If the initialization function has already been consumed (e.g., after
    ///   a previous panic during initialization)
    /// - If the initialization function panics
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 42);
    /// let value = lazy.force();
    /// assert_eq!(*value, 42);
    /// ```
    #[allow(clippy::significant_drop_tightening)]
    pub fn force(&self) -> &T {
        self.value.get_or_init(|| {
            let mut guard = self
                .initializer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let initializer = guard
                .take()
                .expect("ConcurrentLazy: initializer already consumed");
            drop(guard);
            initializer()
        })
    }

    /// Consumes the `ConcurrentLazy` and returns the inner value.
    ///
    /// If the value has been initialized, returns `Ok(value)`.
    /// If it has not been initialized, forces evaluation and returns `Ok(value)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 42);
    /// assert_eq!(lazy.into_inner(), Ok(42));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err(ConcurrentLazyPoisonedError)` if:
    /// - The internal Mutex is poisoned
    /// - The initialization function has already been consumed
    #[allow(clippy::significant_drop_tightening)]
    pub fn into_inner(self) -> Result<T, ConcurrentLazyPoisonedError> {
        if let Some(value) = self.value.into_inner() {
            return Ok(value);
        }

        let mut guard = self
            .initializer
            .lock()
            .map_err(|_| ConcurrentLazyPoisonedError)?;

        let initializer = guard.take().ok_or(ConcurrentLazyPoisonedError)?;
        drop(guard);
        Ok(initializer())
    }
}

impl<T> ConcurrentLazy<T, fn() -> T> {
    /// Creates a new lazy value that is already initialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new_with_value(42);
    /// assert!(lazy.is_initialized());
    /// ```
    #[inline]
    pub fn new_with_value(value: T) -> Self {
        let lazy = Self {
            value: OnceLock::new(),
            initializer: Mutex::new(None),
        };
        let _ = lazy.value.set(value);
        lazy
    }

    /// Creates a pure lazy value (Applicative pure).
    ///
    /// This is equivalent to `new_with_value` and lifts a value into
    /// the `ConcurrentLazy` context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::pure(42);
    /// assert_eq!(*lazy.force(), 42);
    /// ```
    #[inline]
    pub fn pure(value: T) -> Self {
        Self::new_with_value(value)
    }
}

impl<T, F> ConcurrentLazy<T, F> {
    /// Returns a reference to the value if it has been initialized.
    ///
    /// Unlike `force()`, this method does not trigger initialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 42);
    ///
    /// assert!(lazy.get().is_none());
    ///
    /// let _ = lazy.force();
    /// assert!(lazy.get().is_some());
    /// ```
    #[inline]
    pub fn get(&self) -> Option<&T> {
        self.value.get()
    }

    /// Returns whether the value has been initialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 42);
    /// assert!(!lazy.is_initialized());
    ///
    /// let _ = lazy.force();
    /// assert!(lazy.is_initialized());
    /// ```
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.value.get().is_some()
    }
}

impl<T, F: FnOnce() -> T> ConcurrentLazy<T, F> {
    /// Applies a function to the lazy value, producing a new lazy value.
    ///
    /// The resulting lazy value will compute the original value and then
    /// apply the function when forced.
    ///
    /// # Panics
    ///
    /// Panics when forced if the `ConcurrentLazy` instance has been poisoned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 21);
    /// let doubled = lazy.map(|x| x * 2);
    ///
    /// assert_eq!(*doubled.force(), 42);
    /// ```
    pub fn map<U, G>(self, function: G) -> ConcurrentLazy<U, impl FnOnce() -> U>
    where
        G: FnOnce(T) -> U,
    {
        ConcurrentLazy::new(move || {
            let value = self
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            function(value)
        })
    }

    /// Applies a function that returns a `ConcurrentLazy`, then flattens the result.
    ///
    /// This is the monadic bind operation for `ConcurrentLazy`.
    ///
    /// # Panics
    ///
    /// Panics when forced if the `ConcurrentLazy` instance has been poisoned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 21);
    /// let result = lazy.flat_map(|x| ConcurrentLazy::new(move || x * 2));
    ///
    /// assert_eq!(*result.force(), 42);
    /// ```
    pub fn flat_map<U, ResultFunction, G>(
        self,
        function: G,
    ) -> ConcurrentLazy<U, impl FnOnce() -> U>
    where
        ResultFunction: FnOnce() -> U,
        G: FnOnce(T) -> ConcurrentLazy<U, ResultFunction>,
    {
        ConcurrentLazy::new(move || {
            let value = self
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            let lazy_result = function(value);
            lazy_result
                .into_inner()
                .expect("ConcurrentLazy: initialization failed")
        })
    }

    /// Combines two lazy values into a lazy tuple.
    ///
    /// # Panics
    ///
    /// Panics when forced if either `ConcurrentLazy` instance has been poisoned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy1 = ConcurrentLazy::new(|| 1);
    /// let lazy2 = ConcurrentLazy::new(|| "hello");
    /// let combined = lazy1.zip(lazy2);
    ///
    /// assert_eq!(*combined.force(), (1, "hello"));
    /// ```
    pub fn zip<U, OtherFunction>(
        self,
        other: ConcurrentLazy<U, OtherFunction>,
    ) -> ConcurrentLazy<(T, U), impl FnOnce() -> (T, U)>
    where
        OtherFunction: FnOnce() -> U,
    {
        ConcurrentLazy::new(move || {
            let value1 = self
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            let value2 = other
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            (value1, value2)
        })
    }

    /// Combines two lazy values using a function.
    ///
    /// # Panics
    ///
    /// Panics when forced if either `ConcurrentLazy` instance has been poisoned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy1 = ConcurrentLazy::new(|| 20);
    /// let lazy2 = ConcurrentLazy::new(|| 22);
    /// let sum = lazy1.zip_with(lazy2, |a, b| a + b);
    ///
    /// assert_eq!(*sum.force(), 42);
    /// ```
    pub fn zip_with<U, V, OtherFunction, CombineFunction>(
        self,
        other: ConcurrentLazy<U, OtherFunction>,
        function: CombineFunction,
    ) -> ConcurrentLazy<V, impl FnOnce() -> V>
    where
        OtherFunction: FnOnce() -> U,
        CombineFunction: FnOnce(T, U) -> V,
    {
        ConcurrentLazy::new(move || {
            let value1 = self
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            let value2 = other
                .into_inner()
                .expect("ConcurrentLazy: initialization failed");
            function(value1, value2)
        })
    }
}

impl<T: Default> Default for ConcurrentLazy<T> {
    /// Creates a lazy value that computes the default value of `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy: ConcurrentLazy<i32> = ConcurrentLazy::default();
    /// assert_eq!(*lazy.force(), 0);
    /// ```
    fn default() -> Self {
        Self::new(T::default)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for ConcurrentLazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value.get() {
            Some(value) => formatter
                .debug_tuple("ConcurrentLazy")
                .field(value)
                .finish(),
            None => formatter
                .debug_tuple("ConcurrentLazy")
                .field(&"<uninit>")
                .finish(),
        }
    }
}

impl<T: fmt::Display, F> fmt::Display for ConcurrentLazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value.get() {
            Some(value) => write!(formatter, "ConcurrentLazy({value})"),
            None => write!(formatter, "ConcurrentLazy(<uninit>)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[rstest]
    fn test_display_unevaluated_lazy() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert_eq!(format!("{lazy}"), "ConcurrentLazy(<uninit>)");
    }

    #[rstest]
    fn test_display_evaluated_lazy() {
        let lazy = ConcurrentLazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(format!("{lazy}"), "ConcurrentLazy(42)");
    }

    #[rstest]
    fn test_concurrent_lazy_basic_creation() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert!(!lazy.is_initialized());
    }

    #[rstest]
    fn test_concurrent_lazy_force_computes_value() {
        let lazy = ConcurrentLazy::new(|| 42);
        let value = lazy.force();
        assert_eq!(*value, 42);
        assert!(lazy.is_initialized());
    }

    #[rstest]
    fn test_concurrent_lazy_memoization() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            42
        });

        assert_eq!(counter.load(Ordering::SeqCst), 0);
        let _ = lazy.force();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        let _ = lazy.force();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    fn test_concurrent_lazy_new_with_value() {
        let lazy = ConcurrentLazy::new_with_value(42);
        assert!(lazy.is_initialized());
        assert_eq!(*lazy.force(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_map() {
        let lazy = ConcurrentLazy::new(|| 21);
        let doubled = lazy.map(|x| x * 2);
        assert_eq!(*doubled.force(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_flat_map() {
        let lazy = ConcurrentLazy::new(|| 21);
        let result = lazy.flat_map(|x| ConcurrentLazy::new(move || x * 2));
        assert_eq!(*result.force(), 42);
    }

    #[rstest]
    fn test_concurrent_initialization_exactly_once() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = Arc::new(ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            42
        }));

        let handles: Vec<_> = (0..100)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || *lazy.force())
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
