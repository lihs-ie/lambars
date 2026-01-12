use std::fmt;
use std::ops::{Add, Sub};

// =============================================================================
// Position
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    x: i32,
    y: i32,
}

impl Position {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(&self) -> i32 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> i32 {
        self.y
    }

    #[must_use]
    pub const fn move_toward(&self, direction: Direction) -> Self {
        let offset = direction.to_offset();
        Self::new(self.x + offset.x, self.y + offset.y)
    }

    #[must_use]
    pub fn distance_to(&self, other: &Self) -> Distance {
        let dx = (self.x - other.x).unsigned_abs();
        let dy = (self.y - other.y).unsigned_abs();
        Distance::new(dx + dy)
    }

    #[must_use]
    pub const fn add(&self, other: &Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    #[must_use]
    pub const fn subtract(&self, other: &Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Position {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}, {})", self.x, self.y)
    }
}

// =============================================================================
// Direction
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    #[must_use]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    #[must_use]
    pub const fn to_offset(&self) -> Position {
        match self {
            Self::Up => Position::new(0, -1),
            Self::Down => Position::new(0, 1),
            Self::Left => Position::new(-1, 0),
            Self::Right => Position::new(1, 0),
        }
    }

    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Up, Self::Down, Self::Left, Self::Right]
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// Distance
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Distance(u32);

impl Distance {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Add for Distance {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
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
    // Position Tests
    // =========================================================================

    mod position {
        use super::*;

        #[rstest]
        fn new_creates_position_with_coordinates() {
            let position = Position::new(5, 3);
            assert_eq!(position.x(), 5);
            assert_eq!(position.y(), 3);
        }

        #[rstest]
        fn new_with_zero_coordinates() {
            let position = Position::new(0, 0);
            assert_eq!(position.x(), 0);
            assert_eq!(position.y(), 0);
        }

        #[rstest]
        fn new_with_negative_coordinates() {
            let position = Position::new(-10, -20);
            assert_eq!(position.x(), -10);
            assert_eq!(position.y(), -20);
        }

        #[rstest]
        fn new_with_max_coordinates() {
            let position = Position::new(i32::MAX, i32::MAX);
            assert_eq!(position.x(), i32::MAX);
            assert_eq!(position.y(), i32::MAX);
        }

        #[rstest]
        fn new_with_min_coordinates() {
            let position = Position::new(i32::MIN, i32::MIN);
            assert_eq!(position.x(), i32::MIN);
            assert_eq!(position.y(), i32::MIN);
        }

        #[rstest]
        #[case(Direction::Up, Position::new(5, 4))]
        #[case(Direction::Down, Position::new(5, 6))]
        #[case(Direction::Left, Position::new(4, 5))]
        #[case(Direction::Right, Position::new(6, 5))]
        fn move_toward_moves_in_direction(
            #[case] direction: Direction,
            #[case] expected: Position,
        ) {
            let position = Position::new(5, 5);
            let moved = position.move_toward(direction);
            assert_eq!(moved, expected);
        }

        #[rstest]
        fn distance_to_same_position() {
            let position = Position::new(5, 5);
            assert_eq!(position.distance_to(&position), Distance::zero());
        }

        #[rstest]
        fn distance_to_horizontal() {
            let from = Position::new(0, 0);
            let to = Position::new(5, 0);
            assert_eq!(from.distance_to(&to), Distance::new(5));
        }

        #[rstest]
        fn distance_to_vertical() {
            let from = Position::new(0, 0);
            let to = Position::new(0, 7);
            assert_eq!(from.distance_to(&to), Distance::new(7));
        }

        #[rstest]
        fn distance_to_diagonal() {
            let from = Position::new(0, 0);
            let to = Position::new(3, 4);
            assert_eq!(from.distance_to(&to), Distance::new(7));
        }

        #[rstest]
        fn distance_to_with_negative_coordinates() {
            let from = Position::new(-5, -5);
            let to = Position::new(5, 5);
            assert_eq!(from.distance_to(&to), Distance::new(20));
        }

        #[rstest]
        fn add_positions() {
            let position_a = Position::new(2, 3);
            let position_b = Position::new(1, 2);
            assert_eq!(Position::add(&position_a, &position_b), Position::new(3, 5));
        }

        #[rstest]
        fn add_with_negative() {
            let position_a = Position::new(5, 5);
            let position_b = Position::new(-3, -2);
            assert_eq!(Position::add(&position_a, &position_b), Position::new(2, 3));
        }

        #[rstest]
        fn subtract_positions() {
            let position_a = Position::new(5, 7);
            let position_b = Position::new(2, 3);
            assert_eq!(position_a.subtract(&position_b), Position::new(3, 4));
        }

        #[rstest]
        fn subtract_with_negative_result() {
            let position_a = Position::new(2, 3);
            let position_b = Position::new(5, 7);
            assert_eq!(position_a.subtract(&position_b), Position::new(-3, -4));
        }

        #[rstest]
        fn add_operator() {
            let position_a = Position::new(2, 3);
            let position_b = Position::new(1, 2);
            assert_eq!(position_a + position_b, Position::new(3, 5));
        }

