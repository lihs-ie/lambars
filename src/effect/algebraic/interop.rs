//! Interoperability with existing effect systems.
//!
//! This module provides traits and implementations for converting between
//! algebraic effects (`Eff`) and other effect representations like the
//! existing MTL-style monads.
//!
//! # Overview
//!
//! The algebraic effects system can interoperate with:
//!
//! - **Reader Monad**: Convert `Reader<R, A>` to `Eff<ReaderEffect<R>, A>`
//! - **MTL-style traits**: Use `MonadReader`, `MonadState`, etc. with `Eff`
//!
//! # Conversion Traits
//!
//! - [`IntoEff`]: Convert from other effect types to `Eff`
//! - [`FromEff`]: Convert from `Eff` to other effect types
//!
//! # MTL-style Operations
//!
//! For effect rows, this module provides convenience functions that work
//! with the `Member` trait to inject effects into rows:
//!
//! - [`ask`], [`asks`]: Reader operations
//! - [`get`], [`put`], [`modify`]: State operations
//! - [`tell`]: Writer operations
//! - [`throw_error`]: Error operations
//!
//! # Examples
//!
//! Converting a Reader to Eff:
//!
//! ```rust
//! use lambars::effect::Reader;
//! use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler, IntoEff};
//!
//! let reader: Reader<i32, i32> = Reader::new(|env| env * 2);
//! let eff = reader.into_eff();
//!
//! let result = ReaderHandler::new(21).run(eff);
//! assert_eq!(result, 42);
//! ```
//!
//! Using MTL-style operations with effect rows:
//!
//! ```rust
//! use lambars::effect::algebraic::{
//!     Eff, ReaderEffect, StateEffect, ReaderHandler, StateHandler, Handler,
//!     Here, There, Member,
//! };
//! use lambars::effect::algebraic::interop::{ask, get, put};
//! use lambars::EffectRow;
//!
//! type Row = EffectRow![ReaderEffect<i32>, StateEffect<String>];
//!
//! // Use MTL-style operations with explicit index types
//! let computation: Eff<Row, i32> = ask::<i32, Row, Here>();
//! ```

use super::eff::Eff;
use super::effect::Effect;
use super::error::ErrorEffect;
use super::member::Member;
use super::reader::ReaderEffect;
use super::state::StateEffect;
use super::writer::WriterEffect;
use crate::effect::Reader;
use crate::typeclass::Monoid;

// IntoEff Trait

/// Converts a value into an effectful computation.
///
/// This trait enables conversion from other effect representations
/// (like Reader monad) into the algebraic effects system.
///
/// # Type Parameters
///
/// - `E`: The target effect type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::Reader;
/// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler, IntoEff};
///
/// let reader: Reader<i32, String> = Reader::asks(|env| format!("value: {}", env));
/// let eff = reader.into_eff();
///
/// let result = ReaderHandler::new(42).run(eff);
/// assert_eq!(result, "value: 42");
/// ```
pub trait IntoEff<E: Effect> {
    /// The result type of the effectful computation.
    type Value;

    /// Converts this value into an effectful computation.
    fn into_eff(self) -> Eff<E, Self::Value>;
}

// FromEff Trait

/// Creates a value from an effectful computation.
///
/// This trait enables conversion from algebraic effects back to
/// other effect representations.
///
/// # Type Parameters
///
/// - `E`: The source effect type
/// - `A`: The result type of the computation
///
/// # Note
///
/// This trait is more limited than `IntoEff` because converting from
/// `Eff` typically requires running a handler, which may not always
/// be possible in a generic context.
pub trait FromEff<E: Effect, A>: Sized {
    /// Creates this type from an effectful computation.
    fn from_eff(eff: Eff<E, A>) -> Self;
}

// Reader Monad Interop

impl<R: Clone + 'static, A: Clone + 'static> IntoEff<ReaderEffect<R>> for Reader<R, A> {
    type Value = A;

    /// Converts a Reader monad into an Eff computation.
    ///
    /// The resulting computation asks for the environment and then
    /// runs the original Reader with it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    /// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler, IntoEff};
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|env| env * 2);
    /// let eff = reader.into_eff();
    ///
    /// assert_eq!(ReaderHandler::new(21).run(eff), 42);
    /// ```
    fn into_eff(self) -> Eff<ReaderEffect<R>, A> {
        ReaderEffect::ask().fmap(move |environment| self.run(environment))
    }
}

