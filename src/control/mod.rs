//! Control structures for functional programming.
//!
//! This module provides control structures that enable functional
//! programming patterns:
//!
//! - [`Either`]: A value that can be one of two types (used by Trampoline)
//! - [`Lazy`]: Lazy evaluation with memoization (single-threaded)
//! - [`ConcurrentLazy`]: Thread-safe lazy evaluation with memoization
//! - [`Trampoline`]: Stack-safe recursion
//! - [`Continuation`]: Continuation monad for CPS
//! - [`Freer`]: Freer monad for DSL construction
//!
//! # Examples
//!
//! ## Lazy Evaluation
//!
//! ```rust
//! use lambars::control::Lazy;
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
//! ## Thread-Safe Lazy Evaluation
//!
//! ```rust
//! use lambars::control::ConcurrentLazy;
//! use std::sync::Arc;
//! use std::thread;
//!
//! let lazy = Arc::new(ConcurrentLazy::new(|| 42));
//!
//! let handles: Vec<_> = (0..10).map(|_| {
//!     let lazy = Arc::clone(&lazy);
//!     thread::spawn(move || *lazy.force())
//! }).collect();
//!
//! for handle in handles {
//!     assert_eq!(handle.join().unwrap(), 42);
//! }
//! ```
//!
//! ## Stack-Safe Recursion
//!
//! ```rust
//! use lambars::control::Trampoline;
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

mod concurrent_lazy;
mod continuation;
mod either;
mod freer;
mod lazy;
mod trampoline;

pub use concurrent_lazy::{ConcurrentLazy, ConcurrentLazyPoisonedError};
pub use continuation::Continuation;
pub use either::Either;
pub use freer::{Freer, InterpretError};
pub use lazy::{Lazy, LazyPoisonedError};
pub use trampoline::Trampoline;
