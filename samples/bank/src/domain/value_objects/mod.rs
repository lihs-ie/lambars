//! Value objects for the bank domain.
//!
//! Value objects are immutable objects that have no identity. They are defined
//! only by their values and are used to describe characteristics or attributes
//! of domain entities.
//!
//! # Available Value Objects
//!
//! - [`AccountId`] - Unique identifier for bank accounts (UUID v7)
//! - [`Money`] - Monetary amount with currency (implements `Semigroup` and `Monoid`)
//! - [`TransactionId`] - Unique identifier for transactions with idempotency support
//! - [`Timestamp`] - UTC timestamp for events and records
//!
//! # Design Principles
//!
//! All value objects in this module follow these principles:
//!
//! - **Immutability**: Once created, values cannot be changed
//! - **Value equality**: Two instances with the same values are considered equal
//! - **Self-validation**: Invalid values cannot be created (smart constructors)
//! - **Side-effect free**: All operations are pure functions

mod account_id;
mod money;
mod timestamp;
mod transaction_id;

pub use account_id::{AccountId, ValidationError as AccountIdValidationError};
pub use money::{Currency, Money, MoneyError};
pub use timestamp::Timestamp;
pub use transaction_id::{TransactionId, TransactionIdError};
