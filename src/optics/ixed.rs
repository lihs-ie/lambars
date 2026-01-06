//! Ixed combinator for index-based Optional access.
//!
//! This module provides the [`Ixed`] trait which enables Optional-like access
//! to elements in indexed structures like `Vec` and arrays.
//!
//! Unlike [`At`](crate::optics::at::At), `Ixed` only provides access to existing elements and cannot
//! be used to insert new elements at arbitrary indices.
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Optional, ixed::Ixed};
//!
//! let vec = vec![1, 2, 3, 4, 5];
//! let optional = <Vec<i32> as Ixed<usize>>::ix(2);
//!
//! // Get element at index 2
//! assert_eq!(optional.get_option(&vec), Some(&3));
//!
//! // Set element at index 2
//! let updated = optional.set(vec, 100);
//! assert_eq!(updated[2], 100);
//! ```
//!
//! # Difference from At
//!
//! - `At`: For maps, returns Optional, allows insertion/deletion
//! - `Ixed`: For indexed structures, only accesses existing elements

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use crate::optics::Optional;

/// A trait for types that support index-based access to elements.
///
/// Types implementing this trait can provide an Optional that focuses on an
/// element at a specific index.
///
/// # Difference from [`At`](crate::optics::at::At)
///
/// - `Ixed`: Only accesses existing elements, `set` on non-existent indices is a no-op
/// - `At`: For maps, allows insertion of new keys
///
/// # Examples
///
/// ```
/// use lambars::optics::{Optional, ixed::Ixed};
///
/// let vec = vec![1, 2, 3, 4, 5];
/// let optional = <Vec<i32> as Ixed<usize>>::ix(2);
///
/// assert_eq!(optional.get_option(&vec), Some(&3));
///
/// let updated = optional.set(vec, 100);
/// assert_eq!(updated[2], 100);
/// ```
pub trait Ixed<I>: Sized {
    /// The element type.
    type Element;

    /// The Optional type for accessing elements at an index.
    type IxOptional: Optional<Self, Self::Element>;

    /// Returns an Optional that focuses on the element at the given index.
    fn ix(index: I) -> Self::IxOptional;
}

/// An Optional for `Vec<T>` that focuses on an element at a specific index.
///
/// If the index is out of bounds, `get_option` returns `None` and `set` is a no-op.
///
/// # Examples
///
/// ```
/// use lambars::optics::{Optional, ixed::VecIx};
///
/// let optional = VecIx::<i32>::new(2);
/// let vec = vec![1, 2, 3, 4, 5];
///
/// assert_eq!(optional.get_option(&vec), Some(&3));
///
/// // Out of bounds access returns None
/// let out_of_bounds = VecIx::<i32>::new(10);
/// assert_eq!(out_of_bounds.get_option(&vec), None);
/// ```
#[derive(Debug, Clone)]
pub struct VecIx<T> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T> VecIx<T> {
    /// Creates a new `VecIx` for the given index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }
}

impl<T: Clone> Optional<Vec<T>, T> for VecIx<T> {
    fn get_option<'a>(&self, source: &'a Vec<T>) -> Option<&'a T> {
        source.get(self.index)
    }

    fn set(&self, mut source: Vec<T>, value: T) -> Vec<T> {
        if self.index < source.len() {
            source[self.index] = value;
        }
        source
    }
}

impl<T: Clone> Ixed<usize> for Vec<T> {
    type Element = T;
    type IxOptional = VecIx<T>;

    fn ix(index: usize) -> Self::IxOptional {
        VecIx::new(index)
    }
}

/// An Optional for `HashMap<K, V>` that focuses on a value at a specific key.
///
/// Unlike [`HashMapAt`](crate::optics::at::HashMapAt), this Optional will
/// NOT insert the key if it doesn't exist when using `set`.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use lambars::optics::{Optional, ixed::HashMapIx};
///
/// let optional = HashMapIx::<String, i32>::new("key".to_string());
/// let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
///
/// assert_eq!(optional.get_option(&map), Some(&42));
///
/// // set on non-existent key is a no-op
/// let empty_map: HashMap<String, i32> = HashMap::new();
/// let result = optional.set(empty_map.clone(), 100);
/// assert!(!result.contains_key("key"));
/// ```
#[derive(Debug, Clone)]
pub struct HashMapIx<K, V> {
    key: K,
    _marker: PhantomData<V>,
}

impl<K, V> HashMapIx<K, V> {
    /// Creates a new `HashMapIx` for the given key.
    #[must_use]
    pub const fn new(key: K) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

impl<K: Clone + Eq + Hash, V: Clone, S: BuildHasher + Default> Optional<HashMap<K, V, S>, V>
    for HashMapIx<K, V>
{
    fn get_option<'a>(&self, source: &'a HashMap<K, V, S>) -> Option<&'a V> {
        source.get(&self.key)
    }

    fn set(&self, mut source: HashMap<K, V, S>, value: V) -> HashMap<K, V, S> {
        if source.contains_key(&self.key) {
            source.insert(self.key.clone(), value);
        }
        source
    }
}

