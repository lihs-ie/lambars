//! GameSession aggregate root.
//!
//! This module provides the `GameSession` aggregate which represents a single
//! playthrough of the game from start to finish. It maintains consistency
//! across all game-related state including player, floor, enemies, and
//! session metadata.
//!
//! All operations are implemented as pure functions that return new GameSession
//! instances, maintaining immutability throughout.

use crate::common::TurnCount;
use crate::enemy::{Enemy, EntityIdentifier};
use crate::floor::Floor;
use crate::player::Player;

use super::errors::GameSessionError;
use super::events::RandomSeed;
use super::identifier::GameIdentifier;
use super::status::{GameOutcome, GameStatus};

// =============================================================================
// GameSession
// =============================================================================

/// The GameSession aggregate root.
///
/// `GameSession` represents a complete game session, encapsulating all
/// game-related state including:
///
/// - Session identity and metadata
/// - Player state
/// - Current floor state
/// - Enemy states
/// - Turn tracking
/// - Game status (in progress, paused, victory, defeat)
///
/// # Invariants
///
/// - `turn_count` is always >= 0
/// - `event_sequence` is monotonically increasing
/// - Once status is terminal (Victory/Defeat), it cannot change
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::{
///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
///     Mana, Position, Speed, Stat, TurnCount,
/// };
/// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
/// use roguelike_domain::floor::{Floor, FloorIdentifier};
/// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
///
/// // Create player
/// let player = Player::new(
///     PlayerIdentifier::new(),
///     PlayerName::new("Hero").unwrap(),
///     Position::new(5, 5),
///     CombatStats::new(
///         Health::new(100).unwrap(),
///         Health::new(100).unwrap(),
///         Mana::new(50).unwrap(),
///         Mana::new(50).unwrap(),
///         Attack::new(20),
///         Defense::new(15),
///         Speed::new(10),
///     ).unwrap(),
///     BaseStats::new(
///         Stat::new(10).unwrap(),
///         Stat::new(10).unwrap(),
///         Stat::new(10).unwrap(),
///         Stat::new(10).unwrap(),
///     ),
/// );
///
/// // Create floor
/// let floor = Floor::new(
///     FloorIdentifier::new(1),
///     FloorLevel::new(1).unwrap(),
///     80,
///     40,
/// );
///
/// // Create game session
/// let session = GameSession::new(
///     GameIdentifier::new(),
///     player,
///     floor,
///     RandomSeed::new(12345),
/// );
///
/// assert!(session.is_active());
/// assert!(!session.is_terminal());
/// assert_eq!(session.turn_count().value(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct GameSession {
    identifier: GameIdentifier,
    player: Player,
    current_floor: Floor,
    enemies: Vec<Enemy>,
    turn_count: TurnCount,
    status: GameStatus,
    seed: RandomSeed,
    event_sequence: u64,
}

