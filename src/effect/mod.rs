//! Effect system for type-safe side effect handling.
//!
//! This module provides an effect system that allows tracking and
//! composing side effects at the type level.
//!
//! # MTL-Style Type Classes
//!
//! This module provides MTL (Monad Transformer Library) style type classes
//! that abstract common effect patterns:
//!
//! - [`MonadReader`]: Reading from an environment
//! - [`MonadState`]: Stateful computations
//! - [`MonadWriter`]: Accumulating output/logs
//! - [`MonadError`]: Error handling
//!
//! These type classes are designed to be implemented by various monad types
//! (base monads and transformers) to provide a uniform interface for
//! working with effects.
//!
//! # Base Monads
//!
//! - [`Reader`]: Computations that read from an environment
//! - [`State`]: Computations with mutable state
//! - [`Writer`]: Computations that accumulate output
//! - [`IO`]: Computations with deferred side effects
//!
//! # IO Monad
//!
//! The [`IO`] type represents a computation that may perform side effects.
//! Side effects are deferred until `run_unsafe` is called, maintaining
//! referential transparency in pure code.
//!
//! ```rust
//! use lambars::effect::IO;
//!
//! // Create and chain IO actions
//! let io = IO::pure(10)
//!     .fmap(|x| x * 2)
//!     .flat_map(|x| IO::pure(x + 1));
//!
//! // Side effects don't occur until run_unsafe is called
//! assert_eq!(io.run_unsafe(), 21);
//! ```
//!
//! # Do-Notation with eff! Macro
//!
//! The `eff!` macro provides a convenient syntax for chaining monadic
//! operations, similar to Haskell's do-notation:
//!
//! ```rust
//! use lambars::eff;
//! use lambars::typeclass::Monad;
//!
//! let result = eff! {
//!     x <= Some(5);
//!     y <= Some(10);
//!     let z = x + y;
//!     Some(z * 2)
//! };
//! assert_eq!(result, Some(30));
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::MonadError;
//! use lambars::typeclass::Monad;
//!
//! // Using MonadError with Result
//! let computation: Result<i32, String> = Ok(42);
//! let with_recovery = <Result<i32, String>>::catch_error(computation, |e| {
//!     Ok(e.len() as i32)
//! });
//! assert_eq!(with_recovery, Ok(42));
//!
//! let failing: Result<i32, String> = Err("error".to_string());
//! let recovered = <Result<i32, String>>::catch_error(failing, |e| {
//!     Ok(e.len() as i32)
//! });
//! assert_eq!(recovered, Ok(5));
//! ```

// =============================================================================
// MTL-Style Type Classes
// =============================================================================

mod monad_error;
mod monad_reader;
mod monad_state;
mod monad_writer;

pub use monad_error::MonadError;
pub use monad_reader::MonadReader;
pub use monad_state::MonadState;
pub use monad_writer::MonadWriter;

// =============================================================================
// Base Monads
// =============================================================================

mod reader;
mod state;
mod writer;

pub use reader::Reader;
pub use state::State;
pub use writer::Writer;

// =============================================================================
// IO Monad
// =============================================================================

mod io;

pub use io::IO;

// =============================================================================
// AsyncIO Monad (requires async feature)
// =============================================================================

#[cfg(feature = "async")]
mod async_io;

#[cfg(feature = "async")]
pub use async_io::AsyncIO;

#[cfg(feature = "async")]
pub use async_io::TimeoutError;

// =============================================================================
// Do-Notation Macros
// =============================================================================

mod eff_macro;

#[cfg(feature = "async")]
mod eff_async_macro;

// =============================================================================
// Monad Transformers
// =============================================================================

mod except_transformer;
mod reader_transformer;
mod state_transformer;
mod writer_transformer;

pub use except_transformer::ExceptT;
pub use reader_transformer::ReaderT;
pub use state_transformer::StateT;
pub use writer_transformer::WriterT;
