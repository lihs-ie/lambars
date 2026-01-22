#![cfg(feature = "control")]

use lambars::control::{ConcurrentLazy, ConcurrentLazyPoisonedError};
use rstest::rstest;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// =============================================================================
// Basic Construction and Evaluation
// =============================================================================

#[rstest]
fn concurrent_lazy_defers_computation() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let _lazy = ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        42
    });

    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[rstest]
fn concurrent_lazy_force_computes_value() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let lazy = ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        42
    });

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let value = lazy.force();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(*value, 42);
}

#[rstest]
fn concurrent_lazy_force_returns_ref() {
    let lazy = ConcurrentLazy::new(|| "hello".to_string());
    let value = lazy.force();

    assert_eq!(value.len(), 5);
    assert!(value.starts_with("hel"));
}

// =============================================================================
// Memoization
// =============================================================================

#[rstest]
fn concurrent_lazy_memoization_single_computation() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let lazy = ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        42
    });

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    let _ = lazy.force();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    let _ = lazy.force();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    let _ = lazy.force();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_memoization_preserves_value() {
    let lazy = ConcurrentLazy::new(|| "computed_value".to_string());

    let first = lazy.force().clone();
    let second = lazy.force().clone();

    assert_eq!(first, "computed_value");
    assert_eq!(second, "computed_value");
}

// =============================================================================
// new_with_value
// =============================================================================

#[rstest]
fn concurrent_lazy_new_with_value_is_initialized() {
    let lazy = ConcurrentLazy::new_with_value(42);
    assert!(lazy.is_initialized());
}

#[rstest]
fn concurrent_lazy_new_with_value_force_returns_value() {
    let lazy = ConcurrentLazy::new_with_value(42);
    assert_eq!(*lazy.force(), 42);
}

#[rstest]
fn concurrent_lazy_pure_is_alias_for_new_with_value() {
    let lazy = ConcurrentLazy::pure("hello");
    assert!(lazy.is_initialized());
    assert_eq!(*lazy.force(), "hello");
}

// =============================================================================
// get Method
// =============================================================================

#[rstest]
fn concurrent_lazy_get_before_force_returns_none() {
    let lazy = ConcurrentLazy::new(|| 42);
    assert!(lazy.get().is_none());
}

#[rstest]
fn concurrent_lazy_get_after_force_returns_some() {
    let lazy = ConcurrentLazy::new(|| 42);
    let _ = lazy.force();
    assert!(lazy.get().is_some());
    assert_eq!(*lazy.get().unwrap(), 42);
}

#[rstest]
fn concurrent_lazy_get_on_new_with_value_returns_some() {
    let lazy = ConcurrentLazy::new_with_value(42);
    assert!(lazy.get().is_some());
    assert_eq!(*lazy.get().unwrap(), 42);
}

// =============================================================================
// is_initialized
// =============================================================================

#[rstest]
fn concurrent_lazy_is_initialized_false_initially() {
    let lazy = ConcurrentLazy::new(|| 42);
    assert!(!lazy.is_initialized());
}

#[rstest]
fn concurrent_lazy_is_initialized_true_after_force() {
    let lazy = ConcurrentLazy::new(|| 42);
    let _ = lazy.force();
    assert!(lazy.is_initialized());
}

#[rstest]
fn concurrent_lazy_is_initialized_true_for_new_with_value() {
    let lazy = ConcurrentLazy::new_with_value(42);
    assert!(lazy.is_initialized());
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[rstest]
fn concurrent_lazy_initialization_exactly_once() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let lazy = Arc::new(ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        42
    }));

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let lazy = Arc::clone(&lazy);
            thread::spawn(move || *lazy.force())
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().unwrap(), 42);
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_force_same_value_across_threads() {
    let lazy = Arc::new(ConcurrentLazy::new(|| 42));

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let lazy = Arc::clone(&lazy);
            thread::spawn(move || *lazy.force())
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert!(results.iter().all(|&x| x == 42));
}

#[rstest]
fn concurrent_lazy_is_initialized_after_force_in_thread() {
    let lazy = Arc::new(ConcurrentLazy::new(|| 42));

    let lazy_clone = Arc::clone(&lazy);
    let handle = thread::spawn(move || {
        lazy_clone.force();
    });

    handle.join().unwrap();
    assert!(lazy.is_initialized());
}

#[rstest]
fn concurrent_lazy_get_is_eventually_consistent() {
    let lazy = Arc::new(ConcurrentLazy::new(|| 42));

    let lazy_clone = Arc::clone(&lazy);
    let handle = thread::spawn(move || {
        lazy_clone.force();
    });

    handle.join().unwrap();
    assert_eq!(lazy.get(), Some(&42));
}

// =============================================================================
// map
// =============================================================================

#[rstest]
fn concurrent_lazy_map_transforms_value() {
    let lazy = ConcurrentLazy::new(|| 21);
    let doubled = lazy.map(|x| x * 2);
    assert_eq!(*doubled.force(), 42);
}

