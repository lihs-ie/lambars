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

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// TurnResult
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnResult<S> {
    pub session: S,
    pub game_over: Option<GameOutcome>,
}

impl<S> TurnResult<S> {
    #[must_use]
    pub const fn continuing(session: S) -> Self {
        Self {
            session,
            game_over: None,
        }
    }

    #[must_use]
    pub const fn game_ended(session: S, outcome: GameOutcome) -> Self {
        Self {
            session,
            game_over: Some(outcome),
        }
    }

    #[must_use]
    pub const fn is_game_over(&self) -> bool {
        self.game_over.is_some()
    }

    #[must_use]
    pub const fn is_continuing(&self) -> bool {
        self.game_over.is_none()
    }
}

// =============================================================================
// EntityTurnOrder
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTurnOrder {
    identifier: EntityIdentifier,
    speed: Speed,
}

impl EntityTurnOrder {
    #[must_use]
    pub const fn new(identifier: EntityIdentifier, speed: Speed) -> Self {
        Self { identifier, speed }
    }

    #[must_use]
    pub const fn identifier(&self) -> EntityIdentifier {
        self.identifier
    }

    #[must_use]
    pub const fn speed(&self) -> Speed {
        self.speed
    }
}

// =============================================================================
// Step 1: Extract Turn Parameters [Pure]
// =============================================================================

fn extract_turn_params(command: ProcessTurnCommand) -> (GameIdentifier, PlayerCommand) {
    (*command.game_identifier(), command.player_command())
}

// =============================================================================
// Step 2: Load Session from Cache [IO]
// =============================================================================

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
                .ok_or_else(|| WorkflowError::not_found("GameSession", game_identifier.to_string()))
        })
    }
}

// =============================================================================
// Step 3: Process Turn Result [Pure]
// =============================================================================

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

#[allow(clippy::type_complexity)]
fn persist_turn_result<C: SessionCache, E: EventStore>(
    cache: C,
    event_store: E,
    cache_ttl: Duration,
) -> impl Fn(
    Result<
        (
            TurnResult<C::GameSession>,
            Vec<GameSessionEvent>,
            GameIdentifier,
        ),
        WorkflowError,
    >,
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

#[must_use]
pub fn resolve_turn_order(entities: &[EntityTurnOrder]) -> Vec<EntityTurnOrder> {
    let mut sorted = entities.to_vec();
    sorted.sort_by_key(|entity| std::cmp::Reverse(entity.speed().value()));
    sorted
}

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

pub fn process_status_effects<S, F>(session: &S, process_fn: F) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S) -> (S, Vec<GameSessionEvent>),
{
    process_fn(session)
}

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

pub fn end_turn<S, F>(session: &S, current_turn: TurnCount, finalize_fn: F) -> (S, TurnEnded)
where
    S: Clone,
    F: Fn(&S) -> S,
{
    let updated_session = finalize_fn(session);
    let event = TurnEnded::new(current_turn);
    (updated_session, event)
}

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
