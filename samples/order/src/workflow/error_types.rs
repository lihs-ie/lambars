//! workflowError types
//!
//! Expresses errors that occur in the `PlaceOrder` workflow in a type-safe manner.
//!
//! # Type List
//!
//! - [`WorkflowValidationError`] - Input validation error (type alias for `ValidationError`)
//! - [`PricingError`] - Pricing error
//! - [`ServiceInfo`] - Remote service information
//! - [`RemoteServiceError`] - Remote service call error
//! - [`PlaceOrderError`] - Workflow-level error

use thiserror::Error;

// =============================================================================
// WorkflowValidationError
// =============================================================================

/// Workflow-specific validation error
///
/// Defined as a type alias for Phase 1's [`crate::simple_types::ValidationError`].
/// Avoids name collisions within the workflow module and allows clear distinction.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::WorkflowValidationError;
///
/// let error = WorkflowValidationError::new("OrderId", "cannot be empty");
/// assert!(error.to_string().contains("cannot be empty"));
/// ```
pub type WorkflowValidationError = crate::simple_types::ValidationError;

// =============================================================================
// PricingError
// =============================================================================

/// Price calculation error
///
/// Represents errors such as product price not found, invalid promotion code,
/// or price calculation result out of range.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::PricingError;
///
/// let error = PricingError::new("Product not found: W9999");
/// assert_eq!(error.message(), "Product not found: W9999");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("Pricing error: {message}")]
pub struct PricingError {
    message: String,
}

impl PricingError {
    /// Creates a new `PricingError`
    ///
    /// # Arguments
    ///
    /// * `message` - error message
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingError;
    ///
    /// let error = PricingError::new("Invalid promotion code");
    /// assert_eq!(error.message(), "Invalid promotion code");
    /// ```
    #[must_use]
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    /// Returns a reference to error message
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingError;
    ///
    /// let error = PricingError::new("Price out of range");
    /// assert_eq!(error.message(), "Price out of range");
    /// ```
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

// =============================================================================
// ServiceInfo
// =============================================================================

/// External service information
///
/// Used to identify the service where an error occurred within [`RemoteServiceError`].
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ServiceInfo;
///
/// let service = ServiceInfo::new(
///     "AddressValidation".to_string(),
///     "https://api.example.com/validate".to_string(),
/// );
/// assert_eq!(service.name(), "AddressValidation");
/// assert_eq!(service.endpoint(), "https://api.example.com/validate");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceInfo {
    name: String,
    endpoint: String,
}

impl ServiceInfo {
    /// Creates a new `ServiceInfo`
    ///
    /// # Arguments
    ///
    /// * `name` - servicename
    /// * `endpoint` - The service endpoint URL
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ServiceInfo;
    ///
    /// let service = ServiceInfo::new(
    ///     "PricingService".to_string(),
    ///     "https://api.example.com/pricing".to_string(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(name: String, endpoint: String) -> Self {
        Self { name, endpoint }
    }

    /// Returns a reference to servicename
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ServiceInfo;
    ///
    /// let service = ServiceInfo::new("Auth".to_string(), "https://auth.example.com".to_string());
    /// assert_eq!(service.name(), "Auth");
    /// ```
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the endpoint
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ServiceInfo;
    ///
    /// let service = ServiceInfo::new("Auth".to_string(), "https://auth.example.com".to_string());
    /// assert_eq!(service.endpoint(), "https://auth.example.com");
    /// ```
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

// =============================================================================
// RemoteServiceError
// =============================================================================

/// Error during external service call
///
/// Holds information about which service encountered what error.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{RemoteServiceError, ServiceInfo};
///
/// let service = ServiceInfo::new(
///     "AddressValidation".to_string(),
///     "https://api.example.com/validate".to_string(),
/// );
/// let error = RemoteServiceError::new(service, "Connection timeout".to_string());
/// assert_eq!(error.exception_message(), "Connection timeout");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("Remote service error: {service:?} - {exception_message}")]
pub struct RemoteServiceError {
    service: ServiceInfo,
    exception_message: String,
}

impl RemoteServiceError {
    /// Creates a new `RemoteServiceError`
    ///
    /// # Arguments
    ///
    /// * `service` - Information about the service where the error occurred
    /// * `exception_message` - error message
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{RemoteServiceError, ServiceInfo};
    ///
    /// let service = ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
    /// let error = RemoteServiceError::new(service, "500 Internal Server Error".to_string());
    /// ```
    #[must_use]
    pub const fn new(service: ServiceInfo, exception_message: String) -> Self {
        Self {
            service,
            exception_message,
        }
    }

