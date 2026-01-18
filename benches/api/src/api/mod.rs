//! API module for HTTP handlers.
//!
//! This module contains route definitions and request/response handlers.

pub mod advanced;
pub mod bulk;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod project;
pub mod query;
pub mod transaction;
pub mod workflow_eff;

pub use advanced::{async_pipeline, get_task_history, lazy_compute, transform_task};
pub use bulk::{bulk_create_tasks, bulk_update_tasks};
pub use dto::{CreateTaskRequest, TaskResponse, UpdateTaskRequest};
pub use error::{ApiError, ApiErrorResponse, FieldError, ValidationError};
pub use handlers::{AppConfig, AppState, HealthResponse, create_task, health_check};
pub use project::{
    create_project_handler, get_project_handler, get_project_progress_handler,
    get_project_stats_handler,
};
pub use query::{count_by_priority, list_tasks, search_tasks};
pub use transaction::{add_subtask, add_tag, update_status, update_task};
pub use workflow_eff::create_task_eff;
