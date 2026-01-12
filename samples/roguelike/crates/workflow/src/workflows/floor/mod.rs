mod commands;
mod descend_floor;
mod generate_floor;
mod trigger_trap;
mod update_visibility;

// Re-export command types
pub use commands::{
    DescendFloorCommand, GenerateFloorCommand, TriggerTrapCommand, UpdateVisibilityCommand,
};

// Re-export workflow functions
pub use descend_floor::descend_floor;
pub use generate_floor::generate_floor;
pub use trigger_trap::trigger_trap;
pub use update_visibility::update_visibility;

// Re-export configuration types
pub use generate_floor::FloorGenerationConfiguration;
pub use trigger_trap::TrapEffect;

// Re-export pure functions for testing and composition
pub use descend_floor::{
    calculate_next_floor_level, set_player_at_spawn_point, spawn_floor_enemies,
    update_session_for_floor_change, validate_at_down_stairs,
};
pub use generate_floor::{
    get_floor_configuration, place_items, place_stairs, place_traps, update_session_floor,
};
pub use trigger_trap::{
    TrapInfo, apply_trap_effect, calculate_trap_effect, disarm_trap, find_trap_at_position,
};
pub use update_visibility::{
    calculate_field_of_view, get_player_position, update_explored_tiles, update_session_visibility,
};
