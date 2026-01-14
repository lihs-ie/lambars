#![cfg(feature = "control")]
//! Property-based tests for ConcurrentLazy<T, F> laws.

use lambars::control::ConcurrentLazy;
use proptest::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// =============================================================================
// Idempotence Law
// =============================================================================

proptest! {
    /// Idempotence: calling force() multiple times returns the same value
    #[test]
    fn prop_concurrent_lazy_idempotence(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);

        let first = *lazy.force();
        let second = *lazy.force();
        let third = *lazy.force();

        prop_assert_eq!(first, second);
        prop_assert_eq!(second, third);
    }
}

proptest! {
    /// Idempotence for string values
    #[test]
    fn prop_concurrent_lazy_idempotence_string(value in any::<String>()) {
        let lazy = ConcurrentLazy::new(move || value.clone());

        let first = lazy.force().clone();
        let second = lazy.force().clone();

        prop_assert_eq!(first, second);
    }
}

// =============================================================================
// Memoization Law
// =============================================================================

proptest! {
    /// Memoization: the initializer function is called at most once
    #[test]
    fn prop_concurrent_lazy_memoization(value in any::<i32>()) {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            value
        });

        prop_assert_eq!(counter.load(Ordering::SeqCst), 0);
        let _ = lazy.force();
        prop_assert_eq!(counter.load(Ordering::SeqCst), 1);
        let _ = lazy.force();
        let _ = lazy.force();
        let _ = lazy.force();
        prop_assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

// =============================================================================
// Thread Safety Laws
// =============================================================================

proptest! {
    /// Thread safety: concurrent force() calls return the same value
    #[test]
    fn prop_concurrent_lazy_thread_safe_force(value in any::<i32>()) {
        let lazy = Arc::new(ConcurrentLazy::new(move || value));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || *lazy.force())
            })
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        prop_assert!(results.iter().all(|&x| x == value));
    }
}

proptest! {
    /// Thread safety: initialization happens exactly once
    #[test]
    fn prop_concurrent_lazy_initialization_once(value in any::<i32>()) {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let lazy = Arc::new(ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            value
        }));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || *lazy.force())
            })
            .collect();

        for handle in handles {
            let _ = handle.join().unwrap();
        }

        prop_assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: lazy.map(|x| x) == lazy
    #[test]
    fn prop_concurrent_lazy_functor_identity(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);
        let mapped = ConcurrentLazy::new(move || value).map(|x| x);

        // We need to compare the forced values since ConcurrentLazy doesn't implement Eq
        prop_assert_eq!(*lazy.force(), *mapped.force());
    }
}

proptest! {
    /// Functor Composition Law: lazy.map(f).map(g) == lazy.map(|x| g(f(x)))
    #[test]
    fn prop_concurrent_lazy_functor_composition(value in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let lazy1 = ConcurrentLazy::new(move || value);
        let lazy2 = ConcurrentLazy::new(move || value);

        let left = lazy1.map(function1).map(function2);
        let right = lazy2.map(|x| function2(function1(x)));

        prop_assert_eq!(*left.force(), *right.force());
    }
}

proptest! {
    /// Functor Composition Law with type change
    #[test]
    fn prop_concurrent_lazy_functor_composition_type_change(value in any::<i32>()) {
        let function1 = |n: i32| n.to_string();
        let function2 = |s: String| s.len();

        let lazy1 = ConcurrentLazy::new(move || value);
        let lazy2 = ConcurrentLazy::new(move || value);

        let left = lazy1.map(function1).map(function2);
        let right = lazy2.map(|x| function2(function1(x)));

        prop_assert_eq!(*left.force(), *right.force());
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity: ConcurrentLazy::pure(a).flat_map(f) == f(a)
    #[test]
    fn prop_concurrent_lazy_monad_left_identity(value in any::<i32>()) {
        let function = |x: i32| ConcurrentLazy::new(move || x.wrapping_mul(2));

        let left = ConcurrentLazy::pure(value).flat_map(function);
        let right = function(value);

        prop_assert_eq!(*left.force(), *right.force());
    }
}

proptest! {
    /// Monad Right Identity: lazy.flat_map(ConcurrentLazy::pure) == lazy
    #[test]
    fn prop_concurrent_lazy_monad_right_identity(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);
        let flat_mapped = ConcurrentLazy::new(move || value).flat_map(ConcurrentLazy::new_with_value);

        prop_assert_eq!(*lazy.force(), *flat_mapped.force());
    }
}