impl<K: Clone + Eq + Hash, V: Clone, S: BuildHasher + Default> Ixed<K> for HashMap<K, V, S> {
    type Element = V;
    type IxOptional = HashMapIx<K, V>;

    fn ix(key: K) -> Self::IxOptional {
        HashMapIx::new(key)
    }
}

// =============================================================================
// Persistent Data Structure Implementations
// =============================================================================

#[cfg(feature = "persistent")]
mod persistent_implementations {
    use super::{Hash, Ixed, Optional, PhantomData};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};

    /// An Optional for `PersistentVector<T>` that focuses on an element at a specific index.
    #[derive(Debug, Clone)]
    pub struct PersistentVectorIx<T> {
        index: usize,
        _marker: PhantomData<T>,
    }

    impl<T> PersistentVectorIx<T> {
        /// Creates a new `PersistentVectorIx` for the given index.
        #[must_use]
        pub const fn new(index: usize) -> Self {
            Self {
                index,
                _marker: PhantomData,
            }
        }
    }

    impl<T: Clone> Optional<PersistentVector<T>, T> for PersistentVectorIx<T> {
        fn get_option<'a>(&self, source: &'a PersistentVector<T>) -> Option<&'a T> {
            source.get(self.index)
        }

        fn set(&self, source: PersistentVector<T>, value: T) -> PersistentVector<T> {
            source.update(self.index, value).unwrap_or(source)
        }
    }

    impl<T: Clone> Ixed<usize> for PersistentVector<T> {
        type Element = T;
        type IxOptional = PersistentVectorIx<T>;

        fn ix(index: usize) -> Self::IxOptional {
            PersistentVectorIx::new(index)
        }
    }

    /// An Optional for `PersistentHashMap<K, V>` that focuses on a value at a specific key.
    #[derive(Debug, Clone)]
    pub struct PersistentHashMapIx<K, V> {
        key: K,
        _marker: PhantomData<V>,
    }

    impl<K, V> PersistentHashMapIx<K, V> {
        /// Creates a new `PersistentHashMapIx` for the given key.
        #[must_use]
        pub const fn new(key: K) -> Self {
            Self {
                key,
                _marker: PhantomData,
            }
        }
    }

    impl<K: Clone + Eq + Hash, V: Clone> Optional<PersistentHashMap<K, V>, V>
        for PersistentHashMapIx<K, V>
    {
        fn get_option<'a>(&self, source: &'a PersistentHashMap<K, V>) -> Option<&'a V> {
            source.get(&self.key)
        }

        fn set(&self, source: PersistentHashMap<K, V>, value: V) -> PersistentHashMap<K, V> {
            if source.contains_key(&self.key) {
                source.insert(self.key.clone(), value)
            } else {
                source
            }
        }
    }

    impl<K: Clone + Eq + Hash, V: Clone> Ixed<K> for PersistentHashMap<K, V> {
        type Element = V;
        type IxOptional = PersistentHashMapIx<K, V>;

        fn ix(key: K) -> Self::IxOptional {
            PersistentHashMapIx::new(key)
        }
    }

    /// An Optional for `PersistentTreeMap<K, V>` that focuses on a value at a specific key.
    #[derive(Debug, Clone)]
    pub struct PersistentTreeMapIx<K, V> {
        key: K,
        _marker: PhantomData<V>,
    }

    impl<K, V> PersistentTreeMapIx<K, V> {
        /// Creates a new `PersistentTreeMapIx` for the given key.
        #[must_use]
        pub const fn new(key: K) -> Self {
            Self {
                key,
                _marker: PhantomData,
            }
        }
    }

    impl<K: Clone + Ord, V: Clone> Optional<PersistentTreeMap<K, V>, V> for PersistentTreeMapIx<K, V> {
        fn get_option<'a>(&self, source: &'a PersistentTreeMap<K, V>) -> Option<&'a V> {
            source.get(&self.key)
        }

        fn set(&self, source: PersistentTreeMap<K, V>, value: V) -> PersistentTreeMap<K, V> {
            if source.contains_key(&self.key) {
                source.insert(self.key.clone(), value)
            } else {
                source
            }
        }
    }

    impl<K: Clone + Ord, V: Clone> Ixed<K> for PersistentTreeMap<K, V> {
        type Element = V;
        type IxOptional = PersistentTreeMapIx<K, V>;

        fn ix(key: K) -> Self::IxOptional {
            PersistentTreeMapIx::new(key)
        }
    }
}

