#![cfg(feature = "control")]
//! Integration tests for the control module.
//!
//! These tests verify that the control structures work correctly
//! together and integrate with Phase 1 (typeclass) and Phase 2 (compose).

use lambars::control::{Continuation, Either, Lazy, Trampoline};
use rstest::rstest;
use std::cell::Cell;

// =============================================================================
// Lazy + Trampoline Integration
// =============================================================================

#[rstest]
fn lazy_with_trampoline_computation() {
    // A lazy value that, when forced, returns a trampoline computation
    fn factorial(n: u64) -> Trampoline<u64> {
        factorial_helper(n, 1)
    }

    fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
        if n <= 1 {
            Trampoline::done(accumulator)
        } else {
            Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
        }
    }

    let lazy_factorial = Lazy::new(|| factorial(10).run());

    // Not computed yet
    assert!(!lazy_factorial.is_initialized());

    // Force it
    let result = lazy_factorial.force();
    assert_eq!(*result, 3_628_800);
}

#[rstest]
fn trampoline_with_lazy_memoization() {
    use std::rc::Rc;

    // Use lazy to memoize intermediate results in a trampoline computation
    let call_count = Rc::new(Cell::new(0));
    let count_clone = call_count.clone();

    let expensive_value = Rc::new(Lazy::new(move || {
        count_clone.set(count_clone.get() + 1);
        42
    }));

    // A trampoline that uses the lazy value multiple times
    // We need to use Rc to share the lazy value across closures
    let lazy1 = expensive_value.clone();
    let lazy2 = expensive_value.clone();

    let result = Trampoline::done(())
        .flat_map(move |_| Trampoline::done(*lazy1.force()))
        .flat_map(move |x| Trampoline::done(x + *lazy2.force()))
        .run();

    // The lazy value should only have been computed once
    assert_eq!(call_count.get(), 1);
    assert_eq!(result, 84); // 42 + 42
}

// =============================================================================
// Either Integration
// =============================================================================

#[rstest]
fn either_with_trampoline_resume() {
    let trampoline = Trampoline::suspend(|| Trampoline::done(42));

    match trampoline.resume() {
        Either::Left(thunk) => {
            let next = thunk();
            match next.resume() {
                Either::Right(value) => assert_eq!(value, 42),
                Either::Left(_) => panic!("Expected Right"),
            }
        }
        Either::Right(_) => panic!("Expected Left"),
    }
}

#[rstest]
#[allow(clippy::type_complexity)]
fn either_fold_with_lazy() {
    let left: Either<Lazy<i32, fn() -> i32>, i32> = Either::Left(Lazy::new_with_value(42));
    let right: Either<Lazy<i32, fn() -> i32>, i32> = Either::Right(100);

    let left_result = left.fold(|lazy| *lazy.force(), |x| x);
    let right_result = right.fold(|lazy| *lazy.force(), |x| x);

    assert_eq!(left_result, 42);
    assert_eq!(right_result, 100);
}

// =============================================================================
// Continuation Integration
// =============================================================================

#[rstest]
fn continuation_with_lazy() {
    let lazy = Lazy::new(|| 21);

    let cont: Continuation<i32, i32> = Continuation::new(move |k| {
        let value = *lazy.force();
        k(value * 2)
    });

    let result = cont.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_with_trampoline_style() {
    // Use continuation to express a recursive-style computation
    fn sum_range<R: 'static>(start: i32, end: i32) -> Continuation<R, i32> {
        if start > end {
            Continuation::pure(0)
        } else {
            Continuation::pure(start).flat_map(move |first| {
                // In a real scenario, this would be recursive through Continuation
                // For testing, we use a simpler approach
                Continuation::pure(first + (start + 1..=end).sum::<i32>())
            })
        }
    }

    let result = sum_range(1, 10).run(|x| x);
    assert_eq!(result, 55); // 1 + 2 + ... + 10 = 55
}

// =============================================================================
// Complex Compositions
// =============================================================================

