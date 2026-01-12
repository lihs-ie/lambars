//! ProcessTurn workflow implementation.
//!
//! This module provides the workflow for processing a complete game turn.
//! It follows the "IO at the Edges" pattern, separating pure domain logic
//! from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [Pure] Extract turn parameters from command
//! 2. [IO] Load session from cache
//! 3. [Pure] Process turn (validate, execute, generate events)
//! 4. [IO] Persist results (update cache, append events)
//!
//! # Architecture
//!
//! The workflow is composed using `pipe_async!` macro with independent named functions:
//!
//! ```text
//! pipe_async!(
//!     AsyncIO::pure(command),
//!     => extract_turn_params,              // Pure: Command -> (GameId, PlayerCommand)
//!     =>> load_session_from_cache(cache),  // IO: -> AsyncIO<Result<(Session, PlayerCommand, GameId), Error>>
//!     => process_turn_result,              // Pure: -> Result<(TurnResult, Events, GameId), Error>
//!     =>> persist_turn_result(cache, event_store, cache_ttl), // IO
//! )
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::turn::{process_turn, ProcessTurnCommand, PlayerCommand};
//! use roguelike_domain::common::Direction;
//!
//! let workflow = process_turn(&cache, &event_store, &snapshot_store, cache_ttl);
//! let command = ProcessTurnCommand::new(
//!     game_identifier,
//!     PlayerCommand::Move(Direction::Up),
//! );
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe_async;
use roguelike_domain::common::{Speed, StatusEffect, StatusEffectType, TurnCount};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::{
    GameIdentifier, GameOutcome, GameSessionEvent, TurnEnded, TurnStarted,
};

use super::commands::{PlayerCommand, ProcessTurnCommand};
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, SnapshotStore, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// TurnResult
// =============================================================================

/// The result of processing a turn.
///
/// Contains the updated game session and an optional game outcome
/// if the game has ended.
///
/// # Type Parameters
///
/// * `S` - The game session type
///
/// # Examples
///
/// ```ignore
/// let result = TurnResult {
///     session: updated_session,
///     game_over: Some(GameOutcome::Victory),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnResult<S> {
    /// The updated game session after turn processing.
    pub session: S,
    /// The game outcome if the game has ended, None otherwise.
    pub game_over: Option<GameOutcome>,
}

impl<S> TurnResult<S> {
    /// Creates a new turn result with no game over.
    #[must_use]
    pub const fn continuing(session: S) -> Self {
        Self {
            session,
            game_over: None,
        }
    }

    /// Creates a new turn result with a game over condition.
    #[must_use]
    pub const fn game_ended(session: S, outcome: GameOutcome) -> Self {
        Self {
            session,
            game_over: Some(outcome),
        }
    }

    /// Returns true if the game has ended.
    #[must_use]
    pub const fn is_game_over(&self) -> bool {
        self.game_over.is_some()
    }

    /// Returns true if the game is still in progress.
    #[must_use]
    pub const fn is_continuing(&self) -> bool {
        self.game_over.is_none()
    }
}

// =============================================================================
// EntityTurnOrder
// =============================================================================

/// Represents an entity's position in the turn order.
///
/// Used for sorting entities by their speed to determine action order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTurnOrder {
    /// The entity identifier.
    identifier: EntityIdentifier,
    /// The entity's speed stat.
    speed: Speed,
}

impl EntityTurnOrder {
    /// Creates a new entity turn order entry.
    #[must_use]
    pub const fn new(identifier: EntityIdentifier, speed: Speed) -> Self {
        Self { identifier, speed }
    }

    /// Returns the entity identifier.
    #[must_use]
    pub const fn identifier(&self) -> EntityIdentifier {
        self.identifier
    }

    /// Returns the entity's speed.
    #[must_use]
    pub const fn speed(&self) -> Speed {
        self.speed
    }
}

// =============================================================================
// Step 1: Extract Turn Parameters [Pure]
// =============================================================================

