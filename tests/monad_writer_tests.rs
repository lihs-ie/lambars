#![cfg(feature = "effect")]
//! Tests for MonadWriter trait.
//!
//! This module tests the MonadWriter type class which provides
//! log output capabilities.
//!
//! Note: These tests verify trait definitions and signatures.

// =============================================================================
// Trait Definition Tests
// =============================================================================

/// Verify MonadWriter trait exists and requires Monad as supertrait.
/// This is a compile-time check - if this file compiles, the trait is defined correctly.
#[test]
fn monad_writer_trait_compiles() {
    use lambars::effect::MonadWriter;
    use lambars::typeclass::{Monad, Monoid};

    // This verifies that MonadWriter trait exists and requires Monad
    // The function type signature enforces these bounds at compile time
    fn _check<W: Monoid, M: MonadWriter<W>>() {
        fn requires_monad<T: Monad>() {}
        requires_monad::<M>();
    }
    // The function definition itself is sufficient to verify the trait bounds
}

// =============================================================================
// Monoid Requirement Tests
// =============================================================================

/// Verify common monoid types work as MonadWriter output.
#[test]
fn monad_writer_common_monoid_outputs() {
    use lambars::typeclass::Monoid;

    fn check<W: Monoid>() {}

    // String is a Monoid (empty + concatenation)
    check::<String>();

    // Vec<T> is a Monoid (empty + append)
    check::<Vec<String>>();
    check::<Vec<i32>>();

    // Option<T> is a Monoid when T: Semigroup
    check::<Option<String>>();
}

// =============================================================================
// Law Documentation Tests
// =============================================================================

/// MonadWriter Laws (to be verified with actual implementations):
///
/// 1. Tell Monoid Law: tell(w1).then(tell(w2)) == tell(w1.combine(w2))
///    Consecutive tells should be equivalent to telling the combined output.
///
/// 2. Listen Tell Law: listen(tell(w)) == tell(w).map(|_| ((), w))
///    Listening to a tell should return the output along with the result.
///
/// 3. Pass Identity Law: pass(m.map(|a| (a, |w| w))) == m
///    Passing an identity function should not change the computation.
///
/// 4. Censor Definition: censor(f, m) == pass(m.map(|a| (a, f)))
///    Censor is defined in terms of pass.
#[test]
fn monad_writer_laws_are_documented() {
    // This test serves as documentation for MonadWriter laws
    // Actual law verification is done in writer_laws.rs
}
