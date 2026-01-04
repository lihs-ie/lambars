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

use super::PersistentHashMap;
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
    /// Internal hash map with () as value type
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
        let mut set = Self::new();
        for element in iter {
            set = set.insert(element);
        }
        set
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
