//! Fold optics for read-only access to multiple elements.
//!
//! A Fold is a read-only optic that can focus on zero or more elements.
//! Unlike Traversal, Fold does not support modification operations.
//! It is useful for read-only access patterns like filtering.
//!
//! # Relationship to Traversal
//!
//! Every Traversal can be used as a Fold because a Traversal
//! provides all the read operations that a Fold needs, plus
//! modification operations.
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Fold, FunctionFold};
//!
//! let fold: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
//!     |vec: &Vec<i32>| Box::new(vec.iter())
//! );
//!
//! let data = vec![1, 2, 3, 4, 5];
//! let sum: i32 = fold.get_all(&data).sum();
//! assert_eq!(sum, 15);
//! ```

use std::marker::PhantomData;

/// A Fold is a read-only optic that can focus on zero or more elements.
///
/// Unlike Traversal, Fold does not support modification operations.
/// It is useful for read-only access patterns like filtering.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole structure)
/// - `A`: The target type (the focused elements)
///
/// # Laws
///
/// Fold has no specific laws beyond consistency - calling `get_all` on the
/// same source should always return the same sequence of elements.
pub trait Fold<S, A> {
    /// Returns an iterator over references to all focused elements.
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>;

    /// Folds over all focused elements.
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Fold, FunctionFold};
    ///
    /// let fold: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
    ///     |vec: &Vec<i32>| Box::new(vec.iter())
    /// );
    ///
    /// let data = vec![1, 2, 3, 4, 5];
    /// let sum = fold.fold(&data, 0, |accumulator, element| accumulator + element);
    /// assert_eq!(sum, 15);
    /// ```
    fn fold<B, F>(&self, source: &S, initial: B, mut function: F) -> B
    where
        F: FnMut(B, &A) -> B,
    {
        self.get_all(source).fold(initial, |accumulator, element| {
            function(accumulator, element)
        })
    }

    /// Returns the number of focused elements.
    fn length(&self, source: &S) -> usize {
        self.get_all(source).count()
    }

    /// Tests if all focused elements satisfy a predicate.
    ///
    /// Returns `true` if there are no focused elements (vacuously true).
    fn for_all<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&A) -> bool,
    {
        self.get_all(source).all(predicate)
    }

    /// Tests if any focused element satisfies a predicate.
    ///
    /// Returns `false` if there are no focused elements.
    fn exists<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&A) -> bool,
    {
        self.get_all(source).any(predicate)
    }

    /// Returns a reference to the first focused element, if any.
    fn head_option<'a>(&self, source: &'a S) -> Option<&'a A> {
        self.get_all(source).next()
    }

    /// Returns a reference to the last focused element, if any.
    fn last_option<'a>(&self, source: &'a S) -> Option<&'a A> {
        self.get_all(source).last()
    }

    /// Tests if there are no focused elements.
    fn is_empty(&self, source: &S) -> bool {
        self.get_all(source).next().is_none()
    }

    /// Collects all focused elements into a Vec.
    fn to_vec<'a>(&self, source: &'a S) -> Vec<&'a A> {
        self.get_all(source).collect()
    }

    /// Composes this fold with another fold to create a nested fold.
    ///
    /// The resulting fold focuses on elements that can be reached by first
    /// applying this fold, then applying the second fold to each intermediate result.
    ///
    /// # Examples
    ///
    /// ```
    /// use lambars::optics::{Fold, FunctionFold};
    ///
    /// let outer: FunctionFold<Vec<Vec<i32>>, Vec<i32>, _> = FunctionFold::new(
    ///     |vec: &Vec<Vec<i32>>| Box::new(vec.iter())
    /// );
    /// let inner: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
    ///     |vec: &Vec<i32>| Box::new(vec.iter())
    /// );
    ///
    /// let composed = outer.compose(inner);
    /// let data = vec![vec![1, 2], vec![3, 4]];
    /// assert_eq!(composed.length(&data), 4);
    /// ```
    fn compose<B, F2>(self, other: F2) -> ComposedFold<Self, F2, A>
    where
        Self: Sized,
        F2: Fold<A, B>,
    {
        ComposedFold::new(self, other)
    }
}

