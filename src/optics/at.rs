//! At combinator for key-based access with insertion/deletion.
//!
//! This module provides the [`At`] trait which enables Optional-like access
//! to map-like structures where values can be inserted or deleted by key.
//!
//! Unlike [`Ixed`](crate::optics::ixed::Ixed), `At` provides full access to the presence or absence
//! of a value, allowing insertion of new keys and deletion of existing ones.
//!
//! # Examples
//!
//! ```
//! use std::collections::HashMap;
//! use lambars::optics::{Optional, at::At};
//!
//! let mut map: HashMap<String, i32> = HashMap::new();
//! map.insert("key".to_string(), 42);
//!
//! let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());
//!
//! // Get value
//! assert_eq!(optional.get_option(&map), Some(&42));
//!
//! // Set value (update existing key)
//! let map = optional.set(map, 100);
//! assert_eq!(map.get("key"), Some(&100));
//! ```
//!
//! # Difference from Ixed
//!
//! - `At`: Returns `Lens<S, Option<V>>`, allows insertion/deletion
//! - `Ixed`: Returns `Optional<S, V>`, only accesses existing values

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use crate::optics::Optional;

/// A trait for types that support key-based access with insertion/deletion.
///
/// Types implementing this trait can provide an Optional that focuses on a
/// value at a specific key, with the ability to insert new values or delete
/// existing ones.
///
/// # Difference from [`Ixed`](crate::optics::ixed::Ixed)
///
/// - `At`: Allows insertion of new keys via `set`
/// - `Ixed`: Only accesses existing keys, `set` on non-existent keys is a no-op
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use lambars::optics::{Optional, at::At};
///
/// let mut map: HashMap<String, i32> = HashMap::new();
/// let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());
///
/// // Insert a new key
/// let map = optional.set(map, 42);
/// assert_eq!(map.get("key"), Some(&42));
/// ```
pub trait At<K>: Sized {
    /// The value type stored in this container.
    type Value;

    /// The Optional type for accessing values at a key.
    type AtOptional: Optional<Self, Self::Value>;

    /// Returns an Optional that focuses on the value at the given key.
    fn at(key: K) -> Self::AtOptional;
}

/// An Optional for `HashMap` that focuses on a value at a specific key.
///
/// Unlike [`HashMapIx`](crate::optics::ixed::HashMapIx), this Optional will
/// insert the key if it doesn't exist when using `set`.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use lambars::optics::{Optional, at::HashMapAt};
///
/// let optional = HashMapAt::<String, i32>::new("key".to_string());
/// let map: HashMap<String, i32> = HashMap::new();
///
/// // Insert a new key
/// let map = optional.set(map, 42);
/// assert_eq!(map.get("key"), Some(&42));
/// ```
#[derive(Debug, Clone)]
pub struct HashMapAt<K, V> {
    key: K,
    _marker: PhantomData<V>,
}

impl<K, V> HashMapAt<K, V> {
    /// Creates a new `HashMapAt` for the given key.
    #[must_use]
    pub const fn new(key: K) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

impl<K: Clone + Eq + Hash, V: Clone, S: BuildHasher + Default> Optional<HashMap<K, V, S>, V>
    for HashMapAt<K, V>
{
    fn get_option<'a>(&self, source: &'a HashMap<K, V, S>) -> Option<&'a V> {
        source.get(&self.key)
    }

    fn set(&self, mut source: HashMap<K, V, S>, value: V) -> HashMap<K, V, S> {
        source.insert(self.key.clone(), value);
        source
    }
}

impl<K: Clone + Eq + Hash, V: Clone, S: BuildHasher + Default> At<K> for HashMap<K, V, S> {
    type Value = V;
    type AtOptional = HashMapAt<K, V>;

    fn at(key: K) -> Self::AtOptional {
        HashMapAt::new(key)
    }
}

// =============================================================================
// Persistent Data Structure Implementations
// =============================================================================

