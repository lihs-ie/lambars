mod commands;
mod process_turn;
mod wait_turn;

// Re-export command types
pub use commands::{PlayerCommand, ProcessTurnCommand, WaitTurnCommand};

// Re-export result types
pub use process_turn::TurnResult;

// Re-export workflow functions
pub use process_turn::process_turn;
pub use wait_turn::wait_turn;

// Re-export pure functions for testing and composition
pub use process_turn::{
    EntityTurnOrder, apply_status_effect_tick, check_game_over, end_turn, execute_player_command,
    process_all_enemy_turns, process_status_effects, resolve_turn_order, start_turn,
    validate_player_command,
};
pub use wait_turn::{
    WaitBonus, apply_wait_bonus, calculate_hp_regeneration, can_benefit_from_wait,
};
