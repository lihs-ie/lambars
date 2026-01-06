//! Effectful computation type.
//!
//! This module provides the `Eff<E, A>` type that represents a computation
//! that may use effect `E` and produces a value of type `A`.
//!
//! # Stack Safety
//!
//! The implementation uses a `FlatMap` variant internally
//! to ensure deep `flat_map` chains do not overflow the stack.

use super::effect::Effect;
use std::any::Any;
use std::marker::PhantomData;

/// A tag identifying different operations within an effect.
///
/// Each operation of an effect has a unique tag used by handlers
/// to determine which operation is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationTag(pub(crate) u32);

impl OperationTag {
    /// Creates a new operation tag with the given value.
    #[must_use]
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

/// Type alias for type-erased continuation functions.
type Continuation<E, A> = Box<dyn FnOnce(Box<dyn Any>) -> Eff<E, A> + 'static>;

/// Internal structure representing an effect operation.
pub struct EffOperation<E: Effect, A: 'static> {
    pub effect_marker: PhantomData<E>,
    pub operation_tag: OperationTag,
    pub arguments: Box<dyn Any + Send + Sync>,
    pub continuation: Continuation<E, A>,
}

/// Internal structure for deferred `flat_map` (stack safety).
pub struct EffFlatMap<E: Effect, A: 'static> {
    pub source: Box<dyn Any + 'static>,
    pub transform: Continuation<E, A>,
}

/// Internal representation of an effectful computation.
pub enum EffInner<E: Effect, A: 'static> {
    Pure(A),
    Impure(EffOperation<E, A>),
    FlatMap(Box<EffFlatMap<E, A>>),
}

/// An effectful computation.
///
/// `Eff<E, A>` represents a computation that may use effect `E` and
/// produces a value of type `A`. Effects are not executed until
/// a handler is applied.
///
/// # Type Parameters
///
/// - `E`: The effect type this computation uses
/// - `A`: The result type of the computation
///
/// # Monad Laws
///
/// `Eff` satisfies the monad laws:
///
/// 1. **Left Identity**: `Eff::pure(a).flat_map(f) == f(a)`
/// 2. **Right Identity**: `m.flat_map(Eff::pure) == m`
/// 3. **Associativity**: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
///
/// # Stack Safety
///
/// Deep `flat_map` chains are handled using a deferred evaluation strategy
/// combined with iterative execution, preventing stack overflow.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
///
/// let computation = Eff::<NoEffect, i32>::pure(21)
///     .fmap(|x| x * 2);
///
/// let result = PureHandler.run(computation);
/// assert_eq!(result, 42);
/// ```
pub struct Eff<E: Effect, A: 'static> {
    pub(super) inner: EffInner<E, A>,
}

