//! Lazy evaluation with memoization.
//!
//! This module provides the `Lazy<T, F>` type for lazy evaluation.
//! Values are computed only when needed and cached for subsequent accesses.
//!
//! # Examples
//!
//! ```rust
//! use functional_rusty::control::Lazy;
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

use std::cell::{Ref, RefCell, RefMut};
use std::fmt;

/// The internal state of a `Lazy` value.
///
/// This enum tracks whether the lazy value has been initialized,
/// is still pending initialization, or has been poisoned due to
/// a panic during initialization.
#[derive(Debug)]
pub enum LazyState<T, F> {
    /// The value has not been initialized yet.
    /// Contains the initialization function.
    Uninit(F),
    /// The value has been initialized.
    /// Contains the computed value.
    Init(T),
    /// The initialization function panicked.
    /// The lazy value is now unusable.
    Poisoned,
}

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
/// This type is NOT thread-safe. For concurrent access, consider using
/// `std::sync::LazyLock` or wrapping in a `Mutex`.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use functional_rusty::control::Lazy;
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
/// use functional_rusty::control::Lazy;
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
    state: RefCell<LazyState<T, F>>,
}

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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| {
    ///     println!("Initializing...");
    ///     42
    /// });
    /// // Nothing printed yet
    /// ```
    #[inline]
    pub fn new(initializer: F) -> Self {
        Self {
            state: RefCell::new(LazyState::Uninit(initializer)),
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
    /// A `Ref<'_, T>` to the computed value. This is a smart pointer that
    /// maintains the borrow from the internal `RefCell`.
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    /// let value = lazy.force();
    /// assert_eq!(*value, 42);
    /// ```
    pub fn force(&self) -> Ref<'_, T> {
        // First, check if we need to initialize
        // We do this with a short borrow to avoid holding the borrow during initialization
        let needs_initialization = {
            let state = self.state.borrow();
            match &*state {
                LazyState::Init(_) => false,
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
                LazyState::Uninit(_) => true,
            }
        };
        // Borrow is released here

        // If initialization is needed, do it
        if needs_initialization {
            self.initialize();
        }

        // Now return a reference to the initialized value
        Ref::map(self.state.borrow(), |state| match state {
            LazyState::Init(value) => value,
            _ => panic!("Lazy should be initialized at this point"),
        })
    }

    /// Forces evaluation and returns a mutable reference to the value.
    ///
    /// If the value has not been computed yet, the initialization function
    /// is called and the result is cached. Subsequent calls return the
    /// cached value.
    ///
    /// # Returns
    ///
    /// A `RefMut<'_, T>` to the computed value. This is a smart pointer that
    /// maintains the mutable borrow from the internal `RefCell`.
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let mut lazy = Lazy::new(|| vec![1, 2, 3]);
    /// lazy.force_mut().push(4);
    /// assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn force_mut(&mut self) -> RefMut<'_, T> {
        // First, check if we need to initialize
        let needs_initialization = {
            let state = self.state.borrow();
            match &*state {
                LazyState::Init(_) => false,
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
                LazyState::Uninit(_) => true,
            }
        };

        // If initialization is needed, do it
        if needs_initialization {
            self.initialize();
        }

        // Now return a mutable reference to the initialized value
        RefMut::map(self.state.borrow_mut(), |state| match state {
            LazyState::Init(value) => value,
            _ => panic!("Lazy should be initialized at this point"),
        })
    }

    /// Performs the initialization.
    ///
    /// This method takes the initializer function, transitions to Poisoned state,
    /// runs the function, and then transitions to Init state if successful.
    /// If the function panics, the state remains Poisoned.
    fn initialize(&self) {
        let mut state = self.state.borrow_mut();

        // Double-check that we still need to initialize
        // (another code path might have initialized it)
        match &*state {
            LazyState::Init(_) => return,
            LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            LazyState::Uninit(_) => {}
        }

        // Take the initializer and transition to Poisoned state
        // This ensures that if the initializer panics, we stay in Poisoned state
        let LazyState::Uninit(initializer) = std::mem::replace(&mut *state, LazyState::Poisoned)
        else {
            unreachable!()
        };

        // Run the initializer
        let value = initializer();

        // Transition to Init state
        *state = LazyState::Init(value);
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new_with_value(42);
    /// assert!(lazy.is_initialized());
    /// ```
    #[inline]
    pub fn new_with_value(value: T) -> Self {
        Self {
            state: RefCell::new(LazyState::Init(value)),
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
    /// use functional_rusty::control::Lazy;
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
    /// - `Some(Ref<'_, T>)` if the value is initialized
    /// - `None` if the value has not been initialized or is poisoned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    ///
    /// assert!(lazy.get().is_none()); // Not initialized yet
    ///
    /// let _ = lazy.force();
    /// assert!(lazy.get().is_some()); // Now initialized
    /// ```
    pub fn get(&self) -> Option<Ref<'_, T>> {
        let state = self.state.borrow();
        if matches!(&*state, LazyState::Init(_)) {
            Some(Ref::map(state, |s| match s {
                LazyState::Init(value) => value,
                _ => unreachable!(),
            }))
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
    /// - `Some(RefMut<'_, T>)` if the value is initialized
    /// - `None` if the value has not been initialized yet
    ///
    /// # Panics
    ///
    /// Panics if the Lazy is in a poisoned state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let mut lazy = Lazy::new(|| 42);
    /// assert!(lazy.get_mut().is_none());
    /// lazy.force();
    /// assert!(lazy.get_mut().is_some());
    /// assert_eq!(*lazy.get_mut().unwrap(), 42);
    /// ```
    pub fn get_mut(&mut self) -> Option<RefMut<'_, T>> {
        let state = self.state.borrow();
        match &*state {
            LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            LazyState::Init(_) => {
                drop(state);
                Some(RefMut::map(self.state.borrow_mut(), |s| match s {
                    LazyState::Init(value) => value,
                    _ => unreachable!(),
                }))
            }
            LazyState::Uninit(_) => None,
        }
    }

    /// Returns whether the value has been initialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    /// assert!(!lazy.is_initialized());
    ///
    /// let _ = lazy.force();
    /// assert!(lazy.is_initialized());
    /// ```
    #[inline]
    pub fn is_initialized(&self) -> bool {
        matches!(&*self.state.borrow(), LazyState::Init(_))
    }

    /// Returns whether the lazy value has been poisoned.
    ///
    /// A lazy value becomes poisoned if the initialization function panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
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
        matches!(&*self.state.borrow(), LazyState::Poisoned)
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Consumes the Lazy and returns the inner value.
    ///
    /// If the Lazy has been initialized, returns `Ok(value)`.
    /// If it has not been initialized, forces evaluation and returns `Ok(value)`.
    /// If it is poisoned, returns `Err(())`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 42);
    /// assert_eq!(lazy.into_inner(), Ok(42));
    /// ```
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new_with_value(42);
    /// assert_eq!(lazy.into_inner(), Ok(42));
    /// ```
    pub fn into_inner(self) -> Result<T, ()> {
        match self.state.into_inner() {
            LazyState::Init(value) => Ok(value),
            LazyState::Uninit(initializer) => Ok(initializer()),
            LazyState::Poisoned => Err(()),
        }
    }
}

// =============================================================================
// Functor-like Operations (map, flat_map)
// =============================================================================

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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 21);
    /// let doubled = lazy.map(|x| x * 2);
    ///
    /// assert_eq!(*doubled.force(), 42);
    /// ```
    pub fn map<U, G>(self, function: G) -> Lazy<U, impl FnOnce() -> U>
    where
        G: FnOnce(T) -> U,
    {
        Lazy::new(move || {
            let value = match self.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
            function(value)
        })
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy = Lazy::new(|| 21);
    /// let result = lazy.flat_map(|x| Lazy::new(move || x * 2));
    ///
    /// assert_eq!(*result.force(), 42);
    /// ```
    pub fn flat_map<U, FunctionResult, G>(self, function: G) -> Lazy<U, impl FnOnce() -> U>
    where
        FunctionResult: FnOnce() -> U,
        G: FnOnce(T) -> Lazy<U, FunctionResult>,
    {
        Lazy::new(move || {
            let value = match self.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
            let lazy_result = function(value);
            match lazy_result.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            }
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy1 = Lazy::new(|| 1);
    /// let lazy2 = Lazy::new(|| "hello");
    /// let combined = lazy1.zip(lazy2);
    ///
    /// assert_eq!(*combined.force(), (1, "hello"));
    /// ```
    pub fn zip<U, OtherFunction>(
        self,
        other: Lazy<U, OtherFunction>,
    ) -> Lazy<(T, U), impl FnOnce() -> (T, U)>
    where
        OtherFunction: FnOnce() -> U,
    {
        Lazy::new(move || {
            let value1 = match self.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
            let value2 = match other.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
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
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy1 = Lazy::new(|| 20);
    /// let lazy2 = Lazy::new(|| 22);
    /// let sum = lazy1.zip_with(lazy2, |a, b| a + b);
    ///
    /// assert_eq!(*sum.force(), 42);
    /// ```
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
            let value1 = match self.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
            let value2 = match other.state.into_inner() {
                LazyState::Init(value) => value,
                LazyState::Uninit(initializer) => initializer(),
                LazyState::Poisoned => panic!("Lazy instance has been poisoned"),
            };
            function(value1, value2)
        })
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl<T: Default> Default for Lazy<T> {
    /// Creates a lazy value that computes the default value of `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::control::Lazy;
    ///
    /// let lazy: Lazy<i32> = Lazy::default();
    /// assert_eq!(*lazy.force(), 0);
    /// ```
    fn default() -> Self {
        Lazy::new(T::default)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        match &*state {
            LazyState::Init(value) => formatter.debug_tuple("Lazy").field(value).finish(),
            LazyState::Uninit(_) => formatter.debug_tuple("Lazy").field(&"<uninit>").finish(),
            LazyState::Poisoned => formatter.debug_tuple("Lazy").field(&"<poisoned>").finish(),
        }
    }
}

// Note: We intentionally do NOT implement Deref for Lazy.
//
// Reason: RefCell-based implementation returns Ref<'_, T> from force(),
// not &T. Deref requires returning &Target, but we cannot convert
// Ref<'_, T> to &T without breaking borrow checker guarantees.
//
// Users must explicitly call force() to access the value.
// This also makes the laziness explicit in the code.

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::cell::Cell;

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
}
