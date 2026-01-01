#![cfg(feature = "effect")]
//! Tests for the eff! macro (do-notation style syntax for monads).
//!
//! The eff! macro provides a convenient syntax for chaining monadic operations,
//! similar to Haskell's do-notation or Scala's for-comprehension.
//!
//! # Syntax
//!
//! - `pattern <= expression;` - Bind operation (flat_map)
//! - `let pattern = expression;` - Pure let binding
//! - `expression` - Final expression (must be a monad)

#![allow(clippy::unwrap_used)]
#![allow(clippy::let_unit_value)]

use lambars::eff;
use lambars::effect::{Reader, State, Writer};
use lambars::typeclass::Monad;

// =============================================================================
// Option Tests
// =============================================================================

mod option_tests {
    use super::*;

    #[test]
    fn option_basic_flat_map_chain() {
        let result = eff! {
            x <= Some(5);
            y <= Some(10);
            Some(x + y)
        };
        assert_eq!(result, Some(15));
    }

    #[test]
    fn option_with_let_binding() {
        let result = eff! {
            x <= Some(5);
            y <= Some(10);
            let z = x + y;
            Some(z * 2)
        };
        assert_eq!(result, Some(30));
    }

    #[test]
    fn option_short_circuit_on_none() {
        let result: Option<i32> = eff! {
            x <= Some(5);
            y <= None::<i32>;
            Some(x + y)
        };
        assert_eq!(result, None);
    }

    #[test]
    fn option_short_circuit_early() {
        let result: Option<i32> = eff! {
            x <= None::<i32>;
            y <= Some(10);
            Some(x + y)
        };
        assert_eq!(result, None);
    }

    #[test]
    fn option_single_bind() {
        let result = eff! {
            x <= Some(42);
            Some(x)
        };
        assert_eq!(result, Some(42));
    }

    #[test]
    fn option_multiple_let_bindings() {
        let result = eff! {
            x <= Some(2);
            let a = x * 3;
            let b = a + 1;
            y <= Some(10);
            let c = b * y;
            Some(c)
        };
        // x = 2, a = 6, b = 7, y = 10, c = 70
        assert_eq!(result, Some(70));
    }

    #[test]
    fn option_expression_only() {
        let result = eff! {
            x <= Some(5);
            y <= Some(10);
            Some(x + y)
        };
        assert_eq!(result, Some(15));
    }

    #[test]
    fn option_tuple_pattern() {
        let result = eff! {
            (a, b) <= Some((1, 2));
            Some(a + b)
        };
        assert_eq!(result, Some(3));
    }

    #[test]
    fn option_conditional_computation() {
        let result = eff! {
            x <= Some(5);
            y <= if x > 3 { Some(x * 2) } else { None };
            Some(y)
        };
        assert_eq!(result, Some(10));
    }

    #[test]
    fn option_conditional_fails() {
        let result: Option<i32> = eff! {
            x <= Some(2);
            y <= if x > 3 { Some(x * 2) } else { None };
            Some(y)
        };
        assert_eq!(result, None);
    }
}

// =============================================================================
// Result Tests
// =============================================================================

mod result_tests {
    use super::*;

    #[test]
    fn result_basic_flat_map_chain() {
        let result: Result<i32, &str> = eff! {
            x <= Ok(5);
            y <= Ok(10);
            Ok(x + y)
        };
        assert_eq!(result, Ok(15));
    }

    #[test]
    fn result_with_error() {
        let result: Result<i32, &str> = eff! {
            x <= Ok(5);
            y <= Err::<i32, _>("error occurred");
            Ok(x + y)
        };
        assert_eq!(result, Err("error occurred"));
    }

    #[test]
    fn result_early_error() {
        let result: Result<i32, &str> = eff! {
            x <= Err::<i32, _>("early error");
            y <= Ok(10);
            Ok(x + y)
        };
        assert_eq!(result, Err("early error"));
    }

    #[test]
    fn result_with_validation() {
        fn validate_positive(n: i32) -> Result<i32, &'static str> {
            if n > 0 {
                Ok(n)
            } else {
                Err("must be positive")
            }
        }

