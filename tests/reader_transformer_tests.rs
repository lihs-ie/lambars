#![cfg(feature = "effect")]
//! Tests for ReaderT (Reader Transformer).
//!
//! ReaderT adds environment reading capability to any monad.

use lambars::effect::{IO, ReaderT};
use rstest::rstest;

// =============================================================================
// Basic Structure Tests
// =============================================================================

#[rstest]
fn reader_transformer_new_and_run_with_option() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment * 2));
    let result = reader_transformer.run(21);
    assert_eq!(result, Some(42));
}

#[rstest]
fn reader_transformer_new_and_run_with_result() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|environment: i32| Ok(environment * 2));
    let result = reader_transformer.run(21);
    assert_eq!(result, Ok(42));
}

#[rstest]
fn reader_transformer_run_returns_none_when_inner_is_none() {
    let reader_transformer: ReaderT<i32, Option<i32>> = ReaderT::new(|_environment: i32| None);
    let result = reader_transformer.run(42);
    assert_eq!(result, None);
}

// =============================================================================
// pure Tests
// =============================================================================

#[rstest]
fn reader_transformer_pure_with_option() {
    let reader_transformer: ReaderT<i32, Option<i32>> = ReaderT::pure_option(42);
    let result = reader_transformer.run(999); // environment is ignored
    assert_eq!(result, Some(42));
}

#[rstest]
fn reader_transformer_pure_with_result() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> = ReaderT::pure_result(42);
    let result = reader_transformer.run(999);
    assert_eq!(result, Ok(42));
}

// =============================================================================
// lift Tests
// =============================================================================

#[rstest]
fn reader_transformer_lift_option() {
    let inner: Option<i32> = Some(42);
    let reader_transformer: ReaderT<String, Option<i32>> = ReaderT::lift_option(inner);
    let result = reader_transformer.run("ignored".to_string());
    assert_eq!(result, Some(42));
}

#[rstest]
fn reader_transformer_lift_option_none() {
    let inner: Option<i32> = None;
    let reader_transformer: ReaderT<String, Option<i32>> = ReaderT::lift_option(inner);
    let result = reader_transformer.run("ignored".to_string());
    assert_eq!(result, None);
}

#[rstest]
fn reader_transformer_lift_result() {
    let inner: Result<i32, String> = Ok(42);
    let reader_transformer: ReaderT<i32, Result<i32, String>> = ReaderT::lift_result(inner);
    let result = reader_transformer.run(999);
    assert_eq!(result, Ok(42));
}

#[rstest]
fn reader_transformer_lift_result_error() {
    let inner: Result<i32, String> = Err("error".to_string());
    let reader_transformer: ReaderT<i32, Result<i32, String>> = ReaderT::lift_result(inner);
    let result = reader_transformer.run(999);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// fmap (Functor) Tests
// =============================================================================

#[rstest]
fn reader_transformer_fmap_option_some() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment));
    let mapped = reader_transformer.fmap_option(|value| value * 2);
    let result = mapped.run(21);
    assert_eq!(result, Some(42));
}

#[rstest]
fn reader_transformer_fmap_option_none() {
    let reader_transformer: ReaderT<i32, Option<i32>> = ReaderT::new(|_environment: i32| None);
    let mapped = reader_transformer.fmap_option(|value| value * 2);
    let result = mapped.run(21);
    assert_eq!(result, None);
}

#[rstest]
fn reader_transformer_fmap_result_ok() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|environment: i32| Ok(environment));
    let mapped = reader_transformer.fmap_result(|value| value * 2);
    let result = mapped.run(21);
    assert_eq!(result, Ok(42));
}

#[rstest]
fn reader_transformer_fmap_result_error() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|_environment: i32| Err("error".to_string()));
    let mapped = reader_transformer.fmap_result(|value| value * 2);
    let result = mapped.run(21);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// flat_map (Monad) Tests
// =============================================================================

#[rstest]
fn reader_transformer_flat_map_option_some_to_some() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment));

    let chained = reader_transformer
        .flat_map_option(|value| ReaderT::new(move |environment: i32| Some(value + environment)));

    // environment = 10: first returns Some(10), then 10 + 10 = 20
    let result = chained.run(10);
    assert_eq!(result, Some(20));
}

#[rstest]
fn reader_transformer_flat_map_option_some_to_none() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment));

    let chained =
        reader_transformer.flat_map_option(|_value| ReaderT::new(|_environment: i32| None::<i32>));

    let result: Option<i32> = chained.run(10);
    assert_eq!(result, None);
}

#[rstest]
fn reader_transformer_flat_map_option_none_short_circuits() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|_environment: i32| None::<i32>);

    let chained = reader_transformer
        .flat_map_option(|value| ReaderT::new(move |environment: i32| Some(value + environment)));

    let result: Option<i32> = chained.run(10);
    assert_eq!(result, None);
}

#[rstest]
fn reader_transformer_flat_map_result_ok_to_ok() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|environment: i32| Ok(environment));

    let chained = reader_transformer
        .flat_map_result(|value| ReaderT::new(move |environment: i32| Ok(value + environment)));

    let result = chained.run(10);
    assert_eq!(result, Ok(20));
}

#[rstest]
fn reader_transformer_flat_map_result_ok_to_error() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|environment: i32| Ok(environment));

    let chained = reader_transformer.flat_map_result(|_value| {
        ReaderT::new(|_environment: i32| Err::<i32, String>("error".to_string()))
    });

    let result: Result<i32, String> = chained.run(10);
    assert_eq!(result, Err("error".to_string()));
}