impl<E: Effect, A: 'static> Eff<E, A> {
    /// Creates a pure computation that immediately returns the given value.
    ///
    /// This is the `return` / `pure` operation for the Eff monad.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(42);
    /// assert!(computation.is_pure());
    /// ```
    #[must_use]
    #[inline]
    pub const fn pure(value: A) -> Self {
        Self {
            inner: EffInner::Pure(value),
        }
    }

    /// Checks if this computation is a pure value (no pending effects).
    ///
    /// Note: This only checks the immediate structure. A computation
    /// may appear pure but have effects in nested `flat_map` chains.
    #[must_use]
    #[inline]
    pub const fn is_pure(&self) -> bool {
        matches!(&self.inner, EffInner::Pure(_))
    }

    /// Creates an effect operation.
    ///
    /// This function is used by effect definitions to create operations.
    /// Typically accessed through `define_effect!` macro.
    ///
    /// # Note
    ///
    /// This is a low-level API. Prefer using the `define_effect!` macro
    /// to create effects with proper operation tags.
    ///
    /// # Panics
    ///
    /// The returned computation will panic during handler execution if
    /// the handler provides a result of an incorrect type. This indicates
    /// a bug in the handler implementation.
    pub fn perform_raw<R: 'static>(
        operation_tag: OperationTag,
        arguments: impl Any + Send + Sync + 'static,
    ) -> Eff<E, R> {
        Eff {
            inner: EffInner::Impure(EffOperation {
                effect_marker: PhantomData,
                operation_tag,
                arguments: Box::new(arguments),
                continuation: Box::new(|result| {
                    let value = *result
                        .downcast::<R>()
                        .expect("Type mismatch in Eff::perform_raw");
                    Eff::pure(value)
                }),
            }),
        }
    }

    /// Normalizes the computation by evaluating `FlatMap` chains.
    ///
    /// This method converts `FlatMap` variants to `Pure` or `Impure`,
    /// using an iterative approach for stack safety.
    pub(crate) fn normalize(self) -> Self {
        match self.inner {
            EffInner::Pure(_) | EffInner::Impure(_) => self,
            EffInner::FlatMap(flat_map) => Self::normalize_iteratively(*flat_map),
        }
    }

    fn normalize_iteratively(initial_flat_map: EffFlatMap<E, A>) -> Self {
        let mut current_result = (initial_flat_map.transform)(initial_flat_map.source);

        loop {
            match current_result.inner {
                EffInner::Pure(_) | EffInner::Impure(_) => return current_result,
                EffInner::FlatMap(next_flat_map) => {
                    current_result = (next_flat_map.transform)(next_flat_map.source);
                }
            }
        }
    }

    /// Applies a function to the result of this computation.
    ///
    /// This is the `fmap` / `map` operation (Functor).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(21)
    ///     .fmap(|x| x * 2);
    ///
    /// let result = PureHandler.run(computation);
    /// assert_eq!(result, 42);
    /// ```
    pub fn fmap<B: 'static, F>(self, function: F) -> Eff<E, B>
    where
        F: FnOnce(A) -> B + 'static,
    {
        self.flat_map(|value| Eff::pure(function(value)))
    }

    /// Chains this computation with another that depends on its result.
    ///
    /// This is the `bind` / `>>=` operation (Monad).
    ///
    /// Uses deferred evaluation for stack safety.
    ///
    /// # Panics
    ///
    /// This method may panic during handler execution if there is a type
    /// mismatch in the internal type-erased continuation chain. This should
    /// not happen in normal usage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(10)
    ///     .flat_map(|x| Eff::pure(x + 5));
    ///
    /// let result = PureHandler.run(computation);
    /// assert_eq!(result, 15);
    /// ```
    pub fn flat_map<B: 'static, F>(self, function: F) -> Eff<E, B>
    where
        F: FnOnce(A) -> Eff<E, B> + 'static,
    {
        match self.inner {
            EffInner::Pure(value) => function(value),
            EffInner::Impure(operation) => Eff {
                inner: EffInner::Impure(EffOperation {
                    effect_marker: operation.effect_marker,
                    operation_tag: operation.operation_tag,
                    arguments: operation.arguments,
                    continuation: Box::new(move |result| {
                        let next = (operation.continuation)(result);
                        Eff {
                            inner: EffInner::FlatMap(Box::new(EffFlatMap {
                                source: Box::new(next),
                                transform: Box::new(move |source| {
                                    let eff = *source.downcast::<Self>().unwrap();
                                    eff.flat_map(function)
                                }),
                            })),
                        }
                    }),
                }),
            },
            EffInner::FlatMap(flat_map) => {
                let EffFlatMap { source, transform } = *flat_map;
                Eff {
                    inner: EffInner::FlatMap(Box::new(EffFlatMap {
                        source,
                        transform: Box::new(move |src| {
                            let next = transform(src);
                            next.flat_map(function)
                        }),
                    })),
                }
            }
        }
    }

    /// Alias for `flat_map`.
    #[inline]
    pub fn and_then<B: 'static, F>(self, function: F) -> Eff<E, B>
    where
        F: FnOnce(A) -> Eff<E, B> + 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two computations, discarding the first result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(10)
    ///     .then(Eff::pure(42));
    ///
    /// let result = PureHandler.run(computation);
    /// assert_eq!(result, 42);
    /// ```
    #[inline]
    pub fn then<B: 'static>(self, next: Eff<E, B>) -> Eff<E, B> {
        self.flat_map(|_| next)
    }

    /// Combines two computations using a binary function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(10)
    ///     .map2(Eff::pure(20), |a, b| a + b);
    ///
    /// let result = PureHandler.run(computation);
    /// assert_eq!(result, 30);
    /// ```
    pub fn map2<B: 'static, C: 'static, F>(self, other: Eff<E, B>, function: F) -> Eff<E, C>
    where
        F: FnOnce(A, B) -> C + 'static,
    {
        self.flat_map(|value_a| other.fmap(|value_b| function(value_a, value_b)))
    }

    /// Combines two computations into a tuple.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
    ///
    /// let computation = Eff::<NoEffect, i32>::pure(1)
    ///     .product(Eff::pure(2));
    ///
    /// let result = PureHandler.run(computation);
    /// assert_eq!(result, (1, 2));
    /// ```
    #[inline]
    pub fn product<B: 'static>(self, other: Eff<E, B>) -> Eff<E, (A, B)> {
        self.map2(other, |value_a, value_b| (value_a, value_b))
    }
}

