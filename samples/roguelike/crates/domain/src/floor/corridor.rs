use std::fmt;

use crate::common::{Distance, Position};

// =============================================================================
// Corridor
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Corridor {
    start: Position,
    end: Position,
}

impl Corridor {
    #[must_use]
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn start(&self) -> Position {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> Position {
        self.end
    }

    #[must_use]
    pub fn length(&self) -> Distance {
        self.start.distance_to(&self.end)
    }

    #[must_use]
    pub const fn is_horizontal(&self) -> bool {
        self.start.y() == self.end.y()
    }

    #[must_use]
    pub const fn is_vertical(&self) -> bool {
        self.start.x() == self.end.x()
    }

    #[must_use]
    pub const fn is_straight(&self) -> bool {
        self.is_horizontal() || self.is_vertical()
    }

    #[must_use]
    pub const fn reverse(&self) -> Self {
        Self::new(self.end, self.start)
    }

    #[must_use]
    pub fn midpoint(&self) -> Position {
        Position::new(
            (self.start.x() + self.end.x()) / 2,
            (self.start.y() + self.end.y()) / 2,
        )
    }

    #[must_use]
    pub fn contains(&self, position: Position) -> bool {
        if self.is_horizontal() {
            let min_x = self.start.x().min(self.end.x());
            let max_x = self.start.x().max(self.end.x());
            position.y() == self.start.y() && position.x() >= min_x && position.x() <= max_x
        } else if self.is_vertical() {
            let min_y = self.start.y().min(self.end.y());
            let max_y = self.start.y().max(self.end.y());
            position.x() == self.start.x() && position.y() >= min_y && position.y() <= max_y
        } else {
            // For non-straight corridors, check if position is on the bounding box
            // and both distances to endpoints sum to the total length
            let distance_to_start = position.distance_to(&self.start);
            let distance_to_end = position.distance_to(&self.end);
            let total_length = self.length();

            distance_to_start.value() + distance_to_end.value() == total_length.value()
        }
    }
}

impl fmt::Display for Corridor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Corridor({} -> {})", self.start, self.end)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn new_creates_corridor() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        assert_eq!(corridor.start(), Position::new(0, 0));
        assert_eq!(corridor.end(), Position::new(10, 10));
    }

    #[rstest]
    fn new_same_position() {
        let corridor = Corridor::new(Position::new(5, 5), Position::new(5, 5));
        assert_eq!(corridor.start(), corridor.end());
    }

    #[rstest]
    fn length_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert_eq!(corridor.length(), Distance::new(10));
    }

    #[rstest]
    fn length_vertical() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 8));
        assert_eq!(corridor.length(), Distance::new(8));
    }

    #[rstest]
    fn length_diagonal() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(3, 4));
        assert_eq!(corridor.length(), Distance::new(7)); // Manhattan distance
    }

    #[rstest]
    fn length_zero() {
        let corridor = Corridor::new(Position::new(5, 5), Position::new(5, 5));
        assert_eq!(corridor.length(), Distance::zero());
    }

    #[rstest]
    fn is_horizontal_true() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(corridor.is_horizontal());
    }

    #[rstest]
    fn is_horizontal_false() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert!(!corridor.is_horizontal());
    }

    #[rstest]
    fn is_vertical_true() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert!(corridor.is_vertical());
    }

    #[rstest]
    fn is_vertical_false() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(!corridor.is_vertical());
    }

    #[rstest]
    fn is_straight_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(corridor.is_straight());
    }

    #[rstest]
    fn is_straight_vertical() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert!(corridor.is_straight());
    }

    #[rstest]
    fn is_straight_diagonal() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(5, 5));
        assert!(!corridor.is_straight());
    }

    #[rstest]
    fn is_straight_same_position() {
        let corridor = Corridor::new(Position::new(5, 5), Position::new(5, 5));
        // Same position is both horizontal and vertical, hence straight
        assert!(corridor.is_straight());
    }

    #[rstest]
    fn reverse_swaps_endpoints() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let reversed = corridor.reverse();

        assert_eq!(reversed.start(), Position::new(10, 10));
        assert_eq!(reversed.end(), Position::new(0, 0));
    }

    #[rstest]
    fn reverse_twice_is_identity() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let double_reversed = corridor.reverse().reverse();

        assert_eq!(corridor, double_reversed);
    }

    #[rstest]
    fn midpoint_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert_eq!(corridor.midpoint(), Position::new(5, 5));
    }

    #[rstest]
    fn midpoint_vertical() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert_eq!(corridor.midpoint(), Position::new(5, 5));
    }

    #[rstest]
    fn midpoint_diagonal() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        assert_eq!(corridor.midpoint(), Position::new(5, 5));
    }

    #[rstest]
    fn midpoint_odd_length() {
        // For odd length, midpoint is biased toward start (integer division)
        let corridor = Corridor::new(Position::new(0, 0), Position::new(5, 0));
        assert_eq!(corridor.midpoint(), Position::new(2, 0));
    }

    #[rstest]
    fn contains_start() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(corridor.contains(Position::new(0, 5)));
    }

    #[rstest]
    fn contains_end() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(corridor.contains(Position::new(10, 5)));
    }

    #[rstest]
    fn contains_middle_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(corridor.contains(Position::new(5, 5)));
    }

    #[rstest]
    fn contains_middle_vertical() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert!(corridor.contains(Position::new(5, 5)));
    }

    #[rstest]
    fn contains_outside_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(!corridor.contains(Position::new(5, 6)));
    }

    #[rstest]
    fn contains_outside_vertical() {
        let corridor = Corridor::new(Position::new(5, 0), Position::new(5, 10));
        assert!(!corridor.contains(Position::new(6, 5)));
    }

    #[rstest]
    fn contains_beyond_end_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(!corridor.contains(Position::new(11, 5)));
    }

    #[rstest]
    fn contains_before_start_horizontal() {
        let corridor = Corridor::new(Position::new(0, 5), Position::new(10, 5));
        assert!(!corridor.contains(Position::new(-1, 5)));
    }

    #[rstest]
    fn contains_reversed_horizontal() {
        // Corridor from right to left should still work
        let corridor = Corridor::new(Position::new(10, 5), Position::new(0, 5));
        assert!(corridor.contains(Position::new(5, 5)));
    }

    #[rstest]
    fn contains_reversed_vertical() {
        // Corridor from bottom to top should still work
        let corridor = Corridor::new(Position::new(5, 10), Position::new(5, 0));
        assert!(corridor.contains(Position::new(5, 5)));
    }

    #[rstest]
    fn display_format() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 5));
        assert_eq!(format!("{}", corridor), "Corridor((0, 0) -> (10, 5))");
    }

    #[rstest]
    fn equality() {
        let corridor1 = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let corridor2 = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let corridor3 = Corridor::new(Position::new(0, 0), Position::new(5, 5));

        assert_eq!(corridor1, corridor2);
        assert_ne!(corridor1, corridor3);
    }

    #[rstest]
    fn equality_considers_direction() {
        let corridor1 = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let corridor2 = Corridor::new(Position::new(10, 10), Position::new(0, 0));

        // Different direction means different corridors
        assert_ne!(corridor1, corridor2);
    }

    #[rstest]
    fn clone_and_copy() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let cloned = corridor;
        assert_eq!(corridor, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let corridor1 = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let corridor2 = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let corridor3 = Corridor::new(Position::new(0, 0), Position::new(5, 5));

        let mut set = HashSet::new();
        set.insert(corridor1);

        assert!(set.contains(&corridor2));
        assert!(!set.contains(&corridor3));
    }

    #[rstest]
    fn debug_format() {
        let corridor = Corridor::new(Position::new(0, 0), Position::new(10, 10));
        let debug_string = format!("{:?}", corridor);
        assert!(debug_string.contains("Corridor"));
        assert!(debug_string.contains("start"));
        assert!(debug_string.contains("end"));
    }
}
