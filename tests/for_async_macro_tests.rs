//! Unit tests for the `for_async!` macro.
//!
//! These tests verify the correct behavior of the for-comprehension
//! macro with async support.

#![cfg(feature = "async")]
#![allow(deprecated)]

use lambars::effect::AsyncIO;
use lambars::for_async;

// =============================================================================
// Step 2: Basic Iteration (yield terminal)
// =============================================================================

#[tokio::test]
async fn test_single_iteration_vec() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield x * 2
    };
    assert_eq!(result.run_async().await, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_single_iteration_array() {
    let result = for_async! {
        x <= [1, 2, 3];
        yield x + 10
    };
    assert_eq!(result.run_async().await, vec![11, 12, 13]);
}

#[tokio::test]
async fn test_single_iteration_range() {
    let result = for_async! {
        x <= 1..4;
        yield x * x
    };
    assert_eq!(result.run_async().await, vec![1, 4, 9]);
}

// =============================================================================
// Step 3: Nested Iteration
// =============================================================================

#[tokio::test]
async fn test_nested_iteration_two_levels() {
    let result = for_async! {
        x <= vec![1, 2];
        y <= vec![10, 20];
        yield x + y
    };
    assert_eq!(result.run_async().await, vec![11, 21, 12, 22]);
}

#[tokio::test]
async fn test_nested_iteration_three_levels() {
    let result = for_async! {
        x <= vec![1, 2];
        y <= vec![10, 20];
        z <= vec![100, 200];
        yield x + y + z
    };
    assert_eq!(
        result.run_async().await,
        vec![111, 211, 121, 221, 112, 212, 122, 222]
    );
}

// =============================================================================
// Step 4: AsyncIO Bind (<~ operator)
// =============================================================================

#[tokio::test]
async fn test_async_bind_simple() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        doubled <~ AsyncIO::pure(x * 2);
        yield doubled
    };
    assert_eq!(result.run_async().await, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_async_bind_multiple() {
    let result = for_async! {
        x <= vec![1, 2];
        y <~ AsyncIO::pure(x * 10);
        z <~ AsyncIO::pure(y + 1);
        yield z
    };
    assert_eq!(result.run_async().await, vec![11, 21]);
}

#[tokio::test]
async fn test_async_bind_with_collection() {
    let result = for_async! {
        x <= vec![1, 2];
        y <= vec![10, 20];
        sum <~ AsyncIO::pure(x + y);
        yield sum
    };
    assert_eq!(result.run_async().await, vec![11, 21, 12, 22]);
}

// =============================================================================
// Step 5: Tuple Patterns
// =============================================================================

#[tokio::test]
async fn test_tuple_pattern_collection() {
    let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
    let result = for_async! {
        (num, letter) <= pairs;
        yield format!("{}{}", num, letter)
    };
    assert_eq!(result.run_async().await, vec!["1a", "2b", "3c"]);
}

#[tokio::test]
async fn test_tuple_pattern_async_bind() {
    let result = for_async! {
        x <= vec![1, 2];
        (a, b) <~ AsyncIO::pure((x, x * 10));
        yield a + b
    };
    assert_eq!(result.run_async().await, vec![11, 22]);
}

#[tokio::test]
async fn test_tuple_pattern_nested() {
    let nested = vec![((1, 2), "a"), ((3, 4), "b")];
    let result = for_async! {
        ((x, y), label) <= nested;
        yield format!("{}: ({}, {})", label, x, y)
    };
    assert_eq!(result.run_async().await, vec!["a: (1, 2)", "b: (3, 4)"]);
}

// =============================================================================
// Step 6: Wildcard Patterns
// =============================================================================

#[tokio::test]
async fn test_wildcard_collection() {
    let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
    let result = for_async! {
        (_, letter) <= pairs;
        yield letter.to_uppercase()
    };
    assert_eq!(result.run_async().await, vec!["A", "B", "C"]);
}

#[tokio::test]
async fn test_wildcard_full_element() {
    let result = for_async! {
        _ <= vec![1, 2, 3];
        yield "x"
    };
    assert_eq!(result.run_async().await, vec!["x", "x", "x"]);
}

#[tokio::test]
async fn test_wildcard_async_bind() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        _ <~ AsyncIO::pure("ignored");
        yield x
    };
    assert_eq!(result.run_async().await, vec![1, 2, 3]);
}

