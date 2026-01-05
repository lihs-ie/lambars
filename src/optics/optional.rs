//! Optional optics for focusing on elements that may or may not exist.
//!
//! An Optional is an optic that provides get/set access to a value that
//! may or may not be present. It is the result of composing a Lens with a Prism.
//!
//! # Laws
//!
//! Every Optional must satisfy two laws (when the element is present):
//!
//! 1. **`GetOptionSet` Law**: Getting and setting back yields the original.
//!    ```text
//!    if optional.get_option(&source).is_some() then
//!        optional.set(source.clone(), optional.get_option(&source).unwrap().clone()) == source
//!    ```
//!
//! 2. **`SetGetOption` Law**: Setting then getting yields the set value.
//!    ```text
//!    if optional.get_option(&source).is_some() then
//!        optional.get_option(&optional.set(source, value)) == Some(&value)
//!    ```
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Lens, LensComposeExtension, Prism, Optional};
//! use lambars::{lens, prism};
//!
//! #[derive(Clone, PartialEq, Debug)]
//! enum MyOption<T> { Some(T), None }
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Container { maybe_value: MyOption<i32> }
//!
//! let container_lens = lens!(Container, maybe_value);
//! let some_prism = prism!(MyOption<i32>, Some);
//! let optional = container_lens.compose_prism(some_prism);
//!
//! let some_container = Container { maybe_value: MyOption::Some(42) };
//! assert_eq!(optional.get_option(&some_container), Some(&42));
//!
//! let none_container = Container { maybe_value: MyOption::None };
//! assert_eq!(optional.get_option(&none_container), None);
//! ```

use std::marker::PhantomData;

use super::lens::Lens;
use super::prism::Prism;

/// An Optional focuses on a value that may or may not exist.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole structure)
/// - `A`: The target type (the focused element, if present)
///
/// # Laws
///
/// 1. **`GetOptionSet` Law**: If present, getting and setting back yields the original.
/// 2. **`SetGetOption` Law**: If present, setting then getting yields the set value.
pub trait Optional<S, A> {
    /// Attempts to get a reference to the focused element.
    ///
    /// Returns `Some` if the element is present, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// A reference to the focused element if present, `None` otherwise
    fn get_option<'a>(&self, source: &'a S) -> Option<&'a A>;

    /// Sets the focused element to a new value.
    ///
    /// If the element was not present, this will still set the value
    /// (by constructing the intermediate structure).
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `value` - The new value for the focused element
    ///
    /// # Returns
    ///
    /// A new source with the focused element updated
    fn set(&self, source: S, value: A) -> S;

    /// Modifies the focused element if present.
    ///
    /// Returns `Some` with the modified source if the element is present,
    /// `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply to the focused element
    ///
    /// # Returns
    ///
    /// `Some(modified_source)` if the element is present, `None` otherwise
    fn modify_option<F>(&self, source: S, function: F) -> Option<S>
    where
        F: FnOnce(A) -> A,
        A: Clone,
    {
        // Clone the value first to avoid borrow conflict
        let maybe_value = self.get_option(&source).cloned();
        maybe_value.map(|value| {
            let new_value = function(value);
            self.set(source, new_value)
        })
    }

    /// Modifies the focused element if present, otherwise returns the original source.
    ///
    /// This is a convenience method that wraps `modify_option` and returns the
    /// original source unchanged if the element is not present.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply to the focused element
    ///
    /// # Returns
    ///
    /// The modified source if the element is present, original source otherwise
    fn modify<F>(&self, source: S, function: F) -> S
    where
        F: FnOnce(A) -> A,
        A: Clone,
        S: Clone,
    {
        self.modify_option(source.clone(), function)
            .unwrap_or(source)
    }

    /// Checks if the focused element is present.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// `true` if the element is present, `false` otherwise
    fn is_present(&self, source: &S) -> bool {
        self.get_option(source).is_some()
    }

    /// Composes this optional with a prism to focus on a nested optional element.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the prism
    /// - `P`: The type of the prism
    ///
    /// # Arguments
    ///
    /// * `other` - The prism to compose with
    ///
    /// # Returns
    ///
    /// A composed optional that focuses on the nested element
    fn compose<B, P>(self, other: P) -> ComposedOptional<Self, P, A>
    where
        Self: Sized,
        P: Prism<A, B>,
    {
        ComposedOptional::new(self, other)
    }
}

