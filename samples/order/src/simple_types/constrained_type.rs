//! Helper functions for generating constrained types
//!
//! Corresponds to the F# `ConstrainedType` module.
//! Each function is generic and can be used with any newtype.

use regex::Regex;
use rust_decimal::Decimal;

use super::error::ValidationError;

/// Creates a string type with a maximum length constraint
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages
/// * `constructor` - Constructor that takes a string and produces type T
/// * `max_length` - Maximum character count
/// * `value` - Input string
///
/// # Returns
///
/// * `Ok(T)` - On successful validation
/// * `Err(ValidationError)` - For an empty string or exceeding maximum length
///
/// # Errors
///
/// Returns [`ValidationError`] in the following cases:
/// - When the input is an empty string
/// - When the input exceeds the maximum length
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ValidationError;
///
/// #[derive(Debug, PartialEq)]
/// struct Name(String);
///
/// fn create_name(value: &str) -> Result<Name, ValidationError> {
///     order_taking_sample::simple_types::constrained_type::create_string(
///         "Name",
///         Name,
///         50,
///         value,
///     )
/// }
///
/// assert!(create_name("John").is_ok());
/// assert!(create_name("").is_err());
/// ```
pub fn create_string<T, F>(
    field_name: &str,
    constructor: F,
    max_length: usize,
    value: &str,
) -> Result<T, ValidationError>
where
    F: FnOnce(String) -> T,
{
    if value.is_empty() {
        Err(ValidationError::new(field_name, "Must not be empty"))
    } else if value.len() > max_length {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be more than {max_length} chars"),
        ))
    } else {
        Ok(constructor(value.to_string()))
    }
}

/// Creates a string type with maximum length constraint that returns None for empty strings
///
/// Used for optional fields.
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages
/// * `constructor` - Constructor that takes a string and produces type T
/// * `max_length` - Maximum character count
/// * `value` - Input string
///
/// # Returns
///
/// * `Ok(None)` - For an empty string
/// * `Ok(Some(T))` - On successful validation
/// * `Err(ValidationError)` - When exceeding maximum length
///
/// # Errors
///
/// Returns [`ValidationError`] when the input exceeds the maximum length.
pub fn create_string_option<T, F>(
    field_name: &str,
    constructor: F,
    max_length: usize,
    value: &str,
) -> Result<Option<T>, ValidationError>
where
    F: FnOnce(String) -> T,
{
    if value.is_empty() {
        Ok(None)
    } else if value.len() > max_length {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be more than {max_length} chars"),
        ))
    } else {
        Ok(Some(constructor(value.to_string())))
    }
}

/// Creates an integer type with range constraints
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages
/// * `constructor` - Constructor that takes an integer and produces type T
/// * `min_value` - Minimum value
/// * `max_value` - Maximum value
/// * `value` - Input integer
///
/// # Returns
///
/// * `Ok(T)` - On successful validation
/// * `Err(ValidationError)` - If out of range
///
/// # Errors
///
/// Returns [`ValidationError`] in the following cases:
/// - When the input is less than the minimum value
/// - When the input exceeds the maximum value
pub fn create_integer<T, F>(
    field_name: &str,
    constructor: F,
    min_value: u32,
    max_value: u32,
    value: u32,
) -> Result<T, ValidationError>
where
    F: FnOnce(u32) -> T,
{
    if value < min_value {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be less than {min_value}"),
        ))
    } else if value > max_value {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be greater than {max_value}"),
        ))
    } else {
        Ok(constructor(value))
    }
}

/// Creates a decimal type with range constraints
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages
/// * `constructor` - Constructor that takes a decimal and produces type T
/// * `min_value` - Minimum value
/// * `max_value` - Maximum value
/// * `value` - Input decimal
///
/// # Returns
///
/// * `Ok(T)` - On successful validation
/// * `Err(ValidationError)` - If out of range
///
/// # Errors
///
/// Returns [`ValidationError`] in the following cases:
/// - When the input is less than the minimum value
/// - When the input exceeds the maximum value
pub fn create_decimal<T, F>(
    field_name: &str,
    constructor: F,
    min_value: Decimal,
    max_value: Decimal,
    value: Decimal,
) -> Result<T, ValidationError>
where
    F: FnOnce(Decimal) -> T,
{
    if value < min_value {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be less than {min_value}"),
        ))
    } else if value > max_value {
        Err(ValidationError::new(
            field_name,
            &format!("Must not be greater than {max_value}"),
        ))
    } else {
        Ok(constructor(value))
    }
}

