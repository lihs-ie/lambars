//! TriggerTrap workflow implementation.
//!
//! This module provides the workflow for triggering traps when an entity
//! steps on a trap tile. It handles damage calculation, status effects,
//! and trap disarming.
//!
//! # Workflow Steps
//!
//! 1. [IO] Load session from cache
//! 2. [Pure] Find trap at position
//! 3. [Pure] Calculate trap effect
//! 4. [Pure] Apply trap effect to target
//! 5. [Pure] Disarm trap (if one-time trap)
//! 6. [Pure] Generate TrapTriggered event
//! 7. [IO] Update cache
//! 8. [IO] Append events to event store
//!
//! # Trap Types
//!
//! - **Spike**: Deals physical damage (one-time use)
//! - **Poison**: Applies poison status effect (reusable)
//! - **Teleport**: Teleports target to random location (reusable)
//! - **Alarm**: Alerts nearby enemies (reusable)
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::floor::{trigger_trap, TriggerTrapCommand};
//!
//! let workflow = trigger_trap(&cache, &event_store, cache_ttl);
//! let command = TriggerTrapCommand::new(game_identifier, position, target);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::{Damage, Position, StatusEffect, StatusEffectType};
use roguelike_domain::floor::{FloorError, TrapType};
use roguelike_domain::game_session::GameSessionEvent;

use super::TriggerTrapCommand;
use crate::errors::WorkflowError;
use crate::ports::{EventStore, SessionCache, WorkflowResult};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// TrapEffect
// =============================================================================

/// Represents the effect of a triggered trap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrapEffect {
    /// Direct damage dealt by the trap.
    damage: Damage,
    /// Status effect applied by the trap.
    status_effect: Option<StatusEffect>,
    /// Whether the trap teleports the target.
    teleport_destination: Option<Position>,
    /// Whether the trap alerts nearby enemies.
    alerts_enemies: bool,
    /// Whether the trap should be disarmed after triggering.
    should_disarm: bool,
}

impl TrapEffect {
    /// Creates a new trap effect.
    #[must_use]
    pub const fn new(
        damage: Damage,
        status_effect: Option<StatusEffect>,
        teleport_destination: Option<Position>,
        alerts_enemies: bool,
        should_disarm: bool,
    ) -> Self {
        Self {
            damage,
            status_effect,
            teleport_destination,
            alerts_enemies,
            should_disarm,
        }
    }

    /// Creates a damage-only trap effect.
    #[must_use]
    pub const fn damage_only(damage: Damage, should_disarm: bool) -> Self {
        Self {
            damage,
            status_effect: None,
            teleport_destination: None,
            alerts_enemies: false,
            should_disarm,
        }
    }

    /// Creates a status effect trap.
    #[must_use]
    pub const fn with_status(status_effect: StatusEffect, should_disarm: bool) -> Self {
        Self {
            damage: Damage::zero(),
            status_effect: Some(status_effect),
            teleport_destination: None,
            alerts_enemies: false,
            should_disarm,
        }
    }

    /// Creates a teleport trap effect.
    #[must_use]
    pub const fn teleport(destination: Position, should_disarm: bool) -> Self {
        Self {
            damage: Damage::zero(),
            status_effect: None,
            teleport_destination: Some(destination),
            alerts_enemies: false,
            should_disarm,
        }
    }

    /// Creates an alarm trap effect.
    #[must_use]
    pub const fn alarm(should_disarm: bool) -> Self {
        Self {
            damage: Damage::zero(),
            status_effect: None,
            teleport_destination: None,
            alerts_enemies: true,
            should_disarm,
        }
    }

    /// Returns the damage dealt.
    #[must_use]
    pub const fn damage(&self) -> Damage {
        self.damage
    }

    /// Returns the status effect.
    #[must_use]
    pub const fn status_effect(&self) -> Option<StatusEffect> {
        self.status_effect
    }

    /// Returns the teleport destination.
    #[must_use]
    pub const fn teleport_destination(&self) -> Option<Position> {
        self.teleport_destination
    }

    /// Returns whether the trap alerts enemies.
    #[must_use]
    pub const fn alerts_enemies(&self) -> bool {
        self.alerts_enemies
    }

    /// Returns whether the trap should be disarmed.
    #[must_use]
    pub const fn should_disarm(&self) -> bool {
        self.should_disarm
    }
}

// =============================================================================
// TrapInfo
// =============================================================================

/// Information about a trap at a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapInfo {
    /// Position of the trap.
    position: Position,
    /// Type of the trap.
    trap_type: TrapType,
    /// Whether the trap is hidden.
    is_hidden: bool,
    /// Whether the trap is active (not disarmed).
    is_active: bool,
}