/// A Fold implemented using a function.
///
/// This struct allows creating a Fold from a closure that returns an iterator
/// over references to the focused elements.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole structure)
/// - `A`: The target type (the focused elements)
/// - `G`: The getter function type
///
/// # Examples
///
/// ```
/// use lambars::optics::{Fold, FunctionFold};
///
/// // Create a fold that focuses on all elements in a Vec
/// let fold: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
///     |vec: &Vec<i32>| Box::new(vec.iter())
/// );
///
/// let data = vec![1, 2, 3, 4, 5];
/// let sum: i32 = fold.get_all(&data).sum();
/// assert_eq!(sum, 15);
/// ```
pub struct FunctionFold<S, A, G>
where
    G: for<'a> Fn(&'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>,
{
    get_all_function: G,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, G> FunctionFold<S, A, G>
where
    G: for<'a> Fn(&'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>,
{
    /// Creates a new `FunctionFold` from a getter function.
    ///
    /// The getter function should return a boxed iterator over references
    /// to all focused elements in the source structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use lambars::optics::{Fold, FunctionFold};
    ///
    /// let fold: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
    ///     |vec: &Vec<i32>| Box::new(vec.iter())
    /// );
    /// ```
    #[must_use]
    pub const fn new(get_all_function: G) -> Self {
        Self {
            get_all_function,
            _marker: PhantomData,
        }
    }
}

impl<S, A, G> Fold<S, A> for FunctionFold<S, A, G>
where
    G: for<'a> Fn(&'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        (self.get_all_function)(source)
    }
}

impl<S, A, G> Clone for FunctionFold<S, A, G>
where
    G: for<'a> Fn(&'a S) -> Box<dyn Iterator<Item = &'a A> + 'a> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            get_all_function: self.get_all_function.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, G> std::fmt::Debug for FunctionFold<S, A, G>
where
    G: for<'a> Fn(&'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FunctionFold")
            .finish_non_exhaustive()
    }
}

/// A Fold composed of two Folds.
///
/// This allows focusing on nested elements by composing a fold that focuses
/// on an intermediate structure with a fold that focuses on elements within
/// that structure.
///
/// # Type Parameters
///
/// - `F1`: The first fold type (outer)
/// - `F2`: The second fold type (inner)
/// - `A`: The intermediate type
///
/// # Examples
///
/// ```
/// use lambars::optics::{Fold, FunctionFold};
///
/// // Create folds for nested vectors
/// let outer: FunctionFold<Vec<Vec<i32>>, Vec<i32>, _> = FunctionFold::new(
///     |vec: &Vec<Vec<i32>>| Box::new(vec.iter())
/// );
/// let inner: FunctionFold<Vec<i32>, i32, _> = FunctionFold::new(
///     |vec: &Vec<i32>| Box::new(vec.iter())
/// );
///
/// // Compose to focus on all inner elements
/// let composed = outer.compose(inner);
///
/// let data = vec![vec![1, 2], vec![3, 4, 5]];
/// let sum: i32 = composed.fold(&data, 0, |acc, x| acc + x);
/// assert_eq!(sum, 15);
/// ```
pub struct ComposedFold<F1, F2, A> {
    first: F1,
    second: F2,
    _marker: PhantomData<A>,
}

impl<F1, F2, A> ComposedFold<F1, F2, A> {
    /// Creates a new composed fold from two folds.
    #[must_use]
    pub const fn new(first: F1, second: F2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, F1, F2> Fold<S, B> for ComposedFold<F1, F2, A>
where
    F1: Fold<S, A>,
    F2: Fold<A, B> + Clone + 'static,
    A: 'static,
    B: 'static,
    S: 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a B> + 'a> {
        let second = self.second.clone();
        Box::new(
            self.first
                .get_all(source)
                .flat_map(move |intermediate| second.get_all(intermediate)),
        )
    }
}

