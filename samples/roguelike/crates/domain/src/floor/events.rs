use std::fmt;

use crate::common::{FloorLevel, Position};

use super::tile::TrapType;

// =============================================================================
// FloorEntered
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloorEntered {
    floor_level: FloorLevel,
}

impl FloorEntered {
    #[must_use]
    pub const fn new(floor_level: FloorLevel) -> Self {
        Self { floor_level }
    }

    #[must_use]
    pub const fn floor_level(&self) -> FloorLevel {
        self.floor_level
    }
}

impl fmt::Display for FloorEntered {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Entered {}", self.floor_level)
    }
}

// =============================================================================
// TileExplored
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileExplored {
    position: Position,
}

impl TileExplored {
    #[must_use]
    pub const fn new(position: Position) -> Self {
        Self { position }
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

impl fmt::Display for TileExplored {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Tile explored at {}", self.position)
    }
}

// =============================================================================
// TrapTriggered
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrapTriggered {
    position: Position,
    trap_type: TrapType,
}

impl TrapTriggered {
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

impl fmt::Display for TrapTriggered {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} triggered at {}",
            self.trap_type, self.position
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
    // FloorEntered Tests
    // =========================================================================

    mod floor_entered {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let floor_level = FloorLevel::new(5).unwrap();
            let event = FloorEntered::new(floor_level);
            assert_eq!(event.floor_level(), floor_level);
        }

        #[rstest]
        fn new_first_floor() {
            let event = FloorEntered::new(FloorLevel::first());
            assert_eq!(event.floor_level().value(), 1);
        }

        #[rstest]
        fn display_format() {
            let floor_level = FloorLevel::new(3).unwrap();
            let event = FloorEntered::new(floor_level);
            assert_eq!(format!("{}", event), "Entered B3F");
        }

        #[rstest]
        fn equality() {
            let floor_level = FloorLevel::new(5).unwrap();
            let event1 = FloorEntered::new(floor_level);
            let event2 = FloorEntered::new(floor_level);
            let event3 = FloorEntered::new(FloorLevel::first());

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone_and_copy() {
            let event = FloorEntered::new(FloorLevel::first());
            let cloned = event;
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let floor_level = FloorLevel::new(5).unwrap();
            let event1 = FloorEntered::new(floor_level);
            let event2 = FloorEntered::new(floor_level);

            let mut set = HashSet::new();
            set.insert(event1);

            assert!(set.contains(&event2));
        }

        #[rstest]
        fn debug_format() {
            let event = FloorEntered::new(FloorLevel::first());
            let debug_string = format!("{:?}", event);
            assert!(debug_string.contains("FloorEntered"));
            assert!(debug_string.contains("floor_level"));
        }
    }

    // =========================================================================
    // TileExplored Tests
    // =========================================================================

    mod tile_explored {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let position = Position::new(10, 15);
            let event = TileExplored::new(position);
            assert_eq!(event.position(), position);
        }

        #[rstest]
        fn new_at_origin() {
            let event = TileExplored::new(Position::new(0, 0));
            assert_eq!(event.position(), Position::new(0, 0));
        }

        #[rstest]
        fn display_format() {
            let event = TileExplored::new(Position::new(5, 10));
            assert_eq!(format!("{}", event), "Tile explored at (5, 10)");
        }

        #[rstest]
        fn equality() {
            let event1 = TileExplored::new(Position::new(5, 5));
            let event2 = TileExplored::new(Position::new(5, 5));
            let event3 = TileExplored::new(Position::new(10, 10));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone_and_copy() {
            let event = TileExplored::new(Position::new(5, 5));
            let cloned = event;
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let event1 = TileExplored::new(Position::new(5, 5));
            let event2 = TileExplored::new(Position::new(5, 5));

            let mut set = HashSet::new();
            set.insert(event1);

            assert!(set.contains(&event2));
        }

        #[rstest]
        fn debug_format() {
            let event = TileExplored::new(Position::new(5, 5));
            let debug_string = format!("{:?}", event);
            assert!(debug_string.contains("TileExplored"));
            assert!(debug_string.contains("position"));
        }
    }

    // =========================================================================
    // TrapTriggered Tests
    // =========================================================================

    mod trap_triggered {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let position = Position::new(10, 10);
            let event = TrapTriggered::new(position, TrapType::Spike);

            assert_eq!(event.position(), position);
            assert_eq!(event.trap_type(), TrapType::Spike);
        }

        #[rstest]
        #[case(TrapType::Spike)]
        #[case(TrapType::Poison)]
        #[case(TrapType::Teleport)]
        #[case(TrapType::Alarm)]
        fn new_with_all_trap_types(#[case] trap_type: TrapType) {
            let event = TrapTriggered::new(Position::new(0, 0), trap_type);
            assert_eq!(event.trap_type(), trap_type);
        }

        #[rstest]
        fn display_format() {
            let event = TrapTriggered::new(Position::new(5, 5), TrapType::Poison);
            assert_eq!(format!("{}", event), "Poison Trap triggered at (5, 5)");
        }

        #[rstest]
        fn display_format_all_types() {
            let position = Position::new(1, 1);

            let spike = TrapTriggered::new(position, TrapType::Spike);
            assert_eq!(format!("{}", spike), "Spike Trap triggered at (1, 1)");

            let poison = TrapTriggered::new(position, TrapType::Poison);
            assert_eq!(format!("{}", poison), "Poison Trap triggered at (1, 1)");

            let teleport = TrapTriggered::new(position, TrapType::Teleport);
            assert_eq!(format!("{}", teleport), "Teleport Trap triggered at (1, 1)");

            let alarm = TrapTriggered::new(position, TrapType::Alarm);
            assert_eq!(format!("{}", alarm), "Alarm Trap triggered at (1, 1)");
        }

        #[rstest]
        fn equality() {
            let event1 = TrapTriggered::new(Position::new(5, 5), TrapType::Spike);
            let event2 = TrapTriggered::new(Position::new(5, 5), TrapType::Spike);
            let event3 = TrapTriggered::new(Position::new(5, 5), TrapType::Poison);
            let event4 = TrapTriggered::new(Position::new(10, 10), TrapType::Spike);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3); // Different trap type
            assert_ne!(event1, event4); // Different position
        }

        #[rstest]
        fn clone_and_copy() {
            let event = TrapTriggered::new(Position::new(5, 5), TrapType::Teleport);
            let cloned = event;
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let event1 = TrapTriggered::new(Position::new(5, 5), TrapType::Alarm);
            let event2 = TrapTriggered::new(Position::new(5, 5), TrapType::Alarm);

            let mut set = HashSet::new();
            set.insert(event1);

            assert!(set.contains(&event2));
        }

        #[rstest]
        fn debug_format() {
            let event = TrapTriggered::new(Position::new(5, 5), TrapType::Spike);
            let debug_string = format!("{:?}", event);
            assert!(debug_string.contains("TrapTriggered"));
            assert!(debug_string.contains("position"));
            assert!(debug_string.contains("trap_type"));
        }
    }
}
