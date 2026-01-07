//! Either type - a value that can be one of two types.
//!
//! This module provides the `Either<L, R>` type, which represents a value
//! that is either a `Left(L)` or a `Right(R)`. This is commonly used in
//! functional programming for:
//!
//! - Error handling (Left for errors, Right for success)
//! - Branching computations
//! - As the resume type for `Trampoline`
//!
//! # Examples
//!
//! ```rust
//! use lambars::control::Either;
//!
//! // Creating Either values
//! let left: Either<i32, String> = Either::Left(42);
//! let right: Either<i32, String> = Either::Right("hello".to_string());
//!
//! // Pattern matching
//! match left {
//!     Either::Left(n) => println!("Got left: {}", n),
//!     Either::Right(s) => println!("Got right: {}", s),
//! }
//!
//! // Using fold to handle both cases
//! let result = right.fold(
//!     |n| format!("Number: {}", n),
//!     |s| format!("String: {}", s),
//! );
//! assert_eq!(result, "String: hello");
//! ```
//!
//! # `IntoIterator` and Right Bias
//!
//! `Either<L, R>` implements `IntoIterator` with a right-biased behavior:
//!
//! - `Right(value)` yields `value` once (a 1-element iterator)
//! - `Left(_)` yields nothing (an empty iterator)
//!
//! This is consistent with the Scala `Either` type and allows seamless
//! integration with the `for_!` macro and Rust's `for` loops.
//!
//! ## Using with for_! macro
//!
//! ```rust
//! use lambars::{control::Either, for_};
//!
//! // Single Either
//! let result = for_! {
//!     value <= Either::<String, i32>::Right(42);
//!     yield value * 2
//! };
//! assert_eq!(result, vec![84]);
//!
//! // Flattening Vec<Either>
//! let eithers = vec![
//!     Either::<String, i32>::Right(1),
//!     Either::Left("error".to_string()),
//!     Either::Right(3),
//! ];
//! let result = for_! {
//!     either <= eithers;
//!     value <= either;
//!     yield value * 2
//! };
//! assert_eq!(result, vec![2, 6]);
//! ```
//!
//! ## Using with Rust's for loop
//!
//! ```rust
//! use lambars::control::Either;
//!
//! let right: Either<String, i32> = Either::Right(42);
//! for value in right {
//!     println!("Got value: {}", value);
//! }
//!
//! let left: Either<String, i32> = Either::Left("error".to_string());
//! for value in left {
//!     // This block never executes
//!     println!("Got value: {}", value);
//! }
//! ```
//!
//! ## Scala Correspondence
//!
//! This behavior corresponds to Scala's for-comprehension with Either:
//!
//! ```text
//! // Scala
//! val result = for {
//!   x <- Right(42): Either[String, Int]
//! } yield x * 2
//! // result: Either[String, Int] = Right(84)
//! ```

use std::fmt;
use std::hash::Hash;

/// A value that can be one of two types.
///
/// `Either<L, R>` represents a value that is either `Left(L)` or `Right(R)`.
/// By convention:
/// - `Left` is often used to represent failure, error, or the first alternative
/// - `Right` is often used to represent success or the second alternative
///
/// # Type Parameters
///
/// * `L` - The type of the left value
/// * `R` - The type of the right value
///
/// # Examples
///
/// ```rust
/// use lambars::control::Either;
///
/// let success: Either<String, i32> = Either::Right(42);
/// let failure: Either<String, i32> = Either::Left("error".to_string());
///
/// // Map over the right value
/// let doubled = success.map_right(|x| x * 2);
/// assert_eq!(doubled, Either::Right(84));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Either<L, R> {
    /// The left variant, conventionally representing failure or the first alternative.
    Left(L),
    /// The right variant, conventionally representing success or the second alternative.
    Right(R),
}

impl<L, R> Either<L, R> {
    // =========================================================================
    // Type Checking
    // =========================================================================