#[cfg(feature = "persistent")]
mod persistent_implementations {
    use super::{At, Hash, Optional, PhantomData};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap};

    /// An Optional for `PersistentHashMap` that focuses on a value at a specific key.
    #[derive(Debug, Clone)]
    pub struct PersistentHashMapAt<K, V> {
        key: K,
        _marker: PhantomData<V>,
    }

    impl<K, V> PersistentHashMapAt<K, V> {
        /// Creates a new `PersistentHashMapAt` for the given key.
        #[must_use]
        pub const fn new(key: K) -> Self {
            Self {
                key,
                _marker: PhantomData,
            }
        }
    }

    impl<K: Clone + Eq + Hash, V: Clone> Optional<PersistentHashMap<K, V>, V>
        for PersistentHashMapAt<K, V>
    {
        fn get_option<'a>(&self, source: &'a PersistentHashMap<K, V>) -> Option<&'a V> {
            source.get(&self.key)
        }

        fn set(&self, source: PersistentHashMap<K, V>, value: V) -> PersistentHashMap<K, V> {
            source.insert(self.key.clone(), value)
        }
    }

    impl<K: Clone + Eq + Hash, V: Clone> At<K> for PersistentHashMap<K, V> {
        type Value = V;
        type AtOptional = PersistentHashMapAt<K, V>;

        fn at(key: K) -> Self::AtOptional {
            PersistentHashMapAt::new(key)
        }
    }

    /// An Optional for `PersistentTreeMap` that focuses on a value at a specific key.
    #[derive(Debug, Clone)]
    pub struct PersistentTreeMapAt<K, V> {
        key: K,
        _marker: PhantomData<V>,
    }

    impl<K, V> PersistentTreeMapAt<K, V> {
        /// Creates a new `PersistentTreeMapAt` for the given key.
        #[must_use]
        pub const fn new(key: K) -> Self {
            Self {
                key,
                _marker: PhantomData,
            }
        }
    }

    impl<K: Clone + Ord, V: Clone> Optional<PersistentTreeMap<K, V>, V> for PersistentTreeMapAt<K, V> {
        fn get_option<'a>(&self, source: &'a PersistentTreeMap<K, V>) -> Option<&'a V> {
            source.get(&self.key)
        }

        fn set(&self, source: PersistentTreeMap<K, V>, value: V) -> PersistentTreeMap<K, V> {
            source.insert(self.key.clone(), value)
        }
    }

    impl<K: Clone + Ord, V: Clone> At<K> for PersistentTreeMap<K, V> {
        type Value = V;
        type AtOptional = PersistentTreeMapAt<K, V>;

        fn at(key: K) -> Self::AtOptional {
            PersistentTreeMapAt::new(key)
        }
    }
}

#[cfg(feature = "persistent")]
pub use persistent_implementations::*;

/// Convenience function to get an `At` Optional for a type.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use lambars::optics::{Optional, at::at};
///
/// let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
/// let optional = at::<HashMap<String, i32>, _>("key".to_string());
///
/// assert_eq!(optional.get_option(&map), Some(&42));
/// ```
pub fn at<T: At<K>, K>(key: K) -> T::AtOptional {
    T::at(key)
}

#[cfg(test)]
mod tests {
    use super::{At, HashMap, HashMapAt, at};
    use crate::optics::Optional;

    // =========================================================================
    // HashMapAt Tests
    // =========================================================================

