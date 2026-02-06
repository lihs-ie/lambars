//! Input DTOs
//!
//! Defines DTO types used for deserializing API requests.
//!
//! # Type List
//!
//! - [`CustomerInfoDto`] - Customer information DTO
//! - [`AddressDto`] - Address DTO
//! - [`OrderFormLineDto`] - Order line DTO
//! - [`OrderFormDto`] - Order form DTO

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::compound_types::Address;
use crate::workflow::{
    UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};

// =============================================================================
// CustomerInfoDto (REQ-075)
// =============================================================================

/// customer information DTO
///
/// A type for deserializing customer information received from the API.
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::CustomerInfoDto;
///
/// let json = r#"{
///     "first_name": "John",
///     "last_name": "Doe",
///     "email_address": "john@example.com",
///     "vip_status": "Normal"
/// }"#;
///
/// let dto: CustomerInfoDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.first_name, "John");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerInfoDto {
    /// First name
    pub first_name: String,
    /// Last name
    pub last_name: String,
    /// Email address
    pub email_address: String,
    /// VIP status ("Normal" or "VIP")
    pub vip_status: String,
}

impl CustomerInfoDto {
    /// Converts to `UnvalidatedCustomerInfo`
    ///
    /// Converts to the domain type as a pure function. No validation is performed.
    ///
    /// # Returns
    ///
    /// A `UnvalidatedCustomerInfo` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::CustomerInfoDto;
    ///
    /// let dto = CustomerInfoDto {
    ///     first_name: "John".to_string(),
    ///     last_name: "Doe".to_string(),
    ///     email_address: "john@example.com".to_string(),
    ///     vip_status: "Normal".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_customer_info();
    /// assert_eq!(unvalidated.first_name(), "John");
    /// ```
    #[must_use]
    pub fn to_unvalidated_customer_info(&self) -> UnvalidatedCustomerInfo {
        UnvalidatedCustomerInfo::new(
            self.first_name.clone(),
            self.last_name.clone(),
            self.email_address.clone(),
            self.vip_status.clone(),
        )
    }
}

// =============================================================================
// AddressDto (REQ-076)
// =============================================================================

/// address DTO
///
/// A type for deserializing addresses received from the API.
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::AddressDto;
///
/// let json = r#"{
///     "address_line1": "123 Main St",
///     "address_line2": "Apt 4B",
///     "address_line3": "",
///     "address_line4": "",
///     "city": "New York",
///     "zip_code": "10001",
///     "state": "NY",
///     "country": "USA"
/// }"#;
///
/// let dto: AddressDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.city, "New York");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressDto {
    /// Address line 1 (required)
    pub address_line1: String,
    /// Address line 2 (optional, `None` for empty string)
    pub address_line2: String,
    /// Address line 3 (optional, `None` for empty string)
    pub address_line3: String,
    /// Address line 4 (optional, `None` for empty string)
    pub address_line4: String,
    /// City
    pub city: String,
    /// ZIP code
    pub zip_code: String,
    /// State code
    pub state: String,
    /// Country name
    pub country: String,
}

impl AddressDto {
    /// Converts to `UnvalidatedAddress`
    ///
    /// Converts to the domain type as a pure function. No validation is performed.
    ///
    /// # Returns
    ///
    /// A `UnvalidatedAddress` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::AddressDto;
    ///
    /// let dto = AddressDto {
    ///     address_line1: "123 Main St".to_string(),
    ///     address_line2: "".to_string(),
    ///     address_line3: "".to_string(),
    ///     address_line4: "".to_string(),
    ///     city: "New York".to_string(),
    ///     zip_code: "10001".to_string(),
    ///     state: "NY".to_string(),
    ///     country: "USA".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_address();
    /// assert_eq!(unvalidated.city(), "New York");
    /// ```
    #[must_use]
    pub fn to_unvalidated_address(&self) -> UnvalidatedAddress {
        UnvalidatedAddress::new(
            self.address_line1.clone(),
            self.address_line2.clone(),
            self.address_line3.clone(),
            self.address_line4.clone(),
            self.city.clone(),
            self.zip_code.clone(),
            self.state.clone(),
            self.country.clone(),
        )
    }

    /// Creates an `AddressDto` from the domain `Address`
    ///
    /// Converts to DTO as a pure function.
    ///
    /// # Arguments
    ///
    /// * `address` - Source `Address`
    ///
    /// # Returns
    ///
    /// A `AddressDto` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::Address;
    /// use order_taking_sample::dto::AddressDto;
    ///
    /// let address = Address::create(
    ///     "123 Main St", "", "", "", "New York", "10001", "NY", "USA"
    /// ).unwrap();
    ///
    /// let dto = AddressDto::from_address(&address);
    /// assert_eq!(dto.city, "New York");
    /// ```
    #[must_use]
    pub fn from_address(address: &Address) -> Self {
        Self {
            address_line1: address.address_line1().value().to_string(),
            address_line2: address
                .address_line2()
                .map_or_else(String::new, |s| s.value().to_string()),
            address_line3: address
                .address_line3()
                .map_or_else(String::new, |s| s.value().to_string()),
            address_line4: address
                .address_line4()
                .map_or_else(String::new, |s| s.value().to_string()),
            city: address.city().value().to_string(),
            zip_code: address.zip_code().value().to_string(),
            state: address.state().value().to_string(),
            country: address.country().value().to_string(),
        }
    }
}

// =============================================================================
// OrderFormLineDto (REQ-077)
// =============================================================================

