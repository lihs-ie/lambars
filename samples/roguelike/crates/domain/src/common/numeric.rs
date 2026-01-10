//! Numeric value objects for game mechanics.
//!
//! This module provides type-safe numeric types with validation
//! for health, mana, experience, level, and combat statistics.

use std::fmt;
use std::ops::Add;

use super::errors::ValidationError;

// =============================================================================
// Health
// =============================================================================

/// Health points for characters and entities.
///
/// Health values are constrained to 0 <= value <= MAX_HEALTH (9999).
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Health;
///
/// let health = Health::new(100).unwrap();
/// assert_eq!(health.value(), 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Health(u32);

impl Health {
    /// The maximum allowed health value.
    pub const MAX_HEALTH: u32 = 9999;

    /// Creates a new Health with the given value.
    ///
    /// Returns an error if the value exceeds MAX_HEALTH.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Health;
    ///
    /// let health = Health::new(100).unwrap();
    /// assert_eq!(health.value(), 100);
    ///
    /// assert!(Health::new(10000).is_err());
    /// ```
    pub fn new(value: u32) -> Result<Self, ValidationError> {
        if value > Self::MAX_HEALTH {
            return Err(ValidationError::out_of_range(
                "health",
                0,
                Self::MAX_HEALTH,
                value,
            ));
        }
        Ok(Self(value))
    }

    /// Returns the health value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero health.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns maximum health.
    #[must_use]
    pub const fn max() -> Self {
        Self(Self::MAX_HEALTH)
    }

    /// Adds health, saturating at MAX_HEALTH.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Health;
    ///
    /// let health = Health::new(9000).unwrap();
    /// let healed = health.saturating_add(2000);
    /// assert_eq!(healed.value(), Health::MAX_HEALTH);
    /// ```
    #[must_use]
    pub const fn saturating_add(&self, amount: u32) -> Self {
        let new_value = self.0.saturating_add(amount);
        if new_value > Self::MAX_HEALTH {
            Self(Self::MAX_HEALTH)
        } else {
            Self(new_value)
        }
    }

    /// Subtracts health, saturating at 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Health;
    ///
    /// let health = Health::new(50).unwrap();
    /// let damaged = health.saturating_sub(100);
    /// assert_eq!(damaged.value(), 0);
    /// ```
    #[must_use]
    pub const fn saturating_sub(&self, amount: u32) -> Self {
        Self(self.0.saturating_sub(amount))
    }

    /// Returns true if health is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Health {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Mana
// =============================================================================

/// Mana points for magic abilities.
///
/// Mana values are constrained to 0 <= value <= MAX_MANA (999).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mana(u32);

impl Mana {
    /// The maximum allowed mana value.
    pub const MAX_MANA: u32 = 999;

    /// Creates a new Mana with the given value.
    ///
    /// Returns an error if the value exceeds MAX_MANA.
    pub fn new(value: u32) -> Result<Self, ValidationError> {
        if value > Self::MAX_MANA {
            return Err(ValidationError::out_of_range(
                "mana",
                0,
                Self::MAX_MANA,
                value,
            ));
        }
        Ok(Self(value))
    }

    /// Returns the mana value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero mana.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns maximum mana.
    #[must_use]
    pub const fn max() -> Self {
        Self(Self::MAX_MANA)
    }

    /// Adds mana, saturating at MAX_MANA.
    #[must_use]
    pub const fn saturating_add(&self, amount: u32) -> Self {
        let new_value = self.0.saturating_add(amount);
        if new_value > Self::MAX_MANA {
            Self(Self::MAX_MANA)
        } else {
            Self(new_value)
        }
    }

    /// Subtracts mana, saturating at 0.
    #[must_use]
    pub const fn saturating_sub(&self, amount: u32) -> Self {
        Self(self.0.saturating_sub(amount))
    }

