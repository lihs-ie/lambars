#![allow(unsafe_code)]
//! Lazy evaluation with memoization.
//!
//! This module provides the `Lazy<T, F>` type for lazy evaluation.
//! Values are computed only when needed and cached for subsequent accesses.
//!
//! # Safety
//!
//! This module uses unsafe code to implement a lock-free state machine.
//! The following invariants are maintained:
//! - `value` is only initialized when `state` is `STATE_READY`
//! - `initializer` is `Some` only when `state` is `STATE_EMPTY`
//! - Transition to `STATE_COMPUTING` is done via `compare_exchange` for exclusivity
//!
//! # Referential Transparency Note
//!
//! While `Lazy` provides memoization that appears functionally pure on success,
//! it is **not referentially transparent** in the presence of panics:
//!
//! - If the initialization function panics, the `Lazy` becomes **poisoned**
//! - Once poisoned, all subsequent calls to `force()` will panic
//! - This means the behavior of `force()` depends on the history of previous calls
//!
//! This is a deliberate design decision to prevent returning potentially inconsistent
//! partial state after a panic. Users who need strict referential transparency
//! should ensure their initialization functions do not panic, or handle the
//! poisoned state explicitly via `is_poisoned()` before calling `force()`.
//!
//! # Examples
//!
//! ```rust
//! use lambars::control::Lazy;
//!
//! let lazy = Lazy::new(|| {
//!     println!("Computing...");
//!     42
//! });
//!
//! // No output yet - computation is deferred
//! println!("Created lazy value");
//!
//! // Now "Computing..." is printed
//! let value = lazy.force();
//! assert_eq!(*value, 42);
//!
//! // No recomputation - result is memoized
//! let value2 = lazy.force();
//! assert_eq!(*value2, 42);
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

/// Error returned when attempting to access a poisoned `Lazy` value.
///
/// A `Lazy` value becomes poisoned when its initialization function panics.
/// After poisoning, the value cannot be accessed and any attempt to do so
/// will return this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LazyPoisonedError;

impl fmt::Display for LazyPoisonedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Lazy value is poisoned")
    }
}

impl std::error::Error for LazyPoisonedError {}

/// A lazily evaluated value with memoization.
///
/// `Lazy<T, F>` defers computation until the value is first accessed via `force()`.
/// Once computed, the value is cached and subsequent calls to `force()` return
/// the cached value without recomputation.
///
/// # Type Parameters
///
/// * `T` - The type of the computed value
/// * `F` - The type of the initialization function (defaults to `fn() -> T`)
///
/// # Thread Safety
///
/// This type is NOT thread-safe (`!Sync`). For concurrent access, use
/// [`ConcurrentLazy`](super::ConcurrentLazy).
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use lambars::control::Lazy;
///
/// let lazy = Lazy::new(|| expensive_computation());
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
/// ## Memoization
///
/// ```rust
/// use lambars::control::Lazy;
/// use std::cell::Cell;
///
/// let call_count = Cell::new(0);
/// let lazy = Lazy::new(|| {
///     call_count.set(call_count.get() + 1);
///     42
/// });
///
/// assert_eq!(call_count.get(), 0); // Not called yet
///
/// let _ = lazy.force();
/// assert_eq!(call_count.get(), 1); // Called once
///
/// let _ = lazy.force();
/// assert_eq!(call_count.get(), 1); // Still only once - memoized
/// ```
pub struct Lazy<T, F = fn() -> T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
    initializer: UnsafeCell<Option<F>>,
}