// =============================================================================
// Note on TypeClass implementations
// =============================================================================
//
// Eff requires 'static bounds for its type parameters due to the use of
// type-erased continuations (Box<dyn FnOnce(...)>). This makes it incompatible
// with the current typeclass trait definitions which don't have 'static bounds.
//
// Instead of implementing TypeConstructor, Functor, Applicative, and Monad traits,
// Eff provides equivalent functionality through its own methods:
//
// - `pure`: Equivalent to Applicative::pure
// - `fmap`: Equivalent to Functor::fmap
// - `flat_map` / `and_then`: Equivalent to Monad::flat_map
// - `map2`, `product`: Equivalent to Applicative operations
//
// Future work: Consider creating separate traits with 'static bounds for
// effect-based computations, or using GATs more extensively.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::algebraic::NoEffect;
    use rstest::rstest;

    #[rstest]
    fn eff_pure_creates_pure_value() {
        let eff: Eff<NoEffect, i32> = Eff::pure(42);
        assert!(eff.is_pure());
    }

    #[rstest]
    fn eff_pure_with_string() {
        let eff: Eff<NoEffect, String> = Eff::pure("hello".to_string());
        assert!(eff.is_pure());
    }

    #[rstest]
    fn eff_pure_with_complex_type() {
        let eff: Eff<NoEffect, Vec<i32>> = Eff::pure(vec![1, 2, 3]);
        assert!(eff.is_pure());
    }

    #[rstest]
    fn eff_fmap_on_pure_produces_pure() {
        let eff: Eff<NoEffect, i32> = Eff::pure(21);
        let mapped = eff.fmap(|x| x * 2);
        assert!(mapped.is_pure());
    }

    #[rstest]
    fn eff_flat_map_on_pure_produces_result_directly() {
        let eff: Eff<NoEffect, i32> = Eff::pure(10);
        let result = eff.flat_map(|x| Eff::pure(x + 5));
        assert!(result.is_pure());
    }

    #[rstest]
    fn eff_and_then_is_alias_for_flat_map() {
        let eff: Eff<NoEffect, i32> = Eff::pure(10);
        let result = eff.and_then(|x| Eff::pure(x + 5));
        assert!(result.is_pure());
    }

    #[rstest]
    fn eff_then_discards_first_result() {
        let first: Eff<NoEffect, i32> = Eff::pure(10);
        let second: Eff<NoEffect, &str> = Eff::pure("result");
        let result = first.then(second);
        assert!(result.is_pure());
    }

    #[rstest]
    fn eff_map2_combines_two_pure_values() {
        let first: Eff<NoEffect, i32> = Eff::pure(10);
        let second: Eff<NoEffect, i32> = Eff::pure(20);
        let result = first.map2(second, |a, b| a + b);
        assert!(result.is_pure());
    }

    #[rstest]
    fn eff_product_creates_tuple() {
        let first: Eff<NoEffect, i32> = Eff::pure(1);
        let second: Eff<NoEffect, &str> = Eff::pure("hello");
        let result = first.product(second);
        assert!(result.is_pure());
    }

    #[rstest]
    fn operation_tag_new_creates_tag() {
        let tag = OperationTag::new(42);
        assert_eq!(tag.0, 42);
    }

    #[rstest]
    fn operation_tag_equality() {
        let tag1 = OperationTag::new(1);
        let tag2 = OperationTag::new(1);
        let tag3 = OperationTag::new(2);
        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[rstest]
    fn operation_tag_is_debug() {
        let tag = OperationTag::new(42);
        let debug_string = format!("{tag:?}");
        assert!(debug_string.contains("42"));
    }

    #[rstest]
    fn operation_tag_is_clone() {
        let tag = OperationTag::new(42);
        let cloned = tag;
        assert_eq!(tag, cloned);
    }

    #[rstest]
    fn operation_tag_is_copy() {
        let tag = OperationTag::new(42);
        let copied = tag;
        assert_eq!(tag, copied);
    }
}
