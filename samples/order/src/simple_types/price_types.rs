//! Price type definitions
//!
//! Defines `Price` and `BillingAmount`.

use lambars::typeclass::Foldable;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// Price
// =============================================================================

/// Decimal type representing a unit price
///
/// Constrained to the range 0.0 to 1000.00.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::Price;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let price = Price::create(Decimal::from_str("99.99").unwrap()).unwrap();
/// assert_eq!(price.value(), Decimal::from_str("99.99").unwrap());
///
/// // Out of range causes an error
/// assert!(Price::create(Decimal::from_str("-1.0").unwrap()).is_err());
/// assert!(Price::create(Decimal::from_str("1000.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Price(Decimal);

impl Price {
    /// Field name for Price
    const FIELD_NAME: &'static str = "Price";

    /// Returns the minimum value of Price
    fn min_value() -> Decimal {
        Decimal::from_str("0.0").expect("Valid decimal literal")
    }

    /// Returns the maximum value of Price
    fn max_value() -> Decimal {
        Decimal::from_str("1000.00").expect("Valid decimal literal")
    }

    /// Creates a Price from a decimal
    ///
    /// # Arguments
    ///
    /// * `value` - Input decimal
    ///
    /// # Returns
    ///
    /// * `Ok(Price)` - On successful validation
    /// * `Err(ValidationError)` - If out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the value is less than 0.0 or exceeds 1000.00.
    pub fn create(value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            Self::FIELD_NAME,
            Price,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// Creates a Price without validation
    ///
    /// Use only when the value is guaranteed to be valid.
    ///
    /// # Panics
    ///
    /// Panics if a value outside the valid range is passed.
    #[must_use]
    pub fn unsafe_create(value: Decimal) -> Self {
        Self::create(value)
            .unwrap_or_else(|error| panic!("Not expecting Price to be out of bounds: {error}"))
    }

    /// Creates a new Price by multiplying with a quantity
    ///
    /// # Arguments
    ///
    /// * `quantity` - quantity
    ///
    /// # Returns
    ///
    /// * `Ok(Price)` - If the new price is within range
    /// * `Err(ValidationError)` - If the new price is out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the multiplication result exceeds 1000.00.
    pub fn multiply(&self, quantity: Decimal) -> Result<Self, ValidationError> {
        Self::create(quantity * self.0)
    }

    /// Returns the inner decimal value
    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.0
    }
}

// =============================================================================
// BillingAmount
// =============================================================================

/// Decimal type representing a billing amount
///
/// Constrained to the range 0.0 to 10000.00.
/// Used as the sum of multiple `Price` values.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::{BillingAmount, Price};
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let amount = BillingAmount::create(Decimal::from_str("5000.0").unwrap()).unwrap();
/// assert_eq!(amount.value(), Decimal::from_str("5000.0").unwrap());
///
/// // Out of range causes an error
/// assert!(BillingAmount::create(Decimal::from_str("-1.0").unwrap()).is_err());
/// assert!(BillingAmount::create(Decimal::from_str("10000.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BillingAmount(Decimal);

impl BillingAmount {
    /// Field name for `BillingAmount`
    const FIELD_NAME: &'static str = "BillingAmount";

    /// Returns the minimum value of `BillingAmount`
    fn min_value() -> Decimal {
        Decimal::from_str("0.0").expect("Valid decimal literal")
    }

    /// Returns the maximum value of `BillingAmount`
    fn max_value() -> Decimal {
        Decimal::from_str("10000.00").expect("Valid decimal literal")
    }