#[cfg(feature = "persistent")]
pub use persistent_implementations::*;

/// Convenience function to get an `Ix` Optional for a type.
///
/// # Examples
///
/// ```
/// use lambars::optics::{Optional, ixed::ix};
///
/// let vec = vec![1, 2, 3];
/// let optional = ix::<Vec<i32>, _>(1);
///
/// assert_eq!(optional.get_option(&vec), Some(&2));
/// ```
pub fn ix<T: Ixed<I>, I>(index: I) -> T::IxOptional {
    T::ix(index)
}

/// Convenience function that is an alias for `ix`.
///
/// # Examples
///
/// ```
/// use lambars::optics::{Optional, ixed::index};
///
/// let vec = vec![1, 2, 3];
/// let optional = index::<Vec<i32>, _>(1);
///
/// assert_eq!(optional.get_option(&vec), Some(&2));
/// ```
pub fn index<T: Ixed<I>, I>(index: I) -> T::IxOptional {
    T::ix(index)
}

#[cfg(test)]
mod tests {
    use super::{HashMap, HashMapIx, Ixed, Optional, VecIx, index, ix};

    // =========================================================================
    // VecIx Tests
    // =========================================================================

    #[test]
    fn test_vec_ix_get_valid_index() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        assert_eq!(optional.get_option(&vec), Some(&3));
    }

    #[test]
    fn test_vec_ix_get_invalid_index() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(10);

        assert_eq!(optional.get_option(&vec), None);
    }

    #[test]
    fn test_vec_ix_set_valid_index() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        let updated = optional.set(vec, 100);
        assert_eq!(updated, vec![1, 2, 100, 4, 5]);
    }

    #[test]
    fn test_vec_ix_set_invalid_index() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(10);

        let updated = optional.set(vec.clone(), 100);
        assert_eq!(updated, vec);
    }

    #[test]
    fn test_vec_ix_modify() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        let modified = optional.modify(vec, |x| x * 2);
        assert_eq!(modified, vec![1, 2, 6, 4, 5]);
    }

    #[test]
    fn test_vec_ix_is_present() {
        let vec = vec![1, 2, 3, 4, 5];

        let valid_optional = <Vec<i32> as Ixed<usize>>::ix(2);
        assert!(valid_optional.is_present(&vec));

        let invalid_optional = <Vec<i32> as Ixed<usize>>::ix(10);
        assert!(!invalid_optional.is_present(&vec));
    }

    #[test]
    fn test_vec_ix_clone() {
        let optional = VecIx::<i32>::new(2);
        let cloned = optional;
        let vec = vec![1, 2, 3];

        assert_eq!(cloned.get_option(&vec), Some(&3));
    }

    #[test]
    fn test_vec_ix_debug() {
        let optional = VecIx::<i32>::new(2);
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("VecIx"));
    }

    // =========================================================================
    // HashMapIx Tests
    // =========================================================================

    #[test]
    fn test_hashmap_ix_get_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as Ixed<String>>::ix("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_hashmap_ix_get_non_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as Ixed<String>>::ix("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_hashmap_ix_set_existing() {
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();
        let optional = <HashMap<String, i32> as Ixed<String>>::ix("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get("key"), Some(&100));
    }

    #[test]
    fn test_hashmap_ix_set_non_existing_no_effect() {
        // Unlike At, Ixed does not insert new keys
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as Ixed<String>>::ix("new_key".to_string());

        let updated = optional.set(map, 42);
        assert!(!updated.contains_key("new_key"));
    }

    #[test]
    fn test_hashmap_ix_clone() {
        let optional = HashMapIx::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map: HashMap<String, i32> = std::iter::once(("key".to_string(), 42)).collect();

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_hashmap_ix_debug() {
        let optional = HashMapIx::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("HashMapIx"));
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_ix_convenience_function() {
        let vec = vec![1, 2, 3];
        let optional = ix::<Vec<i32>, _>(1);

        assert_eq!(optional.get_option(&vec), Some(&2));
    }

    #[test]
    fn test_index_convenience_function() {
        let vec = vec![1, 2, 3];
        let optional = index::<Vec<i32>, _>(1);

        assert_eq!(optional.get_option(&vec), Some(&2));
    }

    // =========================================================================
    // Optional Law Tests
    // =========================================================================

    #[test]
    fn test_vec_ix_get_set_law() {
        // Law: optional.set(source, optional.get_option(&source).unwrap().clone()) == source
        // (when the index is valid)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        if let Some(value) = optional.get_option(&vec) {
            let reconstructed = optional.set(vec.clone(), *value);
            assert_eq!(reconstructed, vec);
        }
    }

    #[test]
    fn test_vec_ix_set_get_law() {
        // Law: optional.get_option(&optional.set(source, value)) == Some(&value)
        // (when the index is valid)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        let updated = optional.set(vec, 100);
        assert_eq!(optional.get_option(&updated), Some(&100));
    }

    #[test]
    fn test_vec_ix_set_set_law() {
        // Law: optional.set(optional.set(source, v1), v2) == optional.set(source, v2)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Ixed<usize>>::ix(2);

        let set_twice = optional.set(optional.set(vec.clone(), 42), 100);
        let set_once = optional.set(vec, 100);

        assert_eq!(set_twice, set_once);
    }
}

#[cfg(all(test, feature = "persistent"))]
mod persistent_tests {
    use super::{Ixed, Optional};
    use crate::optics::ixed::{PersistentHashMapIx, PersistentTreeMapIx, PersistentVectorIx};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};

    // =========================================================================
    // PersistentVectorIx Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_ix_get_valid() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Ixed<usize>>::ix(2);

        assert_eq!(optional.get_option(&vector), Some(&3));
    }

    #[test]
    fn test_persistent_vector_ix_get_invalid() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Ixed<usize>>::ix(10);

        assert_eq!(optional.get_option(&vector), None);
    }

    #[test]
    fn test_persistent_vector_ix_set_valid() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Ixed<usize>>::ix(2);

        let updated = optional.set(vector, 100);
        assert_eq!(updated.get(2), Some(&100));
    }

    #[test]
    fn test_persistent_vector_ix_set_invalid() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Ixed<usize>>::ix(10);

        let updated = optional.set(vector.clone(), 100);
        assert_eq!(updated.len(), vector.len());
    }

    #[test]
    fn test_persistent_vector_ix_modify() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Ixed<usize>>::ix(2);

        let modified = optional.modify(vector, |x| x * 2);
        assert_eq!(modified.get(2), Some(&6));
    }

    #[test]
    fn test_persistent_vector_ix_clone() {
        let optional = PersistentVectorIx::<i32>::new(2);
        let cloned = optional;
        let vector: PersistentVector<i32> = (1..=5).collect();

        assert_eq!(cloned.get_option(&vector), Some(&3));
    }

    #[test]
    fn test_persistent_vector_ix_debug() {
        let optional = PersistentVectorIx::<i32>::new(2);
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentVectorIx"));
    }

    // =========================================================================
    // PersistentHashMapIx Tests
    // =========================================================================

    #[test]
    fn test_persistent_hashmap_ix_get_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as Ixed<String>>::ix("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_hashmap_ix_get_non_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as Ixed<String>>::ix("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_persistent_hashmap_ix_set_existing() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = <PersistentHashMap<String, i32> as Ixed<String>>::ix("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get("key"), Some(&100));
    }

    #[test]
    fn test_persistent_hashmap_ix_set_non_existing_no_effect() {
        let map = PersistentHashMap::<String, i32>::new();
        let optional = <PersistentHashMap<String, i32> as Ixed<String>>::ix("new_key".to_string());

        let updated = optional.set(map, 42);
        assert!(!updated.contains_key("new_key"));
    }

    #[test]
    fn test_persistent_hashmap_ix_clone() {
        let optional = PersistentHashMapIx::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map = PersistentHashMap::new().insert("key".to_string(), 42);

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_hashmap_ix_debug() {
        let optional = PersistentHashMapIx::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentHashMapIx"));
    }

    // =========================================================================
    // PersistentTreeMapIx Tests
    // =========================================================================

    #[test]
    fn test_persistent_treemap_ix_get_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as Ixed<String>>::ix("key".to_string());

        assert_eq!(optional.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_treemap_ix_get_non_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as Ixed<String>>::ix("other".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[test]
    fn test_persistent_treemap_ix_set_existing() {
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);
        let optional = <PersistentTreeMap<String, i32> as Ixed<String>>::ix("key".to_string());

        let updated = optional.set(map, 100);
        assert_eq!(updated.get(&"key".to_string()), Some(&100));
    }

    #[test]
    fn test_persistent_treemap_ix_set_non_existing_no_effect() {
        let map = PersistentTreeMap::<String, i32>::new();
        let optional = <PersistentTreeMap<String, i32> as Ixed<String>>::ix("new_key".to_string());

        let updated = optional.set(map, 42);
        assert!(!updated.contains_key(&"new_key".to_string()));
    }

    #[test]
    fn test_persistent_treemap_ix_clone() {
        let optional = PersistentTreeMapIx::<String, i32>::new("key".to_string());
        let cloned = optional;
        let map = PersistentTreeMap::new().insert("key".to_string(), 42);

        assert_eq!(cloned.get_option(&map), Some(&42));
    }

    #[test]
    fn test_persistent_treemap_ix_debug() {
        let optional = PersistentTreeMapIx::<String, i32>::new("key".to_string());
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentTreeMapIx"));
    }
}
