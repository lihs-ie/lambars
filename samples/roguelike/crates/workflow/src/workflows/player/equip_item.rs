use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::item::ItemIdentifier;
use roguelike_domain::player::{EquipmentSlot, PlayerError};

use super::EquipItemCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// EquipItem Workflow
// =============================================================================

pub fn equip_item<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(EquipItemCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
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
                    // Step 2-6: [Pure] Find item, determine slot, equip, generate events
                    // Note: In a real implementation, item_type and current_equipment
                    // would be extracted from the session.
                    let item_type = EquippableItemType::Weapon;
                    let current_equipment: Option<ItemIdentifier> = None;

                    let result = equip_item_pure(
                        session.clone(),
                        item_identifier,
                        item_type,
                        current_equipment,
                    );

                    match result {
                        Ok((updated_session, events, _equip_result)) => {
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

pub fn equip_item_pure<S>(
    session: S,
    item_identifier: ItemIdentifier,
    item_type: EquippableItemType,
    current_equipment: Option<ItemIdentifier>,
) -> Result<(S, Vec<GameSessionEvent>, EquipResult), WorkflowError>
where
    S: Clone,
{
    // [Pure] Equipment slot pipeline using pipe!
    pipe!(
        item_type,
        // Step 1: Determine equipment slot
        determine_equipment_slot,
        // Step 2: Validate compatibility
        |slot| {
            validate_equip_compatibility(item_type, slot)
                .map(|()| slot)
                .map_err(|error| WorkflowError::repository("equip_validation", error.to_string()))
        },
        // Step 3: Perform equip operation
        |result: Result<EquipmentSlot, WorkflowError>| {
            result.map(|slot| perform_equip(current_equipment, item_identifier, slot))
        },
        // Step 4: Generate events
        |result: Result<EquipResult, WorkflowError>| {
            result.map(|equip_result| {
                let events: Vec<GameSessionEvent> = vec![];
                (session, events, equip_result)
            })
        }
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquippableItemType {
    Weapon,
    Armor,
    Helmet,
    Accessory,
}

impl EquippableItemType {
    #[must_use]
    pub const fn equipment_slot(self) -> EquipmentSlot {
        match self {
            Self::Weapon => EquipmentSlot::Weapon,
            Self::Armor => EquipmentSlot::Armor,
            Self::Helmet => EquipmentSlot::Helmet,
            Self::Accessory => EquipmentSlot::Accessory,
        }
    }
}

#[must_use]
pub const fn determine_equipment_slot(item_type: EquippableItemType) -> EquipmentSlot {
    item_type.equipment_slot()
}

pub fn validate_equip_compatibility(
    item_type: EquippableItemType,
    target_slot: EquipmentSlot,
) -> Result<(), PlayerError> {
    let expected_slot = item_type.equipment_slot();
    if expected_slot == target_slot {
        Ok(())
    } else {
        Err(PlayerError::cannot_equip_item_type(
            format!("{:?}", item_type),
            target_slot,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipResult {
    equipped_item: ItemIdentifier,
    unequipped_item: Option<ItemIdentifier>,
    slot: EquipmentSlot,
}

impl EquipResult {
    #[must_use]
    pub const fn new(equipped_item: ItemIdentifier, slot: EquipmentSlot) -> Self {
        Self {
            equipped_item,
            unequipped_item: None,
            slot,
        }
    }

    #[must_use]
    pub const fn with_swap(
        equipped_item: ItemIdentifier,
        unequipped_item: ItemIdentifier,
        slot: EquipmentSlot,
    ) -> Self {
        Self {
            equipped_item,
            unequipped_item: Some(unequipped_item),
            slot,
        }
    }

    #[must_use]
    pub const fn equipped_item(&self) -> &ItemIdentifier {
        &self.equipped_item
    }

    #[must_use]
    pub const fn unequipped_item(&self) -> Option<&ItemIdentifier> {
        self.unequipped_item.as_ref()
    }

    #[must_use]
    pub const fn slot(&self) -> EquipmentSlot {
        self.slot
    }

    #[must_use]
    pub const fn is_swap(&self) -> bool {
        self.unequipped_item.is_some()
    }
}

#[must_use]
pub fn perform_equip(
    current_equipment: Option<ItemIdentifier>,
    new_item: ItemIdentifier,
    slot: EquipmentSlot,
) -> EquipResult {
    match current_equipment {
        Some(old_item) => EquipResult::with_swap(new_item, old_item, slot),
        None => EquipResult::new(new_item, slot),
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
    // EquippableItemType Tests
    // =========================================================================

    mod equippable_item_type_tests {
        use super::*;

        #[rstest]
        #[case(EquippableItemType::Weapon, EquipmentSlot::Weapon)]
        #[case(EquippableItemType::Armor, EquipmentSlot::Armor)]
        #[case(EquippableItemType::Helmet, EquipmentSlot::Helmet)]
        #[case(EquippableItemType::Accessory, EquipmentSlot::Accessory)]
        fn equipment_slot_maps_correctly(
            #[case] item_type: EquippableItemType,
            #[case] expected_slot: EquipmentSlot,
        ) {
            assert_eq!(item_type.equipment_slot(), expected_slot);
        }
    }

    // =========================================================================
    // Determine Equipment Slot Tests
    // =========================================================================

    mod determine_equipment_slot_tests {
        use super::*;

        #[rstest]
        fn weapon_goes_to_weapon_slot() {
            let slot = determine_equipment_slot(EquippableItemType::Weapon);
            assert_eq!(slot, EquipmentSlot::Weapon);
        }

        #[rstest]
        fn armor_goes_to_armor_slot() {
            let slot = determine_equipment_slot(EquippableItemType::Armor);
            assert_eq!(slot, EquipmentSlot::Armor);
        }
    }

    // =========================================================================
    // Validate Equip Compatibility Tests
    // =========================================================================

    mod validate_equip_compatibility_tests {
        use super::*;

        #[rstest]
        fn compatible_returns_ok() {
            let result =
                validate_equip_compatibility(EquippableItemType::Weapon, EquipmentSlot::Weapon);
            assert!(result.is_ok());
        }

        #[rstest]
        fn incompatible_returns_error() {
            let result =
                validate_equip_compatibility(EquippableItemType::Weapon, EquipmentSlot::Armor);
            assert!(matches!(
                result,
                Err(PlayerError::CannotEquipItemType { .. })
            ));
        }

        #[rstest]
        fn all_compatible_pairs() {
            let pairs = [
                (EquippableItemType::Weapon, EquipmentSlot::Weapon),
                (EquippableItemType::Armor, EquipmentSlot::Armor),
                (EquippableItemType::Helmet, EquipmentSlot::Helmet),
                (EquippableItemType::Accessory, EquipmentSlot::Accessory),
            ];

            for (item_type, slot) in pairs {
                let result = validate_equip_compatibility(item_type, slot);
                assert!(
                    result.is_ok(),
                    "Expected {:?} to be compatible with {:?}",
                    item_type,
                    slot
                );
            }
        }
    }

    // =========================================================================
    // EquipResult Tests
    // =========================================================================

    mod equip_result_tests {
        use super::*;

        #[rstest]
        fn new_creates_result_without_unequipped() {
            let item = ItemIdentifier::new();
            let result = EquipResult::new(item, EquipmentSlot::Weapon);

            assert_eq!(result.equipped_item(), &item);
            assert!(result.unequipped_item().is_none());
            assert_eq!(result.slot(), EquipmentSlot::Weapon);
            assert!(!result.is_swap());
        }

        #[rstest]
        fn with_swap_creates_result_with_unequipped() {
            let new_item = ItemIdentifier::new();
            let old_item = ItemIdentifier::new();
            let result = EquipResult::with_swap(new_item, old_item, EquipmentSlot::Armor);

            assert_eq!(result.equipped_item(), &new_item);
            assert_eq!(result.unequipped_item(), Some(&old_item));
            assert_eq!(result.slot(), EquipmentSlot::Armor);
            assert!(result.is_swap());
        }
    }

    // =========================================================================
    // Perform Equip Tests
    // =========================================================================

    mod perform_equip_tests {
        use super::*;

        #[rstest]
        fn equip_to_empty_slot() {
            let new_item = ItemIdentifier::new();
            let result = perform_equip(None, new_item, EquipmentSlot::Weapon);

            assert_eq!(result.equipped_item(), &new_item);
            assert!(result.unequipped_item().is_none());
            assert!(!result.is_swap());
        }

        #[rstest]
        fn equip_to_occupied_slot() {
            let new_item = ItemIdentifier::new();
            let old_item = ItemIdentifier::new();
            let result = perform_equip(Some(old_item), new_item, EquipmentSlot::Weapon);

            assert_eq!(result.equipped_item(), &new_item);
            assert_eq!(result.unequipped_item(), Some(&old_item));
            assert!(result.is_swap());
        }

        #[rstest]
        fn swap_preserves_slot() {
            let new_item = ItemIdentifier::new();
            let old_item = ItemIdentifier::new();
            let result = perform_equip(Some(old_item), new_item, EquipmentSlot::Helmet);

            assert_eq!(result.slot(), EquipmentSlot::Helmet);
        }
    }
}
