//! Money value object.
//!
//! Provides a strongly-typed representation of monetary values with currency.
//! Implements `Semigroup` and `Monoid` for functional composition of amounts.

use std::cmp::Ordering;
use std::fmt;

use lambars::control::Either;
use lambars::typeclass::{Monoid, Semigroup};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Supported currencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// Japanese Yen
    JPY,
    /// United States Dollar
    USD,
    /// Euro
    EUR,
}

impl fmt::Display for Currency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JPY => write!(formatter, "JPY"),
            Self::USD => write!(formatter, "USD"),
            Self::EUR => write!(formatter, "EUR"),
        }
    }
}

impl Currency {
    /// Returns the number of decimal places for this currency.
    ///
    /// JPY uses 0 decimal places (no cents).
    /// USD and EUR use 2 decimal places.
    #[must_use]
    pub const fn decimal_places(&self) -> u32 {
        match self {
            Self::JPY => 0,
            Self::USD | Self::EUR => 2,
        }
    }
}

/// Errors that can occur when working with Money.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyError {
    /// Attempted to combine money with different currencies.
    CurrencyMismatch {
        /// The currency of the left operand.
        left: Currency,
        /// The currency of the right operand.
        right: Currency,
    },
    /// The amount string could not be parsed as a valid decimal.
    InvalidAmount(String),
    /// The amount is negative when it should be positive.
    NegativeAmount,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrencyMismatch { left, right } => {
                write!(formatter, "Currency mismatch: {left} vs {right}")
            }
            Self::InvalidAmount(value) => {
                write!(formatter, "Invalid amount: {value}")
            }
            Self::NegativeAmount => {
                write!(formatter, "Amount cannot be negative")
            }
        }
    }
}

impl std::error::Error for MoneyError {}

