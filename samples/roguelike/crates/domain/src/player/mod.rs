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