    /// Returns a reference to serviceinformation
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{RemoteServiceError, ServiceInfo};
    ///
    /// let service = ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
    /// let error = RemoteServiceError::new(service.clone(), "Error".to_string());
    /// assert_eq!(error.service().name(), "API");
    /// ```
    #[must_use]
    pub const fn service(&self) -> &ServiceInfo {
        &self.service
    }

    /// Returns a reference to error message
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{RemoteServiceError, ServiceInfo};
    ///
    /// let service = ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
    /// let error = RemoteServiceError::new(service, "Network error".to_string());
    /// assert_eq!(error.exception_message(), "Network error");
    /// ```
    #[must_use]
    pub fn exception_message(&self) -> &str {
        &self.exception_message
    }
}

// =============================================================================
// PlaceOrderError
// =============================================================================

/// Error for the entire `PlaceOrder` workflow
///
/// A sum type holding either a Validation error, Pricing error, or Remote service error.
/// The `From` trait implementations allow automatic conversion from each error type using the `?` operator.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PlaceOrderError, PricingError};
///
/// let pricing_error = PricingError::new("Product not found");
/// let error: PlaceOrderError = pricing_error.into();
/// assert!(error.is_pricing());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PlaceOrderError {
    /// Input value validation error
    #[error("Validation error: {0}")]
    Validation(WorkflowValidationError),

    /// Price calculation error
    #[error("Pricing error: {0}")]
    Pricing(PricingError),

    /// External service call error
    #[error("Remote service error: {0}")]
    RemoteService(RemoteServiceError),
}

impl PlaceOrderError {
    /// Creates a `PlaceOrderError` from a validation error
    ///
    /// # Arguments
    ///
    /// * `error` - Validation error
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::simple_types::ValidationError;
    /// use order_taking_sample::workflow::PlaceOrderError;
    ///
    /// let validation_error = ValidationError::new("OrderId", "cannot be empty");
    /// let error = PlaceOrderError::validation(validation_error);
    /// assert!(error.is_validation());
    /// ```
    #[must_use]
    pub const fn validation(error: WorkflowValidationError) -> Self {
        Self::Validation(error)
    }

    /// Creates a `PlaceOrderError` from a pricing error
    ///
    /// # Arguments
    ///
    /// * `error` - Pricing error
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderError, PricingError};
    ///
    /// let pricing_error = PricingError::new("Product not found");
    /// let error = PlaceOrderError::pricing(pricing_error);
    /// assert!(error.is_pricing());
    /// ```
    #[must_use]
    pub const fn pricing(error: PricingError) -> Self {
        Self::Pricing(error)
    }

    /// Creates a `PlaceOrderError` from a remote service error
    ///
    /// # Arguments
    ///
    /// * `error` - Remote service error
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderError, RemoteServiceError, ServiceInfo};
    ///
    /// let service = ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
    /// let remote_error = RemoteServiceError::new(service, "Error".to_string());
    /// let error = PlaceOrderError::remote_service(remote_error);
    /// assert!(error.is_remote_service());
    /// ```
    #[must_use]
    pub const fn remote_service(error: RemoteServiceError) -> Self {
        Self::RemoteService(error)
    }

    /// Returns whether this is the `Validation` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::simple_types::ValidationError;
    /// use order_taking_sample::workflow::PlaceOrderError;
    ///
    /// let error = PlaceOrderError::validation(ValidationError::new("field", "error"));
    /// assert!(error.is_validation());
    /// ```
    #[must_use]
    pub const fn is_validation(&self) -> bool {
        matches!(self, Self::Validation(_))
    }

    /// Returns whether this is the `Pricing` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderError, PricingError};
    ///
    /// let error = PlaceOrderError::pricing(PricingError::new("error"));
    /// assert!(error.is_pricing());
    /// ```
    #[must_use]
    pub const fn is_pricing(&self) -> bool {
        matches!(self, Self::Pricing(_))
    }

    /// Returns whether this is the `RemoteService` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderError, RemoteServiceError, ServiceInfo};
    ///
    /// let service = ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
    /// let error = PlaceOrderError::remote_service(RemoteServiceError::new(service, "error".to_string()));
    /// assert!(error.is_remote_service());
    /// ```
    #[must_use]
    pub const fn is_remote_service(&self) -> bool {
        matches!(self, Self::RemoteService(_))
    }
}

