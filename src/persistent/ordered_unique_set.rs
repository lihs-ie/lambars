//! Ordered unique set with automatic state transitions.
//!
//! This module provides [`OrderedUniqueSet`], a persistent collection
//! optimized for storing unique elements with automatic state transitions between
//! small (inline) and large (hash-based) representations.
//!
//! # Overview
//!
//! `OrderedUniqueSet` provides efficient storage for unique elements by:
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
//! | `iter`         | O(1) + O(n)       | O(n) + O(n)         |
//! | `iter_sorted`  | O(n log n)        | O(n) + O(n log n)   |
//!
//! **Note**: For Large state, `iter` and `iter_sorted` incur an additional O(n)
//! allocation cost because `PersistentHashSet::iter` collects elements into a
//! `Vec` first. The `iter_sorted` in Large state performs two allocations:
//! one internal to `PersistentHashSet::iter`, and one in `iter_sorted` for
//! collecting and sorting the elements (sorting is done in-place).
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::OrderedUniqueSet;
//!
//! // Create an empty collection
//! let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
enum OrderedUniqueSetInner<T: Clone + Eq + Hash> {
    /// Empty collection.
    Empty,
    /// Small collection (up to 8 elements) stored inline.
    Small(SmallVec<[T; SMALL_THRESHOLD]>),
    /// Large collection (more than 8 elements) stored in a hash set.
    Large(PersistentHashSet<T>),
}

/// A persistent collection for storing unique elements with automatic state transitions.
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
/// use lambars::persistent::OrderedUniqueSet;
///
/// let collection = OrderedUniqueSet::new()
///     .insert(3)
///     .insert(1)
///     .insert(2);
///
/// // Iteration in sorted order
/// let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
/// assert_eq!(sorted, vec![1, 2, 3]);
/// ```
#[derive(Clone)]
pub struct OrderedUniqueSet<T: Clone + Eq + Hash> {
    inner: OrderedUniqueSetInner<T>,
}