/// Retrieves the entire environment from an effect row.
///
/// This is the MTL-style `ask` operation for effect rows.
///
/// # Type Parameters
///
/// - `R`: The environment type
/// - `Row`: The effect row type
/// - `I`: The index of `ReaderEffect<R>` in the row
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, ReaderEffect, ReaderHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::ask;
///
/// type Row = EffCons<ReaderEffect<i32>, EffNil>;
/// let computation: Eff<Row, i32> = ask::<i32, Row, Here>();
/// let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(computation).unwrap();
/// let result = ReaderHandler::new(42).run(projected);
/// assert_eq!(result, 42);
/// ```
#[must_use]
pub fn ask<R, Row, I>() -> Eff<Row, R>
where
    R: Clone + 'static,
    Row: Effect + Member<ReaderEffect<R>, I>,
{
    Row::inject(ReaderEffect::<R>::ask())
}

/// Projects a value from the environment in an effect row.
///
/// This is the MTL-style `asks` operation for effect rows.
///
/// # Type Parameters
///
/// - `R`: The environment type
/// - `A`: The projected value type
/// - `Row`: The effect row type
/// - `I`: The index of `ReaderEffect<R>` in the row
/// - `F`: The projection function type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, ReaderEffect, ReaderHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::asks;
///
/// type Row = EffCons<ReaderEffect<String>, EffNil>;
/// let computation: Eff<Row, usize> =
///     asks::<String, usize, Row, Here, _>(|s: String| s.len());
///
/// let projected = <Row as Member<ReaderEffect<String>, Here>>::project(computation).unwrap();
/// let result = ReaderHandler::new("hello".to_string()).run(projected);
/// assert_eq!(result, 5);
/// ```
#[must_use]
pub fn asks<R, A, Row, I, F>(projection: F) -> Eff<Row, A>
where
    R: Clone + 'static,
    A: 'static,
    Row: Effect + Member<ReaderEffect<R>, I>,
    F: FnOnce(R) -> A + 'static,
{
    Row::inject(ReaderEffect::<R>::asks(projection))
}
/// Retrieves the current state from an effect row.
///
/// This is the MTL-style `get` operation for effect rows.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `Row`: The effect row type
/// - `I`: The index of `StateEffect<S>` in the row
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, StateEffect, StateHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::get;
///
/// type Row = EffCons<StateEffect<i32>, EffNil>;
/// let computation: Eff<Row, i32> = get::<i32, Row, Here>();
/// let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
/// let (result, final_state) = StateHandler::new(42).run(projected);
/// assert_eq!(result, 42);
/// assert_eq!(final_state, 42);
/// ```
#[must_use]
pub fn get<S, Row, I>() -> Eff<Row, S>
where
    S: Clone + 'static,
    Row: Effect + Member<StateEffect<S>, I>,
{
    Row::inject(StateEffect::<S>::get())
}

/// Sets the state in an effect row.
///
/// This is the MTL-style `put` operation for effect rows.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `Row`: The effect row type
/// - `I`: The index of `StateEffect<S>` in the row
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, StateEffect, StateHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::put;
///
/// type Row = EffCons<StateEffect<i32>, EffNil>;
/// let computation: Eff<Row, ()> = put::<i32, Row, Here>(100);
/// let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
/// let ((), final_state) = StateHandler::new(0).run(projected);
/// assert_eq!(final_state, 100);
/// ```
#[must_use]
pub fn put<S, Row, I>(state: S) -> Eff<Row, ()>
where
    S: Clone + Send + Sync + 'static,
    Row: Effect + Member<StateEffect<S>, I>,
{
    Row::inject(StateEffect::put(state))
}

/// Modifies the state using a function in an effect row.
///
/// This is the MTL-style `modify` operation for effect rows.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `Row`: The effect row type
/// - `I`: The index of `StateEffect<S>` in the row
/// - `F`: The modifier function type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, StateEffect, StateHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::modify;
///
/// type Row = EffCons<StateEffect<i32>, EffNil>;
/// let computation: Eff<Row, ()> =
///     modify::<i32, Row, Here, _>(|x| x * 2);
///
/// let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
/// let ((), final_state) = StateHandler::new(21).run(projected);
/// assert_eq!(final_state, 42);
/// ```
#[must_use]
pub fn modify<S, Row, I, F>(modifier: F) -> Eff<Row, ()>
where
    S: Clone + Send + Sync + 'static,
    Row: Effect + Member<StateEffect<S>, I>,
    F: FnOnce(S) -> S + 'static,
{
    Row::inject(StateEffect::modify(modifier))
}

