//! Optics composition utilities.
//!
//! This module provides extension traits for composing different types of optics.
//! The optics hierarchy determines which compositions are valid:
//!
//! ```text
//! Iso > Lens > Optional > Traversal > Fold
//! Iso > Prism > Optional > Traversal > Fold
//! ```
//!
//! # Composition Rules
//!
//! - Lens + Optional -> Optional
//! - Lens + Traversal -> Traversal
//! - Lens + Fold -> Fold
//! - Traversal + Fold -> Fold
//! - Optional + Optional -> Optional
//! - Optional + Traversal -> Traversal
//!
//! # Example
//!
//! ```
//! use lambars::optics::{Lens, Traversal, Optional};
//! use lambars::optics::optics_compose::LensComposeWithOptional;
//! use lambars::optics::ixed::Ixed;
//! use lambars::lens;
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Container {
//!     items: Vec<i32>,
//! }
//!
//! let items_lens = lens!(Container, items);
//! let first_item = <Vec<i32> as Ixed<usize>>::ix(0);
//! let composed = items_lens.compose_optional(first_item);
//!
//! let container = Container { items: vec![1, 2, 3] };
//! assert_eq!(composed.get_option(&container), Some(&1));
//! ```

use std::marker::PhantomData;

use crate::optics::Fold;
use crate::optics::Lens;
use crate::optics::Optional;
use crate::optics::Traversal;

// =============================================================================
// Lens + Optional -> Optional
// =============================================================================

/// Result of composing a Lens with an Optional.
#[derive(Debug)]
pub struct LensOptionalComposition<L, O, A> {
    lens: L,
    optional: O,
    _marker: PhantomData<A>,
}

impl<L, O, A> LensOptionalComposition<L, O, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(lens: L, optional: O) -> Self {
        Self {
            lens,
            optional,
            _marker: PhantomData,
        }
    }
}

impl<L: Clone, O: Clone, A> Clone for LensOptionalComposition<L, O, A> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            optional: self.optional.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, L, O> Optional<S, B> for LensOptionalComposition<L, O, A>
where
    L: Lens<S, A>,
    O: Optional<A, B>,
    A: Clone + 'static,
{
    fn get_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        let intermediate = self.lens.get(source);
        self.optional.get_option(intermediate)
    }

    fn set(&self, source: S, value: B) -> S {
        let intermediate = self.lens.get(&source).clone();
        let new_intermediate = self.optional.set(intermediate, value);
        self.lens.set(source, new_intermediate)
    }
}

/// Extension trait for composing Lens with Optional.
pub trait LensComposeWithOptional<S, A>: Lens<S, A> {
    /// Composes this lens with an optional to create an optional.
    fn compose_optional<B, O>(self, optional: O) -> LensOptionalComposition<Self, O, A>
    where
        Self: Sized,
        O: Optional<A, B>,
    {
        LensOptionalComposition::new(self, optional)
    }
}

impl<S, A, L> LensComposeWithOptional<S, A> for L where L: Lens<S, A> {}

// =============================================================================
// Lens + Traversal -> Traversal
// =============================================================================

/// Result of composing a Lens with a Traversal.
#[derive(Debug)]
pub struct LensTraversalComposition<L, T, A> {
    lens: L,
    traversal: T,
    _marker: PhantomData<A>,
}

impl<L, T, A> LensTraversalComposition<L, T, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(lens: L, traversal: T) -> Self {
        Self {
            lens,
            traversal,
            _marker: PhantomData,
        }
    }
}

impl<L: Clone, T: Clone, A> Clone for LensTraversalComposition<L, T, A> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            traversal: self.traversal.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, L, T> Traversal<S, B> for LensTraversalComposition<L, T, A>
where
    L: Lens<S, A>,
    T: Traversal<A, B>,
    A: Clone + 'static,
    B: Clone + 'static,
    S: 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a B> + 'a> {
        let intermediate = self.lens.get(source);
        self.traversal.get_all(intermediate)
    }

    fn get_all_owned(&self, source: S) -> Vec<B> {
        let intermediate = self.lens.get(&source).clone();
        self.traversal.get_all_owned(intermediate)
    }

    fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(B) -> B,
    {
        let intermediate = self.lens.get(&source).clone();
        let new_intermediate = self.traversal.modify_all(intermediate, function);
        self.lens.set(source, new_intermediate)
    }
}

