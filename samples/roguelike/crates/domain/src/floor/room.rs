//! Room value object for dungeon floors.
//!
//! This module provides the Room type representing a rectangular room
//! in the dungeon with validation constraints.

use std::fmt;

use crate::common::{Position, ValidationError};

// =============================================================================
// Room
// =============================================================================

/// A rectangular room in the dungeon.
///
/// Rooms are defined by their top-left corner position and dimensions.
/// Both width and height must be at least 3 to form a valid room
/// (walls on all sides with at least one interior tile).
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Position;
/// use roguelike_domain::floor::Room;
///
/// let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
/// assert_eq!(room.width(), 10);
/// assert_eq!(room.height(), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Room {
    top_left: Position,
    width: u32,
    height: u32,
}

impl Room {
    /// The minimum width for a room.
    pub const MIN_WIDTH: u32 = 3;

    /// The minimum height for a room.
    pub const MIN_HEIGHT: u32 = 3;

    /// Creates a new Room with the given position and dimensions.
    ///
    /// Returns an error if width or height is less than the minimum (3).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// // Valid room
    /// let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
    /// assert_eq!(room.width(), 5);
    ///
    /// // Invalid room - width too small
    /// assert!(Room::new(Position::new(0, 0), 2, 5).is_err());
    ///
    /// // Invalid room - height too small
    /// assert!(Room::new(Position::new(0, 0), 5, 2).is_err());
    /// ```
    pub fn new(top_left: Position, width: u32, height: u32) -> Result<Self, ValidationError> {
        if width < Self::MIN_WIDTH {
            return Err(ValidationError::out_of_range(
                "width",
                Self::MIN_WIDTH,
                "unlimited",
                width,
            ));
        }
        if height < Self::MIN_HEIGHT {
            return Err(ValidationError::out_of_range(
                "height",
                Self::MIN_HEIGHT,
                "unlimited",
                height,
            ));
        }
        Ok(Self {
            top_left,
            width,
            height,
        })
    }

    /// Returns the top-left corner position of the room.
    #[must_use]
    pub const fn top_left(&self) -> Position {
        self.top_left
    }

    /// Returns the width of the room.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the room.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the bottom-right corner position of the room.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
    /// assert_eq!(room.bottom_right(), Position::new(14, 12));
    /// ```
    #[must_use]
    pub fn bottom_right(&self) -> Position {
        Position::new(
            self.top_left.x() + (self.width as i32) - 1,
            self.top_left.y() + (self.height as i32) - 1,
        )
    }

    /// Returns the center position of the room.
    ///
    /// For even-sized rooms, the center is biased toward the top-left.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
    /// assert_eq!(room.center(), Position::new(2, 2));
    /// ```
    #[must_use]
    pub fn center(&self) -> Position {
        Position::new(
            self.top_left.x() + (self.width as i32) / 2,
            self.top_left.y() + (self.height as i32) / 2,
        )
    }

