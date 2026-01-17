//! Validated type for parallel error accumulation.
//!
//! Unlike `Either` or `Result`, `Validated` accumulates all errors
//! when combining validations using Applicative operations (map2, map3).
//!
//! # Examples
//!
//! ```rust
//! use bank::domain::validation::{Validated, ValidationErrors};
//! use lambars::typeclass::Applicative;
//!
//! let valid1: Validated<i32> = Validated::valid(1);
//! let valid2: Validated<i32> = Validated::valid(2);
//! let result = valid1.map2(valid2, |a, b| a + b);
//! assert!(result.is_valid());
//!
//! let invalid1: Validated<i32> = Validated::invalid("error 1");
//! let invalid2: Validated<i32> = Validated::invalid("error 2");
//! let result = invalid1.map2(invalid2, |a, b| a + b);
//! // Both errors are accumulated
//! assert_eq!(result.errors().len(), 2);
//! ```

use std::fmt;

use lambars::typeclass::{Applicative, Functor, TypeConstructor};
use serde::{Deserialize, Serialize};

/// A single validation error with a code and message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code for programmatic handling.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

impl ValidationError {
    /// Creates a new validation error.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// A collection of validation errors.
///
/// This type is used as the error type for `Validated` and supports
/// accumulation via the `Semigroup` trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    /// Creates a new collection with a single error.
    #[must_use]
    pub fn single(error: ValidationError) -> Self {
        Self(vec![error])
    }

    /// Creates a new collection from a simple error message.
    #[must_use]
    pub fn from_message(message: impl Into<String>) -> Self {
        Self::single(ValidationError::new("VALIDATION_ERROR", message))
    }

    /// Returns the number of errors.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no errors.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.0.iter()
    }

    /// Combines two error collections.
    #[must_use]
    pub fn combine(mut self, mut other: Self) -> Self {
        self.0.append(&mut other.0);
        self
    }

    /// Converts to a vector of errors.
    #[must_use]
    pub fn into_vec(self) -> Vec<ValidationError> {
        self.0
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(formatter, "{}", messages.join("; "))
    }
}

impl std::error::Error for ValidationErrors {}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ValidationErrors {
    type Item = &'a ValidationError;
    type IntoIter = std::slice::Iter<'a, ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// A validation result that accumulates errors.
///
/// `Validated<A>` is either:
/// - `Valid(A)` - a successful validation with a value
/// - `Invalid(ValidationErrors)` - a failed validation with accumulated errors
///
/// Unlike `Either` or `Result`, when combining two `Invalid` values using
/// `map2` or `map3`, all errors are accumulated rather than short-circuiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validated<A> {
    /// A successful validation result.
    Valid(A),
    /// A failed validation with accumulated errors.
    Invalid(ValidationErrors),
}

impl<A> Validated<A> {
    /// Creates a valid result.
    #[must_use]
    pub const fn valid(value: A) -> Self {
        Self::Valid(value)
    }

    /// Creates an invalid result with a single error message.
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(ValidationErrors::from_message(message))
    }

    /// Creates an invalid result with a structured error.
    #[must_use]
    pub fn invalid_with_code(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid(ValidationErrors::single(ValidationError::new(
            code, message,
        )))
    }

    /// Creates an invalid result from multiple errors.
    #[must_use]
    pub const fn invalid_many(errors: ValidationErrors) -> Self {
        Self::Invalid(errors)
    }

    /// Returns true if this is a valid result.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid(_))
    }

    /// Returns true if this is an invalid result.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }

    /// Returns the errors if this is invalid, or an empty collection if valid.
    #[must_use]
    pub fn errors(&self) -> &ValidationErrors {
        match self {
            Self::Valid(_) => {
                // Return an empty static reference
                static EMPTY: std::sync::OnceLock<ValidationErrors> = std::sync::OnceLock::new();
                EMPTY.get_or_init(|| ValidationErrors(Vec::new()))
            }
            Self::Invalid(errors) => errors,
        }
    }

    /// Converts to an Option, discarding errors.
    #[must_use]
    pub fn to_option(self) -> Option<A> {
        match self {
            Self::Valid(value) => Some(value),
            Self::Invalid(_) => None,
        }
    }

    /// Converts to a Result.
    ///
    /// # Errors
    ///
    /// Returns `Err(ValidationErrors)` if this is an invalid result.
    pub fn to_result(self) -> Result<A, ValidationErrors> {
        match self {
            Self::Valid(value) => Ok(value),
            Self::Invalid(errors) => Err(errors),
        }
    }

    /// Unwraps the valid value, panicking if invalid.
    ///
    /// # Panics
    ///
    /// Panics if this is an invalid result.
    #[must_use]
    pub fn unwrap(self) -> A {
        match self {
            Self::Valid(value) => value,
            Self::Invalid(errors) => {
                panic!("called `Validated::unwrap()` on an Invalid value: {errors}")
            }
        }
    }
}