/// Extension trait for composing Lens with Traversal.
pub trait LensComposeWithTraversal<S, A>: Lens<S, A> {
    /// Composes this lens with a traversal to create a traversal.
    fn compose_traversal<B, T>(self, traversal: T) -> LensTraversalComposition<Self, T, A>
    where
        Self: Sized,
        T: Traversal<A, B>,
    {
        LensTraversalComposition::new(self, traversal)
    }
}

impl<S, A, L> LensComposeWithTraversal<S, A> for L where L: Lens<S, A> {}

// =============================================================================
// Lens + Fold -> Fold
// =============================================================================

/// Result of composing a Lens with a Fold.
#[derive(Debug)]
pub struct LensFoldComposition<L, F, A> {
    lens: L,
    fold: F,
    _marker: PhantomData<A>,
}

impl<L, F, A> LensFoldComposition<L, F, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(lens: L, fold: F) -> Self {
        Self {
            lens,
            fold,
            _marker: PhantomData,
        }
    }
}

impl<L: Clone, F: Clone, A> Clone for LensFoldComposition<L, F, A> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            fold: self.fold.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, L, F2> Fold<S, B> for LensFoldComposition<L, F2, A>
where
    L: Lens<S, A>,
    F2: Fold<A, B>,
    A: 'static,
    B: 'static,
    S: 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a B> + 'a> {
        let intermediate = self.lens.get(source);
        self.fold.get_all(intermediate)
    }

    fn fold<C, G>(&self, source: &S, initial: C, function: G) -> C
    where
        G: FnMut(C, &B) -> C,
    {
        let intermediate = self.lens.get(source);
        self.fold.fold(intermediate, initial, function)
    }

    fn length(&self, source: &S) -> usize {
        let intermediate = self.lens.get(source);
        self.fold.length(intermediate)
    }

    fn for_all<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&B) -> bool,
    {
        let intermediate = self.lens.get(source);
        self.fold.for_all(intermediate, predicate)
    }

    fn exists<P>(&self, source: &S, predicate: P) -> bool
    where
        P: FnMut(&B) -> bool,
    {
        let intermediate = self.lens.get(source);
        self.fold.exists(intermediate, predicate)
    }

    fn head_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        let intermediate = self.lens.get(source);
        self.fold.head_option(intermediate)
    }

    fn last_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        let intermediate = self.lens.get(source);
        self.fold.last_option(intermediate)
    }

    fn is_empty(&self, source: &S) -> bool {
        let intermediate = self.lens.get(source);
        self.fold.is_empty(intermediate)
    }

    fn to_vec<'a>(&self, source: &'a S) -> Vec<&'a B> {
        let intermediate = self.lens.get(source);
        self.fold.to_vec(intermediate)
    }
}

/// Extension trait for composing Lens with Fold.
pub trait LensComposeWithFold<S, A>: Lens<S, A> {
    /// Composes this lens with a fold to create a fold.
    fn compose_fold<B, F2>(self, fold: F2) -> LensFoldComposition<Self, F2, A>
    where
        Self: Sized,
        F2: Fold<A, B>,
    {
        LensFoldComposition::new(self, fold)
    }
}

impl<S, A, L> LensComposeWithFold<S, A> for L where L: Lens<S, A> {}

// =============================================================================
// Traversal + Fold -> Fold
// =============================================================================

/// Result of composing a Traversal with a Fold.
#[derive(Debug)]
pub struct TraversalFoldComposition<T, F, A> {
    traversal: T,
    fold: F,
    _marker: PhantomData<A>,
}

