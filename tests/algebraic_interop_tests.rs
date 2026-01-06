//! Integration tests for algebraic effects interoperability.
//!
//! These tests verify the interoperability between the algebraic effects
//! system and existing effect representations (Reader monad, MTL-style traits).

#![cfg(feature = "effect")]

use lambars::EffectRow;
use lambars::effect::Reader;
use lambars::effect::algebraic::interop::{ask, asks, get, gets, modify, put, tell, throw_error};
use lambars::effect::algebraic::{
    Eff, EffCons, EffNil, ErrorEffect, ErrorHandler, Handler, Here, IntoEff, Member, ReaderEffect,
    ReaderHandler, StateEffect, StateHandler, There, WriterEffect, WriterHandler,
};
use rstest::rstest;

// Type alias for single effect rows (required for Member trait to work)
type ReaderRow<R> = EffCons<ReaderEffect<R>, EffNil>;
type StateRow<S> = EffCons<StateEffect<S>, EffNil>;
type WriterRow<W> = EffCons<WriterEffect<W>, EffNil>;
type ErrorRow<E> = EffCons<ErrorEffect<E>, EffNil>;

// =============================================================================
// IntoEff Trait Tests
// =============================================================================

mod into_eff_tests {
    use super::*;

    #[rstest]
    fn reader_into_eff_basic() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let eff = reader.into_eff();

        let result = ReaderHandler::new(21).run(eff);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn reader_into_eff_with_string_environment() {
        let reader: Reader<String, usize> = Reader::asks(|s: String| s.len());
        let eff = reader.into_eff();

        let result = ReaderHandler::new("hello world".to_string()).run(eff);
        assert_eq!(result, 11);
    }

    #[rstest]
    fn reader_into_eff_pure_value() {
        let reader: Reader<i32, &str> = Reader::pure("constant");
        let eff = reader.into_eff();

        let result = ReaderHandler::new(999).run(eff);
        assert_eq!(result, "constant");
    }

    #[rstest]
    fn reader_into_eff_chained_computation() {
        let reader: Reader<i32, i32> = Reader::ask().flat_map(|x| Reader::new(move |y| x + y + 5));
        let eff = reader.into_eff();

        let result = ReaderHandler::new(10).run(eff);
        assert_eq!(result, 25); // 10 + 10 + 5
    }

    #[rstest]
    fn reader_into_eff_local_environment() {
        let inner = Reader::ask();
        let reader = Reader::local(|x: i32| x * 2, inner);
        let eff = reader.into_eff();

        let result = ReaderHandler::new(21).run(eff);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn reader_into_eff_complex_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct Config {
            port: u16,
            host: String,
        }

        let reader: Reader<Config, String> =
            Reader::asks(|config: Config| format!("{}:{}", config.host, config.port));
        let eff = reader.into_eff();

        let config = Config {
            port: 8080,
            host: "localhost".to_string(),
        };
        let result = ReaderHandler::new(config).run(eff);
        assert_eq!(result, "localhost:8080");
    }
}

// =============================================================================
// MTL-Style Reader Operations Tests
// =============================================================================

mod mtl_reader_tests {
    use super::*;

