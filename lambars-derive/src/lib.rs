//! Derive macros for lambars optics and function composition.
//!
//! This crate provides procedural macros for automatically generating
//! optics (Lens and Prism) implementations for Rust types, as well as
//! function composition utilities.
//!
//! # Available Derive Macros
//!
//! - [`Lenses`]: Generates lens methods for struct fields
//! - [`Prisms`]: Generates prism methods for enum variants
//!
//! # Available Function-like Macros
//!
//! - [`curry!`]: Converts multi-argument closures into curried form
//!
//! # Example: Lenses
//!
//! ```rust,ignore
//! use lambars_derive::Lenses;
//! use lambars::optics::Lens;
//!
//! #[derive(Clone, Lenses)]
//! struct Point {
//!     x: i32,
//!     y: i32,
//! }
//!
//! // Generated methods:
//! // - Point::x_lens() -> impl Lens<Point, i32>
//! // - Point::y_lens() -> impl Lens<Point, i32>
//!
//! let point = Point { x: 10, y: 20 };
//! let x_lens = Point::x_lens();
//! assert_eq!(*x_lens.get(&point), 10);
//! ```
//!
//! # Example: Prisms
//!
//! ```rust,ignore
//! use lambars_derive::Prisms;
//! use lambars::optics::Prism;
//!
//! #[derive(Clone, Prisms)]
//! enum Shape {
//!     Circle(f64),
//!     Rectangle(f64, f64),
//! }
//!
//! // Generated methods:
//! // - Shape::circle_prism() -> impl Prism<Shape, f64>
//! // - Shape::rectangle_prism() -> impl Prism<Shape, (f64, f64)>
//!
//! let circle = Shape::Circle(5.0);
//! let circle_prism = Shape::circle_prism();
//! assert_eq!(circle_prism.preview(&circle), Some(&5.0));
//! ```
//!
//! # Example: Currying
//!
//! ```rust,ignore
//! use lambars::curry;
//!
//! // Curry a closure
//! let add = curry!(|a: i32, b: i32| a + b);
//! assert_eq!(add(5)(3), 8);
//!
//! // Partial application
//! let add_five = add(5);
//! assert_eq!(add_five(10), 15);
//! assert_eq!(add_five(20), 25);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod curry;
mod lenses;
mod prisms;

use proc_macro::TokenStream;

/// Derive macro for generating Lens implementations for struct fields.
///
/// This macro generates a method for each field in the struct that returns
/// a lens focusing on that field. The method name follows the pattern
/// `{field_name}_lens()`.
///
/// # Requirements
///
/// - The struct must be a named struct (not a tuple struct)
/// - The struct should implement `Clone` for `modify` operations
///
/// # Generated Code
///
/// For each field `foo` of type `T`, generates:
///
/// ```rust,ignore
/// impl StructName {
///     pub fn foo_lens() -> impl Lens<StructName, T> + Clone { ... }
/// }
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use lambars_derive::Lenses;
/// use lambars::optics::Lens;
///
/// #[derive(Clone, Debug, PartialEq, Lenses)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let person = Person {
///     name: "Alice".to_string(),
///     age: 30,
/// };
///
/// let name_lens = Person::name_lens();
/// assert_eq!(*name_lens.get(&person), "Alice");
///
/// let updated = name_lens.set(person, "Bob".to_string());
/// assert_eq!(updated.name, "Bob");
/// ```
///
/// # Generics
///
/// The macro supports generic structs. For each generic parameter `T`,
/// you can call the lens method on the concrete type:
///
/// ```rust,ignore
/// #[derive(Clone, Lenses)]
/// struct Container<T> {
///     value: T,
/// }
///
/// let container = Container { value: 42 };
/// let lens = Container::<i32>::value_lens();
/// assert_eq!(*lens.get(&container), 42);
/// ```
#[proc_macro_derive(Lenses)]
pub fn derive_lenses(input: TokenStream) -> TokenStream {
    lenses::derive_lenses_impl(input)
}

