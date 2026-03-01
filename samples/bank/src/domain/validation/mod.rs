//! Validation types for parallel error accumulation.
//!
//! This module provides the `Validated` type which allows accumulating
//! multiple validation errors using Applicative semantics.

mod validated;

pub use validated::{Validated, ValidationError, ValidationErrors};