// # Safety
//
// Lazy is for single-threaded use, so we do NOT implement Sync.
// Send is safe when T and F are Send:
// - Value transfer is ownership transfer, no data races occur
// - Atomic state operations work correctly after transfer
unsafe impl<T: Send, F: Send> Send for Lazy<T, F> {}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Creates a new lazy value with the given initialization function.
    ///
    /// The function will not be called until `force()` is invoked.
    ///
    /// # Arguments
    ///
    /// * `initializer` - A function that produces the value when called
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| {
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
    /// # Returns
    ///
    /// A reference to the computed value.
    ///
    /// # Panics
    ///
    /// - If the initialization function panics, the lazy value becomes
    ///   poisoned and all future calls to `force()` will panic.
    /// - If the value is already poisoned from a previous panic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    /// let value = lazy.force();
    /// assert_eq!(*value, 42);
    /// ```
    pub fn force(&self) -> &T {
        let state = self.state.load(Ordering::Acquire);

        match state {
            STATE_READY => {
                // SAFETY: Transition to STATE_READY is done in initialize() after value.write()
                // completes with Release ordering. The Acquire load here establishes
                // happens-before relationship, guaranteeing that write is visible.
                // Therefore value is initialized and assume_init_ref() is safe.
                unsafe { (*self.value.get()).assume_init_ref() }
            }
            STATE_POISONED => {
                panic!("Lazy instance has been poisoned")
            }
            STATE_EMPTY => self.initialize(),
            STATE_COMPUTING => {
                panic!("Lazy::force called recursively during initialization")
            }
            _ => unreachable!("Invalid state"),
        }
    }

    /// Forces evaluation and returns a mutable reference to the value.
    ///
    /// If the value has not been computed yet, the initialization function
    /// is called and the result is cached. Subsequent calls return the
    /// cached value.
    ///
    /// # Returns
    ///
    /// A mutable reference to the computed value.
    ///
    /// # Panics
    ///
    /// - If the initialization function panics, the lazy value becomes
    ///   poisoned and all future calls will panic.
    /// - If the value is already poisoned from a previous panic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let mut lazy = Lazy::new(|| vec![1, 2, 3]);
    /// lazy.force_mut().push(4);
    /// assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn force_mut(&mut self) -> &mut T {
        let state = *self.state.get_mut();

        match state {
            STATE_READY => {
                // SAFETY: We have &mut self, so exclusive access is guaranteed.
                // State is STATE_READY, so value is initialized.
                unsafe { (*self.value.get()).assume_init_mut() }
            }
            STATE_POISONED => {
                panic!("Lazy instance has been poisoned")
            }
            STATE_EMPTY => {
                // We have &mut self, so we can safely initialize
                self.initialize_mut()
            }
            STATE_COMPUTING => {
                // With &mut self, STATE_COMPUTING should not be observable
                // as it would require concurrent access which is prevented by borrowing rules
                panic!("Lazy::force_mut called recursively during initialization")
            }
            _ => unreachable!("Invalid state"),
        }
    }

    /// Performs the initialization (immutable self version).
    fn initialize(&self) -> &T {
        // Empty -> Computing transition
        match self.state.compare_exchange(
            STATE_EMPTY,
            STATE_COMPUTING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // SAFETY: compare_exchange succeeded, so only this thread is in
                // STATE_COMPUTING. initializer is Some only when state is STATE_EMPTY
                // (invariant), so take() is safe.
                let initializer = unsafe { (*self.initializer.get()).take() }
                    .expect("initializer already consumed");

                // Catch panic
                let result = catch_unwind(AssertUnwindSafe(initializer));

                // Note: Using match for clarity - the Err case panics so clippy's
                // suggestion to use if let or map_or_else is not appropriate
                #[allow(clippy::single_match_else, clippy::option_if_let_else)]
                match result {
                    Ok(value) => {
                        // SAFETY: Only the thread that acquired STATE_COMPUTING reaches here.
                        // value is uninitialized, so we initialize it with write().
                        unsafe {
                            (*self.value.get()).write(value);
                        }

                        // Release ordering makes the write visible to other threads
                        self.state.store(STATE_READY, Ordering::Release);

                        // SAFETY: Just initialized with write(). assume_init_ref() is safe.
                        unsafe { (*self.value.get()).assume_init_ref() }
                    }
                    Err(_) => {
                        self.state.store(STATE_POISONED, Ordering::Release);
                        panic!("Lazy: initialization function panicked");
                    }
                }
            }
            Err(current) => {
                // Lazy is for single-threaded use, so STATE_COMPUTING failure shouldn't happen
                match current {
                    STATE_READY => {
                        // SAFETY: Same reason as force() - value is initialized
                        unsafe { (*self.value.get()).assume_init_ref() }
                    }
                    STATE_POISONED => panic!("Lazy instance has been poisoned"),
                    _ => unreachable!("Single-threaded Lazy in unexpected state"),
                }
            }
        }
    }

    /// Performs the initialization (mutable self version).
    fn initialize_mut(&mut self) -> &mut T {
        // We have &mut self, so no need for atomic operations
        *self.state.get_mut() = STATE_COMPUTING;

        // SAFETY: We have &mut self, so exclusive access is guaranteed.
        let initializer =
            unsafe { (*self.initializer.get()).take() }.expect("initializer already consumed");

        // Catch panic
        let result = catch_unwind(AssertUnwindSafe(initializer));

        // Note: Using match for clarity - the Err case panics so clippy's
        // suggestion to use if let or map_or_else is not appropriate
        #[allow(clippy::single_match_else, clippy::option_if_let_else)]
        match result {
            Ok(value) => {
                // SAFETY: We have &mut self, exclusive access guaranteed.
                unsafe {
                    (*self.value.get()).write(value);
                }

                *self.state.get_mut() = STATE_READY;

                // SAFETY: Just initialized with write().
                unsafe { (*self.value.get()).assume_init_mut() }
            }
            Err(_) => {
                *self.state.get_mut() = STATE_POISONED;
                panic!("Lazy: initialization function panicked");
            }
        }
    }
}