/// Extracts the game identifier and player command from the command.
///
/// Input: ProcessTurnCommand
/// Output: (GameIdentifier, PlayerCommand)
fn extract_turn_params(command: ProcessTurnCommand) -> (GameIdentifier, PlayerCommand) {
    (*command.game_identifier(), command.player_command())
}

// =============================================================================
// Step 2: Load Session from Cache [IO]
// =============================================================================

/// Creates a function that loads session from cache.
///
/// Takes ownership of the cache to satisfy 'static lifetime requirements.
/// Returns a function suitable for use in pipe_async! that transforms
/// (GameIdentifier, PlayerCommand) to AsyncIO<Result<(Session, PlayerCommand, GameIdentifier), WorkflowError>>
#[allow(clippy::type_complexity)]
fn load_session_from_cache<C: SessionCache>(
    cache: C,
) -> impl Fn(
    (GameIdentifier, PlayerCommand),
) -> AsyncIO<Result<(C::GameSession, PlayerCommand, GameIdentifier), WorkflowError>> {
    move |(game_identifier, player_command)| {
        cache.get(&game_identifier).fmap(move |session_option| {
            session_option
                .map(|session| (session, player_command, game_identifier))
                .ok_or_else(|| {
                    WorkflowError::not_found("GameSession", game_identifier.to_string())
                })
        })
    }
}

// =============================================================================
// Step 3: Process Turn Result [Pure]
// =============================================================================

