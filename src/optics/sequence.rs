//! Sequence combinator for head/last element access.
//!
//! This module provides the [`Sequence`] trait which enables Optional-like access
//! to the first and last elements of sequence-like containers.
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Optional, sequence::Sequence};
//!
//! let vec = vec![1, 2, 3, 4, 5];
//! let head = <Vec<i32> as Sequence>::head_optional();
//! let last = <Vec<i32> as Sequence>::last_optional();
//!
//! assert_eq!(head.get_option(&vec), Some(&1));
//! assert_eq!(last.get_option(&vec), Some(&5));
//! ```
//!
//! # Note
//!
//! The name "Sequence" is used instead of "Cons" because in Haskell,
//! Cons refers to cons cells (list constructors), while this trait
//! is about accessing the head and last elements of any sequence.

use std::marker::PhantomData;

use crate::optics::Optional;

/// A trait for sequence-like containers that have a head and last element.
///
/// This trait provides Optional-based access to the first and last elements
/// of a sequence.
pub trait Sequence: Sized {
    /// The element type of this sequence.
    type Element;

    /// The Optional type for accessing the first element.
    type HeadOptional: Optional<Self, Self::Element>;

    /// The Optional type for accessing the last element.
    type LastOptional: Optional<Self, Self::Element>;

    /// Returns an Optional focusing on the first element.
    fn head_optional() -> Self::HeadOptional;

    /// Returns an Optional focusing on the last element.
    fn last_optional() -> Self::LastOptional;
}

/// An Optional focusing on the first element of a `Vec<T>`.
#[derive(Debug, Clone)]
pub struct VecHeadOptional<T> {
    _marker: PhantomData<T>,
}

impl<T> VecHeadOptional<T> {
    /// Creates a new `VecHeadOptional`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for VecHeadOptional<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Optional<Vec<T>, T> for VecHeadOptional<T> {
    fn get_option<'a>(&self, source: &'a Vec<T>) -> Option<&'a T> {
        source.first()
    }

    fn set(&self, mut source: Vec<T>, value: T) -> Vec<T> {
        if !source.is_empty() {
            source[0] = value;
        }
        source
    }
}

/// An Optional focusing on the last element of a `Vec<T>`.
#[derive(Debug, Clone)]
pub struct VecLastOptional<T> {
    _marker: PhantomData<T>,
}

impl<T> VecLastOptional<T> {
    /// Creates a new `VecLastOptional`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for VecLastOptional<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Optional<Vec<T>, T> for VecLastOptional<T> {
    fn get_option<'a>(&self, source: &'a Vec<T>) -> Option<&'a T> {
        source.last()
    }

    fn set(&self, mut source: Vec<T>, value: T) -> Vec<T> {
        if let Some(last) = source.last_mut() {
            *last = value;
        }
        source
    }
}

impl<T: Clone> Sequence for Vec<T> {
    type Element = T;
    type HeadOptional = VecHeadOptional<T>;
    type LastOptional = VecLastOptional<T>;

    fn head_optional() -> Self::HeadOptional {
        VecHeadOptional::new()
    }

    fn last_optional() -> Self::LastOptional {
        VecLastOptional::new()
    }
}

// =============================================================================
// Persistent Data Structure Implementations
// =============================================================================

#[cfg(feature = "persistent")]
mod persistent_implementations {
    use super::{Optional, PhantomData, Sequence};
    use crate::persistent::PersistentVector;

    /// An Optional focusing on the first element of a `PersistentVector<T>`.
    #[derive(Debug, Clone)]
    pub struct PersistentVectorHeadOptional<T> {
        _marker: PhantomData<T>,
    }