/// The result of composing a Lens with a Prism.
///
/// This struct implements the Optional trait, providing access to an element
/// that may or may not be present within a larger structure.
///
/// # Type Parameters
///
/// - `L`: The type of the lens
/// - `P`: The type of the prism
/// - `A`: The intermediate type (target of L, source of P)
///
/// # Example
///
/// ```
/// use lambars::optics::{Lens, LensComposeExtension, Prism, Optional};
/// use lambars::{lens, prism};
///
/// #[derive(Clone, PartialEq, Debug)]
/// enum MyOption<T> { Some(T), None }
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Container { maybe_value: MyOption<i32> }
///
/// let container_lens = lens!(Container, maybe_value);
/// let some_prism = prism!(MyOption<i32>, Some);
/// let optional = container_lens.compose_prism(some_prism);
///
/// let container = Container { maybe_value: MyOption::Some(42) };
/// assert_eq!(optional.get_option(&container), Some(&42));
/// ```
pub struct LensPrismComposition<L, P, A> {
    lens: L,
    prism: P,
    _marker: PhantomData<A>,
}

impl<L, P, A> LensPrismComposition<L, P, A> {
    /// Creates a new `LensPrismComposition`.
    ///
    /// # Arguments
    ///
    /// * `lens` - The lens that focuses on the intermediate structure
    /// * `prism` - The prism that focuses on the final value
    ///
    /// # Returns
    ///
    /// A new `LensPrismComposition`
    #[must_use]
    pub const fn new(lens: L, prism: P) -> Self {
        Self {
            lens,
            prism,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, L, P> Optional<S, B> for LensPrismComposition<L, P, A>
where
    L: Lens<S, A>,
    P: Prism<A, B>,
    A: Clone + 'static,
{
    fn get_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        let intermediate = self.lens.get(source);
        self.prism.preview(intermediate)
    }

    fn set(&self, source: S, value: B) -> S {
        let new_intermediate = self.prism.review(value);
        self.lens.set(source, new_intermediate)
    }
}

impl<L: Clone, P: Clone, A> Clone for LensPrismComposition<L, P, A> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            prism: self.prism.clone(),
            _marker: PhantomData,
        }
    }
}

impl<L: std::fmt::Debug, P: std::fmt::Debug, A> std::fmt::Debug for LensPrismComposition<L, P, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LensPrismComposition")
            .field("lens", &self.lens)
            .field("prism", &self.prism)
            .finish()
    }
}

/// The result of composing two Optionals.
///
/// This struct implements the Optional trait, providing access to a nested
/// optional element.
///
/// # Type Parameters
///
/// - `O1`: The type of the first optional
/// - `O2`: The type of the second optional (or prism)
/// - `A`: The intermediate type
pub struct ComposedOptional<O1, O2, A> {
    first: O1,
    second: O2,
    _marker: PhantomData<A>,
}