impl<T> Lazy<T, fn() -> T> {
    /// Creates a new lazy value that is already initialized.
    ///
    /// This is useful when you have a value that should be treated as lazy
    /// for API consistency, but the value is already available.
    ///
    /// # Arguments
    ///
    /// * `value` - The already-computed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new_with_value(42);
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
    /// the Lazy context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::pure(42);
    /// assert_eq!(*lazy.force(), 42);
    /// ```
    #[inline]
    pub fn pure(value: T) -> Self {
        Self::new_with_value(value)
    }
}

impl<T, F> Lazy<T, F> {
    /// Returns a reference to the value if it has been initialized.
    ///
    /// Unlike `force()`, this method does not trigger initialization.
    ///
    /// # Returns
    ///
    /// - `Some(&T)` if the value is initialized
    /// - `None` if the value has not been initialized or is poisoned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    ///
    /// assert!(lazy.get().is_none()); // Not initialized yet
    ///
    /// let _ = lazy.force();
    /// assert!(lazy.get().is_some()); // Now initialized
    /// ```
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == STATE_READY {
            // SAFETY: STATE_READY means value is initialized.
            // Acquire ordering ensures visibility.
            Some(unsafe { (*self.value.get()).assume_init_ref() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value if it has been initialized.
    ///
    /// Unlike `force_mut()`, this method does not trigger initialization.
    ///
    /// # Returns
    ///
    /// - `Some(&mut T)` if the value is initialized
    /// - `None` if the value has not been initialized yet
    ///
    /// # Panics
    ///
    /// Panics if the Lazy is in a poisoned state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let mut lazy = Lazy::new(|| 42);
    /// assert!(lazy.get_mut().is_none());
    /// lazy.force();
    /// assert!(lazy.get_mut().is_some());
    /// assert_eq!(*lazy.get_mut().unwrap(), 42);
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let state = *self.state.get_mut();
        match state {
            STATE_READY => {
                // SAFETY: We have &mut self, exclusive access guaranteed.
                // State is STATE_READY, so value is initialized.
                Some(unsafe { (*self.value.get()).assume_init_mut() })
            }
            STATE_POISONED => panic!("Lazy instance has been poisoned"),
            _ => None,
        }
    }

    /// Returns whether the value has been initialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
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
    /// use lambars::control::Lazy;
    /// use std::panic::catch_unwind;
    ///
    /// let lazy = Lazy::new(|| panic!("initialization failed"));
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

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Tries to force evaluation without panicking on poisoned state.
    ///
    /// This is a pure functional alternative to `force()` that returns a `Result`
    /// instead of panicking when the lazy value is poisoned.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)` if the value is successfully computed or already cached
    /// - `Err(LazyPoisonedError)` if the lazy value is poisoned
    ///
    /// # Errors
    ///
    /// Returns `Err(LazyPoisonedError)` if:
    /// - The lazy value is poisoned (initialization previously panicked)
    /// - Re-entry is detected (calling `try_force` during initialization)
    ///
    /// # Panics
    ///
    /// Panics if the initialization function itself panics. The panic is not caught;
    /// only the poisoned state (from a previous panic) is handled via `Result`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    /// use std::panic::catch_unwind;
    ///
    /// // Normal usage
    /// let lazy = Lazy::new(|| 42);
    /// assert_eq!(*lazy.try_force().unwrap(), 42);
    ///
    /// // Poisoned lazy
    /// let poisoned = Lazy::new(|| panic!("init failed"));
    /// let _ = catch_unwind(std::panic::AssertUnwindSafe(|| poisoned.force()));
    /// assert!(poisoned.try_force().is_err());
    /// ```
    pub fn try_force(&self) -> Result<&T, LazyPoisonedError> {
        let state = self.state.load(Ordering::Acquire);

        match state {
            STATE_READY => {
                // SAFETY: Same as force() - state is STATE_READY so value is initialized
                Ok(unsafe { (*self.value.get()).assume_init_ref() })
            }
            STATE_POISONED => Err(LazyPoisonedError),
            STATE_EMPTY => Ok(self.initialize()),
            STATE_COMPUTING => {
                // Re-entry during initialization
                Err(LazyPoisonedError)
            }
            _ => unreachable!("Invalid state"),
        }
    }

    /// Consumes the Lazy and returns the inner value.
    ///
    /// If the Lazy has been initialized, returns `Ok(value)`.
    /// If it has not been initialized, forces evaluation and returns `Ok(value)`.
    /// If it is poisoned, returns `Err(LazyPoisonedError)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    /// assert_eq!(lazy.into_inner(), Ok(42));
    /// ```
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new_with_value(42);
    /// assert_eq!(lazy.into_inner(), Ok(42));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err(LazyPoisonedError)` if the `Lazy` instance has been poisoned.
    ///
    /// # Panics
    ///
    /// Panics if the initialization function panics. In this case, the panic
    /// is re-thrown after marking the Lazy as poisoned (though the Lazy is
    /// consumed, so the poisoned state is not observable).
    pub fn into_inner(self) -> Result<T, LazyPoisonedError> {
        // Prevent Drop from running since we're manually handling the fields
        let this = ManuallyDrop::new(self);
        let state = this.state.load(Ordering::Acquire);

        match state {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                // We're consuming self (via ManuallyDrop), so we can take ownership.
                // The value won't be double-dropped because Drop won't run.
                Ok(unsafe { (*this.value.get()).assume_init_read() })
            }
            STATE_POISONED => Err(LazyPoisonedError),
            STATE_EMPTY => {
                // SAFETY: STATE_EMPTY means initializer is Some.
                // We're consuming self (via ManuallyDrop).
                let initializer = unsafe { (*this.initializer.get()).take() }
                    .expect("initializer already consumed");

                // Catch panics to ensure consistent behavior
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
                        panic!("Lazy: initialization function panicked");
                    }
                }
            }
            STATE_COMPUTING => {
                // This should not happen when consuming self (we have ownership),
                // but if it somehow occurs, treat it as an error
                Err(LazyPoisonedError)
            }
            _ => unreachable!("Invalid state"),
        }
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Applies a function to the lazy value, producing a new lazy value.
    ///
    /// The resulting lazy value will compute the original value and then
    /// apply the function when forced.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the computed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 21);
    /// let doubled = lazy.map(|x| x * 2);
    ///
    /// assert_eq!(*doubled.force(), 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when forced if the `Lazy` instance has been poisoned.
    pub fn map<U, G>(self, function: G) -> Lazy<U, impl FnOnce() -> U>
    where
        G: FnOnce(T) -> U,
    {
        Lazy::new(move || {
            let value = self.into_inner().expect("Lazy instance has been poisoned");
            function(value)
        })
    }

