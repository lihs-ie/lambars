//! Persistent (immutable) singly-linked list.
//!
//! This module provides [`PersistentList`], an immutable singly-linked list
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentList` is a cons-list inspired by Lisp/Scheme. It provides:
//!
//! - O(1) prepend (`cons`)
//! - O(1) head access
//! - O(1) tail access
//! - O(n) index access
//! - O(n) append and reverse
//!
//! All operations return new lists without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentList;
//!
//! // Build a list using cons
//! let list = PersistentList::new().cons(3).cons(2).cons(1);
//! assert_eq!(list.head(), Some(&1));
//! assert_eq!(list.len(), 3);
//!
//! // Structural sharing: the original list is preserved
//! let extended = list.cons(0);
//! assert_eq!(list.len(), 3);     // Original unchanged
//! assert_eq!(extended.len(), 4); // New list with prepended element
//!
//! // Build from an iterator
//! let list: PersistentList<i32> = (1..=5).collect();
//! assert_eq!(list.iter().sum::<i32>(), 15);
//! ```
//!
//! # Structural Sharing
//!
//! When you create a new list by prepending an element with `cons`, the new
//! list shares all nodes with the original list:
//!
//! ```text
//! list1: 1 -> 2 -> 3 -> nil
//! list2 = list1.cons(0): 0 -> [1 -> 2 -> 3 -> nil]  // shares [1, 2, 3] with list1
//! ```
//!
//! This makes `cons` an O(1) operation both in time and additional space.

use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;

use crate::typeclass::{
    Applicative, Foldable, Functor, FunctorMut, Monad, Monoid, Semigroup, TypeConstructor,
};

/// Internal node structure for the persistent list.
///
/// Each node contains an element and an optional reference to the next node.
/// Using `Rc` enables structural sharing between lists.
struct Node<T> {
    /// The element stored in this node.
    element: T,
    /// Reference to the next node (if any).
    next: Option<Rc<Self>>,
}

/// A persistent (immutable) singly-linked list.
///
/// `PersistentList` is an immutable data structure that uses structural
/// sharing to efficiently support functional programming patterns.
///
/// # Time Complexity
///
/// | Operation | Complexity |
/// |-----------|------------|
/// | `new`     | O(1)       |
/// | `cons`    | O(1)       |
/// | `head`    | O(1)       |
/// | `tail`    | O(1)       |
/// | `len`     | O(1)       |
/// | `get`     | O(n)       |
/// | `append`  | O(n)       |
/// | `reverse` | O(n)       |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentList;
///
/// let list = PersistentList::singleton(42);
/// assert_eq!(list.head(), Some(&42));
/// ```
#[derive(Clone)]
pub struct PersistentList<T> {
    /// Reference to the head node (if any).
    head: Option<Rc<Node<T>>>,
    /// Cached length for O(1) access.
    length: usize,
}

impl<T> PersistentList<T> {
    /// Creates a new empty list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = PersistentList::new();
    /// assert!(list.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            head: None,
            length: 0,
        }
    }

    /// Creates a list containing a single element.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to store in the list
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::singleton(42);
    /// assert_eq!(list.head(), Some(&42));
    /// assert_eq!(list.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(element: T) -> Self {
        Self::new().cons(element)
    }

    /// Prepends an element to the front of the list.
    ///
    /// This operation creates a new list with the element at the front,
    /// sharing the structure of the original list.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to prepend
    ///
    /// # Returns
    ///
    /// A new list with the element at the front
    ///
    /// # Complexity
    ///
    /// O(1) time and space
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// assert_eq!(list.head(), Some(&1));
    /// assert_eq!(list.len(), 3);
    /// ```
    #[inline]
    #[must_use]
    pub fn cons(&self, element: T) -> Self {
        Self {
            head: Some(Rc::new(Node {
                element,
                next: self.head.clone(),
            })),
            length: self.length + 1,
        }
    }

    /// Returns a reference to the first element of the list.
    ///
    /// Returns `None` if the list is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(2).cons(1);
    /// assert_eq!(list.head(), Some(&1));
    ///
    /// let empty: PersistentList<i32> = PersistentList::new();
    /// assert_eq!(empty.head(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn head(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.element)
    }

    /// Returns the list without its first element.
    ///
    /// If the list is empty, returns an empty list.
    ///
    /// This operation shares structure with the original list.
    ///
    /// # Complexity
    ///
    /// O(1) time and space
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// let tail = list.tail();
    /// assert_eq!(tail.head(), Some(&2));
    /// assert_eq!(tail.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn tail(&self) -> Self {
        self.head.as_ref().map_or_else(Self::new, |node| Self {
            head: node.next.clone(),
            length: self.length.saturating_sub(1),
        })
    }

    /// Decomposes the list into its head and tail.
    ///
    /// Returns `None` if the list is empty.
    ///
    /// # Returns
    ///
    /// `Some((head, tail))` if the list is non-empty, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(2).cons(1);
    /// if let Some((head, tail)) = list.uncons() {
    ///     assert_eq!(*head, 1);
    ///     assert_eq!(tail.head(), Some(&2));
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn uncons(&self) -> Option<(&T, Self)> {
        self.head.as_ref().map(|node| {
            let tail = Self {
                head: node.next.clone(),
                length: self.length.saturating_sub(1),
            };
            (&node.element, tail)
        })
    }

    /// Returns a reference to the element at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index of the element
    ///
    /// # Complexity
    ///
    /// O(n) where n = index
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// assert_eq!(list.get(0), Some(&1));
    /// assert_eq!(list.get(2), Some(&3));
    /// assert_eq!(list.get(10), None);
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut current = &self.head;
        let mut remaining = index;

        while let Some(node) = current {
            if remaining == 0 {
                return Some(&node.element);
            }
            remaining -= 1;
            current = &node.next;
        }
        None
    }

    /// Returns the number of elements in the list.
    ///
    /// # Complexity
    ///
    /// O(1) - the length is cached
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// assert_eq!(list.len(), 3);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the list contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let empty: PersistentList<i32> = PersistentList::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.cons(1);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Returns an iterator over references to the elements.
    ///
    /// The iterator yields elements from front to back.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// let collected: Vec<&i32> = list.iter().collect();
    /// assert_eq!(collected, vec![&1, &2, &3]);
    /// ```
    #[inline]
    #[must_use] 
    pub const fn iter(&self) -> PersistentListIterator<'_, T> {
        PersistentListIterator {
            current: self.head.as_ref(),
        }
    }
}

