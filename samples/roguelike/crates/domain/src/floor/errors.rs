//! Error types for the floor domain module.
//!
//! This module provides error types specific to floor and dungeon operations.

use std::error::Error;
use std::fmt;

// =============================================================================
// FloorError
// =============================================================================

/// Error types for floor operations.
///
/// This enum represents various errors that can occur when working with
/// dungeon floors, rooms, corridors, and tiles.
///
/// # Examples
///
/// ```
/// use roguelike_domain::floor::FloorError;
///
/// let error = FloorError::floor_not_found(42);
/// assert!(format!("{}", error).contains("42"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorError {
    /// The requested floor was not found.
    FloorNotFound {
        /// The identifier of the floor that was not found.
        floor_identifier: u32,
    },

    /// A position was outside the floor bounds.
    PositionOutOfBounds {
        /// The position that was out of bounds.
        position: (i32, i32),
        /// The floor bounds (width, height).
        bounds: (u32, u32),
    },

    /// A tile is not walkable.
    TileNotWalkable {
        /// The position of the non-walkable tile.
        position: (i32, i32),
        /// The type of tile that blocked movement.
        tile_type: String,
    },

    /// No stairs exist at the given position.
    NoStairsAtPosition {
        /// The position where stairs were expected.
        position: (i32, i32),
    },

    /// Rooms are not connected.
    RoomsNotConnected,

    /// Invalid floor generation parameters or result.
    InvalidFloorGeneration {
        /// The reason for the invalid generation.
        reason: String,
    },
}

impl FloorError {
    /// Creates a FloorNotFound error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::floor_not_found(5);
    /// assert!(matches!(error, FloorError::FloorNotFound { floor_identifier: 5 }));
    /// ```
    #[must_use]
    pub const fn floor_not_found(floor_identifier: u32) -> Self {
        Self::FloorNotFound { floor_identifier }
    }

    /// Creates a PositionOutOfBounds error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
    /// assert!(matches!(
    ///     error,
    ///     FloorError::PositionOutOfBounds { position: (100, 50), bounds: (80, 40) }
    /// ));
    /// ```
    #[must_use]
    pub const fn position_out_of_bounds(position: (i32, i32), bounds: (u32, u32)) -> Self {
        Self::PositionOutOfBounds { position, bounds }
    }

    /// Creates a TileNotWalkable error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::tile_not_walkable((5, 5), "Wall");
    /// assert!(matches!(error, FloorError::TileNotWalkable { .. }));
    /// ```
    #[must_use]
    pub fn tile_not_walkable(position: (i32, i32), tile_type: impl Into<String>) -> Self {
        Self::TileNotWalkable {
            position,
            tile_type: tile_type.into(),
        }
    }

    /// Creates a NoStairsAtPosition error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::no_stairs_at_position((10, 10));
    /// assert!(matches!(error, FloorError::NoStairsAtPosition { position: (10, 10) }));
    /// ```
    #[must_use]
    pub const fn no_stairs_at_position(position: (i32, i32)) -> Self {
        Self::NoStairsAtPosition { position }
    }

    /// Creates a RoomsNotConnected error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::rooms_not_connected();
    /// assert!(matches!(error, FloorError::RoomsNotConnected));
    /// ```
    #[must_use]
    pub const fn rooms_not_connected() -> Self {
        Self::RoomsNotConnected
    }

    /// Creates an InvalidFloorGeneration error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorError;
    ///
    /// let error = FloorError::invalid_floor_generation("Floor too small");
    /// assert!(matches!(error, FloorError::InvalidFloorGeneration { .. }));
    /// ```
    #[must_use]
    pub fn invalid_floor_generation(reason: impl Into<String>) -> Self {
        Self::InvalidFloorGeneration {
            reason: reason.into(),
        }
    }

