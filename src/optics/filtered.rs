//! Filtered optics for conditional element access.
//!
//! This module provides:
//!
//! - [`FilteredFold`]: A Fold that yields only elements satisfying a predicate
//! - [`FilteredTraversal`]: A Traversal that modifies only elements satisfying a predicate
//!
//! # Examples
//!
//! ## Filtered Fold
//!
//! ```
//! use lambars::optics::{Fold, filtered::filtered};
//!
//! let data = vec![1, 2, 3, 4, 5, 6];
//! let even_fold = filtered(|x: &i32| x % 2 == 0);
//!
//! let evens: Vec<&i32> = even_fold.to_vec(&data);
//! assert_eq!(evens, vec![&2, &4, &6]);
//! ```
//!
//! ## Filtered Traversal
//!
//! ```
//! use lambars::optics::{Traversal, filtered::filtered_traversal};
//!
//! let data = vec![1, 2, 3, 4, 5, 6];
//! let even_traversal = filtered_traversal(|x: &i32| x % 2 == 0);
//!
//! // Double only even numbers
//! let result = even_traversal.modify_all(data, |x| x * 2);
//! assert_eq!(result, vec![1, 4, 3, 8, 5, 12]);
//! ```

use std::marker::PhantomData;

use crate::optics::{Fold, Traversal};

/// A Fold that filters elements based on a predicate.
///
/// This Fold yields only elements that satisfy the predicate.
///
/// # Type Parameters
///
/// - `S`: The source type (the collection)
/// - `A`: The element type
/// - `P`: The predicate function type
///
/// # Examples
///
/// ```
/// use lambars::optics::{Fold, filtered::FilteredFold};
///
/// let fold: FilteredFold<Vec<i32>, i32, _> = FilteredFold::new(|x: &i32| *x > 0);
///
/// let data = vec![-1, 2, -3, 4, -5, 6];
/// let positives: Vec<&i32> = fold.to_vec(&data);
/// assert_eq!(positives, vec![&2, &4, &6]);
/// ```
pub struct FilteredFold<S, A, P>
where
    P: Fn(&A) -> bool,
{
    predicate: P,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, P> FilteredFold<S, A, P>
where
    P: Fn(&A) -> bool,
{
    /// Creates a new `FilteredFold` with the given predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use lambars::optics::filtered::FilteredFold;
    ///
    /// let fold: FilteredFold<Vec<i32>, i32, _> = FilteredFold::new(|x: &i32| x % 2 == 0);
    /// ```
    #[must_use]
    pub const fn new(predicate: P) -> Self {
        Self {
            predicate,
            _marker: PhantomData,
        }
    }
}

impl<S, A, P> Clone for FilteredFold<S, A, P>
where
    P: Fn(&A) -> bool + Clone,
{
    fn clone(&self) -> Self {
        Self {
            predicate: self.predicate.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, P> std::fmt::Debug for FilteredFold<S, A, P>
where
    P: Fn(&A) -> bool,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FilteredFold")
            .finish_non_exhaustive()
    }
}

impl<A, P> Fold<Vec<A>, A> for FilteredFold<Vec<A>, A, P>
where
    P: Fn(&A) -> bool + Clone + 'static,
    A: 'static,
{
    fn get_all<'a>(&self, source: &'a Vec<A>) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        let predicate = self.predicate.clone();
        Box::new(source.iter().filter(move |element| predicate(element)))
    }
}

/// Creates a `FilteredFold` for `Vec<A>` with the given predicate.
///
/// This is a convenience function for creating a filtered fold over vectors.
///
/// # Examples
///
/// ```
/// use lambars::optics::{Fold, filtered::filtered};
///
/// let even_fold = filtered(|x: &i32| x % 2 == 0);
/// let data = vec![1, 2, 3, 4, 5, 6];
///
/// let evens: Vec<&i32> = even_fold.to_vec(&data);
/// assert_eq!(evens, vec![&2, &4, &6]);
/// ```
#[must_use]
pub const fn filtered<A, P>(predicate: P) -> FilteredFold<Vec<A>, A, P>
where
    P: Fn(&A) -> bool,
{
    FilteredFold::new(predicate)
}

