//! # Order Taking Sample Application
//!
//! A sample order processing application using lambars.
//!
//! ## Overview
//!
//! Based on the F# book "Domain Modeling Made Functional",
//! this sample demonstrates functional domain modeling in Rust.
//!
//! ## Module Structure
//!
//! - `simple_types`: Constrained primitive types (`String50`, `EmailAddress`, `OrderId`, etc.)
//! - `compound_types`: Compound types (`PersonalName`, `CustomerInfo`, `Address`, etc.)
//! - `workflow`: Workflow type definitions (state transitions expressed via types)

#![forbid(unsafe_code)]

pub mod api;
pub mod compound_types;
pub mod dto;
pub mod simple_types;
pub mod workflow;
