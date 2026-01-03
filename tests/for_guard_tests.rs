//! Integration tests for guard expressions in for_! and for_async! macros.
//!
//! This module tests the `if condition;` guard expression syntax that allows
//! filtering during for-comprehension iteration.

#![cfg(feature = "compose")]
#![forbid(unsafe_code)]

use lambars::for_;

// =============================================================================
// for_! Guard Expression Tests
// =============================================================================

/// Basic guard expression - filters even numbers
#[test]
fn test_for_guard_basic_filter() {
    let result = for_! {
        x <= vec![1, 2, 3, 4, 5];
        if x % 2 == 0;
        yield x
    };
    assert_eq!(result, vec![2, 4]);
}

/// All elements pass the guard
#[test]
fn test_for_guard_all_pass() {
    let result = for_! {
        x <= vec![2, 4, 6, 8];
        if x % 2 == 0;
        yield x
    };
    assert_eq!(result, vec![2, 4, 6, 8]);
}

/// All elements are filtered out by guard
#[test]
fn test_for_guard_all_fail() {
    let result = for_! {
        x <= vec![1, 3, 5, 7];
        if x % 2 == 0;
        yield x
    };
    assert_eq!(result, Vec::<i32>::new());
}

/// Empty collection with guard
#[test]
fn test_for_guard_empty_collection() {
    let result = for_! {
        x <= Vec::<i32>::new();
        if x > 0;
        yield x
    };
    assert!(result.is_empty());
}

/// Guard in nested iteration
#[test]
fn test_for_guard_nested_iteration() {
    let result = for_! {
        x <= vec![1, 2, 3];
        y <= vec![10, 20, 30];
        if x + y > 20;
        yield (x, y)
    };
    // x + y > 20 を満たす組み合わせ:
    // (1,20)=21, (1,30)=31, (2,20)=22, (2,30)=32, (3,20)=23, (3,30)=33 = 6 elements
    assert_eq!(result.len(), 6);
    assert!(result.contains(&(1, 20)));
    assert!(result.contains(&(3, 30)));
}

/// Guard placed between two binds
#[test]
fn test_for_guard_between_binds() {
    let result = for_! {
        x <= vec![1, 2, 3, 4];
        if x % 2 == 0;
        y <= vec![10, 20];
        yield (x, y)
    };
    // Only x=2,4 pass, each combined with y=10,20
    assert_eq!(result, vec![(2, 10), (2, 20), (4, 10), (4, 20)]);
}

/// Multiple consecutive guard expressions (AND logic)
#[test]
fn test_for_guard_multiple_consecutive() {
    let result = for_! {
        x <= 1..=100i32;
        if x % 2 == 0;
        if x % 3 == 0;
        if x < 50;
        yield x
    };
    // 6, 12, 18, 24, 30, 36, 42, 48 (multiples of 6 less than 50)
    assert_eq!(result, vec![6, 12, 18, 24, 30, 36, 42, 48]);
}

/// Guard after let binding
#[test]
fn test_for_guard_after_let() {
    let result = for_! {
        x <= vec![1, 2, 3, 4, 5];
        let squared = x * x;
        if squared > 10;
        yield squared
    };
    assert_eq!(result, vec![16, 25]);
}

/// Guard before let binding
#[test]
fn test_for_guard_before_let() {
    let result = for_! {
        x <= vec![1, 2, 3, 4, 5];
        if x > 2;
        let doubled = x * 2;
        yield doubled
    };
    assert_eq!(result, vec![6, 8, 10]);
}

/// Guard with tuple pattern
#[test]
fn test_for_guard_with_tuple_pattern() {
    let pairs = vec![(1, 2), (3, 4), (5, 6), (10, 20)];
    let result = for_! {
        (a, b) <= pairs;
        if b - a > 1;
        yield (a, b)
    };
    assert_eq!(result, vec![(10, 20)]);
}

/// Guard with wildcard pattern
#[test]
fn test_for_guard_with_wildcard() {
    let counter = std::cell::Cell::new(0);
    let result = for_! {
        _ <= vec![1, 2, 3, 4, 5];
        if counter.get() < 3;
        let value = {
            let current = counter.get();
            counter.set(current + 1);
            current
        };
        yield value
    };
    // First 3 iterations pass the guard
    assert_eq!(result, vec![0, 1, 2]);
}

/// Helper function for testing external function in guard
fn is_prime(n: i32) -> bool {
    if n < 2 {
        return false;
    }
    (2..=((n as f64).sqrt() as i32)).all(|i| n % i != 0)
}

