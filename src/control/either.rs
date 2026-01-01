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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
