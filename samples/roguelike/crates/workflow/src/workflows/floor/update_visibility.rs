//! UpdateVisibility workflow implementation.
//!
//! This module provides the workflow for updating tile visibility based on
//! the player's field of view. It uses an iterative approach for
//! calculating visible tiles.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Get player position
//! 3. [Pure] Calculate field of view (visible tiles)
//! 4. [Pure] Update explored tiles
//! 5. [Pure] Update session visibility
//! 6. [Pure] Generate TileExplored events
//! 7. [IO] Update cache
//! 8. [IO] Append events to event store
//!
//! # Field of View Algorithm
//!
//! This implementation uses a simple ray-casting algorithm for visibility.
//! For each tile within the player's view radius, it traces a line from
//! the player to the tile and checks for obstructions.
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::floor::{update_visibility, UpdateVisibilityCommand};
//!
//! let workflow = update_visibility(&cache, &event_store, cache_ttl);
//! let command = UpdateVisibilityCommand::new(game_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::collections::HashSet;
use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::Position;
use roguelike_domain::game_session::GameSessionEvent;

use super::UpdateVisibilityCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

/// Default view radius for the player.
const DEFAULT_VIEW_RADIUS: i32 = 8;

// =============================================================================
// VisibilityResult
// =============================================================================

/// Result of a visibility calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityResult {
    /// Tiles currently visible to the player.
    visible_tiles: HashSet<Position>,
    /// Tiles newly explored (first time visible).
    newly_explored_tiles: HashSet<Position>,
}

impl VisibilityResult {
    /// Creates a new visibility result.
    #[must_use]
    pub fn new(visible_tiles: HashSet<Position>, newly_explored_tiles: HashSet<Position>) -> Self {
        Self {
            visible_tiles,
            newly_explored_tiles,
        }
    }

    /// Returns the visible tiles.
    #[must_use]
    pub fn visible_tiles(&self) -> &HashSet<Position> {
        &self.visible_tiles
    }

    /// Returns the newly explored tiles.
    #[must_use]
    pub fn newly_explored_tiles(&self) -> &HashSet<Position> {
        &self.newly_explored_tiles
    }
}

// =============================================================================
// UpdateVisibility Workflow
// =============================================================================

