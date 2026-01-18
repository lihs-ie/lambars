//! API module for HTTP handlers.
//!
//! This module contains route definitions and request/response handlers.

pub mod dto;
pub mod error;
pub mod handlers;
pub mod workflow_eff;

pub use dto::{CreateTaskRequest, TaskResponse, UpdateTaskRequest};
pub use error::{ApiError, ApiErrorResponse, FieldError, ValidationError};
pub use handlers::{AppState, HealthResponse, create_task, health_check};
pub use workflow_eff::create_task_eff;
