//! Acknowledgment email related types
//!
//! Defines types related to order acknowledgment emails.
//!
//! # Type List
//!
//! - [`HtmlString`] - HTML string
//! - [`OrderAcknowledgment`] - Order acknowledgment email content
//! - [`SendResult`] - Email send result

use crate::simple_types::EmailAddress;

// =============================================================================
// HtmlString
// =============================================================================

/// HTML string
///
/// A newtype to distinguish HTML from plain strings at the type level.
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
    /// Creates an `HtmlString` from an HTML string
    ///
    /// # Arguments
    ///
    /// * `html` - HTML string
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

    /// Returns a reference to the inner  HTML string
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

    /// Consumes and returns the internal `String`
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

/// Order acknowledgment email content
///
/// Holds the recipient address and the HTML body.
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
    /// Creates a new `OrderAcknowledgment`
    ///
    /// # Arguments
    ///
    /// * `email_address` - Recipient email address
    /// * `letter` - Email body (HTML format)
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

    /// Returns a reference to the email address
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

    /// Returns a reference to the email body
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

/// Email send result
///
/// Represents either a successful send or a failed send.
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
    /// Send succeeded
    Sent,

    /// Send failed (includes cases where a decision was made not to send)
    NotSent,
}

impl SendResult {
    /// Returns whether this is the `Sent` variant
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

    /// Returns whether this is the `NotSent` variant
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