impl<T: Clone + Eq + Hash> OrderedUniqueSet<T> {
    /// Creates a new empty collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    /// assert!(collection.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: OrderedUniqueSetInner::Empty,
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
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(1).insert(2);
    /// assert_eq!(collection.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.inner {
            OrderedUniqueSetInner::Empty => 0,
            OrderedUniqueSetInner::Small(vec) => vec.len(),
            OrderedUniqueSetInner::Large(set) => set.len(),
        }
    }

    /// Returns `true` if the collection contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert(42);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.inner, OrderedUniqueSetInner::Empty)
    }

    /// Returns `true` if the collection contains the specified element.
    ///
    /// This method supports borrowed forms of the element type through the
    /// `Borrow` trait. For example, with `OrderedUniqueSet<String>`, you can
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
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(1).insert(2);
    /// assert!(collection.contains(&1));
    /// assert!(!collection.contains(&3));
    ///
    /// // With String elements, you can search using &str
    /// let strings = OrderedUniqueSet::new()
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
            OrderedUniqueSetInner::Empty => false,
            OrderedUniqueSetInner::Small(vec) => vec.iter().any(|item| item.borrow() == element),
            OrderedUniqueSetInner::Large(set) => set.contains(element),
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
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new();
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
            OrderedUniqueSetInner::Empty => {
                let mut vec = SmallVec::new();
                vec.push(element);
                Self {
                    inner: OrderedUniqueSetInner::Small(vec),
                }
            }
            OrderedUniqueSetInner::Small(vec) => {
                if vec.iter().any(|item| item == &element) {
                    return self.clone();
                }

                if vec.len() >= SMALL_THRESHOLD {
                    let set = vec
                        .iter()
                        .cloned()
                        .fold(PersistentHashSet::new(), |set, item| set.insert(item))
                        .insert(element);
                    Self {
                        inner: OrderedUniqueSetInner::Large(set),
                    }
                } else {
                    let mut new_vec = vec.clone();
                    new_vec.push(element);
                    Self {
                        inner: OrderedUniqueSetInner::Small(new_vec),
                    }
                }
            }
            OrderedUniqueSetInner::Large(set) => {
                if set.contains(&element) {
                    return self.clone();
                }
                Self {
                    inner: OrderedUniqueSetInner::Large(set.insert(element)),
                }
            }
        }
    }

    /// Removes an element from the collection, returning a new collection.
    ///
    /// If the element does not exist, returns a clone of the current collection.
    ///
    /// This method supports borrowed forms of the element type through the
    /// `Borrow` trait. For example, with `OrderedUniqueSet<String>`, you can
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
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    /// let collection = collection.remove(&2);
    ///
    /// assert_eq!(collection.len(), 2);
    /// assert!(!collection.contains(&2));
    ///
    /// // With String elements, you can remove using &str
    /// let strings = OrderedUniqueSet::new()
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
            OrderedUniqueSetInner::Empty => Self {
                inner: OrderedUniqueSetInner::Empty,
            },
            OrderedUniqueSetInner::Small(vec) => {
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

                Self {
                    inner: if new_vec.is_empty() {
                        OrderedUniqueSetInner::Empty
                    } else {
                        OrderedUniqueSetInner::Small(new_vec)
                    },
                }
            }
            OrderedUniqueSetInner::Large(set) => {
                if !set.contains(element) {
                    return self.clone();
                }

                let new_set = set.remove(element);

                if new_set.len() <= SMALL_THRESHOLD {
                    let vec: SmallVec<[T; SMALL_THRESHOLD]> = new_set.iter().cloned().collect();
                    Self {
                        inner: if vec.is_empty() {
                            OrderedUniqueSetInner::Empty
                        } else {
                            OrderedUniqueSetInner::Small(vec)
                        },
                    }
                } else {
                    Self {
                        inner: OrderedUniqueSetInner::Large(new_set),
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
    /// - Small state: O(1) for iterator creation, O(n) for full traversal
    /// - Large state: O(n) for iterator creation (collects elements into `Vec`),
    ///   O(n) for full traversal
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    /// let mut elements: Vec<i32> = collection.iter().copied().collect();
    /// elements.sort();
    /// assert_eq!(elements, vec![1, 2, 3]);
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> OrderedUniqueSetIterator<'_, T> {
        OrderedUniqueSetIterator {
            inner: match &self.inner {
                OrderedUniqueSetInner::Empty => OrderedUniqueSetIteratorInner::Empty,
                OrderedUniqueSetInner::Small(vec) => {
                    OrderedUniqueSetIteratorInner::Small(vec.iter())
                }
                OrderedUniqueSetInner::Large(set) => {
                    OrderedUniqueSetIteratorInner::Large(set.iter())
                }
            },
        }
    }

    /// Returns `true` if the collection is in the Empty state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_empty_state(&self) -> bool {
        matches!(self.inner, OrderedUniqueSetInner::Empty)
    }

    /// Returns `true` if the collection is in the Small state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_small_state(&self) -> bool {
        matches!(self.inner, OrderedUniqueSetInner::Small(_))
    }

    /// Returns `true` if the collection is in the Large state.
    ///
    /// This is primarily useful for testing state transitions.
    #[cfg(test)]
    const fn is_large_state(&self) -> bool {
        matches!(self.inner, OrderedUniqueSetInner::Large(_))
    }
}

