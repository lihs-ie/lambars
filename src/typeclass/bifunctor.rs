//! Bifunctor type class - mapping over two type parameters.
//!
//! This module provides the `Bifunctor` trait, which represents types with
//! two type parameters that can both have functions mapped over them.
//!
//! A `Bifunctor` is a generalization of `Functor` for types with two type
//! parameters. While `Functor` transforms `F<A>` to `F<B>`, `Bifunctor`
//! transforms `F<A, B>` to `F<C, D>`.
//!
//! # Laws
//!
//! All `Bifunctor` implementations must satisfy these laws:
//!
//! ## Identity Law
//!
//! Mapping identity functions over a bifunctor should return an equivalent bifunctor:
//!
//! ```text
//! bf.bimap(|x| x, |y| y) == bf
//! ```
//!
//! ## Composition Law
//!
//! Mapping composed functions should be equivalent to mapping them in sequence:
//!
//! ```text
//! bf.bimap(|x| f2(f1(x)), |y| g2(g1(y))) == bf.bimap(f1, g1).bimap(f2, g2)
//! ```
//!
//! ## first/second Consistency Law
//!
//! bimap is equivalent to composing first and second:
//!
//! ```text
//! bf.bimap(f, g) == bf.first(f).second(g) == bf.second(g).first(f)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::Bifunctor;
//! use lambars::control::Either;
//!
//! // Either as a Bifunctor
//! let left: Either<i32, String> = Either::Left(42);
//! let mapped = left.bimap(|x| x * 2, |s: String| s.len());
//! assert_eq!(mapped, Either::Left(84));
//!
//! let right: Either<i32, String> = Either::Right("hello".to_string());
//! let mapped = right.bimap(|x: i32| x * 2, |s| s.len());
//! assert_eq!(mapped, Either::Right(5));
//! ```
//!
//! # Relationship with Functor
//!
//! For right-biased types like `Either` and `Result`, `Bifunctor::second`
//! is equivalent to `Functor::fmap`:
//!
//! ```rust
//! use lambars::typeclass::{Functor, Bifunctor};
//!
//! let result: Result<i32, String> = Ok(42);
//! let by_fmap = result.clone().fmap(|x| x * 2);
//! let by_second = result.second(|x| x * 2);
//! assert_eq!(by_fmap, by_second);
//! ```
//!
//! # Type Parameter Order for Result
//!
//! `Result<T, E>` is implemented as `Bifunctor<E, T>`:
//! - `first`: transforms the error type (E) - equivalent to `map_err`
//! - `second`: transforms the success type (T) - equivalent to `map`
//!
//! This ordering ensures consistency with `Functor` where `fmap` operates
//! on the success value.
//!
//! # Design Note: Trait Constraints
//!
//! Unlike `Functor`, `Bifunctor` does not require `TypeConstructor` or `'static`
//! constraints. This is intentional:
//!
//! - `Bifunctor` uses GAT (`type Target<C, D>`) to express the result type,
//!   avoiding the need for a separate type constructor trait.
//! - No `'static` is required because the default implementations of `first`
//!   and `second` only use identity closures that move values without capturing
//!   any references.

use crate::control::Either;

/// A type class for types with two type parameters that can have functions
/// mapped over both.
///
/// See module-level documentation for laws and detailed examples.
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Bifunctor;
/// use lambars::control::Either;
///
/// let either: Either<i32, String> = Either::Left(42);
/// let result = either.bimap(|x| x * 2, |s: String| s.len());
/// assert_eq!(result, Either::Left(84));
/// ```
pub trait Bifunctor<A, B> {
    /// The resulting type constructor after applying the transformation.
    ///
    /// For `Either<L, R>`, `Target<C, D> = Either<C, D>`.
    /// For `Result<T, E>` (implemented as `Bifunctor<E, T>`), `Target<C, D> = Result<D, C>`.
    /// For `(A, B)`, `Target<C, D> = (C, D)`.
    type Target<C, D>;

    /// Applies two functions to both type parameters simultaneously.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    ///
    /// let tuple = (42, "hello".to_string());
    /// let result = tuple.bimap(|x| x * 2, |s| s.len());
    /// assert_eq!(result, (84, 5));
    /// ```
    fn bimap<C, D, F, G>(self, first_function: F, second_function: G) -> Self::Target<C, D>
    where
        F: FnOnce(A) -> C,
        G: FnOnce(B) -> D;