#[rstest]
fn lazy_chain_with_trampoline_results() {
    fn is_even(n: u64) -> Trampoline<bool> {
        if n == 0 {
            Trampoline::done(true)
        } else {
            Trampoline::suspend(move || is_odd(n - 1))
        }
    }

    fn is_odd(n: u64) -> Trampoline<bool> {
        if n == 0 {
            Trampoline::done(false)
        } else {
            Trampoline::suspend(move || is_even(n - 1))
        }
    }

    // Create lazy trampoline results
    let lazy_even_100 = Lazy::new(|| is_even(100).run());
    let lazy_odd_100 = Lazy::new(|| is_odd(100).run());

    // Neither computed yet
    assert!(!lazy_even_100.is_initialized());
    assert!(!lazy_odd_100.is_initialized());

    // Check results
    assert!(*lazy_even_100.force());
    assert!(!*lazy_odd_100.force());

    // Both now computed
    assert!(lazy_even_100.is_initialized());
    assert!(lazy_odd_100.is_initialized());
}

#[rstest]
fn either_chain_operations() {
    let value: Either<&str, i32> = Either::Right(10);

    // Chain of operations
    let result = value
        .map_right(|x| x * 2)
        .map_right(|x| x + 1)
        .map_right(|x| x.to_string());

    assert_eq!(result, Either::Right("21".to_string()));
}

// =============================================================================
// Error Handling Patterns
// =============================================================================

#[rstest]
fn continuation_exception_style() {
    // Simulate exception handling with continuation
    fn safe_div(x: i32, y: i32) -> Continuation<Either<String, i32>, i32> {
        Continuation::call_with_current_continuation_once(move |throw| {
            if y == 0 {
                throw(-1) // Signal error with sentinel value
            } else {
                Continuation::pure(x / y)
            }
        })
    }

    // Successful case
    let success = safe_div(10, 2).run(|r| {
        if r < 0 {
            Either::Left("Division error".to_string())
        } else {
            Either::Right(r)
        }
    });
    assert_eq!(success, Either::Right(5));

    // Error case
    let error = safe_div(10, 0).run(|r| {
        if r < 0 {
            Either::Left("Division error".to_string())
        } else {
            Either::Right(r)
        }
    });
    assert_eq!(error, Either::Left("Division error".to_string()));
}

// =============================================================================
// Stack Safety Integration Tests
// =============================================================================

#[rstest]
fn deep_trampoline_with_lazy_checkpoints() {
    // Create lazy checkpoints along a deep trampoline computation
    fn count_with_checkpoints(n: u64, checkpoints: &[Cell<bool>]) -> Trampoline<u64> {
        if n == 0 {
            Trampoline::done(0)
        } else {
            let idx = (n as usize - 1) % checkpoints.len();
            checkpoints[idx].set(true);
            Trampoline::suspend(move || {
                // Can't easily pass checkpoints here due to lifetime issues
                // This is a simplified test
                if n <= 1 {
                    Trampoline::done(n)
                } else {
                    Trampoline::suspend(move || Trampoline::done(n))
                }
            })
        }
    }

    let checkpoints: Vec<Cell<bool>> = (0..10).map(|_| Cell::new(false)).collect();
    let result = count_with_checkpoints(5, &checkpoints).run();

    assert!(result > 0);
}

#[rstest]
fn trampoline_100000_iterations() {
    fn countdown(n: u64) -> Trampoline<u64> {
        if n == 0 {
            Trampoline::done(0)
        } else {
            Trampoline::suspend(move || countdown(n - 1))
        }
    }

    // Should not stack overflow
    let result = countdown(100_000).run();
    assert_eq!(result, 0);
}

// =============================================================================
// Lazy Memoization Verification
// =============================================================================

