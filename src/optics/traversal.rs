//! Traversal optics for focusing on multiple elements.
//!
//! A Traversal is an optic that provides access to zero or more elements within a structure.
//! It generalizes both Lens (which focuses on exactly one element) and Prism (which focuses
//! on zero or one element).
//!
//! # Laws
//!
//! Every Traversal must satisfy two laws:
//!
//! 1. **Modify Identity Law**: Applying the identity function yields the original.
//!    ```text
//!    traversal.modify_all(source, |x| x) == source
//!    ```
//!
//! 2. **Modify Composition Law**: Consecutive `modify_all` calls equal a single composed call.
//!    ```text
//!    traversal.modify_all(traversal.modify_all(source, f), g) == traversal.modify_all(source, |x| g(f(x)))
//!    ```
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Traversal, VecTraversal};
//!
//! let traversal: VecTraversal<i32> = VecTraversal::new();
//! let numbers = vec![1, 2, 3, 4, 5];
//!
//! // Get all elements
//! let sum: i32 = traversal.get_all(&numbers).sum();
//! assert_eq!(sum, 15);
//!
//! // Modify all elements
//! let doubled = traversal.modify_all(numbers.clone(), |x| x * 2);
//! assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
//!
//! // Set all elements to the same value
//! let all_zeros = traversal.set_all(numbers, 0);
//! assert_eq!(all_zeros, vec![0, 0, 0, 0, 0]);
//! ```

use std::marker::PhantomData;

/// A Traversal focuses on zero or more elements within a structure.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole structure)
/// - `A`: The target type (the focused elements)
///
/// # Laws
///
/// 1. **Modify Identity Law**: `traversal.modify_all(source, |x| x) == source`
/// 2. **Modify Composition Law**: `traversal.modify_all(traversal.modify_all(source, f), g) == traversal.modify_all(source, |x| g(f(x)))`
pub trait Traversal<S, A> {
    /// Returns an iterator over references to all focused elements.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// An iterator yielding references to all focused elements
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a A> + 'a>;

    /// Returns a vector of all focused elements, taking ownership.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    ///
    /// # Returns
    ///
    /// A vector containing all focused elements
    fn get_all_owned(&self, source: S) -> Vec<A>;

    /// Modifies all focused elements by applying a function.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply to each focused element
    ///
    /// # Returns
    ///
    /// A new source with all focused elements modified
    fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(A) -> A;

    /// Sets all focused elements to the same value.
    ///
    /// This is equivalent to `modify_all(source, |_| value.clone())`.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `value` - The value to set all focused elements to
    ///
    /// # Returns
    ///
    /// A new source with all focused elements set to the given value
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// let numbers = vec![1, 2, 3, 4, 5];
    /// let all_zeros = traversal.set_all(numbers, 0);
    /// assert_eq!(all_zeros, vec![0, 0, 0, 0, 0]);
    /// ```
    fn set_all(&self, source: S, value: A) -> S
    where
        A: Clone,
    {
        self.modify_all(source, |_| value.clone())
    }

    /// Folds over all focused elements.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    /// * `initial` - The initial accumulator value
    /// * `function` - The folding function
    ///
    /// # Returns
    ///
    /// The result of folding over all focused elements
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// let numbers = vec![1, 2, 3, 4, 5];
    /// let sum = traversal.fold(&numbers, 0, |accumulator, element| accumulator + element);
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
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// The number of focused elements
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// assert_eq!(traversal.length(&vec![1, 2, 3, 4, 5]), 5);
    /// assert_eq!(traversal.length(&Vec::<i32>::new()), 0);
    /// ```
    fn length(&self, source: &S) -> usize {
        self.get_all(source).count()
    }

