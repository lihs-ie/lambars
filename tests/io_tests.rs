#![cfg(feature = "effect")]
//! Unit tests for the IO monad.
//!
//! This module tests the IO type's basic functionality and ensures
//! that side effects are properly deferred until `run_unsafe` is called.

use lambars::effect::IO;
use lambars::typeclass::{Applicative, Functor, Monad};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// =============================================================================
// Basic IO Tests
// =============================================================================

mod basic_operations {
    use super::*;

    #[test]
    fn test_io_pure_and_run_unsafe() {
        let io = IO::pure(42);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[test]
    fn test_io_new_and_run_unsafe() {
        let io = IO::new(|| 42 + 8);
        assert_eq!(io.run_unsafe(), 50);
    }

    #[test]
    fn test_io_pure_with_string() {
        let io = IO::pure("hello".to_string());
        assert_eq!(io.run_unsafe(), "hello");
    }

    #[test]
    fn test_io_new_with_closure() {
        let value = 10;
        let io = IO::new(move || value * 3);
        assert_eq!(io.run_unsafe(), 30);
    }
}

// =============================================================================
// Lazy Evaluation Tests (side effects deferred until run_unsafe)
// =============================================================================

mod lazy_evaluation {
    use super::*;

    #[test]
    fn test_io_new_is_lazy() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let io = IO::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
            42
        });

        // Not executed just by creating the IO
        assert!(
            !executed.load(Ordering::SeqCst),
            "IO should not execute on creation"
        );

        // Executed via run_unsafe
        let result = io.run_unsafe();
        assert!(
            executed.load(Ordering::SeqCst),
            "IO should execute on run_unsafe"
        );
        assert_eq!(result, 42);
    }

    #[test]
    fn test_io_map_is_lazy() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let io = IO::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
            21
        })
        .fmap(|x| x * 2);

        // Not executed even after map
        assert!(
            !executed.load(Ordering::SeqCst),
            "IO should not execute after map"
        );

        let result = io.run_unsafe();
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(result, 42);
    }

    #[test]
    fn test_io_flat_map_is_lazy() {
        let first_executed = Arc::new(AtomicBool::new(false));
        let second_executed = Arc::new(AtomicBool::new(false));
        let first_clone = first_executed.clone();
        let second_clone = second_executed.clone();

        let io = IO::new(move || {
            first_clone.store(true, Ordering::SeqCst);
            10
        })
        .flat_map(move |x| {
            let second_clone = second_clone.clone();
            IO::new(move || {
                second_clone.store(true, Ordering::SeqCst);
                x * 2
            })
        });

        // Not executed even after flat_map
        assert!(
            !first_executed.load(Ordering::SeqCst),
            "First IO should not execute after flat_map"
        );
        assert!(
            !second_executed.load(Ordering::SeqCst),
            "Second IO should not execute after flat_map"
        );

        let result = io.run_unsafe();
        assert!(first_executed.load(Ordering::SeqCst));
        assert!(second_executed.load(Ordering::SeqCst));
        assert_eq!(result, 20);
    }
}

// =============================================================================
// Functor (fmap) Tests
// =============================================================================

mod functor {
    use super::*;

    #[test]
    fn test_io_fmap_basic() {
        let io = IO::pure(21).fmap(|x| x * 2);
        assert_eq!(io.run_unsafe(), 42);
    }

    #[test]
    fn test_io_fmap_chain() {
        let io = IO::pure(10)
            .fmap(|x| x + 5)
            .fmap(|x| x * 2)
            .fmap(|x| x - 10);
        assert_eq!(io.run_unsafe(), 20); // ((10 + 5) * 2) - 10 = 20
    }

    #[test]
    fn test_io_fmap_type_change() {
        let io = IO::pure(42).fmap(|x| format!("value: {}", x));
        assert_eq!(io.run_unsafe(), "value: 42");
    }

    #[test]
    fn test_io_fmap_identity() {
        let io = IO::pure(42).fmap(|x| x);
        assert_eq!(io.run_unsafe(), 42);
    }
}

// =============================================================================
// Monad (flat_map) Tests
// =============================================================================

mod monad {
    use super::*;

    #[test]
    fn test_io_flat_map_basic() {
        let io = IO::pure(10).flat_map(|x| IO::pure(x * 2));
        assert_eq!(io.run_unsafe(), 20);
    }

