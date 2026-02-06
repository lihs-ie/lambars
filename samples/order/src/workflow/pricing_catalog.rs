//! Immutable pricing catalog
//!
//! Provides an immutable pricing catalog using `PersistentHashMap`.
//! Manages product code to price mappings and supports immutable updates.
//!
//! # Design Principles
//!
//! - Immutability: all update operations return a new catalog; the original is not modified
//! - Structural sharing: achieves efficient immutable updates via `PersistentHashMap`
//! - Type safety: uses `ProductCode` as a key, preventing type misuse
//!
//! # Usage Examples
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
//! // The original catalog remains empty
//! assert!(catalog.is_empty());
//!
//! // The new catalog has prices set
//! assert_eq!(updated_catalog.len(), 1);
//! ```

use crate::simple_types::{Price, ProductCode};
use lambars::persistent::PersistentHashMap;
use std::rc::Rc;

// =============================================================================
// PricingCatalog type
// =============================================================================

/// Immutable pricing catalog
///
/// Manages product code to price mappings using `PersistentHashMap`.
/// All update operations return a new catalog; the original is not modified.
///
/// # Structural Sharing
///
/// The internal `PersistentHashMap` is based on HAMT (Hash Array Mapped Trie),
/// where unchanged parts are shared during updates. This enables efficient immutable updates.
///
/// # Time Complexity
///
/// | Operation | Complexity |
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
    /// Creates an empty pricing catalog
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

    /// Creates a pricing catalog with a single entry
    ///
    /// # Arguments
    ///
    /// * `product_code` - Product code
    /// * `price` - Price
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

    /// Creates a pricing catalog from an iterator of entries
    ///
    /// # Arguments
    ///
    /// * `entries` - Iterator of (`ProductCode`, `Price`) pairs
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

    /// Returns a new catalog with the price set for the specified product code
    ///
    /// The original catalog is not modified (immutable update).
    ///
    /// # Arguments
    ///
    /// * `product_code` - Product code
    /// * `price` - Price
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
    /// assert!(catalog.is_empty()); // The original is not modified
    /// assert_eq!(new_catalog.len(), 1);
    /// ```
    #[must_use]
    pub fn set_price(&self, product_code: &ProductCode, price: Price) -> Self {
        Self {
            prices: self.prices.insert(product_code.value().to_string(), price),
        }
    }

    /// Gets the price for the specified product code
    ///
    /// # Arguments
    ///
    /// * `product_code` - Product code
    ///
    /// # Returns
    ///
    /// Returns `Some(&Price)` if the price exists, `None` otherwise
    #[must_use]
    pub fn get_price(&self, product_code: &ProductCode) -> Option<&Price> {
        self.prices.get(product_code.value())
    }

    /// Returns a new catalog with the entry for the specified product code removed
    ///
    /// The original catalog is not modified (immutable update).
    ///
    /// # Arguments
    ///
    /// * `product_code` - Product code
    #[must_use]
    pub fn remove_price(&self, product_code: &ProductCode) -> Self {
        Self {
            prices: self.prices.remove(product_code.value()),
        }
    }

    /// Returns whether the specified product code exists
    ///
    /// # Arguments
    ///
    /// * `product_code` - Product code
    #[must_use]
    pub fn contains(&self, product_code: &ProductCode) -> bool {
        self.prices.contains_key(product_code.value())
    }

    /// Returns a new catalog merging two catalogs
    ///
    /// When keys overlap, values from `other` take precedence.
    /// Neither original catalog is modified.
    ///
    /// # Arguments
    ///
    /// * `other` - Catalog to merge
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            prices: self.prices.merge(&other.prices),
        }
    }

    /// Returns the number of entries in the catalog
    #[must_use]
    pub fn len(&self) -> usize {
        self.prices.len()
    }

    /// Returns whether the catalog is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    /// Returns an iterator over all entries in the catalog
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Price)> {
        self.prices.iter()
    }
}

// =============================================================================
// create_catalog_pricing_function
// =============================================================================

/// Generates a pricing function from a `PricingCatalog`
///
/// Applies a default price for product codes not found in the catalog.
///
/// # Arguments
///
/// * `catalog` - Pricing catalog
/// * `default_price` - Default price for products not in the catalog
///
/// # Returns
///
/// A cloneable pricing function
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
/// // Products in the catalog
/// assert_eq!(get_price(&widget_code).value(), Decimal::from(100));
///
/// // Products not in the catalog use the default price
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