    impl<T> PersistentVectorHeadOptional<T> {
        /// Creates a new `PersistentVectorHeadOptional`.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<T> Default for PersistentVectorHeadOptional<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Clone> Optional<PersistentVector<T>, T> for PersistentVectorHeadOptional<T> {
        fn get_option<'a>(&self, source: &'a PersistentVector<T>) -> Option<&'a T> {
            source.get(0)
        }

        fn set(&self, source: PersistentVector<T>, value: T) -> PersistentVector<T> {
            source.update(0, value).unwrap_or(source)
        }
    }

    /// An Optional focusing on the last element of a `PersistentVector<T>`.
    #[derive(Debug, Clone)]
    pub struct PersistentVectorLastOptional<T> {
        _marker: PhantomData<T>,
    }

    impl<T> PersistentVectorLastOptional<T> {
        /// Creates a new `PersistentVectorLastOptional`.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<T> Default for PersistentVectorLastOptional<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Clone> Optional<PersistentVector<T>, T> for PersistentVectorLastOptional<T> {
        fn get_option<'a>(&self, source: &'a PersistentVector<T>) -> Option<&'a T> {
            if source.is_empty() {
                None
            } else {
                source.get(source.len() - 1)
            }
        }

        fn set(&self, source: PersistentVector<T>, value: T) -> PersistentVector<T> {
            if source.is_empty() {
                source
            } else {
                source.update(source.len() - 1, value).unwrap_or(source)
            }
        }
    }

    impl<T: Clone> Sequence for PersistentVector<T> {
        type Element = T;
        type HeadOptional = PersistentVectorHeadOptional<T>;
        type LastOptional = PersistentVectorLastOptional<T>;

        fn head_optional() -> Self::HeadOptional {
            PersistentVectorHeadOptional::new()
        }

        fn last_optional() -> Self::LastOptional {
            PersistentVectorLastOptional::new()
        }
    }
}

#[cfg(feature = "persistent")]
pub use persistent_implementations::*;

/// Creates an Optional focusing on the first element of a sequence.
#[must_use]
pub fn head_option<S: Sequence>() -> S::HeadOptional {
    S::head_optional()
}

/// Creates an Optional focusing on the last element of a sequence.
#[must_use]
pub fn last_option<S: Sequence>() -> S::LastOptional {
    S::last_optional()
}

#[cfg(test)]
mod tests {
    use super::{Optional, Sequence, VecHeadOptional, VecLastOptional, head_option, last_option};

    // =========================================================================
    // VecHeadOptional Tests
    // =========================================================================

