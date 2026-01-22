#![allow(unsafe_code)]
//! Thread-safe lazy evaluation with memoization.
//!
//! This module provides the `ConcurrentLazy<T, F>` type for thread-safe lazy evaluation.
//! Values are computed only when needed and cached for subsequent accesses.
//! Unlike [`Lazy`](super::Lazy), this type can be safely shared between threads.
//!
//! # Safety
//!
//! This module uses unsafe code to implement a lock-free state machine.
//! The following invariants are maintained:
//! - `value` is only initialized when `state` is `STATE_READY`
//! - `initializer` is `Some` only when `state` is `STATE_EMPTY`
//! - Transition to `STATE_COMPUTING` is done via `compare_exchange` for exclusivity
//! - Multiple threads can safely access via Atomic operations and spin-wait
//!
//! # Referential Transparency Note
//!
//! While `ConcurrentLazy` provides memoization that appears functionally pure on success,
//! it is **not referentially transparent** in the presence of panics:
//!
//! - If the initialization function panics, the `ConcurrentLazy` becomes **poisoned**
//! - Once poisoned, all subsequent calls to `force()` will panic
//! - This means the behavior of `force()` depends on the history of previous calls
//!
//! This is a deliberate design decision to prevent returning potentially inconsistent
//! partial state after a panic. Users who need strict referential transparency
//! should ensure their initialization functions do not panic, or handle the
//! poisoned state explicitly via `is_poisoned()` before calling `force()`.
//!
//! # Re-entry Warning
//!
//! Calling `force()` recursively from within the initialization function on the
//! same thread will cause a deadlock detection panic. The spin-wait mechanism
//! has an iteration limit that will trigger a panic if exceeded, preventing
//! infinite spinning.
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

use std::cell::UnsafeCell;
use std::fmt;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, Ordering};

/// State: not yet initialized
const STATE_EMPTY: u8 = 0;
/// State: initialization in progress
const STATE_COMPUTING: u8 = 1;
/// State: initialization complete
const STATE_READY: u8 = 2;
/// State: initialization panicked
const STATE_POISONED: u8 = 3;

/// Error returned when a `ConcurrentLazy` value cannot be initialized.
///
/// This error is returned by [`ConcurrentLazy::into_inner`] when:
/// - The initialization function has already been consumed (e.g., due to a previous panic
///   in another call to `force()` or `into_inner()`)
/// - The lazy value is poisoned (initialization panicked)
///
/// Note: [`ConcurrentLazy::force`] panics instead of returning this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrentLazyPoisonedError;

impl fmt::Display for ConcurrentLazyPoisonedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ConcurrentLazy: initializer already consumed or poisoned"
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
/// After initialization, accessing the value is lock-free (uses `AtomicU8::load`).
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
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
    initializer: UnsafeCell<Option<F>>,
}