        let result: Result<i32, &str> = eff! {
            x <= validate_positive(5);
            y <= validate_positive(10);
            Ok(x + y)
        };
        assert_eq!(result, Ok(15));
    }

    #[test]
    fn result_validation_fails() {
        fn validate_positive(n: i32) -> Result<i32, &'static str> {
            if n > 0 {
                Ok(n)
            } else {
                Err("must be positive")
            }
        }

        let result: Result<i32, &str> = eff! {
            x <= validate_positive(5);
            y <= validate_positive(-3);
            Ok(x + y)
        };
        assert_eq!(result, Err("must be positive"));
    }
}

// =============================================================================
// State Tests
// =============================================================================

mod state_tests {
    use super::*;

    #[test]
    fn state_get_and_pure() {
        let computation: State<i32, i32> = eff! {
            current <= State::get();
            State::pure(current)
        };
        let (result, final_state) = computation.run(42);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[test]
    fn state_modify_and_get() {
        let computation: State<i32, i32> = eff! {
            _ <= State::modify(|x| x + 1);
            new_state <= State::get();
            State::pure(new_state)
        };
        let (result, final_state) = computation.run(0);
        assert_eq!(result, 1);
        assert_eq!(final_state, 1);
    }

    #[test]
    fn state_counter_example() {
        fn increment() -> State<i32, ()> {
            State::modify(|x| x + 1)
        }

        fn get_count() -> State<i32, i32> {
            State::get()
        }

        let computation: State<i32, i32> = eff! {
            _ <= increment();
            _ <= increment();
            _ <= increment();
            count <= get_count();
            State::pure(count)
        };
        let (result, final_state) = computation.run(0);
        assert_eq!(result, 3);
        assert_eq!(final_state, 3);
    }

    #[test]
    fn state_put_and_get() {
        let computation: State<i32, i32> = eff! {
            _ <= State::put(100);
            new_state <= State::get();
            State::pure(new_state)
        };
        let (result, final_state) = computation.run(0);
        assert_eq!(result, 100);
        assert_eq!(final_state, 100);
    }

    #[test]
    fn state_complex_computation() {
        let computation: State<i32, i32> = eff! {
            initial <= State::get();
            _ <= State::modify(|x| x * 2);
            doubled <= State::get();
            let sum = initial + doubled;
            _ <= State::put(sum);
            final_val <= State::get();
            State::pure(final_val)
        };
        // initial = 5, doubled = 10, sum = 15, final_val = 15
        let (result, final_state) = computation.run(5);
        assert_eq!(result, 15);
        assert_eq!(final_state, 15);
    }
}

// =============================================================================
// Reader Tests
// =============================================================================

mod reader_tests {
    use super::*;

    #[test]
    fn reader_ask_and_pure() {
        let computation: Reader<i32, i32> = eff! {
            environment <= Reader::ask();
            Reader::pure(environment * 2)
        };
        assert_eq!(computation.run(21), 42);
    }

    #[test]
    fn reader_asks_projection() {
        #[derive(Clone)]
        struct Config {
            port: u16,
            host: String,
        }

        let computation: Reader<Config, String> = eff! {
            port <= Reader::asks(|c: Config| c.port);
            host <= Reader::asks(|c: Config| c.host);
            Reader::pure(format!("{}:{}", host, port))
        };

        let config = Config {
            port: 8080,
            host: "localhost".to_string(),
        };
        assert_eq!(computation.run(config), "localhost:8080");
    }

    #[test]
    fn reader_with_let_binding() {
        let computation: Reader<i32, i32> = eff! {
            environment <= Reader::ask();
            let doubled = environment * 2;
            let tripled = environment * 3;
            Reader::pure(doubled + tripled)
        };
        // environment = 10, doubled = 20, tripled = 30, result = 50
        assert_eq!(computation.run(10), 50);
    }

    #[test]
    fn reader_chained_asks() {
        let computation: Reader<i32, i32> = eff! {
            a <= Reader::asks(|x: i32| x + 1);
            b <= Reader::asks(|x: i32| x * 2);
            Reader::pure(a + b)
        };
        // x = 10, a = 11, b = 20, result = 31
        assert_eq!(computation.run(10), 31);
    }
}

// =============================================================================
// Writer Tests
// =============================================================================

mod writer_tests {
    use super::*;

    #[test]
    fn writer_tell_and_pure() {
        let computation: Writer<Vec<String>, i32> = eff! {
            _ <= Writer::tell(vec!["log message".to_string()]);
            Writer::pure(42)
        };
        let (result, logs) = computation.run();
        assert_eq!(result, 42);
        assert_eq!(logs, vec!["log message"]);
    }

