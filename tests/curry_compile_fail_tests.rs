//! Compile-fail tests for the curry! macro.
//!
//! These tests verify that invalid usages of curry! produce
//! appropriate compile-time errors.
//!
//! Note: trybuild tests use #[test] as an exception because
//! trybuild's standard usage pattern requires it.

#![cfg(feature = "compose")]

#[test]
fn curry_compile_fail_tests() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/compile_fail/curry_*.rs");
}
