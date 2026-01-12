//! WaitTurn workflow implementation.
//!
//! This module provides a simplified workflow for when the player
//! chooses to wait/rest during their turn. It follows the "IO at
//! the Edges" pattern, separating pure domain logic from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [Pure] Extract game identifier from command
//! 2. [IO] Load session from cache
//! 3. [Pure] Process wait turn (validate, apply bonus, generate events)
//! 4. [IO] Persist results (update cache, append events)
//!
//! # Architecture
//!
//! The workflow is composed using `pipe_async!` macro with independent named functions:
//!
//! ```text
//! pipe_async!(
//!     AsyncIO::pure(command),
//!     => extract_game_identifier,                    // Pure: Command -> GameIdentifier
//!     =>> load_session_from_cache(cache),            // IO: -> AsyncIO<Result<(Session, GameId), Error>>
//!     => process_wait_turn_result,                   // Pure: -> Result<(TurnResult, Events, GameId), Error>
//!     =>> persist_wait_turn_result(cache, event_store, cache_ttl), // IO
//! )
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::turn::{wait_turn, WaitTurnCommand};
//!
//! let workflow = wait_turn(&cache, &event_store, &snapshot_store, cache_ttl);
//! let command = WaitTurnCommand::new(game_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe_async;
use roguelike_domain::common::Health;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent};

use super::commands::WaitTurnCommand;
use super::process_turn::TurnResult;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, SnapshotStore, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

/// Default HP regeneration when waiting.
const WAIT_HP_REGENERATION: u32 = 1;

// =============================================================================
// Step 1: Extract Game Identifier [Pure]
// =============================================================================

/// Extracts the game identifier from the command.
///
/// Input: WaitTurnCommand
/// Output: GameIdentifier
fn extract_game_identifier(command: WaitTurnCommand) -> GameIdentifier {
    *command.game_identifier()
}

// =============================================================================
// Step 2: Load Session from Cache [IO]
// =============================================================================

/// Creates a function that loads session from cache.
///
/// Takes ownership of the cache to satisfy 'static lifetime requirements.
/// Returns a function suitable for use in pipe_async! that transforms
/// GameIdentifier to AsyncIO<Result<(Session, GameIdentifier), WorkflowError>>
#[allow(clippy::type_complexity)]
fn load_session_from_cache<C: SessionCache>(
    cache: C,
) -> impl Fn(GameIdentifier) -> AsyncIO<Result<(C::GameSession, GameIdentifier), WorkflowError>> {
    move |game_identifier| {
        cache.get(&game_identifier).fmap(move |session_option| {
            session_option
                .map(|session| (session, game_identifier))
                .ok_or_else(|| {
                    WorkflowError::not_found("GameSession", game_identifier.to_string())
                })
        })
    }
}

// =============================================================================
// Step 3: Process Wait Turn Result [Pure]
// =============================================================================

/// Processes the wait turn and generates results.
///
/// Input: Result<(Session, GameIdentifier), WorkflowError>
/// Output: Result<(TurnResult<Session>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError>
#[allow(clippy::type_complexity)]
fn process_wait_turn_result<S: Clone>(
    result: Result<(S, GameIdentifier), WorkflowError>,
) -> Result<(TurnResult<S>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError> {
    result.and_then(|(session, game_identifier)| {
        wait_turn_pure(&session).map(|(turn_result, events)| (turn_result, events, game_identifier))
    })
}

// =============================================================================
// Step 4: Persist Wait Turn Result [IO]
// =============================================================================

/// Creates a function that persists the wait turn result.
///
/// Takes ownership of the cache and event store to satisfy 'static lifetime requirements.
/// Returns a function suitable for use in pipe_async! that transforms
/// Result<(TurnResult<Session>, Vec<GameSessionEvent>, GameIdentifier), WorkflowError>
/// to AsyncIO<WorkflowResult<TurnResult<Session>>>
#[allow(clippy::type_complexity)]
fn persist_wait_turn_result<C: SessionCache, E: EventStore>(
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
// WaitBonus
// =============================================================================

/// Represents the bonus granted when a player waits.
///
/// Contains the various benefits applied during a rest turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitBonus {
    /// Amount of HP to regenerate.
    health_regeneration: u32,
}

impl WaitBonus {
    /// Creates a new wait bonus with the specified HP regeneration.
    #[must_use]
    pub const fn new(health_regeneration: u32) -> Self {
        Self {
            health_regeneration,
        }
    }

    /// Returns the HP regeneration amount.
    #[must_use]
    pub const fn health_regeneration(&self) -> u32 {
        self.health_regeneration
    }

    /// Returns the default wait bonus.
    #[must_use]
    pub const fn default_bonus() -> Self {
        Self::new(WAIT_HP_REGENERATION)
    }
}

impl Default for WaitBonus {
    fn default() -> Self {
        Self::default_bonus()
    }
}

// =============================================================================
// WaitTurn Workflow
// =============================================================================

/// Creates a workflow function for processing a wait/rest turn.
///
/// This function returns a closure that processes a simplified turn
/// where the player chooses to wait, potentially gaining rest benefits.
///
/// The workflow is composed using `pipe_async!` macro with independent named functions:
///
/// ```text
/// pipe_async!(
///     AsyncIO::pure(command),
///     => extract_game_identifier,                    // Pure: Command -> GameIdentifier
///     =>> load_session_from_cache(cache),            // IO: -> AsyncIO<Result<(Session, GameId), Error>>
///     => process_wait_turn_result,                   // Pure: -> Result<(TurnResult, Events, GameId), Error>
///     =>> persist_wait_turn_result(cache, event_store, cache_ttl), // IO
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
/// A function that takes a `WaitTurnCommand` and returns an `AsyncIO`
/// that produces a `TurnResult` containing the updated session.
pub fn wait_turn<'a, C, E, S>(
    cache: &'a C,
    event_store: &'a E,
    _snapshot_store: &'a S,
    cache_ttl: Duration,
) -> impl Fn(WaitTurnCommand) -> AsyncIO<WorkflowResult<TurnResult<C::GameSession>>> + 'a
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
            => extract_game_identifier,                                      // Pure: Command -> GameIdentifier
            =>> load_session_from_cache(cache),                              // IO: -> AsyncIO<Result<(Session, GameId), Error>>
            => process_wait_turn_result,                                     // Pure: -> Result<(TurnResult, Events, GameId), Error>
            =>> persist_wait_turn_result(cache_for_persist, event_store, cache_ttl), // IO
        )
    }
}