// # Safety
//
// Send implementation conditions: T: Send + Sync, F: Send
// - T: Send: Value can be transferred to other threads
// - T: Sync: Concurrent &T access from multiple threads is safe
// - F: Send: Closure can be transferred to other threads
//
// Sync implementation conditions: T: Send + Sync, F: Send
// - When sharing &ConcurrentLazy across threads, force() returns &T
// - T: Sync makes sharing &T safe
// - Atomic state machine ensures exactly-once initialization
// - STATE_READY reads are synchronized via Acquire/Release
unsafe impl<T: Send + Sync, F: Send> Send for ConcurrentLazy<T, F> {}
unsafe impl<T: Send + Sync, F: Send> Sync for ConcurrentLazy<T, F> {}

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
    pub const fn new(initializer: F) -> Self {
        Self {
            state: AtomicU8::new(STATE_EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            initializer: UnsafeCell::new(Some(initializer)),
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
    pub fn force(&self) -> &T {
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            match state {
                STATE_READY => {
                    // SAFETY: Transition to STATE_READY is done in do_init() after value.write()
                    // completes with Release ordering. The Acquire load here establishes
                    // happens-before relationship, guaranteeing that write is visible.
                    return unsafe { (*self.value.get()).assume_init_ref() };
                }
                STATE_POISONED => {
                    panic!("ConcurrentLazy instance has been poisoned");
                }
                STATE_EMPTY => {
                    // Use compare_exchange_weak: allows spurious failure but is faster
                    // on some architectures, and we retry in the loop anyway
                    match self.state.compare_exchange_weak(
                        STATE_EMPTY,
                        STATE_COMPUTING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            return self.do_init();
                        }
                        Err(current_state) => {
                            state = current_state;
                        }
                    }
                }
                STATE_COMPUTING => {
                    // Another thread is initializing. Spin-wait with exponential backoff
                    self.spin_wait();
                    state = self.state.load(Ordering::Acquire);
                }
                _ => unreachable!("Invalid state"),
            }
        }
    }

    /// Performs the initialization.
    ///
    /// # Safety
    ///
    /// Must only be called after successfully transitioning to `STATE_COMPUTING`.
    fn do_init(&self) -> &T {
        // SAFETY: compare_exchange succeeded, so only this thread is in
        // STATE_COMPUTING. Other threads are waiting in spin_wait().
        let initializer = unsafe { (*self.initializer.get()).take() }
            .expect("ConcurrentLazy: initializer already consumed");

        let result = catch_unwind(AssertUnwindSafe(initializer));

        // Note: Using match here for clarity. The Err case panics, so clippy's
        // suggestion to use if let or map_or_else is not appropriate.
        #[allow(clippy::single_match_else, clippy::option_if_let_else)]
        match result {
            Ok(value) => {
                // SAFETY: Only the thread that acquired STATE_COMPUTING reaches here.
                // value is uninitialized, so we initialize it with write().
                unsafe {
                    (*self.value.get()).write(value);
                }

                // Release ordering makes the write visible to threads waiting in spin_wait()
                self.state.store(STATE_READY, Ordering::Release);

                // SAFETY: Just initialized with write(). assume_init_ref() is safe.
                unsafe { (*self.value.get()).assume_init_ref() }
            }
            Err(_) => {
                self.state.store(STATE_POISONED, Ordering::Release);
                panic!("ConcurrentLazy: initialization function panicked");
            }
        }
    }

    /// Maximum number of spin iterations before assuming deadlock.
    ///
    /// This value is chosen to be high enough to allow for legitimate long initializations
    /// (even with thread scheduling delays), but low enough to detect re-entry deadlock
    /// within a reasonable time frame.
    const MAX_SPIN_ITERATIONS: u32 = 10_000;

    /// Spin-waits for `STATE_COMPUTING` to transition to another state.
    ///
    /// Uses exponential backoff to reduce CPU usage and contention.
    ///
    /// # Panics
    ///
    /// Panics if the state remains `STATE_COMPUTING` for too many iterations,
    /// which typically indicates re-entry deadlock (calling `force()` recursively
    /// during initialization on the same thread).
    fn spin_wait(&self) {
        let mut spin_count = 0u32;
        let mut total_iterations = 0u32;

        loop {
            // Exponential backoff: spin count increases up to 64 iterations
            let iterations = 1u32 << spin_count.min(6);
            for _ in 0..iterations {
                std::hint::spin_loop();
                total_iterations = total_iterations.saturating_add(1);
            }

            let state = self.state.load(Ordering::Acquire);
            if state != STATE_COMPUTING {
                return;
            }

            // Check for potential deadlock (re-entry or excessively long initialization)
            assert!(
                total_iterations < Self::MAX_SPIN_ITERATIONS,
                "ConcurrentLazy::force potential deadlock detected: \
                 initialization did not complete after {total_iterations} spin iterations. \
                 This may indicate recursive initialization (calling force() \
                 from within the initializer on the same thread)."
            );

            spin_count += 1;

            // After many spins, yield to scheduler to avoid wasting CPU
            if spin_count > 10 {
                std::thread::yield_now();
            }
        }
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
    /// - The lazy value is poisoned (initialization previously panicked)
    /// - The initialization function has already been consumed
    ///
    /// # Panics
    ///
    /// If the initialization function panics during `into_inner`, the panic is
    /// propagated after marking the instance as poisoned.
    pub fn into_inner(self) -> Result<T, ConcurrentLazyPoisonedError> {
        // Prevent Drop from running since we're manually handling the fields
        let this = ManuallyDrop::new(self);
        let state = this.state.load(Ordering::Acquire);

        match state {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                // We're consuming self (via ManuallyDrop), so we can take ownership.
                Ok(unsafe { (*this.value.get()).assume_init_read() })
            }
            STATE_POISONED => Err(ConcurrentLazyPoisonedError),
            STATE_EMPTY => {
                // SAFETY: STATE_EMPTY means initializer is Some.
                let initializer = unsafe { (*this.initializer.get()).take() }
                    .ok_or(ConcurrentLazyPoisonedError)?;

                // Catch panics to ensure we transition to poisoned state
                let result = catch_unwind(AssertUnwindSafe(initializer));

                // Note: Using match for clarity - the Err case panics so clippy's
                // suggestion to use if let or map_or_else is not appropriate
                #[allow(clippy::single_match_else, clippy::option_if_let_else)]
                match result {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        // Store poisoned state for consistency, even though we own the value
                        // This is mostly for documentation purposes since the value is consumed
                        this.state.store(STATE_POISONED, Ordering::Release);
                        panic!("ConcurrentLazy: initialization function panicked");
                    }
                }
            }
            STATE_COMPUTING => {
                // This shouldn't happen when consuming self (we have ownership),
                // but handle it gracefully
                Err(ConcurrentLazyPoisonedError)
            }
            _ => unreachable!("Invalid state"),
        }
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
        Self {
            state: AtomicU8::new(STATE_READY),
            value: UnsafeCell::new(MaybeUninit::new(value)),
            initializer: UnsafeCell::new(None),
        }
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
        if self.state.load(Ordering::Acquire) == STATE_READY {
            // SAFETY: STATE_READY means value is initialized.
            Some(unsafe { (*self.value.get()).assume_init_ref() })
        } else {
            None
        }
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
        self.state.load(Ordering::Acquire) == STATE_READY
    }

    /// Returns whether the lazy value has been poisoned.
    ///
    /// A lazy value becomes poisoned if the initialization function panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    /// use std::panic::catch_unwind;
    ///
    /// let lazy = ConcurrentLazy::new(|| panic!("initialization failed"));
    ///
    /// let _ = catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
    ///
    /// assert!(lazy.is_poisoned());
    /// ```
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_POISONED
    }
}