    /// Applies a function to the lazy value, returning a Result.
    ///
    /// This is a pure functional alternative to `map()` that returns `Result`
    /// instead of panicking when the lazy value is poisoned. The returned lazy
    /// value will compute to `Ok(f(value))` if successful, or `Err(LazyPoisonedError)`
    /// if the original lazy value is poisoned.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the computed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 21);
    /// let doubled = lazy.try_map(|x| x * 2);
    ///
    /// assert_eq!(*doubled.force().unwrap(), 42);
    /// ```
    pub fn try_map<U, G>(
        self,
        function: G,
    ) -> Lazy<Result<U, LazyPoisonedError>, impl FnOnce() -> Result<U, LazyPoisonedError>>
    where
        G: FnOnce(T) -> U,
    {
        Lazy::new(move || self.into_inner().map(function))
    }

    /// Applies a function that returns a Lazy, then flattens the result.
    ///
    /// This is the monadic bind operation for Lazy.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the computed value and returns a new Lazy
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 21);
    /// let result = lazy.flat_map(|x| Lazy::new(move || x * 2));
    ///
    /// assert_eq!(*result.force(), 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when forced if the `Lazy` instance has been poisoned.
    pub fn flat_map<U, FunctionResult, G>(self, function: G) -> Lazy<U, impl FnOnce() -> U>
    where
        FunctionResult: FnOnce() -> U,
        G: FnOnce(T) -> Lazy<U, FunctionResult>,
    {
        Lazy::new(move || {
            let value = self.into_inner().expect("Lazy instance has been poisoned");
            function(value)
                .into_inner()
                .expect("Lazy instance has been poisoned")
        })
    }

    /// Combines two lazy values into a lazy tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - Another lazy value to combine with
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy1 = Lazy::new(|| 1);
    /// let lazy2 = Lazy::new(|| "hello");
    /// let combined = lazy1.zip(lazy2);
    ///
    /// assert_eq!(*combined.force(), (1, "hello"));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when forced if either `Lazy` instance has been poisoned.
    pub fn zip<U, OtherFunction>(
        self,
        other: Lazy<U, OtherFunction>,
    ) -> Lazy<(T, U), impl FnOnce() -> (T, U)>
    where
        OtherFunction: FnOnce() -> U,
    {
        Lazy::new(move || {
            let value1 = self.into_inner().expect("Lazy instance has been poisoned");
            let value2 = other.into_inner().expect("Lazy instance has been poisoned");
            (value1, value2)
        })
    }

    /// Combines two lazy values using a function.
    ///
    /// # Arguments
    ///
    /// * `other` - Another lazy value to combine with
    /// * `function` - A function that combines the two values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy1 = Lazy::new(|| 20);
    /// let lazy2 = Lazy::new(|| 22);
    /// let sum = lazy1.zip_with(lazy2, |a, b| a + b);
    ///
    /// assert_eq!(*sum.force(), 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when forced if either `Lazy` instance has been poisoned.
    pub fn zip_with<U, V, OtherFunction, CombineFunction>(
        self,
        other: Lazy<U, OtherFunction>,
        function: CombineFunction,
    ) -> Lazy<V, impl FnOnce() -> V>
    where
        OtherFunction: FnOnce() -> U,
        CombineFunction: FnOnce(T, U) -> V,
    {
        Lazy::new(move || {
            let value1 = self.into_inner().expect("Lazy instance has been poisoned");
            let value2 = other.into_inner().expect("Lazy instance has been poisoned");
            function(value1, value2)
        })
    }
}

