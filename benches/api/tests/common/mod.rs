//! Common test helpers for integration tests.
//!
//! This module provides shared utilities for creating test fixtures,
//! `AppState` instances, and common test data.
//!
//! # Usage
//!
//! ```ignore
//! mod common;
//! use common::{create_test_app_state, create_and_save_task};
//! ```
//!
//! # Note
//!
//! The `#![allow(dead_code)]` attribute is necessary because Rust compiles each
//! integration test file as a separate crate. Functions used only by specific
//! test files (e.g., `demo_endpoints.rs` or `production_endpoints.rs`) would
//! otherwise generate dead code warnings during compilation of other test files.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};

use arc_swap::ArcSwap;

use task_management_benchmark_api::api::{
    AppConfig, AppState, AppliedConfig, bulk::BulkConfig, handlers::create_stub_external_sources,
    query::SearchCache, query::SearchIndex,
};
use task_management_benchmark_api::domain::{
    EventId, Priority, Task, TaskId, TaskStatus, Timestamp, create_task_created_event,
};
use task_management_benchmark_api::infrastructure::{
    ExternalDataSource, ExternalError, InMemoryEventStore, InMemoryProjectRepository,
    InMemoryTaskRepository, RngProvider, StubExternalDataSource,
};

use lambars::persistent::PersistentVector;

// =============================================================================
// AppState Creation Helpers
// =============================================================================

/// Creates a test `AppState` with in-memory repositories.
///
/// This helper initializes all components synchronously without backfill
/// for faster test execution.
pub fn create_test_app_state() -> AppState {
    let external_sources = create_stub_external_sources();

    AppState {
        task_repository: Arc::new(InMemoryTaskRepository::new()),
        project_repository: Arc::new(InMemoryProjectRepository::new()),
        event_store: Arc::new(InMemoryEventStore::new()),
        config: AppConfig::default(),
        bulk_config: BulkConfig::default(),
        search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
            &PersistentVector::new(),
        ))),
        search_cache: Arc::new(SearchCache::with_default_config()),
        secondary_source: external_sources.secondary_source,
        external_source: external_sources.external_source,
        rng_provider: Arc::new(RngProvider::new_random()),
        cache_hits: Arc::new(AtomicU64::new(0)),
        cache_misses: Arc::new(AtomicU64::new(0)),
        cache_errors: Arc::new(AtomicU64::new(0)),
        cache_strategy: "read-through".to_string(),
        cache_ttl_seconds: 60,
        applied_config: AppliedConfig::default(),
        search_index_rcu_retries: Arc::new(AtomicUsize::new(0)),
        search_index_writer: None,
    }
}

/// Creates an `AppState` with configurable fail injection for external sources.
///
/// # Arguments
///
/// * `secondary_fails` - Whether the secondary source should fail
/// * `external_fails` - Whether the external source should fail
pub fn create_test_app_state_with_fail_injection(
    secondary_fails: bool,
    external_fails: bool,
) -> AppState {
    let secondary_source: Arc<dyn ExternalDataSource + Send + Sync> = if secondary_fails {
        Arc::new(StubExternalDataSource::with_error(
            ExternalError::InjectedFailure("Test failure".to_string()),
            "secondary",
        ))
    } else {
        Arc::new(StubExternalDataSource::not_found("secondary"))
    };

    let external_source: Arc<dyn ExternalDataSource + Send + Sync> = if external_fails {
        Arc::new(StubExternalDataSource::with_error(
            ExternalError::InjectedFailure("Test failure".to_string()),
            "external",
        ))
    } else {
        Arc::new(StubExternalDataSource::not_found("external"))
    };

    AppState {
        task_repository: Arc::new(InMemoryTaskRepository::new()),
        project_repository: Arc::new(InMemoryProjectRepository::new()),
        event_store: Arc::new(InMemoryEventStore::new()),
        config: AppConfig::default(),
        bulk_config: BulkConfig::default(),
        search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
            &PersistentVector::new(),
        ))),
        search_cache: Arc::new(SearchCache::with_default_config()),
        secondary_source,
        external_source,
        rng_provider: Arc::new(RngProvider::new_random()),
        cache_hits: Arc::new(AtomicU64::new(0)),
        cache_misses: Arc::new(AtomicU64::new(0)),
        cache_errors: Arc::new(AtomicU64::new(0)),
        cache_strategy: "read-through".to_string(),
        cache_ttl_seconds: 60,
        applied_config: AppliedConfig::default(),
        search_index_rcu_retries: Arc::new(AtomicUsize::new(0)),
        search_index_writer: None,
    }
}

// =============================================================================
// Task Creation Helpers
// =============================================================================

/// Creates a test task with the given title and saves it to the repository.
///
/// This also writes a `TaskCreated` event to the `EventStore`.
pub async fn create_and_save_task(state: &AppState, title: &str) -> Task {
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task = Task::new(task_id.clone(), title, timestamp.clone());

    // Save the task
    state
        .task_repository
        .save(&task)
        .await
        .expect("Failed to save task");

    // Write a TaskCreated event to the EventStore
    let event = create_task_created_event(
        &task,
        EventId::generate_v7(),
        timestamp,
        1, // First event version
    );
    state
        .event_store
        .append(&event, 0)
        .await
        .expect("Failed to append event");

    task
}

/// Creates a task with specified status and priority.
///
/// This does NOT write events to the `EventStore`.
pub async fn create_task_with_status_priority(
    state: &AppState,
    title: &str,
    status: TaskStatus,
    priority: Priority,
) -> Task {
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task = Task::new(task_id.clone(), title, timestamp.clone())
        .with_status(status)
        .with_priority(priority);

    // Save the task
    state
        .task_repository
        .save(&task)
        .await
        .expect("Failed to save task");

    task
}

/// Saves a task to the repository without writing any events to the `EventStore`.
///
/// Useful for testing demo endpoints that don't rely on `EventStore`.
pub async fn save_task_without_events(state: &AppState, task: &Task) {
    state
        .task_repository
        .save(task)
        .await
        .expect("Failed to save task");
}

// =============================================================================
// Assertion Helpers
// =============================================================================

/// Asserts that two values are approximately equal (for floating point comparisons).
pub fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff < epsilon,
        "Values not approximately equal: actual={actual}, expected={expected}, diff={diff}, epsilon={epsilon}"
    );
}
