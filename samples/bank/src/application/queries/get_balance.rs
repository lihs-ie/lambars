//! Balance query for retrieving account balance.
//!
//! This module provides a pure function to extract balance information
//! from an `Account` aggregate.
//!
//! # Design Principles
//!
//! - **Pure Function**: `get_balance` has no side effects
//! - **Type Safety**: Input and output are strongly typed
//! - **Immutability**: Does not modify the input account
//!
//! # Examples
//!
//! ```rust
//! use bank::application::queries::{get_balance, GetBalanceQuery, BalanceResponse};
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::value_objects::{AccountId, Money, Currency};
//!
//! let account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     balance: Money::new(10000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! let response = get_balance(&account);
//! assert_eq!(response.balance, account.balance);
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::account::aggregate::{Account, AccountStatus};
use crate::domain::value_objects::{AccountId, Money};

/// Input for the balance query.
///
/// Contains the account ID to query the balance for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBalanceQuery {
    /// The ID of the account to query.
    pub account_id: AccountId,
}

impl GetBalanceQuery {
    /// Creates a new balance query for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to query
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::application::queries::GetBalanceQuery;
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let query = GetBalanceQuery::new(AccountId::generate());
    /// ```
    #[must_use]
    pub const fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }
}

/// Response containing account balance information.
///
/// This is a read-optimized response type that contains all relevant
/// balance information for display or API responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceResponse {
    /// The account ID.
    pub account_id: AccountId,
    /// The current balance.
    pub balance: Money,
    /// The current account status.
    pub status: AccountStatus,
    /// The version number (for optimistic concurrency).
    pub version: u64,
}

impl BalanceResponse {
    /// Creates a new balance response.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account ID
    /// * `balance` - The current balance
    /// * `status` - The account status
    /// * `version` - The version number
    #[must_use]
    pub const fn new(
        account_id: AccountId,
        balance: Money,
        status: AccountStatus,
        version: u64,
    ) -> Self {
        Self {
            account_id,
            balance,
            status,
            version,
        }
    }

    /// Returns `true` if the account is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns `true` if the account is frozen.
    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        self.status.is_frozen()
    }

    /// Returns `true` if the account is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.status.is_closed()
    }
}

