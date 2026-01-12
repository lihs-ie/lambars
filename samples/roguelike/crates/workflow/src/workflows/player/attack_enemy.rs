use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use lambars::typeclass::Monoid;
use roguelike_domain::combat::CombatError;
use roguelike_domain::common::{Attack, Damage, DamageModifier, Defense};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::player::{EntityIdentifier as PlayerEntityIdentifier, PlayerAttacked};

use super::AttackEnemyCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

const DEFAULT_MELEE_RANGE: u32 = 1;

// =============================================================================
// AttackEnemy Workflow
// =============================================================================

pub fn attack_enemy<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(AttackEnemyCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let target = *command.target();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Step 2-8: [Pure] Validate, calculate damage, apply, generate events
                    // Note: In a real implementation, attacker_position, target_position,
                    // base_attack, target_defense, and modifiers would be extracted from the session.
                    // For now, we use default values to demonstrate the pattern.
                    let attacker_position = (0, 0);
                    let target_position = (1, 0);
                    let base_attack = Attack::new(10);
                    let target_defense = Defense::new(5);
                    let modifiers: Vec<DamageModifier> = vec![];

                    let result = attack_enemy_pure(
                        session.clone(),
                        attacker_position,
                        target_position,
                        base_attack,
                        target_defense,
                        &modifiers,
                        &target,
                    );

                    match result {
                        Ok((updated_session, events)) => {
                            // Step 9-10: [IO] Update cache and append events
                            let game_identifier_clone = game_identifier;
                            let updated_session_clone = updated_session.clone();

                            cache
                                .set(
                                    &game_identifier_clone,
                                    &updated_session,
                                    DEFAULT_CACHE_TIME_TO_LIVE,
                                )
                                .flat_map(move |()| {
                                    event_store
                                        .append(&game_identifier_clone, &events)
                                        .fmap(move |()| Ok(updated_session_clone))
                                })
                        }
                        Err(error) => AsyncIO::pure(Err(error)),
                    }
                }
                None => AsyncIO::pure(Err(WorkflowError::not_found(
                    "GameSession",
                    game_identifier.to_string(),
                ))),
            }
        })
    }
}

// =============================================================================
// Pure Functions
// =============================================================================

// =============================================================================
// Step Functions (Pure)
// =============================================================================

fn extract_combat_stats(attack: Attack, defense: Defense) -> (Attack, Defense) {
    (attack, defense)
}

fn validate_and_get_stats(
    attacker_position: (i32, i32),
    target_position: (i32, i32),
    base_attack: Attack,
    target_defense: Defense,
) -> Result<(Attack, Defense), WorkflowError> {
    validate_attack_target(attacker_position, target_position, DEFAULT_MELEE_RANGE)
        .map(|()| extract_combat_stats(base_attack, target_defense))
        .map_err(|error| WorkflowError::repository("attack_validation", error.to_string()))
}

fn calculate_damage_from_stats(stats: (Attack, Defense), modifiers: &[DamageModifier]) -> Damage {
    let (attack, defense) = stats;
    calculate_damage(attack, defense, modifiers)
}

fn create_attack_result<S: Clone>(
    session: S,
    target: &EntityIdentifier,
    damage: Damage,
) -> (S, Vec<GameSessionEvent>, Damage) {
    let _event = create_player_attacked_event(target, damage);
    // Note: GameSessionEvent does not have a Player variant yet.
    // Events are returned as an empty vector for now.
    let events: Vec<GameSessionEvent> = vec![];
    (session, events, damage)
}