impl<T, F, A> TraversalFoldComposition<T, F, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(traversal: T, fold: F) -> Self {
        Self {
            traversal,
            fold,
            _marker: PhantomData,
        }
    }
}

impl<T: Clone, F: Clone, A> Clone for TraversalFoldComposition<T, F, A> {
    fn clone(&self) -> Self {
        Self {
            traversal: self.traversal.clone(),
            fold: self.fold.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, T, F2> Fold<S, B> for TraversalFoldComposition<T, F2, A>
where
    T: Traversal<S, A>,
    F2: Fold<A, B> + Clone + 'static,
    A: 'static,
    B: 'static,
    S: 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a B> + 'a> {
        let fold = self.fold.clone();
        Box::new(
            self.traversal
                .get_all(source)
                .flat_map(move |a| fold.get_all(a)),
        )
    }

    fn fold<C, G>(&self, source: &S, initial: C, mut function: G) -> C
    where
        G: FnMut(C, &B) -> C,
    {
        self.traversal
            .get_all(source)
            .fold(initial, |acc, a| self.fold.fold(a, acc, &mut function))
    }

    fn length(&self, source: &S) -> usize {
        self.traversal
            .get_all(source)
            .map(|a| self.fold.length(a))
            .sum()
    }

    fn for_all<P>(&self, source: &S, mut predicate: P) -> bool
    where
        P: FnMut(&B) -> bool,
    {
        self.traversal
            .get_all(source)
            .all(|a| self.fold.for_all(a, &mut predicate))
    }

    fn exists<P>(&self, source: &S, mut predicate: P) -> bool
    where
        P: FnMut(&B) -> bool,
    {
        self.traversal
            .get_all(source)
            .any(|a| self.fold.exists(a, &mut predicate))
    }

    fn head_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        self.traversal
            .get_all(source)
            .find_map(|a| self.fold.head_option(a))
    }

    fn last_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        self.traversal
            .get_all(source)
            .filter_map(|a| self.fold.last_option(a))
            .last()
    }

    fn is_empty(&self, source: &S) -> bool {
        self.traversal
            .get_all(source)
            .all(|a| self.fold.is_empty(a))
    }

    fn to_vec<'a>(&self, source: &'a S) -> Vec<&'a B> {
        self.traversal
            .get_all(source)
            .flat_map(|a| self.fold.to_vec(a))
            .collect()
    }
}

/// Extension trait for composing Traversal with Fold.
pub trait TraversalComposeWithFold<S, A>: Traversal<S, A> {
    /// Composes this traversal with a fold to create a fold.
    fn compose_fold<B, F2>(self, fold: F2) -> TraversalFoldComposition<Self, F2, A>
    where
        Self: Sized,
        F2: Fold<A, B> + Clone + 'static,
    {
        TraversalFoldComposition::new(self, fold)
    }
}

impl<S, A, T> TraversalComposeWithFold<S, A> for T where T: Traversal<S, A> {}

// =============================================================================
// Optional + Optional -> Optional
// =============================================================================

/// Result of composing two Optionals.
#[derive(Debug)]
pub struct OptionalOptionalComposition<O1, O2, A> {
    first: O1,
    second: O2,
    _marker: PhantomData<A>,
}

impl<O1, O2, A> OptionalOptionalComposition<O1, O2, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(first: O1, second: O2) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
}

impl<O1: Clone, O2: Clone, A> Clone for OptionalOptionalComposition<O1, O2, A> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, O1, O2> Optional<S, B> for OptionalOptionalComposition<O1, O2, A>
where
    O1: Optional<S, A>,
    O2: Optional<A, B>,
    A: Clone + 'static,
{
    fn get_option<'a>(&self, source: &'a S) -> Option<&'a B> {
        self.first
            .get_option(source)
            .and_then(|a| self.second.get_option(a))
    }

    fn set(&self, source: S, value: B) -> S {
        if let Some(intermediate) = self.first.get_option(&source).cloned() {
            let new_intermediate = self.second.set(intermediate, value);
            self.first.set(source, new_intermediate)
        } else {
            source
        }
    }
}

