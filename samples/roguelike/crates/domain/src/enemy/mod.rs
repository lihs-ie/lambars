//! Enemy domain module.
//!
//! This module contains all enemy-related domain types including:
//!
//! - **Aggregate**: [`Enemy`] aggregate root for enemy entities
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
//!     Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable, LootEntry
//! };
//! use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
//! use roguelike_domain::item::ItemIdentifier;
//!
//! // Create an enemy
//! let enemy_id = EntityIdentifier::new();
//! let position = Position::new(5, 10);
//! let stats = CombatStats::new(
//!     Health::new(100).unwrap(),
//!     Health::new(100).unwrap(),
//!     Mana::zero(),
//!     Mana::zero(),
//!     Attack::new(15),
//!     Defense::new(5),
//!     Speed::new(10),
//! ).unwrap();
//!
//! let enemy = Enemy::new(
//!     enemy_id,
//!     EnemyType::Goblin,
//!     position,
//!     stats,
//!     AiBehavior::Aggressive,
//!     LootTable::empty(),
//! );
//!
//! println!("Enemy: {} at {} (alive: {})",
//!     enemy.enemy_type(), enemy.position(), enemy.is_alive());
//! ```

mod aggregate;
mod behavior;
mod enemy_type;
mod errors;
mod events;
mod identifier;
mod loot;

// Re-export aggregate
pub use aggregate::Enemy;

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
