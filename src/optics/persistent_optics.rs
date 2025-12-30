//! Optics integration for persistent data structures.
//!
//! This module provides Optional and Traversal implementations for
//! the persistent data structures: PersistentVector, PersistentHashMap,
//! and PersistentTreeMap.
//!
//! # Overview
//!
//! - [`PersistentVectorIndexOptional`]: Focuses on an element at a specific index
//! - [`PersistentVectorTraversal`]: Traverses all elements of a vector
//! - [`PersistentHashMapKeyOptional`]: Focuses on a value for a specific key
//! - [`PersistentHashMapTraversal`]: Traverses all values of a hash map
//! - [`PersistentTreeMapKeyOptional`]: Focuses on a value for a specific key
//! - [`PersistentTreeMapTraversal`]: Traverses all values of a tree map (in key order)
//!
//! # Examples
//!
//! ## PersistentVector with Optional
//!
//! ```
//! use functional_rusty::persistent::PersistentVector;
//! use functional_rusty::optics::{Optional, persistent_optics::index_optional};
//!
//! let vector: PersistentVector<i32> = (1..=5).collect();
//! let optional = index_optional::<i32>(2);
//!
//! assert_eq!(optional.get_option(&vector), Some(&3));
//!
//! let updated = optional.set(vector.clone(), 100);
//! assert_eq!(updated.get(2), Some(&100));
//! ```
//!
//! ## PersistentVector with Traversal
//!
//! ```
//! use functional_rusty::persistent::PersistentVector;
//! use functional_rusty::optics::{Traversal, persistent_optics::persistent_vector_traversal};
//!
//! let vector: PersistentVector<i32> = (1..=5).collect();
//! let traversal = persistent_vector_traversal::<i32>();
//!
//! let doubled = traversal.modify_all(vector, |x| x * 2);
//! // [2, 4, 6, 8, 10]
//! ```
//!
//! ## PersistentHashMap with Optional
//!
//! ```
//! use functional_rusty::persistent::PersistentHashMap;
//! use functional_rusty::optics::{Optional, persistent_optics::key_optional_hashmap};
//!
//! let map = PersistentHashMap::new()
//!     .insert("key".to_string(), 42);
//! let optional = key_optional_hashmap::<String, i32>("key".to_string());
//!
//! assert_eq!(optional.get_option(&map), Some(&42));
//! ```

#![forbid(unsafe_code)]

use std::hash::Hash;
use std::marker::PhantomData;

use crate::optics::{Optional, Traversal};
use crate::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};

// =============================================================================
// PersistentVector Optional
// =============================================================================

/// An Optional that focuses on an element at a specific index in a PersistentVector.
///
/// This Optional returns `Some` if the index is within bounds, `None` otherwise.
///
/// # Type Parameters
///
/// - `T`: The element type of the vector
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentVector;
/// use functional_rusty::optics::{Optional, persistent_optics::index_optional};
///
/// let vector: PersistentVector<i32> = (1..=5).collect();
/// let optional = index_optional::<i32>(2);
///
/// assert_eq!(optional.get_option(&vector), Some(&3));
/// assert_eq!(optional.is_present(&vector), true);
///
/// // Out of bounds returns None
/// let out_of_bounds = index_optional::<i32>(10);
/// assert_eq!(out_of_bounds.get_option(&vector), None);
/// ```
#[derive(Debug, Clone)]
pub struct PersistentVectorIndexOptional<T> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T> PersistentVectorIndexOptional<T> {
    /// Creates a new PersistentVectorIndexOptional for the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index to focus on
    ///
    /// # Returns
    ///
    /// A new PersistentVectorIndexOptional
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }
}