#[rstest]
fn concurrent_lazy_map_is_lazy() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let lazy = ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        21
    });
    let mapped = lazy.map(|x| x * 2);

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    let _ = mapped.force();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_map_chain() {
    let lazy = ConcurrentLazy::new(|| 10);
    let result = lazy.map(|x| x + 1).map(|x| x * 2).map(|x| x - 2);

    assert_eq!(*result.force(), 20); // (10 + 1) * 2 - 2 = 20
}

#[rstest]
fn concurrent_lazy_map_type_change() {
    let lazy = ConcurrentLazy::new(|| 42);
    let stringified = lazy.map(|x| x.to_string());
    assert_eq!(*stringified.force(), "42");
}

// =============================================================================
// flat_map
// =============================================================================

#[rstest]
fn concurrent_lazy_flat_map_basic() {
    let lazy = ConcurrentLazy::new(|| 21);
    let result = lazy.flat_map(|x| ConcurrentLazy::new(move || x * 2));
    assert_eq!(*result.force(), 42);
}

#[rstest]
fn concurrent_lazy_flat_map_is_lazy() {
    let outer_counter = Arc::new(AtomicUsize::new(0));
    let inner_counter = Arc::new(AtomicUsize::new(0));

    let outer_clone = Arc::clone(&outer_counter);
    let lazy = ConcurrentLazy::new(move || {
        outer_clone.fetch_add(1, Ordering::SeqCst);
        21
    });

    let inner_clone = Arc::clone(&inner_counter);
    let result = lazy.flat_map(move |x| {
        let inner_clone2 = Arc::clone(&inner_clone);
        ConcurrentLazy::new(move || {
            inner_clone2.fetch_add(1, Ordering::SeqCst);
            x * 2
        })
    });

    assert_eq!(outer_counter.load(Ordering::SeqCst), 0);
    assert_eq!(inner_counter.load(Ordering::SeqCst), 0);
    let _ = result.force();
    assert_eq!(outer_counter.load(Ordering::SeqCst), 1);
    assert_eq!(inner_counter.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_flat_map_chain() {
    let lazy = ConcurrentLazy::new(|| 10);
    let result = lazy
        .flat_map(|x| ConcurrentLazy::new(move || x + 1))
        .flat_map(|x| ConcurrentLazy::new(move || x * 2));

    assert_eq!(*result.force(), 22); // (10 + 1) * 2 = 22
}

#[rstest]
fn concurrent_lazy_flat_map_with_already_initialized() {
    let lazy = ConcurrentLazy::new(|| 21);
    let result = lazy.flat_map(|x| ConcurrentLazy::new_with_value(x * 2));
    assert_eq!(*result.force(), 42);
}

// =============================================================================
// zip
// =============================================================================

#[rstest]
fn concurrent_lazy_zip_combines_values() {
    let lazy1 = ConcurrentLazy::new(|| 1);
    let lazy2 = ConcurrentLazy::new(|| "hello");
    let combined = lazy1.zip(lazy2);

    assert_eq!(*combined.force(), (1, "hello"));
}

#[rstest]
fn concurrent_lazy_zip_is_lazy() {
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));

    let counter1_clone = Arc::clone(&counter1);
    let lazy1 = ConcurrentLazy::new(move || {
        counter1_clone.fetch_add(1, Ordering::SeqCst);
        1
    });

    let counter2_clone = Arc::clone(&counter2);
    let lazy2 = ConcurrentLazy::new(move || {
        counter2_clone.fetch_add(1, Ordering::SeqCst);
        2
    });

    let combined = lazy1.zip(lazy2);

    assert_eq!(counter1.load(Ordering::SeqCst), 0);
    assert_eq!(counter2.load(Ordering::SeqCst), 0);
    let _ = combined.force();
    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

// =============================================================================
// zip_with
// =============================================================================

#[rstest]
fn concurrent_lazy_zip_with_combines_with_function() {
    let lazy1 = ConcurrentLazy::new(|| 20);
    let lazy2 = ConcurrentLazy::new(|| 22);
    let sum = lazy1.zip_with(lazy2, |a, b| a + b);

    assert_eq!(*sum.force(), 42);
}

#[rstest]
fn concurrent_lazy_zip_with_is_lazy() {
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));

    let counter1_clone = Arc::clone(&counter1);
    let lazy1 = ConcurrentLazy::new(move || {
        counter1_clone.fetch_add(1, Ordering::SeqCst);
        1
    });

    let counter2_clone = Arc::clone(&counter2);
    let lazy2 = ConcurrentLazy::new(move || {
        counter2_clone.fetch_add(1, Ordering::SeqCst);
        2
    });

    let combined = lazy1.zip_with(lazy2, |a, b| a + b);

    assert_eq!(counter1.load(Ordering::SeqCst), 0);
    assert_eq!(counter2.load(Ordering::SeqCst), 0);

    let _ = combined.force();
    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_zip_with_type_change() {
    let lazy1 = ConcurrentLazy::new(|| 42);
    let lazy2 = ConcurrentLazy::new(|| "answer");
    let combined = lazy1.zip_with(lazy2, |n, s| format!("{} is {}", s, n));

    assert_eq!(*combined.force(), "answer is 42");
}

