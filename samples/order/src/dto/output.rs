//! Output DTOs
//!
//! Defines DTO types used for serializing API responses.
//! Implemented in subsequent steps.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::dto::AddressDto;
use crate::workflow::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, PricedOrderLine,
    PricedOrderProductLine, ShippableOrderLine, ShippableOrderPlaced,
};

// =============================================================================
// ShippableOrderLineDto (REQ-079)
// =============================================================================

/// Shippable order line DTO
///
/// A type for serializing line item information within a shipping event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShippableOrderLineDto {
    /// Product code
    pub product_code: String,
    /// Quantity (string format)
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
}

impl ShippableOrderLineDto {
    /// Creates a `ShippableOrderLineDto` from the domain `ShippableOrderLine`
    #[must_use]
    pub fn from_domain(line: &ShippableOrderLine) -> Self {
        Self {
            product_code: line.product_code().value().to_string(),
            quantity: line.quantity().value(),
        }
    }
}

// =============================================================================
// ShippableOrderPlacedDto (REQ-080)
// =============================================================================

/// Shippable order placed event DTO
///
/// A type for serializing a shipping event.
/// PDF data is Base64 encoded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShippableOrderPlacedDto {
    /// Order ID
    pub order_id: String,
    /// Shipping address
    pub shipping_address: AddressDto,
    /// Shipment lines
    pub shipment_lines: Vec<ShippableOrderLineDto>,
    /// PDF file name
    pub pdf_name: String,
    /// PDF data (Base64 encoded)
    pub pdf_data: String,
}

impl ShippableOrderPlacedDto {
    /// Creates a `ShippableOrderPlacedDto` from the domain `ShippableOrderPlaced`
    #[must_use]
    pub fn from_domain(event: &ShippableOrderPlaced) -> Self {
        use base64::Engine;
        let pdf_data = base64::engine::general_purpose::STANDARD.encode(event.pdf().bytes());

        Self {
            order_id: event.order_id().value().to_string(),
            shipping_address: AddressDto::from_address(event.shipping_address()),
            shipment_lines: event
                .shipment_lines()
                .iter()
                .map(ShippableOrderLineDto::from_domain)
                .collect(),
            pdf_name: event.pdf().name().to_string(),
            pdf_data,
        }
    }
}

// =============================================================================
// BillableOrderPlacedDto (REQ-081)
// =============================================================================

/// Billable order placed event DTO
///
/// A type for serializing a billing event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillableOrderPlacedDto {
    /// Order ID
    pub order_id: String,
    /// Billing address
    pub billing_address: AddressDto,
    /// Billing amount (string format)
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_to_bill: Decimal,
}

impl BillableOrderPlacedDto {
    /// Creates a `BillableOrderPlacedDto` from the domain `BillableOrderPlaced`
    #[must_use]
    pub fn from_domain(event: &BillableOrderPlaced) -> Self {
        Self {
            order_id: event.order_id().value().to_string(),
            billing_address: AddressDto::from_address(event.billing_address()),
            amount_to_bill: event.amount_to_bill().value(),
        }
    }
}

// =============================================================================
// OrderAcknowledgmentSentDto (REQ-082)
// =============================================================================

/// Order acknowledgment sent event DTO
///
/// A type for serializing an acknowledgment sent event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAcknowledgmentSentDto {
    /// Order ID
    pub order_id: String,
    /// Recipient email address
    pub email_address: String,
}

impl OrderAcknowledgmentSentDto {
    /// Creates an `OrderAcknowledgmentSentDto` from the domain `OrderAcknowledgmentSent`
    #[must_use]
    pub fn from_domain(event: &OrderAcknowledgmentSent) -> Self {
        Self {
            order_id: event.order_id().value().to_string(),
            email_address: event.email_address().value().to_string(),
        }
    }
}

// =============================================================================
// PricedOrderProductLineDto (REQ-083)
// =============================================================================

/// Priced product order line DTO
///
/// A type for serializing a priced product line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricedOrderProductLineDto {
    /// Order line ID
    pub order_line_id: String,
    /// Product code
    pub product_code: String,
    /// Quantity (string format)
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Line price (string format)
    #[serde(with = "rust_decimal::serde::str")]
    pub line_price: Decimal,
}

impl PricedOrderProductLineDto {
    /// Creates a `PricedOrderProductLineDto` from the domain `PricedOrderProductLine`
    #[must_use]
    pub fn from_domain(line: &PricedOrderProductLine) -> Self {
        Self {
            order_line_id: line.order_line_id().value().to_string(),
            product_code: line.product_code().value().to_string(),
            quantity: line.quantity().value(),
            line_price: line.line_price().value(),
        }
    }
}

// =============================================================================
// PricedOrderLineDto (REQ-084)
// =============================================================================

/// Priced order line DTO
///
/// A type for serializing a priced line (product or comment).
/// Adjacently tagged format discriminated by the `type` field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PricedOrderLineDto {
    /// Product line
    ProductLine(PricedOrderProductLineDto),
    /// Comment line
    CommentLine(String),
}

impl PricedOrderLineDto {
    /// Creates a `PricedOrderLineDto` from the domain `PricedOrderLine`
    #[must_use]
    pub fn from_domain(line: &PricedOrderLine) -> Self {
        match line {
            PricedOrderLine::ProductLine(product_line) => {
                Self::ProductLine(PricedOrderProductLineDto::from_domain(product_line))
            }
            PricedOrderLine::CommentLine(comment) => Self::CommentLine(comment.clone()),
        }
    }
}

// =============================================================================
// PlaceOrderEventDto (REQ-085)
// =============================================================================

/// `PlaceOrder` workflow output event DTO
///
/// A type for serializing events upon workflow completion.
/// Adjacently tagged format discriminated by the `type` field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PlaceOrderEventDto {
    /// Shippable order placed event
    ShippableOrderPlaced(ShippableOrderPlacedDto),
    /// Billable order placed event
    BillableOrderPlaced(BillableOrderPlacedDto),
    /// acknowledgment sent event
    AcknowledgmentSent(OrderAcknowledgmentSentDto),
}

impl PlaceOrderEventDto {
    /// Creates a `PlaceOrderEventDto` from the domain `PlaceOrderEvent`
    #[must_use]
    pub fn from_domain(event: &PlaceOrderEvent) -> Self {
        match event {
            PlaceOrderEvent::ShippableOrderPlaced(e) => {
                Self::ShippableOrderPlaced(ShippableOrderPlacedDto::from_domain(e))
            }
            PlaceOrderEvent::BillableOrderPlaced(e) => {
                Self::BillableOrderPlaced(BillableOrderPlacedDto::from_domain(e))
            }
            PlaceOrderEvent::AcknowledgmentSent(e) => {
                Self::AcknowledgmentSent(OrderAcknowledgmentSentDto::from_domain(e))
            }
        }
    }

    /// Creates a DTO list from a list of domain events
    #[must_use]
    pub fn from_domain_list(events: &[PlaceOrderEvent]) -> Vec<Self> {
        events.iter().map(Self::from_domain).collect()
    }
}