    /// Tests if all focused elements satisfy a predicate.
    ///
    /// Returns `true` if there are no focused elements (vacuously true).
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    /// * `predicate` - The predicate to test
    ///
    /// # Returns
    ///
    /// `true` if all focused elements satisfy the predicate
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// let positive = vec![1, 2, 3, 4, 5];
    /// assert!(traversal.for_all(&positive, |x| *x > 0));
    ///
    /// let mixed = vec![1, -2, 3];
    /// assert!(!traversal.for_all(&mixed, |x| *x > 0));
    /// ```
    fn for_all<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&A) -> bool,
    {
        self.get_all(source).all(predicate)
    }

    /// Tests if any focused element satisfies a predicate.
    ///
    /// Returns `false` if there are no focused elements.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    /// * `predicate` - The predicate to test
    ///
    /// # Returns
    ///
    /// `true` if any focused element satisfies the predicate
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// let numbers = vec![1, 2, 3, 4, 5];
    /// assert!(traversal.exists(&numbers, |x| *x == 3));
    /// assert!(!traversal.exists(&numbers, |x| *x == 10));
    /// ```
    fn exists<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&A) -> bool,
    {
        self.get_all(source).any(predicate)
    }

    /// Returns a reference to the first focused element, if any.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// The first focused element, or `None` if there are none
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let traversal: VecTraversal<i32> = VecTraversal::new();
    /// let numbers = vec![1, 2, 3];
    /// assert_eq!(traversal.head_option(&numbers), Some(&1));
    ///
    /// let empty: Vec<i32> = vec![];
    /// assert_eq!(traversal.head_option(&empty), None);
    /// ```
    fn head_option<'a>(&self, source: &'a S) -> Option<&'a A> {
        self.get_all(source).next()
    }

    /// Composes this traversal with another traversal.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the other traversal
    /// - `T`: The type of the other traversal
    ///
    /// # Arguments
    ///
    /// * `other` - The traversal to compose with
    ///
    /// # Returns
    ///
    /// A composed traversal that focuses on nested elements
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Traversal, VecTraversal};
    ///
    /// let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
    /// let inner: VecTraversal<i32> = VecTraversal::new();
    /// let composed = outer.compose(inner);
    ///
    /// let data = vec![vec![1, 2], vec![3, 4, 5]];
    /// let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
    /// assert_eq!(sum, 15);
    /// ```
    fn compose<B, T>(self, other: T) -> ComposedTraversal<Self, T, A>
    where
        Self: Sized,
        T: Traversal<A, B>,
    {
        ComposedTraversal::new(self, other)
    }
}

// =============================================================================
// VecTraversal - Traversal for Vec<A>
// =============================================================================

/// A Traversal that focuses on all elements of a `Vec`.
///
/// # Type Parameters
///
/// - `A`: The element type of the vector
///
/// # Example
///
/// ```
/// use lambars::optics::{Traversal, VecTraversal};
///
/// let traversal: VecTraversal<i32> = VecTraversal::new();
/// let numbers = vec![1, 2, 3, 4, 5];
///
/// let doubled = traversal.modify_all(numbers, |x| x * 2);
/// assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
/// ```
pub struct VecTraversal<A> {
    _marker: PhantomData<A>,
}

impl<A> VecTraversal<A> {
    /// Creates a new `VecTraversal`.
    ///
    /// # Returns
    ///
    /// A new `VecTraversal`
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<A> Default for VecTraversal<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> Clone for VecTraversal<A> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<A> std::fmt::Debug for VecTraversal<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("VecTraversal").finish()
    }
}

impl<A: 'static> Traversal<Vec<A>, A> for VecTraversal<A> {
    fn get_all<'a>(&self, source: &'a Vec<A>) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Vec<A>) -> Vec<A> {
        source
    }

    fn modify_all<F>(&self, source: Vec<A>, function: F) -> Vec<A>
    where
        F: FnMut(A) -> A,
    {
        source.into_iter().map(function).collect()
    }
}

// =============================================================================
// OptionTraversal - Traversal for Option<A>
// =============================================================================

/// A Traversal that focuses on the element of an `Option` (zero or one element).
///
/// # Type Parameters
///
/// - `A`: The element type of the Option
///
/// # Example
///
/// ```
/// use lambars::optics::{Traversal, OptionTraversal};
///
/// let traversal: OptionTraversal<i32> = OptionTraversal::new();
///
/// let some_value = Some(42);
/// let doubled = traversal.modify_all(some_value, |x| x * 2);
/// assert_eq!(doubled, Some(84));
///
/// let none_value: Option<i32> = None;
/// let result = traversal.modify_all(none_value, |x| x * 2);
/// assert_eq!(result, None);
/// ```
pub struct OptionTraversal<A> {
    _marker: PhantomData<A>,
}