    /// Creates a `BillingAmount` from a decimal
    ///
    /// # Arguments
    ///
    /// * `value` - Input decimal
    ///
    /// # Returns
    ///
    /// * `Ok(BillingAmount)` - On successful validation
    /// * `Err(ValidationError)` - If out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the value is less than 0.0 or exceeds 10000.00.
    pub fn create(value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            Self::FIELD_NAME,
            BillingAmount,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// Creates a `BillingAmount` by summing a slice of `Price` values
    ///
    /// Performs a fold using the lambars `Foldable` trait.
    ///
    /// # Arguments
    ///
    /// * `prices` - A slice of Price values
    ///
    /// # Returns
    ///
    /// * `Ok(BillingAmount)` - If the sum is within range
    /// * `Err(ValidationError)` - If the sum is out of range
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the sum exceeds 10000.00.
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::simple_types::{BillingAmount, Price};
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    ///
    /// let prices = vec![
    ///     Price::create(Decimal::from_str("100.00").unwrap()).unwrap(),
    ///     Price::create(Decimal::from_str("200.00").unwrap()).unwrap(),
    ///     Price::create(Decimal::from_str("300.00").unwrap()).unwrap(),
    /// ];
    ///
    /// let total = BillingAmount::sum_prices(&prices).unwrap();
    /// assert_eq!(total.value(), Decimal::from_str("600.00").unwrap());
    /// ```
    pub fn sum_prices(prices: &[Price]) -> Result<Self, ValidationError> {
        let total = prices
            .to_vec()
            .fold_left(Decimal::ZERO, |accumulator, price| {
                accumulator + price.value()
            });
        Self::create(total)
    }

    /// Returns the inner decimal value
    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.0
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
    // Price Tests
    // =========================================================================

    #[rstest]
    fn test_price_create_valid() {
        let value = Decimal::from_str("500.0").unwrap();
        let result = Price::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_price_create_min() {
        let value = Decimal::from_str("0.0").unwrap();
        let result = Price::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_price_create_max() {
        let value = Decimal::from_str("1000.0").unwrap();
        let result = Price::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_price_create_below_min() {
        let value = Decimal::from_str("-0.01").unwrap();
        let result = Price::create(value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be less than"));
    }

    #[rstest]
    fn test_price_create_above_max() {
        let value = Decimal::from_str("1000.01").unwrap();
        let result = Price::create(value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be greater than"));
    }

    #[rstest]
    fn test_price_unsafe_create_valid() {
        let value = Decimal::from_str("500.0").unwrap();
        let price = Price::unsafe_create(value);

        assert_eq!(price.value(), value);
    }

    #[rstest]
    #[should_panic(expected = "Not expecting Price to be out of bounds")]
    fn test_price_unsafe_create_panic() {
        let value = Decimal::from_str("1001.0").unwrap();
        let _price = Price::unsafe_create(value);
    }

    #[rstest]
    fn test_price_multiply_valid() {
        let price = Price::create(Decimal::from_str("100.0").unwrap()).unwrap();
        let quantity = Decimal::from_str("5").unwrap();
        let result = price.multiply(quantity);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from_str("500.0").unwrap());
    }

    #[rstest]
    fn test_price_multiply_overflow() {
        let price = Price::create(Decimal::from_str("500.0").unwrap()).unwrap();
        let quantity = Decimal::from_str("3").unwrap();
        let result = price.multiply(quantity);

        // 500 * 3 = 1500 > 1000
        assert!(result.is_err());
    }

    #[rstest]
    fn test_price_multiply_zero() {
        let price = Price::create(Decimal::from_str("100.0").unwrap()).unwrap();
        let quantity = Decimal::from_str("0").unwrap();
        let result = price.multiply(quantity);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from_str("0.0").unwrap());
    }

    #[rstest]
    fn test_price_value() {
        let value = Decimal::from_str("99.99").unwrap();
        let price = Price::create(value).unwrap();

        assert_eq!(price.value(), value);
    }

    #[rstest]
    fn test_price_copy() {
        let price = Price::create(Decimal::from_str("100.0").unwrap()).unwrap();
        let copied = price;

        assert_eq!(price.value(), copied.value());
    }

    // =========================================================================
    // BillingAmount Tests
    // =========================================================================

    #[rstest]
    fn test_billing_amount_create_valid() {
        let value = Decimal::from_str("5000.0").unwrap();
        let result = BillingAmount::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_billing_amount_create_min() {
        let value = Decimal::from_str("0.0").unwrap();
        let result = BillingAmount::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_billing_amount_create_max() {
        let value = Decimal::from_str("10000.0").unwrap();
        let result = BillingAmount::create(value);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), value);
    }

    #[rstest]
    fn test_billing_amount_create_below_min() {
        let value = Decimal::from_str("-0.01").unwrap();
        let result = BillingAmount::create(value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "BillingAmount");
        assert!(error.message.contains("Must not be less than"));
    }

    #[rstest]
    fn test_billing_amount_create_above_max() {
        let value = Decimal::from_str("10000.01").unwrap();
        let result = BillingAmount::create(value);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "BillingAmount");
        assert!(error.message.contains("Must not be greater than"));
    }

    #[rstest]
    fn test_billing_amount_sum_prices_empty() {
        let prices: Vec<Price> = vec![];
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from_str("0.0").unwrap());
    }

    #[rstest]
    fn test_billing_amount_sum_prices_single() {
        let prices = vec![Price::create(Decimal::from_str("100.0").unwrap()).unwrap()];
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from_str("100.0").unwrap());
    }

    #[rstest]
    fn test_billing_amount_sum_prices_multiple() {
        let prices = vec![
            Price::create(Decimal::from_str("100.00").unwrap()).unwrap(),
            Price::create(Decimal::from_str("200.00").unwrap()).unwrap(),
            Price::create(Decimal::from_str("300.00").unwrap()).unwrap(),
        ];
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().value(),
            Decimal::from_str("600.00").unwrap()
        );
    }

    #[rstest]
    fn test_billing_amount_sum_prices_max() {
        // 10 items at 1000 = 10000 (maximum value)
        let prices: Vec<Price> = (0..10)
            .map(|_| Price::create(Decimal::from_str("1000.0").unwrap()).unwrap())
            .collect();
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().value(),
            Decimal::from_str("10000.0").unwrap()
        );
    }

    #[rstest]
    fn test_billing_amount_sum_prices_overflow() {
        // 11 items at 1000 = 11000 (exceeds maximum)
        let prices: Vec<Price> = (0..11)
            .map(|_| Price::create(Decimal::from_str("1000.0").unwrap()).unwrap())
            .collect();
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "BillingAmount");
    }

    #[rstest]
    fn test_billing_amount_sum_prices_with_decimals() {
        let prices = vec![
            Price::create(Decimal::from_str("99.99").unwrap()).unwrap(),
            Price::create(Decimal::from_str("50.01").unwrap()).unwrap(),
        ];
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().value(),
            Decimal::from_str("150.00").unwrap()
        );
    }

    #[rstest]
    fn test_billing_amount_value() {
        let value = Decimal::from_str("1234.56").unwrap();
        let amount = BillingAmount::create(value).unwrap();

        assert_eq!(amount.value(), value);
    }

    #[rstest]
    fn test_billing_amount_copy() {
        let amount = BillingAmount::create(Decimal::from_str("1000.0").unwrap()).unwrap();
        let copied = amount;

        assert_eq!(amount.value(), copied.value());
    }
}
