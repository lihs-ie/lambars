//! Ordered unique set with automatic state transitions.
//!
//! This module provides [`OrderedUniqueSet`], a persistent collection
//! optimized for storing unique elements with automatic state transitions between
//! small (inline) and large (sorted vec) representations.
//!
//! # Overview
//!
//! `OrderedUniqueSet` provides efficient storage for unique elements by:
//! - Using inline storage (`SmallVec`) for small collections (up to 8 elements)
//! - Automatically promoting to sorted `Vec` when exceeding 8 elements
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
//! | `insert`       | O(n)              | O(n)                |
//! | `remove`       | O(n)              | O(n)                |
//! | `contains`     | O(n)              | O(log n)            |
//! | `len`          | O(1)              | O(1)                |
//! | `is_empty`     | O(1)              | O(1)                |
//! | `iter`         | O(1) + O(n)       | O(1) + O(n)         |
//! | `iter_sorted`  | O(n log n)        | O(1) + O(n)         |
//! | `merge`        | O(n + m)          | O(n + m)            |
//! | `difference`   | O(n + m)          | O(n + m)            |
//! | `intersection` | O(n + m)          | O(n + m)            |
//!
//! **Note**: For Large state, elements are stored in a sorted `Vec`, which enables
//! O(log n) binary search for `contains` and O(n) iteration without additional sorting.
//! Set operations (`merge`, `difference`, `intersection`) use efficient two-pointer
//! algorithms that run in linear time.
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
use std::cmp::Ordering;
use std::hash::Hash;
use std::sync::Arc;

/// The threshold for transitioning between Small and Large states.
/// Collections with more than this many elements use sorted `Vec`.
const SMALL_THRESHOLD: usize = 8;

/// A sorted, deduplicated vector wrapped in `Arc` for structural sharing.
#[derive(Clone)]
struct SortedVec<T>(Arc<Vec<T>>);

impl<T: Clone + Ord> SortedVec<T> {
    #[inline]
    fn from_sorted(vec: Vec<T>) -> Self {
        #[cfg(debug_assertions)]
        debug_assert!(
            is_strictly_sorted(&vec),
            "{}",
            SORTED_INVARIANT_PANIC_MESSAGE
        );
        Self(Arc::new(vec))
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        &self.0
    }

    #[inline]
    fn contains<Q>(&self, element: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.0
            .binary_search_by(|item| item.borrow().cmp(element))
            .is_ok()
    }

    fn insert(&self, element: T) -> Option<Self> {
        match self.0.binary_search(&element) {
            Ok(_) => None,
            Err(position) => {
                let mut new_vec = Vec::with_capacity(self.0.len() + 1);
                new_vec.extend_from_slice(&self.0[..position]);
                new_vec.push(element);
                new_vec.extend_from_slice(&self.0[position..]);
                Some(Self::from_sorted(new_vec))
            }
        }
    }

    fn remove<Q>(&self, element: &Q) -> Option<Self>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.0
            .binary_search_by(|item| item.borrow().cmp(element))
            .ok()
            .map(|position| {
                let mut new_vec = Vec::with_capacity(self.0.len() - 1);
                new_vec.extend_from_slice(&self.0[..position]);
                new_vec.extend_from_slice(&self.0[position + 1..]);
                Self::from_sorted(new_vec)
            })
    }
}

/// Internal representation of the collection state.
#[derive(Clone)]
enum OrderedUniqueSetInner<T: Clone + Eq + Hash + Ord> {
    Empty,
    Small(SmallVec<[T; SMALL_THRESHOLD]>),
    Large(SortedVec<T>),
}

/// A persistent collection for storing unique elements with automatic state transitions.
///
/// This collection automatically transitions between three states based on size:
/// - Empty: No elements
/// - Small: Up to 8 elements stored inline in a `SmallVec`
/// - Large: More than 8 elements stored in a sorted `Vec`
///
/// All operations are immutable and return new instances.
///
/// # Type Parameters
///
/// * `T` - The element type. Must implement `Clone`, `Eq`, `Hash`, and `Ord`.
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
pub struct OrderedUniqueSet<T: Clone + Eq + Hash + Ord> {
    inner: OrderedUniqueSetInner<T>,
}