    /// Returns `true` if this is a `Left` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert!(left.is_left());
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert!(!right.is_left());
    /// ```
    #[inline]
    pub const fn is_left(&self) -> bool {
        matches!(self, Self::Left(_))
    }

    /// Returns `true` if this is a `Right` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert!(right.is_right());
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert!(!left.is_right());
    /// ```
    #[inline]
    pub const fn is_right(&self) -> bool {
        matches!(self, Self::Right(_))
    }

    // =========================================================================
    // Value Extraction (Consuming)
    // =========================================================================

    /// Converts the `Either` into an `Option<L>`, consuming the either.
    ///
    /// Returns `Some(l)` if this is `Left(l)`, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.left(), Some(42));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.left(), None);
    /// ```
    #[inline]
    pub fn left(self) -> Option<L> {
        match self {
            Self::Left(value) => Some(value),
            Self::Right(_) => None,
        }
    }

    /// Converts the `Either` into an `Option<R>`, consuming the either.
    ///
    /// Returns `Some(r)` if this is `Right(r)`, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.right(), Some("hello".to_string()));
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.right(), None);
    /// ```
    #[inline]
    pub fn right(self) -> Option<R> {
        match self {
            Self::Left(_) => None,
            Self::Right(value) => Some(value),
        }
    }

    // =========================================================================
    // Reference Extraction (Non-consuming)
    // =========================================================================

    /// Returns a reference to the left value if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.left_ref(), Some(&42));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.left_ref(), None);
    /// ```
    #[inline]
    pub const fn left_ref(&self) -> Option<&L> {
        match self {
            Self::Left(value) => Some(value),
            Self::Right(_) => None,
        }
    }

    /// Returns a reference to the right value if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.right_ref(), Some(&"hello".to_string()));
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.right_ref(), None);
    /// ```
    #[inline]
    pub const fn right_ref(&self) -> Option<&R> {
        match self {
            Self::Left(_) => None,
            Self::Right(value) => Some(value),
        }
    }

    // =========================================================================
    // Mapping Operations
    // =========================================================================

    /// Applies a function to the left value if present.
    ///
    /// If this is `Left(l)`, returns `Left(function(l))`.
    /// If this is `Right(r)`, returns `Right(r)` unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// let result = left.map_left(|x| x * 2);
    /// assert_eq!(result, Either::Left(84));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// let result = right.map_left(|x: i32| x * 2);
    /// assert_eq!(result, Either::Right("hello".to_string()));
    /// ```
    #[inline]
    pub fn map_left<T, F>(self, function: F) -> Either<T, R>
    where
        F: FnOnce(L) -> T,
    {
        match self {
            Self::Left(value) => Either::Left(function(value)),
            Self::Right(value) => Either::Right(value),
        }
    }

    /// Applies a function to the right value if present.
    ///
    /// If this is `Right(r)`, returns `Right(function(r))`.
    /// If this is `Left(l)`, returns `Left(l)` unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// let result = right.map_right(|s| s.len());
    /// assert_eq!(result, Either::Right(5));
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// let result = left.map_right(|s: String| s.len());
    /// assert_eq!(result, Either::Left(42));
    /// ```
    #[inline]
    pub fn map_right<T, F>(self, function: F) -> Either<L, T>
    where
        F: FnOnce(R) -> T,
    {
        match self {
            Self::Left(value) => Either::Left(value),
            Self::Right(value) => Either::Right(function(value)),
        }
    }

    /// Applies one of two functions depending on whether this is Left or Right.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// let result = left.bimap(|x| x * 2, |s: String| s.len());
    /// assert_eq!(result, Either::Left(84));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// let result = right.bimap(|x: i32| x * 2, |s| s.len());
    /// assert_eq!(result, Either::Right(5));
    /// ```
    #[inline]
    pub fn bimap<T, U, F, G>(self, left_function: F, right_function: G) -> Either<T, U>
    where
        F: FnOnce(L) -> T,
        G: FnOnce(R) -> U,
    {
        match self {
            Self::Left(value) => Either::Left(left_function(value)),
            Self::Right(value) => Either::Right(right_function(value)),
        }
    }

    // =========================================================================
    // Fold Operation
    // =========================================================================

    /// Eliminates the Either by applying one of two functions.
    ///
    /// This is also known as "case analysis" or "pattern matching" as a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// let result = left.fold(|x| x.to_string(), |s| s);
    /// assert_eq!(result, "42");
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// let result = right.fold(|x: i32| x.to_string(), |s| s);
    /// assert_eq!(result, "hello");
    /// ```
    #[inline]
    pub fn fold<T, F, G>(self, left_function: F, right_function: G) -> T
    where
        F: FnOnce(L) -> T,
        G: FnOnce(R) -> T,
    {
        match self {
            Self::Left(value) => left_function(value),
            Self::Right(value) => right_function(value),
        }
    }

    // =========================================================================
    // Swap Operation
    // =========================================================================

    /// Swaps the Left and Right variants.
    ///
    /// `Left(l)` becomes `Right(l)`, and `Right(r)` becomes `Left(r)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.swap(), Either::Right(42));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.swap(), Either::Left("hello".to_string()));
    /// ```
    #[inline]
    pub fn swap(self) -> Either<R, L> {
        match self {
            Self::Left(value) => Either::Right(value),
            Self::Right(value) => Either::Left(value),
        }
    }

    // =========================================================================
    // Unwrap Operations
    // =========================================================================

    /// Returns the left value, consuming the either.
    ///
    /// # Panics
    ///
    /// Panics if this is a `Right` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.unwrap_left(), 42);
    /// ```
    #[inline]
    pub fn unwrap_left(self) -> L {
        match self {
            Self::Left(value) => value,
            Self::Right(_) => panic!("called `Either::unwrap_left()` on a `Right` value"),
        }
    }

    /// Returns the right value, consuming the either.
    ///
    /// # Panics
    ///
    /// Panics if this is a `Left` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.unwrap_right(), "hello".to_string());
    /// ```
    #[inline]
    pub fn unwrap_right(self) -> R {
        match self {
            Self::Left(_) => panic!("called `Either::unwrap_right()` on a `Left` value"),
            Self::Right(value) => value,
        }
    }

    // =========================================================================
    // Conversion Operations
    // =========================================================================

    /// Converts into a pair of `Option`s.
    ///
    /// Returns `(Some(l), None)` for `Left(l)` and `(None, Some(r))` for `Right(r)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.into_options(), (Some(42), None));
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.into_options(), (None, Some("hello".to_string())));
    /// ```
    #[inline]
    pub fn into_options(self) -> (Option<L>, Option<R>) {
        match self {
            Self::Left(value) => (Some(value), None),
            Self::Right(value) => (None, Some(value)),
        }
    }

    // =========================================================================
    // Iterator Methods
    // =========================================================================

    /// Returns an iterator over a reference to the Right value.
    ///
    /// If this is `Right(value)`, the iterator yields `&value` once.
    /// If this is `Left(_)`, the iterator yields nothing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<String, i32> = Either::Right(42);
    /// let mut iter = right.iter();
    /// assert_eq!(iter.next(), Some(&42));
    /// assert_eq!(iter.next(), None);
    ///
    /// let left: Either<String, i32> = Either::Left("error".to_string());
    /// let mut iter = left.iter();
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> EitherIterator<'_, R> {
        <&Self as IntoIterator>::into_iter(self)
    }
}

