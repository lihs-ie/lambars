//! PricingCatalog tests
//!
//! Implements tests for the PricingCatalog type and create_catalog_pricing_function function.

use order_taking_sample::simple_types::{Price, ProductCode};
use order_taking_sample::workflow::{PricingCatalog, create_catalog_pricing_function};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// Test helpers
// =============================================================================

/// Creates a ProductCode for testing
fn create_test_product_code(code: &str) -> ProductCode {
    ProductCode::create("field", code).unwrap()
}

/// Creates a Price for testing
fn create_test_price(value: u32) -> Price {
    Price::create(Decimal::from(value)).unwrap()
}

// =============================================================================
// Basic operation tests
// =============================================================================

#[rstest]
fn test_pricing_catalog_new_is_empty() {
    let catalog = PricingCatalog::new();
    assert!(catalog.is_empty());
    assert_eq!(catalog.len(), 0);
}

#[rstest]
fn test_pricing_catalog_default_is_empty() {
    let catalog = PricingCatalog::default();
    assert!(catalog.is_empty());
    assert_eq!(catalog.len(), 0);
}

#[rstest]
fn test_pricing_catalog_singleton() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let catalog = PricingCatalog::singleton(&code, price);

    assert_eq!(catalog.len(), 1);
    assert!(catalog.contains(&code));
}

#[rstest]
fn test_pricing_catalog_set_price() {
    let catalog = PricingCatalog::new();
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);

    let new_catalog = catalog.set_price(&code, price);

    assert!(catalog.is_empty()); // The original is not modified
    assert_eq!(new_catalog.len(), 1);
    assert_eq!(new_catalog.get_price(&code), Some(&price));
}

#[rstest]
fn test_pricing_catalog_get_price_existing() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let catalog = PricingCatalog::singleton(&code, price);

    let retrieved = catalog.get_price(&code);
    assert_eq!(retrieved, Some(&price));
}

#[rstest]
fn test_pricing_catalog_get_price_not_found() {
    let catalog = PricingCatalog::new();
    let code = create_test_product_code("W1234");

    assert_eq!(catalog.get_price(&code), None);
}

#[rstest]
fn test_pricing_catalog_remove_price() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let catalog = PricingCatalog::singleton(&code, price);

    let new_catalog = catalog.remove_price(&code);

    assert_eq!(catalog.len(), 1); // Original is not modified
    assert!(new_catalog.is_empty());
}

#[rstest]
fn test_pricing_catalog_contains() {
    let code = create_test_product_code("W1234");
    let other_code = create_test_product_code("G123");
    let price = create_test_price(100);
    let catalog = PricingCatalog::singleton(&code, price);

    assert!(catalog.contains(&code));
    assert!(!catalog.contains(&other_code));
}

// =============================================================================
// from_entries and iter tests
// =============================================================================

#[rstest]
fn test_pricing_catalog_from_entries() {
    let entries = vec![
        (create_test_product_code("W1234"), create_test_price(100)),
        (create_test_product_code("G123"), create_test_price(50)),
    ];
    let catalog = PricingCatalog::from_entries(entries);

    assert_eq!(catalog.len(), 2);
}

#[rstest]
fn test_pricing_catalog_iter() {
    let widget_code = create_test_product_code("W1234");
    let gizmo_code = create_test_product_code("G123");
    let widget_price = create_test_price(100);
    let gizmo_price = create_test_price(50);

    let catalog = PricingCatalog::new()
        .set_price(&widget_code, widget_price)
        .set_price(&gizmo_code, gizmo_price);

    let entries: Vec<_> = catalog.iter().collect();
    assert_eq!(entries.len(), 2);

    // Verify the key is contained
    let keys: Vec<&String> = entries.iter().map(|(key, _)| *key).collect();
    assert!(keys.contains(&&"W1234".to_string()));
    assert!(keys.contains(&&"G123".to_string()));
}

// =============================================================================
// Merge operation tests
// =============================================================================

#[rstest]
fn test_pricing_catalog_merge_disjoint() {
    let widget_code = create_test_product_code("W1234");
    let gizmo_code = create_test_product_code("G123");
    let widget_price = create_test_price(100);
    let gizmo_price = create_test_price(50);

    let catalog1 = PricingCatalog::singleton(&widget_code, widget_price);
    let catalog2 = PricingCatalog::singleton(&gizmo_code, gizmo_price);

    let merged = catalog1.merge(&catalog2);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged.get_price(&widget_code), Some(&widget_price));
    assert_eq!(merged.get_price(&gizmo_code), Some(&gizmo_price));
}