    #[rstest]
    fn ask_returns_environment() {
        type Row = ReaderRow<i32>;
        let computation: Eff<Row, i32> = ask::<i32, Row, Here>();

        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new(42).run(projected);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn asks_projects_environment() {
        type Row = ReaderRow<String>;
        let computation: Eff<Row, usize> = asks::<String, usize, Row, Here, _>(|s: String| s.len());

        let projected = <Row as Member<ReaderEffect<String>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new("hello".to_string()).run(projected);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn ask_in_effect_row() {
        type Row = EffectRow![ReaderEffect<i32>, StateEffect<String>];

        let computation: Eff<Row, i32> = ask::<i32, Row, Here>();

        // Project and run
        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new(100).run(projected);
        assert_eq!(result, 100);
    }

    #[rstest]
    fn ask_at_second_position_in_row() {
        type Row = EffectRow![StateEffect<String>, ReaderEffect<i32>];

        let computation: Eff<Row, i32> = ask::<i32, Row, There<Here>>();

        let projected =
            <Row as Member<ReaderEffect<i32>, There<Here>>>::project(computation).unwrap();
        let result = ReaderHandler::new(50).run(projected);
        assert_eq!(result, 50);
    }

    #[rstest]
    fn asks_with_struct_projection() {
        #[derive(Clone)]
        struct AppConfig {
            max_connections: u32,
            #[allow(dead_code)]
            timeout_seconds: u64,
        }

        type Row = ReaderRow<AppConfig>;
        let computation: Eff<Row, u32> =
            asks::<AppConfig, u32, Row, Here, _>(|config: AppConfig| config.max_connections);

        let config = AppConfig {
            max_connections: 100,
            timeout_seconds: 30,
        };
        let projected =
            <Row as Member<ReaderEffect<AppConfig>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new(config).run(projected);
        assert_eq!(result, 100);
    }
}

// =============================================================================
// MTL-Style State Operations Tests
// =============================================================================

mod mtl_state_tests {
    use super::*;

    #[rstest]
    fn get_returns_current_state() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, i32> = get::<i32, Row, Here>();

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(42).run(projected);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn put_updates_state() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, ()> = put::<i32, Row, Here>(100);

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(0).run(projected);
        assert_eq!(final_state, 100);
    }

    #[rstest]
    fn modify_transforms_state() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, ()> = modify::<i32, Row, Here, _>(|x: i32| x * 2);

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(21).run(projected);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn gets_projects_state() {
        type Row = StateRow<Vec<i32>>;
        let computation: Eff<Row, usize> =
            gets::<Vec<i32>, usize, Row, Here, _>(|v: &Vec<i32>| v.len());

        let projected = <Row as Member<StateEffect<Vec<i32>>, Here>>::project(computation).unwrap();
        let (result, _) = StateHandler::new(vec![1, 2, 3, 4, 5]).run(projected);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn state_operations_in_effect_row() {
        type Row = EffectRow![ReaderEffect<String>, StateEffect<i32>];

        let computation: Eff<Row, ()> = put::<i32, Row, There<Here>>(42);

        let projected =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(0).run(projected);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_get_put_sequence() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, i32> = get::<i32, Row, Here>()
            .flat_map(|x| put::<i32, Row, Here>(x + 10))
            .then(get::<i32, Row, Here>());

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(5).run(projected);
        assert_eq!(result, 15);
        assert_eq!(final_state, 15);
    }

    #[rstest]
    fn state_counter_pattern() {
        type Row = StateRow<i32>;
        let increment = || modify::<i32, Row, Here, _>(|x| x + 1);

        let computation: Eff<Row, i32> = increment()
            .then(increment())
            .then(increment())
            .then(get::<i32, Row, Here>());

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(0).run(projected);
        assert_eq!(result, 3);
        assert_eq!(final_state, 3);
    }
}

// =============================================================================
// MTL-Style Writer Operations Tests
// =============================================================================

mod mtl_writer_tests {
    use super::*;

    #[rstest]
    fn tell_appends_to_log() {
        type Row = WriterRow<String>;
        let computation: Eff<Row, ()> = tell::<String, Row, Here>("hello".to_string());

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, "hello");
    }

    #[rstest]
    fn tell_multiple_messages() {
        type Row = WriterRow<String>;
        let computation: Eff<Row, ()> = tell::<String, Row, Here>("a".to_string())
            .then(tell::<String, Row, Here>("b".to_string()))
            .then(tell::<String, Row, Here>("c".to_string()));

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, "abc");
    }

    #[rstest]
    fn tell_with_vec_monoid() {
        type Row = WriterRow<Vec<String>>;
        let computation: Eff<Row, ()> = tell::<Vec<String>, Row, Here>(vec!["step1".to_string()])
            .then(tell::<Vec<String>, Row, Here>(vec!["step2".to_string()]))
            .then(tell::<Vec<String>, Row, Here>(vec!["step3".to_string()]));

        let projected =
            <Row as Member<WriterEffect<Vec<String>>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(
            log,
            vec![
                "step1".to_string(),
                "step2".to_string(),
                "step3".to_string()
            ]
        );
    }

    #[rstest]
    fn tell_in_effect_row() {
        type Row = EffectRow![ReaderEffect<i32>, WriterEffect<String>];

        let computation: Eff<Row, ()> = tell::<String, Row, There<Here>>("message".to_string());

        let projected =
            <Row as Member<WriterEffect<String>, There<Here>>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, "message");
    }

    #[rstest]
    fn tell_with_computation_result() {
        type Row = WriterRow<String>;
        let computation: Eff<Row, i32> =
            tell::<String, Row, Here>("logging".to_string()).then(Eff::pure(42));

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let (result, log) = WriterHandler::new().run(projected);
        assert_eq!(result, 42);
        assert_eq!(log, "logging");
    }
}

