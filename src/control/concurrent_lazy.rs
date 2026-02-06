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
//! - Multiple threads can safely access via Atomic operations and adaptive
//!   spin + `parking_lot::Condvar` blocking wait
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
//! same thread will cause an immediate panic via thread-local re-entry detection.
//! This prevents deadlock by detecting the recursive call at the `do_init()` entry
//! point before any blocking wait occurs.
//!
//! # Performance Characteristics
//!
//! - **`STATE_READY` fast path**: Lock-free `Acquire` load only (zero overhead after init)
//! - **Waiting strategy**: Adaptive spin (128 iterations) followed by
//!   `parking_lot::Condvar` blocking for longer initializations
//! - **Cache line separation**: `state` (hot path) and `wait_sync` (cold path) are
//!   placed on separate cache lines via `#[repr(C)]` + `#[repr(C, align(64))]`
//!   to prevent false sharing in high-thread-count scenarios
//!
//! # Initialization Function Constraints
//!
//! The initialization function passed to `ConcurrentLazy::new` should complete in a
//! bounded amount of time. While `force()` and `try_force()` will wait indefinitely
//! for initialization to complete, a long-running or non-terminating initialization
//! function will block all threads that call `force()` or `try_force()`.
//!
//! **Recommendations:**
//! - Keep initialization functions short and side-effect-free
//! - For long-running computations, prepare the result on a dedicated thread and
//!   pass the pre-computed value via `ConcurrentLazy::new_with_value`
//! - Do not pass untrusted or user-supplied closures as initialization functions,
//!   as a malicious closure could cause denial-of-service by blocking indefinitely
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

use std::cell::{Cell, UnsafeCell};
use std::fmt;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::{Condvar, Mutex};

/// State: not yet initialized
const STATE_EMPTY: u8 = 0;
/// State: initialization in progress
const STATE_COMPUTING: u8 = 1;
/// State: initialization complete
const STATE_READY: u8 = 2;
/// State: initialization panicked
const STATE_POISONED: u8 = 3;

/// Assumed cache line size for the target architecture (`x86_64`: 64 bytes).
///
/// On ARM servers with 128-byte cache lines, 64-byte alignment still provides
/// partial benefit. Adjust this value if targeting different architectures.
#[allow(dead_code)]
const CACHE_LINE_SIZE: usize = 64;

/// Cache-line aligned wrapper to prevent false sharing between hot and cold fields.
#[repr(C, align(64))]
struct CacheAligned<T>(T);

/// Condvar + Mutex pair for blocking wait during initialization (cold path).
struct WaitSync {
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl WaitSync {
    const fn new() -> Self {
        Self {
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }
}

// Thread-local flag to detect re-entrant initialization and prevent deadlock.
thread_local! {
    static IN_CONCURRENT_LAZY_INIT: Cell<bool> = const { Cell::new(false) };
}

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
///
/// # Memory Layout
///
/// The `#[repr(C)]` layout ensures deterministic field ordering. The `wait_sync`
/// field is wrapped in `CacheAligned` to force it onto a separate cache line
/// from the hot `state` field, preventing false sharing in high-thread-count
/// scenarios.
#[repr(C)]
pub struct ConcurrentLazy<T, F = fn() -> T> {
    // Cache line 1: hot path (frequently accessed by all threads)
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
    initializer: UnsafeCell<Option<F>>,
    // Cache line 2: cold path (accessed only during initialization wait)
    wait_sync: CacheAligned<WaitSync>,
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
            wait_sync: CacheAligned(WaitSync::new()),
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
    /// wait via an adaptive strategy (spin + Condvar) before returning the value.
    ///
    /// # Waiting Mechanism
    ///
    /// 1. **Fast path**: If `STATE_READY`, returns immediately (lock-free `Acquire` load)
    /// 2. **Adaptive spin**: Spins for up to 128 iterations in user-space
    /// 3. **Condvar wait**: Falls back to `parking_lot::Condvar::wait` for
    ///    indefinite blocking until initialization completes
    ///
    /// # Panics
    ///
    /// - If the initialization function has already been consumed (e.g., after
    ///   a previous panic during initialization)
    /// - If the initialization function panics
    /// - If re-entrant initialization is detected (calling `force()` from within
    ///   the initializer on the same thread)
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
            // SAFETY: Transition to STATE_READY is done in do_init() after value.write()
            // completes with Release ordering. The Acquire load here establishes
            // happens-before relationship, guaranteeing that write is visible.
            return unsafe { (*self.value.get()).assume_init_ref() };
        }
        self.force_slow(state)
    }

