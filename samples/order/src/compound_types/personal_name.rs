//! Personal name compound type

use lambars_derive::Lenses;

use crate::simple_types::{String50, ValidationError};

/// Personal name compound type
///
/// Has two fields: last name and first name.
/// Both are required fields constrained to the `String50` type.
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
/// # Using Lens
///
/// ```
/// use order_taking_sample::compound_types::PersonalName;
/// use order_taking_sample::simple_types::String50;
/// use lambars::optics::Lens;
///
/// let name = PersonalName::create("John", "Doe").unwrap();
///
/// // Update first_name (immutable update)
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
    /// Creates a `PersonalName` from first and last name strings
    ///
    /// # Arguments
    ///
    /// * `first_name` - First name string
    /// * `last_name` - Last name string
    ///
    /// # Returns
    ///
    /// * `Ok(PersonalName)` - On successful validation
    /// * `Err(ValidationError)` - If any field is invalid
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the first name or last name is an empty string or exceeds 50 characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::PersonalName;
    ///
    /// let name = PersonalName::create("John", "Doe").unwrap();
    /// assert_eq!(name.first_name().value(), "John");
    ///
    /// // Empty names cause an error
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

    /// Creates a `PersonalName` from pre-validated components
    ///
    /// No validation needed (each type is already validated).
    ///
    /// # Arguments
    ///
    /// * `first_name` - First name
    /// * `last_name` - Last name
    ///
    /// # Returns
    ///
    /// A new `PersonalName` instance
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

    /// Returns a reference to the first name
    ///
    /// # Returns
    ///
    /// A reference to the `String50` first name
    #[must_use]
    pub const fn first_name(&self) -> &String50 {
        &self.first_name
    }

    /// Returns a reference to the last name
    ///
    /// # Returns
    ///
    /// A reference to the `String50` last name
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
    // Tests for create
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
    // Tests for getters
    // =========================================================================

    #[rstest]
    fn test_personal_name_getters() {
        let name = PersonalName::create("Jane", "Smith").unwrap();

        assert_eq!(name.first_name().value(), "Jane");
        assert_eq!(name.last_name().value(), "Smith");
    }

    // =========================================================================
    // Tests for Clone/Eq
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
    // Tests for Lens
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
            // Create a new String50 (fits within 50 characters)
            String50::create("FirstName", &new_value).unwrap()
        });

        assert_eq!(updated.first_name().value(), "John-modified");
    }
}
