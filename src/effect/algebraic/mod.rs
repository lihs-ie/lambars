//! Algebraic Effects - a unified approach to handling computational effects.
//!
//! This module provides an algebraic effects system that allows:
//!
//! - Declaring effects as types
//! - Writing computations that use effects
//! - Handling effects with different interpretations
//!
//! # Core Concepts
//!
//! - [`Effect`]: A trait marking effect types
//! - [`Eff`]: A computation that may use effects
//! - [`Handler`]: Interprets effect operations
//!
//! # Effect Composition
//!
//! Multiple effects can be composed using effect rows:
//!
//! - [`EffNil`]: Empty effect row
//! - [`EffCons`]: Prepend an effect to a row
//! - [`Member`]: Proves an effect is in a row
//! - `EffectRow!`: Macro for convenient row construction
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
//!
//! // Pure computation using Eff
//! let computation: Eff<NoEffect, i32> = Eff::pure(42);
//! let result = PureHandler.run(computation);
//! assert_eq!(result, 42);
//! ```
//!
//! Effect rows for multiple effects:
//!
//! ```rust
//! use lambars::EffectRow;
//! use lambars::effect::algebraic::{ReaderEffect, StateEffect, Effect};
//!
//! // Create a row with multiple effects
//! type MyEffects = EffectRow![ReaderEffect<i32>, StateEffect<String>];
//!
//! fn assert_effect<T: Effect>() {}
//! assert_effect::<MyEffects>();
//! ```

mod eff;
mod effect;
mod error;
mod handler;
pub mod interop;
pub mod macros;
mod member;
mod reader;
mod row;
mod state;
mod writer;

pub use eff::{Eff, OperationTag};
pub use effect::{Effect, NoEffect};
pub use error::{ErrorEffect, ErrorHandler, attempt, catch};
pub use handler::{ComposedHandler, Handler, PureHandler};
pub use interop::{FromEff, IntoEff};
pub use member::{FindIndex, Here, Member, There};
pub use reader::{ReaderEffect, ReaderHandler, run_local};
pub use row::{EffCons, EffNil};
pub use state::{StateEffect, StateHandler};
pub use writer::{WriterEffect, WriterHandler, listen};