    #[test]
    fn test_vec_head_get_non_empty() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vec), Some(&1));
    }

    #[test]
    fn test_vec_head_get_empty() {
        let vec: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vec), None);
    }

    #[test]
    fn test_vec_head_set_non_empty() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        let updated = optional.set(vec, 100);
        assert_eq!(updated, vec![100, 2, 3, 4, 5]);
    }

    #[test]
    fn test_vec_head_set_empty() {
        let vec: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::head_optional();

        let updated = optional.set(vec, 100);
        assert!(updated.is_empty());
    }

    #[test]
    fn test_vec_head_modify() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        let modified = optional.modify(vec, |x| x * 10);
        assert_eq!(modified, vec![10, 2, 3, 4, 5]);
    }

    #[test]
    fn test_vec_head_is_present() {
        let non_empty = vec![1, 2, 3];
        let empty: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::head_optional();

        assert!(optional.is_present(&non_empty));
        assert!(!optional.is_present(&empty));
    }

    #[test]
    fn test_vec_head_clone() {
        let optional = VecHeadOptional::<i32>::new();
        let cloned = optional;
        let vec = vec![1, 2, 3];

        assert_eq!(cloned.get_option(&vec), Some(&1));
    }

    #[test]
    fn test_vec_head_debug() {
        let optional = VecHeadOptional::<i32>::new();
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("VecHeadOptional"));
    }

    #[test]
    fn test_vec_head_default() {
        let optional = VecHeadOptional::<i32>::default();
        let vec = vec![1, 2, 3];

        assert_eq!(optional.get_option(&vec), Some(&1));
    }

    // =========================================================================
    // VecLastOptional Tests
    // =========================================================================

    #[test]
    fn test_vec_last_get_non_empty() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vec), Some(&5));
    }

    #[test]
    fn test_vec_last_get_empty() {
        let vec: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vec), None);
    }

    #[test]
    fn test_vec_last_set_non_empty() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        let updated = optional.set(vec, 100);
        assert_eq!(updated, vec![1, 2, 3, 4, 100]);
    }

    #[test]
    fn test_vec_last_set_empty() {
        let vec: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::last_optional();

        let updated = optional.set(vec, 100);
        assert!(updated.is_empty());
    }

    #[test]
    fn test_vec_last_modify() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        let modified = optional.modify(vec, |x| x * 10);
        assert_eq!(modified, vec![1, 2, 3, 4, 50]);
    }

    #[test]
    fn test_vec_last_is_present() {
        let non_empty = vec![1, 2, 3];
        let empty: Vec<i32> = vec![];
        let optional = <Vec<i32> as Sequence>::last_optional();

        assert!(optional.is_present(&non_empty));
        assert!(!optional.is_present(&empty));
    }

    #[test]
    fn test_vec_last_clone() {
        let optional = VecLastOptional::<i32>::new();
        let cloned = optional;
        let vec = vec![1, 2, 3];

        assert_eq!(cloned.get_option(&vec), Some(&3));
    }

    #[test]
    fn test_vec_last_debug() {
        let optional = VecLastOptional::<i32>::new();
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("VecLastOptional"));
    }

    #[test]
    fn test_vec_last_default() {
        let optional = VecLastOptional::<i32>::default();
        let vec = vec![1, 2, 3];

        assert_eq!(optional.get_option(&vec), Some(&3));
    }

    // =========================================================================
    // Single Element Tests
    // =========================================================================

    #[test]
    fn test_vec_head_single_element() {
        let vec = vec![42];
        let optional = <Vec<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vec), Some(&42));
    }

    #[test]
    fn test_vec_last_single_element() {
        let vec = vec![42];
        let optional = <Vec<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vec), Some(&42));
    }

    #[test]
    fn test_vec_head_and_last_same_for_single_element() {
        let vec = vec![42];
        let head = <Vec<i32> as Sequence>::head_optional();
        let last = <Vec<i32> as Sequence>::last_optional();

        assert_eq!(head.get_option(&vec), last.get_option(&vec));
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_head_option_convenience_function() {
        let vec = vec![1, 2, 3];
        let optional = head_option::<Vec<i32>>();

        assert_eq!(optional.get_option(&vec), Some(&1));
    }

    #[test]
    fn test_last_option_convenience_function() {
        let vec = vec![1, 2, 3];
        let optional = last_option::<Vec<i32>>();

        assert_eq!(optional.get_option(&vec), Some(&3));
    }

    // =========================================================================
    // Optional Law Tests
    // =========================================================================

    #[test]
    fn test_vec_head_get_set_law() {
        // Law: optional.set(source, optional.get_option(&source).unwrap().clone()) == source
        // (when element is present)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        if let Some(value) = optional.get_option(&vec) {
            let reconstructed = optional.set(vec.clone(), *value);
            assert_eq!(reconstructed, vec);
        }
    }

    #[test]
    fn test_vec_head_set_get_law() {
        // Law: optional.get_option(&optional.set(source, value)) == Some(&value)
        // (when element is present)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        let updated = optional.set(vec, 100);
        assert_eq!(optional.get_option(&updated), Some(&100));
    }

    #[test]
    fn test_vec_head_set_set_law() {
        // Law: optional.set(optional.set(source, v1), v2) == optional.set(source, v2)
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::head_optional();

        let set_twice = optional.set(optional.set(vec.clone(), 42), 100);
        let set_once = optional.set(vec, 100);

        assert_eq!(set_twice, set_once);
    }

    #[test]
    fn test_vec_last_get_set_law() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        if let Some(value) = optional.get_option(&vec) {
            let reconstructed = optional.set(vec.clone(), *value);
            assert_eq!(reconstructed, vec);
        }
    }

    #[test]
    fn test_vec_last_set_get_law() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        let updated = optional.set(vec, 100);
        assert_eq!(optional.get_option(&updated), Some(&100));
    }

    #[test]
    fn test_vec_last_set_set_law() {
        let vec = vec![1, 2, 3, 4, 5];
        let optional = <Vec<i32> as Sequence>::last_optional();

        let set_twice = optional.set(optional.set(vec.clone(), 42), 100);
        let set_once = optional.set(vec, 100);

        assert_eq!(set_twice, set_once);
    }
}

#[cfg(all(test, feature = "persistent"))]
mod persistent_tests {
    use super::{Optional, Sequence, head_option, last_option};
    use crate::optics::sequence::{PersistentVectorHeadOptional, PersistentVectorLastOptional};
    use crate::persistent::PersistentVector;

