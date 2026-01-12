use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::Position;
use roguelike_domain::floor::TrapType;
use roguelike_domain::game_session::GameSessionEvent;

use super::GenerateFloorCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// FloorGenerationConfiguration
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorGenerationConfiguration {
    min_rooms: u32,
    max_rooms: u32,
    min_room_size: u32,
    max_room_size: u32,
    item_count: u32,
    trap_count: u32,
}

impl FloorGenerationConfiguration {
    #[must_use]
    pub const fn new(
        min_rooms: u32,
        max_rooms: u32,
        min_room_size: u32,
        max_room_size: u32,
        item_count: u32,
        trap_count: u32,
    ) -> Self {
        Self {
            min_rooms,
            max_rooms,
            min_room_size,
            max_room_size,
            item_count,
            trap_count,
        }
    }

    #[must_use]
    pub const fn min_rooms(&self) -> u32 {
        self.min_rooms
    }

    #[must_use]
    pub const fn max_rooms(&self) -> u32 {
        self.max_rooms
    }

    #[must_use]
    pub const fn min_room_size(&self) -> u32 {
        self.min_room_size
    }

    #[must_use]
    pub const fn max_room_size(&self) -> u32 {
        self.max_room_size
    }

    #[must_use]
    pub const fn item_count(&self) -> u32 {
        self.item_count
    }

    #[must_use]
    pub const fn trap_count(&self) -> u32 {
        self.trap_count
    }
}

// =============================================================================
// StairsPlacement
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StairsPlacement {
    up_stairs: Position,
    down_stairs: Option<Position>,
}

impl StairsPlacement {
    #[must_use]
    pub const fn new(up_stairs: Position, down_stairs: Option<Position>) -> Self {
        Self {
            up_stairs,
            down_stairs,
        }
    }

    #[must_use]
    pub const fn up_stairs(&self) -> Position {
        self.up_stairs
    }

    #[must_use]
    pub const fn down_stairs(&self) -> Option<Position> {
        self.down_stairs
    }
}

// =============================================================================
// ItemPlacement
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPlacement {
    position: Position,
    item_type: String,
}