    /// Returns true if this is a recoverable error.
    ///
    /// Position-related errors are generally recoverable as they indicate
    /// invalid user input that can be corrected.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        match self {
            Self::FloorNotFound { .. } => false,
            Self::PositionOutOfBounds { .. } => true,
            Self::TileNotWalkable { .. } => true,
            Self::NoStairsAtPosition { .. } => true,
            Self::RoomsNotConnected => false,
            Self::InvalidFloorGeneration { .. } => false,
        }
    }

    /// Returns true if this is a position-related error.
    #[must_use]
    pub const fn is_position_error(&self) -> bool {
        matches!(
            self,
            Self::PositionOutOfBounds { .. }
                | Self::TileNotWalkable { .. }
                | Self::NoStairsAtPosition { .. }
        )
    }

    /// Returns true if this is a generation-related error.
    #[must_use]
    pub const fn is_generation_error(&self) -> bool {
        matches!(
            self,
            Self::RoomsNotConnected | Self::InvalidFloorGeneration { .. }
        )
    }
}

impl fmt::Display for FloorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FloorNotFound { floor_identifier } => {
                write!(formatter, "Floor not found: {}", floor_identifier)
            }
            Self::PositionOutOfBounds { position, bounds } => {
                write!(
                    formatter,
                    "Position ({}, {}) is out of bounds (floor size: {}x{})",
                    position.0, position.1, bounds.0, bounds.1
                )
            }
            Self::TileNotWalkable {
                position,
                tile_type,
            } => {
                write!(
                    formatter,
                    "Cannot walk on {} at ({}, {})",
                    tile_type, position.0, position.1
                )
            }
            Self::NoStairsAtPosition { position } => {
                write!(
                    formatter,
                    "No stairs at position ({}, {})",
                    position.0, position.1
                )
            }
            Self::RoomsNotConnected => {
                write!(formatter, "Rooms are not connected")
            }
            Self::InvalidFloorGeneration { reason } => {
                write!(formatter, "Invalid floor generation: {}", reason)
            }
        }
    }
}

