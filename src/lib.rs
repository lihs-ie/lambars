//! # lambars
//!
//! A functional programming library for Rust providing type classes,
//! persistent data structures, and effect systems.
//!
//! ## Overview
//!
//! This library aims to bring functional programming abstractions to Rust
//! that are not provided by the standard library. It includes:
//!
//! - **Type Classes**: Functor, Applicative, Monad, Foldable, Traversable, etc.
//! - **Function Composition**: compose!, pipe!, partial!, curry! macros
//! - **Control Structures**: Lazy evaluation, Trampoline for stack-safe recursion
//! - **Persistent Data Structures**: Immutable Vector, `HashMap`, `HashSet`, List
//! - **Optics**: Lens, Prism, Iso, Traversal for immutable data manipulation
//! - **Effect System**: Type-safe effect handling and composition
//!
//! ## Feature Flags
//!
//! - `typeclass`: Type class traits (Functor, Monad, etc.)
//! - `compose`: Function composition utilities
//! - `control`: Control structures (Lazy, Trampoline)
//! - `persistent`: Persistent data structures
//! - `optics`: Optics (Lens, Prism, etc.)
//! - `effect`: Effect system
//! - `full`: Enable all features
//!
//! ## Example
//!
//! ```rust
//! use lambars::prelude::*;
//!
//! // Example usage will be added as features are implemented
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
// Note: Disabling redundant_closure_for_method_calls due to clippy 0.1.92 panic bug
#![allow(clippy::redundant_closure_for_method_calls)]

/// Prelude module for convenient imports.
///
/// Re-exports commonly used types and traits.
///
/// # Usage
///
/// ```rust
/// use lambars::prelude::*;
/// ```
pub mod prelude {

    #[cfg(feature = "typeclass")]
    pub use crate::typeclass::*;

    #[cfg(feature = "compose")]
    pub use crate::compose::*;

    #[cfg(feature = "control")]
    pub use crate::control::*;

    #[cfg(feature = "persistent")]
    pub use crate::persistent::*;

    #[cfg(feature = "optics")]
    pub use crate::optics::*;

    #[cfg(feature = "effect")]
    pub use crate::effect::*;
}

#[cfg(feature = "typeclass")]
pub mod typeclass;

#[cfg(feature = "compose")]
pub mod compose;

#[cfg(feature = "control")]
pub mod control;

#[cfg(feature = "persistent")]
pub mod persistent;

#[cfg(feature = "optics")]
pub mod optics;

#[cfg(feature = "effect")]
pub mod effect;

#[cfg(test)]
mod tests {
    #[test]
    fn library_compiles() {
        // Basic smoke test to ensure the library compiles
        assert!(true);
    }
}