// =============================================================================
// Default-based Operations
// =============================================================================

impl<L: Default, R> Either<L, R> {
    /// Returns the left value, or default if this is a Right.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.left_or_default(), 42);
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.left_or_default(), 0);
    /// ```
    #[inline]
    pub fn left_or_default(self) -> L {
        match self {
            Self::Left(value) => value,
            Self::Right(_) => L::default(),
        }
    }
}

impl<L, R: Default> Either<L, R> {
    /// Returns the right value, or default if this is a Left.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<i32, String> = Either::Right("hello".to_string());
    /// assert_eq!(right.right_or_default(), "hello".to_string());
    ///
    /// let left: Either<i32, String> = Either::Left(42);
    /// assert_eq!(left.right_or_default(), String::new());
    /// ```
    #[inline]
    pub fn right_or_default(self) -> R {
        match self {
            Self::Left(_) => R::default(),
            Self::Right(value) => value,
        }
    }
}

// =============================================================================
// Debug Implementation
// =============================================================================

impl<L: fmt::Debug, R: fmt::Debug> fmt::Debug for Either<L, R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(value) => formatter.debug_tuple("Left").field(value).finish(),
            Self::Right(value) => formatter.debug_tuple("Right").field(value).finish(),
        }
    }
}

impl<L: fmt::Display, R: fmt::Display> fmt::Display for Either<L, R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(value) => write!(formatter, "Left({value})"),
            Self::Right(value) => write!(formatter, "Right({value})"),
        }
    }
}

