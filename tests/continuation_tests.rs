//! Unit tests for Continuation<R, A> type.
//!
//! Tests cover:
//! - Basic continuation operations (new, pure, run)
//! - map and flat_map operations
//! - call_with_current_continuation_once (call/cc)
//! - Early return patterns
//! - Complex compositions

#![cfg(feature = "control")]

use lambars::control::Continuation;
use rstest::rstest;

// =============================================================================
// Basic Construction
// =============================================================================

#[rstest]
fn continuation_pure_and_run() {
    let cont: Continuation<i32, i32> = Continuation::pure(42);
    let result = cont.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_pure_with_different_types() {
    let cont: Continuation<String, i32> = Continuation::pure(42);
    let result = cont.run(|x| x.to_string());
    assert_eq!(result, "42");
}

#[rstest]
fn continuation_new_basic() {
    // Create a continuation that passes 42 to its continuation
    let cont: Continuation<String, i32> = Continuation::new(|k| k(42));
    let result = cont.run(|x| x.to_string());
    assert_eq!(result, "42");
}

#[rstest]
fn continuation_new_with_computation() {
    // Create a continuation that does some computation before passing to k
    let cont: Continuation<i32, i32> = Continuation::new(|k| {
        let intermediate = 21 * 2; // 42
        k(intermediate)
    });
    let result = cont.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_new_ignoring_continuation() {
    // A continuation that ignores its continuation and returns directly
    let cont: Continuation<i32, i32> = Continuation::new(|_k| 100);
    let result = cont.run(|x| x * 2);
    // The continuation is ignored, so the result is 100, not 100 * 2
    assert_eq!(result, 100);
}

// =============================================================================
// map
// =============================================================================

#[rstest]
fn continuation_map_basic() {
    let cont: Continuation<i32, i32> = Continuation::pure(21);
    let doubled = cont.map(|x| x * 2);
    let result = doubled.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_map_chain() {
    let cont: Continuation<i32, i32> = Continuation::pure(10);
    let result = cont.map(|x| x + 1).map(|x| x * 2).map(|x| x - 2);
    // (10 + 1) * 2 - 2 = 20
    assert_eq!(result.run(|x| x), 20);
}

#[rstest]
fn continuation_map_type_change() {
    let cont: Continuation<String, i32> = Continuation::pure(42);
    let stringified = cont.map(|x| x.to_string());
    let result = stringified.run(|s| s);
    assert_eq!(result, "42");
}

#[rstest]
fn continuation_map_with_closure() {
    let factor = 3;
    let cont: Continuation<i32, i32> = Continuation::pure(14);
    let result = cont.map(move |x| x * factor).run(|x| x);
    assert_eq!(result, 42);
}

// =============================================================================
// flat_map / and_then
// =============================================================================

#[rstest]
fn continuation_flat_map_basic() {
    let cont: Continuation<i32, i32> = Continuation::pure(21);
    let result = cont.flat_map(|x| Continuation::pure(x * 2));
    assert_eq!(result.run(|x| x), 42);
}

#[rstest]
fn continuation_flat_map_chain() {
    let cont: Continuation<i32, i32> = Continuation::pure(10);
    let result = cont
        .flat_map(|x| Continuation::pure(x + 1))
        .flat_map(|x| Continuation::pure(x * 2));
    // (10 + 1) * 2 = 22
    assert_eq!(result.run(|x| x), 22);
}

#[rstest]
fn continuation_flat_map_with_different_result_type() {
    let cont: Continuation<String, i32> = Continuation::pure(42);
    let result = cont.flat_map(|x| Continuation::pure(format!("value: {}", x)));
    assert_eq!(result.run(|s| s), "value: 42");
}

#[rstest]
fn continuation_and_then_is_alias_for_flat_map() {
    let cont: Continuation<i32, i32> = Continuation::pure(21);
    let result = cont.and_then(|x| Continuation::pure(x * 2));
    assert_eq!(result.run(|x| x), 42);
}

// =============================================================================
// then
// =============================================================================

#[rstest]
fn continuation_then_discards_first_result() {
    let first: Continuation<i32, &str> = Continuation::pure("ignored");
    let second: Continuation<i32, i32> = Continuation::pure(42);
    let result = first.then(second);
    assert_eq!(result.run(|x| x), 42);
}

#[rstest]
fn continuation_then_sequences_effects() {
    use std::cell::Cell;

    let first_called = Cell::new(false);
    let second_called = Cell::new(false);

    let first: Continuation<i32, ()> = Continuation::new(move |k| {
        first_called.set(true);
        k(())
    });

    let second: Continuation<i32, i32> = Continuation::new(move |k| {
        second_called.set(true);
        k(42)
    });

    let result = first.then(second).run(|x| x);

    assert_eq!(result, 42);
    // Both should have been called when run is executed
}

// =============================================================================
// call_with_current_continuation_once (call/cc)
// =============================================================================

#[rstest]
fn continuation_call_cc_without_exit() {
    let cont = Continuation::call_with_current_continuation_once(|_exit| Continuation::pure(42));
    let result = cont.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_call_cc_immediate_exit() {
    let cont: Continuation<i32, i32> =
        Continuation::call_with_current_continuation_once(|exit| exit(42));
    let result = cont.run(|x| x);
    assert_eq!(result, 42);
}

#[rstest]
fn continuation_call_cc_conditional_exit_not_triggered() {
    let cont = Continuation::call_with_current_continuation_once(|exit| {
        Continuation::pure(1).flat_map(move |x| {
            if x > 10 {
                exit(x * 100)
            } else {
                Continuation::pure(x + 5)
            }
        })
    });

    // x = 1, which is not > 10, so returns 1 + 5 = 6
    let result = cont.run(|x| x);
    assert_eq!(result, 6);
}

#[rstest]
fn continuation_call_cc_conditional_exit_triggered() {
    let cont = Continuation::call_with_current_continuation_once(|exit| {
        Continuation::pure(20).flat_map(move |x| {
            if x > 10 {
                exit(x * 100)
            } else {
                Continuation::pure(x + 5)
            }
        })
    });

    // x = 20, which is > 10, so exit(20 * 100) = 2000
    let result = cont.run(|x| x);
    assert_eq!(result, 2000);
}

#[rstest]
fn continuation_call_cc_early_return_pattern() {
    // Simulate early return: find first element > 5, or return 0
    let cont = Continuation::call_with_current_continuation_once(|exit| {
        Continuation::pure(3)
            .flat_map(|_| Continuation::pure(4))
            .flat_map(|_| Continuation::pure(7))
            .flat_map(move |x| {
                if x > 5 {
                    exit(x) // Found it! Return early
                } else {
                    Continuation::pure(x)
                }
            })
            .flat_map(|_| Continuation::pure(0)) // This would normally run, but exit short-circuits
    });

    let result = cont.run(|x| x);
    assert_eq!(result, 7);
}

// =============================================================================
// Complex Compositions
// =============================================================================

#[rstest]
fn continuation_complex_composition() {
    let result: i32 = Continuation::pure(10)
        .flat_map(|x| Continuation::pure(x + 5))
        .flat_map(|x| Continuation::pure(x * 2))
        .map(|x| x + 1)
        .run(|x| x);

    // (10 + 5) * 2 + 1 = 31
    assert_eq!(result, 31);
}

#[rstest]
fn continuation_nested_call_cc() {
    let cont = Continuation::call_with_current_continuation_once(|outer_exit| {
        Continuation::call_with_current_continuation_once(move |inner_exit| {
            Continuation::pure(5).flat_map(move |x| {
                if x > 10 {
                    outer_exit(x * 10)
                } else if x > 3 {
                    inner_exit(x * 2)
                } else {
                    Continuation::pure(x)
                }
            })
        })
    });

    // x = 5, which is > 3 but not > 10, so inner_exit(5 * 2) = 10
    let result = cont.run(|x| x);
    assert_eq!(result, 10);
}

#[rstest]
fn continuation_with_final_transformation() {
    let cont: Continuation<String, i32> = Continuation::pure(42);
    let result = cont.map(|x| x * 2).run(|x| format!("The answer is {}", x));

    assert_eq!(result, "The answer is 84");
}

// =============================================================================
// Different R Types
// =============================================================================

#[rstest]
fn continuation_with_string_result() {
    let cont: Continuation<String, i32> = Continuation::pure(42);
    let result = cont.run(|x| format!("value: {}", x));
    assert_eq!(result, "value: 42");
}

#[rstest]
fn continuation_with_vec_result() {
    let cont: Continuation<Vec<i32>, i32> = Continuation::pure(42);
    let result = cont.run(|x| vec![x, x * 2, x * 3]);
    assert_eq!(result, vec![42, 84, 126]);
}

#[rstest]
fn continuation_with_option_result() {
    let cont: Continuation<Option<i32>, i32> = Continuation::pure(42);
    let result = cont.run(Some);
    assert_eq!(result, Some(42));
}

// =============================================================================
// Debug
// =============================================================================

#[rstest]
fn continuation_debug() {
    let cont: Continuation<i32, i32> = Continuation::pure(42);
    let debug_str = format!("{:?}", cont);
    assert!(debug_str.contains("Continuation"));
}

// =============================================================================
// CPS Style Patterns
// =============================================================================

#[rstest]
fn continuation_cps_style_computation() {
    // CPS-style computation that transforms a value
    fn add_cps<R>(x: i32, y: i32) -> Continuation<R, i32>
    where
        R: 'static,
    {
        Continuation::pure(x + y)
    }

    fn mul_cps<R>(x: i32, y: i32) -> Continuation<R, i32>
    where
        R: 'static,
    {
        Continuation::pure(x * y)
    }

    // (3 + 4) * 2 = 14
    let result: i32 = add_cps(3, 4).flat_map(|sum| mul_cps(sum, 2)).run(|x| x);

    assert_eq!(result, 14);
}

#[rstest]
fn continuation_exception_like_pattern() {
    // Simulate try-catch using call/cc
    fn safe_divide(x: i32, y: i32) -> Continuation<Result<i32, String>, i32> {
        Continuation::call_with_current_continuation_once(move |throw| {
            if y == 0 {
                throw(-1) // Signal error
            } else {
                Continuation::pure(x / y)
            }
        })
    }

    // Successful division
    let result = safe_divide(10, 2).run(|x| {
        if x < 0 {
            Err("Division by zero".to_string())
        } else {
            Ok(x)
        }
    });
    assert_eq!(result, Ok(5));

    // Division by zero
    let result = safe_divide(10, 0).run(|x| {
        if x < 0 {
            Err("Division by zero".to_string())
        } else {
            Ok(x)
        }
    });
    assert_eq!(result, Err("Division by zero".to_string()));
}