impl<T: Clone + Eq + Hash + Ord> OrderedUniqueSet<T> {
    /// Returns an iterator over references to the elements in sorted order.
    ///
    /// Elements are sorted according to their `Ord` implementation.
    ///
    /// # Complexity
    ///
    /// - Small state: O(n log n) for sorting
    /// - Large state: O(n) for element collection + O(n log n) for sorting
    ///
    /// # Memory Allocation
    ///
    /// - Small state (n <= 8): Uses `SmallVec` for temporary sorted storage (no heap allocation)
    /// - Large state (n > 8): Two allocations occur:
    ///   1. `PersistentHashSet::iter` collects elements into a `Vec` internally (O(n))
    ///   2. A second `Vec` is allocated for collecting and sorting (sorting is done in-place)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new()
    ///     .insert(3)
    ///     .insert(1)
    ///     .insert(2);
    ///
    /// let sorted: Vec<&i32> = collection.iter_sorted().collect();
    /// assert_eq!(sorted, vec![&1, &2, &3]);
    /// ```
    #[inline]
    #[must_use]
    pub fn iter_sorted(&self) -> OrderedUniqueSetSortedIterator<'_, T> {
        match &self.inner {
            OrderedUniqueSetInner::Empty => OrderedUniqueSetSortedIterator {
                inner: SortedIteratorInner::Empty,
            },
            OrderedUniqueSetInner::Small(vec) => {
                // Use SmallVec for temporary sorted storage to avoid heap allocation
                let mut sorted: SmallVec<[&T; SMALL_THRESHOLD]> = vec.iter().collect();
                sorted.sort_unstable();
                OrderedUniqueSetSortedIterator {
                    inner: SortedIteratorInner::Small(sorted, 0),
                }
            }
            OrderedUniqueSetInner::Large(set) => {
                // For Large state, we need a Vec since the set can exceed SMALL_THRESHOLD
                let mut elements: Vec<&T> = set.iter().collect();
                elements.sort_unstable();
                OrderedUniqueSetSortedIterator {
                    inner: SortedIteratorInner::Large(elements, 0),
                }
            }
        }
    }

    /// Creates an `OrderedUniqueSet` from a sorted, deduplicated iterator.
    ///
    /// This method provides efficient bulk construction by avoiding per-element
    /// persistent clones. It assumes the input iterator yields strictly increasing
    /// elements (sorted and deduplicated).
    ///
    /// # Preconditions
    ///
    /// - The iterator must yield elements in strictly ascending order
    /// - No duplicate elements are allowed
    ///
    /// In debug builds, these preconditions are validated with `debug_assert!`.
    /// In release builds, invalid input results in undefined behavior (incorrect
    /// collection state).
    ///
    /// # Type Constraints
    ///
    /// `T: Ord` is required for debug assertions to validate ordering.
    ///
    /// # Complexity
    ///
    /// O(n) for both Small and Large paths.
    ///
    /// # Memory Allocation
    ///
    /// - Small (n <= 8): Uses `SmallVec` inline storage, no heap allocation
    /// - Large (n > 8): Allocates a `Vec` then builds `PersistentHashSet`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let sorted_elements = vec![1, 3, 5, 7, 9];
    /// let collection = OrderedUniqueSet::from_sorted_iter(sorted_elements);
    /// assert_eq!(collection.len(), 5);
    /// ```
    #[must_use]
    pub fn from_sorted_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut small_buffer: SmallVec<[T; SMALL_THRESHOLD]> = SmallVec::new();
        let mut iter = iter.into_iter();

        for element in iter.by_ref() {
            #[cfg(debug_assertions)]
            debug_assert!(
                small_buffer.last().is_none_or(|last| last < &element),
                "{}",
                SORTED_INVARIANT_PANIC_MESSAGE
            );

            if small_buffer.len() >= SMALL_THRESHOLD {
                let mut vec: Vec<T> = small_buffer.drain(..).collect();
                vec.push(element);
                vec.extend(iter);

                #[cfg(debug_assertions)]
                debug_assert!(
                    is_strictly_sorted(&vec),
                    "{}",
                    SORTED_INVARIANT_PANIC_MESSAGE
                );

                return Self::from_large_vec(vec);
            }
            small_buffer.push(element);
        }

        if small_buffer.is_empty() {
            Self::new()
        } else {
            Self {
                inner: OrderedUniqueSetInner::Small(small_buffer),
            }
        }
    }

    /// Creates an `OrderedUniqueSet` from a sorted, deduplicated `Vec`.
    ///
    /// This method provides efficient bulk construction by consuming a `Vec<T>`
    /// directly, avoiding extra allocations compared to `from_sorted_iter`.
    ///
    /// # Preconditions
    ///
    /// - The vector must contain elements in strictly ascending order
    /// - No duplicate elements are allowed
    ///
    /// In debug builds, these preconditions are validated with `debug_assert!`.
    /// In release builds, invalid input results in undefined behavior (incorrect
    /// collection state).
    ///
    /// # Type Constraints
    ///
    /// `T: Ord` is required for debug assertions to validate ordering.
    ///
    /// # Complexity
    ///
    /// O(n) for both Small and Large paths.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let sorted_vec = vec![2, 4, 6, 8, 10];
    /// let collection = OrderedUniqueSet::from_sorted_vec(sorted_vec);
    /// assert_eq!(collection.len(), 5);
    /// ```
    #[must_use]
    pub fn from_sorted_vec(vec: Vec<T>) -> Self {
        #[cfg(debug_assertions)]
        debug_assert!(
            is_strictly_sorted(&vec),
            "{}",
            SORTED_INVARIANT_PANIC_MESSAGE
        );

        if vec.is_empty() {
            return Self::new();
        }

        if vec.len() <= SMALL_THRESHOLD {
            let small_vec: SmallVec<[T; SMALL_THRESHOLD]> = SmallVec::from_vec(vec);
            Self {
                inner: OrderedUniqueSetInner::Small(small_vec),
            }
        } else {
            Self::from_large_vec(vec)
        }
    }

    /// Returns a sorted `Vec` containing clones of all elements.
    ///
    /// This method provides a convenient way to extract elements in sorted order
    /// for use with APIs that require `Vec<T>` or slices.
    ///
    /// # Complexity
    ///
    /// - Empty: O(1)
    /// - Small: O(n log n) for sorting (clone is O(n))
    /// - Large: O(n) for iteration + O(n log n) for sorting
    ///
    /// # Memory Allocation
    ///
    /// Allocates a new `Vec<T>` to hold the sorted elements. The original
    /// collection remains unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new()
    ///     .insert(3)
    ///     .insert(1)
    ///     .insert(2);
    /// let sorted = collection.to_sorted_vec();
    /// assert_eq!(sorted, vec![1, 2, 3]);
    /// ```
    #[must_use]
    pub fn to_sorted_vec(&self) -> Vec<T> {
        match &self.inner {
            OrderedUniqueSetInner::Empty => Vec::new(),
            OrderedUniqueSetInner::Small(vec) => {
                let mut result: Vec<T> = vec.iter().cloned().collect();
                result.sort();
                result
            }
            OrderedUniqueSetInner::Large(set) => {
                let mut result: Vec<T> = set.iter().cloned().collect();
                result.sort();
                result
            }
        }
    }

    /// Helper method to construct Large state from a Vec.
    ///
    /// Uses `PersistentHashSet::from_iter` which internally uses `TransientHashSet`
    /// for efficient bulk construction without per-element COW overhead.
    fn from_large_vec(vec: Vec<T>) -> Self {
        let set: PersistentHashSet<T> = vec.into_iter().collect();
        Self {
            inner: OrderedUniqueSetInner::Large(set),
        }
    }
}