impl TrapInfo {
    /// Creates a new trap info.
    #[must_use]
    pub const fn new(
        position: Position,
        trap_type: TrapType,
        is_hidden: bool,
        is_active: bool,
    ) -> Self {
        Self {
            position,
            trap_type,
            is_hidden,
            is_active,
        }
    }

    /// Returns the trap position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the trap type.
    #[must_use]
    pub const fn trap_type(&self) -> TrapType {
        self.trap_type
    }

    /// Returns whether the trap is hidden.
    #[must_use]
    pub const fn is_hidden(&self) -> bool {
        self.is_hidden
    }

    /// Returns whether the trap is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.is_active
    }
}

// =============================================================================
// TriggerTrap Workflow
// =============================================================================

/// Creates a workflow function for triggering a trap.
///
/// This function returns a closure that handles trap activation when
/// an entity steps on a trap tile.
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
/// A function that takes a `TriggerTrapCommand` and returns an `AsyncIO`
/// that produces the updated game session or an error.
pub fn trigger_trap<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
    cache_ttl: Duration,
) -> impl Fn(TriggerTrapCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    move |command| {
        let cache = cache.clone();
        let event_store = event_store.clone();
        let game_identifier = *command.game_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |session_option| {
            match session_option {
                Some(session) => {
                    // Steps 2-6: [Pure] Trigger trap
                    let result = trigger_trap_pure(&session, command.position(), command.target());

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
pub fn trigger_trap_with_default_ttl<'a, C, E>(
    cache: &'a C,
    event_store: &'a E,
) -> impl Fn(TriggerTrapCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    C: SessionCache,
    E: EventStore,
{
    trigger_trap(cache, event_store, DEFAULT_CACHE_TIME_TO_LIVE)
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure function that performs the entire trap triggering logic.
fn trigger_trap_pure<S: Clone>(
    _session: &S,
    _position: Position,
    _target: roguelike_domain::enemy::EntityIdentifier,
) -> Result<(S, Vec<GameSessionEvent>), WorkflowError> {
    // Placeholder implementation
    Err(WorkflowError::repository(
        "trigger_trap",
        "GameSession structure not yet connected",
    ))
}

/// Finds a trap at the given position.
///
/// This is a pure function that looks up a trap at a position.
///
/// # Type Parameters
///
/// * `F` - Function to get trap info from session
///
/// # Arguments
///
/// * `position` - The position to check
/// * `get_trap_fn` - Function that retrieves trap info
///
/// # Returns
///
/// `Ok(TrapInfo)` if an active trap exists, `Err(FloorError)` otherwise.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::{find_trap_at_position, TrapInfo};
/// use roguelike_domain::common::Position;
/// use roguelike_domain::floor::TrapType;
///
/// let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, true);
///
/// let result = find_trap_at_position(
///     Position::new(10, 10),
///     |pos| if pos == Position::new(10, 10) { Some(trap) } else { None },
/// );
///
/// assert!(result.is_ok());
/// ```
pub fn find_trap_at_position<F>(position: Position, get_trap_fn: F) -> Result<TrapInfo, FloorError>
where
    F: Fn(Position) -> Option<TrapInfo>,
{
    match get_trap_fn(position) {
        Some(trap) if trap.is_active() => Ok(trap),
        Some(_) => Err(FloorError::trap_already_disarmed(position)),
        None => Err(FloorError::no_trap_at_position(position)),
    }
}

/// Calculates the effect of a trap.
///
/// This is a pure function that determines what happens when a trap is triggered.
///
/// # Arguments
///
/// * `trap` - The trap information
/// * `floor_level` - The current floor level (affects damage scaling)
/// * `teleport_destination` - Optional random destination for teleport traps
///
/// # Returns
///
/// The effect of the triggered trap.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::{calculate_trap_effect, TrapInfo};
/// use roguelike_domain::common::Position;
/// use roguelike_domain::floor::TrapType;
///
/// let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, true);
/// let effect = calculate_trap_effect(&trap, 5, None);
///
/// assert!(effect.damage().value() > 0);
/// assert!(effect.should_disarm());
/// ```
#[must_use]
pub fn calculate_trap_effect(
    trap: &TrapInfo,
    floor_level: u32,
    teleport_destination: Option<Position>,
) -> TrapEffect {
    match trap.trap_type() {
        TrapType::Spike => {
            let base_damage = 10;
            let level_bonus = floor_level * 2;
            TrapEffect::damage_only(Damage::new(base_damage + level_bonus), true)
        }
        TrapType::Poison => {
            let potency = 3 + floor_level / 2;
            let duration = 3 + floor_level / 5;
            let status = StatusEffect::new(StatusEffectType::Poison, duration, potency);
            TrapEffect::with_status(status, false)
        }
        TrapType::Teleport => {
            let destination = teleport_destination.unwrap_or(Position::new(0, 0));
            TrapEffect::teleport(destination, false)
        }
        TrapType::Alarm => TrapEffect::alarm(false),
    }
}

/// Applies a trap effect to a target.
///
/// This is a pure function that applies trap effects.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to apply the effect
///
/// # Arguments
///
/// * `session` - The current game session
/// * `target_id` - The ID of the entity to affect
/// * `effect` - The trap effect to apply
/// * `apply_fn` - Function that applies the effect
///
/// # Returns
///
/// The updated session with effects applied.
pub fn apply_trap_effect<S, F>(
    session: &S,
    target_id: roguelike_domain::enemy::EntityIdentifier,
    effect: &TrapEffect,
    apply_fn: F,
) -> S
where
    S: Clone,
    F: Fn(&S, roguelike_domain::enemy::EntityIdentifier, &TrapEffect) -> S,
{
    apply_fn(session, target_id, effect)
}

/// Disarms a trap if it should be disarmed.
///
/// This is a pure function that marks a trap as inactive.
///
/// # Type Parameters
///
/// * `S` - The session type
/// * `F` - Function to disarm the trap
///
/// # Arguments
///
/// * `session` - The current game session
/// * `trap_position` - The position of the trap
/// * `should_disarm` - Whether the trap should be disarmed
/// * `disarm_fn` - Function that disarms the trap
///
/// # Returns
///
/// A tuple of (updated_session, was_disarmed).
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::disarm_trap;
/// use roguelike_domain::common::Position;
///
/// #[derive(Clone)]
/// struct MockSession { trap_disarmed: bool }
/// let session = MockSession { trap_disarmed: false };
///
/// let (updated, was_disarmed) = disarm_trap(
///     &session,
///     Position::new(10, 10),
///     true,
///     |_, _| MockSession { trap_disarmed: true },
/// );
///
/// assert!(was_disarmed);
/// assert!(updated.trap_disarmed);
/// ```
pub fn disarm_trap<S, F>(
    session: &S,
    trap_position: Position,
    should_disarm: bool,
    disarm_fn: F,
) -> (S, bool)
where
    S: Clone,
    F: Fn(&S, Position) -> S,
{
    if should_disarm {
        (disarm_fn(session, trap_position), true)
    } else {
        (session.clone(), false)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::enemy::EntityIdentifier;
    use rstest::rstest;

    // =========================================================================
    // TrapEffect Tests
    // =========================================================================

    mod trap_effect {
        use super::*;

        #[rstest]
        fn new_creates_effect() {
            let damage = Damage::new(10);
            let status = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect = TrapEffect::new(damage, Some(status), None, false, true);

            assert_eq!(effect.damage(), damage);
            assert!(effect.status_effect().is_some());
            assert!(effect.teleport_destination().is_none());
            assert!(!effect.alerts_enemies());
            assert!(effect.should_disarm());
        }

        #[rstest]
        fn damage_only_creates_correctly() {
            let effect = TrapEffect::damage_only(Damage::new(25), true);

            assert_eq!(effect.damage().value(), 25);
            assert!(effect.status_effect().is_none());
            assert!(effect.should_disarm());
        }

        #[rstest]
        fn with_status_creates_correctly() {
            let status = StatusEffect::new(StatusEffectType::Burn, 5, 8);
            let effect = TrapEffect::with_status(status, false);

            assert_eq!(effect.damage().value(), 0);
            assert!(effect.status_effect().is_some());
            assert!(!effect.should_disarm());
        }

        #[rstest]
        fn teleport_creates_correctly() {
            let destination = Position::new(50, 50);
            let effect = TrapEffect::teleport(destination, false);

            assert_eq!(effect.damage().value(), 0);
            assert_eq!(effect.teleport_destination(), Some(destination));
            assert!(!effect.should_disarm());
        }

        #[rstest]
        fn alarm_creates_correctly() {
            let effect = TrapEffect::alarm(false);

            assert_eq!(effect.damage().value(), 0);
            assert!(effect.alerts_enemies());
            assert!(!effect.should_disarm());
        }
    }

    // =========================================================================
    // TrapInfo Tests
    // =========================================================================

    mod trap_info {
        use super::*;

        #[rstest]
        fn new_creates_info() {
            let info = TrapInfo::new(Position::new(15, 20), TrapType::Poison, true, true);

            assert_eq!(info.position(), Position::new(15, 20));
            assert_eq!(info.trap_type(), TrapType::Poison);
            assert!(info.is_hidden());
            assert!(info.is_active());
        }

        #[rstest]
        fn inactive_trap() {
            let info = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, false);
            assert!(!info.is_active());
        }
    }

    // =========================================================================
    // find_trap_at_position Tests
    // =========================================================================

    mod find_trap_at_position_tests {
        use super::*;

        #[rstest]
        fn finds_active_trap() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, true);

            let result = find_trap_at_position(Position::new(10, 10), |pos| {
                if pos == Position::new(10, 10) {
                    Some(trap)
                } else {
                    None
                }
            });

            assert!(result.is_ok());
            assert_eq!(result.unwrap().trap_type(), TrapType::Spike);
        }

        #[rstest]
        fn returns_error_for_inactive_trap() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, false);

            let result = find_trap_at_position(Position::new(10, 10), |_| Some(trap));

            assert!(result.is_err());
        }

        #[rstest]
        fn returns_error_for_no_trap() {
            let result = find_trap_at_position(Position::new(10, 10), |_| None);

            assert!(result.is_err());
        }
    }

    // =========================================================================
    // calculate_trap_effect Tests
    // =========================================================================

    mod calculate_trap_effect_tests {
        use super::*;

        #[rstest]
        fn spike_trap_deals_damage() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, true);
            let effect = calculate_trap_effect(&trap, 5, None);

            assert!(effect.damage().value() > 0);
            assert!(effect.should_disarm());
        }

        #[rstest]
        fn spike_damage_scales_with_level() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Spike, false, true);

            let effect_low = calculate_trap_effect(&trap, 1, None);
            let effect_high = calculate_trap_effect(&trap, 10, None);

            assert!(effect_high.damage().value() > effect_low.damage().value());
        }

        #[rstest]
        fn poison_trap_applies_status() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Poison, false, true);
            let effect = calculate_trap_effect(&trap, 5, None);

            assert_eq!(effect.damage().value(), 0);
            assert!(effect.status_effect().is_some());
            let status = effect.status_effect().unwrap();
            assert_eq!(status.effect_type(), StatusEffectType::Poison);
            assert!(!effect.should_disarm());
        }

        #[rstest]
        fn teleport_trap_moves_target() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Teleport, false, true);
            let destination = Position::new(50, 50);
            let effect = calculate_trap_effect(&trap, 5, Some(destination));

            assert_eq!(effect.teleport_destination(), Some(destination));
            assert!(!effect.should_disarm());
        }

        #[rstest]
        fn alarm_trap_alerts_enemies() {
            let trap = TrapInfo::new(Position::new(10, 10), TrapType::Alarm, false, true);
            let effect = calculate_trap_effect(&trap, 5, None);

            assert!(effect.alerts_enemies());
            assert!(!effect.should_disarm());
        }
    }

    // =========================================================================
    // apply_trap_effect Tests
    // =========================================================================

    mod apply_trap_effect_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            damage_applied: u32,
            has_status: bool,
        }

        impl MockSession {
            fn new() -> Self {
                Self {
                    damage_applied: 0,
                    has_status: false,
                }
            }
        }

        #[rstest]
        fn applies_effect() {
            let session = MockSession::new();
            let target = EntityIdentifier::new();
            let effect = TrapEffect::damage_only(Damage::new(25), true);

            let updated =
                apply_trap_effect(&session, target, &effect, |_, _, effect| MockSession {
                    damage_applied: effect.damage().value(),
                    has_status: effect.status_effect().is_some(),
                });

            assert_eq!(updated.damage_applied, 25);
            assert!(!updated.has_status);
        }

        #[rstest]
        fn applies_status_effect() {
            let session = MockSession::new();
            let target = EntityIdentifier::new();
            let status = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect = TrapEffect::with_status(status, false);

            let updated =
                apply_trap_effect(&session, target, &effect, |_, _, effect| MockSession {
                    damage_applied: effect.damage().value(),
                    has_status: effect.status_effect().is_some(),
                });

            assert_eq!(updated.damage_applied, 0);
            assert!(updated.has_status);
        }
    }

    // =========================================================================
    // disarm_trap Tests
    // =========================================================================

    mod disarm_trap_tests {
        use super::*;

        #[derive(Clone)]
        struct MockSession {
            trap_disarmed: bool,
        }

        impl MockSession {
            fn new() -> Self {
                Self {
                    trap_disarmed: false,
                }
            }
        }

        #[rstest]
        fn disarms_when_should_disarm() {
            let session = MockSession::new();

            let (updated, was_disarmed) =
                disarm_trap(&session, Position::new(10, 10), true, |_, _| MockSession {
                    trap_disarmed: true,
                });

            assert!(was_disarmed);
            assert!(updated.trap_disarmed);
        }

        #[rstest]
        fn does_not_disarm_when_should_not() {
            let session = MockSession::new();

            let (updated, was_disarmed) =
                disarm_trap(&session, Position::new(10, 10), false, |_, _| MockSession {
                    trap_disarmed: true,
                });

            assert!(!was_disarmed);
            assert!(!updated.trap_disarmed);
        }
    }
}
