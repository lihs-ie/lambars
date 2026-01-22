#![cfg(feature = "control")]
//! Unit tests for Lazy<T, F> type.
//!
//! Tests cover:
//! - Basic lazy evaluation and memoization
//! - Initialization state transitions
//! - Poisoned state handling
//! - map and flat_map operations
//! - zip and zip_with operations

use lambars::control::{Lazy, LazyPoisonedError};
use rstest::rstest;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

// =============================================================================
// Basic Construction and Evaluation
// =============================================================================

#[rstest]
fn lazy_defers_computation() {
    let computed = Cell::new(false);
    let _lazy = Lazy::new(|| {
        computed.set(true);
        42
    });

    // At this point, the computation should NOT have run
    assert!(!computed.get());
}

#[rstest]
fn lazy_force_computes_value() {
    let computed = Cell::new(false);
    let lazy = Lazy::new(|| {
        computed.set(true);
        42
    });

    assert!(!computed.get());

    let value = lazy.force();
    assert!(computed.get());
    assert_eq!(*value, 42);
}

#[rstest]
fn lazy_force_returns_ref() {
    let lazy = Lazy::new(|| "hello".to_string());
    let value = lazy.force();

    // We can call methods on the Ref
    assert_eq!(value.len(), 5);
    assert!(value.starts_with("hel"));
}

// =============================================================================
// Memoization
// =============================================================================

#[rstest]
fn lazy_memoization_single_computation() {
    let call_count = Cell::new(0);
    let lazy = Lazy::new(|| {
        call_count.set(call_count.get() + 1);
        42
    });

    assert_eq!(call_count.get(), 0);

    // First force
    let _ = lazy.force();
    assert_eq!(call_count.get(), 1);

    // Second force - should NOT call again
    let _ = lazy.force();
    assert_eq!(call_count.get(), 1);

    // Third force - still only 1
    let _ = lazy.force();
    assert_eq!(call_count.get(), 1);
}

#[rstest]
fn lazy_memoization_preserves_value() {
    let lazy = Lazy::new(|| "computed_value".to_string());

    let first = lazy.force();
    let second = lazy.force();

    assert_eq!(*first, "computed_value");
    assert_eq!(*second, "computed_value");
}

// =============================================================================
// new_with_value
// =============================================================================

#[rstest]
fn lazy_new_with_value_is_initialized() {
    let lazy = Lazy::new_with_value(42);
    assert!(lazy.is_initialized());
}

#[rstest]
fn lazy_new_with_value_force_returns_value() {
    let lazy = Lazy::new_with_value(42);
    assert_eq!(*lazy.force(), 42);
}

#[rstest]
fn lazy_pure_is_alias_for_new_with_value() {
    let lazy = Lazy::pure("hello");
    assert!(lazy.is_initialized());
    assert_eq!(*lazy.force(), "hello");
}

// =============================================================================
// get Method
// =============================================================================

#[rstest]
fn lazy_get_before_force_returns_none() {
    let lazy = Lazy::new(|| 42);
    assert!(lazy.get().is_none());
}

#[rstest]
fn lazy_get_after_force_returns_some() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    assert!(lazy.get().is_some());
    assert_eq!(*lazy.get().unwrap(), 42);
}

#[rstest]
fn lazy_get_on_new_with_value_returns_some() {
    let lazy = Lazy::new_with_value(42);
    assert!(lazy.get().is_some());
    assert_eq!(*lazy.get().unwrap(), 42);
}

// =============================================================================
// is_initialized
// =============================================================================

#[rstest]
fn lazy_is_initialized_false_initially() {
    let lazy = Lazy::new(|| 42);
    assert!(!lazy.is_initialized());
}

#[rstest]
fn lazy_is_initialized_true_after_force() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    assert!(lazy.is_initialized());
}

#[rstest]
fn lazy_is_initialized_true_for_new_with_value() {
    let lazy = Lazy::new_with_value(42);
    assert!(lazy.is_initialized());
}

// =============================================================================
// Poisoned State
// =============================================================================

#[rstest]
fn lazy_poisoned_after_panic() {
    let lazy = Lazy::new(|| panic!("initialization failed"));

    // Try to force, which should panic
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));
    assert!(result.is_err());

    // Now the lazy should be poisoned
    assert!(lazy.is_poisoned());
}