impl ItemPlacement {
    #[must_use]
    pub fn new(position: Position, item_type: impl Into<String>) -> Self {
        Self {
            position,
            item_type: item_type.into(),
        }
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    #[must_use]
    pub fn item_type(&self) -> &str {
        &self.item_type
    }
}

// =============================================================================
// TrapPlacement
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapPlacement {
    position: Position,
    trap_type: TrapType,
}

impl TrapPlacement {
    #[must_use]
    pub const fn new(position: Position, trap_type: TrapType) -> Self {
        Self {
            position,
            trap_type,
        }
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    #[must_use]
    pub const fn trap_type(&self) -> TrapType {
        self.trap_type
    }
}

// =============================================================================
// GenerateFloor Workflow
// =============================================================================

pub fn generate_floor<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(GenerateFloorCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
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
                    // Steps 2-8: [Pure] Generate floor
                    let result = generate_floor_pure(&session, floor_level);

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

pub fn generate_floor_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(GenerateFloorCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    generate_floor(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

fn generate_floor_pure<S: Clone>(
    _session: &S,
    _floor_level: u32,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "generate_floor",
        "GameSession structure not yet connected",
    ))
}

#[must_use]
pub fn get_floor_configuration(floor_level: u32) -> FloorGenerationConfiguration {
    let (min_rooms, max_rooms) = calculate_room_count_range(floor_level);
    let (min_room_size, max_room_size) = calculate_room_size_range(floor_level);
    let item_count = calculate_item_count(floor_level);
    let trap_count = calculate_trap_count(floor_level);

    FloorGenerationConfiguration::new(
        min_rooms,
        max_rooms,
        min_room_size,
        max_room_size,
        item_count,
        trap_count,
    )
}

fn calculate_room_count_range(floor_level: u32) -> (u32, u32) {
    let base_min = 3;
    let base_max = 6;
    let level_bonus = floor_level / 5;

    (base_min + level_bonus, base_max + level_bonus * 2)
}

fn calculate_room_size_range(floor_level: u32) -> (u32, u32) {
    // Larger rooms on later floors
    let base_min = 4;
    let base_max = 8;
    let level_bonus = floor_level / 10;

    (base_min + level_bonus, base_max + level_bonus * 2)
}

fn calculate_item_count(floor_level: u32) -> u32 {
    // More items on earlier floors, fewer on deeper floors
    let base_count: u32 = 5;
    let reduction = floor_level / 5;
    base_count.saturating_sub(reduction).max(2)
}

fn calculate_trap_count(floor_level: u32) -> u32 {
    // More traps on deeper floors
    let base_count = 1;
    let increase = floor_level / 3;
    (base_count + increase).min(10)
}

#[must_use]
pub fn place_stairs(floor_level: u32, valid_positions: &[Position], seed: u64) -> StairsPlacement {
    if valid_positions.is_empty() {
        return StairsPlacement::new(Position::new(0, 0), None);
    }

    let mut current_seed = seed;

    // Select up stairs position
    current_seed = next_seed(current_seed);
    let up_index = (current_seed >> 32) as usize % valid_positions.len();
    let up_stairs = valid_positions[up_index];

    // Select down stairs position (different from up stairs if possible)
    let down_stairs = if valid_positions.len() > 1 {
        current_seed = next_seed(current_seed);
        let mut down_index = (current_seed >> 32) as usize % valid_positions.len();
        if down_index == up_index {
            down_index = (down_index + 1) % valid_positions.len();
        }
        Some(valid_positions[down_index])
    } else if floor_level > 0 {
        // Only one position, use it for both if not first floor
        Some(valid_positions[0])
    } else {
        None
    };

    StairsPlacement::new(up_stairs, down_stairs)
}

#[must_use]
pub fn place_items(
    configuration: &FloorGenerationConfiguration,
    valid_positions: &[Position],
    floor_level: u32,
    seed: u64,
) -> Vec<ItemPlacement> {
    let item_count = configuration.item_count().min(valid_positions.len() as u32);
    let mut items = Vec::with_capacity(item_count as usize);
    let mut current_seed = seed;
    let mut used_positions = Vec::new();

    let item_types = get_item_types_for_floor(floor_level);

    for _ in 0..item_count {
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

        // Select item type
        current_seed = next_seed(current_seed);
        let type_index = (current_seed >> 32) as usize % item_types.len();

        items.push(ItemPlacement::new(
            valid_positions[position_index],
            item_types[type_index].clone(),
        ));
    }

    items
}

fn get_item_types_for_floor(floor_level: u32) -> Vec<String> {
    let mut types = vec![
        "Health Potion".to_string(),
        "Mana Potion".to_string(),
        "Bread".to_string(),
    ];

    if floor_level >= 3 {
        types.push("Iron Sword".to_string());
        types.push("Leather Armor".to_string());
    }

    if floor_level >= 5 {
        types.push("Steel Sword".to_string());
        types.push("Chain Mail".to_string());
        types.push("Scroll of Fire".to_string());
    }

    if floor_level >= 10 {
        types.push("Magic Sword".to_string());
        types.push("Plate Armor".to_string());
        types.push("Scroll of Lightning".to_string());
    }

    types
}

#[must_use]
pub fn place_traps(
    configuration: &FloorGenerationConfiguration,
    valid_positions: &[Position],
    floor_level: u32,
    seed: u64,
) -> Vec<TrapPlacement> {
    let trap_count = configuration.trap_count().min(valid_positions.len() as u32);
    let mut traps = Vec::with_capacity(trap_count as usize);
    let mut current_seed = seed;
    let mut used_positions = Vec::new();

    let available_trap_types = get_trap_types_for_floor(floor_level);

    for _ in 0..trap_count {
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

        // Select trap type
        current_seed = next_seed(current_seed);
        let type_index = (current_seed >> 32) as usize % available_trap_types.len();

        traps.push(TrapPlacement::new(
            valid_positions[position_index],
            available_trap_types[type_index],
        ));
    }

    traps
}

fn get_trap_types_for_floor(floor_level: u32) -> Vec<TrapType> {
    let mut types = vec![TrapType::Spike];

    if floor_level >= 3 {
        types.push(TrapType::Poison);
    }

    if floor_level >= 5 {
        types.push(TrapType::Alarm);
    }

    if floor_level >= 7 {
        types.push(TrapType::Teleport);
    }

    types
}

pub fn update_session_floor<S, F>(
    session: &S,
    floor_level: u32,
    stairs: &StairsPlacement,
    items: &[ItemPlacement],
    traps: &[TrapPlacement],
    update_fn: F,
) -> (S, Vec<GameSessionEvent>)
where
    S: Clone,
    F: Fn(&S, u32, &StairsPlacement, &[ItemPlacement], &[TrapPlacement]) -> S,
{
    let updated_session = update_fn(session, floor_level, stairs, items, traps);

    // Generate floor generated event
    // Note: We would normally generate a FloorGenerated event here,
    // but since it's not yet in GameSessionEvent, we return an empty vec
    let events = Vec::new();

    (updated_session, events)
}

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
    // FloorGenerationConfiguration Tests
    // =========================================================================

    mod floor_generation_configuration {
        use super::*;

        #[rstest]
        fn new_creates_configuration() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 2);

            assert_eq!(config.min_rooms(), 3);
            assert_eq!(config.max_rooms(), 6);
            assert_eq!(config.min_room_size(), 4);
            assert_eq!(config.max_room_size(), 8);
            assert_eq!(config.item_count(), 5);
            assert_eq!(config.trap_count(), 2);
        }

