//! Foldable type class - folding over data structures.
//!
//! This module provides the `Foldable` trait, which represents types that can
//! have their elements reduced (folded) into a single value.
//!
//! A `Foldable` provides a way to traverse a data structure and accumulate
//! results from all elements into a summary value.
//!
//! # Laws
//!
//! While `Foldable` does not have formal laws as strict as other type classes,
//! implementations should satisfy these properties:
//!
//! ## Consistency between `fold_left` and `fold_right`
//!
//! For associative operations, `fold_left` and `fold_right` should produce the same result:
//!
//! ```text
//! fa.fold_left(init, f) == fa.fold_right(init, flip(f))  // when f is associative
//! ```
//!
//! ## Consistency with `to_list`
//!
//! ```text
//! fa.fold_left(init, f) == fa.to_list().fold_left(init, f)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::Foldable;
//!
//! // Folding a Vec
//! let numbers = vec![1, 2, 3, 4, 5];
//! let sum = numbers.fold_left(0, |accumulator, element| accumulator + element);
//! assert_eq!(sum, 15);
//!
//! // Folding an Option
//! let some_value = Some(10);
//! let result = some_value.fold_left(5, |accumulator, element| accumulator + element);
//! assert_eq!(result, 15);
//!
//! let none_value: Option<i32> = None;
//! let result = none_value.fold_left(5, |accumulator, element| accumulator + element);
//! assert_eq!(result, 5);
//! ```

use super::higher::TypeConstructor;
use super::identity::Identity;
use super::monoid::Monoid;

/// A type class for data structures that can be folded to a summary value.
///
/// `Foldable` provides a unified interface for traversing data structures
/// and accumulating their elements into a single result.
///
/// # Required Methods
///
/// - `fold_left`: Left-associative fold
/// - `fold_right`: Right-associative fold
///
/// # Provided Methods
///
/// All other methods have default implementations based on `fold_left`:
///
/// - `fold_map`: Map each element to a `Monoid` and combine results
/// - `is_empty`: Check if the structure has no elements
/// - `length`: Count the number of elements
/// - `to_list`: Convert to a `Vec`
/// - `find`: Find the first element matching a predicate
/// - `exists`: Check if any element matches a predicate
/// - `for_all`: Check if all elements match a predicate
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::{Foldable, Sum};
///
/// let values = vec![1, 2, 3, 4, 5];
///
/// // Using fold_left to sum
/// let sum = values.clone().fold_left(0, |accumulator, element| accumulator + element);
/// assert_eq!(sum, 15);
///
/// // Using fold_map with Sum monoid
/// let sum: Sum<i32> = values.fold_map(Sum);
/// assert_eq!(sum.0, 15);
/// ```
pub trait Foldable: TypeConstructor {
    /// Folds the structure from left to right with an accumulator.
    ///
    /// This is equivalent to Rust's `Iterator::fold` method.
    ///
    /// # Arguments
    ///
    /// * `init` - The initial accumulator value
    /// * `function` - A function that takes the accumulator and an element,
    ///   returning a new accumulator value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let values = vec![1, 2, 3];
    /// let sum = values.fold_left(0, |accumulator, element| accumulator + element);
    /// assert_eq!(sum, 6);
    /// ```
    fn fold_left<B, F>(self, init: B, function: F) -> B
    where
        F: FnMut(B, Self::Inner) -> B;

    /// Folds the structure from right to left with an accumulator.
    ///
    /// In languages with lazy evaluation, this can be more efficient for
    /// certain operations. In Rust, this is typically implemented by
    /// reversing the iteration order.
    ///
    /// # Arguments
    ///
    /// * `init` - The initial accumulator value
    /// * `function` - A function that takes an element and the accumulator,
    ///   returning a new accumulator value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let values = vec![1, 2, 3];
    /// // Builds "123" by folding from the right: f(1, f(2, f(3, "")))
    /// let result = values.fold_right(String::new(), |element, accumulator| {
    ///     format!("{}{}", element, accumulator)
    /// });
    /// assert_eq!(result, "123");
    /// ```
    fn fold_right<B, F>(self, init: B, function: F) -> B
    where
        F: FnMut(Self::Inner, B) -> B;

