//! MovePlayer workflow implementation.
//!
//! This module provides the workflow for moving the player in the dungeon.
//! It follows the "IO at the Edges" pattern, separating pure domain logic
//! from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Calculate new position
//! 3. [Pure] Validate movement (bounds, walkability)
//! 4. [Pure] Update player position
//! 5. [Pure] Generate PlayerMoved event
//! 6. [IO] Update cache
//! 7. [IO] Append events to event store
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::player::{move_player, MovePlayerCommand};
//! use roguelike_domain::common::Direction;
//!
//! let workflow = move_player(&cache, &event_store);
//! let command = MovePlayerCommand::new(game_identifier, Direction::Up);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use roguelike_domain::common::{Direction, Position};
use roguelike_domain::floor::FloorError;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::player::PlayerMoved;

use super::MovePlayerCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// MovePlayer Workflow
// =============================================================================

/// Creates a workflow function for moving the player.
///
/// This function returns a closure that moves the player in a direction.
/// It uses higher-order functions to inject dependencies, enabling pure
/// functional composition and easy testing.
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
/// A function that takes a `MovePlayerCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::workflows::player::{move_player, MovePlayerCommand};
/// use roguelike_domain::common::Direction;
///
/// let workflow = move_player(&cache, &event_store);
/// let command = MovePlayerCommand::new(game_identifier, Direction::Up);
/// let result = workflow(command).run_async().await;
/// ```
pub fn move_player<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(MovePlayerCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let direction = command.direction();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Step 2-5: [Pure] Calculate position, validate, update, generate events
                    let result = move_player_pure(&session, direction);

                    match result {
                        Ok((updated_session, events)) => {
                            // Step 6-7: [IO] Update cache and append events
                            let game_identifier_clone = game_identifier;
                            let updated_session_clone = updated_session.clone();

                            cache
                                .set(
                                    &game_identifier_clone,
                                    &updated_session,
                                    DEFAULT_CACHE_TIME_TO_LIVE,
                                )
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

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the move player logic.
///
/// This function encapsulates all pure domain logic for moving the player:
/// - Calculates new position
/// - Validates movement
/// - Updates player position
/// - Generates events
///
/// Note: This is a placeholder implementation. The actual implementation
/// depends on the GameSession structure which is defined by the
/// repository/cache implementation.
///
/// # Arguments
///
/// * `session` - The current game session (must implement position access)
/// * `direction` - The direction to move
///
/// # Returns
///
/// A result containing the updated session and events, or an error.
fn move_player_pure<S: Clone>(
    _session: &S,
    _direction: Direction,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Note: This is a placeholder implementation.
    // In a real implementation, we would use pipe! for composition:
    //
    // pipe!(
    //     session,
    //     |s| get_player_position(&s),
    //     |pos| calculate_new_position(pos, direction),
    //     |new_pos| validate_movement(&session.current_floor, new_pos),
    //     |validated_pos| update_session_with_new_position(session, validated_pos)
    // )
    //
    // For now, we return an error indicating this needs to be connected
    // to the actual game session structure
    Err(WorkflowError::repository(
        "move_player",
        "GameSession structure not yet connected",
    ))
}

/// Movement calculation pipeline using `pipe!` macro.
///
/// This is a pure function that demonstrates how to compose movement logic
/// with the `pipe!` macro. It calculates and validates a new position.
///
/// # Arguments
///
/// * `current_position` - The current position
/// * `direction` - The direction to move
/// * `floor_bounds` - The floor dimensions (width, height)
/// * `is_walkable` - A function that checks if a position is walkable
///
/// # Returns
///
/// `Ok((from, to))` if movement is valid, or an error.
///
/// # Example
///
/// ```ignore
/// let result = calculate_movement_pipeline(
///     current_pos,
///     Direction::Up,
///     (80, 40),
///     |pos| floor.is_walkable(pos),
/// );
/// ```
pub fn calculate_movement_pipeline<F>(
    current_position: Position,
    direction: Direction,
    floor_bounds: (u32, u32),
    is_walkable: F,
) -> Result<(Position, Position), FloorError>
where
    F: Fn(Position) -> bool,
{
    // [Pure] Movement calculation pipeline using pipe!
    pipe!(
        current_position,
        // Step 1: Calculate new position
        |pos| calculate_new_position(pos, direction),
        // Step 2: Validate movement and return both positions
        |new_pos| {
            validate_movement(floor_bounds, new_pos, is_walkable)
                .map(|()| (current_position, new_pos))
        }
    )
}

/// Calculates the new position after moving in a direction.
///
/// This is a pure function that determines the target position.
///
/// # Arguments
///
/// * `current` - The current position
/// * `direction` - The direction to move
///
/// # Returns
///
/// The new position after moving one step in the given direction.
#[must_use]
pub fn calculate_new_position(current: Position, direction: Direction) -> Position {
    current.move_toward(direction)
}

/// Validates that a movement is legal.
///
/// This is a pure function that checks:
/// - The target position is within floor bounds
/// - The target tile is walkable
///
/// # Arguments
///
/// * `floor_bounds` - The floor dimensions (width, height)
/// * `target` - The target position to move to
/// * `is_walkable` - A function that checks if a position is walkable
///
/// # Returns
///
/// `Ok(())` if the movement is valid, or an appropriate `FloorError`.
pub fn validate_movement<F>(
    floor_bounds: (u32, u32),
    target: Position,
    is_walkable: F,
) -> Result<(), FloorError>
where
    F: Fn(Position) -> bool,
{
    // Check bounds
    if target.x() < 0
        || target.y() < 0
        || target.x() >= floor_bounds.0 as i32
        || target.y() >= floor_bounds.1 as i32
    {
        return Err(FloorError::position_out_of_bounds(
            (target.x(), target.y()),
            floor_bounds,
        ));
    }

    // Check walkability
    if !is_walkable(target) {
        return Err(FloorError::tile_not_walkable(
            (target.x(), target.y()),
            "Wall",
        ));
    }

    Ok(())
}

/// Creates a PlayerMoved event.
///
/// This is a pure function that generates the domain event.
///
/// # Arguments
///
/// * `from` - The position moved from
/// * `to` - The position moved to
///
/// # Returns
///
/// A `PlayerMoved` event.
#[must_use]
pub fn create_player_moved_event(from: Position, to: Position) -> PlayerMoved {
    PlayerMoved::new(from, to)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Pure Function Tests
    // =========================================================================

    mod calculate_new_position_tests {
        use super::*;

        #[rstest]
        #[case(Position::new(5, 5), Direction::Up, Position::new(5, 4))]
        #[case(Position::new(5, 5), Direction::Down, Position::new(5, 6))]
        #[case(Position::new(5, 5), Direction::Left, Position::new(4, 5))]
        #[case(Position::new(5, 5), Direction::Right, Position::new(6, 5))]
        fn calculates_new_position_correctly(
            #[case] current: Position,
            #[case] direction: Direction,
            #[case] expected: Position,
        ) {
            let result = calculate_new_position(current, direction);
            assert_eq!(result, expected);
        }

        #[rstest]
        fn handles_origin() {
            let result = calculate_new_position(Position::new(0, 0), Direction::Up);
            assert_eq!(result, Position::new(0, -1));
        }

        #[rstest]
        fn handles_negative_positions() {
            let result = calculate_new_position(Position::new(-5, -5), Direction::Left);
            assert_eq!(result, Position::new(-6, -5));
        }
    }

    mod validate_movement_tests {
        use super::*;

        #[rstest]
        fn valid_movement_returns_ok() {
            let floor_bounds = (80, 40);
            let target = Position::new(10, 10);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(result.is_ok());
        }

        #[rstest]
        fn out_of_bounds_negative_x_returns_error() {
            let floor_bounds = (80, 40);
            let target = Position::new(-1, 10);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(matches!(
                result,
                Err(FloorError::PositionOutOfBounds { .. })
            ));
        }

        #[rstest]
        fn out_of_bounds_negative_y_returns_error() {
            let floor_bounds = (80, 40);
            let target = Position::new(10, -1);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(matches!(
                result,
                Err(FloorError::PositionOutOfBounds { .. })
            ));
        }

        #[rstest]
        fn out_of_bounds_exceeds_width_returns_error() {
            let floor_bounds = (80, 40);
            let target = Position::new(80, 10);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(matches!(
                result,
                Err(FloorError::PositionOutOfBounds { .. })
            ));
        }

        #[rstest]
        fn out_of_bounds_exceeds_height_returns_error() {
            let floor_bounds = (80, 40);
            let target = Position::new(10, 40);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(matches!(
                result,
                Err(FloorError::PositionOutOfBounds { .. })
            ));
        }

        #[rstest]
        fn non_walkable_tile_returns_error() {
            let floor_bounds = (80, 40);
            let target = Position::new(10, 10);
            let is_walkable = |_: Position| false;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(matches!(result, Err(FloorError::TileNotWalkable { .. })));
        }

        #[rstest]
        fn edge_position_is_valid() {
            let floor_bounds = (80, 40);
            let target = Position::new(79, 39);
            let is_walkable = |_: Position| true;

            let result = validate_movement(floor_bounds, target, is_walkable);
            assert!(result.is_ok());
        }
    }

    mod create_player_moved_event_tests {
        use super::*;

        #[rstest]
        fn creates_event_with_positions() {
            let from = Position::new(5, 5);
            let to = Position::new(6, 5);

            let event = create_player_moved_event(from, to);

            assert_eq!(event.from(), from);
            assert_eq!(event.to(), to);
        }
    }
}