/// A monetary value with an associated currency.
///
/// `Money` represents an amount of money in a specific currency. It provides:
///
/// - **Type safety**: Currency is tracked at the value level
/// - **Precision**: Uses `Decimal` for accurate financial calculations
/// - **Functional composition**: Implements `Semigroup` and `Monoid`
///
/// # Semigroup and Monoid
///
/// `Money` implements `Semigroup` through `combine`, which adds amounts
/// of the same currency. Combining money with different currencies will panic.
///
/// `Monoid::empty()` requires a default currency. For this implementation,
/// it returns zero JPY. For specific currency empty values, use `Money::zero(currency)`.
///
/// # Examples
///
/// ```rust
/// use bank::domain::value_objects::{Money, Currency};
/// use lambars::typeclass::Semigroup;
///
/// let m1 = Money::new(1000, Currency::JPY);
/// let m2 = Money::new(500, Currency::JPY);
///
/// // Using Semigroup::combine
/// let total = m1.combine(m2);
/// assert_eq!(total.amount().to_string(), "1500");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    /// Creates a new `Money` value with the given amount and currency.
    ///
    /// # Arguments
    ///
    /// * `amount` - The monetary amount as an integer (in the smallest unit for the currency)
    /// * `currency` - The currency of the money
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let jpy = Money::new(1000, Currency::JPY);
    /// let usd = Money::new(1050, Currency::USD); // Represents $10.50
    /// ```
    #[must_use]
    pub fn new(amount: i64, currency: Currency) -> Self {
        Self {
            amount: Decimal::from(amount),
            currency,
        }
    }

    /// Creates a new `Money` value from a `Decimal` amount.
    ///
    /// # Arguments
    ///
    /// * `amount` - The monetary amount as a `Decimal`
    /// * `currency` - The currency of the money
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    ///
    /// let amount = Decimal::from_str("10.50").unwrap();
    /// let money = Money::from_decimal(amount, Currency::USD);
    /// ```
    #[must_use]
    pub const fn from_decimal(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Parses a string amount into `Money` with the given currency.
    ///
    /// # Arguments
    ///
    /// * `amount` - A string representation of the amount
    /// * `currency` - The currency of the money
    ///
    /// # Returns
    ///
    /// * `Either::Right(Money)` if parsing succeeds
    /// * `Either::Left(MoneyError)` if the string cannot be parsed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let result = Money::parse("10.50", Currency::USD);
    /// assert!(result.is_right());
    /// ```
    pub fn parse(amount: &str, currency: Currency) -> Either<MoneyError, Self> {
        amount.parse::<Decimal>().map_or_else(
            |_| Either::Left(MoneyError::InvalidAmount(amount.to_string())),
            |decimal| Either::Right(Self::from_decimal(decimal, currency)),
        )
    }

    /// Creates a zero amount in the specified currency.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency for the zero amount
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let zero = Money::zero(Currency::USD);
    /// assert_eq!(zero.amount().to_string(), "0");
    /// ```
    #[must_use]
    pub const fn zero(currency: Currency) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency,
        }
    }

    /// Returns the amount as a `Decimal`.
    #[must_use]
    pub const fn amount(&self) -> &Decimal {
        &self.amount
    }

    /// Returns the currency.
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns `true` if the amount is zero.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Decimal::is_zero is not const
    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }

    /// Returns `true` if the amount is positive (greater than zero).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Decimal methods are not const
    pub fn is_positive(&self) -> bool {
        self.amount.is_sign_positive() && !self.amount.is_zero()
    }

    /// Returns `true` if the amount is negative.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Decimal::is_sign_negative is not const
    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative()
    }

    /// Adds two money values.
    ///
    /// Returns `Either::Left(MoneyError::CurrencyMismatch)` if currencies differ.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let m1 = Money::new(100, Currency::JPY);
    /// let m2 = Money::new(50, Currency::JPY);
    /// let result = m1.add(&m2);
    /// assert!(result.is_right());
    /// ```
    pub fn add(&self, other: &Self) -> Either<MoneyError, Self> {
        if self.currency == other.currency {
            Either::Right(Self {
                amount: self.amount + other.amount,
                currency: self.currency,
            })
        } else {
            Either::Left(MoneyError::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        }
    }

    /// Subtracts one money value from another.
    ///
    /// Returns `Either::Left(MoneyError::CurrencyMismatch)` if currencies differ.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let m1 = Money::new(100, Currency::JPY);
    /// let m2 = Money::new(30, Currency::JPY);
    /// let result = m1.subtract(&m2);
    /// assert!(result.is_right());
    /// ```
    pub fn subtract(&self, other: &Self) -> Either<MoneyError, Self> {
        if self.currency == other.currency {
            Either::Right(Self {
                amount: self.amount - other.amount,
                currency: self.currency,
            })
        } else {
            Either::Left(MoneyError::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        }
    }

    /// Returns the absolute value of the money.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            amount: self.amount.abs(),
            currency: self.currency,
        }
    }

    /// Negates the money amount.
    #[must_use]
    pub fn negate(&self) -> Self {
        Self {
            amount: -self.amount,
            currency: self.currency,
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.amount, self.currency)
    }
}

impl PartialOrd for Money {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.currency == other.currency {
            self.amount.partial_cmp(&other.amount)
        } else {
            None
        }
    }
}

/// Implements `Semigroup` for `Money`.
///
/// Combines two money values by adding their amounts.
///
/// # Panics
///
/// Panics if the two money values have different currencies.
/// For safe currency-checked addition, use `Money::add` instead.
impl Semigroup for Money {
    fn combine(self, other: Self) -> Self {
        assert_eq!(
            self.currency, other.currency,
            "Cannot combine money with different currencies: {} vs {}",
            self.currency, other.currency
        );
        Self {
            amount: self.amount + other.amount,
            currency: self.currency,
        }
    }

    fn combine_ref(&self, other: &Self) -> Self
    where
        Self: Clone,
    {
        assert_eq!(
            self.currency, other.currency,
            "Cannot combine money with different currencies: {} vs {}",
            self.currency, other.currency
        );
        Self {
            amount: self.amount + other.amount,
            currency: self.currency,
        }
    }
}

