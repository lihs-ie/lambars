//! Route configuration for the Bank API.
//!
//! This module defines all HTTP routes for the API and maps them to handlers.
//!
//! # Routes
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | /accounts | `create_account` | Create a new account |
//! | GET | /accounts/:id | `get_account` | Get account information |
//! | GET | /accounts/:id/balance | `get_balance` | Get account balance |
//! | POST | /accounts/:id/deposit | `deposit` | Deposit money |
//! | POST | /accounts/:id/withdraw | `withdraw` | Withdraw money |
//! | POST | /accounts/:id/transfer | `transfer` | Transfer money |
//! | GET | /accounts/:id/transactions | `get_transactions` | Get transaction history |
//! | GET | /health | `health_check` | Health check endpoint |
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::api::routes::create_router;
//! use bank::infrastructure::dependencies::AppDependencies;
//!
//! let deps = AppDependencies::new().await?;
//! let router = create_router(deps);
//! ```

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::api::handlers::account::{create_account, get_account, get_balance};
use crate::api::handlers::transaction::{deposit, get_transactions, transfer, withdraw};
use crate::infrastructure::AppDependencies;

/// Health check response.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Service status ("healthy" or "unhealthy").
    pub status: String,
    /// Service version.
    pub version: String,
}

/// GET /health - Health check endpoint.
///
/// Returns the health status of the service.
///
/// # Response
///
/// - `200 OK` - Service is healthy
///
/// # Example Response
///
/// ```json
/// {
///     "status": "healthy",
///     "version": "0.1.0"
/// }
/// ```
#[allow(clippy::unused_async)]
pub async fn health_check(
    State(_dependencies): State<AppDependencies>,
) -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    (StatusCode::OK, Json(response))
}

/// Creates the Axum router with all API routes.
///
/// # Arguments
///
/// * `dependencies` - The application dependencies (repositories, services, etc.)
///
/// # Returns
///
/// An Axum `Router` configured with all API routes.
///
/// # Routes
///
/// ## Account Routes
///
/// - `POST /accounts` - Create a new account
/// - `GET /accounts/:id` - Get account information
/// - `GET /accounts/:id/balance` - Get account balance
///
/// ## Transaction Routes
///
/// - `POST /accounts/:id/deposit` - Deposit money
/// - `POST /accounts/:id/withdraw` - Withdraw money
/// - `POST /accounts/:id/transfer` - Transfer money
/// - `GET /accounts/:id/transactions` - Get transaction history
///
/// ## Health Routes
///
/// - `GET /health` - Health check
///
/// # Example
///
/// ```rust,ignore
/// use bank::api::routes::create_router;
/// use bank::infrastructure::dependencies::AppDependencies;
///
/// async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
///     let deps = AppDependencies::new().await?;
///     let router = create_router(deps);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
///     axum::serve(listener, router).await?;
///     Ok(())
/// }
/// ```
pub fn create_router(dependencies: AppDependencies) -> Router {
    Router::new()
        // Account routes
        .route("/accounts", post(create_account))
        .route("/accounts/{id}", get(get_account))
        .route("/accounts/{id}/balance", get(get_balance))
        // Transaction routes
        .route("/accounts/{id}/deposit", post(deposit))
        .route("/accounts/{id}/withdraw", post(withdraw))
        .route("/accounts/{id}/transfer", post(transfer))
        .route("/accounts/{id}/transactions", get(get_transactions))
        // Health check
        .route("/health", get(health_check))
        // Add state
        .with_state(dependencies)
}

/// Creates a router with a test-friendly configuration.
///
/// This is useful for integration tests where you want to inject
/// mock dependencies.
#[cfg(test)]
pub fn create_test_router(dependencies: AppDependencies) -> Router {
    create_router(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // HealthResponse Tests
    // =========================================================================

    #[rstest]
    fn health_response_serializes_correctly() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[rstest]
    fn health_response_clone_produces_equal() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
        };
        let cloned = response.clone();

        assert_eq!(response.status, cloned.status);
        assert_eq!(response.version, cloned.version);
    }

    // =========================================================================
    // Router Configuration Tests
    // =========================================================================

    // Note: Full router tests would require setting up mock dependencies
    // and making actual HTTP requests. These are typically done in integration tests.
}
