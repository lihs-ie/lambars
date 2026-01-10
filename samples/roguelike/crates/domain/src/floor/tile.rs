//! Tile types for dungeon floors.
//!
//! This module provides types for representing tiles in the dungeon:
//! - `Tile`: A single tile with its kind and visibility state
//! - `TileKind`: The type of tile (floor, wall, door, stairs, trap)
//! - `TrapType`: The type of trap

use std::fmt;

// =============================================================================
// TrapType
// =============================================================================

/// The type of trap on a tile.
///
/// Traps cause various effects when triggered by a player or enemy.
///
/// # Examples
///
/// ```
/// use roguelike_domain::floor::TrapType;
///
/// let trap = TrapType::Spike;
/// assert_eq!(format!("{}", trap), "Spike Trap");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrapType {
    /// A spike trap that deals physical damage.
    Spike,
    /// A poison trap that applies poison status.
    Poison,
    /// A teleport trap that moves the target to a random location.
    Teleport,
    /// An alarm trap that alerts nearby enemies.
    Alarm,
}

impl TrapType {
    /// Returns all trap types.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TrapType;
    ///
    /// let types = TrapType::all();
    /// assert_eq!(types.len(), 4);
    /// ```
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Spike, Self::Poison, Self::Teleport, Self::Alarm]
    }
}

impl fmt::Display for TrapType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Spike => "Spike Trap",
            Self::Poison => "Poison Trap",
            Self::Teleport => "Teleport Trap",
            Self::Alarm => "Alarm Trap",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// TileKind
// =============================================================================

/// The kind of tile on the dungeon floor.
///
/// Each tile kind has different properties for movement and interaction.
///
/// # Examples
///
/// ```
/// use roguelike_domain::floor::TileKind;
///
/// let floor = TileKind::Floor;
/// assert!(floor.is_walkable());
///
/// let wall = TileKind::Wall;
/// assert!(!wall.is_walkable());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    /// A walkable floor tile.
    Floor,
    /// A solid wall that blocks movement.
    Wall,
    /// A door that can be opened or closed.
    Door {
        /// Whether the door is currently open.
        is_open: bool,
    },
    /// Stairs leading up to the previous floor.
    StairsUp,
    /// Stairs leading down to the next floor.
    StairsDown,
    /// A trap tile with a specific trap type.
    Trap {
        /// The type of trap.
        trap_type: TrapType,
    },
}

impl TileKind {
    /// Returns true if this tile can be walked on.
    ///
    /// Walls and closed doors are not walkable.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TileKind;
    ///
    /// assert!(TileKind::Floor.is_walkable());
    /// assert!(!TileKind::Wall.is_walkable());
    /// assert!(TileKind::Door { is_open: true }.is_walkable());
    /// assert!(!TileKind::Door { is_open: false }.is_walkable());
    /// ```
    #[must_use]
    pub const fn is_walkable(&self) -> bool {
        match self {
            Self::Floor | Self::StairsUp | Self::StairsDown | Self::Trap { .. } => true,
            Self::Door { is_open } => *is_open,
            Self::Wall => false,
        }
    }

    /// Returns true if this tile blocks line of sight.
    ///
    /// Walls and closed doors block visibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TileKind;
    ///
    /// assert!(TileKind::Wall.blocks_sight());
    /// assert!(TileKind::Door { is_open: false }.blocks_sight());
    /// assert!(!TileKind::Floor.blocks_sight());
    /// ```
    #[must_use]
    pub const fn blocks_sight(&self) -> bool {
        match self {
            Self::Wall => true,
            Self::Door { is_open } => !*is_open,
            Self::Floor | Self::StairsUp | Self::StairsDown | Self::Trap { .. } => false,
        }
    }

    /// Returns true if this tile is a door.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TileKind;
    ///
    /// assert!(TileKind::Door { is_open: true }.is_door());
    /// assert!(!TileKind::Floor.is_door());
    /// ```
    #[must_use]
    pub const fn is_door(&self) -> bool {
        matches!(self, Self::Door { .. })
    }

    /// Returns true if this tile is stairs (up or down).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TileKind;
    ///
    /// assert!(TileKind::StairsUp.is_stairs());
    /// assert!(TileKind::StairsDown.is_stairs());
    /// assert!(!TileKind::Floor.is_stairs());
    /// ```
    #[must_use]
    pub const fn is_stairs(&self) -> bool {
        matches!(self, Self::StairsUp | Self::StairsDown)
    }

