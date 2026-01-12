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

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// EnemyAction
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAction {
    Move(Direction),
    Attack(EntityIdentifier),
    Wait,
}

impl EnemyAction {
    #[must_use]
    pub const fn is_move(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    #[must_use]
    pub const fn is_attack(&self) -> bool {
        matches!(self, Self::Attack(_))
    }

    #[must_use]
    pub const fn is_wait(&self) -> bool {
        matches!(self, Self::Wait)
    }
}

// =============================================================================
// ProcessEnemyTurn Workflow
// =============================================================================

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
