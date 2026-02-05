#![allow(unsafe_code)]
//! Thread-safe lazy evaluation with memoization.
//!
//! This module provides the `ConcurrentLazy<T, F>` type for thread-safe lazy evaluation.
//! Values are computed only when needed and cached for subsequent accesses.
//! Unlike [`Lazy`](super::Lazy), this type can be safely shared between threads.
//!
//! # Safety
//!
//! This module uses unsafe code to implement a state machine with blocking wait.
//! The following invariants are maintained:
//! - `value` is only initialized when `state` is `STATE_READY`
//! - `initializer` is `Some` only when `state` is `STATE_EMPTY`
//! - Transition to `STATE_COMPUTING` is done via `compare_exchange` for exclusivity
//! - Multiple threads can safely access via atomic operations and `Condvar` blocking wait
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
//! # Performance
//!
//! This implementation uses `Mutex` + `Condvar` for blocking wait instead of spin-wait.
//! This significantly reduces CPU usage during contention compared to spin-wait approaches.
//! The READY path remains lock-free with only an atomic load and Acquire fence - the
//! `Mutex` is only acquired when initialization is in progress.
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
use std::sync::{Condvar, Mutex};

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
/// After initialization, accessing the value is lock-free (uses atomic load with Acquire ordering).
/// The `Mutex` + `Condvar` is only used when waiting for another thread to complete
/// initialization, making the common case (accessing an already-initialized value)
/// very fast.
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
    /// Atomic state for lock-free fast path.
    state: AtomicU8,
    /// The cached value (initialized when state is `STATE_READY`).
    value: UnsafeCell<MaybeUninit<T>>,
    /// The initialization function (consumed when initialization starts).
    initializer: UnsafeCell<Option<F>>,
    /// Condvar for blocking wait during initialization.
    /// Only used when another thread is computing - the fast path doesn't touch this.
    wait_condvar: Condvar,
    /// Mutex paired with the Condvar. Used to ensure proper synchronization
    /// between state updates and condvar notifications.
    wait_mutex: Mutex<()>,
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
    #[allow(clippy::missing_const_for_fn)] // Condvar::new and Mutex::new are not const
    pub fn new(initializer: F) -> Self {
        Self {
            state: AtomicU8::new(STATE_EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            initializer: UnsafeCell::new(Some(initializer)),
            wait_condvar: Condvar::new(),
            wait_mutex: Mutex::new(()),
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
    /// block (using `Condvar::wait`) until it completes.
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
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn force(&self) -> &T {
        let state = self.state.load(Ordering::Acquire);

        if state == STATE_READY {
            // SAFETY: Acquire load synchronizes with Release store in do_init().
            return unsafe { (*self.value.get()).assume_init_ref() };
        }

        self.force_slow(state)
    }

    #[cold]
    #[inline(never)]
    fn force_slow(&self, state: u8) -> &T {
        self.force_slow_inner(state)
            .unwrap_or_else(|_| panic!("ConcurrentLazy instance has been poisoned"))
    }

    /// Uses `Condvar::wait` for efficient blocking without CPU spinning.
    #[cold]
    fn block_until_ready(&self) {
        let mut guard = self.wait_mutex.lock().unwrap_or_else(|e| e.into_inner());

        while self.state.load(Ordering::Acquire) == STATE_COMPUTING {
            guard = self
                .wait_condvar
                .wait(guard)
                .unwrap_or_else(|e| e.into_inner());
        }

        drop(guard);
    }

    /// Uses a drop guard to ensure state transitions and waiters are notified if panic occurs.
    fn do_init(&self) -> &T {
        struct InitGuard<'a> {
            state: &'a AtomicU8,
            condvar: &'a Condvar,
            mutex: &'a Mutex<()>,
        }

        impl Drop for InitGuard<'_> {
            fn drop(&mut self) {
                if self.state.load(Ordering::Relaxed) == STATE_COMPUTING {
                    // Acquire mutex to ensure proper synchronization with waiters
                    let _lock = self.mutex.lock().unwrap_or_else(|e| e.into_inner());
                    self.state.store(STATE_POISONED, Ordering::Release);
                    self.condvar.notify_all();
                }
            }
        }

        let guard = InitGuard {
            state: &self.state,
            condvar: &self.wait_condvar,
            mutex: &self.wait_mutex,
        };

        // SAFETY: compare_exchange succeeded, so only this thread is in STATE_COMPUTING.
        let initializer = unsafe { (*self.initializer.get()).take() }
            .expect("ConcurrentLazy: initializer already consumed");

        let value = initializer();

        // SAFETY: Only the thread that acquired STATE_COMPUTING reaches here.
        unsafe { (*self.value.get()).write(value) };

        // Acquire mutex to ensure proper synchronization with waiters before notifying
        {
            let _lock = self.wait_mutex.lock().unwrap_or_else(|e| e.into_inner());
            self.state.store(STATE_READY, Ordering::Release);
            self.wait_condvar.notify_all();
        }
        std::mem::forget(guard); // Disarm the guard

        // SAFETY: Just initialized above.
        unsafe { (*self.value.get()).assume_init_ref() }
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
        let this = ManuallyDrop::new(self);

        match this.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                Ok(unsafe { (*this.value.get()).assume_init_read() })
            }
            STATE_POISONED | STATE_COMPUTING => Err(ConcurrentLazyPoisonedError),
            STATE_EMPTY => {
                // SAFETY: STATE_EMPTY means initializer is Some.
                let initializer = unsafe { (*this.initializer.get()).take() }
                    .ok_or(ConcurrentLazyPoisonedError)?;

                catch_unwind(AssertUnwindSafe(initializer)).map_err(|_| {
                    this.state.store(STATE_POISONED, Ordering::Release);
                    panic!("ConcurrentLazy: initialization function panicked");
                })
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
            wait_condvar: Condvar::new(),
            wait_mutex: Mutex::new(()),
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
        (self.state.load(Ordering::Acquire) == STATE_READY)
            // SAFETY: STATE_READY means value is initialized.
            .then(|| unsafe { (*self.value.get()).assume_init_ref() })
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
    /// Tries to force evaluation without panicking on poisoned state.
    ///
    /// This is a pure functional alternative to `force()` that returns a `Result`
    /// instead of panicking when the lazy value is poisoned.
    ///
    /// Unlike `force()`, this method uses blocking wait to wait for initialization
    /// to complete, which is efficient and does not waste CPU cycles.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)` if the value is successfully computed or already cached
    /// - `Err(ConcurrentLazyPoisonedError)` if the lazy value is poisoned
    ///
    /// # Errors
    ///
    /// Returns `Err(ConcurrentLazyPoisonedError)` if:
    /// - The lazy value is poisoned (initialization previously panicked)
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
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_force(&self) -> Result<&T, ConcurrentLazyPoisonedError> {
        let state = self.state.load(Ordering::Acquire);

        if state == STATE_READY {
            // SAFETY: STATE_READY means value is initialized.
            return Ok(unsafe { (*self.value.get()).assume_init_ref() });
        }

        self.try_force_slow(state)
    }

    #[cold]
    #[inline(never)]
    fn try_force_slow(&self, state: u8) -> Result<&T, ConcurrentLazyPoisonedError> {
        self.force_slow_inner(state)
    }

    /// Core slow-path logic shared by `force_slow` and `try_force_slow`.
    #[cold]
    #[inline(never)]
    fn force_slow_inner(&self, mut state: u8) -> Result<&T, ConcurrentLazyPoisonedError> {
        loop {
            match state {
                STATE_READY => {
                    // SAFETY: Acquire load synchronizes with Release store in do_init().
                    return Ok(unsafe { (*self.value.get()).assume_init_ref() });
                }
                STATE_POISONED => return Err(ConcurrentLazyPoisonedError),
                STATE_EMPTY => {
                    match self.state.compare_exchange_weak(
                        STATE_EMPTY,
                        STATE_COMPUTING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => return Ok(self.do_init()),
                        Err(current_state) => state = current_state,
                    }
                }
                STATE_COMPUTING => {
                    self.block_until_ready();
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
    /// assert_eq!(doubled.force().unwrap(), 42);
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
        self.format_state(formatter, |value, f| fmt::Debug::fmt(value, f))
    }
}

impl<T: fmt::Display, F> fmt::Display for ConcurrentLazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_state(formatter, |value, f| fmt::Display::fmt(value, f))
    }
}

impl<T, F> ConcurrentLazy<T, F> {
    fn format_state<Fmt>(
        &self,
        formatter: &mut fmt::Formatter<'_>,
        format_value: Fmt,
    ) -> fmt::Result
    where
        Fmt: FnOnce(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
    {
        match self.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                let value = unsafe { (*self.value.get()).assume_init_ref() };
                format_value(value, formatter)
            }
            STATE_EMPTY | STATE_COMPUTING => formatter.write_str("<uninit>"),
            STATE_POISONED => formatter.write_str("<poisoned>"),
            _ => unreachable!(),
        }
    }
}

impl<T, F> Drop for ConcurrentLazy<T, F> {
    fn drop(&mut self) {
        if *self.state.get_mut() == STATE_READY {
            // SAFETY: STATE_READY means value is initialized.
            unsafe { (*self.value.get()).assume_init_drop() };
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
    // The current implementation uses Mutex + Condvar for blocking wait, which
    // means a recursive call from the same thread would block indefinitely.
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