proptest! {
    /// Monad Associativity:
    /// lazy.flat_map(f).flat_map(g) == lazy.flat_map(|x| f(x).flat_map(g))
    #[test]
    fn prop_concurrent_lazy_monad_associativity(value in any::<i32>()) {
        let function1 = |x: i32| ConcurrentLazy::new(move || x.wrapping_add(1));
        let function2 = |x: i32| ConcurrentLazy::new(move || x.wrapping_mul(2));

        let lazy1 = ConcurrentLazy::new(move || value);
        let lazy2 = ConcurrentLazy::new(move || value);

        let left = lazy1.flat_map(function1).flat_map(function2);
        let right = lazy2.flat_map(|x| function1(x).flat_map(function2));

        prop_assert_eq!(*left.force(), *right.force());
    }
}

// =============================================================================
// zip Laws
// =============================================================================

proptest! {
    /// zip produces a tuple of both values
    #[test]
    fn prop_concurrent_lazy_zip_produces_tuple(value1 in any::<i32>(), value2 in any::<i32>()) {
        let lazy1 = ConcurrentLazy::new(move || value1);
        let lazy2 = ConcurrentLazy::new(move || value2);
        let zipped = lazy1.zip(lazy2);

        prop_assert_eq!(*zipped.force(), (value1, value2));
    }
}

proptest! {
    /// zip_with applies function to both values
    #[test]
    fn prop_concurrent_lazy_zip_with_applies_function(
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let lazy1 = ConcurrentLazy::new(move || value1);
        let lazy2 = ConcurrentLazy::new(move || value2);
        let combined = lazy1.zip_with(lazy2, |a, b| a.wrapping_add(b));

        prop_assert_eq!(*combined.force(), value1.wrapping_add(value2));
    }
}

// =============================================================================
// new_with_value / pure equivalence
// =============================================================================

proptest! {
    /// new_with_value and pure produce equivalent results
    #[test]
    fn prop_concurrent_lazy_new_with_value_pure_equivalence(value in any::<i32>()) {
        let lazy1 = ConcurrentLazy::new_with_value(value);
        let lazy2 = ConcurrentLazy::pure(value);

        prop_assert_eq!(*lazy1.force(), *lazy2.force());
    }
}

// =============================================================================
// State Transitions
// =============================================================================

proptest! {
    /// State transition: is_initialized is false before force, true after
    #[test]
    fn prop_concurrent_lazy_state_transitions(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);

        prop_assert!(!lazy.is_initialized());
        let _ = lazy.force();
        prop_assert!(lazy.is_initialized());
    }
}

proptest! {
    /// State transition: get returns None before force, Some after
    #[test]
    fn prop_concurrent_lazy_get_state(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);

        prop_assert!(lazy.get().is_none());
        let _ = lazy.force();
        prop_assert!(lazy.get().is_some());
        prop_assert_eq!(*lazy.get().unwrap(), value);
    }
}

// =============================================================================
// Consistency across operations
// =============================================================================

proptest! {
    /// force and get return the same value after initialization
    #[test]
    fn prop_concurrent_lazy_force_get_consistency(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);

        let forced = *lazy.force();
        let gotten = *lazy.get().unwrap();

        prop_assert_eq!(forced, gotten);
    }
}

proptest! {
    /// map preserves value transformation
    #[test]
    fn prop_concurrent_lazy_map_preserves_transformation(value in any::<i32>()) {
        let lazy = ConcurrentLazy::new(move || value);
        let function = |x: i32| x.wrapping_mul(3);

        let mapped = lazy.map(function);

        prop_assert_eq!(*mapped.force(), function(value));
    }
}

proptest! {
    /// flat_map with pure is equivalent to map
    #[test]
    fn prop_concurrent_lazy_flat_map_pure_is_map(value in any::<i32>()) {
        let function = |x: i32| x.wrapping_mul(2);

        let lazy1 = ConcurrentLazy::new(move || value);
        let lazy2 = ConcurrentLazy::new(move || value);

        let mapped = lazy1.map(function);
        let flat_mapped = lazy2.flat_map(|x| ConcurrentLazy::new_with_value(function(x)));

        prop_assert_eq!(*mapped.force(), *flat_mapped.force());
    }
}

// =============================================================================
// into_inner Laws
// =============================================================================

proptest! {
    /// into_inner returns the same value as force
    #[test]
    fn prop_concurrent_lazy_into_inner_equals_force(value in any::<i32>()) {
        let lazy1 = ConcurrentLazy::new(move || value);
        let lazy2 = ConcurrentLazy::new(move || value);

        let forced = *lazy1.force();
        let into_inner = lazy2.into_inner().unwrap();

        prop_assert_eq!(forced, into_inner);
    }
}