/// Derive macro for generating Prism implementations for enum variants.
///
/// This macro generates a method for each variant in the enum that returns
/// a prism focusing on that variant. The method name follows the pattern
/// `{variant_name_snake_case}_prism()`.
///
/// # Requirements
///
/// - The type must be an enum
/// - The enum should implement `Clone` for `modify_or_identity` operations
///
/// # Variant Types
///
/// The macro handles different variant types:
///
/// - **Unit variants** (e.g., `None`): Returns `impl Prism<Enum, ()>`
/// - **Single-field tuple variants** (e.g., `Some(T)`): Returns `impl Prism<Enum, T>`
/// - **Multi-field tuple variants** (e.g., `Point(i32, i32)`): Returns `impl Prism<Enum, (T1, T2, ...)>`
/// - **Struct variants** (e.g., `Click { x: i32, y: i32 }`): Returns `impl Prism<Enum, (T1, T2, ...)>`
///
/// # Generated Code
///
/// For each variant, generates:
///
/// ```rust,ignore
/// impl EnumName {
///     pub fn variant_name_prism() -> impl Prism<EnumName, TargetType> + Clone { ... }
/// }
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use lambars_derive::Prisms;
/// use lambars::optics::Prism;
///
/// #[derive(Clone, Debug, PartialEq, Prisms)]
/// enum Shape {
///     Circle(f64),
///     Rectangle(f64, f64),
///     Point,
/// }
///
/// let circle = Shape::Circle(5.0);
/// let circle_prism = Shape::circle_prism();
/// assert_eq!(circle_prism.preview(&circle), Some(&5.0));
///
/// let rect = Shape::Rectangle(3.0, 4.0);
/// let rect_prism = Shape::rectangle_prism();
/// assert_eq!(rect_prism.preview(&rect), Some(&(3.0, 4.0)));
///
/// let point = Shape::Point;
/// let point_prism = Shape::point_prism();
/// assert_eq!(point_prism.preview(&point), Some(&()));
/// ```
///
/// # Struct Variants
///
/// For struct variants, the fields are converted to a tuple in definition order:
///
/// ```rust,ignore
/// #[derive(Clone, Prisms)]
/// enum Event {
///     Click { x: i32, y: i32 },
/// }
///
/// let click = Event::Click { x: 10, y: 20 };
/// let click_prism = Event::click_prism();
/// assert_eq!(click_prism.preview(&click), Some(&(10, 20)));
/// ```
///
/// # Generics
///
/// The macro supports generic enums:
///
/// ```rust,ignore
/// #[derive(Clone, Prisms)]
/// enum MyOption<T> {
///     Some(T),
///     None,
/// }
///
/// let some = MyOption::Some(42);
/// let prism = MyOption::<i32>::some_prism();
/// assert_eq!(prism.preview(&some), Some(&42));
/// ```
#[proc_macro_derive(Prisms)]
pub fn derive_prisms(input: TokenStream) -> TokenStream {
    prisms::derive_prisms_impl(input)
}

/// Converts a multi-argument closure into curried form.
///
/// Currying transforms a closure that takes multiple arguments into
/// a sequence of closures, each taking a single argument.
///
/// # Usage
///
/// ```rust,ignore
/// use lambars::curry;
///
/// // With a closure
/// let curried = curry!(|a: i32, b: i32| a + b);
/// assert_eq!(curried(5)(3), 8);
///
/// // Partial application
/// let add_five = curried(5);
/// assert_eq!(add_five(10), 15);
/// assert_eq!(add_five(20), 25);
/// ```
///
/// # Wrapping existing functions
///
/// To curry an existing function, wrap it in a closure:
///
/// ```rust,ignore
/// use lambars::curry;
///
/// fn add(a: i32, b: i32) -> i32 { a + b }
///
/// // Wrap the function in a closure
/// let curried = curry!(|a, b| add(a, b));
/// assert_eq!(curried(5)(3), 8);
/// ```
///
/// # Multi-argument closures
///
/// The macro supports closures with 2 or more arguments:
///
/// ```rust,ignore
/// use lambars::curry;
///
/// // 3 arguments
/// let curried = curry!(|a: i32, b: i32, c: i32| a + b + c);
/// assert_eq!(curried(1)(2)(3), 6);
///
/// // 6 arguments
/// let curried = curry!(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| {
///     a + b + c + d + e + f
/// });
/// assert_eq!(curried(1)(2)(3)(4)(5)(6), 21);
/// ```
///
/// # Reusability
///
/// Curried closures and their partial applications can be reused:
///
/// ```rust,ignore
/// use lambars::curry;
///
/// let multiply = curry!(|first: i32, second: i32| first * second);
/// let double = multiply(2);
/// let triple = multiply(3);
///
/// assert_eq!(double(5), 10);
/// assert_eq!(triple(5), 15);
/// assert_eq!(double(5), 10); // Still works!
/// ```
///
/// # Type constraints
///
/// - **Arguments (except the last)**: Must implement `Clone`
/// - **Last argument**: No special constraints
///
/// This is because `Rc::unwrap_or_clone` is used internally to
/// enable reuse of partial applications.
///
/// ```rust,ignore
/// struct NonClone(i32);
///
/// // OK: NonClone as last argument
/// let curried = curry!(|a: i32, b: NonClone| a + b.0);
///
/// // Error: NonClone as non-last argument (Clone required)
/// // let curried = curry!(|a: NonClone, b: i32| a.0 + b);
/// ```
///
/// # Implementation Notes
///
/// The generated code uses `std::rc::Rc` to share the closure and
/// arguments across nested closures. This enables:
///
/// - Multiple calls to the same curried closure
/// - Reuse of partial applications
/// - Support for non-Copy argument types (requires Clone)
#[proc_macro]
pub fn curry(input: TokenStream) -> TokenStream {
    curry::curry_impl(input)
}
