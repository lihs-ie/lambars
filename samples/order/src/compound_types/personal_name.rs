//! 個人名を表す複合型

use lambars_derive::Lenses;

use crate::simple_types::{String50, ValidationError};

/// 個人名を表す複合型
///
/// 姓（LastName）と名（FirstName）の2つのフィールドを持つ。
/// どちらも必須フィールドで、`String50` 型として制約される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::compound_types::PersonalName;
///
/// let name = PersonalName::create("John", "Doe").unwrap();
/// assert_eq!(name.first_name().value(), "John");
/// assert_eq!(name.last_name().value(), "Doe");
/// ```
///
/// # Lens の使用
///
/// ```
/// use order_taking_sample::compound_types::PersonalName;
/// use order_taking_sample::simple_types::String50;
/// use lambars::optics::Lens;
///
/// let name = PersonalName::create("John", "Doe").unwrap();
///
/// // first_name を更新（不変更新）
/// let new_first_name = String50::create("FirstName", "Jonathan").unwrap();
/// let updated = PersonalName::first_name_lens().set(name, new_first_name);
/// assert_eq!(updated.first_name().value(), "Jonathan");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Lenses)]
pub struct PersonalName {
    first_name: String50,
    last_name: String50,
}

impl PersonalName {
    /// 姓と名の文字列から `PersonalName` を生成する
    ///
    /// # Arguments
    ///
    /// * `first_name` - 名（ファーストネーム）の文字列
    /// * `last_name` - 姓（ラストネーム）の文字列
    ///
    /// # Returns
    ///
    /// * `Ok(PersonalName)` - バリデーション成功時
    /// * `Err(ValidationError)` - いずれかのフィールドが無効な場合
    ///
    /// # Errors
    ///
    /// 名または姓が空文字列または50文字を超える場合に `ValidationError` を返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::PersonalName;
    ///
    /// let name = PersonalName::create("John", "Doe").unwrap();
    /// assert_eq!(name.first_name().value(), "John");
    ///
    /// // 空の名前はエラー
    /// assert!(PersonalName::create("", "Doe").is_err());
    /// ```
    pub fn create(first_name: &str, last_name: &str) -> Result<Self, ValidationError> {
        let first_name_validated = String50::create("FirstName", first_name)?;
        let last_name_validated = String50::create("LastName", last_name)?;

        Ok(Self {
            first_name: first_name_validated,
            last_name: last_name_validated,
        })
    }

    /// 既にバリデーション済みの構成要素から `PersonalName` を生成する
    ///
    /// バリデーションは不要（既に各型でバリデーション済み）。
    ///
    /// # Arguments
    ///
    /// * `first_name` - 名
    /// * `last_name` - 姓
    ///
    /// # Returns
    ///
    /// 新しい `PersonalName` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::PersonalName;
    /// use order_taking_sample::simple_types::String50;
    ///
    /// let first_name = String50::create("FirstName", "John").unwrap();
    /// let last_name = String50::create("LastName", "Doe").unwrap();
    ///
    /// let name = PersonalName::create_from_parts(first_name, last_name);
    /// assert_eq!(name.first_name().value(), "John");
    /// ```
    #[must_use]
    pub const fn create_from_parts(first_name: String50, last_name: String50) -> Self {
        Self {
            first_name,
            last_name,
        }
    }

    /// 名への参照を返す
    ///
    /// # Returns
    ///
    /// `String50` 型の名への参照
    #[must_use]
    pub const fn first_name(&self) -> &String50 {
        &self.first_name
    }

    /// 姓への参照を返す
    ///
    /// # Returns
    ///
    /// `String50` 型の姓への参照
    #[must_use]
    pub const fn last_name(&self) -> &String50 {
        &self.last_name
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
    fn test_personal_name_create_valid() {
        let result = PersonalName::create("John", "Doe");

        assert!(result.is_ok());
        let name = result.unwrap();
        assert_eq!(name.first_name().value(), "John");
        assert_eq!(name.last_name().value(), "Doe");
    }

    #[rstest]
    fn test_personal_name_create_first_name_empty() {
        let result = PersonalName::create("", "Doe");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "FirstName");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_personal_name_create_last_name_empty() {
        let result = PersonalName::create("John", "");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "LastName");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_personal_name_create_first_name_too_long() {
        let long_name = "a".repeat(51);
        let result = PersonalName::create(&long_name, "Doe");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "FirstName");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_personal_name_create_last_name_too_long() {
        let long_name = "a".repeat(51);
        let result = PersonalName::create("John", &long_name);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "LastName");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_personal_name_create_boundary_50_chars() {
        let max_name = "a".repeat(50);
        let result = PersonalName::create(&max_name, &max_name);

        assert!(result.is_ok());
        let name = result.unwrap();
        assert_eq!(name.first_name().value().len(), 50);
        assert_eq!(name.last_name().value().len(), 50);
    }

    // =========================================================================
    // Getter のテスト
    // =========================================================================

    #[rstest]
    fn test_personal_name_getters() {
        let name = PersonalName::create("Jane", "Smith").unwrap();

        assert_eq!(name.first_name().value(), "Jane");
        assert_eq!(name.last_name().value(), "Smith");
    }

    // =========================================================================
    // Clone/Eq のテスト
    // =========================================================================

    #[rstest]
    fn test_personal_name_clone() {
        let original = PersonalName::create("John", "Doe").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_personal_name_eq() {
        let name1 = PersonalName::create("John", "Doe").unwrap();
        let name2 = PersonalName::create("John", "Doe").unwrap();
        let name3 = PersonalName::create("Jane", "Doe").unwrap();

        assert_eq!(name1, name2);
        assert_ne!(name1, name3);
    }

    // =========================================================================
    // Lens のテスト
    // =========================================================================

    #[rstest]
    fn test_personal_name_first_name_lens_get() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let lens = PersonalName::first_name_lens();

        assert_eq!(lens.get(&name).value(), "John");
    }

    #[rstest]
    fn test_personal_name_first_name_lens_set() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let lens = PersonalName::first_name_lens();
        let new_first_name = String50::create("FirstName", "Jonathan").unwrap();

        let updated = lens.set(name, new_first_name);

        assert_eq!(updated.first_name().value(), "Jonathan");
        assert_eq!(updated.last_name().value(), "Doe");
    }

    #[rstest]
    fn test_personal_name_last_name_lens_get() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let lens = PersonalName::last_name_lens();

        assert_eq!(lens.get(&name).value(), "Doe");
    }

    #[rstest]
    fn test_personal_name_last_name_lens_set() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let lens = PersonalName::last_name_lens();
        let new_last_name = String50::create("LastName", "Smith").unwrap();

        let updated = lens.set(name, new_last_name);

        assert_eq!(updated.first_name().value(), "John");
        assert_eq!(updated.last_name().value(), "Smith");
    }

    #[rstest]
    fn test_personal_name_lens_modify() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let lens = PersonalName::first_name_lens();

        let updated = lens.modify(name, |old| {
            let new_value = format!("{}-modified", old.value());
            // 新しい String50 を作成（50文字以内に収まる）
            String50::create("FirstName", &new_value).unwrap()
        });

        assert_eq!(updated.first_name().value(), "John-modified");
    }
}