    /// Returns true if mana is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Mana {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Experience
// =============================================================================

/// Experience points for character progression.
///
/// Experience has no upper limit and uses u64 for large values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Experience(u64);

impl Experience {
    /// Creates a new Experience with the given value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the experience value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Returns zero experience.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Adds experience and returns a new Experience.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Experience;
    ///
    /// let experience = Experience::new(100);
    /// let gained = experience.add(50);
    /// assert_eq!(gained.value(), 150);
    /// ```
    #[must_use]
    pub const fn add(&self, amount: u64) -> Self {
        Self(self.0.saturating_add(amount))
    }
}

impl Add for Experience {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for Experience {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Level
// =============================================================================

/// Character level.
///
/// Level values are constrained to MIN_LEVEL (1) <= value <= MAX_LEVEL (99).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    /// The minimum level value.
    pub const MIN_LEVEL: u8 = 1;
    /// The maximum level value.
    pub const MAX_LEVEL: u8 = 99;

    /// Creates a new Level with the given value.
    ///
    /// Returns an error if the value is outside the valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Level;
    ///
    /// let level = Level::new(1).unwrap();
    /// assert_eq!(level.value(), 1);
    ///
    /// assert!(Level::new(0).is_err());
    /// assert!(Level::new(100).is_err());
    /// ```
    pub fn new(value: u8) -> Result<Self, ValidationError> {
        if !(Self::MIN_LEVEL..=Self::MAX_LEVEL).contains(&value) {
            return Err(ValidationError::out_of_range(
                "level",
                Self::MIN_LEVEL,
                Self::MAX_LEVEL,
                value,
            ));
        }
        Ok(Self(value))
    }

    /// Returns the level value.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns level 1.
    #[must_use]
    pub const fn one() -> Self {
        Self(Self::MIN_LEVEL)
    }

    /// Returns the maximum level.
    #[must_use]
    pub const fn max() -> Self {
        Self(Self::MAX_LEVEL)
    }

    /// Increases the level by 1.
    ///
    /// Returns None if already at max level.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Level;
    ///
    /// let level = Level::new(50).unwrap();
    /// let next = level.level_up().unwrap();
    /// assert_eq!(next.value(), 51);
    ///
    /// let max = Level::max();
    /// assert!(max.level_up().is_none());
    /// ```
    #[must_use]
    pub const fn level_up(&self) -> Option<Self> {
        if self.0 >= Self::MAX_LEVEL {
            None
        } else {
            Some(Self(self.0 + 1))
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Lv.{}", self.0)
    }
}

// =============================================================================
// Attack
// =============================================================================

/// Attack power value.
///
/// Attack has no validation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Attack(u32);

impl Attack {
    /// Creates a new Attack with the given value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the attack value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero attack.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Add for Attack {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for Attack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Defense
// =============================================================================

/// Defense power value.
///
/// Defense has no validation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Defense(u32);

impl Defense {
    /// Creates a new Defense with the given value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the defense value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero defense.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Add for Defense {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for Defense {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Speed
// =============================================================================

/// Speed value for action order.
///
/// Speed has no validation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Speed(u32);

impl Speed {
    /// Creates a new Speed with the given value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the speed value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero speed.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Add for Speed {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Damage
// =============================================================================

/// Damage value for combat calculations.
///
/// Damage has no validation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Damage(u32);

impl Damage {
    /// Creates a new Damage with the given value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the damage value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns zero damage.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Add for Damage {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for Damage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// TurnCount
// =============================================================================

/// Game turn counter.
///
/// TurnCount has no upper limit and uses u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TurnCount(u64);

impl TurnCount {
    /// Creates a new TurnCount with the given value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the turn count value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Returns turn zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns the next turn.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::TurnCount;
    ///
    /// let turn = TurnCount::new(10);
    /// assert_eq!(turn.next().value(), 11);
    /// ```
    #[must_use]
    pub const fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for TurnCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Turn {}", self.0)
    }
}

// =============================================================================
// FloorLevel
// =============================================================================

/// Dungeon floor level.
///
/// FloorLevel must be >= 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FloorLevel(u32);

impl FloorLevel {
    /// Creates a new FloorLevel with the given value.
    ///
    /// Returns an error if the value is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::FloorLevel;
    ///
    /// let floor = FloorLevel::new(1).unwrap();
    /// assert_eq!(floor.value(), 1);
    ///
    /// assert!(FloorLevel::new(0).is_err());
    /// ```
    pub fn new(value: u32) -> Result<Self, ValidationError> {
        if value == 0 {
            return Err(ValidationError::out_of_range(
                "floor_level",
                1,
                "unlimited",
                0,
            ));
        }
        Ok(Self(value))
    }

