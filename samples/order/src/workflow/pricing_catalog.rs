//! 不変の価格カタログ
//!
//! `PersistentHashMap` を使用した不変の価格カタログを提供する。
//! 商品コードから価格へのマッピングを管理し、不変更新をサポートする。
//!
//! # 設計原則
//!
//! - 不変性: 全ての更新操作は新しいカタログを返し、元のカタログは変更されない
//! - 構造的共有: `PersistentHashMap` により効率的な不変更新を実現
//! - 型安全性: `ProductCode` をキーとして使用し、型の誤用を防ぐ
//!
//! # 使用例
//!
//! ```
//! use order_taking_sample::workflow::PricingCatalog;
//! use order_taking_sample::simple_types::{ProductCode, Price};
//! use rust_decimal::Decimal;
//!
//! let catalog = PricingCatalog::new();
//! let widget_code = ProductCode::create("field", "W1234").unwrap();
//! let price = Price::create(Decimal::from(100)).unwrap();
//!
//! let updated_catalog = catalog.set_price(&widget_code, price);
//!
//! // 元のカタログは空のまま
//! assert!(catalog.is_empty());
//!
//! // 新しいカタログには価格が設定されている
//! assert_eq!(updated_catalog.len(), 1);
//! ```

use crate::simple_types::{Price, ProductCode};
use lambars::persistent::PersistentHashMap;
use std::rc::Rc;

// =============================================================================
// PricingCatalog 型
// =============================================================================

/// 不変の価格カタログ
///
/// `PersistentHashMap` を使用して商品コードから価格へのマッピングを管理する。
/// 全ての更新操作は新しいカタログを返し、元のカタログは変更されない。
///
/// # 構造的共有
///
/// 内部で使用される `PersistentHashMap` は HAMT（Hash Array Mapped Trie）ベースで、
/// 更新時に変更されない部分は共有される。これにより、効率的な不変更新が可能。
///
/// # 時間計算量
///
/// | 操作 | 計算量 |
/// |------|--------|
/// | get_price | O(log32 N) |
/// | set_price | O(log32 N) |
/// | remove_price | O(log32 N) |
/// | merge | O(N + M) |
///
#[derive(Clone, Debug)]
pub struct PricingCatalog {
    prices: PersistentHashMap<String, Price>,
}

impl Default for PricingCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl PricingCatalog {
    /// 空の価格カタログを作成する
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingCatalog;
    ///
    /// let catalog = PricingCatalog::new();
    /// assert!(catalog.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            prices: PersistentHashMap::new(),
        }
    }

    /// 単一のエントリを持つ価格カタログを作成する
    ///
    /// # Arguments
    ///
    /// * `product_code` - 商品コード
    /// * `price` - 価格
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingCatalog;
    /// use order_taking_sample::simple_types::{ProductCode, Price};
    /// use rust_decimal::Decimal;
    ///
    /// let code = ProductCode::create("field", "W1234").unwrap();
    /// let price = Price::create(Decimal::from(100)).unwrap();
    /// let catalog = PricingCatalog::singleton(&code, price);
    ///
    /// assert_eq!(catalog.len(), 1);
    /// ```
    #[must_use]
    pub fn singleton(product_code: &ProductCode, price: Price) -> Self {
        Self::new().set_price(product_code, price)
    }

    /// エントリのイテレータから価格カタログを作成する
    ///
    /// # Arguments
    ///
    /// * `entries` - (`ProductCode`, `Price`) のイテレータ
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingCatalog;
    /// use order_taking_sample::simple_types::{ProductCode, Price};
    /// use rust_decimal::Decimal;
    ///
    /// let entries = vec![
    ///     (ProductCode::create("field", "W1234").unwrap(), Price::create(Decimal::from(100)).unwrap()),
    ///     (ProductCode::create("field", "G123").unwrap(), Price::create(Decimal::from(50)).unwrap()),
    /// ];
    /// let catalog = PricingCatalog::from_entries(entries);
    ///
    /// assert_eq!(catalog.len(), 2);
    /// ```
    pub fn from_entries(entries: impl IntoIterator<Item = (ProductCode, Price)>) -> Self {
        entries
            .into_iter()
            .fold(Self::new(), |catalog, (code, price)| {
                catalog.set_price(&code, price)
            })
    }

    /// 指定した商品コードの価格を設定した新しいカタログを返す
    ///
    /// 元のカタログは変更されない（不変更新）。
    ///
    /// # Arguments
    ///
    /// * `product_code` - 商品コード
    /// * `price` - 価格
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingCatalog;
    /// use order_taking_sample::simple_types::{ProductCode, Price};
    /// use rust_decimal::Decimal;
    ///
    /// let catalog = PricingCatalog::new();
    /// let code = ProductCode::create("field", "W1234").unwrap();
    /// let price = Price::create(Decimal::from(100)).unwrap();
    ///
    /// let new_catalog = catalog.set_price(&code, price);
    ///
    /// assert!(catalog.is_empty()); // 元は変更されない
    /// assert_eq!(new_catalog.len(), 1);
    /// ```
    #[must_use]
    pub fn set_price(&self, product_code: &ProductCode, price: Price) -> Self {
        Self {
            prices: self.prices.insert(product_code.value().to_string(), price),
        }
    }

    /// 指定した商品コードの価格を取得する
    ///
    /// # Arguments
    ///
    /// * `product_code` - 商品コード
    ///
    /// # Returns
    ///
    /// 価格が存在する場合は `Some(&Price)`、存在しない場合は `None`
    #[must_use]
    pub fn get_price(&self, product_code: &ProductCode) -> Option<&Price> {
        self.prices.get(product_code.value())
    }

    /// 指定した商品コードのエントリを削除した新しいカタログを返す
    ///
    /// 元のカタログは変更されない（不変更新）。
    ///
    /// # Arguments
    ///
    /// * `product_code` - 商品コード
    #[must_use]
    pub fn remove_price(&self, product_code: &ProductCode) -> Self {
        Self {
            prices: self.prices.remove(product_code.value()),
        }
    }

    /// 指定した商品コードが存在するかを返す
    ///
    /// # Arguments
    ///
    /// * `product_code` - 商品コード
    #[must_use]
    pub fn contains(&self, product_code: &ProductCode) -> bool {
        self.prices.contains_key(product_code.value())
    }

    /// 2つのカタログをマージした新しいカタログを返す
    ///
    /// キーが重複する場合は other の値が優先される。
    /// 元のカタログは両方とも変更されない。
    ///
    /// # Arguments
    ///
    /// * `other` - マージするカタログ
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            prices: self.prices.merge(&other.prices),
        }
    }

    /// カタログ内のエントリ数を返す
    #[must_use]
    pub fn len(&self) -> usize {
        self.prices.len()
    }

    /// カタログが空かどうかを返す
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    /// カタログ内の全エントリのイテレータを返す
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Price)> {
        self.prices.iter()
    }
}

