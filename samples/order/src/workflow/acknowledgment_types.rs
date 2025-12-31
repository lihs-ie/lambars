//! 確認メール関連型
//!
//! 注文確認メールに関連する型を定義する。
//!
//! # 型一覧
//!
//! - [`HtmlString`] - HTML 文字列
//! - [`OrderAcknowledgment`] - 注文確認メールの内容
//! - [`SendResult`] - メール送信結果

use crate::simple_types::EmailAddress;

// =============================================================================
// HtmlString
// =============================================================================

/// HTML 文字列
///
/// 通常の文字列と HTML を型レベルで区別するための newtype。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::HtmlString;
///
/// let html = HtmlString::new("<h1>Order Confirmation</h1>".to_string());
/// assert!(html.value().contains("<h1>"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlString(String);

impl HtmlString {
    /// HTML 文字列から `HtmlString` を生成する
    ///
    /// # Arguments
    ///
    /// * `html` - HTML 文字列
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::HtmlString;
    ///
    /// let html = HtmlString::new("<p>Thank you for your order!</p>".to_string());
    /// ```
    #[must_use]
    pub const fn new(html: String) -> Self {
        Self(html)
    }

    /// 内部の HTML 文字列への参照を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::HtmlString;
    ///
    /// let html = HtmlString::new("<strong>Important</strong>".to_string());
    /// assert_eq!(html.value(), "<strong>Important</strong>");
    /// ```
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }

    /// 内部の `String` を消費して返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::HtmlString;
    ///
    /// let html = HtmlString::new("<div>Content</div>".to_string());
    /// let inner = html.into_inner();
    /// assert_eq!(inner, "<div>Content</div>");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

// =============================================================================
// OrderAcknowledgment
// =============================================================================

/// 注文確認メールの内容
///
/// 送信先アドレスと HTML 本文を保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{OrderAcknowledgment, HtmlString};
/// use order_taking_sample::simple_types::EmailAddress;
///
/// let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
/// let letter = HtmlString::new("<h1>Order Confirmed</h1>".to_string());
/// let acknowledgment = OrderAcknowledgment::new(email, letter);
///
/// assert!(acknowledgment.letter().value().contains("Order Confirmed"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderAcknowledgment {
    email_address: EmailAddress,
    letter: HtmlString,
}

impl OrderAcknowledgment {
    /// 新しい `OrderAcknowledgment` を生成する
    ///
    /// # Arguments
    ///
    /// * `email_address` - 送信先メールアドレス
    /// * `letter` - メール本文（HTML 形式）
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{OrderAcknowledgment, HtmlString};
    /// use order_taking_sample::simple_types::EmailAddress;
    ///
    /// let email = EmailAddress::create("EmailAddress", "jane@example.com").unwrap();
    /// let letter = HtmlString::new("<p>Your order is confirmed</p>".to_string());
    /// let acknowledgment = OrderAcknowledgment::new(email, letter);
    /// ```
    #[must_use]
    pub const fn new(email_address: EmailAddress, letter: HtmlString) -> Self {
        Self {
            email_address,
            letter,
        }
    }

    /// メールアドレスへの参照を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{OrderAcknowledgment, HtmlString};
    /// use order_taking_sample::simple_types::EmailAddress;
    ///
    /// let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    /// let letter = HtmlString::new("<p>Hello</p>".to_string());
    /// let acknowledgment = OrderAcknowledgment::new(email, letter);
    /// assert_eq!(acknowledgment.email_address().value(), "test@example.com");
    /// ```
    #[must_use]
    pub const fn email_address(&self) -> &EmailAddress {
        &self.email_address
    }

    /// メール本文への参照を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{OrderAcknowledgment, HtmlString};
    /// use order_taking_sample::simple_types::EmailAddress;
    ///
    /// let email = EmailAddress::create("EmailAddress", "user@example.com").unwrap();
    /// let letter = HtmlString::new("<p>Details here</p>".to_string());
    /// let acknowledgment = OrderAcknowledgment::new(email, letter);
    /// assert!(acknowledgment.letter().value().contains("Details"));
    /// ```
    #[must_use]
    pub const fn letter(&self) -> &HtmlString {
        &self.letter
    }
}

// =============================================================================
// SendResult
// =============================================================================

/// メール送信結果
///
/// 送信成功または送信失敗のいずれかを表す。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::SendResult;
///
/// let sent = SendResult::Sent;
/// assert!(sent.is_sent());
/// assert!(!sent.is_not_sent());
///
/// let not_sent = SendResult::NotSent;
/// assert!(!not_sent.is_sent());
/// assert!(not_sent.is_not_sent());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendResult {
    /// 送信成功
    Sent,

    /// 送信失敗（エラーではなく、送信しない判断をした場合も含む）
    NotSent,
}

impl SendResult {
    /// `Sent` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::SendResult;
    ///
    /// let result = SendResult::Sent;
    /// assert!(result.is_sent());
    /// ```
    #[must_use]
    pub const fn is_sent(&self) -> bool {
        matches!(self, Self::Sent)
    }

    /// `NotSent` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::SendResult;
    ///
    /// let result = SendResult::NotSent;
    /// assert!(result.is_not_sent());
    /// ```
    #[must_use]
    pub const fn is_not_sent(&self) -> bool {
        matches!(self, Self::NotSent)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod html_string_tests {
        use super::*;

        #[test]
        fn test_new_and_value() {
            let html = HtmlString::new("<h1>Title</h1>".to_string());
            assert_eq!(html.value(), "<h1>Title</h1>");
        }

        #[test]
        fn test_into_inner() {
            let html = HtmlString::new("<p>Content</p>".to_string());
            let inner = html.into_inner();
            assert_eq!(inner, "<p>Content</p>");
        }

        #[test]
        fn test_clone() {
            let html1 = HtmlString::new("<div>Test</div>".to_string());
            let html2 = html1.clone();
            assert_eq!(html1, html2);
        }
    }

    mod order_acknowledgment_tests {
        use super::*;

        fn create_email() -> EmailAddress {
            EmailAddress::create("EmailAddress", "test@example.com").unwrap()
        }

        fn create_letter() -> HtmlString {
            HtmlString::new("<h1>Order Confirmation</h1>".to_string())
        }

        #[test]
        fn test_new_and_getters() {
            let email = create_email();
            let letter = create_letter();
            let acknowledgment = OrderAcknowledgment::new(email.clone(), letter.clone());

            assert_eq!(acknowledgment.email_address(), &email);
            assert_eq!(acknowledgment.letter(), &letter);
        }

        #[test]
        fn test_clone() {
            let acknowledgment1 = OrderAcknowledgment::new(create_email(), create_letter());
            let acknowledgment2 = acknowledgment1.clone();
            assert_eq!(acknowledgment1, acknowledgment2);
        }
    }

    mod send_result_tests {
        use super::*;

        #[test]
        fn test_sent() {
            let result = SendResult::Sent;
            assert!(result.is_sent());
            assert!(!result.is_not_sent());
        }

        #[test]
        fn test_not_sent() {
            let result = SendResult::NotSent;
            assert!(!result.is_sent());
            assert!(result.is_not_sent());
        }

        #[test]
        fn test_copy() {
            let result1 = SendResult::Sent;
            let result2 = result1; // Copy
            assert_eq!(result1, result2);
        }
    }
}
