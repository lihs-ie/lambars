//! Unit tests for Either<L, R> type.
//!
//! Either represents a value that can be one of two types:
//! - `Left(L)`: Contains a value of type L
//! - `Right(R)`: Contains a value of type R
//!
//! This type is commonly used in functional programming for:
//! - Error handling (Left for errors, Right for success)
//! - Branching computations
//! - As the resume type for Trampoline

#![cfg(feature = "control")]

use functional_rusty::control::Either;
use rstest::rstest;

// =============================================================================
// Basic Construction and Type Checking
// =============================================================================

#[rstest]
fn either_left_is_left() {
    let value: Either<i32, String> = Either::Left(42);
    assert!(value.is_left());
    assert!(!value.is_right());
}

#[rstest]
fn either_right_is_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert!(value.is_right());
    assert!(!value.is_left());
}

// =============================================================================
// Value Extraction
// =============================================================================

#[rstest]
fn either_left_extraction() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.left(), Some(42));
}

#[rstest]
fn either_left_extraction_from_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.left(), None);
}

#[rstest]
fn either_right_extraction() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.right(), Some("hello".to_string()));
}

#[rstest]
fn either_right_extraction_from_left() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.right(), None);
}

// =============================================================================
// Reference Extraction
// =============================================================================

#[rstest]
fn either_left_ref_extraction() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.left_ref(), Some(&42));
}

#[rstest]
fn either_left_ref_extraction_from_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.left_ref(), None);
}

#[rstest]
fn either_right_ref_extraction() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.right_ref(), Some(&"hello".to_string()));
}

#[rstest]
fn either_right_ref_extraction_from_left() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.right_ref(), None);
}

// =============================================================================
// Mapping Operations
// =============================================================================

#[rstest]
fn either_map_left_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    let result = value.map_left(|x| x * 2);
    assert_eq!(result, Either::Left(84));
}

#[rstest]
fn either_map_left_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let result = value.map_left(|x: i32| x * 2);
    assert_eq!(result, Either::Right("hello".to_string()));
}

#[rstest]
fn either_map_right_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let result = value.map_right(|s| s.len());
    assert_eq!(result, Either::Right(5));
}

#[rstest]
fn either_map_right_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    let result = value.map_right(|s: String| s.len());
    assert_eq!(result, Either::Left(42));
}

// =============================================================================
// Bimap Operation
// =============================================================================

#[rstest]
fn either_bimap_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    let result = value.bimap(|x| x * 2, |s: String| s.len());
    assert_eq!(result, Either::Left(84));
}

#[rstest]
fn either_bimap_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let result = value.bimap(|x: i32| x * 2, |s| s.len());
    assert_eq!(result, Either::Right(5));
}

// =============================================================================
// Fold Operation
// =============================================================================

#[rstest]
fn either_fold_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    let result = value.fold(|x| x.to_string(), |s| s);
    assert_eq!(result, "42");
}

#[rstest]
fn either_fold_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let result = value.fold(|x: i32| x.to_string(), |s| s);
    assert_eq!(result, "hello");
}

// =============================================================================
// Swap Operation
// =============================================================================

#[rstest]
fn either_swap_left_to_right() {
    let value: Either<i32, String> = Either::Left(42);
    let result = value.swap();
    assert_eq!(result, Either::Right(42));
}

#[rstest]
fn either_swap_right_to_left() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let result = value.swap();
    assert_eq!(result, Either::Left("hello".to_string()));
}

// =============================================================================
// Unwrap Operations
// =============================================================================

#[rstest]
fn either_unwrap_left_success() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.unwrap_left(), 42);
}

#[rstest]
#[should_panic(expected = "called `Either::unwrap_left()` on a `Right` value")]
fn either_unwrap_left_panic() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    value.unwrap_left();
}

#[rstest]
fn either_unwrap_right_success() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.unwrap_right(), "hello".to_string());
}

#[rstest]
#[should_panic(expected = "called `Either::unwrap_right()` on a `Left` value")]
fn either_unwrap_right_panic() {
    let value: Either<i32, String> = Either::Left(42);
    value.unwrap_right();
}

// =============================================================================
// Unwrap Or Default Operations
// =============================================================================

#[rstest]
fn either_left_or_default_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.left_or_default(), 42);
}

#[rstest]
fn either_left_or_default_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.left_or_default(), 0);
}

#[rstest]
fn either_right_or_default_on_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    assert_eq!(value.right_or_default(), "hello".to_string());
}

#[rstest]
fn either_right_or_default_on_left() {
    let value: Either<i32, String> = Either::Left(42);
    assert_eq!(value.right_or_default(), String::new());
}

// =============================================================================
// Into Conversions
// =============================================================================

#[rstest]
fn either_into_option_left() {
    let value: Either<i32, String> = Either::Left(42);
    let (left, right): (Option<i32>, Option<String>) = value.into_options();
    assert_eq!(left, Some(42));
    assert_eq!(right, None);
}

#[rstest]
fn either_into_option_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let (left, right): (Option<i32>, Option<String>) = value.into_options();
    assert_eq!(left, None);
    assert_eq!(right, Some("hello".to_string()));
}

// =============================================================================
// Clone and Debug
// =============================================================================

#[rstest]
fn either_clone_left() {
    let value: Either<i32, String> = Either::Left(42);
    let cloned = value.clone();
    assert_eq!(value, cloned);
}

#[rstest]
fn either_clone_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let cloned = value.clone();
    assert_eq!(value, cloned);
}

#[rstest]
fn either_debug_left() {
    let value: Either<i32, String> = Either::Left(42);
    let debug_str = format!("{:?}", value);
    assert_eq!(debug_str, "Left(42)");
}

#[rstest]
fn either_debug_right() {
    let value: Either<i32, String> = Either::Right("hello".to_string());
    let debug_str = format!("{:?}", value);
    assert_eq!(debug_str, "Right(\"hello\")");
}

// =============================================================================
// PartialEq and Eq
// =============================================================================

#[rstest]
fn either_eq_left() {
    let value1: Either<i32, String> = Either::Left(42);
    let value2: Either<i32, String> = Either::Left(42);
    let value3: Either<i32, String> = Either::Left(43);

    assert_eq!(value1, value2);
    assert_ne!(value1, value3);
}

#[rstest]
fn either_eq_right() {
    let value1: Either<i32, String> = Either::Right("hello".to_string());
    let value2: Either<i32, String> = Either::Right("hello".to_string());
    let value3: Either<i32, String> = Either::Right("world".to_string());

    assert_eq!(value1, value2);
    assert_ne!(value1, value3);
}

#[rstest]
fn either_ne_left_right() {
    let left: Either<i32, i32> = Either::Left(42);
    let right: Either<i32, i32> = Either::Right(42);

    assert_ne!(left, right);
}

// =============================================================================
// Hash
// =============================================================================

#[rstest]
fn either_hash_consistency() {
    use std::collections::HashSet;

    let mut set: HashSet<Either<i32, String>> = HashSet::new();
    set.insert(Either::Left(42));
    set.insert(Either::Right("hello".to_string()));

    assert!(set.contains(&Either::Left(42)));
    assert!(set.contains(&Either::Right("hello".to_string())));
    assert!(!set.contains(&Either::Left(43)));
}