    /// Returns the area of the room in tiles.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(0, 0), 5, 4).unwrap();
    /// assert_eq!(room.area(), 20);
    /// ```
    #[must_use]
    pub const fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Returns the interior area of the room (excluding walls).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(0, 0), 5, 4).unwrap();
    /// // Interior is (5-2) * (4-2) = 3 * 2 = 6
    /// assert_eq!(room.interior_area(), 6);
    /// ```
    #[must_use]
    pub const fn interior_area(&self) -> u32 {
        (self.width - 2) * (self.height - 2)
    }

    /// Returns true if the given position is inside this room (including walls).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
    /// assert!(room.contains(Position::new(5, 5)));   // Top-left corner
    /// assert!(room.contains(Position::new(10, 10))); // Interior
    /// assert!(!room.contains(Position::new(4, 5)));  // Outside
    /// ```
    #[must_use]
    pub fn contains(&self, position: Position) -> bool {
        let bottom_right = self.bottom_right();
        position.x() >= self.top_left.x()
            && position.x() <= bottom_right.x()
            && position.y() >= self.top_left.y()
            && position.y() <= bottom_right.y()
    }

    /// Returns true if the given position is in the interior of this room
    /// (excluding walls).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
    /// assert!(!room.is_interior(Position::new(5, 5)));   // Wall (top-left)
    /// assert!(room.is_interior(Position::new(10, 10))); // Interior
    /// ```
    #[must_use]
    pub fn is_interior(&self, position: Position) -> bool {
        let interior_top_left = Position::new(self.top_left.x() + 1, self.top_left.y() + 1);
        let interior_bottom_right = Position::new(
            self.top_left.x() + (self.width as i32) - 2,
            self.top_left.y() + (self.height as i32) - 2,
        );

        position.x() >= interior_top_left.x()
            && position.x() <= interior_bottom_right.x()
            && position.y() >= interior_top_left.y()
            && position.y() <= interior_bottom_right.y()
    }

    /// Returns true if the given position is on the wall of this room.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
    /// assert!(room.is_wall(Position::new(5, 5)));   // Corner wall
    /// assert!(room.is_wall(Position::new(10, 5))); // Top wall
    /// assert!(!room.is_wall(Position::new(10, 10))); // Interior
    /// ```
    #[must_use]
    pub fn is_wall(&self, position: Position) -> bool {
        self.contains(position) && !self.is_interior(position)
    }

    /// Returns true if this room overlaps with another room.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Position;
    /// use roguelike_domain::floor::Room;
    ///
    /// let room1 = Room::new(Position::new(0, 0), 10, 10).unwrap();
    /// let room2 = Room::new(Position::new(5, 5), 10, 10).unwrap();
    /// let room3 = Room::new(Position::new(20, 20), 5, 5).unwrap();
    ///
    /// assert!(room1.overlaps(&room2));
    /// assert!(!room1.overlaps(&room3));
    /// ```
    #[must_use]
    pub fn overlaps(&self, other: &Room) -> bool {
        let self_bottom_right = self.bottom_right();
        let other_bottom_right = other.bottom_right();

        self.top_left.x() <= other_bottom_right.x()
            && self_bottom_right.x() >= other.top_left.x()
            && self.top_left.y() <= other_bottom_right.y()
            && self_bottom_right.y() >= other.top_left.y()
    }
}

impl fmt::Display for Room {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Room({}x{} at {})",
            self.width, self.height, self.top_left
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

    #[rstest]
    fn new_valid_room() {
        let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
        assert_eq!(room.width(), 5);
        assert_eq!(room.height(), 5);
    }

    #[rstest]
    fn new_minimum_size_room() {
        let room = Room::new(Position::new(0, 0), 3, 3).unwrap();
        assert_eq!(room.width(), 3);
        assert_eq!(room.height(), 3);
    }

    #[rstest]
    fn new_width_too_small() {
        let result = Room::new(Position::new(0, 0), 2, 5);
        assert!(result.is_err());
    }

    #[rstest]
    fn new_height_too_small() {
        let result = Room::new(Position::new(0, 0), 5, 2);
        assert!(result.is_err());
    }

    #[rstest]
    fn new_both_dimensions_too_small() {
        let result = Room::new(Position::new(0, 0), 1, 1);
        assert!(result.is_err());
    }

    #[rstest]
    fn top_left_returns_position() {
        let room = Room::new(Position::new(10, 20), 5, 5).unwrap();
        assert_eq!(room.top_left(), Position::new(10, 20));
    }

