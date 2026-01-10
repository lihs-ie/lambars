//! Game status and outcome enums.
//!
//! This module provides enums for tracking game session state and outcomes.

use std::fmt;

// =============================================================================
// GameStatus
// =============================================================================

/// Current status of a game session.
///
/// Represents the current state of an ongoing game session.
///
/// # Examples
///
/// ```
/// use roguelike_domain::game_session::GameStatus;
///
/// let status = GameStatus::InProgress;
/// assert!(status.is_active());
/// assert!(!status.is_terminal());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameStatus {
    /// Game is currently being played.
    #[default]
    InProgress,
    /// Player has achieved victory.
    Victory,
    /// Player has been defeated.
    Defeat,
    /// Game is temporarily paused.
    Paused,
}

impl GameStatus {
    /// Returns true if the game is in an active state (can be played).
    ///
    /// Active states are `InProgress` and `Paused`.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::InProgress.is_active());
    /// assert!(GameStatus::Paused.is_active());
    /// assert!(!GameStatus::Victory.is_active());
    /// assert!(!GameStatus::Defeat.is_active());
    /// ```
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::InProgress | Self::Paused)
    }

    /// Returns true if the game has reached a terminal state.
    ///
    /// Terminal states are `Victory` and `Defeat`.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::Victory.is_terminal());
    /// assert!(GameStatus::Defeat.is_terminal());
    /// assert!(!GameStatus::InProgress.is_terminal());
    /// assert!(!GameStatus::Paused.is_terminal());
    /// ```
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Victory | Self::Defeat)
    }

    /// Returns true if the game is in progress.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::InProgress.is_in_progress());
    /// assert!(!GameStatus::Paused.is_in_progress());
    /// ```
    #[must_use]
    pub const fn is_in_progress(&self) -> bool {
        matches!(self, Self::InProgress)
    }

    /// Returns true if the game is paused.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::Paused.is_paused());
    /// assert!(!GameStatus::InProgress.is_paused());
    /// ```
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Returns true if the game ended in victory.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::Victory.is_victory());
    /// assert!(!GameStatus::Defeat.is_victory());
    /// ```
    #[must_use]
    pub const fn is_victory(&self) -> bool {
        matches!(self, Self::Victory)
    }

    /// Returns true if the game ended in defeat.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameStatus;
    ///
    /// assert!(GameStatus::Defeat.is_defeat());
    /// assert!(!GameStatus::Victory.is_defeat());
    /// ```
    #[must_use]
    pub const fn is_defeat(&self) -> bool {
        matches!(self, Self::Defeat)
    }

    /// Converts to a game outcome if the status is terminal.
    ///
    /// Returns `None` for non-terminal states.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::{GameStatus, GameOutcome};
    ///
    /// assert_eq!(GameStatus::Victory.to_outcome(), Some(GameOutcome::Victory));
    /// assert_eq!(GameStatus::Defeat.to_outcome(), Some(GameOutcome::Defeat));
    /// assert_eq!(GameStatus::InProgress.to_outcome(), None);
    /// ```
    #[must_use]
    pub const fn to_outcome(&self) -> Option<GameOutcome> {
        match self {
            Self::Victory => Some(GameOutcome::Victory),
            Self::Defeat => Some(GameOutcome::Defeat),
            Self::InProgress | Self::Paused => None,
        }
    }
}

impl fmt::Display for GameStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_text = match self {
            Self::InProgress => "In Progress",
            Self::Victory => "Victory",
            Self::Defeat => "Defeat",
            Self::Paused => "Paused",
        };
        write!(formatter, "{}", status_text)
    }
}

// =============================================================================
// GameOutcome
// =============================================================================

/// Final outcome of a completed game session.
///
/// Represents how a game session ended.
///
/// # Examples
///
/// ```
/// use roguelike_domain::game_session::GameOutcome;
///
/// let outcome = GameOutcome::Victory;
/// assert!(outcome.is_success());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    /// Player achieved victory by completing the objective.
    Victory,
    /// Player was defeated (e.g., health reached zero).
    Defeat,
    /// Player abandoned the game session.
    Abandoned,
}