/// Creates a PersistentVectorIndexOptional for the given index.
///
/// This is a convenience function for creating a PersistentVectorIndexOptional.
///
/// # Type Parameters
///
/// - `T`: The element type of the vector
///
/// # Arguments
///
/// * `index` - The zero-based index to focus on
///
/// # Returns
///
/// A new PersistentVectorIndexOptional
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentVector;
/// use functional_rusty::optics::{Optional, persistent_optics::index_optional};
///
/// let vector: PersistentVector<i32> = (1..=5).collect();
/// let optional = index_optional::<i32>(0);
///
/// assert_eq!(optional.get_option(&vector), Some(&1));
/// ```
#[must_use]
pub const fn index_optional<T>(index: usize) -> PersistentVectorIndexOptional<T> {
    PersistentVectorIndexOptional::new(index)
}

impl<T: Clone> Optional<PersistentVector<T>, T> for PersistentVectorIndexOptional<T> {
    fn get_option<'a>(&self, source: &'a PersistentVector<T>) -> Option<&'a T> {
        source.get(self.index)
    }

    fn set(&self, source: PersistentVector<T>, value: T) -> PersistentVector<T> {
        match source.update(self.index, value) {
            Some(updated) => updated,
            None => source, // Index out of bounds, return unchanged
        }
    }
}

// =============================================================================
// PersistentVector Traversal
// =============================================================================

/// A Traversal that focuses on all elements of a PersistentVector.
///
/// # Type Parameters
///
/// - `T`: The element type of the vector
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentVector;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_vector_traversal};
///
/// let vector: PersistentVector<i32> = (1..=5).collect();
/// let traversal = persistent_vector_traversal::<i32>();
///
/// let sum: i32 = traversal.get_all(&vector).sum();
/// assert_eq!(sum, 15);
///
/// let doubled = traversal.modify_all(vector, |x| x * 2);
/// assert_eq!(doubled.get(0), Some(&2));
/// ```
#[derive(Debug, Clone)]
pub struct PersistentVectorTraversal<T> {
    _marker: PhantomData<T>,
}

impl<T> PersistentVectorTraversal<T> {
    /// Creates a new PersistentVectorTraversal.
    ///
    /// # Returns
    ///
    /// A new PersistentVectorTraversal
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for PersistentVectorTraversal<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a PersistentVectorTraversal.
///
/// This is a convenience function for creating a PersistentVectorTraversal.
///
/// # Type Parameters
///
/// - `T`: The element type of the vector
///
/// # Returns
///
/// A new PersistentVectorTraversal
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentVector;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_vector_traversal};
///
/// let vector: PersistentVector<i32> = (1..=3).collect();
/// let traversal = persistent_vector_traversal::<i32>();
///
/// let elements: Vec<&i32> = traversal.get_all(&vector).collect();
/// assert_eq!(elements, vec![&1, &2, &3]);
/// ```
#[must_use]
pub const fn persistent_vector_traversal<T>() -> PersistentVectorTraversal<T> {
    PersistentVectorTraversal::new()
}

impl<T: Clone + 'static> Traversal<PersistentVector<T>, T> for PersistentVectorTraversal<T> {
    fn get_all<'a>(&self, source: &'a PersistentVector<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: PersistentVector<T>) -> Vec<T> {
        source.into_iter().collect()
    }

    fn modify_all<F>(&self, source: PersistentVector<T>, mut function: F) -> PersistentVector<T>
    where
        F: FnMut(T) -> T,
    {
        source
            .into_iter()
            .map(|element| function(element))
            .collect()
    }
}

// =============================================================================
// PersistentHashMap Optional
// =============================================================================

/// An Optional that focuses on a value for a specific key in a PersistentHashMap.
///
/// This Optional returns `Some` if the key exists, `None` otherwise.
/// The `set` operation will insert or update the key-value pair.
///
/// # Type Parameters
///
/// - `K`: The key type of the map
/// - `V`: The value type of the map
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentHashMap;
/// use functional_rusty::optics::{Optional, persistent_optics::key_optional_hashmap};
///
/// let map = PersistentHashMap::new()
///     .insert("key".to_string(), 42);
/// let optional = key_optional_hashmap::<String, i32>("key".to_string());
///
/// assert_eq!(optional.get_option(&map), Some(&42));
///
/// let updated = optional.set(map.clone(), 100);
/// assert_eq!(updated.get("key"), Some(&100));
/// ```
#[derive(Debug, Clone)]
pub struct PersistentHashMapKeyOptional<K, V> {
    key: K,
    _marker: PhantomData<V>,
}