// =============================================================================
// From Implementations
// =============================================================================

impl<L, R> From<Result<R, L>> for Either<L, R> {
    /// Converts a `Result` to an `Either`.
    ///
    /// `Ok(r)` becomes `Right(r)`, and `Err(e)` becomes `Left(e)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let ok: Result<i32, String> = Ok(42);
    /// let either: Either<String, i32> = ok.into();
    /// assert_eq!(either, Either::Right(42));
    ///
    /// let err: Result<i32, String> = Err("error".to_string());
    /// let either: Either<String, i32> = err.into();
    /// assert_eq!(either, Either::Left("error".to_string()));
    /// ```
    #[inline]
    fn from(result: Result<R, L>) -> Self {
        match result {
            Ok(value) => Self::Right(value),
            Err(error) => Self::Left(error),
        }
    }
}

impl<L, R> From<Either<L, R>> for Result<R, L> {
    /// Converts an `Either` to a `Result`.
    ///
    /// `Right(r)` becomes `Ok(r)`, and `Left(l)` becomes `Err(l)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<String, i32> = Either::Right(42);
    /// let result: Result<i32, String> = right.into();
    /// assert_eq!(result, Ok(42));
    ///
    /// let left: Either<String, i32> = Either::Left("error".to_string());
    /// let result: Result<i32, String> = left.into();
    /// assert_eq!(result, Err("error".to_string()));
    /// ```
    #[inline]
    fn from(either: Either<L, R>) -> Self {
        match either {
            Either::Left(value) => Err(value),
            Either::Right(value) => Ok(value),
        }
    }
}

// =============================================================================
// Iterator Types
// =============================================================================

/// An owning iterator over the Right value of an [`Either`].
///
/// This struct is created by the [`into_iter`] method on [`Either`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
///
/// # Right Bias
///
/// This iterator yields exactly one element if the Either was `Right(value)`,
/// and zero elements if it was `Left(_)`. This is consistent with the
/// right-biased behavior of Either in functional programming.
///
/// # Examples
///
/// ```rust
/// use lambars::control::Either;
///
/// let right: Either<String, i32> = Either::Right(42);
/// let mut iterator = right.into_iter();
/// assert_eq!(iterator.next(), Some(42));
/// assert_eq!(iterator.next(), None);
///
/// let left: Either<String, i32> = Either::Left("error".to_string());
/// let mut iterator = left.into_iter();
/// assert_eq!(iterator.next(), None);
/// ```
pub struct EitherIntoIterator<R> {
    inner: std::option::IntoIter<R>,
}

impl<R> EitherIntoIterator<R> {
    /// Creates a new `EitherIntoIterator` from an optional value.
    ///
    /// This is an internal constructor used by the `IntoIterator` implementation.
    #[inline]
    fn new(value: Option<R>) -> Self {
        Self {
            inner: value.into_iter(),
        }
    }
}

