//! Persistent (immutable) data structures.
//!
//! This module provides efficient immutable data structures that use
//! structural sharing to minimize copying:
//!
//! - [`PersistentList`]: Persistent singly-linked list
//! - [`PersistentVector`]: Persistent vector (Radix Balanced Tree)
//! - [`PersistentDeque`]: Persistent double-ended queue (Finger Tree)
//! - [`PersistentHashMap`]: Persistent hash map (HAMT)
//! - [`PersistentHashSet`]: Persistent hash set (based on HAMT)
//! - [`PersistentTreeMap`]: Persistent ordered map (B-Tree)
//!
//! # Structural Sharing
//!
//! All data structures in this module use structural sharing to ensure
//! that operations like prepending, appending, or updating create new
//! versions without copying the entire structure.
//!
//! # Examples
//!
//! ## `PersistentList`
//!
//! ```rust
//! use lambars::persistent::PersistentList;
//!
//! let list = PersistentList::new().cons(3).cons(2).cons(1);
//! assert_eq!(list.head(), Some(&1));
//!
//! // Structural sharing: the original list is preserved
//! let extended = list.cons(0);
//! assert_eq!(list.len(), 3);     // Original unchanged
//! assert_eq!(extended.len(), 4); // New list
//! ```
//!
//! ## `PersistentVector`
//!
//! ```rust
//! use lambars::persistent::PersistentVector;
//!
//! let vector: PersistentVector<i32> = (0..100).collect();
//! assert_eq!(vector.get(50), Some(&50));
//!
//! // Structural sharing: the original vector is preserved
//! let updated = vector.update(50, 999).unwrap();
//! assert_eq!(vector.get(50), Some(&50));     // Original unchanged
//! assert_eq!(updated.get(50), Some(&999));   // New version
//! ```
//!
//! ## `PersistentHashMap`
//!
//! ```rust
//! use lambars::persistent::PersistentHashMap;
//!
//! let map = PersistentHashMap::new()
//!     .insert("one".to_string(), 1)
//!     .insert("two".to_string(), 2);
//! assert_eq!(map.get("one"), Some(&1));
//!
//! // Structural sharing: the original map is preserved
//! let updated = map.insert("one".to_string(), 100);
//! assert_eq!(map.get("one"), Some(&1));       // Original unchanged
//! assert_eq!(updated.get("one"), Some(&100)); // New version
//! ```
//!
//! ## `PersistentHashSet`
//!
//! ```rust
//! use lambars::persistent::PersistentHashSet;
//!
//! let set = PersistentHashSet::new()
//!     .insert(1)
//!     .insert(2)
//!     .insert(3);
//! assert!(set.contains(&1));
//!
//! // Structural sharing: the original set is preserved
//! let updated = set.insert(4);
//! assert_eq!(set.len(), 3);      // Original unchanged
//! assert_eq!(updated.len(), 4);  // New version
//!
//! // Set operations
//! let other: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
//! let union = set.union(&other);
//! let intersection = set.intersection(&other);
//!
//! assert_eq!(union.len(), 4);        // {1, 2, 3, 4}
//! assert_eq!(intersection.len(), 2); // {2, 3}
//! ```
//!
//! ## `PersistentTreeMap`
//!
//! ```rust
//! use lambars::persistent::PersistentTreeMap;
//!
//! let map = PersistentTreeMap::new()
//!     .insert(3, "three")
//!     .insert(1, "one")
//!     .insert(2, "two");
//!
//! // Entries are always in sorted order
//! let keys: Vec<&i32> = map.keys().collect();
//! assert_eq!(keys, vec![&1, &2, &3]);
//!
//! // Structural sharing: the original map is preserved
//! let updated = map.insert(1, "ONE");
//! assert_eq!(map.get(&1), Some(&"one"));  // Original unchanged
//! assert_eq!(updated.get(&1), Some(&"ONE")); // New version
//!
//! // Range queries
//! let range: Vec<(&i32, &&str)> = map.range(1..=2).collect();
//! assert_eq!(range.len(), 2); // 1 and 2
//! ```

// =============================================================================
// Reference Counter Type Alias
// =============================================================================

/// Reference-counted smart pointer type.
///
/// When the `arc` feature is enabled, this is `std::sync::Arc`,
/// which is thread-safe but has slightly higher overhead.
///
/// When the `arc` feature is disabled (default), this is `std::rc::Rc`,
/// which is faster but not thread-safe.
#[cfg(feature = "arc")]
pub(crate) type ReferenceCounter<T> = std::sync::Arc<T>;

#[cfg(not(feature = "arc"))]
pub(crate) type ReferenceCounter<T> = std::rc::Rc<T>;

mod deque;
mod hashmap;
mod hashset;
mod list;
mod treemap;
mod vector;

pub use deque::PersistentDeque;
pub use hashmap::PersistentHashMap;
pub use hashmap::PersistentHashMapIntoIterator;
pub use hashmap::PersistentHashMapIterator;
pub use hashmap::TransientHashMap;
pub use hashset::HashSetView;
pub use hashset::PersistentHashSet;
pub use hashset::PersistentHashSetIntoIterator;
pub use hashset::PersistentHashSetIterator;
pub use hashset::TransientHashSet;
pub use list::PersistentList;
pub use list::PersistentListIntoIterator;
pub use list::PersistentListIterator;
pub use treemap::PersistentTreeMap;
pub use treemap::PersistentTreeMapIntoIterator;
pub use treemap::PersistentTreeMapIterator;
pub use treemap::PersistentTreeMapRangeIterator;
pub use vector::PersistentVector;
pub use vector::PersistentVectorIntoIterator;
pub use vector::PersistentVectorIterator;
pub use vector::TransientVector;

// Rayon parallel iterator re-exports
#[cfg(feature = "rayon")]
pub use hashmap::PersistentHashMapParallelIterator;
#[cfg(feature = "rayon")]
pub use hashmap::PersistentHashMapParallelRefIterator;
#[cfg(feature = "rayon")]
pub use hashset::PersistentHashSetParallelIterator;
#[cfg(feature = "rayon")]
pub use hashset::PersistentHashSetParallelRefIterator;
#[cfg(feature = "rayon")]
pub use list::PersistentListParallelIterator;
#[cfg(feature = "rayon")]
pub use list::PersistentListParallelRefIterator;
#[cfg(feature = "rayon")]
pub use treemap::PersistentTreeMapParallelIterator;
#[cfg(feature = "rayon")]
pub use treemap::PersistentTreeMapParallelRefIterator;
#[cfg(feature = "rayon")]
pub use vector::PersistentVectorParallelIterator;
#[cfg(feature = "rayon")]
pub use vector::PersistentVectorParallelRefIterator;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod reference_counter_tests {
    use super::ReferenceCounter;
    use rstest::rstest;

    #[rstest]
    fn test_reference_counter_clone() {
        let reference_counter: ReferenceCounter<i32> = ReferenceCounter::new(42);
        let reference_counter_clone = reference_counter.clone();
        assert_eq!(*reference_counter, *reference_counter_clone);
    }

    #[rstest]
    fn test_reference_counter_strong_count() {
        let reference_counter: ReferenceCounter<i32> = ReferenceCounter::new(42);
        assert_eq!(ReferenceCounter::strong_count(&reference_counter), 1);
        let reference_counter_clone = reference_counter.clone();
        assert_eq!(ReferenceCounter::strong_count(&reference_counter), 2);
        drop(reference_counter_clone);
        assert_eq!(ReferenceCounter::strong_count(&reference_counter), 1);
    }
}
