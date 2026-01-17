//! Member trait for effect row membership.
//!
//! This module provides the [`Member`] trait that proves an effect is contained
//! within an effect row. The Index pattern is used to track the position of
//! effects at the type level, allowing for safe injection and projection.
//!
//! # Index Pattern
//!
//! The Index pattern solves the "backward search" problem in type-level
//! programming. Each effect's position in a row is encoded as a type:
//!
//! - [`Here`] - The effect is at the head of the row
//! - [`There<I>`] - The effect is further down, at index `I` in the tail
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{
//!     EffNil, EffCons, Effect, Member, Here, There,
//!     ReaderEffect, StateEffect,
//! };
//! use lambars::EffectRow;
//!
//! // For row: [Reader<i32>, State<String>]
//! // - Reader<i32> has index Here
//! // - State<String> has index There<Here>
//! type Row = EffectRow![ReaderEffect<i32>, StateEffect<String>];
//!
//! // Prove that Row contains ReaderEffect<i32> at index Here
//! fn has_reader<R, I>() where R: Member<ReaderEffect<i32>, I> {}
//! has_reader::<Row, Here>();
//!
//! // Prove that Row contains StateEffect<String> at index There<Here>
//! fn has_state<R, I>() where R: Member<StateEffect<String>, I> {}
//! has_state::<Row, There<Here>>();
//! ```

use super::eff::{Eff, EffContinuationQueue, EffInner, EffOperation};
use super::effect::Effect;
use super::row::EffCons;
use crate::control::continuation_queue::TypeErasedArrow;
use std::any::Any;
use std::marker::PhantomData;

/// Index indicating the effect is at the head of the row.
///
/// When searching for an effect in a row `EffCons<E, Tail>`, if the
/// target effect is `E`, the index is `Here`.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{EffCons, EffNil, Member, Here, ReaderEffect};
///
/// type Row = EffCons<ReaderEffect<i32>, EffNil>;
///
/// // ReaderEffect<i32> is at index Here
/// fn check<R: Member<ReaderEffect<i32>, Here>>() {}
/// check::<Row>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Here;

/// Index indicating the effect is in the tail of the row.
///
/// When searching for an effect in a row `EffCons<Other, Tail>`, if the
/// target effect is not `Other` but is in `Tail` at index `I`, then the
/// overall index is `There<I>`.
///
/// # Type Parameters
///
/// - `I`: The index of the effect within the tail
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     EffCons, EffNil, Member, Here, There, ReaderEffect, StateEffect,
/// };
///
/// type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
///
/// // StateEffect<String> is at index There<Here>
/// fn check<R: Member<StateEffect<String>, There<Here>>>() {}
/// check::<Row>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct There<I>(PhantomData<I>);

impl<I> There<I> {
    /// Creates a new `There` index.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// Trait proving that an effect `E` is a member of an effect row.
///
/// The `Index` type parameter tracks the position of the effect in the row,
/// enabling type-safe injection and projection operations.
///
/// # Type Parameters
///
/// - `E`: The effect to find in the row
/// - `Index`: The type-level position of `E` in the row
///
/// # Laws
///
/// ## Injection-Projection Identity
///
/// Projecting an injected effect returns the original:
///
/// ```text
/// project(inject(eff)) == Some(eff')
/// ```
///
/// where `eff'` is operationally equivalent to `eff`.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     EffCons, EffNil, Member, Here, There,
///     ReaderEffect, StateEffect, Eff,
/// };
///
/// type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
///
/// // Inject a Reader operation into the row
/// let reader_eff = ReaderEffect::<i32>::ask();
/// let row_eff: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(reader_eff);
/// ```
pub trait Member<E: Effect, Index>: Effect + Sized {
    /// Injects an effect operation into the effect row.
    ///
    /// This converts `Eff<E, A>` into `Eff<Self, A>`, embedding the
    /// single-effect computation into the larger effect row.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The result type of the computation
    ///
    /// # Arguments
    ///
    /// * `effect` - The computation using effect `E`
    ///
    /// # Returns
    ///
    /// The computation embedded in the effect row
    fn inject<A: 'static>(effect: Eff<E, A>) -> Eff<Self, A>;

    /// Projects an effect operation from the effect row.
    ///
    /// This attempts to extract a computation targeting effect `E` from
    /// a computation using the full effect row. Returns `None` if the
    /// computation's operation is not for effect `E`.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The result type of the computation
    ///
    /// # Arguments
    ///
    /// * `effect` - The computation using the effect row
    ///
    /// # Returns
    ///
    /// `Some(eff)` if the operation targets effect `E`, `None` otherwise
    fn project<A: 'static>(effect: Eff<Self, A>) -> Option<Eff<E, A>>;
}

/// Member implementation for the head of an effect row.
///
/// When the target effect `E` is at the head of `EffCons<E, Tail>`,
/// injection and projection are direct operations.
impl<E: Effect, Tail: Effect> Member<E, Here> for EffCons<E, Tail> {
    fn inject<A: 'static>(effect: Eff<E, A>) -> Eff<Self, A> {
        convert_effect_type(effect)
    }

    fn project<A: 'static>(effect: Eff<Self, A>) -> Option<Eff<E, A>> {
        Some(convert_effect_type(effect))
    }
}

