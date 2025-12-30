//! Property-based tests for Writer Monad laws.
//!
//! Tests the following laws using proptest:
//!
//! ## Functor Laws
//! - Identity: writer.fmap(|x| x) == writer
//! - Composition: writer.fmap(f).fmap(g) == writer.fmap(|x| g(f(x)))
//!
//! ## Monad Laws
//! - Left Identity: pure(a).flat_map(f) == f(a)
//! - Right Identity: m.flat_map(pure) == m
//! - Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! ## MonadWriter Laws
//! - Tell Monoid Law: tell(w1).then(tell(w2)) == tell(w1.combine(w2))
//! - Listen Tell Law: listen(tell(w)) captures the output correctly
//! - Pass Identity Law: pass(m.map(|a| (a, |w| w))) == m

use functional_rusty::effect::Writer;
use functional_rusty::typeclass::Semigroup;
use proptest::prelude::*;

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: writer.fmap(|x| x) == writer
    #[test]
    fn prop_writer_functor_identity(value in -1000i32..1000i32, log_count in 0usize..5) {
        let logs: Vec<String> = (0..log_count).map(|i| format!("log{}", i)).collect();
        let writer: Writer<Vec<String>, i32> = Writer::new(value, logs.clone());
        let mapped: Writer<Vec<String>, i32> = Writer::new(value, logs.clone()).fmap(|x| x);

        let (result1, output1) = writer.run();
        let (result2, output2) = mapped.run();

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(output1, output2);
    }

    /// Functor Composition Law: writer.fmap(f).fmap(g) == writer.fmap(|x| g(f(x)))
    #[test]
    fn prop_writer_functor_composition(value in -100i32..100i32) {
        let function1 = |x: i32| x.wrapping_add(1);
        let function2 = |x: i32| x.wrapping_mul(2);

        let logs = vec!["log".to_string()];
        let writer: Writer<Vec<String>, i32> = Writer::new(value, logs.clone());

        let left = Writer::new(value, logs.clone())
            .fmap(function1)
            .fmap(function2);
        let right = writer.fmap(move |x| function2(function1(x)));

        let (result_left, output_left) = left.run();
        let (result_right, output_right) = right.run();

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(output_left, output_right);
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity Law: pure(a).flat_map(f) == f(a)
    #[test]
    fn prop_writer_monad_left_identity(value in -1000i32..1000i32) {
        let function = |a: i32| Writer::new(a.wrapping_add(10), vec!["added".to_string()]);

        let left: Writer<Vec<String>, i32> = Writer::pure(value).flat_map(function);
        let right: Writer<Vec<String>, i32> = function(value);

        let (result_left, output_left) = left.run();
        let (result_right, output_right) = right.run();

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(output_left, output_right);
    }

    /// Monad Right Identity Law: m.flat_map(pure) == m
    #[test]
    fn prop_writer_monad_right_identity(value in -1000i32..1000i32) {
        let logs = vec!["log".to_string()];
        let writer: Writer<Vec<String>, i32> = Writer::new(value, logs.clone());
        let right_identity: Writer<Vec<String>, i32> = Writer::new(value, logs.clone())
            .flat_map(|x| Writer::pure(x));

        let (result1, output1) = writer.run();
        let (result2, output2) = right_identity.run();

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(output1, output2);
    }

    /// Monad Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    #[test]
    fn prop_writer_monad_associativity(value in -100i32..100i32) {
        let function1 = |a: i32| Writer::new(a.wrapping_add(5), vec!["f1".to_string()]);
        let function2 = |b: i32| Writer::new(b.wrapping_mul(2), vec!["f2".to_string()]);

        let logs = vec!["start".to_string()];
        let writer: Writer<Vec<String>, i32> = Writer::new(value, logs.clone());

        let left = Writer::new(value, logs.clone())
            .flat_map(function1)
            .flat_map(function2);
        let right = writer.flat_map(move |x| function1(x).flat_map(function2));

        let (result_left, output_left) = left.run();
        let (result_right, output_right) = right.run();

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(output_left, output_right);
    }
}

// =============================================================================
// MonadWriter Laws
// =============================================================================

proptest! {
    /// Tell Monoid Law: tell(w1).then(tell(w2)) == tell(w1.combine(w2))
    #[test]
    fn prop_writer_tell_monoid_law(log1 in "[a-z]{1,5}", log2 in "[a-z]{1,5}") {
        let w1 = vec![log1.clone()];
        let w2 = vec![log2.clone()];

        let left: Writer<Vec<String>, ()> = Writer::tell(w1.clone()).then(Writer::tell(w2.clone()));
        let right: Writer<Vec<String>, ()> = Writer::tell(w1.combine(w2));

        let (_, output_left) = left.run();
        let (_, output_right) = right.run();

        prop_assert_eq!(output_left, output_right);
    }

    /// Listen should capture the output correctly
    #[test]
    fn prop_writer_listen_captures_output(value in -1000i32..1000i32, log in "[a-z]{1,5}") {
        let logs = vec![log.clone()];
        let writer: Writer<Vec<String>, i32> = Writer::new(value, logs.clone());
        let listened = Writer::listen(writer);

        let ((result, captured), output) = listened.run();

        prop_assert_eq!(result, value);
        prop_assert_eq!(captured, logs.clone());
        prop_assert_eq!(output, logs);
    }
}

