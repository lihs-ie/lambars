//! Lens optics for focusing on struct fields.
//!
//! A Lens is an optic that provides get/set access to a field within a larger structure.
//! Lenses are composable, allowing access to deeply nested fields.
//!
//! # Laws
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
//! # Examples
//!
//! ```
//! use lambars::optics::{Lens, FunctionLens};
//! use lambars::lens;
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Point { x: i32, y: i32 }
//!
//! // Using lens! macro
//! let x_lens = lens!(Point, x);
//!
//! let point = Point { x: 10, y: 20 };
//! assert_eq!(*x_lens.get(&point), 10);
//!
//! let updated = x_lens.set(point, 100);
//! assert_eq!(updated.x, 100);
//! ```

use std::marker::PhantomData;

/// A Lens focuses on a single field within a larger structure.
///
/// # Type Parameters
///
/// - `S`: The source type (the whole structure)
/// - `A`: The target type (the focused field)
///
/// # Laws
///
/// 1. **GetPut Law**: `lens.set(source, lens.get(&source).clone()) == source`
/// 2. **PutGet Law**: `lens.get(&lens.set(source, value)) == &value`
/// 3. **PutPut Law**: `lens.set(lens.set(source, v1), v2) == lens.set(source, v2)`
pub trait Lens<S, A> {
    /// Gets a reference to the focused field.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// A reference to the focused field
    fn get<'a>(&self, source: &'a S) -> &'a A;

    /// Sets the focused field to a new value, returning a new source.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `value` - The new value for the focused field
    ///
    /// # Returns
    ///
    /// A new source with the focused field updated
    fn set(&self, source: S, value: A) -> S;

    /// Modifies the focused field by applying a function.
    ///
    /// This is equivalent to getting the current value, applying the function,
    /// and setting the result.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply to the focused field
    ///
    /// # Returns
    ///
    /// A new source with the focused field modified
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::Lens;
    /// use lambars::lens;
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Point { x: i32, y: i32 }
    ///
    /// let x_lens = lens!(Point, x);
    /// let point = Point { x: 10, y: 20 };
    /// let doubled = x_lens.modify(point, |x| x * 2);
    /// assert_eq!(doubled.x, 20);
    /// ```
    fn modify<F>(&self, source: S, function: F) -> S
    where
        F: FnOnce(A) -> A,
        A: Clone,
    {
        let current = self.get(&source).clone();
        self.set(source, function(current))
    }

    /// Modifies the focused field by applying a function to a reference.
    ///
    /// This is useful when the transformation function only needs a reference
    /// to compute the new value.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply to a reference of the focused field
    ///
    /// # Returns
    ///
    /// A new source with the focused field modified
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::Lens;
    /// use lambars::lens;
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Person { name: String, age: u32 }
    ///
    /// let name_lens = lens!(Person, name);
    /// let person = Person { name: "alice".to_string(), age: 30 };
    /// let upper = name_lens.modify_ref(person, |name| name.to_uppercase());
    /// assert_eq!(upper.name, "ALICE");
    /// ```
    fn modify_ref<F>(&self, source: S, function: F) -> S
    where
        F: FnOnce(&A) -> A,
    {
        let new_value = function(self.get(&source));
        self.set(source, new_value)
    }

    /// Composes this lens with another lens to focus on a nested field.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The target type of the other lens
    /// - `L`: The type of the other lens
    ///
    /// # Arguments
    ///
    /// * `other` - The lens to compose with
    ///
    /// # Returns
    ///
    /// A composed lens that focuses on the nested field
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::Lens;
    /// use lambars::lens;
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Address { street: String, city: String }
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Person { name: String, address: Address }
    ///
    /// let address_lens = lens!(Person, address);
    /// let street_lens = lens!(Address, street);
    /// let person_street = address_lens.compose(street_lens);
    ///
    /// let person = Person {
    ///     name: "Alice".to_string(),
    ///     address: Address {
    ///         street: "Main St".to_string(),
    ///         city: "Tokyo".to_string(),
    ///     },
    /// };
    ///
    /// assert_eq!(*person_street.get(&person), "Main St");
    /// ```
    fn compose<B, L>(self, other: L) -> ComposedLens<Self, L, A>
    where
        Self: Sized,
        L: Lens<A, B>,
    {
        ComposedLens::new(self, other)
    }

    /// Converts this lens to a traversal.
    ///
    /// A lens can always be viewed as a traversal that focuses on exactly one element.
    ///
    /// # Returns
    ///
    /// A traversal that yields exactly one element
    fn to_traversal(self) -> LensAsTraversal<Self, S, A>
    where
        Self: Sized,
    {
        LensAsTraversal::new(self)
    }
}