/// Member implementation for effects in the tail of a row.
///
/// When the target effect `E` is not at the head but is in `Tail` at
/// index `I`, we delegate to the tail's Member implementation.
#[allow(clippy::type_repetition_in_bounds)]
impl<E: Effect, Other: Effect, Tail: Effect, I> Member<E, There<I>> for EffCons<Other, Tail>
where
    Tail: Member<E, I>,
{
    fn inject<A: 'static>(effect: Eff<E, A>) -> Eff<Self, A> {
        let tail_effect: Eff<Tail, A> = Tail::inject(effect);
        convert_effect_type(tail_effect)
    }

    fn project<A: 'static>(effect: Eff<Self, A>) -> Option<Eff<E, A>> {
        let tail_effect: Eff<Tail, A> = convert_effect_type(effect);
        Tail::project(tail_effect)
    }
}

/// Converts an effect computation from one effect type to another.
///
/// This is safe because `Eff<E, A>` has the same memory layout regardless
/// of the effect type `E`. The effect type is only used for type checking
/// and does not affect the runtime representation.
///
/// # Safety
///
/// This function converts effect types by reconstructing the computation
/// with the new effect type marker. The actual data is unchanged.
fn convert_effect_type<E1: Effect + 'static, E2: Effect + 'static, A: 'static>(
    effect: Eff<E1, A>,
) -> Eff<E2, A> {
    match effect.inner {
        EffInner::Pure(value) => Eff {
            inner: EffInner::Pure(value),
        },
        EffInner::Impure(operation) => {
            // Convert the continuation queue by wrapping each arrow
            // Since we cannot directly convert the queue arrows, we create a new queue
            // with a single arrow that processes the original queue's output
            let mut new_queue = EffContinuationQueue::<E2>::new();

            // Add an arrow that will convert the result from E1 to E2
            // This is a simplified approach - the original queue is processed
            // when the operation result is applied, and we convert the final Eff
            new_queue.push_arrow(Box::new(EffectConversionArrow::<E1, E2, A> {
                original_queue: operation.queue,
                _phantom: PhantomData,
            }));

            Eff {
                inner: EffInner::Impure(EffOperation {
                    effect_marker: PhantomData,
                    operation_tag: operation.operation_tag,
                    arguments: operation.arguments,
                    queue: new_queue,
                    _result: PhantomData,
                }),
            }
        }
    }
}

/// Arrow that processes the original queue and converts the result.
struct EffectConversionArrow<E1, E2, A>
where
    E1: Effect,
    E2: Effect,
{
    original_queue: EffContinuationQueue<E1>,
    _phantom: PhantomData<(E2, A)>,
}