/// A Traversal that only modifies elements satisfying a predicate.
///
/// Elements that don't satisfy the predicate are left unchanged during
/// modification operations.
///
/// # Type Parameters
///
/// - `S`: The source type (the collection)
/// - `A`: The element type
/// - `P`: The predicate function type
///
/// # Examples
///
/// ```
/// use lambars::optics::{Traversal, filtered::FilteredTraversal};
///
/// let traversal: FilteredTraversal<Vec<i32>, i32, _> =
///     FilteredTraversal::new(|x: &i32| x % 2 == 0);
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// let doubled = traversal.modify_all(data, |x| x * 2);
/// assert_eq!(doubled, vec![1, 4, 3, 8, 5, 12]);
/// ```
pub struct FilteredTraversal<S, A, P>
where
    P: Fn(&A) -> bool,
{
    predicate: P,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, P> FilteredTraversal<S, A, P>
where
    P: Fn(&A) -> bool,
{
    /// Creates a new `FilteredTraversal` with the given predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use lambars::optics::filtered::FilteredTraversal;
    ///
    /// let traversal: FilteredTraversal<Vec<i32>, i32, _> =
    ///     FilteredTraversal::new(|x: &i32| x % 2 == 0);
    /// ```
    #[must_use]
    pub const fn new(predicate: P) -> Self {
        Self {
            predicate,
            _marker: PhantomData,
        }
    }
}

impl<S, A, P> Clone for FilteredTraversal<S, A, P>
where
    P: Fn(&A) -> bool + Clone,
{
    fn clone(&self) -> Self {
        Self {
            predicate: self.predicate.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, P> std::fmt::Debug for FilteredTraversal<S, A, P>
where
    P: Fn(&A) -> bool,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FilteredTraversal")
            .finish_non_exhaustive()
    }
}

impl<A, P> Traversal<Vec<A>, A> for FilteredTraversal<Vec<A>, A, P>
where
    P: Fn(&A) -> bool + Clone + 'static,
    A: Clone + 'static,
{
    fn get_all<'a>(&self, source: &'a Vec<A>) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        let predicate = self.predicate.clone();
        Box::new(source.iter().filter(move |element| predicate(element)))
    }

    fn get_all_owned(&self, source: Vec<A>) -> Vec<A> {
        source
            .into_iter()
            .filter(|element| (self.predicate)(element))
            .collect()
    }

    fn modify_all<F>(&self, source: Vec<A>, mut function: F) -> Vec<A>
    where
        F: FnMut(A) -> A,
    {
        source
            .into_iter()
            .map(|element| {
                if (self.predicate)(&element) {
                    function(element)
                } else {
                    element
                }
            })
            .collect()
    }
}