impl<F1: Clone, F2: Clone, A> Clone for ComposedFold<F1, F2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<F1: std::fmt::Debug, F2: std::fmt::Debug, A> std::fmt::Debug for ComposedFold<F1, F2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedFold")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::type_complexity)]
    fn vec_fold<T: 'static>() -> FunctionFold<
        Vec<T>,
        T,
        impl for<'a> Fn(&'a Vec<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> + Clone,
    > {
        FunctionFold::new(|vec: &Vec<T>| Box::new(vec.iter()))
    }

    #[test]
    fn test_get_all() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3];
        let result: Vec<&i32> = fold.get_all(&data).collect();
        assert_eq!(result, vec![&1, &2, &3]);
    }

    #[test]
    fn test_fold_operation() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3, 4, 5];
        let sum = fold.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_length() {
        let fold = vec_fold::<i32>();

        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(fold.length(&data), 5);

        let empty: Vec<i32> = vec![];
        assert_eq!(fold.length(&empty), 0);
    }

    #[test]
    fn test_for_all_true() {
        let fold = vec_fold::<i32>();
        let positive = vec![1, 2, 3, 4, 5];
        assert!(fold.for_all(&positive, |x| *x > 0));
    }

    #[test]
    fn test_for_all_false() {
        let fold = vec_fold::<i32>();
        let mixed = vec![1, -2, 3];
        assert!(!fold.for_all(&mixed, |x| *x > 0));
    }

    #[test]
    fn test_for_all_empty_is_true() {
        let fold = vec_fold::<i32>();
        let empty: Vec<i32> = vec![];
        assert!(fold.for_all(&empty, |x| *x > 0));
    }

    #[test]
    fn test_exists_true() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3, 4, 5];
        assert!(fold.exists(&data, |x| *x == 3));
    }

    #[test]
    fn test_exists_false() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3, 4, 5];
        assert!(!fold.exists(&data, |x| *x == 10));
    }

    #[test]
    fn test_exists_empty_is_false() {
        let fold = vec_fold::<i32>();
        let empty: Vec<i32> = vec![];
        assert!(!fold.exists(&empty, |x| *x == 1));
    }

    #[test]
    fn test_head_option_non_empty() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3];
        assert_eq!(fold.head_option(&data), Some(&1));
    }

    #[test]
    fn test_head_option_empty() {
        let fold = vec_fold::<i32>();
        let empty: Vec<i32> = vec![];
        assert_eq!(fold.head_option(&empty), None);
    }

    #[test]
    fn test_last_option_non_empty() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3];
        assert_eq!(fold.last_option(&data), Some(&3));
    }

    #[test]
    fn test_last_option_empty() {
        let fold = vec_fold::<i32>();
        let empty: Vec<i32> = vec![];
        assert_eq!(fold.last_option(&empty), None);
    }

    #[test]
    fn test_is_empty_true() {
        let fold = vec_fold::<i32>();
        let empty: Vec<i32> = vec![];
        assert!(fold.is_empty(&empty));
    }

    #[test]
    fn test_is_empty_false() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3];
        assert!(!fold.is_empty(&data));
    }

    #[test]
    fn test_to_vec() {
        let fold = vec_fold::<i32>();
        let data = vec![1, 2, 3];
        let collected = fold.to_vec(&data);
        assert_eq!(collected, vec![&1, &2, &3]);
    }

    #[test]
    fn test_compose() {
        let outer = vec_fold::<Vec<i32>>();
        let inner = vec_fold::<i32>();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_clone() {
        let fold = vec_fold::<i32>();
        let cloned = fold.clone();

        let data = vec![1, 2, 3];
        assert_eq!(fold.length(&data), cloned.length(&data));
    }

    #[test]
    fn test_debug() {
        let fold = vec_fold::<i32>();
        let debug_string = format!("{fold:?}");
        assert!(debug_string.contains("FunctionFold"));
    }

    #[test]
    fn test_composed_fold_debug() {
        let outer = vec_fold::<Vec<i32>>();
        let inner = vec_fold::<i32>();
        let composed = outer.compose(inner);

        let debug_string = format!("{composed:?}");
        assert!(debug_string.contains("ComposedFold"));
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_composed_fold_clone() {
        let outer = vec_fold::<Vec<i32>>();
        let inner = vec_fold::<i32>();
        let composed = outer.compose(inner);
        let cloned = composed.clone();

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let sum: i32 = cloned.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }
}
