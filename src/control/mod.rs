//! Control structures for functional programming.
//!
//! This module provides control structures that enable functional
//! programming patterns:
//!
//! - [`Either`]: A value that can be one of two types (used by Trampoline)
//! - [`Lazy`]: Lazy evaluation with memoization
//! - [`Trampoline`]: Stack-safe recursion
//! - [`Continuation`]: Continuation monad for CPS
//!
//! # Examples
//!
//! ## Lazy Evaluation
//!
//! ```rust
//! use functional_rusty::control::Lazy;
//!
//! let lazy = Lazy::new(|| {
//!     println!("Computing...");
//!     42
//! });
//! // "Computing..." is not printed yet
//!
//! let value = lazy.force();
//! // Now "Computing..." is printed and value is 42
//! assert_eq!(*value, 42);
//! ```
//!
//! ## Stack-Safe Recursion
//!
//! ```rust
//! use functional_rusty::control::Trampoline;
//!
//! fn factorial(n: u64) -> Trampoline<u64> {
//!     factorial_helper(n, 1)
//! }
//!
//! fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
//!     if n <= 1 {
//!         Trampoline::done(accumulator)
//!     } else {
//!         Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
//!     }
//! }
//!
//! let result = factorial(10).run();
//! assert_eq!(result, 3628800);
//! ```

mod continuation;
mod either;
mod lazy;
mod trampoline;

pub use continuation::Continuation;
pub use either::Either;
pub use lazy::{Lazy, LazyState};
pub use trampoline::Trampoline;
