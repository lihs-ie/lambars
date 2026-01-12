//! Functor type class - mapping over container values.
//!
//! This module provides the `Functor` trait, which represents types that can
//! have a function applied to their inner value(s) while preserving the structure.
//!
//! A `Functor` is one of the fundamental abstractions in functional programming,
//! allowing you to transform the contents of a container without changing its shape.
//!
//! # Laws
//!
//! All `Functor` implementations must satisfy these laws:
//!
//! ## Identity Law
//!
//! Mapping the identity function over a functor should return an equivalent functor:
//!
//! ```text
//! fa.fmap(|x| x) == fa
//! ```
//!
//! ## Composition Law
//!
//! Mapping two functions in sequence should be equivalent to mapping their composition:
//!
//! ```text
//! fa.fmap(f).fmap(g) == fa.fmap(|x| g(f(x)))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::Functor;
//!
//! // Option as a Functor
//! let some_value: Option<i32> = Some(5);
//! let transformed: Option<String> = some_value.fmap(|n| n.to_string());
//! assert_eq!(transformed, Some("5".to_string()));
//!
//! // None is preserved
//! let none_value: Option<i32> = None;
//! let transformed: Option<String> = none_value.fmap(|n| n.to_string());
//! assert_eq!(transformed, None);
//! ```

use super::higher::TypeConstructor;
use super::identity::Identity;

/// A type class for types that can have a function mapped over their contents.
///
/// `Functor` represents the ability to apply a function to the value(s) inside
/// a container while preserving the container's structure. This is one of the
/// most fundamental abstractions in functional programming.
///
/// # Laws
///
/// ## Identity Law
///
/// Mapping the identity function returns an equivalent functor:
///
/// ```text
/// fa.fmap(|x| x) == fa
/// ```
///
/// ## Composition Law
///
/// Mapping composed functions is equivalent to mapping them in sequence:
///
/// ```text
/// fa.fmap(f).fmap(g) == fa.fmap(|x| g(f(x)))
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Functor;
///
/// let x: Option<i32> = Some(5);
/// let y: Option<String> = x.fmap(|n| n.to_string());
/// assert_eq!(y, Some("5".to_string()));
/// ```
pub trait Functor: TypeConstructor {
    /// Applies a function to the value inside the functor.
    ///
    /// This is the primary operation of the Functor type class. It takes a
    /// function that transforms the inner type and returns a new functor
    /// with the transformed value(s).
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms the inner value
    ///
    /// # Returns
    ///
    /// A new functor with the transformed value(s)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Functor;
    ///
    /// let x: Option<i32> = Some(5);
    /// let y: Option<i32> = x.fmap(|n| n * 2);
    /// assert_eq!(y, Some(10));
    /// ```
    fn fmap<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> B + 'static,
        B: 'static;

