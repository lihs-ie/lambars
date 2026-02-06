//! Constrained string type definitions
//!
//! Defines `String50`, `EmailAddress`, `ZipCode`, and `UsStateCode`.

use regex::Regex;
use std::sync::LazyLock;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// String50
// =============================================================================

/// A string type constrained to 50 characters or fewer
///
/// Used for short string fields such as names and parts of addresses.
/// Empty strings are not allowed.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::String50;
///
/// let name = String50::create("CustomerName", "John Doe").unwrap();
/// assert_eq!(name.value(), "John Doe");
///
/// // Empty string causes an error
/// assert!(String50::create("CustomerName", "").is_err());
///
/// // 51 characters or more causes an error
/// let long_name = "a".repeat(51);
/// assert!(String50::create("CustomerName", &long_name).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct String50(String);

/// Maximum character count for String50
const STRING50_MAX_LENGTH: usize = 50;

impl String50 {
    /// Creates a String50 from a string
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(String50)` - On successful validation
    /// * `Err(ValidationError)` - When the string is empty or exceeds 50 characters
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when the string is empty or exceeds 50 characters.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_string(field_name, String50, STRING50_MAX_LENGTH, value)
    }

    /// Returns `None` for an empty string; otherwise, performs validation
    ///
    /// Used for optional fields.
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(None)` - For an empty string
    /// * `Ok(Some(String50))` - On successful validation
    /// * `Err(ValidationError)` - When the string exceeds 50 characters
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when the string exceeds 50 characters.
    pub fn create_option(field_name: &str, value: &str) -> Result<Option<Self>, ValidationError> {
        constrained_type::create_string_option(field_name, String50, STRING50_MAX_LENGTH, value)
    }