#[rstest]
fn test_pricing_catalog_merge_overlapping() {
    let code = create_test_product_code("W1234");
    let price1 = create_test_price(100);
    let price2 = create_test_price(200);

    let catalog1 = PricingCatalog::singleton(&code, price1);
    let catalog2 = PricingCatalog::singleton(&code, price2);

    // Other's value takes priority
    let merged = catalog1.merge(&catalog2);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged.get_price(&code), Some(&price2));
}

#[rstest]
fn test_pricing_catalog_merge_empty() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let catalog = PricingCatalog::singleton(&code, price);
    let empty = PricingCatalog::new();

    // Merging with empty is same as original
    let merged_with_empty = catalog.merge(&empty);
    assert_eq!(merged_with_empty.len(), 1);
    assert_eq!(merged_with_empty.get_price(&code), Some(&price));

    // Merging empty with non-empty
    let empty_merged_with_catalog = empty.merge(&catalog);
    assert_eq!(empty_merged_with_catalog.len(), 1);
    assert_eq!(empty_merged_with_catalog.get_price(&code), Some(&price));
}

// =============================================================================
// Immutability tests
// =============================================================================

#[rstest]
fn test_pricing_catalog_immutability_set_price() {
    let original = PricingCatalog::new();
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);

    let _updated = original.set_price(&code, price);

    // Original catalog is unchanged
    assert!(original.is_empty());
    assert_eq!(original.len(), 0);
    assert_eq!(original.get_price(&code), None);
}

#[rstest]
fn test_pricing_catalog_immutability_remove_price() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let original = PricingCatalog::singleton(&code, price);

    let _updated = original.remove_price(&code);

    // Original catalog is unchanged
    assert_eq!(original.len(), 1);
    assert!(original.contains(&code));
    assert_eq!(original.get_price(&code), Some(&price));
}

#[rstest]
fn test_pricing_catalog_immutability_merge() {
    let code1 = create_test_product_code("W1234");
    let code2 = create_test_product_code("G123");
    let price1 = create_test_price(100);
    let price2 = create_test_price(50);

    let catalog1 = PricingCatalog::singleton(&code1, price1);
    let catalog2 = PricingCatalog::singleton(&code2, price2);

    let _merged = catalog1.merge(&catalog2);

    // Both catalogs are unchanged
    assert_eq!(catalog1.len(), 1);
    assert_eq!(catalog1.get_price(&code1), Some(&price1));
    assert!(!catalog1.contains(&code2));

    assert_eq!(catalog2.len(), 1);
    assert_eq!(catalog2.get_price(&code2), Some(&price2));
    assert!(!catalog2.contains(&code1));
}

// =============================================================================
// Pricing function tests
// =============================================================================

#[rstest]
fn test_create_catalog_pricing_function_found() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let default_price = create_test_price(10);
    let catalog = PricingCatalog::singleton(&code, price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    // Products in the catalog return the catalog price
    assert_eq!(get_price(&code).value(), Decimal::from(100));
}

#[rstest]
fn test_create_catalog_pricing_function_not_found() {
    let known_code = create_test_product_code("W1234");
    let unknown_code = create_test_product_code("W9999");
    let price = create_test_price(100);
    let default_price = create_test_price(10);
    let catalog = PricingCatalog::singleton(&known_code, price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    // Products not in the catalog return the default price
    assert_eq!(get_price(&unknown_code).value(), Decimal::from(10));
}

#[rstest]
fn test_create_catalog_pricing_function_clone() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let default_price = create_test_price(10);
    let catalog = PricingCatalog::singleton(&code, price);

    let get_price = create_catalog_pricing_function(catalog, default_price);
    let cloned_get_price = get_price.clone();

    // Both return the same result
    assert_eq!(get_price(&code).value(), Decimal::from(100));
    assert_eq!(cloned_get_price(&code).value(), Decimal::from(100));
}

#[rstest]
fn test_create_catalog_pricing_function_multiple_entries() {
    let widget_code = create_test_product_code("W1234");
    let gizmo_code = create_test_product_code("G123");
    let widget_price = create_test_price(100);
    let gizmo_price = create_test_price(50);
    let default_price = create_test_price(10);

    let catalog = PricingCatalog::new()
        .set_price(&widget_code, widget_price)
        .set_price(&gizmo_code, gizmo_price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    assert_eq!(get_price(&widget_code).value(), Decimal::from(100));
    assert_eq!(get_price(&gizmo_code).value(), Decimal::from(50));

    let unknown_code = create_test_product_code("W9999");
    assert_eq!(get_price(&unknown_code).value(), Decimal::from(10));
}