impl<T, F: FnOnce() -> T> ConcurrentLazy<T, F> {
    /// Maximum number of spin iterations for `try_force` before returning Err.
    ///
    /// This value is chosen to be high enough to allow for legitimate long initializations
    /// (even with thread scheduling delays), but low enough to return Err in a reasonable time
    /// instead of blocking indefinitely.
    const TRY_FORCE_MAX_SPIN_ITERATIONS: u32 = 10_000;

    /// Tries to force evaluation without panicking on poisoned state.
    ///
    /// This is a pure functional alternative to `force()` that returns a `Result`
    /// instead of panicking when the lazy value is poisoned or when another thread
    /// is currently initializing the value.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)` if the value is successfully computed or already cached
    /// - `Err(ConcurrentLazyPoisonedError)` if the lazy value is poisoned or
    ///   initialization is in progress and did not complete within the timeout
    ///
    /// # Errors
    ///
    /// Returns `Err(ConcurrentLazyPoisonedError)` if:
    /// - The lazy value is poisoned (initialization previously panicked)
    /// - Another thread is currently initializing and did not complete within the spin timeout
    ///
    /// # Panics
    ///
    /// Panics if the initialization function itself panics. The panic is not caught;
    /// only the poisoned state (from a previous panic) is handled via `Result`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    /// use std::panic::catch_unwind;
    ///
    /// // Normal usage
    /// let lazy = ConcurrentLazy::new(|| 42);
    /// assert_eq!(*lazy.try_force().unwrap(), 42);
    ///
    /// // Poisoned lazy
    /// let poisoned = ConcurrentLazy::new(|| panic!("init failed"));
    /// let _ = catch_unwind(std::panic::AssertUnwindSafe(|| poisoned.force()));
    /// assert!(poisoned.try_force().is_err());
    /// ```
    pub fn try_force(&self) -> Result<&T, ConcurrentLazyPoisonedError> {
        let mut state = self.state.load(Ordering::Acquire);
        let mut total_iterations = 0u32;
        let mut spin_count = 0u32;

        loop {
            match state {
                STATE_READY => {
                    // SAFETY: Same as force() - state is STATE_READY so value is initialized
                    return Ok(unsafe { (*self.value.get()).assume_init_ref() });
                }
                STATE_POISONED => {
                    return Err(ConcurrentLazyPoisonedError);
                }
                STATE_EMPTY => {
                    match self.state.compare_exchange_weak(
                        STATE_EMPTY,
                        STATE_COMPUTING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            return Ok(self.do_init());
                        }
                        Err(s) => {
                            state = s;
                        }
                    }
                }
                STATE_COMPUTING => {
                    // Spin-wait with exponential backoff, but return Err instead of panicking
                    if total_iterations >= Self::TRY_FORCE_MAX_SPIN_ITERATIONS {
                        return Err(ConcurrentLazyPoisonedError);
                    }

                    let iterations = 1u32 << spin_count.min(6);
                    for _ in 0..iterations {
                        std::hint::spin_loop();
                        total_iterations = total_iterations.saturating_add(1);
                    }

                    spin_count += 1;

                    // After many spins, yield to scheduler
                    if spin_count > 10 {
                        std::thread::yield_now();
                    }

                    state = self.state.load(Ordering::Acquire);
                }
                _ => unreachable!("Invalid state"),
            }
        }
    }

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

    /// Applies a function to the lazy value, returning a Result.
    ///
    /// This is a pure functional alternative to `map()` that returns `Result`
    /// instead of panicking when the lazy value is poisoned. The returned lazy
    /// value will compute to `Ok(f(value))` if successful, or `Err(ConcurrentLazyPoisonedError)`
    /// if the original lazy value is poisoned.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the computed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::ConcurrentLazy;
    ///
    /// let lazy = ConcurrentLazy::new(|| 21);
    /// let doubled = lazy.try_map(|x| x * 2);
    ///
    /// assert_eq!(*doubled.force().unwrap(), 42);
    /// ```
    pub fn try_map<U, G>(
        self,
        function: G,
    ) -> ConcurrentLazy<
        Result<U, ConcurrentLazyPoisonedError>,
        impl FnOnce() -> Result<U, ConcurrentLazyPoisonedError>,
    >
    where
        G: FnOnce(T) -> U,
    {
        ConcurrentLazy::new(move || self.into_inner().map(function))
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
            function(value)
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
        match self.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                let value = unsafe { (*self.value.get()).assume_init_ref() };
                fmt::Debug::fmt(value, formatter)
            }
            STATE_EMPTY | STATE_COMPUTING => formatter.write_str("<uninit>"),
            STATE_POISONED => formatter.write_str("<poisoned>"),
            _ => unreachable!(),
        }
    }
}