    // =========================================================================
    // PersistentVectorHeadOptional Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_head_get_non_empty() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vector), Some(&1));
    }

    #[test]
    fn test_persistent_vector_head_get_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vector), None);
    }

    #[test]
    fn test_persistent_vector_head_set_non_empty() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        let updated = optional.set(vector, 100);
        assert_eq!(updated.get(0), Some(&100));
        assert_eq!(updated.get(1), Some(&2));
    }

    #[test]
    fn test_persistent_vector_head_set_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        let updated = optional.set(vector, 100);
        assert!(updated.is_empty());
    }

    #[test]
    fn test_persistent_vector_head_modify() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        let modified = optional.modify(vector, |x| x * 10);
        assert_eq!(modified.get(0), Some(&10));
    }

    #[test]
    fn test_persistent_vector_head_clone() {
        let optional = PersistentVectorHeadOptional::<i32>::new();
        let cloned = optional;
        let vector: PersistentVector<i32> = (1..=5).collect();

        assert_eq!(cloned.get_option(&vector), Some(&1));
    }

    #[test]
    fn test_persistent_vector_head_debug() {
        let optional = PersistentVectorHeadOptional::<i32>::new();
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentVectorHeadOptional"));
    }

    #[test]
    fn test_persistent_vector_head_default() {
        let optional = PersistentVectorHeadOptional::<i32>::default();
        let vector: PersistentVector<i32> = (1..=5).collect();

        assert_eq!(optional.get_option(&vector), Some(&1));
    }

    // =========================================================================
    // PersistentVectorLastOptional Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_last_get_non_empty() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vector), Some(&5));
    }

    #[test]
    fn test_persistent_vector_last_get_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vector), None);
    }

    #[test]
    fn test_persistent_vector_last_set_non_empty() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        let updated = optional.set(vector, 100);
        assert_eq!(updated.get(4), Some(&100));
        assert_eq!(updated.get(3), Some(&4));
    }

    #[test]
    fn test_persistent_vector_last_set_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        let updated = optional.set(vector, 100);
        assert!(updated.is_empty());
    }

    #[test]
    fn test_persistent_vector_last_modify() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        let modified = optional.modify(vector, |x| x * 10);
        assert_eq!(modified.get(4), Some(&50));
    }

    #[test]
    fn test_persistent_vector_last_clone() {
        let optional = PersistentVectorLastOptional::<i32>::new();
        let cloned = optional;
        let vector: PersistentVector<i32> = (1..=5).collect();

        assert_eq!(cloned.get_option(&vector), Some(&5));
    }

    #[test]
    fn test_persistent_vector_last_debug() {
        let optional = PersistentVectorLastOptional::<i32>::new();
        let debug_string = format!("{optional:?}");
        assert!(debug_string.contains("PersistentVectorLastOptional"));
    }

    #[test]
    fn test_persistent_vector_last_default() {
        let optional = PersistentVectorLastOptional::<i32>::default();
        let vector: PersistentVector<i32> = (1..=5).collect();

        assert_eq!(optional.get_option(&vector), Some(&5));
    }

    // =========================================================================
    // Single Element Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_head_single_element() {
        let vector: PersistentVector<i32> = std::iter::once(42).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        assert_eq!(optional.get_option(&vector), Some(&42));
    }

    #[test]
    fn test_persistent_vector_last_single_element() {
        let vector: PersistentVector<i32> = std::iter::once(42).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        assert_eq!(optional.get_option(&vector), Some(&42));
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_head_option_persistent_vector() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = head_option::<PersistentVector<i32>>();

        assert_eq!(optional.get_option(&vector), Some(&1));
    }

    #[test]
    fn test_last_option_persistent_vector() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = last_option::<PersistentVector<i32>>();

        assert_eq!(optional.get_option(&vector), Some(&5));
    }

    // =========================================================================
    // Optional Law Tests
    // =========================================================================

    #[test]
    fn test_persistent_vector_head_get_set_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        if let Some(value) = optional.get_option(&vector) {
            let reconstructed = optional.set(vector.clone(), *value);
            assert_eq!(reconstructed.get(0), vector.get(0));
        }
    }

    #[test]
    fn test_persistent_vector_head_set_get_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        let updated = optional.set(vector, 100);
        assert_eq!(optional.get_option(&updated), Some(&100));
    }

    #[test]
    fn test_persistent_vector_head_set_set_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::head_optional();

        let set_twice = optional.set(optional.set(vector.clone(), 42), 100);
        let set_once = optional.set(vector, 100);

        assert_eq!(set_twice.get(0), set_once.get(0));
    }

    #[test]
    fn test_persistent_vector_last_get_set_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        if let Some(value) = optional.get_option(&vector) {
            let reconstructed = optional.set(vector.clone(), *value);
            assert_eq!(reconstructed.get(4), vector.get(4));
        }
    }

    #[test]
    fn test_persistent_vector_last_set_get_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        let updated = optional.set(vector, 100);
        assert_eq!(optional.get_option(&updated), Some(&100));
    }

    #[test]
    fn test_persistent_vector_last_set_set_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = <PersistentVector<i32> as Sequence>::last_optional();

        let set_twice = optional.set(optional.set(vector.clone(), 42), 100);
        let set_once = optional.set(vector, 100);

        assert_eq!(set_twice.get(4), set_once.get(4));
    }
}