    /// Returns the floor level value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns the first floor (B1F).
    #[must_use]
    pub const fn first() -> Self {
        Self(1)
    }

    /// Descends to the next floor.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::FloorLevel;
    ///
    /// let floor = FloorLevel::new(5).unwrap();
    /// assert_eq!(floor.descend().value(), 6);
    /// ```
    #[must_use]
    pub const fn descend(&self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Ascends to the previous floor.
    ///
    /// Returns None if already on the first floor.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::FloorLevel;
    ///
    /// let floor = FloorLevel::new(5).unwrap();
    /// assert_eq!(floor.ascend().unwrap().value(), 4);
    ///
    /// let first = FloorLevel::first();
    /// assert!(first.ascend().is_none());
    /// ```
    #[must_use]
    pub const fn ascend(&self) -> Option<Self> {
        if self.0 <= 1 {
            None
        } else {
            Some(Self(self.0 - 1))
        }
    }
}

impl fmt::Display for FloorLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "B{}F", self.0)
    }
}

// =============================================================================
// Stat
// =============================================================================

/// Base stat value (e.g., strength, dexterity).
///
/// Stat values are constrained to MIN_STAT (1) <= value <= MAX_STAT (99).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Stat(u32);

impl Stat {
    /// The minimum stat value.
    pub const MIN_STAT: u32 = 1;
    /// The maximum stat value.
    pub const MAX_STAT: u32 = 99;

    /// Creates a new Stat with the given value.
    ///
    /// Returns an error if the value is outside the valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Stat;
    ///
    /// let stat = Stat::new(10).unwrap();
    /// assert_eq!(stat.value(), 10);
    /// ```
    pub fn new(value: u32) -> Result<Self, ValidationError> {
        if !(Self::MIN_STAT..=Self::MAX_STAT).contains(&value) {
            return Err(ValidationError::out_of_range(
                "stat",
                Self::MIN_STAT,
                Self::MAX_STAT,
                value,
            ));
        }
        Ok(Self(value))
    }

    /// Returns the stat value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns the minimum stat.
    #[must_use]
    pub const fn min() -> Self {
        Self(Self::MIN_STAT)
    }

    /// Returns the maximum stat.
    #[must_use]
    pub const fn max() -> Self {
        Self(Self::MAX_STAT)
    }
}

impl fmt::Display for Stat {
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
    // Health Tests
    // =========================================================================

    mod health {
        use super::*;

        #[rstest]
        fn new_valid_value() {
            let health = Health::new(100).unwrap();
            assert_eq!(health.value(), 100);
        }

        #[rstest]
        fn new_zero() {
            let health = Health::new(0).unwrap();
            assert_eq!(health.value(), 0);
        }

        #[rstest]
        fn new_max_value() {
            let health = Health::new(Health::MAX_HEALTH).unwrap();
            assert_eq!(health.value(), Health::MAX_HEALTH);
        }

        #[rstest]
        fn new_exceeds_max() {
            let result = Health::new(Health::MAX_HEALTH + 1);
            assert!(result.is_err());
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Health::zero().value(), 0);
        }

        #[rstest]
        fn max_returns_max() {
            assert_eq!(Health::max().value(), Health::MAX_HEALTH);
        }

        #[rstest]
        fn saturating_add_normal() {
            let health = Health::new(100).unwrap();
            let healed = health.saturating_add(50);
            assert_eq!(healed.value(), 150);
        }