/// A lens implemented using getter and setter functions.
///
/// This is the most common way to create a lens. The `lens!` macro
/// generates a `FunctionLens` internally.
///
/// # Type Parameters
///
/// - `S`: The source type
/// - `A`: The target type
/// - `G`: The getter function type
/// - `St`: The setter function type
///
/// # Example
///
/// ```
/// use lambars::optics::{Lens, FunctionLens};
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Point { x: i32, y: i32 }
///
/// let x_lens = FunctionLens::new(
///     |point: &Point| &point.x,
///     |point: Point, x: i32| Point { x, ..point },
/// );
///
/// let point = Point { x: 10, y: 20 };
/// assert_eq!(*x_lens.get(&point), 10);
/// ```
pub struct FunctionLens<S, A, G, St>
where
    G: Fn(&S) -> &A,
    St: Fn(S, A) -> S,
{
    getter: G,
    setter: St,
    _marker: PhantomData<(S, A)>,
}

impl<S, A, G, St> FunctionLens<S, A, G, St>
where
    G: Fn(&S) -> &A,
    St: Fn(S, A) -> S,
{
    /// Creates a new `FunctionLens` from a getter and setter.
    ///
    /// # Arguments
    ///
    /// * `getter` - A function that extracts the focused field from the source
    /// * `setter` - A function that creates a new source with the field updated
    ///
    /// # Returns
    ///
    /// A new `FunctionLens`
    ///
    /// # Example
    ///
    /// ```
    /// use lambars::optics::{Lens, FunctionLens};
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct Point { x: i32, y: i32 }
    ///
    /// let x_lens = FunctionLens::new(
    ///     |point: &Point| &point.x,
    ///     |point: Point, x: i32| Point { x, ..point },
    /// );
    /// ```
    #[must_use]
    pub const fn new(getter: G, setter: St) -> Self {
        Self {
            getter,
            setter,
            _marker: PhantomData,
        }
    }
}

impl<S, A, G, St> Lens<S, A> for FunctionLens<S, A, G, St>
where
    G: Fn(&S) -> &A,
    St: Fn(S, A) -> S,
{
    fn get<'a>(&self, source: &'a S) -> &'a A {
        (self.getter)(source)
    }

    fn set(&self, source: S, value: A) -> S {
        (self.setter)(source, value)
    }
}

