//! Integration tests for algebraic effect composition.
//!
//! Tests the effect row, member trait, and composed handlers.

#![cfg(feature = "effect")]

use lambars::EffectRow;
use lambars::effect::algebraic::{
    ComposedHandler, Eff, EffCons, EffNil, Effect, FindIndex, Handler, Here, Member, ReaderEffect,
    ReaderHandler, StateEffect, StateHandler, There,
};
use rstest::rstest;

// =============================================================================
// EffectRow! Macro Integration Tests
// =============================================================================

#[rstest]
fn effect_row_empty_is_eff_nil() {
    type Empty = EffectRow![];
    assert_eq!(Empty::NAME, "EffNil");
}

#[rstest]
fn effect_row_single_effect() {
    type Single = EffectRow![ReaderEffect<i32>];
    assert_eq!(Single::NAME, "EffCons");
}

#[rstest]
fn effect_row_multiple_effects() {
    type Multi = EffectRow![ReaderEffect<i32>, StateEffect<String>];
    fn assert_effect<T: Effect>() {}
    assert_effect::<Multi>();
}

#[rstest]
fn effect_row_three_effects() {
    type Three = EffectRow![ReaderEffect<i32>, StateEffect<String>, ReaderEffect<bool>];
    fn assert_effect<T: Effect>() {}
    assert_effect::<Three>();
}

// =============================================================================
// Member Trait Integration Tests
// =============================================================================

#[rstest]
fn member_here_for_first_effect() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    fn check_member<R: Member<ReaderEffect<i32>, Here>>() {}
    check_member::<Row>();
}

#[rstest]
fn member_there_for_second_effect() {
    type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;

    fn check_member<R: Member<StateEffect<String>, There<Here>>>() {}
    check_member::<Row>();
}

#[rstest]
fn member_inject_and_project_preserve_pure_value() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    let original: Eff<ReaderEffect<i32>, i32> = Eff::pure(42);
    let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(original);
    let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected);

    assert!(projected.is_some());
    let result = ReaderHandler::new(0).run(projected.unwrap());
    assert_eq!(result, 42);
}

#[rstest]
fn member_inject_and_project_preserve_operation() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    let original = ReaderEffect::<i32>::ask();
    let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(original);
    let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected);

    assert!(projected.is_some());
    let result = ReaderHandler::new(123).run(projected.unwrap());
    assert_eq!(result, 123);
}

#[rstest]
fn member_inject_second_effect() {
    type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;

    let state_eff = StateEffect::<i32>::get();
    let injected: Eff<Row, i32> = <Row as Member<StateEffect<i32>, There<Here>>>::inject(state_eff);
    let projected = <Row as Member<StateEffect<i32>, There<Here>>>::project(injected);

    assert!(projected.is_some());
    let (result, _) = StateHandler::new(456).run(projected.unwrap());
    assert_eq!(result, 456);
}

// =============================================================================
// FindIndex Integration Tests
// =============================================================================

#[rstest]
fn find_index_for_first_effect() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    fn check_index<R: FindIndex<ReaderEffect<i32>, Index = Here>>() {}
    check_index::<Row>();
}

// =============================================================================
// ComposedHandler Integration Tests
// =============================================================================

#[rstest]
fn composed_handler_holds_both_handlers() {
    let composed = ComposedHandler::new(ReaderHandler::new(42), StateHandler::new(0));

    assert_eq!(*composed.first().environment(), 42);
    assert_eq!(*composed.second().initial_state(), 0);
}

#[rstest]
fn composed_handler_can_be_split() {
    let composed = ComposedHandler::new(ReaderHandler::new(42), StateHandler::new("hello"));

    let (reader, state) = composed.into_parts();
    assert_eq!(*reader.environment(), 42);
    assert_eq!(*state.initial_state(), "hello");
}

// =============================================================================
// Complex Effect Row Scenarios
// =============================================================================

#[rstest]
fn three_effect_row_member_at_each_position() {
    type Row = EffCons<
        ReaderEffect<i32>,
        EffCons<StateEffect<String>, EffCons<ReaderEffect<bool>, EffNil>>,
    >;

    fn check_first<R: Member<ReaderEffect<i32>, Here>>() {}
    fn check_second<R: Member<StateEffect<String>, There<Here>>>() {}
    fn check_third<R: Member<ReaderEffect<bool>, There<There<Here>>>>() {}

    check_first::<Row>();
    check_second::<Row>();
    check_third::<Row>();
}

