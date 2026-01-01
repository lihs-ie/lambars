//! Unit tests for Reader Monad.
//!
//! Tests basic functionality of the Reader monad including:
//! - Creation and execution
//! - Functor operations (fmap)
//! - Monad operations (flat_map, pure)
//! - Reader-specific operations (ask, asks, local)

use lambars::effect::Reader;
use rstest::rstest;

// =============================================================================
// Basic Construction and Execution Tests
// =============================================================================

#[rstest]
fn reader_new_and_run_basic() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    assert_eq!(reader.run(21), 42);
}

#[rstest]
fn reader_new_and_run_with_string_environment() {
    let reader: Reader<String, usize> = Reader::new(|environment: String| environment.len());
    assert_eq!(reader.run("hello".to_string()), 5);
}

#[rstest]
fn reader_new_and_run_with_struct_environment() {
    #[derive(Clone)]
    struct Config {
        port: u16,
        host: String,
    }

    let reader: Reader<Config, String> =
        Reader::new(|config: Config| format!("{}:{}", config.host, config.port));

    let config = Config {
        port: 8080,
        host: "localhost".to_string(),
    };

    assert_eq!(reader.run(config), "localhost:8080");
}

// =============================================================================
// Pure Tests
// =============================================================================

#[rstest]
fn reader_pure_creates_constant_reader() {
    let reader: Reader<i32, &str> = Reader::pure("constant");
    assert_eq!(reader.run(42), "constant");
    assert_eq!(reader.run(0), "constant");
}

#[rstest]
fn reader_pure_ignores_environment() {
    let reader: Reader<String, i32> = Reader::pure(100);
    assert_eq!(reader.run("any environment".to_string()), 100);
}

// =============================================================================
// Functor (fmap) Tests
// =============================================================================

#[rstest]
fn reader_fmap_transforms_result() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let mapped = reader.fmap(|value| value * 2);
    assert_eq!(mapped.run(21), 42);
}

#[rstest]
fn reader_fmap_changes_type() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let mapped = reader.fmap(|value| value.to_string());
    assert_eq!(mapped.run(42), "42");
}

#[rstest]
fn reader_fmap_chained() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let mapped = reader
        .fmap(|value| value + 1)
        .fmap(|value| value * 2)
        .fmap(|value| value.to_string());
    assert_eq!(mapped.run(5), "12"); // (5 + 1) * 2 = 12
}

// =============================================================================
// Applicative Tests
// =============================================================================

#[rstest]
fn reader_map2_combines_two_readers() {
    let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    let combined = reader1.map2(reader2, |a, b| a + b);
    assert_eq!(combined.run(10), 30); // 10 + 20
}

#[rstest]
fn reader_map3_combines_three_readers() {
    let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    let reader3: Reader<i32, i32> = Reader::new(|environment| environment * 3);
    let combined = reader1.map3(reader2, reader3, |a, b, c| a + b + c);
    assert_eq!(combined.run(10), 60); // 10 + 20 + 30
}

#[rstest]
fn reader_product_creates_tuple() {
    let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    let reader2: Reader<i32, &str> = Reader::pure("hello");
    let product = reader1.product(reader2);
    assert_eq!(product.run(42), (42, "hello"));
}

// =============================================================================
// Monad (flat_map) Tests
// =============================================================================

#[rstest]
fn reader_flat_map_chains_readers() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let chained = reader.flat_map(|value| Reader::new(move |environment| value + environment));
    assert_eq!(chained.run(10), 20); // 10 + 10
}

#[rstest]
fn reader_flat_map_with_pure() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let chained = reader.flat_map(|value| Reader::pure(value * 2));
    assert_eq!(chained.run(21), 42);
}

#[rstest]
fn reader_and_then_is_alias_for_flat_map() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    let chained = reader.and_then(|value| Reader::new(move |environment| value + environment));
    assert_eq!(chained.run(10), 20);
}

#[rstest]
fn reader_then_sequences_and_discards_first() {
    let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    let reader2: Reader<i32, &str> = Reader::pure("result");
    let sequenced = reader1.then(reader2);
    assert_eq!(sequenced.run(42), "result");
}

// =============================================================================
// MonadReader - ask Tests
// =============================================================================

#[rstest]
fn reader_ask_returns_environment() {
    let reader: Reader<i32, i32> = Reader::ask();
    assert_eq!(reader.run(42), 42);
}

