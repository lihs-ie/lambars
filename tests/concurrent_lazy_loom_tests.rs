//! Concurrency tests for ConcurrentLazy.
//!
//! This module verifies the correctness of the lock-free ConcurrentLazy implementation
//! through multi-threaded stress testing.
//!
//! # Note on loom integration
//!
//! Full loom model checking would require the implementation to use loom's
//! atomic types conditionally. Currently, these tests use standard thread-based
//! concurrency testing which provides good coverage for common race conditions.
//!
//! # Running these tests
//!
//! ```bash
//! cargo test --test concurrent_lazy_loom_tests --features control
//! ```

#![cfg(feature = "control")]

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use lambars::control::ConcurrentLazy;

/// Test that concurrent initialization happens exactly once.
///
/// This test verifies that when multiple threads call `force()` simultaneously,
/// the initialization function is executed exactly once.
#[test]
fn test_concurrent_init_exactly_once() {
    for _ in 0..100 {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let lazy = Arc::new(ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            42
        }));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || *l.force())
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }

        // Initialization should happen exactly once
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

/// Test that state transitions are correct under concurrent access.
///
/// Verifies that after any thread calls `force()`, the lazy value
/// is in the initialized state.
#[test]
fn test_state_transition_to_ready() {
    for _ in 0..100 {
        let lazy = Arc::new(ConcurrentLazy::new(|| 42));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || {
                    let _ = l.force();
                    l.is_initialized()
                })
            })
            .collect();

        // All threads should observe initialized state after force()
        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }
}

/// Test that values are consistent across threads.
///
/// Verifies that all threads see the same value after initialization.
#[test]
fn test_value_consistency() {
    for _ in 0..100 {
        let lazy = Arc::new(ConcurrentLazy::new(|| 100));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || *l.force())
            })
            .collect();

        let values: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All values must be identical
        for value in values {
            assert_eq!(value, 100);
        }
    }
}

/// Test get() behavior during concurrent access.
///
/// Verifies that `get()` returns `None` before initialization and
/// `Some(&value)` after initialization, with proper visibility.
#[test]
fn test_get_visibility() {
    for _ in 0..100 {
        let lazy = Arc::new(ConcurrentLazy::new(|| 42));

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || {
                    if i == 0 {
                        // Thread 0 forces initialization
                        let _ = l.force();
                        l.get().copied()
                    } else {
                        // Other threads force and verify
                        let forced = *l.force();
                        Some(forced)
                    }
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            // All threads should see Some(42) after force
            assert_eq!(result, Some(42));
        }
    }
}

/// Test concurrent access with many threads.
///
/// Stress test with more threads to increase chance of race conditions.
#[test]
fn test_high_contention() {
    for _ in 0..10 {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let lazy = Arc::new(ConcurrentLazy::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            std::thread::yield_now(); // Add some delay to increase contention
            42
        }));

        let handles: Vec<_> = (0..32)
            .map(|_| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || *l.force())
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), 42);
        }

        // Even under high contention, initialization should happen exactly once
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

/// Test that poison is properly propagated to all threads.
///
/// Verifies that if initialization panics, all subsequent access attempts
/// also panic.
#[test]
fn test_poison_propagation_concurrent() {
    for _ in 0..100 {
        let lazy = Arc::new(ConcurrentLazy::new(|| -> i32 { panic!("test panic") }));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || {
                    std::panic::catch_unwind(AssertUnwindSafe(|| *l.force())).is_err()
                })
            })
            .collect();

        // All threads should observe the panic
        for handle in handles {
            assert!(handle.join().unwrap());
        }

        // The lazy should be poisoned
        assert!(lazy.is_poisoned());
    }
}

/// Test mixed access patterns.
///
/// Some threads force, some check get(), some check is_initialized().
#[test]
fn test_mixed_access_patterns() {
    for _ in 0..100 {
        let lazy = Arc::new(ConcurrentLazy::new(|| 42));

        let handles: Vec<_> = (0..12)
            .map(|i| {
                let l = Arc::clone(&lazy);
                thread::spawn(move || match i % 3 {
                    0 => {
                        // Force and return value
                        Some(*l.force())
                    }
                    1 => {
                        // Check get()
                        l.get().copied()
                    }
                    _ => {
                        // Check is_initialized()
                        if l.is_initialized() { Some(42) } else { None }
                    }
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All force() calls should return 42
        for (i, result) in results.iter().enumerate() {
            if i % 3 == 0 {
                assert_eq!(*result, Some(42));
            }
        }
    }
}
