//! Tests for WriterT (Writer Transformer).
//!
//! WriterT adds output accumulation capability to any monad.

use lambars::effect::{IO, WriterT};
use lambars::typeclass::Monoid;
use rstest::rstest;

// =============================================================================
// Basic Structure Tests
// =============================================================================

#[rstest]
fn writer_transformer_new_and_run_with_option() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((42, vec!["log".to_string()])));
    let result = writer_transformer.run();
    assert_eq!(result, Some((42, vec!["log".to_string()])));
}

#[rstest]
fn writer_transformer_new_and_run_with_result() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Ok((42, vec!["log".to_string()])));
    let result = writer_transformer.run();
    assert_eq!(result, Ok((42, vec!["log".to_string()])));
}

#[rstest]
fn writer_transformer_run_returns_none_when_inner_is_none() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(None::<(i32, Vec<String>)>);
    let result = writer_transformer.run();
    assert_eq!(result, None);
}

// =============================================================================
// pure Tests
// =============================================================================

#[rstest]
fn writer_transformer_pure_with_option() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::pure_option(42);
    let result = writer_transformer.run();
    assert_eq!(result, Some((42, Vec::<String>::empty())));
}

#[rstest]
fn writer_transformer_pure_with_result() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::pure_result(42);
    let result = writer_transformer.run();
    assert_eq!(result, Ok((42, Vec::<String>::empty())));
}

// =============================================================================
// lift Tests
// =============================================================================

#[rstest]
fn writer_transformer_lift_option() {
    let inner: Option<i32> = Some(42);
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::lift_option(inner);
    let result = writer_transformer.run();
    assert_eq!(result, Some((42, Vec::<String>::empty())));
}

#[rstest]
fn writer_transformer_lift_option_none() {
    let inner: Option<i32> = None;
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::lift_option(inner);
    let result = writer_transformer.run();
    assert_eq!(result, None);
}

#[rstest]
fn writer_transformer_lift_result() {
    let inner: Result<i32, String> = Ok(42);
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::lift_result(inner);
    let result = writer_transformer.run();
    assert_eq!(result, Ok((42, Vec::<String>::empty())));
}

#[rstest]
fn writer_transformer_lift_result_error() {
    let inner: Result<i32, String> = Err("error".to_string());
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::lift_result(inner);
    let result = writer_transformer.run();
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// tell Tests
// =============================================================================

#[rstest]
fn writer_transformer_tell_option() {
    let writer_transformer: WriterT<Vec<String>, Option<((), Vec<String>)>> =
        WriterT::<Vec<String>, Option<((), Vec<String>)>>::tell_option(vec!["message".to_string()]);
    let result = writer_transformer.run();
    assert_eq!(result, Some(((), vec!["message".to_string()])));
}

#[rstest]
fn writer_transformer_tell_result() {
    let writer_transformer: WriterT<Vec<String>, Result<((), Vec<String>), String>> =
        WriterT::<Vec<String>, Result<((), Vec<String>), String>>::tell_result(vec![
            "message".to_string(),
        ]);
    let result = writer_transformer.run();
    assert_eq!(result, Ok(((), vec!["message".to_string()])));
}

// =============================================================================
// fmap (Functor) Tests
// =============================================================================

#[rstest]
fn writer_transformer_fmap_option_some() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((21, vec!["log".to_string()])));
    let mapped = writer_transformer.fmap_option(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Some((42, vec!["log".to_string()])));
}

#[rstest]
fn writer_transformer_fmap_option_none() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(None::<(i32, Vec<String>)>);
    let mapped = writer_transformer.fmap_option(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, None);
}

#[rstest]
fn writer_transformer_fmap_result_ok() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Ok((21, vec!["log".to_string()])));
    let mapped = writer_transformer.fmap_result(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Ok((42, vec!["log".to_string()])));
}

#[rstest]
fn writer_transformer_fmap_result_error() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Err::<(i32, Vec<String>), String>("error".to_string()));
    let mapped = writer_transformer.fmap_result(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// flat_map (Monad) Tests
// =============================================================================

#[rstest]
fn writer_transformer_flat_map_option_some_to_some() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((10, vec!["first".to_string()])));

    let chained = writer_transformer
        .flat_map_option(|value| WriterT::new(Some((value * 2, vec!["second".to_string()]))));

    let result = chained.run();
    // Logs should be combined: ["first", "second"]
    assert_eq!(
        result,
        Some((20, vec!["first".to_string(), "second".to_string()]))
    );
}

#[rstest]
fn writer_transformer_flat_map_option_some_to_none() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((10, vec!["first".to_string()])));

    let chained =
        writer_transformer.flat_map_option(|_value| WriterT::new(None::<(i32, Vec<String>)>));

    let result = chained.run();
    assert_eq!(result, None);
}

#[rstest]
fn writer_transformer_flat_map_option_none_short_circuits() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(None::<(i32, Vec<String>)>);

    let chained = writer_transformer
        .flat_map_option(|value| WriterT::new(Some((value * 2, vec!["second".to_string()]))));

    let result = chained.run();
    assert_eq!(result, None);
}

#[rstest]
fn writer_transformer_flat_map_result_ok_to_ok() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Ok((10, vec!["first".to_string()])));

    let chained = writer_transformer
        .flat_map_result(|value| WriterT::new(Ok((value * 2, vec!["second".to_string()]))));

    let result = chained.run();
    assert_eq!(
        result,
        Ok((20, vec!["first".to_string(), "second".to_string()]))
    );
}