impl<T: Clone + Eq + Hash + Ord> OrderedUniqueSet<T> {
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
    /// - O(log n) for `Large` state (binary search)
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
        Q: Ord + ?Sized,
    {
        match &self.inner {
            OrderedUniqueSetInner::Empty => false,
            OrderedUniqueSetInner::Small(vec) => vec.iter().any(|item| item.borrow() == element),
            OrderedUniqueSetInner::Large(sorted_vec) => sorted_vec.contains(element),
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
    /// - O(n) for `Large` state (binary search + Vec rebuild)
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
                    // Transition to Large state: create sorted vec
                    let mut sorted: Vec<T> = vec.iter().cloned().collect();
                    sorted.push(element);
                    sorted.sort();
                    Self {
                        inner: OrderedUniqueSetInner::Large(SortedVec::from_sorted(sorted)),
                    }
                } else {
                    let mut new_vec = vec.clone();
                    new_vec.push(element);
                    Self {
                        inner: OrderedUniqueSetInner::Small(new_vec),
                    }
                }
            }
            OrderedUniqueSetInner::Large(sorted_vec) => sorted_vec.insert(element).map_or_else(
                || self.clone(),
                |new_sorted_vec| Self {
                    inner: OrderedUniqueSetInner::Large(new_sorted_vec),
                },
            ),
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
    /// - O(n) for `Large` state (binary search + Vec rebuild)
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
        Q: Ord + ?Sized,
    {
        match &self.inner {
            OrderedUniqueSetInner::Empty => self.clone(),
            OrderedUniqueSetInner::Small(vec) => {
                let matches = |item: &T| T::borrow(item) == element;
                if !vec.iter().any(matches) {
                    return self.clone();
                }

                let new_vec: SmallVec<[T; SMALL_THRESHOLD]> =
                    vec.iter().filter(|item| !matches(item)).cloned().collect();

                Self {
                    inner: if new_vec.is_empty() {
                        OrderedUniqueSetInner::Empty
                    } else {
                        OrderedUniqueSetInner::Small(new_vec)
                    },
                }
            }
            OrderedUniqueSetInner::Large(sorted_vec) => sorted_vec.remove(element).map_or_else(
                || self.clone(),
                |new_sorted_vec| {
                    if new_sorted_vec.len() <= SMALL_THRESHOLD {
                        let vec: SmallVec<[T; SMALL_THRESHOLD]> =
                            new_sorted_vec.as_slice().iter().cloned().collect();
                        Self {
                            inner: if vec.is_empty() {
                                OrderedUniqueSetInner::Empty
                            } else {
                                OrderedUniqueSetInner::Small(vec)
                            },
                        }
                    } else {
                        Self {
                            inner: OrderedUniqueSetInner::Large(new_sorted_vec),
                        }
                    }
                },
            ),
        }
    }

    /// Returns an iterator over references to the elements.
    ///
    /// For Large state, elements are returned in sorted (ascending) order.
    /// For Small state, order is not guaranteed.
    ///
    /// # Complexity
    ///
    /// - Small state: O(1) for iterator creation, O(n) for full traversal
    /// - Large state: O(1) for iterator creation, O(n) for full traversal
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
                OrderedUniqueSetInner::Large(sorted_vec) => {
                    OrderedUniqueSetIteratorInner::Large(sorted_vec.as_slice().iter())
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

    /// Returns a reference to the smallest element in sorted order, or `None` if empty.
    ///
    /// # Complexity
    ///
    /// - Empty: O(1)
    /// - Small state (n <= 8): O(n) linear scan
    /// - Large state (n > 8): O(1) direct slice access
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
    /// assert_eq!(collection.first_sorted(), Some(&1));
    ///
    /// let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    /// assert_eq!(empty.first_sorted(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn first_sorted(&self) -> Option<&T> {
        match &self.inner {
            OrderedUniqueSetInner::Empty => None,
            OrderedUniqueSetInner::Small(vec) => vec.iter().min(),
            OrderedUniqueSetInner::Large(sorted_vec) => sorted_vec.as_slice().first(),
        }
    }