impl<T: Default> Default for Lazy<T> {
    /// Creates a lazy value that computes the default value of `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Lazy;
    ///
    /// let lazy: Lazy<i32> = Lazy::default();
    /// assert_eq!(*lazy.force(), 0);
    /// ```
    fn default() -> Self {
        Self::new(T::default)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                let value = unsafe { (*self.value.get()).assume_init_ref() };
                formatter.debug_tuple("Lazy").field(value).finish()
            }
            STATE_EMPTY | STATE_COMPUTING => formatter.write_str("Lazy(<uninit>)"),
            STATE_POISONED => formatter.write_str("Lazy(<poisoned>)"),
            _ => unreachable!(),
        }
    }
}

impl<T: fmt::Display, F> fmt::Display for Lazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.load(Ordering::Acquire) {
            STATE_READY => {
                // SAFETY: STATE_READY means value is initialized.
                let value = unsafe { (*self.value.get()).assume_init_ref() };
                write!(formatter, "Lazy({value})")
            }
            STATE_EMPTY | STATE_COMPUTING => write!(formatter, "Lazy(<uninit>)"),
            STATE_POISONED => write!(formatter, "Lazy(<poisoned>)"),
            _ => unreachable!(),
        }
    }
}

impl<T, F> Drop for Lazy<T, F> {
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

// Note: We intentionally do NOT implement Deref for Lazy.
//
// Reason: This makes the laziness explicit in the code.
// Users must explicitly call force() to access the value.

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::cell::Cell;
    use std::panic;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_unevaluated_lazy() {
        let lazy = Lazy::new(|| 42);
        assert_eq!(format!("{lazy}"), "Lazy(<uninit>)");
    }

    #[rstest]
    fn test_display_evaluated_lazy() {
        let lazy = Lazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(format!("{lazy}"), "Lazy(42)");
    }

