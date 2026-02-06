//! Customer information compound type

use lambars_derive::Lenses;

use super::PersonalName;
use crate::simple_types::{EmailAddress, ValidationError, VipStatus};

/// Customer information compound type
///
/// Has three fields: personal name, email address, and VIP status.
/// The nested structure containing `PersonalName` provides a good example of Lens composition.
///
/// # Examples
///
/// ```
/// use order_taking_sample::compound_types::CustomerInfo;
///
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// assert_eq!(customer.name().first_name().value(), "John");
/// assert_eq!(customer.email_address().value(), "john@example.com");
/// ```
///
/// # Using Lens Composition
///
/// ```
/// use order_taking_sample::compound_types::{CustomerInfo, PersonalName};
/// use order_taking_sample::simple_types::String50;
/// use lambars::optics::Lens;
///
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
///
/// // Access deep nesting via Lens composition
/// let name_lens = CustomerInfo::name_lens();
/// let first_name_lens = PersonalName::first_name_lens();
/// let customer_first_name = name_lens.compose(first_name_lens);
///
/// let first_name = customer_first_name.get(&customer);
/// assert_eq!(first_name.value(), "John");
///
/// // Update first_name (immutable update)
/// let new_first_name = String50::create("FirstName", "Jonathan").unwrap();
/// let updated = customer_first_name.set(customer, new_first_name);
/// assert_eq!(updated.name().first_name().value(), "Jonathan");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Lenses)]
pub struct CustomerInfo {
    name: PersonalName,
    email_address: EmailAddress,
    vip_status: VipStatus,
}

impl CustomerInfo {
    /// Creates a `CustomerInfo` from personal name, email, and VIP status
    ///
    /// # Arguments
    ///
    /// * `first_name` - First name
    /// * `last_name` - Last name
    /// * `email` - Email address
    /// * `vip_status` - VIP status ("Normal" or "VIP")
    ///
    /// # Returns
    ///
    /// * `Ok(CustomerInfo)` - On successful validation
    /// * `Err(ValidationError)` - If any field is invalid
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if any field is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::CustomerInfo;
    /// use order_taking_sample::simple_types::VipStatus;
    ///
    /// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "VIP").unwrap();
    /// assert!(matches!(customer.vip_status(), VipStatus::Vip));
    ///
    /// // Invalid email addresses cause an error
    /// assert!(CustomerInfo::create("John", "Doe", "invalid", "Normal").is_err());
    /// ```
    pub fn create(
        first_name: &str,
        last_name: &str,
        email: &str,
        vip_status: &str,
    ) -> Result<Self, ValidationError> {
        let name = PersonalName::create(first_name, last_name)?;
        let email_address = EmailAddress::create("EmailAddress", email)?;
        let vip_status = VipStatus::create("VipStatus", vip_status)?;

        Ok(Self {
            name,
            email_address,
            vip_status,
        })
    }

    /// Creates a `CustomerInfo` from pre-validated components
    ///
    /// No validation needed (each type is already validated).
    ///
    /// # Arguments
    ///
    /// * `name` - Personal name
    /// * `email_address` - Email address
    /// * `vip_status` - VIP status
    ///
    /// # Returns
    ///
    /// A new `CustomerInfo` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::{CustomerInfo, PersonalName};
    /// use order_taking_sample::simple_types::{EmailAddress, VipStatus};
    ///
    /// let name = PersonalName::create("John", "Doe").unwrap();
    /// let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
    /// let status = VipStatus::create("VipStatus", "Normal").unwrap();
    ///
    /// let customer = CustomerInfo::create_from_parts(name, email, status);
    /// assert_eq!(customer.name().first_name().value(), "John");
    /// ```
    #[must_use]
    pub const fn create_from_parts(
        name: PersonalName,
        email_address: EmailAddress,
        vip_status: VipStatus,
    ) -> Self {
        Self {
            name,
            email_address,
            vip_status,
        }
    }

    /// Returns a reference to the personal name
    ///
    /// # Returns
    ///
    /// A reference to the `PersonalName` personal name
    #[must_use]
    pub const fn name(&self) -> &PersonalName {
        &self.name
    }

    /// Returns a reference to the email address
    ///
    /// # Returns
    ///
    /// A reference to the `EmailAddress` email address
    #[must_use]
    pub const fn email_address(&self) -> &EmailAddress {
        &self.email_address
    }

    /// Returns VIP status (copied since it is a Copy type)
    ///
    /// # Returns
    ///
    /// The `VipStatus` VIP status
    #[must_use]
    pub const fn vip_status(&self) -> VipStatus {
        self.vip_status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simple_types::String50;
    use lambars::optics::Lens;
    use rstest::rstest;

    // =========================================================================
    // Tests for create
    // =========================================================================

    #[rstest]
    fn test_customer_info_create_valid() {
        let result = CustomerInfo::create("John", "Doe", "john@example.com", "Normal");

        assert!(result.is_ok());
        let customer = result.unwrap();
        assert_eq!(customer.name().first_name().value(), "John");
        assert_eq!(customer.name().last_name().value(), "Doe");
        assert_eq!(customer.email_address().value(), "john@example.com");
        assert!(matches!(customer.vip_status(), VipStatus::Normal));
    }

    #[rstest]
    fn test_customer_info_create_vip() {
        let result = CustomerInfo::create("Jane", "Smith", "jane@test.org", "VIP");

        assert!(result.is_ok());
        let customer = result.unwrap();
        assert!(matches!(customer.vip_status(), VipStatus::Vip));
    }

    #[rstest]
    fn test_customer_info_create_invalid_name() {
        let result = CustomerInfo::create("", "Doe", "john@example.com", "Normal");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "FirstName");
    }