    /// Returns true if this tile is a trap.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{TileKind, TrapType};
    ///
    /// let trap = TileKind::Trap { trap_type: TrapType::Spike };
    /// assert!(trap.is_trap());
    /// assert!(!TileKind::Floor.is_trap());
    /// ```
    #[must_use]
    pub const fn is_trap(&self) -> bool {
        matches!(self, Self::Trap { .. })
    }

    /// Returns the trap type if this tile is a trap.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{TileKind, TrapType};
    ///
    /// let trap = TileKind::Trap { trap_type: TrapType::Poison };
    /// assert_eq!(trap.trap_type(), Some(TrapType::Poison));
    /// assert_eq!(TileKind::Floor.trap_type(), None);
    /// ```
    #[must_use]
    pub const fn trap_type(&self) -> Option<TrapType> {
        match self {
            Self::Trap { trap_type } => Some(*trap_type),
            _ => None,
        }
    }

    /// Creates a new door tile.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::TileKind;
    ///
    /// let closed_door = TileKind::door(false);
    /// assert!(!closed_door.is_walkable());
    ///
    /// let open_door = TileKind::door(true);
    /// assert!(open_door.is_walkable());
    /// ```
    #[must_use]
    pub const fn door(is_open: bool) -> Self {
        Self::Door { is_open }
    }

    /// Creates a new trap tile.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{TileKind, TrapType};
    ///
    /// let trap = TileKind::trap(TrapType::Teleport);
    /// assert_eq!(trap.trap_type(), Some(TrapType::Teleport));
    /// ```
    #[must_use]
    pub const fn trap(trap_type: TrapType) -> Self {
        Self::Trap { trap_type }
    }
}

impl fmt::Display for TileKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Floor => "Floor".to_string(),
            Self::Wall => "Wall".to_string(),
            Self::Door { is_open } => {
                if *is_open {
                    "Open Door".to_string()
                } else {
                    "Closed Door".to_string()
                }
            }
            Self::StairsUp => "Stairs Up".to_string(),
            Self::StairsDown => "Stairs Down".to_string(),
            Self::Trap { trap_type } => format!("{}", trap_type),
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// Tile
// =============================================================================

/// A single tile on the dungeon floor.
///
/// A tile has a kind and tracks its visibility state (explored and visible).
/// - `is_explored`: The tile has been seen at least once
/// - `is_visible`: The tile is currently in the player's field of view
///
/// # Examples
///
/// ```
/// use roguelike_domain::floor::{Tile, TileKind};
///
/// let tile = Tile::new(TileKind::Floor);
/// assert!(!tile.is_explored());
/// assert!(!tile.is_visible());
///
/// let explored = tile.mark_explored();
/// assert!(explored.is_explored());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    kind: TileKind,
    is_explored: bool,
    is_visible: bool,
}

impl Tile {
    /// Creates a new tile with the given kind.
    ///
    /// The tile is initially unexplored and not visible.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{Tile, TileKind};
    ///
    /// let tile = Tile::new(TileKind::Wall);
    /// assert_eq!(tile.kind(), TileKind::Wall);
    /// assert!(!tile.is_explored());
    /// assert!(!tile.is_visible());
    /// ```
    #[must_use]
    pub const fn new(kind: TileKind) -> Self {
        Self {
            kind,
            is_explored: false,
            is_visible: false,
        }
    }

    /// Creates a tile with full state specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{Tile, TileKind};
    ///
    /// let tile = Tile::with_state(TileKind::Floor, true, true);
    /// assert!(tile.is_explored());
    /// assert!(tile.is_visible());
    /// ```
    #[must_use]
    pub const fn with_state(kind: TileKind, is_explored: bool, is_visible: bool) -> Self {
        Self {
            kind,
            is_explored,
            is_visible,
        }
    }

    /// Returns the kind of this tile.
    #[must_use]
    pub const fn kind(&self) -> TileKind {
        self.kind
    }

    /// Returns true if this tile has been explored.
    #[must_use]
    pub const fn is_explored(&self) -> bool {
        self.is_explored
    }

    /// Returns true if this tile is currently visible.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Returns true if this tile can be walked on.
    #[must_use]
    pub const fn is_walkable(&self) -> bool {
        self.kind.is_walkable()
    }

    /// Returns true if this tile blocks line of sight.
    #[must_use]
    pub const fn blocks_sight(&self) -> bool {
        self.kind.blocks_sight()
    }

    /// Returns a new tile marked as explored.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{Tile, TileKind};
    ///
    /// let tile = Tile::new(TileKind::Floor);
    /// let explored = tile.mark_explored();
    /// assert!(explored.is_explored());
    /// ```
    #[must_use]
    pub const fn mark_explored(self) -> Self {
        Self {
            is_explored: true,
            ..self
        }
    }