impl GameSession {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Creates a new `GameSession` with the given parameters.
    ///
    /// The session starts with:
    /// - Turn count at 0
    /// - Status as InProgress
    /// - No enemies
    /// - Event sequence at 0
    ///
    /// # Arguments
    ///
    /// * `identifier` - Unique game session identifier
    /// * `player` - Initial player state
    /// * `current_floor` - Initial floor state
    /// * `seed` - Random seed for reproducibility
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert!(session.is_active());
    /// ```
    #[must_use]
    pub fn new(
        identifier: GameIdentifier,
        player: Player,
        current_floor: Floor,
        seed: RandomSeed,
    ) -> Self {
        Self {
            identifier,
            player,
            current_floor,
            enemies: Vec::new(),
            turn_count: TurnCount::zero(),
            status: GameStatus::InProgress,
            seed,
            event_sequence: 0,
        }
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Returns a reference to the game session identifier.
    #[must_use]
    pub const fn identifier(&self) -> &GameIdentifier {
        &self.identifier
    }

    /// Returns a reference to the player.
    #[must_use]
    pub const fn player(&self) -> &Player {
        &self.player
    }

    /// Returns a reference to the current floor.
    #[must_use]
    pub const fn current_floor(&self) -> &Floor {
        &self.current_floor
    }

    /// Returns a slice of enemies on the current floor.
    #[must_use]
    pub fn enemies(&self) -> &[Enemy] {
        &self.enemies
    }

    /// Returns the current turn count.
    #[must_use]
    pub const fn turn_count(&self) -> TurnCount {
        self.turn_count
    }

    /// Returns a reference to the current game status.
    #[must_use]
    pub const fn status(&self) -> &GameStatus {
        &self.status
    }

    /// Returns a reference to the random seed.
    #[must_use]
    pub const fn seed(&self) -> &RandomSeed {
        &self.seed
    }

    /// Returns the current event sequence number.
    #[must_use]
    pub const fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Returns true if the game session is in an active state.
    ///
    /// A session is active if it is either InProgress or Paused.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert!(session.is_active());
    /// ```
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns true if the game session has reached a terminal state.
    ///
    /// A session is terminal if it ended in Victory or Defeat.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, GameOutcome, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert!(!session.is_terminal());
    ///
    /// let ended_session = session.end_game(GameOutcome::Victory);
    /// assert!(ended_session.is_terminal());
    /// ```
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    // =========================================================================
    // Domain Methods (Pure Functions)
    // =========================================================================

    /// Returns a new GameSession with the updated player state.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `player` - The new player state
    #[must_use]
    pub fn with_player(self, player: Player) -> Self {
        Self { player, ..self }
    }

    /// Returns a new GameSession with the updated floor state.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `floor` - The new floor state
    #[must_use]
    pub fn with_floor(self, floor: Floor) -> Self {
        Self {
            current_floor: floor,
            ..self
        }
    }

    /// Returns a new GameSession with the updated enemies list.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `enemies` - The new enemies list
    #[must_use]
    pub fn with_enemies(self, enemies: Vec<Enemy>) -> Self {
        Self { enemies, ..self }
    }

    /// Returns a new GameSession with an additional enemy.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `enemy` - The enemy to add
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(10, 10),
    ///     CombatStats::new(
    ///         Health::new(50).unwrap(),
    ///         Health::new(50).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(8),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert_eq!(session.enemies().len(), 0);
    ///
    /// let session_with_enemy = session.add_enemy(enemy);
    /// assert_eq!(session_with_enemy.enemies().len(), 1);
    /// ```
    #[must_use]
    pub fn add_enemy(self, enemy: Enemy) -> Self {
        let mut new_enemies = self.enemies;
        new_enemies.push(enemy);
        Self {
            enemies: new_enemies,
            ..self
        }
    }

    /// Returns a new GameSession with the specified enemy removed.
    ///
    /// If no enemy with the given identifier exists, returns the session unchanged.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The identifier of the enemy to remove
    #[must_use]
    pub fn remove_enemy(self, enemy_identifier: &EntityIdentifier) -> Self {
        let new_enemies = self
            .enemies
            .into_iter()
            .filter(|enemy| enemy.identifier() != enemy_identifier)
            .collect();

        Self {
            enemies: new_enemies,
            ..self
        }
    }

    /// Returns a new GameSession with the specified enemy updated.
    ///
    /// Applies the provided function to the enemy with the given identifier.
    /// If no enemy with the given identifier exists, returns the session unchanged.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The identifier of the enemy to update
    /// * `update_function` - Function to apply to the enemy
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Damage, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let enemy_identifier = EntityIdentifier::new();
    /// let enemy = Enemy::new(
    ///     enemy_identifier,
    ///     EnemyType::Goblin,
    ///     Position::new(10, 10),
    ///     CombatStats::new(
    ///         Health::new(50).unwrap(),
    ///         Health::new(50).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(8),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// ).add_enemy(enemy);
    ///
    /// // Apply damage to the enemy
    /// let updated_session = session.update_enemy(&enemy_identifier, |e| {
    ///     e.take_damage(Damage::new(20))
    /// });
    ///
    /// assert_eq!(updated_session.enemies()[0].health().value(), 30);
    /// ```
    #[must_use]
    pub fn update_enemy<F>(self, enemy_identifier: &EntityIdentifier, update_function: F) -> Self
    where
        F: FnOnce(Enemy) -> Enemy,
    {
        // Find the target enemy and apply the update function
        let target_index = self
            .enemies
            .iter()
            .position(|enemy| enemy.identifier() == enemy_identifier);

        match target_index {
            Some(index) => {
                let mut new_enemies = self.enemies;
                let target_enemy = new_enemies.remove(index);
                let updated_enemy = update_function(target_enemy);
                new_enemies.insert(index, updated_enemy);

                Self {
                    enemies: new_enemies,
                    ..self
                }
            }
            None => self,
        }
    }

    /// Returns a new GameSession with the turn count incremented.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert_eq!(session.turn_count().value(), 0);
    ///
    /// let next_turn = session.increment_turn();
    /// assert_eq!(next_turn.turn_count().value(), 1);
    ///
    /// let another_turn = next_turn.increment_turn();
    /// assert_eq!(another_turn.turn_count().value(), 2);
    /// ```
    #[must_use]
    pub fn increment_turn(self) -> Self {
        Self {
            turn_count: self.turn_count.next(),
            ..self
        }
    }

    /// Ends the game with the specified outcome.
    ///
    /// This transitions the game status to a terminal state based on the outcome.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Arguments
    ///
    /// * `outcome` - The outcome of the game (Victory, Defeat, or Abandoned)
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, GameOutcome, GameStatus, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// let victory_session = session.end_game(GameOutcome::Victory);
    /// assert_eq!(victory_session.status(), &GameStatus::Victory);
    /// assert!(victory_session.is_terminal());
    /// ```
    #[must_use]
    pub fn end_game(self, outcome: GameOutcome) -> Self {
        Self {
            status: outcome.to_status(),
            ..self
        }
    }

    /// Pauses the game session.
    ///
    /// This transitions the game status to Paused.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, GameStatus, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// let paused_session = session.pause();
    /// assert_eq!(paused_session.status(), &GameStatus::Paused);
    /// assert!(paused_session.is_active());
    /// ```
    #[must_use]
    pub fn pause(self) -> Self {
        Self {
            status: GameStatus::Paused,
            ..self
        }
    }

    /// Resumes a paused game session.
    ///
    /// This transitions the game status from Paused back to InProgress.
    ///
    /// # Errors
    ///
    /// Returns `GameSessionError::SessionAlreadyCompleted` if the game is in a
    /// terminal state or if the session is not paused.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, GameStatus, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// let paused_session = session.pause();
    /// let resumed_session = paused_session.resume().unwrap();
    /// assert_eq!(resumed_session.status(), &GameStatus::InProgress);
    /// ```
    pub fn resume(self) -> Result<Self, GameSessionError> {
        if !self.status.is_paused() {
            return Err(GameSessionError::session_already_completed());
        }

        Ok(Self {
            status: GameStatus::InProgress,
            ..self
        })
    }

    /// Returns a new GameSession with the event sequence incremented.
    ///
    /// This is used for event ordering and optimistic concurrency.
    ///
    /// This is an immutable operation that consumes self and returns a new GameSession.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::{
    ///     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
    ///     Mana, Position, Speed, Stat,
    /// };
    /// use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
    /// use roguelike_domain::floor::{Floor, FloorIdentifier};
    /// use roguelike_domain::game_session::{GameSession, GameIdentifier, RandomSeed};
    ///
    /// let player = Player::new(
    ///     PlayerIdentifier::new(),
    ///     PlayerName::new("Hero").unwrap(),
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Mana::new(50).unwrap(),
    ///         Attack::new(20),
    ///         Defense::new(15),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     BaseStats::new(
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///         Stat::new(10).unwrap(),
    ///     ),
    /// );
    ///
    /// let floor = Floor::new(
    ///     FloorIdentifier::new(1),
    ///     FloorLevel::new(1).unwrap(),
    ///     80,
    ///     40,
    /// );
    ///
    /// let session = GameSession::new(
    ///     GameIdentifier::new(),
    ///     player,
    ///     floor,
    ///     RandomSeed::new(42),
    /// );
    ///
    /// assert_eq!(session.event_sequence(), 0);
    ///
    /// let next = session.increment_event_sequence();
    /// assert_eq!(next.event_sequence(), 1);
    /// ```
    #[must_use]
    pub fn increment_event_sequence(self) -> Self {
        Self {
            event_sequence: self.event_sequence.saturating_add(1),
            ..self
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        Attack, BaseStats, CombatStats, Damage, Defense, FloorLevel, Health, Mana, Position, Speed,
        Stat,
    };
    use crate::enemy::{AiBehavior, EnemyType, LootTable};
    use crate::floor::FloorIdentifier;
    use crate::player::{PlayerIdentifier, PlayerName};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    #[fixture]
    fn default_combat_stats() -> CombatStats {
        CombatStats::new(
            Health::new(100).unwrap(),
            Health::new(100).unwrap(),
            Mana::new(50).unwrap(),
            Mana::new(50).unwrap(),
            Attack::new(20),
            Defense::new(15),
            Speed::new(10),
        )
        .unwrap()
    }

    #[fixture]
    fn default_base_stats() -> BaseStats {
        BaseStats::new(
            Stat::new(10).unwrap(),
            Stat::new(10).unwrap(),
            Stat::new(10).unwrap(),
            Stat::new(10).unwrap(),
        )
    }

    #[fixture]
    fn default_player(default_combat_stats: CombatStats, default_base_stats: BaseStats) -> Player {
        Player::new(
            PlayerIdentifier::new(),
            PlayerName::new("TestHero").unwrap(),
            Position::new(5, 5),
            default_combat_stats,
            default_base_stats,
        )
    }

    #[fixture]
    fn default_floor() -> Floor {
        Floor::new(FloorIdentifier::new(1), FloorLevel::new(1).unwrap(), 80, 40)
    }

    #[fixture]
    fn default_session(default_player: Player, default_floor: Floor) -> GameSession {
        GameSession::new(
            GameIdentifier::new(),
            default_player,
            default_floor,
            RandomSeed::new(12345),
        )
    }

    #[fixture]
    fn default_enemy() -> Enemy {
        Enemy::new(
            EntityIdentifier::new(),
            EnemyType::Goblin,
            Position::new(10, 10),
            CombatStats::new(
                Health::new(50).unwrap(),
                Health::new(50).unwrap(),
                Mana::zero(),
                Mana::zero(),
                Attack::new(10),
                Defense::new(5),
                Speed::new(8),
            )
            .unwrap(),
            AiBehavior::Aggressive,
            LootTable::empty(),
        )
    }

    // =========================================================================
    // Constructor Tests
    // =========================================================================

    mod constructor {
        use super::*;

        #[rstest]
        fn new_creates_session_with_correct_values(default_player: Player, default_floor: Floor) {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);

            let session = GameSession::new(
                identifier,
                default_player.clone(),
                default_floor.clone(),
                seed,
            );

            assert_eq!(*session.identifier(), identifier);
            assert_eq!(session.seed(), &seed);
            assert_eq!(session.turn_count().value(), 0);
            assert_eq!(session.event_sequence(), 0);
            assert!(session.enemies().is_empty());
            assert_eq!(session.status(), &GameStatus::InProgress);
        }

        #[rstest]
        fn new_starts_in_progress(default_session: GameSession) {
            assert!(default_session.is_active());
            assert!(!default_session.is_terminal());
            assert_eq!(default_session.status(), &GameStatus::InProgress);
        }

        #[rstest]
        fn new_starts_at_turn_zero(default_session: GameSession) {
            assert_eq!(default_session.turn_count().value(), 0);
        }

        #[rstest]
        fn new_starts_with_no_enemies(default_session: GameSession) {
            assert!(default_session.enemies().is_empty());
        }

        #[rstest]
        fn new_starts_with_event_sequence_zero(default_session: GameSession) {
            assert_eq!(default_session.event_sequence(), 0);
        }
    }

    // =========================================================================
    // Getter Tests
    // =========================================================================

    mod getters {
        use super::*;

        #[rstest]
        fn identifier_returns_correct_value(default_player: Player, default_floor: Floor) {
            let identifier = GameIdentifier::new();
            let session = GameSession::new(
                identifier,
                default_player,
                default_floor,
                RandomSeed::new(42),
            );
            assert_eq!(*session.identifier(), identifier);
        }

        #[rstest]
        fn player_returns_reference(default_session: GameSession) {
            let player = default_session.player();
            assert_eq!(player.name().value(), "TestHero");
        }

        #[rstest]
        fn current_floor_returns_reference(default_session: GameSession) {
            let floor = default_session.current_floor();
            assert_eq!(floor.width(), 80);
            assert_eq!(floor.height(), 40);
        }

        #[rstest]
        fn seed_returns_correct_value(default_player: Player, default_floor: Floor) {
            let seed = RandomSeed::new(99999);
            let session =
                GameSession::new(GameIdentifier::new(), default_player, default_floor, seed);
            assert_eq!(session.seed(), &seed);
        }
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_active_when_in_progress(default_session: GameSession) {
            assert!(default_session.is_active());
        }

        #[rstest]
        fn is_active_when_paused(default_session: GameSession) {
            let paused = default_session.pause();
            assert!(paused.is_active());
        }

        #[rstest]
        fn is_active_when_victory(default_session: GameSession) {
            let victory = default_session.end_game(GameOutcome::Victory);
            assert!(!victory.is_active());
        }

        #[rstest]
        fn is_active_when_defeat(default_session: GameSession) {
            let defeat = default_session.end_game(GameOutcome::Defeat);
            assert!(!defeat.is_active());
        }

        #[rstest]
        fn is_terminal_when_in_progress(default_session: GameSession) {
            assert!(!default_session.is_terminal());
        }

        #[rstest]
        fn is_terminal_when_paused(default_session: GameSession) {
            let paused = default_session.pause();
            assert!(!paused.is_terminal());
        }

        #[rstest]
        fn is_terminal_when_victory(default_session: GameSession) {
            let victory = default_session.end_game(GameOutcome::Victory);
            assert!(victory.is_terminal());
        }

        #[rstest]
        fn is_terminal_when_defeat(default_session: GameSession) {
            let defeat = default_session.end_game(GameOutcome::Defeat);
            assert!(defeat.is_terminal());
        }
    }

    // =========================================================================
    // Player Update Tests
    // =========================================================================

    mod player_update {
        use super::*;

        #[rstest]
        fn with_player_updates_player(
            default_session: GameSession,
            default_combat_stats: CombatStats,
            default_base_stats: BaseStats,
        ) {
            let new_player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("NewHero").unwrap(),
                Position::new(20, 20),
                default_combat_stats,
                default_base_stats,
            );

            let updated = default_session.with_player(new_player);
            assert_eq!(updated.player().name().value(), "NewHero");
            assert_eq!(*updated.player().position(), Position::new(20, 20));
        }

        #[rstest]
        fn with_player_preserves_other_fields(
            default_session: GameSession,
            default_combat_stats: CombatStats,
            default_base_stats: BaseStats,
        ) {
            let original_identifier = *default_session.identifier();
            let original_seed = *default_session.seed();

            let new_player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("NewHero").unwrap(),
                Position::new(20, 20),
                default_combat_stats,
                default_base_stats,
            );

            let updated = default_session.with_player(new_player);
            assert_eq!(*updated.identifier(), original_identifier);
            assert_eq!(*updated.seed(), original_seed);
        }
    }

    // =========================================================================
    // Floor Update Tests
    // =========================================================================

    mod floor_update {
        use super::*;

        #[rstest]
        fn with_floor_updates_floor(default_session: GameSession) {
            let new_floor = Floor::new(
                FloorIdentifier::new(2),
                FloorLevel::new(2).unwrap(),
                100,
                50,
            );

            let updated = default_session.with_floor(new_floor);
            assert_eq!(updated.current_floor().width(), 100);
            assert_eq!(updated.current_floor().height(), 50);
        }

        #[rstest]
        fn with_floor_preserves_other_fields(default_session: GameSession) {
            let original_turn = default_session.turn_count();

            let new_floor = Floor::new(
                FloorIdentifier::new(2),
                FloorLevel::new(2).unwrap(),
                100,
                50,
            );

            let updated = default_session.with_floor(new_floor);
            assert_eq!(updated.turn_count(), original_turn);
        }
    }

    // =========================================================================
    // Enemy Management Tests
    // =========================================================================

    mod enemy_management {
        use super::*;

        #[rstest]
        fn with_enemies_sets_enemies(default_session: GameSession, default_enemy: Enemy) {
            let enemies = vec![default_enemy.clone()];
            let updated = default_session.with_enemies(enemies);
            assert_eq!(updated.enemies().len(), 1);
        }

        #[rstest]
        fn add_enemy_adds_to_list(default_session: GameSession, default_enemy: Enemy) {
            let updated = default_session.add_enemy(default_enemy);
            assert_eq!(updated.enemies().len(), 1);
        }

        #[rstest]
        fn add_enemy_multiple_times(default_session: GameSession) {
            let enemy1 = Enemy::new(
                EntityIdentifier::new(),
                EnemyType::Goblin,
                Position::new(10, 10),
                CombatStats::new(
                    Health::new(50).unwrap(),
                    Health::new(50).unwrap(),
                    Mana::zero(),
                    Mana::zero(),
                    Attack::new(10),
                    Defense::new(5),
                    Speed::new(8),
                )
                .unwrap(),
                AiBehavior::Aggressive,
                LootTable::empty(),
            );

            let enemy2 = Enemy::new(
                EntityIdentifier::new(),
                EnemyType::Skeleton,
                Position::new(20, 20),
                CombatStats::new(
                    Health::new(30).unwrap(),
                    Health::new(30).unwrap(),
                    Mana::zero(),
                    Mana::zero(),
                    Attack::new(8),
                    Defense::new(2),
                    Speed::new(6),
                )
                .unwrap(),
                AiBehavior::Patrol,
                LootTable::empty(),
            );

            let updated = default_session.add_enemy(enemy1).add_enemy(enemy2);
            assert_eq!(updated.enemies().len(), 2);
        }

        #[rstest]
        fn remove_enemy_removes_by_identifier(default_session: GameSession) {
            let enemy_identifier = EntityIdentifier::new();
            let enemy = Enemy::new(
                enemy_identifier,
                EnemyType::Goblin,
                Position::new(10, 10),
                CombatStats::new(
                    Health::new(50).unwrap(),
                    Health::new(50).unwrap(),
                    Mana::zero(),
                    Mana::zero(),
                    Attack::new(10),
                    Defense::new(5),
                    Speed::new(8),
                )
                .unwrap(),
                AiBehavior::Aggressive,
                LootTable::empty(),
            );

            let with_enemy = default_session.add_enemy(enemy);
            assert_eq!(with_enemy.enemies().len(), 1);

            let without_enemy = with_enemy.remove_enemy(&enemy_identifier);
            assert_eq!(without_enemy.enemies().len(), 0);
        }

        #[rstest]
        fn remove_enemy_nonexistent_does_nothing(default_session: GameSession) {
            let nonexistent_identifier = EntityIdentifier::new();
            let updated = default_session.remove_enemy(&nonexistent_identifier);
            assert_eq!(updated.enemies().len(), 0);
        }

        #[rstest]
        fn update_enemy_applies_function(default_session: GameSession) {
            let enemy_identifier = EntityIdentifier::new();
            let enemy = Enemy::new(
                enemy_identifier,
                EnemyType::Goblin,
                Position::new(10, 10),
                CombatStats::new(
                    Health::new(50).unwrap(),
                    Health::new(50).unwrap(),
                    Mana::zero(),
                    Mana::zero(),
                    Attack::new(10),
                    Defense::new(5),
                    Speed::new(8),
                )
                .unwrap(),
                AiBehavior::Aggressive,
                LootTable::empty(),
            );

            let with_enemy = default_session.add_enemy(enemy);

            let updated =
                with_enemy.update_enemy(&enemy_identifier, |e| e.take_damage(Damage::new(20)));

            assert_eq!(updated.enemies()[0].health().value(), 30);
        }

        #[rstest]
        fn update_enemy_nonexistent_does_nothing(
            default_session: GameSession,
            default_enemy: Enemy,
        ) {
            let nonexistent_identifier = EntityIdentifier::new();
            let with_enemy = default_session.add_enemy(default_enemy);

            let updated = with_enemy
                .update_enemy(&nonexistent_identifier, |e| e.take_damage(Damage::new(100)));

            // Original enemy should be unchanged
            assert_eq!(updated.enemies()[0].health().value(), 50);
        }
    }

    // =========================================================================
    // Turn Management Tests
    // =========================================================================

    mod turn_management {
        use super::*;

        #[rstest]
        fn increment_turn_increases_count(default_session: GameSession) {
            let updated = default_session.increment_turn();
            assert_eq!(updated.turn_count().value(), 1);
        }

        #[rstest]
        fn increment_turn_multiple_times(default_session: GameSession) {
            let updated = default_session
                .increment_turn()
                .increment_turn()
                .increment_turn();
            assert_eq!(updated.turn_count().value(), 3);
        }

        #[rstest]
        fn increment_turn_preserves_other_fields(default_session: GameSession) {
            let original_enemies_count = default_session.enemies().len();
            let updated = default_session.increment_turn();
            assert_eq!(updated.enemies().len(), original_enemies_count);
        }
    }

    // =========================================================================
    // Game Ending Tests
    // =========================================================================

    mod game_ending {
        use super::*;

        #[rstest]
        fn end_game_victory_sets_status(default_session: GameSession) {
            let ended = default_session.end_game(GameOutcome::Victory);
            assert_eq!(ended.status(), &GameStatus::Victory);
        }

        #[rstest]
        fn end_game_defeat_sets_status(default_session: GameSession) {
            let ended = default_session.end_game(GameOutcome::Defeat);
            assert_eq!(ended.status(), &GameStatus::Defeat);
        }

        #[rstest]
        fn end_game_abandoned_sets_defeat_status(default_session: GameSession) {
            let ended = default_session.end_game(GameOutcome::Abandoned);
            assert_eq!(ended.status(), &GameStatus::Defeat);
        }

        #[rstest]
        fn end_game_makes_terminal(default_session: GameSession) {
            let ended = default_session.end_game(GameOutcome::Victory);
            assert!(ended.is_terminal());
            assert!(!ended.is_active());
        }
    }

    // =========================================================================
    // Pause/Resume Tests
    // =========================================================================

    mod pause_resume {
        use super::*;

        #[rstest]
        fn pause_sets_paused_status(default_session: GameSession) {
            let paused = default_session.pause();
            assert_eq!(paused.status(), &GameStatus::Paused);
        }

        #[rstest]
        fn pause_keeps_active(default_session: GameSession) {
            let paused = default_session.pause();
            assert!(paused.is_active());
        }

        #[rstest]
        fn resume_sets_in_progress_status(default_session: GameSession) {
            let paused = default_session.pause();
            let resumed = paused.resume().unwrap();
            assert_eq!(resumed.status(), &GameStatus::InProgress);
        }

        #[rstest]
        fn resume_fails_when_not_paused(default_session: GameSession) {
            let result = default_session.resume();
            assert!(result.is_err());
        }

        #[rstest]
        fn resume_fails_when_terminal(default_session: GameSession) {
            let ended = default_session.end_game(GameOutcome::Victory);
            let result = ended.resume();
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Event Sequence Tests
    // =========================================================================

    mod event_sequence {
        use super::*;

        #[rstest]
        fn increment_event_sequence_increases(default_session: GameSession) {
            let updated = default_session.increment_event_sequence();
            assert_eq!(updated.event_sequence(), 1);
        }

        #[rstest]
        fn increment_event_sequence_multiple_times(default_session: GameSession) {
            let updated = default_session
                .increment_event_sequence()
                .increment_event_sequence()
                .increment_event_sequence();
            assert_eq!(updated.event_sequence(), 3);
        }

        #[rstest]
        fn increment_event_sequence_saturates() {
            // Create a session with max event sequence
            let session = GameSession {
                identifier: GameIdentifier::new(),
                player: Player::new(
                    PlayerIdentifier::new(),
                    PlayerName::new("Test").unwrap(),
                    Position::new(0, 0),
                    CombatStats::new(
                        Health::new(100).unwrap(),
                        Health::new(100).unwrap(),
                        Mana::new(50).unwrap(),
                        Mana::new(50).unwrap(),
                        Attack::new(20),
                        Defense::new(15),
                        Speed::new(10),
                    )
                    .unwrap(),
                    BaseStats::new(
                        Stat::new(10).unwrap(),
                        Stat::new(10).unwrap(),
                        Stat::new(10).unwrap(),
                        Stat::new(10).unwrap(),
                    ),
                ),
                current_floor: Floor::new(
                    FloorIdentifier::new(1),
                    FloorLevel::new(1).unwrap(),
                    10,
                    10,
                ),
                enemies: Vec::new(),
                turn_count: TurnCount::zero(),
                status: GameStatus::InProgress,
                seed: RandomSeed::new(42),
                event_sequence: u64::MAX,
            };

            let updated = session.increment_event_sequence();
            assert_eq!(updated.event_sequence(), u64::MAX);
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone {
        use super::*;

        #[rstest]
        fn clone_creates_independent_copy(default_session: GameSession) {
            let cloned = default_session.clone();

            assert_eq!(*cloned.identifier(), *default_session.identifier());
            assert_eq!(cloned.turn_count(), default_session.turn_count());
            assert_eq!(cloned.status(), default_session.status());
            assert_eq!(cloned.seed(), default_session.seed());
        }

        #[rstest]
        fn clone_with_enemies(default_session: GameSession, default_enemy: Enemy) {
            let with_enemy = default_session.add_enemy(default_enemy);
            let cloned = with_enemy.clone();

            assert_eq!(cloned.enemies().len(), 1);
        }
    }
}
