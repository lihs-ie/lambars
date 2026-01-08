//! Persistent (immutable) hash set.
//!
//! This module provides [`PersistentHashSet`], an immutable hash set
//! that uses [`PersistentHashMap`] internally for efficient operations.
//!
//! # Overview
//!
//! `PersistentHashSet` is a wrapper around `PersistentHashMap<T, ()>` that
//! provides set operations like union, intersection, difference, and
//! symmetric difference.
//!
//! - O(log32 N) contains (effectively O(1) for practical sizes)
//! - O(log32 N) insert
//! - O(log32 N) remove
//! - O(1) len and `is_empty`
//!
//! All operations return new sets without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentHashSet;
//!
//! let set = PersistentHashSet::new()
//!     .insert(1)
//!     .insert(2)
//!     .insert(3);
//!
//! assert!(set.contains(&1));
//! assert!(set.contains(&2));
//! assert!(!set.contains(&4));
//!
//! // Structural sharing: the original set is preserved
//! let updated = set.insert(4);
//! assert_eq!(set.len(), 3);      // Original unchanged
//! assert_eq!(updated.len(), 4);  // New version
//! ```
//!
//! # Set Operations
//!
//! ```rust
//! use lambars::persistent::PersistentHashSet;
//!
//! let set_a: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
//! let set_b: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
//!
//! let union = set_a.union(&set_b);           // {1, 2, 3, 4}
//! let intersection = set_a.intersection(&set_b);  // {2, 3}
//! let difference = set_a.difference(&set_b);      // {1}
//! let symmetric_diff = set_a.symmetric_difference(&set_b);  // {1, 4}
//!
//! assert_eq!(union.len(), 4);
//! assert_eq!(intersection.len(), 2);
//! assert_eq!(difference.len(), 1);
//! assert_eq!(symmetric_diff.len(), 2);
//! ```

use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use super::{PersistentHashMap, TransientHashMap};
use crate::typeclass::{Foldable, TypeConstructor};

// =============================================================================
// PersistentHashSet Definition
// =============================================================================

/// A persistent (immutable) hash set based on [`PersistentHashMap`].
///
/// `PersistentHashSet` is an immutable data structure that uses structural
/// sharing to efficiently support functional programming patterns.
///
/// # Time Complexity
///
/// | Operation              | Complexity        |
/// |------------------------|-------------------|
/// | `new`                  | O(1)              |
/// | `contains`             | O(log32 N)        |
/// | `insert`               | O(log32 N)        |
/// | `remove`               | O(log32 N)        |
/// | `len`                  | O(1)              |
/// | `is_empty`             | O(1)              |
/// | `union`                | O(n + m)          |
/// | `intersection`         | O(min(n,m) * log32(max(n,m))) |
/// | `difference`           | O(n * log32 m)    |
/// | `symmetric_difference` | O(n + m)          |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentHashSet;
///
/// let set = PersistentHashSet::singleton(42);
/// assert!(set.contains(&42));
/// assert!(!set.contains(&0));
/// ```
#[derive(Clone)]
pub struct PersistentHashSet<T> {
    inner: PersistentHashMap<T, ()>,
}

