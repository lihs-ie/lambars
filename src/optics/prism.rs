//! Prism optics for focusing on enum variants.
//!
//! A Prism is an optic that provides preview/review access to a variant of an enum.
//! Unlike a Lens which always succeeds, a Prism may fail to extract a value
//! if the enum is not the expected variant.
//!
//! # Laws
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
//! # Examples
//!
//! ```
//! use functional_rusty::optics::{Prism, FunctionPrism};
//! use functional_rusty::prism;
//!
//! #[derive(Clone, PartialEq, Debug)]
//! enum Shape {
//!     Circle(f64),
//!     Rectangle(f64, f64),
//! }
//!
//! // Using prism! macro
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

use std::marker::PhantomData;

/// A Prism focuses on a single variant of an enum.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole enum)
/// - `A`: The target type (the value inside the variant)
///
/// # Laws
///
/// 1. **PreviewReview Law**: `prism.preview(&prism.review(value)) == Some(&value)`
/// 2. **ReviewPreview Law**: If preview succeeds, `prism.review(prism.preview(&source).unwrap().clone()) == source`
pub trait Prism<S, A> {
    /// Attempts to extract the value from the source.
    ///
    /// Returns `Some` if the source is the expected variant, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `source` - The source enum
    ///
    /// # Returns
    ///
    /// A reference to the inner value if the variant matches, `None` otherwise
    fn preview<'a>(&self, source: &'a S) -> Option<&'a A>;

    /// Constructs the source from a value.
    ///
    /// This always succeeds, creating the expected variant from the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap in the variant
    ///
    /// # Returns
    ///
    /// A new source with the value wrapped in the expected variant
    fn review(&self, value: A) -> S;

    /// Extracts the value from the source, taking ownership.
    ///
    /// Returns `Some` if the source is the expected variant, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `source` - The source enum (consumed)
    ///
    /// # Returns
    ///
    /// The inner value if the variant matches, `None` otherwise
    fn preview_owned(&self, source: S) -> Option<A>;

    /// Modifies the value if the source is the expected variant.
    ///
    /// Returns `Some` with the modified source if the variant matches,
    /// `None` if the variant doesn't match.
    ///
    /// # Arguments
    ///
    /// * `source` - The source enum (consumed)
    /// * `function` - The function to apply to the inner value
    ///
    /// # Returns
    ///
    /// `Some(modified_source)` if the variant matches, `None` otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use functional_rusty::optics::Prism;
    /// use functional_rusty::prism;
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum Shape {
    ///     Circle(f64),
    ///     Rectangle(f64, f64),
    /// }
    ///
    /// let circle_prism = prism!(Shape, Circle);
    ///
    /// let circle = Shape::Circle(5.0);
    /// let doubled = circle_prism.modify_option(circle, |r| r * 2.0);
    /// assert!(matches!(doubled, Some(Shape::Circle(r)) if (r - 10.0).abs() < 1e-10));
    ///
    /// let rect = Shape::Rectangle(3.0, 4.0);
    /// let result = circle_prism.modify_option(rect, |r| r * 2.0);
    /// assert!(result.is_none());
    /// ```
    fn modify_option<F>(&self, source: S, function: F) -> Option<S>
    where
        F: FnOnce(A) -> A,
    {
        self.preview_owned(source)
            .map(|value| self.review(function(value)))
    }

    /// Modifies the value if the source is the expected variant, or returns the original.
    ///
    /// If the source is the expected variant, applies the function and returns
    /// the modified source. Otherwise, returns the source unchanged.
    ///
    /// # Arguments
    ///
    /// * `source` - The source enum (consumed)
    /// * `function` - The function to apply to the inner value
    ///
    /// # Returns
    ///
    /// The modified source if the variant matches, the original source otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use functional_rusty::optics::Prism;
    /// use functional_rusty::prism;
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum Shape {
    ///     Circle(f64),
    ///     Rectangle(f64, f64),
    /// }
    ///
    /// let circle_prism = prism!(Shape, Circle);
    ///
    /// let circle = Shape::Circle(5.0);
    /// let doubled = circle_prism.modify_or_identity(circle, |r| r * 2.0);
    /// assert!(matches!(doubled, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
    ///
    /// let rect = Shape::Rectangle(3.0, 4.0);
    /// let unchanged = circle_prism.modify_or_identity(rect.clone(), |r| r * 2.0);
    /// assert_eq!(unchanged, rect);
    /// ```
    fn modify_or_identity<F>(&self, source: S, function: F) -> S
    where
        F: FnOnce(A) -> A,
        S: Clone,
    {
        self.modify_option(source.clone(), function)
            .unwrap_or(source)
    }

    /// Composes this prism with another prism to focus on a nested variant.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the other prism
    /// - `P`: The type of the other prism
    ///
    /// # Arguments
    ///
    /// * `other` - The prism to compose with
    ///
    /// # Returns
    ///
    /// A composed prism that focuses on the nested variant
    ///
    /// # Example
    ///
    /// ```
    /// use functional_rusty::optics::{Prism, FunctionPrism};
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum Outer { Inner(Inner), Empty }
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum Inner { Value(i32), Nothing }
    ///
    /// let outer_inner = FunctionPrism::new(
    ///     |outer: &Outer| match outer {
    ///         Outer::Inner(inner) => Some(inner),
    ///         _ => None,
    ///     },
    ///     |inner: Inner| Outer::Inner(inner),
    ///     |outer: Outer| match outer {
    ///         Outer::Inner(inner) => Some(inner),
    ///         _ => None,
    ///     },
    /// );
    ///
    /// let inner_value = FunctionPrism::new(
    ///     |inner: &Inner| match inner {
    ///         Inner::Value(v) => Some(v),
    ///         _ => None,
    ///     },
    ///     |v: i32| Inner::Value(v),
    ///     |inner: Inner| match inner {
    ///         Inner::Value(v) => Some(v),
    ///         _ => None,
    ///     },
    /// );
    ///
    /// let outer_value = outer_inner.compose(inner_value);
    ///
    /// let data = Outer::Inner(Inner::Value(42));
    /// assert_eq!(outer_value.preview(&data), Some(&42));
    /// ```
    fn compose<B, P>(self, other: P) -> ComposedPrism<Self, P, A>
    where
        Self: Sized,
        P: Prism<A, B>,
    {
        ComposedPrism::new(self, other)
    }

    /// Converts this prism to a traversal.
    ///
    /// A prism can be viewed as a traversal that yields zero or one elements.
    ///
    /// # Returns
    ///
    /// A traversal that yields the focused element if present
    fn to_traversal(self) -> PrismAsTraversal<Self, S, A>
    where
        Self: Sized,
    {
        PrismAsTraversal::new(self)
    }
}

/// A prism implemented using preview, review, and preview_owned functions.
///
/// This is the most common way to create a prism. The `prism!` macro
/// generates a `FunctionPrism` internally.
///
/// # Type Parameters
///
/// - `S`: The source type
/// - `A`: The target type
/// - `Pr`: The preview function type
/// - `Re`: The review function type
/// - `PrOwned`: The preview_owned function type
///
/// # Example
///
/// ```
/// use functional_rusty::optics::{Prism, FunctionPrism};
///
/// #[derive(Clone, PartialEq, Debug)]
/// enum Shape {
///     Circle(f64),
///     Rectangle(f64, f64),
/// }
///
/// let circle_prism = FunctionPrism::new(
///     |shape: &Shape| match shape {
///         Shape::Circle(radius) => Some(radius),
///         _ => None,
///     },
///     |radius: f64| Shape::Circle(radius),
///     |shape: Shape| match shape {
///         Shape::Circle(radius) => Some(radius),
///         _ => None,
///     },
/// );
///
/// let circle = Shape::Circle(5.0);
/// assert_eq!(circle_prism.preview(&circle), Some(&5.0));
/// ```
pub struct FunctionPrism<S, A, Pr, Re, PrOwned>
where
    Pr: Fn(&S) -> Option<&A>,
    Re: Fn(A) -> S,
    PrOwned: Fn(S) -> Option<A>,
{
    preview_function: Pr,
    review_function: Re,
    preview_owned_function: PrOwned,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, Pr, Re, PrOwned> FunctionPrism<S, A, Pr, Re, PrOwned>
where
    Pr: Fn(&S) -> Option<&A>,
    Re: Fn(A) -> S,
    PrOwned: Fn(S) -> Option<A>,
{
    /// Creates a new `FunctionPrism` from preview, review, and preview_owned functions.
    ///
    /// # Arguments
    ///
    /// * `preview_function` - A function that attempts to extract a reference from the source
    /// * `review_function` - A function that constructs the source from a value
    /// * `preview_owned_function` - A function that attempts to extract an owned value from the source
    ///
    /// # Returns
    ///
    /// A new `FunctionPrism`
    #[must_use]
    pub const fn new(
        preview_function: Pr,
        review_function: Re,
        preview_owned_function: PrOwned,
    ) -> Self {
        Self {
            preview_function,
            review_function,
            preview_owned_function,
            _marker: PhantomData,
        }
    }
}

impl<S, A, Pr, Re, PrOwned> Prism<S, A> for FunctionPrism<S, A, Pr, Re, PrOwned>
where
    Pr: Fn(&S) -> Option<&A>,
    Re: Fn(A) -> S,
    PrOwned: Fn(S) -> Option<A>,
{
    fn preview<'a>(&self, source: &'a S) -> Option<&'a A> {
        (self.preview_function)(source)
    }

    fn review(&self, value: A) -> S {
        (self.review_function)(value)
    }

    fn preview_owned(&self, source: S) -> Option<A> {
        (self.preview_owned_function)(source)
    }
}