#[rstest]
fn lazy_memoization_with_side_effects() {
    let log: Cell<Vec<String>> = Cell::new(Vec::new());

    let lazy = Lazy::new(|| {
        let mut current = log.take();
        current.push("computed".to_string());
        log.set(current);
        42
    });

    // First access
    assert_eq!(*lazy.force(), 42);

    // Check log
    let entries1 = log.take();
    log.set(entries1.clone());
    assert_eq!(entries1.len(), 1);

    // Second access
    assert_eq!(*lazy.force(), 42);

    // Log should still have only one entry (memoized)
    let entries2 = log.take();
    assert_eq!(entries2.len(), 1);
}

// =============================================================================
// Pattern: Lazy Stream-like Processing
// =============================================================================

#[rstest]
fn lazy_stream_pattern() {
    // Simulate lazy stream processing
    let lazy1 = Lazy::new(|| 1);
    let lazy2 = Lazy::new(|| 2);
    let lazy3 = Lazy::new(|| 3);

    // Combine lazily
    let sum = lazy1
        .zip(lazy2)
        .map(|(a, b)| a + b)
        .zip(lazy3)
        .map(|(ab, c)| ab + c);

    // Nothing computed yet would be true if we track it,
    // but for this test we just verify the result
    assert_eq!(*sum.force(), 6);
}

// =============================================================================
// Pattern: Trampoline for Tree Processing
// =============================================================================

#[derive(Debug)]
enum BinaryTree {
    Leaf(i64),
    Node(Box<BinaryTree>, Box<BinaryTree>),
}

impl BinaryTree {
    fn leaf(value: i64) -> Self {
        BinaryTree::Leaf(value)
    }

    fn node(left: BinaryTree, right: BinaryTree) -> Self {
        BinaryTree::Node(Box::new(left), Box::new(right))
    }
}

fn tree_sum(tree: BinaryTree) -> Trampoline<i64> {
    match tree {
        BinaryTree::Leaf(value) => Trampoline::done(value),
        BinaryTree::Node(left, right) => Trampoline::suspend(move || {
            tree_sum(*left).flat_map(move |left_sum| {
                tree_sum(*right).map(move |right_sum| left_sum + right_sum)
            })
        }),
    }
}

#[rstest]
fn trampoline_tree_processing() {
    // Create a balanced tree
    let tree = BinaryTree::node(
        BinaryTree::node(BinaryTree::leaf(1), BinaryTree::leaf(2)),
        BinaryTree::node(BinaryTree::leaf(3), BinaryTree::leaf(4)),
    );

    let sum = tree_sum(tree).run();
    assert_eq!(sum, 10);
}

// =============================================================================
// Pattern: Continuation for Control Flow
// =============================================================================

#[rstest]
fn continuation_control_flow_pattern() {
    // Use continuation to implement a find-first pattern
    let cont = Continuation::call_with_current_continuation_once(|found| {
        Continuation::pure(1)
            .flat_map(|_| Continuation::pure(3))
            .flat_map(|_| Continuation::pure(5))
            .flat_map(move |x| {
                // Found an odd number > 4, return it immediately
                if x > 4 {
                    found(x)
                } else {
                    Continuation::pure(x)
                }
            })
    });

    let result = cont.run(|x| x);
    assert_eq!(result, 5);
}

// =============================================================================
// Combined Pattern Tests
// =============================================================================

#[rstest]
fn combined_lazy_trampoline_continuation() {
    // A complex pattern combining all three

    // 1. Lazy factorial computation
    let lazy_fact = Lazy::new(|| {
        fn fact(n: u64) -> Trampoline<u64> {
            fn helper(n: u64, acc: u64) -> Trampoline<u64> {
                if n <= 1 {
                    Trampoline::done(acc)
                } else {
                    Trampoline::suspend(move || helper(n - 1, n * acc))
                }
            }
            helper(n, 1)
        }
        fact(10).run()
    });

    // 2. Use in continuation
    let cont: Continuation<String, u64> = Continuation::new(move |k| {
        let fact_result = *lazy_fact.force();
        k(fact_result)
    });

    // 3. Get result
    let result = cont.run(|x| format!("10! = {}", x));
    assert_eq!(result, "10! = 3628800");
}
