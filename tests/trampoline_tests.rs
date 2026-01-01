//! Unit tests for Trampoline<A> type.
//!
//! Tests cover:
//! - Basic trampoline operations (done, suspend)
//! - Recursive computations (factorial, fibonacci)
//! - Mutual recursion (is_even, is_odd)
//! - Stack safety with deep recursion
//! - map and flat_map operations
//! - resume for step-by-step evaluation
//! - Tree traversal patterns

#![cfg(feature = "control")]

use lambars::control::{Either, Trampoline};
use rstest::rstest;

// =============================================================================
// Basic Construction
// =============================================================================

#[rstest]
fn trampoline_done_returns_value() {
    let trampoline = Trampoline::done(42);
    assert_eq!(trampoline.run(), 42);
}

#[rstest]
fn trampoline_done_with_string() {
    let trampoline = Trampoline::done("hello".to_string());
    assert_eq!(trampoline.run(), "hello");
}

#[rstest]
fn trampoline_pure_is_alias_for_done() {
    let trampoline = Trampoline::pure(42);
    assert_eq!(trampoline.run(), 42);
}

#[rstest]
fn trampoline_suspend_delays_computation() {
    let trampoline = Trampoline::suspend(|| Trampoline::done(42));
    assert_eq!(trampoline.run(), 42);
}

#[rstest]
fn trampoline_nested_suspend() {
    let trampoline = Trampoline::suspend(|| {
        Trampoline::suspend(|| Trampoline::suspend(|| Trampoline::done(42)))
    });
    assert_eq!(trampoline.run(), 42);
}

// =============================================================================
// Factorial (Simple Recursion)
// =============================================================================

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

#[rstest]
#[case(0, 1)]
#[case(1, 1)]
#[case(2, 2)]
#[case(3, 6)]
#[case(4, 24)]
#[case(5, 120)]
#[case(10, 3_628_800)]
#[case(20, 2_432_902_008_176_640_000)]
fn trampoline_factorial(#[case] input: u64, #[case] expected: u64) {
    assert_eq!(factorial(input).run(), expected);
}

// =============================================================================
// Fibonacci (Tail Recursive Version)
// =============================================================================

fn fibonacci(n: u64) -> Trampoline<u64> {
    fibonacci_helper(n, 0, 1)
}

fn fibonacci_helper(n: u64, a: u64, b: u64) -> Trampoline<u64> {
    if n == 0 {
        Trampoline::done(a)
    } else {
        Trampoline::suspend(move || fibonacci_helper(n - 1, b, a + b))
    }
}

#[rstest]
#[case(0, 0)]
#[case(1, 1)]
#[case(2, 1)]
#[case(3, 2)]
#[case(4, 3)]
#[case(5, 5)]
#[case(10, 55)]
#[case(20, 6765)]
fn trampoline_fibonacci(#[case] input: u64, #[case] expected: u64) {
    assert_eq!(fibonacci(input).run(), expected);
}

// =============================================================================
// Mutual Recursion (is_even, is_odd)
// =============================================================================

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

#[rstest]
#[case(0, true)]
#[case(1, false)]
#[case(2, true)]
#[case(3, false)]
#[case(100, true)]
#[case(101, false)]
fn trampoline_is_even(#[case] input: u64, #[case] expected: bool) {
    assert_eq!(is_even(input).run(), expected);
}

#[rstest]
#[case(0, false)]
#[case(1, true)]
#[case(2, false)]
#[case(3, true)]
#[case(100, false)]
#[case(101, true)]
fn trampoline_is_odd(#[case] input: u64, #[case] expected: bool) {
    assert_eq!(is_odd(input).run(), expected);
}

// =============================================================================
// Stack Safety
// =============================================================================

#[rstest]
fn trampoline_stack_safety_100000() {
    fn count_down(n: u64) -> Trampoline<u64> {
        if n == 0 {
            Trampoline::done(0)
        } else {
            Trampoline::suspend(move || count_down(n - 1))
        }
    }

    // This would cause a stack overflow with regular recursion
    let result = count_down(100_000).run();
    assert_eq!(result, 0);
}

#[rstest]
fn trampoline_stack_safety_mutual_recursion_50000() {
    // Deep mutual recursion
    let result = is_even(50_000).run();
    assert!(result);
}