/// Extension trait for composing Optional with Optional.
pub trait OptionalComposeWithOptional<S, A>: Optional<S, A> {
    /// Composes this optional with another optional to create an optional.
    fn compose_optional<B, O>(self, other: O) -> OptionalOptionalComposition<Self, O, A>
    where
        Self: Sized,
        O: Optional<A, B>,
    {
        OptionalOptionalComposition::new(self, other)
    }
}

impl<S, A, O> OptionalComposeWithOptional<S, A> for O where O: Optional<S, A> {}

// =============================================================================
// Optional + Traversal -> Traversal
// =============================================================================

/// Result of composing an Optional with a Traversal.
#[derive(Debug)]
pub struct OptionalTraversalComposition<O, T, A> {
    optional: O,
    traversal: T,
    _marker: PhantomData<A>,
}

impl<O, T, A> OptionalTraversalComposition<O, T, A> {
    /// Creates a new composition.
    #[must_use]
    pub const fn new(optional: O, traversal: T) -> Self {
        Self {
            optional,
            traversal,
            _marker: PhantomData,
        }
    }
}

impl<O: Clone, T: Clone, A> Clone for OptionalTraversalComposition<O, T, A> {
    fn clone(&self) -> Self {
        Self {
            optional: self.optional.clone(),
            traversal: self.traversal.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, A, B, O, T> Traversal<S, B> for OptionalTraversalComposition<O, T, A>
where
    O: Optional<S, A>,
    T: Traversal<A, B>,
    A: Clone + 'static,
    B: Clone + 'static,
    S: Clone + 'static,
{
    fn get_all<'a>(&self, source: &'a S) -> Box<dyn Iterator<Item = &'a B> + 'a> {
        match self.optional.get_option(source) {
            Some(intermediate) => self.traversal.get_all(intermediate),
            None => Box::new(std::iter::empty()),
        }
    }

    fn get_all_owned(&self, source: S) -> Vec<B> {
        self.optional
            .get_option(&source)
            .cloned()
            .map_or_else(Vec::new, |intermediate| {
                self.traversal.get_all_owned(intermediate)
            })
    }

    fn modify_all<F>(&self, source: S, function: F) -> S
    where
        F: FnMut(B) -> B,
    {
        if let Some(intermediate) = self.optional.get_option(&source).cloned() {
            let new_intermediate = self.traversal.modify_all(intermediate, function);
            self.optional.set(source, new_intermediate)
        } else {
            source
        }
    }
}

/// Extension trait for composing Optional with Traversal.
pub trait OptionalComposeWithTraversal<S, A>: Optional<S, A> {
    /// Composes this optional with a traversal to create a traversal.
    fn compose_traversal<B, T>(self, traversal: T) -> OptionalTraversalComposition<Self, T, A>
    where
        Self: Sized,
        T: Traversal<A, B>,
    {
        OptionalTraversalComposition::new(self, traversal)
    }
}

impl<S, A, O> OptionalComposeWithTraversal<S, A> for O where O: Optional<S, A> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lens;
    use crate::optics::each::Each;
    use crate::optics::ixed::Ixed;

    #[derive(Clone, Debug, PartialEq)]
    struct Container {
        items: Vec<i32>,
    }

    // =========================================================================
    // Lens + Optional Tests
    // =========================================================================

    #[test]
    fn test_lens_compose_optional_get_option() {
        let items_lens = lens!(Container, items);
        let first_item = <Vec<i32> as Ixed<usize>>::ix(0);
        let composed = items_lens.compose_optional(first_item);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(composed.get_option(&container), Some(&1));
    }

    #[test]
    fn test_lens_compose_optional_get_option_empty() {
        let items_lens = lens!(Container, items);
        let first_item = <Vec<i32> as Ixed<usize>>::ix(0);
        let composed = items_lens.compose_optional(first_item);

        let container = Container { items: vec![] };
        assert_eq!(composed.get_option(&container), None);
    }

    #[test]
    fn test_lens_compose_optional_set() {
        let items_lens = lens!(Container, items);
        let first_item = <Vec<i32> as Ixed<usize>>::ix(0);
        let composed = items_lens.compose_optional(first_item);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let updated = composed.set(container, 100);
        assert_eq!(updated.items, vec![100, 2, 3]);
    }

    #[test]
    fn test_lens_compose_optional_clone() {
        let items_lens = lens!(Container, items);
        let first_item = <Vec<i32> as Ixed<usize>>::ix(0);
        let composed = items_lens.compose_optional(first_item);
        let cloned = composed.clone();

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(cloned.get_option(&container), Some(&1));
    }

    // =========================================================================
    // Lens + Traversal Tests
    // =========================================================================

    #[test]
    fn test_lens_compose_traversal_get_all() {
        let items_lens = lens!(Container, items);
        let all_items = <Vec<i32> as Each>::each();
        let composed = items_lens.compose_traversal(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let sum: i32 = composed.get_all(&container).sum();
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_lens_compose_traversal_get_all_owned() {
        let items_lens = lens!(Container, items);
        let all_items = <Vec<i32> as Each>::each();
        let composed = items_lens.compose_traversal(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let result = composed.get_all_owned(container);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_lens_compose_traversal_modify_all() {
        let items_lens = lens!(Container, items);
        let all_items = <Vec<i32> as Each>::each();
        let composed = items_lens.compose_traversal(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let modified = composed.modify_all(container, |x| x * 2);
        assert_eq!(modified.items, vec![2, 4, 6]);
    }

    #[test]
    fn test_lens_compose_traversal_clone() {
        let items_lens = lens!(Container, items);
        let all_items = <Vec<i32> as Each>::each();
        let composed = items_lens.compose_traversal(all_items);
        let cloned = composed.clone();

        let container = Container {
            items: vec![1, 2, 3],
        };
        let sum: i32 = cloned.get_all(&container).sum();
        assert_eq!(sum, 6);
    }

    // =========================================================================
    // Lens + Fold Tests
    // =========================================================================

    #[allow(clippy::type_complexity)]
    fn vec_fold<T: 'static>() -> crate::optics::FunctionFold<
        Vec<T>,
        T,
        impl for<'a> Fn(&'a Vec<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> + Clone,
    > {
        crate::optics::FunctionFold::new(|vec: &Vec<T>| Box::new(vec.iter()))
    }

    #[test]
    fn test_lens_compose_fold_get_all() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let result: Vec<&i32> = composed.get_all(&container).collect();
        assert_eq!(result, vec![&1, &2, &3]);
    }

    #[test]
    fn test_lens_compose_fold_fold() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let sum = composed.fold(&container, 0, |acc, x| acc + x);
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_lens_compose_fold_length() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(composed.length(&container), 3);
    }

    #[test]
    fn test_lens_compose_fold_clone() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);
        let cloned = composed.clone();

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(cloned.length(&container), 3);
    }

    // =========================================================================
    // Optional + Optional Tests
    // =========================================================================

    #[test]
    fn test_optional_compose_optional_get_option() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let first_of_first = <Vec<i32> as Ixed<usize>>::ix(0);

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_optional(first_of_first);

        let nested = Nested {
            outer: vec![vec![1, 2], vec![3, 4]],
        };
        assert_eq!(composed.get_option(&nested), Some(&1));
    }

    #[test]
    fn test_optional_compose_optional_get_option_none() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let first_of_first = <Vec<i32> as Ixed<usize>>::ix(0);

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_optional(first_of_first);

        let nested = Nested {
            outer: vec![vec![]],
        };
        assert_eq!(composed.get_option(&nested), None);
    }