impl<T> PersistentHashSet<T> {
    /// Creates a new empty set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = PersistentHashSet::new();
    /// assert!(set.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: PersistentHashMap::new(),
        }
    }

    /// Returns the number of elements in the set.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set = PersistentHashSet::new().insert(1).insert(2);
    /// assert_eq!(set.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert(42);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T: Clone + Hash + Eq> PersistentHashSet<T> {
    /// Creates a set containing a single element.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to include in the set
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set = PersistentHashSet::singleton(42);
    /// assert_eq!(set.len(), 1);
    /// assert!(set.contains(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(element: T) -> Self {
        Self::new().insert(element)
    }

    /// Returns `true` if the set contains the specified element.
    ///
    /// The element may be any borrowed form of the set's element type,
    /// but `Hash` and `Eq` on the borrowed form must match those for
    /// the element type.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to check for
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set = PersistentHashSet::new()
    ///     .insert("hello".to_string())
    ///     .insert("world".to_string());
    ///
    /// // Can use &str to look up String elements
    /// assert!(set.contains("hello"));
    /// assert!(set.contains("world"));
    /// assert!(!set.contains("other"));
    /// ```
    #[must_use]
    pub fn contains<Q>(&self, element: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(element)
    }

    /// Inserts an element into the set.
    ///
    /// If the set already contains the element, returns a set that
    /// is equivalent to the original.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to insert
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set1 = PersistentHashSet::new().insert(1);
    /// let set2 = set1.insert(2);
    ///
    /// assert_eq!(set1.len(), 1); // Original unchanged
    /// assert_eq!(set2.len(), 2); // New version
    /// ```
    #[must_use]
    pub fn insert(&self, element: T) -> Self {
        Self {
            inner: self.inner.insert(element, ()),
        }
    }

    /// Removes an element from the set.
    ///
    /// Returns a new set without the element. If the element doesn't exist,
    /// returns a clone of the original set.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to remove
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set = PersistentHashSet::new().insert(1).insert(2);
    /// let removed = set.remove(&1);
    ///
    /// assert_eq!(set.len(), 2);      // Original unchanged
    /// assert_eq!(removed.len(), 1);  // New version
    /// assert!(!removed.contains(&1));
    /// ```
    #[must_use]
    pub fn remove<Q>(&self, element: &Q) -> Self
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Self {
            inner: self.inner.remove(element),
        }
    }

    /// Returns the union of two sets.
    ///
    /// The union contains all elements that are in either set.
    ///
    /// # Arguments
    ///
    /// * `other` - The other set to union with
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two sets
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set_a: PersistentHashSet<i32> = [1, 2].into_iter().collect();
    /// let set_b: PersistentHashSet<i32> = [2, 3].into_iter().collect();
    ///
    /// let union = set_a.union(&set_b);
    ///
    /// assert_eq!(union.len(), 3);
    /// assert!(union.contains(&1));
    /// assert!(union.contains(&2));
    /// assert!(union.contains(&3));
    /// ```
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.merge(&other.inner),
        }
    }

    /// Returns the intersection of two sets.
    ///
    /// The intersection contains only elements that are in both sets.
    ///
    /// # Arguments
    ///
    /// * `other` - The other set to intersect with
    ///
    /// # Complexity
    ///
    /// O(min(n, m) * log32(max(n, m)))
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set_a: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let set_b: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
    ///
    /// let intersection = set_a.intersection(&set_b);
    ///
    /// assert_eq!(intersection.len(), 2);
    /// assert!(intersection.contains(&2));
    /// assert!(intersection.contains(&3));
    /// ```
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        // Iterate over the smaller set for better performance
        let (smaller, larger) = if self.len() <= other.len() {
            (self, other)
        } else {
            (other, self)
        };

        let mut result = Self::new();
        for element in smaller {
            if larger.contains(element) {
                result = result.insert(element.clone());
            }
        }
        result
    }

    /// Returns the difference of two sets.
    ///
    /// The difference contains elements that are in `self` but not in `other`.
    ///
    /// # Arguments
    ///
    /// * `other` - The set to subtract
    ///
    /// # Complexity
    ///
    /// O(n * log32 m) where n = `self.len()` and m = `other.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set_a: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let set_b: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
    ///
    /// let difference = set_a.difference(&set_b);
    ///
    /// assert_eq!(difference.len(), 1);
    /// assert!(difference.contains(&1));
    /// ```
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for element in self {
            if !other.contains(element) {
                result = result.insert(element.clone());
            }
        }
        result
    }

    /// Returns the symmetric difference of two sets.
    ///
    /// The symmetric difference contains elements that are in either set
    /// but not in both.
    ///
    /// # Arguments
    ///
    /// * `other` - The other set
    ///
    /// # Complexity
    ///
    /// O(n + m)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set_a: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let set_b: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
    ///
    /// let symmetric_diff = set_a.symmetric_difference(&set_b);
    ///
    /// assert_eq!(symmetric_diff.len(), 2);
    /// assert!(symmetric_diff.contains(&1));
    /// assert!(symmetric_diff.contains(&4));
    /// ```
    #[must_use]
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        // Symmetric difference = (A - B) ∪ (B - A)
        let a_minus_b = self.difference(other);
        let b_minus_a = other.difference(self);
        a_minus_b.union(&b_minus_a)
    }

    /// Returns `true` if `self` is a subset of `other`.
    ///
    /// A set is a subset of another if all elements in `self` are also in `other`.
    ///
    /// # Arguments
    ///
    /// * `other` - The potential superset
    ///
    /// # Complexity
    ///
    /// O(n * log32 m) where n = `self.len()` and m = `other.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let subset: PersistentHashSet<i32> = [1, 2].into_iter().collect();
    /// let superset: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    ///
    /// assert!(subset.is_subset(&superset));
    /// assert!(!superset.is_subset(&subset));
    /// ```
    #[must_use]
    pub fn is_subset(&self, other: &Self) -> bool {
        if self.len() > other.len() {
            return false;
        }

        for element in self {
            if !other.contains(element) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if `self` is a superset of `other`.
    ///
    /// A set is a superset of another if all elements in `other` are also in `self`.
    ///
    /// # Arguments
    ///
    /// * `other` - The potential subset
    ///
    /// # Complexity
    ///
    /// O(m * log32 n) where n = `self.len()` and m = `other.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let superset: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let subset: PersistentHashSet<i32> = [1, 2].into_iter().collect();
    ///
    /// assert!(superset.is_superset(&subset));
    /// assert!(!subset.is_superset(&superset));
    /// ```
    #[must_use]
    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    /// Returns `true` if `self` and `other` have no elements in common.
    ///
    /// # Arguments
    ///
    /// * `other` - The other set
    ///
    /// # Complexity
    ///
    /// O(min(n, m) * log32(max(n, m)))
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set_a: PersistentHashSet<i32> = [1, 2].into_iter().collect();
    /// let set_b: PersistentHashSet<i32> = [3, 4].into_iter().collect();
    /// let set_c: PersistentHashSet<i32> = [2, 3].into_iter().collect();
    ///
    /// assert!(set_a.is_disjoint(&set_b));
    /// assert!(!set_a.is_disjoint(&set_c));
    /// ```
    #[must_use]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Iterate over the smaller set for better performance
        let (smaller, larger) = if self.len() <= other.len() {
            (self, other)
        } else {
            (other, self)
        };

        for element in smaller {
            if larger.contains(element) {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over the elements of the set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    ///
    /// for element in set.iter() {
    ///     println!("{}", element);
    /// }
    /// ```
    #[must_use]
    pub fn iter(&self) -> PersistentHashSetIterator<'_, T> {
        PersistentHashSetIterator {
            inner: self.inner.keys().collect::<Vec<_>>().into_iter(),
        }
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// An iterator over the elements of a [`PersistentHashSet`].
pub struct PersistentHashSetIterator<'a, T> {
    inner: std::vec::IntoIter<&'a T>,
}

impl<'a, T> Iterator for PersistentHashSetIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for PersistentHashSetIterator<'_, T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Type alias for the mapped iterator used in `PersistentHashSetIntoIterator`.
type HashSetIntoIteratorInner<T> =
    std::iter::Map<super::hashmap::PersistentHashMapIntoIterator<T, ()>, fn((T, ())) -> T>;

/// An owning iterator over the elements of a [`PersistentHashSet`].
pub struct PersistentHashSetIntoIterator<T>
where
    T: Clone + Hash + Eq,
{
    inner: HashSetIntoIteratorInner<T>,
}

impl<T: Clone + Hash + Eq> Iterator for PersistentHashSetIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T: Clone + Hash + Eq> ExactSizeIterator for PersistentHashSetIntoIterator<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<T> Default for PersistentHashSet<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Hash + Eq> FromIterator<T> for PersistentHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut transient = TransientHashSet::new();
        transient.extend(iter);
        transient.persistent()
    }
}

impl<T: Clone + Hash + Eq> IntoIterator for PersistentHashSet<T> {
    type Item = T;
    type IntoIter = PersistentHashSetIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        fn extract_key<T>((key, ()): (T, ())) -> T {
            key
        }
        PersistentHashSetIntoIterator {
            inner: self.inner.into_iter().map(extract_key),
        }
    }
}

impl<'a, T> IntoIterator for &'a PersistentHashSet<T>
where
    T: Clone + Hash + Eq,
{
    type Item = &'a T;
    type IntoIter = PersistentHashSetIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Clone + Hash + Eq> PartialEq for PersistentHashSet<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for element in self {
            if !other.contains(element) {
                return false;
            }
        }

        true
    }
}

impl<T: Clone + Hash + Eq> Eq for PersistentHashSet<T> {}