impl<T: Clone> PersistentList<T> {
    /// Appends another list to this list.
    ///
    /// Returns a new list containing all elements from this list
    /// followed by all elements from the other list.
    ///
    /// # Arguments
    ///
    /// * `other` - The list to append
    ///
    /// # Complexity
    ///
    /// O(n) where n = `self.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list1 = PersistentList::new().cons(2).cons(1);
    /// let list2 = PersistentList::new().cons(4).cons(3);
    /// let combined = list1.append(&list2);
    ///
    /// let collected: Vec<&i32> = combined.iter().collect();
    /// assert_eq!(collected, vec![&1, &2, &3, &4]);
    /// ```
    #[must_use]
    pub fn append(&self, other: &Self) -> Self {
        if self.is_empty() {
            return other.clone();
        }
        if other.is_empty() {
            return self.clone();
        }

        // Reverse self, then prepend each element to other
        let mut result = other.clone();
        for element in &self.reverse() {
            result = result.cons(element.clone());
        }
        result
    }

    /// Returns a new list with elements in reverse order.
    ///
    /// # Complexity
    ///
    /// O(n) time and space
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(3).cons(2).cons(1);
    /// let reversed = list.reverse();
    ///
    /// let collected: Vec<&i32> = reversed.iter().collect();
    /// assert_eq!(collected, vec![&3, &2, &1]);
    /// ```
    #[must_use]
    pub fn reverse(&self) -> Self {
        let mut result = Self::new();
        for element in self {
            result = result.cons(element.clone());
        }
        result
    }

    /// Applies a function to each element and flattens the results.
    ///
    /// This is the monadic bind operation for lists.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that returns a list for each element
    ///
    /// # Returns
    ///
    /// A list containing all results concatenated
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=3).collect();
    /// let result = list.flat_map_mut(|element| {
    ///     PersistentList::new().cons(element * 2).cons(element)
    /// });
    ///
    /// let collected: Vec<i32> = result.into_iter().collect();
    /// assert_eq!(collected, vec![1, 2, 2, 4, 3, 6]);
    /// ```
    #[must_use]
    pub fn flat_map_mut<B: Clone, F>(self, mut function: F) -> PersistentList<B>
    where
        F: FnMut(T) -> PersistentList<B>,
    {
        let mut result = PersistentList::new();
        for element in self {
            let mapped = function(element);
            result = result.append(&mapped);
        }
        result
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// An iterator over references to elements of a [`PersistentList`].
pub struct PersistentListIterator<'a, T> {
    current: Option<&'a Rc<Node<T>>>,
}

impl<'a, T> Iterator for PersistentListIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.map(|node| {
            self.current = node.next.as_ref();
            &node.element
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // We cannot efficiently compute the remaining length,
        // but we know it's at least 0 and at most the original list length
        (0, None)
    }
}

