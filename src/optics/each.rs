//! Each combinator for element-wise traversal of containers.
//!
//! This module provides the [`Each`] trait and implementations for common
//! container types, enabling element-wise traversal operations.
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Traversal, each::Each};
//!
//! // Access all elements of a Vec
//! let vec = vec![1, 2, 3, 4, 5];
//! let traversal = <Vec<i32> as Each>::each();
//!
//! // Get all elements
//! let elements: Vec<&i32> = traversal.get_all(&vec).collect();
//! assert_eq!(elements, vec![&1, &2, &3, &4, &5]);
//!
//! // Modify all elements
//! let doubled = traversal.modify_all(vec, |x| x * 2);
//! assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
//! ```
//!
//! # Standard Library Types
//!
//! Implementations are provided for:
//!
//! - [`Vec<T>`]
//! - [`Option<T>`]
//! - [`Result<T, E>`] (traverses the Ok value)
//!
//! # Persistent Data Structures
//!
//! When the `persistent` feature is enabled, implementations are also provided for:
//!
//! - `PersistentVector<T>`
//! - `PersistentHashMap<K, V>` (traverses values)
//! - `PersistentTreeMap<K, V>` (traverses values)

use std::marker::PhantomData;

use crate::optics::Traversal;

/// A trait for types that support element-wise traversal.
///
/// Types implementing this trait can provide a Traversal that focuses on
/// all of their elements.
pub trait Each: Sized {
    /// The element type.
    type Element;

    /// The traversal type for this container.
    type EachTraversal: Traversal<Self, Self::Element>;

    /// Returns a Traversal that focuses on all elements.
    fn each() -> Self::EachTraversal;
}

/// A Traversal for `Vec<T>`.
///
/// This traversal focuses on all elements of a vector.
#[derive(Debug, Clone)]
pub struct VecEach<T> {
    _marker: PhantomData<T>,
}

impl<T> VecEach<T> {
    /// Creates a new `VecEach`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for VecEach<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> Traversal<Vec<T>, T> for VecEach<T> {
    fn get_all<'a>(&self, source: &'a Vec<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Vec<T>) -> Vec<T> {
        source
    }

    fn modify_all<F>(&self, source: Vec<T>, function: F) -> Vec<T>
    where
        F: FnMut(T) -> T,
    {
        source.into_iter().map(function).collect()
    }
}

impl<T: Clone + 'static> Each for Vec<T> {
    type Element = T;
    type EachTraversal = VecEach<T>;

    fn each() -> Self::EachTraversal {
        VecEach::new()
    }
}

/// A Traversal for `Option<T>`.
///
/// This traversal focuses on the contained value, if present.
#[derive(Debug, Clone)]
pub struct OptionEach<T> {
    _marker: PhantomData<T>,
}

impl<T> OptionEach<T> {
    /// Creates a new `OptionEach`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for OptionEach<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> Traversal<Option<T>, T> for OptionEach<T> {
    fn get_all<'a>(&self, source: &'a Option<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Option<T>) -> Vec<T> {
        source.into_iter().collect()
    }

    fn modify_all<F>(&self, source: Option<T>, function: F) -> Option<T>
    where
        F: FnMut(T) -> T,
    {
        source.map(function)
    }
}

impl<T: Clone + 'static> Each for Option<T> {
    type Element = T;
    type EachTraversal = OptionEach<T>;

    fn each() -> Self::EachTraversal {
        OptionEach::new()
    }
}

/// A Traversal for `Result<T, E>`.
///
/// This traversal focuses on the Ok value, if present.
#[derive(Debug, Clone)]
pub struct ResultEach<T, E> {
    _marker: PhantomData<(T, E)>,
}

impl<T, E> ResultEach<T, E> {
    /// Creates a new `ResultEach`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T, E> Default for ResultEach<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static, E: Clone + 'static> Traversal<Result<T, E>, T> for ResultEach<T, E> {
    fn get_all<'a>(&self, source: &'a Result<T, E>) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        Box::new(source.iter())
    }

    fn get_all_owned(&self, source: Result<T, E>) -> Vec<T> {
        source.into_iter().collect()
    }

    fn modify_all<F>(&self, source: Result<T, E>, function: F) -> Result<T, E>
    where
        F: FnMut(T) -> T,
    {
        source.map(function)
    }
}