impl<T: Clone + Hash + Eq + fmt::Debug> fmt::Debug for PersistentHashSet<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Clone + Hash + Eq + fmt::Display> fmt::Display for PersistentHashSet<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{{")?;
        let mut first = true;
        for element in self {
            if first {
                first = false;
            } else {
                write!(formatter, ", ")?;
            }
            write!(formatter, "{element}")?;
        }
        write!(formatter, "}}")
    }
}

// =============================================================================
// HashSetView Definition
// =============================================================================

/// Internal trait for type-erased view operations.
///
/// This trait enables dynamic dispatch for view operations, allowing
/// map and `flat_map` to change the element type while maintaining a
/// uniform interface.
trait HashSetViewOperationDynamic<T> {
    /// Creates an iterator over the view's elements.
    fn create_iterator(&self) -> Box<dyn Iterator<Item = T> + '_>;
}

/// Source operation that wraps the original set.
struct SourceOperation<T> {
    source: PersistentHashSet<T>,
}

impl<T: Clone + Hash + Eq + 'static> HashSetViewOperationDynamic<T> for SourceOperation<T> {
    fn create_iterator(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new(self.source.iter().cloned())
    }
}

/// Filter operation that wraps a source operation and a predicate.
struct FilterOperation<T> {
    source: Arc<dyn HashSetViewOperationDynamic<T>>,
    predicate: Arc<dyn Fn(&T) -> bool + 'static>,
}

impl<T: 'static> HashSetViewOperationDynamic<T> for FilterOperation<T> {
    fn create_iterator(&self) -> Box<dyn Iterator<Item = T> + '_> {
        let predicate = Arc::clone(&self.predicate);
        Box::new(
            self.source
                .create_iterator()
                .filter(move |item| predicate(item)),
        )
    }
}

/// Map operation that transforms elements using a function.
struct MapOperation<T, U> {
    source: Arc<dyn HashSetViewOperationDynamic<T>>,
    function: Arc<dyn Fn(T) -> U + 'static>,
}

impl<T: 'static, U: 'static> HashSetViewOperationDynamic<U> for MapOperation<T, U> {
    fn create_iterator(&self) -> Box<dyn Iterator<Item = U> + '_> {
        let function = Arc::clone(&self.function);
        Box::new(
            self.source
                .create_iterator()
                .map(move |item| function(item)),
        )
    }
}

/// `FlatMap` operation that transforms each element into an iterator and flattens.
struct FlatMapOperation<T, U, I>
where
    I: Iterator<Item = U>,
{
    source: Arc<dyn HashSetViewOperationDynamic<T>>,
    function: Arc<dyn Fn(T) -> I + 'static>,
}

impl<T: 'static, U: 'static, I: Iterator<Item = U> + 'static> HashSetViewOperationDynamic<U>
    for FlatMapOperation<T, U, I>
{
    fn create_iterator(&self) -> Box<dyn Iterator<Item = U> + '_> {
        let function = Arc::clone(&self.function);
        Box::new(
            self.source
                .create_iterator()
                .flat_map(move |item| function(item)),
        )
    }
}

/// A lazy evaluation view over a [`PersistentHashSet`].
///
/// Operations (filter, map, `flat_map`) are defined in O(1) time and
/// evaluated lazily during iteration or materialization via `collect()`.
///
/// # Type Parameters
///
/// - `T`: The type of elements produced by this view
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentHashSet;
///
/// let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
///
/// // O(1) to create View
/// let view = set.view();
///
/// // O(1) to chain transformations
/// let transformed = view
///     .filter(|x| *x % 2 == 0)
///     .map(|x| x * 2);
///
/// // Evaluated during iteration
/// for element in transformed.iter() {
///     println!("{}", element);
/// }
///
/// // Or materialize with collect()
/// let result: PersistentHashSet<i32> = set.view()
///     .filter(|x| *x % 2 == 0)
///     .map(|x| x * 2)
///     .collect();
/// ```
///
/// # Complexity
///
/// | Operation | Definition | Iteration | Materialization |
/// |-----------|------------|-----------|-----------------|
/// | view() | O(1) | - | - |
/// | filter(predicate) | O(1) | O(n) | O(n * log32 n) |
/// | map(function) | O(1) | O(n) | O(n * log32 n) |
/// | flat_map(function) | O(1) | O(n * m) | O(n * m * log32(n*m)) |
/// | collect() | - | - | O(n * log32 n) |
pub struct HashSetView<T> {
    operation: Arc<dyn HashSetViewOperationDynamic<T>>,
}

impl<T: Clone + Hash + Eq + 'static> PersistentHashSet<T> {
    /// Creates a lazy evaluation view of this set.
    ///
    /// The view provides lazy access to the set's elements and supports
    /// chaining filter, map, and `flat_map` operations.
    ///
    /// # Complexity
    ///
    /// O(1) - only clones the set reference
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let view = set.view();
    ///
    /// // Access elements through the view
    /// assert_eq!(view.iter().count(), 3);
    /// ```
    #[must_use]
    pub fn view(&self) -> HashSetView<T> {
        HashSetView {
            operation: Arc::new(SourceOperation {
                source: self.clone(),
            }),
        }
    }
}

impl<T> HashSetView<T> {
    /// Returns an iterator over the view's elements.
    ///
    /// The transformation chain is evaluated lazily during iteration.
    ///
    /// # Complexity
    ///
    /// O(n) for full iteration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let view = set.view().filter(|x| *x > 1);
    ///
    /// for element in view.iter() {
    ///     println!("{}", element);  // 2, 3
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.operation.create_iterator()
    }
}