// =============================================================================
// Default
// =============================================================================

#[rstest]
fn concurrent_lazy_default_for_i32() {
    let lazy: ConcurrentLazy<i32> = ConcurrentLazy::default();
    assert_eq!(*lazy.force(), 0);
}

#[rstest]
fn concurrent_lazy_default_for_string() {
    let lazy: ConcurrentLazy<String> = ConcurrentLazy::default();
    assert_eq!(*lazy.force(), "");
}

#[rstest]
fn concurrent_lazy_default_for_vec() {
    let lazy: ConcurrentLazy<Vec<i32>> = ConcurrentLazy::default();
    assert!(lazy.force().is_empty());
}

// =============================================================================
// Debug
// =============================================================================

#[rstest]
fn concurrent_lazy_debug_uninit() {
    let lazy = ConcurrentLazy::new(|| 42);
    let debug_str = format!("{:?}", lazy);
    assert!(debug_str.contains("uninit"));
}

#[rstest]
fn concurrent_lazy_debug_init() {
    let lazy = ConcurrentLazy::new(|| 42);
    let _ = lazy.force();
    let debug_str = format!("{:?}", lazy);
    assert!(debug_str.contains("42"));
}

// =============================================================================
// Display
// =============================================================================

#[rstest]
fn concurrent_lazy_display_uninit() {
    let lazy = ConcurrentLazy::new(|| 42);
    let display_str = format!("{}", lazy);
    assert_eq!(display_str, "ConcurrentLazy(<uninit>)");
}

#[rstest]
fn concurrent_lazy_display_init() {
    let lazy = ConcurrentLazy::new(|| 42);
    let _ = lazy.force();
    let display_str = format!("{}", lazy);
    assert_eq!(display_str, "ConcurrentLazy(42)");
}

// =============================================================================
// into_inner
// =============================================================================

#[rstest]
fn concurrent_lazy_into_inner_uninit_forces_and_returns_ok() {
    let lazy = ConcurrentLazy::new(|| 42);
    assert_eq!(lazy.into_inner(), Ok(42));
}

#[rstest]
fn concurrent_lazy_into_inner_init_returns_ok() {
    let lazy = ConcurrentLazy::new(|| 42);
    let _ = lazy.force();
    assert_eq!(lazy.into_inner(), Ok(42));
}

#[rstest]
fn concurrent_lazy_into_inner_new_with_value_returns_ok() {
    let lazy = ConcurrentLazy::new_with_value(42);
    assert_eq!(lazy.into_inner(), Ok(42));
}

#[rstest]
fn concurrent_lazy_into_inner_pure_returns_ok() {
    let lazy = ConcurrentLazy::pure("hello".to_string());
    assert_eq!(lazy.into_inner(), Ok("hello".to_string()));
}

#[rstest]
fn concurrent_lazy_into_inner_computes_on_demand() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let lazy = ConcurrentLazy::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        42
    });

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(lazy.into_inner(), Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[rstest]
fn concurrent_lazy_into_inner_with_complex_type() {
    let lazy = ConcurrentLazy::new(|| vec![1, 2, 3, 4, 5]);
    let result = lazy.into_inner();
    assert_eq!(result, Ok(vec![1, 2, 3, 4, 5]));
}

// =============================================================================
// Panic Handling in Initializer
// =============================================================================

#[rstest]
#[should_panic(expected = "ConcurrentLazy instance has been poisoned")]
fn concurrent_lazy_force_after_panic_panics() {
    let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));
    let _ = lazy.force();
}

#[rstest]
fn concurrent_lazy_into_inner_after_panic_returns_err() {
    let lazy = ConcurrentLazy::new(|| -> i32 { panic!("initialization failed") });

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = lazy.force();
    }));
    assert_eq!(lazy.into_inner(), Err(ConcurrentLazyPoisonedError));
}

// =============================================================================
// Complex Scenarios
// =============================================================================

#[rstest]
fn concurrent_lazy_complex_composition() {
    let lazy1 = ConcurrentLazy::new(|| 10);
    let lazy2 = ConcurrentLazy::new(|| 20);
    let lazy3 = ConcurrentLazy::new(|| 30);

    let result = lazy1.zip(lazy2).map(|(a, b)| a + b).flat_map(|sum| {
        ConcurrentLazy::new(move || sum)
            .zip(lazy3)
            .map(|(s, c)| s + c)
    });

    assert_eq!(*result.force(), 60); // 10 + 20 + 30 = 60
}

#[rstest]
fn concurrent_lazy_thread_safe_complex_scenario() {
    let lazy = Arc::new(ConcurrentLazy::new(|| vec![1, 2, 3]));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let lazy = Arc::clone(&lazy);
            thread::spawn(move || {
                let value = lazy.force();
                value.iter().sum::<i32>()
            })
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().unwrap(), 6);
    }
}
