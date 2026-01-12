use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use roguelike_domain::common::Position;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::item::ItemIdentifier;
use roguelike_domain::player::PlayerError;

use super::PickUpItemCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// PickUpItem Workflow
// =============================================================================

pub fn pick_up_item<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(PickUpItemCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
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
                    // Step 2-6: [Pure] Find item, validate, remove from floor, add to inventory
                    // Note: In a real implementation, these values would be extracted from the session.
                    // For now, we use a simplified approach that returns an error indicating
                    // the item was not found (since floor_items is empty).
                    let floor_items: Vec<FloorItem<ItemIdentifier>> = vec![];
                    let player_position = Position::new(0, 0);
                    let current_inventory_count = 0u32;
                    let max_inventory_capacity = 20u32;

                    let result = pick_up_item_pure_simplified(
                        session.clone(),
                        &floor_items,
                        &item_identifier,
                        player_position,
                        current_inventory_count,
                        max_inventory_capacity,
                    );

                    match result {
                        Ok((updated_session, events, _floor_items, _picked_item)) => {
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

#[allow(clippy::type_complexity)]
pub fn pick_up_item_pure<S, I, F>(
    session: S,
    floor_items: &[FloorItem<I>],
    item_identifier: &ItemIdentifier,
    player_position: Position,
    current_inventory_count: u32,
    max_inventory_capacity: u32,
    get_identifier: F,
) -> Result<(S, Vec<GameSessionEvent>, Vec<FloorItem<I>>, I), WorkflowError>
where
    S: Clone,
    I: Clone,
    F: Fn(&I) -> &ItemIdentifier,
{
    // [Pure] Inventory update pipeline using pipe!
    pipe!(
        floor_items,
        // Step 1: Find item on floor at player position
        |items| {
            find_item_on_floor(items, item_identifier, player_position, &get_identifier)
                .map(|index| (index, items[index].item().clone()))
                .map_err(|error| WorkflowError::repository("find_item", error.to_string()))
        },
        // Step 2: Validate inventory space
        |result: Result<(usize, I), WorkflowError>| {
            result.and_then(|(index, item)| {
                validate_inventory_space(current_inventory_count, max_inventory_capacity)
                    .map(|()| (index, item))
                    .map_err(|error| {
                        WorkflowError::repository("validate_inventory", error.to_string())
                    })
            })
        },
        // Step 3: Remove item from floor and generate result
        |result: Result<(usize, I), WorkflowError>| {
            result.map(|(index, item)| {
                let updated_floor_items = remove_item_at_index(floor_items, index);
                let events: Vec<GameSessionEvent> = vec![];
                (session, events, updated_floor_items, item)
            })
        }
    )
}

#[allow(clippy::type_complexity)]
fn pick_up_item_pure_simplified<S>(
    session: S,
    floor_items: &[FloorItem<ItemIdentifier>],
    item_identifier: &ItemIdentifier,
    player_position: Position,
    current_inventory_count: u32,
    max_inventory_capacity: u32,
) -> Result<
    (
        S,
        Vec<GameSessionEvent>,
        Vec<FloorItem<ItemIdentifier>>,
        ItemIdentifier,
    ),
    WorkflowError,
>
where
    S: Clone,
{
    // Step 1: Find item on floor at player position
    let find_result = floor_items
        .iter()
        .position(|floor_item| {
            floor_item.position() == player_position && floor_item.item() == item_identifier
        })
        .ok_or_else(|| WorkflowError::not_found("FloorItem", item_identifier.to_string()));

    // Step 2: Validate inventory space
    let validated = find_result.and_then(|index| {
        validate_inventory_space(current_inventory_count, max_inventory_capacity)
            .map(|()| (index, *floor_items[index].item()))
            .map_err(|error| WorkflowError::repository("validate_inventory", error.to_string()))
    });

    // Step 3: Remove item from floor and generate result
    validated.map(|(index, item)| {
        let updated_floor_items = remove_item_at_index(floor_items, index);
        let events: Vec<GameSessionEvent> = vec![];
        (session, events, updated_floor_items, item)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorItem<I> {
    item: I,
    position: Position,
}

impl<I> FloorItem<I> {
    #[must_use]
    pub const fn new(item: I, position: Position) -> Self {
        Self { item, position }
    }

    #[must_use]
    pub const fn item(&self) -> &I {
        &self.item
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    #[must_use]
    pub fn into_item(self) -> I {
        self.item
    }
}

pub fn find_item_on_floor<I, F>(
    floor_items: &[FloorItem<I>],
    item_identifier: &ItemIdentifier,
    player_position: Position,
    get_identifier: F,
) -> Result<usize, PlayerError>
where
    F: Fn(&I) -> &ItemIdentifier,
{
    floor_items
        .iter()
        .position(|floor_item| {
            floor_item.position() == player_position
                && get_identifier(floor_item.item()) == item_identifier
        })
        .ok_or_else(|| PlayerError::item_not_in_inventory(item_identifier.to_string()))
}

pub fn validate_inventory_space(current_count: u32, max_capacity: u32) -> Result<(), PlayerError> {
    if current_count >= max_capacity {
        Err(PlayerError::inventory_full(max_capacity))
    } else {
        Ok(())
    }
}

#[must_use]
pub fn remove_item_at_index<T: Clone>(items: &[T], index: usize) -> Vec<T> {
    items
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, item)| item.clone())
        .collect()
}

#[must_use]
pub fn add_item_to_list<T: Clone>(items: &[T], new_item: T) -> Vec<T> {
    let mut new_items: Vec<T> = items.to_vec();
    new_items.push(new_item);
    new_items
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // FloorItem Tests
    // =========================================================================

    mod floor_item_tests {
        use super::*;

        #[rstest]
        fn new_creates_floor_item() {
            let item = "Sword";
            let position = Position::new(5, 5);
            let floor_item = FloorItem::new(item, position);

            assert_eq!(floor_item.item(), &"Sword");
            assert_eq!(floor_item.position(), position);
        }

        #[rstest]
        fn into_item_consumes_floor_item() {
            let item = String::from("Potion");
            let floor_item = FloorItem::new(item, Position::new(0, 0));
            let extracted = floor_item.into_item();

            assert_eq!(extracted, "Potion");
        }
    }

    // =========================================================================
    // Find Item On Floor Tests
    // =========================================================================

    mod find_item_on_floor_tests {
        use super::*;

        struct TestItem {
            identifier: ItemIdentifier,
        }

        #[rstest]
        fn finds_item_at_player_position() {
            let target_id = ItemIdentifier::new();
            let player_pos = Position::new(5, 5);
            let floor_items = vec![
                FloorItem::new(
                    TestItem {
                        identifier: ItemIdentifier::new(),
                    },
                    Position::new(0, 0),
                ),
                FloorItem::new(
                    TestItem {
                        identifier: target_id,
                    },
                    player_pos,
                ),
            ];

            let result = find_item_on_floor(&floor_items, &target_id, player_pos, |item| {
                &item.identifier
            });

            assert_eq!(result, Ok(1));
        }

        #[rstest]
        fn returns_error_if_item_at_different_position() {
            let target_id = ItemIdentifier::new();
            let player_pos = Position::new(5, 5);
            let floor_items = vec![FloorItem::new(
                TestItem {
                    identifier: target_id,
                },
                Position::new(10, 10),
            )];

            let result = find_item_on_floor(&floor_items, &target_id, player_pos, |item| {
                &item.identifier
            });

            assert!(matches!(
                result,
                Err(PlayerError::ItemNotInInventory { .. })
            ));
        }

        #[rstest]
        fn returns_error_if_item_not_found() {
            let target_id = ItemIdentifier::new();
            let player_pos = Position::new(5, 5);
            let floor_items: Vec<FloorItem<TestItem>> = vec![];

            let result = find_item_on_floor(&floor_items, &target_id, player_pos, |item| {
                &item.identifier
            });

            assert!(matches!(
                result,
                Err(PlayerError::ItemNotInInventory { .. })
            ));
        }
    }

    // =========================================================================
    // Validate Inventory Space Tests
    // =========================================================================

    mod validate_inventory_space_tests {
        use super::*;

        #[rstest]
        fn has_space_returns_ok() {
            let result = validate_inventory_space(10, 20);
            assert!(result.is_ok());
        }

        #[rstest]
        fn full_inventory_returns_error() {
            let result = validate_inventory_space(20, 20);
            assert!(matches!(result, Err(PlayerError::InventoryFull { .. })));
        }

        #[rstest]
        fn empty_inventory_returns_ok() {
            let result = validate_inventory_space(0, 20);
            assert!(result.is_ok());
        }

        #[rstest]
        fn one_slot_remaining_returns_ok() {
            let result = validate_inventory_space(19, 20);
            assert!(result.is_ok());
        }
    }

    // =========================================================================
    // Remove Item At Index Tests
    // =========================================================================

    mod remove_item_at_index_tests {
        use super::*;

        #[rstest]
        fn removes_item_at_beginning() {
            let items = vec![1, 2, 3, 4];
            let result = remove_item_at_index(&items, 0);
            assert_eq!(result, vec![2, 3, 4]);
        }

        #[rstest]
        fn removes_item_at_end() {
            let items = vec![1, 2, 3, 4];
            let result = remove_item_at_index(&items, 3);
            assert_eq!(result, vec![1, 2, 3]);
        }

        #[rstest]
        fn removes_item_in_middle() {
            let items = vec![1, 2, 3, 4];
            let result = remove_item_at_index(&items, 2);
            assert_eq!(result, vec![1, 2, 4]);
        }

        #[rstest]
        fn single_item_list_becomes_empty() {
            let items = vec![1];
            let result = remove_item_at_index(&items, 0);
            assert!(result.is_empty());
        }

        #[rstest]
        fn original_list_unchanged() {
            let items = vec![1, 2, 3];
            let _ = remove_item_at_index(&items, 1);
            assert_eq!(items, vec![1, 2, 3]);
        }
    }

    // =========================================================================
    // Add Item To List Tests
    // =========================================================================

    mod add_item_to_list_tests {
        use super::*;

        #[rstest]
        fn adds_item_to_empty_list() {
            let items: Vec<i32> = vec![];
            let result = add_item_to_list(&items, 1);
            assert_eq!(result, vec![1]);
        }

        #[rstest]
        fn adds_item_to_existing_list() {
            let items = vec![1, 2, 3];
            let result = add_item_to_list(&items, 4);
            assert_eq!(result, vec![1, 2, 3, 4]);
        }

        #[rstest]
        fn original_list_unchanged() {
            let items = vec![1, 2, 3];
            let _ = add_item_to_list(&items, 4);
            assert_eq!(items, vec![1, 2, 3]);
        }
    }
}