        #[rstest]
        fn saturating_add_saturates() {
            let health = Health::new(9000).unwrap();
            let healed = health.saturating_add(2000);
            assert_eq!(healed.value(), Health::MAX_HEALTH);
        }

        #[rstest]
        fn saturating_sub_normal() {
            let health = Health::new(100).unwrap();
            let damaged = health.saturating_sub(30);
            assert_eq!(damaged.value(), 70);
        }

        #[rstest]
        fn saturating_sub_saturates() {
            let health = Health::new(50).unwrap();
            let damaged = health.saturating_sub(100);
            assert_eq!(damaged.value(), 0);
        }

        #[rstest]
        fn is_zero_when_zero() {
            assert!(Health::zero().is_zero());
        }

        #[rstest]
        fn is_zero_when_not_zero() {
            let health = Health::new(1).unwrap();
            assert!(!health.is_zero());
        }

        #[rstest]
        fn display_format() {
            let health = Health::new(100).unwrap();
            assert_eq!(format!("{}", health), "100");
        }

        #[rstest]
        fn ordering() {
            let low = Health::new(10).unwrap();
            let high = Health::new(100).unwrap();
            assert!(low < high);
        }
    }

    // =========================================================================
    // Mana Tests
    // =========================================================================

    mod mana {
        use super::*;

        #[rstest]
        fn new_valid_value() {
            let mana = Mana::new(100).unwrap();
            assert_eq!(mana.value(), 100);
        }

        #[rstest]
        fn new_zero() {
            let mana = Mana::new(0).unwrap();
            assert_eq!(mana.value(), 0);
        }

        #[rstest]
        fn new_max_value() {
            let mana = Mana::new(Mana::MAX_MANA).unwrap();
            assert_eq!(mana.value(), Mana::MAX_MANA);
        }

        #[rstest]
        fn new_exceeds_max() {
            let result = Mana::new(Mana::MAX_MANA + 1);
            assert!(result.is_err());
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Mana::zero().value(), 0);
        }

        #[rstest]
        fn max_returns_max() {
            assert_eq!(Mana::max().value(), Mana::MAX_MANA);
        }

        #[rstest]
        fn saturating_add_normal() {
            let mana = Mana::new(100).unwrap();
            let restored = mana.saturating_add(50);
            assert_eq!(restored.value(), 150);
        }

        #[rstest]
        fn saturating_add_saturates() {
            let mana = Mana::new(900).unwrap();
            let restored = mana.saturating_add(200);
            assert_eq!(restored.value(), Mana::MAX_MANA);
        }

        #[rstest]
        fn saturating_sub_normal() {
            let mana = Mana::new(100).unwrap();
            let used = mana.saturating_sub(30);
            assert_eq!(used.value(), 70);
        }

        #[rstest]
        fn saturating_sub_saturates() {
            let mana = Mana::new(50).unwrap();
            let used = mana.saturating_sub(100);
            assert_eq!(used.value(), 0);
        }

        #[rstest]
        fn is_zero_when_zero() {
            assert!(Mana::zero().is_zero());
        }

        #[rstest]
        fn is_zero_when_not_zero() {
            let mana = Mana::new(1).unwrap();
            assert!(!mana.is_zero());
        }

        #[rstest]
        fn display_format() {
            let mana = Mana::new(100).unwrap();
            assert_eq!(format!("{}", mana), "100");
        }
    }

    // =========================================================================
    // Experience Tests
    // =========================================================================

    mod experience {
        use super::*;

        #[rstest]
        fn new_creates_experience() {
            let experience = Experience::new(1000);
            assert_eq!(experience.value(), 1000);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Experience::zero().value(), 0);
        }

        #[rstest]
        fn add_method() {
            let experience = Experience::new(100);
            let gained = Experience::add(&experience, 50);
            assert_eq!(gained.value(), 150);
        }

        #[rstest]
        fn add_operator() {
            let experience1 = Experience::new(100);
            let experience2 = Experience::new(50);
            assert_eq!((experience1 + experience2).value(), 150);
        }