impl<T: Clone + Eq + Hash> Default for OrderedUniqueSet<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over references to elements in an `OrderedUniqueSet`.
pub struct OrderedUniqueSetIterator<'a, T: Clone + Eq + Hash> {
    inner: OrderedUniqueSetIteratorInner<'a, T>,
}

enum OrderedUniqueSetIteratorInner<'a, T: Clone + Eq + Hash> {
    Empty,
    Small(std::slice::Iter<'a, T>),
    Large(super::PersistentHashSetIterator<'a, T>),
}

impl<'a, T: Clone + Eq + Hash> Iterator for OrderedUniqueSetIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            OrderedUniqueSetIteratorInner::Empty => None,
            OrderedUniqueSetIteratorInner::Small(iter) => iter.next(),
            OrderedUniqueSetIteratorInner::Large(iter) => iter.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            OrderedUniqueSetIteratorInner::Empty => (0, Some(0)),
            OrderedUniqueSetIteratorInner::Small(iter) => iter.size_hint(),
            OrderedUniqueSetIteratorInner::Large(iter) => iter.size_hint(),
        }
    }
}

impl<T: Clone + Eq + Hash> ExactSizeIterator for OrderedUniqueSetIterator<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            OrderedUniqueSetIteratorInner::Empty => 0,
            OrderedUniqueSetIteratorInner::Small(iter) => iter.len(),
            OrderedUniqueSetIteratorInner::Large(iter) => iter.len(),
        }
    }
}