    #[test]
    fn writer_multiple_tells() {
        fn log(message: &str) -> Writer<Vec<String>, ()> {
            Writer::tell(vec![message.to_string()])
        }

        let computation: Writer<Vec<String>, i32> = eff! {
            _ <= log("step 1");
            _ <= log("step 2");
            _ <= log("step 3");
            Writer::pure(42)
        };
        let (result, logs) = computation.run();
        assert_eq!(result, 42);
        assert_eq!(logs, vec!["step 1", "step 2", "step 3"]);
    }

    #[test]
    fn writer_computation_with_logging() {
        fn log(message: &str) -> Writer<Vec<String>, ()> {
            Writer::tell(vec![message.to_string()])
        }

        fn compute_with_log(n: i32) -> Writer<Vec<String>, i32> {
            Writer::new(n * 2, vec![format!("computed: {}", n * 2)])
        }

        let computation: Writer<Vec<String>, i32> = eff! {
            _ <= log("starting");
            x <= compute_with_log(5);
            _ <= log("finished");
            Writer::pure(x)
        };
        let (result, logs) = computation.run();
        assert_eq!(result, 10);
        assert_eq!(logs, vec!["starting", "computed: 10", "finished"]);
    }

    #[test]
    fn writer_with_let_binding() {
        fn log(message: &str) -> Writer<Vec<String>, ()> {
            Writer::tell(vec![message.to_string()])
        }

        let computation: Writer<Vec<String>, i32> = eff! {
            x <= Writer::pure(5);
            let doubled = x * 2;
            _ <= log(&format!("doubled: {}", doubled));
            Writer::pure(doubled)
        };
        let (result, logs) = computation.run();
        assert_eq!(result, 10);
        assert_eq!(logs, vec!["doubled: 10"]);
    }
}

// =============================================================================
// Complex Composition Tests
// =============================================================================

mod complex_tests {
    use super::*;

    #[test]
    fn option_nested_structure() {
        let outer = Some((1, Some(2)));
        let result = eff! {
            (a, inner) <= outer;
            b <= inner;
            Some(a + b)
        };
        assert_eq!(result, Some(3));
    }

    #[test]
    fn option_with_closures() {
        let double = |x: i32| Some(x * 2);
        let add_one = |x: i32| Some(x + 1);

        let result = eff! {
            x <= Some(5);
            y <= double(x);
            z <= add_one(y);
            Some(z)
        };
        assert_eq!(result, Some(11)); // 5 -> 10 -> 11
    }

    #[test]
    fn result_with_different_error_types_same_type() {
        fn parse_int(s: &str) -> Result<i32, String> {
            s.parse::<i32>().map_err(|e| e.to_string())
        }

        fn validate_range(n: i32) -> Result<i32, String> {
            if (0..=100).contains(&n) {
                Ok(n)
            } else {
                Err("out of range".to_string())
            }
        }

        let result: Result<i32, String> = eff! {
            x <= parse_int("42");
            y <= validate_range(x);
            Ok(y * 2)
        };
        assert_eq!(result, Ok(84));
    }

    #[test]
    fn deeply_nested_computation() {
        let result = eff! {
            a <= Some(1);
            b <= Some(2);
            c <= Some(3);
            d <= Some(4);
            e <= Some(5);
            let sum = a + b + c + d + e;
            Some(sum)
        };
        assert_eq!(result, Some(15));
    }

    #[test]
    fn mixed_operations() {
        let result = eff! {
            x <= Some(10);
            let doubled = x * 2;
            y <= Some(doubled);
            let tripled = y * 3;
            z <= Some(tripled);
            Some(z)
        };
        // x = 10, doubled = 20, y = 20, tripled = 60, z = 60
        assert_eq!(result, Some(60));
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn single_expression_only() {
        let result = eff! {
            Some(42)
        };
        assert_eq!(result, Some(42));
    }

    #[test]
    fn wildcard_pattern() {
        let result = eff! {
            _ <= Some(5);
            Some(42)
        };
        assert_eq!(result, Some(42));
    }

    #[test]
    fn unit_type_handling() {
        let result: Option<()> = eff! {
            x <= Some(5);
            _ <= Some(x + 1);
            Some(())
        };
        assert_eq!(result, Some(()));
    }

    #[test]
    fn string_handling() {
        let result = eff! {
            s <= Some("hello".to_string());
            Some(format!("{} world", s))
        };
        assert_eq!(result, Some("hello world".to_string()));
    }
}
