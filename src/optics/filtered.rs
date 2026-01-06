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
#[must_use]
pub const fn filtered<A, P>(predicate: P) -> FilteredFold<Vec<A>, A, P>
where
    P: Fn(&A) -> bool,
{
    FilteredFold::new(predicate)
}

/// A Traversal that only modifies elements satisfying a predicate.
///
/// Elements that don't satisfy the predicate are left unchanged.
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

    // =========================================================================
    // FilteredFold Tests
    // =========================================================================

    #[test]
    fn test_filtered_fold_filters_elements() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let evens: Vec<&i32> = fold.to_vec(&data);
        assert_eq!(evens, vec![&2, &4, &6]);
    }

    #[test]
    fn test_filtered_fold_empty_result() {
        let fold = filtered(|x: &i32| *x > 100);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result: Vec<&i32> = fold.to_vec(&data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filtered_fold_all_match() {
        let fold = filtered(|x: &i32| *x > 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result: Vec<&i32> = fold.to_vec(&data);
        assert_eq!(result, vec![&1, &2, &3, &4, &5, &6]);
    }

    #[test]
    fn test_filtered_fold_empty_source() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data: Vec<i32> = vec![];

        let result: Vec<&i32> = fold.to_vec(&data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filtered_fold_fold_operation() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let sum = fold.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 12); // 2 + 4 + 6
    }

    #[test]
    fn test_filtered_fold_length() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.length(&data), 3);
    }

    #[test]
    fn test_filtered_fold_for_all() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        // All filtered elements are even (by definition)
        assert!(fold.for_all(&data, |x| x % 2 == 0));
        // Not all filtered elements are greater than 3
        assert!(!fold.for_all(&data, |x| *x > 3));
    }

    #[test]
    fn test_filtered_fold_exists() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert!(fold.exists(&data, |x| *x == 4));
        assert!(!fold.exists(&data, |x| *x == 3)); // 3 is odd, not in filtered result
    }

    #[test]
    fn test_filtered_fold_head_option() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.head_option(&data), Some(&2));
    }

    #[test]
    fn test_filtered_fold_head_option_empty() {
        let fold = filtered(|x: &i32| *x > 100);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.head_option(&data), None);
    }

    #[test]
    fn test_filtered_fold_last_option() {
        let fold = filtered(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(fold.last_option(&data), Some(&6));
    }

    #[test]
    fn test_filtered_fold_is_empty() {
        let fold = filtered(|x: &i32| *x > 100);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert!(fold.is_empty(&data));

        let fold2 = filtered(|x: &i32| x % 2 == 0);
        assert!(!fold2.is_empty(&data));
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

    // =========================================================================
    // FilteredTraversal Tests
    // =========================================================================

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
        assert_eq!(sum, 12); // 2 + 4 + 6
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

    #[test]
    fn test_filtered_traversal_exists() {
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        assert!(traversal.exists(&data, |x| *x == 4));
        assert!(!traversal.exists(&data, |x| *x == 3));
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

    // =========================================================================
    // Traversal Law Tests
    // =========================================================================

    #[test]
    fn test_filtered_traversal_identity_law() {
        // Law: traversal.modify_all(source, |x| x) == source
        let traversal = filtered_traversal(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = traversal.modify_all(data.clone(), |x| x);
        assert_eq!(result, data);
    }

    #[test]
    fn test_filtered_traversal_composition_law() {
        // For FilteredTraversal, the composition law only holds when the predicate
        // is invariant under the transformations. In general, f(x) may change whether
        // the predicate holds.
        //
        // Here we test with a predicate that checks for positive numbers,
        // and functions that preserve positivity.
        let traversal = filtered_traversal(|x: &i32| *x > 0);
        let data = vec![-1, 2, 3, -4, 5, 6];

        let function_f = |x: i32| x + 10;
        let function_g = |x: i32| x * 2;

        // Apply f then g to positive elements
        let sequential =
            traversal.modify_all(traversal.modify_all(data.clone(), function_f), function_g);
        // Apply composed function to positive elements
        let composed = traversal.modify_all(data, |x| function_g(function_f(x)));

        assert_eq!(sequential, composed);
    }
}
