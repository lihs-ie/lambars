//! Address compound type

use lambars_derive::Lenses;

use crate::simple_types::{String50, UsStateCode, ValidationError, ZipCode};

/// Address compound type
///
/// Assumes US address format with multiple address lines (some optional),
/// city, ZIP code, state, and country.
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
    /// Creates an `Address` from strings
    ///
    /// `address_line2` through `address_line4` become `None` for empty strings.
    ///
    /// # Arguments
    ///
    /// * `address_line1` - Address line 1 (required)
    /// * `address_line2` - Address line 2 (`None` for empty string)
    /// * `address_line3` - Address line 3 (`None` for empty string)
    /// * `address_line4` - Address line 4 (`None` for empty string)
    /// * `city` - City (required)
    /// * `zip_code` - ZIP code (5 digits, required)
    /// * `state` - State code (2 characters, required)
    /// * `country` - Country name (required)
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - On successful validation
    /// * `Err(ValidationError)` - If any field is invalid
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when a required field is invalid or an optional field exceeds 50 characters.
    /// returns `ValidationError`.
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::Address;
    ///
    /// // All fields specified
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
    /// // Optional fields omitted
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

    /// Creates an `Address` from pre-validated components
    ///
    /// No validation needed (each type is already validated).
    ///
    /// # Arguments
    ///
    /// * `address_line1` - address line 1
    /// * `address_line2` - address line 2 (optional)
    /// * `address_line3` - address line 3 (optional)
    /// * `address_line4` - address line 4 (optional)
    /// * `city` - City
    /// * `zip_code` - ZIP code
    /// * `state` - State code
    /// * `country` - Country name
    ///
    /// # Returns
    ///
    /// A new `Address` instance
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

    /// Returns a reference to address line 1
    ///
    /// # Returns
    ///
    /// A reference to the `String50` address line 1
    #[must_use]
    pub const fn address_line1(&self) -> &String50 {
        &self.address_line1
    }

    /// Returns a reference to address line 2 (optional)
    ///
    /// # Returns
    ///
    /// Returns `Some(&String50)` if address line 2 is set, otherwise `None`
    #[must_use]
    pub const fn address_line2(&self) -> Option<&String50> {
        self.address_line2.as_ref()
    }

    /// Returns a reference to address line 3 (optional)
    ///
    /// # Returns
    ///
    /// Returns `Some(&String50)` if address line 3 is set, otherwise `None`
    #[must_use]
    pub const fn address_line3(&self) -> Option<&String50> {
        self.address_line3.as_ref()
    }

    /// Returns a reference to address line 4 (optional)
    ///
    /// # Returns
    ///
    /// Returns `Some(&String50)` if address line 4 is set, otherwise `None`
    #[must_use]
    pub const fn address_line4(&self) -> Option<&String50> {
        self.address_line4.as_ref()
    }

    /// Returns a reference to the city
    ///
    /// # Returns
    ///
    /// A reference to the `String50` city
    #[must_use]
    pub const fn city(&self) -> &String50 {
        &self.city
    }

    /// Returns a reference to the ZIP code
    ///
    /// # Returns
    ///
    /// A reference to the `ZipCode` ZIP code
    #[must_use]
    pub const fn zip_code(&self) -> &ZipCode {
        &self.zip_code
    }

    /// Returns a reference to the state code
    ///
    /// # Returns
    ///
    /// A reference to the `UsStateCode` state code
    #[must_use]
    pub const fn state(&self) -> &UsStateCode {
        &self.state
    }

    /// Returns a reference to the country name
    ///
    /// # Returns
    ///
    /// A reference to the `String50` country name
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
    // Tests for create
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
            "1234", // 4 digits
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
            "XX", // Invalid state code
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
    // Tests for create_from_parts
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
    // Tests for getters
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
    // Tests for Clone/Eq
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
    // Tests for Lens
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
