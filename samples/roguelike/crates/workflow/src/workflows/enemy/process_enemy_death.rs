//! ProcessEnemyDeath workflow implementation.
//!
//! This module provides the workflow for processing an enemy's death.
//! It follows the "IO at the Edges" pattern, separating pure domain logic
//! from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Find enemy by identifier
//! 3. [Pure] Calculate loot based on enemy type
//! 4. [Pure] Drop items at enemy position
//! 5. [Pure] Remove enemy from session
//! 6. [Pure] Generate EnemyDied event
//! 7. [IO] Update cache
//! 8. [IO] Append events to event store
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::enemy::{process_enemy_death, ProcessEnemyDeathCommand};
//!
//! let workflow = process_enemy_death(&cache, &event_store, cache_ttl);
//! let command = ProcessEnemyDeathCommand::new(game_identifier, entity_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::Position;
use roguelike_domain::enemy::{EnemyDied, EnemyType, EntityIdentifier, LootEntry, LootTable};
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::item::ItemIdentifier;

use super::ProcessEnemyDeathCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// DroppedItem
// =============================================================================

/// Represents an item dropped on the floor.
///
/// This structure contains information about an item that was
/// dropped when an enemy died.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedItem {
    /// The item identifier.
    item_identifier: ItemIdentifier,
    /// The position where the item was dropped.
    position: Position,
    /// The quantity of items dropped.
    quantity: u32,
}

impl DroppedItem {
    /// Creates a new dropped item.
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier, position: Position, quantity: u32) -> Self {
        Self {
            item_identifier,
            position,
            quantity,
        }
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    /// Returns the drop position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the quantity.
    #[must_use]
    pub const fn quantity(&self) -> u32 {
        self.quantity
    }
}

// =============================================================================
// ProcessEnemyDeath Workflow
// =============================================================================

