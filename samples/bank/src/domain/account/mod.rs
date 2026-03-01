//! Account aggregate and related types.
//!
//! This module contains the Account aggregate root and all associated types
//! for managing bank accounts in the domain layer.
//!
//! # Structure
//!
//! - [`aggregate`] - Account aggregate root with Lens support
//! - [`commands`] - Command definitions for account operations
//! - [`errors`] - Domain errors using Either type
//! - [`events`] - Domain events for account state changes
//!
//! # Design Principles
//!
//! This module follows functional programming principles:
//!
//! - **Immutability**: All data structures are immutable
//! - **Pure Functions**: Domain logic is side-effect free
//! - **Type Safety**: Strong typing with ADTs for events and errors
//! - **Event Sourcing**: State derived from events via Foldable
//!
//! # Examples
//!
//! ## Creating an Account from Events
//!
//! ```rust
//! use bank::domain::account::{
//!     aggregate::{Account, AccountStatus},
//!     events::{AccountEvent, AccountOpened, EventId},
//! };
//! use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
//! use lambars::persistent::PersistentList;
//!
//! // Create an AccountOpened event
//! let opened = AccountOpened {
//!     event_id: EventId::generate(),
//!     account_id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     initial_balance: Money::new(10000, Currency::JPY),
//!     opened_at: Timestamp::now(),
//! };
//!
//! // Build event list and reconstruct account
//! let events = PersistentList::singleton(AccountEvent::Opened(opened));
//! let account = Account::from_events(&events);
//! assert!(account.is_some());
//! ```
//!
//! ## Using Lenses for Immutable Updates
//!
//! ```rust
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::value_objects::{AccountId, Money, Currency};
//! use lambars::optics::Lens;
//!
//! let account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     balance: Money::new(10000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! // Use lens to update balance immutably
//! let new_balance = Money::new(15000, Currency::JPY);
//! let updated = Account::balance_lens().set(account, new_balance);
//! ```
//!
//! ## Domain Validation
//!
//! ```rust
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::account::errors::DomainError;
//! use bank::domain::value_objects::{AccountId, Money, Currency};
//!
//! let account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     balance: Money::new(10000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! // Validate withdrawal
//! let result = account.can_withdraw(&Money::new(5000, Currency::JPY));
//! assert!(result.is_right()); // Withdrawal is permitted
//! ```

pub mod aggregate;
pub mod commands;
pub mod errors;
pub mod events;

pub use aggregate::*;
pub use commands::*;
pub use errors::*;
pub use events::*;