    #[test]
    fn test_io_flat_map_chain() {
        let io = IO::pure(5)
            .flat_map(|x| IO::pure(x + 10))
            .flat_map(|x| IO::pure(x * 2));
        assert_eq!(io.run_unsafe(), 30); // (5 + 10) * 2 = 30
    }

    #[test]
    fn test_io_and_then_alias() {
        let io = IO::pure(10).and_then(|x| IO::pure(x + 5));
        assert_eq!(io.run_unsafe(), 15);
    }

    #[test]
    fn test_io_then_discards_result() {
        let execution_count = Arc::new(AtomicUsize::new(0));
        let count_clone = execution_count.clone();

        let io = IO::new(move || {
            count_clone.fetch_add(1, Ordering::SeqCst);
            "first result"
        })
        .then(IO::pure("second result".to_string()));

        let result = io.run_unsafe();
        assert_eq!(result, "second result");
        assert_eq!(
            execution_count.load(Ordering::SeqCst),
            1,
            "First IO should have executed"
        );
    }
}

// =============================================================================
// Applicative (map2, product) Tests
// =============================================================================

mod applicative {
    use super::*;

    #[test]
    fn test_io_map2() {
        let io1 = IO::pure(10);
        let io2 = IO::pure(20);
        let io = io1.map2(io2, |a, b| a + b);
        assert_eq!(io.run_unsafe(), 30);
    }

    #[test]
    fn test_io_map2_with_different_types() {
        let io1 = IO::pure(42);
        let io2 = IO::pure("hello".to_string());
        let io = io1.map2(io2, |n, s| format!("{}: {}", s, n));
        assert_eq!(io.run_unsafe(), "hello: 42");
    }

    #[test]
    fn test_io_product() {
        let io1 = IO::pure(10);
        let io2 = IO::pure("hello".to_string());
        let io = io1.product(io2);
        assert_eq!(io.run_unsafe(), (10, "hello".to_string()));
    }
}

// =============================================================================
// Convenience Constructor Tests
// =============================================================================

mod convenience_constructors {
    use super::*;

    #[test]
    fn test_io_print_line_is_lazy() {
        // print_line returns an IO but does not produce output until run_unsafe
        let io = IO::print_line("test message");
        // No output since run_unsafe is not called
        drop(io);
    }

    #[test]
    fn test_io_delay_is_lazy() {
        use std::time::Duration;

        let start = std::time::Instant::now();
        let io = IO::delay(Duration::from_millis(100));

        // Creating a delay IO does not block
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "delay should not execute on creation"
        );

        // Blocks on run_unsafe
        #[allow(clippy::let_unit_value)]
        let () = io.run_unsafe();
        assert!(
            start.elapsed() >= Duration::from_millis(100),
            "delay should wait on run_unsafe"
        );
    }

    #[test]
    fn test_io_catch_recovers_from_panic() {
        let panicking_io = IO::new(|| panic!("intentional panic for testing"));

        let recovered_io = IO::catch(panicking_io, |_| "recovered".to_string());
        let result = recovered_io.run_unsafe();

        assert_eq!(result, "recovered");
    }

    #[test]
    fn test_io_catch_does_not_interfere_with_success() {
        let successful_io = IO::pure("success".to_string());
        let with_catch = IO::catch(successful_io, |_| "recovered".to_string());

        assert_eq!(with_catch.run_unsafe(), "success");
    }
}

// =============================================================================
// Composite Operation Tests
// =============================================================================

mod composite_operations {
    use super::*;

    #[test]
    fn test_io_complex_chain() {
        let io = IO::pure(1)
            .flat_map(|x| IO::pure(x + 1))
            .fmap(|x| x * 10)
            .flat_map(|x| IO::pure(x + 5))
            .fmap(|x| format!("result: {}", x));

        assert_eq!(io.run_unsafe(), "result: 25");
    }

    #[test]
    fn test_io_side_effect_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let order1 = order.clone();
        let order2 = order.clone();
        let order3 = order.clone();

        let io = IO::new(move || {
            order1.lock().unwrap().push(1);
            "first"
        })
        .flat_map(move |_| {
            IO::new(move || {
                order2.lock().unwrap().push(2);
                "second"
            })
        })
        .flat_map(move |_| {
            IO::new(move || {
                order3.lock().unwrap().push(3);
                "third"
            })
        });

        let _ = io.run_unsafe();
        assert_eq!(*order.lock().unwrap(), vec![1, 2, 3]);
    }
}
