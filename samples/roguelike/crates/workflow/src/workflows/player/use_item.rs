use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::item::ItemIdentifier;
use roguelike_domain::player::PlayerError;

use super::UseItemCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// UseItem Workflow
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
struct InventoryItem {
    identifier: ItemIdentifier,
    effect: ItemEffect,
}

impl InventoryItem {
    #[cfg(test)]
    fn new(identifier: ItemIdentifier, effect: ItemEffect) -> Self {
        Self { identifier, effect }
    }
}

pub fn use_item<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(UseItemCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let item_identifier = *command.item_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Step 2-6: [Pure] Find item, validate, apply effect, remove, generate events
                    // Note: In a real implementation, these values would be extracted from the session.
                    // For now, we use a simplified approach.
                    let inventory: Vec<InventoryItem> = vec![];
                    let current_health = 100u32;
                    let max_health = 100u32;

                    let result = use_item_pure_simplified(
                        session.clone(),
                        &inventory,
                        &item_identifier,
                        current_health,
                        max_health,
                    );

                    match result {
                        Ok((updated_session, events, _new_health)) => {
                            // Step 7-8: [IO] Update cache and append events
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

pub fn use_item_pure<S, I, F, G>(
    session: S,
    inventory: &[I],
    item_identifier: &ItemIdentifier,
    get_identifier: F,
    get_effect: G,
    current_health: u32,
    max_health: u32,
) -> Result<(S, Vec<GameSessionEvent>, u32), WorkflowError>
where
    S: Clone,
    F: Fn(&I) -> &ItemIdentifier,
    G: Fn(&I) -> ItemEffect,
{
    // [Pure] Item usage pipeline using pipe!
    pipe!(
        inventory,
        // Step 1: Find item in inventory
        |inv| {
            find_item_in_inventory(inv, item_identifier, &get_identifier)
                .map(|index| (index, &inv[index]))
                .map_err(|error| WorkflowError::repository("find_item", error.to_string()))
        },
        // Step 2: Get item effect and calculate result
        |result: Result<(usize, &I), WorkflowError>| {
            result.map(|(_index, item)| get_effect(item))
        },
        // Step 3: Apply effect and generate events
        |result: Result<ItemEffect, WorkflowError>| {
            result.map(|effect| {
                let new_health = match effect.effect_type() {
                    ItemEffectType::HealHealth => {
                        apply_heal_health(current_health, max_health, effect.value())
                    }
                    ItemEffectType::HealMana => {
                        apply_heal_mana(current_health, max_health, effect.value())
                    }
                    _ => current_health,
                };
                let events: Vec<GameSessionEvent> = vec![];
                (session, events, new_health)
            })
        }
    )
}

fn use_item_pure_simplified<S>(
    session: S,
    inventory: &[InventoryItem],
    item_identifier: &ItemIdentifier,
    current_health: u32,
    max_health: u32,
) -> Result<(S, Vec<GameSessionEvent>, u32), WorkflowError>
where
    S: Clone,
{
    // Step 1: Find item in inventory
    let find_result = inventory
        .iter()
        .position(|item| &item.identifier == item_identifier)
        .ok_or_else(|| WorkflowError::not_found("InventoryItem", item_identifier.to_string()));

    // Step 2: Get effect and apply
    find_result.map(|index| {
        let effect = inventory[index].effect.clone();
        let new_health = match effect.effect_type() {
            ItemEffectType::HealHealth => {
                apply_heal_health(current_health, max_health, effect.value())
            }
            ItemEffectType::HealMana => apply_heal_mana(current_health, max_health, effect.value()),
            _ => current_health,
        };
        let events: Vec<GameSessionEvent> = vec![];
        (session, events, new_health)
    })
}

pub fn find_item_in_inventory<I, F>(
    inventory: &[I],
    item_identifier: &ItemIdentifier,
    get_identifier: F,
) -> Result<usize, PlayerError>
where
    F: Fn(&I) -> &ItemIdentifier,
{
    inventory
        .iter()
        .position(|item| get_identifier(item) == item_identifier)
        .ok_or_else(|| PlayerError::item_not_in_inventory(item_identifier.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEffectType {
    HealHealth,
    HealMana,
    BuffAttack,
    BuffDefense,
    CureStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEffect {
    effect_type: ItemEffectType,
    value: u32,
    duration: u32,
}

impl ItemEffect {
    #[must_use]
    pub const fn new(effect_type: ItemEffectType, value: u32, duration: u32) -> Self {
        Self {
            effect_type,
            value,
            duration,
        }
    }

    #[must_use]
    pub const fn heal_health(amount: u32) -> Self {
        Self::new(ItemEffectType::HealHealth, amount, 0)
    }

    #[must_use]
    pub const fn heal_mana(amount: u32) -> Self {
        Self::new(ItemEffectType::HealMana, amount, 0)
    }

    #[must_use]
    pub const fn buff_attack(amount: u32, duration: u32) -> Self {
        Self::new(ItemEffectType::BuffAttack, amount, duration)
    }

    #[must_use]
    pub const fn buff_defense(amount: u32, duration: u32) -> Self {
        Self::new(ItemEffectType::BuffDefense, amount, duration)
    }

    #[must_use]
    pub const fn effect_type(&self) -> ItemEffectType {
        self.effect_type
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.value
    }

    #[must_use]
    pub const fn duration(&self) -> u32 {
        self.duration
    }

    #[must_use]
    pub const fn is_instant(&self) -> bool {
        self.duration == 0
    }
}

#[must_use]
pub const fn apply_heal_health(current_health: u32, max_health: u32, heal_amount: u32) -> u32 {
    let new_health = current_health.saturating_add(heal_amount);
    if new_health > max_health {
        max_health
    } else {
        new_health
    }
}

#[must_use]
pub const fn apply_heal_mana(current_mana: u32, max_mana: u32, heal_amount: u32) -> u32 {
    let new_mana = current_mana.saturating_add(heal_amount);
    if new_mana > max_mana {
        max_mana
    } else {
        new_mana
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
    // Find Item In Inventory Tests
    // =========================================================================

    mod find_item_in_inventory_tests {
        use super::*;

        struct TestItem {
            identifier: ItemIdentifier,
        }

        #[rstest]
        fn finds_existing_item() {
            let target_id = ItemIdentifier::new();
            let inventory = vec![
                TestItem {
                    identifier: ItemIdentifier::new(),
                },
                TestItem {
                    identifier: target_id,
                },
                TestItem {
                    identifier: ItemIdentifier::new(),
                },
            ];

            let result = find_item_in_inventory(&inventory, &target_id, |item| &item.identifier);

            assert_eq!(result, Ok(1));
        }

        #[rstest]
        fn returns_error_for_missing_item() {
            let target_id = ItemIdentifier::new();
            let inventory = vec![
                TestItem {
                    identifier: ItemIdentifier::new(),
                },
                TestItem {
                    identifier: ItemIdentifier::new(),
                },
            ];

            let result = find_item_in_inventory(&inventory, &target_id, |item| &item.identifier);

            assert!(matches!(
                result,
                Err(PlayerError::ItemNotInInventory { .. })
            ));
        }

        #[rstest]
        fn empty_inventory_returns_error() {
            let target_id = ItemIdentifier::new();
            let inventory: Vec<TestItem> = vec![];

            let result = find_item_in_inventory(&inventory, &target_id, |item| &item.identifier);

            assert!(matches!(
                result,
                Err(PlayerError::ItemNotInInventory { .. })
            ));
        }

        #[rstest]
        fn finds_first_item() {
            let target_id = ItemIdentifier::new();
            let inventory = vec![TestItem {
                identifier: target_id,
            }];

            let result = find_item_in_inventory(&inventory, &target_id, |item| &item.identifier);

            assert_eq!(result, Ok(0));
        }
    }

    // =========================================================================
    // Item Effect Tests
    // =========================================================================

    mod item_effect_tests {
        use super::*;

        #[rstest]
        fn heal_health_effect() {
            let effect = ItemEffect::heal_health(50);
            assert_eq!(effect.effect_type(), ItemEffectType::HealHealth);
            assert_eq!(effect.value(), 50);
            assert!(effect.is_instant());
        }

        #[rstest]
        fn heal_mana_effect() {
            let effect = ItemEffect::heal_mana(30);
            assert_eq!(effect.effect_type(), ItemEffectType::HealMana);
            assert_eq!(effect.value(), 30);
            assert!(effect.is_instant());
        }

        #[rstest]
        fn buff_attack_effect() {
            let effect = ItemEffect::buff_attack(10, 5);
            assert_eq!(effect.effect_type(), ItemEffectType::BuffAttack);
            assert_eq!(effect.value(), 10);
            assert_eq!(effect.duration(), 5);
            assert!(!effect.is_instant());
        }

        #[rstest]
        fn buff_defense_effect() {
            let effect = ItemEffect::buff_defense(15, 3);
            assert_eq!(effect.effect_type(), ItemEffectType::BuffDefense);
            assert_eq!(effect.value(), 15);
            assert_eq!(effect.duration(), 3);
            assert!(!effect.is_instant());
        }
    }

    // =========================================================================
    // Apply Heal Health Tests
    // =========================================================================

    mod apply_heal_health_tests {
        use super::*;

        #[rstest]
        fn heals_within_max() {
            let result = apply_heal_health(50, 100, 30);
            assert_eq!(result, 80);
        }

        #[rstest]
        fn caps_at_max_health() {
            let result = apply_heal_health(80, 100, 50);
            assert_eq!(result, 100);
        }

        #[rstest]
        fn already_at_max() {
            let result = apply_heal_health(100, 100, 50);
            assert_eq!(result, 100);
        }

        #[rstest]
        fn heal_from_zero() {
            let result = apply_heal_health(0, 100, 50);
            assert_eq!(result, 50);
        }

        #[rstest]
        fn heal_with_zero_amount() {
            let result = apply_heal_health(50, 100, 0);
            assert_eq!(result, 50);
        }
    }

    // =========================================================================
    // Apply Heal Mana Tests
    // =========================================================================

    mod apply_heal_mana_tests {
        use super::*;

        #[rstest]
        fn heals_within_max() {
            let result = apply_heal_mana(30, 100, 40);
            assert_eq!(result, 70);
        }

        #[rstest]
        fn caps_at_max_mana() {
            let result = apply_heal_mana(80, 100, 50);
            assert_eq!(result, 100);
        }

        #[rstest]
        fn already_at_max() {
            let result = apply_heal_mana(100, 100, 30);
            assert_eq!(result, 100);
        }

        #[rstest]
        fn heal_from_zero() {
            let result = apply_heal_mana(0, 100, 50);
            assert_eq!(result, 50);
        }
    }
}