/// Processes the turn and generates results.
///
/// Input: Result<(Session, PlayerCommand, GameIdentifier), WorkflowError>
/// Output: Result<(TurnResult<Session>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError>
#[allow(clippy::type_complexity)]
fn process_turn_result<S: Clone>(
    result: Result<(S, PlayerCommand, GameIdentifier), WorkflowError>,
) -> Result<(TurnResult<S>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError> {
    result.and_then(|(session, player_command, game_identifier)| {
        process_turn_pure(&session, player_command)
            .map(|(turn_result, events)| (turn_result, events, game_identifier))
    })
}

// =============================================================================
// Step 4: Persist Turn Result [IO]
// =============================================================================

/// Creates a function that persists the turn result.
///
/// Takes ownership of the cache and event store to satisfy 'static lifetime requirements.
/// Returns a function suitable for use in pipe_async! that transforms
/// Result<(TurnResult<Session>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError>
/// to AsyncIO<WorkflowResult<TurnResult<Session>>>
#[allow(clippy::type_complexity)]
fn persist_turn_result<C: SessionCache, E: EventStore>(
    cache: C,
    event_store: E,
    cache_ttl: Duration,
) -> impl Fn(
    Result<(TurnResult<C::GameSession>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError>,
) -> AsyncIO<WorkflowResult<TurnResult<C::GameSession>>> {
    move |result| match result {
        Err(error) => AsyncIO::pure(Err(error)),
        Ok((turn_result, events, game_identifier)) => {
            let turn_result_clone = turn_result.clone();
            let cache_for_set = cache.clone();
            let event_store_for_append = event_store.clone();

            cache_for_set
                .set(&game_identifier, &turn_result.session, cache_ttl)
                .flat_map(move |()| {
                    event_store_for_append
                        .append(&game_identifier, &events)
                        .fmap(move |()| Ok(turn_result_clone))
                })
        }
    }
}

// =============================================================================
// ProcessTurn Workflow
// =============================================================================

/// Creates a workflow function for processing a complete turn.
///
/// This function returns a closure that processes a full game turn including:
/// - Player action
/// - All enemy actions (in speed order)
/// - Status effect processing
/// - Game over condition checking
///
/// The workflow is composed using `pipe_async!` macro with independent named functions:
///
/// ```text
/// pipe_async!(
///     AsyncIO::pure(command),
///     => extract_turn_params,              // Pure: Command -> (GameId, PlayerCommand)
///     =>> load_session_from_cache(cache),  // IO: -> AsyncIO<Result<(Session, PlayerCommand, GameId), Error>>
///     => process_turn_result,              // Pure: -> Result<(TurnResult, Events, GameId), Error>
///     =>> persist_turn_result(cache, event_store, cache_ttl), // IO
/// )
/// ```
///
/// # Type Parameters
///
/// * `C` - Cache type implementing `SessionCache`
/// * `E` - Event store type implementing `EventStore`
/// * `S` - Snapshot store type implementing `SnapshotStore`
///
/// # Arguments
///
/// * `cache` - The session cache for fast access
/// * `event_store` - The event store for event sourcing
/// * `snapshot_store` - The snapshot store for optimization
/// * `cache_ttl` - Time-to-live for cached sessions
///
/// # Returns
///
/// A function that takes a `ProcessTurnCommand` and returns an `AsyncIO`
/// that produces a `TurnResult` containing the updated session and
/// optional game over condition.
pub fn process_turn<'a, C, E, S>(
    cache: &'a C,
    event_store: &'a E,
    _snapshot_store: &'a S,
    cache_ttl: Duration,
) -> impl Fn(ProcessTurnCommand) -> AsyncIO<WorkflowResult<TurnResult<C::GameSession>>> + 'a
where
    C: SessionCache,
    E: EventStore,
    S: SnapshotStore,
{
    move |command| {
        // Clone dependencies for use in AsyncIO closures (they require 'static)
        let cache = cache.clone();
        let cache_for_persist = cache.clone();
        let event_store = event_store.clone();

        pipe_async!(
            AsyncIO::pure(command),
            => extract_turn_params,                                    // Pure: Command -> (GameId, PlayerCommand)
            =>> load_session_from_cache(cache),                        // IO: -> AsyncIO<Result<(Session, PlayerCommand, GameId), Error>>
            => process_turn_result,                                    // Pure: -> Result<(TurnResult, Events, GameId), Error>
            =>> persist_turn_result(cache_for_persist, event_store, cache_ttl), // IO
        )
    }
}

/// Creates a workflow function with default cache TTL.
pub fn process_turn_with_default_ttl<'a, C, E, S>(
    cache: &'a C,
    event_store: &'a E,
    snapshot_store: &'a S,
) -> impl Fn(ProcessTurnCommand) -> AsyncIO<WorkflowResult<TurnResult<C::GameSession>>> + 'a
where
    C: SessionCache,
    E: EventStore,
    S: SnapshotStore,
{
    process_turn(
        cache,
        event_store,
        snapshot_store,
        DEFAULT_CACHE_TIME_TO_LIVE,
    )
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire turn processing logic.
fn process_turn_pure<S: Clone>(
    _session: &S,
    _player_command: PlayerCommand,
) -> Result<(TurnResult<S>, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "process_turn",
        "GameSession structure not yet connected",
    ))
}

/// Starts a new turn by incrementing the turn counter.
///
/// This is a pure function that returns a new session with
/// the turn counter incremented and the appropriate event.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to update the session turn counter
///
/// # Arguments
///
/// * `session` - The current game session
/// * `current_turn` - The current turn count
/// * `update_fn` - Function that updates the session with new turn count
///
/// # Returns
///
/// A tuple of (updated_session, turn_started_event).
///
/// # Examples
///
/// ```ignore
/// let (updated, event) = start_turn(
///     &session,
///     TurnCount::new(5),
///     |s, turn| s.with_turn(turn),
/// );
/// assert_eq!(event.turn().value(), 6);
/// ```
pub fn start_turn<S, F>(session: &S, current_turn: TurnCount, update_fn: F) -> (S, TurnStarted)
where
    S: Clone,
    F: Fn(&S, TurnCount) -> S,
{
    let next_turn = current_turn.next();
    let updated_session = update_fn(session, next_turn);
    let event = TurnStarted::new(next_turn);
    (updated_session, event)
}