    /// Returns a reference to the inner stringvalue
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// EmailAddress
// =============================================================================

/// A string type constrained to email address format
///
/// Validates that the string contains at least an @ symbol.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::EmailAddress;
///
/// let email = EmailAddress::create("Email", "user@example.com").unwrap();
/// assert_eq!(email.value(), "user@example.com");
///
/// // Causes an error if it does not contain @
/// assert!(EmailAddress::create("Email", "invalid-email").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmailAddress(String);

/// Regex pattern for email addresses
/// .+@.+ : Matches anything@anything format
static EMAIL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.+@.+$").expect("Invalid email regex pattern"));

impl EmailAddress {
    /// Creates an `EmailAddress` from an email-formatted string
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(EmailAddress)` - On successful validation
    /// * `Err(ValidationError)` - When the string is empty or does not contain @
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when the string is empty or does not contain @.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, EmailAddress, &EMAIL_PATTERN, value)
    }

    /// Returns a reference to the inner Email addressstring
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// ZipCode
// =============================================================================

/// A type representing a 5-digit zip code
///
/// Assumes US ZIP code format.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ZipCode;
///
/// let zip = ZipCode::create("ZipCode", "12345").unwrap();
/// assert_eq!(zip.value(), "12345");
///
/// // 4 digits causes an error
/// assert!(ZipCode::create("ZipCode", "1234").is_err());
///
/// // 6 digits causes an error
/// assert!(ZipCode::create("ZipCode", "123456").is_err());
///
/// // Including letters causes an error
/// assert!(ZipCode::create("ZipCode", "1234A").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZipCode(String);

/// Regex pattern for `ZipCode` (5-digit number)
static ZIP_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{5}$").expect("Invalid zip code regex pattern"));

impl ZipCode {
    /// Creates a `ZipCode` from a 5-digit numeric string
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(ZipCode)` - On successful validation
    /// * `Err(ValidationError)` - When the string is not a 5-digit number
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` when the string is not a 5-digit number.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, ZipCode, &ZIP_CODE_PATTERN, value)
    }

    /// Returns a reference to the inner zip code string
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// UsStateCode
// =============================================================================

/// A type representing a 2-character US state code
///
/// Only accepts valid state codes.
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::UsStateCode;
///
/// let state = UsStateCode::create("State", "CA").unwrap();
/// assert_eq!(state.value(), "CA");
///
/// // Invalid state code causes an error
/// assert!(UsStateCode::create("State", "XX").is_err());
///
/// // Lowercase causes an error
/// assert!(UsStateCode::create("State", "ca").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UsStateCode(String);

/// Regex pattern for `UsStateCode` (valid US state codes)
/// AL, AK, AZ, AR, CA, CO, CT, DE, DC, FL, GA, HI, ID, IL, IN, IA, KS, KY, LA,
/// MA, MD, ME, MI, MN, MO, MS, MT, NC, ND, NE, NH, NJ, NM, NV, NY, OH, OK, OR,
/// PA, RI, SC, SD, TN, TX, UT, VA, VT, WA, WI, WV, WY
static US_STATE_CODE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(A[KLRZ]|C[AOT]|D[CE]|FL|GA|HI|I[ADLN]|K[SY]|LA|M[ADEINOST]|N[CDEHJMVY]|O[HKR]|P[AR]|RI|S[CD]|T[NX]|UT|V[AIT]|W[AIVY])$",
    )
    .expect("Invalid US state code regex pattern")
});

impl UsStateCode {
    /// Creates a `UsStateCode` from a 2-character state code
    ///
    /// # Arguments
    ///
    /// * `field_name` - Field name used in error messages
    /// * `value` - Input string
    ///
    /// # Returns
    ///
    /// * `Ok(UsStateCode)` - On successful validation
    /// * `Err(ValidationError)` - Invalid state codewhen
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` for an invalid state code.
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, UsStateCode, &US_STATE_CODE_PATTERN, value)
    }

    /// Returns a reference to the inner state code string
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // String50 Tests
    // =========================================================================

    #[rstest]
    fn test_string50_create_valid() {
        let result = String50::create("Name", "John Doe");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "John Doe");
    }

    #[rstest]
    fn test_string50_create_empty() {
        let result = String50::create("Name", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Name");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_string50_create_too_long() {
        let long_string = "a".repeat(51);
        let result = String50::create("Name", &long_string);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Name");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_string50_create_exactly_50_chars() {
        let exact_string = "a".repeat(50);
        let result = String50::create("Name", &exact_string);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), exact_string);
    }

    #[rstest]
    fn test_string50_create_option_empty() {
        let result = String50::create_option("Name", "");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[rstest]
    fn test_string50_create_option_valid() {
        let result = String50::create_option("Name", "John");

        assert!(result.is_ok());
        let option = result.unwrap();
        assert!(option.is_some());
        assert_eq!(option.unwrap().value(), "John");
    }

    #[rstest]
    fn test_string50_create_option_too_long() {
        let long_string = "a".repeat(51);
        let result = String50::create_option("Name", &long_string);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_string50_value() {
        let string50 = String50::create("Name", "Test").unwrap();

        assert_eq!(string50.value(), "Test");
    }

    #[rstest]
    fn test_string50_clone() {
        let original = String50::create("Name", "Test").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    // =========================================================================
    // EmailAddress Tests
    // =========================================================================

    #[rstest]
    fn test_email_address_create_valid() {
        let result = EmailAddress::create("Email", "user@example.com");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "user@example.com");
    }

    #[rstest]
    fn test_email_address_create_simple_valid() {
        // Minimal valid email address
        let result = EmailAddress::create("Email", "a@b");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "a@b");
    }

    #[rstest]
    fn test_email_address_create_empty() {
        let result = EmailAddress::create("Email", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Email");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_email_address_create_no_at() {
        let result = EmailAddress::create("Email", "invalid-email");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Email");
        assert!(error.message.contains("must match the pattern"));
    }

    #[rstest]
    fn test_email_address_create_at_only() {
        // @ alone is insufficient (.+ pattern requires something before and after)
        let result = EmailAddress::create("Email", "@");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_email_address_create_at_start() {
        // When @ is at the beginning (nothing before it)
        let result = EmailAddress::create("Email", "@example.com");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_email_address_create_at_end() {
        // When @ is at the end (nothing after it)
        let result = EmailAddress::create("Email", "user@");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_email_address_value() {
        let email = EmailAddress::create("Email", "test@test.com").unwrap();

        assert_eq!(email.value(), "test@test.com");
    }

    // =========================================================================
    // ZipCode Tests
    // =========================================================================

    #[rstest]
    fn test_zip_code_create_valid() {
        let result = ZipCode::create("ZipCode", "12345");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "12345");
    }

    #[rstest]
    fn test_zip_code_create_all_zeros() {
        let result = ZipCode::create("ZipCode", "00000");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_zip_code_create_all_nines() {
        let result = ZipCode::create("ZipCode", "99999");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_zip_code_create_empty() {
        let result = ZipCode::create("ZipCode", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ZipCode");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_zip_code_create_4_digits() {
        let result = ZipCode::create("ZipCode", "1234");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_zip_code_create_6_digits() {
        let result = ZipCode::create("ZipCode", "123456");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_zip_code_create_with_letters() {
        let result = ZipCode::create("ZipCode", "1234A");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_zip_code_create_with_dash() {
        // ZIP+4 format is not supported
        let result = ZipCode::create("ZipCode", "12345-6789");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_zip_code_value() {
        let zip = ZipCode::create("ZipCode", "90210").unwrap();

        assert_eq!(zip.value(), "90210");
    }

    // =========================================================================
    // UsStateCode Tests
    // =========================================================================

    #[rstest]
    fn test_us_state_code_create_valid_ca() {
        let result = UsStateCode::create("State", "CA");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "CA");
    }

    #[rstest]
    fn test_us_state_code_create_valid_ny() {
        let result = UsStateCode::create("State", "NY");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "NY");
    }

    #[rstest]
    fn test_us_state_code_create_valid_dc() {
        // DC is Washington D.C.
        let result = UsStateCode::create("State", "DC");

        assert!(result.is_ok());
    }

    #[rstest]
    #[case("AL")]
    #[case("AK")]
    #[case("AZ")]
    #[case("AR")]
    #[case("CA")]
    #[case("CO")]
    #[case("CT")]
    #[case("DE")]
    #[case("DC")]
    #[case("FL")]
    #[case("GA")]
    #[case("HI")]
    #[case("ID")]
    #[case("IL")]
    #[case("IN")]
    #[case("IA")]
    #[case("KS")]
    #[case("KY")]
    #[case("LA")]
    #[case("MA")]
    #[case("MD")]
    #[case("ME")]
    #[case("MI")]
    #[case("MN")]
    #[case("MO")]
    #[case("MS")]
    #[case("MT")]
    #[case("NC")]
    #[case("ND")]
    #[case("NE")]
    #[case("NH")]
    #[case("NJ")]
    #[case("NM")]
    #[case("NV")]
    #[case("NY")]
    #[case("OH")]
    #[case("OK")]
    #[case("OR")]
    #[case("PA")]
    #[case("RI")]
    #[case("SC")]
    #[case("SD")]
    #[case("TN")]
    #[case("TX")]
    #[case("UT")]
    #[case("VA")]
    #[case("VT")]
    #[case("WA")]
    #[case("WI")]
    #[case("WV")]
    #[case("WY")]
    fn test_us_state_code_create_all_valid_codes(#[case] code: &str) {
        let result = UsStateCode::create("State", code);

        assert!(result.is_ok(), "Failed for state code: {code}");
    }

    #[rstest]
    fn test_us_state_code_create_empty() {
        let result = UsStateCode::create("State", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "State");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_us_state_code_create_invalid_xx() {
        let result = UsStateCode::create("State", "XX");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_us_state_code_create_lowercase() {
        // Lowercase is invalid
        let result = UsStateCode::create("State", "ca");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_us_state_code_create_too_long() {
        let result = UsStateCode::create("State", "CAL");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_us_state_code_create_single_char() {
        let result = UsStateCode::create("State", "C");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_us_state_code_value() {
        let state = UsStateCode::create("State", "TX").unwrap();

        assert_eq!(state.value(), "TX");
    }
}