    #[rstest]
    fn bottom_right_calculation() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert_eq!(room.bottom_right(), Position::new(14, 12));
    }

    #[rstest]
    fn bottom_right_at_origin() {
        let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
        assert_eq!(room.bottom_right(), Position::new(4, 4));
    }

    #[rstest]
    fn center_odd_dimensions() {
        let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
        assert_eq!(room.center(), Position::new(2, 2));
    }

    #[rstest]
    fn center_even_dimensions() {
        let room = Room::new(Position::new(0, 0), 6, 6).unwrap();
        // For even sizes, center is biased toward top-left
        assert_eq!(room.center(), Position::new(3, 3));
    }

    #[rstest]
    fn center_with_offset() {
        let room = Room::new(Position::new(10, 10), 5, 5).unwrap();
        assert_eq!(room.center(), Position::new(12, 12));
    }

    #[rstest]
    fn area_calculation() {
        let room = Room::new(Position::new(0, 0), 5, 4).unwrap();
        assert_eq!(room.area(), 20);
    }

    #[rstest]
    fn interior_area_calculation() {
        let room = Room::new(Position::new(0, 0), 5, 4).unwrap();
        // Interior is (5-2) * (4-2) = 3 * 2 = 6
        assert_eq!(room.interior_area(), 6);
    }

    #[rstest]
    fn interior_area_minimum_room() {
        let room = Room::new(Position::new(0, 0), 3, 3).unwrap();
        // Interior is (3-2) * (3-2) = 1 * 1 = 1
        assert_eq!(room.interior_area(), 1);
    }

    #[rstest]
    fn contains_top_left_corner() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.contains(Position::new(5, 5)));
    }

    #[rstest]
    fn contains_bottom_right_corner() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.contains(Position::new(14, 12)));
    }

    #[rstest]
    fn contains_interior_point() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.contains(Position::new(10, 10)));
    }

    #[rstest]
    fn contains_outside_left() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.contains(Position::new(4, 10)));
    }

    #[rstest]
    fn contains_outside_right() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.contains(Position::new(15, 10)));
    }

    #[rstest]
    fn contains_outside_top() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.contains(Position::new(10, 4)));
    }

    #[rstest]
    fn contains_outside_bottom() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.contains(Position::new(10, 13)));
    }

    #[rstest]
    fn is_interior_center() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.is_interior(Position::new(10, 10)));
    }

    #[rstest]
    fn is_interior_just_inside_top_left() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.is_interior(Position::new(6, 6)));
    }

    #[rstest]
    fn is_interior_just_inside_bottom_right() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.is_interior(Position::new(13, 11)));
    }

    #[rstest]
    fn is_interior_on_wall_returns_false() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.is_interior(Position::new(5, 5))); // Corner
        assert!(!room.is_interior(Position::new(10, 5))); // Top wall
        assert!(!room.is_interior(Position::new(5, 10))); // Left wall
    }

    #[rstest]
    fn is_wall_corner() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.is_wall(Position::new(5, 5)));
    }

    #[rstest]
    fn is_wall_edge() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(room.is_wall(Position::new(10, 5))); // Top wall
        assert!(room.is_wall(Position::new(5, 10))); // Left wall
        assert!(room.is_wall(Position::new(14, 10))); // Right wall
        assert!(room.is_wall(Position::new(10, 12))); // Bottom wall
    }

    #[rstest]
    fn is_wall_interior_returns_false() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.is_wall(Position::new(10, 10)));
    }

    #[rstest]
    fn is_wall_outside_returns_false() {
        let room = Room::new(Position::new(5, 5), 10, 8).unwrap();
        assert!(!room.is_wall(Position::new(0, 0)));
    }

    #[rstest]
    fn overlaps_same_position() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        assert!(room1.overlaps(&room2));
    }

    #[rstest]
    fn overlaps_partial_overlap() {
        let room1 = Room::new(Position::new(0, 0), 10, 10).unwrap();
        let room2 = Room::new(Position::new(5, 5), 10, 10).unwrap();
        assert!(room1.overlaps(&room2));
    }

    #[rstest]
    fn overlaps_touching_corners() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(4, 4), 5, 5).unwrap();
        assert!(room1.overlaps(&room2)); // They share the corner
    }

    #[rstest]
    fn overlaps_no_overlap() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(10, 10), 5, 5).unwrap();
        assert!(!room1.overlaps(&room2));
    }

    #[rstest]
    fn overlaps_adjacent_horizontal() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(5, 0), 5, 5).unwrap();
        // Adjacent rooms don't overlap (they share an edge)
        assert!(!room1.overlaps(&room2));
    }

    #[rstest]
    fn overlaps_adjacent_vertical() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(0, 5), 5, 5).unwrap();
        assert!(!room1.overlaps(&room2));
    }

    #[rstest]
    fn display_format() {
        let room = Room::new(Position::new(10, 20), 15, 12).unwrap();
        assert_eq!(format!("{}", room), "Room(15x12 at (10, 20))");
    }

    #[rstest]
    fn equality() {
        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room3 = Room::new(Position::new(0, 0), 6, 5).unwrap();

        assert_eq!(room1, room2);
        assert_ne!(room1, room3);
    }

    #[rstest]
    fn clone_and_copy() {
        let room = Room::new(Position::new(5, 5), 10, 10).unwrap();
        let cloned = room;
        assert_eq!(room, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let room1 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room2 = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let room3 = Room::new(Position::new(0, 0), 6, 6).unwrap();

        let mut set = HashSet::new();
        set.insert(room1);

        assert!(set.contains(&room2));
        assert!(!set.contains(&room3));
    }

    #[rstest]
    fn debug_format() {
        let room = Room::new(Position::new(0, 0), 5, 5).unwrap();
        let debug_string = format!("{:?}", room);
        assert!(debug_string.contains("Room"));
        assert!(debug_string.contains("width"));
        assert!(debug_string.contains("height"));
    }
}