/// An owning iterator over elements of a [`PersistentList`].
pub struct PersistentListIntoIterator<T> {
    list: PersistentList<T>,
}

impl<T: Clone> Iterator for PersistentListIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((head, tail)) = self.list.uncons() {
            let element = head.clone();
            self.list = tail;
            Some(element)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.list.length, Some(self.list.length))
    }
}

impl<T: Clone> ExactSizeIterator for PersistentListIntoIterator<T> {
    fn len(&self) -> usize {
        self.list.length
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<T> Default for PersistentList<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> FromIterator<T> for PersistentList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let elements: Vec<T> = iter.into_iter().collect();
        let mut list = Self::new();
        for element in elements.into_iter().rev() {
            list = list.cons(element);
        }
        list
    }
}

impl<T: Clone> IntoIterator for PersistentList<T> {
    type Item = T;
    type IntoIter = PersistentListIntoIterator<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        PersistentListIntoIterator { list: self }
    }
}

impl<'a, T> IntoIterator for &'a PersistentList<T> {
    type Item = &'a T;
    type IntoIter = PersistentListIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: PartialEq> PartialEq for PersistentList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: Eq> Eq for PersistentList<T> {}

impl<T: fmt::Debug> fmt::Debug for PersistentList<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

// =============================================================================
// Type Class Implementations
// =============================================================================

impl<T> TypeConstructor for PersistentList<T> {
    type Inner = T;
    type WithType<B> = PersistentList<B>;
}

impl<T: Clone> Functor for PersistentList<T> {
    fn fmap<B, F>(self, function: F) -> PersistentList<B>
    where
        F: FnOnce(T) -> B,
    {
        // FnOnce can only be called once, so this only works for single-element lists
        self.head()
            .map_or_else(PersistentList::new, |head| PersistentList::singleton(function(head.clone())))
    }

    fn fmap_ref<B, F>(&self, function: F) -> PersistentList<B>
    where
        F: FnOnce(&T) -> B,
    {
        self.head()
            .map_or_else(PersistentList::new, |head| PersistentList::singleton(function(head)))
    }
}

impl<T: Clone> FunctorMut for PersistentList<T> {
    fn fmap_mut<B, F>(self, mut function: F) -> PersistentList<B>
    where
        F: FnMut(T) -> B,
    {
        // Collect elements, transform them, and build the result list
        let elements: Vec<T> = self.into_iter().collect();
        let mut result = PersistentList::new();
        // Build list from end to start to maintain order
        for element in elements.into_iter().rev() {
            result = PersistentList {
                head: Some(Rc::new(Node {
                    element: function(element),
                    next: result.head,
                })),
                length: result.length + 1,
            };
        }
        result
    }

    fn fmap_ref_mut<B, F>(&self, mut function: F) -> PersistentList<B>
    where
        F: FnMut(&T) -> B,
    {
        // Build the list: collect elements, then build from end to start
        let elements: Vec<_> = self.iter().collect();
        let mut result = PersistentList::new();
        for element in elements.into_iter().rev() {
            result = PersistentList {
                head: Some(Rc::new(Node {
                    element: function(element),
                    next: result.head,
                })),
                length: result.length + 1,
            };
        }
        result
    }
}

impl<T: Clone> Applicative for PersistentList<T> {
    fn pure<A>(value: A) -> PersistentList<A> {
        PersistentList::singleton(value)
    }

    fn map2<B, C, F>(self, other: Self::WithType<B>, _function: F) -> Self::WithType<C>
    where
        F: FnOnce(T, B) -> C,
    {
        // For FnOnce, we can only support single-element lists
        // This is a type system limitation - the Applicative trait requires FnOnce
        // but we cannot extract elements without Clone on B
        // For proper list applicative, use ApplicativeList trait instead
        let _ = (self, other);
        PersistentList::new()
    }

    fn map3<B, C, D, F>(
        self,
        second: Self::WithType<B>,
        third: Self::WithType<C>,
        _function: F,
    ) -> Self::WithType<D>
    where
        F: FnOnce(T, B, C) -> D,
    {
        // Similar limitation as map2 - requires Clone on B and C for proper implementation
        let _ = (self, second, third);
        PersistentList::new()
    }

    fn apply<B, Output>(self, other: Self::WithType<B>) -> Self::WithType<Output>
    where
        Self: Sized,
        T: FnOnce(B) -> Output,
    {
        // Similar limitation - cannot properly implement without Clone on B
        let _ = (self, other);
        PersistentList::new()
    }
}

impl<T: Clone> Foldable for PersistentList<T> {
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
        // Reverse and fold_left
        self.reverse()
            .into_iter()
            .fold(init, |accumulator, element| function(element, accumulator))
    }