#[rstest]
#[should_panic(expected = "Lazy instance has been poisoned")]
fn lazy_force_on_poisoned_panics() {
    let lazy = Lazy::new(|| panic!("initialization failed"));

    // First force - causes panic and poisons
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));

    // Second force - should panic with "poisoned" message
    let _ = lazy.force();
}

#[rstest]
fn lazy_is_poisoned_false_initially() {
    let lazy = Lazy::new(|| 42);
    assert!(!lazy.is_poisoned());
}

#[rstest]
fn lazy_is_poisoned_false_after_successful_init() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    assert!(!lazy.is_poisoned());
}

// =============================================================================
// map
// =============================================================================

#[rstest]
fn lazy_map_transforms_value() {
    let lazy = Lazy::new(|| 21);
    let doubled = lazy.map(|x| x * 2);
    assert_eq!(*doubled.force(), 42);
}

#[rstest]
fn lazy_map_is_lazy() {
    let computed = Cell::new(false);
    let lazy = Lazy::new(|| {
        computed.set(true);
        21
    });
    let mapped = lazy.map(|x| x * 2);

    // Neither the original nor mapped should have computed yet
    assert!(!computed.get());

    // Force the mapped value
    let _ = mapped.force();
    assert!(computed.get());
}

#[rstest]
fn lazy_map_chain() {
    let lazy = Lazy::new(|| 10);
    let result = lazy.map(|x| x + 1).map(|x| x * 2).map(|x| x - 2);

    // (10 + 1) * 2 - 2 = 20
    assert_eq!(*result.force(), 20);
}

#[rstest]
fn lazy_map_type_change() {
    let lazy = Lazy::new(|| 42);
    let stringified = lazy.map(|x| x.to_string());
    assert_eq!(*stringified.force(), "42");
}

// =============================================================================
// flat_map
// =============================================================================

#[rstest]
fn lazy_flat_map_basic() {
    let lazy = Lazy::new(|| 21);
    let result = lazy.flat_map(|x| Lazy::new(move || x * 2));
    assert_eq!(*result.force(), 42);
}

#[rstest]
fn lazy_flat_map_is_lazy() {
    use std::rc::Rc;

    let outer_computed = Rc::new(Cell::new(false));
    let inner_computed = Rc::new(Cell::new(false));

    let outer_clone = outer_computed.clone();
    let lazy = Lazy::new(move || {
        outer_clone.set(true);
        21
    });

    let inner_clone = inner_computed.clone();
    let result = lazy.flat_map(move |x| {
        let inner_clone2 = inner_clone.clone();
        Lazy::new(move || {
            inner_clone2.set(true);
            x * 2
        })
    });

    // Nothing should be computed yet
    assert!(!outer_computed.get());
    assert!(!inner_computed.get());

    // Force the result
    let _ = result.force();
    assert!(outer_computed.get());
    assert!(inner_computed.get());
}

#[rstest]
fn lazy_flat_map_chain() {
    let lazy = Lazy::new(|| 10);
    let result = lazy
        .flat_map(|x| Lazy::new(move || x + 1))
        .flat_map(|x| Lazy::new(move || x * 2));

    // (10 + 1) * 2 = 22
    assert_eq!(*result.force(), 22);
}

#[rstest]
fn lazy_flat_map_with_already_initialized() {
    let lazy = Lazy::new(|| 21);
    let result = lazy.flat_map(|x| Lazy::new_with_value(x * 2));
    assert_eq!(*result.force(), 42);
}

// =============================================================================
// zip
// =============================================================================

#[rstest]
fn lazy_zip_combines_values() {
    let lazy1 = Lazy::new(|| 1);
    let lazy2 = Lazy::new(|| "hello");
    let combined = lazy1.zip(lazy2);

    assert_eq!(*combined.force(), (1, "hello"));
}

#[rstest]
fn lazy_zip_is_lazy() {
    let computed1 = Cell::new(false);
    let computed2 = Cell::new(false);

    let lazy1 = Lazy::new(|| {
        computed1.set(true);
        1
    });
    let lazy2 = Lazy::new(|| {
        computed2.set(true);
        2
    });

    let combined = lazy1.zip(lazy2);

    // Nothing computed yet
    assert!(!computed1.get());
    assert!(!computed2.get());

    // Force
    let _ = combined.force();
    assert!(computed1.get());
    assert!(computed2.get());
}

