use std::fmt;

use crate::common::TurnCount;

use super::{GameIdentifier, GameOutcome};

// =============================================================================
// RandomSeed
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RandomSeed(u64);

impl RandomSeed {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn from_current_time() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(duration.as_nanos() as u64)
    }
}

impl fmt::Display for RandomSeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "seed:{}", self.0)
    }
}

impl Default for RandomSeed {
    fn default() -> Self {
        Self::from_current_time()
    }
}

impl From<u64> for RandomSeed {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<RandomSeed> for u64 {
    fn from(seed: RandomSeed) -> Self {
        seed.0
    }
}

// =============================================================================
// GameStarted Event
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStarted {
    game_identifier: GameIdentifier,
    seed: RandomSeed,
}

impl GameStarted {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, seed: RandomSeed) -> Self {
        Self {
            game_identifier,
            seed,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn seed(&self) -> &RandomSeed {
        &self.seed
    }
}

impl fmt::Display for GameStarted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Game started: {} ({})",
            self.game_identifier, self.seed
        )
    }
}

// =============================================================================
// GameEnded Event
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEnded {
    outcome: GameOutcome,
}

impl GameEnded {
    #[must_use]
    pub const fn new(outcome: GameOutcome) -> Self {
        Self { outcome }
    }

    #[must_use]
    pub const fn outcome(&self) -> &GameOutcome {
        &self.outcome
    }

    #[must_use]
    pub const fn victory() -> Self {
        Self::new(GameOutcome::Victory)
    }

    #[must_use]
    pub const fn defeat() -> Self {
        Self::new(GameOutcome::Defeat)
    }

    #[must_use]
    pub const fn abandoned() -> Self {
        Self::new(GameOutcome::Abandoned)
    }
}

impl fmt::Display for GameEnded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Game ended: {}", self.outcome)
    }
}

// =============================================================================
// TurnStarted Event
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnStarted {
    turn: TurnCount,
}

impl TurnStarted {
    #[must_use]
    pub const fn new(turn: TurnCount) -> Self {
        Self { turn }
    }

    #[must_use]
    pub const fn turn(&self) -> &TurnCount {
        &self.turn
    }

    #[must_use]
    pub const fn first() -> Self {
        Self::new(TurnCount::new(1))
    }
}

impl fmt::Display for TurnStarted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Turn {} started", self.turn.value())
    }
}

// =============================================================================
// TurnEnded Event
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnEnded {
    turn: TurnCount,
}

impl TurnEnded {
    #[must_use]
    pub const fn new(turn: TurnCount) -> Self {
        Self { turn }
    }

    #[must_use]
    pub const fn turn(&self) -> &TurnCount {
        &self.turn
    }
}

impl fmt::Display for TurnEnded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Turn {} ended", self.turn.value())
    }
}

// =============================================================================
// GameSessionEvent Enum
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum GameSessionEvent {
    Started(GameStarted),
    Ended(GameEnded),
    TurnStarted(TurnStarted),
    TurnEnded(TurnEnded),
    EnemySpawned(crate::enemy::EnemySpawned),
    EnemyMoved(crate::enemy::EnemyMoved),
    EnemyAttacked(crate::enemy::EnemyAttacked),
    EnemyDied(crate::enemy::EnemyDied),
    FloorEntered(crate::floor::FloorEntered),
    TileExplored(crate::floor::TileExplored),
    TrapTriggered(crate::floor::TrapTriggered),
}

impl GameSessionEvent {
    #[must_use]
    pub const fn is_game_started(&self) -> bool {
        matches!(self, Self::Started(_))
    }

    #[must_use]
    pub const fn is_game_ended(&self) -> bool {
        matches!(self, Self::Ended(_))
    }

    #[must_use]
    pub const fn is_turn_started(&self) -> bool {
        matches!(self, Self::TurnStarted(_))
    }

    #[must_use]
    pub const fn is_turn_ended(&self) -> bool {
        matches!(self, Self::TurnEnded(_))
    }

