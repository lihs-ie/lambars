//! DescendFloor workflow implementation.
//!
//! This module provides the workflow for descending to the next floor
//! when the player reaches the down staircase.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Validate player is at down stairs
//! 3. [Pure] Calculate next floor level
//! 4. [Pure] Generate new floor data
//! 5. [Pure] Set player at spawn point (up stairs)
//! 6. [Pure] Spawn enemies for the new floor
//! 7. [Pure] Update session for floor change
//! 8. [Pure] Generate PlayerDescended event
//! 9. [IO] Update cache
//! 10. [IO] Append events to event store
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::floor::{descend_floor, DescendFloorCommand};
//!
//! let workflow = descend_floor(&cache, &event_store, cache_ttl);
//! let command = DescendFloorCommand::new(game_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::Position;
use roguelike_domain::floor::FloorError;
use roguelike_domain::game_session::GameSessionEvent;

use super::DescendFloorCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// EnemySpawnInfo
// =============================================================================

/// Information about an enemy to be spawned on the floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemySpawnInfo {
    /// Position where the enemy will spawn.
    position: Position,
    /// Type of enemy to spawn.
    enemy_type: String,
    /// Level of the enemy.
    level: u32,
}

impl EnemySpawnInfo {
    /// Creates a new enemy spawn info.
    #[must_use]
    pub fn new(position: Position, enemy_type: impl Into<String>, level: u32) -> Self {
        Self {
            position,
            enemy_type: enemy_type.into(),
            level,
        }
    }

    /// Returns the spawn position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the enemy type.
    #[must_use]
    pub fn enemy_type(&self) -> &str {
        &self.enemy_type
    }

    /// Returns the enemy level.
    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }
}

// =============================================================================
// DescendFloor Workflow
// =============================================================================

