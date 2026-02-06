//! Error DTOs
//!
//! Defines types for serializing API response errors.

use serde::{Deserialize, Serialize};

use crate::workflow::{PlaceOrderError, PricingError, RemoteServiceError, WorkflowValidationError};

// =============================================================================
// PlaceOrderErrorDto (REQ-086)
// =============================================================================

/// `PlaceOrder` workflow error DTO
///
/// A type for serializing errors that occurred in the workflow.
/// Adjacently tagged format discriminated by the `type` field.
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
    /// Validation error
    Validation {
        /// Field name
        field_name: String,
        /// Error message
        message: String,
    },
    /// Pricing error
    Pricing {
        /// Error message
        message: String,
    },
    /// Remote service error
    RemoteService {
        /// Service name
        service_name: String,
        /// Service endpoint
        service_endpoint: String,
        /// Error message
        message: String,
    },
}

impl PlaceOrderErrorDto {
    /// Creates a `PlaceOrderErrorDto` from the domain `PlaceOrderError`
    ///
    /// Converts to DTO as a pure function.
    ///
    /// # Arguments
    ///
    /// * `error` - Source `PlaceOrderError`
    ///
    /// # Returns
    ///
    /// A `PlaceOrderErrorDto` instance
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

    /// Creates a `PlaceOrderErrorDto` from a `WorkflowValidationError`
    #[must_use]
    fn from_validation_error(error: &WorkflowValidationError) -> Self {
        Self::Validation {
            field_name: error.field_name.clone(),
            message: error.message.clone(),
        }
    }

    /// Creates a `PlaceOrderErrorDto` from a `PricingError`
    #[must_use]
    fn from_pricing_error(error: &PricingError) -> Self {
        Self::Pricing {
            message: error.message().to_string(),
        }
    }

    /// Creates a `PlaceOrderErrorDto` from a `RemoteServiceError`
    #[must_use]
    fn from_remote_service_error(error: &RemoteServiceError) -> Self {
        Self::RemoteService {
            service_name: error.service().name().to_string(),
            service_endpoint: error.service().endpoint().to_string(),
            message: error.exception_message().to_string(),
        }
    }
}