// =============================================================================
// TypeConstructor Implementation
// =============================================================================

impl<A> TypeConstructor for Validated<A> {
    type Inner = A;
    type WithType<B> = Validated<B>;
}

// =============================================================================
// Functor Implementation
// =============================================================================

impl<A> Functor for Validated<A> {
    fn fmap<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> B + 'static,
        B: 'static,
    {
        match self {
            Self::Valid(value) => Validated::Valid(function(value)),
            Self::Invalid(errors) => Validated::Invalid(errors),
        }
    }

    fn fmap_ref<B, F>(&self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(&Self::Inner) -> B + 'static,
        B: 'static,
    {
        match self {
            Self::Valid(value) => Validated::Valid(function(value)),
            Self::Invalid(errors) => Validated::Invalid(errors.clone()),
        }
    }
}

// =============================================================================
// Applicative Implementation
// =============================================================================

#[allow(clippy::use_self)]
impl<A: 'static> Applicative for Validated<A> {
    fn pure<B>(value: B) -> Validated<B>
    where
        B: 'static,
    {
        Validated::Valid(value)
    }

    fn map2<B, C, F>(self, other: Validated<B>, function: F) -> Validated<C>
    where
        F: FnOnce(A, B) -> C + 'static,
        B: 'static,
        C: 'static,
    {
        match (self, other) {
            (Self::Valid(a), Validated::Valid(b)) => Validated::Valid(function(a, b)),
            (Self::Invalid(e1), Validated::Invalid(e2)) => Validated::Invalid(e1.combine(e2)),
            (Self::Invalid(errors), Validated::Valid(_))
            | (Self::Valid(_), Validated::Invalid(errors)) => Validated::Invalid(errors),
        }
    }

    fn map3<B, C, D, F>(
        self,
        second: Validated<B>,
        third: Validated<C>,
        function: F,
    ) -> Validated<D>
    where
        F: FnOnce(A, B, C) -> D + 'static,
        B: 'static,
        C: 'static,
        D: 'static,
    {
        // Combine all three, accumulating errors
        match (self, second, third) {
            (Self::Valid(a), Validated::Valid(b), Validated::Valid(c)) => {
                Validated::Valid(function(a, b, c))
            }
            (Self::Invalid(e1), Validated::Invalid(e2), Validated::Invalid(e3)) => {
                Validated::Invalid(e1.combine(e2).combine(e3))
            }
            (Self::Invalid(e1), Validated::Invalid(e2), Validated::Valid(_)) => {
                Validated::Invalid(e1.combine(e2))
            }
            (Self::Invalid(e1), Validated::Valid(_), Validated::Invalid(e3)) => {
                Validated::Invalid(e1.combine(e3))
            }
            (Self::Valid(_), Validated::Invalid(e2), Validated::Invalid(e3)) => {
                Validated::Invalid(e2.combine(e3))
            }
            (Self::Invalid(e), Validated::Valid(_), Validated::Valid(_))
            | (Self::Valid(_), Validated::Invalid(e), Validated::Valid(_))
            | (Self::Valid(_), Validated::Valid(_), Validated::Invalid(e)) => Validated::Invalid(e),
        }
    }

