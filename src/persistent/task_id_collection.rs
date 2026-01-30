//! Task ID collection with automatic state transitions.
//!
//! This module provides [`TaskIdCollection`], a specialized persistent collection
//! optimized for storing task identifiers with automatic state transitions between
//! small (inline) and large (hash-based) representations.
//!
//! # Overview
//!
//! `TaskIdCollection` provides efficient storage for task IDs by:
//! - Using inline storage (`SmallVec`) for small collections (up to 8 elements)
//! - Automatically promoting to `PersistentHashSet` when exceeding 8 elements
//! - Automatically demoting back to inline storage when size drops to 8 or fewer
//!
//! # Functional Programming Principles
//!
//! All operations follow functional programming principles:
//! - **Referential Transparency**: Same inputs always produce same outputs
//! - **Immutability**: All operations return new instances without modifying the original
//! - **No Side Effects**: Pure functions with no observable side effects
//!
//! # Time Complexity
//!
//! | Operation      | Small (n <= 8)    | Large (n > 8)       |
//! |----------------|-------------------|---------------------|
//! | `insert`       | O(n)              | O(log32 n)          |
//! | `remove`       | O(n)              | O(log32 n)          |
//! | `contains`     | O(n)              | O(log32 n)          |
//! | `len`          | O(1)              | O(1)                |
//! | `is_empty`     | O(1)              | O(1)                |
//! | `iter`         | O(n) traversal    | O(n) traversal      |
//! | `iter_sorted`  | O(n log n)        | O(n log n)          |
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::TaskIdCollection;
//!
//! // Create an empty collection
//! let collection: TaskIdCollection<i32> = TaskIdCollection::new();
//! assert!(collection.is_empty());
//!
//! // Insert elements (returns new collection, original unchanged)
//! let collection = collection.insert(1).insert(2).insert(3);
//! assert_eq!(collection.len(), 3);
//! assert!(collection.contains(&1));
//!
//! // Duplicate insertion is idempotent
//! let collection2 = collection.insert(1);
//! assert_eq!(collection2.len(), 3);
//!
//! // Remove elements
//! let collection3 = collection.remove(&2);
//! assert_eq!(collection3.len(), 2);
//! assert!(!collection3.contains(&2));
//!
//! // Iterate in sorted order
//! let sorted: Vec<&i32> = collection.iter_sorted().collect();
//! assert_eq!(sorted, vec![&1, &2, &3]);
//! ```
//!
//! # State Transitions
//!
//! ```text
//!                    insert (n < 8)
//!     Empty ─────────────────────────────► Small
//!       ▲                                    │
//!       │ remove (n == 0)                    │ insert (n == 8)
//!       │                                    ▼
//!       └─────────────── Small ◄──────── Large
//!                     remove (n == 8)
//! ```

use smallvec::SmallVec;
use std::borrow::Borrow;
use std::hash::Hash;

use super::PersistentHashSet;

/// The threshold for transitioning between Small and Large states.
/// Collections with more than this many elements use `PersistentHashSet`.
const SMALL_THRESHOLD: usize = 8;

/// Internal representation of the collection state.
///
/// This enum is not publicly accessible to prevent external construction
/// that could violate internal invariants.
#[derive(Clone)]
enum TaskIdCollectionInner<T: Clone + Eq + Hash> {
    /// Empty collection.
    Empty,
    /// Small collection (up to 8 elements) stored inline.
    Small(SmallVec<[T; SMALL_THRESHOLD]>),
    /// Large collection (more than 8 elements) stored in a hash set.
    Large(PersistentHashSet<T>),
}

/// A persistent collection optimized for task identifiers.
///
/// This collection automatically transitions between three states based on size:
/// - Empty: No elements
/// - Small: Up to 8 elements stored inline in a `SmallVec`
/// - Large: More than 8 elements stored in a `PersistentHashSet`
///
/// All operations are immutable and return new instances.
///
/// # Type Parameters
///
/// * `T` - The element type. Must implement `Clone`, `Eq`, and `Hash`.
///   `Ord` is only required for `iter_sorted`.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::TaskIdCollection;
///
/// let collection = TaskIdCollection::new()
///     .insert(3)
///     .insert(1)
///     .insert(2);
///
/// // Iteration in sorted order
/// let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
/// assert_eq!(sorted, vec![1, 2, 3]);
/// ```
#[derive(Clone)]
pub struct TaskIdCollection<T: Clone + Eq + Hash> {
    inner: TaskIdCollectionInner<T>,
}