    /// Returns a reference to the largest element in sorted order, or `None` if empty.
    ///
    /// # Complexity
    ///
    /// - Empty: O(1)
    /// - Small state (n <= 8): O(n) linear scan
    /// - Large state (n > 8): O(1) direct slice access
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
    /// assert_eq!(collection.last_sorted(), Some(&3));
    ///
    /// let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    /// assert_eq!(empty.last_sorted(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn last_sorted(&self) -> Option<&T> {
        match &self.inner {
            OrderedUniqueSetInner::Empty => None,
            OrderedUniqueSetInner::Small(vec) => vec.iter().max(),
            OrderedUniqueSetInner::Large(sorted_vec) => sorted_vec.as_slice().last(),
        }
    }

    /// Returns a reference to the underlying sorted slice if in `Large` state.
    ///
    /// This method provides zero-copy access to the sorted elements when the
    /// collection is in the `Large` state (more than 8 elements). Returns `None`
    /// for `Empty` and `Small` states, where elements are not stored as a sorted
    /// contiguous slice.
    ///
    /// # Complexity
    ///
    /// O(1) in all cases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let small = OrderedUniqueSet::new().insert(1).insert(2);
    /// assert!(small.as_sorted_slice().is_none());
    ///
    /// let large = OrderedUniqueSet::from_sorted_iter(1..=20);
    /// let slice = large.as_sorted_slice().unwrap();
    /// assert_eq!(slice.len(), 20);
    /// assert_eq!(slice[0], 1);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_sorted_slice(&self) -> Option<&[T]> {
        match &self.inner {
            OrderedUniqueSetInner::Large(sorted_vec) => Some(sorted_vec.as_slice()),
            _ => None,
        }
    }

    /// Returns an iterator over references to the elements in sorted order.
    ///
    /// Elements are sorted according to their `Ord` implementation.
    ///
    /// # Complexity
    ///
    /// - Small state: O(n log n) for sorting
    /// - Large state: O(1) for iterator creation, O(n) for full traversal
    ///   (no sorting needed - elements are already sorted)
    ///
    /// # Memory Allocation
    ///
    /// - Small state (n <= 8): Uses `SmallVec` for temporary sorted storage (no heap allocation)
    /// - Large state (n > 8): No allocation (iterator over sorted slice)
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
            OrderedUniqueSetInner::Large(sorted_vec) => {
                // For Large state, elements are already sorted - just iterate over slice
                OrderedUniqueSetSortedIterator {
                    inner: SortedIteratorInner::Large(sorted_vec.as_slice().iter()),
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
    /// In release builds, invalid input yields an incorrect collection state
    /// (logic error, not memory unsafety).
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
    /// - Large (n > 8): Allocates a `Vec` wrapped in `SortedVec` for structural sharing
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
                let buffered_len = small_buffer.len();
                let (lower, _) = iter.size_hint();
                let mut vec = Vec::with_capacity(buffered_len + 1 + lower);
                vec.extend(small_buffer.drain(..));
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
    /// In release builds, invalid input yields an incorrect collection state
    /// (logic error, not memory unsafety).
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
            Self {
                inner: OrderedUniqueSetInner::Small(SmallVec::from_vec(vec)),
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
    /// - Large: O(n) for clone (already sorted)
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
            OrderedUniqueSetInner::Large(sorted_vec) => {
                // Already sorted, just clone
                sorted_vec.as_slice().to_vec()
            }
        }
    }

    /// Helper method to construct Large state from a sorted Vec.
    ///
    /// The input vec must already be sorted and deduplicated.
    fn from_large_vec(vec: Vec<T>) -> Self {
        Self {
            inner: OrderedUniqueSetInner::Large(SortedVec::from_sorted(vec)),
        }
    }

    /// Merges two sets, returning a new set containing all elements from both.
    ///
    /// This operation is equivalent to set union. Duplicate elements are
    /// included only once in the result.
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two sets.
    /// Uses an index-based two-pointer merge algorithm with a disjoint fast path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let set1 = OrderedUniqueSet::from_sorted_iter([1, 3, 5]);
    /// let set2 = OrderedUniqueSet::from_sorted_iter([2, 3, 4]);
    /// let merged = set1.merge(&set2);
    /// assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        if self.is_empty() {
            return other.clone();
        }
        if other.is_empty() {
            return self.clone();
        }

        let left_owned;
        let left_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &self.inner {
            sorted_vec.as_slice()
        } else {
            left_owned = self.to_sorted_vec();
            left_owned.as_slice()
        };

        let right_owned;
        let right_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &other.inner {
            sorted_vec.as_slice()
        } else {
            right_owned = other.to_sorted_vec();
            right_owned.as_slice()
        };

        let result = merge_slices(left_slice, right_slice);
        Self::from_sorted_vec(result)
    }

    /// Returns the set difference (self - other).
    ///
    /// Returns a new set containing elements that are in self but not in other.
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two sets.
    /// Uses an index-based two-pointer algorithm with a disjoint fast path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
    /// let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);
    /// let diff = set1.difference(&set2);
    /// assert_eq!(diff.to_sorted_vec(), vec![1, 2]);
    /// ```
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return self.clone();
        }

        let left_owned;
        let left_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &self.inner {
            sorted_vec.as_slice()
        } else {
            left_owned = self.to_sorted_vec();
            left_owned.as_slice()
        };

        let right_owned;
        let right_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &other.inner {
            sorted_vec.as_slice()
        } else {
            right_owned = other.to_sorted_vec();
            right_owned.as_slice()
        };

        let result = difference_slices(left_slice, right_slice);
        Self::from_sorted_vec(result)
    }

    /// Returns the set intersection (self & other).
    ///
    /// Returns a new set containing elements that are in both self and other.
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two sets.
    /// Uses an index-based two-pointer algorithm with a disjoint fast path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::OrderedUniqueSet;
    ///
    /// let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
    /// let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);
    /// let inter = set1.intersection(&set2);
    /// assert_eq!(inter.to_sorted_vec(), vec![3, 4, 5]);
    /// ```
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new();
        }

        let left_owned;
        let left_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &self.inner {
            sorted_vec.as_slice()
        } else {
            left_owned = self.to_sorted_vec();
            left_owned.as_slice()
        };

        let right_owned;
        let right_slice = if let OrderedUniqueSetInner::Large(sorted_vec) = &other.inner {
            sorted_vec.as_slice()
        } else {
            right_owned = other.to_sorted_vec();
            right_owned.as_slice()
        };

        let result = intersection_slices(left_slice, right_slice);
        Self::from_sorted_vec(result)
    }
}

impl<T: Clone + Eq + Hash + Ord> Default for OrderedUniqueSet<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over references to elements in an `OrderedUniqueSet`.
pub struct OrderedUniqueSetIterator<'a, T> {
    inner: OrderedUniqueSetIteratorInner<'a, T>,
}

enum OrderedUniqueSetIteratorInner<'a, T> {
    Empty,
    Small(std::slice::Iter<'a, T>),
    Large(std::slice::Iter<'a, T>),
}

impl<'a, T> Iterator for OrderedUniqueSetIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            OrderedUniqueSetIteratorInner::Empty => None,
            OrderedUniqueSetIteratorInner::Small(iter)
            | OrderedUniqueSetIteratorInner::Large(iter) => iter.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            OrderedUniqueSetIteratorInner::Empty => (0, Some(0)),
            OrderedUniqueSetIteratorInner::Small(iter)
            | OrderedUniqueSetIteratorInner::Large(iter) => iter.size_hint(),
        }
    }
}

impl<T> ExactSizeIterator for OrderedUniqueSetIterator<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            OrderedUniqueSetIteratorInner::Empty => 0,
            OrderedUniqueSetIteratorInner::Small(iter)
            | OrderedUniqueSetIteratorInner::Large(iter) => iter.len(),
        }
    }
}

/// Iterator over references to elements in sorted order.
///
/// This iterator uses different internal representations based on collection size:
/// - Empty: No storage needed
/// - Small: Uses `SmallVec` to avoid heap allocation for small collections
/// - Large: Direct slice iterator (no allocation, already sorted)
pub struct OrderedUniqueSetSortedIterator<'a, T> {
    inner: SortedIteratorInner<'a, T>,
}