impl<K, V> PersistentHashMapKeyOptional<K, V> {
    /// Creates a new PersistentHashMapKeyOptional for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to focus on
    ///
    /// # Returns
    ///
    /// A new PersistentHashMapKeyOptional
    #[must_use]
    pub const fn new(key: K) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

/// Creates a PersistentHashMapKeyOptional for the given key.
///
/// This is a convenience function for creating a PersistentHashMapKeyOptional.
///
/// # Type Parameters
///
/// - `K`: The key type of the map
/// - `V`: The value type of the map
///
/// # Arguments
///
/// * `key` - The key to focus on
///
/// # Returns
///
/// A new PersistentHashMapKeyOptional
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentHashMap;
/// use functional_rusty::optics::{Optional, persistent_optics::key_optional_hashmap};
///
/// let map = PersistentHashMap::new()
///     .insert("hello".to_string(), "world".to_string());
/// let optional = key_optional_hashmap::<String, String>("hello".to_string());
///
/// assert!(optional.is_present(&map));
/// ```
#[must_use]
pub const fn key_optional_hashmap<K, V>(key: K) -> PersistentHashMapKeyOptional<K, V> {
    PersistentHashMapKeyOptional::new(key)
}

impl<K: Clone + Hash + Eq, V: Clone> Optional<PersistentHashMap<K, V>, V>
    for PersistentHashMapKeyOptional<K, V>
{
    fn get_option<'a>(&self, source: &'a PersistentHashMap<K, V>) -> Option<&'a V> {
        source.get(&self.key)
    }

    fn set(&self, source: PersistentHashMap<K, V>, value: V) -> PersistentHashMap<K, V> {
        source.insert(self.key.clone(), value)
    }
}

// =============================================================================
// PersistentHashMap Traversal
// =============================================================================

/// A Traversal that focuses on all values of a PersistentHashMap.
///
/// Note that the iteration order is not guaranteed.
///
/// # Type Parameters
///
/// - `K`: The key type of the map
/// - `V`: The value type of the map
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentHashMap;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_hashmap_traversal};
///
/// let map = PersistentHashMap::new()
///     .insert("a".to_string(), 1)
///     .insert("b".to_string(), 2);
/// let traversal = persistent_hashmap_traversal::<String, i32>();
///
/// let sum: i32 = traversal.get_all(&map).sum();
/// assert_eq!(sum, 3);
/// ```
#[derive(Debug, Clone)]
pub struct PersistentHashMapTraversal<K, V> {
    _marker: PhantomData<(K, V)>,
}

impl<K, V> PersistentHashMapTraversal<K, V> {
    /// Creates a new PersistentHashMapTraversal.
    ///
    /// # Returns
    ///
    /// A new PersistentHashMapTraversal
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V> Default for PersistentHashMapTraversal<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a PersistentHashMapTraversal.
///
/// This is a convenience function for creating a PersistentHashMapTraversal.
///
/// # Type Parameters
///
/// - `K`: The key type of the map
/// - `V`: The value type of the map
///
/// # Returns
///
/// A new PersistentHashMapTraversal
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentHashMap;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_hashmap_traversal};
///
/// let map = PersistentHashMap::new()
///     .insert("x".to_string(), 10)
///     .insert("y".to_string(), 20);
/// let traversal = persistent_hashmap_traversal::<String, i32>();
///
/// assert_eq!(traversal.length(&map), 2);
/// ```
#[must_use]
pub const fn persistent_hashmap_traversal<K, V>() -> PersistentHashMapTraversal<K, V> {
    PersistentHashMapTraversal::new()
}