impl<S, A, Pr, Re, PrOwned> Clone for FunctionPrism<S, A, Pr, Re, PrOwned>
where
    Pr: Fn(&S) -> Option<&A> + Clone,
    Re: Fn(A) -> S + Clone,
    PrOwned: Fn(S) -> Option<A> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            preview_function: self.preview_function.clone(),
            review_function: self.review_function.clone(),
            preview_owned_function: self.preview_owned_function.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, Pr, Re, PrOwned> std::fmt::Debug for FunctionPrism<S, A, Pr, Re, PrOwned>
where
    Pr: Fn(&S) -> Option<&A>,
    Re: Fn(A) -> S,
    PrOwned: Fn(S) -> Option<A>,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FunctionPrism")
            .finish_non_exhaustive()
    }
}

/// A prism composed of two prisms.
///
/// This allows focusing on nested variants by composing a prism that focuses on
/// an intermediate enum with a prism that focuses on a variant within that enum.
///
/// # Type Parameters
///
/// - `P1`: The type of the outer prism
/// - `P2`: The type of the inner prism
/// - `A`: The intermediate type (target of P1, source of P2)
///
/// # Example
///
/// ```
/// use functional_rusty::optics::{Prism, FunctionPrism};
///
/// #[derive(Clone, PartialEq, Debug)]
/// enum Outer { Inner(Inner), Empty }
///
/// #[derive(Clone, PartialEq, Debug)]
/// enum Inner { Value(i32), Nothing }
///
/// let outer_inner = FunctionPrism::new(
///     |outer: &Outer| match outer {
///         Outer::Inner(inner) => Some(inner),
///         _ => None,
///     },
///     |inner: Inner| Outer::Inner(inner),
///     |outer: Outer| match outer {
///         Outer::Inner(inner) => Some(inner),
///         _ => None,
///     },
/// );
///
/// let inner_value = FunctionPrism::new(
///     |inner: &Inner| match inner {
///         Inner::Value(v) => Some(v),
///         _ => None,
///     },
///     |v: i32| Inner::Value(v),
///     |inner: Inner| match inner {
///         Inner::Value(v) => Some(v),
///         _ => None,
///     },
/// );
///
/// let outer_value = outer_inner.compose(inner_value);
///
/// let data = Outer::Inner(Inner::Value(42));
/// assert_eq!(outer_value.preview(&data), Some(&42));
/// ```
pub struct ComposedPrism<P1, P2, A> {
    first: P1,
    second: P2,
    _marker: PhantomData<A>,
}

