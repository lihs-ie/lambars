//! DTO (Data Transfer Object) module
//!
//! Defines serializable/deserializable types used at the API boundary.
//! Also provides conversion functions to and from domain types.
//!
//! # Module Structure
//!
//! - [`input`] - Input DTOs (`OrderFormDto`, etc.)
//! - [`output`] - Output DTOs (`PlaceOrderEventDto`, etc.)
//! - [`error`] - Error DTOs (`PlaceOrderErrorDto`)
//!
//! # Design Principles
//!
//! - All DTOs implement `Serialize` and `Deserialize`
//! - Conversions to domain types are implemented as pure functions
//! - Decimals are serialized as strings (to preserve precision)

pub mod error;
pub mod input;
pub mod output;

// Re-exports
pub use error::PlaceOrderErrorDto;
pub use input::{AddressDto, CustomerInfoDto, OrderFormDto, OrderFormLineDto};
pub use output::{
    BillableOrderPlacedDto, OrderAcknowledgmentSentDto, PlaceOrderEventDto, PricedOrderLineDto,
    PricedOrderProductLineDto, ShippableOrderLineDto, ShippableOrderPlacedDto,
};
