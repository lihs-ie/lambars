//! ProcessEnemyTurn workflow implementation.
//!
//! This module provides the workflow for processing an enemy's turn.
//! It follows the "IO at the Edges" pattern, separating pure domain logic
//! from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Find enemy by identifier
//! 3. [Pure] Validate enemy is active (alive)
//! 4. [Pure] Decide enemy action based on AI behavior
//! 5. [Pure] Execute the decided action
//! 6. [Pure] Generate enemy events
//! 7. [IO] Update cache
//! 8. [IO] Append events to event store
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::enemy::{process_enemy_turn, ProcessEnemyTurnCommand};
//!
//! let workflow = process_enemy_turn(&cache, &event_store, cache_ttl);
//! let command = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::{Direction, Position};
use roguelike_domain::enemy::{
    AiBehavior, EnemyAttacked, EnemyError, EnemyMoved, EntityIdentifier,
};
use roguelike_domain::game_session::GameSessionEvent;

use super::ProcessEnemyTurnCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// EnemyAction
// =============================================================================

/// Represents an action that an enemy can perform during its turn.
///
/// This enum is used by the AI decision-making system to determine
/// what action an enemy should take.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::EnemyAction;
/// use roguelike_domain::common::Direction;
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let move_action = EnemyAction::Move(Direction::Up);
/// let attack_action = EnemyAction::Attack(EntityIdentifier::new());
/// let wait_action = EnemyAction::Wait;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAction {
    /// Move in a direction.
    Move(Direction),
    /// Attack a target entity.
    Attack(EntityIdentifier),
    /// Wait (skip turn).
    Wait,
}

impl EnemyAction {
    /// Returns true if this is a move action.
    #[must_use]
    pub const fn is_move(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    /// Returns true if this is an attack action.
    #[must_use]
    pub const fn is_attack(&self) -> bool {
        matches!(self, Self::Attack(_))
    }

    /// Returns true if this is a wait action.
    #[must_use]
    pub const fn is_wait(&self) -> bool {
        matches!(self, Self::Wait)
    }
}

// =============================================================================
// ProcessEnemyTurn Workflow
// =============================================================================

/// Creates a workflow function for processing an enemy's turn.
///
/// This function returns a closure that processes an enemy's AI decision
/// and executes the resulting action. It uses higher-order functions to
/// inject dependencies, enabling pure functional composition and easy testing.
///
/// # Type Parameters
///
/// * `C` - Cache type implementing `SessionCache`
/// * `E` - Event store type implementing `EventStore`
///
/// # Arguments
///
/// * `cache` - The session cache for fast access
/// * `event_store` - The event store for event sourcing
/// * `cache_ttl` - Time-to-live for cached sessions
///
/// # Returns
///
/// A function that takes a `ProcessEnemyTurnCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::workflows::enemy::{process_enemy_turn, ProcessEnemyTurnCommand};
///
/// let workflow = process_enemy_turn(&cache, &event_store, Duration::from_secs(300));
/// let command = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);
/// let result = workflow(command).run_async().await;
/// ```
pub fn process_enemy_turn<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(ProcessEnemyTurnCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let entity_identifier = *command.entity_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Steps 2-6: [Pure] Process enemy turn
                    let result = process_enemy_turn_pure(&session, entity_identifier);

                    match result {
                        Ok((updated_session, events)) => {
                            // Steps 7-8: [IO] Update cache and append events
                            let game_identifier_clone = game_identifier;
                            let updated_session_clone = updated_session.clone();

                            cache
                                .set(&game_identifier_clone, &updated_session, cache_ttl)
                                .flat_map(move |()| {
                                    event_store
                                        .append(&game_identifier_clone, &events)
                                        .fmap(move |()| Ok(updated_session_clone))
                                })
                        }
                        Err(error) => AsyncIO::pure(Err(error)),
                    }
                }
                None => AsyncIO::pure(Err(WorkflowError::not_found(
                    "GameSession",
                    game_identifier.to_string(),
                ))),
            }
        })
    }
}

