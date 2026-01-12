mod aggregate;
mod errors;
mod events;
mod identifier;
mod status;

// Re-export aggregate
pub use aggregate::GameSession;

// Re-export identifier types
pub use identifier::GameIdentifier;

// Re-export status types
pub use status::{GameOutcome, GameStatus};

// Re-export error types
pub use errors::GameSessionError;

// Re-export event types
pub use events::{GameEnded, GameSessionEvent, GameStarted, RandomSeed, TurnEnded, TurnStarted};