impl<T: Clone + 'static, E: Clone + 'static> Each for Result<T, E> {
    type Element = T;
    type EachTraversal = ResultEach<T, E>;

    fn each() -> Self::EachTraversal {
        ResultEach::new()
    }
}

// =============================================================================
// Persistent Data Structure Implementations
// =============================================================================

#[cfg(feature = "persistent")]
mod persistent_implementations {
    use super::{Each, PhantomData, Traversal};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};
    use std::hash::Hash;

    /// A Traversal for `PersistentVector<T>`.
    #[derive(Debug, Clone)]
    pub struct PersistentVectorEach<T> {
        _marker: PhantomData<T>,
    }

    impl<T> PersistentVectorEach<T> {
        /// Creates a new `PersistentVectorEach`.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<T> Default for PersistentVectorEach<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Clone + 'static> Traversal<PersistentVector<T>, T> for PersistentVectorEach<T> {
        fn get_all<'a>(
            &self,
            source: &'a PersistentVector<T>,
        ) -> Box<dyn Iterator<Item = &'a T> + 'a> {
            Box::new(source.iter())
        }

        fn get_all_owned(&self, source: PersistentVector<T>) -> Vec<T> {
            source.into_iter().collect()
        }

        fn modify_all<F>(&self, source: PersistentVector<T>, function: F) -> PersistentVector<T>
        where
            F: FnMut(T) -> T,
        {
            source.into_iter().map(function).collect()
        }
    }

    impl<T: Clone + 'static> Each for PersistentVector<T> {
        type Element = T;
        type EachTraversal = PersistentVectorEach<T>;

        fn each() -> Self::EachTraversal {
            PersistentVectorEach::new()
        }
    }

    /// A Traversal for values in `PersistentHashMap<K, V>`.
    #[derive(Debug, Clone)]
    pub struct PersistentHashMapEach<K, V> {
        _marker: PhantomData<(K, V)>,
    }

    impl<K, V> PersistentHashMapEach<K, V> {
        /// Creates a new `PersistentHashMapEach`.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<K, V> Default for PersistentHashMapEach<K, V> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<K: Clone + Hash + Eq + 'static, V: Clone + 'static> Traversal<PersistentHashMap<K, V>, V>
        for PersistentHashMapEach<K, V>
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
            for (key, value) in source {
                result = result.insert(key, function(value));
            }
            result
        }
    }

    impl<K: Clone + Hash + Eq + 'static, V: Clone + 'static> Each for PersistentHashMap<K, V> {
        type Element = V;
        type EachTraversal = PersistentHashMapEach<K, V>;

        fn each() -> Self::EachTraversal {
            PersistentHashMapEach::new()
        }
    }

    /// A Traversal for values in `PersistentTreeMap<K, V>`.
    #[derive(Debug, Clone)]
    pub struct PersistentTreeMapEach<K, V> {
        _marker: PhantomData<(K, V)>,
    }

    impl<K, V> PersistentTreeMapEach<K, V> {
        /// Creates a new `PersistentTreeMapEach`.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<K, V> Default for PersistentTreeMapEach<K, V> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<K: Clone + Ord + 'static, V: Clone + 'static> Traversal<PersistentTreeMap<K, V>, V>
        for PersistentTreeMapEach<K, V>
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
            for (key, value) in source {
                result = result.insert(key, function(value));
            }
            result
        }
    }

    impl<K: Clone + Ord + 'static, V: Clone + 'static> Each for PersistentTreeMap<K, V> {
        type Element = V;
        type EachTraversal = PersistentTreeMapEach<K, V>;

        fn each() -> Self::EachTraversal {
            PersistentTreeMapEach::new()
        }
    }
}

#[cfg(feature = "persistent")]
pub use persistent_implementations::*;

/// Convenience function to get the `each` traversal for a type.
///
/// # Example
///
/// ```
/// use lambars::optics::{Traversal, each::each};
///
/// let vec = vec![1, 2, 3, 4, 5];
/// let traversal = each::<Vec<i32>>();
///
/// let doubled = traversal.modify_all(vec, |x| x * 2);
/// assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
/// ```
#[must_use]
pub fn each<T: Each>() -> T::EachTraversal {
    T::each()
}

