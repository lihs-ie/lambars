//! Basic types used in the order domain (Simple Types)
//!
//! Implements the F# Single Case Discriminated Union pattern using Rust's newtype pattern,
//! following the "Make Illegal States Unrepresentable" principle.
//!
//! # Overview
//!
//! This module provides basic types used in the order processing domain.
//! Each type uses the Smart Constructor pattern to guarantee that only validated values are held.
//!
//! # Type Categories
//!
//! - **Constrained string types**: `String50`, `EmailAddress`, `ZipCode`, `UsStateCode`
//! - **ID types**: `OrderId`, `OrderLineId`
//! - **Product code types**: `WidgetCode`, `GizmoCode`, `ProductCode`
//! - **Quantity types**: `UnitQuantity`, `KilogramQuantity`, `OrderQuantity`
//! - **Price types**: `Price`, `BillingAmount`
//! - **Others**: `VipStatus`, `PromotionCode`, `PdfAttachment`
//!
//! # Usage Examples
//!
//! ```
//! use order_taking_sample::simple_types::{
//!     OrderId, ProductCode, Price, BillingAmount
//! };
//! use rust_decimal::Decimal;
//! use std::str::FromStr;
//!
//! // Create an OrderId (with validation)
//! let order_id = OrderId::create("OrderId", "ORD-2024-001").unwrap();
//! assert_eq!(order_id.value(), "ORD-2024-001");
//!
//! // Create a ProductCode (auto-detects Widget or Gizmo)
//! let widget = ProductCode::create("ProductCode", "W1234").unwrap();
//! let gizmo = ProductCode::create("ProductCode", "G123").unwrap();
//!
//! // Create Price and calculate the sum
//! let price1 = Price::create(Decimal::from_str("100.00").unwrap()).unwrap();
//! let price2 = Price::create(Decimal::from_str("200.00").unwrap()).unwrap();
//! let total = BillingAmount::sum_prices(&[price1, price2]).unwrap();
//! assert_eq!(total.value(), Decimal::from_str("300.00").unwrap());
//! ```
//!
//! # Integration with lambars
//!
//! This module leverages the following lambars library features:
//!
//! - Monadic error handling with `Result<T, ValidationError>`
//! - Fold operation in `BillingAmount::sum_prices` using the `Foldable` trait

pub mod constrained_type;
mod error;
mod identifier_types;
mod misc_types;
mod price_types;
mod product_types;
mod quantity_types;
mod string_types;

// =============================================================================
// Type re-exports
// =============================================================================

// Error types
pub use error::ValidationError;

// String types
pub use string_types::{EmailAddress, String50, UsStateCode, ZipCode};

// ID types
pub use identifier_types::{OrderId, OrderLineId};

// Product code types
pub use product_types::{GizmoCode, ProductCode, WidgetCode};

// Quantity types
pub use quantity_types::{KilogramQuantity, OrderQuantity, UnitQuantity};

// Price types
pub use price_types::{BillingAmount, Price};

// Other types
pub use misc_types::{PdfAttachment, PromotionCode, VipStatus};