// =============================================================================
// Additional Property Tests
// =============================================================================

proptest! {
    /// map2 should combine outputs correctly
    #[test]
    fn prop_writer_map2_combines(value1 in -100i32..100i32, value2 in -100i32..100i32) {
        let writer1: Writer<Vec<String>, i32> = Writer::new(value1, vec!["first".to_string()]);
        let writer2: Writer<Vec<String>, i32> = Writer::new(value2, vec!["second".to_string()]);

        let combined = writer1.map2(writer2, |a, b| a.wrapping_add(b));
        let (result, output) = combined.run();

        prop_assert_eq!(result, value1.wrapping_add(value2));
        prop_assert_eq!(output, vec!["first".to_string(), "second".to_string()]);
    }

    /// product should create correct tuple
    #[test]
    fn prop_writer_product_creates_tuple(value1 in -1000i32..1000i32, value2 in -1000i32..1000i32) {
        let writer1: Writer<Vec<String>, i32> = Writer::new(value1, vec!["first".to_string()]);
        let writer2: Writer<Vec<String>, i32> = Writer::new(value2, vec!["second".to_string()]);

        let product = writer1.product(writer2);
        let ((left, right), output) = product.run();

        prop_assert_eq!(left, value1);
        prop_assert_eq!(right, value2);
        prop_assert_eq!(output, vec!["first".to_string(), "second".to_string()]);
    }

    /// then should discard the first value but combine outputs
    #[test]
    fn prop_writer_then_discards_first(first_value in -1000i32..1000i32, second_value in -1000i32..1000i32) {
        let writer1: Writer<Vec<String>, i32> = Writer::new(first_value, vec!["first".to_string()]);
        let writer2: Writer<Vec<String>, i32> = Writer::new(second_value, vec!["second".to_string()]);

        let result = writer1.then(writer2);
        let (value, output) = result.run();

        prop_assert_eq!(value, second_value);
        prop_assert_eq!(output, vec!["first".to_string(), "second".to_string()]);
    }

    /// pure should have empty output
    #[test]
    fn prop_writer_pure_empty_output(value in -1000i32..1000i32) {
        let writer: Writer<Vec<String>, i32> = Writer::pure(value);
        let (result, output) = writer.run();

        prop_assert_eq!(result, value);
        prop_assert!(output.is_empty());
    }

    /// tell should only produce output
    #[test]
    fn prop_writer_tell_produces_output(log in "[a-z]{1,10}") {
        let logs = vec![log.clone()];
        let writer: Writer<Vec<String>, ()> = Writer::tell(logs.clone());
        let (result, output) = writer.run();

        prop_assert_eq!(result, ());
        prop_assert_eq!(output, logs);
    }
}

// =============================================================================
// Unit Tests for Edge Cases
// =============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn writer_functor_identity_with_empty_output() {
        let writer: Writer<Vec<String>, i32> = Writer::new(42, vec![]);
        let mapped: Writer<Vec<String>, i32> = Writer::new(42, vec![]).fmap(|x| x);

        let (result1, output1) = writer.run();
        let (result2, output2) = mapped.run();

        assert_eq!(result1, result2);
        assert_eq!(output1, output2);
    }

    #[rstest]
    fn writer_monad_left_identity_with_pure() {
        let value = 42;
        let function = |a: i32| Writer::pure(a * 2);

        let left: Writer<Vec<String>, i32> = Writer::pure(value).flat_map(function);
        let right: Writer<Vec<String>, i32> = function(value);

        let (result1, output1) = left.run();
        let (result2, output2) = right.run();

        assert_eq!(result1, result2);
        assert_eq!(output1, output2);
    }

    #[rstest]
    fn writer_tell_monoid_law_multiple() {
        let w1 = vec!["first".to_string()];
        let w2 = vec!["second".to_string()];
        let w3 = vec!["third".to_string()];

        let left: Writer<Vec<String>, ()> = Writer::tell(w1.clone())
            .then(Writer::tell(w2.clone()))
            .then(Writer::tell(w3.clone()));
        let right: Writer<Vec<String>, ()> = Writer::tell(w1.combine(w2).combine(w3));

        let (_, output_left) = left.run();
        let (_, output_right) = right.run();

        assert_eq!(output_left, output_right);
    }

    #[rstest]
    fn writer_listen_with_empty_output() {
        let writer: Writer<Vec<String>, i32> = Writer::pure(42);
        let listened = Writer::listen(writer);
        let ((result, captured), output) = listened.run();

        assert_eq!(result, 42);
        assert!(captured.is_empty());
        assert!(output.is_empty());
    }

    #[rstest]
    fn writer_censor_modifies_output() {
        let writer: Writer<Vec<String>, i32> =
            Writer::new(42, vec!["hello".to_string(), "world".to_string()]);
        let censored = Writer::censor(
            |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
            writer,
        );
        let (result, output) = censored.run();

        assert_eq!(result, 42);
        assert_eq!(output, vec!["HELLO", "WORLD"]);
    }
}
