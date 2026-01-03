//! Property-based tests for Either IntoIterator implementation.

#![cfg(feature = "control")]

use lambars::control::Either;
use proptest::prelude::*;

// =============================================================================
// Strategy Definitions
// =============================================================================

fn arb_either_i32() -> impl Strategy<Value = Either<String, i32>> {
    prop_oneof![
        any::<i32>().prop_map(Either::Right),
        "[a-z]{1,10}".prop_map(Either::Left),
    ]
}

// =============================================================================
// Iterator Law Tests
// =============================================================================

proptest! {
    /// size_hint must be accurate for Either iterators.
    /// For Either, size_hint is always exact (0 or 1).
    #[test]
    fn prop_size_hint_matches_count(either in arb_either_i32()) {
        let iterator = either.clone().into_iter();
        let (lower, upper) = iterator.size_hint();
        let count = either.into_iter().count();

        prop_assert!(lower <= count);
        prop_assert!(upper == Some(count));
    }

    /// ExactSizeIterator::len must match count.
    #[test]
    fn prop_len_matches_count(either in arb_either_i32()) {
        let iterator = either.clone().into_iter();
        let len = iterator.len();
        let count = either.into_iter().count();

        prop_assert_eq!(len, count);
    }

    /// collect().len() must match count.
    #[test]
    fn prop_collect_len_matches_count(either in arb_either_i32()) {
        let collected: Vec<_> = either.clone().into_iter().collect();
        let count = either.into_iter().count();

        prop_assert_eq!(collected.len(), count);
    }
}

// =============================================================================
// Right Bias Tests
// =============================================================================

proptest! {
    /// Right(x).into_iter().collect() == vec![x]
    #[test]
    fn prop_right_yields_value(value: i32) {
        let right: Either<String, i32> = Either::Right(value);
        let collected: Vec<i32> = right.into_iter().collect();

        prop_assert_eq!(collected, vec![value]);
    }

    /// Left(e).into_iter().collect() == vec![]
    #[test]
    fn prop_left_yields_nothing(error in "[a-z]{1,10}") {
        let left: Either<String, i32> = Either::Left(error);
        let collected: Vec<i32> = left.into_iter().collect();

        prop_assert_eq!(collected, Vec::<i32>::new());
    }

    /// Right(x).into_iter().next() == Some(x)
    #[test]
    fn prop_right_next_is_some(value: i32) {
        let right: Either<String, i32> = Either::Right(value);
        let next = right.into_iter().next();

        prop_assert_eq!(next, Some(value));
    }

    /// Left(e).into_iter().next() == None
    #[test]
    fn prop_left_next_is_none(error in "[a-z]{1,10}") {
        let left: Either<String, i32> = Either::Left(error);
        let next = left.into_iter().next();

        prop_assert_eq!(next, None);
    }
}

// =============================================================================
// Reference Iterator Tests
// =============================================================================

proptest! {
    /// &Right(x).into_iter().collect() == vec![&x]
    #[test]
    fn prop_right_ref_yields_reference(value: i32) {
        let right: Either<String, i32> = Either::Right(value);
        let collected: Vec<&i32> = (&right).into_iter().collect();

        prop_assert_eq!(collected, vec![&value]);
        // right should still be usable
        prop_assert!(right.is_right());
    }

    /// &Left(e).into_iter().collect() == vec![]
    #[test]
    fn prop_left_ref_yields_nothing(error in "[a-z]{1,10}") {
        let left: Either<String, i32> = Either::Left(error.clone());
        let collected: Vec<&i32> = (&left).into_iter().collect();

        prop_assert_eq!(collected, Vec::<&i32>::new());
        // left should still be usable
        prop_assert!(left.is_left());
    }
}

// =============================================================================
// FusedIterator Tests
// =============================================================================

proptest! {
    /// FusedIterator: after returning None, always returns None.
    #[test]
    fn prop_fused_iterator(either in arb_either_i32()) {
        let mut iterator = either.into_iter();

        // Exhaust the iterator
        while iterator.next().is_some() {}

        // FusedIterator guarantees continued None
        prop_assert!(iterator.next().is_none());
        prop_assert!(iterator.next().is_none());
        prop_assert!(iterator.next().is_none());
    }
}

// =============================================================================
// DoubleEndedIterator Tests
// =============================================================================

proptest! {
    /// DoubleEndedIterator: next_back on Right returns the value.
    #[test]
    fn prop_double_ended_right(value: i32) {
        let right: Either<String, i32> = Either::Right(value);
        let mut iterator = right.into_iter();

        prop_assert_eq!(iterator.next_back(), Some(value));
        prop_assert_eq!(iterator.next_back(), None);
    }

    /// DoubleEndedIterator: next_back on Left returns None.
    #[test]
    fn prop_double_ended_left(error in "[a-z]{1,10}") {
        let left: Either<String, i32> = Either::Left(error);
        let mut iterator = left.into_iter();

        prop_assert_eq!(iterator.next_back(), None);
    }
}