    #[rstest]
    fn test_customer_info_create_invalid_last_name() {
        let result = CustomerInfo::create("John", "", "john@example.com", "Normal");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "LastName");
    }

    #[rstest]
    fn test_customer_info_create_invalid_email() {
        let result = CustomerInfo::create("John", "Doe", "invalid-email", "Normal");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "EmailAddress");
    }

    #[rstest]
    fn test_customer_info_create_invalid_vip_status() {
        let result = CustomerInfo::create("John", "Doe", "john@example.com", "Premium");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "VipStatus");
    }

    // =========================================================================
    // Tests for create_from_parts
    // =========================================================================

    #[rstest]
    fn test_customer_info_create_from_parts() {
        let name = PersonalName::create("John", "Doe").unwrap();
        let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
        let status = VipStatus::create("VipStatus", "VIP").unwrap();

        let customer = CustomerInfo::create_from_parts(name, email, status);

        assert_eq!(customer.name().first_name().value(), "John");
        assert!(matches!(customer.vip_status(), VipStatus::Vip));
    }

    // =========================================================================
    // Tests for getters
    // =========================================================================

    #[rstest]
    fn test_customer_info_getters() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();

        assert_eq!(customer.name().first_name().value(), "John");
        assert_eq!(customer.name().last_name().value(), "Doe");
        assert_eq!(customer.email_address().value(), "john@example.com");
        assert!(matches!(customer.vip_status(), VipStatus::Normal));
    }

    // =========================================================================
    // Tests for Clone/Eq
    // =========================================================================

    #[rstest]
    fn test_customer_info_clone() {
        let original = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn test_customer_info_eq() {
        let customer1 = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let customer2 = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let customer3 = CustomerInfo::create("Jane", "Doe", "jane@example.com", "VIP").unwrap();

        assert_eq!(customer1, customer2);
        assert_ne!(customer1, customer3);
    }

    // =========================================================================
    // Tests for Lens
    // =========================================================================

    #[rstest]
    fn test_customer_info_name_lens_get() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let lens = CustomerInfo::name_lens();

        let name = lens.get(&customer);
        assert_eq!(name.first_name().value(), "John");
    }

    #[rstest]
    fn test_customer_info_name_lens_set() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let lens = CustomerInfo::name_lens();
        let new_name = PersonalName::create("Jane", "Smith").unwrap();

        let updated = lens.set(customer, new_name);

        assert_eq!(updated.name().first_name().value(), "Jane");
        assert_eq!(updated.name().last_name().value(), "Smith");
        assert_eq!(updated.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_customer_info_email_lens_get() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let lens = CustomerInfo::email_address_lens();

        assert_eq!(lens.get(&customer).value(), "john@example.com");
    }

    #[rstest]
    fn test_customer_info_email_lens_set() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let lens = CustomerInfo::email_address_lens();
        let new_email = EmailAddress::create("EmailAddress", "jane@test.org").unwrap();

        let updated = lens.set(customer, new_email);

        assert_eq!(updated.email_address().value(), "jane@test.org");
        assert_eq!(updated.name().first_name().value(), "John");
    }

    #[rstest]
    fn test_customer_info_vip_status_lens_get() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "VIP").unwrap();
        let lens = CustomerInfo::vip_status_lens();

        assert!(matches!(*lens.get(&customer), VipStatus::Vip));
    }

    #[rstest]
    fn test_customer_info_vip_status_lens_set() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let lens = CustomerInfo::vip_status_lens();

        let updated = lens.set(customer, VipStatus::Vip);

        assert!(matches!(updated.vip_status(), VipStatus::Vip));
        assert_eq!(updated.name().first_name().value(), "John");
    }

    // =========================================================================
    // Tests for Lens composition
    // =========================================================================

    #[rstest]
    fn test_customer_info_composed_lens_first_name() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();

        let name_lens = CustomerInfo::name_lens();
        let first_name_lens = PersonalName::first_name_lens();
        let composed = name_lens.compose(first_name_lens);

        // get
        assert_eq!(composed.get(&customer).value(), "John");

        // set
        let new_first_name = String50::create("FirstName", "Jonathan").unwrap();
        let updated = composed.set(customer, new_first_name);
        assert_eq!(updated.name().first_name().value(), "Jonathan");
        assert_eq!(updated.name().last_name().value(), "Doe");
        assert_eq!(updated.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_customer_info_composed_lens_last_name() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();

        let name_lens = CustomerInfo::name_lens();
        let last_name_lens = PersonalName::last_name_lens();
        let composed = name_lens.compose(last_name_lens);

        // get
        assert_eq!(composed.get(&customer).value(), "Doe");

        // set
        let new_last_name = String50::create("LastName", "Smith").unwrap();
        let updated = composed.set(customer, new_last_name);
        assert_eq!(updated.name().first_name().value(), "John");
        assert_eq!(updated.name().last_name().value(), "Smith");
    }

    #[rstest]
    fn test_customer_info_composed_lens_modify() {
        let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();

        let name_lens = CustomerInfo::name_lens();
        let first_name_lens = PersonalName::first_name_lens();
        let composed = name_lens.compose(first_name_lens);

        let updated = composed.modify(customer, |old| {
            let new_value = old.value().to_uppercase();
            String50::create("FirstName", &new_value).unwrap()
        });

        assert_eq!(updated.name().first_name().value(), "JOHN");
    }
}
