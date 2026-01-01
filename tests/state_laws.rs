//! Property-based tests for State Monad laws.
//!
//! Tests the following laws using proptest:
//!
//! ## Functor Laws
//! - Identity: state.fmap(|x| x) == state
//! - Composition: state.fmap(f).fmap(g) == state.fmap(|x| g(f(x)))
//!
//! ## Monad Laws
//! - Left Identity: pure(a).flat_map(f) == f(a)
//! - Right Identity: m.flat_map(pure) == m
//! - Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! ## MonadState Laws
//! - Get Put Law: get().flat_map(|s| put(s)) == pure(())
//! - Put Get Law: put(s).then(get()) returns s
//! - Put Put Law: put(s1).then(put(s2)) == put(s2)
//! - Modify Composition: modify(f).then(modify(g)) == modify(|s| g(f(s)))

use lambars::effect::State;
use proptest::prelude::*;

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: state.fmap(|x| x) == state
    #[test]
    fn prop_state_functor_identity(initial_state in -1000i32..1000i32) {
        let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        let mapped: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1)).fmap(|x| x);

        let (result1, final1) = state.run(initial_state);
        let (result2, final2) = mapped.run(initial_state);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(final1, final2);
    }

    /// Functor Composition Law: state.fmap(f).fmap(g) == state.fmap(|x| g(f(x)))
    #[test]
    fn prop_state_functor_composition(initial_state in -100i32..100i32) {
        let function1 = |x: i32| x.wrapping_add(1);
        let function2 = |x: i32| x.wrapping_mul(2);

        let state: State<i32, i32> = State::new(|s: i32| (s, s));

        let left = State::new(|s: i32| (s, s))
            .fmap(function1)
            .fmap(function2);
        let right = state.fmap(move |x| function2(function1(x)));

        let (result_left, final_left) = left.run(initial_state);
        let (result_right, final_right) = right.run(initial_state);

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(final_left, final_right);
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity Law: pure(a).flat_map(f) == f(a)
    #[test]
    fn prop_state_monad_left_identity(value in -1000i32..1000i32, initial_state in -1000i32..1000i32) {
        let function = |a: i32| State::new(move |s: i32| (a.wrapping_add(s), s.wrapping_add(1)));

        let left: State<i32, i32> = State::pure(value).flat_map(function);
        let right: State<i32, i32> = function(value);

        let (result_left, final_left) = left.run(initial_state);
        let (result_right, final_right) = right.run(initial_state);

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(final_left, final_right);
    }

    /// Monad Right Identity Law: m.flat_map(pure) == m
    #[test]
    fn prop_state_monad_right_identity(initial_state in -1000i32..1000i32) {
        let state: State<i32, i32> = State::new(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)));
        let right_identity: State<i32, i32> = State::new(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)))
            .flat_map(|x| State::pure(x));

        let (result1, final1) = state.run(initial_state);
        let (result2, final2) = right_identity.run(initial_state);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(final1, final2);
    }

    /// Monad Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    #[test]
    fn prop_state_monad_associativity(initial_state in -100i32..100i32) {
        let function1 = |a: i32| State::new(move |s: i32| (a.wrapping_add(s), s.wrapping_add(1)));
        let function2 = |b: i32| State::new(move |s: i32| (b.wrapping_mul(s), s.wrapping_mul(2)));

        let state: State<i32, i32> = State::new(|s: i32| (s, s));

        let left = State::new(|s: i32| (s, s))
            .flat_map(function1)
            .flat_map(function2);
        let right = state.flat_map(move |x| function1(x).flat_map(function2));

        let (result_left, final_left) = left.run(initial_state);
        let (result_right, final_right) = right.run(initial_state);

        prop_assert_eq!(result_left, result_right);
        prop_assert_eq!(final_left, final_right);
    }
}

// =============================================================================
// MonadState Laws
// =============================================================================

proptest! {
    /// Get Put Law: get().flat_map(|s| put(s)) is equivalent to pure(()) in terms of state
    #[test]
    fn prop_state_get_put_law(initial_state in -1000i32..1000i32) {
        let get_put: State<i32, ()> = State::get().flat_map(|s| State::put(s));
        let noop: State<i32, ()> = State::pure(());

        let (_, final1) = get_put.run(initial_state);
        let (_, final2) = noop.run(initial_state);

        prop_assert_eq!(final1, final2);
    }

    /// Put Get Law: put(s).then(get()) returns s
    #[test]
    fn prop_state_put_get_law(initial_state in -1000i32..1000i32, new_state in -1000i32..1000i32) {
        let put_get: State<i32, i32> = State::put(new_state).then(State::get());

        let (result, final_state) = put_get.run(initial_state);

        prop_assert_eq!(result, new_state);
        prop_assert_eq!(final_state, new_state);
    }

    /// Put Put Law: put(s1).then(put(s2)) == put(s2)
    #[test]
    fn prop_state_put_put_law(initial_state in -1000i32..1000i32, state1 in -1000i32..1000i32, state2 in -1000i32..1000i32) {
        let put_put: State<i32, ()> = State::put(state1).then(State::put(state2));
        let single_put: State<i32, ()> = State::put(state2);

        let (_, final1) = put_put.run(initial_state);
        let (_, final2) = single_put.run(initial_state);

        prop_assert_eq!(final1, final2);
    }

    /// Modify Composition Law: modify(f).then(modify(g)) == modify(|s| g(f(s)))
    #[test]
    fn prop_state_modify_composition_law(initial_state in -50i32..50i32) {
        let modifier_f = |s: i32| s.wrapping_add(10);
        let modifier_g = |s: i32| s.wrapping_mul(2);

        let chained: State<i32, ()> = State::modify(modifier_f).then(State::modify(modifier_g));
        let composed: State<i32, ()> = State::modify(move |s| modifier_g(modifier_f(s)));

        let (_, final1) = chained.run(initial_state);
        let (_, final2) = composed.run(initial_state);

        prop_assert_eq!(final1, final2);
    }
}