impl<T: Clone + Eq + Hash> TaskIdCollection<T> {
    /// Creates a new empty collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    /// assert!(collection.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: TaskIdCollectionInner::Empty,
        }
    }

    /// Returns the number of elements in the collection.
    ///
    /// # Complexity
    ///
    /// O(1) for all states.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new().insert(1).insert(2);
    /// assert_eq!(collection.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.inner {
            TaskIdCollectionInner::Empty => 0,
            TaskIdCollectionInner::Small(vec) => vec.len(),
            TaskIdCollectionInner::Large(set) => set.len(),
        }
    }

    /// Returns `true` if the collection contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let empty: TaskIdCollection<i32> = TaskIdCollection::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert(42);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.inner, TaskIdCollectionInner::Empty)
    }

    /// Returns `true` if the collection contains the specified element.
    ///
    /// This method supports borrowed forms of the element type through the
    /// `Borrow` trait. For example, with `TaskIdCollection<String>`, you can
    /// search using `&str` directly without allocating a new `String`.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to check for
    ///
    /// # Complexity
    ///
    /// - O(n) for `Small` state (linear search)
    /// - O(log32 n) for `Large` state (hash lookup)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new().insert(1).insert(2);
    /// assert!(collection.contains(&1));
    /// assert!(!collection.contains(&3));
    ///
    /// // With String elements, you can search using &str
    /// let strings = TaskIdCollection::new()
    ///     .insert("hello".to_string())
    ///     .insert("world".to_string());
    /// assert!(strings.contains("hello")); // No allocation needed
    /// ```
    #[inline]
    #[must_use]
    pub fn contains<Q>(&self, element: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match &self.inner {
            TaskIdCollectionInner::Empty => false,
            TaskIdCollectionInner::Small(vec) => vec.iter().any(|item| item.borrow() == element),
            TaskIdCollectionInner::Large(set) => set.contains(element),
        }
    }

    /// Inserts an element into the collection, returning a new collection.
    ///
    /// If the element already exists, returns a clone of the current collection
    /// (idempotent operation).
    ///
    /// # State Transitions
    ///
    /// - `Empty` -> `Small` when inserting the first element
    /// - `Small` -> `Large` when inserting the 9th element
    ///
    /// # Arguments
    ///
    /// * `element` - The element to insert
    ///
    /// # Complexity
    ///
    /// - O(n) for `Small` state (duplicate check + potential copy)
    /// - O(log32 n) for `Large` state (hash-based insertion)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new();
    /// let collection = collection.insert(42);
    ///
    /// assert_eq!(collection.len(), 1);
    /// assert!(collection.contains(&42));
    ///
    /// // Duplicate insertion is idempotent
    /// let collection2 = collection.insert(42);
    /// assert_eq!(collection2.len(), 1);
    /// ```
    #[must_use]
    pub fn insert(&self, element: T) -> Self {
        match &self.inner {
            TaskIdCollectionInner::Empty => {
                let mut vec = SmallVec::new();
                vec.push(element);
                Self {
                    inner: TaskIdCollectionInner::Small(vec),
                }
            }
            TaskIdCollectionInner::Small(vec) => {
                // Check for duplicate
                if vec.iter().any(|item| item == &element) {
                    return self.clone();
                }

                // Check if we need to promote to Large
                if vec.len() >= SMALL_THRESHOLD {
                    // Promote to Large
                    let mut set = PersistentHashSet::new();
                    for item in vec {
                        set = set.insert(item.clone());
                    }
                    set = set.insert(element);
                    Self {
                        inner: TaskIdCollectionInner::Large(set),
                    }
                } else {
                    // Stay in Small
                    let mut new_vec = vec.clone();
                    new_vec.push(element);
                    Self {
                        inner: TaskIdCollectionInner::Small(new_vec),
                    }
                }
            }
            TaskIdCollectionInner::Large(set) => {
                // Check for duplicate
                if set.contains(&element) {
                    return self.clone();
                }
                Self {
                    inner: TaskIdCollectionInner::Large(set.insert(element)),
                }
            }
        }
    }

    /// Removes an element from the collection, returning a new collection.
    ///
    /// If the element does not exist, returns a clone of the current collection.
    ///
    /// This method supports borrowed forms of the element type through the
    /// `Borrow` trait. For example, with `TaskIdCollection<String>`, you can
    /// remove using `&str` directly without allocating a new `String`.
    ///
    /// # State Transitions
    ///
    /// - `Small` -> `Empty` when removing the last element
    /// - `Large` -> `Small` when size drops to 8 or fewer elements
    ///
    /// # Arguments
    ///
    /// * `element` - The element to remove
    ///
    /// # Complexity
    ///
    /// - O(n) for `Small` state (linear search + potential copy)
    /// - O(n) for `Large` state when demoting (need to collect all elements)
    /// - O(log32 n) for `Large` state without demotion
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    /// let collection = collection.remove(&2);
    ///
    /// assert_eq!(collection.len(), 2);
    /// assert!(!collection.contains(&2));
    ///
    /// // With String elements, you can remove using &str
    /// let strings = TaskIdCollection::new()
    ///     .insert("hello".to_string())
    ///     .insert("world".to_string());
    /// let strings = strings.remove("hello"); // No allocation needed
    /// assert!(!strings.contains("hello"));
    /// ```
    #[must_use]
    pub fn remove<Q>(&self, element: &Q) -> Self
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match &self.inner {
            TaskIdCollectionInner::Empty => Self {
                inner: TaskIdCollectionInner::Empty,
            },
            TaskIdCollectionInner::Small(vec) => {
                if !vec
                    .iter()
                    .any(|item| <T as Borrow<Q>>::borrow(item) == element)
                {
                    return self.clone();
                }

                let new_vec: SmallVec<[T; SMALL_THRESHOLD]> = vec
                    .iter()
                    .filter(|item| <T as Borrow<Q>>::borrow(item) != element)
                    .cloned()
                    .collect();

                if new_vec.is_empty() {
                    Self {
                        inner: TaskIdCollectionInner::Empty,
                    }
                } else {
                    Self {
                        inner: TaskIdCollectionInner::Small(new_vec),
                    }
                }
            }
            TaskIdCollectionInner::Large(set) => {
                if !set.contains(element) {
                    return self.clone();
                }

                let new_set = set.remove(element);

                // Check if we need to demote to Small
                if new_set.len() <= SMALL_THRESHOLD {
                    let vec: SmallVec<[T; SMALL_THRESHOLD]> = new_set.iter().cloned().collect();
                    if vec.is_empty() {
                        Self {
                            inner: TaskIdCollectionInner::Empty,
                        }
                    } else {
                        Self {
                            inner: TaskIdCollectionInner::Small(vec),
                        }
                    }
                } else {
                    Self {
                        inner: TaskIdCollectionInner::Large(new_set),
                    }
                }
            }
        }
    }

    /// Returns an iterator over references to the elements.
    ///
    /// The order of elements is not guaranteed.
    ///
    /// # Complexity
    ///
    /// - O(1) for iterator creation
    /// - O(n) for full traversal
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    /// let mut elements: Vec<i32> = collection.iter().copied().collect();
    /// elements.sort();
    /// assert_eq!(elements, vec![1, 2, 3]);
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> TaskIdCollectionIterator<'_, T> {
        TaskIdCollectionIterator {
            inner: match &self.inner {
                TaskIdCollectionInner::Empty => TaskIdCollectionIteratorInner::Empty,
                TaskIdCollectionInner::Small(vec) => {
                    TaskIdCollectionIteratorInner::Small(vec.iter())
                }
                TaskIdCollectionInner::Large(set) => {
                    TaskIdCollectionIteratorInner::Large(set.iter())
                }
            },
        }
    }

    /// Returns `true` if the collection is in the Empty state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_empty_state(&self) -> bool {
        matches!(self.inner, TaskIdCollectionInner::Empty)
    }

    /// Returns `true` if the collection is in the Small state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_small_state(&self) -> bool {
        matches!(self.inner, TaskIdCollectionInner::Small(_))
    }

    /// Returns `true` if the collection is in the Large state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_large_state(&self) -> bool {
        matches!(self.inner, TaskIdCollectionInner::Large(_))
    }
}