/// Creates a workflow function with default cache TTL.
pub fn wait_turn_with_default_ttl<'a, C, E, S>(
    cache: &'a C,
    event_store: &'a E,
    snapshot_store: &'a S,
) -> impl Fn(WaitTurnCommand) -> AsyncIO<WorkflowResult<TurnResult<C::GameSession>>> + 'a
where
    C: SessionCache,
    E: EventStore,
    S: SnapshotStore,
{
    wait_turn(
        cache,
        event_store,
        snapshot_store,
        DEFAULT_CACHE_TIME_TO_LIVE,
    )
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire wait turn processing logic.
fn wait_turn_pure<S: Clone>(
    _session: &S,
) -> Result<(TurnResult<S>, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "wait_turn",
        "GameSession structure not yet connected",
    ))
}

/// Applies the wait bonus to the player.
///
/// This is a pure function that applies rest benefits such as
/// HP regeneration when the player chooses to wait.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function that applies the bonus to the session
///
/// # Arguments
///
/// * `session` - The current game session
/// * `bonus` - The wait bonus to apply
/// * `apply_fn` - Function that updates the session with the bonus
///
/// # Returns
///
/// A tuple of (updated_session, generated_events).
///
/// # Examples
///
/// ```ignore
/// let bonus = WaitBonus::default_bonus();
/// let (updated, events) = apply_wait_bonus(
///     &session,
///     &bonus,
///     |s, b| {
///         // Apply HP regen
///         let new_hp = s.player_health().add(b.health_regeneration());
///         s.with_player_health(new_hp)
///     },
/// );
/// ```
pub fn apply_wait_bonus<S, F>(
    session: &S,
    bonus: &WaitBonus,
    apply_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, &WaitBonus) -> S,
{
    let updated_session = apply_fn(session, bonus);
    // Note: We could generate a PlayerWaited event here when it's added to GameSessionEvent
    let events = Vec::new();
    (updated_session, events)
}

/// Calculates the HP regeneration amount for a wait turn.
///
/// This is a pure function that determines how much HP the player
/// should regenerate based on their current state.
///
/// # Arguments
///
/// * `current_health` - The player's current health
/// * `max_health` - The player's maximum health
/// * `base_regeneration` - The base HP regeneration amount
///
/// # Returns
///
/// The amount of HP to regenerate (clamped to not exceed max health).
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::calculate_hp_regeneration;
/// use roguelike_domain::common::Health;
///
/// let current = Health::new(50).unwrap();
/// let max = Health::new(100).unwrap();
/// let regen = calculate_hp_regeneration(&current, &max, 5);
///
/// assert_eq!(regen, 5);
/// ```
#[must_use]
pub fn calculate_hp_regeneration(
    current_health: &Health,
    max_health: &Health,
    base_regeneration: u32,
) -> u32 {
    let missing_hp = max_health.value().saturating_sub(current_health.value());
    base_regeneration.min(missing_hp)
}

