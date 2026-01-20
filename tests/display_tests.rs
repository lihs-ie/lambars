//! Integration tests for Display trait implementations.
//!
//! This module tests that all types in the library correctly implement
//! the Display trait with consistent formatting.

#![cfg(all(feature = "control", feature = "persistent", feature = "effect"))]

use lambars::control::{Either, Lazy, Trampoline};
use lambars::effect::{AsyncIO, IO, Reader, State, Writer};
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentList, PersistentTreeMap, PersistentVector,
};
use std::panic;

// =============================================================================
// Control Module Display Tests
// =============================================================================

#[test]
fn test_either_left_display() {
    let left: Either<i32, String> = Either::Left(42);
    assert_eq!(format!("{}", left), "Left(42)");
}

#[test]
fn test_either_right_display() {
    let right: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(format!("{}", right), "Right(hello)");
}

#[test]
fn test_lazy_uninit_display() {
    let lazy = Lazy::new(|| 42);
    assert_eq!(format!("{}", lazy), "Lazy(<uninit>)");
}

#[test]
fn test_lazy_evaluated_display() {
    let lazy = Lazy::new(|| 42);
    let _ = lazy.force();
    assert_eq!(format!("{}", lazy), "Lazy(42)");
}

#[test]
fn test_lazy_poisoned_display() {
    let lazy = Lazy::new(|| -> i32 { panic!("initialization failed") });
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| lazy.force()));
    assert_eq!(format!("{}", lazy), "Lazy(<poisoned>)");
}

#[test]
fn test_trampoline_done_display() {
    let trampoline = Trampoline::done(42);
    assert_eq!(format!("{}", trampoline), "Done(42)");
}

#[test]
fn test_trampoline_suspend_display() {
    let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(42));
    assert_eq!(format!("{}", trampoline), "<Suspend>");
}

#[test]
fn test_trampoline_flatmap_display() {
    // Note: flat_map on Done now evaluates eagerly (performance optimization),
    // so we use Suspend to create a FlatMapInternal state
    let trampoline: Trampoline<i32> =
        Trampoline::suspend(|| Trampoline::done(21)).flat_map(|value| Trampoline::done(value * 2));
    assert_eq!(format!("{}", trampoline), "<FlatMap>");
}

// =============================================================================
// Persistent Data Structures Display Tests
// =============================================================================

#[test]
fn test_persistent_list_empty_display() {
    let list: PersistentList<i32> = PersistentList::new();
    assert_eq!(format!("{}", list), "[]");
}

#[test]
fn test_persistent_list_elements_display() {
    let list: PersistentList<i32> = (1..=3).collect();
    assert_eq!(format!("{}", list), "[1, 2, 3]");
}

#[test]
fn test_persistent_vector_empty_display() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert_eq!(format!("{}", vector), "[]");
}

#[test]
fn test_persistent_vector_elements_display() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    assert_eq!(format!("{}", vector), "[1, 2, 3]");
}

#[test]
fn test_persistent_hashset_empty_display() {
    let set: PersistentHashSet<i32> = PersistentHashSet::new();
    assert_eq!(format!("{}", set), "{}");
}

#[test]
fn test_persistent_hashset_single_display() {
    let set = PersistentHashSet::singleton(42);
    assert_eq!(format!("{}", set), "{42}");
}

#[test]
fn test_persistent_hashmap_empty_display() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    assert_eq!(format!("{}", map), "{}");
}

#[test]
fn test_persistent_hashmap_single_display() {
    let map = PersistentHashMap::singleton("key".to_string(), 42);
    assert_eq!(format!("{}", map), "{key: 42}");
}

#[test]
fn test_persistent_treemap_empty_display() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert_eq!(format!("{}", map), "{}");
}

#[test]
fn test_persistent_treemap_sorted_display() {
    let map = PersistentTreeMap::new()
        .insert(3, "three".to_string())
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());
    // TreeMap should display in sorted order
    assert_eq!(format!("{}", map), "{1: one, 2: two, 3: three}");
}

// =============================================================================
// Effect Module Display Tests
// =============================================================================

#[test]
fn test_writer_display() {
    let writer: Writer<String, i32> = Writer::new(42, "log".to_string());
    assert_eq!(format!("{}", writer), "Writer(42, log)");
}

#[test]
fn test_reader_display() {
    let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    assert_eq!(format!("{}", reader), "<Reader::Deferred>");

    let reader_pure: Reader<i32, i32> = Reader::pure(42);
    assert_eq!(format!("{}", reader_pure), "<Reader::Pure>");
}

#[test]
fn test_state_display() {
    let state: State<i32, i32> = State::new(|state| (state * 2, state + 1));
    assert_eq!(format!("{}", state), "<State::Deferred>");

    let state_pure: State<i32, i32> = State::pure(42);
    assert_eq!(format!("{}", state_pure), "<State::Pure>");
}

#[test]
fn test_io_display() {
    let io = IO::pure(42);
    assert_eq!(format!("{}", io), "<IO>");
}

#[test]
fn test_async_io_display() {
    let async_io = AsyncIO::pure(42);
    assert_eq!(format!("{}", async_io), "<AsyncIO>");
}

// =============================================================================
// Consistency Tests - Verify format strings are user-friendly
// =============================================================================

#[test]
fn test_display_output_is_human_readable() {
    // Verify that Display output differs from Debug output for complex types
    let list: PersistentList<i32> = (1..=3).collect();

    let display_output = format!("{}", list);
    let debug_output = format!("{:?}", list);

    // Display should be more human-readable (no quotes around elements)
    assert!(!display_output.contains('"'));
    // Debug uses the standard debug formatter
    assert!(debug_output.starts_with('['));
}

#[test]
fn test_display_is_consistent_with_standard_library() {
    // Verify our Display implementations follow Rust conventions
    // Similar to standard library collection Debug formatting (e.g., Vec<T>)

    let vector: PersistentVector<i32> = (1..=3).collect();
    let output = format!("{}", vector);

    // Should use square brackets for sequence types
    assert!(output.starts_with('['));
    assert!(output.ends_with(']'));
    // Should use comma-space separator
    assert!(output.contains(", "));
}
