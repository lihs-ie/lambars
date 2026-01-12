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