impl<A> OptionTraversal<A> {
    /// Creates a new `OptionTraversal`.
    ///
    /// # Returns
    ///
    /// A new `OptionTraversal`
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<A> Default for OptionTraversal<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> Clone for OptionTraversal<A> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<A> std::fmt::Debug for OptionTraversal<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("OptionTraversal").finish()
    }
}

impl<A: 'static> Traversal<Option<A>, A> for OptionTraversal<A> {
    fn get_all<'a>(&self, source: &'a Option<A>) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Option<A>) -> Vec<A> {
        source.into_iter().collect()
    }

    fn modify_all<F>(&self, source: Option<A>, function: F) -> Option<A>
    where
        F: FnMut(A) -> A,
    {
        source.map(function)
    }
}

// =============================================================================
// ResultTraversal - Traversal for Result<A, E>
// =============================================================================

/// A Traversal that focuses on the `Ok` value of a `Result` (zero or one element).
///
/// # Type Parameters
///
/// - `A`: The success value type
/// - `E`: The error type
///
/// # Example
///
/// ```
/// use lambars::optics::{Traversal, ResultTraversal};
///
/// let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
///
/// let ok_value: Result<i32, String> = Ok(42);
/// let doubled = traversal.modify_all(ok_value, |x| x * 2);
/// assert_eq!(doubled, Ok(84));
///
/// let err_value: Result<i32, String> = Err("error".to_string());
/// let result = traversal.modify_all(err_value, |x| x * 2);
/// assert_eq!(result, Err("error".to_string()));
/// ```
pub struct ResultTraversal<A, E> {
    _marker: PhantomData<(A, E)>,
}

impl<A, E> ResultTraversal<A, E> {
    /// Creates a new `ResultTraversal`.
    ///
    /// # Returns
    ///
    /// A new `ResultTraversal`
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<A, E> Default for ResultTraversal<A, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, E> Clone for ResultTraversal<A, E> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<A, E> std::fmt::Debug for ResultTraversal<A, E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ResultTraversal").finish()
    }
}

impl<A: 'static, E: 'static> Traversal<Result<A, E>, A> for ResultTraversal<A, E> {
    fn get_all<'a>(&self, source: &'a Result<A, E>) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Result<A, E>) -> Vec<A> {
        source.into_iter().collect()
    }

    fn modify_all<F>(&self, source: Result<A, E>, function: F) -> Result<A, E>
    where
        F: FnMut(A) -> A,
    {
        source.map(function)
    }
}

// =============================================================================
// ComposedTraversal - Composition of two Traversals
// =============================================================================

/// A Traversal composed of two Traversals.
///
/// This allows focusing on nested elements by composing a traversal that focuses
/// on an intermediate structure with a traversal that focuses on elements within
/// that structure.
///
/// # Type Parameters
///
/// - `T1`: The type of the outer traversal
/// - `T2`: The type of the inner traversal
/// - `A`: The intermediate type (target of T1, source of T2)
///
/// # Example
///
/// ```
/// use lambars::optics::{Traversal, VecTraversal};
///
/// let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
/// let inner: VecTraversal<i32> = VecTraversal::new();
/// let composed = outer.compose(inner);
///
/// let data = vec![vec![1, 2], vec![3, 4, 5]];
/// let doubled = composed.modify_all(data, |x| x * 2);
/// assert_eq!(doubled, vec![vec![2, 4], vec![6, 8, 10]]);
/// ```
pub struct ComposedTraversal<T1, T2, A> {
    first: T1,
    second: T2,
    _marker: PhantomData<A>,
}

