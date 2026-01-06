//! Effect row types for composing multiple effects.
//!
//! This module provides type-level lists for representing compositions
//! of multiple effects. The [`EffNil`] type represents an empty effect row,
//! and [`EffCons`] constructs a row by prepending an effect to an existing row.
//!
//! # Type-Level List Pattern
//!
//! Effect rows use the `HList` (heterogeneous list) pattern to represent
//! ordered collections of effects at the type level:
//!
//! - `EffNil` - Empty row (base case)
//! - `EffCons<E, Tail>` - Row with effect `E` prepended to `Tail`
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{EffNil, EffCons, Effect};
//! use lambars::effect::algebraic::{ReaderEffect, StateEffect};
//!
//! // Single effect row
//! type SingleRow = EffCons<ReaderEffect<i32>, EffNil>;
//!
//! // Multiple effects row
//! type MultiRow = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
//!
//! // Using the EffectRow! macro (more convenient)
//! use lambars::EffectRow;
//! type MacroRow = EffectRow![ReaderEffect<i32>, StateEffect<String>];
//! ```

use super::effect::Effect;
use std::marker::PhantomData;

/// An empty effect row.
///
/// `EffNil` represents a row containing no effects. It serves as the
/// base case for the type-level list structure.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{EffNil, Effect};
///
/// assert_eq!(EffNil::NAME, "EffNil");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct EffNil;

impl Effect for EffNil {
    const NAME: &'static str = "EffNil";
}

/// A non-empty effect row with an effect prepended.
///
/// `EffCons<E, Tail>` represents a row where effect `E` is the first
/// element and `Tail` is the rest of the row.
///
/// # Type Parameters
///
/// - `E`: The effect at the head of this row
/// - `Tail`: The remaining effects (another row type)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{EffNil, EffCons, Effect, ReaderEffect};
///
/// // A row containing only ReaderEffect<i32>
/// type SingleRow = EffCons<ReaderEffect<i32>, EffNil>;
///
/// // Effect row implements Effect trait
/// fn assert_is_effect<T: Effect>() {}
/// assert_is_effect::<SingleRow>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct EffCons<E: Effect, Tail: Effect> {
    _effect: PhantomData<E>,
    _tail: PhantomData<Tail>,
}

impl<E: Effect, Tail: Effect> EffCons<E, Tail> {
    /// Creates a new `EffCons` instance.
    ///
    /// This is primarily useful for documentation purposes since
    /// `EffCons` is a zero-sized type used only at the type level.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _effect: PhantomData,
            _tail: PhantomData,
        }
    }
}

impl<E: Effect, Tail: Effect> Effect for EffCons<E, Tail> {
    const NAME: &'static str = "EffCons";
}