    /// Slow path for `force()`: handles `STATE_EMPTY`, `STATE_COMPUTING`, and `STATE_POISONED`.
    ///
    /// This method is intentionally `#[inline(never)]` to keep the fast path (`force()`)
    /// small enough for the compiler to inline at every call site. The fast path is just
    /// an `Acquire` load + branch, which is essentially zero-cost after initialization.
    #[inline(never)]
    fn force_slow(&self, mut state: u8) -> &T {
        loop {
            match state {
                STATE_READY => {
                    // SAFETY: Same as force() fast path.
                    return unsafe { (*self.value.get()).assume_init_ref() };
                }
                STATE_POISONED => {
                    panic!("ConcurrentLazy instance has been poisoned");
                }
                STATE_EMPTY => {
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
                    self.wait_on_initialization();
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
    ///
    /// # Re-entry Detection
    ///
    /// Uses `IN_CONCURRENT_LAZY_INIT` thread-local flag to detect re-entrant calls.
    /// If `force()` is called from within the initializer on the same thread,
    /// this method panics immediately instead of blocking forever on the Condvar.
    fn do_init(&self) -> &T {
        IN_CONCURRENT_LAZY_INIT.with(|flag| {
            assert!(
                !flag.replace(true),
                "ConcurrentLazy::force re-entrant initialization detected: \
                 force() was called from within the initializer on the same thread. \
                 This would cause a deadlock."
            );
        });

        // SAFETY: compare_exchange succeeded, so only this thread is in STATE_COMPUTING.
        let initializer = unsafe { (*self.initializer.get()).take() }
            .expect("ConcurrentLazy: initializer already consumed");

        let result = catch_unwind(AssertUnwindSafe(initializer));

        let succeeded = result.is_ok_and(|value| {
            // SAFETY: Only the thread that acquired STATE_COMPUTING reaches here.
            unsafe { (*self.value.get()).write(value) };
            true
        });

        // Common cleanup: publish state, wake waiters, clear re-entry flag
        self.state.store(
            if succeeded {
                STATE_READY
            } else {
                STATE_POISONED
            },
            Ordering::Release,
        );
        self.wait_sync.0.condvar.notify_all();
        IN_CONCURRENT_LAZY_INIT.with(|flag| flag.set(false));

        assert!(
            succeeded,
            "ConcurrentLazy: initialization function panicked"
        );

        // SAFETY: value was written above when succeeded is true.
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    /// Number of spin iterations before yielding to the OS scheduler.
    /// After this threshold, `thread::yield_now()` is called to reduce
    /// contention under high thread counts.
    const SPIN_BEFORE_YIELD: u32 = 16;

    /// Total number of spin iterations (including yield phase) before
    /// falling back to `parking_lot::Condvar` blocking wait.
    const ADAPTIVE_SPIN_LIMIT: u32 = 64;

    /// Spins then blocks via Condvar until `state` leaves `STATE_COMPUTING`.
    ///
    /// Uses a three-phase adaptive strategy:
    /// 1. **Pure spin** (0..`SPIN_BEFORE_YIELD`): `spin_loop()` hint only
    /// 2. **Yield spin** (`SPIN_BEFORE_YIELD`..`ADAPTIVE_SPIN_LIMIT`):
    ///    `spin_loop()` + `thread::yield_now()` to reduce contention
    /// 3. **Condvar wait**: Falls back to `parking_lot::Condvar::wait` for
    ///    indefinite blocking until state transitions away from `STATE_COMPUTING`
    ///
    /// Re-entrant initialization (which would cause deadlock) is already detected
    /// by the `IN_CONCURRENT_LAZY_INIT` thread-local flag in `do_init()`, so no
    /// timeout-based deadlock detection is needed here.
    fn spin_then_wait(&self) {
        for iteration in 0..Self::ADAPTIVE_SPIN_LIMIT {
            if self.state.load(Ordering::Acquire) != STATE_COMPUTING {
                return;
            }
            std::hint::spin_loop();
            if iteration >= Self::SPIN_BEFORE_YIELD {
                std::thread::yield_now();
            }
        }

        let mut guard = self.wait_sync.0.mutex.lock();
        while self.state.load(Ordering::Acquire) == STATE_COMPUTING {
            self.wait_sync.0.condvar.wait(&mut guard);
        }
    }

    /// Waits for `STATE_COMPUTING` to transition to another state.
    ///
    /// This method blocks indefinitely until initialization completes or the
    /// `ConcurrentLazy` becomes poisoned. Re-entrant initialization is detected
    /// by `do_init()` via a thread-local flag before any blocking wait occurs.
    fn wait_on_initialization(&self) {
        self.spin_then_wait();
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
        let mut this = ManuallyDrop::new(self);
        let state = this.state.load(Ordering::Acquire);

        match state {
            STATE_READY => {
                // SAFETY: STATE_READY guarantees value is initialized.
                let value = unsafe { (*this.value.get()).assume_init_read() };
                // Set state to STATE_EMPTY so that Drop impl does not call
                // assume_init_drop() on the already-moved value.
                this.state.store(STATE_EMPTY, Ordering::Relaxed);
                // SAFETY: Value has been moved out and state is no longer READY,
                // so Drop will not attempt to drop the value again. This ensures
                // non-value fields (WaitSync, initializer) are properly released.
                unsafe { ManuallyDrop::drop(&mut this) };
                Ok(value)
            }
            STATE_POISONED | STATE_COMPUTING => {
                // SAFETY: State is not READY, so Drop will not attempt to drop
                // the uninitialized value. This releases non-value fields.
                unsafe { ManuallyDrop::drop(&mut this) };
                Err(ConcurrentLazyPoisonedError)
            }
            STATE_EMPTY => {
                // SAFETY: STATE_EMPTY guarantees initializer is Some.
                let initializer = unsafe { (*this.initializer.get()).take() }.ok_or_else(|| {
                    // SAFETY: Drop non-value fields before returning error.
                    unsafe { ManuallyDrop::drop(&mut this) };
                    ConcurrentLazyPoisonedError
                })?;

                let result = catch_unwind(AssertUnwindSafe(initializer));
                #[allow(clippy::single_match_else, clippy::option_if_let_else)]
                match result {
                    Ok(value) => {
                        // SAFETY: Drop non-value fields (initializer already taken).
                        unsafe { ManuallyDrop::drop(&mut this) };
                        Ok(value)
                    }
                    Err(_) => {
                        this.state.store(STATE_POISONED, Ordering::Release);
                        // SAFETY: Drop non-value fields before panicking.
                        unsafe { ManuallyDrop::drop(&mut this) };
                        panic!("ConcurrentLazy: initialization function panicked");
                    }
                }
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
            wait_sync: CacheAligned(WaitSync::new()),
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
    /// Tries to force evaluation without panicking on poisoned state.
    ///
    /// This is a pure functional alternative to `force()` that returns a `Result`
    /// instead of panicking when the lazy value is poisoned. If another thread is
    /// currently initializing the value, this method waits indefinitely (using the
    /// same adaptive spin + Condvar strategy as `force()`) until initialization
    /// completes, then returns the result or the poisoned error.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)` if the value is successfully computed or already cached
    /// - `Err(ConcurrentLazyPoisonedError)` if the lazy value is poisoned
    ///   (initialization previously panicked)
    ///
    /// # Errors
    ///
    /// Returns `Err(ConcurrentLazyPoisonedError)` only if:
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
    pub fn try_force(&self) -> Result<&T, ConcurrentLazyPoisonedError> {
        let mut state = self.state.load(Ordering::Acquire);

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
                        Err(current_state) => {
                            state = current_state;
                        }
                    }
                }
                STATE_COMPUTING => {
                    self.spin_then_wait();
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
        match self.get() {
            Some(value) => fmt::Debug::fmt(value, formatter),
            None if self.is_poisoned() => formatter.write_str("<poisoned>"),
            None => formatter.write_str("<uninit>"),
        }
    }
}

impl<T: fmt::Display, F> fmt::Display for ConcurrentLazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Some(value) => fmt::Display::fmt(value, formatter),
            None if self.is_poisoned() => formatter.write_str("<poisoned>"),
            None => formatter.write_str("<uninit>"),
        }
    }
}

impl<T, F> Drop for ConcurrentLazy<T, F> {
    fn drop(&mut self) {
        if *self.state.get_mut() == STATE_READY {
            // SAFETY: STATE_READY guarantees value is initialized; &mut self guarantees exclusivity.
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

    // =========================================================================
    // Parking lot + Cache Line Separation Tests (Phase 1)
    // =========================================================================

    #[rstest]
    fn test_cache_aligned_alignment() {
        assert!(std::mem::align_of::<CacheAligned<WaitSync>>() >= CACHE_LINE_SIZE);
    }

    #[rstest]
    fn test_wait_sync_creation() {
        let wait_sync = WaitSync::new();
        let _guard = wait_sync.mutex.lock();
    }

    #[rstest]
    fn test_concurrent_lazy_struct_has_repr_c_layout() {
        let lazy = ConcurrentLazy::new(|| 42);
        let state_addr = &raw const lazy.state as usize;
        let wait_sync_addr = &raw const lazy.wait_sync as usize;
        assert!(wait_sync_addr > state_addr);
        assert_eq!(wait_sync_addr % CACHE_LINE_SIZE, 0);
    }

    #[rstest]
    fn test_reentrant_initialization_flag_mechanism() {
        IN_CONCURRENT_LAZY_INIT.with(|flag| {
            assert!(!flag.get());
            flag.set(true);
            assert!(flag.get());
            flag.set(false);
            assert!(!flag.get());
        });

        let lazy = ConcurrentLazy::new(|| 42);
        assert_eq!(*lazy.force(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_multiple_threads_wait_on_slow_init() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = Arc::new(ConcurrentLazy::new(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
            42
        }));

        let handles: Vec<_> = (0..32)
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

    #[rstest]
    fn test_concurrent_lazy_notify_all_wakes_waiting_threads() {
        let barrier = Arc::new(std::sync::Barrier::new(17));
        let lazy = Arc::new(ConcurrentLazy::new({
            let barrier = Arc::clone(&barrier);
            move || {
                barrier.wait();
                std::thread::sleep(std::time::Duration::from_millis(5));
                42
            }
        }));

        let handles: Vec<_> = (0..16)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    *lazy.force()
                })
            })
            .collect();

        let lazy_init = Arc::clone(&lazy);
        let init_handle = thread::spawn(move || *lazy_init.force());

        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }
        assert_eq!(init_handle.join().unwrap(), 42);
    }

    #[rstest]
    fn test_concurrent_lazy_poisoned_notifies_waiting_threads() {
        let lazy = Arc::new(ConcurrentLazy::new(|| -> i32 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            panic!("initialization failed");
        }));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || {
                    let result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));
                    result.is_err() // Should be Err (panic)
                })
            })
            .collect();

        for handle in handles {
            assert!(
                handle.join().unwrap(),
                "Thread should have observed panic/poisoned state"
            );
        }
    }

    #[rstest]
    fn test_try_force_waits_for_initialization() {
        use std::sync::atomic::AtomicBool;

        let started = Arc::new(AtomicBool::new(false));
        let started_clone = Arc::clone(&started);

        let lazy = Arc::new(ConcurrentLazy::new(move || {
            started_clone.store(true, AtomicOrdering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
            42
        }));

        let lazy_init = Arc::clone(&lazy);
        let init_handle = thread::spawn(move || *lazy_init.force());

        while !started.load(AtomicOrdering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // try_force waits indefinitely for initialization to complete, then returns Ok
        assert_eq!(*lazy.try_force().unwrap(), 42);
        assert_eq!(init_handle.join().unwrap(), 42);
    }

    #[rstest]
    fn test_try_force_returns_err_only_on_poisoned() {
        let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.force()));

        // try_force returns Err only when poisoned
        assert!(lazy.is_poisoned());
        assert_eq!(lazy.try_force(), Err(ConcurrentLazyPoisonedError));
    }

    #[rstest]
    fn test_adaptive_spin_completes_fast_init() {
        let lazy = Arc::new(ConcurrentLazy::new(|| 42));

        let handles: Vec<_> = (0..16)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || *lazy.force())
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }
    }

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
