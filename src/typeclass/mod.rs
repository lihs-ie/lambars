//! Type class traits for functional programming abstractions.
//!
//! This module provides the fundamental type classes (traits) that form
//! the foundation of functional programming in Rust:
//!
//! - [`Functor`]: Mapping over container values
//! - [`FunctorMut`]: Mapping with mutable functions for multi-element containers
//! - [`Bifunctor`]: Mapping over two type parameters
//! - [`Applicative`]: Applying functions within containers
//! - [`Alternative`]: Monoid structure on Applicative functors
//! - [`Monad`]: Sequencing computations with dependency
//! - [`Flatten`]: Flattening nested monadic structures
//! - [`Foldable`]: Folding over structures to produce summary values
//! - [`Traversable`]: Traversing structures with effects
//! - [`Semigroup`]: Associative binary operations
//! - [`Monoid`]: Semigroup with identity element
//!
//! ## Higher-Kinded Types Emulation
//!
//! Rust does not have native support for higher-kinded types (HKT).
//! This library uses Generic Associated Types (GAT) to emulate HKT
//! behavior, allowing us to define traits like Functor and Monad
//! in a generic way.
//!
//! ## Foundation Types
//!
//! - [`TypeConstructor`]: Trait for emulating higher-kinded types
//! - [`Identity`]: Identity wrapper type (identity functor)
//! - [`Sum`], [`Product`]: Numeric wrappers for different monoid operations
//! - [`Max`], [`Min`]: Bounded numeric wrappers
//! - [`Bounded`]: Trait for types with minimum and maximum values
//!
//! ## Algebraic Structures
//!
//! - [`Semigroup`]: Types with an associative binary operation (`combine`)
//! - [`Monoid`]: Semigroups with an identity element (`empty`)
//!
//! # Examples
//!
//! ## Using Semigroup
//!
//! ```rust
//! use lambars::typeclass::Semigroup;
//!
//! // String concatenation
//! let hello = String::from("Hello, ");
//! let world = String::from("World!");
//! assert_eq!(hello.combine(world), "Hello, World!");
//!
//! // Vec concatenation
//! let vec1 = vec![1, 2];
//! let vec2 = vec![3, 4];
//! assert_eq!(vec1.combine(vec2), vec![1, 2, 3, 4]);
//! ```
//!
//! ## Using Monoid
//!
//! ```rust
//! use lambars::typeclass::{Semigroup, Monoid, Sum};
//!
//! // Combining with identity element
//! let value = String::from("hello");
//! assert_eq!(String::empty().combine(value.clone()), value);
//!
//! // Folding a collection with combine_all
//! let numbers = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
//! assert_eq!(Sum::combine_all(numbers), Sum::new(6));
//! ```
//!
//! ## Using Applicative
//!
//! ```rust
//! use lambars::typeclass::Applicative;
//!
//! // Lifting a pure value
//! let x: Option<i32> = <Option<()>>::pure(42);
//! assert_eq!(x, Some(42));
//!
//! // Combining two Option values
//! let a = Some(1);
//! let b = Some(2);
//! let sum = a.map2(b, |x, y| x + y);
//! assert_eq!(sum, Some(3));
//! ```
//!
//! ## Using Alternative
//!
//! ```rust
//! use lambars::typeclass::{Alternative, Functor};
//!
//! // Using empty as a failure value
//! let empty: Option<i32> = <Option<()>>::empty();
//! assert_eq!(empty, None);
//!
//! // Using alt for fallback
//! let first: Option<i32> = None;
//! let second: Option<i32> = Some(42);
//! assert_eq!(first.alt(second), Some(42));
//!
//! // Using guard for conditional filtering
//! fn filter_positive(n: i32) -> Option<i32> {
//!     <Option<()>>::guard(n > 0).fmap(move |_| n)
//! }
//! assert_eq!(filter_positive(5), Some(5));
//! assert_eq!(filter_positive(-3), None);
//! ```

mod alternative;
mod applicative;
mod bifunctor;
mod foldable;
mod functor;
mod higher;
mod identity;
mod monad;
mod monoid;
mod semigroup;
mod traversable;
mod wrappers;

pub use alternative::{Alternative, AlternativeVec};
pub use applicative::{Applicative, ApplicativeVec};
pub use bifunctor::Bifunctor;
pub use foldable::Foldable;
pub use functor::{Functor, FunctorMut};
pub use higher::TypeConstructor;
pub use identity::Identity;
pub use monad::{Flatten, Monad, MonadVec};
pub use monoid::Monoid;
pub use semigroup::Semigroup;
pub use traversable::Traversable;
pub use wrappers::{Bounded, Max, Min, Product, Sum};

#[cfg(feature = "effect")]
pub use traversable::{IOLike, ReaderLike, StateLike};

#[cfg(all(feature = "effect", feature = "async"))]
pub use traversable::AsyncIOLike;