/// Constructs an effect row from a list of effect types.
///
/// This macro provides a convenient syntax for building effect rows
/// without manually nesting `EffCons` types.
///
/// # Syntax
///
/// - `EffectRow![]` - Empty row (`EffNil`)
/// - `EffectRow![E]` - Single effect row (`EffCons<E, EffNil>`)
/// - `EffectRow![E1, E2, ...]` - Multiple effects row
///
/// # Examples
///
/// ```rust
/// use lambars::EffectRow;
/// use lambars::effect::algebraic::{ReaderEffect, StateEffect, WriterEffect, Effect};
///
/// // Empty row
/// type Empty = EffectRow![];
///
/// // Single effect
/// type Single = EffectRow![ReaderEffect<i32>];
///
/// // Multiple effects
/// type Multiple = EffectRow![ReaderEffect<i32>, StateEffect<String>];
///
/// // Three effects with trailing comma
/// type Three = EffectRow![
///     ReaderEffect<i32>,
///     StateEffect<String>,
///     WriterEffect<Vec<String>>,
/// ];
///
/// // All are valid Effect types
/// fn assert_effect<T: Effect>() {}
/// assert_effect::<Empty>();
/// assert_effect::<Single>();
/// assert_effect::<Multiple>();
/// assert_effect::<Three>();
/// ```
#[macro_export]
macro_rules! EffectRow {
    () => { $crate::effect::algebraic::EffNil };
    ($effect:ty) => {
        $crate::effect::algebraic::EffCons<$effect, $crate::effect::algebraic::EffNil>
    };
    ($effect:ty, $($rest:ty),+ $(,)?) => {
        $crate::effect::algebraic::EffCons<$effect, $crate::EffectRow!($($rest),+)>
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::algebraic::{ReaderEffect, StateEffect, WriterEffect};
    use rstest::rstest;

    #[rstest]
    fn eff_nil_name_is_correct() {
        assert_eq!(EffNil::NAME, "EffNil");
    }

    #[rstest]
    fn eff_nil_type_id_is_consistent() {
        let first = EffNil::type_id();
        let second = EffNil::type_id();
        assert_eq!(first, second);
    }

    #[rstest]
    fn eff_nil_is_debug() {
        let nil = EffNil;
        let debug_string = format!("{nil:?}");
        assert_eq!(debug_string, "EffNil");
    }

    #[rstest]
    fn eff_nil_is_clone() {
        let nil = EffNil;
        let cloned = nil;
        assert_eq!(nil, cloned);
    }

    #[rstest]
    fn eff_nil_is_copy() {
        let nil = EffNil;
        let copied = nil;
        assert_eq!(nil, copied);
    }

    #[rstest]
    fn eff_nil_is_eq() {
        let first = EffNil;
        let second = EffNil;
        assert_eq!(first, second);
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn eff_nil_is_default() {
        let default_nil = EffNil::default();
        assert_eq!(default_nil, EffNil);
    }

    #[rstest]
    fn eff_nil_is_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EffNil);
        assert!(set.contains(&EffNil));
    }

    #[rstest]
    fn eff_cons_name_is_correct() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        assert_eq!(Row::NAME, "EffCons");
    }

    #[rstest]
    fn eff_cons_new_creates_instance() {
        let cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let debug_string = format!("{cons:?}");
        assert!(debug_string.contains("EffCons"));
    }

    #[rstest]
    fn eff_cons_type_id_is_consistent() {
        type Row = EffCons<ReaderEffect<i32>, EffNil>;
        let first = Row::type_id();
        let second = Row::type_id();
        assert_eq!(first, second);
    }

    #[rstest]
    fn eff_cons_different_effects_have_different_type_ids() {
        type Row1 = EffCons<ReaderEffect<i32>, EffNil>;
        type Row2 = EffCons<StateEffect<i32>, EffNil>;
        assert_ne!(Row1::type_id(), Row2::type_id());
    }

    #[rstest]
    fn eff_cons_is_debug() {
        let cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let debug_string = format!("{cons:?}");
        assert!(debug_string.contains("EffCons"));
    }

    #[rstest]
    fn eff_cons_is_clone() {
        let cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let cloned = cons;
        assert_eq!(cons, cloned);
    }

    #[rstest]
    fn eff_cons_is_copy() {
        let cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let copied = cons;
        assert_eq!(cons, copied);
    }

    #[rstest]
    fn eff_cons_is_eq() {
        let first: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let second: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        assert_eq!(first, second);
    }

    #[rstest]
    fn eff_cons_is_default() {
        let default_cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::default();
        let explicit_cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        assert_eq!(default_cons, explicit_cons);
    }

    #[rstest]
    fn eff_cons_is_hash() {
        use std::collections::HashSet;
        let cons: EffCons<ReaderEffect<i32>, EffNil> = EffCons::new();
        let mut set = HashSet::new();
        set.insert(cons);
        assert!(set.contains(&EffCons::new()));
    }

    #[rstest]
    #[allow(clippy::items_after_statements)]
    fn eff_cons_nested_two_effects() {
        type Row = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        assert_eq!(Row::NAME, "EffCons");
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Row>();
    }

    #[rstest]
    fn eff_cons_nested_three_effects() {
        type Row = EffCons<
            ReaderEffect<i32>,
            EffCons<StateEffect<String>, EffCons<WriterEffect<Vec<String>>, EffNil>>,
        >;
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Row>();
    }

    // EffectRow! Macro Tests

    #[rstest]
    fn effect_row_macro_empty() {
        type Empty = EffectRow![];
        fn assert_is_eff_nil<T>()
        where
            T: Effect,
        {
        }
        assert_is_eff_nil::<Empty>();
        assert_eq!(Empty::NAME, "EffNil");
    }

    #[rstest]
    fn effect_row_macro_single() {
        type Single = EffectRow![ReaderEffect<i32>];
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Single>();
        assert_eq!(Single::NAME, "EffCons");
    }

    #[rstest]
    fn effect_row_macro_two_effects() {
        type Two = EffectRow![ReaderEffect<i32>, StateEffect<String>];
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Two>();
    }

    #[rstest]
    fn effect_row_macro_three_effects() {
        type Three = EffectRow![
            ReaderEffect<i32>,
            StateEffect<String>,
            WriterEffect<Vec<String>>
        ];
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Three>();
    }

    #[rstest]
    fn effect_row_macro_with_trailing_comma() {
        type WithTrailing = EffectRow![ReaderEffect<i32>, StateEffect<String>,];
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<WithTrailing>();
    }

    #[rstest]
    fn effect_row_macro_equivalent_to_manual() {
        type Manual = EffCons<ReaderEffect<i32>, EffCons<StateEffect<String>, EffNil>>;
        type Macro = EffectRow![ReaderEffect<i32>, StateEffect<String>];

        // Type IDs should be equal since they're the same type
        assert_eq!(Manual::type_id(), Macro::type_id());
    }

    #[rstest]
    fn eff_nil_implements_effect() {
        fn assert_effect<T: Effect>() {}
        assert_effect::<EffNil>();
    }

    #[rstest]
    fn eff_cons_implements_effect() {
        fn assert_effect<T: Effect>() {}
        assert_effect::<EffCons<ReaderEffect<i32>, EffNil>>();
    }

    #[rstest]
    #[allow(clippy::items_after_statements)]
    fn effect_row_with_generic_effects() {
        struct CustomEffect<T>(PhantomData<T>);
        impl<T: 'static> Effect for CustomEffect<T> {
            const NAME: &'static str = "Custom";
        }

        type Row = EffectRow![CustomEffect<i32>, CustomEffect<String>];
        fn assert_is_effect<T: Effect>() {}
        assert_is_effect::<Row>();
    }
}