impl<T: Clone + Hash + Eq + 'static> HashSetView<T> {
    /// Returns a new view containing only elements that satisfy the predicate.
    ///
    /// Operation definition is O(1); evaluation occurs during iteration.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for elements to include
    ///
    /// # Complexity
    ///
    /// - Definition: O(1)
    /// - Iteration: O(n)
    /// - Materialization: O(n * log32 n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
    /// let evens = set.view().filter(|x| *x % 2 == 0);
    ///
    /// let result: PersistentHashSet<i32> = evens.collect();
    /// assert!(result.contains(&2));
    /// assert!(result.contains(&4));
    /// assert!(!result.contains(&1));
    /// ```
    #[must_use]
    pub fn filter<P>(self, predicate: P) -> Self
    where
        P: Fn(&T) -> bool + 'static,
    {
        Self {
            operation: Arc::new(FilterOperation {
                source: self.operation,
                predicate: Arc::new(predicate),
            }),
        }
    }

    /// Fully evaluates the view and produces a new [`PersistentHashSet`].
    ///
    /// Duplicate elements from the transformation chain are automatically removed.
    ///
    /// # Complexity
    ///
    /// O(n * log32 n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
    ///
    /// let result: PersistentHashSet<i32> = set
    ///     .view()
    ///     .filter(|x| *x % 2 == 0)
    ///     .map(|x| x * 2)
    ///     .collect();
    ///
    /// assert_eq!(result.len(), 2);
    /// assert!(result.contains(&4));   // 2 * 2
    /// assert!(result.contains(&8));   // 4 * 2
    /// ```
    #[must_use]
    pub fn collect(self) -> PersistentHashSet<T> {
        self.iter().collect()
    }

    /// Applies a function to each element and returns a new view.
    ///
    /// Operation definition is O(1); evaluation occurs during iteration.
    ///
    /// Duplicate elements after transformation are removed during `collect()`.
    /// For example: `[1, 2, 3].map(|x| x % 2)` produces `{0, 1}` after collect.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to transform each element
    ///
    /// # Complexity
    ///
    /// - Definition: O(1)
    /// - Iteration: O(n)
    /// - Materialization: O(n * log32 n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// let doubled: PersistentHashSet<i32> = set.view().map(|x| x * 2).collect();
    ///
    /// assert!(doubled.contains(&2));
    /// assert!(doubled.contains(&4));
    /// assert!(doubled.contains(&6));
    /// ```
    #[must_use]
    pub fn map<U, F>(self, function: F) -> HashSetView<U>
    where
        F: Fn(T) -> U + 'static,
        U: Clone + Hash + Eq + 'static,
    {
        HashSetView {
            operation: Arc::new(MapOperation {
                source: self.operation,
                function: Arc::new(function),
            }),
        }
    }

    /// Applies a function that returns an iterator to each element
    /// and flattens the results into a single view.
    ///
    /// Operation definition is O(1); evaluation occurs during iteration.
    ///
    /// This is the Monad `bind` operation for sets. Each element is
    /// transformed into an iterator, and all results are flattened
    /// into a single set (duplicates removed during `collect()`).
    ///
    /// # Arguments
    ///
    /// * `function` - A function that returns an iterator for each element
    ///
    /// # Complexity
    ///
    /// - Definition: O(1)
    /// - Iteration: O(n * m) where m is average iterator size
    /// - Materialization: O(n * m * log32(n * m))
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2].into_iter().collect();
    /// let result: PersistentHashSet<i32> = set
    ///     .view()
    ///     .flat_map(|x| vec![x, x * 10].into_iter())
    ///     .collect();
    ///
    /// assert!(result.contains(&1));
    /// assert!(result.contains(&10));
    /// assert!(result.contains(&2));
    /// assert!(result.contains(&20));
    /// ```
    #[must_use]
    pub fn flat_map<U, I, F>(self, function: F) -> HashSetView<U>
    where
        F: Fn(T) -> I + 'static,
        I: Iterator<Item = U> + 'static,
        U: Clone + Hash + Eq + 'static,
    {
        HashSetView {
            operation: Arc::new(FlatMapOperation {
                source: self.operation,
                function: Arc::new(function),
            }),
        }
    }

    /// Returns `true` if any element satisfies the predicate.
    ///
    /// Returns `false` for empty views.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for matching elements
    ///
    /// # Complexity
    ///
    /// O(n) worst case, but short-circuits on first match
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    ///
    /// assert!(set.view().any(|x| *x == 2));
    /// assert!(!set.view().any(|x| *x > 10));
    /// ```
    #[must_use]
    pub fn any<P>(&self, predicate: P) -> bool
    where
        P: Fn(&T) -> bool,
    {
        self.iter().any(|item| predicate(&item))
    }

    /// Returns `true` if all elements satisfy the predicate.
    ///
    /// Returns `true` for empty views (vacuous truth).
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true for matching elements
    ///
    /// # Complexity
    ///
    /// O(n) worst case, but short-circuits on first non-match
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [2, 4, 6].into_iter().collect();
    ///
    /// assert!(set.view().all(|x| *x % 2 == 0));
    /// assert!(!set.view().all(|x| *x > 3));
    /// ```
    #[must_use]
    pub fn all<P>(&self, predicate: P) -> bool
    where
        P: Fn(&T) -> bool,
    {
        self.iter().all(|item| predicate(&item))
    }

    /// Returns the number of elements in the view.
    ///
    /// # Complexity
    ///
    /// O(n) - requires full iteration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
    ///
    /// assert_eq!(set.view().count(), 5);
    /// assert_eq!(set.view().filter(|x| *x % 2 == 0).count(), 2);
    /// ```
    #[must_use]
    pub fn count(&self) -> usize {
        self.iter().count()
    }

    /// Returns `true` if the view contains no elements.
    ///
    /// # Complexity
    ///
    /// O(1) if no transformations, O(n) worst case with transformations
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    /// assert!(empty.view().is_empty());
    ///
    /// let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    /// assert!(!set.view().is_empty());
    /// assert!(set.view().filter(|x| *x > 100).is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

impl<T> Clone for HashSetView<T> {
    fn clone(&self) -> Self {
        Self {
            operation: Arc::clone(&self.operation),
        }
    }
}

// =============================================================================
// Type Class Implementations
// =============================================================================

impl<T> TypeConstructor for PersistentHashSet<T> {
    type Inner = T;
    type WithType<B> = PersistentHashSet<B>;
}

impl<T: Clone + Hash + Eq> Foldable for PersistentHashSet<T> {
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
        // For unordered collections (hash-based), fold_right is semantically
        // equivalent to fold_left since there is no defined order.
        self.into_iter()
            .fold(init, |accumulator, element| function(element, accumulator))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    fn length(&self) -> usize {
        self.inner.len()
    }
}

// =============================================================================
// TransientHashSet Definition
// =============================================================================