// =============================================================================
// MTL-Style Error Operations Tests
// =============================================================================

mod mtl_error_tests {
    use super::*;

    #[rstest]
    fn throw_error_creates_error() {
        type Row = ErrorRow<String>;
        let computation: Eff<Row, i32> = throw_error::<String, i32, Row, Here>("error".to_string());

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("error".to_string()));
    }

    #[rstest]
    fn throw_error_short_circuits() {
        type Row = ErrorRow<String>;
        let computation: Eff<Row, i32> =
            throw_error::<String, i32, Row, Here>("early".to_string()).fmap(|x| x + 100);

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("early".to_string()));
    }

    #[rstest]
    fn throw_error_in_effect_row() {
        type Row = EffectRow![ReaderEffect<i32>, ErrorEffect<String>];

        let computation: Eff<Row, i32> =
            throw_error::<String, i32, Row, There<Here>>("oops".to_string());

        let projected =
            <Row as Member<ErrorEffect<String>, There<Here>>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("oops".to_string()));
    }

    #[rstest]
    fn pure_value_is_ok() {
        type Row = ErrorRow<String>;
        let computation: Eff<Row, i32> = Eff::pure(42);

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Ok(42));
    }
}

// =============================================================================
// Combined Effect Row Tests
// =============================================================================

mod combined_effect_tests {
    use super::*;

    #[rstest]
    fn reader_and_state_in_same_row() {
        type Row = EffectRow![ReaderEffect<i32>, StateEffect<i32>];

        // Use reader
        let read_computation: Eff<Row, i32> = ask::<i32, Row, Here>();

        // Use state
        let state_computation: Eff<Row, i32> = get::<i32, Row, There<Here>>();

        // Verify reader works
        let reader_projected =
            <Row as Member<ReaderEffect<i32>, Here>>::project(read_computation).unwrap();
        let reader_result = ReaderHandler::new(10).run(reader_projected);
        assert_eq!(reader_result, 10);

        // Verify state works
        let state_projected =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(state_computation).unwrap();
        let (state_result, _) = StateHandler::new(20).run(state_projected);
        assert_eq!(state_result, 20);
    }

    #[rstest]
    fn three_effect_row_operations() {
        type Row = EffectRow![
            ReaderEffect<i32>,
            StateEffect<String>,
            WriterEffect<Vec<i32>>
        ];

        // Reader at Here
        let _read: Eff<Row, i32> = ask::<i32, Row, Here>();

        // State at There<Here>
        let _state_get: Eff<Row, String> = get::<String, Row, There<Here>>();

        // Writer at There<There<Here>>
        let _write: Eff<Row, ()> = tell::<Vec<i32>, Row, There<There<Here>>>(vec![1, 2, 3]);

        // If this compiles, the types work correctly
    }

    #[rstest]
    fn four_effect_row_operations() {
        type Row = EffectRow![
            ReaderEffect<i32>,
            StateEffect<String>,
            WriterEffect<Vec<i32>>,
            ErrorEffect<String>
        ];

        // All four effects can be used
        let _read: Eff<Row, i32> = ask::<i32, Row, Here>();
        let _state: Eff<Row, String> = get::<String, Row, There<Here>>();
        let _write: Eff<Row, ()> = tell::<Vec<i32>, Row, There<There<Here>>>(vec![1]);
        let _error: Eff<Row, i32> =
            throw_error::<String, i32, Row, There<There<There<Here>>>>("error".to_string());
    }
}

// =============================================================================
// Interoperability Scenario Tests
// =============================================================================

mod scenario_tests {
    use super::*;

    #[rstest]
    fn convert_existing_reader_to_eff() {
        // Existing Reader-based code
        fn get_multiplied_value() -> Reader<i32, i32> {
            Reader::ask().flat_map(|x| Reader::new(move |y| x * y))
        }

        // Convert to Eff
        let reader = get_multiplied_value();
        let eff = reader.into_eff();

        // Run with handler
        let result = ReaderHandler::new(7).run(eff);
        assert_eq!(result, 49); // 7 * 7
    }

