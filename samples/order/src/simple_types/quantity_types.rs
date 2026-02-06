//! Quantity type definitions
//!
//! Defines `UnitQuantity`, `KilogramQuantity`, and `OrderQuantity`.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;

use super::constrained_type;
use super::error::ValidationError;
use super::product_types::ProductCode;

// =============================================================================
// UnitQuantity
// =============================================================================

/// Integer type representing a unit count
///
/// Constrained to the range 1 to 1000.
/// Used for Widget product quantities.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::UnitQuantity;
///
/// let quantity = UnitQuantity::create("Quantity", 100).unwrap();
/// assert_eq!(quantity.value(), 100);
///
/// // Out of range causes an error
/// assert!(UnitQuantity::create("Quantity", 0).is_err());
/// assert!(UnitQuantity::create("Quantity", 1001).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnitQuantity(u32);

/// `UnitQuantity` minimum value
const UNIT_QUANTITY_MIN: u32 = 1;

/// `UnitQuantity` maximum value
const UNIT_QUANTITY_MAX: u32 = 1000;

impl UnitQuantity {
    /// Creates a `UnitQuantity` from an integer
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input integer
    ///
    /// # Returns
    ///
    /// * `Ok(UnitQuantity)` - On successful validation
    /// * `Err(ValidationError)` - If out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the value is less than 1 or exceeds 1000.
    pub fn create(field_name: &str, value: u32) -> Result<Self, ValidationError> {
        constrained_type::create_integer(
            field_name,
            Self,
            UNIT_QUANTITY_MIN,
            UNIT_QUANTITY_MAX,
            value,
        )
    }

    /// Returns the inner integer value
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

// =============================================================================
// KilogramQuantity
// =============================================================================

/// Decimal type representing weight in kilograms
///
/// Constrained to the range 0.05 to 100.00.
/// Used for Gizmo product quantities.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::KilogramQuantity;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let quantity = KilogramQuantity::create(
///     "Weight",
///     Decimal::from_str("50.0").unwrap()
/// ).unwrap();
/// assert_eq!(quantity.value(), Decimal::from_str("50.0").unwrap());
///
/// // Out of range causes an error
/// assert!(KilogramQuantity::create("Weight", Decimal::from_str("0.04").unwrap()).is_err());
/// assert!(KilogramQuantity::create("Weight", Decimal::from_str("100.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KilogramQuantity(Decimal);

impl KilogramQuantity {
    /// Returns the minimum value of `KilogramQuantity`
    fn min_value() -> Decimal {
        Decimal::from_str("0.05").expect("Valid decimal literal")
    }

    /// Returns the maximum value of `KilogramQuantity`
    fn max_value() -> Decimal {
        Decimal::from_str("100.00").expect("Valid decimal literal")
    }

    /// Creates a `KilogramQuantity` from a decimal
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input decimal
    ///
    /// # Returns
    ///
    /// * `Ok(KilogramQuantity)` - On successful validation
    /// * `Err(ValidationError)` - If out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the value is less than 0.05 or exceeds 100.00.
    pub fn create(field_name: &str, value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            field_name,
            Self,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// Returns the inner decimal value
    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.0
    }
}

// =============================================================================
// OrderQuantity
// =============================================================================

/// Sum type representing an order quantity
///
/// Holds either a unit count or kilogram weight.
/// Which type is used depends on the product code.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::{OrderQuantity, ProductCode};
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// // Unit count for Widget products
/// let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let unit_quantity = OrderQuantity::create(
///     "Quantity",
///     &widget_code,
///     Decimal::from_str("10").unwrap()
/// ).unwrap();
/// assert!(matches!(unit_quantity, OrderQuantity::Unit(_)));
///
/// // Weight for Gizmo products
/// let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
/// let kg_quantity = OrderQuantity::create(
///     "Quantity",
///     &gizmo_code,
///     Decimal::from_str("5.5").unwrap()
/// ).unwrap();
/// assert!(matches!(kg_quantity, OrderQuantity::Kilogram(_)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderQuantity {
    /// Widget product unit count
    Unit(UnitQuantity),
    /// Gizmo product weight
    Kilogram(KilogramQuantity),
}