/// A transient (temporarily mutable) hash set for efficient batch updates.
///
/// `TransientHashSet` is a wrapper around [`TransientHashMap<T, ()>`](TransientHashMap)
/// that provides efficient mutable operations for building a hash set.
/// After batch updates, convert to [`PersistentHashSet`] using [`persistent()`](Self::persistent).
///
/// # Design
///
/// - Internally uses `TransientHashMap<T, ()>` for all operations
/// - `PhantomData<Rc<()>>` ensures `!Send` and `!Sync` for thread safety
/// - Clone/Copy traits are intentionally not implemented (linear type semantics)
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::{PersistentHashSet, TransientHashSet};
///
/// // Build a set efficiently using transient operations
/// let mut transient = TransientHashSet::new();
/// transient.insert(1);
/// transient.insert(2);
/// transient.insert(3);
///
/// // Convert to persistent set
/// let persistent = transient.persistent();
/// assert!(persistent.contains(&1));
/// assert_eq!(persistent.len(), 3);
/// ```
///
/// # Transient-Persistent Pattern
///
/// ```rust
/// use lambars::persistent::PersistentHashSet;
///
/// // Start with a persistent set
/// let persistent: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
///
/// // Convert to transient for batch updates
/// let mut transient = persistent.transient();
/// transient.insert(4);
/// transient.insert(5);
/// transient.remove(&1);
///
/// // Convert back to persistent
/// let new_persistent = transient.persistent();
/// assert_eq!(new_persistent.len(), 4);
/// assert!(!new_persistent.contains(&1));
/// assert!(new_persistent.contains(&4));
/// ```
pub struct TransientHashSet<T> {
    inner: TransientHashMap<T, ()>,
    /// Marker to ensure `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

// Static assertions to verify TransientHashSet is not Send/Sync
static_assertions::assert_not_impl_any!(TransientHashSet<i32>: Send, Sync);
static_assertions::assert_not_impl_any!(TransientHashSet<String>: Send, Sync);

// Arc feature verification: even with Arc, TransientHashSet remains !Send/!Sync
#[cfg(feature = "arc")]
mod arc_send_sync_verification_hashset {
    use super::TransientHashSet;
    use std::sync::Arc;

    // Arc<T> where T: Send+Sync is Send+Sync, but TransientHashSet should still be !Send/!Sync
    static_assertions::assert_not_impl_any!(TransientHashSet<Arc<i32>>: Send, Sync);
    static_assertions::assert_not_impl_any!(TransientHashSet<Arc<String>>: Send, Sync);
}

// =============================================================================
// TransientHashSet Implementation
// =============================================================================

impl<T> TransientHashSet<T> {
    /// Returns the number of elements in the set.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// assert_eq!(transient.len(), 0);
    /// transient.insert(42);
    /// assert_eq!(transient.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// assert!(transient.is_empty());
    /// transient.insert(42);
    /// assert!(!transient.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T: Clone + Hash + Eq> TransientHashSet<T> {
    /// Creates a new empty `TransientHashSet`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let transient: TransientHashSet<i32> = TransientHashSet::new();
    /// assert!(transient.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: TransientHashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Returns `true` if the set contains the specified element.
    ///
    /// The element may be any borrowed form of the set's element type,
    /// but `Hash` and `Eq` on the borrowed form must match those for
    /// the element type.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to check for
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<String> = TransientHashSet::new();
    /// transient.insert("hello".to_string());
    ///
    /// // Can use &str to look up String elements
    /// assert!(transient.contains("hello"));
    /// assert!(!transient.contains("world"));
    /// ```
    #[must_use]
    pub fn contains<Q>(&self, element: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(element)
    }

    /// Inserts an element into the set.
    ///
    /// Returns `true` if the element was newly inserted, `false` if it was already present.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to insert
    ///
    /// # Returns
    ///
    /// `true` if the element was not present, `false` if it was already present.
    ///
    /// # Complexity
    ///
    /// O(log32 N) amortized
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// assert!(transient.insert(1));   // New element
    /// assert!(!transient.insert(1));  // Already exists
    /// assert_eq!(transient.len(), 1);
    /// ```
    pub fn insert(&mut self, element: T) -> bool {
        self.inner.insert(element, ()).is_none()
    }

    /// Removes an element from the set.
    ///
    /// Returns `true` if the element was present and removed, `false` if it was not present.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to remove
    ///
    /// # Returns
    ///
    /// `true` if the element was removed, `false` if it was not present.
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// transient.insert(1);
    /// transient.insert(2);
    ///
    /// assert!(transient.remove(&1));   // Was present
    /// assert!(!transient.remove(&1));  // Already removed
    /// assert_eq!(transient.len(), 1);
    /// ```
    pub fn remove<Q>(&mut self, element: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(element).is_some()
    }

    /// Extends the set with elements from an iterator.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator over elements to insert
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// transient.extend([1, 2, 3, 4, 5]);
    /// assert_eq!(transient.len(), 5);
    /// ```
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for element in iter {
            self.insert(element);
        }
    }

    /// Converts this transient set into a persistent set.
    ///
    /// This consumes the `TransientHashSet` and returns a `PersistentHashSet`.
    /// The conversion is O(1) as it simply moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashSet;
    ///
    /// let mut transient: TransientHashSet<i32> = TransientHashSet::new();
    /// transient.insert(1);
    /// transient.insert(2);
    /// let persistent = transient.persistent();
    /// assert_eq!(persistent.len(), 2);
    /// assert!(persistent.contains(&1));
    /// ```
    #[must_use]
    pub fn persistent(self) -> PersistentHashSet<T> {
        PersistentHashSet {
            inner: self.inner.persistent(),
        }
    }
}

impl<T: Clone + Hash + Eq> Default for TransientHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Hash + Eq> FromIterator<T> for TransientHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut transient = Self::new();
        transient.extend(iter);
        transient
    }
}

// =============================================================================
// PersistentHashSet::transient() method
// =============================================================================

