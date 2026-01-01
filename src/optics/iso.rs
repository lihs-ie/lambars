//! Iso optics for isomorphic type conversions.
//!
//! An Iso (isomorphism) is an optic that represents a bidirectional conversion
//! between two types where no information is lost. It is the strongest form
//! of optic, meaning it can be used as either a Lens or a Prism.
//!
//! # Laws
//!
//! Every Iso must satisfy two laws:
//!
//! 1. **`GetReverseGet` Law**: Converting forward then backward yields the original.
//!    ```text
//!    iso.reverse_get(iso.get(source)) == source
//!    ```
//!
//! 2. **`ReverseGetGet` Law**: Converting backward then forward yields the original.
//!    ```text
//!    iso.get(iso.reverse_get(value)) == value
//!    ```
//!
//! # Examples
//!
//! ```
//! use lambars::optics::{Iso, FunctionIso};
//! use lambars::iso;
//!
//! // String <-> Vec<char> conversion
//! let string_chars_iso = FunctionIso::new(
//!     |s: String| s.chars().collect::<Vec<_>>(),
//!     |chars: Vec<char>| chars.into_iter().collect::<String>(),
//! );
//!
//! let original = "hello".to_string();
//! let chars = string_chars_iso.get(original.clone());
//! assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
//!
//! let back = string_chars_iso.reverse_get(chars);
//! assert_eq!(back, original);
//! ```

use std::marker::PhantomData;

use super::{Lens, Prism};

/// An Iso represents an isomorphism between two types.
///
/// An isomorphism is a bidirectional conversion where no information is lost.
/// This is the strongest form of optic.
///
/// # Type Parameters
///
/// - `S`: The source type
/// - `A`: The target type
///
/// # Laws
///
/// 1. **`GetReverseGet` Law**: `iso.reverse_get(iso.get(source)) == source`
/// 2. **`ReverseGetGet` Law**: `iso.get(iso.reverse_get(value)) == value`
pub trait Iso<S, A> {
    /// Converts from the source type to the target type.
    ///
    /// # Arguments
    ///
    /// * `source` - The source value to convert
    ///
    /// # Returns
    ///
    /// The converted target value
    fn get(&self, source: S) -> A;

    /// Converts from the target type back to the source type.
    ///
    /// # Arguments
    ///
    /// * `value` - The target value to convert back
    ///
    /// # Returns
    ///
    /// The converted source value
    fn reverse_get(&self, value: A) -> S;