impl Error for FloorError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // FloorNotFound Tests
    // =========================================================================

    mod floor_not_found {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::floor_not_found(42);
            assert!(matches!(
                error,
                FloorError::FloorNotFound {
                    floor_identifier: 42
                }
            ));
        }

        #[rstest]
        fn display() {
            let error = FloorError::floor_not_found(42);
            assert_eq!(format!("{}", error), "Floor not found: 42");
        }

        #[rstest]
        fn is_not_recoverable() {
            let error = FloorError::floor_not_found(1);
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn is_not_position_error() {
            let error = FloorError::floor_not_found(1);
            assert!(!error.is_position_error());
        }

        #[rstest]
        fn is_not_generation_error() {
            let error = FloorError::floor_not_found(1);
            assert!(!error.is_generation_error());
        }
    }

    // =========================================================================
    // PositionOutOfBounds Tests
    // =========================================================================

    mod position_out_of_bounds {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
            assert!(matches!(
                error,
                FloorError::PositionOutOfBounds {
                    position: (100, 50),
                    bounds: (80, 40)
                }
            ));
        }

        #[rstest]
        fn display() {
            let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
            assert_eq!(
                format!("{}", error),
                "Position (100, 50) is out of bounds (floor size: 80x40)"
            );
        }

        #[rstest]
        fn is_recoverable() {
            let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn is_position_error() {
            let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
            assert!(error.is_position_error());
        }

        #[rstest]
        fn is_not_generation_error() {
            let error = FloorError::position_out_of_bounds((100, 50), (80, 40));
            assert!(!error.is_generation_error());
        }

        #[rstest]
        fn with_negative_position() {
            let error = FloorError::position_out_of_bounds((-5, -10), (80, 40));
            assert_eq!(
                format!("{}", error),
                "Position (-5, -10) is out of bounds (floor size: 80x40)"
            );
        }
    }

    // =========================================================================
    // TileNotWalkable Tests
    // =========================================================================

    mod tile_not_walkable {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::tile_not_walkable((5, 5), "Wall");
            assert!(matches!(error, FloorError::TileNotWalkable { .. }));
        }

        #[rstest]
        fn display() {
            let error = FloorError::tile_not_walkable((5, 5), "Wall");
            assert_eq!(format!("{}", error), "Cannot walk on Wall at (5, 5)");
        }

        #[rstest]
        fn with_string_tile_type() {
            let error = FloorError::tile_not_walkable((5, 5), String::from("Closed Door"));
            assert_eq!(format!("{}", error), "Cannot walk on Closed Door at (5, 5)");
        }

        #[rstest]
        fn is_recoverable() {
            let error = FloorError::tile_not_walkable((5, 5), "Wall");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn is_position_error() {
            let error = FloorError::tile_not_walkable((5, 5), "Wall");
            assert!(error.is_position_error());
        }

        #[rstest]
        fn is_not_generation_error() {
            let error = FloorError::tile_not_walkable((5, 5), "Wall");
            assert!(!error.is_generation_error());
        }
    }

    // =========================================================================
    // NoStairsAtPosition Tests
    // =========================================================================

    mod no_stairs_at_position {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::no_stairs_at_position((10, 10));
            assert!(matches!(
                error,
                FloorError::NoStairsAtPosition { position: (10, 10) }
            ));
        }

        #[rstest]
        fn display() {
            let error = FloorError::no_stairs_at_position((10, 10));
            assert_eq!(format!("{}", error), "No stairs at position (10, 10)");
        }

        #[rstest]
        fn is_recoverable() {
            let error = FloorError::no_stairs_at_position((10, 10));
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn is_position_error() {
            let error = FloorError::no_stairs_at_position((10, 10));
            assert!(error.is_position_error());
        }

        #[rstest]
        fn is_not_generation_error() {
            let error = FloorError::no_stairs_at_position((10, 10));
            assert!(!error.is_generation_error());
        }
    }

    // =========================================================================
    // RoomsNotConnected Tests
    // =========================================================================

    mod rooms_not_connected {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::rooms_not_connected();
            assert!(matches!(error, FloorError::RoomsNotConnected));
        }

        #[rstest]
        fn display() {
            let error = FloorError::rooms_not_connected();
            assert_eq!(format!("{}", error), "Rooms are not connected");
        }

        #[rstest]
        fn is_not_recoverable() {
            let error = FloorError::rooms_not_connected();
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn is_not_position_error() {
            let error = FloorError::rooms_not_connected();
            assert!(!error.is_position_error());
        }

        #[rstest]
        fn is_generation_error() {
            let error = FloorError::rooms_not_connected();
            assert!(error.is_generation_error());
        }
    }

    // =========================================================================
    // InvalidFloorGeneration Tests
    // =========================================================================

    mod invalid_floor_generation {
        use super::*;

        #[rstest]
        fn constructor() {
            let error = FloorError::invalid_floor_generation("Floor too small");
            assert!(matches!(error, FloorError::InvalidFloorGeneration { .. }));
        }

        #[rstest]
        fn display() {
            let error = FloorError::invalid_floor_generation("Floor too small");
            assert_eq!(
                format!("{}", error),
                "Invalid floor generation: Floor too small"
            );
        }

        #[rstest]
        fn with_string_reason() {
            let error =
                FloorError::invalid_floor_generation(String::from("Not enough space for rooms"));
            assert_eq!(
                format!("{}", error),
                "Invalid floor generation: Not enough space for rooms"
            );
        }

        #[rstest]
        fn is_not_recoverable() {
            let error = FloorError::invalid_floor_generation("error");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn is_not_position_error() {
            let error = FloorError::invalid_floor_generation("error");
            assert!(!error.is_position_error());
        }

        #[rstest]
        fn is_generation_error() {
            let error = FloorError::invalid_floor_generation("error");
            assert!(error.is_generation_error());
        }
    }

    // =========================================================================
    // Common Tests
    // =========================================================================

    mod common {
        use super::*;

        #[rstest]
        fn equality() {
            let error1 = FloorError::floor_not_found(1);
            let error2 = FloorError::floor_not_found(1);
            let error3 = FloorError::floor_not_found(2);

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn clone() {
            let error = FloorError::position_out_of_bounds((10, 10), (80, 40));
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn debug_format() {
            let error = FloorError::rooms_not_connected();
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("RoomsNotConnected"));
        }

        #[rstest]
        fn error_trait_implementation() {
            fn accepts_error<E: Error>(_: E) {}

            accepts_error(FloorError::floor_not_found(1));
        }
    }
}