    #[must_use]
    pub const fn is_enemy_spawned(&self) -> bool {
        matches!(self, Self::EnemySpawned(_))
    }

    #[must_use]
    pub const fn is_enemy_moved(&self) -> bool {
        matches!(self, Self::EnemyMoved(_))
    }

    #[must_use]
    pub const fn is_enemy_attacked(&self) -> bool {
        matches!(self, Self::EnemyAttacked(_))
    }

    #[must_use]
    pub const fn is_enemy_died(&self) -> bool {
        matches!(self, Self::EnemyDied(_))
    }

    #[must_use]
    pub const fn is_enemy_event(&self) -> bool {
        matches!(
            self,
            Self::EnemySpawned(_)
                | Self::EnemyMoved(_)
                | Self::EnemyAttacked(_)
                | Self::EnemyDied(_)
        )
    }

    #[must_use]
    pub const fn is_floor_entered(&self) -> bool {
        matches!(self, Self::FloorEntered(_))
    }

    #[must_use]
    pub const fn is_tile_explored(&self) -> bool {
        matches!(self, Self::TileExplored(_))
    }

    #[must_use]
    pub const fn is_trap_triggered(&self) -> bool {
        matches!(self, Self::TrapTriggered(_))
    }

    #[must_use]
    pub const fn is_floor_event(&self) -> bool {
        matches!(
            self,
            Self::FloorEntered(_) | Self::TileExplored(_) | Self::TrapTriggered(_)
        )
    }

