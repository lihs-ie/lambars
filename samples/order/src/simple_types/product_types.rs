//! Product code type definitions
//!
//! Defines `WidgetCode`, `GizmoCode`, and `ProductCode`.

use regex::Regex;
use std::sync::LazyLock;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// WidgetCode
// =============================================================================

/// Type representing a Widget product code
///
/// "W" followed by 4 digits (W\d{4} pattern).
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::WidgetCode;
///
/// let code = WidgetCode::create("ProductCode", "W1234").unwrap();
/// assert_eq!(code.value(), "W1234");
///
/// // Invalid format causes an error
/// assert!(WidgetCode::create("ProductCode", "G123").is_err());
/// assert!(WidgetCode::create("ProductCode", "W123").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetCode(String);

/// Regex pattern for `WidgetCode`
static WIDGET_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^W\d{4}$").expect("Invalid widget code regex pattern"));

impl WidgetCode {
    /// Creates a `WidgetCode` from a string in W + 4 digits format
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(WidgetCode)` - On successful validation
    /// * `Err(ValidationError)` - On pattern mismatch
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` for an empty string or pattern mismatch.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, Self, &WIDGET_CODE_PATTERN, value)
    }

    /// Returns a reference to the inner code string
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// GizmoCode
// =============================================================================

/// Type representing a Gizmo product code
///
/// "G" followed by 3 digits (G\d{3} pattern).
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::GizmoCode;
///
/// let code = GizmoCode::create("ProductCode", "G123").unwrap();
/// assert_eq!(code.value(), "G123");
///
/// // Invalid format causes an error
/// assert!(GizmoCode::create("ProductCode", "W1234").is_err());
/// assert!(GizmoCode::create("ProductCode", "G12").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GizmoCode(String);

/// Regex pattern for `GizmoCode`
static GIZMO_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^G\d{3}$").expect("Invalid gizmo code regex pattern"));

impl GizmoCode {
    /// Creates a `GizmoCode` from a string in G + 3 digits format
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(GizmoCode)` - On successful validation
    /// * `Err(ValidationError)` - On pattern mismatch
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` for an empty string or pattern mismatch.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, Self, &GIZMO_CODE_PATTERN, value)
    }

    /// Returns a reference to the inner code string
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// ProductCode
// =============================================================================

/// Sum type representing a product code
///
/// Holds either a Widget code or a Gizmo code.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ProductCode;
///
/// // Widget code
/// let widget = ProductCode::create("ProductCode", "W1234").unwrap();
/// assert!(matches!(widget, ProductCode::Widget(_)));
/// assert_eq!(widget.value(), "W1234");
///
/// // Gizmo code
/// let gizmo = ProductCode::create("ProductCode", "G123").unwrap();
/// assert!(matches!(gizmo, ProductCode::Gizmo(_)));
/// assert_eq!(gizmo.value(), "G123");
///
/// // Unknown format causes an error
/// assert!(ProductCode::create("ProductCode", "X999").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProductCode {
    /// Widget product code
    Widget(WidgetCode),
    /// Gizmo product code
    Gizmo(GizmoCode),
}

impl ProductCode {
    /// Creates a `ProductCode` from a string
    ///
    /// Determines Widget or Gizmo based on the first character.
    /// - Starting with "W": interpreted as `WidgetCode`
    /// - Starting with "G": interpreted as `GizmoCode`
    /// - Otherwise: error
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `code` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(ProductCode)` - On successful validation
    /// * `Err(ValidationError)` - On pattern mismatch
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` for an empty string or unrecognized format.
    pub fn create(field_name: &str, code: &str) -> Result<Self, ValidationError> {
        if code.is_empty() {
            return Err(ValidationError::new(field_name, "Must not be empty"));
        }

        if code.starts_with('W') {
            WidgetCode::create(field_name, code).map(Self::Widget)
        } else if code.starts_with('G') {
            GizmoCode::create(field_name, code).map(Self::Gizmo)
        } else {
            Err(ValidationError::new(
                field_name,
                &format!("Format not recognized '{code}'"),
            ))
        }
    }

    /// Returns a reference to the inner code string
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Widget(widget_code) => widget_code.value(),
            Self::Gizmo(gizmo_code) => gizmo_code.value(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // WidgetCode Tests
    // =========================================================================

    #[rstest]
    fn test_widget_code_create_valid() {
        let result = WidgetCode::create("ProductCode", "W1234");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "W1234");
    }