#[rstest]
fn reader_ask_with_struct_environment() {
    #[derive(Clone, PartialEq, Debug)]
    struct Config {
        value: i32,
    }

    let reader: Reader<Config, Config> = Reader::ask();
    let config = Config { value: 100 };
    assert_eq!(reader.run(config.clone()), config);
}

// =============================================================================
// MonadReader - asks Tests
// =============================================================================

#[rstest]
fn reader_asks_projects_from_environment() {
    #[derive(Clone)]
    struct Config {
        port: u16,
        host: String,
    }

    let port_reader: Reader<Config, u16> = Reader::asks(|config: Config| config.port);
    let config = Config {
        port: 8080,
        host: "localhost".to_string(),
    };
    assert_eq!(port_reader.run(config), 8080);
}

#[rstest]
fn reader_asks_with_transformation() {
    let reader: Reader<i32, String> =
        Reader::asks(|environment: i32| format!("value: {}", environment));
    assert_eq!(reader.run(42), "value: 42");
}

// =============================================================================
// MonadReader - local Tests
// =============================================================================

#[rstest]
fn reader_local_modifies_environment() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    let local_reader = Reader::local(|environment| environment + 10, reader);
    assert_eq!(local_reader.run(5), 30); // (5 + 10) * 2
}

#[rstest]
fn reader_local_nested() {
    let reader: Reader<i32, i32> = Reader::ask();
    let inner = Reader::local(|environment| environment * 2, reader);
    let outer = Reader::local(|environment| environment + 10, inner);
    assert_eq!(outer.run(5), 30); // (5 + 10) * 2
}

#[rstest]
fn reader_local_with_flat_map() {
    let reader: Reader<i32, i32> = Reader::ask();
    let modified = Reader::local(|environment| environment * 2, reader);
    let chained = modified.flat_map(|value| Reader::new(move |environment| value + environment));
    assert_eq!(chained.run(10), 30); // (10 * 2) + 10 = 30
}

// =============================================================================
// Complex Use Cases
// =============================================================================

#[rstest]
fn reader_dependency_injection_pattern() {
    #[derive(Clone)]
    struct Database {
        connection_string: String,
    }

    #[derive(Clone)]
    struct AppConfig {
        database: Database,
        debug_mode: bool,
    }

    fn get_connection_string() -> Reader<AppConfig, String> {
        Reader::asks(|config: AppConfig| config.database.connection_string)
    }

    fn is_debug_mode() -> Reader<AppConfig, bool> {
        Reader::asks(|config: AppConfig| config.debug_mode)
    }

    fn get_status() -> Reader<AppConfig, String> {
        get_connection_string().map2(is_debug_mode(), |connection_string, debug| {
            format!("DB: {}, Debug: {}", connection_string, debug)
        })
    }

    let config = AppConfig {
        database: Database {
            connection_string: "postgres://localhost".to_string(),
        },
        debug_mode: true,
    };

    assert_eq!(
        get_status().run(config),
        "DB: postgres://localhost, Debug: true"
    );
}

#[rstest]
fn reader_multiple_flat_map_chains() {
    let computation: Reader<i32, i32> = Reader::ask()
        .flat_map(|a| Reader::ask().fmap(move |b| a + b))
        .flat_map(|sum| Reader::pure(sum * 2));

    assert_eq!(computation.run(5), 20); // (5 + 5) * 2
}

#[rstest]
fn reader_with_local_and_flat_map() {
    let inner: Reader<i32, i32> = Reader::ask();
    let doubled = Reader::local(|environment| environment * 2, inner);

    let computation = Reader::ask()
        .flat_map(move |original| doubled.clone().fmap(move |modified| original + modified));

    assert_eq!(computation.run(10), 30); // 10 + (10 * 2)
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
fn reader_with_unit_environment() {
    let reader: Reader<(), i32> = Reader::pure(42);
    assert_eq!(reader.run(()), 42);
}

#[rstest]
fn reader_with_unit_result() {
    let reader: Reader<i32, ()> = Reader::new(|_| ());
    assert_eq!(reader.run(42), ());
}

#[rstest]
fn reader_with_reference_types() {
    // This test ensures Reader works with Clone types that contain owned data
    let reader: Reader<Vec<i32>, usize> = Reader::new(|environment: Vec<i32>| environment.len());
    assert_eq!(reader.run(vec![1, 2, 3]), 3);
}