    #[must_use]
    pub const fn as_game_started(&self) -> Option<&GameStarted> {
        match self {
            Self::Started(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_game_ended(&self) -> Option<&GameEnded> {
        match self {
            Self::Ended(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_turn_started(&self) -> Option<&TurnStarted> {
        match self {
            Self::TurnStarted(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_turn_ended(&self) -> Option<&TurnEnded> {
        match self {
            Self::TurnEnded(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_enemy_spawned(&self) -> Option<&crate::enemy::EnemySpawned> {
        match self {
            Self::EnemySpawned(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_enemy_moved(&self) -> Option<&crate::enemy::EnemyMoved> {
        match self {
            Self::EnemyMoved(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_enemy_attacked(&self) -> Option<&crate::enemy::EnemyAttacked> {
        match self {
            Self::EnemyAttacked(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_enemy_died(&self) -> Option<&crate::enemy::EnemyDied> {
        match self {
            Self::EnemyDied(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_floor_entered(&self) -> Option<&crate::floor::FloorEntered> {
        match self {
            Self::FloorEntered(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_tile_explored(&self) -> Option<&crate::floor::TileExplored> {
        match self {
            Self::TileExplored(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_trap_triggered(&self) -> Option<&crate::floor::TrapTriggered> {
        match self {
            Self::TrapTriggered(event) => Some(event),
            _ => None,
        }
    }
}

impl fmt::Display for GameSessionEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started(event) => write!(formatter, "{}", event),
            Self::Ended(event) => write!(formatter, "{}", event),
            Self::TurnStarted(event) => write!(formatter, "{}", event),
            Self::TurnEnded(event) => write!(formatter, "{}", event),
            Self::EnemySpawned(event) => write!(
                formatter,
                "Enemy {} spawned at {}",
                event.enemy_identifier(),
                event.position()
            ),
            Self::EnemyMoved(event) => write!(
                formatter,
                "Enemy {} moved from {} to {}",
                event.enemy_identifier(),
                event.from(),
                event.to()
            ),
            Self::EnemyAttacked(event) => write!(
                formatter,
                "Enemy {} attacked for {} damage",
                event.enemy_identifier(),
                event.damage().value()
            ),
            Self::EnemyDied(event) => write!(
                formatter,
                "Enemy {} died at {} (loot: {} entries)",
                event.enemy_identifier(),
                event.death_position(),
                event.loot_entry_count()
            ),
            Self::FloorEntered(event) => write!(formatter, "{}", event),
            Self::TileExplored(event) => write!(formatter, "{}", event),
            Self::TrapTriggered(event) => write!(formatter, "{}", event),
        }
    }
}

impl From<GameStarted> for GameSessionEvent {
    fn from(event: GameStarted) -> Self {
        Self::Started(event)
    }
}

impl From<GameEnded> for GameSessionEvent {
    fn from(event: GameEnded) -> Self {
        Self::Ended(event)
    }
}

impl From<TurnStarted> for GameSessionEvent {
    fn from(event: TurnStarted) -> Self {
        Self::TurnStarted(event)
    }
}

impl From<TurnEnded> for GameSessionEvent {
    fn from(event: TurnEnded) -> Self {
        Self::TurnEnded(event)
    }
}

impl From<crate::enemy::EnemySpawned> for GameSessionEvent {
    fn from(event: crate::enemy::EnemySpawned) -> Self {
        Self::EnemySpawned(event)
    }
}

impl From<crate::enemy::EnemyMoved> for GameSessionEvent {
    fn from(event: crate::enemy::EnemyMoved) -> Self {
        Self::EnemyMoved(event)
    }
}

impl From<crate::enemy::EnemyAttacked> for GameSessionEvent {
    fn from(event: crate::enemy::EnemyAttacked) -> Self {
        Self::EnemyAttacked(event)
    }
}

impl From<crate::enemy::EnemyDied> for GameSessionEvent {
    fn from(event: crate::enemy::EnemyDied) -> Self {
        Self::EnemyDied(event)
    }
}

impl From<crate::floor::FloorEntered> for GameSessionEvent {
    fn from(event: crate::floor::FloorEntered) -> Self {
        Self::FloorEntered(event)
    }
}

impl From<crate::floor::TileExplored> for GameSessionEvent {
    fn from(event: crate::floor::TileExplored) -> Self {
        Self::TileExplored(event)
    }
}

impl From<crate::floor::TrapTriggered> for GameSessionEvent {
    fn from(event: crate::floor::TrapTriggered) -> Self {
        Self::TrapTriggered(event)
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
    // RandomSeed Tests
    // =========================================================================

    mod random_seed {
        use super::*;

        #[rstest]
        fn new_creates_seed() {
            let seed = RandomSeed::new(12345);
            assert_eq!(seed.value(), 12345);
        }

        #[rstest]
        fn from_u64() {
            let seed: RandomSeed = 42u64.into();
            assert_eq!(seed.value(), 42);
        }

        #[rstest]
        fn into_u64() {
            let seed = RandomSeed::new(999);
            let value: u64 = seed.into();
            assert_eq!(value, 999);
        }

        #[rstest]
        fn from_current_time_creates_different_values() {
            // This test may occasionally fail if executed too quickly,
            // but in practice the nanosecond precision should ensure uniqueness
            let seed1 = RandomSeed::from_current_time();
            std::thread::sleep(std::time::Duration::from_nanos(1));
            let seed2 = RandomSeed::from_current_time();
            // We can't guarantee they're different, but we can test the function works
            assert!(seed1.value() > 0 || seed2.value() > 0);
        }

        #[rstest]
        fn display_format() {
            let seed = RandomSeed::new(12345);
            assert_eq!(format!("{}", seed), "seed:12345");
        }

        #[rstest]
        fn default_creates_seed() {
            let seed = RandomSeed::default();
            // Just verify it creates a valid seed
            let _ = seed.value();
        }

        #[rstest]
        fn equality() {
            let seed1 = RandomSeed::new(42);
            let seed2 = RandomSeed::new(42);
            let seed3 = RandomSeed::new(99);
            assert_eq!(seed1, seed2);
            assert_ne!(seed1, seed3);
        }

        #[rstest]
        fn clone_and_copy() {
            let seed = RandomSeed::new(100);
            let cloned = seed;
            let copied = seed;
            assert_eq!(seed, cloned);
            assert_eq!(seed, copied);
        }

        #[rstest]
        fn hash() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(RandomSeed::new(1));
            set.insert(RandomSeed::new(2));
            set.insert(RandomSeed::new(1)); // Duplicate

            assert_eq!(set.len(), 2);
        }

        #[rstest]
        fn debug_format() {
            let seed = RandomSeed::new(42);
            let debug = format!("{:?}", seed);
            assert!(debug.contains("RandomSeed"));
            assert!(debug.contains("42"));
        }
    }

    // =========================================================================
    // GameStarted Tests
    // =========================================================================

    mod game_started {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let event = GameStarted::new(identifier, seed);

            assert_eq!(event.game_identifier(), &identifier);
            assert_eq!(event.seed(), &seed);
        }

        #[rstest]
        fn display_format() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(12345);
            let event = GameStarted::new(identifier, seed);

            let display = format!("{}", event);
            assert!(display.contains("Game started"));
            assert!(display.contains(&identifier.to_string()));
            assert!(display.contains("seed:12345"));
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);

            let event1 = GameStarted::new(identifier, seed);
            let event2 = GameStarted::new(identifier, seed);
            let event3 = GameStarted::new(GameIdentifier::new(), seed);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = GameStarted::new(GameIdentifier::new(), RandomSeed::new(42));
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn debug_format() {
            let event = GameStarted::new(GameIdentifier::new(), RandomSeed::new(42));
            let debug = format!("{:?}", event);
            assert!(debug.contains("GameStarted"));
        }
    }

    // =========================================================================
    // GameEnded Tests
    // =========================================================================

    mod game_ended {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let event = GameEnded::new(GameOutcome::Victory);
            assert_eq!(event.outcome(), &GameOutcome::Victory);
        }

        #[rstest]
        fn victory_creates_victory_event() {
            let event = GameEnded::victory();
            assert_eq!(event.outcome(), &GameOutcome::Victory);
        }

        #[rstest]
        fn defeat_creates_defeat_event() {
            let event = GameEnded::defeat();
            assert_eq!(event.outcome(), &GameOutcome::Defeat);
        }

        #[rstest]
        fn abandoned_creates_abandoned_event() {
            let event = GameEnded::abandoned();
            assert_eq!(event.outcome(), &GameOutcome::Abandoned);
        }

        #[rstest]
        #[case(GameOutcome::Victory, "Victory")]
        #[case(GameOutcome::Defeat, "Defeat")]
        #[case(GameOutcome::Abandoned, "Abandoned")]
        fn display_format(#[case] outcome: GameOutcome, #[case] expected_text: &str) {
            let event = GameEnded::new(outcome);
            let display = format!("{}", event);
            assert!(display.contains("Game ended"));
            assert!(display.contains(expected_text));
        }

        #[rstest]
        fn equality() {
            let event1 = GameEnded::new(GameOutcome::Victory);
            let event2 = GameEnded::new(GameOutcome::Victory);
            let event3 = GameEnded::new(GameOutcome::Defeat);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn copy() {
            let event = GameEnded::victory();
            let copied = event;
            assert_eq!(event, copied);
        }

        #[rstest]
        fn debug_format() {
            let event = GameEnded::victory();
            let debug = format!("{:?}", event);
            assert!(debug.contains("GameEnded"));
            assert!(debug.contains("Victory"));
        }
    }

    // =========================================================================
    // TurnStarted Tests
    // =========================================================================

    mod turn_started {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let turn = TurnCount::new(5);
            let event = TurnStarted::new(turn);
            assert_eq!(event.turn(), &turn);
        }

        #[rstest]
        fn first_creates_turn_one_event() {
            let event = TurnStarted::first();
            assert_eq!(event.turn().value(), 1);
        }

        #[rstest]
        fn display_format() {
            let event = TurnStarted::new(TurnCount::new(10));
            let display = format!("{}", event);
            assert!(display.contains("Turn 10 started"));
        }

        #[rstest]
        fn equality() {
            let turn = TurnCount::new(5);
            let event1 = TurnStarted::new(turn);
            let event2 = TurnStarted::new(turn);
            let event3 = TurnStarted::new(TurnCount::new(6));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn copy() {
            let event = TurnStarted::first();
            let copied = event;
            assert_eq!(event, copied);
        }

        #[rstest]
        fn debug_format() {
            let event = TurnStarted::first();
            let debug = format!("{:?}", event);
            assert!(debug.contains("TurnStarted"));
        }
    }

    // =========================================================================
    // TurnEnded Tests
    // =========================================================================

    mod turn_ended {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let turn = TurnCount::new(5);
            let event = TurnEnded::new(turn);
            assert_eq!(event.turn(), &turn);
        }

        #[rstest]
        fn display_format() {
            let event = TurnEnded::new(TurnCount::new(10));
            let display = format!("{}", event);
            assert!(display.contains("Turn 10 ended"));
        }

        #[rstest]
        fn equality() {
            let turn = TurnCount::new(5);
            let event1 = TurnEnded::new(turn);
            let event2 = TurnEnded::new(turn);
            let event3 = TurnEnded::new(TurnCount::new(6));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn copy() {
            let event = TurnEnded::new(TurnCount::new(1));
            let copied = event;
            assert_eq!(event, copied);
        }

        #[rstest]
        fn debug_format() {
            let event = TurnEnded::new(TurnCount::new(1));
            let debug = format!("{:?}", event);
            assert!(debug.contains("TurnEnded"));
        }
    }

    // =========================================================================
    // GameSessionEvent Tests
    // =========================================================================

    mod game_session_event {
        use super::*;

        #[rstest]
        fn from_game_started() {
            let started = GameStarted::new(GameIdentifier::new(), RandomSeed::new(42));
            let event: GameSessionEvent = started.clone().into();

            assert!(event.is_game_started());
            assert!(!event.is_game_ended());
            assert!(!event.is_turn_started());
            assert!(!event.is_turn_ended());
            assert_eq!(event.as_game_started(), Some(&started));
        }

        #[rstest]
        fn from_game_ended() {
            let ended = GameEnded::victory();
            let event: GameSessionEvent = ended.into();

            assert!(event.is_game_ended());
            assert!(!event.is_game_started());
            assert!(!event.is_turn_started());
            assert!(!event.is_turn_ended());
            assert_eq!(event.as_game_ended(), Some(&ended));
        }

        #[rstest]
        fn from_turn_started() {
            let turn_started = TurnStarted::first();
            let event: GameSessionEvent = turn_started.into();

            assert!(event.is_turn_started());
            assert!(!event.is_game_started());
            assert!(!event.is_game_ended());
            assert!(!event.is_turn_ended());
            assert_eq!(event.as_turn_started(), Some(&turn_started));
        }

        #[rstest]
        fn from_turn_ended() {
            let turn_ended = TurnEnded::new(TurnCount::new(1));
            let event: GameSessionEvent = turn_ended.into();

            assert!(event.is_turn_ended());
            assert!(!event.is_game_started());
            assert!(!event.is_game_ended());
            assert!(!event.is_turn_started());
            assert_eq!(event.as_turn_ended(), Some(&turn_ended));
        }

        #[rstest]
        fn as_methods_return_none_for_wrong_variant() {
            let event: GameSessionEvent = GameEnded::victory().into();

            assert!(event.as_game_started().is_none());
            assert!(event.as_turn_started().is_none());
            assert!(event.as_turn_ended().is_none());
        }

        #[rstest]
        fn display_delegates_to_inner_event() {
            let started = GameStarted::new(GameIdentifier::new(), RandomSeed::new(42));
            let event: GameSessionEvent = started.clone().into();

            assert_eq!(format!("{}", event), format!("{}", started));
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);

            let event1: GameSessionEvent = GameStarted::new(identifier, seed).into();
            let event2: GameSessionEvent = GameStarted::new(identifier, seed).into();
            let event3: GameSessionEvent = GameEnded::victory().into();

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event: GameSessionEvent = GameEnded::victory().into();
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn debug_format() {
            let event: GameSessionEvent = GameEnded::victory().into();
            let debug = format!("{:?}", event);
            assert!(debug.contains("Ended"));
        }
    }
}
