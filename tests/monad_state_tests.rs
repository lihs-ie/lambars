#![cfg(feature = "effect")]
//! Tests for MonadState trait.
//!
//! This module tests the MonadState type class which provides
//! stateful computation capabilities.
//!
//! Note: These tests verify trait definitions and signatures.

// =============================================================================
// Trait Definition Tests
// =============================================================================

/// Verify MonadState trait exists and requires Monad as supertrait.
/// This is a compile-time check - if this file compiles, the trait is defined correctly.
#[test]
fn monad_state_trait_compiles() {
    use lambars::effect::MonadState;
    use lambars::typeclass::Monad;

    // This verifies that MonadState trait exists and requires Monad
    // The function type signature enforces these bounds at compile time
    fn _check<S, M: MonadState<S>>() {
        fn requires_monad<T: Monad>() {}
        requires_monad::<M>();
    }
    // The function definition itself is sufficient to verify the trait bounds
}

// =============================================================================
// Law Documentation Tests
// =============================================================================

/// MonadState Laws (to be verified with actual implementations):
///
/// 1. Get Put Law: get().flat_map(|s| put(s)) == pure(())
///    Getting and then putting the same state is a no-op.
///
/// 2. Put Get Law: put(s).then(get()) returns s
///    After putting a state, get should return that state.
///
/// 3. Put Put Law: put(s1).then(put(s2)) == put(s2)
///    Consecutive puts result in the last put winning.
///
/// 4. Modify Composition Law: modify(f).then(modify(g)) == modify(|s| g(f(s)))
///    Consecutive modifies compose the functions.
#[test]
fn monad_state_laws_are_documented() {
    // This test serves as documentation for MonadState laws
    // Actual law verification is done in state_laws.rs
}