    fn apply<B, Output>(self, other: Validated<B>) -> Validated<Output>
    where
        A: FnOnce(B) -> Output + 'static,
        B: 'static,
        Output: 'static,
    {
        self.map2(other, |f, b| f(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambars::typeclass::Applicative;
    use rstest::rstest;

    // =========================================================================
    // ValidationError Tests
    // =========================================================================

    #[rstest]
    fn validation_error_new_creates_error_with_code_and_message() {
        let error = ValidationError::new("INVALID_INPUT", "Input is invalid");

        assert_eq!(error.code, "INVALID_INPUT");
        assert_eq!(error.message, "Input is invalid");
    }

    #[rstest]
    fn validation_error_display_shows_code_and_message() {
        let error = ValidationError::new("CODE", "message");

        assert_eq!(format!("{error}"), "CODE: message");
    }

    // =========================================================================
    // ValidationErrors Tests
    // =========================================================================

    #[rstest]
    fn validation_errors_single_creates_collection_with_one_error() {
        let error = ValidationError::new("CODE", "message");
        let errors = ValidationErrors::single(error);

        assert_eq!(errors.len(), 1);
    }

    #[rstest]
    fn validation_errors_from_message_creates_default_error() {
        let errors = ValidationErrors::from_message("something went wrong");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors.iter().next().unwrap().code, "VALIDATION_ERROR");
    }

    #[rstest]
    fn validation_errors_combine_merges_all_errors() {
        let e1 = ValidationErrors::from_message("error 1");
        let e2 = ValidationErrors::from_message("error 2");
        let combined = e1.combine(e2);

        assert_eq!(combined.len(), 2);
    }

    #[rstest]
    fn validation_errors_display_joins_messages() {
        let e1 = ValidationErrors::from_message("error 1");
        let e2 = ValidationErrors::from_message("error 2");
        let combined = e1.combine(e2);

        let display = format!("{combined}");
        assert!(display.contains("error 1"));
        assert!(display.contains("error 2"));
    }

    #[rstest]
    fn validation_errors_into_iter_yields_all_errors() {
        let e1 = ValidationErrors::from_message("error 1");
        let e2 = ValidationErrors::from_message("error 2");
        let combined = e1.combine(e2);

        let count = combined.into_iter().count();
        assert_eq!(count, 2);
    }

    // =========================================================================
    // Validated Construction Tests
    // =========================================================================

    #[rstest]
    fn validated_valid_creates_valid_result() {
        let result: Validated<i32> = Validated::valid(42);

        assert!(result.is_valid());
        assert!(!result.is_invalid());
    }

    #[rstest]
    fn validated_invalid_creates_invalid_result() {
        let result: Validated<i32> = Validated::invalid("error");

        assert!(result.is_invalid());
        assert!(!result.is_valid());
    }

    #[rstest]
    fn validated_invalid_with_code_creates_structured_error() {
        let result: Validated<i32> = Validated::invalid_with_code("CUSTOM_CODE", "custom message");

        assert!(result.is_invalid());
        let errors = result.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors.iter().next().unwrap().code, "CUSTOM_CODE");
    }

    // =========================================================================
    // Validated Conversion Tests
    // =========================================================================

    #[rstest]
    fn validated_to_option_returns_some_for_valid() {
        let result: Validated<i32> = Validated::valid(42);
        assert_eq!(result.to_option(), Some(42));
    }

    #[rstest]
    fn validated_to_option_returns_none_for_invalid() {
        let result: Validated<i32> = Validated::invalid("error");
        assert_eq!(result.to_option(), None);
    }

    #[rstest]
    fn validated_to_result_returns_ok_for_valid() {
        let result: Validated<i32> = Validated::valid(42);
        assert_eq!(result.to_result(), Ok(42));
    }

    #[rstest]
    fn validated_to_result_returns_err_for_invalid() {
        let result: Validated<i32> = Validated::invalid("error");
        assert!(result.to_result().is_err());
    }

    #[rstest]
    fn validated_unwrap_returns_value_for_valid() {
        let result: Validated<i32> = Validated::valid(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[rstest]
    #[should_panic(expected = "Invalid")]
    fn validated_unwrap_panics_for_invalid() {
        let result: Validated<i32> = Validated::invalid("error");
        let _ = result.unwrap();
    }

    // =========================================================================
    // Functor Tests
    // =========================================================================

    #[rstest]
    fn validated_fmap_applies_function_to_valid() {
        let result: Validated<i32> = Validated::valid(21);
        let doubled = result.fmap(|x| x * 2);

        assert_eq!(doubled.unwrap(), 42);
    }

    #[rstest]
    fn validated_fmap_preserves_invalid() {
        let result: Validated<i32> = Validated::invalid("error");
        let doubled = result.fmap(|x| x * 2);

        assert!(doubled.is_invalid());
    }

    // =========================================================================
    // Applicative Tests - Key Feature: Error Accumulation
    // =========================================================================

    #[rstest]
    fn validated_pure_creates_valid() {
        let result: Validated<i32> = <Validated<()>>::pure(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[rstest]
    fn validated_map2_valid_valid_returns_valid() {
        let a: Validated<i32> = Validated::valid(1);
        let b: Validated<i32> = Validated::valid(2);

        let result = a.map2(b, |x, y| x + y);

        assert!(result.is_valid());
        assert_eq!(result.unwrap(), 3);
    }

    #[rstest]
    fn validated_map2_valid_invalid_returns_invalid() {
        let a: Validated<i32> = Validated::valid(1);
        let b: Validated<i32> = Validated::invalid("error");

        let result = a.map2(b, |x, y| x + y);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 1);
    }

    #[rstest]
    fn validated_map2_invalid_valid_returns_invalid() {
        let a: Validated<i32> = Validated::invalid("error");
        let b: Validated<i32> = Validated::valid(2);

        let result = a.map2(b, |x, y| x + y);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 1);
    }

    #[rstest]
    fn validated_map2_invalid_invalid_accumulates_all_errors() {
        let a: Validated<i32> = Validated::invalid("error 1");
        let b: Validated<i32> = Validated::invalid("error 2");

        let result = a.map2(b, |x, y| x + y);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 2);
    }

    #[rstest]
    fn validated_map3_accumulates_all_errors() {
        let a: Validated<i32> = Validated::invalid("error 1");
        let b: Validated<i32> = Validated::invalid("error 2");
        let c: Validated<i32> = Validated::invalid("error 3");

        let result = a.map3(b, c, |x, y, z| x + y + z);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 3);
    }

    #[rstest]
    fn validated_map3_all_valid_returns_valid() {
        let a: Validated<i32> = Validated::valid(1);
        let b: Validated<i32> = Validated::valid(2);
        let c: Validated<i32> = Validated::valid(3);

        let result = a.map3(b, c, |x, y, z| x + y + z);

        assert!(result.is_valid());
        assert_eq!(result.unwrap(), 6);
    }

    #[rstest]
    fn validated_apply_accumulates_errors() {
        let f: Validated<fn(i32) -> i32> = Validated::invalid("function error");
        let v: Validated<i32> = Validated::invalid("value error");

        let result = f.apply(v);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 2);
    }