/// Iterator over references to elements in sorted order.
///
/// This iterator uses different internal representations based on collection size:
/// - Empty: No storage needed
/// - Small: Uses `SmallVec` to avoid heap allocation for small collections
/// - Large: Uses `Vec` for large collections
pub struct OrderedUniqueSetSortedIterator<'a, T> {
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

impl<'a, T> Iterator for OrderedUniqueSetSortedIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            SortedIteratorInner::Empty => None,
            SortedIteratorInner::Small(elements, index) => {
                elements.get(*index).copied().inspect(|_| *index += 1)
            }
            SortedIteratorInner::Large(elements, index) => {
                elements.get(*index).copied().inspect(|_| *index += 1)
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

impl<T> ExactSizeIterator for OrderedUniqueSetSortedIterator<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            SortedIteratorInner::Empty => 0,
            SortedIteratorInner::Small(elements, index) => elements.len() - *index,
            SortedIteratorInner::Large(elements, index) => elements.len() - *index,
        }
    }
}

impl<T: Clone + Eq + Hash + std::fmt::Debug> std::fmt::Debug for OrderedUniqueSet<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Clone + Eq + Hash> PartialEq for OrderedUniqueSet<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        // All elements in self must be in other
        self.iter().all(|element| other.contains(element))
    }
}

impl<T: Clone + Eq + Hash> Eq for OrderedUniqueSet<T> {}

