//! Validation error type definition

use thiserror::Error;

/// Struct representing a validation error
///
/// Used commonly by all constrained types.
/// Holds a field name and error message.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ValidationError;
///
/// let error = ValidationError::new("OrderId", "Must not be empty");
/// assert_eq!(error.field_name, "OrderId");
/// assert_eq!(error.message, "Must not be empty");
/// assert_eq!(error.to_string(), "OrderId: Must not be empty");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{field_name}: {message}")]
pub struct ValidationError {
    /// Name of the field where the error occurred
    pub field_name: String,
    /// Error message
    pub message: String,
}

impl ValidationError {
    /// Creates a new `ValidationError`
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the field where the error occurred
    /// * `message` - error message
    ///
    /// # Returns
    ///
    /// A new `ValidationError` instance
    #[must_use]
    pub fn new(field_name: &str, message: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_validation_error_new() {
        let error = ValidationError::new("OrderId", "Must not be empty");

        assert_eq!(error.field_name, "OrderId");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_validation_error_display() {
        let error = ValidationError::new("OrderId", "Must not be empty");

        assert_eq!(error.to_string(), "OrderId: Must not be empty");
    }

    #[rstest]
    fn test_validation_error_error_trait() {
        let error = ValidationError::new("OrderId", "Must not be empty");

        // Verify that std::error::Error is implemented
        let _: &dyn std::error::Error = &error;
    }

    #[rstest]
    fn test_validation_error_clone() {
        let error = ValidationError::new("Price", "Must be positive");
        let cloned = error.clone();

        assert_eq!(error, cloned);
    }

    #[rstest]
    fn test_validation_error_eq() {
        let error1 = ValidationError::new("OrderId", "Must not be empty");
        let error2 = ValidationError::new("OrderId", "Must not be empty");
        let error3 = ValidationError::new("OrderId", "Different message");

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }
}
