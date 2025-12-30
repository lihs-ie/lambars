//! Standard optics that are commonly used.
//!
//! This module provides pre-defined optics for common use cases.

use super::{FunctionIso, Iso};

/// Creates an identity Iso that doesn't transform the value.
///
/// The identity Iso satisfies:
/// - `iso.get(x) == x`
/// - `iso.reverse_get(x) == x`
///
/// # Type Parameters
///
/// - `T`: The type to create an identity Iso for
///
/// # Returns
///
/// An identity Iso
///
/// # Example
///
/// ```
/// use functional_rusty::optics::{Iso, iso_identity};
///
/// let identity_iso = iso_identity::<i32>();
///
/// assert_eq!(identity_iso.get(42), 42);
/// assert_eq!(identity_iso.reverse_get(42), 42);
/// ```
#[must_use]
pub fn iso_identity<T>() -> impl Iso<T, T> + Clone {
    FunctionIso::new(|x: T| x, |x: T| x)
}

/// Creates an Iso that swaps the elements of a tuple.
///
/// Converts `(A, B)` to `(B, A)` and vice versa.
///
/// # Type Parameters
///
/// - `A`: The first element type
/// - `B`: The second element type
///
/// # Returns
///
/// A swap Iso
///
/// # Example
///
/// ```
/// use functional_rusty::optics::{Iso, iso_swap};
///
/// let swap_iso = iso_swap::<i32, String>();
///
/// let tuple = (42, "hello".to_string());
/// let swapped = swap_iso.get(tuple.clone());
/// assert_eq!(swapped, ("hello".to_string(), 42));
///
/// let back = swap_iso.reverse_get(swapped);
/// assert_eq!(back, tuple);
/// ```
#[must_use]
pub fn iso_swap<A, B>() -> impl Iso<(A, B), (B, A)> + Clone {
    FunctionIso::new(|(a, b): (A, B)| (b, a), |(b, a): (B, A)| (a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_identity() {
        let identity_iso = iso_identity::<i32>();

        assert_eq!(identity_iso.get(42), 42);
        assert_eq!(identity_iso.reverse_get(42), 42);

        // Roundtrip
        let value = 100;
        assert_eq!(identity_iso.reverse_get(identity_iso.get(value)), value);
    }

    #[test]
    fn test_iso_identity_with_string() {
        let identity_iso = iso_identity::<String>();

        let value = "hello".to_string();
        assert_eq!(identity_iso.get(value.clone()), value);
    }

    #[test]
    fn test_iso_swap() {
        let swap_iso = iso_swap::<i32, String>();

        let tuple = (42, "hello".to_string());
        let swapped = swap_iso.get(tuple.clone());
        assert_eq!(swapped, ("hello".to_string(), 42));

        let back = swap_iso.reverse_get(swapped);
        assert_eq!(back, tuple);
    }

    #[test]
    fn test_iso_swap_roundtrip() {
        let swap_iso = iso_swap::<i32, String>();

        let tuple = (42, "hello".to_string());
        let roundtrip = swap_iso.reverse_get(swap_iso.get(tuple.clone()));
        assert_eq!(roundtrip, tuple);
    }
}