impl<S, A, G, St> Clone for FunctionLens<S, A, G, St>
where
    G: Fn(&S) -> &A + Clone,
    St: Fn(S, A) -> S + Clone,
{
    fn clone(&self) -> Self {
        Self {
            getter: self.getter.clone(),
            setter: self.setter.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, G, St> std::fmt::Debug for FunctionLens<S, A, G, St>
where
    G: Fn(&S) -> &A,
    St: Fn(S, A) -> S,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FunctionLens")
            .finish_non_exhaustive()
    }
}

/// A lens composed of two lenses.
///
/// This allows focusing on nested fields by composing a lens that focuses on
/// an intermediate structure with a lens that focuses on a field within that structure.
///
/// # Type Parameters
///
/// - `L1`: The type of the outer lens
/// - `L2`: The type of the inner lens
/// - `A`: The intermediate type (target of L1, source of L2)
///
/// # Example
///
/// ```
/// use lambars::optics::Lens;
/// use lambars::lens;
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Inner { value: i32 }
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Outer { inner: Inner }
///
/// let inner_lens = lens!(Outer, inner);
/// let value_lens = lens!(Inner, value);
/// let outer_value = inner_lens.compose(value_lens);
///
/// let data = Outer { inner: Inner { value: 42 } };
/// assert_eq!(*outer_value.get(&data), 42);
/// ```
pub struct ComposedLens<L1, L2, A> {
    first: L1,
    second: L2,
    _marker: PhantomData<A>,
}

impl<L1, L2, A> ComposedLens<L1, L2, A> {
    /// Creates a new composed lens.
    ///
    /// # Arguments
    ///
    /// * `first` - The outer lens (focuses on the intermediate structure)
    /// * `second` - The inner lens (focuses on the final field)
    ///
    /// # Returns
    ///
    /// A new `ComposedLens`
    #[must_use]
    pub const fn new(first: L1, second: L2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, L1, L2> Lens<S, B> for ComposedLens<L1, L2, A>
where
    L1: Lens<S, A>,
    L2: Lens<A, B>,
    A: Clone + 'static,
{
    fn get<'a>(&self, source: &'a S) -> &'a B {
        let intermediate = self.first.get(source);
        self.second.get(intermediate)
    }

    fn set(&self, source: S, value: B) -> S {
        let intermediate = self.first.get(&source).clone();
        let new_intermediate = self.second.set(intermediate, value);
        self.first.set(source, new_intermediate)
    }
}

impl<L1: Clone, L2: Clone, A> Clone for ComposedLens<L1, L2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<L1: std::fmt::Debug, L2: std::fmt::Debug, A> std::fmt::Debug for ComposedLens<L1, L2, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComposedLens")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// A lens converted to a traversal.
///
/// This wrapper allows using a lens where a traversal is expected.
/// It will always yield exactly one element.
///
/// # Type Parameters
///
/// - `L`: The type of the underlying lens
/// - `S`: The source type
/// - `A`: The target type
pub struct LensAsTraversal<L, S, A> {
    pub(crate) lens: L,
    _marker: PhantomData<(S, A)>,
}

impl<L, S, A> LensAsTraversal<L, S, A> {
    /// Creates a new `LensAsTraversal` from a lens.
    ///
    /// # Arguments
    ///
    /// * `lens` - The lens to wrap
    ///
    /// # Returns
    ///
    /// A new `LensAsTraversal`
    #[must_use]
    pub const fn new(lens: L) -> Self {
        Self {
            lens,
            _marker: PhantomData,
        }
    }
}

impl<L, S, A> LensAsTraversal<L, S, A>
where
    L: Lens<S, A>,
    A: 'static,
{
    /// Returns an iterator over the focused element(s).
    ///
    /// For a lens, this always yields exactly one element.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure
    ///
    /// # Returns
    ///
    /// An iterator yielding the focused element
    pub fn get_all<'a>(&self, source: &'a S) -> impl Iterator<Item = &'a A>
    where
        A: 'a,
    {
        std::iter::once(self.lens.get(source))
    }

    /// Modifies all focused elements by applying a function.
    ///
    /// For a lens, this modifies exactly one element.
    ///
    /// # Arguments
    ///
    /// * `source` - The source structure (consumed)
    /// * `function` - The function to apply
    ///
    /// # Returns
    ///
    /// A new source with the focused element(s) modified
    pub fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(A) -> A,
        A: Clone,
    {
        let mut function = function;
        self.lens.modify(source, |value| function(value))
    }
}

impl<L: Clone, S, A> Clone for LensAsTraversal<L, S, A> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            _marker: PhantomData,
        }
    }
}

impl<L: std::fmt::Debug, S, A> std::fmt::Debug for LensAsTraversal<L, S, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LensAsTraversal")
            .field("lens", &self.lens)
            .finish()
    }
}