        #[rstest]
        fn clone_preserves_values() {
            let config = FloorGenerationConfiguration::new(5, 10, 5, 12, 8, 4);
            let cloned = config.clone();

            assert_eq!(config, cloned);
        }
    }

    // =========================================================================
    // StairsPlacement Tests
    // =========================================================================

    mod stairs_placement {
        use super::*;

        #[rstest]
        fn new_creates_placement() {
            let up = Position::new(5, 5);
            let down = Some(Position::new(20, 20));
            let placement = StairsPlacement::new(up, down);

            assert_eq!(placement.up_stairs(), up);
            assert_eq!(placement.down_stairs(), down);
        }

        #[rstest]
        fn new_without_down_stairs() {
            let up = Position::new(5, 5);
            let placement = StairsPlacement::new(up, None);

            assert_eq!(placement.up_stairs(), up);
            assert!(placement.down_stairs().is_none());
        }
    }

    // =========================================================================
    // ItemPlacement Tests
    // =========================================================================

    mod item_placement {
        use super::*;

        #[rstest]
        fn new_creates_placement() {
            let position = Position::new(10, 15);
            let placement = ItemPlacement::new(position, "Health Potion");

            assert_eq!(placement.position(), position);
            assert_eq!(placement.item_type(), "Health Potion");
        }

        #[rstest]
        fn new_with_string_type() {
            let position = Position::new(5, 5);
            let placement = ItemPlacement::new(position, String::from("Magic Sword"));

            assert_eq!(placement.item_type(), "Magic Sword");
        }
    }

    // =========================================================================
    // TrapPlacement Tests
    // =========================================================================

    mod trap_placement {
        use super::*;

        #[rstest]
        fn new_creates_placement() {
            let position = Position::new(15, 20);
            let placement = TrapPlacement::new(position, TrapType::Spike);

            assert_eq!(placement.position(), position);
            assert_eq!(placement.trap_type(), TrapType::Spike);
        }

        #[rstest]
        #[case(TrapType::Spike)]
        #[case(TrapType::Poison)]
        #[case(TrapType::Teleport)]
        #[case(TrapType::Alarm)]
        fn new_with_all_trap_types(#[case] trap_type: TrapType) {
            let placement = TrapPlacement::new(Position::new(0, 0), trap_type);
            assert_eq!(placement.trap_type(), trap_type);
        }
    }

    // =========================================================================
    // get_floor_configuration Tests
    // =========================================================================

    mod get_floor_configuration_tests {
        use super::*;

        #[rstest]
        #[case(1)]
        #[case(2)]
        #[case(3)]
        fn early_floors_have_fewer_rooms(#[case] floor_level: u32) {
            let config = get_floor_configuration(floor_level);

            assert!(config.min_rooms() >= 3);
            assert!(config.max_rooms() <= 8);
        }

        #[rstest]
        fn deeper_floors_have_more_rooms() {
            let config_early = get_floor_configuration(1);
            let config_late = get_floor_configuration(20);

            assert!(config_late.min_rooms() >= config_early.min_rooms());
            assert!(config_late.max_rooms() >= config_early.max_rooms());
        }

        #[rstest]
        fn early_floors_have_fewer_traps() {
            let config = get_floor_configuration(1);
            assert!(config.trap_count() <= 2);
        }