impl<O1, O2, A> ComposedOptional<O1, O2, A> {
    /// Creates a new `ComposedOptional`.
    ///
    /// # Arguments
    ///
    /// * `first` - The first optional
    /// * `second` - The second optional or prism
    ///
    /// # Returns
    ///
    /// A new `ComposedOptional`
    #[must_use]
    pub const fn new(first: O1, second: O2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, O1, O2> Optional<S, B> for ComposedOptional<O1, O2, A>
where
    O1: Optional<S, A>,
    O2: Prism<A, B>,
    A: Clone + 'static,
{
    fn get_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        self.first
            .get_option(source)
            .and_then(|intermediate| self.second.preview(intermediate))
    }

    fn set(&self, source: S, value: B) -> S {
        // Get the intermediate value, modify it, and set it back
        if let Some(intermediate) = self.first.get_option(&source).cloned() {
            // Create new intermediate with the value set
            let _ = intermediate; // We need to review to create the intermediate
        }
        // Use review to create the new intermediate value
        let new_intermediate = self.second.review(value);
        self.first.set(source, new_intermediate)
    }
}

impl<O1: Clone, O2: Clone, A> Clone for ComposedOptional<O1, O2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<O1: std::fmt::Debug, O2: std::fmt::Debug, A> std::fmt::Debug for ComposedOptional<O1, O2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedOptional")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

// Extend Lens trait to support compose_prism
impl<S, A, L> LensComposeExtension<S, A> for L where L: Lens<S, A> {}

/// Extension trait for Lens to compose with Prism.
pub trait LensComposeExtension<S, A>: Lens<S, A> {
    /// Composes this lens with a prism to create an Optional.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the prism
    /// - `P`: The type of the prism
    ///
    /// # Arguments
    ///
    /// * `prism` - The prism to compose with
    ///
    /// # Returns
    ///
    /// An Optional that focuses on the prism's target within this lens's target
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Lens, LensComposeExtension, Prism, Optional};
    /// use lambars::{lens, prism};
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum MyOption<T> { Some(T), None }
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Container { maybe_value: MyOption<i32> }
    ///
    /// let container_lens = lens!(Container, maybe_value);
    /// let some_prism = prism!(MyOption<i32>, Some);
    /// let optional = container_lens.compose_prism(some_prism);
    ///
    /// let container = Container { maybe_value: MyOption::Some(42) };
    /// assert_eq!(optional.get_option(&container), Some(&42));
    /// ```
    fn compose_prism<B, P>(self, prism: P) -> LensPrismComposition<Self, P, A>
    where
        Self: Sized,
        P: Prism<A, B>,
    {
        LensPrismComposition::new(self, prism)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lens;
    use crate::prism;

    #[derive(Clone, PartialEq, Debug)]
    enum MyOption<T> {
        Some(T),
        None,
    }

    #[derive(Clone, PartialEq, Debug)]
    struct Container {
        maybe_value: MyOption<i32>,
    }

    #[test]
    fn test_lens_prism_composition_get_option_some() {
        let container_lens = lens!(Container, maybe_value);
        let some_prism = prism!(MyOption<i32>, Some);
        let optional = container_lens.compose_prism(some_prism);

        let container = Container {
            maybe_value: MyOption::Some(42),
        };
        assert_eq!(optional.get_option(&container), Some(&42));
    }

    #[test]
    fn test_lens_prism_composition_get_option_none() {
        let container_lens = lens!(Container, maybe_value);
        let some_prism = prism!(MyOption<i32>, Some);
        let optional = container_lens.compose_prism(some_prism);

        let container = Container {
            maybe_value: MyOption::None,
        };
        assert_eq!(optional.get_option(&container), None);
    }

    #[test]
    fn test_lens_prism_composition_set() {
        let container_lens = lens!(Container, maybe_value);
        let some_prism = prism!(MyOption<i32>, Some);
        let optional = container_lens.compose_prism(some_prism);

        let container = Container {
            maybe_value: MyOption::Some(42),
        };
        let updated = optional.set(container, 100);
        assert_eq!(updated.maybe_value, MyOption::Some(100));
    }

    #[test]
    fn test_lens_prism_composition_is_present() {
        let container_lens = lens!(Container, maybe_value);
        let some_prism = prism!(MyOption<i32>, Some);
        let optional = container_lens.compose_prism(some_prism);

        let some_container = Container {
            maybe_value: MyOption::Some(42),
        };
        assert!(optional.is_present(&some_container));

        let none_container = Container {
            maybe_value: MyOption::None,
        };
        assert!(!optional.is_present(&none_container));
    }
}