/// order line DTO
///
/// A type for deserializing order lines received from the API.
/// Quantity is serialized as a string to preserve precision.
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::OrderFormLineDto;
/// use rust_decimal::Decimal;
///
/// let json = r#"{
///     "order_line_id": "line-001",
///     "product_code": "W1234",
///     "quantity": "10"
/// }"#;
///
/// let dto: OrderFormLineDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.product_code, "W1234");
/// assert_eq!(dto.quantity, Decimal::from(10));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFormLineDto {
    /// Order line ID
    pub order_line_id: String,
    /// Product code
    pub product_code: String,
    /// Quantity (Decimal in string format)
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
}

impl OrderFormLineDto {
    /// Converts to `UnvalidatedOrderLine`
    ///
    /// Converts to the domain type as a pure function. No validation is performed.
    ///
    /// # Returns
    ///
    /// A `UnvalidatedOrderLine` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::OrderFormLineDto;
    /// use rust_decimal::Decimal;
    ///
    /// let dto = OrderFormLineDto {
    ///     order_line_id: "line-001".to_string(),
    ///     product_code: "W1234".to_string(),
    ///     quantity: Decimal::from(10),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_order_line();
    /// assert_eq!(unvalidated.product_code(), "W1234");
    /// ```
    #[must_use]
    pub fn to_unvalidated_order_line(&self) -> UnvalidatedOrderLine {
        UnvalidatedOrderLine::new(
            self.order_line_id.clone(),
            self.product_code.clone(),
            self.quantity,
        )
    }
}

// =============================================================================
// OrderFormDto (REQ-078)
// =============================================================================

/// order form DTO
///
/// A type for deserializing an entire order received from the API.
/// Used as input to the `PlaceOrder` workflow.
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::{OrderFormDto, CustomerInfoDto, AddressDto, OrderFormLineDto};
/// use rust_decimal::Decimal;
///
/// let dto = OrderFormDto {
///     order_id: "order-001".to_string(),
///     customer_info: CustomerInfoDto {
///         first_name: "John".to_string(),
///         last_name: "Doe".to_string(),
///         email_address: "john@example.com".to_string(),
///         vip_status: "Normal".to_string(),
///     },
///     shipping_address: AddressDto {
///         address_line1: "123 Main St".to_string(),
///         address_line2: "".to_string(),
///         address_line3: "".to_string(),
///         address_line4: "".to_string(),
///         city: "New York".to_string(),
///         zip_code: "10001".to_string(),
///         state: "NY".to_string(),
///         country: "USA".to_string(),
///     },
///     billing_address: AddressDto {
///         address_line1: "123 Main St".to_string(),
///         address_line2: "".to_string(),
///         address_line3: "".to_string(),
///         address_line4: "".to_string(),
///         city: "New York".to_string(),
///         zip_code: "10001".to_string(),
///         state: "NY".to_string(),
///         country: "USA".to_string(),
///     },
///     lines: vec![OrderFormLineDto {
///         order_line_id: "line-001".to_string(),
///         product_code: "W1234".to_string(),
///         quantity: Decimal::from(10),
///     }],
///     promotion_code: "".to_string(),
/// };
///
/// let unvalidated = dto.to_unvalidated_order();
/// assert_eq!(unvalidated.order_id(), "order-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFormDto {
    /// Order ID
    pub order_id: String,
    /// Customer information
    pub customer_info: CustomerInfoDto,
    /// Shipping address
    pub shipping_address: AddressDto,
    /// Billing address
    pub billing_address: AddressDto,
    /// Order lines
    pub lines: Vec<OrderFormLineDto>,
    /// Promotion code (may be empty string)
    pub promotion_code: String,
}

impl OrderFormDto {
    /// Converts to `UnvalidatedOrder`
    ///
    /// Converts to the domain type as a pure function. No validation is performed.
    ///
    /// # Returns
    ///
    /// A `UnvalidatedOrder` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::{OrderFormDto, CustomerInfoDto, AddressDto, OrderFormLineDto};
    /// use rust_decimal::Decimal;
    ///
    /// let dto = OrderFormDto {
    ///     order_id: "order-001".to_string(),
    ///     customer_info: CustomerInfoDto {
    ///         first_name: "John".to_string(),
    ///         last_name: "Doe".to_string(),
    ///         email_address: "john@example.com".to_string(),
    ///         vip_status: "Normal".to_string(),
    ///     },
    ///     shipping_address: AddressDto {
    ///         address_line1: "123 Main St".to_string(),
    ///         address_line2: "".to_string(),
    ///         address_line3: "".to_string(),
    ///         address_line4: "".to_string(),
    ///         city: "New York".to_string(),
    ///         zip_code: "10001".to_string(),
    ///         state: "NY".to_string(),
    ///         country: "USA".to_string(),
    ///     },
    ///     billing_address: AddressDto {
    ///         address_line1: "123 Main St".to_string(),
    ///         address_line2: "".to_string(),
    ///         address_line3: "".to_string(),
    ///         address_line4: "".to_string(),
    ///         city: "New York".to_string(),
    ///         zip_code: "10001".to_string(),
    ///         state: "NY".to_string(),
    ///         country: "USA".to_string(),
    ///     },
    ///     lines: vec![],
    ///     promotion_code: "".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_order();
    /// assert_eq!(unvalidated.order_id(), "order-001");
    /// ```
    #[must_use]
    pub fn to_unvalidated_order(&self) -> UnvalidatedOrder {
        let customer_info = self.customer_info.to_unvalidated_customer_info();
        let shipping_address = self.shipping_address.to_unvalidated_address();
        let billing_address = self.billing_address.to_unvalidated_address();
        let lines: Vec<UnvalidatedOrderLine> = self
            .lines
            .iter()
            .map(OrderFormLineDto::to_unvalidated_order_line)
            .collect();

        UnvalidatedOrder::new(
            self.order_id.clone(),
            customer_info,
            shipping_address,
            billing_address,
            lines,
            self.promotion_code.clone(),
        )
    }
}