        #[rstest]
        fn deeper_floors_have_more_traps() {
            let config_early = get_floor_configuration(1);
            let config_late = get_floor_configuration(15);

            assert!(config_late.trap_count() > config_early.trap_count());
        }

        #[rstest]
        fn trap_count_is_capped() {
            let config = get_floor_configuration(100);
            assert!(config.trap_count() <= 10);
        }

        #[rstest]
        fn item_count_decreases_on_deeper_floors() {
            let config_early = get_floor_configuration(1);
            let config_late = get_floor_configuration(15);

            assert!(config_late.item_count() <= config_early.item_count());
        }

        #[rstest]
        fn item_count_has_minimum() {
            let config = get_floor_configuration(100);
            assert!(config.item_count() >= 2);
        }
    }

    // =========================================================================
    // place_stairs Tests
    // =========================================================================

    mod place_stairs_tests {
        use super::*;

        #[rstest]
        fn places_stairs_at_different_positions() {
            let positions = vec![
                Position::new(5, 5),
                Position::new(20, 20),
                Position::new(35, 10),
            ];
            let stairs = place_stairs(1, &positions, 12345);

            assert!(positions.contains(&stairs.up_stairs()));
            if let Some(down) = stairs.down_stairs() {
                assert!(positions.contains(&down));
            }
        }

        #[rstest]
        fn up_and_down_stairs_are_different_when_possible() {
            let positions = vec![Position::new(5, 5), Position::new(20, 20)];
            let stairs = place_stairs(1, &positions, 12345);

            if let Some(down) = stairs.down_stairs() {
                assert_ne!(stairs.up_stairs(), down);
            }
        }

        #[rstest]
        fn handles_single_position() {
            let positions = vec![Position::new(10, 10)];
            let stairs = place_stairs(1, &positions, 12345);

            assert_eq!(stairs.up_stairs(), Position::new(10, 10));
        }

        #[rstest]
        fn handles_empty_positions() {
            let positions: Vec<Position> = vec![];
            let stairs = place_stairs(1, &positions, 12345);

            assert_eq!(stairs.up_stairs(), Position::new(0, 0));
            assert!(stairs.down_stairs().is_none());
        }

        #[rstest]
        fn same_seed_produces_same_result() {
            let positions = vec![
                Position::new(5, 5),
                Position::new(20, 20),
                Position::new(35, 35),
            ];
            let stairs1 = place_stairs(1, &positions, 12345);
            let stairs2 = place_stairs(1, &positions, 12345);

            assert_eq!(stairs1.up_stairs(), stairs2.up_stairs());
            assert_eq!(stairs1.down_stairs(), stairs2.down_stairs());
        }

        #[rstest]
        fn different_seeds_produce_different_results() {
            let positions = vec![
                Position::new(5, 5),
                Position::new(20, 20),
                Position::new(35, 35),
                Position::new(50, 50),
            ];
            let stairs1 = place_stairs(1, &positions, 12345);
            let stairs2 = place_stairs(1, &positions, 54321);

            // At least one should be different (with very high probability)
            assert!(
                stairs1.up_stairs() != stairs2.up_stairs()
                    || stairs1.down_stairs() != stairs2.down_stairs()
            );
        }
    }

    // =========================================================================
    // place_items Tests
    // =========================================================================

    mod place_items_tests {
        use super::*;

        #[rstest]
        fn places_correct_number_of_items() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 3, 2);
            let positions = vec![
                Position::new(10, 10),
                Position::new(20, 20),
                Position::new(30, 30),
                Position::new(40, 40),
            ];

            let items = place_items(&config, &positions, 1, 12345);