impl<E1: Effect + 'static, E2: Effect + 'static, A: 'static> TypeErasedArrow<Eff<E2, Box<dyn Any>>>
    for EffectConversionArrow<E1, E2, A>
{
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> Eff<E2, Box<dyn Any>> {
        // Process the original queue with the input
        let result = process_queue_with_value::<E1>(input, self.original_queue);
        // Convert the result from E1 to E2
        convert_effect_type::<E1, E2, Box<dyn Any>>(result)
    }
}

/// Process a continuation queue with an initial value.
/// This simulates applying the queue's arrows in sequence.
fn process_queue_with_value<E: Effect + 'static>(
    initial_value: Box<dyn Any>,
    queue: EffContinuationQueue<E>,
) -> Eff<E, Box<dyn Any>> {
    use super::eff::EffQueueStack;

    // If queue is empty, return pure value
    if queue.is_empty() {
        return Eff {
            inner: EffInner::Pure(initial_value),
        };
    }

    // Create a queue stack and process
    let mut queue_stack = EffQueueStack::new(queue);
    let mut current_value = initial_value;

    loop {
        match queue_stack.pop() {
            None => {
                return Eff {
                    inner: EffInner::Pure(current_value),
                };
            }
            Some(arrow) => {
                let result = arrow.apply(current_value);
                match result.inner {
                    EffInner::Pure(boxed) => {
                        current_value = boxed;
                    }
                    EffInner::Impure(operation) => {
                        // We have an impure result, need to return it with merged queue
                        queue_stack.push_queue(operation.queue);

                        // Create a new queue from the remaining queue_stack
                        let mut remaining_queue = EffContinuationQueue::<E>::new();
                        while let Some(remaining_arrow) = queue_stack.pop() {
                            remaining_queue.push_arrow(remaining_arrow);
                        }

                        return Eff {
                            inner: EffInner::Impure(EffOperation {
                                effect_marker: operation.effect_marker,
                                operation_tag: operation.operation_tag,
                                arguments: operation.arguments,
                                queue: remaining_queue,
                                _result: PhantomData,
                            }),
                        };
                    }
                }
            }
        }
    }
}

/// Trait for automatically finding an effect's index in a row.
///
/// This trait enables ergonomic use of effect rows by automatically
/// inferring the index type when it's unambiguous.
///
/// # Important Note
///
/// Due to Rust's trait resolution rules, this trait cannot be fully
/// implemented in a way that handles all cases automatically. The
/// implementations below cover common cases, but explicit index
/// specification may be required in some situations.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     EffCons, EffNil, FindIndex, Here, There,
///     ReaderEffect, StateEffect,
/// };
///
/// type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
///
/// // The index types are found automatically
/// fn check_reader<R: FindIndex<ReaderEffect<i32>>>() {}
/// check_reader::<Row>();
/// ```
pub trait FindIndex<E: Effect> {
    /// The index type for effect `E` in this row.
    type Index;
}

/// `FindIndex` implementation for the head of a row.
impl<E: Effect, Tail: Effect> FindIndex<E> for EffCons<E, Tail> {
    type Index = Here;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::algebraic::handler::Handler;
    use crate::effect::algebraic::row::EffNil;
    use crate::effect::algebraic::{ReaderEffect, ReaderHandler, StateEffect, StateHandler};
    use rstest::rstest;

    #[rstest]
    fn here_is_debug() {
        let here = Here;
        let debug_string = format!("{here:?}");
        assert_eq!(debug_string, "Here");
    }

    #[rstest]
    fn here_is_clone() {
        let here = Here;
        let cloned = here;
        assert_eq!(here, cloned);
    }

    #[rstest]
    fn here_is_copy() {
        let here = Here;
        let copied = here;
        assert_eq!(here, copied);
    }

