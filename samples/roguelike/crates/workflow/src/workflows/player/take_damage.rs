//! TakeDamage workflow implementation.
//!
//! This module provides the workflow for the player taking damage.
//! It follows the "IO at the Edges" pattern, separating pure domain logic
//! from IO operations.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Calculate damage reduction (using Semigroup for modifiers)
//! 3. [Pure] Apply damage to player health
//! 4. [Pure] Check if player died
//! 5. [Pure] Generate PlayerDamaged event
//! 6. [Pure] Generate PlayerDied event if applicable
//! 7. [IO] Update cache
//! 8. [IO] Append events to event store
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::player::{take_damage, TakeDamageCommand};
//! use roguelike_domain::common::Damage;
//!
//! let workflow = take_damage(&cache, &event_store);
//! let command = TakeDamageCommand::new(game_identifier, source, Damage::new(10));
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
use lambars::typeclass::Monoid;
use roguelike_domain::common::{Damage, DamageModifier, Defense};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameSessionEvent;
use roguelike_domain::player::{EntityIdentifier as PlayerEntityIdentifier, PlayerDamaged};

use super::TakeDamageCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// TakeDamage Workflow
// =============================================================================

/// Creates a workflow function for taking damage.
///
/// This function returns a closure that applies damage to the player.
/// It uses higher-order functions to inject dependencies, enabling pure
/// functional composition and easy testing.
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
///
/// # Returns
///
/// A function that takes a `TakeDamageCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::workflows::player::{take_damage, TakeDamageCommand};
/// use roguelike_domain::common::Damage;
///
/// let workflow = take_damage(&cache, &event_store);
/// let command = TakeDamageCommand::new(game_identifier, source, Damage::new(10));
/// let result = workflow(command).run_async().await;
/// ```
pub fn take_damage<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(TakeDamageCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();
        let source = *command.source();
        let base_damage = command.base_damage();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Step 2-6: [Pure] Calculate damage, apply, check death, generate events
                    // Note: In a real implementation, these values would be extracted from the session.
                    let defense = Defense::new(5);
                    let modifiers: Vec<DamageModifier> = vec![];
                    let current_health = 100u32;

                    let result = take_damage_pure(
                        session.clone(),
                        &source,
                        base_damage,
                        defense,
                        &modifiers,
                        current_health,
                    );

                    match result {
                        Ok((updated_session, events, _damage_result)) => {
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

/// Pure function that performs the take damage logic using `pipe!` macro.
///
/// This function encapsulates all pure domain logic for taking damage:
/// - Calculates damage reduction (using Monoid for defense modifiers)
/// - Applies damage to health
/// - Checks for death
/// - Generates events
///
/// # Arguments
///
/// * `session` - The current game session
/// * `source` - The source of the damage
/// * `base_damage` - The base damage amount
/// * `defense` - The player's defense stat
/// * `modifiers` - Damage reduction modifiers
/// * `current_health` - The player's current health
///
/// # Returns
///
/// A result containing the updated session, events, and take damage result.
///
/// # Example
///
/// ```ignore
/// let result = take_damage_pure(
///     session,
///     &source_id,
///     Damage::new(20),
///     Defense::new(5),
///     &modifiers,
///     100,
/// );
/// ```
pub fn take_damage_pure<S>(
    session: S,
    source: &EntityIdentifier,
    base_damage: Damage,
    defense: Defense,
    modifiers: &[DamageModifier],
    current_health: u32,
) -> Result<(S, Vec<GameSessionEvent>, TakeDamageResult), WorkflowError>
where
    S: Clone,
{
    // [Pure] Damage reduction pipeline using pipe! with Monoid semantics
    let result = pipe!(
        base_damage,
        // Step 1: Calculate damage after defense and modifiers (using Monoid)
        |damage| calculate_damage_taken(damage, defense, modifiers),
        // Step 2: Apply damage to health
        |final_damage| {
            let new_health = apply_damage_to_health(current_health, final_damage);
            (final_damage, new_health)
        },
        // Step 3: Check for player death and create result
        |(final_damage, new_health)| {
            let died = is_player_dead(new_health);
            TakeDamageResult::new(final_damage, new_health, died)
        }
    );

    // Generate events based on result
    let _event = create_player_damaged_event(source, result.damage_taken());
    // Note: GameSessionEvent does not have a Player variant yet.
    // Events are returned as an empty vector for now.
    let events: Vec<GameSessionEvent> = vec![];

    Ok((session, events, result))
}

/// Calculates the final damage after defense and modifiers.
///
/// Uses Monoid semantics to combine damage reduction modifiers.
/// The formula is: `base_damage - defense` (minimum 0), then apply modifiers.
///
/// # Arguments
///
/// * `base_damage` - The incoming damage amount
/// * `defense` - The player's defense stat
/// * `modifiers` - A slice of damage modifiers (e.g., armor, buffs)
///
/// # Returns
///
/// The final damage amount after all reductions.
#[must_use]
pub fn calculate_damage_taken(
    base_damage: Damage,
    defense: Defense,
    modifiers: &[DamageModifier],
) -> Damage {
    // Apply defense reduction
    let after_defense = base_damage.value().saturating_sub(defense.value());

    // Apply modifiers using Monoid semantics
    let combined_modifier = combine_damage_reduction_modifiers(modifiers);
    combined_modifier.apply(Damage::new(after_defense))
}

/// Combines multiple damage reduction modifiers using Monoid semantics.
///
/// # Arguments
///
/// * `modifiers` - A slice of damage modifiers to combine
///
/// # Returns
///
/// A combined damage modifier (identity if slice is empty).
#[must_use]
pub fn combine_damage_reduction_modifiers(modifiers: &[DamageModifier]) -> DamageModifier {
    DamageModifier::combine_all(modifiers.iter().copied())
}

/// Applies damage to the player's health.
///
/// This is a pure function that calculates the new health value.
///
/// # Arguments
///
/// * `current_health` - The player's current health
/// * `damage` - The damage to apply
///
/// # Returns
///
/// The new health value (minimum 0).
#[must_use]
pub fn apply_damage_to_health(current_health: u32, damage: Damage) -> u32 {
    current_health.saturating_sub(damage.value())
}

/// Checks if the player has died (health is 0).
///
/// # Arguments
///
/// * `health` - The player's current health
///
/// # Returns
///
/// `true` if health is 0, `false` otherwise.
#[must_use]
pub const fn is_player_dead(health: u32) -> bool {
    health == 0
}

/// Represents the result of taking damage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeDamageResult {
    /// The damage that was actually taken (after reductions).
    damage_taken: Damage,
    /// The player's health after taking damage.
    remaining_health: u32,
    /// Whether the player died from this damage.
    player_died: bool,
}

impl TakeDamageResult {
    /// Creates a new take damage result.
    #[must_use]
    pub const fn new(damage_taken: Damage, remaining_health: u32, player_died: bool) -> Self {
        Self {
            damage_taken,
            remaining_health,
            player_died,
        }
    }

    /// Returns the damage that was taken.
    #[must_use]
    pub const fn damage_taken(&self) -> Damage {
        self.damage_taken
    }

    /// Returns the remaining health.
    #[must_use]
    pub const fn remaining_health(&self) -> u32 {
        self.remaining_health
    }

    /// Returns whether the player died.
    #[must_use]
    pub const fn player_died(&self) -> bool {
        self.player_died
    }
}

/// Performs the full damage calculation and application.
///
/// This is a pure function that combines all damage logic.
///
/// # Arguments
///
/// * `current_health` - The player's current health
/// * `base_damage` - The incoming damage
/// * `defense` - The player's defense stat
/// * `modifiers` - Damage reduction modifiers
///
/// # Returns
///
/// A `TakeDamageResult` with all the damage details.
#[must_use]
pub fn perform_take_damage(
    current_health: u32,
    base_damage: Damage,
    defense: Defense,
    modifiers: &[DamageModifier],
) -> TakeDamageResult {
    let final_damage = calculate_damage_taken(base_damage, defense, modifiers);
    let new_health = apply_damage_to_health(current_health, final_damage);
    let died = is_player_dead(new_health);

    TakeDamageResult::new(final_damage, new_health, died)
}

/// Creates a PlayerDamaged event.
///
/// # Arguments
///
/// * `source` - The entity that caused the damage
/// * `damage` - The damage amount
///
/// # Returns
///
/// A `PlayerDamaged` event.
#[must_use]
pub fn create_player_damaged_event(source: &EntityIdentifier, damage: Damage) -> PlayerDamaged {
    PlayerDamaged::new(PlayerEntityIdentifier::new(source.to_string()), damage)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Calculate Damage Taken Tests
    // =========================================================================

    mod calculate_damage_taken_tests {
        use super::*;

        #[rstest]
        fn basic_damage_reduction() {
            let result = calculate_damage_taken(Damage::new(20), Defense::new(5), &[]);
            assert_eq!(result.value(), 15);
        }

        #[rstest]
        fn defense_exceeds_damage() {
            let result = calculate_damage_taken(Damage::new(10), Defense::new(20), &[]);
            assert_eq!(result.value(), 0);
        }

        #[rstest]
        fn zero_defense() {
            let result = calculate_damage_taken(Damage::new(10), Defense::new(0), &[]);
            assert_eq!(result.value(), 10);
        }

        #[rstest]
        fn zero_damage() {
            let result = calculate_damage_taken(Damage::new(0), Defense::new(10), &[]);
            assert_eq!(result.value(), 0);
        }

        #[rstest]
        fn damage_with_reduction_modifier() {
            let modifiers = vec![DamageModifier::new(0.5, 0)]; // 50% multiplier
            let result = calculate_damage_taken(Damage::new(20), Defense::new(0), &modifiers);
            assert_eq!(result.value(), 10);
        }
    }

    // =========================================================================
    // Apply Damage To Health Tests
    // =========================================================================

    mod apply_damage_to_health_tests {
        use super::*;

        #[rstest]
        fn reduces_health() {
            let result = apply_damage_to_health(100, Damage::new(30));
            assert_eq!(result, 70);
        }

        #[rstest]
        fn damage_exceeds_health() {
            let result = apply_damage_to_health(50, Damage::new(100));
            assert_eq!(result, 0);
        }

        #[rstest]
        fn exact_damage() {
            let result = apply_damage_to_health(50, Damage::new(50));
            assert_eq!(result, 0);
        }

        #[rstest]
        fn zero_damage() {
            let result = apply_damage_to_health(100, Damage::new(0));
            assert_eq!(result, 100);
        }

        #[rstest]
        fn already_at_zero() {
            let result = apply_damage_to_health(0, Damage::new(10));
            assert_eq!(result, 0);
        }
    }

    // =========================================================================
    // Is Player Dead Tests
    // =========================================================================

    mod is_player_dead_tests {
        use super::*;

        #[rstest]
        fn zero_health_is_dead() {
            assert!(is_player_dead(0));
        }

        #[rstest]
        fn one_health_is_alive() {
            assert!(!is_player_dead(1));
        }

        #[rstest]
        fn full_health_is_alive() {
            assert!(!is_player_dead(100));
        }
    }

    // =========================================================================
    // TakeDamageResult Tests
    // =========================================================================

    mod take_damage_result_tests {
        use super::*;

        #[rstest]
        fn new_creates_result() {
            let result = TakeDamageResult::new(Damage::new(10), 90, false);

            assert_eq!(result.damage_taken(), Damage::new(10));
            assert_eq!(result.remaining_health(), 90);
            assert!(!result.player_died());
        }

        #[rstest]
        fn death_result() {
            let result = TakeDamageResult::new(Damage::new(100), 0, true);

            assert_eq!(result.remaining_health(), 0);
            assert!(result.player_died());
        }
    }

    // =========================================================================
    // Perform Take Damage Tests
    // =========================================================================

    mod perform_take_damage_tests {
        use super::*;

        #[rstest]
        fn basic_damage() {
            let result = perform_take_damage(100, Damage::new(30), Defense::new(10), &[]);

            assert_eq!(result.damage_taken(), Damage::new(20));
            assert_eq!(result.remaining_health(), 80);
            assert!(!result.player_died());
        }

        #[rstest]
        fn fatal_damage() {
            let result = perform_take_damage(50, Damage::new(100), Defense::new(0), &[]);

            assert_eq!(result.remaining_health(), 0);
            assert!(result.player_died());
        }

        #[rstest]
        fn barely_survives() {
            let result = perform_take_damage(100, Damage::new(99), Defense::new(0), &[]);

            assert_eq!(result.remaining_health(), 1);
            assert!(!result.player_died());
        }

        #[rstest]
        fn exactly_lethal() {
            let result = perform_take_damage(50, Damage::new(60), Defense::new(10), &[]);

            assert_eq!(result.remaining_health(), 0);
            assert!(result.player_died());
        }

        #[rstest]
        fn with_damage_reduction_modifier() {
            // 50% damage multiplier
            let modifiers = vec![DamageModifier::new(0.5, 0)];
            let result = perform_take_damage(100, Damage::new(40), Defense::new(0), &modifiers);

            assert_eq!(result.damage_taken(), Damage::new(20));
            assert_eq!(result.remaining_health(), 80);
        }
    }

    // =========================================================================
    // Create Player Damaged Event Tests
    // =========================================================================

    mod create_player_damaged_event_tests {
        use super::*;

        #[rstest]
        fn creates_event_with_source_and_damage() {
            let source = EntityIdentifier::new();
            let damage = Damage::new(15);

            let event = create_player_damaged_event(&source, damage);

            assert_eq!(event.source().value(), source.to_string().as_str());
            assert_eq!(event.damage(), damage);
        }
    }
}
