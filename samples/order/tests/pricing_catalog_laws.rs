//! PricingCatalog proptest law verification
//!
//! Verifies PricingCatalog invariants and laws using proptest.

use order_taking_sample::simple_types::{Price, ProductCode};
use order_taking_sample::workflow::PricingCatalog;
use proptest::prelude::*;
use rust_decimal::Decimal;

// =============================================================================
// Arbitrary implementations
// =============================================================================

/// Arbitrary generation for ProductCode
fn arb_product_code() -> impl Strategy<Value = ProductCode> {
    prop_oneof![
        (1000u32..10000u32).prop_map(|n| ProductCode::create("field", &format!("W{n}")).unwrap()),
        (100u32..1000u32).prop_map(|n| ProductCode::create("field", &format!("G{n}")).unwrap()),
    ]
}

/// Arbitrary generation for Price
fn arb_price() -> impl Strategy<Value = Price> {
    (0u32..1000u32).prop_map(|n| Price::create(Decimal::from(n)).unwrap())
}

/// Arbitrary generation for PricingCatalog
fn arb_pricing_catalog() -> impl Strategy<Value = PricingCatalog> {
    prop::collection::vec((arb_product_code(), arb_price()), 0..10)
        .prop_map(PricingCatalog::from_entries)
}

// =============================================================================
// lawverification
// =============================================================================

proptest! {
    /// Verify a price set with set_price can be retrieved with get_price
    #[test]
    fn test_set_get_roundtrip(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new().set_price(&code, price);
        prop_assert_eq!(catalog.get_price(&code), Some(&price));
    }

    /// Verify setting the same price for the same product code yields the same result
    #[test]
    fn test_set_idempotent(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog1 = PricingCatalog::new().set_price(&code, price);
        let catalog2 = catalog1.set_price(&code, price);

        prop_assert_eq!(catalog1.get_price(&code), catalog2.get_price(&code));
    }

    /// Verify remove_price after set_price results in None
    #[test]
    fn test_remove_after_set(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new()
            .set_price(&code, price)
            .remove_price(&code);

        prop_assert!(catalog.get_price(&code).is_none());
    }

    /// Verify the associativity law of merge
    /// (a.merge(b)).merge(c) and a.merge(b.merge(c)) yield the same result
    #[test]
    fn test_merge_associativity(
        catalog_a in arb_pricing_catalog(),
        catalog_b in arb_pricing_catalog(),
        catalog_c in arb_pricing_catalog()
    ) {
        let left = catalog_a.merge(&catalog_b).merge(&catalog_c);
        let right = catalog_a.merge(&catalog_b.merge(&catalog_c));

        // Verify both catalogs have the same keys and values
        prop_assert_eq!(left.len(), right.len());

        for (key, left_value) in left.iter() {
            let code = ProductCode::create("field", key).unwrap();
            let right_value = right.get_price(&code);
            prop_assert_eq!(Some(left_value), right_value);
        }
    }

    /// Verify merging with an empty catalog is an identity operation
    #[test]
    fn test_merge_identity(catalog in arb_pricing_catalog()) {
        let empty = PricingCatalog::new();
        let merged = catalog.merge(&empty);

        prop_assert_eq!(merged.len(), catalog.len());

        for (key, value) in catalog.iter() {
            let code = ProductCode::create("field", key).unwrap();
            prop_assert_eq!(merged.get_price(&code), Some(value));
        }
    }

    /// Verify merging an empty catalog from the left yields the same result
    #[test]
    fn test_merge_identity_left(catalog in arb_pricing_catalog()) {
        let empty = PricingCatalog::new();
        let merged = empty.merge(&catalog);

        prop_assert_eq!(merged.len(), catalog.len());

        for (key, value) in catalog.iter() {
            let code = ProductCode::create("field", key).unwrap();
            prop_assert_eq!(merged.get_price(&code), Some(value));
        }
    }

    /// Verify a value overwritten with set_price can be retrieved
    #[test]
    fn test_set_overwrites_previous(
        code in arb_product_code(),
        price1 in arb_price(),
        price2 in arb_price()
    ) {
        let catalog = PricingCatalog::new()
            .set_price(&code, price1)
            .set_price(&code, price2);

        prop_assert_eq!(catalog.get_price(&code), Some(&price2));
    }

    /// Verify contains returns true after set_price
    #[test]
    fn test_contains_after_set(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new().set_price(&code, price);
        prop_assert!(catalog.contains(&code));
    }

    /// Verify contains returns false after remove_price
    #[test]
    fn test_not_contains_after_remove(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new()
            .set_price(&code, price)
            .remove_price(&code);

        prop_assert!(!catalog.contains(&code));
    }

    /// Verify len increases with set_price (when the key is new)
    #[test]
    fn test_len_increases_on_new_key(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new();
        let updated = catalog.set_price(&code, price);

        prop_assert_eq!(catalog.len(), 0);
        prop_assert_eq!(updated.len(), 1);
    }

    /// is_empty is true only for an empty catalog
    #[test]
    fn test_is_empty_consistency(catalog in arb_pricing_catalog()) {
        prop_assert_eq!(catalog.is_empty(), catalog.len() == 0);
    }
}