enum SortedIteratorInner<'a, T> {
    Empty,
    Small(SmallVec<[&'a T; SMALL_THRESHOLD]>, usize),
    Large(std::slice::Iter<'a, T>),
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
            SortedIteratorInner::Large(iter) => iter.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = match &self.inner {
            SortedIteratorInner::Empty => 0,
            SortedIteratorInner::Small(elements, index) => elements.len() - *index,
            SortedIteratorInner::Large(iter) => iter.len(),
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
            SortedIteratorInner::Large(iter) => iter.len(),
        }
    }
}

impl<T: Clone + Eq + Hash + Ord + std::fmt::Debug> std::fmt::Debug for OrderedUniqueSet<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Clone + Eq + Hash + Ord> PartialEq for OrderedUniqueSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|element| other.contains(element))
    }
}

impl<T: Clone + Eq + Hash + Ord> Eq for OrderedUniqueSet<T> {}

impl<'a, T: Clone + Eq + Hash + Ord> IntoIterator for &'a OrderedUniqueSet<T> {
    type Item = &'a T;
    type IntoIter = OrderedUniqueSetIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Merges two sorted, deduplicated slices into a new sorted, deduplicated `Vec`.
///
/// Uses an index-based two-pointer algorithm with an integrated disjoint fast path.
/// When the ranges do not overlap (`left.last() < right.first()` or vice versa),
/// the comparison loop is skipped entirely and elements are concatenated directly.
///
/// # Preconditions
///
/// Both `left` and `right` must be sorted in strictly ascending order (no duplicates).
///
/// # Complexity
///
/// O(n + m) where n = `left.len()`, m = `right.len()`.
/// Disjoint case: two `extend_from_slice` calls (no per-element comparison).
fn merge_slices<T: Clone + Ord>(left: &[T], right: &[T]) -> Vec<T> {
    if left.is_empty() {
        return right.to_vec();
    }
    if right.is_empty() {
        return left.to_vec();
    }

    // Disjoint fast path: no overlap between ranges
    // SAFETY: Both slices are non-empty (checked above), so last()/first() are safe.
    if left.last().unwrap() < right.first().unwrap() {
        let mut result = Vec::with_capacity(left.len() + right.len());
        result.extend_from_slice(left);
        result.extend_from_slice(right);
        return result;
    }
    if right.last().unwrap() < left.first().unwrap() {
        let mut result = Vec::with_capacity(left.len() + right.len());
        result.extend_from_slice(right);
        result.extend_from_slice(left);
        return result;
    }

    // General two-pointer merge with deduplication
    let mut result = Vec::with_capacity(left.len() + right.len());
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                result.push(left[left_index].clone());
                left_index += 1;
            }
            Ordering::Greater => {
                result.push(right[right_index].clone());
                right_index += 1;
            }
            Ordering::Equal => {
                result.push(left[left_index].clone());
                left_index += 1;
                right_index += 1;
            }
        }
    }

    // Tail: copy remaining elements in bulk
    if left_index < left.len() {
        result.extend_from_slice(&left[left_index..]);
    }
    if right_index < right.len() {
        result.extend_from_slice(&right[right_index..]);
    }

    result
}

/// Computes the set difference of two sorted, deduplicated slices.
///
/// Returns a new sorted `Vec` containing elements that are in `left` but not in `right`.
///
/// Uses an index-based two-pointer algorithm with a disjoint fast path.
///
/// # Preconditions
///
/// Both `left` and `right` must be sorted in strictly ascending order (no duplicates).
///
/// # Complexity
///
/// O(n + m) where n = `left.len()`, m = `right.len()`.
fn difference_slices<T: Clone + Ord>(left: &[T], right: &[T]) -> Vec<T> {
    if left.is_empty() {
        return Vec::new();
    }
    if right.is_empty() {
        return left.to_vec();
    }

    // Disjoint fast path: if ranges don't overlap, left is entirely the result
    if left.last().unwrap() < right.first().unwrap()
        || right.last().unwrap() < left.first().unwrap()
    {
        return left.to_vec();
    }

    let mut result = Vec::with_capacity(left.len());
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                result.push(left[left_index].clone());
                left_index += 1;
            }
            Ordering::Greater => {
                right_index += 1;
            }
            Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
        }
    }

    // Remaining left elements are all in the difference
    if left_index < left.len() {
        result.extend_from_slice(&left[left_index..]);
    }

    result
}

/// Computes the set intersection of two sorted, deduplicated slices.
///
/// Returns a new sorted `Vec` containing elements that are in both `left` and `right`.
///
/// Uses an index-based two-pointer algorithm with a disjoint fast path.
///
/// # Preconditions
///
/// Both `left` and `right` must be sorted in strictly ascending order (no duplicates).
///
/// # Complexity
///
/// O(n + m) where n = `left.len()`, m = `right.len()`.
fn intersection_slices<T: Clone + Ord>(left: &[T], right: &[T]) -> Vec<T> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }

    // Disjoint fast path: if ranges don't overlap, intersection is empty
    if left.last().unwrap() < right.first().unwrap()
        || right.last().unwrap() < left.first().unwrap()
    {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(left.len().min(right.len()));
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                left_index += 1;
            }
            Ordering::Greater => {
                right_index += 1;
            }
            Ordering::Equal => {
                result.push(left[left_index].clone());
                left_index += 1;
                right_index += 1;
            }
        }
    }

    result
}