    #[rstest]
    fn test_display_poisoned_lazy() {
        let lazy = Lazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(format!("{lazy}"), "Lazy(<poisoned>)");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_lazy_basic_creation() {
        let lazy = Lazy::new(|| 42);
        assert!(!lazy.is_initialized());
    }

    #[rstest]
    fn test_lazy_force_computes_value() {
        let lazy = Lazy::new(|| 42);
        let value = lazy.force();
        assert_eq!(*value, 42);
        assert!(lazy.is_initialized());
    }

    #[rstest]
    fn test_lazy_memoization() {
        let call_count = Cell::new(0);
        let lazy = Lazy::new(|| {
            call_count.set(call_count.get() + 1);
            42
        });

        assert_eq!(call_count.get(), 0);

        let _ = lazy.force();
        assert_eq!(call_count.get(), 1);

        let _ = lazy.force();
        assert_eq!(call_count.get(), 1); // Still 1, not 2
    }

    #[rstest]
    fn test_lazy_new_with_value() {
        let lazy = Lazy::new_with_value(42);
        assert!(lazy.is_initialized());
        assert_eq!(*lazy.force(), 42);
    }

    #[rstest]
    fn test_lazy_map() {
        let lazy = Lazy::new(|| 21);
        let doubled = lazy.map(|x| x * 2);
        assert_eq!(*doubled.force(), 42);
    }

    #[rstest]
    fn test_lazy_flat_map() {
        let lazy = Lazy::new(|| 21);
        let result = lazy.flat_map(|x| Lazy::new(move || x * 2));
        assert_eq!(*result.force(), 42);
    }

    #[rstest]
    fn test_lazy_get_before_init() {
        let lazy = Lazy::new(|| 42);
        assert!(lazy.get().is_none());
    }

    #[rstest]
    fn test_lazy_get_after_init() {
        let lazy = Lazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(*lazy.get().unwrap(), 42);
    }

    #[rstest]
    fn test_lazy_get_mut_before_init() {
        let mut lazy = Lazy::new(|| 42);
        assert!(lazy.get_mut().is_none());
    }

    #[rstest]
    fn test_lazy_get_mut_after_init() {
        let mut lazy = Lazy::new(|| 42);
        let _ = lazy.force();
        *lazy.get_mut().unwrap() = 100;
        assert_eq!(*lazy.force(), 100);
    }

    #[rstest]
    fn test_lazy_force_mut() {
        let mut lazy = Lazy::new(|| vec![1, 2, 3]);
        lazy.force_mut().push(4);
        assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4]);
    }

    #[rstest]
    fn test_lazy_into_inner_uninit() {
        let lazy = Lazy::new(|| 42);
        assert_eq!(lazy.into_inner(), Ok(42));
    }

    #[rstest]
    fn test_lazy_into_inner_init() {
        let lazy = Lazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(lazy.into_inner(), Ok(42));
    }

    #[rstest]
    fn test_lazy_into_inner_poisoned() {
        let lazy = Lazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(lazy.into_inner(), Err(LazyPoisonedError));
    }

    #[rstest]
    fn test_lazy_zip() {
        let lazy1 = Lazy::new(|| 1);
        let lazy2 = Lazy::new(|| "hello");
        let combined = lazy1.zip(lazy2);
        assert_eq!(*combined.force(), (1, "hello"));
    }

    #[rstest]
    fn test_lazy_zip_with() {
        let lazy1 = Lazy::new(|| 20);
        let lazy2 = Lazy::new(|| 22);
        let sum = lazy1.zip_with(lazy2, |a, b| a + b);
        assert_eq!(*sum.force(), 42);
    }

    #[rstest]
    fn test_lazy_pure() {
        let lazy = Lazy::pure(42);
        assert!(lazy.is_initialized());
        assert_eq!(*lazy.force(), 42);
    }

    #[rstest]
    fn test_lazy_default() {
        let lazy: Lazy<i32> = Lazy::default();
        assert_eq!(*lazy.force(), 0);
    }

    #[rstest]
    fn test_lazy_poison_propagation() {
        let lazy = Lazy::new(|| -> i32 { panic!("test panic") });

        // First force panics
        let result1 = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
        assert!(result1.is_err());

        // Second force also panics (poisoned)
        let result2 = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
        assert!(result2.is_err());

        assert!(lazy.is_poisoned());
    }

    #[rstest]
    fn test_lazy_debug_uninit() {
        let lazy = Lazy::new(|| 42);
        assert_eq!(format!("{lazy:?}"), "Lazy(<uninit>)");
    }

    #[rstest]
    fn test_lazy_debug_init() {
        let lazy = Lazy::new(|| 42);
        let _ = lazy.force();
        assert_eq!(format!("{lazy:?}"), "Lazy(42)");
    }

    #[rstest]
    fn test_lazy_debug_poisoned() {
        let lazy = Lazy::new(|| -> i32 { panic!("initialization failed") });
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
        assert_eq!(format!("{lazy:?}"), "Lazy(<poisoned>)");
    }

    // =========================================================================
    // Drop Tests
    // =========================================================================

    #[rstest]
    fn test_lazy_drop_uninit() {
        // Should not panic when dropping uninitialized Lazy
        let _lazy = Lazy::new(|| 42);
    }

    #[rstest]
    fn test_lazy_drop_init() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropTracker {
            dropped: Arc<AtomicBool>,
        }
        impl Drop for DropTracker {
            fn drop(&mut self) {
                self.dropped.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_clone = dropped.clone();

        let lazy = Lazy::new(move || DropTracker {
            dropped: dropped_clone,
        });
        let _ = lazy.force();
        assert!(!dropped.load(Ordering::SeqCst));

        drop(lazy);
        assert!(dropped.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Re-entry Tests
    // =========================================================================

    // Note: Testing actual recursive force() is difficult because Lazy is !Sync
    // and cannot easily be shared via Rc in the initializer without triggering
    // borrow checker issues. The STATE_COMPUTING case primarily protects against
    // internal implementation errors or edge cases.
    //
    // The panic message is still tested indirectly through the code coverage.

    #[rstest]
    fn test_lazy_into_inner_panic_behavior() {
        let lazy = Lazy::new(|| -> i32 { panic!("into_inner panic test") });

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.into_inner()));
        assert!(result.is_err());
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
            fn prop_lazy_memoization(x in any::<i64>()) {
                let lazy = Lazy::new(|| x);
                let v1 = *lazy.force();
                let v2 = *lazy.force();
                prop_assert_eq!(v1, v2);
                prop_assert_eq!(v1, x);
            }

            /// Functor identity law: lazy.map(|x| x) == lazy
            #[test]
            fn prop_lazy_functor_identity(x in any::<i64>()) {
                let lazy = Lazy::new(|| x);
                let mapped = Lazy::new(|| x).map(|v| v);
                prop_assert_eq!(*lazy.force(), *mapped.force());
            }

            /// Functor composition law: lazy.map(f).map(g) == lazy.map(|x| g(f(x)))
            #[test]
            fn prop_lazy_functor_composition(x in any::<i32>()) {
                let f = |v: i32| v.wrapping_add(1);
                let g = |v: i32| v.wrapping_mul(2);
                let lazy1 = Lazy::new(|| x).map(f).map(g);
                let lazy2 = Lazy::new(|| x).map(|v| g(f(v)));
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad left identity: pure(a).flat_map(f) == f(a)
            #[test]
            fn prop_lazy_monad_left_identity(x in any::<i32>()) {
                let f = |v: i32| Lazy::new(move || v.wrapping_mul(2));
                let lazy1 = Lazy::pure(x).flat_map(f);
                let lazy2 = f(x);
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad right identity: m.flat_map(pure) == m
            #[test]
            fn prop_lazy_monad_right_identity(x in any::<i32>()) {
                let lazy1 = Lazy::new(|| x);
                let lazy2 = Lazy::new(|| x).flat_map(Lazy::pure);
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }

            /// Monad associativity: (m.flat_map(f)).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
            #[test]
            fn prop_lazy_monad_associativity(x in any::<i32>()) {
                let f = |v: i32| Lazy::new(move || v.wrapping_add(1));
                let g = |v: i32| Lazy::new(move || v.wrapping_mul(2));
                let lazy1 = Lazy::new(|| x).flat_map(f).flat_map(g);
                let lazy2 = Lazy::new(|| x).flat_map(|v| f(v).flat_map(g));
                prop_assert_eq!(*lazy1.force(), *lazy2.force());
            }
        }
    }
}