    // =========================================================================
    // Practical Use Case Tests
    // =========================================================================

    #[rstest]
    fn validated_form_validation_accumulates_all_field_errors() {
        fn validate_name(name: &str) -> Validated<String> {
            if name.is_empty() {
                Validated::invalid_with_code("INVALID_NAME", "Name cannot be empty")
            } else {
                Validated::valid(name.to_string())
            }
        }

        fn validate_age(age: i32) -> Validated<i32> {
            if age < 0 {
                Validated::invalid_with_code("INVALID_AGE", "Age cannot be negative")
            } else {
                Validated::valid(age)
            }
        }

        fn validate_email(email: &str) -> Validated<String> {
            if email.contains('@') {
                Validated::valid(email.to_string())
            } else {
                Validated::invalid_with_code("INVALID_EMAIL", "Email must contain @")
            }
        }

        // All validations fail
        let name = validate_name("");
        let age = validate_age(-5);
        let email = validate_email("invalid");

        let result = name.map3(age, email, |n, a, e| (n, a, e));

        assert!(result.is_invalid());
        // All three errors should be accumulated
        assert_eq!(result.errors().len(), 3);

        // Verify each error code is present
        let codes: Vec<&str> = result.errors().iter().map(|e| e.code.as_str()).collect();
        assert!(codes.contains(&"INVALID_NAME"));
        assert!(codes.contains(&"INVALID_AGE"));
        assert!(codes.contains(&"INVALID_EMAIL"));
    }

    #[rstest]
    fn validated_product_accumulates_errors() {
        let a: Validated<i32> = Validated::invalid("error a");
        let b: Validated<i32> = Validated::invalid("error b");

        let result = a.product(b);

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 2);
    }
}