    /// Returns the reversed Iso (swaps the direction).
    ///
    /// The reversed Iso converts from A to S using `reverse_get` as `get`
    /// and `get` as `reverse_get`.
    ///
    /// # Returns
    ///
    /// A reversed Iso
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Iso, FunctionIso};
    ///
    /// let string_chars_iso = FunctionIso::new(
    ///     |s: String| s.chars().collect::<Vec<_>>(),
    ///     |chars: Vec<char>| chars.into_iter().collect::<String>(),
    /// );
    ///
    /// let chars_string_iso = string_chars_iso.reverse();
    ///
    /// let chars = vec!['h', 'i'];
    /// let string = chars_string_iso.get(chars);
    /// assert_eq!(string, "hi");
    /// ```
    fn reverse(self) -> ReversedIso<Self>
    where
        Self: Sized,
    {
        ReversedIso::new(self)
    }

    /// Applies a function to the converted value and converts back.
    ///
    /// This is equivalent to: `iso.reverse_get(function(iso.get(source)))`
    ///
    /// # Arguments
    ///
    /// * `source` - The source value
    /// * `function` - The function to apply to the converted value
    ///
    /// # Returns
    ///
    /// A new source value with the modification applied
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Iso, FunctionIso};
    ///
    /// let string_chars_iso = FunctionIso::new(
    ///     |s: String| s.chars().collect::<Vec<_>>(),
    ///     |chars: Vec<char>| chars.into_iter().collect::<String>(),
    /// );
    ///
    /// let original = "hello".to_string();
    /// let reversed_string = string_chars_iso.modify(original, |mut chars| {
    ///     chars.reverse();
    ///     chars
    /// });
    /// assert_eq!(reversed_string, "olleh");
    /// ```
    fn modify<F>(&self, source: S, function: F) -> S
    where
        F: FnOnce(A) -> A,
    {
        let converted = self.get(source);
        self.reverse_get(function(converted))
    }

    /// Composes this Iso with another Iso to create a combined Iso.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the other Iso
    /// - `I`: The type of the other Iso
    ///
    /// # Arguments
    ///
    /// * `other` - The Iso to compose with
    ///
    /// # Returns
    ///
    /// A composed Iso that converts from S to B
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Iso, FunctionIso};
    ///
    /// let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);
    /// let iso2 = FunctionIso::new(|x: i64| x.to_string(), |s: String| s.parse::<i64>().unwrap());
    ///
    /// let composed = iso1.compose(iso2);
    ///
    /// let result = composed.get(42);
    /// assert_eq!(result, "42");
    /// ```
    fn compose<B, I>(self, other: I) -> ComposedIso<Self, I, A>
    where
        Self: Sized,
        I: Iso<A, B>,
    {
        ComposedIso::new(self, other)
    }

    /// Converts this Iso to a Lens.
    ///
    /// Since an Iso is stronger than a Lens, this conversion is always possible.
    /// The resulting Lens will use a cached value internally for `get` to return a reference.
    ///
    /// # Returns
    ///
    /// A Lens that behaves like this Iso
    fn to_lens(self) -> IsoAsLens<Self, S, A>
    where
        Self: Sized,
    {
        IsoAsLens::new(self)
    }

    /// Converts this Iso to a Prism.
    ///
    /// Since an Iso is stronger than a Prism, this conversion is always possible.
    /// The resulting Prism will always succeed in `preview`.
    ///
    /// # Returns
    ///
    /// A Prism that behaves like this Iso
    fn to_prism(self) -> IsoAsPrism<Self, S, A>
    where
        Self: Sized,
    {
        IsoAsPrism::new(self)
    }
}

/// An Iso implemented using get and `reverse_get` functions.
///
/// This is the most common way to create an Iso. The `iso!` macro
/// generates a `FunctionIso` internally.
///
/// # Type Parameters
///
/// - `S`: The source type
/// - `A`: The target type
/// - `G`: The get function type
/// - `Rg`: The `reverse_get` function type
///
/// # Example
///
/// ```
/// use lambars::optics::{Iso, FunctionIso};
///
/// // String <-> Vec<char> conversion
/// let string_chars_iso = FunctionIso::new(
///     |s: String| s.chars().collect::<Vec<_>>(),
///     |chars: Vec<char>| chars.into_iter().collect::<String>(),
/// );
///
/// let chars = string_chars_iso.get("hello".to_string());
/// assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
/// ```
pub struct FunctionIso<S, A, G, Rg>
where
    G: Fn(S) -> A,
    Rg: Fn(A) -> S,
{
    get_function: G,
    reverse_get_function: Rg,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, G, Rg> FunctionIso<S, A, G, Rg>
where
    G: Fn(S) -> A,
    Rg: Fn(A) -> S,
{
    /// Creates a new `FunctionIso` from get and `reverse_get` functions.
    ///
    /// # Arguments
    ///
    /// * `get_function` - A function that converts from S to A
    /// * `reverse_get_function` - A function that converts from A to S
    ///
    /// # Returns
    ///
    /// A new `FunctionIso`
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Iso, FunctionIso};
    ///
    /// let string_chars_iso = FunctionIso::new(
    ///     |s: String| s.chars().collect::<Vec<_>>(),
    ///     |chars: Vec<char>| chars.into_iter().collect::<String>(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(get_function: G, reverse_get_function: Rg) -> Self {
        Self {
            get_function,
            reverse_get_function,
            _marker: PhantomData,
        }
    }
}

