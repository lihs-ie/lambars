#![cfg(feature = "optics")]
//! Unit tests for Iso optics.
//!
//! Tests the Iso trait and related implementations.

use lambars::iso;
use lambars::optics::{FunctionIso, Iso, Lens, Prism};
use rstest::rstest;

// =============================================================================
// Test Data Structures
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
struct Wrapper<T>(T);

// =============================================================================
// Iso Trait Existence Tests
// =============================================================================

#[test]
fn test_iso_trait_exists() {
    fn assert_iso<I: Iso<String, Vec<char>>>(_iso: I) {}

    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    assert_iso(string_chars_iso);
}

// =============================================================================
// FunctionIso Basic Tests
// =============================================================================

#[test]
fn test_function_iso_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let original = "hello".to_string();
    let chars = string_chars_iso.get(original);
    assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_function_iso_reverse_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let chars = vec!['h', 'i'];
    let string = string_chars_iso.reverse_get(chars);
    assert_eq!(string, "hi");
}

#[test]
fn test_function_iso_roundtrip_get_then_reverse_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let original = "hello".to_string();
    let chars = string_chars_iso.get(original.clone());
    let back = string_chars_iso.reverse_get(chars);
    assert_eq!(back, original);
}

#[test]
fn test_function_iso_roundtrip_reverse_get_then_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let original_chars = vec!['w', 'o', 'r', 'l', 'd'];
    let string = string_chars_iso.reverse_get(original_chars.clone());
    let back = string_chars_iso.get(string);
    assert_eq!(back, original_chars);
}

#[test]
fn test_function_iso_with_numeric_types() {
    // Celsius to Fahrenheit conversion
    let celsius_fahrenheit_iso = FunctionIso::new(
        |celsius: f64| celsius * 9.0 / 5.0 + 32.0,
        |fahrenheit: f64| (fahrenheit - 32.0) * 5.0 / 9.0,
    );

    let celsius = 0.0;
    let fahrenheit = celsius_fahrenheit_iso.get(celsius);
    assert!((fahrenheit - 32.0).abs() < 1e-10);

    let back = celsius_fahrenheit_iso.reverse_get(fahrenheit);
    assert!((back - celsius).abs() < 1e-10);
}

// =============================================================================
// ReversedIso Tests
// =============================================================================

#[test]
fn test_reversed_iso_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let chars_string_iso = string_chars_iso.reverse();

    let chars = vec!['h', 'i'];
    let string = chars_string_iso.get(chars);
    assert_eq!(string, "hi");
}

#[test]
fn test_reversed_iso_reverse_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let chars_string_iso = string_chars_iso.reverse();

    let string = "hello".to_string();
    let chars = chars_string_iso.reverse_get(string);
    assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_reversed_iso_roundtrip() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let chars_string_iso = string_chars_iso.reverse();

    let original = vec!['t', 'e', 's', 't'];
    let string = chars_string_iso.get(original.clone());
    let back = chars_string_iso.reverse_get(string);
    assert_eq!(back, original);
}

#[test]
fn test_double_reverse_is_identity() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let double_reversed = string_chars_iso.clone().reverse().reverse();

    let original = "test".to_string();
    let result = double_reversed.get(original.clone());
    assert_eq!(result, string_chars_iso.get(original));
}

// =============================================================================
// ComposedIso Tests
// =============================================================================

#[test]
fn test_iso_compose_get() {
    // String <-> Vec<char> <-> String (reversed)
    // So String <-> String through Vec<char>
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    // Vec<char> <-> Vec<char> but with transformation (e.g., uppercase)
    let uppercase_iso = FunctionIso::new(
        |chars: Vec<char>| chars.into_iter().map(|c| c.to_ascii_uppercase()).collect(),
        |chars: Vec<char>| chars.into_iter().map(|c| c.to_ascii_lowercase()).collect(),
    );

    let composed = string_chars_iso.compose(uppercase_iso);

    let original = "hello".to_string();
    let result = composed.get(original);
    assert_eq!(result, vec!['H', 'E', 'L', 'L', 'O']);
}

#[test]
fn test_iso_compose_reverse_get() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let uppercase_iso = FunctionIso::new(
        |chars: Vec<char>| chars.into_iter().map(|c| c.to_ascii_uppercase()).collect(),
        |chars: Vec<char>| chars.into_iter().map(|c| c.to_ascii_lowercase()).collect(),
    );

    let composed = string_chars_iso.compose(uppercase_iso);

    let uppercase_chars = vec!['H', 'E', 'L', 'L', 'O'];
    let result = composed.reverse_get(uppercase_chars);
    assert_eq!(result, "hello");
}

#[test]
fn test_iso_compose_roundtrip() {
    let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);

    let iso2 = FunctionIso::new(
        |x: i64| x.to_string(),
        |s: String| s.parse::<i64>().unwrap(),
    );

    let composed = iso1.compose(iso2);

    let original = 42i32;
    let string = composed.get(original);
    assert_eq!(string, "42");

    let back = composed.reverse_get(string);
    assert_eq!(back, original);
}

// =============================================================================
// Modify Tests
// =============================================================================