/// Creates a string type that matches a regular expression pattern
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages
/// * `constructor` - Constructor that takes a string and produces type T
/// * `pattern` - Compiled regular expression pattern
/// * `value` - Input string
///
/// # Returns
///
/// * `Ok(T)` - On successful validation
/// * `Err(ValidationError)` - For an empty string or pattern mismatch
///
/// # Errors
///
/// Returns [`ValidationError`] in the following cases:
/// - When the input is an empty string
/// - When the input does not match the pattern
///
/// # Note
///
/// Without anchors (^$), the regex pattern performs partial matching.
/// If exact matching is needed, the caller should include anchors.
pub fn create_like<T, F>(
    field_name: &str,
    constructor: F,
    pattern: &Regex,
    value: &str,
) -> Result<T, ValidationError>
where
    F: FnOnce(String) -> T,
{
    if value.is_empty() {
        Err(ValidationError::new(field_name, "Must not be empty"))
    } else if pattern.is_match(value) {
        Ok(constructor(value.to_string()))
    } else {
        let pattern_str = pattern.as_str();
        Err(ValidationError::new(
            field_name,
            &format!("'{value}' must match the pattern '{pattern_str}'"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::str::FromStr;

    // Simple wrapper type for testing
    #[derive(Debug, PartialEq)]
    struct TestString(String);

    #[derive(Debug, PartialEq)]
    struct TestInteger(u32);

    #[derive(Debug, PartialEq)]
    struct TestDecimal(Decimal);

    // =========================================================================
    // create_string Tests
    // =========================================================================

    #[rstest]
    fn test_create_string_valid() {
        let result = create_string("Name", TestString, 50, "John");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestString("John".to_string()));
    }

    #[rstest]
    fn test_create_string_empty() {
        let result = create_string("Name", TestString, 50, "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Name");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_create_string_too_long() {
        let long_string = "a".repeat(51);
        let result = create_string("Name", TestString, 50, &long_string);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Name");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_create_string_boundary_exactly_max() {
        let exact_string = "a".repeat(50);
        let result = create_string("Name", TestString, 50, &exact_string);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_create_string_boundary_one_over() {
        let over_string = "a".repeat(51);
        let result = create_string("Name", TestString, 50, &over_string);

        assert!(result.is_err());
    }

    // =========================================================================
    // create_string_option Tests
    // =========================================================================

    #[rstest]
    fn test_create_string_option_empty_returns_none() {
        let result = create_string_option("Name", TestString, 50, "");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[rstest]
    fn test_create_string_option_valid_returns_some() {
        let result = create_string_option("Name", TestString, 50, "John");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TestString("John".to_string())));
    }

    #[rstest]
    fn test_create_string_option_too_long() {
        let long_string = "a".repeat(51);
        let result = create_string_option("Name", TestString, 50, &long_string);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    // =========================================================================
    // create_integer Tests
    // =========================================================================

    #[rstest]
    fn test_create_integer_valid() {
        let result = create_integer("Quantity", TestInteger, 1, 1000, 500);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestInteger(500));
    }

    #[rstest]
    fn test_create_integer_below_min() {
        let result = create_integer("Quantity", TestInteger, 1, 1000, 0);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be less than 1");
    }

    #[rstest]
    fn test_create_integer_above_max() {
        let result = create_integer("Quantity", TestInteger, 1, 1000, 1001);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be greater than 1000");
    }

    #[rstest]
    fn test_create_integer_boundary_min() {
        let result = create_integer("Quantity", TestInteger, 1, 1000, 1);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestInteger(1));
    }

    #[rstest]
    fn test_create_integer_boundary_max() {
        let result = create_integer("Quantity", TestInteger, 1, 1000, 1000);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestInteger(1000));
    }

    // =========================================================================
    // create_decimal Tests
    // =========================================================================

    #[rstest]
    fn test_create_decimal_valid() {
        let value = Decimal::from_str("50.00").unwrap();
        let min = Decimal::from_str("0.0").unwrap();
        let max = Decimal::from_str("100.00").unwrap();
        let result = create_decimal("Price", TestDecimal, min, max, value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestDecimal(value));
    }

    #[rstest]
    fn test_create_decimal_below_min() {
        let value = Decimal::from_str("-0.01").unwrap();
        let min = Decimal::from_str("0.0").unwrap();
        let max = Decimal::from_str("100.00").unwrap();
        let result = create_decimal("Price", TestDecimal, min, max, value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be less than"));
    }

    #[rstest]
    fn test_create_decimal_above_max() {
        let value = Decimal::from_str("100.01").unwrap();
        let min = Decimal::from_str("0.0").unwrap();
        let max = Decimal::from_str("100.00").unwrap();
        let result = create_decimal("Price", TestDecimal, min, max, value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be greater than"));
    }

    #[rstest]
    fn test_create_decimal_boundary_min() {
        let value = Decimal::from_str("0.0").unwrap();
        let min = Decimal::from_str("0.0").unwrap();
        let max = Decimal::from_str("100.00").unwrap();
        let result = create_decimal("Price", TestDecimal, min, max, value);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_create_decimal_boundary_max() {
        let value = Decimal::from_str("100.00").unwrap();
        let min = Decimal::from_str("0.0").unwrap();
        let max = Decimal::from_str("100.00").unwrap();
        let result = create_decimal("Price", TestDecimal, min, max, value);

        assert!(result.is_ok());
    }

    // =========================================================================
    // create_like Tests
    // =========================================================================

    #[rstest]
    fn test_create_like_valid() {
        let pattern = Regex::new(r"^W\d{4}$").unwrap();
        let result = create_like("ProductCode", TestString, &pattern, "W1234");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestString("W1234".to_string()));
    }

    #[rstest]
    fn test_create_like_empty() {
        let pattern = Regex::new(r"^W\d{4}$").unwrap();
        let result = create_like("ProductCode", TestString, &pattern, "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_create_like_no_match() {
        let pattern = Regex::new(r"^W\d{4}$").unwrap();
        let result = create_like("ProductCode", TestString, &pattern, "G123");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert!(error.message.contains("must match the pattern"));
        assert!(error.message.contains("G123"));
    }

    #[rstest]
    fn test_create_like_partial_match_without_anchors() {
        // Patterns without anchors perform partial matching
        let pattern = Regex::new(r"\d{4}").unwrap();
        let result = create_like("Code", TestString, &pattern, "prefix1234suffix");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_create_like_full_match_with_anchors() {
        // Patterns with anchors require exact matching
        let pattern = Regex::new(r"^\d{4}$").unwrap();
        let result = create_like("Code", TestString, &pattern, "prefix1234suffix");

        assert!(result.is_err());
    }
}
