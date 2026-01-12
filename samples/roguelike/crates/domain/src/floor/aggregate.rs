use std::fmt;

use crate::common::{FloorLevel, Position};

use super::{Corridor, FloorError, FloorIdentifier, Room, Tile, TileKind};

// =============================================================================
// Floor Aggregate
// =============================================================================

#[derive(Debug, Clone)]
pub struct Floor {
    identifier: FloorIdentifier,
    level: FloorLevel,
    tiles: Vec<Vec<Tile>>,
    rooms: Vec<Room>,
    corridors: Vec<Corridor>,
    spawn_points: Vec<Position>,
    stairs_up: Option<Position>,
    stairs_down: Option<Position>,
}

impl Floor {
    // =========================================================================
    // Constructor
    // =========================================================================

    #[must_use]
    pub fn new(identifier: FloorIdentifier, level: FloorLevel, width: u32, height: u32) -> Self {
        let tiles = (0..height)
            .map(|_| (0..width).map(|_| Tile::new(TileKind::Wall)).collect())
            .collect();

        Self {
            identifier,
            level,
            tiles,
            rooms: Vec::new(),
            corridors: Vec::new(),
            spawn_points: Vec::new(),
            stairs_up: None,
            stairs_down: None,
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    #[must_use]
    pub fn with_tiles(mut self, tiles: Vec<Vec<Tile>>) -> Self {
        self.tiles = tiles;
        self
    }

    #[must_use]
    pub fn with_rooms(mut self, rooms: Vec<Room>) -> Self {
        self.rooms = rooms;
        self
    }

    #[must_use]
    pub fn with_corridors(mut self, corridors: Vec<Corridor>) -> Self {
        self.corridors = corridors;
        self
    }

    #[must_use]
    pub fn with_spawn_points(mut self, spawn_points: Vec<Position>) -> Self {
        self.spawn_points = spawn_points;
        self
    }

    #[must_use]
    pub fn with_stairs_up(mut self, position: Position) -> Self {
        self.stairs_up = Some(position);
        self
    }

    #[must_use]
    pub fn with_stairs_down(mut self, position: Position) -> Self {
        self.stairs_down = Some(position);
        self
    }

    // =========================================================================
    // Getters
    // =========================================================================

    #[must_use]
    pub const fn identifier(&self) -> &FloorIdentifier {
        &self.identifier
    }

    #[must_use]
    pub const fn level(&self) -> FloorLevel {
        self.level
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.tiles.first().map_or(0, |row| row.len() as u32)
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.tiles.len() as u32
    }

    #[must_use]
    pub const fn tiles(&self) -> &Vec<Vec<Tile>> {
        &self.tiles
    }

    #[must_use]
    pub fn rooms(&self) -> &[Room] {
        &self.rooms
    }

    #[must_use]
    pub fn corridors(&self) -> &[Corridor] {
        &self.corridors
    }

    #[must_use]
    pub fn spawn_points(&self) -> &[Position] {
        &self.spawn_points
    }

    #[must_use]
    pub const fn stairs_up(&self) -> Option<&Position> {
        self.stairs_up.as_ref()
    }

    #[must_use]
    pub const fn stairs_down(&self) -> Option<&Position> {
        self.stairs_down.as_ref()
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    #[must_use]
    pub fn get_tile(&self, position: &Position) -> Option<&Tile> {
        if !self.is_in_bounds(position) {
            return None;
        }

        let y = position.y() as usize;
        let x = position.x() as usize;

        self.tiles.get(y).and_then(|row| row.get(x))
    }

    #[must_use]
    pub fn is_walkable(&self, position: &Position) -> bool {
        self.get_tile(position)
            .is_some_and(|tile| tile.is_walkable())
    }

    #[must_use]
    pub fn is_in_bounds(&self, position: &Position) -> bool {
        let x = position.x();
        let y = position.y();

        x >= 0 && y >= 0 && (x as u32) < self.width() && (y as u32) < self.height()
    }

    // =========================================================================
    // Domain Methods (Pure Functions)
    // =========================================================================

    pub fn set_tile(self, position: Position, tile: Tile) -> Result<Self, FloorError> {
        if !self.is_in_bounds(&position) {
            return Err(FloorError::position_out_of_bounds(
                (position.x(), position.y()),
                (self.width(), self.height()),
            ));
        }

        let y = position.y() as usize;
        let x = position.x() as usize;

        let mut new_tiles = self.tiles;
        new_tiles[y][x] = tile;

        Ok(Self {
            tiles: new_tiles,
            ..self
        })
    }

    pub fn explore_tile(self, position: Position) -> Result<Self, FloorError> {
        if !self.is_in_bounds(&position) {
            return Err(FloorError::position_out_of_bounds(
                (position.x(), position.y()),
                (self.width(), self.height()),
            ));
        }

        let y = position.y() as usize;
        let x = position.x() as usize;

        let mut new_tiles = self.tiles;
        new_tiles[y][x] = new_tiles[y][x].mark_explored();

        Ok(Self {
            tiles: new_tiles,
            ..self
        })
    }

    pub fn set_tile_visibility(
        self,
        position: Position,
        visible: bool,
    ) -> Result<Self, FloorError> {
        if !self.is_in_bounds(&position) {
            return Err(FloorError::position_out_of_bounds(
                (position.x(), position.y()),
                (self.width(), self.height()),
            ));
        }

        let y = position.y() as usize;
        let x = position.x() as usize;

        let mut new_tiles = self.tiles;
        new_tiles[y][x] = new_tiles[y][x].set_visible(visible);

        Ok(Self {
            tiles: new_tiles,
            ..self
        })
    }
}

impl fmt::Display for Floor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Floor {} (Level {}, {}x{})",
            self.identifier,
            self.level.value(),
            self.width(),
            self.height()
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_floor(width: u32, height: u32) -> Floor {
        Floor::new(
            FloorIdentifier::new(1),
            FloorLevel::new(1).unwrap(),
            width,
            height,
        )
    }

    fn create_floor_with_mixed_tiles() -> Floor {
        let tiles = vec![
            vec![
                Tile::new(TileKind::Wall),
                Tile::new(TileKind::Floor),
                Tile::new(TileKind::Wall),
            ],
            vec![
                Tile::new(TileKind::Floor),
                Tile::new(TileKind::Floor),
                Tile::new(TileKind::Floor),
            ],
            vec![
                Tile::new(TileKind::Wall),
                Tile::new(TileKind::Floor),
                Tile::new(TileKind::Wall),
            ],
        ];

        Floor::new(FloorIdentifier::new(1), FloorLevel::new(1).unwrap(), 3, 3).with_tiles(tiles)
    }

    // =========================================================================
    // Constructor Tests
    // =========================================================================

    mod constructor {
        use super::*;

        #[rstest]
        fn new_creates_floor_with_correct_dimensions() {
            let floor = create_floor(10, 8);
            assert_eq!(floor.width(), 10);
            assert_eq!(floor.height(), 8);
        }

        #[rstest]
        fn new_initializes_all_tiles_as_walls() {
            let floor = create_floor(5, 5);

            for y in 0..5 {
                for x in 0..5 {
                    let position = Position::new(x, y);
                    let tile = floor.get_tile(&position).unwrap();
                    assert_eq!(tile.kind(), TileKind::Wall);
                }
            }
        }

        #[rstest]
        fn new_initializes_empty_rooms() {
            let floor = create_floor(10, 10);
            assert!(floor.rooms().is_empty());
        }

        #[rstest]
        fn new_initializes_empty_corridors() {
            let floor = create_floor(10, 10);
            assert!(floor.corridors().is_empty());
        }

        #[rstest]
        fn new_initializes_empty_spawn_points() {
            let floor = create_floor(10, 10);
            assert!(floor.spawn_points().is_empty());
        }

        #[rstest]
        fn new_initializes_no_stairs() {
            let floor = create_floor(10, 10);
            assert!(floor.stairs_up().is_none());
            assert!(floor.stairs_down().is_none());
        }

        #[rstest]
        fn new_preserves_identifier() {
            let identifier = FloorIdentifier::new(42);
            let floor = Floor::new(identifier, FloorLevel::new(1).unwrap(), 10, 10);
            assert_eq!(*floor.identifier(), identifier);
        }

        #[rstest]
        fn new_preserves_level() {
            let level = FloorLevel::new(5).unwrap();
            let floor = Floor::new(FloorIdentifier::new(1), level, 10, 10);
            assert_eq!(floor.level(), level);
        }
    }

    // =========================================================================
    // Builder Tests
    // =========================================================================

    mod builder {
        use super::*;

        #[rstest]
        fn with_tiles_replaces_tiles() {
            let tiles = vec![
                vec![Tile::new(TileKind::Floor), Tile::new(TileKind::Floor)],
                vec![Tile::new(TileKind::Floor), Tile::new(TileKind::Floor)],
            ];

            let floor = create_floor(2, 2).with_tiles(tiles);

            for y in 0..2 {
                for x in 0..2 {
                    let position = Position::new(x, y);
                    let tile = floor.get_tile(&position).unwrap();
                    assert_eq!(tile.kind(), TileKind::Floor);
                }
            }
        }

        #[rstest]
        fn with_rooms_sets_rooms() {
            let rooms = vec![
                Room::new(Position::new(1, 1), 5, 5).unwrap(),
                Room::new(Position::new(10, 10), 4, 4).unwrap(),
            ];

            let floor = create_floor(20, 20).with_rooms(rooms);
            assert_eq!(floor.rooms().len(), 2);
        }

        #[rstest]
        fn with_corridors_sets_corridors() {
            let corridors = vec![
                Corridor::new(Position::new(5, 3), Position::new(10, 3)),
                Corridor::new(Position::new(7, 3), Position::new(7, 8)),
            ];

            let floor = create_floor(20, 20).with_corridors(corridors);
            assert_eq!(floor.corridors().len(), 2);
        }

        #[rstest]
        fn with_spawn_points_sets_spawn_points() {
            let spawn_points = vec![
                Position::new(5, 5),
                Position::new(10, 10),
                Position::new(15, 15),
            ];

            let floor = create_floor(20, 20).with_spawn_points(spawn_points);
            assert_eq!(floor.spawn_points().len(), 3);
        }

        #[rstest]
        fn with_stairs_up_sets_stairs() {
            let floor = create_floor(20, 20).with_stairs_up(Position::new(3, 3));
            assert_eq!(floor.stairs_up(), Some(&Position::new(3, 3)));
        }

        #[rstest]
        fn with_stairs_down_sets_stairs() {
            let floor = create_floor(20, 20).with_stairs_down(Position::new(15, 15));
            assert_eq!(floor.stairs_down(), Some(&Position::new(15, 15)));
        }

        #[rstest]
        fn builder_methods_can_be_chained() {
            let rooms = vec![Room::new(Position::new(1, 1), 5, 5).unwrap()];
            let corridors = vec![Corridor::new(Position::new(5, 3), Position::new(10, 3))];
            let spawn_points = vec![Position::new(5, 5)];

            let floor = create_floor(20, 20)
                .with_rooms(rooms)
                .with_corridors(corridors)
                .with_spawn_points(spawn_points)
                .with_stairs_up(Position::new(3, 3))
                .with_stairs_down(Position::new(15, 15));

            assert_eq!(floor.rooms().len(), 1);
            assert_eq!(floor.corridors().len(), 1);
            assert_eq!(floor.spawn_points().len(), 1);
            assert!(floor.stairs_up().is_some());
            assert!(floor.stairs_down().is_some());
        }
    }

    // =========================================================================
    // Query Tests
    // =========================================================================

    mod query {
        use super::*;

        #[rstest]
        fn get_tile_returns_tile_at_position() {
            let floor = create_floor_with_mixed_tiles();

            let tile = floor.get_tile(&Position::new(1, 1)).unwrap();
            assert_eq!(tile.kind(), TileKind::Floor);

            let wall = floor.get_tile(&Position::new(0, 0)).unwrap();
            assert_eq!(wall.kind(), TileKind::Wall);
        }

        #[rstest]
        fn get_tile_returns_none_for_negative_x() {
            let floor = create_floor(10, 10);
            assert!(floor.get_tile(&Position::new(-1, 5)).is_none());
        }

        #[rstest]
        fn get_tile_returns_none_for_negative_y() {
            let floor = create_floor(10, 10);
            assert!(floor.get_tile(&Position::new(5, -1)).is_none());
        }

        #[rstest]
        fn get_tile_returns_none_for_x_out_of_bounds() {
            let floor = create_floor(10, 10);
            assert!(floor.get_tile(&Position::new(10, 5)).is_none());
        }

        #[rstest]
        fn get_tile_returns_none_for_y_out_of_bounds() {
            let floor = create_floor(10, 10);
            assert!(floor.get_tile(&Position::new(5, 10)).is_none());
        }

        #[rstest]
        fn is_walkable_returns_true_for_floor_tile() {
            let floor = create_floor_with_mixed_tiles();
            assert!(floor.is_walkable(&Position::new(1, 1)));
        }

        #[rstest]
        fn is_walkable_returns_false_for_wall_tile() {
            let floor = create_floor_with_mixed_tiles();
            assert!(!floor.is_walkable(&Position::new(0, 0)));
        }

        #[rstest]
        fn is_walkable_returns_false_for_out_of_bounds() {
            let floor = create_floor(10, 10);
            assert!(!floor.is_walkable(&Position::new(100, 100)));
        }

        #[rstest]
        fn is_in_bounds_returns_true_for_origin() {
            let floor = create_floor(10, 10);
            assert!(floor.is_in_bounds(&Position::new(0, 0)));
        }

        #[rstest]
        fn is_in_bounds_returns_true_for_max_valid_position() {
            let floor = create_floor(10, 10);
            assert!(floor.is_in_bounds(&Position::new(9, 9)));
        }

        #[rstest]
        fn is_in_bounds_returns_false_for_negative_position() {
            let floor = create_floor(10, 10);
            assert!(!floor.is_in_bounds(&Position::new(-1, -1)));
        }

        #[rstest]
        fn is_in_bounds_returns_false_for_exceeding_width() {
            let floor = create_floor(10, 10);
            assert!(!floor.is_in_bounds(&Position::new(10, 5)));
        }

        #[rstest]
        fn is_in_bounds_returns_false_for_exceeding_height() {
            let floor = create_floor(10, 10);
            assert!(!floor.is_in_bounds(&Position::new(5, 10)));
        }
    }

    // =========================================================================
    // Domain Method Tests
    // =========================================================================

    mod domain_methods {
        use super::*;

        #[rstest]
        fn set_tile_changes_tile() {
            let floor = create_floor(10, 10);
            let position = Position::new(5, 5);

            let new_floor = floor
                .set_tile(position, Tile::new(TileKind::Floor))
                .unwrap();

            assert_eq!(
                new_floor.get_tile(&position).unwrap().kind(),
                TileKind::Floor
            );
        }

        #[rstest]
        fn set_tile_preserves_other_tiles() {
            let floor = create_floor(10, 10);
            let target = Position::new(5, 5);
            let other = Position::new(3, 3);

            let new_floor = floor.set_tile(target, Tile::new(TileKind::Floor)).unwrap();

            // Other tile should still be a wall
            assert_eq!(new_floor.get_tile(&other).unwrap().kind(), TileKind::Wall);
        }

        #[rstest]
        fn set_tile_returns_error_for_out_of_bounds() {
            let floor = create_floor(10, 10);
            let position = Position::new(100, 100);

            let result = floor.set_tile(position, Tile::new(TileKind::Floor));
            assert!(result.is_err());

            match result.unwrap_err() {
                FloorError::PositionOutOfBounds { position: pos, .. } => {
                    assert_eq!(pos, (100, 100));
                }
                _ => panic!("Expected PositionOutOfBounds error"),
            }
        }

        #[rstest]
        fn explore_tile_marks_tile_as_explored() {
            let floor = create_floor(10, 10);
            let position = Position::new(5, 5);

            // Initially not explored
            assert!(!floor.get_tile(&position).unwrap().is_explored());

            let new_floor = floor.explore_tile(position).unwrap();
            assert!(new_floor.get_tile(&position).unwrap().is_explored());
        }

        #[rstest]
        fn explore_tile_preserves_other_tiles() {
            let floor = create_floor(10, 10);
            let target = Position::new(5, 5);
            let other = Position::new(3, 3);

            let new_floor = floor.explore_tile(target).unwrap();

            // Other tile should still be unexplored
            assert!(!new_floor.get_tile(&other).unwrap().is_explored());
        }

        #[rstest]
        fn explore_tile_returns_error_for_out_of_bounds() {
            let floor = create_floor(10, 10);
            let position = Position::new(100, 100);

            let result = floor.explore_tile(position);
            assert!(result.is_err());
        }

        #[rstest]
        fn set_tile_visibility_true_marks_visible() {
            let floor = create_floor(10, 10);
            let position = Position::new(5, 5);

            let new_floor = floor.set_tile_visibility(position, true).unwrap();
            let tile = new_floor.get_tile(&position).unwrap();

            assert!(tile.is_visible());
        }

        #[rstest]
        fn set_tile_visibility_true_also_marks_explored() {
            let floor = create_floor(10, 10);
            let position = Position::new(5, 5);

            let new_floor = floor.set_tile_visibility(position, true).unwrap();
            let tile = new_floor.get_tile(&position).unwrap();

            assert!(tile.is_explored());
        }

        #[rstest]
        fn set_tile_visibility_false_hides_tile() {
            let floor = create_floor(10, 10);
            let position = Position::new(5, 5);

            let visible_floor = floor.set_tile_visibility(position, true).unwrap();
            let hidden_floor = visible_floor.set_tile_visibility(position, false).unwrap();
            let tile = hidden_floor.get_tile(&position).unwrap();

            assert!(!tile.is_visible());
            // Still explored even when not visible
            assert!(tile.is_explored());
        }

        #[rstest]
        fn set_tile_visibility_returns_error_for_out_of_bounds() {
            let floor = create_floor(10, 10);
            let position = Position::new(100, 100);

            let result = floor.set_tile_visibility(position, true);
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn display_format() {
            let floor = Floor::new(
                FloorIdentifier::new(42),
                FloorLevel::new(3).unwrap(),
                80,
                40,
            );

            let display = format!("{}", floor);
            assert!(display.contains("Floor#42"));
            assert!(display.contains("Level 3"));
            assert!(display.contains("80x40"));
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone {
        use super::*;

        #[rstest]
        fn clone_creates_independent_copy() {
            let floor = create_floor(10, 10).with_stairs_up(Position::new(3, 3));

            let cloned = floor.clone();

            assert_eq!(*cloned.identifier(), *floor.identifier());
            assert_eq!(cloned.level(), floor.level());
            assert_eq!(cloned.width(), floor.width());
            assert_eq!(cloned.height(), floor.height());
            assert_eq!(cloned.stairs_up(), floor.stairs_up());
        }
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    mod edge_cases {
        use super::*;

        #[rstest]
        fn floor_with_zero_dimensions_has_zero_size() {
            let floor = create_floor(0, 0);
            assert_eq!(floor.width(), 0);
            assert_eq!(floor.height(), 0);
        }

        #[rstest]
        fn floor_with_width_one_height_one() {
            let floor = create_floor(1, 1);
            assert_eq!(floor.width(), 1);
            assert_eq!(floor.height(), 1);
            assert!(floor.is_in_bounds(&Position::new(0, 0)));
            assert!(!floor.is_in_bounds(&Position::new(1, 0)));
        }
    }
}