#[test]
fn test_iso_modify() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let original = "hello".to_string();
    let result = string_chars_iso.modify(original, |mut chars| {
        chars.reverse();
        chars
    });

    assert_eq!(result, "olleh");
}

#[test]
fn test_iso_modify_with_identity_function() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let original = "hello".to_string();
    let result = string_chars_iso.modify(original.clone(), |chars| chars);

    assert_eq!(result, original);
}

// =============================================================================
// to_lens Tests
// =============================================================================

#[test]
fn test_iso_to_lens_get() {
    // Wrapper<i32> <-> i32
    let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));

    let as_lens = wrapper_iso.to_lens();

    let wrapped = Wrapper(42);
    assert_eq!(*as_lens.get(&wrapped), 42);
}

#[test]
fn test_iso_to_lens_set() {
    let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));

    let as_lens = wrapper_iso.to_lens();

    let wrapped = Wrapper(42);
    let updated = as_lens.set(wrapped, 100);
    assert_eq!(updated, Wrapper(100));
}

#[test]
fn test_iso_to_lens_modify() {
    let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));

    let as_lens = wrapper_iso.to_lens();

    let wrapped = Wrapper(10);
    let doubled = as_lens.modify(wrapped, |x| x * 2);
    assert_eq!(doubled, Wrapper(20));
}

// =============================================================================
// to_prism Tests
// =============================================================================

#[test]
fn test_iso_to_prism_preview() {
    let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));

    let as_prism = wrapper_iso.to_prism();

    let wrapped = Wrapper(42);
    assert_eq!(as_prism.preview(&wrapped), Some(&42));
}

#[test]
fn test_iso_to_prism_review() {
    let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));

    let as_prism = wrapper_iso.to_prism();

    let constructed = as_prism.review(100);
    assert_eq!(constructed, Wrapper(100));
}

// =============================================================================
// iso! Macro Tests
// =============================================================================

#[test]
fn test_iso_macro_basic() {
    let swap = iso!(|(a, b): (i32, String)| (b, a), |(b, a): (String, i32)| (
        a, b
    ));

    let tuple = (42, "hello".to_string());
    let swapped = swap.get(tuple.clone());
    assert_eq!(swapped, ("hello".to_string(), 42));

    let back = swap.reverse_get(swapped);
    assert_eq!(back, tuple);
}

#[test]
fn test_iso_macro_with_closures() {
    let double_half = iso!(|x: i32| x * 2, |x: i32| x / 2);

    assert_eq!(double_half.get(5), 10);
    assert_eq!(double_half.reverse_get(10), 5);
}

// =============================================================================
// Standard Iso Tests (iso_identity, iso_swap)
// =============================================================================

#[test]
fn test_iso_identity() {
    use lambars::optics::iso_identity;

    let identity_iso = iso_identity::<i32>();

    assert_eq!(identity_iso.get(42), 42);
    assert_eq!(identity_iso.reverse_get(42), 42);

    // Roundtrip
    let value = 100;
    assert_eq!(identity_iso.reverse_get(identity_iso.get(value)), value);
}

#[test]
fn test_iso_identity_with_string() {
    use lambars::optics::iso_identity;

    let identity_iso = iso_identity::<String>();

    let value = "hello".to_string();
    assert_eq!(identity_iso.get(value.clone()), value);
}

#[test]
fn test_iso_swap() {
    use lambars::optics::iso_swap;

    let swap_iso = iso_swap::<i32, String>();

    let tuple = (42, "hello".to_string());
    let swapped = swap_iso.get(tuple.clone());
    assert_eq!(swapped, ("hello".to_string(), 42));

    let back = swap_iso.reverse_get(swapped);
    assert_eq!(back, tuple);
}

#[test]
fn test_iso_swap_double_is_identity() {
    use lambars::optics::iso_swap;

    let swap_iso = iso_swap::<i32, String>();
    let swap_back = iso_swap::<String, i32>();

    let composed = swap_iso.compose(swap_back);

    let original = (42, "hello".to_string());
    let result = composed.get(original.clone());
    assert_eq!(result, original);
}

// =============================================================================
// Clone and Debug Tests
// =============================================================================

#[test]
fn test_function_iso_clone() {
    let iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let cloned = iso.clone();

    let original = "test".to_string();
    assert_eq!(iso.get(original.clone()), cloned.get(original));
}

#[test]
fn test_function_iso_debug() {
    let iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let debug_str = format!("{:?}", iso);
    assert!(debug_str.contains("FunctionIso"));
}

#[test]
fn test_reversed_iso_clone() {
    let iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let reversed = iso.reverse();
    let cloned = reversed.clone();

    let original = vec!['t', 'e', 's', 't'];
    assert_eq!(reversed.get(original.clone()), cloned.get(original));
}

#[test]
fn test_composed_iso_clone() {
    let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);

    let iso2 = FunctionIso::new(|x: i64| x * 2, |x: i64| x / 2);

    let composed = iso1.compose(iso2);
    let cloned = composed.clone();

    assert_eq!(composed.get(10), cloned.get(10));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_iso_with_empty_string() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let empty = String::new();
    let chars = string_chars_iso.get(empty.clone());
    assert!(chars.is_empty());

    let back = string_chars_iso.reverse_get(chars);
    assert_eq!(back, empty);
}