impl<S, A, G, Rg> Iso<S, A> for FunctionIso<S, A, G, Rg>
where
    G: Fn(S) -> A,
    Rg: Fn(A) -> S,
{
    fn get(&self, source: S) -> A {
        (self.get_function)(source)
    }

    fn reverse_get(&self, value: A) -> S {
        (self.reverse_get_function)(value)
    }
}

impl<S, A, G, Rg> Clone for FunctionIso<S, A, G, Rg>
where
    G: Fn(S) -> A + Clone,
    Rg: Fn(A) -> S + Clone,
{
    fn clone(&self) -> Self {
        Self {
            get_function: self.get_function.clone(),
            reverse_get_function: self.reverse_get_function.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, G, Rg> std::fmt::Debug for FunctionIso<S, A, G, Rg>
where
    G: Fn(S) -> A,
    Rg: Fn(A) -> S,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FunctionIso")
            .finish_non_exhaustive()
    }
}

/// A reversed Iso that swaps the direction of conversion.
///
/// # Type Parameters
///
/// - `I`: The type of the underlying Iso
///
/// # Example
///
/// ```
/// use lambars::optics::{Iso, FunctionIso};
///
/// let string_chars_iso = FunctionIso::new(
///     |s: String| s.chars().collect::<Vec<_>>(),
///     |chars: Vec<char>| chars.into_iter().collect::<String>(),
/// );
///
/// let chars_string_iso = string_chars_iso.reverse();
///
/// let chars = vec!['h', 'i'];
/// let string = chars_string_iso.get(chars);
/// assert_eq!(string, "hi");
/// ```
pub struct ReversedIso<I> {
    inner: I,
}

impl<I> ReversedIso<I> {
    /// Creates a new `ReversedIso` from an Iso.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Iso to reverse
    ///
    /// # Returns
    ///
    /// A new `ReversedIso`
    #[must_use]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }
}

impl<S, A, I> Iso<A, S> for ReversedIso<I>
where
    I: Iso<S, A>,
{
    fn get(&self, source: A) -> S {
        self.inner.reverse_get(source)
    }

    fn reverse_get(&self, value: S) -> A {
        self.inner.get(value)
    }
}

impl<I: Clone> Clone for ReversedIso<I> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<I: std::fmt::Debug> std::fmt::Debug for ReversedIso<I> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReversedIso")
            .field("inner", &self.inner)
            .finish()
    }
}

/// A composed Iso that chains two Isos together.
///
/// # Type Parameters
///
/// - `I1`: The type of the first Iso
/// - `I2`: The type of the second Iso
/// - `A`: The intermediate type (target of I1, source of I2)
///
/// # Example
///
/// ```
/// use lambars::optics::{Iso, FunctionIso};
///
/// let iso1 = FunctionIso::new(|x: i32| x as i64, |x: i64| x as i32);
/// let iso2 = FunctionIso::new(|x: i64| x.to_string(), |s: String| s.parse::<i64>().unwrap());
///
/// let composed = iso1.compose(iso2);
///
/// let result = composed.get(42);
/// assert_eq!(result, "42");
///
/// let back = composed.reverse_get("42".to_string());
/// assert_eq!(back, 42);
/// ```
pub struct ComposedIso<I1, I2, A> {
    first: I1,
    second: I2,
    _marker: PhantomData<A>,
}

