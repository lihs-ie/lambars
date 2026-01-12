use crate::common::{BaseStats, CombatStats, Damage, Experience, Level, Position, StatusEffect};
use crate::player::{EquipmentSlots, Inventory, PlayerError, PlayerIdentifier, PlayerName};

// =============================================================================
// Player
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    identifier: PlayerIdentifier,
    name: PlayerName,
    position: Position,
    stats: CombatStats,
    base_stats: BaseStats,
    level: Level,
    experience: Experience,
    equipment: EquipmentSlots,
    inventory: Inventory,
    status_effects: Vec<StatusEffect>,
}

impl Player {
    // =========================================================================
    // Constructor
    // =========================================================================

    #[must_use]
    pub fn new(
        identifier: PlayerIdentifier,
        name: PlayerName,
        position: Position,
        stats: CombatStats,
        base_stats: BaseStats,
    ) -> Self {
        Self {
            identifier,
            name,
            position,
            stats,
            base_stats,
            level: Level::one(),
            experience: Experience::zero(),
            equipment: EquipmentSlots::empty(),
            inventory: Inventory::default(),
            status_effects: Vec::new(),
        }
    }

    // =========================================================================
    // Getters
    // =========================================================================

    #[must_use]
    pub const fn identifier(&self) -> &PlayerIdentifier {
        &self.identifier
    }

    #[must_use]
    pub const fn name(&self) -> &PlayerName {
        &self.name
    }

    #[must_use]
    pub const fn position(&self) -> &Position {
        &self.position
    }

    #[must_use]
    pub const fn stats(&self) -> &CombatStats {
        &self.stats
    }

    #[must_use]
    pub const fn base_stats(&self) -> &BaseStats {
        &self.base_stats
    }

    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    #[must_use]
    pub const fn experience(&self) -> Experience {
        self.experience
    }

    #[must_use]
    pub const fn equipment(&self) -> &EquipmentSlots {
        &self.equipment
    }

    #[must_use]
    pub const fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    #[must_use]
    pub fn status_effects(&self) -> &[StatusEffect] {
        &self.status_effects
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.stats.is_alive()
    }

    #[must_use]
    pub fn can_level_up(&self) -> bool {
        if self.level.value() >= Level::MAX_LEVEL {
            return false;
        }
        let required_experience = self.experience_for_next_level();
        self.experience.value() >= required_experience
    }

    #[must_use]
    fn experience_for_next_level(&self) -> u64 {
        u64::from(self.level.value()) * 100
    }

    // =========================================================================
    // Domain Methods (Pure Functions)
    // =========================================================================

    #[must_use]
    pub fn move_to(self, new_position: Position) -> Self {
        Self {
            position: new_position,
            ..self
        }
    }

    #[must_use]
    pub fn take_damage(self, damage: Damage) -> Self {
        let new_health = self.stats.health().saturating_sub(damage.value());
        // Using unwrap here is safe because new_health <= max_health is guaranteed
        // by saturating_sub (it can only decrease, never increase)
        let new_stats = self.stats.with_health(new_health).unwrap();
        Self {
            stats: new_stats,
            ..self
        }
    }

    #[must_use]
    pub fn heal(self, amount: u32) -> Self {
        let new_health = self.stats.health().saturating_add(amount);
        // Cap at max_health
        let capped_health = if new_health.value() > self.stats.max_health().value() {
            self.stats.max_health()
        } else {
            new_health
        };
        // Using unwrap is safe because capped_health <= max_health is guaranteed
        let new_stats = self.stats.with_health(capped_health).unwrap();
        Self {
            stats: new_stats,
            ..self
        }
    }

    #[must_use]
    pub fn gain_experience(self, amount: Experience) -> Self {
        Self {
            experience: self.experience + amount,
            ..self
        }
    }

    pub fn level_up(self) -> Result<Self, PlayerError> {
        if !self.can_level_up() {
            return Err(PlayerError::LevelCapReached);
        }

        let new_level = self.level.level_up().ok_or(PlayerError::LevelCapReached)?;

        Ok(Self {
            level: new_level,
            ..self
        })
    }

