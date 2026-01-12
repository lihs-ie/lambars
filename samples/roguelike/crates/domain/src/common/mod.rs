mod coordinate;
mod errors;
mod numeric;
mod rarity;
mod stats;
mod status;

// Re-export coordinate types
pub use coordinate::{Direction, Distance, Position};

// Re-export error types
pub use errors::{DomainError, ValidationError};

// Re-export numeric types
pub use numeric::{
    Attack, Damage, Defense, Experience, FloorLevel, Health, Level, Mana, Speed, Stat, TurnCount,
};

// Re-export rarity type
pub use rarity::Rarity;

// Re-export stats types
pub use stats::{BaseStats, CombatStats, DamageModifier};

// Re-export status types
pub use status::{StatusEffect, StatusEffectType};