#[rstest]
fn writer_transformer_flat_map_result_ok_to_error() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Ok((10, vec!["first".to_string()])));

    let chained = writer_transformer.flat_map_result(|_value| {
        WriterT::new(Err::<(i32, Vec<String>), String>("error".to_string()))
    });

    let result = chained.run();
    assert_eq!(result, Err("error".to_string()));
}

#[rstest]
fn writer_transformer_flat_map_result_error_short_circuits() {
    let writer_transformer: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
        WriterT::new(Err::<(i32, Vec<String>), String>("error".to_string()));

    let chained = writer_transformer
        .flat_map_result(|value| WriterT::new(Ok((value * 2, vec!["second".to_string()]))));

    let result = chained.run();
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// listen Tests
// =============================================================================

#[rstest]
fn writer_transformer_listen_option() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((42, vec!["log".to_string()])));

    let listened = WriterT::listen_option(writer_transformer);
    let result = listened.run();

    // listen returns ((value, captured_output), output)
    assert_eq!(
        result,
        Some(((42, vec!["log".to_string()]), vec!["log".to_string()]))
    );
}

#[rstest]
fn writer_transformer_listen_option_none() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(None::<(i32, Vec<String>)>);

    let listened = WriterT::listen_option(writer_transformer);
    let result = listened.run();

    assert_eq!(result, None);
}

// =============================================================================
// WriterT with IO Tests
// =============================================================================

#[rstest]
fn writer_transformer_with_io_basic() {
    let writer_transformer: WriterT<Vec<String>, IO<(i32, Vec<String>)>> =
        WriterT::new(IO::pure((42, vec!["log".to_string()])));

    let io_result = writer_transformer.run();
    let (value, output) = io_result.run_unsafe();
    assert_eq!(value, 42);
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_transformer_lift_io() {
    let inner = IO::pure(42);
    let writer_transformer: WriterT<Vec<String>, IO<(i32, Vec<String>)>> = WriterT::lift_io(inner);

    let io_result = writer_transformer.run();
    let (value, output) = io_result.run_unsafe();
    assert_eq!(value, 42);
    assert!(output.is_empty());
}

#[rstest]
fn writer_transformer_tell_io() {
    let writer_transformer: WriterT<Vec<String>, IO<((), Vec<String>)>> =
        WriterT::<Vec<String>, IO<((), Vec<String>)>>::tell_io(vec!["message".to_string()]);

    let io_result = writer_transformer.run();
    let (_, output) = io_result.run_unsafe();
    assert_eq!(output, vec!["message"]);
}

#[rstest]
fn writer_transformer_fmap_io() {
    let writer_transformer: WriterT<Vec<String>, IO<(i32, Vec<String>)>> =
        WriterT::new(IO::pure((21, vec!["log".to_string()])));

    let mapped = writer_transformer.fmap_io(|value| value * 2);

    let io_result = mapped.run();
    let (value, output) = io_result.run_unsafe();
    assert_eq!(value, 42);
    assert_eq!(output, vec!["log"]);
}

#[rstest]
fn writer_transformer_flat_map_io() {
    let writer_transformer: WriterT<Vec<String>, IO<(i32, Vec<String>)>> =
        WriterT::new(IO::pure((10, vec!["first".to_string()])));

    let chained = writer_transformer
        .flat_map_io(|value| WriterT::new(IO::pure((value * 2, vec!["second".to_string()]))));

    let io_result = chained.run();
    let (value, output) = io_result.run_unsafe();
    assert_eq!(value, 20);
    assert_eq!(output, vec!["first", "second"]);
}

// =============================================================================
// Clone Tests
// =============================================================================

#[rstest]
fn writer_transformer_clone() {
    let writer_transformer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
        WriterT::new(Some((42, vec!["log".to_string()])));
    let cloned = writer_transformer.clone();

    assert_eq!(
        writer_transformer.run(),
        Some((42, vec!["log".to_string()]))
    );
    assert_eq!(cloned.run(), Some((42, vec!["log".to_string()])));
}

// =============================================================================
// Practical Examples
// =============================================================================

#[rstest]
fn writer_transformer_logging_example() {
    fn log(message: &str) -> WriterT<Vec<String>, Option<((), Vec<String>)>> {
        WriterT::<Vec<String>, Option<((), Vec<String>)>>::tell_option(vec![message.to_string()])
    }

    fn computation_with_logging() -> WriterT<Vec<String>, Option<(i32, Vec<String>)>> {
        log("Starting computation")
            .flat_map_option(|_| WriterT::new(Some((10, vec!["step 1".to_string()]))))
            .flat_map_option(|value| {
                log("Processing value").flat_map_option(move |_| WriterT::pure_option(value * 2))
            })
            .flat_map_option(|result| {
                log("Computation complete").flat_map_option(move |_| WriterT::pure_option(result))
            })
    }

    let result = computation_with_logging().run();
    assert_eq!(
        result,
        Some((
            20,
            vec![
                "Starting computation".to_string(),
                "step 1".to_string(),
                "Processing value".to_string(),
                "Computation complete".to_string()
            ]
        ))
    );
}

#[rstest]
fn writer_transformer_accumulate_sum_example() {
    use lambars::typeclass::Sum;

    fn add_to_sum(value: i32) -> WriterT<Sum<i32>, Option<((), Sum<i32>)>> {
        WriterT::<Sum<i32>, Option<((), Sum<i32>)>>::tell_option(Sum(value))
    }

    let computation = add_to_sum(10)
        .flat_map_option(|_| add_to_sum(20))
        .flat_map_option(|_| add_to_sum(12));

    let result = computation.run();
    assert_eq!(result, Some(((), Sum(42))));
}