// =============================================================================
// create_catalog_pricing_function
// =============================================================================

/// `PricingCatalog` から価格取得関数を生成する
///
/// カタログに存在しない商品コードにはデフォルト価格を適用する。
///
/// # Arguments
///
/// * `catalog` - 価格カタログ
/// * `default_price` - カタログにない商品のデフォルト価格
///
/// # Returns
///
/// Clone 可能な価格取得関数
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PricingCatalog, create_catalog_pricing_function};
/// use order_taking_sample::simple_types::{ProductCode, Price};
/// use rust_decimal::Decimal;
///
/// let widget_code = ProductCode::create("field", "W1234").unwrap();
/// let widget_price = Price::create(Decimal::from(100)).unwrap();
/// let catalog = PricingCatalog::singleton(&widget_code, widget_price);
///
/// let default_price = Price::create(Decimal::from(10)).unwrap();
/// let get_price = create_catalog_pricing_function(catalog, default_price);
///
/// // カタログにある商品
/// assert_eq!(get_price(&widget_code).value(), Decimal::from(100));
///
/// // カタログにない商品はデフォルト価格
/// let unknown_code = ProductCode::create("field", "W9999").unwrap();
/// assert_eq!(get_price(&unknown_code).value(), Decimal::from(10));
/// ```
pub fn create_catalog_pricing_function(
    catalog: PricingCatalog,
    default_price: Price,
) -> impl Fn(&ProductCode) -> Price + Clone {
    let catalog = Rc::new(catalog);
    move |product_code: &ProductCode| {
        catalog
            .get_price(product_code)
            .copied()
            .unwrap_or(default_price)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use rust_decimal::Decimal;

    fn create_test_product_code(code: &str) -> ProductCode {
        ProductCode::create("field", code).unwrap()
    }

    fn create_test_price(value: u32) -> Price {
        Price::create(Decimal::from(value)).unwrap()
    }

    #[rstest]
    fn test_new_creates_empty_catalog() {
        let catalog = PricingCatalog::new();
        assert!(catalog.is_empty());
    }

    #[rstest]
    fn test_set_and_get_price() {
        let code = create_test_product_code("W1234");
        let price = create_test_price(100);
        let catalog = PricingCatalog::new().set_price(&code, price);

        assert_eq!(catalog.get_price(&code), Some(&price));
    }

    #[rstest]
    fn test_create_catalog_pricing_function_basic() {
        let code = create_test_product_code("W1234");
        let price = create_test_price(100);
        let default = create_test_price(10);
        let catalog = PricingCatalog::singleton(&code, price);

        let get_price = create_catalog_pricing_function(catalog, default);

        assert_eq!(get_price(&code).value(), Decimal::from(100));
    }
}
