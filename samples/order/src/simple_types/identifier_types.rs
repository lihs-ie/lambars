//! ID 型の定義
//!
//! `OrderId`, `OrderLineId` を定義する。

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// OrderId
// =============================================================================

/// 注文を一意に識別する ID 型
///
/// 空でない50文字以下の文字列。
/// `HashMap` のキーとして使用可能（`Hash` を derive）。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::OrderId;
///
/// let order_id = OrderId::create("OrderId", "ORD-2024-001").unwrap();
/// assert_eq!(order_id.value(), "ORD-2024-001");
///
/// // 空文字列はエラー
/// assert!(OrderId::create("OrderId", "").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderId(String);

/// `OrderId` の最大文字数
const ORDER_ID_MAX_LENGTH: usize = 50;

impl OrderId {
    /// 文字列から `OrderId` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(OrderId)` - バリデーション成功時
    /// * `Err(ValidationError)` - 空文字列または50文字超過時
    ///
    /// # Errors
    ///
    /// 空文字列または50文字を超える場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_string(field_name, OrderId, ORDER_ID_MAX_LENGTH, value)
    }

    /// 内部の ID 文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// OrderLineId
// =============================================================================

/// 注文明細を一意に識別する ID 型
///
/// 空でない50文字以下の文字列。
/// `HashMap` のキーとして使用可能（`Hash` を derive）。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::OrderLineId;
///
/// let line_id = OrderLineId::create("OrderLineId", "LINE-001").unwrap();
/// assert_eq!(line_id.value(), "LINE-001");
///
/// // 空文字列はエラー
/// assert!(OrderLineId::create("OrderLineId", "").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderLineId(String);

/// `OrderLineId` の最大文字数
const ORDER_LINE_ID_MAX_LENGTH: usize = 50;

impl OrderLineId {
    /// 文字列から `OrderLineId` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(OrderLineId)` - バリデーション成功時
    /// * `Err(ValidationError)` - 空文字列または50文字超過時
    ///
    /// # Errors
    ///
    /// 空文字列または50文字を超える場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_string(field_name, OrderLineId, ORDER_LINE_ID_MAX_LENGTH, value)
    }

    /// 内部の ID 文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::collections::HashMap;

    // =========================================================================
    // OrderId Tests
    // =========================================================================

    #[rstest]
    fn test_order_id_create_valid() {
        let result = OrderId::create("OrderId", "ORD-2024-001");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "ORD-2024-001");
    }

    #[rstest]
    fn test_order_id_create_empty() {
        let result = OrderId::create("OrderId", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderId");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_order_id_create_too_long() {
        let long_id = "a".repeat(51);
        let result = OrderId::create("OrderId", &long_id);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderId");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_order_id_create_exactly_50_chars() {
        let exact_id = "a".repeat(50);
        let result = OrderId::create("OrderId", &exact_id);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_order_id_hash() {
        // HashMap のキーとして使用できることを確認
        let order_id = OrderId::create("OrderId", "ORD-001").unwrap();
        let mut map: HashMap<OrderId, String> = HashMap::new();

        map.insert(order_id.clone(), "Test Order".to_string());

        assert_eq!(map.get(&order_id), Some(&"Test Order".to_string()));
    }

    #[rstest]
    fn test_order_id_value() {
        let order_id = OrderId::create("OrderId", "TEST-123").unwrap();

        assert_eq!(order_id.value(), "TEST-123");
    }

    #[rstest]
    fn test_order_id_clone() {
        let original = OrderId::create("OrderId", "ORD-001").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_order_id_eq() {
        let id1 = OrderId::create("OrderId", "ORD-001").unwrap();
        let id2 = OrderId::create("OrderId", "ORD-001").unwrap();
        let id3 = OrderId::create("OrderId", "ORD-002").unwrap();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // =========================================================================
    // OrderLineId Tests
    // =========================================================================

    #[rstest]
    fn test_order_line_id_create_valid() {
        let result = OrderLineId::create("OrderLineId", "LINE-001");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "LINE-001");
    }

    #[rstest]
    fn test_order_line_id_create_empty() {
        let result = OrderLineId::create("OrderLineId", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderLineId");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_order_line_id_create_too_long() {
        let long_id = "a".repeat(51);
        let result = OrderLineId::create("OrderLineId", &long_id);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderLineId");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_order_line_id_create_exactly_50_chars() {
        let exact_id = "a".repeat(50);
        let result = OrderLineId::create("OrderLineId", &exact_id);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_order_line_id_hash() {
        // HashMap のキーとして使用できることを確認
        let line_id = OrderLineId::create("OrderLineId", "LINE-001").unwrap();
        let mut map: HashMap<OrderLineId, i32> = HashMap::new();

        map.insert(line_id.clone(), 100);

        assert_eq!(map.get(&line_id), Some(&100));
    }

    #[rstest]
    fn test_order_line_id_value() {
        let line_id = OrderLineId::create("OrderLineId", "LINE-ABC").unwrap();

        assert_eq!(line_id.value(), "LINE-ABC");
    }

    #[rstest]
    fn test_order_line_id_clone() {
        let original = OrderLineId::create("OrderLineId", "LINE-001").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_order_line_id_eq() {
        let id1 = OrderLineId::create("OrderLineId", "LINE-001").unwrap();
        let id2 = OrderLineId::create("OrderLineId", "LINE-001").unwrap();
        let id3 = OrderLineId::create("OrderLineId", "LINE-002").unwrap();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