impl<T: Clone + Hash + Eq> PersistentHashSet<T> {
    /// Converts this persistent set into a transient set.
    ///
    /// This consumes the `PersistentHashSet` and returns a `TransientHashSet`
    /// that can be efficiently mutated. The conversion is O(1) as it simply
    /// moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashSet;
    ///
    /// let persistent: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    ///
    /// // Convert to transient for batch updates
    /// let mut transient = persistent.transient();
    /// transient.insert(4);
    /// transient.insert(5);
    /// transient.remove(&1);
    ///
    /// // Convert back to persistent
    /// let new_persistent = transient.persistent();
    /// assert_eq!(new_persistent.len(), 4);
    /// assert!(!new_persistent.contains(&1));
    /// assert!(new_persistent.contains(&4));
    /// ```
    #[must_use]
    pub fn transient(self) -> TransientHashSet<T> {
        TransientHashSet {
            inner: self.inner.transient(),
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// Serde Support
// =============================================================================

#[cfg(feature = "serde")]
#[allow(clippy::explicit_iter_loop)]
impl<T: serde::Serialize + Clone + Hash + Eq> serde::Serialize for PersistentHashSet<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for element in self {
            seq.serialize_element(&element)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
struct PersistentHashSetVisitor<T> {
    marker: std::marker::PhantomData<T>,
}

#[cfg(feature = "serde")]
impl<T> PersistentHashSetVisitor<T> {
    const fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::de::Visitor<'de> for PersistentHashSetVisitor<T>
where
    T: serde::Deserialize<'de> + Clone + Hash + Eq,
{
    type Value = PersistentHashSet<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        // Note: Sequential insert ensures gradual memory usage even for large inputs.
        let mut set = PersistentHashSet::new();
        while let Some(element) = seq.next_element()? {
            set = set.insert(element);
        }
        Ok(set)
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for PersistentHashSet<T>
where
    T: serde::Deserialize<'de> + Clone + Hash + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(PersistentHashSetVisitor::new())
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
    fn test_display_empty_hashset() {
        let set: PersistentHashSet<i32> = PersistentHashSet::new();
        assert_eq!(format!("{set}"), "{}");
    }

    #[rstest]
    fn test_display_single_element_hashset() {
        let set = PersistentHashSet::singleton(42);
        assert_eq!(format!("{set}"), "{42}");
    }

    #[rstest]
    fn test_display_multiple_elements_hashset() {
        let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let display = format!("{set}");
        // HashSet is unordered, so we check that the format is correct
        assert!(display.starts_with('{'));
        assert!(display.ends_with('}'));
        assert!(display.contains('1'));
        assert!(display.contains('2'));
        assert!(display.contains('3'));
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_new_creates_empty() {
        let set: PersistentHashSet<i32> = PersistentHashSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[rstest]
    fn test_singleton() {
        let set = PersistentHashSet::singleton(42);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&42));
    }

    #[rstest]
    fn test_insert_and_contains() {
        let set = PersistentHashSet::new().insert(1).insert(2).insert(3);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(!set.contains(&4));
    }

    #[rstest]
    fn test_remove() {
        let set = PersistentHashSet::new().insert(1).insert(2);
        let removed = set.remove(&1);

        assert_eq!(removed.len(), 1);
        assert!(!removed.contains(&1));
        assert!(removed.contains(&2));
    }

    #[rstest]
    fn test_union() {
        let set_a = PersistentHashSet::new().insert(1).insert(2);
        let set_b = PersistentHashSet::new().insert(2).insert(3);
        let union = set_a.union(&set_b);

        assert_eq!(union.len(), 3);
        assert!(union.contains(&1));
        assert!(union.contains(&2));
        assert!(union.contains(&3));
    }

    #[rstest]
    fn test_intersection() {
        let set_a = PersistentHashSet::new().insert(1).insert(2).insert(3);
        let set_b = PersistentHashSet::new().insert(2).insert(3).insert(4);
        let intersection = set_a.intersection(&set_b);

        assert_eq!(intersection.len(), 2);
        assert!(intersection.contains(&2));
        assert!(intersection.contains(&3));
    }

    #[rstest]
    fn test_difference() {
        let set_a = PersistentHashSet::new().insert(1).insert(2).insert(3);
        let set_b = PersistentHashSet::new().insert(2).insert(3).insert(4);
        let difference = set_a.difference(&set_b);

        assert_eq!(difference.len(), 1);
        assert!(difference.contains(&1));
    }

    #[rstest]
    fn test_symmetric_difference() {
        let set_a = PersistentHashSet::new().insert(1).insert(2).insert(3);
        let set_b = PersistentHashSet::new().insert(2).insert(3).insert(4);
        let symmetric_difference = set_a.symmetric_difference(&set_b);

        assert_eq!(symmetric_difference.len(), 2);
        assert!(symmetric_difference.contains(&1));
        assert!(symmetric_difference.contains(&4));
    }

    #[rstest]
    fn test_is_subset() {
        let subset = PersistentHashSet::new().insert(1).insert(2);
        let superset = PersistentHashSet::new().insert(1).insert(2).insert(3);

        assert!(subset.is_subset(&superset));
        assert!(!superset.is_subset(&subset));
    }

    #[rstest]
    fn test_is_disjoint() {
        let set_a = PersistentHashSet::new().insert(1).insert(2);
        let set_b = PersistentHashSet::new().insert(3).insert(4);
        let set_c = PersistentHashSet::new().insert(2).insert(3);

        assert!(set_a.is_disjoint(&set_b));
        assert!(!set_a.is_disjoint(&set_c));
    }

    #[rstest]
    fn test_from_iter() {
        let set: PersistentHashSet<i32> = vec![1, 2, 3].into_iter().collect();

        assert_eq!(set.len(), 3);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
    }

    #[rstest]
    fn test_eq() {
        let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
        let set2 = PersistentHashSet::new().insert(3).insert(1).insert(2);

        assert_eq!(set1, set2);
    }

    #[rstest]
    fn test_fold_left() {
        let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
        let sum = set.fold_left(0, |accumulator, element| accumulator + element);

        assert_eq!(sum, 15);
    }
}

// =============================================================================
// Send + Sync Tests (arc feature only)
// =============================================================================

#[cfg(all(test, feature = "arc"))]
mod send_sync_tests {
    use super::*;
    use rstest::rstest;

    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}

    #[rstest]
    fn test_hashset_is_send() {
        assert_send::<PersistentHashSet<i32>>();
        assert_send::<PersistentHashSet<String>>();
    }

    #[rstest]
    fn test_hashset_is_sync() {
        assert_sync::<PersistentHashSet<i32>>();
        assert_sync::<PersistentHashSet<String>>();
    }

    #[rstest]
    fn test_hashset_send_sync_combined() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<PersistentHashSet<i32>>();
        is_send_sync::<PersistentHashSet<String>>();
    }
}

// =============================================================================
// Multithread Tests (arc feature only)
// =============================================================================

#[cfg(all(test, feature = "arc"))]
mod multithread_tests {
    use super::*;
    use rstest::rstest;
    use std::sync::Arc;
    use std::thread;

    #[rstest]
    fn test_hashset_shared_across_threads() {
        let set = Arc::new(PersistentHashSet::new().insert(1).insert(2).insert(3));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let set_clone = Arc::clone(&set);
                thread::spawn(move || {
                    assert!(set_clone.contains(&1));
                    assert!(set_clone.contains(&2));
                    assert!(set_clone.contains(&3));
                    assert_eq!(set_clone.len(), 3);
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[rstest]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn test_hashset_concurrent_insert() {
        let base_set = Arc::new(PersistentHashSet::new().insert(0));

        let results: Vec<_> = (1..=4)
            .map(|index| {
                let set_clone = Arc::clone(&base_set);
                thread::spawn(move || {
                    let new_set = set_clone.insert(index);
                    assert!(new_set.contains(&index));
                    assert!(new_set.contains(&0));
                    new_set
                })
            })
            .map(|handle| handle.join().expect("Thread panicked"))
            .collect();

        // Each thread should have created an independent set with 2 elements
        for (index, set) in results.iter().enumerate() {
            assert_eq!(set.len(), 2);
            assert!(set.contains(&((index + 1) as i32)));
        }

        // Original set should be unchanged
        assert_eq!(base_set.len(), 1);
    }

    #[rstest]
    fn test_hashset_referential_transparency() {
        let set = Arc::new(PersistentHashSet::new().insert(1).insert(2));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let set_clone = Arc::clone(&set);
                thread::spawn(move || {
                    let updated = set_clone.insert(3);
                    // Original should be unchanged
                    assert_eq!(set_clone.len(), 2);
                    assert!(!set_clone.contains(&3));
                    // New set should have the addition
                    assert_eq!(updated.len(), 3);
                    assert!(updated.contains(&3));
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Original should still be unchanged
        assert_eq!(set.len(), 2);
    }

    #[rstest]
    fn test_hashset_concurrent_set_operations() {
        let set_a = Arc::new(PersistentHashSet::new().insert(1).insert(2).insert(3));
        let set_b = Arc::new(PersistentHashSet::new().insert(2).insert(3).insert(4));

        let handles: Vec<_> = (0..4)
            .map(|index| {
                let set_a_clone = Arc::clone(&set_a);
                let set_b_clone = Arc::clone(&set_b);
                thread::spawn(move || match index % 4 {
                    0 => {
                        let union = set_a_clone.union(&set_b_clone);
                        assert_eq!(union.len(), 4);
                    }
                    1 => {
                        let intersection = set_a_clone.intersection(&set_b_clone);
                        assert_eq!(intersection.len(), 2);
                    }
                    2 => {
                        let difference = set_a_clone.difference(&set_b_clone);
                        assert_eq!(difference.len(), 1);
                    }
                    3 => {
                        let symmetric_difference = set_a_clone.symmetric_difference(&set_b_clone);
                        assert_eq!(symmetric_difference.len(), 2);
                    }
                    _ => unreachable!(),
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use rstest::rstest;
    use std::collections::HashSet;

    #[rstest]
    fn test_serialize_empty() {
        let set: PersistentHashSet<i32> = PersistentHashSet::new();
        let json = serde_json::to_string(&set).unwrap();
        assert_eq!(json, "[]");
    }

    #[rstest]
    fn test_serialize_single_element() {
        let set = PersistentHashSet::singleton(42);
        let json = serde_json::to_string(&set).unwrap();
        assert_eq!(json, "[42]");
    }

    #[rstest]
    fn test_serialize_multiple_elements() {
        let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let json = serde_json::to_string(&set).unwrap();
        let parsed: Vec<i32> = serde_json::from_str(&json).unwrap();
        let parsed_set: HashSet<i32> = parsed.into_iter().collect();
        assert_eq!(parsed_set, [1, 2, 3].into_iter().collect());
    }

    #[rstest]
    fn test_deserialize_empty() {
        let json = "[]";
        let set: PersistentHashSet<i32> = serde_json::from_str(json).unwrap();
        assert!(set.is_empty());
    }

    #[rstest]
    fn test_deserialize_single_element() {
        let json = "[42]";
        let set: PersistentHashSet<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(set.len(), 1);
        assert!(set.contains(&42));
    }

    #[rstest]
    fn test_deserialize_multiple_elements() {
        let json = "[1,2,3]";
        let set: PersistentHashSet<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(set.len(), 3);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
    }

    #[rstest]
    fn test_roundtrip_empty() {
        let original: PersistentHashSet<i32> = PersistentHashSet::new();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentHashSet<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_roundtrip_large() {
        let original: PersistentHashSet<i32> = (1..=100).collect();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentHashSet<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_element_preservation() {
        let set: PersistentHashSet<i32> = (0..100).collect();
        let json = serde_json::to_string(&set).unwrap();
        let restored: PersistentHashSet<i32> = serde_json::from_str(&json).unwrap();
        for element_value in 0..100 {
            assert!(restored.contains(&element_value));
        }
    }

    #[rstest]
    fn test_serialize_strings() {
        let set: PersistentHashSet<String> = vec!["hello".to_string(), "world".to_string()]
            .into_iter()
            .collect();
        let json = serde_json::to_string(&set).unwrap();
        let parsed: Vec<String> = serde_json::from_str(&json).unwrap();
        let parsed_set: HashSet<String> = parsed.into_iter().collect();
        let expected: HashSet<String> = ["hello".to_string(), "world".to_string()]
            .into_iter()
            .collect();
        assert_eq!(parsed_set, expected);
    }

    #[rstest]
    fn test_deserialize_strings() {
        let json = r#"["hello","world"]"#;
        let set: PersistentHashSet<String> = serde_json::from_str(json).unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains("hello"));
        assert!(set.contains("world"));
    }

    #[rstest]
    fn test_deserialize_deduplicates() {
        let json = "[1,2,2,3,3,3]";
        let set: PersistentHashSet<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(set.len(), 3);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
    }
}

// =============================================================================
// TransientHashSet Tests
// =============================================================================

#[cfg(test)]
mod transient_hashset_tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Basic Functionality Tests
    // =========================================================================

    #[rstest]
    fn test_transient_hashset_new() {
        let transient: TransientHashSet<i32> = TransientHashSet::new();
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_transient_hashset_insert_and_contains() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        assert!(transient.insert(1));
        assert!(transient.insert(2));
        assert!(transient.insert(3));

        assert_eq!(transient.len(), 3);
        assert!(transient.contains(&1));
        assert!(transient.contains(&2));
        assert!(transient.contains(&3));
        assert!(!transient.contains(&4));
    }

    #[rstest]
    fn test_transient_hashset_insert_duplicate_returns_false() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        assert!(transient.insert(1)); // New element
        assert!(!transient.insert(1)); // Already exists
        assert_eq!(transient.len(), 1);
    }

    #[rstest]
    fn test_transient_hashset_remove() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        transient.insert(1);
        transient.insert(2);
        transient.insert(3);

        assert!(transient.remove(&1)); // Was present
        assert_eq!(transient.len(), 2);
        assert!(!transient.contains(&1));
        assert!(transient.contains(&2));

        assert!(!transient.remove(&1)); // Already removed
    }

    #[rstest]
    fn test_transient_hashset_extend() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        transient.extend([1, 2, 3, 4, 5]);
        assert_eq!(transient.len(), 5);
        for element in 1..=5 {
            assert!(transient.contains(&element));
        }
    }

    #[rstest]
    fn test_transient_hashset_from_iterator() {
        let transient: TransientHashSet<i32> = [1, 2, 3].into_iter().collect();
        assert_eq!(transient.len(), 3);
        assert!(transient.contains(&1));
        assert!(transient.contains(&2));
        assert!(transient.contains(&3));
    }

    #[rstest]
    fn test_transient_hashset_default() {
        let transient: TransientHashSet<i32> = TransientHashSet::default();
        assert!(transient.is_empty());
    }

    // =========================================================================
    // Persistent Conversion Tests
    // =========================================================================

    #[rstest]
    fn test_transient_hashset_persistent() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        transient.insert(1);
        transient.insert(2);
        transient.insert(3);

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 3);
        assert!(persistent.contains(&1));
        assert!(persistent.contains(&2));
        assert!(persistent.contains(&3));
    }

    #[rstest]
    fn test_persistent_hashset_transient() {
        let persistent: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();

        let mut transient = persistent.transient();
        transient.insert(4);
        transient.insert(5);
        transient.remove(&1);

        let new_persistent = transient.persistent();
        assert_eq!(new_persistent.len(), 4);
        assert!(!new_persistent.contains(&1));
        assert!(new_persistent.contains(&2));
        assert!(new_persistent.contains(&3));
        assert!(new_persistent.contains(&4));
        assert!(new_persistent.contains(&5));
    }

    // =========================================================================
    // Transient-Persistent Roundtrip Law Tests
    // =========================================================================

    #[rstest]
    fn test_roundtrip_empty_set() {
        let original: PersistentHashSet<i32> = PersistentHashSet::new();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_roundtrip_single_element() {
        let original = PersistentHashSet::singleton(42);
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_roundtrip_multiple_elements() {
        let original: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_roundtrip_large_set() {
        let original: PersistentHashSet<i32> = (0..1000).collect();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_roundtrip_string_elements() {
        let original: PersistentHashSet<String> = vec![
            "hello".to_string(),
            "world".to_string(),
            "foo".to_string(),
            "bar".to_string(),
        ]
        .into_iter()
        .collect();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    // =========================================================================
    // Mutation Equivalence Law Tests (Insert)
    // =========================================================================

    #[rstest]
    fn test_insert_equivalence_empty_set() {
        let original: PersistentHashSet<i32> = PersistentHashSet::new();
        let element = 42;

        let via_transient = {
            let mut transient = original.clone().transient();
            transient.insert(element);
            transient.persistent()
        };
        let via_persistent = original.insert(element);

        assert_eq!(via_transient, via_persistent);
    }

    #[rstest]
    fn test_insert_equivalence_existing_element() {
        let original: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let element = 2; // Already exists

        let via_transient = {
            let mut transient = original.clone().transient();
            transient.insert(element);
            transient.persistent()
        };
        let via_persistent = original.insert(element);

        assert_eq!(via_transient, via_persistent);
    }

    #[rstest]
    fn test_insert_equivalence_new_element() {
        let original: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let element = 4; // New element

        let via_transient = {
            let mut transient = original.clone().transient();
            transient.insert(element);
            transient.persistent()
        };
        let via_persistent = original.insert(element);

        assert_eq!(via_transient, via_persistent);
    }

    // =========================================================================
    // Mutation Equivalence Law Tests (Remove)
    // =========================================================================

    #[rstest]
    fn test_remove_equivalence_existing_element() {
        let original: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let element = 2;

        let via_transient = {
            let mut transient = original.clone().transient();
            transient.remove(&element);
            transient.persistent()
        };
        let via_persistent = original.remove(&element);

        assert_eq!(via_transient, via_persistent);
    }

    #[rstest]
    fn test_remove_equivalence_non_existing_element() {
        let original: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
        let element = 100; // Non-existing

        let via_transient = {
            let mut transient = original.clone().transient();
            transient.remove(&element);
            transient.persistent()
        };
        let via_persistent = original.remove(&element);

        assert_eq!(via_transient, via_persistent);
    }

    // =========================================================================
    // Batch Operations Tests
    // =========================================================================

    #[rstest]
    fn test_transient_batch_operations() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();

        // Insert many elements
        for element in 0..100 {
            transient.insert(element);
        }

        // Remove some elements
        for element in 25..75 {
            transient.remove(&element);
        }

        let persistent = transient.persistent();

        // Verify the results
        assert_eq!(persistent.len(), 50); // 0..25 and 75..100

        for element in 0..25 {
            assert!(persistent.contains(&element));
        }

        for element in 25..75 {
            assert!(!persistent.contains(&element));
        }

        for element in 75..100 {
            assert!(persistent.contains(&element));
        }
    }

    #[rstest]
    fn test_transient_many_insertions() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();

        for element in 0..1000 {
            transient.insert(element);
        }

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 1000);

        for element in 0..1000 {
            assert!(persistent.contains(&element));
        }
    }

    // =========================================================================
    // String Element Tests (Borrow trait usage)
    // =========================================================================

    #[rstest]
    fn test_transient_hashset_string_borrow() {
        let mut transient: TransientHashSet<String> = TransientHashSet::new();
        transient.insert("hello".to_string());
        transient.insert("world".to_string());

        // Test using &str (borrowed form)
        assert!(transient.contains("hello"));
        assert!(transient.contains("world"));
        assert!(!transient.contains("other"));

        assert!(transient.remove("hello"));
        assert!(!transient.contains("hello"));
    }

    // =========================================================================
    // Edge Cases Tests
    // =========================================================================

    #[rstest]
    fn test_transient_hashset_empty_to_persistent() {
        let transient: TransientHashSet<i32> = TransientHashSet::new();
        let persistent = transient.persistent();
        assert!(persistent.is_empty());
    }

    #[rstest]
    fn test_transient_remove_from_empty() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        assert!(!transient.remove(&42));
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_transient_insert_and_remove_same_element() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        assert!(transient.insert(42));
        assert!(transient.remove(&42));
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_transient_extend_with_duplicates() {
        let mut transient: TransientHashSet<i32> = TransientHashSet::new();
        transient.extend([1, 2, 2, 3, 3, 3]);
        assert_eq!(transient.len(), 3);
    }
}