/// Checks if the player can benefit from waiting.
///
/// This is a pure function that determines if there's any
/// benefit to waiting (e.g., player is not at full HP).
///
/// # Arguments
///
/// * `current_health` - The player's current health
/// * `max_health` - The player's maximum health
///
/// # Returns
///
/// `true` if the player can benefit from waiting, `false` otherwise.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::can_benefit_from_wait;
/// use roguelike_domain::common::Health;
///
/// let damaged = Health::new(50).unwrap();
/// let max = Health::new(100).unwrap();
/// assert!(can_benefit_from_wait(&damaged, &max));
///
/// let full = Health::new(100).unwrap();
/// assert!(!can_benefit_from_wait(&full, &max));
/// ```
#[must_use]
pub const fn can_benefit_from_wait(current_health: &Health, max_health: &Health) -> bool {
    current_health.value() < max_health.value()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // WaitBonus Tests
    // =========================================================================

    mod wait_bonus {
        use super::*;

        #[rstest]
        fn new_creates_bonus() {
            let bonus = WaitBonus::new(5);
            assert_eq!(bonus.health_regeneration(), 5);
        }

        #[rstest]
        fn default_bonus_has_correct_value() {
            let bonus = WaitBonus::default_bonus();
            assert_eq!(bonus.health_regeneration(), WAIT_HP_REGENERATION);
        }

        #[rstest]
        fn default_trait_matches_default_bonus() {
            let default = WaitBonus::default();
            let bonus = WaitBonus::default_bonus();
            assert_eq!(default, bonus);
        }

        #[rstest]
        fn equality() {
            let bonus1 = WaitBonus::new(5);
            let bonus2 = WaitBonus::new(5);
            let bonus3 = WaitBonus::new(10);

            assert_eq!(bonus1, bonus2);
            assert_ne!(bonus1, bonus3);
        }

        #[rstest]
        fn clone() {
            let bonus = WaitBonus::new(10);
            let cloned = bonus;
            assert_eq!(bonus, cloned);
        }

        #[rstest]
        fn debug_format() {
            let bonus = WaitBonus::new(5);
            let debug = format!("{:?}", bonus);
            assert!(debug.contains("WaitBonus"));
            assert!(debug.contains("5"));
        }
    }

    // =========================================================================
    // apply_wait_bonus Tests
    // =========================================================================

    mod apply_wait_bonus_tests {
        use super::*;

        #[rstest]
        fn applies_bonus_to_session() {
            let session = 0i32;
            let bonus = WaitBonus::new(10);

            let (updated, _events) =
                apply_wait_bonus(&session, &bonus, |s, b| s + b.health_regeneration() as i32);

            assert_eq!(updated, 10);
        }

        #[rstest]
        fn returns_empty_events() {
            let session = String::from("session");
            let bonus = WaitBonus::default();

            let (_, events) = apply_wait_bonus(&session, &bonus, |s, _| s.clone());

            assert!(events.is_empty());
        }

        #[rstest]
        fn passes_bonus_to_function() {
            let session = 0u32;
            let bonus = WaitBonus::new(42);

            let (updated, _) = apply_wait_bonus(&session, &bonus, |s, b| {
                // The bonus is passed to the function
                s + b.health_regeneration()
            });

            assert_eq!(updated, 42);
        }
    }

    // =========================================================================
    // calculate_hp_regeneration Tests
    // =========================================================================

    mod calculate_hp_regeneration_tests {
        use super::*;

        #[rstest]
        fn returns_base_when_missing_more() {
            let current = Health::new(50).unwrap();
            let max = Health::new(100).unwrap();

            let regen = calculate_hp_regeneration(&current, &max, 5);

            assert_eq!(regen, 5);
        }

        #[rstest]
        fn clamps_to_missing_hp() {
            let current = Health::new(98).unwrap();
            let max = Health::new(100).unwrap();

            let regen = calculate_hp_regeneration(&current, &max, 5);

            assert_eq!(regen, 2); // Only 2 HP missing
        }

        #[rstest]
        fn returns_zero_at_full_hp() {
            let current = Health::new(100).unwrap();
            let max = Health::new(100).unwrap();

            let regen = calculate_hp_regeneration(&current, &max, 5);

            assert_eq!(regen, 0);
        }

        #[rstest]
        fn handles_zero_base_regen() {
            let current = Health::new(50).unwrap();
            let max = Health::new(100).unwrap();

            let regen = calculate_hp_regeneration(&current, &max, 0);

            assert_eq!(regen, 0);
        }

        #[rstest]
        fn handles_low_current_health() {
            let current = Health::new(1).unwrap();
            let max = Health::new(100).unwrap();

            let regen = calculate_hp_regeneration(&current, &max, 10);

            assert_eq!(regen, 10);
        }
    }

    // =========================================================================
    // can_benefit_from_wait Tests
    // =========================================================================

    mod can_benefit_from_wait_tests {
        use super::*;

        #[rstest]
        fn returns_true_when_damaged() {
            let current = Health::new(50).unwrap();
            let max = Health::new(100).unwrap();

            assert!(can_benefit_from_wait(&current, &max));
        }

        #[rstest]
        fn returns_false_at_full_hp() {
            let current = Health::new(100).unwrap();
            let max = Health::new(100).unwrap();

            assert!(!can_benefit_from_wait(&current, &max));
        }

        #[rstest]
        fn returns_true_when_one_hp_missing() {
            let current = Health::new(99).unwrap();
            let max = Health::new(100).unwrap();

            assert!(can_benefit_from_wait(&current, &max));
        }

        #[rstest]
        fn returns_true_at_one_hp() {
            let current = Health::new(1).unwrap();
            let max = Health::new(100).unwrap();

            assert!(can_benefit_from_wait(&current, &max));
        }
    }
}