        #[rstest]
        fn display_format() {
            let experience = Experience::new(12345);
            assert_eq!(format!("{}", experience), "12345");
        }

        #[rstest]
        fn ordering() {
            let low = Experience::new(100);
            let high = Experience::new(1000);
            assert!(low < high);
        }
    }

    // =========================================================================
    // Level Tests
    // =========================================================================

    mod level {
        use super::*;

        #[rstest]
        fn new_valid_level() {
            let level = Level::new(1).unwrap();
            assert_eq!(level.value(), 1);
        }

        #[rstest]
        fn new_max_level() {
            let level = Level::new(99).unwrap();
            assert_eq!(level.value(), 99);
        }

        #[rstest]
        fn new_zero_fails() {
            assert!(Level::new(0).is_err());
        }

        #[rstest]
        fn new_exceeds_max_fails() {
            assert!(Level::new(100).is_err());
        }

        #[rstest]
        fn one_returns_level_one() {
            assert_eq!(Level::one().value(), 1);
        }

        #[rstest]
        fn max_returns_max_level() {
            assert_eq!(Level::max().value(), 99);
        }

        #[rstest]
        fn level_up_normal() {
            let level = Level::new(50).unwrap();
            let next = level.level_up().unwrap();
            assert_eq!(next.value(), 51);
        }

        #[rstest]
        fn level_up_at_max() {
            let level = Level::max();
            assert!(level.level_up().is_none());
        }

        #[rstest]
        fn display_format() {
            let level = Level::new(42).unwrap();
            assert_eq!(format!("{}", level), "Lv.42");
        }

        #[rstest]
        fn ordering() {
            let low = Level::new(10).unwrap();
            let high = Level::new(50).unwrap();
            assert!(low < high);
        }
    }

    // =========================================================================
    // Attack Tests
    // =========================================================================

    mod attack {
        use super::*;

        #[rstest]
        fn new_creates_attack() {
            let attack = Attack::new(50);
            assert_eq!(attack.value(), 50);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Attack::zero().value(), 0);
        }

        #[rstest]
        fn add_operator() {
            let attack1 = Attack::new(30);
            let attack2 = Attack::new(20);
            assert_eq!((attack1 + attack2).value(), 50);
        }

        #[rstest]
        fn display_format() {
            let attack = Attack::new(100);
            assert_eq!(format!("{}", attack), "100");
        }
    }

    // =========================================================================
    // Defense Tests
    // =========================================================================

    mod defense {
        use super::*;

        #[rstest]
        fn new_creates_defense() {
            let defense = Defense::new(30);
            assert_eq!(defense.value(), 30);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Defense::zero().value(), 0);
        }

        #[rstest]
        fn add_operator() {
            let defense1 = Defense::new(20);
            let defense2 = Defense::new(15);
            assert_eq!((defense1 + defense2).value(), 35);
        }

        #[rstest]
        fn display_format() {
            let defense = Defense::new(50);
            assert_eq!(format!("{}", defense), "50");
        }
    }

    // =========================================================================
    // Speed Tests
    // =========================================================================

    mod speed {
        use super::*;

        #[rstest]
        fn new_creates_speed() {
            let speed = Speed::new(25);
            assert_eq!(speed.value(), 25);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Speed::zero().value(), 0);
        }

        #[rstest]
        fn add_operator() {
            let speed1 = Speed::new(10);
            let speed2 = Speed::new(5);
            assert_eq!((speed1 + speed2).value(), 15);
        }

