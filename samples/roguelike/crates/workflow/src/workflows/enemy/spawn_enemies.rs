use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::Position;
use roguelike_domain::enemy::{AiBehavior, EnemySpawned, EnemyType, EntityIdentifier};
use roguelike_domain::game_session::GameSessionEvent;

use super::SpawnEnemiesCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// SpawnConfiguration
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnConfiguration {
    min_enemies: u32,
    max_enemies: u32,
    allowed_types: Vec<EnemyType>,
    allow_bosses: bool,
}

impl SpawnConfiguration {
    #[must_use]
    pub const fn new(
        min_enemies: u32,
        max_enemies: u32,
        allowed_types: Vec<EnemyType>,
        allow_bosses: bool,
    ) -> Self {
        Self {
            min_enemies,
            max_enemies,
            allowed_types,
            allow_bosses,
        }
    }

    #[must_use]
    pub const fn min_enemies(&self) -> u32 {
        self.min_enemies
    }

    #[must_use]
    pub const fn max_enemies(&self) -> u32 {
        self.max_enemies
    }

    #[must_use]
    pub fn allowed_types(&self) -> &[EnemyType] {
        &self.allowed_types
    }

    #[must_use]
    pub const fn allow_bosses(&self) -> bool {
        self.allow_bosses
    }
}

// =============================================================================
// EnemyInstance
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyInstance {
    identifier: EntityIdentifier,
    enemy_type: EnemyType,
    behavior: AiBehavior,
    position: Position,
}

impl EnemyInstance {
    #[must_use]
    pub const fn new(
        identifier: EntityIdentifier,
        enemy_type: EnemyType,
        behavior: AiBehavior,
        position: Position,
    ) -> Self {
        Self {
            identifier,
            enemy_type,
            behavior,
            position,
        }
    }

    #[must_use]
    pub const fn identifier(&self) -> EntityIdentifier {
        self.identifier
    }

    #[must_use]
    pub const fn enemy_type(&self) -> EnemyType {
        self.enemy_type
    }