#[rstest]
fn trampoline_stack_safety_with_flat_map() {
    fn nested_flat_map(n: u64) -> Trampoline<u64> {
        if n == 0 {
            Trampoline::done(0)
        } else {
            Trampoline::suspend(move || nested_flat_map(n - 1))
                .flat_map(|x| Trampoline::done(x + 1))
        }
    }

    // Each step adds 1, so n steps should give n
    // Note: flat_map chains are more expensive than simple suspend chains
    let result = nested_flat_map(1_000).run();
    assert_eq!(result, 1_000);
}

// =============================================================================
// map
// =============================================================================

#[rstest]
fn trampoline_map_on_done() {
    let trampoline = Trampoline::done(21);
    let doubled = trampoline.map(|x| x * 2);
    assert_eq!(doubled.run(), 42);
}

#[rstest]
fn trampoline_map_on_suspend() {
    let trampoline = Trampoline::suspend(|| Trampoline::done(21));
    let doubled = trampoline.map(|x| x * 2);
    assert_eq!(doubled.run(), 42);
}

#[rstest]
fn trampoline_map_chain() {
    let trampoline = Trampoline::done(10);
    let result = trampoline.map(|x| x + 1).map(|x| x * 2).map(|x| x - 2);
    // (10 + 1) * 2 - 2 = 20
    assert_eq!(result.run(), 20);
}

#[rstest]
fn trampoline_map_type_change() {
    let trampoline = Trampoline::done(42);
    let stringified = trampoline.map(|x| x.to_string());
    assert_eq!(stringified.run(), "42");
}

// =============================================================================
// flat_map / and_then
// =============================================================================

#[rstest]
fn trampoline_flat_map_on_done() {
    let trampoline = Trampoline::done(21);
    let result = trampoline.flat_map(|x| Trampoline::done(x * 2));
    assert_eq!(result.run(), 42);
}

#[rstest]
fn trampoline_flat_map_on_suspend() {
    let trampoline = Trampoline::suspend(|| Trampoline::done(21));
    let result = trampoline.flat_map(|x| Trampoline::done(x * 2));
    assert_eq!(result.run(), 42);
}

#[rstest]
fn trampoline_flat_map_chain() {
    let trampoline = Trampoline::done(10);
    let result = trampoline
        .flat_map(|x| Trampoline::done(x + 1))
        .flat_map(|x| Trampoline::done(x * 2));
    // (10 + 1) * 2 = 22
    assert_eq!(result.run(), 22);
}

#[rstest]
fn trampoline_flat_map_with_suspend() {
    let trampoline = Trampoline::done(10);
    let result = trampoline
        .flat_map(|x| Trampoline::suspend(move || Trampoline::done(x + 1)))
        .flat_map(|x| Trampoline::suspend(move || Trampoline::done(x * 2)));
    // (10 + 1) * 2 = 22
    assert_eq!(result.run(), 22);
}

#[rstest]
fn trampoline_and_then_is_alias_for_flat_map() {
    let trampoline = Trampoline::done(21);
    let result = trampoline.and_then(|x| Trampoline::done(x * 2));
    assert_eq!(result.run(), 42);
}

// =============================================================================
// then
// =============================================================================

#[rstest]
fn trampoline_then_discards_first_result() {
    let first = Trampoline::done("ignored");
    let second = Trampoline::done(42);
    let result = first.then(second);
    assert_eq!(result.run(), 42);
}

#[rstest]
fn trampoline_then_with_suspend() {
    let first = Trampoline::suspend(|| Trampoline::done("ignored"));
    let second = Trampoline::suspend(|| Trampoline::done(42));
    let result = first.then(second);
    assert_eq!(result.run(), 42);
}

// =============================================================================
// resume
// =============================================================================

#[rstest]
fn trampoline_resume_done_returns_right() {
    let trampoline = Trampoline::done(42);
    match trampoline.resume() {
        Either::Right(value) => assert_eq!(value, 42),
        Either::Left(_) => panic!("Expected Right"),
    }
}

#[rstest]
fn trampoline_resume_suspend_returns_left() {
    let trampoline = Trampoline::suspend(|| Trampoline::done(42));
    match trampoline.resume() {
        Either::Left(_thunk) => { /* Expected */ }
        Either::Right(_) => panic!("Expected Left"),
    }
}

#[rstest]
fn trampoline_resume_step_by_step() {
    let trampoline = Trampoline::suspend(|| Trampoline::suspend(|| Trampoline::done(42)));

    // First resume - should get a thunk
    let first_resume = trampoline.resume();
    let thunk1 = match first_resume {
        Either::Left(thunk) => thunk,
        Either::Right(_) => panic!("Expected Left on first resume"),
    };

    // Second resume
    let second_resume = thunk1().resume();
    let thunk2 = match second_resume {
        Either::Left(thunk) => thunk,
        Either::Right(_) => panic!("Expected Left on second resume"),
    };

    // Third resume - should get the value
    let third_resume = thunk2().resume();
    match third_resume {
        Either::Right(value) => assert_eq!(value, 42),
        Either::Left(_) => panic!("Expected Right on third resume"),
    }
}

