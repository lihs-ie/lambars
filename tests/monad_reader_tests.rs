#![cfg(feature = "effect")]
//! Tests for MonadReader trait.
//!
//! This module tests the MonadReader type class which provides
//! environment reading capabilities.
//!
//! Note: These tests verify trait definitions and signatures.

// =============================================================================
// Trait Definition Tests
// =============================================================================

/// Verify MonadReader trait exists and requires Monad as supertrait.
/// This is a compile-time check - if this file compiles, the trait is defined correctly.
#[test]
fn monad_reader_trait_compiles() {
    use lambars::effect::MonadReader;
    use lambars::typeclass::Monad;

    // This verifies that MonadReader trait exists and requires Monad
    // The function type signature enforces these bounds at compile time
    fn _check<R, M: MonadReader<R>>() {
        fn requires_monad<T: Monad>() {}
        requires_monad::<M>();
    }
    // The function definition itself is sufficient to verify the trait bounds
}

// =============================================================================
// Law Documentation Tests
// =============================================================================

/// MonadReader Laws (to be verified with actual implementations):
///
/// 1. Ask Local Identity: local(|r| r, m) == m
///    Applying identity modifier should not change the computation.
///
/// 2. Ask Local Composition: local(f, local(g, m)) == local(|r| g(f(r)), m)
///    local should compose modifiers correctly.
///
/// 3. Ask Retrieval: ask.run(r) == r
///    ask should return the environment unchanged.
#[test]
fn monad_reader_laws_are_documented() {
    // This test serves as documentation for MonadReader laws
    // Actual law verification is done in reader_laws.rs
}