/// Guard using external function
#[test]
fn test_for_guard_with_external_function() {
    let result = for_! {
        x <= 1..=20i32;
        if is_prime(x);
        yield x
    };
    assert_eq!(result, vec![2, 3, 5, 7, 11, 13, 17, 19]);
}

/// Complex interleaving of binds, guards, and let bindings
#[test]
fn test_for_guard_interleaved_complex() {
    let result = for_! {
        x <= vec![1, 2, 3];
        if x > 1;
        y <= vec![10, 20, 30];
        let sum = x + y;
        if sum > 25;
        let product = x * y;
        yield (sum, product)
    };
    // x=2: y=20 -> sum=22 (NG), y=30 -> sum=32 (OK) -> (32, 60)
    // x=3: y=10 -> sum=13 (NG), y=20 -> sum=23 (NG), y=30 -> sum=33 (OK) -> (33, 90)
    assert_eq!(result, vec![(32, 60), (33, 90)]);
}

/// Guard with closure condition
#[test]
fn test_for_guard_with_closure() {
    let threshold = 5;
    let filter = |x: &i32| *x > threshold;

    let result = for_! {
        x <= vec![1, 3, 5, 7, 9];
        if filter(&x);
        yield x
    };
    assert_eq!(result, vec![7, 9]);
}

/// Triple nested iteration with guards at different levels
#[test]
fn test_for_guard_triple_nested() {
    let result = for_! {
        x <= vec![1, 2, 3];
        if x != 2;
        y <= vec![10, 20];
        if y > 10;
        z <= vec![100, 200];
        if z == 100;
        yield (x, y, z)
    };
    // x=1,3 pass first guard
    // y=20 passes second guard
    // z=100 passes third guard
    // Results: (1, 20, 100), (3, 20, 100)
    assert_eq!(result, vec![(1, 20, 100), (3, 20, 100)]);
}

// =============================================================================
// for_async! Guard Expression Tests
// =============================================================================

#[cfg(feature = "async")]
mod async_tests {
    use lambars::effect::AsyncIO;
    use lambars::for_async;

    /// Basic guard in for_async!
    #[tokio::test]
    async fn test_for_async_guard_basic_filter() {
        let result = for_async! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result.run_async().await, vec![2, 4]);
    }

    /// Guard with AsyncIO bind
    #[tokio::test]
    async fn test_for_async_guard_with_async_bind() {
        let result = for_async! {
            x <= vec![1, 2, 3, 4, 5];
            data <~ AsyncIO::pure(x * 10);
            if data > 20;
            yield data
        };
        assert_eq!(result.run_async().await, vec![30, 40, 50]);
    }

    /// Multiple guards in for_async!
    #[tokio::test]
    async fn test_for_async_guard_multiple() {
        let result = for_async! {
            x <= 1..=20i32;
            if x % 2 == 0;
            if x > 10;
            yield x
        };
        assert_eq!(result.run_async().await, vec![12, 14, 16, 18, 20]);
    }

    /// Guard with let binding in for_async!
    #[tokio::test]
    async fn test_for_async_guard_with_let() {
        let result = for_async! {
            x <= vec![1, 2, 3, 4, 5];
            let squared = x * x;
            if squared > 10;
            yield squared
        };
        assert_eq!(result.run_async().await, vec![16, 25]);
    }

    /// Guard before AsyncIO bind
    #[tokio::test]
    async fn test_for_async_guard_before_async_bind() {
        let result = for_async! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 1;
            data <~ AsyncIO::pure(x * 100);
            yield data
        };
        assert_eq!(result.run_async().await, vec![100, 300, 500]);
    }

    /// Guard in nested iteration for_async!
    #[tokio::test]
    async fn test_for_async_guard_nested() {
        let result = for_async! {
            x <= vec![1, 2, 3];
            if x > 1;
            y <= vec![10, 20];
            yield x + y
        };
        assert_eq!(result.run_async().await, vec![12, 22, 13, 23]);
    }

    /// All elements filtered out in for_async!
    #[tokio::test]
    async fn test_for_async_guard_all_fail() {
        let result = for_async! {
            x <= vec![1, 3, 5, 7];
            if x % 2 == 0;
            yield x
        };
        assert!(result.run_async().await.is_empty());
    }

    /// Complex guard with async operations
    #[tokio::test]
    async fn test_for_async_guard_complex() {
        let result = for_async! {
            x <= vec![1, 2, 3];
            if x > 1;
            value <~ AsyncIO::pure(x * 10);
            if value < 25;
            let final_value = value + 5;
            yield final_value
        };
        // x=2: value=20 (<25) -> final=25
        // x=3: value=30 (>=25) -> filtered out
        assert_eq!(result.run_async().await, vec![25]);
    }
}