    /// Returns a new tile with the given visibility state.
    ///
    /// When a tile becomes visible, it is also marked as explored.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{Tile, TileKind};
    ///
    /// let tile = Tile::new(TileKind::Floor);
    /// let visible = tile.set_visible(true);
    /// assert!(visible.is_visible());
    /// assert!(visible.is_explored()); // Also marked as explored
    /// ```
    #[must_use]
    pub const fn set_visible(self, is_visible: bool) -> Self {
        Self {
            is_visible,
            is_explored: if is_visible { true } else { self.is_explored },
            ..self
        }
    }

    /// Returns a new tile with a different kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::{Tile, TileKind};
    ///
    /// let floor_tile = Tile::new(TileKind::Floor);
    /// let door_tile = floor_tile.with_kind(TileKind::Door { is_open: false });
    /// assert_eq!(door_tile.kind(), TileKind::Door { is_open: false });
    /// ```
    #[must_use]
    pub const fn with_kind(self, kind: TileKind) -> Self {
        Self { kind, ..self }
    }
}

impl fmt::Display for Tile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visibility = if self.is_visible {
            "visible"
        } else if self.is_explored {
            "explored"
        } else {
            "unknown"
        };
        write!(formatter, "{} ({})", self.kind, visibility)
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
    // TrapType Tests
    // =========================================================================

    mod trap_type {
        use super::*;

        #[rstest]
        fn all_returns_four_types() {
            let types = TrapType::all();
            assert_eq!(types.len(), 4);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let types = TrapType::all();
            assert!(types.contains(&TrapType::Spike));
            assert!(types.contains(&TrapType::Poison));
            assert!(types.contains(&TrapType::Teleport));
            assert!(types.contains(&TrapType::Alarm));
        }

        #[rstest]
        #[case(TrapType::Spike, "Spike Trap")]
        #[case(TrapType::Poison, "Poison Trap")]
        #[case(TrapType::Teleport, "Teleport Trap")]
        #[case(TrapType::Alarm, "Alarm Trap")]
        fn display_format(#[case] trap_type: TrapType, #[case] expected: &str) {
            assert_eq!(format!("{}", trap_type), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(TrapType::Spike, TrapType::Spike);
            assert_ne!(TrapType::Spike, TrapType::Poison);
        }

        #[rstest]
        fn clone_and_copy() {
            let trap = TrapType::Teleport;
            let cloned = trap;
            assert_eq!(trap, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(TrapType::Spike);

            assert!(set.contains(&TrapType::Spike));
            assert!(!set.contains(&TrapType::Poison));
        }

        #[rstest]
        fn debug_format() {
            let trap = TrapType::Alarm;
            let debug_string = format!("{:?}", trap);
            assert!(debug_string.contains("Alarm"));
        }
    }

    // =========================================================================
    // TileKind Tests
    // =========================================================================

    mod tile_kind {
        use super::*;

        #[rstest]
        fn floor_is_walkable() {
            assert!(TileKind::Floor.is_walkable());
        }

        #[rstest]
        fn wall_is_not_walkable() {
            assert!(!TileKind::Wall.is_walkable());
        }

        #[rstest]
        fn open_door_is_walkable() {
            assert!(TileKind::Door { is_open: true }.is_walkable());
        }

        #[rstest]
        fn closed_door_is_not_walkable() {
            assert!(!TileKind::Door { is_open: false }.is_walkable());
        }

        #[rstest]
        fn stairs_up_is_walkable() {
            assert!(TileKind::StairsUp.is_walkable());
        }

        #[rstest]
        fn stairs_down_is_walkable() {
            assert!(TileKind::StairsDown.is_walkable());
        }

        #[rstest]
        fn trap_is_walkable() {
            let trap = TileKind::Trap {
                trap_type: TrapType::Spike,
            };
            assert!(trap.is_walkable());
        }

        #[rstest]
        fn wall_blocks_sight() {
            assert!(TileKind::Wall.blocks_sight());
        }

        #[rstest]
        fn closed_door_blocks_sight() {
            assert!(TileKind::Door { is_open: false }.blocks_sight());
        }

        #[rstest]
        fn open_door_does_not_block_sight() {
            assert!(!TileKind::Door { is_open: true }.blocks_sight());
        }

        #[rstest]
        fn floor_does_not_block_sight() {
            assert!(!TileKind::Floor.blocks_sight());
        }

        #[rstest]
        fn stairs_do_not_block_sight() {
            assert!(!TileKind::StairsUp.blocks_sight());
            assert!(!TileKind::StairsDown.blocks_sight());
        }

        #[rstest]
        fn trap_does_not_block_sight() {
            let trap = TileKind::Trap {
                trap_type: TrapType::Poison,
            };
            assert!(!trap.blocks_sight());
        }

        #[rstest]
        fn is_door_returns_true_for_doors() {
            assert!(TileKind::Door { is_open: true }.is_door());
            assert!(TileKind::Door { is_open: false }.is_door());
        }

        #[rstest]
        fn is_door_returns_false_for_non_doors() {
            assert!(!TileKind::Floor.is_door());
            assert!(!TileKind::Wall.is_door());
        }

        #[rstest]
        fn is_stairs_returns_true_for_stairs() {
            assert!(TileKind::StairsUp.is_stairs());
            assert!(TileKind::StairsDown.is_stairs());
        }

        #[rstest]
        fn is_stairs_returns_false_for_non_stairs() {
            assert!(!TileKind::Floor.is_stairs());
            assert!(!TileKind::Wall.is_stairs());
        }

        #[rstest]
        fn is_trap_returns_true_for_traps() {
            let trap = TileKind::Trap {
                trap_type: TrapType::Teleport,
            };
            assert!(trap.is_trap());
        }

        #[rstest]
        fn is_trap_returns_false_for_non_traps() {
            assert!(!TileKind::Floor.is_trap());
            assert!(!TileKind::Wall.is_trap());
        }

        #[rstest]
        fn trap_type_returns_type_for_traps() {
            let trap = TileKind::Trap {
                trap_type: TrapType::Alarm,
            };
            assert_eq!(trap.trap_type(), Some(TrapType::Alarm));
        }

        #[rstest]
        fn trap_type_returns_none_for_non_traps() {
            assert_eq!(TileKind::Floor.trap_type(), None);
            assert_eq!(TileKind::Wall.trap_type(), None);
        }

        #[rstest]
        fn door_constructor() {
            let closed = TileKind::door(false);
            let open = TileKind::door(true);

            assert_eq!(closed, TileKind::Door { is_open: false });
            assert_eq!(open, TileKind::Door { is_open: true });
        }

        #[rstest]
        fn trap_constructor() {
            let trap = TileKind::trap(TrapType::Spike);
            assert_eq!(
                trap,
                TileKind::Trap {
                    trap_type: TrapType::Spike
                }
            );
        }

        #[rstest]
        #[case(TileKind::Floor, "Floor")]
        #[case(TileKind::Wall, "Wall")]
        #[case(TileKind::Door { is_open: true }, "Open Door")]
        #[case(TileKind::Door { is_open: false }, "Closed Door")]
        #[case(TileKind::StairsUp, "Stairs Up")]
        #[case(TileKind::StairsDown, "Stairs Down")]
        fn display_format(#[case] kind: TileKind, #[case] expected: &str) {
            assert_eq!(format!("{}", kind), expected);
        }

        #[rstest]
        fn trap_display_format() {
            let trap = TileKind::Trap {
                trap_type: TrapType::Spike,
            };
            assert_eq!(format!("{}", trap), "Spike Trap");
        }

        #[rstest]
        fn equality() {
            assert_eq!(TileKind::Floor, TileKind::Floor);
            assert_ne!(TileKind::Floor, TileKind::Wall);
            assert_eq!(
                TileKind::Door { is_open: true },
                TileKind::Door { is_open: true }
            );
            assert_ne!(
                TileKind::Door { is_open: true },
                TileKind::Door { is_open: false }
            );
        }

        #[rstest]
        fn clone_and_copy() {
            let kind = TileKind::StairsDown;
            let cloned = kind;
            assert_eq!(kind, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(TileKind::Floor);

            assert!(set.contains(&TileKind::Floor));
            assert!(!set.contains(&TileKind::Wall));
        }

        #[rstest]
        fn debug_format() {
            let kind = TileKind::Wall;
            let debug_string = format!("{:?}", kind);
            assert!(debug_string.contains("Wall"));
        }
    }

    // =========================================================================
    // Tile Tests
    // =========================================================================

    mod tile {
        use super::*;

        #[rstest]
        fn new_creates_unexplored_invisible_tile() {
            let tile = Tile::new(TileKind::Floor);
            assert_eq!(tile.kind(), TileKind::Floor);
            assert!(!tile.is_explored());
            assert!(!tile.is_visible());
        }

        #[rstest]
        fn with_state_creates_tile_with_given_state() {
            let tile = Tile::with_state(TileKind::Wall, true, true);
            assert_eq!(tile.kind(), TileKind::Wall);
            assert!(tile.is_explored());
            assert!(tile.is_visible());
        }

        #[rstest]
        fn with_state_unexplored_invisible() {
            let tile = Tile::with_state(TileKind::Floor, false, false);
            assert!(!tile.is_explored());
            assert!(!tile.is_visible());
        }

        #[rstest]
        fn with_state_explored_invisible() {
            let tile = Tile::with_state(TileKind::Floor, true, false);
            assert!(tile.is_explored());
            assert!(!tile.is_visible());
        }

        #[rstest]
        fn is_walkable_delegates_to_kind() {
            let floor = Tile::new(TileKind::Floor);
            let wall = Tile::new(TileKind::Wall);

            assert!(floor.is_walkable());
            assert!(!wall.is_walkable());
        }

        #[rstest]
        fn blocks_sight_delegates_to_kind() {
            let floor = Tile::new(TileKind::Floor);
            let wall = Tile::new(TileKind::Wall);

            assert!(!floor.blocks_sight());
            assert!(wall.blocks_sight());
        }

        #[rstest]
        fn mark_explored_sets_explored() {
            let tile = Tile::new(TileKind::Floor);
            let explored = tile.mark_explored();

            assert!(explored.is_explored());
            assert!(!explored.is_visible()); // Visibility unchanged
        }

        #[rstest]
        fn mark_explored_preserves_kind() {
            let tile = Tile::new(TileKind::Door { is_open: true });
            let explored = tile.mark_explored();

            assert_eq!(explored.kind(), TileKind::Door { is_open: true });
        }

        #[rstest]
        fn set_visible_true_also_marks_explored() {
            let tile = Tile::new(TileKind::Floor);
            let visible = tile.set_visible(true);

            assert!(visible.is_visible());
            assert!(visible.is_explored());
        }

        #[rstest]
        fn set_visible_false_preserves_explored() {
            let tile = Tile::with_state(TileKind::Floor, true, true);
            let hidden = tile.set_visible(false);

            assert!(!hidden.is_visible());
            assert!(hidden.is_explored()); // Still explored
        }

        #[rstest]
        fn set_visible_false_on_unexplored_stays_unexplored() {
            let tile = Tile::new(TileKind::Floor);
            let hidden = tile.set_visible(false);

            assert!(!hidden.is_visible());
            assert!(!hidden.is_explored());
        }

        #[rstest]
        fn with_kind_changes_kind() {
            let floor = Tile::new(TileKind::Floor);
            let wall = floor.with_kind(TileKind::Wall);

            assert_eq!(wall.kind(), TileKind::Wall);
        }

        #[rstest]
        fn with_kind_preserves_visibility_state() {
            let tile = Tile::with_state(TileKind::Floor, true, true);
            let changed = tile.with_kind(TileKind::Wall);

            assert!(changed.is_explored());
            assert!(changed.is_visible());
        }

        #[rstest]
        fn display_format_unknown() {
            let tile = Tile::new(TileKind::Floor);
            assert_eq!(format!("{}", tile), "Floor (unknown)");
        }

        #[rstest]
        fn display_format_explored() {
            let tile = Tile::with_state(TileKind::Wall, true, false);
            assert_eq!(format!("{}", tile), "Wall (explored)");
        }

        #[rstest]
        fn display_format_visible() {
            let tile = Tile::with_state(TileKind::StairsDown, true, true);
            assert_eq!(format!("{}", tile), "Stairs Down (visible)");
        }

        #[rstest]
        fn equality() {
            let tile1 = Tile::new(TileKind::Floor);
            let tile2 = Tile::new(TileKind::Floor);
            let tile3 = Tile::new(TileKind::Wall);

            assert_eq!(tile1, tile2);
            assert_ne!(tile1, tile3);
        }

        #[rstest]
        fn equality_considers_visibility_state() {
            let unexplored = Tile::new(TileKind::Floor);
            let explored = Tile::with_state(TileKind::Floor, true, false);

            assert_ne!(unexplored, explored);
        }

        #[rstest]
        fn clone_and_copy() {
            let tile = Tile::with_state(TileKind::Floor, true, true);
            let cloned = tile;
            assert_eq!(tile, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let tile1 = Tile::new(TileKind::Floor);
            let tile2 = Tile::new(TileKind::Floor);
            let tile3 = Tile::new(TileKind::Wall);

            let mut set = HashSet::new();
            set.insert(tile1);

            assert!(set.contains(&tile2));
            assert!(!set.contains(&tile3));
        }

        #[rstest]
        fn debug_format() {
            let tile = Tile::new(TileKind::Floor);
            let debug_string = format!("{:?}", tile);
            assert!(debug_string.contains("Tile"));
            assert!(debug_string.contains("Floor"));
        }
    }
}