    #[must_use]
    pub fn apply_status_effect(self, effect: StatusEffect) -> Self {
        let mut new_effects = self.status_effects;

        if effect.effect_type().can_stack() {
            // Stackable effects are simply added
            new_effects.push(effect);
        } else {
            // Non-stackable effects replace existing ones of the same type
            // if the new effect has a longer duration
            let existing_index = new_effects
                .iter()
                .position(|existing_effect| existing_effect.effect_type() == effect.effect_type());

            match existing_index {
                Some(index) => {
                    if effect.remaining_turns() > new_effects[index].remaining_turns() {
                        new_effects[index] = effect;
                    }
                }
                None => {
                    new_effects.push(effect);
                }
            }
        }

        Self {
            status_effects: new_effects,
            ..self
        }
    }

    #[must_use]
    pub fn tick_status_effects(self) -> Self {
        let new_effects: Vec<StatusEffect> = self
            .status_effects
            .iter()
            .filter_map(|effect| effect.tick())
            .collect();

        Self {
            status_effects: new_effects,
            ..self
        }
    }

    #[must_use]
    pub fn with_inventory(self, inventory: Inventory) -> Self {
        Self { inventory, ..self }
    }

    #[must_use]
    pub fn with_equipment(self, equipment: EquipmentSlots) -> Self {
        Self { equipment, ..self }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{Attack, Defense, Health, Mana, Speed, Stat, StatusEffectType};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    #[fixture]
    fn default_stats() -> CombatStats {
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
    fn default_player(default_stats: CombatStats, default_base_stats: BaseStats) -> Player {
        Player::new(
            PlayerIdentifier::new(),
            PlayerName::new("Hero").unwrap(),
            Position::new(0, 0),
            default_stats,
            default_base_stats,
        )
    }

    // =========================================================================
    // Constructor Tests
    // =========================================================================

    mod constructor {
        use super::*;

        #[rstest]
        fn new_creates_player(default_stats: CombatStats, default_base_stats: BaseStats) {
            let identifier = PlayerIdentifier::new();
            let name = PlayerName::new("Test").unwrap();
            let position = Position::new(5, 10);

            let player = Player::new(
                identifier,
                name.clone(),
                position,
                default_stats,
                default_base_stats,
            );

            assert_eq!(*player.identifier(), identifier);
            assert_eq!(player.name().value(), "Test");
            assert_eq!(*player.position(), Position::new(5, 10));
            assert_eq!(player.level().value(), 1);
            assert_eq!(player.experience().value(), 0);
            assert!(player.status_effects().is_empty());
        }

        #[rstest]
        fn new_starts_at_level_one(default_player: Player) {
            assert_eq!(default_player.level().value(), 1);
        }

        #[rstest]
        fn new_starts_with_zero_experience(default_player: Player) {
            assert_eq!(default_player.experience().value(), 0);
        }

        #[rstest]
        fn new_starts_with_empty_equipment(default_player: Player) {
            assert!(default_player.equipment().is_all_empty());
        }

        #[rstest]
        fn new_starts_with_empty_inventory(default_player: Player) {
            assert!(default_player.inventory().is_empty());
        }

        #[rstest]
        fn new_starts_with_no_status_effects(default_player: Player) {
            assert!(default_player.status_effects().is_empty());
        }
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_alive_when_health_positive(default_player: Player) {
            assert!(default_player.is_alive());
        }

        #[rstest]
        fn is_alive_when_health_zero(default_base_stats: BaseStats) {
            let stats = CombatStats::new(
                Health::zero(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            )
            .unwrap();

            let player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Dead").unwrap(),
                Position::new(0, 0),
                stats,
                default_base_stats,
            );

            assert!(!player.is_alive());
        }

        #[rstest]
        fn can_level_up_when_enough_experience(default_player: Player) {
            // Level 1 needs 100 exp
            let player = default_player.gain_experience(Experience::new(100));
            assert!(player.can_level_up());
        }

        #[rstest]
        fn can_level_up_when_not_enough_experience(default_player: Player) {
            let player = default_player.gain_experience(Experience::new(99));
            assert!(!player.can_level_up());
        }

        #[rstest]
        fn can_level_up_at_max_level(default_stats: CombatStats, default_base_stats: BaseStats) {
            let mut player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Max").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            // Level up to max
            for level in 1u64..99 {
                let required = level * 100;
                player = player.gain_experience(Experience::new(required));
                player = player.level_up().unwrap();
            }

            assert_eq!(player.level().value(), 99);
            assert!(!player.can_level_up());
        }
    }

    // =========================================================================
    // Movement Tests
    // =========================================================================

    mod movement {
        use super::*;

        #[rstest]
        fn move_to_updates_position(default_player: Player) {
            let new_position = Position::new(10, 20);
            let moved = default_player.move_to(new_position);
            assert_eq!(*moved.position(), new_position);
        }

        #[rstest]
        fn move_to_preserves_other_fields(default_player: Player) {
            let original_health = default_player.stats().health();
            let original_level = default_player.level();

            let moved = default_player.move_to(Position::new(10, 20));

            assert_eq!(moved.stats().health(), original_health);
            assert_eq!(moved.level(), original_level);
        }
    }

    // =========================================================================
    // Damage and Healing Tests
    // =========================================================================

    mod damage_and_healing {
        use super::*;

        #[rstest]
        fn take_damage_reduces_health(default_player: Player) {
            let damaged = default_player.take_damage(Damage::new(30));
            assert_eq!(damaged.stats().health().value(), 70);
        }

        #[rstest]
        fn take_damage_saturates_at_zero(default_player: Player) {
            let damaged = default_player.take_damage(Damage::new(200));
            assert_eq!(damaged.stats().health().value(), 0);
            assert!(!damaged.is_alive());
        }

        #[rstest]
        fn take_damage_with_zero_damage(default_player: Player) {
            let damaged = default_player.take_damage(Damage::zero());
            assert_eq!(damaged.stats().health().value(), 100);
        }

        #[rstest]
        fn heal_restores_health(default_base_stats: BaseStats) {
            let stats = CombatStats::new(
                Health::new(50).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            )
            .unwrap();

            let player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Wounded").unwrap(),
                Position::new(0, 0),
                stats,
                default_base_stats,
            );

            let healed = player.heal(30);
            assert_eq!(healed.stats().health().value(), 80);
        }

        #[rstest]
        fn heal_caps_at_max_health(default_player: Player) {
            let healed = default_player.heal(100);
            assert_eq!(healed.stats().health().value(), 100);
        }

        #[rstest]
        fn heal_with_zero_amount(default_base_stats: BaseStats) {
            let stats = CombatStats::new(
                Health::new(50).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            )
            .unwrap();

            let player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Wounded").unwrap(),
                Position::new(0, 0),
                stats,
                default_base_stats,
            );

            let healed = player.heal(0);
            assert_eq!(healed.stats().health().value(), 50);
        }
    }