// =============================================================================
// Additional Property Tests
// =============================================================================

proptest! {
    /// get followed by gets should be equivalent to gets alone
    #[test]
    fn prop_state_get_gets_equivalence(initial_state in -1000i32..1000i32) {
        let via_get: State<i32, i32> = State::get().fmap(|s: i32| s.wrapping_mul(2));
        let via_gets: State<i32, i32> = State::gets(|s: &i32| s.wrapping_mul(2));

        let (result1, final1) = via_get.run(initial_state);
        let (result2, final2) = via_gets.run(initial_state);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(final1, final2);
    }

    /// map2 should combine states correctly
    #[test]
    fn prop_state_map2_combines(initial_state in -100i32..100i32) {
        let state1: State<i32, i32> = State::new(|s: i32| (s, s.wrapping_add(1)));
        let state2: State<i32, i32> = State::new(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)));

        let combined = state1.map2(state2, |a, b| a.wrapping_add(b));

        let (result, final_state) = combined.run(initial_state);
        // state1 runs first: (initial, initial+1)
        // state2 runs with initial+1: ((initial+1)*2, initial+2)
        // combined: initial + (initial+1)*2
        let expected_result = initial_state.wrapping_add((initial_state.wrapping_add(1)).wrapping_mul(2));
        let expected_final = initial_state.wrapping_add(2);

        prop_assert_eq!(result, expected_result);
        prop_assert_eq!(final_state, expected_final);
    }

    /// product should create correct tuple
    #[test]
    fn prop_state_product_creates_tuple(initial_state in -1000i32..1000i32) {
        let state1: State<i32, i32> = State::new(|s: i32| (s, s.wrapping_add(1)));
        let state2: State<i32, i32> = State::new(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)));

        let product = state1.product(state2);
        let ((left, right), final_state) = product.run(initial_state);

        prop_assert_eq!(left, initial_state);
        prop_assert_eq!(right, (initial_state.wrapping_add(1)).wrapping_mul(2));
        prop_assert_eq!(final_state, initial_state.wrapping_add(2));
    }

    /// then should discard the first value but sequence the state effects
    #[test]
    fn prop_state_then_discards_first(initial_state in -1000i32..1000i32, second_value in -1000i32..1000i32) {
        let state1: State<i32, i32> = State::new(|s: i32| (s, s.wrapping_add(10)));
        let state2: State<i32, i32> = State::pure(second_value);

        let result = state1.then(state2);
        let (value, final_state) = result.run(initial_state);

        prop_assert_eq!(value, second_value);
        prop_assert_eq!(final_state, initial_state.wrapping_add(10));
    }

    /// modify should not produce a value but should update state
    #[test]
    fn prop_state_modify_updates_state(initial_state in -100i32..100i32) {
        let modify_double: State<i32, ()> = State::modify(|s: i32| s.wrapping_mul(2));

        let (result, final_state) = modify_double.run(initial_state);

        prop_assert_eq!(result, ());
        prop_assert_eq!(final_state, initial_state.wrapping_mul(2));
    }

    /// new and from_transition should be equivalent
    #[test]
    fn prop_new_equals_from_transition(initial_state in -1000i32..1000i32) {
        let via_new: State<i32, i32> = State::new(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)));
        let via_from_transition: State<i32, i32> = State::from_transition(|s: i32| (s.wrapping_mul(2), s.wrapping_add(1)));

        let (result1, final1) = via_new.run(initial_state);
        let (result2, final2) = via_from_transition.run(initial_state);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(final1, final2);
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
    fn state_functor_identity_with_zero() {
        let state: State<i32, i32> = State::new(|s: i32| (s, s));
        let mapped: State<i32, i32> = State::new(|s: i32| (s, s)).fmap(|x| x);

        let (result1, final1) = state.run(0);
        let (result2, final2) = mapped.run(0);

        assert_eq!(result1, result2);
        assert_eq!(final1, final2);
    }

    #[rstest]
    fn state_monad_left_identity_with_pure() {
        let value = 42;
        let function = |a: i32| State::pure(a * 2);

        let left: State<i32, i32> = State::pure(value).flat_map(function);
        let right: State<i32, i32> = function(value);

        let (result1, final1) = left.run(0);
        let (result2, final2) = right.run(0);

        assert_eq!(result1, result2);
        assert_eq!(final1, final2);
    }

    #[rstest]
    fn state_get_put_is_noop_for_state() {
        let get_put: State<i32, ()> = State::get().flat_map(|s| State::put(s));

        for initial_state in [-100, -1, 0, 1, 100] {
            let (_, final_state) = get_put.run(initial_state);
            assert_eq!(final_state, initial_state);
        }
    }

    #[rstest]
    fn state_put_get_returns_put_value() {
        for new_state in [-100, -1, 0, 1, 100] {
            let put_get: State<i32, i32> = State::put(new_state).then(State::get());
            let (result, final_state) = put_get.run(999);
            assert_eq!(result, new_state);
            assert_eq!(final_state, new_state);
        }
    }

    #[rstest]
    fn state_modify_composition() {
        let f = |x: i32| x + 10;
        let g = |x: i32| x * 2;

        let chained: State<i32, ()> = State::modify(f).then(State::modify(g));
        let composed: State<i32, ()> = State::modify(move |s| g(f(s)));

        for initial_state in [-100, -1, 0, 1, 100] {
            let (_, final1) = chained.run(initial_state);
            let (_, final2) = composed.run(initial_state);
            assert_eq!(final1, final2);
        }
    }
}