/// Projects a value from the state in an effect row.
///
/// This is the MTL-style `gets` operation for effect rows.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `A`: The projected value type
/// - `Row`: The effect row type
/// - `I`: The index of `StateEffect<S>` in the row
/// - `F`: The projection function type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, StateEffect, StateHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::gets;
///
/// type Row = EffCons<StateEffect<Vec<i32>>, EffNil>;
/// let computation: Eff<Row, usize> =
///     gets::<Vec<i32>, usize, Row, Here, _>(|v| v.len());
///
/// let projected = <Row as Member<StateEffect<Vec<i32>>, Here>>::project(computation).unwrap();
/// let (result, _) = StateHandler::new(vec![1, 2, 3]).run(projected);
/// assert_eq!(result, 3);
/// ```
#[must_use]
pub fn gets<S, A, Row, I, F>(projection: F) -> Eff<Row, A>
where
    S: Clone + 'static,
    A: 'static,
    Row: Effect + Member<StateEffect<S>, I>,
    F: FnOnce(&S) -> A + 'static,
{
    Row::inject(StateEffect::gets(projection))
}
/// Appends output to the log in an effect row.
///
/// This is the MTL-style `tell` operation for effect rows.
///
/// # Type Parameters
///
/// - `W`: The output type (must implement `Monoid`)
/// - `Row`: The effect row type
/// - `I`: The index of `WriterEffect<W>` in the row
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, WriterEffect, WriterHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::tell;
///
/// type Row = EffCons<WriterEffect<String>, EffNil>;
/// let computation: Eff<Row, ()> =
///     tell::<String, Row, Here>("hello".to_string());
///
/// let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
/// let ((), log) = WriterHandler::new().run(projected);
/// assert_eq!(log, "hello");
/// ```
#[must_use]
pub fn tell<W, Row, I>(output: W) -> Eff<Row, ()>
where
    W: Monoid + Clone + Send + Sync + 'static,
    Row: Effect + Member<WriterEffect<W>, I>,
{
    Row::inject(WriterEffect::tell(output))
}
/// Throws an error in an effect row.
///
/// This is the MTL-style `throw_error` operation for effect rows.
///
/// # Type Parameters
///
/// - `E`: The error type
/// - `A`: The result type (never actually produced)
/// - `Row`: The effect row type
/// - `I`: The index of `ErrorEffect<E>` in the row
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     Eff, ErrorEffect, ErrorHandler, Handler, Here, Member, EffCons, EffNil,
/// };
/// use lambars::effect::algebraic::interop::throw_error;
///
/// type Row = EffCons<ErrorEffect<String>, EffNil>;
/// let computation: Eff<Row, i32> =
///     throw_error::<String, i32, Row, Here>("error".to_string());
///
/// let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
/// let result = ErrorHandler::new().run(projected);
/// assert_eq!(result, Err("error".to_string()));
/// ```
#[must_use]
pub fn throw_error<E, A, Row, I>(error: E) -> Eff<Row, A>
where
    E: Clone + Send + Sync + 'static,
    A: 'static,
    Row: Effect + Member<ErrorEffect<E>, I>,
{
    Row::inject(ErrorEffect::throw(error))
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::algebraic::handler::Handler;
    use crate::effect::algebraic::member::{Here, There};
    use crate::effect::algebraic::row::{EffCons, EffNil};
    use crate::effect::algebraic::{ErrorHandler, ReaderHandler, StateHandler, WriterHandler};
    use rstest::rstest;
    #[rstest]
    fn into_eff_trait_is_defined() {
        fn assert_into_eff<T: IntoEff<ReaderEffect<i32>>>() {}
        let _ = assert_into_eff::<Reader<i32, i32>>;
    }
    #[rstest]
    fn from_eff_trait_is_defined() {
        fn assert_from_eff<T: FromEff<ReaderEffect<i32>, i32>>() {}
        // FromEff is defined but no implementations yet
        let _ = assert_from_eff::<()>;
    }

    // Unit type implements FromEff trivially for testing
    impl<E: Effect, A> FromEff<E, A> for () {
        fn from_eff(_eff: Eff<E, A>) -> Self {}
    }
    #[rstest]
    fn reader_into_eff_simple() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let eff = reader.into_eff();

        let result = ReaderHandler::new(21).run(eff);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn reader_into_eff_with_asks() {
        let reader: Reader<String, usize> = Reader::asks(|s: String| s.len());
        let eff = reader.into_eff();

        let result = ReaderHandler::new("hello".to_string()).run(eff);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn reader_into_eff_pure() {
        let reader: Reader<i32, &str> = Reader::pure("constant");
        let eff = reader.into_eff();

        let result = ReaderHandler::new(999).run(eff);
        assert_eq!(result, "constant");
    }

    #[rstest]
    fn reader_into_eff_with_flat_map() {
        let reader: Reader<i32, i32> = Reader::ask().flat_map(|x| Reader::new(move |y| x + y));
        let eff = reader.into_eff();

        let result = ReaderHandler::new(10).run(eff);
        assert_eq!(result, 20); // 10 + 10
    }

    #[rstest]
    fn reader_into_eff_preserves_local() {
        let reader = Reader::local(|x: i32| x * 2, Reader::ask());
        let eff = reader.into_eff();

        let result = ReaderHandler::new(21).run(eff);
        assert_eq!(result, 42);
    }
    #[rstest]
    fn ask_in_effect_row_here() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let computation: Eff<Row, i32> = ask::<i32, Row, Here>();

        let projected = <Row as Member<ReaderEffect<i32>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new(42).run(projected);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn ask_in_effect_row_there() {
        type Row = EffCons<StateEffect<String>, EffCons<ReaderEffect<i32>, EffNil>>;
        let computation: Eff<Row, i32> = ask::<i32, Row, There<Here>>();

        let projected =
            <Row as Member<ReaderEffect<i32>, There<Here>>>::project(computation).unwrap();
        let result = ReaderHandler::new(100).run(projected);
        assert_eq!(result, 100);
    }
    #[rstest]
    fn asks_in_effect_row_here() {
        type Row = EffCons<ReaderEffect<String>, EffNil>;
        let computation: Eff<Row, usize> = asks::<String, usize, Row, Here, _>(|s: String| s.len());

        let projected = <Row as Member<ReaderEffect<String>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new("hello".to_string()).run(projected);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn asks_in_effect_row_with_sum() {
        type Row = EffCons<ReaderEffect<Vec<i32>>, EffNil>;
        let computation: Eff<Row, i32> =
            asks::<Vec<i32>, i32, Row, Here, _>(|v: Vec<i32>| v.iter().sum());

        let projected =
            <Row as Member<ReaderEffect<Vec<i32>>, Here>>::project(computation).unwrap();
        let result = ReaderHandler::new(vec![1, 2, 3]).run(projected);
        assert_eq!(result, 6);
    }
    #[rstest]
    fn get_in_effect_row_here() {
        type Row = EffCons<StateEffect<i32>, EffNil>;
        let computation: Eff<Row, i32> = get::<i32, Row, Here>();

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(42).run(projected);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn get_in_effect_row_there() {
        type Row = EffCons<ReaderEffect<String>, EffCons<StateEffect<i32>, EffNil>>;
        let computation: Eff<Row, i32> = get::<i32, Row, There<Here>>();

        let projected =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(computation).unwrap();
        let (result, final_state) = StateHandler::new(50).run(projected);
        assert_eq!(result, 50);
        assert_eq!(final_state, 50);
    }
    #[rstest]
    fn put_in_effect_row_here() {
        type Row = EffCons<StateEffect<i32>, EffNil>;
        let computation: Eff<Row, ()> = put::<i32, Row, Here>(100);

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(0).run(projected);
        assert_eq!(final_state, 100);
    }

    #[rstest]
    fn put_in_effect_row_with_value() {
        type Row = EffCons<StateEffect<i32>, EffNil>;
        let computation: Eff<Row, ()> = put::<i32, Row, Here>(42);

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(0).run(projected);
        assert_eq!(final_state, 42);
    }
    #[rstest]
    fn modify_in_effect_row_here() {
        type Row = EffCons<StateEffect<i32>, EffNil>;
        let computation: Eff<Row, ()> = modify::<i32, Row, Here, _>(|x| x * 2);

        let projected = <Row as Member<StateEffect<i32>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(21).run(projected);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn modify_in_effect_row_with_vec() {
        type Row = EffCons<StateEffect<Vec<i32>>, EffNil>;
        let computation: Eff<Row, ()> = modify::<Vec<i32>, Row, Here, _>(|mut v| {
            v.push(4);
            v
        });

        let projected = <Row as Member<StateEffect<Vec<i32>>, Here>>::project(computation).unwrap();
        let ((), final_state) = StateHandler::new(vec![1, 2, 3]).run(projected);
        assert_eq!(final_state, vec![1, 2, 3, 4]);
    }
    #[rstest]
    fn gets_in_effect_row_here() {
        type Row = EffCons<StateEffect<Vec<i32>>, EffNil>;
        let computation: Eff<Row, usize> = gets::<Vec<i32>, usize, Row, Here, _>(|v| v.len());

        let projected = <Row as Member<StateEffect<Vec<i32>>, Here>>::project(computation).unwrap();
        let (result, _) = StateHandler::new(vec![1, 2, 3]).run(projected);
        assert_eq!(result, 3);
    }

    #[rstest]
    fn gets_in_effect_row_with_string() {
        type Row = EffCons<StateEffect<String>, EffNil>;
        let computation: Eff<Row, usize> = gets::<String, usize, Row, Here, _>(|s| s.len());

        let projected = <Row as Member<StateEffect<String>, Here>>::project(computation).unwrap();
        let (result, _) = StateHandler::new("hello".to_string()).run(projected);
        assert_eq!(result, 5);
    }
    #[rstest]
    fn tell_in_effect_row_here() {
        type Row = EffCons<WriterEffect<String>, EffNil>;
        let computation: Eff<Row, ()> = tell::<String, Row, Here>("hello".to_string());

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, "hello");
    }

    #[rstest]
    fn tell_in_effect_row_with_vec() {
        type Row = EffCons<WriterEffect<Vec<String>>, EffNil>;
        let computation: Eff<Row, ()> = tell::<Vec<String>, Row, Here>(vec!["message".to_string()]);

        let projected =
            <Row as Member<WriterEffect<Vec<String>>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, vec!["message".to_string()]);
    }

    #[rstest]
    fn tell_multiple_times_in_row() {
        type Row = EffCons<WriterEffect<String>, EffNil>;
        let computation: Eff<Row, ()> = tell::<String, Row, Here>("a".to_string())
            .then(tell::<String, Row, Here>("b".to_string()))
            .then(tell::<String, Row, Here>("c".to_string()));

        let projected = <Row as Member<WriterEffect<String>, Here>>::project(computation).unwrap();
        let ((), log) = WriterHandler::new().run(projected);
        assert_eq!(log, "abc");
    }
    #[rstest]
    fn throw_error_in_effect_row_here() {
        type Row = EffCons<ErrorEffect<String>, EffNil>;
        let computation: Eff<Row, i32> = throw_error::<String, i32, Row, Here>("error".to_string());

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("error".to_string()));
    }

    #[rstest]
    fn throw_error_in_effect_row_with_oops() {
        type Row = EffCons<ErrorEffect<String>, EffNil>;
        let computation: Eff<Row, i32> = throw_error::<String, i32, Row, Here>("oops".to_string());

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("oops".to_string()));
    }

    #[rstest]
    fn throw_error_short_circuits_in_row() {
        type Row = EffCons<ErrorEffect<String>, EffNil>;
        let computation: Eff<Row, i32> =
            throw_error::<String, i32, Row, Here>("early".to_string()).fmap(|x| x + 1);

        let projected = <Row as Member<ErrorEffect<String>, Here>>::project(computation).unwrap();
        let result = ErrorHandler::new().run(projected);
        assert_eq!(result, Err("early".to_string()));
    }
    #[rstest]
    fn combined_reader_state_row() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<i32>, EffNil>>;

        // Use reader at index Here
        let reader_computation: Eff<Row, i32> = ask::<i32, Row, Here>();

        // Use state at index There<Here>
        let state_computation: Eff<Row, i32> = get::<i32, Row, There<Here>>();

        // Project and run reader
        let reader_projected =
            <Row as Member<ReaderEffect<i32>, Here>>::project(reader_computation).unwrap();
        let reader_result = ReaderHandler::new(10).run(reader_projected);
        assert_eq!(reader_result, 10);

        // Project and run state
        let state_projected =
            <Row as Member<StateEffect<i32>, There<Here>>>::project(state_computation).unwrap();
        let (state_result, _) = StateHandler::new(20).run(state_projected);
        assert_eq!(state_result, 20);
    }

    #[rstest]
    fn three_effect_row() {
        type Row = EffCons<
            ReaderEffect<i32>,
            EffCons<StateEffect<String>, EffCons<WriterEffect<Vec<i32>>, EffNil>>,
        >;

        // Reader at Here
        let read: Eff<Row, i32> = ask::<i32, Row, Here>();

        // State at There<Here>
        let state_get: Eff<Row, String> = get::<String, Row, There<Here>>();

        // Writer at There<There<Here>>
        let write: Eff<Row, ()> = tell::<Vec<i32>, Row, There<There<Here>>>(vec![1, 2, 3]);

        // Verify types compile
        let _ = read;
        let _ = state_get;
        let _ = write;
    }
}