#[rstest]
fn inject_and_project_third_effect() {
    type Row = EffCons<
        ReaderEffect<i32>,
        EffCons<StateEffect<String>, EffCons<ReaderEffect<bool>, EffNil>>,
    >;

    let bool_reader = ReaderEffect::<bool>::ask();
    let injected: Eff<Row, bool> =
        <Row as Member<ReaderEffect<bool>, There<There<Here>>>>::inject(bool_reader);
    let projected = <Row as Member<ReaderEffect<bool>, There<There<Here>>>>::project(injected);

    assert!(projected.is_some());
    let result = ReaderHandler::new(true).run(projected.unwrap());
    assert!(result);
}

// =============================================================================
// Chained Operations with Effect Rows
// =============================================================================

#[rstest]
fn reader_effect_with_fmap_through_row() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    let original = ReaderEffect::<i32>::ask().fmap(|x| x * 2);
    let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(original);
    let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected).unwrap();

    let result = ReaderHandler::new(21).run(projected);
    assert_eq!(result, 42);
}

#[rstest]
fn state_effect_with_modify_through_row() {
    type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;

    let state_eff = StateEffect::<i32>::modify(|x| x + 10).then(StateEffect::get());
    let injected: Eff<Row, i32> = <Row as Member<StateEffect<i32>, There<Here>>>::inject(state_eff);
    let projected = <Row as Member<StateEffect<i32>, There<Here>>>::project(injected).unwrap();

    let (result, final_state) = StateHandler::new(5).run(projected);
    assert_eq!(result, 15);
    assert_eq!(final_state, 15);
}

// =============================================================================
// Sequential Handler Application (Simulated Multiple Effects)
// =============================================================================

#[rstest]
fn sequential_reader_then_state() {
    // Simulate handling multiple effects by running handlers sequentially
    // First, run a reader computation
    let reader_result = ReaderHandler::new(10).run(ReaderEffect::<i32>::ask().fmap(|x| x * 2));
    assert_eq!(reader_result, 20);

    // Then, use the result in a state computation
    let (state_result, final_state) = StateHandler::new(reader_result)
        .run(StateEffect::<i32>::modify(|x| x + 5).then(StateEffect::get()));
    assert_eq!(state_result, 25);
    assert_eq!(final_state, 25);
}

#[rstest]
fn composed_handler_sequential_run() {
    let composed = ComposedHandler::new(ReaderHandler::new(10), StateHandler::new(0));

    // Run reader first
    let reader_result = composed.first().clone().run(ReaderEffect::<i32>::ask());
    assert_eq!(reader_result, 10);

    // Run state with reader result
    let (state_result, final_state) = composed
        .second()
        .clone()
        .run(StateEffect::<i32>::modify(move |x| x + reader_result).then(StateEffect::get()));
    assert_eq!(state_result, 10);
    assert_eq!(final_state, 10);
}

// =============================================================================
// Type Safety Tests
// =============================================================================

#[rstest]
fn effect_row_type_ids_are_distinct() {
    type Row1 = EffectRow![ReaderEffect<i32>];
    type Row2 = EffectRow![StateEffect<i32>];
    type Row3 = EffectRow![ReaderEffect<i32>, StateEffect<String>];

    assert_ne!(Row1::type_id(), Row2::type_id());
    assert_ne!(Row1::type_id(), Row3::type_id());
    assert_ne!(Row2::type_id(), Row3::type_id());
}

#[rstest]
fn effect_row_same_structure_same_type_id() {
    type Row1 = EffectRow![ReaderEffect<i32>, StateEffect<String>];
    type Row2 = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;

    assert_eq!(Row1::type_id(), Row2::type_id());
}

// =============================================================================
// Stack Safety Tests
// =============================================================================

#[rstest]
fn deep_inject_project_chain_is_stack_safe() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    let mut computation: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
    for _ in 0..100 {
        computation = computation.fmap(|x| x + 1);
    }

    let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(computation);
    let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected).unwrap();

    let result = ReaderHandler::new(0).run(projected);
    assert_eq!(result, 100);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
fn empty_effect_row_is_valid_effect() {
    type Empty = EffectRow![];
    fn assert_effect<T: Effect>() {}
    assert_effect::<Empty>();
    assert_eq!(Empty::NAME, "EffNil");
}

#[rstest]
fn pure_computation_through_row_preserves_value() {
    type Row = EffCons<ReaderEffect<i32>, EffNil>;

    let pure_eff: Eff<ReaderEffect<i32>, String> = Eff::pure("hello".to_string());
    let injected: Eff<Row, String> = <Row as Member<ReaderEffect<i32>, Here>>::inject(pure_eff);
    let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected).unwrap();

    let result = ReaderHandler::new(0).run(projected);
    assert_eq!(result, "hello");
}
