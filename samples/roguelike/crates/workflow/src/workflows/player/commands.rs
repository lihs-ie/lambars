use roguelike_domain::common::{Damage, Direction};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_domain::item::ItemIdentifier;

// =============================================================================
// MovePlayerCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlayerCommand {
    game_identifier: GameIdentifier,
    direction: Direction,
}

impl MovePlayerCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, direction: Direction) -> Self {
        Self {
            game_identifier,
            direction,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
}

// =============================================================================
// AttackEnemyCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackEnemyCommand {
    game_identifier: GameIdentifier,
    target: EntityIdentifier,
}

impl AttackEnemyCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, target: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            target,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn target(&self) -> &EntityIdentifier {
        &self.target
    }
}

// =============================================================================
// UseItemCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseItemCommand {
    game_identifier: GameIdentifier,
    item_identifier: ItemIdentifier,
}

impl UseItemCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// PickUpItemCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickUpItemCommand {
    game_identifier: GameIdentifier,
    item_identifier: ItemIdentifier,
}

impl PickUpItemCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// EquipItemCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipItemCommand {
    game_identifier: GameIdentifier,
    item_identifier: ItemIdentifier,
}

impl EquipItemCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// TakeDamageCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeDamageCommand {
    game_identifier: GameIdentifier,
    source: EntityIdentifier,
    base_damage: Damage,
}

impl TakeDamageCommand {
    #[must_use]
    pub const fn new(
        game_identifier: GameIdentifier,
        source: EntityIdentifier,
        base_damage: Damage,
    ) -> Self {
        Self {
            game_identifier,
            source,
            base_damage,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn source(&self) -> &EntityIdentifier {
        &self.source
    }

    #[must_use]
    pub const fn base_damage(&self) -> Damage {
        self.base_damage
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
    // MovePlayerCommand Tests
    // =========================================================================

    mod move_player_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let command = MovePlayerCommand::new(identifier, Direction::Up);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.direction(), Direction::Up);
        }

        #[rstest]
        #[case(Direction::Up)]
        #[case(Direction::Down)]
        #[case(Direction::Left)]
        #[case(Direction::Right)]
        fn new_with_all_directions(#[case] direction: Direction) {
            let identifier = GameIdentifier::new();
            let command = MovePlayerCommand::new(identifier, direction);
            assert_eq!(command.direction(), direction);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let command1 = MovePlayerCommand::new(identifier, Direction::Up);
            let command2 = MovePlayerCommand::new(identifier, Direction::Up);
            let command3 = MovePlayerCommand::new(identifier, Direction::Down);
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = MovePlayerCommand::new(GameIdentifier::new(), Direction::Up);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = MovePlayerCommand::new(GameIdentifier::new(), Direction::Up);
            let debug = format!("{:?}", command);
            assert!(debug.contains("MovePlayerCommand"));
            assert!(debug.contains("Up"));
        }
    }

    // =========================================================================
    // AttackEnemyCommand Tests
    // =========================================================================

    mod attack_enemy_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let command = AttackEnemyCommand::new(identifier, target);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.target(), &target);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let command1 = AttackEnemyCommand::new(identifier, target);
            let command2 = AttackEnemyCommand::new(identifier, target);
            let command3 = AttackEnemyCommand::new(identifier, EntityIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = AttackEnemyCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = AttackEnemyCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("AttackEnemyCommand"));
        }
    }

    // =========================================================================
    // UseItemCommand Tests
    // =========================================================================

    mod use_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = UseItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = UseItemCommand::new(identifier, item);
            let command2 = UseItemCommand::new(identifier, item);
            let command3 = UseItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = UseItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = UseItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("UseItemCommand"));
        }
    }

    // =========================================================================
    // PickUpItemCommand Tests
    // =========================================================================

    mod pick_up_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = PickUpItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = PickUpItemCommand::new(identifier, item);
            let command2 = PickUpItemCommand::new(identifier, item);
            let command3 = PickUpItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = PickUpItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = PickUpItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("PickUpItemCommand"));
        }
    }

    // =========================================================================
    // EquipItemCommand Tests
    // =========================================================================

    mod equip_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = EquipItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = EquipItemCommand::new(identifier, item);
            let command2 = EquipItemCommand::new(identifier, item);
            let command3 = EquipItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = EquipItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = EquipItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("EquipItemCommand"));
        }
    }

    // =========================================================================
    // TakeDamageCommand Tests
    // =========================================================================

    mod take_damage_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let source = EntityIdentifier::new();
            let damage = Damage::new(10);
            let command = TakeDamageCommand::new(identifier, source, damage);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.source(), &source);
            assert_eq!(command.base_damage(), damage);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let source = EntityIdentifier::new();
            let damage = Damage::new(10);
            let command1 = TakeDamageCommand::new(identifier, source, damage);
            let command2 = TakeDamageCommand::new(identifier, source, damage);
            let command3 = TakeDamageCommand::new(identifier, source, Damage::new(20));
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = TakeDamageCommand::new(
                GameIdentifier::new(),
                EntityIdentifier::new(),
                Damage::new(10),
            );
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = TakeDamageCommand::new(
                GameIdentifier::new(),
                EntityIdentifier::new(),
                Damage::new(10),
            );
            let debug = format!("{:?}", command);
            assert!(debug.contains("TakeDamageCommand"));
        }
    }
}