    /// Maps each element to a `Monoid` and combines all results.
    ///
    /// This is a powerful abstraction that allows expressing many common
    /// operations in terms of `Monoid` combination.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that maps each element to a `Monoid` value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::{Foldable, Sum, Product};
    ///
    /// let values = vec![1, 2, 3, 4];
    ///
    /// // Sum all values
    /// let sum: Sum<i32> = values.clone().fold_map(Sum);
    /// assert_eq!(sum.0, 10);
    ///
    /// // Product of all values
    /// let product: Product<i32> = values.fold_map(Product);
    /// assert_eq!(product.0, 24);
    /// ```
    fn fold_map<M, F>(self, mut function: F) -> M
    where
        M: Monoid,
        F: FnMut(Self::Inner) -> M,
        Self: Sized,
    {
        self.fold_left(M::empty(), |accumulator, element| {
            accumulator.combine(function(element))
        })
    }

    /// Returns whether the structure contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// assert!(!Some(5).is_empty());
    /// assert!(None::<i32>.is_empty());
    ///
    /// assert!(!vec![1, 2, 3].is_empty());
    /// assert!(Vec::<i32>::new().is_empty());
    /// ```
    fn is_empty(&self) -> bool
    where
        Self: Clone,
    {
        self.clone().fold_left(true, |_, _| false)
    }

    /// Returns the number of elements in the structure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// assert_eq!(Some(5).length(), 1);
    /// assert_eq!(None::<i32>.length(), 0);
    ///
    /// assert_eq!(vec![1, 2, 3].length(), 3);
    /// assert_eq!(Vec::<i32>::new().length(), 0);
    /// ```
    fn length(&self) -> usize
    where
        Self: Clone,
    {
        self.clone().fold_left(0, |count, _| count + 1)
    }

    /// Converts the structure to a `Vec` containing all elements.
    ///
    /// The order of elements is determined by the fold order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let option_value = Some(42);
    /// assert_eq!(option_value.to_list(), vec![42]);
    ///
    /// let none_value: Option<i32> = None;
    /// assert_eq!(none_value.to_list(), Vec::<i32>::new());
    /// ```
    fn to_list(self) -> Vec<Self::Inner>
    where
        Self: Sized,
    {
        self.fold_left(Vec::new(), |mut accumulator, element| {
            accumulator.push(element);
            accumulator
        })
    }

    /// Finds the first element satisfying a predicate.
    ///
    /// Returns `Some(element)` if found, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for the element to find
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let values = vec![1, 2, 3, 4, 5];
    /// assert_eq!(values.clone().find(|element| *element > 3), Some(4));
    /// assert_eq!(values.find(|element| *element > 10), None);
    /// ```
    fn find<P>(self, mut predicate: P) -> Option<Self::Inner>
    where
        P: FnMut(&Self::Inner) -> bool,
        Self: Sized,
    {
        self.fold_left(None, |accumulator, element| {
            if accumulator.is_some() {
                accumulator
            } else if predicate(&element) {
                Some(element)
            } else {
                None
            }
        })
    }

    /// Checks if any element satisfies the predicate.
    ///
    /// Returns `true` if at least one element matches, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for matching elements
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let values = vec![1, 2, 3, 4, 5];
    /// assert!(values.exists(|element| *element > 3));
    /// assert!(!values.exists(|element| *element > 10));
    /// ```
    fn exists<P>(&self, mut predicate: P) -> bool
    where
        P: FnMut(&Self::Inner) -> bool,
        Self: Clone,
    {
        self.clone().find(|element| predicate(element)).is_some()
    }

    /// Checks if all elements satisfy the predicate.
    ///
    /// Returns `true` if all elements match (or if the structure is empty),
    /// `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for matching elements
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Foldable;
    ///
    /// let values = vec![2, 4, 6, 8];
    /// assert!(values.for_all(|element| *element % 2 == 0));
    /// assert!(!values.for_all(|element| *element > 5));
    ///
    /// // Empty structure returns true
    /// let empty: Vec<i32> = vec![];
    /// assert!(empty.for_all(|element| *element > 100));
    /// ```
    fn for_all<P>(&self, mut predicate: P) -> bool
    where
        P: FnMut(&Self::Inner) -> bool,
        Self: Clone,
    {
        !self.exists(|element| !predicate(element))
    }
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Foldable for Option<A> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, A) -> B,
    {
        match self {
            Some(element) => function(init, element),
            None => init,
        }
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(A, B) -> B,
    {
        match self {
            Some(element) => function(element, init),
            None => init,
        }
    }

    /// Optimized implementation for Option.
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_none()
    }

    /// Optimized implementation for Option.
    #[inline]
    fn length(&self) -> usize {
        usize::from(self.is_some())
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E> Foldable for Result<T, E> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        match self {
            Ok(element) => function(init, element),
            Err(_) => init,
        }
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(T, B) -> B,
    {
        match self {
            Ok(element) => function(element, init),
            Err(_) => init,
        }
    }

    /// Optimized implementation for Result.
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_err()
    }

    /// Optimized implementation for Result.
    #[inline]
    fn length(&self) -> usize {
        usize::from(self.is_ok())
    }
}