/// Creates a `FilteredTraversal` for `Vec<A>` with the given predicate.
///
/// This is a convenience function for creating a filtered traversal over vectors.
///
/// # Examples
///
/// ```
/// use lambars::optics::{Traversal, filtered::filtered_traversal};
///
/// let even_traversal = filtered_traversal(|x: &i32| x % 2 == 0);
/// let data = vec![1, 2, 3, 4, 5, 6];
///
/// // Double only even numbers
/// let doubled = even_traversal.modify_all(data, |x| x * 2);
/// assert_eq!(doubled, vec![1, 4, 3, 8, 5, 12]);
/// ```
#[must_use]
pub const fn filtered_traversal<A, P>(predicate: P) -> FilteredTraversal<Vec<A>, A, P>
where
    P: Fn(&A) -> bool,
{
    FilteredTraversal::new(predicate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_filtered_fold_filters_elements() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let evens: Vec<&i32> = fold.to_vec(&data);
        assert_eq!(evens, vec![&2, &4, &6]);
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], 100, true)]
    #[case(vec![], 0, true)]
    fn test_filtered_fold_empty_result(
        #[case] data: Vec<i32>,
        #[case] threshold: i32,
        #[case] expected_empty: bool,
    ) {
        let fold = filtered(move |x: &i32| *x > threshold);
        let result: Vec<&i32> = fold.to_vec(&data);
        assert_eq!(result.is_empty(), expected_empty);
    }

    #[test]
    fn test_filtered_fold_all_match() {
        let fold = filtered(|x: &i32| *x > 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result: Vec<&i32> = fold.to_vec(&data);
        assert_eq!(result, vec![&1, &2, &3, &4, &5, &6]);
    }

    #[test]
    fn test_filtered_fold_fold_operation() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let sum = fold.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 12);
    }

    #[test]
    fn test_filtered_fold_length() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.length(&data), 3);
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], 0, true)]
    #[case(vec![1, 2, 3, 4, 5, 6], 3, false)]
    fn test_filtered_fold_for_all(
        #[case] data: Vec<i32>,
        #[case] threshold: i32,
        #[case] expected: bool,
    ) {
        let fold = filtered(|x: &i32| x % 2 == 0);
        assert_eq!(fold.for_all(&data, |x| *x > threshold), expected);
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], 4, true)]
    #[case(vec![1, 2, 3, 4, 5, 6], 3, false)]
    fn test_filtered_fold_exists(
        #[case] data: Vec<i32>,
        #[case] target: i32,
        #[case] expected: bool,
    ) {
        let fold = filtered(|x: &i32| x % 2 == 0);
        assert_eq!(fold.exists(&data, |x| *x == target), expected);
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], Some(2))]
    #[case(vec![1, 3, 5], None)]
    fn test_filtered_fold_head_option(#[case] data: Vec<i32>, #[case] expected: Option<i32>) {
        let fold = filtered(|x: &i32| x % 2 == 0);
        assert_eq!(fold.head_option(&data).copied(), expected);
    }

    #[test]
    fn test_filtered_fold_last_option() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.last_option(&data), Some(&6));
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], 100, true)]
    #[case(vec![1, 2, 3, 4, 5, 6], 0, false)]
    fn test_filtered_fold_is_empty(
        #[case] data: Vec<i32>,
        #[case] threshold: i32,
        #[case] expected: bool,
    ) {
        let fold = filtered(move |x: &i32| *x > threshold);
        assert_eq!(fold.is_empty(&data), expected);
    }

    #[test]
    fn test_filtered_fold_clone() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let cloned = fold.clone();
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(cloned.length(&data), 3);
    }

    #[test]
    fn test_filtered_fold_debug() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let debug_string = format!("{fold:?}");
        assert!(debug_string.contains("FilteredFold"));
    }

    #[test]
    fn test_filtered_traversal_modify_matching_elements() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.modify_all(data, |x| x * 2);
        assert_eq!(result, vec![1, 4, 3, 8, 5, 12]);
    }

    #[test]
    fn test_filtered_traversal_no_matching_elements() {
        let traversal = filtered_traversal(|x: &i32| *x > 100);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.modify_all(data.clone(), |x| x * 2);
        assert_eq!(result, data);
    }

    #[test]
    fn test_filtered_traversal_all_matching() {
        let traversal = filtered_traversal(|x: &i32| *x > 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.modify_all(data, |x| x * 2);
        assert_eq!(result, vec![2, 4, 6, 8, 10, 12]);
    }

    #[test]
    fn test_filtered_traversal_empty_source() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data: Vec<i32> = vec![];

        let result = traversal.modify_all(data, |x| x * 2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filtered_traversal_get_all() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let evens: Vec<&i32> = traversal.get_all(&data).collect();
        assert_eq!(evens, vec![&2, &4, &6]);
    }

    #[test]
    fn test_filtered_traversal_get_all_owned() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let evens = traversal.get_all_owned(data);
        assert_eq!(evens, vec![2, 4, 6]);
    }

    #[test]
    fn test_filtered_traversal_set_all() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.set_all(data, 0);
        assert_eq!(result, vec![1, 0, 3, 0, 5, 0]);
    }

    #[test]
    fn test_filtered_traversal_fold_operation() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let sum = traversal.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 12);
    }

    #[test]
    fn test_filtered_traversal_length() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(traversal.length(&data), 3);
    }

    #[test]
    fn test_filtered_traversal_for_all() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert!(traversal.for_all(&data, |x| x % 2 == 0));
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5, 6], 4, true)]
    #[case(vec![1, 2, 3, 4, 5, 6], 3, false)]
    fn test_filtered_traversal_exists(
        #[case] data: Vec<i32>,
        #[case] target: i32,
        #[case] expected: bool,
    ) {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        assert_eq!(traversal.exists(&data, |x| *x == target), expected);
    }

    #[test]
    fn test_filtered_traversal_head_option() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(traversal.head_option(&data), Some(&2));
    }

    #[test]
    fn test_filtered_traversal_clone() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let cloned = traversal.clone();
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(cloned.length(&data), 3);
    }

    #[test]
    fn test_filtered_traversal_debug() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("FilteredTraversal"));
    }

    #[test]
    fn test_filtered_traversal_identity_law() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.modify_all(data.clone(), |x| x);
        assert_eq!(result, data);
    }

    #[test]
    fn test_filtered_traversal_composition_law() {
        let traversal = filtered_traversal(|x: &i32| *x > 0);
        let data = vec![-1, 2, 3, -4, 5, 6];

        let function_f = |x: i32| x + 10;
        let function_g = |x: i32| x * 2;

        let sequential =
            traversal.modify_all(traversal.modify_all(data.clone(), function_f), function_g);
        let composed = traversal.modify_all(data, |x| function_g(function_f(x)));

        assert_eq!(sequential, composed);
    }
}
