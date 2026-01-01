//! Unit tests for Writer Monad.
//!
//! Tests basic functionality of the Writer monad including:
//! - Creation and execution (run, eval, exec)
//! - Functor operations (fmap)
//! - Monad operations (flat_map, pure)
//! - Writer-specific operations (tell, listen, pass, censor)

use lambars::effect::Writer;
use rstest::rstest;

// =============================================================================
// Basic Construction and Execution Tests
// =============================================================================

#[rstest]
fn writer_new_and_run_basic() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["initial log".to_string()]);
    let (result, output) = writer.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["initial log"]);
}

#[rstest]
fn writer_eval_returns_result() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
    let result = writer.eval();
    assert_eq!(result, 42);
}

#[rstest]
fn writer_exec_returns_output() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
    let output = writer.exec();
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_new_with_string_output() {
    let writer: Writer<String, i32> = Writer::new(42, "log message".to_string());
    let (result, output) = writer.run();
    assert_eq!(result, 42);
    assert_eq!(output, "log message");
}

// =============================================================================
// Pure Tests
// =============================================================================

#[rstest]
fn writer_pure_creates_empty_output() {
    let writer: Writer<Vec<String>, i32> = Writer::pure(42);
    let (result, output) = writer.run();
    assert_eq!(result, 42);
    assert!(output.is_empty());
}

#[rstest]
fn writer_pure_with_string_output() {
    let writer: Writer<String, i32> = Writer::pure(42);
    let (result, output) = writer.run();
    assert_eq!(result, 42);
    assert!(output.is_empty());
}

// =============================================================================
// Functor (fmap) Tests
// =============================================================================