/// Extracts balance information from an account.
///
/// This is a pure function that transforms an `Account` into a `BalanceResponse`.
/// It has no side effects and does not modify the input.
///
/// # Arguments
///
/// * `account` - The account to extract balance information from
///
/// # Returns
///
/// A `BalanceResponse` containing the account's balance information
///
/// # Examples
///
/// ```rust
/// use bank::application::queries::{get_balance, BalanceResponse};
/// use bank::domain::account::aggregate::{Account, AccountStatus};
/// use bank::domain::value_objects::{AccountId, Money, Currency};
///
/// let account = Account {
///     id: AccountId::generate(),
///     owner_name: "Alice".to_string(),
///     balance: Money::new(10000, Currency::JPY),
///     status: AccountStatus::Active,
///     version: 1,
/// };
///
/// let response = get_balance(&account);
///
/// assert_eq!(response.account_id, account.id);
/// assert_eq!(response.balance, account.balance);
/// assert_eq!(response.status, AccountStatus::Active);
/// assert_eq!(response.version, 1);
/// ```
#[must_use]
pub fn get_balance(account: &Account) -> BalanceResponse {
    BalanceResponse {
        account_id: account.id,
        balance: account.balance.clone(),
        status: account.status,
        version: account.version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_active_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: Money::new(10000, Currency::JPY),
            status: AccountStatus::Active,
            version: 1,
        }
    }

    fn create_frozen_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: Money::new(5000, Currency::JPY),
            status: AccountStatus::Frozen,
            version: 3,
        }
    }

    fn create_closed_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: Money::zero(Currency::JPY),
            status: AccountStatus::Closed,
            version: 5,
        }
    }

    // =========================================================================
    // GetBalanceQuery Tests
    // =========================================================================

    #[rstest]
    fn get_balance_query_new_creates_query() {
        let account_id = AccountId::generate();
        let query = GetBalanceQuery::new(account_id);

        assert_eq!(query.account_id, account_id);
    }

    #[rstest]
    fn get_balance_query_clone_produces_equal() {
        let query = GetBalanceQuery::new(AccountId::generate());
        let cloned = query.clone();

        assert_eq!(query, cloned);
    }

    #[rstest]
    fn get_balance_query_serialize_deserialize_roundtrip() {
        let query = GetBalanceQuery::new(AccountId::generate());
        let serialized = serde_json::to_string(&query).unwrap();
        let deserialized: GetBalanceQuery = serde_json::from_str(&serialized).unwrap();

        assert_eq!(query, deserialized);
    }

    // =========================================================================
    // BalanceResponse Tests
    // =========================================================================

    #[rstest]
    fn balance_response_new_creates_response() {
        let account_id = AccountId::generate();
        let balance = Money::new(10000, Currency::JPY);
        let status = AccountStatus::Active;
        let version = 1;

        let response = BalanceResponse::new(account_id, balance.clone(), status, version);

        assert_eq!(response.account_id, account_id);
        assert_eq!(response.balance, balance);
        assert_eq!(response.status, status);
        assert_eq!(response.version, version);
    }

    #[rstest]
    fn balance_response_is_active_returns_true_for_active() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::new(10000, Currency::JPY),
            AccountStatus::Active,
            1,
        );

        assert!(response.is_active());
        assert!(!response.is_frozen());
        assert!(!response.is_closed());
    }

    #[rstest]
    fn balance_response_is_frozen_returns_true_for_frozen() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::new(5000, Currency::JPY),
            AccountStatus::Frozen,
            2,
        );

        assert!(!response.is_active());
        assert!(response.is_frozen());
        assert!(!response.is_closed());
    }

    #[rstest]
    fn balance_response_is_closed_returns_true_for_closed() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::zero(Currency::JPY),
            AccountStatus::Closed,
            5,
        );

        assert!(!response.is_active());
        assert!(!response.is_frozen());
        assert!(response.is_closed());
    }

    #[rstest]
    fn balance_response_clone_produces_equal() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::new(10000, Currency::JPY),
            AccountStatus::Active,
            1,
        );
        let cloned = response.clone();

        assert_eq!(response, cloned);
    }

    #[rstest]
    fn balance_response_serialize_deserialize_roundtrip() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::new(10000, Currency::JPY),
            AccountStatus::Active,
            1,
        );
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: BalanceResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(response, deserialized);
    }

    // =========================================================================
    // get_balance Function Tests
    // =========================================================================

    #[rstest]
    fn get_balance_extracts_all_fields_from_active_account() {
        let account = create_active_account();

        let response = get_balance(&account);

        assert_eq!(response.account_id, account.id);
        assert_eq!(response.balance, account.balance);
        assert_eq!(response.status, AccountStatus::Active);
        assert_eq!(response.version, account.version);
    }

    #[rstest]
    fn get_balance_extracts_all_fields_from_frozen_account() {
        let account = create_frozen_account();

        let response = get_balance(&account);

        assert_eq!(response.account_id, account.id);
        assert_eq!(response.balance, account.balance);
        assert_eq!(response.status, AccountStatus::Frozen);
        assert_eq!(response.version, account.version);
    }

    #[rstest]
    fn get_balance_extracts_all_fields_from_closed_account() {
        let account = create_closed_account();

        let response = get_balance(&account);

        assert_eq!(response.account_id, account.id);
        assert_eq!(response.balance, account.balance);
        assert_eq!(response.status, AccountStatus::Closed);
        assert_eq!(response.version, account.version);
    }

    #[rstest]
    fn get_balance_preserves_various_balances() {
        let test_cases = vec![
            Money::zero(Currency::JPY),
            Money::new(1, Currency::JPY),
            Money::new(100, Currency::JPY),
            Money::new(10000, Currency::JPY),
            Money::new(1_000_000, Currency::JPY),
            Money::new(1050, Currency::USD), // $10.50
            Money::new(999, Currency::EUR),
        ];

        for balance in test_cases {
            let account = Account {
                id: AccountId::generate(),
                owner_name: "Test User".to_string(),
                balance: balance.clone(),
                status: AccountStatus::Active,
                version: 1,
            };

            let response = get_balance(&account);

            assert_eq!(response.balance, balance);
        }
    }

    #[rstest]
    fn get_balance_preserves_various_versions() {
        let test_versions: Vec<u64> = vec![1, 2, 10, 100, 1000, u64::MAX];

        for version in test_versions {
            let account = Account {
                id: AccountId::generate(),
                owner_name: "Test User".to_string(),
                balance: Money::new(10000, Currency::JPY),
                status: AccountStatus::Active,
                version,
            };

            let response = get_balance(&account);

            assert_eq!(response.version, version);
        }
    }

    // =========================================================================
    // Pure Function Property Tests
    // =========================================================================

    #[rstest]
    fn get_balance_is_referentially_transparent() {
        let account = create_active_account();

        // Calling the function multiple times with the same input
        // should produce the same output (referential transparency)
        let response1 = get_balance(&account);
        let response2 = get_balance(&account);
        let response3 = get_balance(&account);

        assert_eq!(response1, response2);
        assert_eq!(response2, response3);
    }

    #[rstest]
    fn get_balance_does_not_modify_input() {
        let original_account = create_active_account();
        let account_clone = original_account.clone();

        let _ = get_balance(&original_account);

        // The account should be unchanged after calling get_balance
        assert_eq!(original_account, account_clone);
    }

    // =========================================================================
    // Debug and Display Tests
    // =========================================================================

    #[rstest]
    fn get_balance_query_debug_is_implemented() {
        let query = GetBalanceQuery::new(AccountId::generate());

        // Should not panic
        let debug_output = format!("{query:?}");
        assert!(!debug_output.is_empty());
    }

    #[rstest]
    fn balance_response_debug_is_implemented() {
        let response = BalanceResponse::new(
            AccountId::generate(),
            Money::new(10000, Currency::JPY),
            AccountStatus::Active,
            1,
        );

        // Should not panic
        let debug_output = format!("{response:?}");
        assert!(!debug_output.is_empty());
    }
}
