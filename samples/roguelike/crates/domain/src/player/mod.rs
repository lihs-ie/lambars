//! Player domain module.
//!
//! This module provides all types related to the Player aggregate:
//!
//! ## Identifiers
//!
//! - [`PlayerIdentifier`]: UUID-based unique player identifier
//! - [`PlayerName`]: Validated player name with length constraints
//!
//! ## Equipment
//!
//! - [`EquipmentSlot`]: Enum representing equipment slot types (Weapon, Armor, Helmet, Accessory)
//! - [`EquipmentSlots`]: Container for all equipment slots
//!
//! ## Errors
//!
//! - [`PlayerError`]: All possible player-related errors
//!
//! ## Events
//!
//! - [`PlayerMoved`]: Player moved from one position to another
//! - [`PlayerAttacked`]: Player attacked an entity
//! - [`PlayerDamaged`]: Player received damage
//! - [`PlayerLeveledUp`]: Player gained a level
//! - [`PlayerDied`]: Player died
//! - [`ExperienceGained`]: Player gained experience points
//! - [`PlayerEvent`]: Sum type for all player domain events
//! - [`EntityIdentifier`]: Identifier for game entities
//!
//! ## Note
//!
//! The `Inventory` and `ItemStack` types are not yet implemented as they
//! depend on the `Item` type from the item domain which is not yet available.
//!
//! # Examples
//!
//! ```
//! use roguelike_domain::player::{
//!     PlayerIdentifier, PlayerName, EquipmentSlot, EquipmentSlots,
//! };
//!
//! // Create player identifiers
//! let id = PlayerIdentifier::new();
//! let name = PlayerName::new("Hero").unwrap();
//!
//! // Work with equipment
//! let equipment = EquipmentSlots::empty()
//!     .equip(EquipmentSlot::Weapon, "sword".to_string());
//! assert!(equipment.is_slot_occupied(EquipmentSlot::Weapon));
//! ```

mod errors;
mod events;
mod identifier;
mod inventory;

// Re-export identifier types
pub use identifier::{PlayerIdentifier, PlayerName};

// Re-export inventory/equipment types
pub use inventory::{EquipmentSlot, EquipmentSlots};

// Re-export error types
pub use errors::PlayerError;

// Re-export event types
pub use events::{
    EntityIdentifier, ExperienceGained, PlayerAttacked, PlayerDamaged, PlayerDied, PlayerEvent,
    PlayerLeveledUp, PlayerMoved,
};