impl GameOutcome {
    /// Returns true if the outcome represents a successful completion.
    ///
    /// Only `Victory` is considered a success.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameOutcome;
    ///
    /// assert!(GameOutcome::Victory.is_success());
    /// assert!(!GameOutcome::Defeat.is_success());
    /// assert!(!GameOutcome::Abandoned.is_success());
    /// ```
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Victory)
    }

    /// Returns true if the outcome represents a failure.
    ///
    /// Both `Defeat` and `Abandoned` are considered failures.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameOutcome;
    ///
    /// assert!(GameOutcome::Defeat.is_failure());
    /// assert!(GameOutcome::Abandoned.is_failure());
    /// assert!(!GameOutcome::Victory.is_failure());
    /// ```
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Defeat | Self::Abandoned)
    }

    /// Returns true if the game was abandoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::GameOutcome;
    ///
    /// assert!(GameOutcome::Abandoned.is_abandoned());
    /// assert!(!GameOutcome::Victory.is_abandoned());
    /// ```
    #[must_use]
    pub const fn is_abandoned(&self) -> bool {
        matches!(self, Self::Abandoned)
    }

    /// Converts to a game status.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::game_session::{GameOutcome, GameStatus};
    ///
    /// assert_eq!(GameOutcome::Victory.to_status(), GameStatus::Victory);
    /// assert_eq!(GameOutcome::Defeat.to_status(), GameStatus::Defeat);
    /// assert_eq!(GameOutcome::Abandoned.to_status(), GameStatus::Defeat);
    /// ```
    #[must_use]
    pub const fn to_status(&self) -> GameStatus {
        match self {
            Self::Victory => GameStatus::Victory,
            Self::Defeat | Self::Abandoned => GameStatus::Defeat,
        }
    }
}