/// Implements `Monoid` for `Money`.
///
/// The identity element is zero JPY. For zero values in other currencies,
/// use `Money::zero(currency)`.
///
/// # Note
///
/// This implementation uses JPY as the default currency for the empty value.
/// When combining with money of a different currency, ensure you use
/// `Money::zero(currency)` instead of `Money::empty()`.
impl Monoid for Money {
    fn empty() -> Self {
        Self::zero(Currency::JPY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Currency Tests
    // =========================================================================

    #[rstest]
    fn currency_display() {
        assert_eq!(format!("{}", Currency::JPY), "JPY");
        assert_eq!(format!("{}", Currency::USD), "USD");
        assert_eq!(format!("{}", Currency::EUR), "EUR");
    }

    #[rstest]
    fn currency_decimal_places() {
        assert_eq!(Currency::JPY.decimal_places(), 0);
        assert_eq!(Currency::USD.decimal_places(), 2);
        assert_eq!(Currency::EUR.decimal_places(), 2);
    }

    // =========================================================================
    // Money Construction Tests
    // =========================================================================

    #[rstest]
    fn new_creates_money() {
        let money = Money::new(1000, Currency::JPY);

        assert_eq!(*money.amount(), Decimal::from(1000));
        assert_eq!(money.currency(), Currency::JPY);
    }

    #[rstest]
    fn from_decimal_creates_money() {
        let amount = Decimal::new(1050, 2); // 10.50
        let money = Money::from_decimal(amount, Currency::USD);

        assert_eq!(*money.amount(), amount);
        assert_eq!(money.currency(), Currency::USD);
    }

    #[rstest]
    fn parse_valid_amount_returns_right() {
        let result = Money::parse("10.50", Currency::USD);

        assert!(result.is_right());
        let money = result.unwrap_right();
        assert_eq!(money.amount().to_string(), "10.50");
        assert_eq!(money.currency(), Currency::USD);
    }

    #[rstest]
    fn parse_invalid_amount_returns_left() {
        let result = Money::parse("not-a-number", Currency::USD);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(error, MoneyError::InvalidAmount("not-a-number".to_string()));
    }

    #[rstest]
    fn zero_creates_zero_money() {
        let zero = Money::zero(Currency::EUR);

        assert!(zero.is_zero());
        assert_eq!(zero.currency(), Currency::EUR);
    }

    // =========================================================================
    // Money Property Tests
    // =========================================================================

    #[rstest]
    fn is_zero_returns_true_for_zero() {
        let zero = Money::new(0, Currency::JPY);
        assert!(zero.is_zero());
    }

    #[rstest]
    fn is_zero_returns_false_for_non_zero() {
        let money = Money::new(100, Currency::JPY);
        assert!(!money.is_zero());
    }

    #[rstest]
    fn is_positive_returns_true_for_positive() {
        let money = Money::new(100, Currency::JPY);
        assert!(money.is_positive());
    }

    #[rstest]
    fn is_positive_returns_false_for_zero() {
        let money = Money::new(0, Currency::JPY);
        assert!(!money.is_positive());
    }

    #[rstest]
    fn is_positive_returns_false_for_negative() {
        let money = Money::new(-100, Currency::JPY);
        assert!(!money.is_positive());
    }

    #[rstest]
    fn is_negative_returns_true_for_negative() {
        let money = Money::new(-100, Currency::JPY);
        assert!(money.is_negative());
    }

    #[rstest]
    fn is_negative_returns_false_for_positive() {
        let money = Money::new(100, Currency::JPY);
        assert!(!money.is_negative());
    }

    // =========================================================================
    // Money Arithmetic Tests
    // =========================================================================

    #[rstest]
    fn add_same_currency_returns_right() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(50, Currency::JPY);
        let result = m1.add(&m2);

        assert!(result.is_right());
        let sum = result.unwrap_right();
        assert_eq!(*sum.amount(), Decimal::from(150));
    }

    #[rstest]
    fn add_different_currency_returns_left() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(50, Currency::USD);
        let result = m1.add(&m2);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            MoneyError::CurrencyMismatch {
                left: Currency::JPY,
                right: Currency::USD,
            }
        );
    }

    #[rstest]
    fn subtract_same_currency_returns_right() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(30, Currency::JPY);
        let result = m1.subtract(&m2);

        assert!(result.is_right());
        let difference = result.unwrap_right();
        assert_eq!(*difference.amount(), Decimal::from(70));
    }

    #[rstest]
    fn subtract_different_currency_returns_left() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(30, Currency::USD);
        let result = m1.subtract(&m2);

        assert!(result.is_left());
    }

    #[rstest]
    fn abs_positive_remains_positive() {
        let money = Money::new(100, Currency::JPY);
        let result = money.abs();

        assert_eq!(*result.amount(), Decimal::from(100));
    }

    #[rstest]
    fn abs_negative_becomes_positive() {
        let money = Money::new(-100, Currency::JPY);
        let result = money.abs();

        assert_eq!(*result.amount(), Decimal::from(100));
    }

    #[rstest]
    fn negate_positive_becomes_negative() {
        let money = Money::new(100, Currency::JPY);
        let result = money.negate();

        assert_eq!(*result.amount(), Decimal::from(-100));
    }

    #[rstest]
    fn negate_negative_becomes_positive() {
        let money = Money::new(-100, Currency::JPY);
        let result = money.negate();

        assert_eq!(*result.amount(), Decimal::from(100));
    }

    // =========================================================================
    // Semigroup Tests
    // =========================================================================

    #[rstest]
    fn semigroup_combine_same_currency() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(50, Currency::JPY);
        let result = m1.combine(m2);

        assert_eq!(*result.amount(), Decimal::from(150));
    }

    #[rstest]
    #[should_panic(expected = "Cannot combine money with different currencies")]
    fn semigroup_combine_different_currency_panics() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(50, Currency::USD);
        let _ = m1.combine(m2);
    }

    #[rstest]
    fn semigroup_combine_ref_same_currency() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(50, Currency::JPY);
        let result = m1.combine_ref(&m2);

        assert_eq!(*result.amount(), Decimal::from(150));
    }

    #[rstest]
    fn semigroup_associativity() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(200, Currency::JPY);
        let m3 = Money::new(300, Currency::JPY);

        let left = m1.clone().combine(m2.clone()).combine(m3.clone());
        let right = m1.combine(m2.combine(m3));

        assert_eq!(left, right);
    }

    // =========================================================================
    // Monoid Tests
    // =========================================================================

    #[rstest]
    fn monoid_empty_is_zero_jpy() {
        let empty = Money::empty();

        assert!(empty.is_zero());
        assert_eq!(empty.currency(), Currency::JPY);
    }

    #[rstest]
    fn monoid_left_identity() {
        let money = Money::new(100, Currency::JPY);
        let result = Money::empty().combine(money.clone());

        assert_eq!(result, money);
    }

    #[rstest]
    fn monoid_right_identity() {
        let money = Money::new(100, Currency::JPY);
        let result = money.clone().combine(Money::empty());

        assert_eq!(result, money);
    }

    #[rstest]
    fn monoid_combine_all() {
        let amounts = vec![
            Money::new(100, Currency::JPY),
            Money::new(200, Currency::JPY),
            Money::new(300, Currency::JPY),
        ];
        let result = Money::combine_all(amounts);

        assert_eq!(*result.amount(), Decimal::from(600));
    }

    #[rstest]
    fn monoid_combine_all_empty() {
        let amounts: Vec<Money> = vec![];
        let result = Money::combine_all(amounts);

        assert!(result.is_zero());
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_formats_correctly() {
        let money = Money::new(1000, Currency::JPY);
        assert_eq!(format!("{money}"), "1000 JPY");
    }

    // =========================================================================
    // PartialOrd Tests
    // =========================================================================

    #[rstest]
    fn partial_cmp_same_currency() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(200, Currency::JPY);

        assert!(m1 < m2);
        assert!(m2 > m1);
    }

    #[rstest]
    fn partial_cmp_equal() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(100, Currency::JPY);

        assert_eq!(m1.partial_cmp(&m2), Some(Ordering::Equal));
    }

    #[rstest]
    fn partial_cmp_different_currency_returns_none() {
        let m1 = Money::new(100, Currency::JPY);
        let m2 = Money::new(100, Currency::USD);

        assert_eq!(m1.partial_cmp(&m2), None);
    }

    // =========================================================================
    // MoneyError Tests
    // =========================================================================

    #[rstest]
    fn money_error_display_currency_mismatch() {
        let error = MoneyError::CurrencyMismatch {
            left: Currency::JPY,
            right: Currency::USD,
        };
        assert_eq!(format!("{error}"), "Currency mismatch: JPY vs USD");
    }

    #[rstest]
    fn money_error_display_invalid_amount() {
        let error = MoneyError::InvalidAmount("bad".to_string());
        assert_eq!(format!("{error}"), "Invalid amount: bad");
    }

    #[rstest]
    fn money_error_display_negative_amount() {
        let error = MoneyError::NegativeAmount;
        assert_eq!(format!("{error}"), "Amount cannot be negative");
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_roundtrip() {
        let original = Money::new(1000, Currency::JPY);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Money = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }
}