#[rstest]
fn reader_transformer_flat_map_result_error_short_circuits() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|_environment: i32| Err::<i32, String>("error".to_string()));

    let chained = reader_transformer
        .flat_map_result(|value| ReaderT::new(move |environment: i32| Ok(value + environment)));

    let result: Result<i32, String> = chained.run(10);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// ask (MonadReader) Tests
// =============================================================================

#[rstest]
fn reader_transformer_ask_option() {
    let reader_transformer: ReaderT<i32, Option<i32>> = ReaderT::ask_option();
    let result = reader_transformer.run(42);
    assert_eq!(result, Some(42));
}

#[rstest]
fn reader_transformer_ask_result() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> = ReaderT::ask_result();
    let result = reader_transformer.run(42);
    assert_eq!(result, Ok(42));
}

// =============================================================================
// local (MonadReader) Tests
// =============================================================================

#[rstest]
fn reader_transformer_local_option() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment * 2));

    let modified = ReaderT::local_option(|environment| environment + 10, reader_transformer);

    // environment = 5, modified to 15, then * 2 = 30
    let result = modified.run(5);
    assert_eq!(result, Some(30));
}

#[rstest]
fn reader_transformer_local_result() {
    let reader_transformer: ReaderT<i32, Result<i32, String>> =
        ReaderT::new(|environment: i32| Ok(environment * 2));

    let modified = ReaderT::local_result(|environment| environment + 10, reader_transformer);

    let result = modified.run(5);
    assert_eq!(result, Ok(30));
}

// =============================================================================
// ReaderT with IO Tests
// =============================================================================

#[rstest]
fn reader_transformer_with_io_basic() {
    let reader_transformer: ReaderT<i32, IO<i32>> =
        ReaderT::new(|environment: i32| IO::pure(environment * 2));

    let io_result = reader_transformer.run(21);
    let result = io_result.run_unsafe();
    assert_eq!(result, 42);
}

#[rstest]
fn reader_transformer_lift_io() {
    let inner = IO::pure(42);
    let reader_transformer: ReaderT<String, IO<i32>> = ReaderT::lift_io(inner);

    let io_result = reader_transformer.run("ignored".to_string());
    let result = io_result.run_unsafe();
    assert_eq!(result, 42);
}

#[rstest]
fn reader_transformer_fmap_io() {
    let reader_transformer: ReaderT<i32, IO<i32>> =
        ReaderT::new(|environment: i32| IO::pure(environment));

    let mapped = reader_transformer.fmap_io(|value| value * 2);

    let io_result = mapped.run(21);
    let result = io_result.run_unsafe();
    assert_eq!(result, 42);
}

#[rstest]
fn reader_transformer_flat_map_io() {
    let reader_transformer: ReaderT<i32, IO<i32>> =
        ReaderT::new(|environment: i32| IO::pure(environment));

    let chained = reader_transformer
        .flat_map_io(|value| ReaderT::new(move |environment: i32| IO::pure(value + environment)));

    let io_result = chained.run(10);
    let result = io_result.run_unsafe();
    assert_eq!(result, 20);
}

#[rstest]
fn reader_transformer_ask_io() {
    let reader_transformer: ReaderT<i32, IO<i32>> = ReaderT::ask_io();

    let io_result = reader_transformer.run(42);
    let result = io_result.run_unsafe();
    assert_eq!(result, 42);
}

// =============================================================================
// Clone Tests
// =============================================================================

#[rstest]
fn reader_transformer_clone() {
    let reader_transformer: ReaderT<i32, Option<i32>> =
        ReaderT::new(|environment: i32| Some(environment * 2));
    let cloned = reader_transformer.clone();

    assert_eq!(reader_transformer.run(21), Some(42));
    assert_eq!(cloned.run(21), Some(42));
}

// =============================================================================
// Practical Examples
// =============================================================================

#[rstest]
fn reader_transformer_config_example() {
    #[derive(Clone)]
    struct Config {
        database_url: String,
        port: u16,
    }

    fn get_database_url() -> ReaderT<Config, Option<String>> {
        ReaderT::new(|config: Config| Some(config.database_url))
    }

    fn get_port() -> ReaderT<Config, Option<u16>> {
        ReaderT::new(|config: Config| Some(config.port))
    }

    fn get_connection_string() -> ReaderT<Config, Option<String>> {
        get_database_url()
            .flat_map_option(|url| get_port().fmap_option(move |port| format!("{}:{}", url, port)))
    }

    let config = Config {
        database_url: "localhost".to_string(),
        port: 5432,
    };

    let result = get_connection_string().run(config);
    assert_eq!(result, Some("localhost:5432".to_string()));
}

#[rstest]
fn reader_transformer_nested_readers() {
    // Test chaining multiple reader transformers
    let computation = ReaderT::<i32, Option<i32>>::ask_option()
        .flat_map_option(|environment| ReaderT::new(move |_: i32| Some(environment * 2)))
        .flat_map_option(|value| {
            ReaderT::<i32, Option<i32>>::ask_option()
                .fmap_option(move |environment: i32| value + environment)
        });

    // environment = 10
    // ask returns 10
    // map to 20
    // ask again returns 10, result = 20 + 10 = 30
    let result = computation.run(10);
    assert_eq!(result, Some(30));
}