// =============================================================================
// Tree Traversal
// =============================================================================

#[derive(Debug, Clone)]
enum Tree<T> {
    Leaf(T),
    Node(Box<Tree<T>>, Box<Tree<T>>),
}

impl<T> Tree<T> {
    fn leaf(value: T) -> Self {
        Tree::Leaf(value)
    }

    fn node(left: Tree<T>, right: Tree<T>) -> Self {
        Tree::Node(Box::new(left), Box::new(right))
    }
}

fn tree_sum(tree: Tree<i64>) -> Trampoline<i64> {
    match tree {
        Tree::Leaf(value) => Trampoline::done(value),
        Tree::Node(left, right) => Trampoline::suspend(move || {
            tree_sum(*left).flat_map(move |left_sum| {
                tree_sum(*right).map(move |right_sum| left_sum + right_sum)
            })
        }),
    }
}

#[rstest]
fn trampoline_tree_sum_leaf() {
    let tree = Tree::leaf(42);
    assert_eq!(tree_sum(tree).run(), 42);
}

#[rstest]
fn trampoline_tree_sum_simple_node() {
    let tree = Tree::node(Tree::leaf(10), Tree::leaf(20));
    assert_eq!(tree_sum(tree).run(), 30);
}

#[rstest]
fn trampoline_tree_sum_nested() {
    let tree = Tree::node(
        Tree::node(Tree::leaf(1), Tree::leaf(2)),
        Tree::node(Tree::leaf(3), Tree::leaf(4)),
    );
    assert_eq!(tree_sum(tree).run(), 10);
}

fn tree_count<T: 'static>(tree: Tree<T>) -> Trampoline<usize> {
    match tree {
        Tree::Leaf(_) => Trampoline::done(1),
        Tree::Node(left, right) => Trampoline::suspend(move || {
            tree_count(*left).flat_map(move |left_count| {
                tree_count(*right).map(move |right_count| left_count + right_count)
            })
        }),
    }
}

#[rstest]
fn trampoline_tree_count() {
    let tree = Tree::node(
        Tree::node(Tree::leaf(1), Tree::leaf(2)),
        Tree::node(Tree::leaf(3), Tree::node(Tree::leaf(4), Tree::leaf(5))),
    );
    assert_eq!(tree_count(tree).run(), 5);
}

// =============================================================================
// Debug
// =============================================================================

#[rstest]
fn trampoline_debug_done() {
    let trampoline = Trampoline::done(42);
    let debug_str = format!("{:?}", trampoline);
    assert!(debug_str.contains("Done"));
    assert!(debug_str.contains("42"));
}

#[rstest]
fn trampoline_debug_suspend() {
    let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(42));
    let debug_str = format!("{:?}", trampoline);
    assert!(debug_str.contains("Suspend"));
}

// =============================================================================
// Complex Scenarios
// =============================================================================

#[rstest]
fn trampoline_complex_computation() {
    // Compute sum of 1 to n using trampoline
    fn sum_to(n: u64) -> Trampoline<u64> {
        sum_to_helper(n, 0)
    }

    fn sum_to_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
        if n == 0 {
            Trampoline::done(accumulator)
        } else {
            Trampoline::suspend(move || sum_to_helper(n - 1, accumulator + n))
        }
    }

    // Sum of 1 to 100 = 100 * 101 / 2 = 5050
    assert_eq!(sum_to(100).run(), 5050);
}

#[rstest]
fn trampoline_ackermann_small() {
    // Ackermann function is extremely recursive, good test for stack safety
    // We use a small value to keep the test reasonable
    fn ackermann(m: u64, n: u64) -> Trampoline<u64> {
        if m == 0 {
            Trampoline::done(n + 1)
        } else if n == 0 {
            Trampoline::suspend(move || ackermann(m - 1, 1))
        } else {
            Trampoline::suspend(move || {
                ackermann(m, n - 1).flat_map(move |inner| ackermann(m - 1, inner))
            })
        }
    }

    // A(2, 2) = 7
    assert_eq!(ackermann(2, 2).run(), 7);
    // A(3, 2) = 29
    assert_eq!(ackermann(3, 2).run(), 29);
}