/// Message constant for panic when `from_sorted_*` receives invalid input.
const SORTED_INVARIANT_PANIC_MESSAGE: &str =
    "from_sorted_* requires strictly increasing elements (sorted + deduplicated)";

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

    // =========================================================================
    // SortedVec Large representation tests (Phase 1-1)
    // =========================================================================

    #[rstest]
    fn large_iter_sorted_returns_ascending_order_without_sort() {
        // When Large uses SortedVec, iter_sorted should be O(n) not O(n log n)
        let collection = OrderedUniqueSet::from_sorted_iter(1..=20);
        assert!(collection.is_large_state());

        // Should return in ascending order
        let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
        assert_eq!(sorted, (1..=20).collect::<Vec<_>>());
    }

    #[rstest]
    fn large_iter_returns_ascending_order() {
        // With SortedVec representation, iter should also return ascending order
        let collection = OrderedUniqueSet::from_sorted_iter(1..=15);
        assert!(collection.is_large_state());

        let elements: Vec<i32> = collection.iter().copied().collect();
        // Elements should be in ascending order (SortedVec property)
        assert_eq!(elements, (1..=15).collect::<Vec<_>>());
    }

    #[rstest]
    fn large_contains_uses_binary_search() {
        // This test verifies the behavior, not the implementation
        // Binary search should correctly find elements
        let collection = OrderedUniqueSet::from_sorted_iter(1..=1000);
        assert!(collection.is_large_state());

        // Check various elements - all in range should be found
        assert!(collection.contains(&1));
        assert!(collection.contains(&500));
        assert!(collection.contains(&501));
        assert!(collection.contains(&1000));
        // Elements outside range should not be found
        assert!(!collection.contains(&0));
        assert!(!collection.contains(&1001));
    }

    #[rstest]
    fn large_contains_finds_all_elements() {
        let collection = OrderedUniqueSet::from_sorted_iter(1..=100);
        assert!(collection.is_large_state());

        for i in 1..=100 {
            assert!(collection.contains(&i), "Should contain {i}");
        }
        assert!(!collection.contains(&0));
        assert!(!collection.contains(&101));
    }

    // =========================================================================
    // merge tests (Phase 1-2)
    // =========================================================================

    #[rstest]
    fn merge_empty_with_empty_returns_empty() {
        let empty1: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let empty2: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let merged = empty1.merge(&empty2);
        assert!(merged.is_empty());
        assert!(merged.is_empty_state());
    }

    #[rstest]
    fn merge_empty_with_non_empty_returns_other() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);

        let merged = empty.merge(&non_empty);
        assert_eq!(merged.len(), 5);
        assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn merge_non_empty_with_empty_returns_self() {
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let merged = non_empty.merge(&empty);
        assert_eq!(merged.len(), 5);
        assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn merge_disjoint_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 3, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([2, 4, 6]);

        let merged = set1.merge(&set2);
        assert_eq!(merged.len(), 6);
        assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn merge_overlapping_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);

        let merged = set1.merge(&set2);
        assert_eq!(merged.len(), 7);
        assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[rstest]
    fn merge_identical_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);

        let merged = set1.merge(&set2);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.to_sorted_vec(), vec![1, 2, 3]);
    }

    #[rstest]
    fn merge_small_with_large_returns_large() {
        let small = OrderedUniqueSet::from_sorted_iter([1, 2, 3]); // Small state
        let large = OrderedUniqueSet::from_sorted_iter(10..=20); // Large state

        assert!(small.is_small_state());
        assert!(large.is_large_state());

        let merged = small.merge(&large);
        // Result should be Large since total > 8
        assert!(merged.is_large_state());
        assert_eq!(merged.len(), 3 + 11);
    }

    #[rstest]
    fn merge_is_commutative() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 3, 5, 7, 9]);
        let set2 = OrderedUniqueSet::from_sorted_iter([2, 4, 6, 8, 10]);

        let merged1 = set1.merge(&set2);
        let merged2 = set2.merge(&set1);

        assert_eq!(merged1, merged2);
    }

    #[rstest]
    fn merge_preserves_original_collections() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([4, 5, 6]);

        let _ = set1.merge(&set2);

        // Original collections unchanged (immutability)
        assert_eq!(set1.to_sorted_vec(), vec![1, 2, 3]);
        assert_eq!(set2.to_sorted_vec(), vec![4, 5, 6]);
    }

    // =========================================================================
    // difference tests (Phase 1-3)
    // =========================================================================

    #[rstest]
    fn difference_empty_with_empty_returns_empty() {
        let empty1: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let empty2: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let diff = empty1.difference(&empty2);
        assert!(diff.is_empty());
    }

    #[rstest]
    fn difference_empty_with_non_empty_returns_empty() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);

        let diff = empty.difference(&non_empty);
        assert!(diff.is_empty());
    }

    #[rstest]
    fn difference_non_empty_with_empty_returns_self() {
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let diff = non_empty.difference(&empty);
        assert_eq!(diff.len(), 5);
        assert_eq!(diff.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn difference_disjoint_sets_returns_self() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([4, 5, 6]);

        let diff = set1.difference(&set2);
        assert_eq!(diff.len(), 3);
        assert_eq!(diff.to_sorted_vec(), vec![1, 2, 3]);
    }

    #[rstest]
    fn difference_overlapping_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);

        let diff = set1.difference(&set2);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff.to_sorted_vec(), vec![1, 2]);
    }

    #[rstest]
    fn difference_identical_sets_returns_empty() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);

        let diff = set1.difference(&set2);
        assert!(diff.is_empty());
    }

    #[rstest]
    fn difference_subset_returns_empty() {
        let set1 = OrderedUniqueSet::from_sorted_iter([2, 3, 4]);
        let set2 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);

        let diff = set1.difference(&set2);
        assert!(diff.is_empty());
    }

    #[rstest]
    fn difference_superset_returns_difference() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([2, 3, 4]);

        let diff = set1.difference(&set2);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff.to_sorted_vec(), vec![1, 5]);
    }

    #[rstest]
    fn difference_preserves_original_collections() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5]);

        let _ = set1.difference(&set2);

        // Original collections unchanged
        assert_eq!(set1.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
        assert_eq!(set2.to_sorted_vec(), vec![3, 4, 5]);
    }

    // =========================================================================
    // intersection tests (Phase 1-4)
    // =========================================================================

    #[rstest]
    fn intersection_empty_with_empty_returns_empty() {
        let empty1: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let empty2: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let inter = empty1.intersection(&empty2);
        assert!(inter.is_empty());
    }

    #[rstest]
    fn intersection_empty_with_non_empty_returns_empty() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);

        let inter = empty.intersection(&non_empty);
        assert!(inter.is_empty());
    }

    #[rstest]
    fn intersection_non_empty_with_empty_returns_empty() {
        let non_empty = OrderedUniqueSet::from_sorted_iter(1..=5);
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();

        let inter = non_empty.intersection(&empty);
        assert!(inter.is_empty());
    }

    #[rstest]
    fn intersection_disjoint_sets_returns_empty() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([4, 5, 6]);

        let inter = set1.intersection(&set2);
        assert!(inter.is_empty());
    }

    #[rstest]
    fn intersection_overlapping_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);

        let inter = set1.intersection(&set2);
        assert_eq!(inter.len(), 3);
        assert_eq!(inter.to_sorted_vec(), vec![3, 4, 5]);
    }

    #[rstest]
    fn intersection_identical_sets() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);
        let set2 = OrderedUniqueSet::from_sorted_iter([1, 2, 3]);

        let inter = set1.intersection(&set2);
        assert_eq!(inter.len(), 3);
        assert_eq!(inter.to_sorted_vec(), vec![1, 2, 3]);
    }

    #[rstest]
    fn intersection_subset_returns_subset() {
        let set1 = OrderedUniqueSet::from_sorted_iter([2, 3, 4]);
        let set2 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);

        let inter = set1.intersection(&set2);
        assert_eq!(inter.len(), 3);
        assert_eq!(inter.to_sorted_vec(), vec![2, 3, 4]);
    }

    #[rstest]
    fn intersection_is_commutative() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);

        let inter1 = set1.intersection(&set2);
        let inter2 = set2.intersection(&set1);

        assert_eq!(inter1, inter2);
    }

    #[rstest]
    fn intersection_preserves_original_collections() {
        let set1 = OrderedUniqueSet::from_sorted_iter([1, 2, 3, 4, 5]);
        let set2 = OrderedUniqueSet::from_sorted_iter([3, 4, 5, 6, 7]);

        let _ = set1.intersection(&set2);

        // Original collections unchanged
        assert_eq!(set1.to_sorted_vec(), vec![1, 2, 3, 4, 5]);
        assert_eq!(set2.to_sorted_vec(), vec![3, 4, 5, 6, 7]);
    }

    // =========================================================================
    // first_sorted / last_sorted tests
    // =========================================================================

    #[rstest]
    fn first_sorted_empty_returns_none() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        assert_eq!(empty.first_sorted(), None);
    }

    #[rstest]
    fn last_sorted_empty_returns_none() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        assert_eq!(empty.last_sorted(), None);
    }

    #[rstest]
    fn first_sorted_small_returns_minimum() {
        let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
        assert!(!collection.is_large_state());
        assert_eq!(collection.first_sorted(), Some(&1));
    }

    #[rstest]
    fn last_sorted_small_returns_maximum() {
        let collection = OrderedUniqueSet::new().insert(3).insert(1).insert(2);
        assert!(!collection.is_large_state());
        assert_eq!(collection.last_sorted(), Some(&3));
    }

    #[rstest]
    fn first_sorted_large_returns_minimum() {
        let collection = (1..=20).fold(OrderedUniqueSet::new(), |set, value| set.insert(value));
        assert!(collection.is_large_state());
        assert_eq!(collection.first_sorted(), Some(&1));
    }

    #[rstest]
    fn last_sorted_large_returns_maximum() {
        let collection = (1..=20).fold(OrderedUniqueSet::new(), |set, value| set.insert(value));
        assert!(collection.is_large_state());
        assert_eq!(collection.last_sorted(), Some(&20));
    }

    #[rstest]
    fn first_sorted_single_element() {
        let collection = OrderedUniqueSet::new().insert(42);
        assert_eq!(collection.first_sorted(), Some(&42));
    }

    #[rstest]
    fn last_sorted_single_element() {
        let collection = OrderedUniqueSet::new().insert(42);
        assert_eq!(collection.last_sorted(), Some(&42));
    }

    #[rstest]
    fn first_last_sorted_consistent_with_iter_sorted() {
        let collection = OrderedUniqueSet::from_sorted_iter([5, 10, 15, 20, 25]);
        let sorted: Vec<&i32> = collection.iter_sorted().collect();
        assert_eq!(collection.first_sorted(), sorted.first().copied());
        assert_eq!(collection.last_sorted(), sorted.last().copied());
    }

    #[rstest]
    fn first_last_sorted_large_consistent_with_iter_sorted() {
        let collection = OrderedUniqueSet::from_sorted_iter(1..=100);
        assert!(collection.is_large_state());
        let sorted: Vec<&i32> = collection.iter_sorted().collect();
        assert_eq!(collection.first_sorted(), sorted.first().copied());
        assert_eq!(collection.last_sorted(), sorted.last().copied());
    }

    // =========================================================================
    // as_sorted_slice tests
    // =========================================================================

    #[rstest]
    fn as_sorted_slice_empty_returns_none() {
        let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
        assert!(empty.as_sorted_slice().is_none());
    }

    #[rstest]
    fn as_sorted_slice_small_returns_none() {
        let small = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
        assert!(!small.is_large_state());
        assert!(small.as_sorted_slice().is_none());
    }

    #[rstest]
    fn as_sorted_slice_large_returns_sorted_slice() {
        let large = OrderedUniqueSet::from_sorted_iter(1..=20);
        assert!(large.is_large_state());
        let slice = large
            .as_sorted_slice()
            .expect("should return Some for Large state");
        assert_eq!(slice.len(), 20);
        assert_eq!(slice[0], 1);
        assert_eq!(slice[19], 20);
    }

    #[rstest]
    fn as_sorted_slice_consistent_with_iter_sorted() {
        let collection = OrderedUniqueSet::from_sorted_iter(1..=50);
        assert!(collection.is_large_state());
        let slice = collection.as_sorted_slice().unwrap();
        let from_iterator: Vec<&i32> = collection.iter_sorted().collect();
        assert_eq!(slice.len(), from_iterator.len());
        for (slice_element, iterator_element) in slice.iter().zip(from_iterator.iter()) {
            assert_eq!(slice_element, *iterator_element);
        }
    }

    // =========================================================================
    // merge_slices tests
    // =========================================================================

    #[rstest]
    fn merge_slices_both_empty() {
        let result: Vec<i32> = merge_slices(&[], &[]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn merge_slices_left_empty() {
        let result = merge_slices(&[], &[1, 2, 3]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn merge_slices_right_empty() {
        let result = merge_slices(&[1, 2, 3], &[]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn merge_slices_single_elements_disjoint() {
        let result = merge_slices(&[1], &[2]);
        assert_eq!(result, vec![1, 2]);
    }

    #[rstest]
    fn merge_slices_single_elements_duplicate() {
        let result = merge_slices(&[1], &[1]);
        assert_eq!(result, vec![1]);
    }

    #[rstest]
    fn merge_slices_disjoint_left_before_right() {
        let result = merge_slices(&[1, 2, 3], &[4, 5, 6]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn merge_slices_disjoint_right_before_left() {
        let result = merge_slices(&[4, 5, 6], &[1, 2, 3]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn merge_slices_overlapping() {
        let result = merge_slices(&[1, 3, 5, 7], &[2, 3, 6, 8]);
        assert_eq!(result, vec![1, 2, 3, 5, 6, 7, 8]);
    }

    #[rstest]
    fn merge_slices_fully_overlapping() {
        let result = merge_slices(&[1, 2, 3], &[1, 2, 3]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn merge_slices_interleaved() {
        let result = merge_slices(&[1, 3, 5], &[2, 4, 6]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn merge_slices_large() {
        let left: Vec<i32> = (1..=1000).step_by(2).collect(); // odd: 1,3,5,...,999
        let right: Vec<i32> = (2..=1000).step_by(2).collect(); // even: 2,4,6,...,1000
        let result = merge_slices(&left, &right);
        let expected: Vec<i32> = (1..=1000).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn merge_slices_large_with_overlap() {
        let left: Vec<i32> = (1..=500).collect();
        let right: Vec<i32> = (250..=750).collect();
        let result = merge_slices(&left, &right);
        let expected: Vec<i32> = (1..=750).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn merge_slices_subset() {
        let result = merge_slices(&[2, 3, 4], &[1, 2, 3, 4, 5]);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    // =========================================================================
    // difference_slices tests
    // =========================================================================

    #[rstest]
    fn difference_slices_both_empty() {
        let result: Vec<i32> = difference_slices(&[], &[]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn difference_slices_left_empty() {
        let result = difference_slices(&[], &[1, 2, 3]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn difference_slices_right_empty() {
        let result = difference_slices(&[1, 2, 3], &[]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn difference_slices_single_elements_different() {
        let result = difference_slices(&[1], &[2]);
        assert_eq!(result, vec![1]);
    }

    #[rstest]
    fn difference_slices_single_elements_same() {
        let result = difference_slices(&[1], &[1]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn difference_slices_disjoint_left_before_right() {
        let result = difference_slices(&[1, 2, 3], &[4, 5, 6]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn difference_slices_disjoint_right_before_left() {
        let result = difference_slices(&[4, 5, 6], &[1, 2, 3]);
        assert_eq!(result, vec![4, 5, 6]);
    }

    #[rstest]
    fn difference_slices_overlapping() {
        let result = difference_slices(&[1, 2, 3, 4, 5], &[3, 4, 5, 6, 7]);
        assert_eq!(result, vec![1, 2]);
    }

    #[rstest]
    fn difference_slices_fully_overlapping() {
        let result = difference_slices(&[1, 2, 3], &[1, 2, 3]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn difference_slices_subset() {
        let result = difference_slices(&[2, 3, 4], &[1, 2, 3, 4, 5]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn difference_slices_superset() {
        let result = difference_slices(&[1, 2, 3, 4, 5], &[2, 3, 4]);
        assert_eq!(result, vec![1, 5]);
    }

    #[rstest]
    fn difference_slices_large() {
        let left: Vec<i32> = (1..=1000).collect();
        let right: Vec<i32> = (501..=1500).collect();
        let result = difference_slices(&left, &right);
        let expected: Vec<i32> = (1..=500).collect();
        assert_eq!(result, expected);
    }

    // =========================================================================
    // intersection_slices tests
    // =========================================================================

    #[rstest]
    fn intersection_slices_both_empty() {
        let result: Vec<i32> = intersection_slices(&[], &[]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_left_empty() {
        let result = intersection_slices(&[], &[1, 2, 3]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_right_empty() {
        let result = intersection_slices(&[1, 2, 3], &[]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_single_elements_different() {
        let result = intersection_slices(&[1], &[2]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_single_elements_same() {
        let result = intersection_slices(&[1], &[1]);
        assert_eq!(result, vec![1]);
    }

    #[rstest]
    fn intersection_slices_disjoint_left_before_right() {
        let result = intersection_slices(&[1, 2, 3], &[4, 5, 6]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_disjoint_right_before_left() {
        let result = intersection_slices(&[4, 5, 6], &[1, 2, 3]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn intersection_slices_overlapping() {
        let result = intersection_slices(&[1, 2, 3, 4, 5], &[3, 4, 5, 6, 7]);
        assert_eq!(result, vec![3, 4, 5]);
    }

    #[rstest]
    fn intersection_slices_fully_overlapping() {
        let result = intersection_slices(&[1, 2, 3], &[1, 2, 3]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn intersection_slices_subset() {
        let result = intersection_slices(&[2, 3, 4], &[1, 2, 3, 4, 5]);
        assert_eq!(result, vec![2, 3, 4]);
    }

    #[rstest]
    fn intersection_slices_large() {
        let left: Vec<i32> = (1..=1000).collect();
        let right: Vec<i32> = (501..=1500).collect();
        let result = intersection_slices(&left, &right);
        let expected: Vec<i32> = (501..=1000).collect();
        assert_eq!(result, expected);
    }

    // =========================================================================
    // Slice-based merge/difference/intersection integration tests
    // =========================================================================

    #[rstest]
    fn merge_large_large_uses_slice_merge() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=100);
        let large2 = OrderedUniqueSet::from_sorted_iter(50..=150);
        assert!(large1.is_large_state());
        assert!(large2.is_large_state());

        let merged = large1.merge(&large2);
        let expected: Vec<i32> = (1..=150).collect();
        assert_eq!(merged.to_sorted_vec(), expected);
    }

    #[rstest]
    fn merge_large_small_uses_slice_merge() {
        let large = OrderedUniqueSet::from_sorted_iter(1..=20);
        let small = OrderedUniqueSet::from_sorted_iter([15, 16, 17, 25, 30]);
        assert!(large.is_large_state());
        assert!(small.is_small_state());

        let merged = large.merge(&small);
        assert_eq!(
            merged.to_sorted_vec(),
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30
            ]
        );
    }

    #[rstest]
    fn merge_small_large_uses_slice_merge() {
        let small = OrderedUniqueSet::from_sorted_iter([1, 5, 10]);
        let large = OrderedUniqueSet::from_sorted_iter(1..=20);
        assert!(small.is_small_state());
        assert!(large.is_large_state());

        let merged = small.merge(&large);
        let expected: Vec<i32> = (1..=20).collect();
        assert_eq!(merged.to_sorted_vec(), expected);
    }

    #[rstest]
    fn merge_large_disjoint_fast_path() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=50);
        let large2 = OrderedUniqueSet::from_sorted_iter(51..=100);
        assert!(large1.is_large_state());
        assert!(large2.is_large_state());

        let merged = large1.merge(&large2);
        let expected: Vec<i32> = (1..=100).collect();
        assert_eq!(merged.to_sorted_vec(), expected);
    }

    #[rstest]
    fn difference_large_large_uses_slice_difference() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=100);
        let large2 = OrderedUniqueSet::from_sorted_iter(50..=150);
        assert!(large1.is_large_state());
        assert!(large2.is_large_state());

        let diff = large1.difference(&large2);
        let expected: Vec<i32> = (1..=49).collect();
        assert_eq!(diff.to_sorted_vec(), expected);
    }

    #[rstest]
    fn difference_large_disjoint_fast_path() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=50);
        let large2 = OrderedUniqueSet::from_sorted_iter(51..=100);

        let diff = large1.difference(&large2);
        let expected: Vec<i32> = (1..=50).collect();
        assert_eq!(diff.to_sorted_vec(), expected);
    }

    #[rstest]
    fn intersection_large_large_uses_slice_intersection() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=100);
        let large2 = OrderedUniqueSet::from_sorted_iter(50..=150);
        assert!(large1.is_large_state());
        assert!(large2.is_large_state());

        let inter = large1.intersection(&large2);
        let expected: Vec<i32> = (50..=100).collect();
        assert_eq!(inter.to_sorted_vec(), expected);
    }

    #[rstest]
    fn intersection_large_disjoint_fast_path_returns_empty() {
        let large1 = OrderedUniqueSet::from_sorted_iter(1..=50);
        let large2 = OrderedUniqueSet::from_sorted_iter(51..=100);

        let inter = large1.intersection(&large2);
        assert!(inter.is_empty());
    }
}
