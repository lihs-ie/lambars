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
use std::hash::{Hash, Hasher};
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

    /// Builds a list from a Vec efficiently.
    ///
    /// Uses `Vec::pop()` to consume elements from the end, which is O(1),
    /// avoiding the need for reverse iteration.
    ///
    /// # Arguments
    ///
    /// * `elements` - The Vec containing elements to build the list from
    ///
    /// # Returns
    ///
    /// A new list with elements in the same order as the Vec
    fn build_from_vec(mut elements: Vec<T>) -> Self {
        let length = elements.len();
        if length == 0 {
            return Self::new();
        }

        // Build from end to start using Vec::pop()
        let mut head: Option<Rc<Node<T>>> = None;
        while let Some(element) = elements.pop() {
            head = Some(Rc::new(Node {
                element,
                next: head,
            }));
        }

        Self { head, length }
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

    /// Finds the index of the first element that satisfies the predicate.
    ///
    /// Returns `Some(index)` if an element is found, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns `true` for the target element
    ///
    /// # Complexity
    ///
    /// O(n) worst case, O(k) where k is the index of the first match
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let index = list.find_index(|x| *x > 3);
    /// // index = Some(3)  (element 4 is at index 3)
    ///
    /// let not_found = list.find_index(|x| *x > 10);
    /// // not_found = None
    /// ```
    #[must_use]
    pub fn find_index<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(&T) -> bool,
    {
        self.iter().position(predicate)
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

        // Collect self elements to Vec (avoiding reverse() which creates N Rc::new calls)
        // This reduces Rc::new calls from 2N to N
        let mut elements: Vec<T> = self.iter().cloned().collect();

        // Use Vec::pop() to iterate in reverse order and cons to other
        let mut result = other.clone();
        while let Some(element) = elements.pop() {
            result = Self {
                head: Some(Rc::new(Node {
                    element,
                    next: result.head,
                })),
                length: result.length + 1,
            };
        }
        result
    }

    /// Prepends multiple elements to the front of the list.
    ///
    /// The elements from the iterator are added such that the first element
    /// of the iterator becomes the first element of the resulting list.
    /// That is, `list.extend_front([1, 2, 3])` is equivalent to
    /// `list.cons(3).cons(2).cons(1)`, but more efficient.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator over elements to prepend
    ///
    /// # Returns
    ///
    /// A new list with the elements prepended (original list unchanged)
    ///
    /// # Complexity
    ///
    /// O(m) where m = `iter.count()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::new().cons(4).cons(3);
    /// let extended = list.extend_front(vec![1, 2]);
    ///
    /// let collected: Vec<&i32> = extended.iter().collect();
    /// assert_eq!(collected, vec![&1, &2, &3, &4]);
    /// ```
    #[must_use]
    pub fn extend_front<I: IntoIterator<Item = T>>(&self, iter: I) -> Self {
        let mut elements: Vec<T> = iter.into_iter().collect();
        if elements.is_empty() {
            return self.clone();
        }

        let additional_length = elements.len();
        let mut head = self.head.clone();
        let mut current_length = self.length;

        // Use Vec::pop() to iterate in reverse order
        while let Some(element) = elements.pop() {
            head = Some(Rc::new(Node {
                element,
                next: head,
            }));
            current_length += 1;
        }

        debug_assert_eq!(current_length, self.length + additional_length);
        Self {
            head,
            length: current_length,
        }
    }

    /// Creates a list from a slice.
    ///
    /// The first element of the slice becomes the first element of the list.
    ///
    /// # Arguments
    ///
    /// * `slice` - The slice to build the list from
    ///
    /// # Returns
    ///
    /// A new list containing the elements from the slice
    ///
    /// # Complexity
    ///
    /// O(n) where n = `slice.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list = PersistentList::from_slice(&[1, 2, 3]);
    /// assert_eq!(list.head(), Some(&1));
    /// assert_eq!(list.len(), 3);
    /// ```
    #[must_use]
    pub fn from_slice(slice: &[T]) -> Self {
        let length = slice.len();
        if length == 0 {
            return Self::new();
        }

        // Iterate slice in reverse order (DoubleEndedIterator makes this efficient)
        let mut head: Option<Rc<Node<T>>> = None;
        for element in slice.iter().rev() {
            head = Some(Rc::new(Node {
                element: element.clone(),
                next: head,
            }));
        }

        Self { head, length }
    }

    /// Returns a new list containing the first `count` elements.
    ///
    /// If `count` exceeds the list's length, returns a copy of the entire list.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements to take from the front
    ///
    /// # Complexity
    ///
    /// O(min(n, count))
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let taken = list.take(3);
    /// // taken = [1, 2, 3]
    ///
    /// let over = list.take(10);
    /// // over = [1, 2, 3, 4, 5] (entire list)
    ///
    /// let zero = list.take(0);
    /// // zero = []
    /// ```
    #[must_use]
    pub fn take(&self, count: usize) -> Self {
        let actual_count = count.min(self.len());
        self.iter().take(actual_count).cloned().collect()
    }

    /// Returns a new list with the first `count` elements removed.
    ///
    /// If `count` exceeds the list's length, returns an empty list.
    /// This method uses structural sharing for the resulting list.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements to skip from the front
    ///
    /// # Complexity
    ///
    /// O(min(n, count)) for traversal, O(1) for structural sharing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let dropped = list.drop_first(2);
    /// // dropped = [3, 4, 5]
    ///
    /// let all_dropped = list.drop_first(10);
    /// // all_dropped = []
    ///
    /// let none_dropped = list.drop_first(0);
    /// // none_dropped = [1, 2, 3, 4, 5]
    /// ```
    #[must_use]
    pub fn drop_first(&self, count: usize) -> Self {
        let mut current = self.clone();
        for _ in 0..count.min(self.len()) {
            current = current.tail();
        }
        current
    }

    /// Splits the list at the given index.
    ///
    /// Returns a tuple of two lists: the first contains elements before the index,
    /// and the second contains elements from the index onward.
    ///
    /// This is equivalent to `(self.take(index), self.drop_first(index))`.
    ///
    /// # Arguments
    ///
    /// * `index` - The position at which to split the list
    ///
    /// # Complexity
    ///
    /// O(index)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let (left, right) = list.split_at(2);
    /// // left = [1, 2]
    /// // right = [3, 4, 5]
    ///
    /// let (empty_left, all) = list.split_at(0);
    /// // empty_left = []
    /// // all = [1, 2, 3, 4, 5]
    /// ```
    #[must_use]
    pub fn split_at(&self, index: usize) -> (Self, Self) {
        (self.take(index), self.drop_first(index))
    }

    /// Folds the list using the first element as the initial accumulator.
    ///
    /// Returns `None` if the list is empty, otherwise returns `Some(result)`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that combines the accumulator with each element
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let sum = list.fold_left1(|accumulator, x| accumulator + x);
    /// // sum = Some(15)
    ///
    /// let empty: PersistentList<i32> = PersistentList::new();
    /// let result = empty.fold_left1(|accumulator, x| accumulator + x);
    /// // result = None
    /// ```
    #[must_use]
    pub fn fold_left1<F>(&self, mut function: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        let mut iter = self.iter();
        let first = iter.next()?.clone();
        Some(iter.fold(first, |accumulator, x| function(accumulator, x.clone())))
    }

    /// Folds the list from the right using the last element as the initial accumulator.
    ///
    /// Returns `None` if the list is empty, otherwise returns `Some(result)`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that combines each element with the accumulator
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=5).collect();
    /// let sum = list.fold_right1(|x, accumulator| x + accumulator);
    /// // sum = Some(15)
    ///
    /// let list2: PersistentList<i32> = (1..=4).collect();
    /// let result = list2.fold_right1(|x, accumulator| x - accumulator);
    /// // result = Some(1 - (2 - (3 - 4))) = Some(-2)
    /// ```
    #[must_use]
    #[allow(clippy::needless_collect)]
    pub fn fold_right1<F>(&self, mut function: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        let elements: Vec<T> = self.iter().cloned().collect();
        let mut iter = elements.into_iter().rev();
        let last = iter.next()?;
        Some(iter.fold(last, |accumulator, x| function(x, accumulator)))
    }

    /// Returns a list of intermediate accumulator values from a left fold.
    ///
    /// The returned list starts with the initial value and includes each
    /// intermediate result of applying the function.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial accumulator value
    /// * `function` - A function that combines the accumulator with each element
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=4).collect();
    /// let scanned = list.scan_left(0, |accumulator, x| accumulator + x);
    /// // scanned = [0, 1, 3, 6, 10]
    ///
    /// let empty: PersistentList<i32> = PersistentList::new();
    /// let scanned_empty = empty.scan_left(0, |accumulator, x| accumulator + x);
    /// // scanned_empty = [0]
    /// ```
    #[must_use]
    pub fn scan_left<B, F>(&self, initial: B, mut function: F) -> PersistentList<B>
    where
        B: Clone,
        F: FnMut(B, &T) -> B,
    {
        let mut results = Vec::with_capacity(self.len() + 1);
        let mut accumulator = initial;
        results.push(accumulator.clone());

        for element in self {
            accumulator = function(accumulator, element);
            results.push(accumulator.clone());
        }

        results.into_iter().collect()
    }

    /// Partitions the list into two lists based on a predicate.
    ///
    /// Returns a tuple where the first list contains elements for which the
    /// predicate returns `true`, and the second list contains elements for
    /// which it returns `false`. Order is preserved in both lists.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns `true` for elements to include
    ///   in the first list
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list: PersistentList<i32> = (1..=6).collect();
    /// let (evens, odds) = list.partition(|x| x % 2 == 0);
    /// // evens = [2, 4, 6]
    /// // odds = [1, 3, 5]
    /// ```
    #[must_use]
    pub fn partition<P>(&self, predicate: P) -> (Self, Self)
    where
        P: Fn(&T) -> bool,
    {
        let mut pass = Vec::new();
        let mut fail = Vec::new();

        for element in self {
            if predicate(element) {
                pass.push(element.clone());
            } else {
                fail.push(element.clone());
            }
        }

        (pass.into_iter().collect(), fail.into_iter().collect())
    }

    /// Zips this list with another list into a list of pairs.
    ///
    /// The resulting list has the length of the shorter input list.
    /// If either list is empty, returns an empty list.
    ///
    /// # Arguments
    ///
    /// * `other` - The list to zip with
    ///
    /// # Complexity
    ///
    /// O(min(n, m)) where n and m are the lengths of the two lists
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let list1: PersistentList<i32> = (1..=3).collect();
    /// let list2: PersistentList<char> = vec!['a', 'b', 'c'].into_iter().collect();
    /// let zipped = list1.zip(&list2);
    /// // zipped = [(1, 'a'), (2, 'b'), (3, 'c')]
    ///
    /// // Different lengths
    /// let short: PersistentList<i32> = (1..=2).collect();
    /// let zipped_short = short.zip(&list2);
    /// // zipped_short = [(1, 'a'), (2, 'b')]
    /// ```
    #[must_use]
    pub fn zip<U: Clone>(&self, other: &PersistentList<U>) -> PersistentList<(T, U)> {
        self.iter()
            .zip(other.iter())
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect()
    }

    /// Returns a new list with the separator inserted between each element.
    ///
    /// # Arguments
    ///
    /// * `separator` - The element to insert between each pair of elements
    ///
    /// # Returns
    ///
    /// A new list with separators inserted between elements. Returns an empty list
    /// if the original list is empty, and returns a single-element list unchanged.
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
    /// let list: PersistentList<i32> = (1..=4).collect();
    /// let result = list.intersperse(0);
    ///
    /// let collected: Vec<i32> = result.iter().cloned().collect();
    /// assert_eq!(collected, vec![1, 0, 2, 0, 3, 0, 4]);
    /// ```
    #[must_use]
    pub fn intersperse(&self, separator: T) -> Self {
        let mut iter = self.iter();
        let Some(first) = iter.next() else {
            return Self::new();
        };

        let result_length = self.len() * 2 - 1;
        let mut result = Vec::with_capacity(result_length);
        result.push(first.clone());

        for element in iter {
            result.push(separator.clone());
            result.push(element.clone());
        }

        result.into_iter().collect()
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
// Specialized Methods for Tuple Elements
// =============================================================================

impl<A: Clone, B: Clone> PersistentList<(A, B)> {
    /// Separates a list of pairs into two lists.
    ///
    /// This is the inverse operation of [`zip`].
    ///
    /// # Returns
    ///
    /// A tuple containing two lists: one with all first elements and one with all
    /// second elements.
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
    /// let pairs: PersistentList<(i32, char)> =
    ///     vec![(1, 'a'), (2, 'b'), (3, 'c')].into_iter().collect();
    /// let (numbers, chars) = pairs.unzip();
    ///
    /// let numbers_collected: Vec<i32> = numbers.iter().cloned().collect();
    /// let chars_collected: Vec<char> = chars.iter().cloned().collect();
    /// assert_eq!(numbers_collected, vec![1, 2, 3]);
    /// assert_eq!(chars_collected, vec!['a', 'b', 'c']);
    /// ```
    ///
    /// [`zip`]: PersistentList::zip
    #[must_use]
    pub fn unzip(&self) -> (PersistentList<A>, PersistentList<B>) {
        let mut first_elements = Vec::with_capacity(self.len());
        let mut second_elements = Vec::with_capacity(self.len());
        for (a, b) in self {
            first_elements.push(a.clone());
            second_elements.push(b.clone());
        }
        (
            first_elements.into_iter().collect(),
            second_elements.into_iter().collect(),
        )
    }
}

// =============================================================================
// Specialized Methods for Nested Lists
// =============================================================================

impl<T: Clone> PersistentList<PersistentList<T>> {
    /// Inserts a separator list between each inner list and flattens the result.
    ///
    /// This is equivalent to `intersperse` followed by `flatten`.
    ///
    /// # Arguments
    ///
    /// * `separator` - The list to insert between each pair of inner lists
    ///
    /// # Returns
    ///
    /// A flattened list with separators inserted between the original inner lists.
    ///
    /// # Complexity
    ///
    /// O(n * m) time and space, where n is the number of inner lists and m is
    /// the average length of inner lists
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentList;
    ///
    /// let inner1: PersistentList<i32> = vec![1, 2].into_iter().collect();
    /// let inner2: PersistentList<i32> = vec![3, 4].into_iter().collect();
    /// let outer: PersistentList<PersistentList<i32>> =
    ///     vec![inner1, inner2].into_iter().collect();
    /// let separator: PersistentList<i32> = vec![0].into_iter().collect();
    /// let result = outer.intercalate(&separator);
    ///
    /// let collected: Vec<i32> = result.iter().cloned().collect();
    /// assert_eq!(collected, vec![1, 2, 0, 3, 4]);
    /// ```
    #[must_use]
    pub fn intercalate(&self, separator: &PersistentList<T>) -> PersistentList<T> {
        let mut iter = self.iter();
        let Some(first) = iter.next() else {
            return PersistentList::new();
        };

        let mut result: Vec<T> = first.iter().cloned().collect();

        for inner in iter {
            result.extend(separator.iter().cloned());
            result.extend(inner.iter().cloned());
        }

        result.into_iter().collect()
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
        Self::build_from_vec(elements)
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

/// Computes a hash value for this list.
///
/// The hash is computed by first hashing the length, then hashing each
/// element in order. This ensures that:
///
/// - Lists with different lengths have different hashes (with high probability)
/// - The order of elements affects the hash value
/// - Equal lists produce equal hash values (Hash-Eq consistency)
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentList;
/// use std::collections::HashMap;
///
/// let mut map: HashMap<PersistentList<i32>, &str> = HashMap::new();
/// let key: PersistentList<i32> = (1..=3).collect();
/// map.insert(key.clone(), "value");
/// assert_eq!(map.get(&key), Some(&"value"));
/// ```
impl<T: Hash> Hash for PersistentList<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the length first to distinguish lists of different lengths
        self.length.hash(state);
        // Hash each element in order
        for element in self {
            element.hash(state);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for PersistentList<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl<T: fmt::Display> fmt::Display for PersistentList<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[")?;
        let mut first = true;
        for element in self {
            if first {
                first = false;
            } else {
                write!(formatter, ", ")?;
            }
            write!(formatter, "{element}")?;
        }
        write!(formatter, "]")
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
        self.head().map_or_else(PersistentList::new, |head| {
            PersistentList::singleton(function(head.clone()))
        })
    }

    fn fmap_ref<B, F>(&self, function: F) -> PersistentList<B>
    where
        F: FnOnce(&T) -> B,
    {
        self.head().map_or_else(PersistentList::new, |head| {
            PersistentList::singleton(function(head))
        })
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

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_empty_list() {
        let list: PersistentList<i32> = PersistentList::new();
        assert_eq!(format!("{list}"), "[]");
    }

    #[rstest]
    fn test_display_single_element_list() {
        let list = PersistentList::singleton(42);
        assert_eq!(format!("{list}"), "[42]");
    }

    #[rstest]
    fn test_display_multiple_elements_list() {
        let list: PersistentList<i32> = (1..=3).collect();
        assert_eq!(format!("{list}"), "[1, 2, 3]");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

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
        let debug = format!("{list:?}");
        assert!(debug.contains('1'));
        assert!(debug.contains('2'));
        assert!(debug.contains('3'));
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
        let sum = list.fold_left(0, |accumulator, x| accumulator + x);
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

    // =========================================================================
    // take Tests
    // =========================================================================

    #[rstest]
    fn test_take_basic() {
        let list: PersistentList<i32> = (1..=5).collect();
        let taken = list.take(3);
        let collected: Vec<&i32> = taken.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);
        assert_eq!(taken.len(), 3);
    }

    #[rstest]
    fn test_take_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let taken = list.take(5);
        assert!(taken.is_empty());
        assert_eq!(taken.len(), 0);
    }

    #[rstest]
    fn test_take_zero() {
        let list: PersistentList<i32> = (1..=5).collect();
        let taken = list.take(0);
        assert!(taken.is_empty());
        assert_eq!(taken.len(), 0);
    }

    #[rstest]
    fn test_take_exceeds_length() {
        let list: PersistentList<i32> = (1..=3).collect();
        let taken = list.take(10);
        let collected: Vec<&i32> = taken.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);
        assert_eq!(taken.len(), 3);
    }

    #[rstest]
    fn test_take_exact_length() {
        let list: PersistentList<i32> = (1..=5).collect();
        let taken = list.take(5);
        assert_eq!(list, taken);
    }

    // =========================================================================
    // drop_first Tests
    // =========================================================================

    #[rstest]
    fn test_drop_first_basic() {
        let list: PersistentList<i32> = (1..=5).collect();
        let dropped = list.drop_first(2);
        let collected: Vec<&i32> = dropped.iter().collect();
        assert_eq!(collected, vec![&3, &4, &5]);
        assert_eq!(dropped.len(), 3);
    }

    #[rstest]
    fn test_drop_first_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let dropped = list.drop_first(5);
        assert!(dropped.is_empty());
        assert_eq!(dropped.len(), 0);
    }

    #[rstest]
    fn test_drop_first_zero() {
        let list: PersistentList<i32> = (1..=5).collect();
        let dropped = list.drop_first(0);
        assert_eq!(list, dropped);
    }

    #[rstest]
    fn test_drop_first_exceeds_length() {
        let list: PersistentList<i32> = (1..=3).collect();
        let dropped = list.drop_first(10);
        assert!(dropped.is_empty());
        assert_eq!(dropped.len(), 0);
    }

    #[rstest]
    fn test_drop_first_exact_length() {
        let list: PersistentList<i32> = (1..=5).collect();
        let dropped = list.drop_first(5);
        assert!(dropped.is_empty());
    }

    // =========================================================================
    // split_at Tests
    // =========================================================================

    #[rstest]
    fn test_split_at_basic() {
        let list: PersistentList<i32> = (1..=5).collect();
        let (left, right) = list.split_at(2);
        let left_collected: Vec<&i32> = left.iter().collect();
        let right_collected: Vec<&i32> = right.iter().collect();
        assert_eq!(left_collected, vec![&1, &2]);
        assert_eq!(right_collected, vec![&3, &4, &5]);
    }

    #[rstest]
    fn test_split_at_zero() {
        let list: PersistentList<i32> = (1..=5).collect();
        let (left, right) = list.split_at(0);
        assert!(left.is_empty());
        assert_eq!(right, list);
    }

    #[rstest]
    fn test_split_at_length() {
        let list: PersistentList<i32> = (1..=5).collect();
        let (left, right) = list.split_at(5);
        assert_eq!(left, list);
        assert!(right.is_empty());
    }

    #[rstest]
    fn test_split_at_exceeds_length() {
        let list: PersistentList<i32> = (1..=3).collect();
        let (left, right) = list.split_at(10);
        assert_eq!(left, list);
        assert!(right.is_empty());
    }

    #[rstest]
    fn test_split_at_law() {
        let list: PersistentList<i32> = (1..=5).collect();
        let (left, right) = list.split_at(3);
        assert_eq!(left, list.take(3));
        assert_eq!(right, list.drop_first(3));
    }

    #[rstest]
    fn test_split_at_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let (left, right) = list.split_at(2);
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    // =========================================================================
    // find_index Tests
    // =========================================================================

    #[rstest]
    fn test_find_index_found() {
        let list: PersistentList<i32> = (1..=5).collect();
        let index = list.find_index(|x| *x > 3);
        assert_eq!(index, Some(3));
    }

    #[rstest]
    fn test_find_index_not_found() {
        let list: PersistentList<i32> = (1..=5).collect();
        let index = list.find_index(|x| *x > 10);
        assert_eq!(index, None);
    }

    #[rstest]
    fn test_find_index_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let index = list.find_index(|x| *x > 0);
        assert_eq!(index, None);
    }

    #[rstest]
    fn test_find_index_first_match() {
        let list: PersistentList<i32> = vec![1, 3, 3, 3, 5].into_iter().collect();
        let index = list.find_index(|x| *x == 3);
        assert_eq!(index, Some(1));
    }

    #[rstest]
    fn test_find_index_at_start() {
        let list: PersistentList<i32> = (1..=5).collect();
        let index = list.find_index(|x| *x == 1);
        assert_eq!(index, Some(0));
    }

    #[rstest]
    fn test_find_index_at_end() {
        let list: PersistentList<i32> = (1..=5).collect();
        let index = list.find_index(|x| *x == 5);
        assert_eq!(index, Some(4));
    }

    // =========================================================================
    // fold_left1 Tests
    // =========================================================================

    #[rstest]
    fn test_fold_left1_basic() {
        let list: PersistentList<i32> = (1..=5).collect();
        let sum = list.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(sum, Some(15));
    }

    #[rstest]
    fn test_fold_left1_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let result = list.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_fold_left1_single_element() {
        let list: PersistentList<i32> = vec![42].into_iter().collect();
        let result = list.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn test_fold_left1_subtraction() {
        let list: PersistentList<i32> = (1..=4).collect();
        let result = list.fold_left1(|accumulator, x| accumulator - x);
        assert_eq!(result, Some(1 - 2 - 3 - 4));
    }

    #[rstest]
    fn test_fold_left1_max() {
        let list: PersistentList<i32> = vec![3, 1, 4, 1, 5, 9, 2, 6].into_iter().collect();
        let result =
            list.fold_left1(|accumulator, x| if accumulator > x { accumulator } else { x });
        assert_eq!(result, Some(9));
    }

    // =========================================================================
    // fold_right1 Tests
    // =========================================================================

    #[rstest]
    fn test_fold_right1_basic() {
        let list: PersistentList<i32> = (1..=5).collect();
        let sum = list.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(sum, Some(15));
    }

    #[rstest]
    fn test_fold_right1_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let result = list.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_fold_right1_single_element() {
        let list: PersistentList<i32> = vec![42].into_iter().collect();
        let result = list.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn test_fold_right1_subtraction() {
        let list: PersistentList<i32> = (1..=4).collect();
        let result = list.fold_right1(|x, accumulator| x - accumulator);
        assert_eq!(result, Some(1 - (2 - (3 - 4))));
    }

    #[rstest]
    fn test_fold_right1_list_construction() {
        let list: PersistentList<String> =
            vec!["a", "b", "c"].into_iter().map(String::from).collect();
        let result = list.fold_right1(|x, accumulator| format!("({x} {accumulator})"));
        assert_eq!(result, Some("(a (b c))".to_string()));
    }

    // =========================================================================
    // scan_left Tests
    // =========================================================================

    #[rstest]
    fn test_scan_left_basic() {
        let list: PersistentList<i32> = (1..=4).collect();
        let scanned = list.scan_left(0, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![0, 1, 3, 6, 10]);
    }

    #[rstest]
    fn test_scan_left_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let scanned = list.scan_left(0, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![0]);
    }

    #[rstest]
    fn test_scan_left_single_element() {
        let list: PersistentList<i32> = vec![5].into_iter().collect();
        let scanned = list.scan_left(10, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![10, 15]);
    }

    #[rstest]
    fn test_scan_left_type_change() {
        let list: PersistentList<i32> = (1..=3).collect();
        let scanned = list.scan_left(String::new(), |accumulator, x| format!("{accumulator}{x}"));
        let collected: Vec<String> = scanned.iter().cloned().collect();
        assert_eq!(
            collected,
            vec![
                String::new(),
                "1".to_string(),
                "12".to_string(),
                "123".to_string()
            ]
        );
    }

    #[rstest]
    fn test_scan_left_running_max() {
        let list: PersistentList<i32> = vec![3, 1, 4, 1, 5, 9, 2, 6].into_iter().collect();
        let scanned = list.scan_left(i32::MIN, |accumulator, x| accumulator.max(*x));
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![i32::MIN, 3, 3, 4, 4, 5, 9, 9, 9]);
    }

    // =========================================================================
    // partition Tests
    // =========================================================================

    #[rstest]
    fn test_partition_basic() {
        let list: PersistentList<i32> = (1..=6).collect();
        let (evens, odds) = list.partition(|x| x % 2 == 0);
        let evens_collected: Vec<i32> = evens.iter().copied().collect();
        let odds_collected: Vec<i32> = odds.iter().copied().collect();
        assert_eq!(evens_collected, vec![2, 4, 6]);
        assert_eq!(odds_collected, vec![1, 3, 5]);
    }

    #[rstest]
    fn test_partition_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let (pass, fail) = list.partition(|x| x % 2 == 0);
        assert!(pass.is_empty());
        assert!(fail.is_empty());
    }

    #[rstest]
    fn test_partition_all_pass() {
        let list: PersistentList<i32> = (2..=8).step_by(2).collect();
        let (pass, fail) = list.partition(|x| x % 2 == 0);
        let pass_collected: Vec<i32> = pass.iter().copied().collect();
        assert_eq!(pass_collected, vec![2, 4, 6, 8]);
        assert!(fail.is_empty());
    }

    #[rstest]
    fn test_partition_all_fail() {
        let list: PersistentList<i32> = (1..=7).step_by(2).collect();
        let (pass, fail) = list.partition(|x| x % 2 == 0);
        assert!(pass.is_empty());
        let fail_collected: Vec<i32> = fail.iter().copied().collect();
        assert_eq!(fail_collected, vec![1, 3, 5, 7]);
    }

    #[rstest]
    fn test_partition_preserves_order() {
        let list: PersistentList<i32> = (1..=10).collect();
        let (pass, fail) = list.partition(|x| x % 3 == 0);
        let pass_collected: Vec<i32> = pass.iter().copied().collect();
        let fail_collected: Vec<i32> = fail.iter().copied().collect();
        assert_eq!(pass_collected, vec![3, 6, 9]);
        assert_eq!(fail_collected, vec![1, 2, 4, 5, 7, 8, 10]);
    }

    // =========================================================================
    // zip Tests
    // =========================================================================

    #[rstest]
    fn test_zip_basic() {
        let list1: PersistentList<i32> = (1..=3).collect();
        let list2: PersistentList<char> = vec!['a', 'b', 'c'].into_iter().collect();
        let zipped = list1.zip(&list2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
    }

    #[rstest]
    fn test_zip_empty_first() {
        let list1: PersistentList<i32> = PersistentList::new();
        let list2: PersistentList<char> = vec!['a', 'b', 'c'].into_iter().collect();
        let zipped = list1.zip(&list2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_empty_second() {
        let list1: PersistentList<i32> = (1..=3).collect();
        let list2: PersistentList<char> = PersistentList::new();
        let zipped = list1.zip(&list2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_both_empty() {
        let list1: PersistentList<i32> = PersistentList::new();
        let list2: PersistentList<char> = PersistentList::new();
        let zipped = list1.zip(&list2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_different_lengths_first_shorter() {
        let list1: PersistentList<i32> = (1..=2).collect();
        let list2: PersistentList<char> = vec!['a', 'b', 'c', 'd'].into_iter().collect();
        let zipped = list1.zip(&list2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b')]);
    }

    #[rstest]
    fn test_zip_different_lengths_second_shorter() {
        let list1: PersistentList<i32> = (1..=5).collect();
        let list2: PersistentList<char> = vec!['a', 'b'].into_iter().collect();
        let zipped = list1.zip(&list2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b')]);
    }

    // =========================================================================
    // unzip Tests
    // =========================================================================

    #[rstest]
    fn test_unzip_basic() {
        let list: PersistentList<(i32, char)> =
            vec![(1, 'a'), (2, 'b'), (3, 'c')].into_iter().collect();
        let (first, second) = list.unzip();
        let first_collected: Vec<i32> = first.iter().copied().collect();
        let second_collected: Vec<char> = second.iter().copied().collect();
        assert_eq!(first_collected, vec![1, 2, 3]);
        assert_eq!(second_collected, vec!['a', 'b', 'c']);
    }

    #[rstest]
    fn test_unzip_empty() {
        let list: PersistentList<(i32, char)> = PersistentList::new();
        let (first, second) = list.unzip();
        assert!(first.is_empty());
        assert!(second.is_empty());
    }

    #[rstest]
    fn test_unzip_single_element() {
        let list: PersistentList<(i32, char)> = vec![(42, 'x')].into_iter().collect();
        let (first, second) = list.unzip();
        let first_collected: Vec<i32> = first.iter().copied().collect();
        let second_collected: Vec<char> = second.iter().copied().collect();
        assert_eq!(first_collected, vec![42]);
        assert_eq!(second_collected, vec!['x']);
    }

    #[rstest]
    fn test_unzip_roundtrip_with_zip() {
        let list1: PersistentList<i32> = (1..=5).collect();
        let list2: PersistentList<char> = vec!['a', 'b', 'c', 'd', 'e'].into_iter().collect();
        let zipped = list1.zip(&list2);
        let (unzipped1, unzipped2) = zipped.unzip();
        let collected1: Vec<i32> = unzipped1.iter().copied().collect();
        let collected2: Vec<char> = unzipped2.iter().copied().collect();
        assert_eq!(collected1, vec![1, 2, 3, 4, 5]);
        assert_eq!(collected2, vec!['a', 'b', 'c', 'd', 'e']);
    }

    // =========================================================================
    // intersperse Tests
    // =========================================================================

    #[rstest]
    fn test_intersperse_basic() {
        let list: PersistentList<i32> = (1..=4).collect();
        let result = list.intersperse(0);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 0, 2, 0, 3, 0, 4]);
    }

    #[rstest]
    fn test_intersperse_empty() {
        let list: PersistentList<i32> = PersistentList::new();
        let result = list.intersperse(0);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_intersperse_single_element() {
        let list: PersistentList<i32> = vec![42].into_iter().collect();
        let result = list.intersperse(0);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![42]);
    }

    #[rstest]
    fn test_intersperse_two_elements() {
        let list: PersistentList<char> = vec!['a', 'b'].into_iter().collect();
        let result = list.intersperse('-');
        let collected: Vec<char> = result.iter().copied().collect();
        assert_eq!(collected, vec!['a', '-', 'b']);
    }

    #[rstest]
    fn test_intersperse_strings() {
        let list: PersistentList<String> =
            vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
                .into_iter()
                .collect();
        let result = list.intersperse(",".to_string());
        let collected: Vec<String> = result.iter().cloned().collect();
        assert_eq!(
            collected,
            vec![
                "foo".to_string(),
                ",".to_string(),
                "bar".to_string(),
                ",".to_string(),
                "baz".to_string()
            ]
        );
    }

    // =========================================================================
    // intercalate Tests
    // =========================================================================

    #[rstest]
    fn test_intercalate_basic() {
        let inner1: PersistentList<i32> = vec![1, 2].into_iter().collect();
        let inner2: PersistentList<i32> = vec![3, 4].into_iter().collect();
        let inner3: PersistentList<i32> = vec![5, 6].into_iter().collect();
        let outer: PersistentList<PersistentList<i32>> =
            vec![inner1, inner2, inner3].into_iter().collect();
        let separator: PersistentList<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 0, 3, 4, 0, 5, 6]);
    }

    #[rstest]
    fn test_intercalate_empty_outer() {
        let outer: PersistentList<PersistentList<i32>> = PersistentList::new();
        let separator: PersistentList<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_intercalate_single_inner() {
        let inner: PersistentList<i32> = vec![1, 2, 3].into_iter().collect();
        let outer: PersistentList<PersistentList<i32>> = vec![inner].into_iter().collect();
        let separator: PersistentList<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_intercalate_empty_separator() {
        let inner1: PersistentList<i32> = vec![1, 2].into_iter().collect();
        let inner2: PersistentList<i32> = vec![3, 4].into_iter().collect();
        let outer: PersistentList<PersistentList<i32>> = vec![inner1, inner2].into_iter().collect();
        let separator: PersistentList<i32> = PersistentList::new();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn test_intercalate_empty_inner_lists() {
        let inner1: PersistentList<i32> = PersistentList::new();
        let inner2: PersistentList<i32> = PersistentList::new();
        let outer: PersistentList<PersistentList<i32>> = vec![inner1, inner2].into_iter().collect();
        let separator: PersistentList<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![0]);
    }

    #[rstest]
    fn test_intercalate_multi_element_separator() {
        let inner1: PersistentList<char> = vec!['a', 'b'].into_iter().collect();
        let inner2: PersistentList<char> = vec!['c', 'd'].into_iter().collect();
        let outer: PersistentList<PersistentList<char>> =
            vec![inner1, inner2].into_iter().collect();
        let separator: PersistentList<char> = vec!['-', '-'].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<char> = result.iter().copied().collect();
        assert_eq!(collected, vec!['a', 'b', '-', '-', 'c', 'd']);
    }
}