impl<K: Clone + Hash + Eq + 'static, V: Clone + 'static> Traversal<PersistentHashMap<K, V>, V>
    for PersistentHashMapTraversal<K, V>
{
    fn get_all<'a>(
        &self,
        source: &'a PersistentHashMap<K, V>,
    ) -> Box<dyn Iterator<Item = &'a V> + 'a> {
        Box::new(source.iter().map(|(_, value)| value))
    }

    fn get_all_owned(&self, source: PersistentHashMap<K, V>) -> Vec<V> {
        source.into_iter().map(|(_, value)| value).collect()
    }

    fn modify_all<F>(
        &self,
        source: PersistentHashMap<K, V>,
        mut function: F,
    ) -> PersistentHashMap<K, V>
    where
        F: FnMut(V) -> V,
    {
        let mut result = PersistentHashMap::new();
        for (key, value) in source.into_iter() {
            result = result.insert(key, function(value));
        }
        result
    }
}

// =============================================================================
// PersistentTreeMap Optional
// =============================================================================

/// An Optional that focuses on a value for a specific key in a PersistentTreeMap.
///
/// This Optional returns `Some` if the key exists, `None` otherwise.
/// The `set` operation will insert or update the key-value pair.
///
/// # Type Parameters
///
/// - `K`: The key type of the map (must implement Ord)
/// - `V`: The value type of the map
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentTreeMap;
/// use functional_rusty::optics::{Optional, persistent_optics::key_optional_treemap};
///
/// let map = PersistentTreeMap::new()
///     .insert(1, "one")
///     .insert(2, "two");
/// let optional = key_optional_treemap::<i32, &str>(1);
///
/// assert_eq!(optional.get_option(&map), Some(&"one"));
///
/// let updated = optional.set(map.clone(), "ONE");
/// assert_eq!(updated.get(&1), Some(&"ONE"));
/// ```
#[derive(Debug, Clone)]
pub struct PersistentTreeMapKeyOptional<K, V> {
    key: K,
    _marker: PhantomData<V>,
}

impl<K, V> PersistentTreeMapKeyOptional<K, V> {
    /// Creates a new PersistentTreeMapKeyOptional for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to focus on
    ///
    /// # Returns
    ///
    /// A new PersistentTreeMapKeyOptional
    #[must_use]
    pub const fn new(key: K) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

/// Creates a PersistentTreeMapKeyOptional for the given key.
///
/// This is a convenience function for creating a PersistentTreeMapKeyOptional.
///
/// # Type Parameters
///
/// - `K`: The key type of the map (must implement Ord)
/// - `V`: The value type of the map
///
/// # Arguments
///
/// * `key` - The key to focus on
///
/// # Returns
///
/// A new PersistentTreeMapKeyOptional
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentTreeMap;
/// use functional_rusty::optics::{Optional, persistent_optics::key_optional_treemap};
///
/// let map = PersistentTreeMap::new()
///     .insert(42, "answer");
/// let optional = key_optional_treemap::<i32, &str>(42);
///
/// assert!(optional.is_present(&map));
/// ```
#[must_use]
pub const fn key_optional_treemap<K, V>(key: K) -> PersistentTreeMapKeyOptional<K, V> {
    PersistentTreeMapKeyOptional::new(key)
}

impl<K: Clone + Ord, V: Clone> Optional<PersistentTreeMap<K, V>, V>
    for PersistentTreeMapKeyOptional<K, V>
{
    fn get_option<'a>(&self, source: &'a PersistentTreeMap<K, V>) -> Option<&'a V> {
        source.get(&self.key)
    }

    fn set(&self, source: PersistentTreeMap<K, V>, value: V) -> PersistentTreeMap<K, V> {
        source.insert(self.key.clone(), value)
    }
}

