//! API module for HTTP handlers.
//!
//! This module contains route definitions and request/response handlers.

pub mod dto;
pub mod error;
pub mod handlers;
pub mod query;
pub mod transaction;
pub mod workflow_eff;

pub use dto::{CreateTaskRequest, TaskResponse, UpdateTaskRequest};
pub use error::{ApiError, ApiErrorResponse, FieldError, ValidationError};
pub use handlers::{AppState, HealthResponse, create_task, health_check};
pub use query::{count_by_priority, list_tasks, search_tasks};
pub use transaction::{add_subtask, add_tag, update_status, update_task};
pub use workflow_eff::create_task_eff;
