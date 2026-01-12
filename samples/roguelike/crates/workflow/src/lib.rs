pub mod commands;
pub mod errors;
pub mod ports;
pub mod workflows;

// Re-export common types for convenience
pub use errors::WorkflowError;
pub use ports::{
    EventStore, GameSessionRepository, RandomGenerator, SessionCache, SnapshotStore, WorkflowResult,
};

// Re-export workflow types
pub use workflows::game_session::{
    CreateGameCommand, CreateSnapshotCommand, EndGameCommand, ResumeGameCommand,
    SessionStateAccessor, create_game, create_snapshot, end_game, reconstruct_from_events,
    resume_game,
};

// Re-export turn workflow types
pub use workflows::turn::{PlayerCommand, ProcessTurnCommand, TurnResult, process_turn};

// Re-export player workflow types
pub use workflows::player::{
    AttackEnemyCommand, EquipItemCommand, MovePlayerCommand, PickUpItemCommand, TakeDamageCommand,
    UseItemCommand, attack_enemy, equip_item, move_player, pick_up_item, take_damage, use_item,
};
