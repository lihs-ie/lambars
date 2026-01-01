#![cfg(feature = "typeclass")]
//! Integration tests for lambars library.
//!
//! These tests verify the public API of the library works correctly
//! across module boundaries.

use rstest::rstest;

/// Smoke test to ensure the library is accessible from integration tests.
#[rstest]
fn library_is_accessible() {
    // This test ensures the library compiles and is accessible
    // More specific tests will be added as features are implemented
    // The test passes by successfully compiling
}

/// Parameterized test example using rstest.
/// This pattern will be used for property-based testing of type class laws.
#[rstest]
#[case(1, 2, 3)]
#[case(0, 0, 0)]
#[case(-1, 1, 0)]
fn addition_is_commutative(#[case] a: i32, #[case] b: i32, #[case] expected: i32) {
    assert_eq!(a + b, expected);
    assert_eq!(b + a, expected);
}

/// Fixture example for reusable test setup.
#[rstest]
fn with_fixture(#[values(1, 2, 3, 4, 5)] value: i32) {
    assert!(value > 0);
    assert!(value <= 5);
}