#[cfg(test)]
mod tests {
    use super::{Each, OptionEach, ResultEach, Traversal, VecEach, each};

    // =========================================================================
    // VecEach Tests
    // =========================================================================

    #[test]
    fn test_vec_each_get_all() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&vec).collect();
        assert_eq!(elements, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn test_vec_each_get_all_owned() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let elements = traversal.get_all_owned(vec);
        assert_eq!(elements, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_vec_each_modify_all() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let doubled = traversal.modify_all(vec, |x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_vec_each_set_all() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let zeroed = traversal.set_all(vec, 0);
        assert_eq!(zeroed, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_vec_each_empty() {
        let vec: Vec<i32> = vec![];
        let traversal = <Vec<i32> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&vec).collect();
        assert!(elements.is_empty());
    }

    #[test]
    fn test_vec_each_fold() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let sum = traversal.fold(&vec, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_vec_each_length() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        assert_eq!(traversal.length(&vec), 5);
    }

    #[test]
    fn test_vec_each_clone() {
        let traversal = VecEach::<i32>::new();
        let cloned = traversal;
        let vec = vec![1, 2, 3];

        assert_eq!(cloned.length(&vec), 3);
    }

    #[test]
    fn test_vec_each_debug() {
        let traversal = VecEach::<i32>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("VecEach"));
    }

    #[test]
    fn test_vec_each_default() {
        let traversal = VecEach::<i32>::default();
        let vec = vec![1, 2, 3];

        assert_eq!(traversal.length(&vec), 3);
    }

    // =========================================================================
    // OptionEach Tests
    // =========================================================================

    #[test]
    fn test_option_each_get_all_some() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&option).collect();
        assert_eq!(elements, vec![&42]);
    }

    #[test]
    fn test_option_each_get_all_none() {
        let option: Option<i32> = None;
        let traversal = <Option<i32> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&option).collect();
        assert!(elements.is_empty());
    }

    #[test]
    fn test_option_each_get_all_owned_some() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let elements = traversal.get_all_owned(option);
        assert_eq!(elements, vec![42]);
    }

    #[test]
    fn test_option_each_get_all_owned_none() {
        let option: Option<i32> = None;
        let traversal = <Option<i32> as Each>::each();

        let elements = traversal.get_all_owned(option);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_option_each_modify_all_some() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let doubled = traversal.modify_all(option, |x| x * 2);
        assert_eq!(doubled, Some(84));
    }

    #[test]
    fn test_option_each_modify_all_none() {
        let option: Option<i32> = None;
        let traversal = <Option<i32> as Each>::each();

        let result = traversal.modify_all(option, |x| x * 2);
        assert_eq!(result, None);
    }

    #[test]
    fn test_option_each_fold_some() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let sum = traversal.fold(&option, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 42);
    }

    #[test]
    fn test_option_each_fold_none() {
        let option: Option<i32> = None;
        let traversal = <Option<i32> as Each>::each();

        let sum = traversal.fold(&option, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_option_each_clone() {
        let traversal = OptionEach::<i32>::new();
        let cloned = traversal;
        let option = Some(42);

        assert_eq!(cloned.length(&option), 1);
    }

    #[test]
    fn test_option_each_debug() {
        let traversal = OptionEach::<i32>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("OptionEach"));
    }

    #[test]
    fn test_option_each_default() {
        let traversal = OptionEach::<i32>::default();
        let option = Some(42);

        assert_eq!(traversal.length(&option), 1);
    }

    // =========================================================================
    // ResultEach Tests
    // =========================================================================

    #[test]
    fn test_result_each_get_all_ok() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&result).collect();
        assert_eq!(elements, vec![&42]);
    }

    #[test]
    fn test_result_each_get_all_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let traversal = <Result<i32, String> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&result).collect();
        assert!(elements.is_empty());
    }

    #[test]
    fn test_result_each_get_all_owned_ok() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let elements = traversal.get_all_owned(result);
        assert_eq!(elements, vec![42]);
    }

    #[test]
    fn test_result_each_get_all_owned_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let traversal = <Result<i32, String> as Each>::each();

        let elements = traversal.get_all_owned(result);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_result_each_modify_all_ok() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let doubled = traversal.modify_all(result, |x| x * 2);
        assert_eq!(doubled, Ok(84));
    }

    #[test]
    fn test_result_each_modify_all_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let traversal = <Result<i32, String> as Each>::each();

        let modified = traversal.modify_all(result, |x| x * 2);
        assert_eq!(modified, Err("error".to_string()));
    }

    #[test]
    fn test_result_each_fold_ok() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let sum = traversal.fold(&result, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 42);
    }

    #[test]
    fn test_result_each_fold_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let traversal = <Result<i32, String> as Each>::each();

        let sum = traversal.fold(&result, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_result_each_clone() {
        let traversal = ResultEach::<i32, String>::new();
        let cloned = traversal;
        let result: Result<i32, String> = Ok(42);

        assert_eq!(cloned.length(&result), 1);
    }

    #[test]
    fn test_result_each_debug() {
        let traversal = ResultEach::<i32, String>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("ResultEach"));
    }

    #[test]
    fn test_result_each_default() {
        let traversal = ResultEach::<i32, String>::default();
        let result: Result<i32, String> = Ok(42);

        assert_eq!(traversal.length(&result), 1);
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_each_convenience_function() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = each::<Vec<i32>>();

        let doubled = traversal.modify_all(vec, |x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
    }

    // =========================================================================
    // Traversal Law Tests
    // =========================================================================

    #[test]
    fn test_vec_each_identity_law() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let result = traversal.modify_all(vec.clone(), |x| x);
        assert_eq!(result, vec);
    }

    #[test]
    fn test_vec_each_composition_law() {
        let vec = vec![1, 2, 3, 4, 5];
        let traversal = <Vec<i32> as Each>::each();

        let function_f = |x: i32| x + 1;
        let function_g = |x: i32| x * 2;

        let sequential =
            traversal.modify_all(traversal.modify_all(vec.clone(), function_f), function_g);
        let composed = traversal.modify_all(vec, |x| function_g(function_f(x)));

        assert_eq!(sequential, composed);
    }

    #[test]
    fn test_option_each_identity_law() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let result = traversal.modify_all(option, |x| x);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_option_each_composition_law() {
        let option = Some(42);
        let traversal = <Option<i32> as Each>::each();

        let function_f = |x: i32| x + 1;
        let function_g = |x: i32| x * 2;

        let sequential = traversal.modify_all(traversal.modify_all(option, function_f), function_g);
        let composed = traversal.modify_all(Some(42), |x| function_g(function_f(x)));

        assert_eq!(sequential, composed);
    }

    #[test]
    fn test_result_each_identity_law() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let modified = traversal.modify_all(result, |x| x);
        assert_eq!(modified, Ok(42));
    }

    #[test]
    fn test_result_each_composition_law() {
        let result: Result<i32, String> = Ok(42);
        let traversal = <Result<i32, String> as Each>::each();

        let function_f = |x: i32| x + 1;
        let function_g = |x: i32| x * 2;

        let sequential = traversal.modify_all(traversal.modify_all(result, function_f), function_g);
        let composed = traversal.modify_all(Ok(42), |x| function_g(function_f(x)));

        assert_eq!(sequential, composed);
    }
}