    /// Applies a function to the first type parameter only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    /// use lambars::control::Either;
    ///
    /// let either: Either<i32, String> = Either::Left(42);
    /// let result = either.first(|x| x.to_string());
    /// assert_eq!(result, Either::Left("42".to_string()));
    /// ```
    #[inline]
    fn first<C, F>(self, function: F) -> Self::Target<C, B>
    where
        F: FnOnce(A) -> C,
        Self: Sized,
    {
        self.bimap(function, |b| b)
    }

    /// Applies a function to the second type parameter only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    /// use lambars::control::Either;
    ///
    /// let either: Either<i32, String> = Either::Right("hello".to_string());
    /// let result = either.second(|s| s.len());
    /// assert_eq!(result, Either::Right(5));
    /// ```
    #[inline]
    fn second<D, G>(self, function: G) -> Self::Target<A, D>
    where
        G: FnOnce(B) -> D,
        Self: Sized,
    {
        self.bimap(|a| a, function)
    }

    /// Applies two functions to references of both type parameters without consuming self.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    ///
    /// let tuple = (42, "hello".to_string());
    /// let result = tuple.bimap_ref(|x| x * 2, |s| s.len());
    /// assert_eq!(tuple.0, 42); // tuple is still available
    /// assert_eq!(result, (84, 5));
    /// ```
    fn bimap_ref<C, D, F, G>(&self, first_function: F, second_function: G) -> Self::Target<C, D>
    where
        F: FnOnce(&A) -> C,
        G: FnOnce(&B) -> D;

    /// Applies a function to a reference of the first type parameter.
    ///
    /// Requires `B: Clone` because the untransformed value must be cloned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    /// use lambars::control::Either;
    ///
    /// let either: Either<String, i32> = Either::Left("hello".to_string());
    /// let result = either.first_ref(|s| s.len());
    /// assert!(either.is_left()); // either is still available
    /// assert_eq!(result, Either::Left(5));
    /// ```
    #[inline]
    fn first_ref<C, F>(&self, function: F) -> Self::Target<C, B>
    where
        B: Clone,
        F: FnOnce(&A) -> C,
        Self: Sized,
    {
        self.bimap_ref(function, |b| b.clone())
    }

    /// Applies a function to a reference of the second type parameter.
    ///
    /// Requires `A: Clone` because the untransformed value must be cloned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Bifunctor;
    /// use lambars::control::Either;
    ///
    /// let either: Either<String, i32> = Either::Right(42);
    /// let result = either.second_ref(|n| n * 2);
    /// assert!(either.is_right()); // either is still available
    /// assert_eq!(result, Either::Right(84));
    /// ```
    #[inline]
    fn second_ref<D, G>(&self, function: G) -> Self::Target<A, D>
    where
        A: Clone,
        G: FnOnce(&B) -> D,
        Self: Sized,
    {
        self.bimap_ref(|a| a.clone(), function)
    }
}

impl<L, R> Bifunctor<L, R> for Either<L, R> {
    type Target<C, D> = Either<C, D>;

    #[inline]
    fn bimap<C, D, F, G>(self, first_function: F, second_function: G) -> Either<C, D>
    where
        F: FnOnce(L) -> C,
        G: FnOnce(R) -> D,
    {
        match self {
            Self::Left(left) => Either::Left(first_function(left)),
            Self::Right(right) => Either::Right(second_function(right)),
        }
    }

    #[inline]
    fn first<C, F>(self, function: F) -> Either<C, R>
    where
        F: FnOnce(L) -> C,
    {
        match self {
            Self::Left(left) => Either::Left(function(left)),
            Self::Right(right) => Either::Right(right),
        }
    }

    #[inline]
    fn second<D, G>(self, function: G) -> Either<L, D>
    where
        G: FnOnce(R) -> D,
    {
        match self {
            Self::Left(left) => Either::Left(left),
            Self::Right(right) => Either::Right(function(right)),
        }
    }

    #[inline]
    fn bimap_ref<C, D, F, G>(&self, first_function: F, second_function: G) -> Either<C, D>
    where
        F: FnOnce(&L) -> C,
        G: FnOnce(&R) -> D,
    {
        match self {
            Self::Left(left) => Either::Left(first_function(left)),
            Self::Right(right) => Either::Right(second_function(right)),
        }
    }

