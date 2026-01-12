//! Player domain module.
//!
//! This module provides all types related to the Player aggregate:
//!
//! ## Aggregate
//!
//! - [`Player`]: The Player aggregate root
//!
//! ## Identifiers
//!
//! - [`PlayerIdentifier`]: UUID-based unique player identifier
//! - [`PlayerName`]: Validated player name with length constraints
//!
//! ## Equipment & Inventory
//!
//! - [`EquipmentSlot`]: Enum representing equipment slot types (Weapon, Armor, Helmet, Accessory)
//! - [`EquipmentSlots`]: Container for all equipment slots
//! - [`Inventory`]: Container for player's item stacks
//! - [`ItemStack`]: A stack of items with quantity
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
//! # Examples
//!
//! ```
//! use roguelike_domain::common::{
//!     Attack, BaseStats, CombatStats, Defense, Health, Mana, Position, Speed, Stat,
//! };
//! use roguelike_domain::player::{
//!     Player, PlayerIdentifier, PlayerName, EquipmentSlot, EquipmentSlots,
//! };
//!
//! // Create a player
//! let player = Player::new(
//!     PlayerIdentifier::new(),
//!     PlayerName::new("Hero").unwrap(),
//!     Position::new(0, 0),
//!     CombatStats::new(
//!         Health::new(100).unwrap(),
//!         Health::new(100).unwrap(),
//!         Mana::new(50).unwrap(),
//!         Mana::new(50).unwrap(),
//!         Attack::new(20),
//!         Defense::new(15),
//!         Speed::new(10),
//!     ).unwrap(),
//!     BaseStats::new(
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!     ),
//! );
//!
//! assert!(player.is_alive());
//!
//! // Work with equipment
//! let equipment = EquipmentSlots::empty()
//!     .equip(EquipmentSlot::Weapon, "sword".to_string());
//! assert!(equipment.is_slot_occupied(EquipmentSlot::Weapon));
//! ```

mod aggregate;
mod errors;
mod events;
mod identifier;
mod inventory;

// Re-export aggregate
pub use aggregate::Player;

// Re-export identifier types
pub use identifier::{PlayerIdentifier, PlayerName};

// Re-export inventory/equipment types
pub use inventory::{EquipmentSlot, EquipmentSlots, Inventory, ItemStack};

// Re-export error types
pub use errors::PlayerError;

// Re-export event types
pub use events::{
    EntityIdentifier, ExperienceGained, PlayerAttacked, PlayerDamaged, PlayerDied, PlayerEvent,
    PlayerLeveledUp, PlayerMoved,
};