impl<I1, I2, A> ComposedIso<I1, I2, A> {
    /// Creates a new `ComposedIso` from two Isos.
    ///
    /// # Arguments
    ///
    /// * `first` - The first Iso (converts from S to A)
    /// * `second` - The second Iso (converts from A to B)
    ///
    /// # Returns
    ///
    /// A new `ComposedIso`
    #[must_use]
    pub const fn new(first: I1, second: I2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, I1, I2> Iso<S, B> for ComposedIso<I1, I2, A>
where
    I1: Iso<S, A>,
    I2: Iso<A, B>,
{
    fn get(&self, source: S) -> B {
        let intermediate = self.first.get(source);
        self.second.get(intermediate)
    }

    fn reverse_get(&self, value: B) -> S {
        let intermediate = self.second.reverse_get(value);
        self.first.reverse_get(intermediate)
    }
}

impl<I1: Clone, I2: Clone, A> Clone for ComposedIso<I1, I2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I1: std::fmt::Debug, I2: std::fmt::Debug, A> std::fmt::Debug for ComposedIso<I1, I2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedIso")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// An Iso used as a Lens.
///
/// This wrapper allows using an Iso where a Lens is expected.
/// It caches the converted value internally to provide a reference.
///
/// # Type Parameters
///
/// - `I`: The type of the underlying Iso
/// - `S`: The source type
/// - `A`: The target type
pub struct IsoAsLens<I, S, A> {
    iso: I,
    _marker: PhantomData<(S, A)>,
}

impl<I, S, A> IsoAsLens<I, S, A> {
    /// Creates a new `IsoAsLens` from an Iso.
    ///
    /// # Arguments
    ///
    /// * `iso` - The Iso to wrap
    ///
    /// # Returns
    ///
    /// A new `IsoAsLens`
    #[must_use]
    pub const fn new(iso: I) -> Self {
        Self {
            iso,
            _marker: PhantomData,
        }
    }
}

impl<I, S, A> IsoAsLens<I, S, A>
where
    I: Iso<S, A>,
    S: Clone,
    A: 'static,
{
    /// Gets the value and stores it for returning a reference.
    ///
    /// Note: This implementation uses a leaked Box to provide a reference,
    /// which is not ideal for long-running applications. For production use,
    /// consider using the Iso directly.
    fn get_internal(&self, source: &S) -> A {
        self.iso.get(source.clone())
    }
}

impl<I, S, A> Lens<S, A> for IsoAsLens<I, S, A>
where
    I: Iso<S, A>,
    S: Clone,
    A: Clone + 'static,
{
    fn get<'a>(&self, source: &'a S) -> &'a A {
        // This is a workaround since Iso::get takes ownership.
        // We leak a Box to provide a 'static reference that satisfies the lifetime.
        // This is acceptable for testing but should be used with caution in production.
        let value = self.get_internal(source);
        Box::leak(Box::new(value))
    }

    fn set(&self, _source: S, value: A) -> S {
        self.iso.reverse_get(value)
    }
}

impl<I: Clone, S, A> Clone for IsoAsLens<I, S, A> {
    fn clone(&self) -> Self {
        Self {
            iso: self.iso.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: std::fmt::Debug, S, A> std::fmt::Debug for IsoAsLens<I, S, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IsoAsLens")
            .field("iso", &self.iso)
            .finish()
    }
}

/// An Iso used as a Prism.
///
/// This wrapper allows using an Iso where a Prism is expected.
/// The preview operation will always succeed since an Iso is a total function.
///
/// # Type Parameters
///
/// - `I`: The type of the underlying Iso
/// - `S`: The source type
/// - `A`: The target type
pub struct IsoAsPrism<I, S, A> {
    iso: I,
    _marker: PhantomData<(S, A)>,
}

impl<I, S, A> IsoAsPrism<I, S, A> {
    /// Creates a new `IsoAsPrism` from an Iso.
    ///
    /// # Arguments
    ///
    /// * `iso` - The Iso to wrap
    ///
    /// # Returns
    ///
    /// A new `IsoAsPrism`
    #[must_use]
    pub const fn new(iso: I) -> Self {
        Self {
            iso,
            _marker: PhantomData,
        }
    }
}

impl<I, S, A> IsoAsPrism<I, S, A>
where
    I: Iso<S, A>,
    S: Clone,
{
    /// Gets the value and stores it for returning a reference.
    fn get_internal(&self, source: &S) -> A {
        self.iso.get(source.clone())
    }
}

impl<I, S, A> Prism<S, A> for IsoAsPrism<I, S, A>
where
    I: Iso<S, A>,
    S: Clone,
    A: Clone + 'static,
{
    fn preview<'a>(&self, source: &'a S) -> Option<&'a A> {
        // Similar workaround as IsoAsLens
        let value = self.get_internal(source);
        Some(Box::leak(Box::new(value)))
    }

    fn review(&self, value: A) -> S {
        self.iso.reverse_get(value)
    }

    fn preview_owned(&self, source: S) -> Option<A> {
        Some(self.iso.get(source))
    }
}

