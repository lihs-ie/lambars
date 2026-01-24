//! API module for HTTP handlers.
//!
//! This module contains route definitions and request/response handlers.

pub mod advanced;
pub mod alternative;
pub mod applicative;
pub mod async_pipeline;
pub mod bifunctor;
pub mod bulk;
pub mod dto;
pub mod effects;
pub mod error;
pub mod handlers;
pub mod misc;
pub mod optics_advanced;
pub mod ordered;
pub mod project;
pub mod query;
pub mod recursive;
pub mod transaction;
pub mod traversable;
pub mod typeclass;
pub mod workflow_eff;

pub use advanced::{async_pipeline, get_task_history, lazy_compute, transform_task};
pub use alternative::{
    aggregate_sources, filter_conditional, first_available, resolve_config, search_fallback,
};
pub use applicative::{build_from_parts, compute_parallel, dashboard, validate_collect_all};
pub use async_pipeline::{
    batch_process_async, conditional_pipeline, transform_async, workflow_async,
};
pub use bifunctor::{
    batch_transform_results, convert_error_domain, enrich_error, process_with_error_transform,
    transform_pair,
};
pub use bulk::{bulk_create_tasks, bulk_update_tasks};
pub use dto::{CreateTaskRequest, TaskResponse, UpdateTaskRequest};
pub use effects::{execute_state_workflow, execute_workflow, update_with_optics};
pub use error::{ApiError, ApiErrorResponse, FieldError, ValidationError};
pub use handlers::{
    AppConfig, AppState, HealthResponse, create_task, delete_task, get_task, health_check,
};
pub use misc::{
    aggregate_numeric, concurrent_lazy, deque_operations, freer_workflow, partial_apply,
};
pub use optics_advanced::{
    batch_update_field, nested_access, update_filtered, update_metadata_key, update_optional,
};
pub use ordered::{projects_leaderboard, tasks_by_deadline, tasks_timeline};
pub use project::{
    create_project_handler, get_project_handler, get_project_progress_handler,
    get_project_stats_handler,
};
pub use query::{count_by_priority, list_tasks, search_tasks};
pub use recursive::{aggregate_tree, flatten_subtasks, resolve_dependencies};
pub use transaction::{add_subtask, add_tag, update_status, update_task};
pub use traversable::{
    collect_optional, enrich_batch, execute_sequential, fetch_batch, validate_batch,
};
pub use typeclass::{
    flatten_demo, functor_mut_demo, identity_demo, monad_error_demo, monad_transformers,
};
pub use workflow_eff::create_task_eff;