// =============================================================================
// PersistentTreeMap Traversal
// =============================================================================

/// A Traversal that focuses on all values of a PersistentTreeMap.
///
/// The values are yielded in key order (ascending).
///
/// # Type Parameters
///
/// - `K`: The key type of the map (must implement Ord)
/// - `V`: The value type of the map
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentTreeMap;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_treemap_traversal};
///
/// let map = PersistentTreeMap::new()
///     .insert(3, "three")
///     .insert(1, "one")
///     .insert(2, "two");
/// let traversal = persistent_treemap_traversal::<i32, &str>();
///
/// // Values are in key order: 1, 2, 3
/// let values: Vec<&&str> = traversal.get_all(&map).collect();
/// assert_eq!(values, vec![&"one", &"two", &"three"]);
/// ```
#[derive(Debug, Clone)]
pub struct PersistentTreeMapTraversal<K, V> {
    _marker: PhantomData<(K, V)>,
}

impl<K, V> PersistentTreeMapTraversal<K, V> {
    /// Creates a new PersistentTreeMapTraversal.
    ///
    /// # Returns
    ///
    /// A new PersistentTreeMapTraversal
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V> Default for PersistentTreeMapTraversal<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a PersistentTreeMapTraversal.
///
/// This is a convenience function for creating a PersistentTreeMapTraversal.
///
/// # Type Parameters
///
/// - `K`: The key type of the map (must implement Ord)
/// - `V`: The value type of the map
///
/// # Returns
///
/// A new PersistentTreeMapTraversal
///
/// # Example
///
/// ```
/// use functional_rusty::persistent::PersistentTreeMap;
/// use functional_rusty::optics::{Traversal, persistent_optics::persistent_treemap_traversal};
///
/// let map = PersistentTreeMap::new()
///     .insert(1, 10)
///     .insert(2, 20);
/// let traversal = persistent_treemap_traversal::<i32, i32>();
///
/// let sum: i32 = traversal.get_all(&map).sum();
/// assert_eq!(sum, 30);
/// ```
#[must_use]
pub const fn persistent_treemap_traversal<K, V>() -> PersistentTreeMapTraversal<K, V> {
    PersistentTreeMapTraversal::new()
}

impl<K: Clone + Ord + 'static, V: Clone + 'static> Traversal<PersistentTreeMap<K, V>, V>
    for PersistentTreeMapTraversal<K, V>
{
    fn get_all<'a>(
        &self,
        source: &'a PersistentTreeMap<K, V>,
    ) -> Box<dyn Iterator<Item = &'a V> + 'a> {
        Box::new(source.iter().map(|(_, value)| value))
    }

    fn get_all_owned(&self, source: PersistentTreeMap<K, V>) -> Vec<V> {
        source.into_iter().map(|(_, value)| value).collect()
    }

