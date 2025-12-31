//! 数量型の定義
//!
//! `UnitQuantity`, `KilogramQuantity`, `OrderQuantity` を定義する。

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;

use super::constrained_type;
use super::error::ValidationError;
use super::product_types::ProductCode;

// =============================================================================
// UnitQuantity
// =============================================================================

/// 個数を表す整数型
///
/// 1から1000の範囲に制約される。
/// Widget 製品の数量に使用する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::UnitQuantity;
///
/// let quantity = UnitQuantity::create("Quantity", 100).unwrap();
/// assert_eq!(quantity.value(), 100);
///
/// // 範囲外はエラー
/// assert!(UnitQuantity::create("Quantity", 0).is_err());
/// assert!(UnitQuantity::create("Quantity", 1001).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnitQuantity(u32);

/// `UnitQuantity` の最小値
const UNIT_QUANTITY_MIN: u32 = 1;

/// `UnitQuantity` の最大値
const UNIT_QUANTITY_MAX: u32 = 1000;

impl UnitQuantity {
    /// 整数から `UnitQuantity` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力整数
    ///
    /// # Returns
    ///
    /// * `Ok(UnitQuantity)` - バリデーション成功時
    /// * `Err(ValidationError)` - 範囲外の場合
    ///
    /// # Errors
    ///
    /// 値が 1 未満または 1000 を超える場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: u32) -> Result<Self, ValidationError> {
        constrained_type::create_integer(
            field_name,
            Self,
            UNIT_QUANTITY_MIN,
            UNIT_QUANTITY_MAX,
            value,
        )
    }

    /// 内部の整数値を返す
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

// =============================================================================
// KilogramQuantity
// =============================================================================

/// 重量（キログラム）を表す小数型
///
/// 0.05から100.00の範囲に制約される。
/// Gizmo 製品の数量に使用する。
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
/// // 範囲外はエラー
/// assert!(KilogramQuantity::create("Weight", Decimal::from_str("0.04").unwrap()).is_err());
/// assert!(KilogramQuantity::create("Weight", Decimal::from_str("100.01").unwrap()).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KilogramQuantity(Decimal);

impl KilogramQuantity {
    /// `KilogramQuantity` の最小値を取得する
    fn min_value() -> Decimal {
        Decimal::from_str("0.05").expect("Valid decimal literal")
    }

    /// `KilogramQuantity` の最大値を取得する
    fn max_value() -> Decimal {
        Decimal::from_str("100.00").expect("Valid decimal literal")
    }

    /// 小数から `KilogramQuantity` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力小数
    ///
    /// # Returns
    ///
    /// * `Ok(KilogramQuantity)` - バリデーション成功時
    /// * `Err(ValidationError)` - 範囲外の場合
    ///
    /// # Errors
    ///
    /// 値が 0.05 未満または 100.00 を超える場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: Decimal) -> Result<Self, ValidationError> {
        constrained_type::create_decimal(
            field_name,
            Self,
            Self::min_value(),
            Self::max_value(),
            value,
        )
    }

    /// 内部の小数値を返す
    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.0
    }
}

// =============================================================================
// OrderQuantity
// =============================================================================

/// 注文数量を表す直和型
///
/// 個数（Unit）または重量（Kilogram）のいずれかを保持する。
/// 製品コードによってどちらを使用するかが決まる。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::{OrderQuantity, ProductCode};
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// // Widget 製品の場合は個数
/// let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let unit_quantity = OrderQuantity::create(
///     "Quantity",
///     &widget_code,
///     Decimal::from_str("10").unwrap()
/// ).unwrap();
/// assert!(matches!(unit_quantity, OrderQuantity::Unit(_)));
///
/// // Gizmo 製品の場合は重量
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
    /// Widget 製品の個数
    Unit(UnitQuantity),
    /// Gizmo 製品の重量
    Kilogram(KilogramQuantity),
}

impl OrderQuantity {
    /// 製品コードと数量から `OrderQuantity` を生成する
    ///
    /// Widget なら `UnitQuantity`、Gizmo なら `KilogramQuantity` として解釈する。
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `product_code` - 製品コード
    /// * `quantity` - 数量（Decimal）
    ///
    /// # Returns
    ///
    /// * `Ok(OrderQuantity)` - バリデーション成功時
    /// * `Err(ValidationError)` - 数量が範囲外の場合
    ///
    /// # Errors
    ///
    /// Widget 製品で整数変換できない場合、または数量が範囲外の場合に
    /// `ValidationError` を返す。
    pub fn create(
        field_name: &str,
        product_code: &ProductCode,
        quantity: Decimal,
    ) -> Result<Self, ValidationError> {
        match product_code {
            ProductCode::Widget(widget_code) => {
                // Decimal を u32 に変換
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

    /// 数量を Decimal として返す
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
        // Copy トレイトが実装されていることを確認
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
        // Copy トレイトが実装されていることを確認
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
        // Widget 製品で範囲外の数量
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
        // Widget 製品で小数を指定した場合
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("10.5").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        // to_u32() は切り捨てを行うので、10.5 -> 10 として成功するはず
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from(10));
    }

    #[rstest]
    fn test_order_quantity_create_unit_negative() {
        // Widget 製品で負の数
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = Decimal::from_str("-1").unwrap();
        let result = OrderQuantity::create("Quantity", &product_code, quantity);

        // 負の Decimal は to_u32() で None を返す
        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_quantity_create_kilogram_invalid() {
        // Gizmo 製品で範囲外の数量
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
