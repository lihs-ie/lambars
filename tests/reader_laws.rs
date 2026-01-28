#![cfg(feature = "effect")]
//! Property-based tests for Reader Monad laws.
//!
//! Tests the following laws using proptest:
//!
//! ## Functor Laws
//! - Identity: reader.fmap(|x| x) == reader
//! - Composition: reader.fmap(f).fmap(g) == reader.fmap(|x| g(f(x)))
//!
//! ## Monad Laws
//! - Left Identity: pure(a).flat_map(f) == f(a)
//! - Right Identity: m.flat_map(pure) == m
//! - Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! ## MonadReader Laws
//! - Ask Local Identity: local(|r| r, m) == m
//! - Ask Local Composition: local(f, local(g, m)) == local(|r| g(f(r)), m)
//! - Ask Retrieval: ask().run(r) == r

use lambars::effect::Reader;
use proptest::prelude::*;

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: reader.fmap(|x| x) == reader
    #[test]
    fn prop_reader_functor_identity(environment in -1000i32..1000i32, value in -1000i32..1000i32) {
        let reader: Reader<i32, i32> = Reader::new(move |_| value);
        let mapped = Reader::new(move |_| value).fmap(|x| x);

        prop_assert_eq!(reader.run(environment), mapped.run(environment));
    }

    /// Functor Composition Law: reader.fmap(f).fmap(g) == reader.fmap(|x| g(f(x)))
    #[test]
    fn prop_reader_functor_composition(environment in -100i32..100i32) {
        let function1 = |x: i32| x.wrapping_add(1);
        let function2 = |x: i32| x.wrapping_mul(2);

        let reader: Reader<i32, i32> = Reader::new(|environment| environment);

        let left = Reader::new(|environment: i32| environment)
            .fmap(function1)
            .fmap(function2);
        let right = reader.fmap(move |x| function2(function1(x)));

        prop_assert_eq!(left.run(environment), right.run(environment));
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity Law: pure(a).flat_map(f) == f(a)
    #[test]
    fn prop_reader_monad_left_identity(value in -1000i32..1000i32, environment in -1000i32..1000i32) {
        let function = |a: i32| Reader::new(move |environment: i32| a.wrapping_add(environment));

        let left: Reader<i32, i32> = Reader::pure(value).flat_map(function);
        let right: Reader<i32, i32> = function(value);

        prop_assert_eq!(left.run(environment), right.run(environment));
    }

    /// Monad Right Identity Law: m.flat_map(pure) == m
    #[test]
    fn prop_reader_monad_right_identity(environment in -1000i32..1000i32) {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment);
        let right_identity: Reader<i32, i32> = Reader::new(|environment: i32| environment)
            .flat_map(Reader::pure);

        prop_assert_eq!(reader.run(environment), right_identity.run(environment));
    }

    /// Monad Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    #[test]
    fn prop_reader_monad_associativity(environment in -100i32..100i32) {
        let function1 = |a: i32| Reader::new(move |environment: i32| a.wrapping_add(environment));
        let function2 = |b: i32| Reader::new(move |environment: i32| b.wrapping_mul(environment));

        let reader: Reader<i32, i32> = Reader::new(|environment| environment);

        let left = Reader::new(|environment: i32| environment)
            .flat_map(function1)
            .flat_map(function2);
        let right = reader.flat_map(move |x| function1(x).flat_map(function2));

        prop_assert_eq!(left.run(environment), right.run(environment));
    }
}

// =============================================================================
// MonadReader Laws
// =============================================================================

proptest! {
    /// Ask Local Identity Law: local(|r| r, m) == m
    #[test]
    fn prop_reader_local_identity(environment in -1000i32..1000i32) {
        let reader: Reader<i32, i32> = Reader::new(|environment: i32| environment.wrapping_mul(2));
        let local_identity: Reader<i32, i32> = Reader::local(
            |r| r,
            Reader::new(|environment: i32| environment.wrapping_mul(2))
        );

        prop_assert_eq!(reader.run(environment), local_identity.run(environment));
    }

    /// Ask Local Composition Law: local(f, local(g, m)) == local(|r| g(f(r)), m)
    #[test]
    fn prop_reader_local_composition(environment in -50i32..50i32) {
        let modifier_f = |r: i32| r.wrapping_add(10);
        let modifier_g = |r: i32| r.wrapping_mul(2);

        let reader: Reader<i32, i32> = Reader::new(|environment: i32| environment);

        let left = Reader::local(
            modifier_f,
            Reader::local(modifier_g, Reader::new(|environment: i32| environment))
        );
        let right = Reader::local(
            move |r| modifier_g(modifier_f(r)),
            reader
        );

        prop_assert_eq!(left.run(environment), right.run(environment));
    }

    /// Ask Retrieval Law: ask().run(r) == r
    #[test]
    fn prop_reader_ask_retrieval(environment in -1000i32..1000i32) {
        let ask_reader: Reader<i32, i32> = Reader::ask();
        prop_assert_eq!(ask_reader.run(environment), environment);
    }
}

// =============================================================================
// Applicative Laws
// =============================================================================