/// Creates a workflow function for processing an enemy's death.
///
/// This function returns a closure that handles loot generation and
/// enemy removal. It uses higher-order functions to inject dependencies,
/// enabling pure functional composition and easy testing.
///
/// # Type Parameters
///
/// * `C` - Cache type implementing `SessionCache`
/// * `E` - Event store type implementing `EventStore`
///
/// # Arguments
///
/// * `cache` - The session cache for fast access
/// * `event_store` - The event store for event sourcing
/// * `cache_ttl` - Time-to-live for cached sessions
///
/// # Returns
///
/// A function that takes a `ProcessEnemyDeathCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
pub fn process_enemy_death<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(ProcessEnemyDeathCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let entity_identifier = *command.entity_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Steps 2-6: [Pure] Process enemy death
                    let result = process_enemy_death_pure(&session, entity_identifier);

                    match result {
                        Ok((updated_session, events)) => {
                            // Steps 7-8: [IO] Update cache and append events
                            let game_identifier_clone = game_identifier;
                            let updated_session_clone = updated_session.clone();

                            cache
                                .set(&game_identifier_clone, &updated_session, cache_ttl)
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

/// Creates a workflow function with default cache TTL.
pub fn process_enemy_death_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(ProcessEnemyDeathCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    process_enemy_death(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire enemy death processing logic.
fn process_enemy_death_pure<S: Clone>(
    _session: &S,
    _entity_identifier: EntityIdentifier,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "process_enemy_death",
        "GameSession structure not yet connected",
    ))
}

/// Calculates the loot table for a defeated enemy.
///
/// This is a pure function that determines what items an enemy
/// may drop based on its type.
///
/// # Arguments
///
/// * `enemy_type` - The type of the defeated enemy
/// * `seed` - Random seed for loot determination
///
/// # Returns
///
/// A loot table containing potential drops.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::calculate_loot;
/// use roguelike_domain::enemy::EnemyType;
///
/// let loot_table = calculate_loot(EnemyType::Goblin, 12345);
/// // Goblins may drop items
/// ```
#[must_use]
pub fn calculate_loot(enemy_type: EnemyType, seed: u64) -> LootTable {
    let base_loot = get_base_loot_table(enemy_type);

    // Use seed to determine which items actually drop
    filter_loot_by_chance(&base_loot, seed)
}

/// Gets the base loot table for an enemy type.
fn get_base_loot_table(enemy_type: EnemyType) -> LootTable {
    match enemy_type {
        EnemyType::Slime => create_loot_table(&[
            (0.3, 1, 2), // 30% chance, 1-2 items
        ]),
        EnemyType::Bat => create_loot_table(&[(0.2, 1, 1)]),
        EnemyType::Goblin => create_loot_table(&[(0.5, 1, 3)]),
        EnemyType::Spider => create_loot_table(&[(0.4, 1, 2)]),
        EnemyType::Skeleton => create_loot_table(&[(0.6, 1, 2)]),
        EnemyType::Zombie => create_loot_table(&[(0.5, 1, 3)]),
        EnemyType::Orc => create_loot_table(&[(0.7, 1, 3)]),
        EnemyType::Ghost => create_loot_table(&[(0.4, 1, 1)]),
        EnemyType::Minotaur => create_loot_table(&[
            (1.0, 2, 5), // Boss - guaranteed loot
        ]),
        EnemyType::Dragon => create_loot_table(&[
            (1.0, 3, 7), // Boss - guaranteed loot
        ]),
    }
}

/// Creates a loot table from drop specifications.
fn create_loot_table(specs: &[(f32, u32, u32)]) -> LootTable {
    specs
        .iter()
        .fold(LootTable::empty(), |table, (drop_rate, min, max)| {
            // Create a new item identifier for each entry
            // In a real implementation, this would reference actual item types
            if let Ok(entry) = LootEntry::new(ItemIdentifier::new(), *drop_rate, *min, *max) {
                table.with_entry(entry)
            } else {
                table
            }
        })
}

/// Filters a loot table based on drop chances using a seed.
fn filter_loot_by_chance(loot_table: &LootTable, seed: u64) -> LootTable {
    let mut current_seed = seed;

    loot_table.iter().fold(LootTable::empty(), |table, entry| {
        // Generate random value for this entry
        current_seed = current_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let roll = ((current_seed >> 32) as f32) / (u32::MAX as f32);

        if roll <= entry.drop_rate() {
            table.with_entry(*entry)
        } else {
            table
        }
    })
}

/// Drops items at a specific position on the floor.
///
/// This is a pure function that creates dropped item instances
/// from a loot table.
///
/// # Arguments
///
/// * `loot_table` - The loot table containing items to drop
/// * `position` - The position where items should be dropped
/// * `seed` - Random seed for quantity determination
///
/// # Returns
///
/// A vector of dropped items.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::{calculate_loot, drop_items_at_position};
/// use roguelike_domain::enemy::EnemyType;
/// use roguelike_domain::common::Position;
///
/// let loot_table = calculate_loot(EnemyType::Goblin, 12345);
/// let dropped = drop_items_at_position(&loot_table, Position::new(10, 10), 54321);
/// ```
#[must_use]
pub fn drop_items_at_position(
    loot_table: &LootTable,
    position: Position,
    seed: u64,
) -> Vec<DroppedItem> {
    let mut current_seed = seed;

    loot_table
        .iter()
        .map(|entry| {
            // Determine quantity
            let quantity = if entry.has_fixed_quantity() {
                entry.min_quantity()
            } else {
                current_seed = current_seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1);
                let range = entry.max_quantity() - entry.min_quantity() + 1;
                let random_offset = ((current_seed >> 32) as u32) % range;
                entry.min_quantity() + random_offset
            };

            DroppedItem::new(entry.item_identifier(), position, quantity)
        })
        .collect()
}

/// Removes an enemy from the session.
///
/// This is a pure function that creates an updated session
/// without the specified enemy.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to remove an enemy from the session
///
/// # Arguments
///
/// * `session` - The current game session
/// * `entity_identifier` - The identifier of the enemy to remove
/// * `death_position` - The position where the enemy died
/// * `loot_table` - The loot table for the enemy
/// * `remove_enemy` - Function that removes an enemy from the session
///
/// # Returns
///
/// A tuple of (updated_session, death_event).
///
/// # Examples
///
/// ```ignore
/// let (updated_session, event) = remove_enemy_from_session(
///     &session,
///     entity_identifier,
///     Position::new(10, 20),
///     loot_table,
///     |s, id| s.without_enemy(id),
/// );
/// ```
pub fn remove_enemy_from_session<S, F>(
    session: &S,
    entity_identifier: EntityIdentifier,
    death_position: Position,
    loot_table: &LootTable,
    remove_enemy: F,
) -> (S, GameSessionEvent)
where
    S: Clone,
    F: Fn(&S, EntityIdentifier) -> S,
{
    let updated_session = remove_enemy(session, entity_identifier);
    let event = EnemyDied::from_loot_table(entity_identifier, death_position, loot_table);

    (updated_session, GameSessionEvent::EnemyDied(event))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // DroppedItem Tests
    // =========================================================================

    mod dropped_item {
        use super::*;

        #[rstest]
        fn new_creates_dropped_item() {
            let item_id = ItemIdentifier::new();
            let position = Position::new(10, 20);
            let dropped = DroppedItem::new(item_id, position, 3);

            assert_eq!(dropped.item_identifier(), item_id);
            assert_eq!(dropped.position(), position);
            assert_eq!(dropped.quantity(), 3);
        }

        #[rstest]
        fn clone_preserves_values() {
            let item_id = ItemIdentifier::new();
            let position = Position::new(15, 25);
            let dropped = DroppedItem::new(item_id, position, 5);
            let cloned = dropped.clone();

            assert_eq!(dropped.item_identifier(), cloned.item_identifier());
            assert_eq!(dropped.position(), cloned.position());
            assert_eq!(dropped.quantity(), cloned.quantity());
        }

        #[rstest]
        fn equality() {
            let item_id = ItemIdentifier::new();
            let position = Position::new(10, 10);
            let dropped1 = DroppedItem::new(item_id, position, 2);
            let dropped2 = DroppedItem::new(item_id, position, 2);

            assert_eq!(dropped1, dropped2);
        }

        #[rstest]
        fn inequality_different_quantity() {
            let item_id = ItemIdentifier::new();
            let position = Position::new(10, 10);
            let dropped1 = DroppedItem::new(item_id, position, 2);
            let dropped2 = DroppedItem::new(item_id, position, 5);

            assert_ne!(dropped1, dropped2);
        }
    }

    // =========================================================================
    // calculate_loot Tests
    // =========================================================================

    mod calculate_loot_tests {
        use super::*;

        #[rstest]
        fn boss_enemies_have_guaranteed_loot() {
            // Test with multiple seeds to ensure guaranteed loot
            for seed in [12345u64, 54321, 99999, 11111] {
                let loot = calculate_loot(EnemyType::Dragon, seed);
                assert!(!loot.is_empty(), "Dragon should always drop loot");
            }
        }

        #[rstest]
        fn minotaur_has_guaranteed_loot() {
            for seed in [12345u64, 54321, 99999, 11111] {
                let loot = calculate_loot(EnemyType::Minotaur, seed);
                assert!(!loot.is_empty(), "Minotaur should always drop loot");
            }
        }

        #[rstest]
        fn same_seed_produces_same_loot() {
            let loot1 = calculate_loot(EnemyType::Goblin, 12345);
            let loot2 = calculate_loot(EnemyType::Goblin, 12345);

            assert_eq!(loot1.len(), loot2.len());
        }

        #[rstest]
        fn different_enemy_types_have_different_base_tables() {
            let slime_base = get_base_loot_table(EnemyType::Slime);
            let dragon_base = get_base_loot_table(EnemyType::Dragon);

            // Dragon should have higher drop rates
            assert!(dragon_base.total_drop_rate() > slime_base.total_drop_rate());
        }
    }

    // =========================================================================
    // drop_items_at_position Tests
    // =========================================================================

    mod drop_items_at_position_tests {
        use super::*;

        #[rstest]
        fn drops_items_at_correct_position() {
            let item_id = ItemIdentifier::new();
            let entry = LootEntry::new(item_id, 1.0, 1, 3).unwrap();
            let loot_table = LootTable::empty().with_entry(entry);
            let position = Position::new(15, 25);

            let dropped = drop_items_at_position(&loot_table, position, 12345);

            assert!(!dropped.is_empty());
            for item in &dropped {
                assert_eq!(item.position(), position);
            }
        }

        #[rstest]
        fn respects_quantity_range() {
            let item_id = ItemIdentifier::new();
            let entry = LootEntry::new(item_id, 1.0, 2, 5).unwrap();
            let loot_table = LootTable::empty().with_entry(entry);
            let position = Position::new(10, 10);

            // Test with multiple seeds
            for seed in [12345u64, 54321, 99999] {
                let dropped = drop_items_at_position(&loot_table, position, seed);

                for item in &dropped {
                    assert!(item.quantity() >= 2);
                    assert!(item.quantity() <= 5);
                }
            }
        }

        #[rstest]
        fn fixed_quantity_always_drops_exact_amount() {
            let item_id = ItemIdentifier::new();
            let entry = LootEntry::new(item_id, 1.0, 3, 3).unwrap(); // Fixed quantity
            let loot_table = LootTable::empty().with_entry(entry);
            let position = Position::new(10, 10);

            for seed in [12345u64, 54321, 99999] {
                let dropped = drop_items_at_position(&loot_table, position, seed);

                for item in &dropped {
                    assert_eq!(item.quantity(), 3);
                }
            }
        }

        #[rstest]
        fn empty_loot_table_drops_nothing() {
            let loot_table = LootTable::empty();
            let position = Position::new(10, 10);

            let dropped = drop_items_at_position(&loot_table, position, 12345);

            assert!(dropped.is_empty());
        }

        #[rstest]
        fn same_seed_produces_same_quantities() {
            let item_id = ItemIdentifier::new();
            let entry = LootEntry::new(item_id, 1.0, 1, 10).unwrap();
            let loot_table = LootTable::empty().with_entry(entry);
            let position = Position::new(10, 10);

            let dropped1 = drop_items_at_position(&loot_table, position, 12345);
            let dropped2 = drop_items_at_position(&loot_table, position, 12345);

            assert_eq!(dropped1.len(), dropped2.len());
            for (item1, item2) in dropped1.iter().zip(dropped2.iter()) {
                assert_eq!(item1.quantity(), item2.quantity());
            }
        }
    }

    // =========================================================================
    // remove_enemy_from_session Tests
    // =========================================================================

    mod remove_enemy_from_session_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            enemy_count: usize,
        }

        impl MockSession {
            fn new(enemy_count: usize) -> Self {
                Self { enemy_count }
            }

            fn without_enemy(&self) -> Self {
                Self {
                    enemy_count: self.enemy_count.saturating_sub(1),
                }
            }
        }

        #[rstest]
        fn removes_enemy_from_session() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(10, 20);
            let loot_table = LootTable::empty();

            let (updated_session, _) = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            assert_eq!(updated_session.enemy_count, 4);
        }

        #[rstest]
        fn generates_enemy_died_event() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(10, 20);
            let loot_table = LootTable::empty();

            let (_, event) = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            assert!(matches!(event, GameSessionEvent::EnemyDied(_)));
        }

        #[rstest]
        fn event_contains_correct_enemy_identifier() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(10, 20);
            let loot_table = LootTable::empty();

            let (_, event) = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            if let GameSessionEvent::EnemyDied(died_event) = event {
                assert_eq!(died_event.enemy_identifier(), entity_identifier);
            } else {
                panic!("Expected EnemyDied event");
            }
        }

        #[rstest]
        fn event_contains_loot_entry_count() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(10, 20);
            let item_id = ItemIdentifier::new();
            let entry = LootEntry::new(item_id, 1.0, 1, 1).unwrap();
            let loot_table = LootTable::empty().with_entry(entry);

            let (_, event) = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            if let GameSessionEvent::EnemyDied(died_event) = event {
                assert!(died_event.has_loot());
                assert_eq!(died_event.loot_entry_count(), 1);
            } else {
                panic!("Expected EnemyDied event");
            }
        }

        #[rstest]
        fn event_contains_death_position() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(15, 25);
            let loot_table = LootTable::empty();

            let (_, event) = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            if let GameSessionEvent::EnemyDied(died_event) = event {
                assert_eq!(died_event.death_position(), death_position);
            } else {
                panic!("Expected EnemyDied event");
            }
        }

        #[rstest]
        fn original_session_unchanged() {
            let session = MockSession::new(5);
            let entity_identifier = EntityIdentifier::new();
            let death_position = Position::new(10, 20);
            let loot_table = LootTable::empty();

            let _ = remove_enemy_from_session(
                &session,
                entity_identifier,
                death_position,
                &loot_table,
                |s, _| s.without_enemy(),
            );

            // Original session should be unchanged
            assert_eq!(session.enemy_count, 5);
        }
    }
}