/// Creates a workflow function with default cache TTL.
///
/// This is a convenience function that uses the default cache time-to-live.
///
/// # Type Parameters
///
/// * `C` - Cache type implementing `SessionCache`
/// * `E` - Event store type implementing `EventStore`
///
/// # Arguments
///
/// * `cache` - The session cache for fast access
/// * `event_store` - The event store for event sourcing
///
/// # Returns
///
/// A function that takes a `ProcessEnemyTurnCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
pub fn process_enemy_turn_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(ProcessEnemyTurnCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    process_enemy_turn(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire enemy turn processing logic.
///
/// This function encapsulates all pure domain logic for processing an enemy turn:
/// - Find enemy by identifier
/// - Validate enemy is active
/// - Decide action using AI
/// - Execute action
/// - Generate events
///
/// # Arguments
///
/// * `session` - The current game session
/// * `entity_identifier` - The identifier of the enemy to process
///
/// # Returns
///
/// A result containing the updated session and events, or an error.
fn process_enemy_turn_pure<S: Clone>(
    _session: &S,
    _entity_identifier: EntityIdentifier,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Note: This is a placeholder implementation.
    // The actual implementation depends on the GameSession structure
    // which is defined by the repository/cache implementation.
    //
    // In a real implementation, we would:
    // 1. Find the enemy in the session by identifier
    // 2. Validate the enemy is active (alive)
    // 3. Get the player position for AI decision making
    // 4. Decide action based on behavior and game state
    // 5. Execute the action and update the session
    // 6. Generate appropriate events

    Err(WorkflowError::repository(
        "process_enemy_turn",
        "GameSession structure not yet connected",
    ))
}

/// Finds an enemy by its identifier in the game state.
///
/// This is a pure function that searches for an enemy in the session.
///
/// # Type Parameters
///
/// * `S` - The session type (must provide enemy access)
/// * `Enemy` - The enemy type
///
/// # Arguments
///
/// * `session` - The current game session
/// * `entity_identifier` - The identifier of the enemy to find
///
/// # Returns
///
/// `Some(enemy)` if found, `None` otherwise.
///
/// # Examples
///
/// ```ignore
/// let enemy = find_enemy_by_id(&session, entity_identifier);
/// match enemy {
///     Some(e) => println!("Found enemy at {:?}", e.position()),
///     None => println!("Enemy not found"),
/// }
/// ```
#[must_use]
pub fn find_enemy_by_id<S, Enemy>(
    _session: &S,
    _entity_identifier: EntityIdentifier,
) -> Option<Enemy>
where
    S: Clone,
    Enemy: Clone,
{
    // Placeholder: actual implementation depends on GameSession structure
    None
}

/// Validates that an enemy is active (alive).
///
/// This is a pure function that checks if an enemy can take actions.
///
/// # Arguments
///
/// * `enemy_identifier` - The identifier of the enemy
/// * `is_alive` - Whether the enemy is alive
///
/// # Returns
///
/// `Ok(())` if the enemy is active, `Err(EnemyError)` if dead.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::validate_enemy_active;
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let identifier = EntityIdentifier::new();
///
/// // Alive enemy passes validation
/// assert!(validate_enemy_active(identifier, true).is_ok());
///
/// // Dead enemy fails validation
/// assert!(validate_enemy_active(identifier, false).is_err());
/// ```
pub fn validate_enemy_active(
    enemy_identifier: EntityIdentifier,
    is_alive: bool,
) -> Result<(), EnemyError> {
    if is_alive {
        Ok(())
    } else {
        Err(EnemyError::enemy_already_dead(enemy_identifier.to_string()))
    }
}

/// Decides what action an enemy should take based on its AI behavior.
///
/// This is a pure function that implements the AI decision-making logic.
/// The decision is based on:
/// - The enemy's behavior pattern
/// - The distance and direction to the player
/// - Whether the enemy can attack
///
/// # Arguments
///
/// * `behavior` - The AI behavior pattern of the enemy
/// * `enemy_position` - The current position of the enemy
/// * `player_position` - The current position of the player
/// * `can_attack` - Whether the enemy is adjacent to the player
/// * `player_entity_identifier` - The player's entity identifier (for attack target)
///
/// # Returns
///
/// The action the enemy should take.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::{decide_enemy_action, EnemyAction};
/// use roguelike_domain::enemy::{AiBehavior, EntityIdentifier};
/// use roguelike_domain::common::Position;
///
/// let enemy_position = Position::new(5, 5);
/// let player_position = Position::new(7, 5);
/// let player_id = EntityIdentifier::new();
///
/// // Aggressive enemy moves toward player
/// let action = decide_enemy_action(
///     AiBehavior::Aggressive,
///     enemy_position,
///     player_position,
///     false,
///     player_id,
/// );
/// assert!(action.is_move());
/// ```
#[must_use]
pub fn decide_enemy_action(
    behavior: AiBehavior,
    enemy_position: Position,
    player_position: Position,
    can_attack: bool,
    player_entity_identifier: EntityIdentifier,
) -> EnemyAction {
    // If can attack and behavior allows combat initiation, attack
    if can_attack && behavior.initiates_combat() {
        return EnemyAction::Attack(player_entity_identifier);
    }

    // Calculate direction based on behavior
    let direction = calculate_movement_direction(behavior, enemy_position, player_position);

    match direction {
        Some(dir) => EnemyAction::Move(dir),
        None => EnemyAction::Wait,
    }
}

/// Calculates the movement direction based on behavior and positions.
///
/// # Arguments
///
/// * `behavior` - The AI behavior pattern
/// * `enemy_position` - The enemy's current position
/// * `player_position` - The player's current position
///
/// # Returns
///
/// `Some(Direction)` if the enemy should move, `None` if it should wait.
fn calculate_movement_direction(
    behavior: AiBehavior,
    enemy_position: Position,
    player_position: Position,
) -> Option<Direction> {
    let delta_x = player_position.x() - enemy_position.x();
    let delta_y = player_position.y() - enemy_position.y();

    match behavior {
        AiBehavior::Aggressive => {
            // Move toward player
            direction_toward(delta_x, delta_y)
        }
        AiBehavior::Flee => {
            // Move away from player
            direction_toward(-delta_x, -delta_y)
        }
        AiBehavior::Defensive | AiBehavior::Passive => {
            // Stay in place
            None
        }
        AiBehavior::Patrol => {
            // For patrol, simplified: move toward player if in range
            if delta_x.abs() <= 3 && delta_y.abs() <= 3 {
                direction_toward(delta_x, delta_y)
            } else {
                None
            }
        }
    }
}

/// Determines a single direction to move based on delta coordinates.
///
/// Prioritizes horizontal movement over vertical.
fn direction_toward(delta_x: i32, delta_y: i32) -> Option<Direction> {
    if delta_x > 0 {
        Some(Direction::Right)
    } else if delta_x < 0 {
        Some(Direction::Left)
    } else if delta_y > 0 {
        Some(Direction::Down)
    } else if delta_y < 0 {
        Some(Direction::Up)
    } else {
        None
    }
}

/// Executes an enemy action and returns the result.
///
/// This is a pure function that applies an action to the game state.
///
/// # Type Parameters
///
/// * `S` - The session type
///
/// # Arguments
///
/// * `session` - The current game session
/// * `enemy_identifier` - The identifier of the acting enemy
/// * `enemy_position` - The current position of the enemy
/// * `action` - The action to execute
///
/// # Returns
///
/// A result containing the updated session and generated events.
///
/// # Examples
///
/// ```ignore
/// let (updated_session, events) = execute_enemy_action(
///     &session,
///     enemy_identifier,
///     enemy_position,
///     EnemyAction::Move(Direction::Up),
/// )?;
/// ```
pub fn execute_enemy_action<S: Clone>(
    session: &S,
    enemy_identifier: EntityIdentifier,
    enemy_position: Position,
    action: EnemyAction,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    match action {
        EnemyAction::Move(direction) => {
            execute_move(session, enemy_identifier, enemy_position, direction)
        }
        EnemyAction::Attack(target) => execute_attack(session, enemy_identifier, target),
        EnemyAction::Wait => {
            // No state change, no events
            Ok((session.clone(), Vec::new()))
        }
    }
}

/// Executes a move action.
fn execute_move<S: Clone>(
    session: &S,
    enemy_identifier: EntityIdentifier,
    enemy_position: Position,
    direction: Direction,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    let new_position = enemy_position.move_toward(direction);

    // In a real implementation, we would:
    // 1. Validate the new position is walkable
    // 2. Update the enemy position in the session
    // 3. Generate the EnemyMoved event

    let event = EnemyMoved::new(enemy_identifier, enemy_position, new_position);

    // Placeholder: return unchanged session with event
    // Real implementation would update the session
    Ok((session.clone(), vec![GameSessionEvent::EnemyMoved(event)]))
}

/// Executes an attack action.
fn execute_attack<S: Clone>(
    session: &S,
    enemy_identifier: EntityIdentifier,
    _target: EntityIdentifier,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // In a real implementation, we would:
    // 1. Calculate damage based on enemy stats
    // 2. Apply damage to target (player)
    // 3. Generate the appropriate events

    let damage = roguelike_domain::common::Damage::new(10); // Placeholder damage
    let event = EnemyAttacked::new(enemy_identifier, damage);

    // Placeholder: return unchanged session with event
    // Real implementation would update the session
    Ok((
        session.clone(),
        vec![GameSessionEvent::EnemyAttacked(event)],
    ))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // EnemyAction Tests
    // =========================================================================

    mod enemy_action {
        use super::*;

        #[rstest]
        fn move_action_is_move() {
            let action = EnemyAction::Move(Direction::Up);
            assert!(action.is_move());
            assert!(!action.is_attack());
            assert!(!action.is_wait());
        }

        #[rstest]
        fn attack_action_is_attack() {
            let action = EnemyAction::Attack(EntityIdentifier::new());
            assert!(!action.is_move());
            assert!(action.is_attack());
            assert!(!action.is_wait());
        }

        #[rstest]
        fn wait_action_is_wait() {
            let action = EnemyAction::Wait;
            assert!(!action.is_move());
            assert!(!action.is_attack());
            assert!(action.is_wait());
        }

        #[rstest]
        #[case(Direction::Up)]
        #[case(Direction::Down)]
        #[case(Direction::Left)]
        #[case(Direction::Right)]
        fn move_action_with_all_directions(#[case] direction: Direction) {
            let action = EnemyAction::Move(direction);
            assert!(action.is_move());
        }

        #[rstest]
        fn equality() {
            let action1 = EnemyAction::Move(Direction::Up);
            let action2 = EnemyAction::Move(Direction::Up);
            let action3 = EnemyAction::Move(Direction::Down);

            assert_eq!(action1, action2);
            assert_ne!(action1, action3);
        }

        #[rstest]
        fn clone() {
            let action = EnemyAction::Move(Direction::Up);
            let cloned = action;
            assert_eq!(action, cloned);
        }

        #[rstest]
        fn debug_format() {
            let action = EnemyAction::Move(Direction::Up);
            let debug = format!("{:?}", action);
            assert!(debug.contains("Move"));
            assert!(debug.contains("Up"));
        }
    }

    // =========================================================================
    // validate_enemy_active Tests
    // =========================================================================

    mod validate_enemy_active_tests {
        use super::*;

        #[rstest]
        fn alive_enemy_passes_validation() {
            let identifier = EntityIdentifier::new();
            let result = validate_enemy_active(identifier, true);
            assert!(result.is_ok());
        }

        #[rstest]
        fn dead_enemy_fails_validation() {
            let identifier = EntityIdentifier::new();
            let result = validate_enemy_active(identifier, false);
            assert!(result.is_err());
            assert!(matches!(result, Err(EnemyError::EnemyAlreadyDead { .. })));
        }
    }

    // =========================================================================
    // decide_enemy_action Tests
    // =========================================================================

    mod decide_enemy_action_tests {
        use super::*;

        #[rstest]
        fn aggressive_enemy_attacks_when_adjacent() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(6, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Aggressive,
                enemy_position,
                player_position,
                true, // can attack
                player_id,
            );

            assert_eq!(action, EnemyAction::Attack(player_id));
        }

        #[rstest]
        fn aggressive_enemy_moves_toward_player_when_not_adjacent() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(10, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Aggressive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Move(Direction::Right));
        }

        #[rstest]
        fn flee_enemy_moves_away_from_player() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(7, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Flee,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Move(Direction::Left));
        }

        #[rstest]
        fn passive_enemy_waits() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(10, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Passive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Wait);
        }

        #[rstest]
        fn defensive_enemy_waits() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(10, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Defensive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Wait);
        }

        #[rstest]
        fn patrol_enemy_moves_toward_player_when_in_range() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(7, 5); // 2 tiles away
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Patrol,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Move(Direction::Right));
        }

        #[rstest]
        fn patrol_enemy_waits_when_player_out_of_range() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(15, 5); // 10 tiles away
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Patrol,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Wait);
        }

        #[rstest]
        fn patrol_enemy_attacks_when_adjacent() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(6, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Patrol,
                enemy_position,
                player_position,
                true,
                player_id,
            );

            assert_eq!(action, EnemyAction::Attack(player_id));
        }

        #[rstest]
        fn movement_prioritizes_horizontal() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(10, 10); // Both x and y different
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Aggressive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            // Should prioritize horizontal (Right) over vertical (Down)
            assert_eq!(action, EnemyAction::Move(Direction::Right));
        }

        #[rstest]
        fn movement_uses_vertical_when_same_x() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(5, 10);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Aggressive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Move(Direction::Down));
        }

        #[rstest]
        fn same_position_results_in_wait() {
            let enemy_position = Position::new(5, 5);
            let player_position = Position::new(5, 5);
            let player_id = EntityIdentifier::new();

            let action = decide_enemy_action(
                AiBehavior::Aggressive,
                enemy_position,
                player_position,
                false,
                player_id,
            );

            assert_eq!(action, EnemyAction::Wait);
        }
    }

    // =========================================================================
    // execute_enemy_action Tests
    // =========================================================================

    mod execute_enemy_action_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession;

        #[rstest]
        fn execute_move_generates_enemy_moved_event() {
            let session = MockSession;
            let enemy_identifier = EntityIdentifier::new();
            let enemy_position = Position::new(5, 5);
            let action = EnemyAction::Move(Direction::Up);

            let result = execute_enemy_action(&session, enemy_identifier, enemy_position, action);

            assert!(result.is_ok());
            let (_, events) = result.unwrap();
            assert_eq!(events.len(), 1);
            assert!(matches!(events[0], GameSessionEvent::EnemyMoved(_)));
        }

        #[rstest]
        fn execute_attack_generates_enemy_attacked_event() {
            let session = MockSession;
            let enemy_identifier = EntityIdentifier::new();
            let enemy_position = Position::new(5, 5);
            let target = EntityIdentifier::new();
            let action = EnemyAction::Attack(target);

            let result = execute_enemy_action(&session, enemy_identifier, enemy_position, action);

            assert!(result.is_ok());
            let (_, events) = result.unwrap();
            assert_eq!(events.len(), 1);
            assert!(matches!(events[0], GameSessionEvent::EnemyAttacked(_)));
        }

        #[rstest]
        fn execute_wait_generates_no_events() {
            let session = MockSession;
            let enemy_identifier = EntityIdentifier::new();
            let enemy_position = Position::new(5, 5);
            let action = EnemyAction::Wait;

            let result = execute_enemy_action(&session, enemy_identifier, enemy_position, action);

            assert!(result.is_ok());
            let (_, events) = result.unwrap();
            assert!(events.is_empty());
        }
    }
}