impl<I: Clone, S, A> Clone for IsoAsPrism<I, S, A> {
    fn clone(&self) -> Self {
        Self {
            iso: self.iso.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: std::fmt::Debug, S, A> std::fmt::Debug for IsoAsPrism<I, S, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IsoAsPrism")
            .field("iso", &self.iso)
            .finish()
    }
}

/// Creates an Iso from get and `reverse_get` functions.
///
/// # Syntax
///
/// ```text
/// iso!(get_function, reverse_get_function)
/// ```
///
/// # Example
///
/// ```
/// use lambars::optics::Iso;
/// use lambars::iso;
///
/// let swap = iso!(
///     |(a, b): (i32, String)| (b, a),
///     |(b, a): (String, i32)| (a, b)
/// );
///
/// let tuple = (42, "hello".to_string());
/// let swapped = swap.get(tuple.clone());
/// assert_eq!(swapped, ("hello".to_string(), 42));
///
/// let back = swap.reverse_get(swapped);
/// assert_eq!(back, tuple);
/// ```
#[macro_export]
macro_rules! iso {
    ($get:expr, $reverse_get:expr) => {
        $crate::optics::FunctionIso::new($get, $reverse_get)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_iso_get() {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>(),
        );

        let original = "hello".to_string();
        let chars = string_chars_iso.get(original);
        assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_function_iso_reverse_get() {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>(),
        );

        let chars = vec!['h', 'i'];
        let string = string_chars_iso.reverse_get(chars);
        assert_eq!(string, "hi");
    }

    #[test]
    fn test_reversed_iso() {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>(),
        );

        let chars_string_iso = string_chars_iso.reverse();

        let chars = vec!['h', 'i'];
        let string = chars_string_iso.get(chars);
        assert_eq!(string, "hi");
    }

    #[test]
    fn test_iso_compose() {
        #[allow(clippy::cast_possible_truncation)]
        let iso1 = FunctionIso::new(|x: i32| i64::from(x), |x: i64| x as i32);

        let iso2 = FunctionIso::new(
            |x: i64| x.to_string(),
            |s: String| s.parse::<i64>().unwrap(),
        );

        let composed = iso1.compose(iso2);

        let result = composed.get(42);
        assert_eq!(result, "42");

        let back = composed.reverse_get("42".to_string());
        assert_eq!(back, 42);
    }

    #[test]
    fn test_iso_modify() {
        let string_chars_iso = FunctionIso::new(
            |s: String| s.chars().collect::<Vec<_>>(),
            |chars: Vec<char>| chars.into_iter().collect::<String>(),
        );

        let original = "hello".to_string();
        let result = string_chars_iso.modify(original, |mut chars| {
            chars.reverse();
            chars
        });

        assert_eq!(result, "olleh");
    }

    #[test]
    fn test_iso_macro() {
        let swap = iso!(|(a, b): (i32, String)| (b, a), |(b, a): (String, i32)| (
            a, b
        ));

        let tuple = (42, "hello".to_string());
        let swapped = swap.get(tuple);
        assert_eq!(swapped, ("hello".to_string(), 42));
    }
}