impl<'a, T: Clone + Eq + Hash> IntoIterator for &'a OrderedUniqueSet<T> {
    type Item = &'a T;
    type IntoIter = OrderedUniqueSetIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Message constant for panic when `from_sorted_*` receives invalid input.
const SORTED_INVARIANT_PANIC_MESSAGE: &str =
    "from_sorted_* requires strictly increasing elements (sorted + deduplicated)";

/// Checks if a slice is strictly sorted (ascending) and has no duplicates.
///
/// Returns `true` if the slice is strictly sorted, meaning each element is
/// strictly less than the next. An empty slice or a slice with one element
/// is considered strictly sorted.
///
/// This function is used for debug assertions in bulk construction methods
/// to validate preconditions.
///
/// # Arguments
///
/// * `slice` - The slice to check
///
/// # Returns
///
/// `true` if the slice is strictly sorted, `false` otherwise
///
/// # Complexity
///
/// O(n) where n is the length of the slice.
#[cfg(debug_assertions)]
#[inline]
fn is_strictly_sorted<T: Ord>(slice: &[T]) -> bool {
    slice.windows(2).all(|window| window[0] < window[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_new_creates_empty() {
        let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        assert!(collection.is_empty_state());
    }

    #[rstest]
    fn test_small_threshold_constant() {
        assert_eq!(SMALL_THRESHOLD, 8);
    }

    #[rstest]
    fn test_insert_transitions_empty_to_small() {
        let collection = OrderedUniqueSet::new().insert(1);
        assert!(collection.is_small_state());
    }

    #[rstest]
    fn test_insert_transitions_small_to_large() {
        let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        for i in 1..=9 {
            collection = collection.insert(i);
        }
        assert!(collection.is_large_state());
    }

    #[rstest]
    fn test_remove_transitions_large_to_small() {
        let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        for i in 1..=9 {
            collection = collection.insert(i);
        }
        assert!(collection.is_large_state());

        let collection = collection.remove(&9);
        assert!(collection.is_small_state());
    }

    #[rstest]
    fn test_remove_transitions_small_to_empty() {
        let collection = OrderedUniqueSet::new().insert(1);
        let collection = collection.remove(&1);
        assert!(collection.is_empty_state());
    }

    #[rstest]
    fn test_equality() {
        let collection1 = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
        let collection2 = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
        assert_eq!(collection1, collection2);
    }

    #[rstest]
    fn test_inequality_different_elements() {
        let collection1 = OrderedUniqueSet::new().insert(1).insert(2);
        let collection2 = OrderedUniqueSet::new().insert(1).insert(3);
        assert_ne!(collection1, collection2);
    }

    #[rstest]
    fn test_inequality_different_lengths() {
        let collection1 = OrderedUniqueSet::new().insert(1).insert(2);
        let collection2 = OrderedUniqueSet::new().insert(1);
        assert_ne!(collection1, collection2);
    }

    #[rstest]
    fn test_borrow_contains_with_str() {
        let collection = OrderedUniqueSet::new()
            .insert("apple".to_string())
            .insert("banana".to_string());

        // Search using &str without allocating String
        assert!(collection.contains("apple"));
        assert!(collection.contains("banana"));
        assert!(!collection.contains("cherry"));
    }

    #[rstest]
    fn test_borrow_remove_with_str() {
        let collection = OrderedUniqueSet::new()
            .insert("apple".to_string())
            .insert("banana".to_string());

        // Remove using &str without allocating String
        let collection = collection.remove("apple");
        assert!(!collection.contains("apple"));
        assert!(collection.contains("banana"));
    }

    // =========================================================================
    // from_sorted_iter tests
    // =========================================================================

    #[rstest]
    fn from_sorted_iter_empty_returns_empty_state() {
        let collection: OrderedUniqueSet<i32> =
            OrderedUniqueSet::from_sorted_iter(std::iter::empty());
        assert!(collection.is_empty_state());
        assert_eq!(collection.len(), 0);
    }

    #[rstest]
    #[case::one_element(vec![1])]
    #[case::two_elements(vec![1, 2])]
    #[case::eight_elements(vec![1, 2, 3, 4, 5, 6, 7, 8])]
    fn from_sorted_iter_small_returns_small_state(#[case] elements: Vec<i32>) {
        let collection = OrderedUniqueSet::from_sorted_iter(elements.clone());
        assert!(collection.is_small_state());
        assert_eq!(collection.len(), elements.len());
        for element in &elements {
            assert!(collection.contains(element));
        }
    }

    #[rstest]
    #[case::nine_elements(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])]
    #[case::twenty_elements((1..=20).collect())]
    fn from_sorted_iter_large_returns_large_state(#[case] elements: Vec<i32>) {
        let collection = OrderedUniqueSet::from_sorted_iter(elements.clone());
        assert!(collection.is_large_state());
        assert_eq!(collection.len(), elements.len());
        for element in &elements {
            assert!(collection.contains(element));
        }
    }

    #[rstest]
    fn from_sorted_iter_preserves_all_elements() {
        let elements: Vec<i32> = (1..=15).collect();
        let collection = OrderedUniqueSet::from_sorted_iter(elements.clone());

        let mut collected: Vec<i32> = collection.iter().copied().collect();
        collected.sort_unstable();
        assert_eq!(collected, elements);
    }

    #[rstest]
    fn from_sorted_iter_iter_sorted_yields_ascending_order() {
        let elements: Vec<i32> = (1..=10).collect();
        let collection = OrderedUniqueSet::from_sorted_iter(elements.clone());

        let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
        assert_eq!(sorted, elements);
    }

    #[rstest]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "strictly increasing")]
    fn from_sorted_iter_unsorted_panics_in_debug() {
        let _ = OrderedUniqueSet::from_sorted_iter([3, 1, 2]);
    }

    #[rstest]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "strictly increasing")]
    fn from_sorted_iter_duplicate_panics_in_debug() {
        let _ = OrderedUniqueSet::from_sorted_iter([1, 2, 2, 3]);
    }

    #[rstest]
    fn from_sorted_iter_matches_fold_insert_result() {
        let elements: Vec<i32> = (1..=20).collect();
        let from_iter = OrderedUniqueSet::from_sorted_iter(elements.clone());
        let from_fold = elements
            .into_iter()
            .fold(OrderedUniqueSet::new(), |acc, e| acc.insert(e));

        assert_eq!(from_iter, from_fold);
    }

    // =========================================================================
    // from_sorted_vec tests
    // =========================================================================

    #[rstest]
    fn from_sorted_vec_empty_returns_empty_state() {
        let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::from_sorted_vec(vec![]);
        assert!(collection.is_empty_state());
        assert_eq!(collection.len(), 0);
    }

    #[rstest]
    #[case::one_element(vec![1])]
    #[case::two_elements(vec![1, 2])]
    #[case::eight_elements(vec![1, 2, 3, 4, 5, 6, 7, 8])]
    fn from_sorted_vec_small_returns_small_state(#[case] elements: Vec<i32>) {
        let collection = OrderedUniqueSet::from_sorted_vec(elements.clone());
        assert!(collection.is_small_state());
        assert_eq!(collection.len(), elements.len());
        for element in &elements {
            assert!(collection.contains(element));
        }
    }

    #[rstest]
    #[case::nine_elements(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])]
    #[case::twenty_elements((1..=20).collect())]
    fn from_sorted_vec_large_returns_large_state(#[case] elements: Vec<i32>) {
        let collection = OrderedUniqueSet::from_sorted_vec(elements.clone());
        assert!(collection.is_large_state());
        assert_eq!(collection.len(), elements.len());
        for element in &elements {
            assert!(collection.contains(element));
        }
    }

    #[rstest]
    fn from_sorted_vec_preserves_all_elements() {
        let elements: Vec<i32> = (1..=15).collect();
        let collection = OrderedUniqueSet::from_sorted_vec(elements.clone());

        let mut collected: Vec<i32> = collection.iter().copied().collect();
        collected.sort_unstable();
        assert_eq!(collected, elements);
    }

    #[rstest]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "strictly increasing")]
    fn from_sorted_vec_unsorted_panics_in_debug() {
        let _ = OrderedUniqueSet::from_sorted_vec(vec![3, 1, 2]);
    }

    #[rstest]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "strictly increasing")]
    fn from_sorted_vec_duplicate_panics_in_debug() {
        let _ = OrderedUniqueSet::from_sorted_vec(vec![1, 2, 2, 3]);
    }

    // =========================================================================
    // to_sorted_vec tests
    // =========================================================================

    #[rstest]
    fn to_sorted_vec_empty_returns_empty_vec() {
        let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        assert!(collection.to_sorted_vec().is_empty());
    }

    #[rstest]
    fn to_sorted_vec_small_returns_sorted_vec() {
        // Insert in non-sorted order
        let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
        assert!(collection.is_small_state());

        let sorted = collection.to_sorted_vec();
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    #[rstest]
    fn to_sorted_vec_large_returns_sorted_vec() {
        let mut collection = OrderedUniqueSet::new();
        for i in (1..=10).rev() {
            collection = collection.insert(i);
        }
        assert!(collection.is_large_state());

        let sorted = collection.to_sorted_vec();
        assert_eq!(sorted, (1..=10).collect::<Vec<_>>());
    }

    #[rstest]
    fn to_sorted_vec_preserves_original_collection() {
        let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
        let _ = collection.to_sorted_vec();

        // Original collection should still be usable
        assert_eq!(collection.len(), 3);
        assert!(collection.contains(&1));
        assert!(collection.contains(&2));
        assert!(collection.contains(&3));
    }
}