// =============================================================================
// zip_with
// =============================================================================

#[rstest]
fn lazy_zip_with_combines_with_function() {
    let lazy1 = Lazy::new(|| 20);
    let lazy2 = Lazy::new(|| 22);
    let sum = lazy1.zip_with(lazy2, |a, b| a + b);

    assert_eq!(*sum.force(), 42);
}

#[rstest]
fn lazy_zip_with_is_lazy() {
    let computed1 = Cell::new(false);
    let computed2 = Cell::new(false);

    let lazy1 = Lazy::new(|| {
        computed1.set(true);
        1
    });
    let lazy2 = Lazy::new(|| {
        computed2.set(true);
        2
    });

    let combined = lazy1.zip_with(lazy2, |a, b| a + b);

    assert!(!computed1.get());
    assert!(!computed2.get());

    let _ = combined.force();
    assert!(computed1.get());
    assert!(computed2.get());
}

#[rstest]
fn lazy_zip_with_type_change() {
    let lazy1 = Lazy::new(|| 42);
    let lazy2 = Lazy::new(|| "answer");
    let combined = lazy1.zip_with(lazy2, |n, s| format!("{} is {}", s, n));

    assert_eq!(*combined.force(), "answer is 42");
}

// =============================================================================
// Default
// =============================================================================

#[rstest]
fn lazy_default_for_i32() {
    let lazy: Lazy<i32> = Lazy::default();
    assert_eq!(*lazy.force(), 0);
}

#[rstest]
fn lazy_default_for_string() {
    let lazy: Lazy<String> = Lazy::default();
    assert_eq!(*lazy.force(), "");
}

#[rstest]
fn lazy_default_for_vec() {
    let lazy: Lazy<Vec<i32>> = Lazy::default();
    assert!(lazy.force().is_empty());
}

// =============================================================================
// Debug
// =============================================================================

#[rstest]
fn lazy_debug_uninit() {
    let lazy = Lazy::new(|| 42);
    let debug_str = format!("{:?}", lazy);
    assert!(debug_str.contains("uninit"));
}

#[rstest]
fn lazy_debug_init() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    let debug_str = format!("{:?}", lazy);
    assert!(debug_str.contains("42"));
}

#[rstest]
fn lazy_debug_poisoned() {
    let lazy = Lazy::new(|| panic!("test"));
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));

    let debug_str = format!("{:?}", lazy);
    assert!(debug_str.contains("poisoned"));
}

// =============================================================================
// Complex Scenarios
// =============================================================================

#[rstest]
fn lazy_fibonacci_memoization() {
    // Simulate memoized fibonacci using multiple Lazy values
    let fib_0 = Lazy::new_with_value(0u64);
    let fib_1 = Lazy::new_with_value(1u64);

    let fib_2 = fib_0.zip_with(fib_1, |a, b| a + b);
    assert_eq!(*fib_2.force(), 1);
}

#[rstest]
fn lazy_complex_composition() {
    let lazy1 = Lazy::new(|| 10);
    let lazy2 = Lazy::new(|| 20);
    let lazy3 = Lazy::new(|| 30);

    let result = lazy1
        .zip(lazy2)
        .map(|(a, b)| a + b)
        .flat_map(|sum| Lazy::new(move || sum).zip(lazy3).map(|(s, c)| s + c));

    // 10 + 20 + 30 = 60
    assert_eq!(*result.force(), 60);
}

// =============================================================================
// force_mut
// =============================================================================

#[rstest]
fn lazy_force_mut_computes_and_returns_mutable_ref() {
    let mut lazy = Lazy::new(|| vec![1, 2, 3]);
    lazy.force_mut().push(4);
    assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4]);
}

#[rstest]
fn lazy_force_mut_on_initialized_returns_mutable_ref() {
    let mut lazy = Lazy::new_with_value(vec![1, 2, 3]);
    lazy.force_mut().push(4);
    assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4]);
}

#[rstest]
fn lazy_force_mut_initializes_if_needed() {
    let computed = Cell::new(false);
    let mut lazy = Lazy::new(|| {
        computed.set(true);
        42
    });

    assert!(!computed.get());

    let value = lazy.force_mut();
    assert!(computed.get());
    assert_eq!(*value, 42);
}