impl<T: fmt::Display, F> fmt::Display for ConcurrentLazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                let value = unsafe { (*self.value.get()).assume_init_ref() };
                fmt::Display::fmt(value, formatter)
            }
            STATE_EMPTY | STATE_COMPUTING => formatter.write_str("<uninit>"),
            STATE_POISONED => formatter.write_str("<poisoned>"),
            _ => unreachable!(),
        }
    }
}

impl<T, F> Drop for ConcurrentLazy<T, F> {
    fn drop(&mut self) {
        // Only drop the value if it was initialized
        if *self.state.get_mut() == STATE_READY {
            // SAFETY: STATE_READY means value is initialized.
            // We have &mut self, so exclusive access is guaranteed.
            unsafe {
                (*self.value.get()).assume_init_drop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::thread;

    #[rstest]
    fn test_display_unevaluated_lazy() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert_eq!(format!("{lazy}"), "<uninit>");
    }

    #[rstest]
    fn test_display_evaluated_lazy() {
        let lazy = ConcurrentLazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(format!("{lazy}"), "42");
    }

    #[rstest]
    fn test_display_poisoned_lazy() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(format!("{lazy}"), "<poisoned>");
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
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
            42
        });

        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
        let _ = lazy.force();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        let _ = lazy.force();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
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
    fn test_concurrent_lazy_zip() {
        let lazy1 = ConcurrentLazy::new(|| 1);
        let lazy2 = ConcurrentLazy::new(|| "hello");
        let combined = lazy1.zip(lazy2);
        assert_eq!(*combined.force(), (1, "hello"));
    }