impl<P1, P2, A> ComposedPrism<P1, P2, A> {
    /// Creates a new composed prism.
    ///
    /// # Arguments
    ///
    /// * `first` - The outer prism (focuses on the intermediate enum)
    /// * `second` - The inner prism (focuses on the final value)
    ///
    /// # Returns
    ///
    /// A new `ComposedPrism`
    #[must_use]
    pub const fn new(first: P1, second: P2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, P1, P2> Prism<S, B> for ComposedPrism<P1, P2, A>
where
    P1: Prism<S, A>,
    P2: Prism<A, B>,
    A: Clone + 'static,
{
    fn preview<'a>(&self, source: &'a S) -> Option<&'a B> {
        self.first
            .preview(source)
            .and_then(|intermediate| self.second.preview(intermediate))
    }

    fn review(&self, value: B) -> S {
        let intermediate = self.second.review(value);
        self.first.review(intermediate)
    }

    fn preview_owned(&self, source: S) -> Option<B> {
        self.first
            .preview_owned(source)
            .and_then(|intermediate| self.second.preview_owned(intermediate))
    }
}

impl<P1: Clone, P2: Clone, A> Clone for ComposedPrism<P1, P2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<P1: std::fmt::Debug, P2: std::fmt::Debug, A> std::fmt::Debug for ComposedPrism<P1, P2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedPrism")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// A prism converted to a traversal.
///
/// This wrapper allows using a prism where a traversal is expected.
/// It will yield zero or one elements.
///
/// # Type Parameters
///
/// - `P`: The type of the underlying prism
/// - `S`: The source type
/// - `A`: The target type
pub struct PrismAsTraversal<P, S, A> {
    pub(crate) prism: P,
    _marker: PhantomData<(S, A)>,
}