// =============================================================================
// Step 7: Let Bindings
// =============================================================================

#[tokio::test]
async fn test_let_binding_simple() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        let doubled = x * 2;
        yield doubled
    };
    assert_eq!(result.run_async().await, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_let_binding_with_async() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        data <~ AsyncIO::pure(x * 10);
        let processed = data + 1;
        yield processed
    };
    assert_eq!(result.run_async().await, vec![11, 21, 31]);
}

#[tokio::test]
async fn test_let_binding_multiple() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        let doubled = x * 2;
        let squared = doubled * doubled;
        yield squared
    };
    assert_eq!(result.run_async().await, vec![4, 16, 36]);
}

#[tokio::test]
async fn test_let_tuple_binding() {
    let result = for_async! {
        pair <= vec![(1, 2), (3, 4), (5, 6)];
        let (a, b) = pair;
        yield a + b
    };
    assert_eq!(result.run_async().await, vec![3, 7, 11]);
}

// =============================================================================
// Step 8: Empty Collections
// =============================================================================

#[tokio::test]
async fn test_empty_source_collection() {
    let empty: Vec<i32> = vec![];
    let result = for_async! {
        x <= empty;
        yield x * 2
    };
    assert_eq!(result.run_async().await, Vec::<i32>::new());
}

#[tokio::test]
async fn test_empty_nested_collection() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        y <= if x == 2 { vec![] } else { vec![x] };
        yield y
    };
    assert_eq!(result.run_async().await, vec![1, 3]);
}

// =============================================================================
// Step 9: Deferred Execution
// =============================================================================

#[tokio::test]
async fn test_deferred_execution() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let execution_count = Arc::new(AtomicUsize::new(0));
    let counter = execution_count.clone();

    // Create the for_async computation
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield {
            counter.fetch_add(1, Ordering::SeqCst);
            x
        }
    };

    // Not executed yet - counter should be 0
    assert_eq!(execution_count.load(Ordering::SeqCst), 0);

    // Execute
    let values = result.run_async().await;

    // Now execution should have happened 3 times
    assert_eq!(execution_count.load(Ordering::SeqCst), 3);
    assert_eq!(values, vec![1, 2, 3]);
}

// =============================================================================
// Step 10: Composition (fmap, flat_map)
// =============================================================================

#[tokio::test]
async fn test_fmap_composition() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield x * 2
    }
    .fmap(|vec| vec.into_iter().sum::<i32>());

    assert_eq!(result.run_async().await, 12);
}

#[tokio::test]
async fn test_flat_map_composition() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield x * 2
    }
    .flat_map(|vec| AsyncIO::pure(vec.len()));

    assert_eq!(result.run_async().await, 3);
}

// =============================================================================
// Additional Tests
// =============================================================================

#[tokio::test]
async fn test_string_iteration() {
    let result = for_async! {
        s <= vec!["hello", "world"];
        yield s.to_uppercase()
    };
    assert_eq!(result.run_async().await, vec!["HELLO", "WORLD"]);
}

#[tokio::test]
async fn test_complex_async_chain() {
    // For x=1: a=10, for y=3: b=300, yield 310
    //          a=10, for y=4: b=400, yield 410
    // For x=2: a=20, for y=3: b=300, yield 320
    //          a=20, for y=4: b=400, yield 420
    let result = for_async! {
        x <= vec![1, 2];
        a <~ AsyncIO::pure(x * 10);
        y <= vec![3, 4];
        b <~ AsyncIO::pure(y * 100);
        yield a + b
    };
    assert_eq!(result.run_async().await, vec![310, 410, 320, 420]);
}
