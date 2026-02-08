//! Task Management Benchmark API Library
//!
//! This library provides the core functionality for the task management
//! benchmark application, demonstrating the lambars library.

#![allow(deprecated)]
// AppState contains multiple Arc fields; some test-only code paths create temporaries
// that exceed clippy's 16KiB stack array threshold due to monomorphisation.
#![cfg_attr(test, allow(clippy::large_stack_arrays))]

pub mod api;
pub mod domain;
pub mod infrastructure;
