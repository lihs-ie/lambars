//! PricingCatalog proptest 法則検証
//!
//! PricingCatalog の不変条件と法則を proptest で検証する。

use order_taking_sample::simple_types::{Price, ProductCode};
use order_taking_sample::workflow::PricingCatalog;
use proptest::prelude::*;
use rust_decimal::Decimal;

// =============================================================================
// Arbitrary 実装
// =============================================================================

/// ProductCode の Arbitrary 生成
fn arb_product_code() -> impl Strategy<Value = ProductCode> {
    prop_oneof![
        (1000u32..10000u32).prop_map(|n| ProductCode::create("field", &format!("W{n}")).unwrap()),
        (100u32..1000u32).prop_map(|n| ProductCode::create("field", &format!("G{n}")).unwrap()),
    ]
}

/// Price の Arbitrary 生成
fn arb_price() -> impl Strategy<Value = Price> {
    (0u32..1000u32).prop_map(|n| Price::create(Decimal::from(n)).unwrap())
}

/// PricingCatalog の Arbitrary 生成
fn arb_pricing_catalog() -> impl Strategy<Value = PricingCatalog> {
    prop::collection::vec((arb_product_code(), arb_price()), 0..10)
        .prop_map(PricingCatalog::from_entries)
}

// =============================================================================
// 法則検証
// =============================================================================

proptest! {
    /// set_price した価格を get_price で取得できることを検証
    #[test]
    fn test_set_get_roundtrip(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new().set_price(&code, price);
        prop_assert_eq!(catalog.get_price(&code), Some(&price));
    }

    /// 同じ商品コードに同じ価格を設定しても結果が同じことを検証
    #[test]
    fn test_set_idempotent(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog1 = PricingCatalog::new().set_price(&code, price);
        let catalog2 = catalog1.set_price(&code, price);

        prop_assert_eq!(catalog1.get_price(&code), catalog2.get_price(&code));
    }

    /// set_price した後に remove_price すると None になることを検証
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

    /// merge の結合法則を検証
    /// (a.merge(b)).merge(c) と a.merge(b.merge(c)) は同じ結果を持つ
    #[test]
    fn test_merge_associativity(
        catalog_a in arb_pricing_catalog(),
        catalog_b in arb_pricing_catalog(),
        catalog_c in arb_pricing_catalog()
    ) {
        let left = catalog_a.merge(&catalog_b).merge(&catalog_c);
        let right = catalog_a.merge(&catalog_b.merge(&catalog_c));

        // 両方のカタログが同じキーと値を持つことを確認
        prop_assert_eq!(left.len(), right.len());

        for (key, left_value) in left.iter() {
            let code = ProductCode::create("field", key).unwrap();
            let right_value = right.get_price(&code);
            prop_assert_eq!(Some(left_value), right_value);
        }
    }

    /// 空のカタログとのマージは恒等写像であることを検証
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

    /// 空カタログを左から merge しても同じ結果であることを検証
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

    /// set_price で上書きした値が取得できることを検証
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

    /// contains は set_price 後に true になることを検証
    #[test]
    fn test_contains_after_set(
        code in arb_product_code(),
        price in arb_price()
    ) {
        let catalog = PricingCatalog::new().set_price(&code, price);
        prop_assert!(catalog.contains(&code));
    }

    /// contains は remove_price 後に false になることを検証
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

    /// len は set_price で増加することを検証（新規キーの場合）
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

    /// is_empty は空のカタログでのみ true
    #[test]
    fn test_is_empty_consistency(catalog in arb_pricing_catalog()) {
        prop_assert_eq!(catalog.is_empty(), catalog.len() == 0);
    }
}