/// Validates that a player command is legal in the current game state.
///
/// This is a pure function that checks if the command can be executed.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Validation function that checks if command is legal
///
/// # Arguments
///
/// * `session` - The current game session
/// * `command` - The player command to validate
/// * `validate_fn` - Function that validates the command
///
/// # Returns
///
/// `Ok(())` if the command is valid, `Err(WorkflowError)` otherwise.
///
/// # Examples
///
/// ```ignore
/// let result = validate_player_command(
///     &session,
///     PlayerCommand::Move(Direction::Up),
///     |s, cmd| {
///         // Check if move is valid
///         Ok(())
///     },
/// );
/// ```
pub fn validate_player_command<S, F>(
    session: &S,
    command: PlayerCommand,
    validate_fn: F,
) -> Result<(), WorkflowError>
where
    F: Fn(&S, PlayerCommand) -> Result<(), WorkflowError>,
{
    validate_fn(session, command)
}

/// Executes a player command and updates the session.
///
/// This is a pure function that applies the player's action.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function that applies the command to the session
///
/// # Arguments
///
/// * `session` - The current game session
/// * `command` - The player command to execute
/// * `execute_fn` - Function that applies the command and returns events
///
/// # Returns
///
/// A tuple of (updated_session, generated_events).
///
/// # Examples
///
/// ```ignore
/// let (updated, events) = execute_player_command(
///     &session,
///     PlayerCommand::Move(Direction::Up),
///     |s, cmd| {
///         // Apply move
///         (s.with_player_position(new_pos), vec![])
///     },
/// );
/// ```
pub fn execute_player_command<S, F>(
    session: &S,
    command: PlayerCommand,
    execute_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, PlayerCommand) -> (S, Vec<GameSessionEvent>),
{
    execute_fn(session, command)
}

/// Resolves the turn order for all entities based on speed.
///
/// This is a pure function that sorts entities by their speed
/// in descending order (faster entities act first).
///
/// # Arguments
///
/// * `entities` - A slice of entity turn order entries
///
/// # Returns
///
/// A new vector with entities sorted by speed (descending).
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::{resolve_turn_order, EntityTurnOrder};
/// use roguelike_domain::enemy::EntityIdentifier;
/// use roguelike_domain::common::Speed;
///
/// let slow = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(5));
/// let fast = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
/// let entities = vec![slow, fast];
///
/// let ordered = resolve_turn_order(&entities);
/// // Fast entity should be first
/// assert!(ordered[0].speed().value() > ordered[1].speed().value());
/// ```
#[must_use]
pub fn resolve_turn_order(entities: &[EntityTurnOrder]) -> Vec<EntityTurnOrder> {
    let mut sorted = entities.to_vec();
    sorted.sort_by_key(|entity| std::cmp::Reverse(entity.speed().value()));
    sorted
}

/// Processes all enemy turns in order using fold.
///
/// This is a pure function that iterates through all enemies
/// in turn order and applies their actions.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function that processes a single enemy turn
///
/// # Arguments
///
/// * `session` - The current game session
/// * `enemy_order` - The sorted list of enemy turn orders
/// * `process_fn` - Function that processes a single enemy and returns updated session + events
///
/// # Returns
///
/// A tuple of (final_session, all_enemy_events).
///
/// # Examples
///
/// ```ignore
/// let (updated, events) = process_all_enemy_turns(
///     &session,
///     &enemy_order,
///     |s, enemy_id| {
///         // Process enemy AI and return updated session
///         (s.clone(), vec![])
///     },
/// );
/// ```
pub fn process_all_enemy_turns<S, F>(
    session: &S,
    enemy_order: &[EntityTurnOrder],
    process_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, EntityIdentifier) -> (S, Vec<GameSessionEvent>),
{
    enemy_order.iter().fold(
        (session.clone(), Vec::new()),
        |(current_session, mut events), entry| {
            let (updated_session, enemy_events) = process_fn(&current_session, entry.identifier());
            events.extend(enemy_events);
            (updated_session, events)
        },
    )
}