    #[rstest]
    fn test_widget_code_create_valid_all_zeros() {
        let result = WidgetCode::create("ProductCode", "W0000");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_widget_code_create_valid_all_nines() {
        let result = WidgetCode::create("ProductCode", "W9999");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_widget_code_create_empty() {
        let result = WidgetCode::create("ProductCode", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_widget_code_create_3_digits() {
        let result = WidgetCode::create("ProductCode", "W123");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_widget_code_create_5_digits() {
        let result = WidgetCode::create("ProductCode", "W12345");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_widget_code_create_wrong_prefix() {
        let result = WidgetCode::create("ProductCode", "G1234");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_widget_code_create_lowercase_prefix() {
        let result = WidgetCode::create("ProductCode", "w1234");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_widget_code_value() {
        let code = WidgetCode::create("ProductCode", "W5555").unwrap();

        assert_eq!(code.value(), "W5555");
    }

    // =========================================================================
    // GizmoCode Tests
    // =========================================================================

    #[rstest]
    fn test_gizmo_code_create_valid() {
        let result = GizmoCode::create("ProductCode", "G123");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "G123");
    }

    #[rstest]
    fn test_gizmo_code_create_valid_all_zeros() {
        let result = GizmoCode::create("ProductCode", "G000");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_gizmo_code_create_valid_all_nines() {
        let result = GizmoCode::create("ProductCode", "G999");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_gizmo_code_create_empty() {
        let result = GizmoCode::create("ProductCode", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_gizmo_code_create_2_digits() {
        let result = GizmoCode::create("ProductCode", "G12");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_gizmo_code_create_4_digits() {
        let result = GizmoCode::create("ProductCode", "G1234");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_gizmo_code_create_wrong_prefix() {
        let result = GizmoCode::create("ProductCode", "W123");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_gizmo_code_create_lowercase_prefix() {
        let result = GizmoCode::create("ProductCode", "g123");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_gizmo_code_value() {
        let code = GizmoCode::create("ProductCode", "G555").unwrap();

        assert_eq!(code.value(), "G555");
    }

    // =========================================================================
    // ProductCode Tests
    // =========================================================================

    #[rstest]
    fn test_product_code_create_widget() {
        let result = ProductCode::create("ProductCode", "W1234");

        assert!(result.is_ok());
        let product_code = result.unwrap();
        assert!(matches!(product_code, ProductCode::Widget(_)));
        assert_eq!(product_code.value(), "W1234");
    }

    #[rstest]
    fn test_product_code_create_gizmo() {
        let result = ProductCode::create("ProductCode", "G123");

        assert!(result.is_ok());
        let product_code = result.unwrap();
        assert!(matches!(product_code, ProductCode::Gizmo(_)));
        assert_eq!(product_code.value(), "G123");
    }

    #[rstest]
    fn test_product_code_create_empty() {
        let result = ProductCode::create("ProductCode", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_product_code_create_unknown_prefix() {
        let result = ProductCode::create("ProductCode", "X999");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert!(error.message.contains("Format not recognized"));
        assert!(error.message.contains("X999"));
    }

    #[rstest]
    fn test_product_code_create_invalid_widget() {
        // Starts with "W" but invalid format
        let result = ProductCode::create("ProductCode", "W12");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_product_code_create_invalid_gizmo() {
        // Starts with "G" but invalid format
        let result = ProductCode::create("ProductCode", "G12");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_product_code_value_widget() {
        let product_code = ProductCode::create("ProductCode", "W1111").unwrap();

        assert_eq!(product_code.value(), "W1111");
    }

    #[rstest]
    fn test_product_code_value_gizmo() {
        let product_code = ProductCode::create("ProductCode", "G111").unwrap();

        assert_eq!(product_code.value(), "G111");
    }

    #[rstest]
    fn test_product_code_pattern_match_widget() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();

        match product_code {
            ProductCode::Widget(widget_code) => {
                assert_eq!(widget_code.value(), "W1234");
            }
            ProductCode::Gizmo(_) => {
                panic!("Expected Widget variant");
            }
        }
    }

    #[rstest]
    fn test_product_code_pattern_match_gizmo() {
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();

        match product_code {
            ProductCode::Widget(_) => {
                panic!("Expected Gizmo variant");
            }
            ProductCode::Gizmo(gizmo_code) => {
                assert_eq!(gizmo_code.value(), "G123");
            }
        }
    }

    #[rstest]
    fn test_product_code_clone() {
        let original = ProductCode::create("ProductCode", "W1234").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_product_code_eq() {
        let code1 = ProductCode::create("ProductCode", "W1234").unwrap();
        let code2 = ProductCode::create("ProductCode", "W1234").unwrap();
        let code3 = ProductCode::create("ProductCode", "G123").unwrap();

        assert_eq!(code1, code2);
        assert_ne!(code1, code3);
    }

    #[rstest]
    fn test_widget_and_gizmo_with_similar_numbers() {
        // Same digits but different types
        let widget = ProductCode::create("ProductCode", "W0123").unwrap();
        let gizmo = ProductCode::create("ProductCode", "G012").unwrap();

        assert!(matches!(widget, ProductCode::Widget(_)));
        assert!(matches!(gizmo, ProductCode::Gizmo(_)));
        assert_ne!(widget, gizmo);
    }
}
