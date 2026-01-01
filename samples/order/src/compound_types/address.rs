//! 住所を表す複合型

use lambars_derive::Lenses;

use crate::simple_types::{String50, UsStateCode, ValidationError, ZipCode};

/// 住所を表す複合型
///
/// 米国の住所形式を想定し、複数の住所行（一部はオプショナル）、
/// 市、郵便番号、州、国を持つ。
///
/// # Examples
///
/// ```
/// use order_taking_sample::compound_types::Address;
///
/// let address = Address::create(
///     "123 Main St",
///     "Apt 4B",
///     "",
///     "",
///     "New York",
///     "10001",
///     "NY",
///     "USA",
/// ).unwrap();
///
/// assert_eq!(address.address_line1().value(), "123 Main St");
/// assert_eq!(address.address_line2().map(|s| s.value()), Some("Apt 4B"));
/// assert!(address.address_line3().is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Lenses)]
#[allow(clippy::struct_field_names)]
pub struct Address {
    address_line1: String50,
    address_line2: Option<String50>,
    address_line3: Option<String50>,
    address_line4: Option<String50>,
    city: String50,
    zip_code: ZipCode,
    state: UsStateCode,
    country: String50,
}

impl Address {
    /// 文字列から `Address` を生成する
    ///
    /// `address_line2` から `address_line4` は空文字列の場合 `None` になる。
    ///
    /// # Arguments
    ///
    /// * `address_line1` - 住所行1（必須）
    /// * `address_line2` - 住所行2（空文字列で `None`）
    /// * `address_line3` - 住所行3（空文字列で `None`）
    /// * `address_line4` - 住所行4（空文字列で `None`）
    /// * `city` - 市（必須）
    /// * `zip_code` - 郵便番号（5桁、必須）
    /// * `state` - 州コード（2文字、必須）
    /// * `country` - 国名（必須）
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - バリデーション成功時
    /// * `Err(ValidationError)` - いずれかのフィールドが無効な場合
    ///
    /// # Errors
    ///
    /// 必須フィールドが無効な場合、またはオプショナルフィールドが50文字を超える場合に
    /// `ValidationError` を返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::Address;
    ///
    /// // 全フィールド指定
    /// let address = Address::create(
    ///     "123 Main St",
    ///     "Apt 4B",
    ///     "Building A",
    ///     "Floor 5",
    ///     "New York",
    ///     "10001",
    ///     "NY",
    ///     "USA",
    /// ).unwrap();
    ///
    /// // オプショナルフィールドを省略
    /// let address2 = Address::create(
    ///     "456 Oak Ave",
    ///     "",
    ///     "",
    ///     "",
    ///     "Los Angeles",
    ///     "90001",
    ///     "CA",
    ///     "USA",
    /// ).unwrap();
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        address_line1: &str,
        address_line2: &str,
        address_line3: &str,
        address_line4: &str,
        city: &str,
        zip_code: &str,
        state: &str,
        country: &str,
    ) -> Result<Self, ValidationError> {
        let address_line1_validated = String50::create("AddressLine1", address_line1)?;
        let address_line2_validated = String50::create_option("AddressLine2", address_line2)?;
        let address_line3_validated = String50::create_option("AddressLine3", address_line3)?;
        let address_line4_validated = String50::create_option("AddressLine4", address_line4)?;
        let city_validated = String50::create("City", city)?;
        let zip_code_validated = ZipCode::create("ZipCode", zip_code)?;
        let state_validated = UsStateCode::create("State", state)?;
        let country_validated = String50::create("Country", country)?;

        Ok(Self {
            address_line1: address_line1_validated,
            address_line2: address_line2_validated,
            address_line3: address_line3_validated,
            address_line4: address_line4_validated,
            city: city_validated,
            zip_code: zip_code_validated,
            state: state_validated,
            country: country_validated,
        })
    }

    /// 既にバリデーション済みの構成要素から `Address` を生成する
    ///
    /// バリデーションは不要（既に各型でバリデーション済み）。
    ///
    /// # Arguments
    ///
    /// * `address_line1` - 住所行1
    /// * `address_line2` - 住所行2（オプショナル）
    /// * `address_line3` - 住所行3（オプショナル）
    /// * `address_line4` - 住所行4（オプショナル）
    /// * `city` - 市
    /// * `zip_code` - 郵便番号
    /// * `state` - 州コード
    /// * `country` - 国名
    ///
    /// # Returns
    ///
    /// 新しい `Address` インスタンス
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn create_from_parts(
        address_line1: String50,
        address_line2: Option<String50>,
        address_line3: Option<String50>,
        address_line4: Option<String50>,
        city: String50,
        zip_code: ZipCode,
        state: UsStateCode,
        country: String50,
    ) -> Self {
        Self {
            address_line1,
            address_line2,
            address_line3,
            address_line4,
            city,
            zip_code,
            state,
            country,
        }
    }

    /// 住所行1への参照を返す
    ///
    /// # Returns
    ///
    /// `String50` 型の住所行1への参照
    #[must_use]
    pub const fn address_line1(&self) -> &String50 {
        &self.address_line1
    }

    /// 住所行2への参照を返す（オプショナル）
    ///
    /// # Returns
    ///
    /// 住所行2が設定されている場合は `Some(&String50)`、そうでなければ `None`
    #[must_use]
    pub const fn address_line2(&self) -> Option<&String50> {
        self.address_line2.as_ref()
    }

    /// 住所行3への参照を返す（オプショナル）
    ///
    /// # Returns
    ///
    /// 住所行3が設定されている場合は `Some(&String50)`、そうでなければ `None`
    #[must_use]
    pub const fn address_line3(&self) -> Option<&String50> {
        self.address_line3.as_ref()
    }

    /// 住所行4への参照を返す（オプショナル）
    ///
    /// # Returns
    ///
    /// 住所行4が設定されている場合は `Some(&String50)`、そうでなければ `None`
    #[must_use]
    pub const fn address_line4(&self) -> Option<&String50> {
        self.address_line4.as_ref()
    }

    /// 市への参照を返す
    ///
    /// # Returns
    ///
    /// `String50` 型の市への参照
    #[must_use]
    pub const fn city(&self) -> &String50 {
        &self.city
    }

    /// 郵便番号への参照を返す
    ///
    /// # Returns
    ///
    /// `ZipCode` 型の郵便番号への参照
    #[must_use]
    pub const fn zip_code(&self) -> &ZipCode {
        &self.zip_code
    }

    /// 州コードへの参照を返す
    ///
    /// # Returns
    ///
    /// `UsStateCode` 型の州コードへの参照
    #[must_use]
    pub const fn state(&self) -> &UsStateCode {
        &self.state
    }

    /// 国名への参照を返す
    ///
    /// # Returns
    ///
    /// `String50` 型の国名への参照
    #[must_use]
    pub const fn country(&self) -> &String50 {
        &self.country
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambars::optics::Lens;
    use rstest::rstest;

    // =========================================================================
    // create のテスト
    // =========================================================================

    #[rstest]
    fn test_address_create_valid_all_fields() {
        let result = Address::create(
            "123 Main St",
            "Apt 4B",
            "Building A",
            "Floor 5",
            "New York",
            "10001",
            "NY",
            "USA",
        );

        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(address.address_line1().value(), "123 Main St");
        assert_eq!(address.address_line2().map(|s| s.value()), Some("Apt 4B"));
        assert_eq!(
            address.address_line3().map(|s| s.value()),
            Some("Building A")
        );
        assert_eq!(address.address_line4().map(|s| s.value()), Some("Floor 5"));
        assert_eq!(address.city().value(), "New York");
        assert_eq!(address.zip_code().value(), "10001");
        assert_eq!(address.state().value(), "NY");
        assert_eq!(address.country().value(), "USA");
    }

    #[rstest]
    fn test_address_create_valid_required_only() {
        let result = Address::create(
            "456 Oak Ave",
            "",
            "",
            "",
            "Los Angeles",
            "90001",
            "CA",
            "USA",
        );

        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(address.address_line1().value(), "456 Oak Ave");
        assert!(address.address_line2().is_none());
        assert!(address.address_line3().is_none());
        assert!(address.address_line4().is_none());
        assert_eq!(address.city().value(), "Los Angeles");
    }

    #[rstest]
    fn test_address_create_valid_partial_optional() {
        let result = Address::create(
            "789 Pine Rd",
            "Suite 100",
            "",
            "",
            "Chicago",
            "60601",
            "IL",
            "USA",
        );

        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(
            address.address_line2().map(|s| s.value()),
            Some("Suite 100")
        );
        assert!(address.address_line3().is_none());
    }

    #[rstest]
    fn test_address_create_invalid_address_line1_empty() {
        let result = Address::create("", "", "", "", "New York", "10001", "NY", "USA");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "AddressLine1");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_address_create_invalid_city_empty() {
        let result = Address::create("123 Main St", "", "", "", "", "10001", "NY", "USA");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "City");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_address_create_invalid_zip_code() {
        let result = Address::create(
            "123 Main St",
            "",
            "",
            "",
            "New York",
            "1234", // 4桁
            "NY",
            "USA",
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ZipCode");
        assert!(error.message.contains("must match the pattern"));
    }

    #[rstest]
    fn test_address_create_invalid_state() {
        let result = Address::create(
            "123 Main St",
            "",
            "",
            "",
            "New York",
            "10001",
            "XX", // 無効な州コード
            "USA",
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "State");
        assert!(error.message.contains("must match the pattern"));
    }

    #[rstest]
    fn test_address_create_invalid_country_empty() {
        let result = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Country");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_address_create_invalid_address_line2_too_long() {
        let long_line = "a".repeat(51);
        let result = Address::create(
            "123 Main St",
            &long_line,
            "",
            "",
            "New York",
            "10001",
            "NY",
            "USA",
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "AddressLine2");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    // =========================================================================
    // create_from_parts のテスト
    // =========================================================================

    #[rstest]
    fn test_address_create_from_parts() {
        let address_line1 = String50::create("AddressLine1", "123 Main St").unwrap();
        let city = String50::create("City", "New York").unwrap();
        let zip_code = ZipCode::create("ZipCode", "10001").unwrap();
        let state = UsStateCode::create("State", "NY").unwrap();
        let country = String50::create("Country", "USA").unwrap();

        let address = Address::create_from_parts(
            address_line1,
            None,
            None,
            None,
            city,
            zip_code,
            state,
            country,
        );

        assert_eq!(address.address_line1().value(), "123 Main St");
        assert!(address.address_line2().is_none());
        assert_eq!(address.city().value(), "New York");
    }

    // =========================================================================
    // Getter のテスト
    // =========================================================================

    #[rstest]
    fn test_address_getters() {
        let address = Address::create(
            "123 Main St",
            "Apt 4B",
            "",
            "",
            "New York",
            "10001",
            "NY",
            "USA",
        )
        .unwrap();

        assert_eq!(address.address_line1().value(), "123 Main St");
        assert_eq!(address.address_line2().map(|s| s.value()), Some("Apt 4B"));
        assert!(address.address_line3().is_none());
        assert!(address.address_line4().is_none());
        assert_eq!(address.city().value(), "New York");
        assert_eq!(address.zip_code().value(), "10001");
        assert_eq!(address.state().value(), "NY");
        assert_eq!(address.country().value(), "USA");
    }

    // =========================================================================
    // Clone/Eq のテスト
    // =========================================================================

    #[rstest]
    fn test_address_clone() {
        let original = Address::create(
            "123 Main St",
            "Apt 4B",
            "",
            "",
            "New York",
            "10001",
            "NY",
            "USA",
        )
        .unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_address_eq() {
        let address1 =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let address2 =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let address3 = Address::create(
            "456 Oak Ave",
            "",
            "",
            "",
            "Los Angeles",
            "90001",
            "CA",
            "USA",
        )
        .unwrap();

        assert_eq!(address1, address2);
        assert_ne!(address1, address3);
    }

    // =========================================================================
    // Lens のテスト
    // =========================================================================

    #[rstest]
    fn test_address_address_line1_lens_get() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::address_line1_lens();

        assert_eq!(lens.get(&address).value(), "123 Main St");
    }

    #[rstest]
    fn test_address_address_line1_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::address_line1_lens();
        let new_line1 = String50::create("AddressLine1", "456 Oak Ave").unwrap();

        let updated = lens.set(address, new_line1);

        assert_eq!(updated.address_line1().value(), "456 Oak Ave");
        assert_eq!(updated.city().value(), "New York");
    }

    #[rstest]
    fn test_address_address_line2_lens_get() {
        let address = Address::create(
            "123 Main St",
            "Apt 4B",
            "",
            "",
            "New York",
            "10001",
            "NY",
            "USA",
        )
        .unwrap();
        let lens = Address::address_line2_lens();

        let value = lens.get(&address);
        assert!(value.is_some());
        assert_eq!(value.as_ref().unwrap().value(), "Apt 4B");
    }

    #[rstest]
    fn test_address_address_line2_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::address_line2_lens();
        let new_line2 = Some(String50::create("AddressLine2", "Suite 200").unwrap());

        let updated = lens.set(address, new_line2);

        assert_eq!(
            updated.address_line2().map(|s| s.value()),
            Some("Suite 200")
        );
    }

    #[rstest]
    fn test_address_city_lens_get() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::city_lens();

        assert_eq!(lens.get(&address).value(), "New York");
    }

    #[rstest]
    fn test_address_city_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::city_lens();
        let new_city = String50::create("City", "Los Angeles").unwrap();

        let updated = lens.set(address, new_city);

        assert_eq!(updated.city().value(), "Los Angeles");
        assert_eq!(updated.address_line1().value(), "123 Main St");
    }

    #[rstest]
    fn test_address_zip_code_lens_get() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::zip_code_lens();

        assert_eq!(lens.get(&address).value(), "10001");
    }

    #[rstest]
    fn test_address_zip_code_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::zip_code_lens();
        let new_zip = ZipCode::create("ZipCode", "90001").unwrap();

        let updated = lens.set(address, new_zip);

        assert_eq!(updated.zip_code().value(), "90001");
    }

    #[rstest]
    fn test_address_state_lens_get() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::state_lens();

        assert_eq!(lens.get(&address).value(), "NY");
    }

    #[rstest]
    fn test_address_state_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::state_lens();
        let new_state = UsStateCode::create("State", "CA").unwrap();

        let updated = lens.set(address, new_state);

        assert_eq!(updated.state().value(), "CA");
    }

    #[rstest]
    fn test_address_country_lens_get() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::country_lens();

        assert_eq!(lens.get(&address).value(), "USA");
    }

    #[rstest]
    fn test_address_country_lens_set() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let lens = Address::country_lens();
        let new_country = String50::create("Country", "Canada").unwrap();

        let updated = lens.set(address, new_country);

        assert_eq!(updated.country().value(), "Canada");
    }
}