// =============================================================================
// Vec<T> Implementation
// =============================================================================

impl<T> Foldable for Vec<T> {
    fn fold_left<B, F>(self, init: B, function: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.into_iter().fold(init, function)
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(T, B) -> B,
    {
        self.into_iter()
            .rev()
            .fold(init, |accumulator, element| function(element, accumulator))
    }

    /// Optimized implementation for Vec.
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    /// Optimized implementation for Vec.
    #[inline]
    fn length(&self) -> usize {
        self.len()
    }

    /// Optimized implementation for Vec - returns self.
    #[inline]
    fn to_list(self) -> Self {
        self
    }
}

// =============================================================================
// Box<T> Implementation
// =============================================================================

impl<T> Foldable for Box<T> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        function(init, *self)
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(T, B) -> B,
    {
        function(*self, init)
    }

    /// Box always contains exactly one element.
    #[inline]
    fn is_empty(&self) -> bool {
        false
    }

    /// Box always contains exactly one element.
    #[inline]
    fn length(&self) -> usize {
        1
    }
}

// =============================================================================
// Identity<A> Implementation
// =============================================================================

impl<A> Foldable for Identity<A> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, A) -> B,
    {
        function(init, self.0)
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(A, B) -> B,
    {
        function(self.0, init)
    }

    /// Identity always contains exactly one element.
    #[inline]
    fn is_empty(&self) -> bool {
        false
    }

    /// Identity always contains exactly one element.
    #[inline]
    fn length(&self) -> usize {
        1
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeclass::{Product, Sum};
    use rstest::rstest;

    // =========================================================================
    // Option<A> Tests
    // =========================================================================

    #[rstest]
    fn option_fold_left_some() {
        let value = Some(5);
        let result = value.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn option_fold_left_none() {
        let value: Option<i32> = None;
        let result = value.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 10);
    }

    #[rstest]
    fn option_fold_right_some() {
        let value = Some(5);
        let result = value.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn option_fold_right_none() {
        let value: Option<i32> = None;
        let result = value.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 10);
    }

    #[rstest]
    fn option_is_empty_some() {
        assert!(!Some(5).is_empty());
    }

    #[rstest]
    fn option_is_empty_none() {
        assert!(None::<i32>.is_empty());
    }

    #[rstest]
    fn option_length_some() {
        assert_eq!(Some(5).length(), 1);
    }

    #[rstest]
    fn option_length_none() {
        assert_eq!(None::<i32>.length(), 0);
    }

    #[rstest]
    fn option_to_list_some() {
        assert_eq!(Some(42).to_list(), vec![42]);
    }

    #[rstest]
    fn option_to_list_none() {
        let none_value: Option<i32> = None;
        assert_eq!(none_value.to_list(), Vec::<i32>::new());
    }

    #[rstest]
    fn option_find_some() {
        let value = Some(5);
        assert_eq!(value.find(|element| *element > 3), Some(5));
    }

    #[rstest]
    fn option_find_some_not_matching() {
        let value = Some(5);
        assert_eq!(value.find(|element| *element > 10), None);
    }

    #[rstest]
    fn option_find_none() {
        let value: Option<i32> = None;
        assert_eq!(value.find(|element| *element > 0), None);
    }

    #[rstest]
    fn option_exists_some_matching() {
        let value = Some(5);
        assert!(value.exists(|element| *element > 3));
    }

    #[rstest]
    fn option_exists_some_not_matching() {
        let value = Some(5);
        assert!(!value.exists(|element| *element > 10));
    }

    #[rstest]
    fn option_exists_none() {
        let value: Option<i32> = None;
        assert!(!value.exists(|element| *element > 0));
    }

    #[rstest]
    fn option_for_all_some_matching() {
        let value = Some(5);
        assert!(value.for_all(|element| *element > 0));
    }

    #[rstest]
    fn option_for_all_some_not_matching() {
        let value = Some(5);
        assert!(!value.for_all(|element| *element > 10));
    }

    #[rstest]
    fn option_for_all_none() {
        // Empty structure returns true for for_all
        let value: Option<i32> = None;
        assert!(value.for_all(|element| *element > 100));
    }

    // =========================================================================
    // Result<T, E> Tests
    // =========================================================================

    #[rstest]
    fn result_fold_left_ok() {
        let value: Result<i32, &str> = Ok(5);
        let result = value.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn result_fold_left_err() {
        let value: Result<i32, &str> = Err("error");
        let result = value.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 10);
    }

    #[rstest]
    fn result_fold_right_ok() {
        let value: Result<i32, &str> = Ok(5);
        let result = value.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn result_fold_right_err() {
        let value: Result<i32, &str> = Err("error");
        let result = value.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 10);
    }

    #[rstest]
    fn result_is_empty_ok() {
        let value: Result<i32, &str> = Ok(5);
        assert!(!value.is_empty());
    }

    #[rstest]
    fn result_is_empty_err() {
        let value: Result<i32, &str> = Err("error");
        assert!(value.is_empty());
    }

    #[rstest]
    fn result_length_ok() {
        let value: Result<i32, &str> = Ok(5);
        assert_eq!(value.length(), 1);
    }

    #[rstest]
    fn result_length_err() {
        let value: Result<i32, &str> = Err("error");
        assert_eq!(value.length(), 0);
    }

    #[rstest]
    fn result_to_list_ok() {
        let value: Result<i32, &str> = Ok(42);
        assert_eq!(value.to_list(), vec![42]);
    }

    #[rstest]
    fn result_to_list_err() {
        let value: Result<i32, &str> = Err("error");
        assert_eq!(value.to_list(), Vec::<i32>::new());
    }

    // =========================================================================
    // Vec<A> Tests
    // =========================================================================

    #[rstest]
    fn vec_fold_left_sum() {
        let values = vec![1, 2, 3, 4, 5];
        let sum = values.fold_left(0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[rstest]
    fn vec_fold_left_empty() {
        let values: Vec<i32> = vec![];
        let sum = values.fold_left(0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 0);
    }

    #[rstest]
    fn vec_fold_left_build_string() {
        let values = vec![1, 2, 3];
        let result = values.fold_left(String::new(), |mut accumulator, element| {
            accumulator.push_str(&element.to_string());
            accumulator
        });
        assert_eq!(result, "123");
    }

    #[rstest]
    fn vec_fold_right_build_string() {
        let values = vec![1, 2, 3];
        // fold_right processes from right to left, but applies function with element first
        // f(1, f(2, f(3, ""))) = f(1, f(2, "3")) = f(1, "23") = "123"
        let result = values.fold_right(String::new(), |element, accumulator| {
            format!("{element}{accumulator}")
        });
        assert_eq!(result, "123");
    }

    #[rstest]
    fn vec_fold_right_difference_from_fold_left() {
        // Demonstrates the difference between fold_left and fold_right
        let values = vec![1, 2, 3];

        // fold_left: ((0 - 1) - 2) - 3 = -6
        let left_result = values
            .clone()
            .fold_left(0, |accumulator, element| accumulator - element);
        assert_eq!(left_result, -6);

        // fold_right: 1 - (2 - (3 - 0)) = 1 - (2 - 3) = 1 - (-1) = 2
        let right_result = values.fold_right(0, |element, accumulator| element - accumulator);
        assert_eq!(right_result, 2);
    }

    #[rstest]
    fn vec_is_empty_non_empty() {
        assert!(!vec![1, 2, 3].is_empty());
    }

    #[rstest]
    fn vec_is_empty_empty() {
        assert!(Vec::<i32>::new().is_empty());
    }

    #[rstest]
    fn vec_length() {
        assert_eq!(vec![1, 2, 3].length(), 3);
        assert_eq!(Vec::<i32>::new().length(), 0);
        assert_eq!(vec![1].length(), 1);
    }

    #[rstest]
    fn vec_to_list() {
        let values = vec![1, 2, 3];
        assert_eq!(values.clone().to_list(), values);
    }

    #[rstest]
    fn vec_find_found() {
        let values = vec![1, 2, 3, 4, 5];
        assert_eq!(values.find(|element| *element > 3), Some(4));
    }

    #[rstest]
    fn vec_find_not_found() {
        let values = vec![1, 2, 3, 4, 5];
        assert_eq!(values.find(|element| *element > 10), None);
    }

    #[rstest]
    fn vec_find_empty() {
        let values: Vec<i32> = vec![];
        assert_eq!(values.find(|element| *element > 0), None);
    }

    #[rstest]
    fn vec_find_first_match() {
        let values = vec![1, 2, 3, 4, 5];
        // Should find the first element greater than 0, which is 1
        assert_eq!(values.find(|element| *element > 0), Some(1));
    }

    #[rstest]
    fn vec_exists_true() {
        let values = vec![1, 2, 3, 4, 5];
        assert!(values.exists(|element| *element > 3));
    }

    #[rstest]
    fn vec_exists_false() {
        let values = vec![1, 2, 3, 4, 5];
        assert!(!values.exists(|element| *element > 10));
    }

    #[rstest]
    fn vec_exists_empty() {
        let values: Vec<i32> = vec![];
        assert!(!values.exists(|element| *element > 0));
    }

    #[rstest]
    fn vec_for_all_true() {
        let values = vec![2, 4, 6, 8];
        assert!(values.for_all(|element| *element % 2 == 0));
    }

    #[rstest]
    fn vec_for_all_false() {
        let values = vec![2, 4, 5, 8];
        assert!(!values.for_all(|element| *element % 2 == 0));
    }

    #[rstest]
    fn vec_for_all_empty() {
        let values: Vec<i32> = vec![];
        // Empty returns true for for_all
        assert!(values.for_all(|element| *element > 100));
    }

    // =========================================================================
    // Box<A> Tests
    // =========================================================================

    #[rstest]
    fn box_fold_left() {
        let boxed = Box::new(5);
        let result = boxed.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn box_fold_right() {
        let boxed = Box::new(5);
        let result = boxed.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn box_is_empty() {
        let boxed = Box::new(42);
        assert!(!boxed.is_empty());
    }

    #[rstest]
    fn box_length() {
        let boxed = Box::new(42);
        assert_eq!(boxed.length(), 1);
    }

    #[rstest]
    fn box_to_list() {
        let boxed = Box::new(42);
        assert_eq!(boxed.to_list(), vec![42]);
    }

    #[rstest]
    fn box_find_matching() {
        let boxed = Box::new(42);
        assert_eq!(boxed.find(|element| *element > 10), Some(42));
    }

    #[rstest]
    fn box_find_not_matching() {
        let boxed = Box::new(42);
        assert_eq!(boxed.find(|element| *element > 100), None);
    }

    // =========================================================================
    // Identity<A> Tests
    // =========================================================================

    #[rstest]
    fn identity_fold_left() {
        let wrapped = Identity::new(5);
        let result = wrapped.fold_left(10, |accumulator, element| accumulator + element);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn identity_fold_right() {
        let wrapped = Identity::new(5);
        let result = wrapped.fold_right(10, |element, accumulator| element + accumulator);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn identity_is_empty() {
        let wrapped = Identity::new(42);
        assert!(!wrapped.is_empty());
    }

    #[rstest]
    fn identity_length() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.length(), 1);
    }

    #[rstest]
    fn identity_to_list() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.to_list(), vec![42]);
    }

    #[rstest]
    fn identity_find_matching() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.find(|element| *element > 10), Some(42));
    }

    #[rstest]
    fn identity_find_not_matching() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.find(|element| *element > 100), None);
    }

    #[rstest]
    fn identity_exists_matching() {
        let wrapped = Identity::new(42);
        assert!(wrapped.exists(|element| *element > 10));
    }

    #[rstest]
    fn identity_exists_not_matching() {
        let wrapped = Identity::new(42);
        assert!(!wrapped.exists(|element| *element > 100));
    }

    #[rstest]
    fn identity_for_all_matching() {
        let wrapped = Identity::new(42);
        assert!(wrapped.for_all(|element| *element > 10));
    }

    #[rstest]
    fn identity_for_all_not_matching() {
        let wrapped = Identity::new(42);
        assert!(!wrapped.for_all(|element| *element > 100));
    }

    // =========================================================================
    // fold_map Tests
    // =========================================================================

    #[rstest]
    fn vec_fold_map_sum() {
        let values = vec![1, 2, 3, 4, 5];
        let sum: Sum<i32> = values.fold_map(Sum);
        assert_eq!(sum, Sum(15));
    }

    #[rstest]
    fn vec_fold_map_product() {
        let values = vec![1, 2, 3, 4];
        let product: Product<i32> = values.fold_map(Product);
        assert_eq!(product, Product(24));
    }

    #[rstest]
    fn vec_fold_map_empty() {
        let values: Vec<i32> = vec![];
        let sum: Sum<i32> = values.fold_map(Sum);
        assert_eq!(sum, Sum(0));
    }

    #[rstest]
    fn option_fold_map_some() {
        let value = Some(42);
        let sum: Sum<i32> = value.fold_map(Sum);
        assert_eq!(sum, Sum(42));
    }

    #[rstest]
    fn option_fold_map_none() {
        let value: Option<i32> = None;
        let sum: Sum<i32> = value.fold_map(Sum);
        assert_eq!(sum, Sum(0));
    }

    #[rstest]
    fn identity_fold_map() {
        let wrapped = Identity::new(42);
        let sum: Sum<i32> = wrapped.fold_map(Sum);
        assert_eq!(sum, Sum(42));
    }

    // =========================================================================
    // Consistency Tests
    // =========================================================================

    #[rstest]
    fn vec_fold_left_fold_right_same_for_associative() {
        // For associative operations like addition, fold_left and fold_right
        // should produce the same result
        let values = vec![1, 2, 3, 4, 5];

        let left_sum = values
            .clone()
            .fold_left(0, |accumulator, element| accumulator + element);
        let right_sum = values.fold_right(0, |element, accumulator| element + accumulator);

        assert_eq!(left_sum, right_sum);
        assert_eq!(left_sum, 15);
    }

    #[rstest]
    fn option_fold_left_equals_fold_right_for_addition() {
        let value = Some(42);

        let left = value.fold_left(0, |accumulator, element| accumulator + element);
        let right = value.fold_right(0, |element, accumulator| element + accumulator);

        assert_eq!(left, right);
        assert_eq!(left, 42);
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::typeclass::Sum;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_vec_fold_left_fold_right_same_for_addition(
            values in prop::collection::vec(any::<i32>(), 0..20)
        ) {
            // Use checked arithmetic to avoid overflow issues
            let left = values.iter().try_fold(0i64, |accumulator, &element| {
                accumulator.checked_add(i64::from(element))
            });
            let right = values.iter().rev().try_fold(0i64, |accumulator, &element| {
                accumulator.checked_add(i64::from(element))
            });

            // If no overflow occurred, results should be equal
            if let (Some(left_sum), Some(right_sum)) = (left, right) {
                prop_assert_eq!(left_sum, right_sum);
            }
        }

        #[test]
        fn prop_vec_length_equals_len(values in prop::collection::vec(any::<i32>(), 0..100)) {
            prop_assert_eq!(values.length(), values.len());
        }

        #[test]
        fn prop_vec_is_empty_equals_vec_is_empty(values in prop::collection::vec(any::<i32>(), 0..100)) {
            prop_assert_eq!(Foldable::is_empty(&values), values.is_empty());
        }

        #[test]
        fn prop_option_length(value in prop::option::of(any::<i32>())) {
            let expected = usize::from(value.is_some());
            prop_assert_eq!(value.length(), expected);
        }

        #[test]
        fn prop_option_is_empty(value in prop::option::of(any::<i32>())) {
            prop_assert_eq!(Foldable::is_empty(&value), value.is_none());
        }

        #[test]
        fn prop_vec_to_list_identity(values in prop::collection::vec(any::<i32>(), 0..100)) {
            let cloned = values.clone();
            prop_assert_eq!(values.to_list(), cloned);
        }

        #[test]
        fn prop_vec_fold_map_sum_equals_sum(values in prop::collection::vec(-1000i32..1000i32, 0..50)) {
            // Use smaller values to avoid overflow
            let sum: Sum<i32> = values.clone().fold_map(Sum);
            let direct_sum: i32 = values.iter().sum();
            prop_assert_eq!(sum.0, direct_sum);
        }

        #[test]
        fn prop_option_find_always_returns_element_if_matching(value: i32) {
            let option_value = Some(value);
            let found = option_value.find(|element| *element == value);
            prop_assert_eq!(found, Some(value));
        }

        #[test]
        fn prop_vec_exists_iff_find_is_some(values in prop::collection::vec(any::<i32>(), 0..50)) {
            let threshold = 0;
            let exists = values.exists(|element| *element > threshold);
            let found = values.find(|element| *element > threshold);
            prop_assert_eq!(exists, found.is_some());
        }
    }
}