/// Creates a workflow function for descending to the next floor.
///
/// This function returns a closure that handles the player descending
/// to the next dungeon floor. It validates that the player is at a
/// down staircase, generates the new floor, and updates all state.
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
/// A function that takes a `DescendFloorCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
pub fn descend_floor<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(DescendFloorCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
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
                    // Steps 2-8: [Pure] Process floor descent
                    let result = descend_floor_pure(&session);

                    match result {
                        Ok((updated_session, events)) => {
                            // Steps 9-10: [IO] Update cache and append events
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
pub fn descend_floor_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(DescendFloorCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    descend_floor(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire floor descent logic.
fn descend_floor_pure<S: Clone>(_session: &S) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "descend_floor",
        "GameSession structure not yet connected",
    ))
}

/// Validates that the player is at a down staircase.
///
/// This is a pure function that checks if the player's current position
/// contains a down staircase.
///
/// # Arguments
///
/// * `player_position` - The player's current position
/// * `down_stairs_position` - The position of the down stairs on the current floor
///
/// # Returns
///
/// `Ok(())` if the player is at the down stairs, `Err(FloorError)` otherwise.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::validate_at_down_stairs;
/// use roguelike_domain::common::Position;
///
/// let player = Position::new(20, 20);
/// let stairs = Position::new(20, 20);
///
/// assert!(validate_at_down_stairs(player, stairs).is_ok());
///
/// let player_away = Position::new(5, 5);
/// assert!(validate_at_down_stairs(player_away, stairs).is_err());
/// ```
pub fn validate_at_down_stairs(
    player_position: Position,
    down_stairs_position: Position,
) -> Result<(), FloorError> {
    if player_position == down_stairs_position {
        Ok(())
    } else {
        Err(FloorError::not_at_stairs(player_position))
    }
}

/// Calculates the next floor level.
///
/// This is a pure function that computes the floor level after descending.
///
/// # Arguments
///
/// * `current_floor_level` - The current floor level
///
/// # Returns
///
/// The next floor level.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::calculate_next_floor_level;
///
/// assert_eq!(calculate_next_floor_level(1), 2);
/// assert_eq!(calculate_next_floor_level(5), 6);
/// ```
#[must_use]
pub const fn calculate_next_floor_level(current_floor_level: u32) -> u32 {
    current_floor_level.saturating_add(1)
}

/// Sets the player at the spawn point (up stairs) of the new floor.
///
/// This is a pure function that returns the new player position.
///
/// # Arguments
///
/// * `up_stairs_position` - The position of the up stairs on the new floor
///
/// # Returns
///
/// The position where the player should spawn.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::set_player_at_spawn_point;
/// use roguelike_domain::common::Position;
///
/// let stairs = Position::new(10, 10);
/// let spawn = set_player_at_spawn_point(stairs);
/// assert_eq!(spawn, stairs);
/// ```
#[must_use]
pub const fn set_player_at_spawn_point(up_stairs_position: Position) -> Position {
    up_stairs_position
}

/// Spawns enemies for the new floor.
///
/// This is a pure function that determines which enemies to spawn
/// based on the floor level.
///
/// # Arguments
///
/// * `floor_level` - The floor level
/// * `valid_positions` - Valid positions where enemies can spawn
/// * `seed` - Random seed for reproducible spawning
///
/// # Returns
///
/// A vector of enemy spawn information.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::spawn_floor_enemies;
/// use roguelike_domain::common::Position;
///
/// let positions = vec![Position::new(5, 5), Position::new(10, 10), Position::new(15, 15)];
/// let enemies = spawn_floor_enemies(3, &positions, 12345);
/// assert!(!enemies.is_empty());
/// ```
#[must_use]
pub fn spawn_floor_enemies(
    floor_level: u32,
    valid_positions: &[Position],
    seed: u64,
) -> Vec<EnemySpawnInfo> {
    let enemy_count = calculate_enemy_count(floor_level).min(valid_positions.len() as u32);
    let mut enemies = Vec::with_capacity(enemy_count as usize);
    let mut current_seed = seed;
    let mut used_positions = Vec::new();

    let available_enemy_types = get_enemy_types_for_floor(floor_level);

    for _ in 0..enemy_count {
        // Find a unique position
        current_seed = next_seed(current_seed);
        let mut position_index = (current_seed >> 32) as usize % valid_positions.len();

        // Skip already used positions
        let mut attempts = 0;
        while used_positions.contains(&position_index) && attempts < valid_positions.len() {
            position_index = (position_index + 1) % valid_positions.len();
            attempts += 1;
        }

        if attempts >= valid_positions.len() {
            break;
        }

        used_positions.push(position_index);

        // Select enemy type
        current_seed = next_seed(current_seed);
        let type_index = (current_seed >> 32) as usize % available_enemy_types.len();

        // Calculate enemy level based on floor level
        current_seed = next_seed(current_seed);
        let level_variance = ((current_seed >> 32) % 3) as u32;
        let enemy_level = floor_level.saturating_sub(1) + level_variance;

        enemies.push(EnemySpawnInfo::new(
            valid_positions[position_index],
            available_enemy_types[type_index].clone(),
            enemy_level.max(1),
        ));
    }

    enemies
}

/// Calculates the number of enemies to spawn based on floor level.
fn calculate_enemy_count(floor_level: u32) -> u32 {
    let base_count = 3;
    let level_bonus = floor_level / 3;
    (base_count + level_bonus).min(15)
}

/// Gets available enemy types for a floor level.
fn get_enemy_types_for_floor(floor_level: u32) -> Vec<String> {
    let mut types = vec!["Rat".to_string(), "Slime".to_string()];

    if floor_level >= 3 {
        types.push("Goblin".to_string());
        types.push("Skeleton".to_string());
    }

    if floor_level >= 5 {
        types.push("Orc".to_string());
        types.push("Zombie".to_string());
    }

    if floor_level >= 7 {
        types.push("Ogre".to_string());
        types.push("Wraith".to_string());
    }

    if floor_level >= 10 {
        types.push("Troll".to_string());
        types.push("Vampire".to_string());
        types.push("Dark Knight".to_string());
    }

    types
}

/// Updates the session for a floor change.
///
/// This is a pure function that immutably updates the session with
/// new floor data and returns the updated session along with events.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to update the session
///
/// # Arguments
///
/// * `session` - The current game session
/// * `new_floor_level` - The new floor level
/// * `player_position` - The new player position
/// * `enemies` - The enemies to spawn
/// * `update_fn` - Function that updates the session
///
/// # Returns
///
/// A tuple of (updated_session, generated_events).
pub fn update_session_for_floor_change<S, F>(
    session: &S,
    new_floor_level: u32,
    player_position: Position,
    enemies: &[EnemySpawnInfo],
    update_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, u32, Position, &[EnemySpawnInfo]) -> S,
{
    let updated_session = update_fn(session, new_floor_level, player_position, enemies);

    // Generate floor descent events
    // Note: We would normally generate a PlayerDescended event here,
    // but since it's not yet in GameSessionEvent, we return an empty vec
    let events = Vec::new();

    (updated_session, events)
}

/// Simple LCG for deterministic random numbers.
fn next_seed(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // EnemySpawnInfo Tests
    // =========================================================================

    mod enemy_spawn_info {
        use super::*;

        #[rstest]
        fn new_creates_info() {
            let info = EnemySpawnInfo::new(Position::new(10, 10), "Goblin", 5);

            assert_eq!(info.position(), Position::new(10, 10));
            assert_eq!(info.enemy_type(), "Goblin");
            assert_eq!(info.level(), 5);
        }

        #[rstest]
        fn clone_preserves_values() {
            let info = EnemySpawnInfo::new(Position::new(20, 30), "Orc", 10);
            let cloned = info.clone();

            assert_eq!(info, cloned);
        }
    }

    // =========================================================================
    // validate_at_down_stairs Tests
    // =========================================================================

    mod validate_at_down_stairs_tests {
        use super::*;

        #[rstest]
        fn returns_ok_when_at_stairs() {
            let player = Position::new(20, 20);
            let stairs = Position::new(20, 20);

            assert!(validate_at_down_stairs(player, stairs).is_ok());
        }

        #[rstest]
        fn returns_error_when_not_at_stairs() {
            let player = Position::new(5, 5);
            let stairs = Position::new(20, 20);

            assert!(validate_at_down_stairs(player, stairs).is_err());
        }

        #[rstest]
        fn position_must_be_exact() {
            let player = Position::new(20, 21);
            let stairs = Position::new(20, 20);

            assert!(validate_at_down_stairs(player, stairs).is_err());
        }
    }

    // =========================================================================
    // calculate_next_floor_level Tests
    // =========================================================================

    mod calculate_next_floor_level_tests {
        use super::*;

        #[rstest]
        #[case(1, 2)]
        #[case(5, 6)]
        #[case(10, 11)]
        #[case(99, 100)]
        fn calculates_correctly(#[case] current: u32, #[case] expected: u32) {
            assert_eq!(calculate_next_floor_level(current), expected);
        }

        #[rstest]
        fn handles_max_value() {
            assert_eq!(calculate_next_floor_level(u32::MAX), u32::MAX);
        }
    }

    // =========================================================================
    // set_player_at_spawn_point Tests
    // =========================================================================

    mod set_player_at_spawn_point_tests {
        use super::*;

        #[rstest]
        fn returns_stairs_position() {
            let stairs = Position::new(15, 25);
            assert_eq!(set_player_at_spawn_point(stairs), stairs);
        }

        #[rstest]
        #[case(Position::new(0, 0))]
        #[case(Position::new(50, 50))]
        #[case(Position::new(100, 200))]
        fn works_with_various_positions(#[case] position: Position) {
            assert_eq!(set_player_at_spawn_point(position), position);
        }
    }

    // =========================================================================
    // spawn_floor_enemies Tests
    // =========================================================================

    mod spawn_floor_enemies_tests {
        use super::*;

        #[rstest]
        fn spawns_enemies() {
            let positions = vec![
                Position::new(5, 5),
                Position::new(10, 10),
                Position::new(15, 15),
                Position::new(20, 20),
                Position::new(25, 25),
            ];

            let enemies = spawn_floor_enemies(1, &positions, 12345);

            assert!(!enemies.is_empty());
        }

        #[rstest]
        fn more_enemies_on_deeper_floors() {
            let positions: Vec<Position> =
                (0..20).map(|index| Position::new(index * 5, 10)).collect();

            let enemies_early = spawn_floor_enemies(1, &positions, 12345);
            let enemies_late = spawn_floor_enemies(15, &positions, 12345);

            assert!(enemies_late.len() >= enemies_early.len());
        }

        #[rstest]
        fn enemies_spawn_at_unique_positions() {
            let positions: Vec<Position> =
                (0..20).map(|index| Position::new(index * 5, 10)).collect();

            let enemies = spawn_floor_enemies(5, &positions, 12345);

            for (i, enemy) in enemies.iter().enumerate() {
                for (j, other) in enemies.iter().enumerate() {
                    if i != j {
                        assert_ne!(enemy.position(), other.position());
                    }
                }
            }
        }

        #[rstest]
        fn early_floors_have_basic_enemies() {
            let positions: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let enemies = spawn_floor_enemies(1, &positions, 12345);

            for enemy in &enemies {
                assert!(
                    enemy.enemy_type() == "Rat" || enemy.enemy_type() == "Slime",
                    "Expected basic enemy type, got: {}",
                    enemy.enemy_type()
                );
            }
        }

        #[rstest]
        fn same_seed_produces_same_enemies() {
            let positions: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let enemies1 = spawn_floor_enemies(5, &positions, 12345);
            let enemies2 = spawn_floor_enemies(5, &positions, 12345);

            assert_eq!(enemies1.len(), enemies2.len());
            for (enemy1, enemy2) in enemies1.iter().zip(enemies2.iter()) {
                assert_eq!(enemy1.position(), enemy2.position());
                assert_eq!(enemy1.enemy_type(), enemy2.enemy_type());
                assert_eq!(enemy1.level(), enemy2.level());
            }
        }

        #[rstest]
        fn enemy_level_scales_with_floor() {
            let positions: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let enemies_early = spawn_floor_enemies(1, &positions, 12345);
            let enemies_late = spawn_floor_enemies(10, &positions, 12345);

            let avg_early: u32 =
                enemies_early.iter().map(|e| e.level()).sum::<u32>() / enemies_early.len() as u32;
            let avg_late: u32 =
                enemies_late.iter().map(|e| e.level()).sum::<u32>() / enemies_late.len() as u32;

            assert!(avg_late > avg_early);
        }

        #[rstest]
        fn handles_empty_positions() {
            let positions: Vec<Position> = vec![];
            let enemies = spawn_floor_enemies(5, &positions, 12345);
            assert!(enemies.is_empty());
        }

        #[rstest]
        fn respects_available_positions() {
            let positions = vec![Position::new(5, 5), Position::new(10, 10)];
            let enemies = spawn_floor_enemies(10, &positions, 12345);
            assert!(enemies.len() <= 2);
        }
    }

    // =========================================================================
    // update_session_for_floor_change Tests
    // =========================================================================

    mod update_session_for_floor_change_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            floor_level: u32,
            player_position: Position,
            enemy_count: usize,
        }

        impl MockSession {
            fn new() -> Self {
                Self {
                    floor_level: 1,
                    player_position: Position::new(0, 0),
                    enemy_count: 0,
                }
            }
        }

        #[rstest]
        fn updates_session_correctly() {
            let session = MockSession::new();
            let enemies = vec![
                EnemySpawnInfo::new(Position::new(10, 10), "Goblin", 3),
                EnemySpawnInfo::new(Position::new(20, 20), "Skeleton", 4),
            ];

            let (updated, _) = update_session_for_floor_change(
                &session,
                5,
                Position::new(15, 15),
                &enemies,
                |_session, level, pos, spawned| MockSession {
                    floor_level: level,
                    player_position: pos,
                    enemy_count: spawned.len(),
                },
            );

            assert_eq!(updated.floor_level, 5);
            assert_eq!(updated.player_position, Position::new(15, 15));
            assert_eq!(updated.enemy_count, 2);
        }

        #[rstest]
        fn returns_events_list() {
            let session = MockSession::new();
            let (_, events) = update_session_for_floor_change(
                &session,
                2,
                Position::new(5, 5),
                &[],
                |s, _, _, _| s.clone(),
            );

            // Currently returns empty vec as event types aren't defined yet
            assert!(events.is_empty());
        }
    }
}