        #[rstest]
        fn sub_operator() {
            let position_a = Position::new(5, 7);
            let position_b = Position::new(2, 3);
            assert_eq!(position_a - position_b, Position::new(3, 4));
        }

        #[rstest]
        fn display_format() {
            let position = Position::new(10, 20);
            assert_eq!(format!("{}", position), "(10, 20)");
        }

        #[rstest]
        fn display_format_with_negative() {
            let position = Position::new(-5, -10);
            assert_eq!(format!("{}", position), "(-5, -10)");
        }

        #[rstest]
        fn equality() {
            let position1 = Position::new(5, 10);
            let position2 = Position::new(5, 10);
            let position3 = Position::new(10, 5);

            assert_eq!(position1, position2);
            assert_ne!(position1, position3);
        }

        #[rstest]
        fn clone() {
            let position = Position::new(5, 10);
            let cloned = position;
            assert_eq!(position, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let position1 = Position::new(5, 10);
            let position2 = Position::new(5, 10);
            let position3 = Position::new(10, 5);

            let mut set = HashSet::new();
            set.insert(position1);

            assert!(set.contains(&position2));
            assert!(!set.contains(&position3));
        }
    }

    // =========================================================================
    // Direction Tests
    // =========================================================================

    mod direction {
        use super::*;

        #[rstest]
        #[case(Direction::Up, Direction::Down)]
        #[case(Direction::Down, Direction::Up)]
        #[case(Direction::Left, Direction::Right)]
        #[case(Direction::Right, Direction::Left)]
        fn opposite_returns_correct_direction(
            #[case] direction: Direction,
            #[case] expected: Direction,
        ) {
            assert_eq!(direction.opposite(), expected);
        }

        #[rstest]
        fn opposite_is_involution() {
            for direction in Direction::all() {
                assert_eq!(direction.opposite().opposite(), direction);
            }
        }

        #[rstest]
        #[case(Direction::Up, Position::new(0, -1))]
        #[case(Direction::Down, Position::new(0, 1))]
        #[case(Direction::Left, Position::new(-1, 0))]
        #[case(Direction::Right, Position::new(1, 0))]
        fn to_offset_returns_correct_offset(
            #[case] direction: Direction,
            #[case] expected: Position,
        ) {
            assert_eq!(direction.to_offset(), expected);
        }

        #[rstest]
        fn all_returns_four_directions() {
            let directions = Direction::all();
            assert_eq!(directions.len(), 4);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let directions = Direction::all();
            assert!(directions.contains(&Direction::Up));
            assert!(directions.contains(&Direction::Down));
            assert!(directions.contains(&Direction::Left));
            assert!(directions.contains(&Direction::Right));
        }

        #[rstest]
        #[case(Direction::Up, "Up")]
        #[case(Direction::Down, "Down")]
        #[case(Direction::Left, "Left")]
        #[case(Direction::Right, "Right")]
        fn display_format(#[case] direction: Direction, #[case] expected: &str) {
            assert_eq!(format!("{}", direction), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(Direction::Up, Direction::Up);
            assert_ne!(Direction::Up, Direction::Down);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(Direction::Up);

            assert!(set.contains(&Direction::Up));
            assert!(!set.contains(&Direction::Down));
        }
    }

    // =========================================================================
    // Distance Tests
    // =========================================================================

    mod distance {
        use super::*;

        #[rstest]
        fn new_creates_distance() {
            let distance = Distance::new(10);
            assert_eq!(distance.value(), 10);
        }

        #[rstest]
        fn new_with_zero() {
            let distance = Distance::new(0);
            assert_eq!(distance.value(), 0);
        }

        #[rstest]
        fn new_with_max_value() {
            let distance = Distance::new(u32::MAX);
            assert_eq!(distance.value(), u32::MAX);
        }

        #[rstest]
        fn zero_returns_zero_distance() {
            let distance = Distance::zero();
            assert_eq!(distance.value(), 0);
        }

        #[rstest]
        fn add_operator() {
            let distance1 = Distance::new(5);
            let distance2 = Distance::new(3);
            assert_eq!(distance1 + distance2, Distance::new(8));
        }

        #[rstest]
        fn add_with_zero() {
            let distance = Distance::new(10);
            assert_eq!(distance + Distance::zero(), distance);
        }

        #[rstest]
        fn display_format() {
            let distance = Distance::new(42);
            assert_eq!(format!("{}", distance), "42");
        }

        #[rstest]
        fn ordering() {
            let small = Distance::new(5);
            let large = Distance::new(10);

            assert!(small < large);
            assert!(large > small);
            assert!(small <= Distance::new(5));
            assert!(small >= Distance::new(5));
        }

        #[rstest]
        fn equality() {
            let distance1 = Distance::new(5);
            let distance2 = Distance::new(5);
            let distance3 = Distance::new(10);

            assert_eq!(distance1, distance2);
            assert_ne!(distance1, distance3);
        }

        #[rstest]
        fn clone() {
            let distance = Distance::new(10);
            let cloned = distance;
            assert_eq!(distance, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let distance1 = Distance::new(10);
            let distance2 = Distance::new(10);
            let distance3 = Distance::new(20);

            let mut set = HashSet::new();
            set.insert(distance1);

            assert!(set.contains(&distance2));
            assert!(!set.contains(&distance3));
        }
    }
}
