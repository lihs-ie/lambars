#![cfg(feature = "async")]
//! Tests for the eff_async! macro.
//!
//! This module tests the eff_async! macro which provides do-notation syntax
//! for AsyncIO monad operations.

use lambars::eff_async;
use lambars::effect::AsyncIO;
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// =============================================================================
// Basic Bind Operations
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_single_bind() {
    let result = eff_async! {
        x <= AsyncIO::pure(5);
        AsyncIO::pure(x * 2)
    };
    assert_eq!(result.run_async().await, 10);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_multiple_binds() {
    let result = eff_async! {
        x <= AsyncIO::pure(5);
        y <= AsyncIO::pure(10);
        z <= AsyncIO::pure(15);
        AsyncIO::pure(x + y + z)
    };
    assert_eq!(result.run_async().await, 30);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_with_let() {
    let result = eff_async! {
        x <= AsyncIO::pure(5);
        let doubled = x * 2;
        y <= AsyncIO::pure(10);
        let sum = doubled + y;
        AsyncIO::pure(sum)
    };
    assert_eq!(result.run_async().await, 20);
}

// =============================================================================
// Pattern Matching
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_wildcard_pattern() {
    let result = eff_async! {
        _ <= AsyncIO::pure("ignored");
        AsyncIO::pure(42)
    };
    assert_eq!(result.run_async().await, 42);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_tuple_pattern() {
    let result = eff_async! {
        (x, y) <= AsyncIO::pure((10, 20));
        AsyncIO::pure(x + y)
    };
    assert_eq!(result.run_async().await, 30);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_nested_tuple_pattern() {
    let result = eff_async! {
        ((a, b), c) <= AsyncIO::pure(((1, 2), 3));
        AsyncIO::pure(a + b + c)
    };
    assert_eq!(result.run_async().await, 6);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_let_tuple_pattern() {
    let result = eff_async! {
        pair <= AsyncIO::pure((10, 20));
        let (x, y) = pair;
        AsyncIO::pure(x * y)
    };
    assert_eq!(result.run_async().await, 200);
}

// =============================================================================
// Complex Expressions
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_with_function_calls() {
    fn double(x: i32) -> i32 {
        x * 2
    }

    let result = eff_async! {
        x <= AsyncIO::pure(5);
        let doubled = double(x);
        y <= AsyncIO::pure(doubled);
        AsyncIO::pure(y + 1)
    };
    assert_eq!(result.run_async().await, 11);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_with_closures() {
    let multiplier = 3;

    let result = eff_async! {
        x <= AsyncIO::pure(5);
        let result = x * multiplier;
        AsyncIO::pure(result)
    };
    assert_eq!(result.run_async().await, 15);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_nested() {
    let inner = eff_async! {
        x <= AsyncIO::pure(10);
        AsyncIO::pure(x * 2)
    };

    let outer = eff_async! {
        y <= inner;
        z <= AsyncIO::pure(5);
        AsyncIO::pure(y + z)
    };

    assert_eq!(outer.run_async().await, 25);
}

// =============================================================================
// Side Effects and Ordering
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_execution_order() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let order1 = order.clone();
    let order2 = order.clone();
    let order3 = order.clone();

    let async_io = eff_async! {
        x <= AsyncIO::new(move || {
            let o = order1.clone();
            async move {
                o.lock().unwrap().push(1);
                10
            }
        });
        y <= AsyncIO::new(move || {
            let o = order2.clone();
            async move {
                o.lock().unwrap().push(2);
                20
            }
        });
        let sum = x + y;
        _ <= AsyncIO::new(move || {
            let o = order3.clone();
            async move {
                o.lock().unwrap().push(3);
            }
        });
        AsyncIO::pure(sum)
    };

    let result = async_io.run_async().await;
    assert_eq!(result, 30);

    let execution_order = order.lock().unwrap().clone();
    assert_eq!(execution_order, vec![1, 2, 3]);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_is_lazy() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let async_io = eff_async! {
        x <= AsyncIO::new(move || {
            let cnt = counter_clone.clone();
            async move {
                cnt.fetch_add(1, Ordering::SeqCst);
                42
            }
        });
        AsyncIO::pure(x)
    };

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let result = async_io.run_async().await;
    assert_eq!(result, 42);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// =============================================================================
// Type Transformations
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_type_change() {
    let result = eff_async! {
        x <= AsyncIO::pure(42);
        let string = format!("value: {}", x);
        AsyncIO::pure(string)
    };
    assert_eq!(result.run_async().await, "value: 42");
}

#[rstest]
#[tokio::test]
async fn test_eff_async_with_string() {
    let result = eff_async! {
        s <= AsyncIO::pure("hello".to_string());
        let upper = s.to_uppercase();
        AsyncIO::pure(upper)
    };
    assert_eq!(result.run_async().await, "HELLO");
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_eff_async_single_expression() {
    let result = eff_async! {
        AsyncIO::pure(42)
    };
    assert_eq!(result.run_async().await, 42);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_only_let() {
    let result = eff_async! {
        let x = 10;
        let y = 20;
        AsyncIO::pure(x + y)
    };
    assert_eq!(result.run_async().await, 30);
}

#[rstest]
#[tokio::test]
async fn test_eff_async_complex_workflow() {
    fn validate(x: i32) -> AsyncIO<Result<i32, String>> {
        if x > 0 {
            AsyncIO::pure(Ok(x))
        } else {
            AsyncIO::pure(Err("negative".to_string()))
        }
    }

    fn transform(x: i32) -> AsyncIO<i32> {
        AsyncIO::pure(x * 2)
    }

    let result = eff_async! {
        validated <= validate(10);
        let value = validated.expect("should be valid");
        transformed <= transform(value);
        AsyncIO::pure(transformed)
    };

    assert_eq!(result.run_async().await, 20);
}