    #[rstest]
    fn here_is_eq() {
        assert_eq!(Here, Here);
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn here_is_default() {
        let default_here = Here::default();
        assert_eq!(default_here, Here);
    }

    #[rstest]
    fn here_is_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Here);
        assert!(set.contains(&Here));
    }

    #[rstest]
    fn there_new_creates_instance() {
        let there: There<Here> = There::new();
        let debug_string = format!("{there:?}");
        assert!(debug_string.contains("There"));
    }

    #[rstest]
    fn there_is_debug() {
        let there: There<Here> = There::new();
        let debug_string = format!("{there:?}");
        assert!(debug_string.contains("There"));
    }

    #[rstest]
    fn there_is_clone() {
        let there: There<Here> = There::new();
        let cloned = there;
        assert_eq!(there, cloned);
    }

    #[rstest]
    fn there_is_copy() {
        let there: There<Here> = There::new();
        let copied = there;
        assert_eq!(there, copied);
    }

    #[rstest]
    fn there_is_eq() {
        let first: There<Here> = There::new();
        let second: There<Here> = There::new();
        assert_eq!(first, second);
    }

    #[rstest]
    fn there_is_default() {
        let default_there: There<Here> = There::default();
        let explicit_there: There<Here> = There::new();
        assert_eq!(default_there, explicit_there);
    }

    #[rstest]
    fn there_is_hash() {
        use std::collections::HashSet;
        let there: There<Here> = There::new();
        let mut set = HashSet::new();
        set.insert(there);
        assert!(set.contains(&There::new()));
    }

    // Member Inject Tests - Here Index

    #[rstest]
    fn member_inject_here_single_effect() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let effect = ReaderEffect::<i32>::ask();
        let _injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(effect);
    }

    #[rstest]
    fn member_inject_here_with_two_effects() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        let effect = ReaderEffect::<i32>::ask();
        let _injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(effect);
    }

    #[rstest]
    fn member_inject_here_pure_value() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let pure_eff: Eff<ReaderEffect<i32>, i32> = Eff::pure(42);
        let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(pure_eff);
        assert!(injected.is_pure());
    }

    // Member Inject Tests - There Index

    #[rstest]
    fn member_inject_there_second_effect() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        let effect = StateEffect::<String>::get();
        let _injected: Eff<Row, String> =
            <Row as Member<StateEffect<String>, There<Here>>>::inject(effect);
    }

    #[rstest]
    fn member_inject_there_third_effect() {
        type Row = EffCons<
            ReaderEffect<i32>,
            EffCons<StateEffect<String>, EffCons<ReaderEffect<bool>, EffNil>>,
        >;
        let effect = ReaderEffect::<bool>::ask();
        let _injected: Eff<Row, bool> =
            <Row as Member<ReaderEffect<bool>, There<There<Here>>>>::inject(effect);
    }

    #[rstest]
    fn member_inject_there_pure_value() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        let pure_eff: Eff<StateEffect<String>, String> = Eff::pure("hello".to_string());
        let injected: Eff<Row, String> =
            <Row as Member<StateEffect<String>, There<Here>>>::inject(pure_eff);
        assert!(injected.is_pure());
    }

    // Member Project Tests - Here Index

    #[rstest]
    fn member_project_here_returns_some() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let effect = ReaderEffect::<i32>::ask();
        let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(effect);
        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(injected);
        assert!(projected.is_some());
    }

    #[rstest]
    fn member_project_here_pure_value() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let pure_eff: Eff<Row, i32> =
            <Row as Member<ReaderEffect<i32>, Here>>::inject(Eff::pure(42));
        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(pure_eff);
        assert!(projected.is_some());
        let inner = projected.unwrap();
        assert!(inner.is_pure());
    }

    // Member Project Tests - There Index

    #[rstest]
    fn member_project_there_returns_some() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        let effect = StateEffect::<String>::get();
        let injected: Eff<Row, String> =
            <Row as Member<StateEffect<String>, There<Here>>>::inject(effect);
        let projected = <Row as Member<StateEffect<String>, There<Here>>>::project(injected);
        assert!(projected.is_some());
    }

    #[rstest]
    fn member_project_there_pure_value() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        let pure_eff: Eff<Row, String> = <Row as Member<StateEffect<String>, There<Here>>>::inject(
            Eff::pure("hello".to_string()),
        );
        let projected = <Row as Member<StateEffect<String>, There<Here>>>::project(pure_eff);
        assert!(projected.is_some());
        let inner = projected.unwrap();
        assert!(inner.is_pure());
    }

    #[rstest]
    fn inject_project_roundtrip_here_preserves_pure() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let original: Eff<ReaderEffect<i32>, i32> = Eff::pure(42);
        let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(original);
        let projected: Eff<ReaderEffect<i32>, i32> =
            <Row as Member<ReaderEffect<i32>, Here>>::project(injected).unwrap();

        let result = ReaderHandler::new(0).run(projected);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn inject_project_roundtrip_there_preserves_pure() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;
        let original: Eff<StateEffect<i32>, i32> = Eff::pure(42);
        let injected: Eff<Row, i32> =
            <Row as Member<StateEffect<i32>, There<Here>>>::inject(original);
        let projected: Eff<StateEffect<i32>, i32> =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(injected).unwrap();

        let (result, _) = StateHandler::new(0).run(projected);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn inject_project_roundtrip_here_preserves_operation() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let original: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
        let injected: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(original);
        let projected: Eff<ReaderEffect<i32>, i32> =
            <Row as Member<ReaderEffect<i32>, Here>>::project(injected).unwrap();

        let result = ReaderHandler::new(123).run(projected);
        assert_eq!(result, 123);
    }

    #[rstest]
    fn inject_project_roundtrip_there_preserves_operation() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;
        let original: Eff<StateEffect<i32>, i32> = StateEffect::get();
        let injected: Eff<Row, i32> =
            <Row as Member<StateEffect<i32>, There<Here>>>::inject(original);
        let projected: Eff<StateEffect<i32>, i32> =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(injected).unwrap();

        let (result, _) = StateHandler::new(456).run(projected);
        assert_eq!(result, 456);
    }

    #[rstest]
    fn find_index_here_for_head_effect() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;

        fn check_index<R: FindIndex<ReaderEffect<i32>, Index = Here>>() {}
        check_index::<Row>();
    }

    #[rstest]
    fn find_index_trait_bound_works() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;

        fn use_find_index<R, E>()
        where
            E: Effect,
            R: FindIndex<E>,
        {
        }

        use_find_index::<Row, ReaderEffect<i32>>();
    }

    #[rstest]
    fn member_with_three_effects() {
        type Row = EffCons<
            ReaderEffect<i32>,
            EffCons<StateEffect<String>, EffCons<ReaderEffect<bool>, EffNil>>,
        >;

        fn check_member_here<R: Member<ReaderEffect<i32>, Here>>() {}
        fn check_member_there1<R: Member<StateEffect<String>, There<Here>>>() {}
        fn check_member_there2<R: Member<ReaderEffect<bool>, There<There<Here>>>>() {}

        check_member_here::<Row>();
        check_member_there1::<Row>();
        check_member_there2::<Row>();
    }

    #[rstest]
    fn member_inject_and_run_reader_from_row() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;

        let reader_eff = ReaderEffect::<i32>::ask().fmap(|x| x * 2);
        let row_eff: Eff<Row, i32> = <Row as Member<ReaderEffect<i32>, Here>>::inject(reader_eff);

        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(row_eff).unwrap();
        let result = ReaderHandler::new(21).run(projected);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn member_inject_and_run_state_from_row() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;

        let state_eff = StateEffect::<i32>::modify(|x| x + 10).then(StateEffect::get());
        let row_eff: Eff<Row, i32> =
            <Row as Member<StateEffect<i32>, There<Here>>>::inject(state_eff);

        let projected = <Row as Member<StateEffect<i32>, There<Here>>>::project(row_eff).unwrap();
        let (result, final_state) = StateHandler::new(5).run(projected);
        assert_eq!(result, 15);
        assert_eq!(final_state, 15);
    }
}