    #[test]
    fn test_optional_compose_optional_set() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let first_of_first = <Vec<i32> as Ixed<usize>>::ix(0);

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_optional(first_of_first);

        let nested = Nested {
            outer: vec![vec![1, 2], vec![3, 4]],
        };
        let updated = composed.set(nested, 100);
        assert_eq!(updated.outer, vec![vec![100, 2], vec![3, 4]]);
    }

    #[test]
    fn test_optional_compose_optional_clone() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let first_of_first = <Vec<i32> as Ixed<usize>>::ix(0);

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_optional(first_of_first);
        let cloned = composed.clone();

        let nested = Nested {
            outer: vec![vec![1, 2], vec![3, 4]],
        };
        assert_eq!(cloned.get_option(&nested), Some(&1));
    }

    // =========================================================================
    // Optional + Traversal Tests
    // =========================================================================

    #[test]
    fn test_optional_compose_traversal_get_all() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested {
            outer: vec![vec![1, 2, 3], vec![4, 5]],
        };
        let result: Vec<&i32> = composed.get_all(&nested).collect();
        assert_eq!(result, vec![&1, &2, &3]);
    }

    #[test]
    fn test_optional_compose_traversal_get_all_empty() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested { outer: vec![] };
        let result: Vec<&i32> = composed.get_all(&nested).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_optional_compose_traversal_modify_all() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested {
            outer: vec![vec![1, 2, 3], vec![4, 5]],
        };
        let modified = composed.modify_all(nested, |x| x * 10);
        assert_eq!(modified.outer, vec![vec![10, 20, 30], vec![4, 5]]);
    }

    #[test]
    fn test_optional_compose_traversal_clone() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);
        let cloned = composed.clone();

        let nested = Nested {
            outer: vec![vec![1, 2, 3], vec![4, 5]],
        };
        let result: Vec<&i32> = cloned.get_all(&nested).collect();
        assert_eq!(result, vec![&1, &2, &3]);
    }

    // =========================================================================
    // Traversal + Fold Tests
    // =========================================================================

    #[allow(clippy::type_complexity)]
    fn nested_vec_fold<T: 'static>() -> crate::optics::FunctionFold<
        Vec<T>,
        T,
        impl for<'a> Fn(&'a Vec<T>) -> Box<dyn Iterator<Item = &'a T> + 'a> + Clone,
    > {
        crate::optics::FunctionFold::new(|vec: &Vec<T>| Box::new(vec.iter()))
    }

    #[test]
    fn test_traversal_compose_fold_get_all() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        let result: Vec<&i32> = composed.get_all(&nested).collect();
        assert_eq!(result, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn test_traversal_compose_fold_fold() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        let sum = composed.fold(&nested, 0, |acc, x| acc + x);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_traversal_compose_fold_length() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert_eq!(composed.length(&nested), 5);
    }

    #[test]
    fn test_traversal_compose_fold_for_all() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let all_positive = vec![vec![1, 2], vec![3, 4, 5]];
        assert!(composed.for_all(&all_positive, |x| *x > 0));

        let has_negative = vec![vec![1, -2], vec![3, 4]];
        assert!(!composed.for_all(&has_negative, |x| *x > 0));
    }

    #[test]
    fn test_traversal_compose_fold_exists() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert!(composed.exists(&nested, |x| *x == 3));
        assert!(!composed.exists(&nested, |x| *x == 100));
    }

    #[test]
    fn test_traversal_compose_fold_head_option() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert_eq!(composed.head_option(&nested), Some(&1));

        let empty: Vec<Vec<i32>> = vec![];
        assert_eq!(composed.head_option(&empty), None);
    }

    #[test]
    fn test_traversal_compose_fold_last_option() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert_eq!(composed.last_option(&nested), Some(&5));

        let empty: Vec<Vec<i32>> = vec![];
        assert_eq!(composed.last_option(&empty), None);
    }

    #[test]
    fn test_traversal_compose_fold_is_empty() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert!(!composed.is_empty(&nested));

        let empty: Vec<Vec<i32>> = vec![];
        assert!(composed.is_empty(&empty));

        let all_empty = vec![vec![], vec![]];
        assert!(composed.is_empty(&all_empty));
    }

    #[test]
    fn test_traversal_compose_fold_clone() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);
        let cloned = composed.clone();

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        assert_eq!(cloned.length(&nested), 5);
    }

    #[test]
    fn test_traversal_compose_fold_to_vec() {
        let all_vecs = <Vec<Vec<i32>> as Each>::each();
        let all_items = nested_vec_fold::<i32>();
        let composed = all_vecs.compose_fold(all_items);

        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        let result = composed.to_vec(&nested);
        assert_eq!(result, vec![&1, &2, &3, &4, &5]);
    }

    // =========================================================================
    // LensFoldComposition Additional Tests
    // =========================================================================

    #[test]
    fn test_lens_compose_fold_for_all() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let all_positive = Container {
            items: vec![1, 2, 3],
        };
        assert!(composed.for_all(&all_positive, |x| *x > 0));

        let has_negative = Container {
            items: vec![1, -2, 3],
        };
        assert!(!composed.for_all(&has_negative, |x| *x > 0));
    }

    #[test]
    fn test_lens_compose_fold_exists() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert!(composed.exists(&container, |x| *x == 2));
        assert!(!composed.exists(&container, |x| *x == 100));
    }

    #[test]
    fn test_lens_compose_fold_head_option() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(composed.head_option(&container), Some(&1));

        let empty_container = Container { items: vec![] };
        assert_eq!(composed.head_option(&empty_container), None);
    }

    #[test]
    fn test_lens_compose_fold_last_option() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(composed.last_option(&container), Some(&3));

        let empty_container = Container { items: vec![] };
        assert_eq!(composed.last_option(&empty_container), None);
    }

    #[test]
    fn test_lens_compose_fold_is_empty() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert!(!composed.is_empty(&container));

        let empty_container = Container { items: vec![] };
        assert!(composed.is_empty(&empty_container));
    }

    #[test]
    fn test_lens_compose_fold_to_vec() {
        let items_lens = lens!(Container, items);
        let all_items = vec_fold::<i32>();
        let composed = items_lens.compose_fold(all_items);

        let container = Container {
            items: vec![1, 2, 3],
        };
        let result = composed.to_vec(&container);
        assert_eq!(result, vec![&1, &2, &3]);
    }

    // =========================================================================
    // OptionalOptionalComposition set when first optional is None
    // =========================================================================

    #[test]
    fn test_optional_compose_optional_set_when_first_optional_is_none() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let first_of_first = <Vec<i32> as Ixed<usize>>::ix(0);

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_optional(first_of_first);

        let nested = Nested { outer: vec![] };
        let updated = composed.set(nested.clone(), 100);
        assert_eq!(updated, nested);
    }

    // =========================================================================
    // OptionalTraversalComposition get_all_owned and modify_all when None
    // =========================================================================

    #[test]
    fn test_optional_compose_traversal_get_all_owned() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested {
            outer: vec![vec![1, 2, 3], vec![4, 5]],
        };
        let result = composed.get_all_owned(nested);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_optional_compose_traversal_get_all_owned_when_none() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested { outer: vec![] };
        let result = composed.get_all_owned(nested);
        assert!(result.is_empty());
    }

    #[test]
    fn test_optional_compose_traversal_modify_all_when_none() {
        #[derive(Clone, Debug, PartialEq)]
        struct Nested {
            outer: Vec<Vec<i32>>,
        }

        let outer_lens = lens!(Nested, outer);
        let first_vec = <Vec<Vec<i32>> as Ixed<usize>>::ix(0);
        let all_items = <Vec<i32> as Each>::each();

        let outer_optional = outer_lens.compose_optional(first_vec);
        let composed = outer_optional.compose_traversal(all_items);

        let nested = Nested { outer: vec![] };
        let modified = composed.modify_all(nested.clone(), |x| x * 10);
        assert_eq!(modified, nested);
    }
}