impl<T1, T2, A> ComposedTraversal<T1, T2, A> {
    /// Creates a new composed traversal.
    ///
    /// # Arguments
    ///
    /// * `first` - The outer traversal
    /// * `second` - The inner traversal
    ///
    /// # Returns
    ///
    /// A new `ComposedTraversal`
    #[must_use]
    pub const fn new(first: T1, second: T2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<T1: Clone, T2: Clone, A> Clone for ComposedTraversal<T1, T2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T1: std::fmt::Debug, T2: std::fmt::Debug, A> std::fmt::Debug for ComposedTraversal<T1, T2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedTraversal")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<S, A, B, T1, T2> Traversal<S, B> for ComposedTraversal<T1, T2, A>
where
    T1: Traversal<S, A>,
    T2: Traversal<A, B> + Clone + 'static,
    A: Clone + 'static,
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

    fn get_all_owned(&self, source: S) -> Vec<B> {
        self.first
            .get_all_owned(source)
            .into_iter()
            .flat_map(|intermediate| self.second.get_all_owned(intermediate))
            .collect()
    }

    fn modify_all<F>(&self, source: S, mut function: F) -> S
    where
        F: FnMut(B) -> B,
    {
        let second = &self.second;
        // We need to use a cell to allow the mutable closure to be called multiple times
        let function_cell = std::cell::RefCell::new(&mut function);

        self.first.modify_all(source, |intermediate| {
            second.modify_all(intermediate, |element| {
                (*function_cell.borrow_mut())(element)
            })
        })
    }
}

// =============================================================================
// LensAsTraversal - Traversal trait implementation for Lens
// =============================================================================

use super::Lens;
use super::lens::LensAsTraversal;

impl<L, S, A> Traversal<S, A> for LensAsTraversal<L, S, A>
where
    L: Lens<S, A>,
    A: Clone + 'static,
    S: 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        Box::new(std::iter::once(self.lens.get(source)))
    }

    fn get_all_owned(&self, source: S) -> Vec<A> {
        vec![self.lens.get(&source).clone()]
    }

    fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(A) -> A,
    {
        self.lens.modify(source, function)
    }

    fn length(&self, _source: &S) -> usize {
        1
    }
}

// =============================================================================
// PrismAsTraversal - Traversal trait implementation for Prism
// =============================================================================

use super::Prism;
use super::prism::PrismAsTraversal;

impl<P, S, A> Traversal<S, A> for PrismAsTraversal<P, S, A>
where
    P: Prism<S, A>,
    A: Clone + 'static,
    S: Clone + 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a A> + 'a> {
        Box::new(self.prism.preview(source).into_iter())
    }

    fn get_all_owned(&self, source: S) -> Vec<A> {
        self.prism.preview(&source).cloned().into_iter().collect()
    }

    fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(A) -> A,
    {
        self.prism.modify_or_identity(source, function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_traversal_basic() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3, 4, 5];

        let sum: i32 = traversal.get_all(&numbers).sum();
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_vec_traversal_modify() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3];

        let doubled = traversal.modify_all(numbers, |x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[test]
    fn test_option_traversal_some() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let value = Some(42);

        let collected: Vec<&i32> = traversal.get_all(&value).collect();
        assert_eq!(collected, vec![&42]);
    }

    #[test]
    fn test_option_traversal_none() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let value: Option<i32> = None;

        let collected: Vec<&i32> = traversal.get_all(&value).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_result_traversal_ok() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let value: Result<i32, String> = Ok(42);

        let collected: Vec<&i32> = traversal.get_all(&value).collect();
        assert_eq!(collected, vec![&42]);
    }

    #[test]
    fn test_result_traversal_err() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let value: Result<i32, String> = Err("error".to_string());

        let collected: Vec<&i32> = traversal.get_all(&value).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_composed_traversal() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_traversal_fold() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3, 4, 5];

        let sum = traversal.fold(&numbers, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_traversal_length() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        assert_eq!(traversal.length(&vec![1, 2, 3, 4, 5]), 5);
        assert_eq!(traversal.length(&Vec::<i32>::new()), 0);
    }

    #[test]
    fn test_traversal_for_all() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        assert!(traversal.for_all(&vec![1, 2, 3, 4, 5], |x| *x > 0));
        assert!(!traversal.for_all(&vec![1, -2, 3], |x| *x > 0));
    }

    #[test]
    fn test_traversal_exists() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        assert!(traversal.exists(&vec![1, 2, 3, 4, 5], |x| *x == 3));
        assert!(!traversal.exists(&vec![1, 2, 3, 4, 5], |x| *x == 10));
    }

    #[test]
    fn test_traversal_head_option() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        assert_eq!(traversal.head_option(&vec![1, 2, 3]), Some(&1));
        assert_eq!(traversal.head_option(&Vec::<i32>::new()), None);
    }
}