impl OrderQuantity {
    /// Creates an `OrderQuantity` from a product code and quantity
    ///
    /// Interpreted as `UnitQuantity` for Widget, or `KilogramQuantity` for Gizmo.
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `product_code` - Product code
    /// * `quantity` - Quantity (Decimal)
    ///
    /// # Returns
    ///
    /// * `Ok(OrderQuantity)` - On successful validation
    /// * `Err(ValidationError)` - When the quantity is out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when integer conversion fails for Widget products, or when the quantity is out of range.
    /// returns `ValidationError`.
    pub fn create(
        field_name: &str,
        product_code: &ProductCode,
        quantity: Decimal,
    ) -> Result<Self, ValidationError> {
        match product_code {
            ProductCode::Widget(widget_code) => {
                // Convert Decimal to u32
                let integer_quantity = quantity.to_u32().ok_or_else(|| {
                    ValidationError::new(
                        field_name,
                        &format!(
                            "Quantity '{}' must be a valid integer for Widget product '{}'. \
                             Widget products require a whole number quantity between 1 and 1000.",
                            quantity,
                            widget_code.value()
                        ),
                    )
                })?;
                UnitQuantity::create(field_name, integer_quantity).map(Self::Unit)
            }
            ProductCode::Gizmo(_) => {
                KilogramQuantity::create(field_name, quantity).map(Self::Kilogram)
            }
        }
    }

    /// Returns the quantity as a Decimal
    #[must_use]
    pub fn value(&self) -> Decimal {
        match self {
            Self::Unit(unit_quantity) => Decimal::from(unit_quantity.value()),
            Self::Kilogram(kilogram_quantity) => kilogram_quantity.value(),
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
    // UnitQuantity Tests
    // =========================================================================

    #[rstest]
    fn test_unit_quantity_create_valid() {
        let result = UnitQuantity::create("Quantity", 500);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 500);
    }

    #[rstest]
    fn test_unit_quantity_create_min() {
        let result = UnitQuantity::create("Quantity", 1);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 1);
    }

