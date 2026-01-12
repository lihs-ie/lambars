mod commands;
mod process_enemy_death;
mod process_enemy_turn;
mod spawn_enemies;

// Re-export command types
pub use commands::{ProcessEnemyDeathCommand, ProcessEnemyTurnCommand, SpawnEnemiesCommand};

// Re-export EnemyAction type
pub use process_enemy_turn::EnemyAction;

// Re-export workflow functions
pub use process_enemy_death::process_enemy_death;
pub use process_enemy_turn::process_enemy_turn;
pub use spawn_enemies::spawn_enemies;

// Re-export pure functions for testing and composition
pub use process_enemy_death::{calculate_loot, drop_items_at_position, remove_enemy_from_session};
pub use process_enemy_turn::{
    decide_enemy_action, execute_enemy_action, find_enemy_by_id, validate_enemy_active,
};
pub use spawn_enemies::{
    add_enemies_to_session, find_valid_spawn_points, generate_enemies, get_spawn_configuration,
};
