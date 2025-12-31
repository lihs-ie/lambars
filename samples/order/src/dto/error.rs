//! エラー DTO
//!
//! API レスポンスのエラーをシリアライズするための型を定義する。

use serde::{Deserialize, Serialize};

use crate::workflow::{PlaceOrderError, PricingError, RemoteServiceError, WorkflowValidationError};

// =============================================================================
// PlaceOrderErrorDto (REQ-086)
// =============================================================================

/// `PlaceOrder` ワークフローのエラー DTO
///
/// ワークフローで発生したエラーをシリアライズするための型。
/// `type` フィールドで判別する隣接タグ形式。
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::PlaceOrderErrorDto;
/// use order_taking_sample::workflow::{PlaceOrderError, PricingError};
///
/// let error = PlaceOrderError::Pricing(PricingError::new("Product not found"));
/// let dto = PlaceOrderErrorDto::from_domain(&error);
///
/// match dto {
///     PlaceOrderErrorDto::Pricing { message } => {
///         assert_eq!(message, "Product not found");
///     }
///     _ => panic!("Expected Pricing error"),
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlaceOrderErrorDto {
    /// バリデーションエラー
    Validation {
        /// フィールド名
        field_name: String,
        /// エラーメッセージ
        message: String,
    },
    /// 価格計算エラー
    Pricing {
        /// エラーメッセージ
        message: String,
    },
    /// 外部サービスエラー
    RemoteService {
        /// サービス名
        service_name: String,
        /// サービスエンドポイント
        service_endpoint: String,
        /// エラーメッセージ
        message: String,
    },
}

impl PlaceOrderErrorDto {
    /// ドメインの `PlaceOrderError` から `PlaceOrderErrorDto` を生成する
    ///
    /// 純粋関数として DTO に変換する。
    ///
    /// # Arguments
    ///
    /// * `error` - 変換元の `PlaceOrderError`
    ///
    /// # Returns
    ///
    /// `PlaceOrderErrorDto` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::PlaceOrderErrorDto;
    /// use order_taking_sample::workflow::{PlaceOrderError, PricingError};
    ///
    /// let error = PlaceOrderError::Pricing(PricingError::new("Product not found"));
    /// let dto = PlaceOrderErrorDto::from_domain(&error);
    ///
    /// let json = serde_json::to_string(&dto).unwrap();
    /// assert!(json.contains("\"type\":\"Pricing\""));
    /// ```
    #[must_use]
    pub fn from_domain(error: &PlaceOrderError) -> Self {
        match error {
            PlaceOrderError::Validation(e) => Self::from_validation_error(e),
            PlaceOrderError::Pricing(e) => Self::from_pricing_error(e),
            PlaceOrderError::RemoteService(e) => Self::from_remote_service_error(e),
        }
    }

    /// `WorkflowValidationError` から `PlaceOrderErrorDto` を生成する
    #[must_use]
    fn from_validation_error(error: &WorkflowValidationError) -> Self {
        Self::Validation {
            field_name: error.field_name.clone(),
            message: error.message.clone(),
        }
    }

    /// `PricingError` から `PlaceOrderErrorDto` を生成する
    #[must_use]
    fn from_pricing_error(error: &PricingError) -> Self {
        Self::Pricing {
            message: error.message().to_string(),
        }
    }

    /// `RemoteServiceError` から `PlaceOrderErrorDto` を生成する
    #[must_use]
    fn from_remote_service_error(error: &RemoteServiceError) -> Self {
        Self::RemoteService {
            service_name: error.service().name().to_string(),
            service_endpoint: error.service().endpoint().to_string(),
            message: error.exception_message().to_string(),
        }
    }
}
