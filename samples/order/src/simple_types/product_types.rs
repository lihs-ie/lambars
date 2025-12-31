//! 製品コード型の定義
//!
//! `WidgetCode`, `GizmoCode`, `ProductCode` を定義する。

use regex::Regex;
use std::sync::LazyLock;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// WidgetCode
// =============================================================================

/// Widget 製品のコードを表す型
///
/// "W" で始まり、続いて4桁の数字（W\d{4} パターン）。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::WidgetCode;
///
/// let code = WidgetCode::create("ProductCode", "W1234").unwrap();
/// assert_eq!(code.value(), "W1234");
///
/// // 形式が不正な場合はエラー
/// assert!(WidgetCode::create("ProductCode", "G123").is_err());
/// assert!(WidgetCode::create("ProductCode", "W123").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetCode(String);

/// `WidgetCode` の正規表現パターン
static WIDGET_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^W\d{4}$").expect("Invalid widget code regex pattern"));

impl WidgetCode {
    /// W + 4桁の形式の文字列から `WidgetCode` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(WidgetCode)` - バリデーション成功時
    /// * `Err(ValidationError)` - パターン不一致時
    ///
    /// # Errors
    ///
    /// 空文字列またはパターン不一致の場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, Self, &WIDGET_CODE_PATTERN, value)
    }

    /// 内部のコード文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// GizmoCode
// =============================================================================

/// Gizmo 製品のコードを表す型
///
/// "G" で始まり、続いて3桁の数字（G\d{3} パターン）。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::GizmoCode;
///
/// let code = GizmoCode::create("ProductCode", "G123").unwrap();
/// assert_eq!(code.value(), "G123");
///
/// // 形式が不正な場合はエラー
/// assert!(GizmoCode::create("ProductCode", "W1234").is_err());
/// assert!(GizmoCode::create("ProductCode", "G12").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GizmoCode(String);

/// `GizmoCode` の正規表現パターン
static GIZMO_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^G\d{3}$").expect("Invalid gizmo code regex pattern"));

impl GizmoCode {
    /// G + 3桁の形式の文字列から `GizmoCode` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(GizmoCode)` - バリデーション成功時
    /// * `Err(ValidationError)` - パターン不一致時
    ///
    /// # Errors
    ///
    /// 空文字列またはパターン不一致の場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, Self, &GIZMO_CODE_PATTERN, value)
    }

    /// 内部のコード文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// ProductCode
// =============================================================================

/// 製品コードを表す直和型
///
/// Widget コードまたは Gizmo コードのいずれかを保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ProductCode;
///
/// // Widget コード
/// let widget = ProductCode::create("ProductCode", "W1234").unwrap();
/// assert!(matches!(widget, ProductCode::Widget(_)));
/// assert_eq!(widget.value(), "W1234");
///
/// // Gizmo コード
/// let gizmo = ProductCode::create("ProductCode", "G123").unwrap();
/// assert!(matches!(gizmo, ProductCode::Gizmo(_)));
/// assert_eq!(gizmo.value(), "G123");
///
/// // 不明な形式はエラー
/// assert!(ProductCode::create("ProductCode", "X999").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProductCode {
    /// Widget 製品のコード
    Widget(WidgetCode),
    /// Gizmo 製品のコード
    Gizmo(GizmoCode),
}

impl ProductCode {
    /// 文字列から `ProductCode` を生成する
    ///
    /// 先頭文字で Widget か Gizmo かを判定する。
    /// - "W" で始まる場合: `WidgetCode` として解釈
    /// - "G" で始まる場合: `GizmoCode` として解釈
    /// - それ以外: エラー
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `code` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(ProductCode)` - バリデーション成功時
    /// * `Err(ValidationError)` - パターン不一致時
    ///
    /// # Errors
    ///
    /// 空文字列または認識できない形式の場合に `ValidationError` を返す。
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

    /// 内部のコード文字列への参照を返す
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
        // W で始まるが形式が不正
        let result = ProductCode::create("ProductCode", "W12");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_product_code_create_invalid_gizmo() {
        // G で始まるが形式が不正
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
        // 同じ数字でも型が違う
        let widget = ProductCode::create("ProductCode", "W0123").unwrap();
        let gizmo = ProductCode::create("ProductCode", "G012").unwrap();

        assert!(matches!(widget, ProductCode::Widget(_)));
        assert!(matches!(gizmo, ProductCode::Gizmo(_)));
        assert_ne!(widget, gizmo);
    }
}