impl From<WorkflowValidationError> for PlaceOrderError {
    fn from(error: WorkflowValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<PricingError> for PlaceOrderError {
    fn from(error: PricingError) -> Self {
        Self::Pricing(error)
    }
}

impl From<RemoteServiceError> for PlaceOrderError {
    fn from(error: RemoteServiceError) -> Self {
        Self::RemoteService(error)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod pricing_error_tests {
        use super::*;

        #[test]
        fn test_new_and_message() {
            let error = PricingError::new("Product not found: W9999");
            assert_eq!(error.message(), "Product not found: W9999");
        }

        #[test]
        fn test_display() {
            let error = PricingError::new("Invalid promotion code");
            let display = error.to_string();
            assert!(display.contains("Invalid promotion code"));
        }

        #[test]
        fn test_clone() {
            let error1 = PricingError::new("error");
            let error2 = error1.clone();
            assert_eq!(error1, error2);
        }
    }

    mod service_info_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let service = ServiceInfo::new(
                "AddressValidation".to_string(),
                "https://api.example.com/validate".to_string(),
            );
            assert_eq!(service.name(), "AddressValidation");
            assert_eq!(service.endpoint(), "https://api.example.com/validate");
        }

        #[test]
        fn test_clone() {
            let service1 =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let service2 = service1.clone();
            assert_eq!(service1, service2);
        }
    }

    mod remote_service_error_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let service = ServiceInfo::new(
                "AddressValidation".to_string(),
                "https://api.example.com/validate".to_string(),
            );
            let error = RemoteServiceError::new(service.clone(), "Connection timeout".to_string());
            assert_eq!(error.service().name(), "AddressValidation");
            assert_eq!(error.exception_message(), "Connection timeout");
        }

        #[test]
        fn test_display() {
            let service =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let error = RemoteServiceError::new(service, "Network error".to_string());
            let display = error.to_string();
            assert!(display.contains("Network error"));
        }

        #[test]
        fn test_clone() {
            let service =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let error1 = RemoteServiceError::new(service, "Error".to_string());
            let error2 = error1.clone();
            assert_eq!(error1, error2);
        }
    }

    mod place_order_error_tests {
        use super::*;
        use crate::simple_types::ValidationError;

        #[test]
        fn test_validation_variant() {
            let validation_error = ValidationError::new("OrderId", "cannot be empty");
            let error = PlaceOrderError::validation(validation_error);
            assert!(error.is_validation());
            assert!(!error.is_pricing());
            assert!(!error.is_remote_service());
        }

        #[test]
        fn test_pricing_variant() {
            let pricing_error = PricingError::new("Product not found");
            let error = PlaceOrderError::pricing(pricing_error);
            assert!(!error.is_validation());
            assert!(error.is_pricing());
            assert!(!error.is_remote_service());
        }

        #[test]
        fn test_remote_service_variant() {
            let service =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let remote_error = RemoteServiceError::new(service, "Error".to_string());
            let error = PlaceOrderError::remote_service(remote_error);
            assert!(!error.is_validation());
            assert!(!error.is_pricing());
            assert!(error.is_remote_service());
        }

        #[test]
        fn test_from_validation_error() {
            let validation_error = ValidationError::new("field", "error");
            let error: PlaceOrderError = validation_error.into();
            assert!(error.is_validation());
        }

        #[test]
        fn test_from_pricing_error() {
            let pricing_error = PricingError::new("error");
            let error: PlaceOrderError = pricing_error.into();
            assert!(error.is_pricing());
        }

        #[test]
        fn test_from_remote_service_error() {
            let service =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let remote_error = RemoteServiceError::new(service, "error".to_string());
            let error: PlaceOrderError = remote_error.into();
            assert!(error.is_remote_service());
        }

        #[test]
        fn test_display_validation() {
            let validation_error = ValidationError::new("OrderId", "cannot be empty");
            let error = PlaceOrderError::validation(validation_error);
            let display = error.to_string();
            assert!(display.contains("Validation error"));
        }

        #[test]
        fn test_display_pricing() {
            let pricing_error = PricingError::new("Product not found");
            let error = PlaceOrderError::pricing(pricing_error);
            let display = error.to_string();
            assert!(display.contains("Pricing error"));
        }

        #[test]
        fn test_display_remote_service() {
            let service =
                ServiceInfo::new("API".to_string(), "https://api.example.com".to_string());
            let remote_error = RemoteServiceError::new(service, "Network error".to_string());
            let error = PlaceOrderError::remote_service(remote_error);
            let display = error.to_string();
            assert!(display.contains("Remote service error"));
        }

        #[test]
        fn test_clone() {
            let pricing_error = PricingError::new("error");
            let error1 = PlaceOrderError::pricing(pricing_error);
            let error2 = error1.clone();
            assert_eq!(error1, error2);
        }
    }
}
