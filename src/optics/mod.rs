//! Optics for immutable data manipulation.
//!
//! This module provides optics - composable accessors for immutable
//! data structures. Optics allow you to focus on specific parts of a data
//! structure, enabling type-safe reading and updating of deeply nested fields.
//!
//! # Optics Hierarchy
//!
//! ```text
//! Iso <: Lens
//! Iso <: Prism
//! Lens <: Traversal
//! Prism <: Traversal
//! Lens + Prism = Optional
//! ```
//!
//! # Available Optics
//!
//! - [`Lens`]: Focus on a single field (get/set access)
//! - [`Prism`]: Focus on a variant of an enum (preview/review access)
//! - [`Optional`]: Focus on a value that may or may not exist (Lens + Prism composition)
//! - [`Iso`]: Isomorphism between types (bidirectional conversion)
//! - [`Traversal`]: Focus on multiple elements (batch access)
//!
//! # Example with Lens
//!
//! ```
//! use lambars::optics::{Lens, FunctionLens};
//! use lambars::lens;
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Address { street: String, city: String }
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Person { name: String, address: Address }
//!
//! // Create lenses using the macro
//! let address_lens = lens!(Person, address);
//! let street_lens = lens!(Address, street);
//!
//! // Compose lenses to focus on nested fields
//! let person_street = address_lens.compose(street_lens);
//!
//! let person = Person {
//!     name: "Alice".to_string(),
//!     address: Address {
//!         street: "Main St".to_string(),
//!         city: "Tokyo".to_string(),
//!     },
//! };
//!
//! // Get nested field
//! assert_eq!(*person_street.get(&person), "Main St");
//!
//! // Set nested field (returns new structure)
//! let updated = person_street.set(person, "Oak Ave".to_string());
//! assert_eq!(updated.address.street, "Oak Ave");
//! assert_eq!(updated.address.city, "Tokyo"); // Other fields unchanged
//! ```
//!
//! # Example with Prism
//!
//! ```
//! use lambars::optics::{Prism, FunctionPrism};
//! use lambars::prism;
//!
//! #[derive(Clone, PartialEq, Debug)]
//! enum Shape {
//!     Circle(f64),
//!     Rectangle(f64, f64),
//! }
//!
//! let circle_prism = prism!(Shape, Circle);
//!
//! let circle = Shape::Circle(5.0);
//! assert_eq!(circle_prism.preview(&circle), Some(&5.0));
//!
//! let rect = Shape::Rectangle(3.0, 4.0);
//! assert_eq!(circle_prism.preview(&rect), None);
//!
//! let constructed = circle_prism.review(10.0);
//! assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
//! ```
//!
//! # Example with Optional (Lens + Prism)
//!
//! ```
//! use lambars::optics::{Lens, LensComposeExtension, Prism, Optional};
//! use lambars::{lens, prism};
//!
//! #[derive(Clone, PartialEq, Debug)]
//! enum MyOption<T> { Some(T), None }
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Container { maybe_value: MyOption<i32> }
//!
//! let container_lens = lens!(Container, maybe_value);
//! let some_prism = prism!(MyOption<i32>, Some);
//! let optional = container_lens.compose_prism(some_prism);
//!
//! let some_container = Container { maybe_value: MyOption::Some(42) };
//! assert_eq!(optional.get_option(&some_container), Some(&42));
//!
//! let none_container = Container { maybe_value: MyOption::None };
//! assert_eq!(optional.get_option(&none_container), None);
//! ```
//!
//! # Example with Iso
//!
//! ```
//! use lambars::optics::{Iso, FunctionIso};
//! use lambars::iso;
//!
//! // String <-> Vec<char> isomorphism
//! let string_chars_iso = FunctionIso::new(
//!     |s: String| s.chars().collect::<Vec<_>>(),
//!     |chars: Vec<char>| chars.into_iter().collect::<String>(),
//! );
//!
//! let original = "hello".to_string();
//! let chars = string_chars_iso.get(original.clone());
//! assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
//!
//! // Roundtrip: get then reverse_get returns original
//! let back = string_chars_iso.reverse_get(chars);
//! assert_eq!(back, original);
//!
//! // Using the iso! macro
//! let swap = iso!(
//!     |(a, b): (i32, String)| (b, a),
//!     |(b, a): (String, i32)| (a, b)
//! );
//!
//! let tuple = (42, "hello".to_string());
//! let swapped = swap.get(tuple.clone());
//! assert_eq!(swapped, ("hello".to_string(), 42));
//! ```
//!
//! # Lens Laws
//!
//! Every Lens must satisfy three laws:
//!
//! 1. **GetPut Law**: Getting and setting back yields the original.
//!    ```text
//!    lens.set(source, lens.get(&source).clone()) == source
//!    ```
//!
//! 2. **PutGet Law**: Setting then getting yields the set value.
//!    ```text
//!    lens.get(&lens.set(source, value)) == &value
//!    ```
//!
//! 3. **PutPut Law**: Two consecutive sets is equivalent to the last set.
//!    ```text
//!    lens.set(lens.set(source, v1), v2) == lens.set(source, v2)
//!    ```
//!
//! # Prism Laws
//!
//! Every Prism must satisfy two laws:
//!
//! 1. **PreviewReview Law**: Reviewing then previewing yields the original value.
//!    ```text
//!    prism.preview(&prism.review(value)) == Some(&value)
//!    ```
//!
//! 2. **ReviewPreview Law**: If preview succeeds, reviewing the result yields the original.
//!    ```text
//!    if prism.preview(source).is_some() then
//!        prism.review(prism.preview(source).unwrap().clone()) == source
//!    ```
//!
//! # Optional Laws
//!
//! Every Optional must satisfy two laws (when the element is present):
//!
//! 1. **GetOptionSet Law**: Getting and setting back yields the original.
//!    ```text
//!    if optional.get_option(&source).is_some() then
//!        optional.set(source.clone(), optional.get_option(&source).unwrap().clone()) == source
//!    ```
//!
//! 2. **SetGetOption Law**: Setting then getting yields the set value.
//!    ```text
//!    if optional.get_option(&source).is_some() then
//!        optional.get_option(&optional.set(source, value)) == Some(&value)
//!    ```
//!
//! # Iso Laws
//!
//! Every Iso must satisfy two laws:
//!
//! 1. **GetReverseGet Law**: Converting forward then backward yields the original.
//!    ```text
//!    iso.reverse_get(iso.get(source)) == source
//!    ```
//!
//! 2. **ReverseGetGet Law**: Converting backward then forward yields the original.
//!    ```text
//!    iso.get(iso.reverse_get(value)) == value
//!    ```

mod iso;
mod lens;
mod optional;
pub mod persistent_optics;
mod prism;
mod standard_optics;
mod traversal;

// Re-export all lens-related types and traits
pub use lens::ComposedLens;
pub use lens::FunctionLens;
pub use lens::Lens;
pub use lens::LensAsTraversal;

// Re-export all prism-related types and traits
pub use prism::ComposedPrism;
pub use prism::FunctionPrism;
pub use prism::Prism;
pub use prism::PrismAsTraversal;

// Re-export all optional-related types and traits
pub use optional::ComposedOptional;
pub use optional::LensComposeExtension;
pub use optional::LensPrismComposition;
pub use optional::Optional;

// Re-export all iso-related types and traits
pub use iso::ComposedIso;
pub use iso::FunctionIso;
pub use iso::Iso;
pub use iso::IsoAsLens;
pub use iso::IsoAsPrism;
pub use iso::ReversedIso;

// Re-export standard optics
pub use standard_optics::iso_identity;
pub use standard_optics::iso_swap;

// Re-export all traversal-related types and traits
pub use traversal::ComposedTraversal;
pub use traversal::OptionTraversal;
pub use traversal::ResultTraversal;
pub use traversal::Traversal;
pub use traversal::VecTraversal;
