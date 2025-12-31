//! その他の型の定義
//!
//! `VipStatus`, `PromotionCode`, `PdfAttachment` を定義する。

use super::error::ValidationError;

// =============================================================================
// VipStatus
// =============================================================================

/// 顧客の VIP ステータスを表す列挙型
///
/// Normal（通常）または Vip（VIP 顧客）のいずれか。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::VipStatus;
///
/// let normal = VipStatus::create("VipStatus", "Normal").unwrap();
/// assert!(matches!(normal, VipStatus::Normal));
/// assert_eq!(normal.value(), "Normal");
///
/// let vip = VipStatus::create("VipStatus", "VIP").unwrap();
/// assert!(matches!(vip, VipStatus::Vip));
/// assert_eq!(vip.value(), "VIP");
///
/// // 無効な値はエラー
/// assert!(VipStatus::create("VipStatus", "Premium").is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VipStatus {
    /// 通常の顧客
    Normal,
    /// VIP 顧客
    Vip,
}

impl VipStatus {
    /// 文字列から `VipStatus` を生成する
    ///
    /// "normal", "Normal", "vip", "VIP" を受け入れる。
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(VipStatus)` - バリデーション成功時
    /// * `Err(ValidationError)` - 無効な値の場合
    ///
    /// # Errors
    ///
    /// "normal", "Normal", "vip", "VIP" 以外の値の場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        match value {
            "normal" | "Normal" => Ok(Self::Normal),
            "vip" | "VIP" => Ok(Self::Vip),
            _ => Err(ValidationError::new(
                field_name,
                "Must be one of 'Normal', 'VIP'",
            )),
        }
    }

    /// `VipStatus` を文字列として返す
    #[must_use]
    pub const fn value(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Vip => "VIP",
        }
    }
}

// =============================================================================
// PromotionCode
// =============================================================================

/// プロモーションコードを表す型
///
/// 特にバリデーションなしの単純なラッパー型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::PromotionCode;
///
/// let promo = PromotionCode::new("SUMMER2024".to_string());
/// assert_eq!(promo.value(), "SUMMER2024");
///
/// // 空文字列も許可される（バリデーションなし）
/// let empty = PromotionCode::new(String::new());
/// assert_eq!(empty.value(), "");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PromotionCode(String);

impl PromotionCode {
    /// 文字列から `PromotionCode` を生成する
    ///
    /// バリデーションなしで直接生成する。
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    /// 内部のプロモーションコード文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// PdfAttachment
// =============================================================================

/// PDF 添付ファイルを表す構造体
///
/// ファイル名とバイトデータを保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::PdfAttachment;
///
/// let pdf = PdfAttachment::new(
///     "invoice.pdf".to_string(),
///     vec![0x25, 0x50, 0x44, 0x46]  // %PDF
/// );
/// assert_eq!(pdf.name(), "invoice.pdf");
/// assert_eq!(pdf.bytes(), &[0x25, 0x50, 0x44, 0x46]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfAttachment {
    /// ファイル名
    name: String,
    /// PDF のバイトデータ
    bytes: Vec<u8>,
}

impl PdfAttachment {
    /// ファイル名とバイトデータから `PdfAttachment` を生成する
    #[must_use]
    pub const fn new(name: String, bytes: Vec<u8>) -> Self {
        Self { name, bytes }
    }

    /// ファイル名への参照を返す
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// バイトデータへの参照を返す
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
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
    // VipStatus Tests
    // =========================================================================

    #[rstest]
    fn test_vip_status_create_normal_lowercase() {
        let result = VipStatus::create("VipStatus", "normal");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VipStatus::Normal));
    }

    #[rstest]
    fn test_vip_status_create_normal_capitalized() {
        let result = VipStatus::create("VipStatus", "Normal");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VipStatus::Normal));
    }

    #[rstest]
    fn test_vip_status_create_vip_lowercase() {
        let result = VipStatus::create("VipStatus", "vip");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VipStatus::Vip));
    }

    #[rstest]
    fn test_vip_status_create_vip_uppercase() {
        let result = VipStatus::create("VipStatus", "VIP");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VipStatus::Vip));
    }

    #[rstest]
    fn test_vip_status_create_invalid() {
        let result = VipStatus::create("VipStatus", "Premium");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "VipStatus");
        assert_eq!(error.message, "Must be one of 'Normal', 'VIP'");
    }

    #[rstest]
    fn test_vip_status_create_empty() {
        let result = VipStatus::create("VipStatus", "");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_vip_status_create_mixed_case() {
        // "Vip" や "NORMAL" はエラー（厳密なマッチング）
        let result1 = VipStatus::create("VipStatus", "Vip");
        let result2 = VipStatus::create("VipStatus", "NORMAL");

        assert!(result1.is_err());
        assert!(result2.is_err());
    }

    #[rstest]
    fn test_vip_status_value_normal() {
        let status = VipStatus::create("VipStatus", "Normal").unwrap();

        assert_eq!(status.value(), "Normal");
    }

    #[rstest]
    fn test_vip_status_value_vip() {
        let status = VipStatus::create("VipStatus", "VIP").unwrap();

        assert_eq!(status.value(), "VIP");
    }

    #[rstest]
    fn test_vip_status_copy() {
        let status = VipStatus::create("VipStatus", "Normal").unwrap();
        let copied = status;

        assert_eq!(status.value(), copied.value());
    }

    #[rstest]
    fn test_vip_status_clone() {
        let status = VipStatus::create("VipStatus", "VIP").unwrap();
        let cloned = status;

        assert_eq!(status, cloned);
    }

    #[rstest]
    fn test_vip_status_eq() {
        let normal1 = VipStatus::create("VipStatus", "Normal").unwrap();
        let normal2 = VipStatus::create("VipStatus", "normal").unwrap();
        let vip = VipStatus::create("VipStatus", "VIP").unwrap();

        assert_eq!(normal1, normal2);
        assert_ne!(normal1, vip);
    }

    // =========================================================================
    // PromotionCode Tests
    // =========================================================================

    #[rstest]
    fn test_promotion_code_new() {
        let promo = PromotionCode::new("SUMMER2024".to_string());

        assert_eq!(promo.value(), "SUMMER2024");
    }

    #[rstest]
    fn test_promotion_code_new_empty() {
        let promo = PromotionCode::new(String::new());

        assert_eq!(promo.value(), "");
    }

    #[rstest]
    fn test_promotion_code_new_special_chars() {
        let promo = PromotionCode::new("PROMO-50%OFF!".to_string());

        assert_eq!(promo.value(), "PROMO-50%OFF!");
    }

    #[rstest]
    fn test_promotion_code_value() {
        let promo = PromotionCode::new("TEST123".to_string());

        assert_eq!(promo.value(), "TEST123");
    }

    #[rstest]
    fn test_promotion_code_clone() {
        let original = PromotionCode::new("CLONE".to_string());
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_promotion_code_eq() {
        let promo1 = PromotionCode::new("CODE1".to_string());
        let promo2 = PromotionCode::new("CODE1".to_string());
        let promo3 = PromotionCode::new("CODE2".to_string());

        assert_eq!(promo1, promo2);
        assert_ne!(promo1, promo3);
    }

    // =========================================================================
    // PdfAttachment Tests
    // =========================================================================

    #[rstest]
    fn test_pdf_attachment_new() {
        let pdf = PdfAttachment::new("test.pdf".to_string(), vec![1, 2, 3, 4, 5]);

        assert_eq!(pdf.name(), "test.pdf");
        assert_eq!(pdf.bytes(), &[1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn test_pdf_attachment_new_empty() {
        let pdf = PdfAttachment::new(String::new(), Vec::new());

        assert_eq!(pdf.name(), "");
        assert_eq!(pdf.bytes(), &[] as &[u8]);
    }

    #[rstest]
    fn test_pdf_attachment_new_pdf_header() {
        // PDF ファイルの先頭バイト（%PDF-）
        let pdf_header = vec![0x25, 0x50, 0x44, 0x46, 0x2D];
        let pdf = PdfAttachment::new("invoice.pdf".to_string(), pdf_header.clone());

        assert_eq!(pdf.bytes(), &pdf_header);
    }

    #[rstest]
    fn test_pdf_attachment_name() {
        let pdf = PdfAttachment::new("document.pdf".to_string(), vec![]);

        assert_eq!(pdf.name(), "document.pdf");
    }

    #[rstest]
    fn test_pdf_attachment_bytes() {
        let data = vec![0u8; 100];
        let pdf = PdfAttachment::new("large.pdf".to_string(), data.clone());

        assert_eq!(pdf.bytes().len(), 100);
        assert_eq!(pdf.bytes(), &data);
    }

    #[rstest]
    fn test_pdf_attachment_clone() {
        let original = PdfAttachment::new("original.pdf".to_string(), vec![1, 2, 3]);
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.name(), cloned.name());
        assert_eq!(original.bytes(), cloned.bytes());
    }

    #[rstest]
    fn test_pdf_attachment_eq() {
        let pdf1 = PdfAttachment::new("same.pdf".to_string(), vec![1, 2, 3]);
        let pdf2 = PdfAttachment::new("same.pdf".to_string(), vec![1, 2, 3]);
        let pdf3 = PdfAttachment::new("different.pdf".to_string(), vec![1, 2, 3]);
        let pdf4 = PdfAttachment::new("same.pdf".to_string(), vec![4, 5, 6]);

        assert_eq!(pdf1, pdf2);
        assert_ne!(pdf1, pdf3);
        assert_ne!(pdf1, pdf4);
    }
}