    #[inline]
    fn first_ref<C, F>(&self, function: F) -> Either<C, R>
    where
        R: Clone,
        F: FnOnce(&L) -> C,
    {
        match self {
            Self::Left(left) => Either::Left(function(left)),
            Self::Right(right) => Either::Right(right.clone()),
        }
    }

    #[inline]
    fn second_ref<D, G>(&self, function: G) -> Either<L, D>
    where
        L: Clone,
        G: FnOnce(&R) -> D,
    {
        match self {
            Self::Left(left) => Either::Left(left.clone()),
            Self::Right(right) => Either::Right(function(right)),
        }
    }
}

/// `Result<T, E>` is implemented as `Bifunctor<E, T>`:
/// - `first`: transforms the error type (E) - equivalent to `map_err`
/// - `second`: transforms the success type (T) - equivalent to `map`
///
/// This ordering ensures consistency with `Functor` where `fmap` operates
/// on the success value (T), making `Bifunctor::second` equivalent to
/// `Functor::fmap`.
impl<T, E> Bifunctor<E, T> for Result<T, E> {
    type Target<C, D> = Result<D, C>;

    #[inline]
    fn bimap<C, D, F, G>(self, first_function: F, second_function: G) -> Result<D, C>
    where
        F: FnOnce(E) -> C,
        G: FnOnce(T) -> D,
    {
        match self {
            Ok(value) => Ok(second_function(value)),
            Err(error) => Err(first_function(error)),
        }
    }

    #[inline]
    fn first<C, F>(self, function: F) -> Result<T, C>
    where
        F: FnOnce(E) -> C,
    {
        self.map_err(function)
    }

    #[inline]
    fn second<D, G>(self, function: G) -> Result<D, E>
    where
        G: FnOnce(T) -> D,
    {
        self.map(function)
    }

    #[inline]
    fn bimap_ref<C, D, F, G>(&self, first_function: F, second_function: G) -> Result<D, C>
    where
        F: FnOnce(&E) -> C,
        G: FnOnce(&T) -> D,
    {
        match self {
            Ok(value) => Ok(second_function(value)),
            Err(error) => Err(first_function(error)),
        }
    }

    #[inline]
    fn first_ref<C, F>(&self, function: F) -> Result<T, C>
    where
        T: Clone,
        F: FnOnce(&E) -> C,
    {
        match self {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(function(error)),
        }
    }

    #[inline]
    fn second_ref<D, G>(&self, function: G) -> Result<D, E>
    where
        E: Clone,
        G: FnOnce(&T) -> D,
    {
        match self {
            Ok(value) => Ok(function(value)),
            Err(error) => Err(error.clone()),
        }
    }
}

impl<A, B> Bifunctor<A, B> for (A, B) {
    type Target<C, D> = (C, D);

    #[inline]
    fn bimap<C, D, F, G>(self, first_function: F, second_function: G) -> (C, D)
    where
        F: FnOnce(A) -> C,
        G: FnOnce(B) -> D,
    {
        (first_function(self.0), second_function(self.1))
    }

    #[inline]
    fn first<C, F>(self, function: F) -> (C, B)
    where
        F: FnOnce(A) -> C,
    {
        (function(self.0), self.1)
    }

    #[inline]
    fn second<D, G>(self, function: G) -> (A, D)
    where
        G: FnOnce(B) -> D,
    {
        (self.0, function(self.1))
    }

    #[inline]
    fn bimap_ref<C, D, F, G>(&self, first_function: F, second_function: G) -> (C, D)
    where
        F: FnOnce(&A) -> C,
        G: FnOnce(&B) -> D,
    {
        (first_function(&self.0), second_function(&self.1))
    }

    #[inline]
    fn first_ref<C, F>(&self, function: F) -> (C, B)
    where
        B: Clone,
        F: FnOnce(&A) -> C,
    {
        (function(&self.0), self.1.clone())
    }

    #[inline]
    fn second_ref<D, G>(&self, function: G) -> (A, D)
    where
        A: Clone,
        G: FnOnce(&B) -> D,
    {
        (self.0.clone(), function(&self.1))
    }
}