impl<T: Clone + Eq + Hash + Ord> TaskIdCollection<T> {
    /// Returns an iterator over references to the elements in sorted order.
    ///
    /// Elements are sorted according to their `Ord` implementation.
    ///
    /// # Complexity
    ///
    /// O(n log n) for sorting.
    ///
    /// # Memory Allocation
    ///
    /// - Small state (n <= 8): Uses `SmallVec` for temporary sorted storage (no heap allocation)
    /// - Large state (n > 8): Allocates a `Vec` for sorting
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TaskIdCollection;
    ///
    /// let collection = TaskIdCollection::new()
    ///     .insert(3)
    ///     .insert(1)
    ///     .insert(2);
    ///
    /// let sorted: Vec<&i32> = collection.iter_sorted().collect();
    /// assert_eq!(sorted, vec![&1, &2, &3]);
    /// ```
    #[inline]
    #[must_use]
    pub fn iter_sorted(&self) -> TaskIdCollectionSortedIterator<'_, T> {
        match &self.inner {
            TaskIdCollectionInner::Empty => TaskIdCollectionSortedIterator {
                inner: SortedIteratorInner::Empty,
            },
            TaskIdCollectionInner::Small(vec) => {
                // Use SmallVec for temporary sorted storage to avoid heap allocation
                let mut sorted: SmallVec<[&T; SMALL_THRESHOLD]> = vec.iter().collect();
                sorted.sort_unstable();
                TaskIdCollectionSortedIterator {
                    inner: SortedIteratorInner::Small(sorted, 0),
                }
            }
            TaskIdCollectionInner::Large(set) => {
                // For Large state, we need a Vec since the set can exceed SMALL_THRESHOLD
                let mut elements: Vec<&T> = set.iter().collect();
                elements.sort_unstable();
                TaskIdCollectionSortedIterator {
                    inner: SortedIteratorInner::Large(elements, 0),
                }
            }
        }
    }
}

