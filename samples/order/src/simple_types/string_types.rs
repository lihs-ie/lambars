//! 文字列制約型の定義
//!
//! `String50`, `EmailAddress`, `ZipCode`, `UsStateCode` を定義する。

use regex::Regex;
use std::sync::LazyLock;

use super::constrained_type;
use super::error::ValidationError;

// =============================================================================
// String50
// =============================================================================

/// 50文字以下に制約された文字列型
///
/// 名前、住所の一部など、短い文字列フィールドに使用する。
/// 空文字列は許可しない。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::String50;
///
/// let name = String50::create("CustomerName", "John Doe").unwrap();
/// assert_eq!(name.value(), "John Doe");
///
/// // 空文字列はエラー
/// assert!(String50::create("CustomerName", "").is_err());
///
/// // 51文字以上はエラー
/// let long_name = "a".repeat(51);
/// assert!(String50::create("CustomerName", &long_name).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct String50(String);

/// String50 の最大文字数
const STRING50_MAX_LENGTH: usize = 50;

impl String50 {
    /// 文字列から String50 を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(String50)` - バリデーション成功時
    /// * `Err(ValidationError)` - 空文字列または50文字超過時
    ///
    /// # Errors
    ///
    /// 空文字列または50文字を超える場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_string(field_name, String50, STRING50_MAX_LENGTH, value)
    }

    /// 空文字列の場合は None を返し、それ以外はバリデーションを行う
    ///
    /// オプショナルなフィールドに使用する。
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(None)` - 空文字列の場合
    /// * `Ok(Some(String50))` - バリデーション成功時
    /// * `Err(ValidationError)` - 50文字超過時
    ///
    /// # Errors
    ///
    /// 50文字を超える場合に `ValidationError` を返す。
    pub fn create_option(field_name: &str, value: &str) -> Result<Option<Self>, ValidationError> {
        constrained_type::create_string_option(field_name, String50, STRING50_MAX_LENGTH, value)
    }

    /// 内部の文字列値への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// EmailAddress
// =============================================================================

/// メールアドレス形式に制約された文字列型
///
/// 最低限、@ を含む文字列であることを検証する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::EmailAddress;
///
/// let email = EmailAddress::create("Email", "user@example.com").unwrap();
/// assert_eq!(email.value(), "user@example.com");
///
/// // @ を含まない場合はエラー
/// assert!(EmailAddress::create("Email", "invalid-email").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmailAddress(String);

/// メールアドレスの正規表現パターン
/// .+@.+ : 何か@何か の形式
static EMAIL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.+@.+$").expect("Invalid email regex pattern"));

impl EmailAddress {
    /// メールアドレス形式の文字列から `EmailAddress` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(EmailAddress)` - バリデーション成功時
    /// * `Err(ValidationError)` - 空文字列または @ を含まない場合
    ///
    /// # Errors
    ///
    /// 空文字列または @ を含まない場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, EmailAddress, &EMAIL_PATTERN, value)
    }

    /// 内部のメールアドレス文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// ZipCode
// =============================================================================

/// 5桁の郵便番号を表す型
///
/// 米国の ZIP コード形式を想定する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::ZipCode;
///
/// let zip = ZipCode::create("ZipCode", "12345").unwrap();
/// assert_eq!(zip.value(), "12345");
///
/// // 4桁はエラー
/// assert!(ZipCode::create("ZipCode", "1234").is_err());
///
/// // 6桁はエラー
/// assert!(ZipCode::create("ZipCode", "123456").is_err());
///
/// // 文字を含むとエラー
/// assert!(ZipCode::create("ZipCode", "1234A").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZipCode(String);

/// `ZipCode` の正規表現パターン（5桁の数字）
static ZIP_CODE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{5}$").expect("Invalid zip code regex pattern"));

impl ZipCode {
    /// 5桁の数字文字列から `ZipCode` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(ZipCode)` - バリデーション成功時
    /// * `Err(ValidationError)` - 5桁の数字でない場合
    ///
    /// # Errors
    ///
    /// 5桁の数字でない場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, ZipCode, &ZIP_CODE_PATTERN, value)
    }

    /// 内部の郵便番号文字列への参照を返す
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// UsStateCode
// =============================================================================

/// 米国の2文字の州コードを表す型
///
/// 有効な州コードのみを受け入れる。
///
/// # Examples
///
/// ```
/// use order_taking_sample::simple_types::UsStateCode;
///
/// let state = UsStateCode::create("State", "CA").unwrap();
/// assert_eq!(state.value(), "CA");
///
/// // 無効な州コードはエラー
/// assert!(UsStateCode::create("State", "XX").is_err());
///
/// // 小文字はエラー
/// assert!(UsStateCode::create("State", "ca").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UsStateCode(String);

/// `UsStateCode` の正規表現パターン（有効な米国州コード）
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
    /// 2文字の州コードから `UsStateCode` を生成する
    ///
    /// # Arguments
    ///
    /// * `field_name` - エラーメッセージに使用するフィールド名
    /// * `value` - 入力文字列
    ///
    /// # Returns
    ///
    /// * `Ok(UsStateCode)` - バリデーション成功時
    /// * `Err(ValidationError)` - 無効な州コードの場合
    ///
    /// # Errors
    ///
    /// 無効な州コードの場合に `ValidationError` を返す。
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        constrained_type::create_like(field_name, UsStateCode, &US_STATE_CODE_PATTERN, value)
    }

    /// 内部の州コード文字列への参照を返す
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
        // 最低限の有効なメールアドレス
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
        // @ のみでは不十分（.+ パターンにより前後に何か必要）
        let result = EmailAddress::create("Email", "@");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_email_address_create_at_start() {
        // @ が先頭にある場合（前に何もない）
        let result = EmailAddress::create("Email", "@example.com");

        assert!(result.is_err());
    }

    #[rstest]
    fn test_email_address_create_at_end() {
        // @ が末尾にある場合（後に何もない）
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
        // ZIP+4 形式はサポートしない
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
        // DC はワシントン D.C.
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
        // 小文字は無効
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