        #[rstest]
        fn display_format() {
            let speed = Speed::new(30);
            assert_eq!(format!("{}", speed), "30");
        }
    }

    // =========================================================================
    // Damage Tests
    // =========================================================================

    mod damage {
        use super::*;

        #[rstest]
        fn new_creates_damage() {
            let damage = Damage::new(100);
            assert_eq!(damage.value(), 100);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(Damage::zero().value(), 0);
        }

        #[rstest]
        fn add_operator() {
            let damage1 = Damage::new(50);
            let damage2 = Damage::new(25);
            assert_eq!((damage1 + damage2).value(), 75);
        }

        #[rstest]
        fn display_format() {
            let damage = Damage::new(150);
            assert_eq!(format!("{}", damage), "150");
        }
    }

    // =========================================================================
    // TurnCount Tests
    // =========================================================================

    mod turn_count {
        use super::*;

        #[rstest]
        fn new_creates_turn_count() {
            let turn = TurnCount::new(10);
            assert_eq!(turn.value(), 10);
        }

        #[rstest]
        fn zero_returns_zero() {
            assert_eq!(TurnCount::zero().value(), 0);
        }

        #[rstest]
        fn next_increments() {
            let turn = TurnCount::new(10);
            assert_eq!(turn.next().value(), 11);
        }

        #[rstest]
        fn display_format() {
            let turn = TurnCount::new(42);
            assert_eq!(format!("{}", turn), "Turn 42");
        }

        #[rstest]
        fn ordering() {
            let early = TurnCount::new(10);
            let late = TurnCount::new(100);
            assert!(early < late);
        }
    }

    // =========================================================================
    // FloorLevel Tests
    // =========================================================================

    mod floor_level {
        use super::*;

        #[rstest]
        fn new_valid_floor() {
            let floor = FloorLevel::new(1).unwrap();
            assert_eq!(floor.value(), 1);
        }

        #[rstest]
        fn new_zero_fails() {
            assert!(FloorLevel::new(0).is_err());
        }

        #[rstest]
        fn first_returns_floor_one() {
            assert_eq!(FloorLevel::first().value(), 1);
        }

        #[rstest]
        fn descend_increments() {
            let floor = FloorLevel::new(5).unwrap();
            assert_eq!(floor.descend().value(), 6);
        }

        #[rstest]
        fn ascend_decrements() {
            let floor = FloorLevel::new(5).unwrap();
            assert_eq!(floor.ascend().unwrap().value(), 4);
        }

        #[rstest]
        fn ascend_from_first_returns_none() {
            let floor = FloorLevel::first();
            assert!(floor.ascend().is_none());
        }

        #[rstest]
        fn display_format() {
            let floor = FloorLevel::new(3).unwrap();
            assert_eq!(format!("{}", floor), "B3F");
        }

        #[rstest]
        fn ordering() {
            let shallow = FloorLevel::new(1).unwrap();
            let deep = FloorLevel::new(10).unwrap();
            assert!(shallow < deep);
        }
    }

    // =========================================================================
    // Stat Tests
    // =========================================================================

    mod stat {
        use super::*;

        #[rstest]
        fn new_valid_stat() {
            let stat = Stat::new(10).unwrap();
            assert_eq!(stat.value(), 10);
        }

        #[rstest]
        fn new_min_value() {
            let stat = Stat::new(Stat::MIN_STAT).unwrap();
            assert_eq!(stat.value(), Stat::MIN_STAT);
        }

        #[rstest]
        fn new_max_value() {
            let stat = Stat::new(Stat::MAX_STAT).unwrap();
            assert_eq!(stat.value(), Stat::MAX_STAT);
        }

        #[rstest]
        fn new_zero_fails() {
            assert!(Stat::new(0).is_err());
        }

        #[rstest]
        fn new_exceeds_max_fails() {
            assert!(Stat::new(100).is_err());
        }

        #[rstest]
        fn min_returns_min_stat() {
            assert_eq!(Stat::min().value(), Stat::MIN_STAT);
        }

        #[rstest]
        fn max_returns_max_stat() {
            assert_eq!(Stat::max().value(), Stat::MAX_STAT);
        }

        #[rstest]
        fn display_format() {
            let stat = Stat::new(50).unwrap();
            assert_eq!(format!("{}", stat), "50");
        }

        #[rstest]
        fn ordering() {
            let low = Stat::new(10).unwrap();
            let high = Stat::new(50).unwrap();
            assert!(low < high);
        }
    }
}