impl fmt::Display for GameOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let outcome_text = match self {
            Self::Victory => "Victory",
            Self::Defeat => "Defeat",
            Self::Abandoned => "Abandoned",
        };
        write!(formatter, "{}", outcome_text)
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
    // GameStatus Tests
    // =========================================================================

    mod game_status {
        use super::*;

        #[rstest]
        #[case(GameStatus::InProgress, true)]
        #[case(GameStatus::Paused, true)]
        #[case(GameStatus::Victory, false)]
        #[case(GameStatus::Defeat, false)]
        fn is_active(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_active(), expected);
        }

        #[rstest]
        #[case(GameStatus::Victory, true)]
        #[case(GameStatus::Defeat, true)]
        #[case(GameStatus::InProgress, false)]
        #[case(GameStatus::Paused, false)]
        fn is_terminal(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_terminal(), expected);
        }

        #[rstest]
        #[case(GameStatus::InProgress, true)]
        #[case(GameStatus::Paused, false)]
        #[case(GameStatus::Victory, false)]
        #[case(GameStatus::Defeat, false)]
        fn is_in_progress(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_in_progress(), expected);
        }

        #[rstest]
        #[case(GameStatus::Paused, true)]
        #[case(GameStatus::InProgress, false)]
        #[case(GameStatus::Victory, false)]
        #[case(GameStatus::Defeat, false)]
        fn is_paused(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_paused(), expected);
        }

        #[rstest]
        #[case(GameStatus::Victory, true)]
        #[case(GameStatus::Defeat, false)]
        #[case(GameStatus::InProgress, false)]
        #[case(GameStatus::Paused, false)]
        fn is_victory(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_victory(), expected);
        }

        #[rstest]
        #[case(GameStatus::Defeat, true)]
        #[case(GameStatus::Victory, false)]
        #[case(GameStatus::InProgress, false)]
        #[case(GameStatus::Paused, false)]
        fn is_defeat(#[case] status: GameStatus, #[case] expected: bool) {
            assert_eq!(status.is_defeat(), expected);
        }

        #[rstest]
        #[case(GameStatus::Victory, Some(GameOutcome::Victory))]
        #[case(GameStatus::Defeat, Some(GameOutcome::Defeat))]
        #[case(GameStatus::InProgress, None)]
        #[case(GameStatus::Paused, None)]
        fn to_outcome(#[case] status: GameStatus, #[case] expected: Option<GameOutcome>) {
            assert_eq!(status.to_outcome(), expected);
        }

        #[rstest]
        #[case(GameStatus::InProgress, "In Progress")]
        #[case(GameStatus::Victory, "Victory")]
        #[case(GameStatus::Defeat, "Defeat")]
        #[case(GameStatus::Paused, "Paused")]
        fn display(#[case] status: GameStatus, #[case] expected: &str) {
            assert_eq!(format!("{}", status), expected);
        }

        #[rstest]
        fn default_is_in_progress() {
            assert_eq!(GameStatus::default(), GameStatus::InProgress);
        }

        #[rstest]
        fn equality() {
            assert_eq!(GameStatus::InProgress, GameStatus::InProgress);
            assert_ne!(GameStatus::InProgress, GameStatus::Victory);
        }

        #[rstest]
        fn clone() {
            let status = GameStatus::InProgress;
            let cloned = status;
            assert_eq!(status, cloned);
        }

        #[rstest]
        fn copy() {
            let status = GameStatus::Victory;
            let copied = status;
            assert_eq!(status, copied);
        }

        #[rstest]
        fn hash() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(GameStatus::InProgress);
            set.insert(GameStatus::Victory);
            set.insert(GameStatus::InProgress); // Duplicate

            assert_eq!(set.len(), 2);
        }

        #[rstest]
        fn debug_format() {
            let status = GameStatus::InProgress;
            let debug = format!("{:?}", status);
            assert!(debug.contains("InProgress"));
        }
    }

    // =========================================================================
    // GameOutcome Tests
    // =========================================================================

    mod game_outcome {
        use super::*;

        #[rstest]
        #[case(GameOutcome::Victory, true)]
        #[case(GameOutcome::Defeat, false)]
        #[case(GameOutcome::Abandoned, false)]
        fn is_success(#[case] outcome: GameOutcome, #[case] expected: bool) {
            assert_eq!(outcome.is_success(), expected);
        }

        #[rstest]
        #[case(GameOutcome::Victory, false)]
        #[case(GameOutcome::Defeat, true)]
        #[case(GameOutcome::Abandoned, true)]
        fn is_failure(#[case] outcome: GameOutcome, #[case] expected: bool) {
            assert_eq!(outcome.is_failure(), expected);
        }

        #[rstest]
        #[case(GameOutcome::Abandoned, true)]
        #[case(GameOutcome::Victory, false)]
        #[case(GameOutcome::Defeat, false)]
        fn is_abandoned(#[case] outcome: GameOutcome, #[case] expected: bool) {
            assert_eq!(outcome.is_abandoned(), expected);
        }

        #[rstest]
        #[case(GameOutcome::Victory, GameStatus::Victory)]
        #[case(GameOutcome::Defeat, GameStatus::Defeat)]
        #[case(GameOutcome::Abandoned, GameStatus::Defeat)]
        fn to_status(#[case] outcome: GameOutcome, #[case] expected: GameStatus) {
            assert_eq!(outcome.to_status(), expected);
        }

        #[rstest]
        #[case(GameOutcome::Victory, "Victory")]
        #[case(GameOutcome::Defeat, "Defeat")]
        #[case(GameOutcome::Abandoned, "Abandoned")]
        fn display(#[case] outcome: GameOutcome, #[case] expected: &str) {
            assert_eq!(format!("{}", outcome), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(GameOutcome::Victory, GameOutcome::Victory);
            assert_ne!(GameOutcome::Victory, GameOutcome::Defeat);
        }

        #[rstest]
        fn clone() {
            let outcome = GameOutcome::Victory;
            let cloned = outcome;
            assert_eq!(outcome, cloned);
        }

        #[rstest]
        fn copy() {
            let outcome = GameOutcome::Defeat;
            let copied = outcome;
            assert_eq!(outcome, copied);
        }

        #[rstest]
        fn hash() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(GameOutcome::Victory);
            set.insert(GameOutcome::Defeat);
            set.insert(GameOutcome::Abandoned);
            set.insert(GameOutcome::Victory); // Duplicate

            assert_eq!(set.len(), 3);
        }

        #[rstest]
        fn debug_format() {
            let outcome = GameOutcome::Victory;
            let debug = format!("{:?}", outcome);
            assert!(debug.contains("Victory"));
        }
    }

    // =========================================================================
    // Conversion Tests
    // =========================================================================

    mod conversions {
        use super::*;

        #[rstest]
        fn status_to_outcome_roundtrip_for_victory() {
            let status = GameStatus::Victory;
            let outcome = status.to_outcome().unwrap();
            let back_to_status = outcome.to_status();
            assert_eq!(status, back_to_status);
        }

        #[rstest]
        fn status_to_outcome_roundtrip_for_defeat() {
            let status = GameStatus::Defeat;
            let outcome = status.to_outcome().unwrap();
            let back_to_status = outcome.to_status();
            assert_eq!(status, back_to_status);
        }

        #[rstest]
        fn abandoned_maps_to_defeat_status() {
            // Abandoned has no direct GameStatus equivalent
            // It maps to Defeat as it's a form of failure
            let outcome = GameOutcome::Abandoned;
            assert_eq!(outcome.to_status(), GameStatus::Defeat);
        }
    }
}