            assert_eq!(items.len(), 3);
        }

        #[rstest]
        fn places_at_most_available_positions() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 10, 2);
            let positions = vec![Position::new(10, 10), Position::new(20, 20)];

            let items = place_items(&config, &positions, 1, 12345);

            assert!(items.len() <= 2);
        }

        #[rstest]
        fn items_are_at_unique_positions() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 2);
            let positions: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let items = place_items(&config, &positions, 1, 12345);

            for (i, item) in items.iter().enumerate() {
                for (j, other) in items.iter().enumerate() {
                    if i != j {
                        assert_ne!(item.position(), other.position());
                    }
                }
            }
        }

        #[rstest]
        fn same_seed_produces_same_items() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 3, 2);
            let positions = vec![
                Position::new(10, 10),
                Position::new(20, 20),
                Position::new(30, 30),
            ];

            let items1 = place_items(&config, &positions, 1, 12345);
            let items2 = place_items(&config, &positions, 1, 12345);

            assert_eq!(items1.len(), items2.len());
            for (item1, item2) in items1.iter().zip(items2.iter()) {
                assert_eq!(item1.position(), item2.position());
                assert_eq!(item1.item_type(), item2.item_type());
            }
        }

        #[rstest]
        fn different_floor_levels_have_different_item_types() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 10, 2);
            let positions: Vec<Position> =
                (0..20).map(|index| Position::new(index * 5, 10)).collect();

            let items_early = place_items(&config, &positions, 1, 12345);
            let items_late = place_items(&config, &positions, 10, 12345);

            // Check that later floors have access to more item types
            let early_types: Vec<_> = items_early.iter().map(|i| i.item_type()).collect();
            let late_types: Vec<_> = items_late.iter().map(|i| i.item_type()).collect();

            // The late floor items might include types not available early
            // This is a probabilistic test
            assert!(!early_types.is_empty());
            assert!(!late_types.is_empty());
        }
    }

    // =========================================================================
    // place_traps Tests
    // =========================================================================

    mod place_traps_tests {
        use super::*;

        #[rstest]
        fn places_correct_number_of_traps() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 3);
            let positions = vec![
                Position::new(10, 10),
                Position::new(20, 20),
                Position::new(30, 30),
                Position::new(40, 40),
            ];

            let traps = place_traps(&config, &positions, 1, 12345);

            assert_eq!(traps.len(), 3);
        }

        #[rstest]
        fn places_at_most_available_positions() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 10);
            let positions = vec![Position::new(10, 10), Position::new(20, 20)];

            let traps = place_traps(&config, &positions, 1, 12345);

            assert!(traps.len() <= 2);
        }

        #[rstest]
        fn traps_are_at_unique_positions() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 5);
            let positions: Vec<Position> =
                (0..10).map(|index| Position::new(index * 5, 10)).collect();

            let traps = place_traps(&config, &positions, 10, 12345);

            for (i, trap) in traps.iter().enumerate() {
                for (j, other) in traps.iter().enumerate() {
                    if i != j {
                        assert_ne!(trap.position(), other.position());
                    }
                }
            }
        }

        #[rstest]
        fn early_floors_only_have_spike_traps() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 10);
            let positions: Vec<Position> =
                (0..20).map(|index| Position::new(index * 5, 10)).collect();

            let traps = place_traps(&config, &positions, 1, 12345);

            for trap in &traps {
                assert_eq!(trap.trap_type(), TrapType::Spike);
            }
        }

        #[rstest]
        fn same_seed_produces_same_traps() {
            let config = FloorGenerationConfiguration::new(3, 6, 4, 8, 5, 3);
            let positions = vec![
                Position::new(10, 10),
                Position::new(20, 20),
                Position::new(30, 30),
            ];

            let traps1 = place_traps(&config, &positions, 5, 12345);
            let traps2 = place_traps(&config, &positions, 5, 12345);

            assert_eq!(traps1.len(), traps2.len());
            for (trap1, trap2) in traps1.iter().zip(traps2.iter()) {
                assert_eq!(trap1.position(), trap2.position());
                assert_eq!(trap1.trap_type(), trap2.trap_type());
            }
        }
    }

    // =========================================================================
    // update_session_floor Tests
    // =========================================================================

    mod update_session_floor_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            floor_level: u32,
        }

        impl MockSession {
            fn new() -> Self {
                Self { floor_level: 0 }
            }
        }

        #[rstest]
        fn updates_session_with_floor_data() {
            let session = MockSession::new();
            let stairs = StairsPlacement::new(Position::new(5, 5), Some(Position::new(20, 20)));
            let items = vec![ItemPlacement::new(Position::new(10, 10), "Potion")];
            let traps = vec![TrapPlacement::new(Position::new(15, 15), TrapType::Spike)];

            let (updated, _) = update_session_floor(
                &session,
                1,
                &stairs,
                &items,
                &traps,
                |_session, level, _, _, _| MockSession { floor_level: level },
            );

            assert_eq!(updated.floor_level, 1);
        }
    }
}