/// Processes status effects for all entities at turn end.
///
/// This is a pure function that applies status effect damage/healing
/// and decrements effect durations.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function that processes status effects
///
/// # Arguments
///
/// * `session` - The current game session
/// * `process_fn` - Function that processes effects and returns updated session + events
///
/// # Returns
///
/// A tuple of (updated_session, status_effect_events).
///
/// # Examples
///
/// ```ignore
/// let (updated, events) = process_status_effects(
///     &session,
///     |s| {
///         // Apply poison damage, tick down durations, etc.
///         (s.clone(), vec![])
///     },
/// );
/// ```
pub fn process_status_effects<S, F>(session: &S, process_fn: F) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S) -> (S, Vec<GameSessionEvent>),
{
    process_fn(session)
}

/// Applies a single status effect tick to an entity.
///
/// This is a helper function for processing individual status effects.
///
/// # Arguments
///
/// * `effect` - The status effect to process
///
/// # Returns
///
/// A tuple of (optional_damage, optional_continuing_effect).
/// The damage value if the effect deals damage per turn.
/// The continuing effect if it hasn't expired.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::apply_status_effect_tick;
/// use roguelike_domain::common::{StatusEffect, StatusEffectType, Damage};
///
/// let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);
/// let (damage, remaining) = apply_status_effect_tick(&poison);
///
/// assert!(damage.is_some());
/// assert!(remaining.is_some());
/// ```
#[must_use]
pub fn apply_status_effect_tick(effect: &StatusEffect) -> (Option<u32>, Option<StatusEffect>) {
    let damage = match effect.effect_type() {
        StatusEffectType::Poison | StatusEffectType::Burn => Some(effect.potency()),
        StatusEffectType::Regeneration => None, // Healing is separate
        _ => None,
    };

    let remaining = effect.tick();

    (damage, remaining)
}

/// Ends the current turn.
///
/// This is a pure function that finalizes turn processing.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to finalize the turn
///
/// # Arguments
///
/// * `session` - The current game session
/// * `current_turn` - The current turn count
/// * `finalize_fn` - Function that performs any end-of-turn cleanup
///
/// # Returns
///
/// A tuple of (updated_session, turn_ended_event).
///
/// # Examples
///
/// ```ignore
/// let (updated, event) = end_turn(
///     &session,
///     TurnCount::new(5),
///     |s| s.clone(),
/// );
/// assert_eq!(event.turn().value(), 5);
/// ```
pub fn end_turn<S, F>(session: &S, current_turn: TurnCount, finalize_fn: F) -> (S, TurnEnded)
where
    S: Clone,
    F: Fn(&S) -> S,
{
    let updated_session = finalize_fn(session);
    let event = TurnEnded::new(current_turn);
    (updated_session, event)
}