#[rstest]
fn lazy_force_mut_modifies_value() {
    let mut lazy = Lazy::new(|| 10);
    *lazy.force_mut() = 42;
    assert_eq!(*lazy.force(), 42);
}

#[rstest]
#[should_panic(expected = "Lazy instance has been poisoned")]
fn lazy_force_mut_on_poisoned_panics() {
    let mut lazy = Lazy::new(|| panic!("initialization failed"));

    // First force - causes panic and poisons
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));

    // force_mut should panic with "poisoned" message
    let _ = lazy.force_mut();
}

// =============================================================================
// get_mut
// =============================================================================

#[rstest]
fn lazy_get_mut_before_force_returns_none() {
    let mut lazy = Lazy::new(|| 42);
    assert!(lazy.get_mut().is_none());
}

#[rstest]
fn lazy_get_mut_after_force_returns_some() {
    let mut lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    assert!(lazy.get_mut().is_some());
}

#[rstest]
fn lazy_get_mut_returns_mutable_ref() {
    let mut lazy = Lazy::new(|| 10);
    let _ = lazy.force();

    if let Some(value) = lazy.get_mut() {
        *value = 42;
    }
    assert_eq!(*lazy.force(), 42);
}

#[rstest]
fn lazy_get_mut_on_new_with_value_returns_some() {
    let mut lazy = Lazy::new_with_value(42);
    assert!(lazy.get_mut().is_some());

    if let Some(value) = lazy.get_mut() {
        *value = 100;
    }
    assert_eq!(*lazy.force(), 100);
}

#[rstest]
fn lazy_get_mut_with_vec() {
    let mut lazy = Lazy::new(|| vec![1, 2, 3]);
    let _ = lazy.force();

    if let Some(vec) = lazy.get_mut() {
        vec.push(4);
        vec.push(5);
    }
    assert_eq!(lazy.force().as_slice(), &[1, 2, 3, 4, 5]);
}

#[rstest]
#[should_panic(expected = "Lazy instance has been poisoned")]
fn lazy_get_mut_on_poisoned_panics() {
    let mut lazy = Lazy::new(|| panic!("initialization failed"));

    // First force - causes panic and poisons
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));

    // get_mut should panic with "poisoned" message
    let _ = lazy.get_mut();
}

// =============================================================================
// into_inner
// =============================================================================

#[rstest]
fn lazy_into_inner_uninit_forces_and_returns_ok() {
    let lazy = Lazy::new(|| 42);
    assert_eq!(lazy.into_inner(), Ok(42));
}

#[rstest]
fn lazy_into_inner_init_returns_ok() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    // Cannot call into_inner after force because force borrows
    // Test with new_with_value instead
    let lazy2 = Lazy::new_with_value(100);
    assert_eq!(lazy2.into_inner(), Ok(100));
}

#[rstest]
fn lazy_into_inner_new_with_value_returns_ok() {
    let lazy = Lazy::new_with_value(42);
    assert_eq!(lazy.into_inner(), Ok(42));
}

#[rstest]
fn lazy_into_inner_pure_returns_ok() {
    let lazy = Lazy::pure("hello".to_string());
    assert_eq!(lazy.into_inner(), Ok("hello".to_string()));
}

#[rstest]
fn lazy_into_inner_poisoned_returns_err() {
    let lazy = Lazy::new(|| panic!("initialization failed"));

    // First force - causes panic and poisons
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));

    // into_inner should return Err(LazyPoisonedError) for poisoned
    assert_eq!(lazy.into_inner(), Err(LazyPoisonedError));
}

#[rstest]
fn lazy_into_inner_computes_on_demand() {
    let computed = Cell::new(false);
    let lazy = Lazy::new(|| {
        computed.set(true);
        42
    });

    assert!(!computed.get());
    let result = lazy.into_inner();
    assert!(computed.get());
    assert_eq!(result, Ok(42));
}

#[rstest]
fn lazy_into_inner_with_complex_type() {
    let lazy = Lazy::new(|| vec![1, 2, 3, 4, 5]);
    let result = lazy.into_inner();
    assert_eq!(result, Ok(vec![1, 2, 3, 4, 5]));
}