impl<R> Iterator for EitherIntoIterator<R> {
    type Item = R;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.count()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n)
    }
}

impl<R> ExactSizeIterator for EitherIntoIterator<R> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<R> std::iter::FusedIterator for EitherIntoIterator<R> {}

impl<R> DoubleEndedIterator for EitherIntoIterator<R> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

/// An iterator over a reference to the Right value of an [`Either`].
///
/// This struct is created by the [`into_iter`] method on [`&Either`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
///
/// # Right Bias
///
/// This iterator yields exactly one reference if the Either is `Right(value)`,
/// and zero references if it is `Left(_)`. The original Either is not consumed.
///
/// # Examples
///
/// ```rust
/// use lambars::control::Either;
///
/// let right: Either<String, i32> = Either::Right(42);
/// for value in &right {
///     assert_eq!(*value, 42);
/// }
/// // right is still usable
/// assert!(right.is_right());
/// ```
pub struct EitherIterator<'a, R> {
    inner: std::option::IntoIter<&'a R>,
}

impl<'a, R> EitherIterator<'a, R> {
    /// Creates a new `EitherIterator` from an optional reference.
    ///
    /// This is an internal constructor used by the `IntoIterator` implementation.
    #[inline]
    fn new(value: Option<&'a R>) -> Self {
        Self {
            inner: value.into_iter(),
        }
    }
}

impl<'a, R> Iterator for EitherIterator<'a, R> {
    type Item = &'a R;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.count()
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.inner.last()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n)
    }
}

impl<R> ExactSizeIterator for EitherIterator<'_, R> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<R> std::iter::FusedIterator for EitherIterator<'_, R> {}

impl<R> DoubleEndedIterator for EitherIterator<'_, R> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

// =============================================================================
// IntoIterator Implementation
// =============================================================================

impl<L, R> IntoIterator for Either<L, R> {
    type Item = R;
    type IntoIter = EitherIntoIterator<R>;

    /// Creates an owning iterator over the Right value.
    ///
    /// If this is `Right(value)`, the iterator yields `value` once.
    /// If this is `Left(_)`, the iterator yields nothing.
    ///
    /// This implements the right-biased behavior of Either, consistent with
    /// the Scala Either type and the Option type in Rust.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<String, i32> = Either::Right(42);
    /// let collected: Vec<i32> = right.into_iter().collect();
    /// assert_eq!(collected, vec![42]);
    ///
    /// let left: Either<String, i32> = Either::Left("error".to_string());
    /// let collected: Vec<i32> = left.into_iter().collect();
    /// assert_eq!(collected, vec![]);
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Right(value) => EitherIntoIterator::new(Some(value)),
            Self::Left(_) => EitherIntoIterator::new(None),
        }
    }
}

impl<'a, L, R> IntoIterator for &'a Either<L, R> {
    type Item = &'a R;
    type IntoIter = EitherIterator<'a, R>;

    /// Creates an iterator over a reference to the Right value.
    ///
    /// If this is `Right(value)`, the iterator yields `&value` once.
    /// If this is `Left(_)`, the iterator yields nothing.
    ///
    /// The original Either is not consumed and can be used again.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Either;
    ///
    /// let right: Either<String, i32> = Either::Right(42);
    /// for value in &right {
    ///     assert_eq!(*value, 42);
    /// }
    /// // right can still be used
    /// assert_eq!(right.right(), Some(42));
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Either::Right(value) => EitherIterator::new(Some(value)),
            Either::Left(_) => EitherIterator::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // EitherIntoIterator Tests - Step 1
    // =========================================================================