    #[must_use]
    pub const fn behavior(&self) -> AiBehavior {
        self.behavior
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

// =============================================================================
// SpawnEnemies Workflow
// =============================================================================

pub fn spawn_enemies<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(SpawnEnemiesCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let floor_level = command.floor_level();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Steps 2-6: [Pure] Spawn enemies
                    let result = spawn_enemies_pure(&session, floor_level);

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

pub fn spawn_enemies_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(SpawnEnemiesCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    spawn_enemies(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

fn spawn_enemies_pure<S: Clone>(
    _session: &S,
    _floor_level: u32,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "spawn_enemies",
        "GameSession structure not yet connected",
    ))
}

#[must_use]
pub fn get_spawn_configuration(floor_level: u32) -> SpawnConfiguration {
    let (min_enemies, max_enemies) = calculate_enemy_count_range(floor_level);
    let allowed_types = determine_allowed_enemy_types(floor_level);
    let allow_bosses = floor_level >= 10 && floor_level.is_multiple_of(5);

    SpawnConfiguration::new(min_enemies, max_enemies, allowed_types, allow_bosses)
}

fn calculate_enemy_count_range(floor_level: u32) -> (u32, u32) {
    let base_min = 2;
    let base_max = 5;
    let level_bonus = floor_level / 3;

    (base_min + level_bonus, base_max + level_bonus * 2)
}

fn determine_allowed_enemy_types(floor_level: u32) -> Vec<EnemyType> {
    match floor_level {
        1..=3 => vec![EnemyType::Slime, EnemyType::Bat, EnemyType::Goblin],
        4..=6 => vec![
            EnemyType::Slime,
            EnemyType::Bat,
            EnemyType::Goblin,
            EnemyType::Spider,
            EnemyType::Skeleton,
        ],
        7..=9 => vec![
            EnemyType::Goblin,
            EnemyType::Spider,
            EnemyType::Skeleton,
            EnemyType::Zombie,
            EnemyType::Orc,
        ],
        10..=14 => vec![
            EnemyType::Skeleton,
            EnemyType::Zombie,
            EnemyType::Orc,
            EnemyType::Ghost,
            EnemyType::Minotaur,
        ],
        _ => vec![
            EnemyType::Orc,
            EnemyType::Ghost,
            EnemyType::Minotaur,
            EnemyType::Dragon,
        ],
    }
}

#[must_use]
pub fn find_valid_spawn_points<F>(
    floor_bounds: (u32, u32),
    count: u32,
    is_valid_spawn: F,
    seed: u64,
) -> Vec<Position>
where
    F: Fn(Position) -> bool,
{
    let mut positions = Vec::with_capacity(count as usize);
    let mut current_seed = seed;

    // Simple LCG for deterministic random positions
    for _ in 0..count * 10 {
        // Try up to 10x to find valid positions
        if positions.len() >= count as usize {
            break;
        }

        // Generate random position
        current_seed = current_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let x = ((current_seed >> 32) as u32 % floor_bounds.0) as i32;
        current_seed = current_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let y = ((current_seed >> 32) as u32 % floor_bounds.1) as i32;

        let position = Position::new(x, y);

        // Check if position is valid and not already used
        if is_valid_spawn(position) && !positions.contains(&position) {
            positions.push(position);
        }
    }

    positions
}

#[must_use]
pub fn generate_enemies(
    configuration: &SpawnConfiguration,
    spawn_points: &[Position],
    seed: u64,
) -> Vec<EnemyInstance> {
    let mut enemies = Vec::with_capacity(spawn_points.len());
    let mut current_seed = seed;

    for position in spawn_points {
        // Select enemy type
        current_seed = current_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let type_index = (current_seed >> 32) as usize % configuration.allowed_types().len();
        let enemy_type = configuration.allowed_types()[type_index];

        // Skip bosses if not allowed
        if enemy_type.is_boss() && !configuration.allow_bosses() {
            continue;
        }

        // Determine behavior based on enemy type
        let behavior = determine_default_behavior(enemy_type);

        // Create enemy instance
        let enemy = EnemyInstance::new(EntityIdentifier::new(), enemy_type, behavior, *position);
        enemies.push(enemy);
    }

    enemies
}

fn determine_default_behavior(enemy_type: EnemyType) -> AiBehavior {
    match enemy_type {
        EnemyType::Goblin | EnemyType::Orc | EnemyType::Dragon => AiBehavior::Aggressive,
        EnemyType::Skeleton | EnemyType::Zombie | EnemyType::Minotaur => AiBehavior::Patrol,
        EnemyType::Slime | EnemyType::Spider => AiBehavior::Defensive,
        EnemyType::Bat => AiBehavior::Flee,
        EnemyType::Ghost => AiBehavior::Passive,
    }
}

pub fn add_enemies_to_session<S, F>(
    session: &S,
    enemies: &[EnemyInstance],
    add_enemy: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, &EnemyInstance) -> S,
{
    let (final_session, events) = enemies.iter().fold(
        (session.clone(), Vec::new()),
        |(current_session, mut current_events), enemy| {
            let updated_session = add_enemy(&current_session, enemy);

            // Generate spawn event
            let event = EnemySpawned::new(enemy.identifier(), enemy.enemy_type(), enemy.position());
            current_events.push(GameSessionEvent::EnemySpawned(event));

            (updated_session, current_events)
        },
    );

    (final_session, events)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // SpawnConfiguration Tests
    // =========================================================================

    mod spawn_configuration {
        use super::*;

        #[rstest]
        fn new_creates_configuration() {
            let config =
                SpawnConfiguration::new(2, 5, vec![EnemyType::Goblin, EnemyType::Slime], false);

            assert_eq!(config.min_enemies(), 2);
            assert_eq!(config.max_enemies(), 5);
            assert_eq!(config.allowed_types().len(), 2);
            assert!(!config.allow_bosses());
        }

        #[rstest]
        fn clone_preserves_values() {
            let config = SpawnConfiguration::new(3, 8, vec![EnemyType::Dragon], true);
            let cloned = config.clone();

            assert_eq!(config, cloned);
        }
    }

    // =========================================================================
    // EnemyInstance Tests
    // =========================================================================

    mod enemy_instance {
        use super::*;

        #[rstest]
        fn new_creates_instance() {
            let identifier = EntityIdentifier::new();
            let position = Position::new(10, 20);
            let instance = EnemyInstance::new(
                identifier,
                EnemyType::Goblin,
                AiBehavior::Aggressive,
                position,
            );

            assert_eq!(instance.identifier(), identifier);
            assert_eq!(instance.enemy_type(), EnemyType::Goblin);
            assert_eq!(instance.behavior(), AiBehavior::Aggressive);
            assert_eq!(instance.position(), position);
        }

        #[rstest]
        fn clone_preserves_values() {
            let instance = EnemyInstance::new(
                EntityIdentifier::new(),
                EnemyType::Dragon,
                AiBehavior::Aggressive,
                Position::new(5, 5),
            );
            let cloned = instance.clone();

            assert_eq!(instance.enemy_type(), cloned.enemy_type());
            assert_eq!(instance.behavior(), cloned.behavior());
            assert_eq!(instance.position(), cloned.position());
        }
    }

    // =========================================================================
    // get_spawn_configuration Tests
    // =========================================================================

    mod get_spawn_configuration_tests {
        use super::*;

        #[rstest]
        #[case(1)]
        #[case(2)]
        #[case(3)]
        fn early_floors_have_weak_enemies(#[case] floor_level: u32) {
            let config = get_spawn_configuration(floor_level);

            assert!(config.allowed_types().contains(&EnemyType::Slime));
            assert!(!config.allowed_types().contains(&EnemyType::Dragon));
            assert!(!config.allow_bosses());
        }

        #[rstest]
        fn floor_10_allows_bosses() {
            let config = get_spawn_configuration(10);
            assert!(config.allow_bosses());
        }

        #[rstest]
        fn floor_15_allows_bosses() {
            let config = get_spawn_configuration(15);
            assert!(config.allow_bosses());
        }

        #[rstest]
        fn floor_12_does_not_allow_bosses() {
            let config = get_spawn_configuration(12);
            assert!(!config.allow_bosses());
        }

        #[rstest]
        fn higher_floors_have_more_enemies() {
            let config_low = get_spawn_configuration(1);
            let config_high = get_spawn_configuration(20);

            assert!(config_high.min_enemies() > config_low.min_enemies());
            assert!(config_high.max_enemies() > config_low.max_enemies());
        }

        #[rstest]
        fn late_floors_have_stronger_enemies() {
            let config = get_spawn_configuration(15);

            assert!(
                config.allowed_types().contains(&EnemyType::Dragon)
                    || config.allowed_types().contains(&EnemyType::Minotaur)
            );
            assert!(!config.allowed_types().contains(&EnemyType::Slime));
        }
    }

    // =========================================================================
    // find_valid_spawn_points Tests
    // =========================================================================

    mod find_valid_spawn_points_tests {
        use super::*;

        #[rstest]
        fn finds_requested_count_when_possible() {
            let positions = find_valid_spawn_points((80, 40), 5, |_| true, 12345);

            assert_eq!(positions.len(), 5);
        }

        #[rstest]
        fn returns_fewer_when_not_enough_valid_positions() {
            // Only allow a small area
            let positions = find_valid_spawn_points(
                (10, 10),
                100, // Request more than possible
                |pos| pos.x() == 5 && pos.y() == 5,
                12345,
            );

            assert!(positions.len() <= 1);
        }

        #[rstest]
        fn respects_validity_check() {
            let positions =
                find_valid_spawn_points((80, 40), 5, |pos| pos.x() >= 10 && pos.y() >= 10, 12345);

            for pos in &positions {
                assert!(pos.x() >= 10);
                assert!(pos.y() >= 10);
            }
        }

        #[rstest]
        fn produces_unique_positions() {
            let positions = find_valid_spawn_points((80, 40), 10, |_| true, 12345);

            for (index, pos) in positions.iter().enumerate() {
                for (other_index, other_pos) in positions.iter().enumerate() {
                    if index != other_index {
                        assert_ne!(pos, other_pos);
                    }
                }
            }
        }

        #[rstest]
        fn same_seed_produces_same_result() {
            let positions1 = find_valid_spawn_points((80, 40), 5, |_| true, 12345);
            let positions2 = find_valid_spawn_points((80, 40), 5, |_| true, 12345);

            assert_eq!(positions1, positions2);
        }

        #[rstest]
        fn different_seeds_produce_different_results() {
            let positions1 = find_valid_spawn_points((80, 40), 5, |_| true, 12345);
            let positions2 = find_valid_spawn_points((80, 40), 5, |_| true, 54321);

            assert_ne!(positions1, positions2);
        }
    }

    // =========================================================================
    // generate_enemies Tests
    // =========================================================================

    mod generate_enemies_tests {
        use super::*;

        #[rstest]
        fn generates_enemies_for_each_spawn_point() {
            let config = get_spawn_configuration(5);
            let spawn_points = vec![
                Position::new(10, 10),
                Position::new(20, 20),
                Position::new(30, 30),
            ];

            let enemies = generate_enemies(&config, &spawn_points, 12345);

            assert!(enemies.len() <= spawn_points.len());
        }

        #[rstest]
        fn uses_allowed_enemy_types() {
            let config =
                SpawnConfiguration::new(1, 10, vec![EnemyType::Goblin, EnemyType::Slime], false);
            let spawn_points: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let enemies = generate_enemies(&config, &spawn_points, 12345);

            for enemy in &enemies {
                assert!(
                    enemy.enemy_type() == EnemyType::Goblin
                        || enemy.enemy_type() == EnemyType::Slime
                );
            }
        }

        #[rstest]
        fn assigns_appropriate_behaviors() {
            let config = SpawnConfiguration::new(1, 10, vec![EnemyType::Goblin], false);
            let spawn_points = vec![Position::new(10, 10)];

            let enemies = generate_enemies(&config, &spawn_points, 12345);

            for enemy in &enemies {
                if enemy.enemy_type() == EnemyType::Goblin {
                    assert_eq!(enemy.behavior(), AiBehavior::Aggressive);
                }
            }
        }

        #[rstest]
        fn same_seed_produces_same_enemies() {
            let config = get_spawn_configuration(5);
            let spawn_points = vec![Position::new(10, 10), Position::new(20, 20)];

            let enemies1 = generate_enemies(&config, &spawn_points, 12345);
            let enemies2 = generate_enemies(&config, &spawn_points, 12345);

            assert_eq!(enemies1.len(), enemies2.len());
            for (enemy1, enemy2) in enemies1.iter().zip(enemies2.iter()) {
                assert_eq!(enemy1.enemy_type(), enemy2.enemy_type());
                assert_eq!(enemy1.behavior(), enemy2.behavior());
                assert_eq!(enemy1.position(), enemy2.position());
            }
        }
    }

    // =========================================================================
    // add_enemies_to_session Tests
    // =========================================================================

    mod add_enemies_to_session_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            enemy_count: usize,
        }

        impl MockSession {
            fn new() -> Self {
                Self { enemy_count: 0 }
            }

            fn with_enemy(&self) -> Self {
                Self {
                    enemy_count: self.enemy_count + 1,
                }
            }
        }

        #[rstest]
        fn adds_all_enemies_to_session() {
            let session = MockSession::new();
            let enemies = vec![
                EnemyInstance::new(
                    EntityIdentifier::new(),
                    EnemyType::Goblin,
                    AiBehavior::Aggressive,
                    Position::new(10, 10),
                ),
                EnemyInstance::new(
                    EntityIdentifier::new(),
                    EnemyType::Slime,
                    AiBehavior::Defensive,
                    Position::new(20, 20),
                ),
            ];

            let (updated_session, _) =
                add_enemies_to_session(&session, &enemies, |s, _| s.with_enemy());

            assert_eq!(updated_session.enemy_count, 2);
        }

        #[rstest]
        fn generates_spawn_events_for_all_enemies() {
            let session = MockSession::new();
            let enemies = vec![
                EnemyInstance::new(
                    EntityIdentifier::new(),
                    EnemyType::Goblin,
                    AiBehavior::Aggressive,
                    Position::new(10, 10),
                ),
                EnemyInstance::new(
                    EntityIdentifier::new(),
                    EnemyType::Slime,
                    AiBehavior::Defensive,
                    Position::new(20, 20),
                ),
            ];

            let (_, events) = add_enemies_to_session(&session, &enemies, |s, _| s.with_enemy());

            assert_eq!(events.len(), 2);
            for event in &events {
                assert!(matches!(event, GameSessionEvent::EnemySpawned(_)));
            }
        }

        #[rstest]
        fn empty_enemies_produces_no_changes() {
            let session = MockSession::new();
            let enemies: Vec<EnemyInstance> = vec![];

            let (updated_session, events) =
                add_enemies_to_session(&session, &enemies, |s, _| s.with_enemy());

            assert_eq!(updated_session.enemy_count, 0);
            assert!(events.is_empty());
        }
    }
}
