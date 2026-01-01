//! 金額型の定義
//!
//! `Price`, `BillingAmount` を定義する。

use lambars::typeclass::Foldable;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// Price
// =============================================================================

/// 単価を表す小数型
///
/// 0.0から1000.00の範囲に制約される。
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
/// // 範囲外はエラー
/// assert!(Price::create(Decimal::from_str("-1.0").unwrap()).is_err());
/// assert!(Price::create(Decimal::from_str("1000.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Price(Decimal);

impl Price {
    /// Price のフィールド名
    const FIELD_NAME: &'static str = "Price";

    /// Price の最小値を取得する
    fn min_value() -> Decimal {
        Decimal::from_str("0.0").expect("Valid decimal literal")
    }

    /// Price の最大値を取得する
    fn max_value() -> Decimal {
        Decimal::from_str("1000.00").expect("Valid decimal literal")
    }

    /// 小数から Price を生成する
    ///
    /// # Arguments
    ///
    /// * `value` - 入力小数
    ///
    /// # Returns
    ///
    /// * `Ok(Price)` - バリデーション成功時
    /// * `Err(ValidationError)` - 範囲外の場合
    ///
    /// # Errors
    ///
    /// 値が 0.0 未満または 1000.00 を超える場合に `ValidationError` を返す。
    pub fn create(value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            Self::FIELD_NAME,
            Price,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// バリデーションなしで Price を生成する
    ///
    /// 値が有効であることが確実な場合のみ使用する。
    ///
    /// # Panics
    ///
    /// 範囲外の値が渡された場合に panic する。
    #[must_use]
    pub fn unsafe_create(value: Decimal) -> Self {
        Self::create(value)
            .unwrap_or_else(|error| panic!("Not expecting Price to be out of bounds: {error}"))
    }

    /// 数量を掛けて新しい Price を生成する
    ///
    /// # Arguments
    ///
    /// * `quantity` - 数量
    ///
    /// # Returns
    ///
    /// * `Ok(Price)` - 新しい価格が範囲内の場合
    /// * `Err(ValidationError)` - 新しい価格が範囲外の場合
    ///
    /// # Errors
    ///
    /// 乗算結果が 1000.00 を超える場合に `ValidationError` を返す。
    pub fn multiply(&self, quantity: Decimal) -> Result<Self, ValidationError> {
        Self::create(quantity * self.0)
    }

    /// 内部の小数値を返す
    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.0
    }
}

// =============================================================================
// BillingAmount
// =============================================================================

/// 請求金額を表す小数型
///
/// 0.0から10000.00の範囲に制約される。
/// 複数の Price の合計として使用される。
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
/// // 範囲外はエラー
/// assert!(BillingAmount::create(Decimal::from_str("-1.0").unwrap()).is_err());
/// assert!(BillingAmount::create(Decimal::from_str("10000.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BillingAmount(Decimal);

impl BillingAmount {
    /// `BillingAmount` のフィールド名
    const FIELD_NAME: &'static str = "BillingAmount";

    /// `BillingAmount` の最小値を取得する
    fn min_value() -> Decimal {
        Decimal::from_str("0.0").expect("Valid decimal literal")
    }

    /// `BillingAmount` の最大値を取得する
    fn max_value() -> Decimal {
        Decimal::from_str("10000.00").expect("Valid decimal literal")
    }

    /// 小数から `BillingAmount` を生成する
    ///
    /// # Arguments
    ///
    /// * `value` - 入力小数
    ///
    /// # Returns
    ///
    /// * `Ok(BillingAmount)` - バリデーション成功時
    /// * `Err(ValidationError)` - 範囲外の場合
    ///
    /// # Errors
    ///
    /// 値が 0.0 未満または 10000.00 を超える場合に `ValidationError` を返す。
    pub fn create(value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            Self::FIELD_NAME,
            BillingAmount,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// `Price` のスライスを合計して `BillingAmount` を生成する
    ///
    /// lambars の `Foldable` トレイトを使用して畳み込みを行う。
    ///
    /// # Arguments
    ///
    /// * `prices` - Price のスライス
    ///
    /// # Returns
    ///
    /// * `Ok(BillingAmount)` - 合計が範囲内の場合
    /// * `Err(ValidationError)` - 合計が範囲外の場合
    ///
    /// # Errors
    ///
    /// 合計が 10000.00 を超える場合に `ValidationError` を返す。
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

    /// 内部の小数値を返す
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
        // 10個の1000円で10000円（最大値）
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
        // 11個の1000円で11000円（最大値超過）
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