#[cfg(all(test, feature = "persistent"))]
mod persistent_tests {
    use super::{Each, Traversal};
    use crate::optics::each::{PersistentHashMapEach, PersistentTreeMapEach, PersistentVectorEach};
    use crate::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};

    // =========================================================================
    // PersistentVectorEach Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_each_get_all() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = <PersistentVector<i32> as Each>::each();

        let elements: Vec<&i32> = traversal.get_all(&vector).collect();
        assert_eq!(elements, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn test_persistent_vector_each_get_all_owned() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = <PersistentVector<i32> as Each>::each();

        let elements = traversal.get_all_owned(vector);
        assert_eq!(elements, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_persistent_vector_each_modify_all() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = <PersistentVector<i32> as Each>::each();

        let doubled = traversal.modify_all(vector, |x| x * 2);
        let elements: Vec<i32> = doubled.into_iter().collect();
        assert_eq!(elements, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_persistent_vector_each_clone() {
        let traversal = PersistentVectorEach::<i32>::new();
        let cloned = traversal;
        let vector: PersistentVector<i32> = (1..=3).collect();

        assert_eq!(cloned.length(&vector), 3);
    }

    #[test]
    fn test_persistent_vector_each_debug() {
        let traversal = PersistentVectorEach::<i32>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("PersistentVectorEach"));
    }

    #[test]
    fn test_persistent_vector_each_default() {
        let traversal = PersistentVectorEach::<i32>::default();
        let vector: PersistentVector<i32> = (1..=3).collect();

        assert_eq!(traversal.length(&vector), 3);
    }

    // =========================================================================
    // PersistentHashMapEach Tests
    // =========================================================================

    #[test]
    fn test_persistent_hashmap_each_get_all() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let traversal = <PersistentHashMap<String, i32> as Each>::each();

        let sum: i32 = traversal.fold(&map, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_persistent_hashmap_each_get_all_owned() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let traversal = <PersistentHashMap<String, i32> as Each>::each();

        let elements = traversal.get_all_owned(map);
        assert_eq!(elements.len(), 2);
        assert!(elements.contains(&1));
        assert!(elements.contains(&2));
    }

    #[test]
    fn test_persistent_hashmap_each_modify_all() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let traversal = <PersistentHashMap<String, i32> as Each>::each();

        let doubled = traversal.modify_all(map, |x| x * 2);
        assert_eq!(doubled.get("a"), Some(&2));
        assert_eq!(doubled.get("b"), Some(&4));
    }

    #[test]
    fn test_persistent_hashmap_each_clone() {
        let traversal = PersistentHashMapEach::<String, i32>::new();
        let cloned = traversal;
        let map = PersistentHashMap::new().insert("a".to_string(), 1);

        assert_eq!(cloned.length(&map), 1);
    }

    #[test]
    fn test_persistent_hashmap_each_debug() {
        let traversal = PersistentHashMapEach::<String, i32>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("PersistentHashMapEach"));
    }

    #[test]
    fn test_persistent_hashmap_each_default() {
        let traversal = PersistentHashMapEach::<String, i32>::default();
        let map = PersistentHashMap::new().insert("a".to_string(), 1);

        assert_eq!(traversal.length(&map), 1);
    }

    // =========================================================================
    // PersistentTreeMapEach Tests
    // =========================================================================

    #[test]
    fn test_persistent_treemap_each_get_all() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string())
            .insert(3, "three".to_string());
        let traversal = <PersistentTreeMap<i32, String> as Each>::each();

        let values: Vec<&String> = traversal.get_all(&map).collect();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_persistent_treemap_each_get_all_owned() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let traversal = <PersistentTreeMap<i32, String> as Each>::each();

        let elements = traversal.get_all_owned(map);
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_persistent_treemap_each_modify_all() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let traversal = <PersistentTreeMap<i32, String> as Each>::each();

        let uppercased = traversal.modify_all(map, |s| s.to_uppercase());
        assert_eq!(uppercased.get(&1), Some(&"ONE".to_string()));
        assert_eq!(uppercased.get(&2), Some(&"TWO".to_string()));
    }

    #[test]
    fn test_persistent_treemap_each_clone() {
        let traversal = PersistentTreeMapEach::<i32, String>::new();
        let cloned = traversal;
        let map = PersistentTreeMap::new().insert(1, "one".to_string());

        assert_eq!(cloned.length(&map), 1);
    }

    #[test]
    fn test_persistent_treemap_each_debug() {
        let traversal = PersistentTreeMapEach::<i32, String>::new();
        let debug_string = format!("{traversal:?}");
        assert!(debug_string.contains("PersistentTreeMapEach"));
    }

    #[test]
    fn test_persistent_treemap_each_default() {
        let traversal = PersistentTreeMapEach::<i32, String>::default();
        let map = PersistentTreeMap::new().insert(1, "one".to_string());

        assert_eq!(traversal.length(&map), 1);
    }
}