    #[rstest]
    fn test_unit_quantity_create_max() {
        let result = UnitQuantity::create("Quantity", 1000);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 1000);
    }

    #[rstest]
    fn test_unit_quantity_create_below_min() {
        let result = UnitQuantity::create("Quantity", 0);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be less than 1");
    }

    #[rstest]
    fn test_unit_quantity_create_above_max() {
        let result = UnitQuantity::create("Quantity", 1001);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be greater than 1000");
    }

    #[rstest]
    fn test_unit_quantity_value() {
        let quantity = UnitQuantity::create("Quantity", 42).unwrap();

        assert_eq!(quantity.value(), 42);
    }

    #[rstest]
    fn test_unit_quantity_copy() {
        // Verify that Copy trait is implemented
        let quantity = UnitQuantity::create("Quantity", 100).unwrap();
        let copied = quantity;

        assert_eq!(quantity.value(), copied.value());
    }

    #[rstest]
    fn test_unit_quantity_clone() {
        let quantity = UnitQuantity::create("Quantity", 100).unwrap();
        let cloned = quantity;

        assert_eq!(quantity, cloned);
    }

    // =========================================================================
    // KilogramQuantity Tests
    // =========================================================================

    #[rstest]
    fn test_kilogram_quantity_create_valid() {
        let value = Decimal::from_str("50.0").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_kilogram_quantity_create_min() {
        let value = Decimal::from_str("0.05").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_kilogram_quantity_create_max() {
        let value = Decimal::from_str("100.00").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_kilogram_quantity_create_below_min() {
        let value = Decimal::from_str("0.04").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Weight");
        assert!(error.message.contains("Must not be less than"));
    }

    #[rstest]
    fn test_kilogram_quantity_create_above_max() {
        let value = Decimal::from_str("100.01").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Weight");
        assert!(error.message.contains("Must not be greater than"));
    }

    #[rstest]
    fn test_kilogram_quantity_create_zero() {
        let value = Decimal::from_str("0.0").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_kilogram_quantity_create_negative() {
        let value = Decimal::from_str("-1.0").unwrap();
        let result = KilogramQuantity::create("Weight", value);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_kilogram_quantity_value() {
        let value = Decimal::from_str("25.5").unwrap();
        let quantity = KilogramQuantity::create("Weight", value).unwrap();

        assert_eq!(quantity.value(), value);
    }

    #[rstest]
    fn test_kilogram_quantity_copy() {
        // Verify that Copy trait is implemented
        let value = Decimal::from_str("10.0").unwrap();
        let quantity = KilogramQuantity::create("Weight", value).unwrap();
        let copied = quantity;

        assert_eq!(quantity.value(), copied.value());
    }

    // =========================================================================
    // OrderQuantity Tests
    // =========================================================================

    #[rstest]
    fn test_order_quantity_create_unit() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("10").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_ok());
        let order_quantity = result.unwrap();
        assert!(matches!(order_quantity, OrderQuantity::Unit(_)));
        assert_eq!(order_quantity.value(), Decimal::from(10));
    }

    #[rstest]
    fn test_order_quantity_create_kilogram() {
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();
        let quantity = Decimal::from_str("5.5").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_ok());
        let order_quantity = result.unwrap();
        assert!(matches!(order_quantity, OrderQuantity::Kilogram(_)));
        assert_eq!(order_quantity.value(), Decimal::from_str("5.5").unwrap());
    }

    #[rstest]
    fn test_order_quantity_create_unit_invalid() {
        // Out-of-range quantity for Widget products
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("0").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_create_unit_over_max() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("1001").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_create_unit_with_decimal() {
        // When a decimal is specified for Widget products
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("10.5").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        // to_u32() truncates, so 10.5 -> 10 should succeed
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from(10));
    }

    #[rstest]
    fn test_order_quantity_create_unit_negative() {
        // Negative number for Widget products
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("-1").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        // Negative Decimal returns None from to_u32()
        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_create_kilogram_invalid() {
        // Out-of-range quantity for Gizmo products
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();
        let quantity = Decimal::from_str("0.04").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_create_kilogram_over_max() {
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();
        let quantity = Decimal::from_str("100.01").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_value_unit() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("42").unwrap();
        let order_quantity = OrderQuantity::create("Quantity", &product_code, quantity).unwrap();

        assert_eq!(order_quantity.value(), Decimal::from(42));
    }

    #[rstest]
    fn test_order_quantity_value_kilogram() {
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();
        let quantity = Decimal::from_str("12.345").unwrap();
        let order_quantity = OrderQuantity::create("Quantity", &product_code, quantity).unwrap();

        assert_eq!(order_quantity.value(), Decimal::from_str("12.345").unwrap());
    }

    #[rstest]
    fn test_order_quantity_copy() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("10").unwrap();
        let order_quantity = OrderQuantity::create("Quantity", &product_code, quantity).unwrap();
        let copied = order_quantity;

        assert_eq!(order_quantity.value(), copied.value());
    }

    #[rstest]
    fn test_order_quantity_pattern_match() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();

        let unit = OrderQuantity::create("Quantity", &widget_code, Decimal::from(10)).unwrap();
        let kg = OrderQuantity::create("Quantity", &gizmo_code, Decimal::from_str("5.0").unwrap())
            .unwrap();

        match unit {
            OrderQuantity::Unit(u) => assert_eq!(u.value(), 10),
            OrderQuantity::Kilogram(_) => panic!("Expected Unit variant"),
        }

        match kg {
            OrderQuantity::Unit(_) => panic!("Expected Kilogram variant"),
            OrderQuantity::Kilogram(k) => {
                assert_eq!(k.value(), Decimal::from_str("5.0").unwrap());
            }
        }
    }
}