    #[rstest]
    fn test_concurrent_lazy_zip_with() {
        let lazy1 = ConcurrentLazy::new(|| 20);
        let lazy2 = ConcurrentLazy::new(|| 22);
        let sum = lazy1.zip_with(lazy2, |a, b| a + b);
        assert_eq!(*sum.force(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_pure() {
        let lazy = ConcurrentLazy::pure(42);
        assert!(lazy.is_initialized());
        assert_eq!(*lazy.force(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_default() {
        let lazy: ConcurrentLazy<i32> = ConcurrentLazy::default();
        assert_eq!(*lazy.force(), 0);
    }

    #[rstest]
    fn test_concurrent_lazy_get_before_init() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert!(lazy.get().is_none());
    }

    #[rstest]
    fn test_concurrent_lazy_get_after_init() {
        let lazy = ConcurrentLazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(*lazy.get().unwrap(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_into_inner_uninit() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert_eq!(lazy.into_inner(), Ok(42));
    }

    #[rstest]
    fn test_concurrent_lazy_into_inner_init() {
        let lazy = ConcurrentLazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(lazy.into_inner(), Ok(42));
    }

    #[rstest]
    fn test_concurrent_lazy_into_inner_poisoned() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(lazy.into_inner(), Err(ConcurrentLazyPoisonedError));
    }

    #[rstest]
    fn test_concurrent_lazy_poison_propagation() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("test panic") });

        let result1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
        assert!(result1.is_err());

        let result2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
        assert!(result2.is_err());

        assert!(lazy.is_poisoned());
    }

    #[rstest]
    fn test_concurrent_lazy_debug_uninit() {
        let lazy = ConcurrentLazy::new(|| 42);
        assert_eq!(format!("{lazy:?}"), "<uninit>");
    }

    #[rstest]
    fn test_concurrent_lazy_debug_init() {
        let lazy = ConcurrentLazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(format!("{lazy:?}"), "42");
    }

    #[rstest]
    fn test_concurrent_lazy_debug_poisoned() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(format!("{lazy:?}"), "<poisoned>");
    }

    #[rstest]
    fn test_concurrent_initialization_exactly_once() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = Arc::new(ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
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

        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
    }

    // =========================================================================
    // Drop Tests
    // =========================================================================

    #[rstest]
    fn test_concurrent_lazy_drop_uninit() {
        // Should not panic when dropping uninitialized ConcurrentLazy
        let _lazy = ConcurrentLazy::new(|| 42);
    }

    #[rstest]
    fn test_concurrent_lazy_drop_init() {
        struct DropTracker {
            dropped: Arc<AtomicUsize>,
        }
        impl Drop for DropTracker {
            fn drop(&mut self) {
                self.dropped.fetch_add(1, AtomicOrdering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicUsize::new(0));
        let dropped_clone = dropped.clone();

        let lazy = ConcurrentLazy::new(move || DropTracker {
            dropped: dropped_clone,
        });
        let _ = lazy.force();
        assert_eq!(dropped.load(AtomicOrdering::SeqCst), 0);

        drop(lazy);
        assert_eq!(dropped.load(AtomicOrdering::SeqCst), 1);
    }

    // =========================================================================
    // Panic Handling Tests
    // =========================================================================

    #[rstest]
    fn test_concurrent_lazy_into_inner_panic_behavior() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("into_inner panic test") });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.into_inner()));
        assert!(result.is_err());
    }

    // Note: Testing actual re-entry deadlock detection is difficult because
    // it requires calling force() from within the initializer on the same thread.
    // The MAX_SPIN_ITERATIONS constant is set high enough to allow legitimate
    // long-running initializations but will eventually detect true deadlocks.
    //
    // A proper test would require simulating a scenario where the initializer
    // calls force() recursively, which is architecturally prevented by the
    // ownership model in most cases.

    // =========================================================================
    // Functor/Monad Law Property Tests
    // =========================================================================

    mod law_property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Single evaluation: force() always returns the same value
            #[test]
            fn prop_concurrent_lazy_memoization(x in any::<i64>()) {
                let lazy = ConcurrentLazy::new(|| x);
                let v1 = *lazy.force();
                let v2 = *lazy.force();
                prop_assert_eq!(v1, v2);
                prop_assert_eq!(v1, x);
            }

            /// Functor identity law: lazy.map(|x| x) == lazy
            #[test]
            fn prop_concurrent_lazy_functor_identity(x in any::<i64>()) {
                let lazy = ConcurrentLazy::new(|| x);
                let mapped = ConcurrentLazy::new(|| x).map(|v| v);
                prop_assert_eq!(*lazy.force(), *mapped.force());
            }

            /// Functor composition law: lazy.map(f).map(g) == lazy.map(|x| g(f(x)))
            #[test]
            fn prop_concurrent_lazy_functor_composition(x in any::<i32>()) {
                let f = |v: i32| v.wrapping_add(1);
                let g = |v: i32| v.wrapping_mul(2);
                let lazy1 = ConcurrentLazy::new(|| x).map(f).map(g);
                let lazy2 = ConcurrentLazy::new(|| x).map(|v| g(f(v)));
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad left identity: pure(a).flat_map(f) == f(a)
            #[test]
            fn prop_concurrent_lazy_monad_left_identity(x in any::<i32>()) {
                let f = |v: i32| ConcurrentLazy::new(move || v.wrapping_mul(2));
                let lazy1 = ConcurrentLazy::pure(x).flat_map(f);
                let lazy2 = f(x);
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad right identity: m.flat_map(pure) == m
            #[test]
            fn prop_concurrent_lazy_monad_right_identity(x in any::<i32>()) {
                let lazy1 = ConcurrentLazy::new(|| x);
                let lazy2 = ConcurrentLazy::new(|| x).flat_map(ConcurrentLazy::pure);
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad associativity: (m.flat_map(f)).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
            #[test]
            fn prop_concurrent_lazy_monad_associativity(x in any::<i32>()) {
                let f = |v: i32| ConcurrentLazy::new(move || v.wrapping_add(1));
                let g = |v: i32| ConcurrentLazy::new(move || v.wrapping_mul(2));
                let lazy1 = ConcurrentLazy::new(|| x).flat_map(f).flat_map(g);
                let lazy2 = ConcurrentLazy::new(|| x).flat_map(|v| f(v).flat_map(g));
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }
        }
    }
}