#[test]
fn test_iso_with_unicode() {
    let string_chars_iso = FunctionIso::new(
        |s: String| s.chars().collect::<Vec<_>>(),
        |chars: Vec<char>| chars.into_iter().collect::<String>(),
    );

    let unicode = "hello".to_string();
    let chars = string_chars_iso.get(unicode.clone());
    let back = string_chars_iso.reverse_get(chars);
    assert_eq!(back, unicode);
}

// =============================================================================
// Parameterized Tests
// =============================================================================

#[rstest]
#[case(0)]
#[case(1)]
#[case(-1)]
#[case(i32::MAX)]
#[case(i32::MIN)]
fn test_iso_identity_various_integers(#[case] value: i32) {
    use lambars::optics::iso_identity;

    let identity_iso = iso_identity::<i32>();

    assert_eq!(identity_iso.get(value), value);
    assert_eq!(identity_iso.reverse_get(value), value);
}

#[rstest]
#[case((1, "a".to_string()))]
#[case((0, String::new()))]
#[case((-42, "test".to_string()))]
fn test_iso_swap_various_tuples(#[case] tuple: (i32, String)) {
    use lambars::optics::iso_swap;

    let swap_iso = iso_swap::<i32, String>();

    let swapped = swap_iso.get(tuple.clone());
    assert_eq!(swapped.0, tuple.1);
    assert_eq!(swapped.1, tuple.0);

    let back = swap_iso.reverse_get(swapped);
    assert_eq!(back, tuple);
}

// =============================================================================
// IsoAsLens Additional Tests
// =============================================================================

mod iso_as_lens_additional_tests {
    use super::*;

    #[test]
    fn test_iso_as_lens_debug() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_lens = wrapper_iso.to_lens();

        let debug_str = format!("{:?}", as_lens);
        assert!(debug_str.contains("IsoAsLens"));
    }

    #[test]
    fn test_iso_as_lens_clone() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_lens = wrapper_iso.to_lens();
        let cloned = as_lens.clone();

        let wrapped = Wrapper(42);
        assert_eq!(*as_lens.get(&wrapped), *cloned.get(&wrapped));
    }
}

// =============================================================================
// IsoAsPrism Additional Tests
// =============================================================================

mod iso_as_prism_additional_tests {
    use super::*;

    #[test]
    fn test_iso_as_prism_debug() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_prism = wrapper_iso.to_prism();

        let debug_str = format!("{:?}", as_prism);
        assert!(debug_str.contains("IsoAsPrism"));
    }

    #[test]
    fn test_iso_as_prism_clone() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_prism = wrapper_iso.to_prism();
        let cloned = as_prism.clone();

        let wrapped = Wrapper(42);
        assert_eq!(as_prism.preview(&wrapped), cloned.preview(&wrapped));
    }

    #[test]
    fn test_iso_as_prism_preview_owned() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_prism = wrapper_iso.to_prism();

        let wrapped = Wrapper(42);
        assert_eq!(as_prism.preview_owned(wrapped), Some(42));
    }

    #[test]
    fn test_iso_as_prism_modify_option() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_prism = wrapper_iso.to_prism();

        let wrapped = Wrapper(10);
        let doubled = as_prism.modify_option(wrapped, |x| x * 2);
        assert_eq!(doubled, Some(Wrapper(20)));
    }

    #[test]
    fn test_iso_as_prism_modify_or_identity() {
        let wrapper_iso = FunctionIso::new(|w: Wrapper<i32>| w.0, |value: i32| Wrapper(value));
        let as_prism = wrapper_iso.to_prism();

        let wrapped = Wrapper(10);
        let doubled = as_prism.modify_or_identity(wrapped, |x| x * 2);
        assert_eq!(doubled, Wrapper(20));
    }
}

// =============================================================================
// ReversedIso Additional Tests
// =============================================================================

mod reversed_iso_additional_tests {
    use super::*;

    #[test]
    fn test_reversed_iso_debug() {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>(),
        );

        let reversed = string_chars_iso.reverse();
        let debug_str = format!("{:?}", reversed);
        assert!(debug_str.contains("ReversedIso"));
    }
}

// =============================================================================
// ComposedIso Additional Tests
// =============================================================================

mod composed_iso_additional_tests {
    use super::*;

    #[test]
    fn test_composed_iso_debug() {
        let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);
        let iso2 = FunctionIso::new(|x: i64| x * 2, |x: i64| x / 2);

        let composed = iso1.compose(iso2);
        let debug_str = format!("{:?}", composed);
        assert!(debug_str.contains("ComposedIso"));
    }

    #[test]
    fn test_composed_iso_modify() {
        let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);
        let iso2 = FunctionIso::new(
            |x: i64| x.to_string(),
            |s: String| s.parse::<i64>().unwrap(),
        );

        let composed = iso1.compose(iso2);
        let result = composed.modify(42, |s| s + "0");
        // "42" + "0" = "420", parsed as 420
        assert_eq!(result, 420);
    }
}