impl<T: Clone + Eq + Hash> Default for TaskIdCollection<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Iterators
// =============================================================================

/// Iterator over references to elements in a `TaskIdCollection`.
pub struct TaskIdCollectionIterator<'a, T: Clone + Eq + Hash> {
    inner: TaskIdCollectionIteratorInner<'a, T>,
}

enum TaskIdCollectionIteratorInner<'a, T: Clone + Eq + Hash> {
    Empty,
    Small(std::slice::Iter<'a, T>),
    Large(super::PersistentHashSetIterator<'a, T>),
}

impl<'a, T: Clone + Eq + Hash> Iterator for TaskIdCollectionIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            TaskIdCollectionIteratorInner::Empty => None,
            TaskIdCollectionIteratorInner::Small(iter) => iter.next(),
            TaskIdCollectionIteratorInner::Large(iter) => iter.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            TaskIdCollectionIteratorInner::Empty => (0, Some(0)),
            TaskIdCollectionIteratorInner::Small(iter) => iter.size_hint(),
            TaskIdCollectionIteratorInner::Large(iter) => iter.size_hint(),
        }
    }
}

impl<T: Clone + Eq + Hash> ExactSizeIterator for TaskIdCollectionIterator<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            TaskIdCollectionIteratorInner::Empty => 0,
            TaskIdCollectionIteratorInner::Small(iter) => iter.len(),
            TaskIdCollectionIteratorInner::Large(iter) => iter.len(),
        }
    }
}

/// Iterator over references to elements in sorted order.
///
/// This iterator uses different internal representations based on collection size:
/// - Empty: No storage needed
/// - Small: Uses `SmallVec` to avoid heap allocation for small collections
/// - Large: Uses `Vec` for large collections
pub struct TaskIdCollectionSortedIterator<'a, T> {
    inner: SortedIteratorInner<'a, T>,
}