impl<P, S, A> PrismAsTraversal<P, S, A> {
    /// Creates a new `PrismAsTraversal` from a prism.
    ///
    /// # Arguments
    ///
    /// * `prism` - The prism to wrap
    ///
    /// # Returns
    ///
    /// A new `PrismAsTraversal`
    #[must_use]
    pub const fn new(prism: P) -> Self {
        Self {
            prism,
            _marker: PhantomData,
        }
    }
}

impl<P, S, A> PrismAsTraversal<P, S, A>
where
    P: Prism<S, A>,
    A: 'static,
{
    /// Returns an iterator over the focused element(s).
    ///
    /// For a prism, this yields zero or one elements.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// An iterator yielding the focused element if present
    pub fn get_all<'a>(&self, source: &'a S) -> impl Iterator<Item = &'a A>
    where
        A: 'a,
    {
        self.prism.preview(source).into_iter()
    }

    /// Modifies all focused elements by applying a function.
    ///
    /// For a prism, this modifies zero or one elements.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply
    ///
    /// # Returns
    ///
    /// A new source with the focused element(s) modified (if present),
    /// or the original source unchanged
    pub fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(A) -> A,
        S: Clone,
    {
        let mut function = function;
        self.prism
            .modify_or_identity(source, |value| function(value))
    }
}

impl<P: Clone, S, A> Clone for PrismAsTraversal<P, S, A> {
    fn clone(&self) -> Self {
        Self {
            prism: self.prism.clone(),
            _marker: PhantomData,
        }
    }
}

impl<P: std::fmt::Debug, S, A> std::fmt::Debug for PrismAsTraversal<P, S, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrismAsTraversal")
            .field("prism", &self.prism)
            .finish()
    }
}

