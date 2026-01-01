//! Property-based tests for Iso laws.
//!
//! Tests that Iso implementations satisfy the mathematical laws.

use lambars::iso;
use lambars::optics::{FunctionIso, Iso, Lens, Prism};
use proptest::prelude::*;

// =============================================================================
// Iso Laws
// =============================================================================

// GetReverseGet Law: iso.reverse_get(iso.get(source)) == source
// ReverseGetGet Law: iso.get(iso.reverse_get(value)) == value

proptest! {
    // =========================================================================
    // String <-> Vec<char> Iso Laws
    // =========================================================================

    #[test]
    fn prop_get_reverse_get_law_string_chars(source in ".*") {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        let intermediate = string_chars_iso.get(source.clone());
        let roundtrip = string_chars_iso.reverse_get(intermediate);
        prop_assert_eq!(roundtrip, source);
    }

    #[test]
    fn prop_reverse_get_get_law_string_chars(chars in prop::collection::vec(any::<char>(), 0..100)) {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        let intermediate = string_chars_iso.reverse_get(chars.clone());
        let roundtrip = string_chars_iso.get(intermediate);
        prop_assert_eq!(roundtrip, chars);
    }

    // =========================================================================
    // Identity Iso Laws
    // =========================================================================

    #[test]
    fn prop_identity_get_reverse_get_law(value: i32) {
        use lambars::optics::iso_identity;

        let identity_iso = iso_identity::<i32>();
        let roundtrip = identity_iso.reverse_get(identity_iso.get(value));
        prop_assert_eq!(roundtrip, value);
    }

    #[test]
    fn prop_identity_reverse_get_get_law(value: i32) {
        use lambars::optics::iso_identity;

        let identity_iso = iso_identity::<i32>();
        let roundtrip = identity_iso.get(identity_iso.reverse_get(value));
        prop_assert_eq!(roundtrip, value);
    }

    // =========================================================================
    // Swap Iso Laws
    // =========================================================================

    #[test]
    fn prop_swap_get_reverse_get_law(first: i32, second in ".*") {
        use lambars::optics::iso_swap;

        let swap_iso = iso_swap::<i32, String>();
        let source = (first, second);
        let intermediate = swap_iso.get(source.clone());
        let roundtrip = swap_iso.reverse_get(intermediate);
        prop_assert_eq!(roundtrip, source);
    }

    #[test]
    fn prop_swap_reverse_get_get_law(first in ".*", second: i32) {
        use lambars::optics::iso_swap;

        let swap_iso = iso_swap::<i32, String>();
        let value = (first, second);
        let intermediate = swap_iso.reverse_get(value.clone());
        let roundtrip = swap_iso.get(intermediate);
        prop_assert_eq!(roundtrip, value);
    }

    // =========================================================================
    // Reversed Iso Laws
    // =========================================================================

    #[test]
    fn prop_reversed_get_reverse_get_law(chars in prop::collection::vec(any::<char>(), 0..100)) {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        let reversed = string_chars_iso.reverse();

        let intermediate = reversed.get(chars.clone());
        let roundtrip = reversed.reverse_get(intermediate);
        prop_assert_eq!(roundtrip, chars);
    }

    #[test]
    fn prop_reversed_reverse_get_get_law(source in ".*") {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        let reversed = string_chars_iso.reverse();

        let intermediate = reversed.reverse_get(source.clone());
        let roundtrip = reversed.get(intermediate);
        prop_assert_eq!(roundtrip, source);
    }

    // =========================================================================
    // Composed Iso Laws
    // =========================================================================

    #[test]
    fn prop_composed_get_reverse_get_law(value: i32) {
        let iso1 = FunctionIso::new(
            |x: i32| x as i64,
            |x: i64| x as i32
        );

        let iso2 = FunctionIso::new(
            |x: i64| x * 2,
            |x: i64| x / 2
        );

        let composed = iso1.compose(iso2);

        let intermediate = composed.get(value);
        let roundtrip = composed.reverse_get(intermediate);
        prop_assert_eq!(roundtrip, value);
    }

    #[test]
    fn prop_composed_reverse_get_get_law(value in -1000000i64..1000000i64) {
        // Note: This test uses even numbers only to avoid rounding issues with division
        // We also limit the range to avoid overflow
        let value = value * 2;  // Make it even

        let iso1 = FunctionIso::new(
            |x: i32| x as i64,
            |x: i64| x as i32
        );

        let iso2 = FunctionIso::new(
            |x: i64| x.saturating_mul(2),
            |x: i64| x / 2
        );

        let composed = iso1.compose(iso2);

        // For this composition to satisfy the law, the value must be within i32 range after division
        if value >= i32::MIN as i64 * 2 && value <= i32::MAX as i64 * 2 {
            let intermediate = composed.reverse_get(value);
            let roundtrip = composed.get(intermediate);
            prop_assert_eq!(roundtrip, value);
        }
    }

    // =========================================================================
    // iso! Macro Laws
    // =========================================================================

    #[test]
    fn prop_iso_macro_get_reverse_get_law(first: i32, second in ".*") {
        let swap = iso!(
            |(a, b): (i32, String)| (b, a),
            |(b, a): (String, i32)| (a, b)
        );

        let source = (first, second);
        let intermediate = swap.get(source.clone());
        let roundtrip = swap.reverse_get(intermediate);
        prop_assert_eq!(roundtrip, source);
    }

    #[test]
    fn prop_iso_macro_reverse_get_get_law(first in ".*", second: i32) {
        let swap = iso!(
            |(a, b): (i32, String)| (b, a),
            |(b, a): (String, i32)| (a, b)
        );

        let value = (first, second);
        let intermediate = swap.reverse_get(value.clone());
        let roundtrip = swap.get(intermediate);
        prop_assert_eq!(roundtrip, value);
    }

    // =========================================================================
    // Modify Laws
    // =========================================================================

    #[test]
    fn prop_modify_identity_law(source in ".*") {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        let result = string_chars_iso.modify(source.clone(), |chars| chars);
        prop_assert_eq!(result, source);
    }

    #[test]
    fn prop_modify_composition_law(source in ".*") {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>()
        );

        // Apply two transformations separately
        let function1 = |mut chars: Vec<char>| { chars.reverse(); chars };
        let function2 = |chars: Vec<char>| chars.into_iter().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>();

        let result1 = string_chars_iso.modify(
            string_chars_iso.modify(source.clone(), function1),
            function2
        );

        // Apply composed transformation
        let composed = |chars: Vec<char>| {
            let mut chars = chars;
            chars.reverse();
            chars.into_iter().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>()
        };

        let result2 = string_chars_iso.modify(source, composed);

        prop_assert_eq!(result1, result2);
    }

    // =========================================================================
    // to_lens Laws (Lens laws for IsoAsLens)
    // =========================================================================

    #[test]
    fn prop_iso_as_lens_get_put_law(value: i32) {
        #[derive(Clone, PartialEq, Debug)]
        struct Wrapper(i32);

        let wrapper_iso = FunctionIso::new(
            |w: Wrapper| w.0,
            |value: i32| Wrapper(value)
        );

        let as_lens = wrapper_iso.to_lens();

        let source = Wrapper(value);
        let gotten = as_lens.get(&source).clone();
        let result = as_lens.set(source.clone(), gotten);
        prop_assert_eq!(result, source);
    }

    #[test]
    fn prop_iso_as_lens_put_get_law(original: i32, new_value: i32) {
        #[derive(Clone, PartialEq, Debug)]
        struct Wrapper(i32);

        let wrapper_iso = FunctionIso::new(
            |w: Wrapper| w.0,
            |value: i32| Wrapper(value)
        );

        let as_lens = wrapper_iso.to_lens();

        let source = Wrapper(original);
        let updated = as_lens.set(source, new_value);
        prop_assert_eq!(*as_lens.get(&updated), new_value);
    }

    #[test]
    fn prop_iso_as_lens_put_put_law(original: i32, value1: i32, value2: i32) {
        #[derive(Clone, PartialEq, Debug)]
        struct Wrapper(i32);

        let wrapper_iso = FunctionIso::new(
            |w: Wrapper| w.0,
            |value: i32| Wrapper(value)
        );

        let as_lens = wrapper_iso.to_lens();

        let source = Wrapper(original);
        let left = as_lens.set(as_lens.set(source.clone(), value1), value2);
        let right = as_lens.set(source, value2);
        prop_assert_eq!(left, right);
    }

    // =========================================================================
    // to_prism Laws (Prism laws for IsoAsPrism)
    // =========================================================================

    #[test]
    fn prop_iso_as_prism_preview_review_law(value: i32) {
        #[derive(Clone, PartialEq, Debug)]
        struct Wrapper(i32);

        let wrapper_iso = FunctionIso::new(
            |w: Wrapper| w.0,
            |value: i32| Wrapper(value)
        );

        let as_prism = wrapper_iso.to_prism();

        let source = as_prism.review(value);
        let previewed = as_prism.preview(&source);
        prop_assert_eq!(previewed, Some(&value));
    }

    #[test]
    fn prop_iso_as_prism_review_preview_law(value: i32) {
        #[derive(Clone, PartialEq, Debug)]
        struct Wrapper(i32);

        let wrapper_iso = FunctionIso::new(
            |w: Wrapper| w.0,
            |value: i32| Wrapper(value)
        );

        let as_prism = wrapper_iso.to_prism();

        let source = Wrapper(value);
        if let Some(previewed_value) = as_prism.preview(&source) {
            let reconstructed = as_prism.review(previewed_value.clone());
            prop_assert_eq!(reconstructed, source);
        }
    }
}
