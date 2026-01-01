//! `MonadState` type class - stateful computation capability.
//!
//! This module provides the `MonadState` trait which abstracts
//! the ability to read and modify state. This is the core
//! abstraction of the State monad pattern.
//!
//! # Laws
//!
//! All `MonadState` implementations must satisfy these laws:
//!
//! ## Get Put Law
//!
//! Getting and then putting the same state is a no-op:
//!
//! ```text
//! get().flat_map(|s| put(s)) == pure(())
//! ```
//!
//! ## Put Get Law
//!
//! After putting a state, get should return that state:
//!
//! ```text
//! put(s).then(get()) returns s
//! ```
//!
//! ## Put Put Law
//!
//! Consecutive puts result in the last put winning:
//!
//! ```text
//! put(s1).then(put(s2)) == put(s2)
//! ```
//!
//! ## Modify Composition Law
//!
//! Consecutive modifies compose the functions:
//!
//! ```text
//! modify(f).then(modify(g)) == modify(|s| g(f(s)))
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::effect::MonadState;
//!
//! // State implementation will provide concrete examples
//! ```

use crate::typeclass::Monad;

/// A type class for monads that can read and modify state.
///
/// `MonadState<S>` extends `Monad` with the ability to access and
/// modify a state of type `S`. This is useful for computations that
/// need to maintain mutable state in a pure functional way.
///
/// # Type Parameters
///
/// - `S`: The state type
///
/// # Laws
///
/// ## Get Put Law
///
/// Getting and then putting the same state is a no-op:
///
/// ```text
/// get().flat_map(|s| put(s)) == pure(())
/// ```
///
/// ## Put Get Law
///
/// After putting a state, get should return that state:
///
/// ```text
/// put(s).then(get()) returns s
/// ```
///
/// ## Put Put Law
///
/// Consecutive puts result in the last put winning:
///
/// ```text
/// put(s1).then(put(s2)) == put(s2)
/// ```
///
/// ## Modify Composition Law
///
/// Consecutive modifies compose the functions:
///
/// ```text
/// modify(f).then(modify(g)) == modify(|s| g(f(s)))
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::{MonadState, State};
///
/// let computation = State::get()
///     .flat_map(|current: i32| {
///         State::put(current + 1).map(|_| current)
///     });
///
/// let (result, final_state) = computation.run(10);
/// assert_eq!(result, 10);
/// assert_eq!(final_state, 11);
/// ```
pub trait MonadState<S>: Monad {
    /// Retrieves the current state.
    ///
    /// Returns a computation that produces the current state
    /// without modifying it.
    ///
    /// # Returns
    ///
    /// A computation that produces the current state.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadState, State};
    ///
    /// let state: State<i32, i32> = State::get();
    /// let (result, final_state) = state.run(42);
    /// assert_eq!(result, 42);
    /// assert_eq!(final_state, 42);
    /// ```
    fn get() -> Self::WithType<S>
    where
        S: Clone;

    /// Sets the state to a new value.
    ///
    /// Returns a computation that replaces the current state
    /// with the given value and produces unit.
    ///
    /// # Arguments
    ///
    /// * `state` - The new state value
    ///
    /// # Returns
    ///
    /// A computation that sets the state and produces unit.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadState, State};
    ///
    /// let state: State<i32, ()> = State::put(100);
    /// let (_, final_state) = state.run(42);
    /// assert_eq!(final_state, 100);
    /// ```
    fn put(state: S) -> Self::WithType<()>;

    /// Applies a state transition function.
    ///
    /// This is the most general state operation. It takes a function
    /// that receives the current state and produces both a result
    /// and a new state.
    ///
    /// In Haskell's mtl, this corresponds to the `state` function.
    ///
    /// # Arguments
    ///
    /// * `transition` - A function that takes the current state and
    ///   returns a tuple of (result, `new_state`)
    ///
    /// # Returns
    ///
    /// A computation that applies the transition function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadState, State};
    ///
    /// let computation: State<i32, String> = State::state(|s| {
    ///     (format!("was: {}", s), s + 1)
    /// });
    /// let (result, final_state) = computation.run(10);
    /// assert_eq!(result, "was: 10");
    /// assert_eq!(final_state, 11);
    /// ```
    fn state<A, F>(transition: F) -> Self::WithType<A>
    where
        F: FnOnce(S) -> (A, S) + 'static;

    /// Modifies the state using a function.
    ///
    /// Returns a computation that applies the given function to
    /// the current state and produces unit.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the state
    ///
    /// # Returns
    ///
    /// A computation that modifies the state and produces unit.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadState, State};
    ///
    /// let state: State<i32, ()> = State::modify(|x| x * 2);
    /// let (_, final_state) = state.run(21);
    /// assert_eq!(final_state, 42);
    /// ```
    fn modify<F>(modifier: F) -> Self::WithType<()>
    where
        F: FnOnce(S) -> S + 'static;

    /// Projects a value from the state.
    ///
    /// Returns a computation that applies a projection function to
    /// the current state without modifying it.
    ///
    /// # Arguments
    ///
    /// * `projection` - A function that extracts a value from the state
    ///
    /// # Returns
    ///
    /// A computation that produces the projected value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadState, State};
    ///
    /// struct Counter { value: i32, increments: u32 }
    ///
    /// let computation: State<Counter, i32> = State::gets(|c| c.value);
    /// ```
    fn gets<A, F>(projection: F) -> Self::WithType<A>
    where
        F: FnOnce(&S) -> A + 'static,
        S: 'static;
}

#[cfg(test)]
mod tests {
    // =========================================================================
    // Trait Definition Tests
    // =========================================================================

    #[test]
    fn monad_state_trait_compiles() {
        // Just verify the trait module compiles and is accessible
        // The trait requires Monad as a supertrait
        use super::MonadState;

        // This function signature proves the trait is properly defined
        fn _requires_monad_state<S, M: MonadState<S>>() {}

        // The test passes if this file compiles
    }
}