    #[rstest]
    fn mtl_style_computation_with_eff() {
        type Row = StateRow<i32>;

        // Create an Eff computation using MTL-style operations
        let computation: Eff<Row, i32> = get::<i32, Row, Here>().flat_map(|x| {
            modify::<i32, Row, Here, _>(move |s| s + x).then(get::<i32, Row, Here>())
        });

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(10).run(projected);
        assert_eq!(result, 20); // 10 + 10
        assert_eq!(final_state, 20);
    }

    #[rstest]
    fn logging_computation() {
        type Row = WriterRow<Vec<String>>;

        fn log(message: &str) -> Eff<Row, ()> {
            tell::<Vec<String>, Row, Here>(vec![message.to_string()])
        }

        let computation: Eff<Row, i32> = log("Starting")
            .then(Eff::pure(10))
            .flat_map(|x| log("Processing").fmap(move |_| x * 2))
            .flat_map(|x| log("Finishing").fmap(move |_| x));

        let projected =
            <Row as Member<WriterEffect<Vec<String>>, Here>>::project(computation).unwrap();
        let (result, log_output) = WriterHandler::new().run(projected);
        assert_eq!(result, 20);
        assert_eq!(
            log_output,
            vec![
                "Starting".to_string(),
                "Processing".to_string(),
                "Finishing".to_string()
            ]
        );
    }

    #[rstest]
    fn validation_with_error_effect() {
        type Row = ErrorRow<String>;

        fn validate_positive(x: i32) -> Eff<Row, i32> {
            if x > 0 {
                Eff::pure(x)
            } else {
                throw_error::<String, i32, Row, Here>("Value must be positive".to_string())
            }
        }

        fn validate_less_than_100(x: i32) -> Eff<Row, i32> {
            if x < 100 {
                Eff::pure(x)
            } else {
                throw_error::<String, i32, Row, Here>("Value must be less than 100".to_string())
            }
        }

        // Valid value
        let valid_computation = validate_positive(50).flat_map(validate_less_than_100);
        let projected =
            <Row as Member<ErrorEffect<String>, Here>>::project(valid_computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Ok(50));

        // Invalid: not positive
        let invalid_positive = validate_positive(-5).flat_map(validate_less_than_100);
        let projected =
            <Row as Member<ErrorEffect<String>, Here>>::project(invalid_positive).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("Value must be positive".to_string()));

        // Invalid: too large
        let invalid_large = validate_positive(150).flat_map(validate_less_than_100);
        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(invalid_large).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("Value must be less than 100".to_string()));
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[rstest]
    fn empty_string_in_reader() {
        let reader: Reader<String, usize> = Reader::asks(|s: String| s.len());
        let eff = reader.into_eff();

        let result = ReaderHandler::new(String::new()).run(eff);
        assert_eq!(result, 0);
    }

    #[rstest]
    fn zero_state() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, i32> = get::<i32, Row, Here>()
            .flat_map(|x| put::<i32, Row, Here>(x + 1))
            .then(get::<i32, Row, Here>());

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(0).run(projected);
        assert_eq!(result, 1);
        assert_eq!(final_state, 1);
    }

    #[rstest]
    fn empty_log() {
        type Row = WriterRow<String>;
        let computation: Eff<Row, i32> = Eff::pure(42);

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let (result, log) = WriterHandler::new().run(projected);
        assert_eq!(result, 42);
        assert_eq!(log, "");
    }

    #[rstest]
    fn deeply_nested_fmap() {
        type Row = ReaderRow<i32>;
        let computation: Eff<Row, i32> = ask::<i32, Row, Here>();
        let mapped = computation
            .fmap(|x| x + 1)
            .fmap(|x| x + 1)
            .fmap(|x| x + 1)
            .fmap(|x| x + 1)
            .fmap(|x| x + 1);

        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(mapped).unwrap();
        let result = ReaderHandler::new(0).run(projected);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn deeply_nested_flat_map() {
        type Row = StateRow<i32>;
        let computation: Eff<Row, i32> = get::<i32, Row, Here>()
            .flat_map(|_| modify::<i32, Row, Here, _>(|x| x + 1).then(get::<i32, Row, Here>()))
            .flat_map(|_| modify::<i32, Row, Here, _>(|x| x + 1).then(get::<i32, Row, Here>()))
            .flat_map(|_| modify::<i32, Row, Here, _>(|x| x + 1).then(get::<i32, Row, Here>()));

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(0).run(projected);
        assert_eq!(result, 3);
        assert_eq!(final_state, 3);
    }
}
