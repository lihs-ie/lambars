//! PricingCatalog テスト
//!
//! PricingCatalog 型と create_catalog_pricing_function 関数のテストを実装する。

use order_taking_sample::simple_types::{Price, ProductCode};
use order_taking_sample::workflow::{PricingCatalog, create_catalog_pricing_function};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// テストヘルパー
// =============================================================================

/// テスト用の ProductCode を作成する
fn create_test_product_code(code: &str) -> ProductCode {
    ProductCode::create("field", code).unwrap()
}

/// テスト用の Price を作成する
fn create_test_price(value: u32) -> Price {
    Price::create(Decimal::from(value)).unwrap()
}

// =============================================================================
// 基本操作テスト
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

    assert!(catalog.is_empty()); // 元は変更されない
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

    assert_eq!(catalog.len(), 1); // 元は変更されない
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
// from_entries と iter テスト
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

    // キーが含まれていることを確認
    let keys: Vec<&String> = entries.iter().map(|(key, _)| *key).collect();
    assert!(keys.contains(&&"W1234".to_string()));
    assert!(keys.contains(&&"G123".to_string()));
}

// =============================================================================
// マージ操作テスト
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

    // other の値が優先される
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

    // 空とのマージは元と同じ
    let merged_with_empty = catalog.merge(&empty);
    assert_eq!(merged_with_empty.len(), 1);
    assert_eq!(merged_with_empty.get_price(&code), Some(&price));

    // 空に対するマージ
    let empty_merged_with_catalog = empty.merge(&catalog);
    assert_eq!(empty_merged_with_catalog.len(), 1);
    assert_eq!(empty_merged_with_catalog.get_price(&code), Some(&price));
}

// =============================================================================
// 不変性テスト
// =============================================================================

#[rstest]
fn test_pricing_catalog_immutability_set_price() {
    let original = PricingCatalog::new();
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);

    let _updated = original.set_price(&code, price);

    // 元のカタログは変更されていない
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

    // 元のカタログは変更されていない
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

    // 両方のカタログは変更されていない
    assert_eq!(catalog1.len(), 1);
    assert_eq!(catalog1.get_price(&code1), Some(&price1));
    assert!(!catalog1.contains(&code2));

    assert_eq!(catalog2.len(), 1);
    assert_eq!(catalog2.get_price(&code2), Some(&price2));
    assert!(!catalog2.contains(&code1));
}

// =============================================================================
// 価格取得関数テスト
// =============================================================================

#[rstest]
fn test_create_catalog_pricing_function_found() {
    let code = create_test_product_code("W1234");
    let price = create_test_price(100);
    let default_price = create_test_price(10);
    let catalog = PricingCatalog::singleton(&code, price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    // カタログにある商品はカタログの価格を返す
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

    // カタログにない商品はデフォルト価格を返す
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

    // 両方とも同じ結果を返す
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
