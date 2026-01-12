//! Integration tests for infrastructure adapters.
//!
//! These tests require Docker containers to be running:
//!
//! ```bash
//! cd samples/roguelike
//! docker compose up -d mysql redis
//! ```
//!
//! # Environment Requirements
//!
//! - MySQL: `mysql://roguelike:roguelikepassword@localhost:3306/roguelike`
//! - Redis: `redis://localhost:6379`

mod mysql_tests;
mod redis_tests;