    /// Applies a function to a reference of the value inside the functor.
    ///
    /// This method is useful when you want to transform the functor's contents
    /// without consuming it, or when the inner type does not implement `Clone`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes a reference to the inner value
    ///
    /// # Returns
    ///
    /// A new functor with the transformed value(s)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Functor;
    ///
    /// let x: Option<String> = Some("hello".to_string());
    /// let y: Option<usize> = x.fmap_ref(|s| s.len());
    /// assert_eq!(y, Some(5));
    /// // x is still available here
    /// ```
    fn fmap_ref<B, F>(&self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(&Self::Inner) -> B + 'static,
        B: 'static;

    /// Replaces the value inside the functor with a constant value.
    ///
    /// This is equivalent to `fmap(|_| value)`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to place inside the functor
    ///
    /// # Returns
    ///
    /// A new functor containing the given value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Functor;
    ///
    /// let x: Option<i32> = Some(5);
    /// assert_eq!(x.replace("replaced"), Some("replaced"));
    ///
    /// let y: Option<i32> = None;
    /// assert_eq!(y.replace("replaced"), None);
    /// ```
    #[inline]
    fn replace<B>(self, value: B) -> Self::WithType<B>
    where
        Self: Sized,
        B: 'static,
    {
        self.fmap(|_| value)
    }

    /// Discards the value inside the functor, replacing it with `()`.
    ///
    /// This is useful when you only care about the structure/effect of
    /// the functor and not the value it contains.
    ///
    /// This is equivalent to `replace(())` or `fmap(|_| ())`.
    ///
    /// # Returns
    ///
    /// A new functor containing `()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Functor;
    ///
    /// let x: Option<i32> = Some(5);
    /// assert_eq!(x.void(), Some(()));
    ///
    /// let y: Option<i32> = None;
    /// assert_eq!(y.void(), None);
    /// ```
    #[inline]
    fn void(self) -> Self::WithType<()>
    where
        Self: Sized,
    {
        self.replace(())
    }
}

/// An extension of `Functor` for containers with multiple elements.
///
/// While `Functor::fmap` takes a `FnOnce` (which can only be called once),
/// containers like `Vec` need to apply the function to multiple elements.
/// This trait provides `fmap_mut` which takes a `FnMut` that can be called
/// multiple times.
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::FunctorMut;
///
/// let numbers = vec![1, 2, 3];
/// let doubled: Vec<i32> = numbers.fmap_mut(|n| n * 2);
/// assert_eq!(doubled, vec![2, 4, 6]);
/// ```
pub trait FunctorMut: Functor {
    /// Applies a mutable function to each element in the functor.
    ///
    /// This method is necessary for containers with multiple elements,
    /// as `FnOnce` can only be called once.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that can be called multiple times
    ///
    /// # Returns
    ///
    /// A new functor with all elements transformed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::FunctorMut;
    ///
    /// let v = vec![1, 2, 3];
    /// let result: Vec<i32> = v.fmap_mut(|x| x + 1);
    /// assert_eq!(result, vec![2, 3, 4]);
    /// ```
    fn fmap_mut<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnMut(Self::Inner) -> B;