    fn modify_all<F>(
        &self,
        source: PersistentTreeMap<K, V>,
        mut function: F,
    ) -> PersistentTreeMap<K, V>
    where
        F: FnMut(V) -> V,
    {
        let mut result = PersistentTreeMap::new();
        for (key, value) in source.into_iter() {
            result = result.insert(key, function(value));
        }
        result
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod persistent_vector_index_optional_tests {
        use super::*;

        #[test]
        fn test_new_creates_optional_with_index() {
            let optional: PersistentVectorIndexOptional<i32> =
                PersistentVectorIndexOptional::new(5);
            let vector: PersistentVector<i32> = (0..=10).collect();
            assert_eq!(optional.get_option(&vector), Some(&5));
        }

        #[test]
        fn test_index_optional_convenience_function() {
            let optional = index_optional::<i32>(3);
            let vector: PersistentVector<i32> = (0..=5).collect();
            assert_eq!(optional.get_option(&vector), Some(&3));
        }

        #[test]
        fn test_clone() {
            let optional = index_optional::<i32>(2);
            let cloned = optional.clone();
            let vector: PersistentVector<i32> = (0..=5).collect();
            assert_eq!(cloned.get_option(&vector), Some(&2));
        }
    }

    mod persistent_vector_traversal_tests {
        use super::*;

        #[test]
        fn test_new_creates_traversal() {
            let traversal: PersistentVectorTraversal<i32> = PersistentVectorTraversal::new();
            let vector: PersistentVector<i32> = (1..=3).collect();
            assert_eq!(traversal.length(&vector), 3);
        }

        #[test]
        fn test_default() {
            let traversal: PersistentVectorTraversal<i32> = PersistentVectorTraversal::default();
            let vector: PersistentVector<i32> = (1..=3).collect();
            assert_eq!(traversal.length(&vector), 3);
        }

        #[test]
        fn test_clone() {
            let traversal = persistent_vector_traversal::<i32>();
            let cloned = traversal.clone();
            let vector: PersistentVector<i32> = (1..=3).collect();
            assert_eq!(cloned.length(&vector), 3);
        }
    }

    mod persistent_hashmap_key_optional_tests {
        use super::*;

        #[test]
        fn test_new_creates_optional_with_key() {
            let optional: PersistentHashMapKeyOptional<String, i32> =
                PersistentHashMapKeyOptional::new("test".to_string());
            let map = PersistentHashMap::new().insert("test".to_string(), 42);
            assert_eq!(optional.get_option(&map), Some(&42));
        }

        #[test]
        fn test_clone() {
            let optional = key_optional_hashmap::<String, i32>("key".to_string());
            let cloned = optional.clone();
            let map = PersistentHashMap::new().insert("key".to_string(), 100);
            assert_eq!(cloned.get_option(&map), Some(&100));
        }
    }

    mod persistent_hashmap_traversal_tests {
        use super::*;

        #[test]
        fn test_new_creates_traversal() {
            let traversal: PersistentHashMapTraversal<String, i32> =
                PersistentHashMapTraversal::new();
            let map = PersistentHashMap::new()
                .insert("a".to_string(), 1)
                .insert("b".to_string(), 2);
            assert_eq!(traversal.length(&map), 2);
        }

        #[test]
        fn test_default() {
            let traversal: PersistentHashMapTraversal<String, i32> =
                PersistentHashMapTraversal::default();
            let map = PersistentHashMap::new().insert("x".to_string(), 10);
            assert_eq!(traversal.length(&map), 1);
        }
    }

    mod persistent_treemap_key_optional_tests {
        use super::*;

        #[test]
        fn test_new_creates_optional_with_key() {
            let optional: PersistentTreeMapKeyOptional<i32, String> =
                PersistentTreeMapKeyOptional::new(1);
            let map = PersistentTreeMap::new().insert(1, "one".to_string());
            assert_eq!(optional.get_option(&map), Some(&"one".to_string()));
        }

        #[test]
        fn test_clone() {
            let optional = key_optional_treemap::<i32, String>(5);
            let cloned = optional.clone();
            let map = PersistentTreeMap::new().insert(5, "five".to_string());
            assert_eq!(cloned.get_option(&map), Some(&"five".to_string()));
        }
    }

    mod persistent_treemap_traversal_tests {
        use super::*;

        #[test]
        fn test_new_creates_traversal() {
            let traversal: PersistentTreeMapTraversal<i32, String> =
                PersistentTreeMapTraversal::new();
            let map = PersistentTreeMap::new()
                .insert(1, "one".to_string())
                .insert(2, "two".to_string());
            assert_eq!(traversal.length(&map), 2);
        }

        #[test]
        fn test_default() {
            let traversal: PersistentTreeMapTraversal<i32, i32> =
                PersistentTreeMapTraversal::default();
            let map = PersistentTreeMap::new().insert(1, 100);
            assert_eq!(traversal.length(&map), 1);
        }
    }
}