#[rstest]
fn writer_fmap_transforms_result() {
    let writer: Writer<Vec<String>, i32> = Writer::new(21, vec!["log".to_string()]);
    let mapped = writer.fmap(|value| value * 2);
    let (result, output) = mapped.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_fmap_changes_type() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
    let mapped = writer.fmap(|value| value.to_string());
    let (result, output) = mapped.run();
    assert_eq!(result, "42");
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_fmap_chained() {
    let writer: Writer<Vec<String>, i32> = Writer::new(5, vec!["log".to_string()]);
    let mapped = writer
        .fmap(|value| value + 1)
        .fmap(|value| value * 2)
        .fmap(|value| value.to_string());
    let (result, output) = mapped.run();
    assert_eq!(result, "12"); // (5 + 1) * 2 = 12
    assert_eq!(output, vec!["log"]);
}

// =============================================================================
// Applicative Tests
// =============================================================================

#[rstest]
fn writer_map2_combines_outputs() {
    let writer1: Writer<Vec<String>, i32> = Writer::new(10, vec!["first".to_string()]);
    let writer2: Writer<Vec<String>, i32> = Writer::new(20, vec!["second".to_string()]);
    let combined = writer1.map2(writer2, |a, b| a + b);
    let (result, output) = combined.run();
    assert_eq!(result, 30);
    assert_eq!(output, vec!["first", "second"]);
}

#[rstest]
fn writer_product_creates_tuple() {
    let writer1: Writer<Vec<String>, i32> = Writer::new(42, vec!["first".to_string()]);
    let writer2: Writer<Vec<String>, &str> = Writer::new("hello", vec!["second".to_string()]);
    let product = writer1.product(writer2);
    let ((first, second), output) = product.run();
    assert_eq!(first, 42);
    assert_eq!(second, "hello");
    assert_eq!(output, vec!["first", "second"]);
}

// =============================================================================
// Monad (flat_map) Tests
// =============================================================================

#[rstest]
fn writer_flat_map_chains_writers() {
    let writer: Writer<Vec<String>, i32> = Writer::new(10, vec!["first".to_string()]);
    let chained = writer.flat_map(|value| Writer::new(value * 2, vec!["second".to_string()]));
    let (result, output) = chained.run();
    assert_eq!(result, 20);
    assert_eq!(output, vec!["first", "second"]);
}

#[rstest]
fn writer_flat_map_with_pure() {
    let writer: Writer<Vec<String>, i32> = Writer::new(21, vec!["log".to_string()]);
    let chained = writer.flat_map(|value| Writer::pure(value * 2));
    let (result, output) = chained.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_and_then_is_alias_for_flat_map() {
    let writer: Writer<Vec<String>, i32> = Writer::new(10, vec!["first".to_string()]);
    let chained = writer.and_then(|value| Writer::new(value + 5, vec!["second".to_string()]));
    let (result, output) = chained.run();
    assert_eq!(result, 15);
    assert_eq!(output, vec!["first", "second"]);
}

#[rstest]
fn writer_then_sequences_and_discards_first() {
    let writer1: Writer<Vec<String>, i32> = Writer::new(42, vec!["first".to_string()]);
    let writer2: Writer<Vec<String>, &str> = Writer::new("result", vec!["second".to_string()]);
    let sequenced = writer1.then(writer2);
    let (result, output) = sequenced.run();
    assert_eq!(result, "result");
    assert_eq!(output, vec!["first", "second"]);
}

// =============================================================================
// MonadWriter - tell Tests
// =============================================================================

#[rstest]
fn writer_tell_appends_output() {
    let writer: Writer<Vec<String>, ()> = Writer::tell(vec!["message".to_string()]);
    let (result, output) = writer.run();
    assert_eq!(result, ());
    assert_eq!(output, vec!["message"]);
}

#[rstest]
fn writer_tell_with_string() {
    let writer: Writer<String, ()> = Writer::tell("hello world".to_string());
    let (result, output) = writer.run();
    assert_eq!(result, ());
    assert_eq!(output, "hello world");
}

#[rstest]
fn writer_tell_chained() {
    let writer = Writer::tell(vec!["first".to_string()])
        .then(Writer::tell(vec!["second".to_string()]))
        .then(Writer::tell(vec!["third".to_string()]));
    let (_, output) = writer.run();
    assert_eq!(output, vec!["first", "second", "third"]);
}

// =============================================================================
// MonadWriter - listen Tests
// =============================================================================

#[rstest]
fn writer_listen_captures_output() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
    let listened = Writer::listen(writer);
    let ((result, captured), output) = listened.run();
    assert_eq!(result, 42);
    assert_eq!(captured, vec!["log"]);
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_listen_with_tell() {
    let writer = Writer::tell(vec!["message".to_string()]).fmap(|_| 42);
    let listened = Writer::listen(writer);
    let ((result, captured), output) = listened.run();
    assert_eq!(result, 42);
    assert_eq!(captured, vec!["message"]);
    assert_eq!(output, vec!["message"]);
}

#[rstest]
fn writer_listen_with_chained_writers() {
    let writer = Writer::tell(vec!["first".to_string()])
        .then(Writer::tell(vec!["second".to_string()]))
        .fmap(|_| 42);
    let listened = Writer::listen(writer);
    let ((result, captured), output) = listened.run();
    assert_eq!(result, 42);
    assert_eq!(captured, vec!["first", "second"]);
    assert_eq!(output, vec!["first", "second"]);
}

// =============================================================================
// MonadWriter - pass Tests
// =============================================================================

#[rstest]
fn writer_pass_transforms_output() {
    let writer: Writer<Vec<String>, (i32, fn(Vec<String>) -> Vec<String>)> = Writer::new(
        (
            42,
            (|output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect())
                as fn(Vec<String>) -> Vec<String>,
        ),
        vec!["hello".to_string()],
    );
    let passed = Writer::pass(writer);
    let (result, output) = passed.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["HELLO"]);
}

#[rstest]
fn writer_pass_with_identity() {
    let writer: Writer<Vec<String>, (i32, fn(Vec<String>) -> Vec<String>)> = Writer::new(
        (
            42,
            (|output: Vec<String>| output) as fn(Vec<String>) -> Vec<String>,
        ),
        vec!["unchanged".to_string()],
    );
    let passed = Writer::pass(writer);
    let (result, output) = passed.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["unchanged"]);
}