    #[test]
    fn test_hashmap_at_get_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_hashmap_at_get_non_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_hashmap_at_set_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get("key"), Some(&100));
    }

    #[test]
    fn test_hashmap_at_set_new_key() {
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as At<String>>::at("new_key".to_string());

        let updated = optional.set(map, 42);
        assert_eq!(updated.get("new_key"), Some(&42));
    }

    #[test]
    fn test_hashmap_at_modify_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let modified = optional.modify(map, |x| x * 2);
        assert_eq!(modified.get("key"), Some(&84));
    }

    #[test]
    fn test_hashmap_at_is_present() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        assert!(optional.is_present(&map));

        let other_optional = <HashMap<String, i32> as At<String>>::at("other".to_string());
        assert!(!other_optional.is_present(&map));
    }

    #[test]
    fn test_hashmap_at_clone() {
        let optional = HashMapAt::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_hashmap_at_debug() {
        let optional = HashMapAt::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("HashMapAt"));
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_at_convenience_function() {
        let map: HashMap<String, i32> = std::iter::once(("a".to_string(), 1)).collect();
        let optional = at::<HashMap<String, i32>, _>("a".to_string());

        assert_eq!(optional.get_option(&map), Some(&1));
    }

    // =========================================================================
    // Optional Law Tests
    // =========================================================================

    #[test]
    fn test_hashmap_at_get_set_law() {
        // Law: optional.set(source, optional.get_option(&source).unwrap().clone()) == source
        // (when the key exists)
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        if let Some(value) = optional.get_option(&map) {
            let reconstructed = optional.set(map.clone(), *value);
            assert_eq!(reconstructed.get("key"), map.get("key"));
        }
    }

    #[test]
    fn test_hashmap_at_set_get_law() {
        // Law: optional.get_option(&optional.set(source, value)) == Some(&value)
        // (after setting, the value is always present)
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let updated = optional.set(map, 42);
        assert_eq!(optional.get_option(&updated), Some(&42));
    }

    #[test]
    fn test_hashmap_at_set_set_law() {
        // Law: optional.set(optional.set(source, v1), v2) == optional.set(source, v2)
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let set_twice = optional.set(optional.set(map.clone(), 42), 100);
        let set_once = optional.set(map, 100);

        assert_eq!(set_twice.get("key"), set_once.get("key"));
    }
}

#[cfg(all(test, feature = "persistent"))]
mod persistent_tests {
    use super::{At, Optional};
    use crate::optics::at::{PersistentHashMapAt, PersistentTreeMapAt};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap};

    // =========================================================================
    // PersistentHashMapAt Tests
    // =========================================================================

    #[test]
    fn test_persistent_hashmap_at_get_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as At<String>>::at("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_hashmap_at_get_non_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as At<String>>::at("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_persistent_hashmap_at_set_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as At<String>>::at("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get("key"), Some(&100));
    }

    #[test]
    fn test_persistent_hashmap_at_set_new_key() {
        let map = PersistentHashMap::<String, i32>::new();
        let optional = <PersistentHashMap<String, i32> as At<String>>::at("new_key".to_string());

        let updated = optional.set(map, 42);
        assert_eq!(updated.get("new_key"), Some(&42));
    }

    #[test]
    fn test_persistent_hashmap_at_modify() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as At<String>>::at("key".to_string());

        let modified = optional.modify(map, |x| x * 2);
        assert_eq!(modified.get("key"), Some(&84));
    }

    #[test]
    fn test_persistent_hashmap_at_clone() {
        let optional = PersistentHashMapAt::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map = PersistentHashMap::new().insert("key".to_string(), 42);

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_hashmap_at_debug() {
        let optional = PersistentHashMapAt::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentHashMapAt"));
    }

    // =========================================================================
    // PersistentTreeMapAt Tests
    // =========================================================================

    #[test]
    fn test_persistent_treemap_at_get_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as At<String>>::at("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_treemap_at_get_non_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as At<String>>::at("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_persistent_treemap_at_set_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as At<String>>::at("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get(&"key".to_string()), Some(&100));
    }

    #[test]
    fn test_persistent_treemap_at_set_new_key() {
        let map = PersistentTreeMap::<String, i32>::new();
        let optional = <PersistentTreeMap<String, i32> as At<String>>::at("new_key".to_string());

        let updated = optional.set(map, 42);
        assert_eq!(updated.get(&"new_key".to_string()), Some(&42));
    }

    #[test]
    fn test_persistent_treemap_at_modify() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as At<String>>::at("key".to_string());

        let modified = optional.modify(map, |x| x * 2);
        assert_eq!(modified.get(&"key".to_string()), Some(&84));
    }

    #[test]
    fn test_persistent_treemap_at_clone() {
        let optional = PersistentTreeMapAt::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_treemap_at_debug() {
        let optional = PersistentTreeMapAt::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentTreeMapAt"));
    }
}