/// Creates a workflow function for updating tile visibility.
///
/// This function returns a closure that recalculates the player's field of view
/// and updates which tiles are visible and explored.
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
/// A function that takes an `UpdateVisibilityCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
pub fn update_visibility<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(UpdateVisibilityCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Steps 2-6: [Pure] Calculate visibility
                    let result = update_visibility_pure(&session);

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
pub fn update_visibility_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(UpdateVisibilityCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    update_visibility(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire visibility update logic.
fn update_visibility_pure<S: Clone>(
    _session: &S,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "update_visibility",
        "GameSession structure not yet connected",
    ))
}

/// Gets the player position from the session.
///
/// This is a pure function that extracts the player's position from
/// the game session.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to extract player position
///
/// # Arguments
///
/// * `session` - The game session
/// * `extract_fn` - Function that extracts the position
///
/// # Returns
///
/// The player's current position.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::get_player_position;
/// use roguelike_domain::common::Position;
///
/// struct MockSession { position: Position }
/// let session = MockSession { position: Position::new(10, 20) };
/// let position = get_player_position(&session, |s| s.position);
/// assert_eq!(position, Position::new(10, 20));
/// ```
pub fn get_player_position<S, F>(session: &S, extract_fn: F) -> Position
where
    F: Fn(&S) -> Position,
{
    extract_fn(session)
}

/// Calculates the field of view for a given position.
///
/// This is a pure function that determines which tiles are visible from
/// the player's position, using a simple ray-casting algorithm.
///
/// # Arguments
///
/// * `origin` - The position to calculate FOV from
/// * `view_radius` - The maximum view distance
/// * `is_blocking` - Function that returns true if a tile blocks vision
///
/// # Returns
///
/// A set of positions that are visible from the origin.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::calculate_field_of_view;
/// use roguelike_domain::common::Position;
///
/// let visible = calculate_field_of_view(
///     Position::new(10, 10),
///     3,
///     |_| false, // No blocking tiles
/// );
///
/// assert!(visible.contains(&Position::new(10, 10)));
/// assert!(visible.contains(&Position::new(11, 10)));
/// assert!(visible.contains(&Position::new(10, 11)));
/// ```
#[must_use]
pub fn calculate_field_of_view<F>(
    origin: Position,
    view_radius: i32,
    is_blocking: F,
) -> HashSet<Position>
where
    F: Fn(Position) -> bool,
{
    let mut visible = HashSet::new();

    // The origin is always visible
    visible.insert(origin);

    let origin_x = origin.x();
    let origin_y = origin.y();

    // Cast rays to all positions within the view radius
    for delta_y in -view_radius..=view_radius {
        for delta_x in -view_radius..=view_radius {
            // Skip if outside the circular view radius
            if delta_x * delta_x + delta_y * delta_y > view_radius * view_radius {
                continue;
            }

            let target_x = origin_x + delta_x;
            let target_y = origin_y + delta_y;
            let target = Position::new(target_x, target_y);

            // Cast ray from origin to target
            if is_visible_from_origin(origin, target, &is_blocking) {
                visible.insert(target);
            }
        }
    }

    visible
}

/// Calculates field of view with default radius.
#[must_use]
pub fn calculate_field_of_view_default<F>(origin: Position, is_blocking: F) -> HashSet<Position>
where
    F: Fn(Position) -> bool,
{
    calculate_field_of_view(origin, DEFAULT_VIEW_RADIUS, is_blocking)
}

/// Checks if a target position is visible from the origin.
///
/// Uses Bresenham's line algorithm to trace a ray from origin to target.
fn is_visible_from_origin<F>(origin: Position, target: Position, is_blocking: &F) -> bool
where
    F: Fn(Position) -> bool,
{
    let mut current_x = origin.x();
    let mut current_y = origin.y();
    let target_x = target.x();
    let target_y = target.y();

    let delta_x = (target_x - current_x).abs();
    let delta_y = (target_y - current_y).abs();
    let step_x = if current_x < target_x { 1 } else { -1 };
    let step_y = if current_y < target_y { 1 } else { -1 };

    let mut error = delta_x - delta_y;

    loop {
        // Check if we reached the target
        if current_x == target_x && current_y == target_y {
            return true;
        }

        // Check if current position blocks vision (but don't block on origin)
        let current_position = Position::new(current_x, current_y);
        if current_position != origin && is_blocking(current_position) {
            return false;
        }

        let error2 = 2 * error;

        if error2 > -delta_y {
            error -= delta_y;
            current_x += step_x;
        }

        if error2 < delta_x {
            error += delta_x;
            current_y += step_y;
        }
    }
}

/// Updates the set of explored tiles.
///
/// This is a pure function that combines previously explored tiles with
/// newly visible tiles to create the updated explored set.
///
/// # Arguments
///
/// * `previously_explored` - The set of tiles that were already explored
/// * `currently_visible` - The set of tiles currently visible
///
/// # Returns
///
/// A tuple of (updated_explored_tiles, newly_explored_tiles).
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::update_explored_tiles;
/// use roguelike_domain::common::Position;
/// use std::collections::HashSet;
///
/// let mut explored = HashSet::new();
/// explored.insert(Position::new(5, 5));
/// explored.insert(Position::new(6, 5));
///
/// let mut visible = HashSet::new();
/// visible.insert(Position::new(6, 5));
/// visible.insert(Position::new(7, 5));
///
/// let (updated, newly_explored) = update_explored_tiles(&explored, &visible);
///
/// assert_eq!(updated.len(), 3);
/// assert_eq!(newly_explored.len(), 1);
/// assert!(newly_explored.contains(&Position::new(7, 5)));
/// ```
#[must_use]
pub fn update_explored_tiles(
    previously_explored: &HashSet<Position>,
    currently_visible: &HashSet<Position>,
) -> (HashSet<Position>, HashSet<Position>) {
    // Find newly explored tiles (visible but not previously explored)
    let newly_explored: HashSet<Position> = currently_visible
        .difference(previously_explored)
        .copied()
        .collect();

    // Combine for total explored
    let updated_explored: HashSet<Position> = previously_explored
        .union(currently_visible)
        .copied()
        .collect();

    (updated_explored, newly_explored)
}

/// Updates the session with new visibility information.
///
/// This is a pure function that immutably updates the session with
/// visibility data and returns the updated session along with events.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to update the session
///
/// # Arguments
///
/// * `session` - The current game session
/// * `visibility_result` - The visibility calculation result
/// * `update_fn` - Function that updates the session
///
/// # Returns
///
/// A tuple of (updated_session, generated_events).
pub fn update_session_visibility<S, F>(
    session: &S,
    visibility_result: &VisibilityResult,
    update_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, &VisibilityResult) -> S,
{
    let updated_session = update_fn(session, visibility_result);

    // Generate tile explored events
    // Note: We would normally generate TileExplored events here,
    // but since they aren't yet used in the workflow, we return an empty vec
    let events = Vec::new();

    (updated_session, events)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // VisibilityResult Tests
    // =========================================================================

    mod visibility_result {
        use super::*;

        #[rstest]
        fn new_creates_result() {
            let mut visible = HashSet::new();
            visible.insert(Position::new(5, 5));
            visible.insert(Position::new(6, 5));

            let mut newly_explored = HashSet::new();
            newly_explored.insert(Position::new(6, 5));

            let result = VisibilityResult::new(visible.clone(), newly_explored.clone());

            assert_eq!(result.visible_tiles(), &visible);
            assert_eq!(result.newly_explored_tiles(), &newly_explored);
        }

        #[rstest]
        fn empty_result() {
            let result = VisibilityResult::new(HashSet::new(), HashSet::new());

            assert!(result.visible_tiles().is_empty());
            assert!(result.newly_explored_tiles().is_empty());
        }
    }

    // =========================================================================
    // get_player_position Tests
    // =========================================================================

    mod get_player_position_tests {
        use super::*;

        struct MockSession {
            position: Position,
        }

        #[rstest]
        fn extracts_position() {
            let session = MockSession {
                position: Position::new(25, 35),
            };

            let position = get_player_position(&session, |s| s.position);

            assert_eq!(position, Position::new(25, 35));
        }
    }

    // =========================================================================
    // calculate_field_of_view Tests
    // =========================================================================

    mod calculate_field_of_view_tests {
        use super::*;

        #[rstest]
        fn origin_is_always_visible() {
            let origin = Position::new(10, 10);
            let visible = calculate_field_of_view(origin, 5, |_| false);

            assert!(visible.contains(&origin));
        }

        #[rstest]
        fn zero_radius_shows_only_origin() {
            let origin = Position::new(10, 10);
            let visible = calculate_field_of_view(origin, 0, |_| false);

            assert_eq!(visible.len(), 1);
            assert!(visible.contains(&origin));
        }

        #[rstest]
        fn small_radius_shows_nearby_tiles() {
            let origin = Position::new(10, 10);
            let visible = calculate_field_of_view(origin, 1, |_| false);

            // Should see origin and 8 adjacent tiles
            assert!(visible.len() >= 5);
            assert!(visible.contains(&Position::new(10, 10)));
            assert!(visible.contains(&Position::new(11, 10)));
            assert!(visible.contains(&Position::new(9, 10)));
            assert!(visible.contains(&Position::new(10, 11)));
            assert!(visible.contains(&Position::new(10, 9)));
        }

        #[rstest]
        fn blocking_tiles_stop_vision() {
            let origin = Position::new(10, 10);

            // Place a wall at (12, 10)
            let is_blocking = |position: Position| position == Position::new(12, 10);

            let visible = calculate_field_of_view(origin, 5, is_blocking);

            // Should see up to the wall but not beyond
            assert!(visible.contains(&Position::new(11, 10)));
            // The wall itself should be visible (you can see the wall)
            assert!(visible.contains(&Position::new(12, 10)));
            // But beyond the wall should be blocked
            assert!(!visible.contains(&Position::new(13, 10)));
        }

        #[rstest]
        fn circular_field_of_view() {
            let origin = Position::new(10, 10);
            let radius = 3;
            let visible = calculate_field_of_view(origin, radius, |_| false);

            // Check that tiles at corners of a square are not visible (outside circle)
            // For radius 3, the corners at distance sqrt(3^2 + 3^2) = 4.24 are outside

            // But tiles within the circle should be visible
            assert!(visible.contains(&Position::new(12, 10))); // distance 2
            assert!(visible.contains(&Position::new(10, 12))); // distance 2
            assert!(visible.contains(&Position::new(13, 10))); // distance 3
        }

        #[rstest]
        fn deterministic_results() {
            let origin = Position::new(15, 15);
            let is_blocking = |position: Position| position.x() == 17 && position.y() == 15;

            let visible1 = calculate_field_of_view(origin, 4, is_blocking);
            let visible2 = calculate_field_of_view(origin, 4, is_blocking);

            assert_eq!(visible1, visible2);
        }
    }

    // =========================================================================
    // update_explored_tiles Tests
    // =========================================================================

    mod update_explored_tiles_tests {
        use super::*;

        #[rstest]
        fn combines_explored_and_visible() {
            let mut explored = HashSet::new();
            explored.insert(Position::new(1, 1));
            explored.insert(Position::new(2, 1));

            let mut visible = HashSet::new();
            visible.insert(Position::new(2, 1));
            visible.insert(Position::new(3, 1));

            let (updated, newly_explored) = update_explored_tiles(&explored, &visible);

            assert_eq!(updated.len(), 3);
            assert!(updated.contains(&Position::new(1, 1)));
            assert!(updated.contains(&Position::new(2, 1)));
            assert!(updated.contains(&Position::new(3, 1)));

            assert_eq!(newly_explored.len(), 1);
            assert!(newly_explored.contains(&Position::new(3, 1)));
        }

        #[rstest]
        fn no_new_tiles_explored() {
            let mut explored = HashSet::new();
            explored.insert(Position::new(5, 5));
            explored.insert(Position::new(6, 5));

            let mut visible = HashSet::new();
            visible.insert(Position::new(5, 5));

            let (updated, newly_explored) = update_explored_tiles(&explored, &visible);

            assert_eq!(updated.len(), 2);
            assert!(newly_explored.is_empty());
        }

        #[rstest]
        fn all_tiles_new() {
            let explored = HashSet::new();

            let mut visible = HashSet::new();
            visible.insert(Position::new(10, 10));
            visible.insert(Position::new(11, 10));

            let (updated, newly_explored) = update_explored_tiles(&explored, &visible);

            assert_eq!(updated.len(), 2);
            assert_eq!(newly_explored.len(), 2);
        }

        #[rstest]
        fn empty_inputs() {
            let explored = HashSet::new();
            let visible = HashSet::new();

            let (updated, newly_explored) = update_explored_tiles(&explored, &visible);

            assert!(updated.is_empty());
            assert!(newly_explored.is_empty());
        }
    }

    // =========================================================================
    // update_session_visibility Tests
    // =========================================================================

    mod update_session_visibility_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            visible_count: usize,
            explored_count: usize,
        }

        impl MockSession {
            fn new() -> Self {
                Self {
                    visible_count: 0,
                    explored_count: 0,
                }
            }
        }

        #[rstest]
        fn updates_session() {
            let session = MockSession::new();

            let mut visible = HashSet::new();
            visible.insert(Position::new(5, 5));
            visible.insert(Position::new(6, 5));
            visible.insert(Position::new(7, 5));

            let mut newly_explored = HashSet::new();
            newly_explored.insert(Position::new(6, 5));

            let visibility = VisibilityResult::new(visible, newly_explored);

            let (updated, _) =
                update_session_visibility(&session, &visibility, |_, result| MockSession {
                    visible_count: result.visible_tiles().len(),
                    explored_count: result.newly_explored_tiles().len(),
                });

            assert_eq!(updated.visible_count, 3);
            assert_eq!(updated.explored_count, 1);
        }

        #[rstest]
        fn returns_events_list() {
            let session = MockSession::new();
            let visibility = VisibilityResult::new(HashSet::new(), HashSet::new());

            let (_, events) = update_session_visibility(&session, &visibility, |s, _| s.clone());

            // Currently returns empty vec as event types aren't defined yet
            assert!(events.is_empty());
        }
    }

    // =========================================================================
    // is_visible_from_origin Tests
    // =========================================================================

    mod is_visible_from_origin_tests {
        use super::*;

        #[rstest]
        fn target_equals_origin() {
            let origin = Position::new(5, 5);
            assert!(is_visible_from_origin(origin, origin, &|_| false));
        }

        #[rstest]
        fn unblocked_line_of_sight() {
            let origin = Position::new(5, 5);
            let target = Position::new(10, 5);

            assert!(is_visible_from_origin(origin, target, &|_| false));
        }

        #[rstest]
        fn blocked_line_of_sight() {
            let origin = Position::new(5, 5);
            let target = Position::new(10, 5);

            // Block at position 7, 5
            let is_blocking = |position: Position| position == Position::new(7, 5);

            assert!(!is_visible_from_origin(origin, target, &is_blocking));
        }

        #[rstest]
        fn diagonal_visibility() {
            let origin = Position::new(5, 5);
            let target = Position::new(8, 8);

            assert!(is_visible_from_origin(origin, target, &|_| false));
        }

        #[rstest]
        fn adjacent_target_visible_even_if_blocking() {
            let origin = Position::new(5, 5);
            let target = Position::new(6, 5);

            // Even if we consider all tiles blocking, adjacent should be visible
            // because we don't block on the origin
            let is_blocking = |position: Position| position != origin;

            // The target itself is visible even if it blocks
            assert!(is_visible_from_origin(origin, target, &is_blocking));
        }
    }
}