/// Creates a lens for a struct field.
///
/// This macro generates a `FunctionLens` that focuses on the specified field
/// of the given struct type.
///
/// # Syntax
///
/// ```text
/// lens!(StructType, field_name)
/// ```
///
/// # Requirements
///
/// The struct must support the struct update syntax (`..source`), which means
/// all fields not being updated must implement `Clone` or be moved.
///
/// # Example
///
/// ```
/// use lambars::optics::Lens;
/// use lambars::lens;
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Point { x: i32, y: i32 }
///
/// let x_lens = lens!(Point, x);
/// let y_lens = lens!(Point, y);
///
/// let point = Point { x: 10, y: 20 };
///
/// // Get
/// assert_eq!(*x_lens.get(&point), 10);
/// assert_eq!(*y_lens.get(&point), 20);
///
/// // Set
/// let updated = x_lens.set(point, 100);
/// assert_eq!(updated, Point { x: 100, y: 20 });
///
/// // Modify
/// let doubled = x_lens.modify(updated, |x| x * 2);
/// assert_eq!(doubled.x, 200);
/// ```
#[macro_export]
macro_rules! lens {
    ($struct_type:ident, $field:ident) => {
        $crate::optics::FunctionLens::new(
            |source: &$struct_type| &source.$field,
            |mut source: $struct_type, value| {
                source.$field = value;
                source
            },
        )
    };
    ($struct_type:ident < $($generic:tt),+ >, $field:ident) => {
        $crate::optics::FunctionLens::new(
            |source: &$struct_type<$($generic),+>| &source.$field,
            |mut source: $struct_type<$($generic),+>, value| {
                source.$field = value;
                source
            },
        )
    };
    ($struct_type:path, $field:ident) => {
        $crate::optics::FunctionLens::new(
            |source: &$struct_type| &source.$field,
            |mut source: $struct_type, value| {
                source.$field = value;
                source
            },
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[test]
    fn test_function_lens_get() {
        let x_lens = FunctionLens::new(
            |point: &Point| &point.x,
            |point: Point, x: i32| Point { x, ..point },
        );

        let point = Point { x: 10, y: 20 };
        assert_eq!(*x_lens.get(&point), 10);
    }

    #[test]
    fn test_function_lens_set() {
        let x_lens = FunctionLens::new(
            |point: &Point| &point.x,
            |point: Point, x: i32| Point { x, ..point },
        );

        let point = Point { x: 10, y: 20 };
        let updated = x_lens.set(point, 100);
        assert_eq!(updated.x, 100);
        assert_eq!(updated.y, 20);
    }

    #[test]
    fn test_lens_modify() {
        let x_lens = lens!(Point, x);
        let point = Point { x: 10, y: 20 };
        let doubled = x_lens.modify(point, |x| x * 2);
        assert_eq!(doubled.x, 20);
    }

    #[test]
    fn test_lens_compose() {
        #[derive(Clone, PartialEq, Debug)]
        struct Inner {
            value: i32,
        }

        #[derive(Clone, PartialEq, Debug)]
        struct Outer {
            inner: Inner,
        }

        let inner_lens = lens!(Outer, inner);
        let value_lens = lens!(Inner, value);
        let composed = inner_lens.compose(value_lens);

        let data = Outer {
            inner: Inner { value: 42 },
        };

        assert_eq!(*composed.get(&data), 42);

        let updated = composed.set(data, 100);
        assert_eq!(updated.inner.value, 100);
    }

    #[test]
    fn test_lens_macro() {
        let x_lens = lens!(Point, x);
        let point = Point { x: 10, y: 20 };
        assert_eq!(*x_lens.get(&point), 10);
    }

    #[test]
    fn test_lens_to_traversal() {
        let x_lens = lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        let all: Vec<&i32> = traversal.get_all(&point).collect();
        assert_eq!(all, vec![&10]);
    }
}