pub fn attack_enemy_pure<S>(
    session: S,
    attacker_position: (i32, i32),
    target_position: (i32, i32),
    base_attack: Attack,
    target_defense: Defense,
    modifiers: &[DamageModifier],
    target: &EntityIdentifier,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError>
where
    S: Clone,
{
    // [Pure] Damage calculation pipeline using pipe! with named functions
    let validation_result = validate_and_get_stats(
        attacker_position,
        target_position,
        base_attack,
        target_defense,
    );

    pipe!(
        validation_result,
        // Step 1: Calculate damage with Monoid semantics
        |stats_result: Result<(Attack, Defense), WorkflowError>| {
            stats_result.map(|stats| calculate_damage_from_stats(stats, modifiers))
        },
        // Step 2: Create result with session and events
        |damage_result: Result<Damage, WorkflowError>| {
            damage_result.map(|damage| {
                let (updated_session, events, _damage) =
                    create_attack_result(session, target, damage);
                (updated_session, events)
            })
        }
    )
}

pub fn validate_attack_target(
    attacker_position: (i32, i32),
    target_position: (i32, i32),
    attack_range: u32,
) -> Result<(), CombatError> {
    let distance = calculate_manhattan_distance(attacker_position, target_position);

    if distance > attack_range {
        return Err(CombatError::target_not_in_range(
            attacker_position,
            target_position,
            attack_range,
        ));
    }

    Ok(())
}

#[must_use]
pub fn calculate_manhattan_distance(from: (i32, i32), to: (i32, i32)) -> u32 {
    let dx = (from.0 - to.0).unsigned_abs();
    let dy = (from.1 - to.1).unsigned_abs();
    dx + dy
}

#[must_use]
pub fn calculate_damage(
    base_attack: Attack,
    target_defense: Defense,
    modifiers: &[DamageModifier],
) -> Damage {
    // Base damage calculation
    let base_damage = base_attack.value().saturating_sub(target_defense.value());
    let base_damage = base_damage.max(1); // Minimum 1 damage

    // Apply modifiers using Semigroup semantics
    let combined_modifier = combine_damage_modifiers(modifiers);
    let modified_damage = combined_modifier.apply(Damage::new(base_damage));

    // Ensure minimum 1 damage
    if modified_damage.value() == 0 {
        Damage::new(1)
    } else {
        modified_damage
    }
}

#[must_use]
pub fn combine_damage_modifiers(modifiers: &[DamageModifier]) -> DamageModifier {
    DamageModifier::combine_all(modifiers.iter().copied())
}

#[must_use]
pub const fn is_entity_dead(health: u32) -> bool {
    health == 0
}

#[must_use]
pub fn create_player_attacked_event(target: &EntityIdentifier, damage: Damage) -> PlayerAttacked {
    PlayerAttacked::new(PlayerEntityIdentifier::new(target.to_string()), damage)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Validate Attack Target Tests
    // =========================================================================

    mod validate_attack_target_tests {
        use super::*;

        #[rstest]
        fn adjacent_target_is_valid() {
            let result = validate_attack_target((5, 5), (6, 5), DEFAULT_MELEE_RANGE);
            assert!(result.is_ok());
        }

        #[rstest]
        fn same_position_is_valid() {
            let result = validate_attack_target((5, 5), (5, 5), DEFAULT_MELEE_RANGE);
            assert!(result.is_ok());
        }

        #[rstest]
        fn diagonal_adjacent_is_out_of_range_for_melee() {
            // Manhattan distance is 2 for diagonal
            let result = validate_attack_target((5, 5), (6, 6), DEFAULT_MELEE_RANGE);
            assert!(matches!(result, Err(CombatError::TargetNotInRange { .. })));
        }

        #[rstest]
        fn distant_target_is_out_of_range() {
            let result = validate_attack_target((0, 0), (10, 10), DEFAULT_MELEE_RANGE);
            assert!(matches!(result, Err(CombatError::TargetNotInRange { .. })));
        }

        #[rstest]
        fn extended_range_allows_diagonal() {
            let result = validate_attack_target((5, 5), (6, 6), 2);
            assert!(result.is_ok());
        }

        #[rstest]
        fn ranged_attack_at_distance() {
            let result = validate_attack_target((0, 0), (5, 5), 10);
            assert!(result.is_ok());
        }
    }

    // =========================================================================
    // Calculate Manhattan Distance Tests
    // =========================================================================

    mod calculate_manhattan_distance_tests {
        use super::*;

        #[rstest]
        #[case((0, 0), (0, 0), 0)]
        #[case((0, 0), (1, 0), 1)]
        #[case((0, 0), (0, 1), 1)]
        #[case((0, 0), (1, 1), 2)]
        #[case((0, 0), (3, 4), 7)]
        #[case((5, 5), (10, 10), 10)]
        fn calculates_correct_distance(
            #[case] from: (i32, i32),
            #[case] to: (i32, i32),
            #[case] expected: u32,
        ) {
            let result = calculate_manhattan_distance(from, to);
            assert_eq!(result, expected);
        }

        #[rstest]
        fn handles_negative_coordinates() {
            let result = calculate_manhattan_distance((-5, -5), (5, 5));
            assert_eq!(result, 20);
        }

        #[rstest]
        fn is_symmetric() {
            let distance1 = calculate_manhattan_distance((0, 0), (5, 3));
            let distance2 = calculate_manhattan_distance((5, 3), (0, 0));
            assert_eq!(distance1, distance2);
        }
    }

    // =========================================================================
    // Calculate Damage Tests
    // =========================================================================

    mod calculate_damage_tests {
        use super::*;

        #[rstest]
        fn basic_damage_calculation() {
            let attack = Attack::new(10);
            let defense = Defense::new(3);
            let result = calculate_damage(attack, defense, &[]);
            assert_eq!(result.value(), 7);
        }

        #[rstest]
        fn minimum_damage_is_one() {
            let attack = Attack::new(5);
            let defense = Defense::new(10);
            let result = calculate_damage(attack, defense, &[]);
            assert_eq!(result.value(), 1);
        }

        #[rstest]
        fn equal_attack_and_defense() {
            let attack = Attack::new(10);
            let defense = Defense::new(10);
            let result = calculate_damage(attack, defense, &[]);
            assert_eq!(result.value(), 1); // Minimum 1 damage
        }

        #[rstest]
        fn damage_with_multiplier_modifier() {
            let attack = Attack::new(10);
            let defense = Defense::new(0);
            let modifiers = vec![DamageModifier::new(2.0, 0)];
            let result = calculate_damage(attack, defense, &modifiers);
            assert_eq!(result.value(), 20);
        }

        #[rstest]
        fn damage_with_additive_modifier() {
            let attack = Attack::new(10);
            let defense = Defense::new(0);
            let modifiers = vec![DamageModifier::new(1.0, 5)];
            let result = calculate_damage(attack, defense, &modifiers);
            assert_eq!(result.value(), 15);
        }

        #[rstest]
        fn damage_with_multiple_modifiers() {
            let attack = Attack::new(10);
            let defense = Defense::new(0);
            let modifiers = vec![
                DamageModifier::new(2.0, 0), // multiplier
                DamageModifier::new(1.0, 5), // additive
            ];
            let result = calculate_damage(attack, defense, &modifiers);
            // Combined: (2.0 * 1.0, 0 + 5) = (2.0, 5)
            // Applied: 10 * 2.0 + 5 = 25
            assert_eq!(result.value(), 25);
        }
    }

    // =========================================================================
    // Combine Damage Modifiers Tests
    // =========================================================================

    mod combine_damage_modifiers_tests {
        use super::*;

        #[rstest]
        fn empty_modifiers_returns_identity() {
            let combined = combine_damage_modifiers(&[]);
            let result = combined.apply(Damage::new(10));
            assert_eq!(result.value(), 10);
        }

        #[rstest]
        fn single_modifier_is_applied() {
            let modifiers = vec![DamageModifier::new(2.0, 0)];
            let combined = combine_damage_modifiers(&modifiers);
            let result = combined.apply(Damage::new(10));
            assert_eq!(result.value(), 20);
        }

        #[rstest]
        fn multiple_modifiers_are_combined() {
            let modifiers = vec![
                DamageModifier::new(1.0, 5), // additive
                DamageModifier::new(2.0, 0), // multiplier
            ];
            let combined = combine_damage_modifiers(&modifiers);
            // Combined: multipliers multiply (1.0 * 2.0 = 2.0), bonuses add (5 + 0 = 5)
            // Applied: 10 * 2.0 + 5 = 25
            let result = combined.apply(Damage::new(10));
            assert_eq!(result.value(), 25);
        }
    }

    // =========================================================================
    // Is Entity Dead Tests
    // =========================================================================

    mod is_entity_dead_tests {
        use super::*;

        #[rstest]
        fn zero_health_is_dead() {
            assert!(is_entity_dead(0));
        }

        #[rstest]
        fn positive_health_is_alive() {
            assert!(!is_entity_dead(1));
            assert!(!is_entity_dead(100));
        }
    }

    // =========================================================================
    // Create Player Attacked Event Tests
    // =========================================================================

    mod create_player_attacked_event_tests {
        use super::*;

        #[rstest]
        fn creates_event_with_target_and_damage() {
            let target = EntityIdentifier::new();
            let damage = Damage::new(10);

            let event = create_player_attacked_event(&target, damage);

            assert_eq!(event.target().value(), target.to_string().as_str());
            assert_eq!(event.damage(), damage);
        }
    }
}