    /// Applies a mutable function to references of each element.
    ///
    /// Like `fmap_ref`, but can be called multiple times.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes references and can be called multiple times
    ///
    /// # Returns
    ///
    /// A new functor with all elements transformed
    fn fmap_ref_mut<B, F>(&self, function: F) -> Self::WithType<B>
    where
        F: FnMut(&Self::Inner) -> B;
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Functor for Option<A> {
    #[inline]
    fn fmap<B, F>(self, function: F) -> Option<B>
    where
        F: FnOnce(A) -> B,
    {
        self.map(function)
    }

    #[inline]
    fn fmap_ref<B, F>(&self, function: F) -> Option<B>
    where
        F: FnOnce(&A) -> B,
    {
        self.as_ref().map(function)
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone> Functor for Result<T, E> {
    #[inline]
    fn fmap<B, F>(self, function: F) -> Result<B, E>
    where
        F: FnOnce(T) -> B,
    {
        self.map(function)
    }

    #[inline]
    fn fmap_ref<B, F>(&self, function: F) -> Result<B, E>
    where
        F: FnOnce(&T) -> B,
    {
        match self {
            Ok(value) => Ok(function(value)),
            Err(error) => Err(error.clone()),
        }
    }
}

// =============================================================================
// Vec<T> Implementation
// =============================================================================

impl<T> Functor for Vec<T> {
    /// Maps a function over a single-element Vec.
    ///
    /// Note: For multi-element Vecs, use `fmap_mut` instead, as `FnOnce`
    /// can only be called once. This implementation will only work correctly
    /// for empty or single-element Vecs.
    #[inline]
    fn fmap<B, F>(self, function: F) -> Vec<B>
    where
        F: FnOnce(T) -> B,
    {
        // For single-element or empty Vec, FnOnce is sufficient
        // For multi-element Vec, this will only transform the first element
        // Users should use fmap_mut for proper multi-element transformation
        let mut iter = self.into_iter();
        iter.next().map_or_else(Vec::new, |first| {
            let mut result = Vec::with_capacity(iter.len() + 1);
            result.push(function(first));
            // Note: remaining elements are dropped as FnOnce cannot be reused
            result
        })
    }

    #[inline]
    fn fmap_ref<B, F>(&self, function: F) -> Vec<B>
    where
        F: FnOnce(&T) -> B,
    {
        let mut iter = self.iter();
        iter.next().map_or_else(Vec::new, |first| {
            let mut result = Vec::with_capacity(self.len());
            result.push(function(first));
            result
        })
    }
}

impl<T> FunctorMut for Vec<T> {
    #[inline]
    fn fmap_mut<B, F>(self, function: F) -> Vec<B>
    where
        F: FnMut(T) -> B,
    {
        self.into_iter().map(function).collect()
    }

    #[inline]
    fn fmap_ref_mut<B, F>(&self, function: F) -> Vec<B>
    where
        F: FnMut(&T) -> B,
    {
        self.iter().map(function).collect()
    }
}

// =============================================================================
// Box<T> Implementation
// =============================================================================

impl<T> Functor for Box<T> {
    #[inline]
    fn fmap<B, F>(self, function: F) -> Box<B>
    where
        F: FnOnce(T) -> B,
    {
        Box::new(function(*self))
    }

    #[inline]
    fn fmap_ref<B, F>(&self, function: F) -> Box<B>
    where
        F: FnOnce(&T) -> B,
    {
        Box::new(function(self.as_ref()))
    }
}

// =============================================================================
// Identity<A> Implementation
// =============================================================================

impl<A> Functor for Identity<A> {
    #[inline]
    fn fmap<B, F>(self, function: F) -> Identity<B>
    where
        F: FnOnce(A) -> B,
    {
        Identity(function(self.0))
    }

    #[inline]
    fn fmap_ref<B, F>(&self, function: F) -> Identity<B>
    where
        F: FnOnce(&A) -> B,
    {
        Identity(function(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Option<A> Tests
    // =========================================================================

    #[rstest]
    fn option_fmap_some() {
        let x: Option<i32> = Some(5);
        let y: Option<String> = x.fmap(|n| n.to_string());
        assert_eq!(y, Some("5".to_string()));
    }

    #[rstest]
    fn option_fmap_none() {
        let x: Option<i32> = None;
        let y: Option<String> = x.fmap(|n| n.to_string());
        assert_eq!(y, None);
    }

    #[rstest]
    fn option_fmap_ref_some() {
        let x: Option<String> = Some("hello".to_string());
        let y: Option<usize> = x.fmap_ref(|s| s.len());
        assert_eq!(y, Some(5));
        // Verify x is still available
        assert_eq!(x, Some("hello".to_string()));
    }

    #[rstest]
    fn option_fmap_ref_none() {
        let x: Option<String> = None;
        let y: Option<usize> = x.fmap_ref(|s| s.len());
        assert_eq!(y, None);
    }

    #[rstest]
    fn option_replace_some() {
        let x: Option<i32> = Some(5);
        assert_eq!(x.replace("replaced"), Some("replaced"));
    }

    #[rstest]
    fn option_replace_none() {
        let x: Option<i32> = None;
        assert_eq!(x.replace("replaced"), None);
    }

    #[rstest]
    fn option_void_some() {
        let x: Option<i32> = Some(5);
        assert_eq!(x.void(), Some(()));
    }

    #[rstest]
    fn option_void_none() {
        let x: Option<i32> = None;
        assert_eq!(x.void(), None);
    }

    // =========================================================================
    // Result<T, E> Tests
    // =========================================================================

    #[rstest]
    fn result_fmap_ok() {
        let x: Result<i32, &str> = Ok(5);
        let y: Result<String, &str> = x.fmap(|n| n.to_string());
        assert_eq!(y, Ok("5".to_string()));
    }

    #[rstest]
    fn result_fmap_err() {
        let x: Result<i32, &str> = Err("error");
        let y: Result<String, &str> = x.fmap(|n| n.to_string());
        assert_eq!(y, Err("error"));
    }

    #[rstest]
    fn result_fmap_ref_ok() {
        let x: Result<String, String> = Ok("hello".to_string());
        let y: Result<usize, String> = x.fmap_ref(|s| s.len());
        assert_eq!(y, Ok(5));
        // Verify x is still available
        assert_eq!(x, Ok("hello".to_string()));
    }

    #[rstest]
    fn result_fmap_ref_err() {
        let x: Result<String, String> = Err("error".to_string());
        let y: Result<usize, String> = x.fmap_ref(|s| s.len());
        assert_eq!(y, Err("error".to_string()));
    }

    #[rstest]
    fn result_replace_ok() {
        let x: Result<i32, &str> = Ok(5);
        assert_eq!(x.replace("replaced"), Ok("replaced"));
    }

    #[rstest]
    fn result_replace_err() {
        let x: Result<i32, &str> = Err("error");
        assert_eq!(x.replace("replaced"), Err("error"));
    }

    #[rstest]
    fn result_void_ok() {
        let x: Result<i32, &str> = Ok(5);
        assert_eq!(x.void(), Ok(()));
    }

    #[rstest]
    fn result_void_err() {
        let x: Result<i32, &str> = Err("error");
        assert_eq!(x.void(), Err("error"));
    }

    // =========================================================================
    // Vec<A> Tests (FunctorMut)
    // =========================================================================

    #[rstest]
    fn vec_fmap_mut_transforms_all_elements() {
        let numbers = vec![1, 2, 3];
        let doubled: Vec<i32> = numbers.fmap_mut(|n| n * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[rstest]
    fn vec_fmap_mut_empty() {
        let empty: Vec<i32> = vec![];
        let result: Vec<String> = empty.fmap_mut(|n| n.to_string());
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_fmap_ref_mut_transforms_all_elements() {
        let strings = vec!["hello".to_string(), "world".to_string()];
        let lengths: Vec<usize> = strings.fmap_ref_mut(|s| s.len());
        assert_eq!(lengths, vec![5, 5]);
        // Verify original is still available
        assert_eq!(strings, vec!["hello".to_string(), "world".to_string()]);
    }

    #[rstest]
    fn vec_fmap_single_element() {
        // For single element, fmap with FnOnce works correctly
        let single = vec![42];
        let result: Vec<String> = single.fmap(|n| n.to_string());
        assert_eq!(result, vec!["42".to_string()]);
    }

    #[rstest]
    fn vec_fmap_empty() {
        let empty: Vec<i32> = vec![];
        let result: Vec<String> = empty.fmap(|n| n.to_string());
        assert!(result.is_empty());
    }

    // =========================================================================
    // Box<A> Tests
    // =========================================================================

    #[rstest]
    fn box_fmap_transforms_value() {
        let boxed = Box::new(42);
        let result: Box<String> = boxed.fmap(|n| n.to_string());
        assert_eq!(*result, "42".to_string());
    }

    #[rstest]
    fn box_fmap_ref_transforms_value() {
        let boxed = Box::new("hello".to_string());
        let result: Box<usize> = boxed.fmap_ref(|s| s.len());
        assert_eq!(*result, 5);
        // Verify original is still available
        assert_eq!(*boxed, "hello".to_string());
    }

    #[rstest]
    fn box_replace() {
        let boxed = Box::new(42);
        let result: Box<&str> = boxed.replace("replaced");
        assert_eq!(*result, "replaced");
    }

    #[rstest]
    fn box_void() {
        let boxed = Box::new(42);
        let result: Box<()> = boxed.void();
        assert_eq!(*result, ());
    }

    // =========================================================================
    // Identity<A> Tests
    // =========================================================================

    #[rstest]
    fn identity_fmap_transforms_value() {
        let wrapped = Identity::new(42);
        let result: Identity<String> = wrapped.fmap(|n| n.to_string());
        assert_eq!(result, Identity::new("42".to_string()));
    }

    #[rstest]
    fn identity_fmap_ref_transforms_value() {
        let wrapped = Identity::new("hello".to_string());
        let result: Identity<usize> = wrapped.fmap_ref(|s| s.len());
        assert_eq!(result, Identity::new(5));
        // Verify original is still available
        assert_eq!(wrapped, Identity::new("hello".to_string()));
    }

    #[rstest]
    fn identity_replace() {
        let wrapped = Identity::new(42);
        let result: Identity<&str> = wrapped.replace("replaced");
        assert_eq!(result, Identity::new("replaced"));
    }

    #[rstest]
    fn identity_void() {
        let wrapped = Identity::new(42);
        let result: Identity<()> = wrapped.void();
        assert_eq!(result, Identity::new(()));
    }

    // =========================================================================
    // Law Tests (Unit Tests)
    // =========================================================================

    /// Identity law: fa.fmap(|x| x) == fa
    #[rstest]
    fn option_identity_law() {
        let some_value: Option<i32> = Some(42);
        assert_eq!(some_value.fmap(|x| x), some_value);

        let none_value: Option<i32> = None;
        assert_eq!(none_value.fmap(|x| x), none_value);
    }

    /// Composition law: fa.fmap(f).fmap(g) == fa.fmap(|x| g(f(x)))
    #[rstest]
    fn option_composition_law() {
        let some_value: Option<i32> = Some(5);
        let function1 = |n: i32| n + 1;
        let function2 = |n: i32| n * 2;

        let left = some_value.fmap(function1).fmap(function2);
        let right = some_value.fmap(move |x| function2(function1(x)));

        assert_eq!(left, right);
        assert_eq!(left, Some(12)); // (5 + 1) * 2 = 12
    }

    #[rstest]
    fn result_identity_law() {
        let ok_value: Result<i32, &str> = Ok(42);
        assert_eq!(ok_value.fmap(|x| x), ok_value);

        let err_value: Result<i32, &str> = Err("error");
        assert_eq!(err_value.fmap(|x| x), err_value);
    }

    #[rstest]
    fn result_composition_law() {
        let ok_value: Result<i32, &str> = Ok(5);
        let function1 = |n: i32| n + 1;
        let function2 = |n: i32| n * 2;

        let left = ok_value.fmap(function1).fmap(function2);
        let right = ok_value.fmap(move |x| function2(function1(x)));

        assert_eq!(left, right);
    }

    #[rstest]
    fn box_identity_law() {
        let boxed: Box<i32> = Box::new(42);
        let cloned = Box::new(42);
        assert_eq!(boxed.fmap(|x| x), cloned);
    }

    #[rstest]
    fn box_composition_law() {
        let boxed: Box<i32> = Box::new(5);
        let function1 = |n: i32| n + 1;
        let function2 = |n: i32| n * 2;

        let left = Box::new(5).fmap(function1).fmap(function2);
        let right = boxed.fmap(move |x| function2(function1(x)));

        assert_eq!(left, right);
    }

    #[rstest]
    fn identity_wrapper_identity_law() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.fmap(|x| x), wrapped);
    }

    #[rstest]
    fn identity_wrapper_composition_law() {
        let wrapped = Identity::new(5);
        let function1 = |n: i32| n + 1;
        let function2 = |n: i32| n * 2;

        let left = wrapped.fmap(function1).fmap(function2);
        let right = wrapped.fmap(move |x| function2(function1(x)));

        assert_eq!(left, right);
    }

    /// For Vec, we test the laws using `FunctorMut::fmap_mut`
    #[rstest]
    fn vec_identity_law_with_fmap_mut() {
        let vec_value = vec![1, 2, 3];
        assert_eq!(vec_value.clone().fmap_mut(|x| x), vec_value);
    }

    #[rstest]
    fn vec_composition_law_with_fmap_mut() {
        let vec_value = vec![1, 2, 3];
        let function1 = |n: i32| n + 1;
        let function2 = |n: i32| n * 2;

        let left: Vec<i32> = vec_value.clone().fmap_mut(function1).fmap_mut(function2);
        let right: Vec<i32> = vec_value.fmap_mut(|x| function2(function1(x)));

        assert_eq!(left, right);
        assert_eq!(left, vec![4, 6, 8]); // [(1+1)*2, (2+1)*2, (3+1)*2]
    }
}