    #[rstest]
    fn test_either_into_iterator_new_some() {
        let iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.len(), 1);
    }

    #[rstest]
    fn test_either_into_iterator_new_none() {
        let iterator: EitherIntoIterator<i32> = EitherIntoIterator::new(None);
        assert_eq!(iterator.len(), 0);
    }

    // =========================================================================
    // EitherIntoIterator Tests - Step 2: Iterator
    // =========================================================================

    #[rstest]
    fn test_either_into_iterator_next_some() {
        let mut iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.next(), Some(42));
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_either_into_iterator_next_none() {
        let mut iterator: EitherIntoIterator<i32> = EitherIntoIterator::new(None);
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_either_into_iterator_size_hint_some() {
        let iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.size_hint(), (1, Some(1)));
    }

    #[rstest]
    fn test_either_into_iterator_size_hint_none() {
        let iterator: EitherIntoIterator<i32> = EitherIntoIterator::new(None);
        assert_eq!(iterator.size_hint(), (0, Some(0)));
    }

    // =========================================================================
    // EitherIntoIterator Tests - Step 3: Additional Traits
    // =========================================================================

    #[rstest]
    fn test_either_into_iterator_exact_size() {
        let iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.len(), 1);

        let iterator: EitherIntoIterator<i32> = EitherIntoIterator::new(None);
        assert_eq!(iterator.len(), 0);
    }

    #[rstest]
    fn test_either_into_iterator_fused() {
        let mut iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.next(), Some(42));
        assert_eq!(iterator.next(), None);
        // FusedIterator guarantees this continues to return None
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_either_into_iterator_double_ended() {
        let mut iterator = EitherIntoIterator::new(Some(42));
        assert_eq!(iterator.next_back(), Some(42));
        assert_eq!(iterator.next_back(), None);

        let mut iterator: EitherIntoIterator<i32> = EitherIntoIterator::new(None);
        assert_eq!(iterator.next_back(), None);
    }

    // =========================================================================
    // IntoIterator Tests - Step 4
    // =========================================================================

    #[rstest]
    fn test_right_into_iter_yields_value() {
        let right: Either<String, i32> = Either::Right(42);
        let mut iterator = right.into_iter();
        assert_eq!(iterator.next(), Some(42));
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_left_into_iter_yields_nothing() {
        let left: Either<String, i32> = Either::Left("error".to_string());
        let mut iterator = left.into_iter();
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_right_into_iter_collect() {
        let right: Either<String, i32> = Either::Right(42);
        let collected: Vec<i32> = right.into_iter().collect();
        assert_eq!(collected, vec![42]);
    }

    #[rstest]
    fn test_left_into_iter_collect() {
        let left: Either<String, i32> = Either::Left("error".to_string());
        let collected: Vec<i32> = left.into_iter().collect();
        assert_eq!(collected, Vec::<i32>::new());
    }

    #[rstest]
    fn test_into_iter_for_loop() {
        let right: Either<String, i32> = Either::Right(42);
        let mut sum = 0;
        for value in right {
            sum += value;
        }
        assert_eq!(sum, 42);

        let left: Either<String, i32> = Either::Left("error".to_string());
        let mut count = 0;
        for _ in left {
            count += 1;
        }
        assert_eq!(count, 0);
    }

    // =========================================================================
    // EitherIterator Tests - Step 5
    // =========================================================================

    #[rstest]
    fn test_either_iterator_new_some() {
        let value = 42;
        let iterator = EitherIterator::new(Some(&value));
        assert_eq!(iterator.len(), 1);
    }

    #[rstest]
    fn test_either_iterator_new_none() {
        let iterator: EitherIterator<'_, i32> = EitherIterator::new(None);
        assert_eq!(iterator.len(), 0);
    }

    #[rstest]
    fn test_either_iterator_next() {
        let value = 42;
        let mut iterator = EitherIterator::new(Some(&value));
        assert_eq!(iterator.next(), Some(&42));
        assert_eq!(iterator.next(), None);
    }

    // =========================================================================
    // IntoIterator for &Either Tests - Step 6
    // =========================================================================

    #[rstest]
    fn test_right_ref_into_iter() {
        let right: Either<String, i32> = Either::Right(42);
        let collected: Vec<&i32> = (&right).into_iter().collect();
        assert_eq!(collected, vec![&42]);
        // right is still usable
        assert!(right.is_right());
    }

    #[rstest]
    fn test_left_ref_into_iter() {
        let left: Either<String, i32> = Either::Left("error".to_string());
        let collected: Vec<&i32> = (&left).into_iter().collect();
        assert_eq!(collected, Vec::<&i32>::new());
        // left is still usable
        assert!(left.is_left());
    }

    #[rstest]
    fn test_ref_iter_does_not_consume() {
        let right: Either<String, i32> = Either::Right(42);

        // First iteration
        let mut sum = 0;
        for value in &right {
            sum += value;
        }
        assert_eq!(sum, 42);

        // Second iteration - right is still usable
        let collected: Vec<&i32> = (&right).into_iter().collect();
        assert_eq!(collected, vec![&42]);
    }

    #[rstest]
    fn test_ref_iter_flat_map() {
        let eithers = [
            Either::<String, i32>::Right(1),
            Either::Left("error".to_string()),
            Either::Right(3),
        ];

        let sum: i32 = eithers.iter().flat_map(|either| either.into_iter()).sum();
        assert_eq!(sum, 4); // 1 + 3
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_left() {
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(format!("{left}"), "Left(42)");
    }

    #[rstest]
    fn test_display_right() {
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(format!("{right}"), "Right(hello)");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_either_left_construction() {
        let value: Either<i32, String> = Either::Left(42);
        assert!(value.is_left());
        assert!(!value.is_right());
    }

    #[rstest]
    fn test_either_right_construction() {
        let value: Either<i32, String> = Either::Right("hello".to_string());
        assert!(value.is_right());
        assert!(!value.is_left());
    }

    #[rstest]
    fn test_result_conversion_roundtrip() {
        let ok: Result<i32, String> = Ok(42);
        let either: Either<String, i32> = ok.into();
        let result: Result<i32, String> = either.into();
        assert_eq!(result, Ok(42));

        let err: Result<i32, String> = Err("error".to_string());
        let either: Either<String, i32> = err.into();
        let result: Result<i32, String> = either.into();
        assert_eq!(result, Err("error".to_string()));
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_serialize_left() {
        let left: Either<i32, String> = Either::Left(42);
        let json = serde_json::to_string(&left).unwrap();
        assert_eq!(json, r#"{"Left":42}"#);
    }

    #[rstest]
    fn test_serialize_right() {
        let right: Either<i32, String> = Either::Right("hello".to_string());
        let json = serde_json::to_string(&right).unwrap();
        assert_eq!(json, r#"{"Right":"hello"}"#);
    }

    #[rstest]
    fn test_deserialize_left() {
        let json = r#"{"Left":42}"#;
        let either: Either<i32, String> = serde_json::from_str(json).unwrap();
        assert_eq!(either, Either::Left(42));
    }

    #[rstest]
    fn test_deserialize_right() {
        let json = r#"{"Right":"hello"}"#;
        let either: Either<i32, String> = serde_json::from_str(json).unwrap();
        assert_eq!(either, Either::Right("hello".to_string()));
    }

    #[rstest]
    fn test_roundtrip_left() {
        let original: Either<i32, String> = Either::Left(42);
        let json = serde_json::to_string(&original).unwrap();
        let restored: Either<i32, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_roundtrip_right() {
        let original: Either<i32, String> = Either::Right("hello".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let restored: Either<i32, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_variant_distinction() {
        let left: Either<i32, i32> = Either::Left(42);
        let right: Either<i32, i32> = Either::Right(42);
        let left_json = serde_json::to_string(&left).unwrap();
        let right_json = serde_json::to_string(&right).unwrap();
        assert_ne!(left_json, right_json);
    }
}
