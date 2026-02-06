//! Compound types used in the order domain
//!
//! Combines the basic types defined in Phase 1 to express higher-level domain entities.
//! Uses lambars Optics (Lens) for efficient immutable data updates.
//!
//! # Type List
//!
//! - [`PersonalName`] - Personal name (first and last name)
//! - [`CustomerInfo`] - Customer information (name, email, VIP status)
//! - [`Address`] - Address
//!
//! # Lens Usage Examples
//!
//! ```
//! use order_taking_sample::compound_types::{PersonalName, CustomerInfo};
//! use order_taking_sample::simple_types::String50;
//! use lambars::optics::Lens;
//!
//! // Get first_name from CustomerInfo (via Lens composition)
//! let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
//!
//! let name_lens = CustomerInfo::name_lens();
//! let first_name_lens = PersonalName::first_name_lens();
//! let customer_first_name = name_lens.compose(first_name_lens);
//!
//! let first_name = customer_first_name.get(&customer);
//! assert_eq!(first_name.value(), "John");
//! ```

mod address;
mod customer_info;
mod personal_name;

pub use address::Address;
pub use customer_info::CustomerInfo;
pub use personal_name::PersonalName;
