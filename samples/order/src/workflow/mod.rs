//! Workflow type definition module
//!
//! Defines types used in the `PlaceOrder` workflow.
//! Expresses state transitions via types, preventing invalid states at the type level.
//!
//! # State Transition Diagram
//!
//! ```text
//! UnvalidatedOrder -> ValidatedOrder -> PricedOrder -> PricedOrderWithShippingMethod -> PlaceOrderEvent[]
//! ```
//!
//! # Choosing Between Function Composition Macros
//!
//! lambars provides two function composition macros.
//!
//! ## pipe! macro
//!
//! Used for immediate value transformation (data flow style).
//!
//! ```ignore
//! // Values flow from left to right
//! let result = pipe!(value, f, g, h); // h(g(f(value)))
//! ```
//!
//! **Use cases**:
//! - One-time transformation chain
//! - When you want to omit intermediate variables
//! - When you want to express data flow from left to right
//!
//! ## compose! macro
//!
//! Used to generate reusable composed functions (function composition style).
//!
//! ```ignore
//! // Right-to-left function composition (mathematical composition)
//! let composed = compose!(h, g, f);
//! let result = composed(value); // h(g(f(value)))
//! ```
//!
//! **Use cases**:
//! - When reusing a composed function in multiple places
//! - When you want to name a composed function to clarify intent
//! - When passing a function to higher-order functions (`map`, `filter_map`, etc.)
//! - When you want to apply point-free style
//!
//! ## Criteria for choosing between them
//!
//! | Situation | Recommended Macro |
//! |------|-----------|
//! | One-time transformation | pipe! |
//! | Reuse in multiple places | compose! |
//! | Want to name the composed function | compose! |
//! | Pass to `map`/`filter_map` | compose! (for multiple functions) |
//! | Want to make data flow explicit | pipe! |
//!
//! ## Example: Choosing macros for event creation
//!
//! ```ignore
//! // Define a reusable composed function with compose!
//! let to_shipping_event = compose!(
//!     PlaceOrderEvent::ShippableOrderPlaced,
//!     create_shipping_event
//! );
//!
//! // Apply the composed function
//! let shipping_events = vec![to_shipping_event(priced_order)];
//!
//! // One-time transformation with pipe! (without compose!)
//! let shipping_events = vec![pipe!(
//!     priced_order,
//!     create_shipping_event,
//!     PlaceOrderEvent::ShippableOrderPlaced
//! )];
//! ```
//!
//! # Module Structure
//!
//! - [`error_types`] - Error types (validation, pricing, remote service)
//! - [`unvalidated_types`] - Unvalidated input types
//! - [`validated_types`] - Validated types
//! - [`priced_types`] - Priced types
//! - [`shipping_types`] - Shipping-related types
//! - [`acknowledgment_types`] - Acknowledgment email types
//! - [`output_types`] - Output event types
//! - [`events`] - Event generation functions
//! - [`place_order`] - Workflow integration function

pub mod acknowledgment_types;
pub mod error_types;
pub mod events;
pub mod output_types;
pub mod place_order;
pub mod priced_types;
pub mod pricing;
pub mod pricing_catalog;
pub mod shipping;
pub mod shipping_types;
pub mod unvalidated_types;
pub mod validated_types;
pub mod validation;

// =============================================================================
// Type re-exports
// =============================================================================

pub use acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
pub use error_types::{
    PlaceOrderError, PricingError, RemoteServiceError, ServiceInfo, WorkflowValidationError,
};
pub use events::{create_billing_event, create_events, create_shipping_event, make_shipment_line};
pub use output_types::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, ShippableOrderLine,
    ShippableOrderPlaced,
};
pub use place_order::place_order;
pub use priced_types::{PricedOrder, PricedOrderLine, PricedOrderProductLine};
pub use pricing::price_order;
pub use shipping::{
    ShippingRegion, acknowledge_order, acknowledge_order_with_logging, add_shipping_info_to_order,
    calculate_shipping_cost, classify_shipping_region, free_vip_shipping,
};
pub use shipping_types::{PricedOrderWithShippingMethod, ShippingInfo, ShippingMethod};
pub use unvalidated_types::{
    UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};
pub use validated_types::{
    AddressValidationError, CheckedAddress, PricingMethod, ValidatedOrder, ValidatedOrderLine,
};
pub use validation::validate_order;

// Phase 10: PricingCatalog
pub use pricing_catalog::{PricingCatalog, create_catalog_pricing_function};