    // =========================================================================
    // Experience and Level Tests
    // =========================================================================

    mod experience_and_level {
        use super::*;

        #[rstest]
        fn gain_experience_increases_experience(default_player: Player) {
            let player = default_player.gain_experience(Experience::new(50));
            assert_eq!(player.experience().value(), 50);
        }

        #[rstest]
        fn gain_experience_accumulates(default_player: Player) {
            let player = default_player
                .gain_experience(Experience::new(30))
                .gain_experience(Experience::new(20));
            assert_eq!(player.experience().value(), 50);
        }

        #[rstest]
        fn level_up_increases_level(default_player: Player) {
            let player = default_player.gain_experience(Experience::new(100));
            let leveled = player.level_up().unwrap();
            assert_eq!(leveled.level().value(), 2);
        }

        #[rstest]
        fn level_up_fails_without_experience(default_player: Player) {
            let result = default_player.level_up();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), PlayerError::LevelCapReached));
        }

        #[rstest]
        fn level_up_fails_at_max_level(default_stats: CombatStats, default_base_stats: BaseStats) {
            let mut player = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Max").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            // Level up to max
            for level in 1u64..99 {
                let required = level * 100;
                player = player.gain_experience(Experience::new(required));
                player = player.level_up().unwrap();
            }

            // Try to level up at max
            let result = player.gain_experience(Experience::new(10000)).level_up();
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Status Effect Tests
    // =========================================================================

    mod status_effects {
        use super::*;

        #[rstest]
        fn apply_status_effect_adds_effect(default_player: Player) {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let poisoned = default_player.apply_status_effect(effect);
            assert_eq!(poisoned.status_effects().len(), 1);
            assert_eq!(
                poisoned.status_effects()[0].effect_type(),
                StatusEffectType::Poison
            );
        }

        #[rstest]
        fn apply_status_effect_replaces_shorter_duration(default_player: Player) {
            let short_effect = StatusEffect::new(StatusEffectType::Poison, 2, 5);
            let long_effect = StatusEffect::new(StatusEffectType::Poison, 5, 5);

            let player = default_player
                .apply_status_effect(short_effect)
                .apply_status_effect(long_effect);

            assert_eq!(player.status_effects().len(), 1);
            assert_eq!(player.status_effects()[0].remaining_turns(), 5);
        }

        #[rstest]
        fn apply_status_effect_keeps_longer_duration(default_player: Player) {
            let long_effect = StatusEffect::new(StatusEffectType::Poison, 5, 5);
            let short_effect = StatusEffect::new(StatusEffectType::Poison, 2, 5);

            let player = default_player
                .apply_status_effect(long_effect)
                .apply_status_effect(short_effect);

            assert_eq!(player.status_effects().len(), 1);
            assert_eq!(player.status_effects()[0].remaining_turns(), 5);
        }

        #[rstest]
        fn apply_status_effect_stacks_shield(default_player: Player) {
            let shield1 = StatusEffect::new(StatusEffectType::Shield, 3, 10);
            let shield2 = StatusEffect::new(StatusEffectType::Shield, 3, 20);

            let player = default_player
                .apply_status_effect(shield1)
                .apply_status_effect(shield2);

            assert_eq!(player.status_effects().len(), 2);
        }

        #[rstest]
        fn tick_status_effects_decreases_duration(default_player: Player) {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let player = default_player.apply_status_effect(effect);

            let ticked = player.tick_status_effects();

            assert_eq!(ticked.status_effects().len(), 1);
            assert_eq!(ticked.status_effects()[0].remaining_turns(), 2);
        }

        #[rstest]
        fn tick_status_effects_removes_expired(default_player: Player) {
            let effect = StatusEffect::new(StatusEffectType::Poison, 1, 5);
            let player = default_player.apply_status_effect(effect);

            let ticked = player.tick_status_effects();

            assert_eq!(ticked.status_effects().len(), 0);
        }

        #[rstest]
        fn tick_status_effects_handles_multiple(default_player: Player) {
            let poison = StatusEffect::new(StatusEffectType::Poison, 2, 5);
            let burn = StatusEffect::new(StatusEffectType::Burn, 1, 3);
            let haste = StatusEffect::new(StatusEffectType::Haste, 3, 10);

            let player = default_player
                .apply_status_effect(poison)
                .apply_status_effect(burn)
                .apply_status_effect(haste);

            let ticked = player.tick_status_effects();

            // Burn should be removed, poison and haste remain
            assert_eq!(ticked.status_effects().len(), 2);
        }
    }

    // =========================================================================
    // Equipment and Inventory Tests
    // =========================================================================

    mod equipment_and_inventory {
        use super::*;
        use crate::player::EquipmentSlot;

        #[rstest]
        fn with_equipment_updates_equipment(default_player: Player) {
            let new_equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());

            let player = default_player.with_equipment(new_equipment.clone());

            assert_eq!(*player.equipment(), new_equipment);
        }

        #[rstest]
        fn with_inventory_updates_inventory(default_player: Player) {
            let new_inventory = Inventory::with_capacity(30);

            let player = default_player.with_inventory(new_inventory.clone());

            assert_eq!(*player.inventory(), new_inventory);
        }
    }

    // =========================================================================
    // Equality and Clone Tests
    // =========================================================================

    mod equality_and_clone {
        use super::*;

        #[rstest]
        fn equality_same_identifier(default_stats: CombatStats, default_base_stats: BaseStats) {
            let identifier = PlayerIdentifier::new();

            let player1 = Player::new(
                identifier,
                PlayerName::new("Hero").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            let player2 = Player::new(
                identifier,
                PlayerName::new("Hero").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            assert_eq!(player1, player2);
        }

        #[rstest]
        fn inequality_different_identifier(
            default_stats: CombatStats,
            default_base_stats: BaseStats,
        ) {
            let player1 = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Hero").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            let player2 = Player::new(
                PlayerIdentifier::new(),
                PlayerName::new("Hero").unwrap(),
                Position::new(0, 0),
                default_stats,
                default_base_stats,
            );

            assert_ne!(player1, player2);
        }

        #[rstest]
        fn clone_creates_equal_player(default_player: Player) {
            let cloned = default_player.clone();
            assert_eq!(default_player, cloned);
        }
    }
}
