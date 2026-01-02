//! Function composition utilities.
//!
//! This module provides macros and functions for composing functions
//! in a functional programming style. It enables declarative, point-free
//! programming patterns that are common in functional languages.
//!
//! # Overview
//!
//! The module provides the following utilities:
//!
//! - [`compose!`]: Compose functions right-to-left (mathematical composition)
//! - [`pipe!`]: Compose functions left-to-right (data flow style)
//! - [`partial!`]: Partial function application with placeholder support
//! - [`curry2!`] through [`curry6!`]: Convert multi-argument functions to curried form
//! - [`for_!`]: Scala-style for-comprehension over iterators
//!
//! # Helper Functions
//!
//! - [`identity`]: The identity function - returns its argument unchanged
//! - [`constant`]: Creates a function that always returns the same value
//! - [`flip`]: Swaps the arguments of a binary function
//!
//! # Examples
//!
//! ## Function Composition (right-to-left)
//!
//! ```
//! use lambars::compose;
//!
//! fn add_one(x: i32) -> i32 { x + 1 }
//! fn double(x: i32) -> i32 { x * 2 }
//!
//! // compose!(f, g)(x) = f(g(x))
//! let composed = compose!(add_one, double);
//! assert_eq!(composed(5), 11); // add_one(double(5)) = add_one(10) = 11
//! ```
//!
//! ## Pipeline (left-to-right)
//!
//! ```
//! use lambars::pipe;
//!
//! fn add_one(x: i32) -> i32 { x + 1 }
//! fn double(x: i32) -> i32 { x * 2 }
//!
//! // pipe!(x, f, g) = g(f(x))
//! let result = pipe!(5, double, add_one);
//! assert_eq!(result, 11); // add_one(double(5)) = 11
//! ```
//!
//! ## Partial Application
//!
//! ```
//! use lambars::partial;
//!
//! fn add(first: i32, second: i32) -> i32 { first + second }
//!
//! // Use __ as a placeholder for arguments that should remain as parameters.
//! // Note: Do NOT import __ - it is matched as a literal token by the macro.
//! let add_five = partial!(add, 5, __);
//! assert_eq!(add_five(3), 8);
//! ```
//!
//! ## Currying
//!
//! ```
//! use lambars::curry2;
//!
//! fn add(first: i32, second: i32) -> i32 { first + second }
//!
//! let curried_add = curry2!(add);
//! let add_five = curried_add(5);
//! assert_eq!(add_five(3), 8);
//! ```
//!
//! # Mathematical Background
//!
//! ## Function Composition
//!
//! Function composition creates a new function by combining two functions.
//! Given `f: B -> C` and `g: A -> B`, the composition `(f . g): A -> C` is defined as:
//!
//! ```text
//! (f . g)(x) = f(g(x))
//! ```
//!
//! The [`compose!`] macro implements this right-to-left composition.
//!
//! ## Pipeline
//!
//! Pipeline is the reverse notation, reading left-to-right:
//!
//! ```text
//! x |> f |> g |> h = h(g(f(x)))
//! ```
//!
//! The [`pipe!`] macro implements this pattern, which often matches the
//! mental model of data flowing through transformations.
//!
//! ## Partial Application
//!
//! Partial application fixes some arguments of a function, producing a new
//! function with fewer arguments:
//!
//! ```text
//! partial(f, a, _)(b) = f(a, b)
//! ```
//!
//! ## Currying
//!
//! Currying transforms a multi-argument function into a sequence of
//! single-argument functions:
//!
//! ```text
//! curry(f)(a)(b)(c) = f(a, b, c)
//! ```
//!
//! # Laws
//!
//! ## Composition Laws
//!
//! - **Associativity**: `compose!(f, compose!(g, h)) == compose!(compose!(f, g), h)`
//! - **Left Identity**: `compose!(identity, f) == f`
//! - **Right Identity**: `compose!(f, identity) == f`
//!
//! ## Flip Laws
//!
//! - **Double Flip Identity**: `flip(flip(f)) == f`
//! - **Flip Definition**: `flip(f)(a, b) == f(b, a)`

mod compose_macro;
mod curry_macro;
#[cfg(feature = "async")]
mod for_async_macro;
mod for_macro;
mod partial_macro;
mod pipe_macro;
mod utils;

// Re-export helper functions
pub use utils::{__, Placeholder, constant, flip, identity};

// Re-export macros (they are already at crate root via #[macro_export])
pub use crate::compose;
pub use crate::curry2;
pub use crate::curry3;
pub use crate::curry4;
pub use crate::curry5;
pub use crate::curry6;
pub use crate::for_;
pub use crate::partial;
pub use crate::pipe;
