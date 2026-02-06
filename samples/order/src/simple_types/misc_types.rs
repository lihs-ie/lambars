//! Miscellaneous type definitions
//!
//! Defines `VipStatus`, `PromotionCode`, and `PdfAttachment`.

use super::error::ValidationError;

// =============================================================================
// VipStatus
// =============================================================================

/// Enum representing a customer's VIP status
///
/// Either Normal (regular) or Vip (VIP customer).
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
/// // Invalid values cause an error
/// assert!(VipStatus::create("VipStatus", "Premium").is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VipStatus {
    /// Regular customer
    Normal,
    /// VIP customer
    Vip,
}

impl VipStatus {
    /// Creates a `VipStatus` from a string
    ///
    /// Accepts "normal", "Normal", "vip", "VIP".
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(VipStatus)` - On successful validation
    /// * `Err(ValidationError)` - If the value is invalid
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` for values other than "normal", "Normal", "vip", "VIP".
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

    /// Returns the `VipStatus` as a string
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

/// Type representing a promotion code
///
/// A simple wrapper type with no validation.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::PromotionCode;
///
/// let promo = PromotionCode::new("SUMMER2024".to_string());
/// assert_eq!(promo.value(), "SUMMER2024");
///
/// // Empty strings are allowed (no validation)
/// let empty = PromotionCode::new(String::new());
/// assert_eq!(empty.value(), "");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PromotionCode(String);

impl PromotionCode {
    /// Creates a `PromotionCode` from a string
    ///
    /// Creates directly without validation.
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns a reference to the inner Promotion codestring
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// PdfAttachment
// =============================================================================

/// Struct representing a PDF attachment
///
/// Holds a file name and byte data.
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
    /// File name
    name: String,
    /// PDF byte data
    bytes: Vec<u8>,
}

impl PdfAttachment {
    /// Creates a `PdfAttachment` from a file name and byte data
    #[must_use]
    pub const fn new(name: String, bytes: Vec<u8>) -> Self {
        Self { name, bytes }
    }

    /// Returns a reference to the file name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the byte data
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
        // "Vip" and "NORMAL" are errors (strict matching)
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
        // Leading bytes of a PDF file (%PDF-)
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