    #[inline]
    fn is_empty(&self) -> bool
    where
        Self: Clone,
    {
        self.head.is_none()
    }

    #[inline]
    fn length(&self) -> usize
    where
        Self: Clone,
    {
        self.length
    }
}

impl<T: Clone> Monad for PersistentList<T> {
    fn flat_map<B, F>(self, function: F) -> PersistentList<B>
    where
        F: FnOnce(T) -> PersistentList<B>,
    {
        // FnOnce can only be called once, so this only works for single-element lists
        self.head()
            .map_or_else(PersistentList::new, |head| function(head.clone()))
    }
}

impl<T: Clone> Semigroup for PersistentList<T> {
    fn combine(self, other: Self) -> Self {
        self.append(&other)
    }
}

impl<T: Clone> Monoid for PersistentList<T> {
    fn empty() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_new_creates_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[rstest]
    fn test_singleton() {
        let list = PersistentList::singleton(42);
        assert_eq!(list.head(), Some(&42));
        assert_eq!(list.len(), 1);
    }

    #[rstest]
    fn test_cons() {
        let list = PersistentList::new().cons(1).cons(2).cons(3);
        assert_eq!(list.head(), Some(&3));
        assert_eq!(list.len(), 3);
    }

    #[rstest]
    fn test_tail() {
        let list = PersistentList::new().cons(1).cons(2).cons(3);
        let tail = list.tail();
        assert_eq!(tail.head(), Some(&2));
        assert_eq!(tail.len(), 2);
    }

    #[rstest]
    fn test_uncons() {
        let list = PersistentList::new().cons(1).cons(2);
        let (head, tail) = list.uncons().unwrap();
        assert_eq!(*head, 2);
        assert_eq!(tail.head(), Some(&1));
    }

    #[rstest]
    fn test_get() {
        let list = PersistentList::new().cons(3).cons(2).cons(1);
        assert_eq!(list.get(0), Some(&1));
        assert_eq!(list.get(1), Some(&2));
        assert_eq!(list.get(2), Some(&3));
        assert_eq!(list.get(3), None);
    }

    #[rstest]
    fn test_iter() {
        let list = PersistentList::new().cons(3).cons(2).cons(1);
        let collected: Vec<&i32> = list.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);
    }

    #[rstest]
    fn test_reverse() {
        let list: PersistentList<i32> = (1..=3).collect();
        let reversed = list.reverse();
        let collected: Vec<&i32> = reversed.iter().collect();
        assert_eq!(collected, vec![&3, &2, &1]);
    }

    #[rstest]
    fn test_append() {
        let list1: PersistentList<i32> = (1..=2).collect();
        let list2: PersistentList<i32> = (3..=4).collect();
        let combined = list1.append(&list2);
        let collected: Vec<&i32> = combined.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3, &4]);
    }

    #[rstest]
    fn test_from_iter() {
        let list: PersistentList<i32> = (1..=5).collect();
        assert_eq!(list.len(), 5);
        assert_eq!(list.head(), Some(&1));
    }

    #[rstest]
    fn test_into_iter() {
        let list: PersistentList<i32> = (1..=3).collect();
        let collected: Vec<i32> = list.into_iter().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_eq() {
        let list1: PersistentList<i32> = (1..=3).collect();
        let list2: PersistentList<i32> = (1..=3).collect();
        let list3: PersistentList<i32> = (1..=4).collect();
        assert_eq!(list1, list2);
        assert_ne!(list1, list3);
    }

    #[rstest]
    fn test_debug() {
        let list: PersistentList<i32> = (1..=3).collect();
        let debug = format!("{:?}", list);
        assert!(debug.contains("1"));
        assert!(debug.contains("2"));
        assert!(debug.contains("3"));
    }

    #[rstest]
    fn test_fmap_mut() {
        let list: PersistentList<i32> = (1..=3).collect();
        let doubled: PersistentList<i32> = list.fmap_mut(|x| x * 2);
        let collected: Vec<&i32> = doubled.iter().collect();
        assert_eq!(collected, vec![&2, &4, &6]);
    }

    #[rstest]
    fn test_fold_left() {
        let list: PersistentList<i32> = (1..=5).collect();
        let sum = list.fold_left(0, |acc, x| acc + x);
        assert_eq!(sum, 15);
    }

    #[rstest]
    fn test_semigroup_combine() {
        let list1: PersistentList<i32> = (1..=2).collect();
        let list2: PersistentList<i32> = (3..=4).collect();
        let combined = list1.combine(list2);
        let collected: Vec<&i32> = combined.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3, &4]);
    }

    #[rstest]
    fn test_monoid_empty() {
        let empty: PersistentList<i32> = PersistentList::empty();
        assert!(empty.is_empty());
    }
}