/// Internal representation for sorted iterator.
enum SortedIteratorInner<'a, T> {
    /// Empty collection.
    Empty,
    /// Small collection sorted in `SmallVec`.
    Small(SmallVec<[&'a T; SMALL_THRESHOLD]>, usize),
    /// Large collection sorted in `Vec`.
    Large(Vec<&'a T>, usize),
}

impl<'a, T> Iterator for TaskIdCollectionSortedIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            SortedIteratorInner::Empty => None,
            SortedIteratorInner::Small(elements, index) => {
                let element = elements.get(*index).copied();
                if element.is_some() {
                    *index += 1;
                }
                element
            }
            SortedIteratorInner::Large(elements, index) => {
                let element = elements.get(*index).copied();
                if element.is_some() {
                    *index += 1;
                }
                element
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = match &self.inner {
            SortedIteratorInner::Empty => 0,
            SortedIteratorInner::Small(elements, index) => elements.len() - *index,
            SortedIteratorInner::Large(elements, index) => elements.len() - *index,
        };
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for TaskIdCollectionSortedIterator<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            SortedIteratorInner::Empty => 0,
            SortedIteratorInner::Small(elements, index) => elements.len() - *index,
            SortedIteratorInner::Large(elements, index) => elements.len() - *index,
        }
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl<T: Clone + Eq + Hash + std::fmt::Debug> std::fmt::Debug for TaskIdCollection<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Clone + Eq + Hash> PartialEq for TaskIdCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        // All elements in self must be in other
        self.iter().all(|element| other.contains(element))
    }
}

impl<T: Clone + Eq + Hash> Eq for TaskIdCollection<T> {}

impl<'a, T: Clone + Eq + Hash> IntoIterator for &'a TaskIdCollection<T> {
    type Item = &'a T;
    type IntoIter = TaskIdCollectionIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
        let collection: TaskIdCollection<i32> = TaskIdCollection::new();
        assert!(collection.is_empty_state());
    }

    #[rstest]
    fn test_small_threshold_constant() {
        assert_eq!(SMALL_THRESHOLD, 8);
    }

    #[rstest]
    fn test_insert_transitions_empty_to_small() {
        let collection = TaskIdCollection::new().insert(1);
        assert!(collection.is_small_state());
    }

    #[rstest]
    fn test_insert_transitions_small_to_large() {
        let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
        for i in 1..=9 {
            collection = collection.insert(i);
        }
        assert!(collection.is_large_state());
    }

    #[rstest]
    fn test_remove_transitions_large_to_small() {
        let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
        for i in 1..=9 {
            collection = collection.insert(i);
        }
        assert!(collection.is_large_state());

        let collection = collection.remove(&9);
        assert!(collection.is_small_state());
    }

    #[rstest]
    fn test_remove_transitions_small_to_empty() {
        let collection = TaskIdCollection::new().insert(1);
        let collection = collection.remove(&1);
        assert!(collection.is_empty_state());
    }

    #[rstest]
    fn test_equality() {
        let collection1 = TaskIdCollection::new().insert(1).insert(2).insert(3);
        let collection2 = TaskIdCollection::new().insert(3).insert(1).insert(2);
        assert_eq!(collection1, collection2);
    }

    #[rstest]
    fn test_inequality_different_elements() {
        let collection1 = TaskIdCollection::new().insert(1).insert(2);
        let collection2 = TaskIdCollection::new().insert(1).insert(3);
        assert_ne!(collection1, collection2);
    }

    #[rstest]
    fn test_inequality_different_lengths() {
        let collection1 = TaskIdCollection::new().insert(1).insert(2);
        let collection2 = TaskIdCollection::new().insert(1);
        assert_ne!(collection1, collection2);
    }

    #[rstest]
    fn test_borrow_contains_with_str() {
        let collection = TaskIdCollection::new()
            .insert("apple".to_string())
            .insert("banana".to_string());

        // Search using &str without allocating String
        assert!(collection.contains("apple"));
        assert!(collection.contains("banana"));
        assert!(!collection.contains("cherry"));
    }

    #[rstest]
    fn test_borrow_remove_with_str() {
        let collection = TaskIdCollection::new()
            .insert("apple".to_string())
            .insert("banana".to_string());

        // Remove using &str without allocating String
        let collection = collection.remove("apple");
        assert!(!collection.contains("apple"));
        assert!(collection.contains("banana"));
    }
}
