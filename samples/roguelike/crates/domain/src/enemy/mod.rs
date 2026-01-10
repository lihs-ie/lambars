//! Enemy domain module.
//!
//! This module contains all enemy-related domain types including:
//!
//! - **Identifiers**: [`EntityIdentifier`] for uniquely identifying enemies
//! - **Types**: [`EnemyType`] for different enemy species
//! - **Behaviors**: [`AiBehavior`] for AI behavior patterns
//! - **Loot**: [`LootTable`] and [`LootEntry`] for item drops
//! - **Errors**: [`EnemyError`] for enemy-related failures
//! - **Events**: Domain events for enemy lifecycle
//!
//! # Examples
//!
//! ```
//! use roguelike_domain::enemy::{
//!     EntityIdentifier, EnemyType, AiBehavior, LootTable, LootEntry
//! };
//! use roguelike_domain::item::ItemIdentifier;
//!
//! // Create an enemy identifier
//! let enemy_id = EntityIdentifier::new();
//!
//! // Define enemy type and behavior
//! let enemy_type = EnemyType::Goblin;
//! let behavior = AiBehavior::Aggressive;
//!
//! // Create a loot table
//! let item_id = ItemIdentifier::new();
//! let entry = LootEntry::new(item_id, 0.5, 1, 3).unwrap();
//! let loot_table = LootTable::empty().with_entry(entry);
//!
//! println!("Enemy: {} ({}) with {} loot entries",
//!     enemy_type, behavior, loot_table.len());
//! ```

mod behavior;
mod enemy_type;
mod errors;
mod events;
mod identifier;
mod loot;

// Re-export identifier types
pub use identifier::EntityIdentifier;

// Re-export enemy type
pub use enemy_type::EnemyType;

// Re-export behavior
pub use behavior::AiBehavior;

// Re-export loot types
pub use loot::{LootEntry, LootTable};

// Re-export errors
pub use errors::EnemyError;

// Re-export events
pub use events::{EnemyAttacked, EnemyDied, EnemyEvent, EnemyMoved, EnemySpawned};