/// Creates a prism for an enum variant.
///
/// This macro generates a `FunctionPrism` that focuses on the specified variant
/// of the given enum type.
///
/// # Syntax
///
/// ```text
/// prism!(EnumType, VariantName)
/// prism!(EnumType<T, ...>, VariantName)
/// ```
///
/// # Limitations
///
/// This macro only works with tuple variants that have a single value.
/// For variants with multiple fields or named fields, use `FunctionPrism::new` directly.
///
/// # Example
///
/// ```
/// use functional_rusty::optics::Prism;
/// use functional_rusty::prism;
///
/// #[derive(Clone, PartialEq, Debug)]
/// enum MyOption<T> {
///     Some(T),
///     None,
/// }
///
/// let some_prism = prism!(MyOption<i32>, Some);
///
/// let some_value = MyOption::Some(42);
/// assert_eq!(some_prism.preview(&some_value), Some(&42));
///
/// let none_value: MyOption<i32> = MyOption::None;
/// assert_eq!(some_prism.preview(&none_value), None);
///
/// let constructed = some_prism.review(100);
/// assert_eq!(constructed, MyOption::Some(100));
/// ```
#[macro_export]
macro_rules! prism {
    ($enum_type:ident, $variant:ident) => {
        $crate::optics::FunctionPrism::new(
            |source: &$enum_type| match *source {
                $enum_type::$variant(ref value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            |value| $enum_type::$variant(value),
            |source: $enum_type| match source {
                $enum_type::$variant(value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
        )
    };
    ($enum_type:ident < $($generic:tt),+ >, $variant:ident) => {
        $crate::optics::FunctionPrism::new(
            |source: &$enum_type<$($generic),+>| match *source {
                $enum_type::$variant(ref value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            |value| $enum_type::$variant(value),
            |source: $enum_type<$($generic),+>| match source {
                $enum_type::$variant(value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
        )
    };
    ($enum_type:path, $variant:ident) => {
        $crate::optics::FunctionPrism::new(
            |source: &$enum_type| match *source {
                <$enum_type>::$variant(ref value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            |value| <$enum_type>::$variant(value),
            |source: $enum_type| match source {
                <$enum_type>::$variant(value) => Some(value),
                #[allow(unreachable_patterns)]
                _ => None,
            },
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    enum Shape {
        Circle(f64),
        Rectangle(f64, f64),
    }

    #[test]
    fn test_function_prism_preview_match() {
        let circle_prism = FunctionPrism::new(
            |shape: &Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
            |radius: f64| Shape::Circle(radius),
            |shape: Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
        );

        let circle = Shape::Circle(5.0);
        assert_eq!(circle_prism.preview(&circle), Some(&5.0));
    }

    #[test]
    fn test_function_prism_preview_no_match() {
        let circle_prism = FunctionPrism::new(
            |shape: &Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
            |radius: f64| Shape::Circle(radius),
            |shape: Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
        );

        let rect = Shape::Rectangle(3.0, 4.0);
        assert_eq!(circle_prism.preview(&rect), None);
    }

    #[test]
    fn test_function_prism_review() {
        let circle_prism = FunctionPrism::new(
            |shape: &Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
            |radius: f64| Shape::Circle(radius),
            |shape: Shape| match shape {
                Shape::Circle(radius) => Some(radius),
                _ => None,
            },
        );

        let constructed = circle_prism.review(10.0);
        assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
    }

    #[test]
    fn test_prism_macro() {
        let circle_prism = prism!(Shape, Circle);
        let circle = Shape::Circle(5.0);
        assert_eq!(circle_prism.preview(&circle), Some(&5.0));
    }

    #[test]
    fn test_prism_modify_option() {
        let circle_prism = prism!(Shape, Circle);
        let circle = Shape::Circle(5.0);
        let doubled = circle_prism.modify_option(circle, |r| r * 2.0);
        assert!(matches!(doubled, Some(Shape::Circle(r)) if (r - 10.0).abs() < 1e-10));
    }

    #[test]
    fn test_prism_to_traversal() {
        let circle_prism = prism!(Shape, Circle);
        let traversal = circle_prism.to_traversal();

        let circle = Shape::Circle(5.0);
        let all: Vec<&f64> = traversal.get_all(&circle).collect();
        assert_eq!(all, vec![&5.0]);

        let rect = Shape::Rectangle(3.0, 4.0);
        let all: Vec<&f64> = traversal.get_all(&rect).collect();
        assert!(all.is_empty());
    }
}