/// Checks if the game has reached an end condition.
///
/// This is a pure function that evaluates victory and defeat conditions.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function that checks game over conditions
///
/// # Arguments
///
/// * `session` - The current game session
/// * `check_fn` - Function that returns the game outcome if game has ended
///
/// # Returns
///
/// `Some(GameOutcome)` if the game has ended, `None` otherwise.
///
/// # Examples
///
/// ```ignore
/// let outcome = check_game_over(&session, |s| {
///     if player_health <= 0 {
///         Some(GameOutcome::Defeat)
///     } else if cleared_all_floors {
///         Some(GameOutcome::Victory)
///     } else {
///         None
///     }
/// });
/// ```
pub fn check_game_over<S, F>(session: &S, check_fn: F) -> Option<GameOutcome>
where
    F: Fn(&S) -> Option<GameOutcome>,
{
    check_fn(session)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // TurnResult Tests
    // =========================================================================

    mod turn_result {
        use super::*;

        #[rstest]
        fn continuing_creates_result_without_game_over() {
            let result = TurnResult::continuing(42);

            assert_eq!(result.session, 42);
            assert!(result.game_over.is_none());
            assert!(result.is_continuing());
            assert!(!result.is_game_over());
        }

        #[rstest]
        fn game_ended_creates_result_with_outcome() {
            let result = TurnResult::game_ended("session", GameOutcome::Victory);

            assert_eq!(result.session, "session");
            assert_eq!(result.game_over, Some(GameOutcome::Victory));
            assert!(result.is_game_over());
            assert!(!result.is_continuing());
        }

        #[rstest]
        fn game_ended_with_defeat() {
            let result = TurnResult::game_ended(100, GameOutcome::Defeat);

            assert_eq!(result.game_over, Some(GameOutcome::Defeat));
        }

        #[rstest]
        fn equality() {
            let result1 = TurnResult::continuing(42);
            let result2 = TurnResult::continuing(42);
            let result3 = TurnResult::continuing(99);
            let result4 = TurnResult::game_ended(42, GameOutcome::Victory);

            assert_eq!(result1, result2);
            assert_ne!(result1, result3);
            assert_ne!(result1, result4);
        }

        #[rstest]
        fn clone() {
            let result = TurnResult::game_ended("test", GameOutcome::Victory);
            let cloned = result.clone();
            assert_eq!(result, cloned);
        }

        #[rstest]
        fn debug_format() {
            let result = TurnResult::continuing(42);
            let debug = format!("{:?}", result);
            assert!(debug.contains("TurnResult"));
        }
    }

    // =========================================================================
    // EntityTurnOrder Tests
    // =========================================================================

    mod entity_turn_order {
        use super::*;

        #[rstest]
        fn new_creates_entry() {
            let identifier = EntityIdentifier::new();
            let speed = Speed::new(10);
            let entry = EntityTurnOrder::new(identifier, speed);

            assert_eq!(entry.identifier(), identifier);
            assert_eq!(entry.speed(), speed);
        }

        #[rstest]
        fn equality() {
            let identifier = EntityIdentifier::new();
            let speed = Speed::new(10);

            let entry1 = EntityTurnOrder::new(identifier, speed);
            let entry2 = EntityTurnOrder::new(identifier, speed);
            let entry3 = EntityTurnOrder::new(identifier, Speed::new(5));

            assert_eq!(entry1, entry2);
            assert_ne!(entry1, entry3);
        }

        #[rstest]
        fn clone() {
            let entry = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let cloned = entry;
            assert_eq!(entry, cloned);
        }
    }

    // =========================================================================
    // resolve_turn_order Tests
    // =========================================================================

    mod resolve_turn_order_tests {
        use super::*;

        #[rstest]
        fn sorts_by_speed_descending() {
            let slow = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(5));
            let medium = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let fast = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(15));

            let entities = vec![slow, fast, medium];
            let ordered = resolve_turn_order(&entities);

            assert_eq!(ordered[0].speed().value(), 15);
            assert_eq!(ordered[1].speed().value(), 10);
            assert_eq!(ordered[2].speed().value(), 5);
        }

        #[rstest]
        fn handles_empty_list() {
            let entities: Vec<EntityTurnOrder> = vec![];
            let ordered = resolve_turn_order(&entities);
            assert!(ordered.is_empty());
        }

        #[rstest]
        fn handles_single_entity() {
            let entity = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let entities = vec![entity];
            let ordered = resolve_turn_order(&entities);

            assert_eq!(ordered.len(), 1);
            assert_eq!(ordered[0].speed().value(), 10);
        }

        #[rstest]
        fn preserves_order_for_equal_speeds() {
            let entity1 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let entity2 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));

            let entities = vec![entity1, entity2];
            let ordered = resolve_turn_order(&entities);

            assert_eq!(ordered.len(), 2);
            // Both have same speed
            assert_eq!(ordered[0].speed().value(), 10);
            assert_eq!(ordered[1].speed().value(), 10);
        }
    }

    // =========================================================================
    // start_turn Tests
    // =========================================================================

    mod start_turn_tests {
        use super::*;

        #[rstest]
        fn increments_turn_counter() {
            let session = String::from("session");
            let current_turn = TurnCount::new(5);

            let (updated, event) = start_turn(&session, current_turn, |s, _turn| s.clone());

            assert_eq!(updated, "session");
            assert_eq!(event.turn().value(), 6);
        }

        #[rstest]
        fn starts_from_turn_one() {
            let session = 42;
            let current_turn = TurnCount::new(0);

            let (_updated, event) = start_turn(&session, current_turn, |s, _turn| *s);

            assert_eq!(event.turn().value(), 1);
        }
    }

    // =========================================================================
    // validate_player_command Tests
    // =========================================================================

    mod validate_player_command_tests {
        use super::*;
        use roguelike_domain::common::Direction;

        #[rstest]
        fn returns_ok_for_valid_command() {
            let session = "session";
            let command = PlayerCommand::Move(Direction::Up);

            let result = validate_player_command(&session, command, |_, _| Ok(()));

            assert!(result.is_ok());
        }

        #[rstest]
        fn returns_error_for_invalid_command() {
            let session = "session";
            let command = PlayerCommand::Move(Direction::Up);

            let result = validate_player_command(&session, command, |_, _| {
                Err(WorkflowError::conflict("Invalid move"))
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn passes_session_to_validator() {
            let session = 42;
            let command = PlayerCommand::Wait;

            let result = validate_player_command(&session, command, |s, _| {
                if *s == 42 {
                    Ok(())
                } else {
                    Err(WorkflowError::conflict("Wrong session"))
                }
            });

            assert!(result.is_ok());
        }

        #[rstest]
        fn passes_command_to_validator() {
            let session = "session";
            let command = PlayerCommand::Wait;

            let result = validate_player_command(&session, command, |_, cmd| {
                if cmd.is_wait() {
                    Ok(())
                } else {
                    Err(WorkflowError::conflict("Expected wait"))
                }
            });

            assert!(result.is_ok());
        }
    }

    // =========================================================================
    // execute_player_command Tests
    // =========================================================================

    mod execute_player_command_tests {
        use super::*;
        use roguelike_domain::common::Direction;

        #[rstest]
        fn applies_command_to_session() {
            let session = 0i32;
            let command = PlayerCommand::Move(Direction::Up);

            let (updated, events) =
                execute_player_command(&session, command, |s, _| (s + 1, vec![]));

            assert_eq!(updated, 1);
            assert!(events.is_empty());
        }

        #[rstest]
        fn generates_events() {
            let session = String::from("session");
            let command = PlayerCommand::Wait;
            let event = TurnStarted::first();

            let (_, events) = execute_player_command(&session, command, |s, _| {
                (s.clone(), vec![GameSessionEvent::TurnStarted(event)])
            });

            assert_eq!(events.len(), 1);
        }
    }

    // =========================================================================
    // process_all_enemy_turns Tests
    // =========================================================================

    mod process_all_enemy_turns_tests {
        use super::*;

        #[rstest]
        fn processes_empty_list() {
            let session = 0i32;
            let enemies: Vec<EntityTurnOrder> = vec![];

            let (updated, events) =
                process_all_enemy_turns(&session, &enemies, |s, _| (*s, vec![]));

            assert_eq!(updated, 0);
            assert!(events.is_empty());
        }

        #[rstest]
        fn folds_over_all_enemies() {
            let session = 0i32;
            let enemy1 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let enemy2 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(5));
            let enemies = vec![enemy1, enemy2];

            let (updated, _events) =
                process_all_enemy_turns(&session, &enemies, |s, _| (s + 1, vec![]));

            // Should have processed 2 enemies
            assert_eq!(updated, 2);
        }

        #[rstest]
        fn accumulates_events() {
            let session = String::from("session");
            let enemy1 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let enemy2 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(5));
            let enemies = vec![enemy1, enemy2];

            let (_, events) = process_all_enemy_turns(&session, &enemies, |s, _| {
                (
                    s.clone(),
                    vec![GameSessionEvent::TurnStarted(TurnStarted::first())],
                )
            });

            // Should have 2 events (one per enemy)
            assert_eq!(events.len(), 2);
        }

        #[rstest]
        fn processes_in_order() {
            let session = Vec::<u32>::new();
            let enemy1 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(10));
            let enemy2 = EntityTurnOrder::new(EntityIdentifier::new(), Speed::new(5));
            let enemies = vec![enemy1, enemy2];

            let (updated, _) = process_all_enemy_turns(&session, &enemies, |s, _| {
                let mut new_vec = s.clone();
                new_vec.push(new_vec.len() as u32);
                (new_vec, vec![])
            });

            // Should have processed in order
            assert_eq!(updated, vec![0, 1]);
        }
    }

    // =========================================================================
    // process_status_effects Tests
    // =========================================================================

    mod process_status_effects_tests {
        use super::*;

        #[rstest]
        fn applies_process_function() {
            let session = 0i32;

            let (updated, _) = process_status_effects(&session, |s| (*s + 10, vec![]));

            assert_eq!(updated, 10);
        }

        #[rstest]
        fn returns_events() {
            let session = String::from("session");
            let event = TurnStarted::first();

            let (_, events) = process_status_effects(&session, |s| {
                (s.clone(), vec![GameSessionEvent::TurnStarted(event)])
            });

            assert_eq!(events.len(), 1);
        }
    }

    // =========================================================================
    // apply_status_effect_tick Tests
    // =========================================================================

    mod apply_status_effect_tick_tests {
        use super::*;

        #[rstest]
        fn poison_deals_damage() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let (damage, remaining) = apply_status_effect_tick(&effect);

            assert_eq!(damage, Some(5));
            assert!(remaining.is_some());
            assert_eq!(remaining.unwrap().remaining_turns(), 2);
        }

        #[rstest]
        fn burn_deals_damage() {
            let effect = StatusEffect::new(StatusEffectType::Burn, 2, 10);
            let (damage, remaining) = apply_status_effect_tick(&effect);

            assert_eq!(damage, Some(10));
            assert!(remaining.is_some());
        }

        #[rstest]
        fn regeneration_no_damage() {
            let effect = StatusEffect::new(StatusEffectType::Regeneration, 5, 3);
            let (damage, remaining) = apply_status_effect_tick(&effect);

            assert!(damage.is_none());
            assert!(remaining.is_some());
        }

        #[rstest]
        fn effect_expires_at_one_turn() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 1, 5);
            let (_damage, remaining) = apply_status_effect_tick(&effect);

            assert!(remaining.is_none());
        }

        #[rstest]
        fn stun_no_damage() {
            let effect = StatusEffect::new(StatusEffectType::Stun, 2, 0);
            let (damage, _) = apply_status_effect_tick(&effect);

            assert!(damage.is_none());
        }
    }

    // =========================================================================
    // end_turn Tests
    // =========================================================================

    mod end_turn_tests {
        use super::*;

        #[rstest]
        fn creates_turn_ended_event() {
            let session = String::from("session");
            let turn = TurnCount::new(5);

            let (_, event) = end_turn(&session, turn, |s| s.clone());

            assert_eq!(event.turn().value(), 5);
        }

        #[rstest]
        fn applies_finalize_function() {
            let session = 0i32;
            let turn = TurnCount::new(1);

            let (updated, _) = end_turn(&session, turn, |s| s + 1);

            assert_eq!(updated, 1);
        }
    }

    // =========================================================================
    // check_game_over Tests
    // =========================================================================

    mod check_game_over_tests {
        use super::*;

        #[rstest]
        fn returns_none_when_game_continues() {
            let session = "session";

            let result = check_game_over(&session, |_| None);

            assert!(result.is_none());
        }

        #[rstest]
        fn returns_victory_when_won() {
            let session = "victory";

            let result = check_game_over(&session, |s| {
                if *s == "victory" {
                    Some(GameOutcome::Victory)
                } else {
                    None
                }
            });

            assert_eq!(result, Some(GameOutcome::Victory));
        }

        #[rstest]
        fn returns_defeat_when_lost() {
            let session = 0; // Player health is 0

            let result = check_game_over(&session, |s| {
                if *s <= 0 {
                    Some(GameOutcome::Defeat)
                } else {
                    None
                }
            });

            assert_eq!(result, Some(GameOutcome::Defeat));
        }
    }
}