proptest! {
    /// Applicative Identity Law: pure(id).apply(v) == v
    /// We test: pure(x).fmap(|x| x) == pure(x)
    #[test]
    fn prop_reader_applicative_identity(value in -1000i32..1000i32, environment in -1000i32..1000i32) {
        let pure_value: Reader<i32, i32> = Reader::pure(value);
        let mapped: Reader<i32, i32> = Reader::pure(value).fmap(|x| x);

        prop_assert_eq!(pure_value.run(environment), mapped.run(environment));
    }

    /// Applicative Homomorphism Law: pure(f).apply(pure(x)) == pure(f(x))
    /// We test this using map2 which is equivalent to liftA2
    #[test]
    fn prop_reader_applicative_homomorphism(value in -1000i32..1000i32, environment in -1000i32..1000i32) {
        fn add_one(x: i32) -> i32 {
            x.wrapping_add(1)
        }

        let function_reader: Reader<i32, fn(i32) -> i32> = Reader::pure(add_one as fn(i32) -> i32);
        let value_reader: Reader<i32, i32> = Reader::pure(value);
        let left: Reader<i32, i32> = function_reader.apply(value_reader);
        let right: Reader<i32, i32> = Reader::pure(add_one(value));

        prop_assert_eq!(left.run(environment), right.run(environment));
    }
}

// =============================================================================
// Additional Property Tests
// =============================================================================

proptest! {
    /// ask followed by asks should be equivalent to asks alone
    #[test]
    fn prop_reader_ask_asks_equivalence(environment in -1000i32..1000i32) {
        let via_ask: Reader<i32, i32> =
            Reader::ask().fmap(|value: i32| value.wrapping_mul(2));
        let via_asks: Reader<i32, i32> = Reader::asks(|environment: i32| environment.wrapping_mul(2));

        prop_assert_eq!(via_ask.run(environment), via_asks.run(environment));
    }

    /// map2 should combine readers correctly
    #[test]
    fn prop_reader_map2_combines(environment in -100i32..100i32) {
        let reader1: Reader<i32, i32> = Reader::new(|input_value: i32| input_value);
        let reader2: Reader<i32, i32> = Reader::new(|input_value: i32| input_value.wrapping_mul(2));

        let combined = reader1.map2(reader2, |a, b| a.wrapping_add(b));

        prop_assert_eq!(
            combined.run(environment),
            environment.wrapping_add(environment.wrapping_mul(2))
        );
    }

    /// product should create correct tuple
    #[test]
    fn prop_reader_product_creates_tuple(environment in -1000i32..1000i32) {
        let reader1: Reader<i32, i32> = Reader::new(|input_value: i32| input_value);
        let reader2: Reader<i32, i32> = Reader::new(|input_value: i32| input_value.wrapping_mul(2));

        let product = reader1.product(reader2);
        let (left, right) = product.run(environment);

        prop_assert_eq!(left, environment);
        prop_assert_eq!(right, environment.wrapping_mul(2));
    }

    /// then should discard the first value
    #[test]
    fn prop_reader_then_discards_first(environment in -1000i32..1000i32, second_value in -1000i32..1000i32) {
        let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
        let reader2: Reader<i32, i32> = Reader::pure(second_value);

        let result = reader1.then(reader2);

        prop_assert_eq!(result.run(environment), second_value);
    }

    /// local should not affect outer computations
    #[test]
    fn prop_reader_local_scoped(environment in -50i32..50i32) {
        let outer: Reader<i32, i32> = Reader::ask();
        let inner: Reader<i32, i32> = Reader::local(
            |environment| environment.wrapping_mul(10),
            Reader::ask()
        );

        let combined = outer.map2(inner, |outer_value, inner_value| (outer_value, inner_value));
        let (outer_result, inner_result) = combined.run(environment);

        prop_assert_eq!(outer_result, environment);
        prop_assert_eq!(inner_result, environment.wrapping_mul(10));
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
    fn reader_functor_identity_with_zero() {
        let reader: Reader<i32, i32> = Reader::new(|_| 0);
        let mapped = Reader::new(|_: i32| 0).fmap(|x| x);
        assert_eq!(reader.run(0), mapped.run(0));
    }

    #[rstest]
    fn reader_monad_left_identity_with_pure() {
        let value = 42;
        let function = |a: i32| Reader::pure(a * 2);

        let left: Reader<i32, i32> = Reader::pure(value).flat_map(function);
        let right: Reader<i32, i32> = function(value);

        assert_eq!(left.run(0), right.run(0));
    }

    #[rstest]
    fn reader_local_with_identity_is_noop() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let local_identity = Reader::local(|r| r, Reader::new(|environment: i32| environment * 2));

        for environment in [-100, -1, 0, 1, 100] {
            assert_eq!(
                reader.run_cloned(environment),
                local_identity.run_cloned(environment)
            );
        }
    }

    #[rstest]
    fn reader_ask_matches_new_identity() {
        let ask: Reader<i32, i32> = Reader::ask();
        let identity: Reader<i32, i32> = Reader::new(|environment| environment);

        for environment in [-100, -1, 0, 1, 100] {
            assert_eq!(
                ask.run_cloned(environment),
                identity.run_cloned(environment)
            );
        }
    }
}