// =============================================================================
// MonadWriter - censor Tests
// =============================================================================

#[rstest]
fn writer_censor_modifies_output() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["hello".to_string()]);
    let censored = Writer::censor(
        |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
        writer,
    );
    let (result, output) = censored.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["HELLO"]);
}

#[rstest]
fn writer_censor_with_tell() {
    let writer = Writer::tell(vec!["secret".to_string()]).fmap(|_| 42);
    let censored = Writer::censor(|_: Vec<String>| vec!["[REDACTED]".to_string()], writer);
    let (result, output) = censored.run();
    assert_eq!(result, 42);
    assert_eq!(output, vec!["[REDACTED]"]);
}

// =============================================================================
// Complex Use Cases
// =============================================================================

#[rstest]
fn writer_logging_pattern() {
    fn log(message: &str) -> Writer<Vec<String>, ()> {
        Writer::tell(vec![message.to_string()])
    }

    fn compute_with_logging(value: i32) -> Writer<Vec<String>, i32> {
        log("Starting computation")
            .then(log(&format!("Input value: {}", value)))
            .then(Writer::new(
                value * 2,
                vec!["Doubled the value".to_string()],
            ))
            .flat_map(|doubled| log(&format!("Result: {}", doubled)).fmap(move |_| doubled))
    }

    let (result, logs) = compute_with_logging(21).run();
    assert_eq!(result, 42);
    assert_eq!(
        logs,
        vec![
            "Starting computation",
            "Input value: 21",
            "Doubled the value",
            "Result: 42"
        ]
    );
}

#[rstest]
fn writer_metrics_accumulation() {
    #[derive(Clone, Debug, PartialEq)]
    struct Metrics {
        operation_count: i32,
        total_time_ms: i32,
    }

    impl Default for Metrics {
        fn default() -> Self {
            Metrics {
                operation_count: 0,
                total_time_ms: 0,
            }
        }
    }

    impl lambars::typeclass::Semigroup for Metrics {
        fn combine(self, other: Self) -> Self {
            Metrics {
                operation_count: self.operation_count + other.operation_count,
                total_time_ms: self.total_time_ms + other.total_time_ms,
            }
        }
    }

    impl lambars::typeclass::Monoid for Metrics {
        fn empty() -> Self {
            Metrics::default()
        }
    }

    fn record_operation(time_ms: i32) -> Writer<Metrics, ()> {
        Writer::tell(Metrics {
            operation_count: 1,
            total_time_ms: time_ms,
        })
    }

    let computation = record_operation(10)
        .then(record_operation(20))
        .then(record_operation(30))
        .fmap(|_| "done");

    let (result, metrics) = computation.run();
    assert_eq!(result, "done");
    assert_eq!(
        metrics,
        Metrics {
            operation_count: 3,
            total_time_ms: 60
        }
    );
}

#[rstest]
fn writer_multiple_flat_map_chains() {
    let computation: Writer<Vec<String>, i32> = Writer::tell(vec!["start".to_string()])
        .then(Writer::pure(5))
        .flat_map(|a| Writer::new(a + 10, vec!["added 10".to_string()]))
        .flat_map(|b| Writer::new(b * 2, vec!["doubled".to_string()]));

    let (result, output) = computation.run();
    assert_eq!(result, 30); // (5 + 10) * 2
    assert_eq!(output, vec!["start", "added 10", "doubled"]);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
fn writer_with_unit_result() {
    let writer: Writer<Vec<String>, ()> = Writer::tell(vec!["log only".to_string()]);
    let (result, output) = writer.run();
    assert_eq!(result, ());
    assert_eq!(output, vec!["log only"]);
}

#[rstest]
fn writer_with_empty_output() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec![]);
    let (result, output) = writer.run();
    assert_eq!(result, 42);
    assert!(output.is_empty());
}

#[rstest]
fn writer_run_can_be_called_multiple_times() {
    let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
    assert_eq!(writer.run(), (42, vec!["log".to_string()]));
    assert_eq!(writer.run(), (42, vec!["log".to_string()]));
}
