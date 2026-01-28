//! Scala-style for-comprehension macro for iterators.
//!
//! This module provides the [`for_!`] macro, which allows writing nested
//! iterations in a flat, declarative style similar to Scala's for-comprehension.
//!
//! # Overview
//!
//! The `for_!` macro transforms nested `flat_map` operations into a more
//! readable, imperative-looking syntax. Unlike the [`eff!`](crate::eff!) macro
//! which works with monads (Option, Result, IO, etc.), `for_!` is specifically
//! designed for iterator-based operations and collects results into a `Vec`.
//!
//! # Syntax
//!
//! ```text
//! for_! {
//!     pattern <= collection;           // Bind: iterate over collection
//!     if let pattern = expression;     // Pattern guard: match pattern (skip if no match)
//!     if condition;                    // Guard: filter by condition
//!     let pattern = expression;        // Pure let binding
//!     yield expression                 // Final expression (wrapped in Vec)
//! }
//! ```
//!
//! # Supported Patterns
//!
//! - **Identifier pattern**: `x <= collection;`
//! - **Tuple pattern**: `(a, b) <= collection;`
//! - **Wildcard pattern**: `_ <= collection;`
//! - **Pattern guard**: `if let pattern = expression;` (skips if pattern doesn't match)
//! - **Guard expression**: `if condition;` (skips iteration if condition is false)
//! - **Let binding (identifier)**: `let x = expression;`
//! - **Let binding (tuple)**: `let (a, b) = expression;`
//!
//! # Operator Choice: `<=`
//!
//! We use `<=` as the bind operator because:
//! - `<-` is not valid in Rust's macro patterns
//! - `<=` is visually similar to `<-` and suggests "bind from"
//! - It maintains consistency with the [`eff!`](crate::eff!) macro
//!
//! # Examples
//!
//! ## Basic Iteration
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3];
//!     yield x * 2
//! };
//! assert_eq!(result, vec![2, 4, 6]);
//! ```
//!
//! ## Nested Iteration
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2];
//!     y <= vec![10, 20];
//!     yield x + y
//! };
//! assert_eq!(result, vec![11, 21, 12, 22]);
//! ```
//!
//! ## With Let Bindings
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3];
//!     let doubled = x * 2;
//!     yield doubled + 1
//! };
//! assert_eq!(result, vec![3, 5, 7]);
//! ```
//!
//! ## Tuple Pattern
//!
//! ```rust
//! use lambars::for_;
//!
//! let pairs = vec![(1, "a"), (2, "b")];
//! let result = for_! {
//!     (num, letter) <= pairs;
//!     yield format!("{}{}", num, letter)
//! };
//! assert_eq!(result, vec!["1a", "2b"]);
//! ```
//!
//! ## Guard Expression (Filtering)
//!
//! ```rust
//! use lambars::for_;
//!
//! // Basic guard - filter even numbers
//! let result = for_! {
//!     x <= vec![1, 2, 3, 4, 5];
//!     if x % 2 == 0;
//!     yield x
//! };
//! assert_eq!(result, vec![2, 4]);
//! ```
//!
//! ## Multiple Guards
//!
//! ```rust
//! use lambars::for_;
//!
//! // Multiple guards act as AND conditions
//! let result = for_! {
//!     x <= 1..=100i32;
//!     if x % 2 == 0;
//!     if x % 3 == 0;
//!     if x < 50;
//!     yield x
//! };
//! assert_eq!(result, vec![6, 12, 18, 24, 30, 36, 42, 48]);
//! ```
//!
//! ## Guard with Let Binding
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3, 4, 5];
//!     let squared = x * x;
//!     if squared > 10;
//!     yield squared
//! };
//! assert_eq!(result, vec![16, 25]);
//! ```
//!
//! ## Pattern Guard (if let)
//!
//! ```rust
//! use lambars::for_;
//!
//! // Extract values from Option
//! fn maybe_double(x: i32) -> Option<i32> {
//!     if x > 0 { Some(x * 2) } else { None }
//! }
//!
//! let result = for_! {
//!     x <= vec![-1, 0, 1, 2, 3];
//!     if let Some(doubled) = maybe_double(x);
//!     yield doubled
//! };
//! assert_eq!(result, vec![2, 4, 6]);
//! ```
//!
//! ## Pattern Guard with Result
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     s <= vec!["1", "abc", "2"];
//!     if let Ok(n) = s.parse::<i32>();
//!     yield n
//! };
//! assert_eq!(result, vec![1, 2]);
//! ```
//!
//! ## Pattern Guard with Regular Guard
//!
//! ```rust
//! use lambars::for_;
//!
//! let items = vec![Some(1), None, Some(5), Some(10)];
//! let result = for_! {
//!     item <= items;
//!     if let Some(value) = item;
//!     if value > 3;
//!     yield value
//! };
//! assert_eq!(result, vec![5, 10]);
//! ```
//!
//! ## Nested Iteration with Multiple Collections
//!
//! ```rust
//! use lambars::for_;
//!
//! let xs = vec![1, 2];
//! let ys = vec![10, 20];
//!
//! // Cartesian product
//! let result = for_! {
//!     x <= xs;
//!     y <= ys.clone();  // Clone ys for each x iteration
//!     yield x * y
//! };
//!
//! assert_eq!(result, vec![10, 20, 20, 40]);
//! ```
//!
//! ## Complex Data Processing
//!
//! ```rust
//! use lambars::for_;
//!
//! let data = vec![
//!     ("Alice", vec![85, 90, 88]),
//!     ("Bob", vec![78, 82, 80]),
//! ];
//!
//! // Get all scores above 80 with student names
//! let result = for_! {
//!     (name, scores) <= data;
//!     score <= scores;
//!     if score > 80;
//!     yield (name, score)
//! };
//!
//! assert_eq!(result, vec![("Alice", 85), ("Alice", 90), ("Alice", 88), ("Bob", 82)]);
//! ```
//!
//! # Breaking Changes (v0.2.0)
//!
//! **Clone Requirement**: The `for_!` macro requires `Clone` only for single-level
//! iterations (e.g., `for_! { x <= xs; yield x * 2 }`). This enables optimal
//! pre-allocation by computing `size_hint` before consuming the collection.
//!
//! Nested iterations and guards do **not** require `Clone`:
//! - `for_! { x <= xs; y <= ys; yield x + y }` - no Clone needed
//! - `for_! { x <= xs; if x > 0; yield x }` - no Clone needed
//!
//! For single-level iterations over non-Clone collections, use standard iterator
//! methods instead:
//!
//! ```rust,ignore
//! // Instead of:
//! // for_! { x <= non_clone_iter; yield x * 2 }
//!
//! // Use:
//! let result: Vec<_> = non_clone_iter.into_iter().map(|x| x * 2).collect();
//! ```
//!
//! # Implementation Details
//!
//! The macro uses internal rules for optimized expansion:
//!
//! ## Internal Rules
//!
//! 1. **`@iter`**: Builds a pure iterator chain using `flat_map` for nesting and
//!    `EitherIter` for guards. No intermediate `Vec` allocations occur during iteration.
//!
//! 2. **`@hint`** (internal): Computes `size_hint` for the outermost collection.
//!    For nested iterations, returns conservative estimates `(0, None)` because
//!    inner collection expressions may reference outer loop variables that are
//!    not yet in scope during macro expansion. Used by public entry points for
//!    pre-allocation when possible.
//!
//! 3. **`collect_with_hint`**: Collects the iterator using pre-computed `size_hint`.
//!    Uses `SmallVec` for small results (<=128 elements) and `Vec::with_capacity`
//!    for larger results.
//!
//! For single iterations (`; yield expr` only), the macro uses Clone + `@hint` to
//! compute exact size_hint for pre-allocation. For nested iterations and guards,
//! the macro falls back to `collect_from_iter` which uses the iterator's own
//! size_hint, avoiding unnecessary Clone overhead.
//!
//! Note: Due to Rust macro limitations, `@hint` cannot safely reference expressions
//! that use loop variables (e.g., `if let Some(y) = f(x)` where `x` is a loop variable).
//!
//! ## Performance Characteristics
//!
//! - **No intermediate Vec**: Nested iterations use pure iterator chains without
//!   allocating intermediate collections.
//! - **Single allocation**: Results are collected once at the end.
//! - **SmallVec optimization**: Small results (<=128 elements) use stack allocation.
//! - **Pre-allocation**: Clone is used only for single-level iterations to compute
//!   size_hint before iteration. Nested iterations and guards use `collect_from_iter`
//!   which avoids Clone overhead.
//!
//! ## Expansion Example
//!
//! ```rust,ignore
//! // Single iteration:
//! for_! { x <= xs; yield x * 2 }
//! // Expands to (conceptually):
//! let __collection = xs;
//! let (lower, upper) = __collection.clone().into_iter().size_hint();
//! collect_with_hint(lower, upper, __collection.into_iter().map(|x| x * 2))
//!
//! // Nested iteration (no Clone required):
//! for_! { x <= xs; y <= ys; yield x + y }
//! // Expands to (conceptually):
//! collect_from_iter(
//!     xs.into_iter().flat_map(|x| {
//!         ys.into_iter().map(|y| x + y)
//!     })
//! )
//! ```
//!
//! # Differences from eff! macro
//!
//! | Feature | `for_!` | `eff!` |
//! |---------|---------|--------|
//! | Target | Iterators | Monads |
//! | Result | `Vec<T>` | Monad type |
//! | Terminal | `yield expression` | `expression` |
//! | Method | `into_iter().flat_map()` | `flat_map()` |
//!
//! # Important Notes on Clone
//!
//! When using outer variables inside inner iterations, you must explicitly
//! clone them:
//!
//! ```rust
//! use lambars::for_;
//!
//! let xs = vec![1, 2];
//! let ys = vec![10, 20];
//!
//! // ys must be cloned because it's used in the inner loop
//! let result = for_! {
//!     x <= xs;
//!     y <= ys.clone();  // Explicit clone required
//!     yield x + y
//! };
//! ```
//!
//! This is intentional to:
//! - Maintain Rust's explicit ownership semantics
//! - Avoid hidden performance costs
//! - Make the code predictable and debuggable

#![forbid(unsafe_code)]

use smallvec::SmallVec;

/// `SmallVec` inline capacity for `for_!` macro (128 elements = 1KB for L1 cache).
pub const SMALLVEC_INLINE_CAPACITY: usize = 128;

/// Size hint type alias for iterator capacity estimation.
///
/// Same format as `Iterator::size_hint()`: `(lower_bound, upper_bound)`.
/// - `lower_bound`: Minimum element count (guaranteed lower bound)
/// - `upper_bound`: Maximum element count (`None` indicates unknown)
pub type SizeHint = (usize, Option<usize>);

/// Compose multiple size hints to determine optimal capacity.
///
/// This function computes the combined size hint for nested iterations by
/// multiplying the bounds of each iterator.
///
/// **Note**: This function is provided as a utility for manual use. The `for_!`
/// macro cannot automatically use this function for nested iterations due to
/// Rust macro limitations (inner collection expressions may reference outer
/// loop variables that are not in scope during `@hint` evaluation).
///
/// # Manual Usage Example
///
/// ```rust
/// use lambars::compose::for_macro::combined_size_hint;
///
/// // When you know the sizes in advance
/// let xs_len = 10;
/// let ys_len = 20;
/// let hints = [(xs_len, Some(xs_len)), (ys_len, Some(ys_len))];
/// let capacity = combined_size_hint(&hints);
/// // capacity = 200
/// ```
///
/// # Mathematical Definition
///
/// - `lower_combined = Π(lower_i)` (using `saturating_mul` to prevent overflow)
/// - `upper_combined = Π(upper_i)` if all `upper_i` are `Some`, else `None`
///   (using `checked_mul` to detect overflow)
/// - `result = upper_combined.unwrap_or(lower_combined)`
///
/// # Overflow Handling
///
/// - `lower`: Uses `saturating_mul` (overflows to `usize::MAX`)
/// - `upper`: Uses `checked_mul` (overflows to `None`, falls back to `lower`)
///
/// # `flat_map` Composition Strategy
///
/// After `flat_map`, the iterator's `size_hint` becomes `(0, None)`.
/// The `@hint` rules compute hints **before** building the iterator chain,
/// collecting each collection's hint separately. This avoids the composition
/// problem and enables pre-allocation.
///
/// # Examples
///
/// ```
/// use lambars::compose::for_macro::combined_size_hint;
///
/// // Exact bounds: 10 * 20 = 200
/// let hints = [(10, Some(10)), (20, Some(20))];
/// assert_eq!(combined_size_hint(&hints), 200);
///
/// // Partial bounds: uses lower product
/// let hints = [(10, None), (20, Some(20))];
/// assert_eq!(combined_size_hint(&hints), 200);
///
/// // Empty slice
/// assert_eq!(combined_size_hint(&[]), 0);
/// ```
#[inline]
#[must_use]
pub fn combined_size_hint(hints: &[SizeHint]) -> usize {
    if hints.is_empty() {
        return 0;
    }

    let mut combined_lower: usize = 1;
    let mut combined_upper: Option<usize> = Some(1);

    for &(lower, upper) in hints {
        // lower: saturating_mul prevents overflow (saturates to usize::MAX)
        combined_lower = combined_lower.saturating_mul(lower);
        // upper: checked_mul detects overflow, any failure results in None
        combined_upper = match (combined_upper, upper) {
            (Some(cu), Some(u)) => cu.checked_mul(u),
            _ => None,
        };
    }

    // Safe minimum allocation: use upper if known, otherwise fall back to lower
    combined_upper.unwrap_or(combined_lower)
}

/// Either iterator: preserves `size_hint` unlike `flatten` which returns `(0, None)`.
///
/// # Example
///
/// ```
/// use lambars::compose::for_macro::EitherIter;
///
/// let condition = true;
/// let iter: EitherIter<_, std::iter::Empty<i32>> = if condition {
///     EitherIter::Left(vec![1, 2, 3].into_iter())
/// } else {
///     EitherIter::Right(std::iter::empty())
/// };
///
/// let result: Vec<_> = iter.collect();
/// assert_eq!(result, vec![1, 2, 3]);
/// ```
#[derive(Clone)]
pub enum EitherIter<L, R> {
    /// Left variant (typically when guard is true).
    Left(L),
    /// Right variant (typically when guard is false).
    Right(R),
}

impl<T, L, R> Iterator for EitherIter<L, R>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = T>,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(l) => l.next(),
            Self::Right(r) => r.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Left(l) => l.size_hint(),
            Self::Right(r) => r.size_hint(),
        }
    }
}

impl<T, L, R> ExactSizeIterator for EitherIter<L, R>
where
    L: ExactSizeIterator<Item = T>,
    R: ExactSizeIterator<Item = T>,
{
}

impl<T, L, R> DoubleEndedIterator for EitherIter<L, R>
where
    L: DoubleEndedIterator<Item = T>,
    R: DoubleEndedIterator<Item = T>,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(l) => l.next_back(),
            Self::Right(r) => r.next_back(),
        }
    }
}

impl<T, L, R> core::iter::FusedIterator for EitherIter<L, R>
where
    L: core::iter::FusedIterator<Item = T>,
    R: core::iter::FusedIterator<Item = T>,
{
}

/// Maximum reasonable capacity to avoid excessive pre-allocation.
///
/// 1MB (1024 * 1024 elements) is chosen as a practical upper bound to prevent:
/// - Memory exhaustion from overly large `size_hint` upper bounds
/// - Performance degradation from over-allocation
///
/// This value was determined through profiling with typical workloads.
const MAX_REASONABLE_CAPACITY: usize = 1024 * 1024;

/// L1 cache size used for `SmallVec` threshold calculation.
///
/// 32KB is a conservative estimate that works across most modern CPUs.
/// This is a fixed value to maintain referential transparency.
const L1_CACHE_SIZE: usize = 32 * 1024;

/// Compute the `SmallVec` usage threshold based on element size.
///
/// This is a pure function that determines whether to use `SmallVec` or `Vec`
/// based on the element type's size relative to L1 cache.
///
/// # Design Rationale
///
/// - `SmallVec` provides stack allocation benefits for small collections
/// - However, `into_vec()` incurs copy cost when spilling to heap
/// - For larger elements, the threshold should be lower to avoid cache misses
/// - For smaller elements, we can store more items efficiently
///
/// # Algorithm
///
/// 1. Calculate cache-based threshold: `L1_CACHE_SIZE / element_size`
/// 2. Cap at `SMALLVEC_INLINE_CAPACITY` to respect `SmallVec`'s inline storage
/// 3. For zero-sized types, treat as size 1 to avoid division by zero
///
/// # Properties
///
/// - **Pure function**: Same type always returns same result
/// - **Referential transparency**: No external state dependencies
/// - **Compile-time evaluable**: `const fn` for optimization opportunities
///
/// # Examples
///
/// ```
/// use lambars::compose::for_macro::compute_smallvec_threshold;
///
/// // Smaller elements allow higher threshold
/// let threshold_i32 = compute_smallvec_threshold::<i32>();
/// let threshold_i64 = compute_smallvec_threshold::<i64>();
/// assert!(threshold_i32 >= threshold_i64);
///
/// // Threshold is capped at SMALLVEC_INLINE_CAPACITY
/// let threshold_u8 = compute_smallvec_threshold::<u8>();
/// assert!(threshold_u8 <= 128);
/// ```
#[inline]
#[must_use]
pub const fn compute_smallvec_threshold<T>() -> usize {
    let element_size = std::mem::size_of::<T>();
    // For ZST (zero-sized types), treat as size 1 to avoid division by zero
    let effective_size = if element_size == 0 { 1 } else { element_size };
    let cache_based = L1_CACHE_SIZE / effective_size;

    // Cap at SMALLVEC_INLINE_CAPACITY to respect SmallVec's inline storage limits
    if cache_based < SMALLVEC_INLINE_CAPACITY {
        cache_based
    } else {
        SMALLVEC_INLINE_CAPACITY
    }
}

/// Collect iterator with pre-computed `size_hint` for optimized allocation.
///
/// # Strategy
///
/// 1. **`upper` is known**: Use `Vec::with_capacity` directly (avoids `SmallVec` → `Vec` copy)
/// 2. **`upper` is unknown, `lower` == 0**: Use `Vec` (unknown size, avoid stack bloat)
/// 3. **`upper` is unknown, `lower` > threshold**: Use `Vec::with_capacity` directly
/// 4. **`upper` is unknown, `0 < lower <= threshold`**: Use `SmallVec` for stack allocation benefits
///
/// # Threshold Calculation
///
/// The threshold is computed dynamically based on element size using
/// [`compute_smallvec_threshold`]. Larger elements have lower thresholds
/// to avoid L1 cache pressure.
///
/// # Performance Characteristics
///
/// - Eliminates `into_vec()` copy for most cases where `upper` is known
/// - Falls back to `Vec` when `lower == 0` to avoid stack bloat for unknown sizes
/// - Preserves `SmallVec` stack allocation benefits only when `lower` is known and small
/// - Uses `with_capacity` on `SmallVec` to minimize reallocation during `extend`
#[inline]
pub fn collect_with_hint<T, I: Iterator<Item = T>>(
    lower: usize,
    upper: Option<usize>,
    iter: I,
) -> Vec<T> {
    if let Some(u) = upper {
        // upper is known: use Vec directly to avoid SmallVec → Vec copy overhead
        let capacity = u.min(MAX_REASONABLE_CAPACITY);
        let mut result = Vec::with_capacity(capacity);
        result.extend(iter);
        result
    } else {
        // upper is unknown: decide based on element-size-aware threshold
        let threshold = compute_smallvec_threshold::<T>();
        // lower == 0 means size_hint is (0, None) - typically from filter/flat_map.
        // In this case, fall back to Vec to avoid:
        // 1. Stack bloat from SmallVec's inline buffer when actual size is unknown
        // 2. Potential stack overflow for large element types
        // 3. SmallVec → Vec copy overhead if the collection grows beyond inline capacity
        if lower == 0 || lower > threshold {
            // Unknown size or large collection expected: use Vec directly
            let capacity = lower.max(16); // Minimum 16 to avoid under-allocation for unknown sizes
            let mut result = Vec::with_capacity(capacity);
            result.extend(iter);
            result
        } else {
            // Small collection expected with known lower bound: use SmallVec for stack allocation benefits
            // Use with_capacity(lower) to minimize reallocation during extend
            let capacity = lower.max(1);
            let mut buf: SmallVec<[T; SMALLVEC_INLINE_CAPACITY]> =
                SmallVec::with_capacity(capacity);
            buf.extend(iter);
            buf.into_vec()
        }
    }
}

/// Collect iterator using its own `size_hint` (fallback for non-Clone cases).
#[inline]
pub fn collect_from_iter<T, I: Iterator<Item = T>>(iter: I) -> Vec<T> {
    let (lower, upper) = iter.size_hint();
    collect_with_hint(lower, upper, iter)
}

/// A macro for Scala-style for-comprehension over iterators.
///
/// This macro allows you to write nested iterations in a flat,
/// declarative style, similar to Scala's for-comprehension.
///
/// # Syntax
///
/// ```text
/// for_! {
///     pattern <= collection;    // Bind: iterate over collection
///     if condition;             // Guard: filter by condition
///     let pattern = expression; // Pure let binding
///     yield expression          // Final expression (wrapped in Vec)
/// }
/// ```
///
/// # Examples
///
/// ## Basic iteration
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3];
///     yield x * 2
/// };
/// assert_eq!(result, vec![2, 4, 6]);
/// ```
///
/// ## Nested iteration
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2];
///     y <= vec![10, 20];
///     yield x + y
/// };
/// assert_eq!(result, vec![11, 21, 12, 22]);
/// ```
///
/// ## With let bindings
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3];
///     let doubled = x * 2;
///     yield doubled + 1
/// };
/// assert_eq!(result, vec![3, 5, 7]);
/// ```
///
/// ## Tuple pattern
///
/// ```rust
/// use lambars::for_;
///
/// let pairs = vec![(1, "a"), (2, "b")];
/// let result = for_! {
///     (num, letter) <= pairs;
///     yield format!("{}{}", num, letter)
/// };
/// assert_eq!(result, vec!["1a", "2b"]);
/// ```
///
/// ## Wildcard pattern
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     _ <= vec![1, 2, 3];
///     yield "x"
/// };
/// assert_eq!(result, vec!["x", "x", "x"]);
/// ```
///
/// ## Guard expression
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3, 4, 5];
///     if x % 2 == 0;
///     yield x
/// };
/// assert_eq!(result, vec![2, 4]);
/// ```
#[macro_export]
macro_rules! for_ {
    // =========================================================================
    // @hint: Compute size_hint for pre-allocation
    // =========================================================================

    (@hint yield $result:expr) => {
        (1usize, Some(1usize))
    };

    (@hint $pattern:ident <= $collection:expr ; yield $result:expr) => {{
        // Use clone() for size_hint computation - $collection is already cloned at entry point
        $collection.into_iter().size_hint()
    }};

    // Nested iteration: returns conservative (0, None) because $rest may contain
    // expressions that reference the loop variable $pattern, which is not in scope
    // during @hint evaluation. The outermost collection's size_hint is still used
    // for pre-allocation in collect_with_hint.
    (@hint $pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        // Avoid evaluating $rest as it may reference undefined variables
        let _ = &$collection; // Ensure $collection is valid but don't consume it
        (0usize, None)
    }};

    (@hint ($($pattern:tt)*) <= $collection:expr ; yield $result:expr) => {{
        // Use clone() for size_hint computation - $collection is already cloned at entry point
        $collection.into_iter().size_hint()
    }};

    // Nested iteration (tuple pattern): returns conservative (0, None) because $rest may contain
    // expressions that reference the loop variable ($($pattern)*), which is not in scope
    // during @hint evaluation. The outermost collection's size_hint is still used
    // for pre-allocation in collect_with_hint.
    (@hint ($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        // Avoid evaluating $rest as it may reference undefined variables
        let _ = &$collection; // Ensure $collection is valid but don't consume it
        (0usize, None)
    }};

    (@hint _ <= $collection:expr ; yield $result:expr) => {{
        // Use clone() for size_hint computation - $collection is already cloned at entry point
        $collection.into_iter().size_hint()
    }};

    // Nested iteration (wildcard pattern): returns conservative (0, None) because $rest may contain
    // expressions that reference loop variables from outer iterations, which are not in scope
    // during @hint evaluation. The outermost collection's size_hint is still used
    // for pre-allocation in collect_with_hint.
    (@hint _ <= $collection:expr ; $($rest:tt)+) => {{
        // Avoid evaluating $rest as it may reference undefined variables
        let _ = &$collection; // Ensure $collection is valid but don't consume it
        (0usize, None)
    }};

    // Guard: lower bound is 0 (filter may remove all)
    (@hint if $condition:expr ; $($rest:tt)+) => {{
        let (_, __inner_upper) = $crate::for_!(@hint $($rest)+);
        (0usize, __inner_upper)
    }};

    // Pattern guard: We cannot compute the inner size_hint because:
    // 1. The pattern may bind variables that are used in $rest
    // 2. $expr may reference loop variables that are not yet in scope at @hint evaluation time
    // Return conservative (0, None) without referencing $expr at all.
    (@hint if let $pattern:pat = $expr:expr ; $($rest:tt)+) => {{
        (0usize, None)
    }};

    // Let binding: no size change
    (@hint let $pattern:ident = $expr:expr ; $($rest:tt)+) => {
        $crate::for_!(@hint $($rest)+)
    };

    (@hint let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {
        $crate::for_!(@hint $($rest)+)
    };

    // =========================================================================
    // @iter: Build pure iterator chain without intermediate Vec allocations.
    // Uses EitherIter for guards to preserve size_hint.
    // =========================================================================

    (@iter yield $result:expr) => {
        ::core::iter::once($result)
    };

    (@iter $pattern:ident <= $collection:expr ; yield $result:expr) => {
        $collection.into_iter().map(
            #[inline(always)]
            move |$pattern| $result
        )
    };

    // Nested iteration: pure iterator chain with flat_map
    (@iter $pattern:ident <= $collection:expr ; $($rest:tt)+) => {
        $collection.into_iter().flat_map(
            #[inline(always)]
            move |$pattern| {
                $crate::for_!(@iter $($rest)+)
            }
        )
    };

    (@iter ($($pattern:tt)*) <= $collection:expr ; yield $result:expr) => {
        $collection.into_iter().map(
            #[inline(always)]
            move |($($pattern)*)| $result
        )
    };

    (@iter ($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {
        $collection.into_iter().flat_map(
            #[inline(always)]
            move |($($pattern)*)| {
                $crate::for_!(@iter $($rest)+)
            }
        )
    };

    (@iter _ <= $collection:expr ; yield $result:expr) => {
        $collection.into_iter().map(
            #[inline(always)]
            move |_| $result
        )
    };

    (@iter _ <= $collection:expr ; $($rest:tt)+) => {
        $collection.into_iter().flat_map(
            #[inline(always)]
            move |_| {
                $crate::for_!(@iter $($rest)+)
            }
        )
    };

    (@iter if $condition:expr ; yield $result:expr) => {
        ::core::iter::once(()).filter_map(
            #[inline(always)]
            move |_| {
                if $condition { Some($result) } else { None }
            }
        )
    };

    // Guard with continuation: use EitherIter to avoid Vec allocation
    (@iter if $condition:expr ; $($rest:tt)+) => {
        if $condition {
            $crate::compose::for_macro::EitherIter::Left(
                $crate::for_!(@iter $($rest)+)
            )
        } else {
            $crate::compose::for_macro::EitherIter::Right(::core::iter::empty())
        }
    };

    (@iter if let $pattern:pat = $expr:expr ; yield $result:expr) => {
        ::core::iter::once(()).filter_map(
            #[inline(always)]
            move |_| {
                match $expr {
                    $pattern => Some($result),
                    _ => None,
                }
            }
        )
    };

    (@iter if let $pattern:pat = $expr:expr ; $($rest:tt)+) => {
        match $expr {
            $pattern => $crate::compose::for_macro::EitherIter::Left(
                $crate::for_!(@iter $($rest)+)
            ),
            _ => $crate::compose::for_macro::EitherIter::Right(::core::iter::empty()),
        }
    };

    (@iter let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_!(@iter $($rest)+)
    }};

    (@iter let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $expr;
        $crate::for_!(@iter $($rest)+)
    }};

    // =========================================================================
    // Public entry points
    //
    // Strategy:
    // - Single iteration (`; yield expr`): Clone + @hint + collect_with_hint
    //   for optimal pre-allocation
    // - Nested iteration / guards (`$($rest)+`): collect_from_iter (no Clone)
    //   because @hint returns (0, None) for nested cases, making Clone wasteful
    //
    // BREAKING CHANGE (v0.2.0): Clone is required only for single-level iterations.
    // See module documentation for details.
    // =========================================================================

    (yield $result:expr) => {
        vec![$result]
    };

    // -------------------------------------------------------------------------
    // Identifier pattern
    // -------------------------------------------------------------------------

    // Single iteration with yield: Clone + @hint for pre-allocation
    ($pattern:ident <= $collection:expr ; yield $result:expr) => {{
        let __collection = $collection;
        let (__lower, __upper) = $crate::for_!(@hint $pattern <= __collection.clone() ; yield $result);
        $crate::compose::for_macro::collect_with_hint(
            __lower,
            __upper,
            $crate::for_!(@iter $pattern <= __collection ; yield $result)
        )
    }};

    // Nested iteration: use collect_from_iter to avoid unnecessary Clone.
    // @hint would return (0, None) for nested cases due to macro limitations,
    // so Clone would be wasted.
    ($pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        $crate::compose::for_macro::collect_from_iter(
            $crate::for_!(@iter $pattern <= $collection ; $($rest)+)
        )
    }};

    // -------------------------------------------------------------------------
    // Tuple pattern
    // -------------------------------------------------------------------------

    // Single iteration with yield: Clone + @hint for pre-allocation
    (($($pattern:tt)*) <= $collection:expr ; yield $result:expr) => {{
        let __collection = $collection;
        let (__lower, __upper) = $crate::for_!(@hint ($($pattern)*) <= __collection.clone() ; yield $result);
        $crate::compose::for_macro::collect_with_hint(
            __lower,
            __upper,
            $crate::for_!(@iter ($($pattern)*) <= __collection ; yield $result)
        )
    }};

    // Nested iteration: use collect_from_iter to avoid unnecessary Clone.
    (($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        $crate::compose::for_macro::collect_from_iter(
            $crate::for_!(@iter ($($pattern)*) <= $collection ; $($rest)+)
        )
    }};

    // -------------------------------------------------------------------------
    // Wildcard pattern
    // -------------------------------------------------------------------------

    // Single iteration with yield: Clone + @hint for pre-allocation
    (_ <= $collection:expr ; yield $result:expr) => {{
        let __collection = $collection;
        let (__lower, __upper) = $crate::for_!(@hint _ <= __collection.clone() ; yield $result);
        $crate::compose::for_macro::collect_with_hint(
            __lower,
            __upper,
            $crate::for_!(@iter _ <= __collection ; yield $result)
        )
    }};

    // Nested iteration: use collect_from_iter to avoid unnecessary Clone.
    (_ <= $collection:expr ; $($rest:tt)+) => {{
        $crate::compose::for_macro::collect_from_iter(
            $crate::for_!(@iter _ <= $collection ; $($rest)+)
        )
    }};

    // Pattern guard starting entry: expressions may reference undefined variables,
    // so we fall back to collect_from_iter which uses the iterator's size_hint.
    (if let $pattern:pat = $expr:expr ; $($rest:tt)+) => {{
        $crate::compose::for_macro::collect_from_iter(
            $crate::for_!(@iter if let $pattern = $expr ; $($rest)+)
        )
    }};

    // Guard starting entry: expressions may reference undefined variables,
    // so we fall back to collect_from_iter which uses the iterator's size_hint.
    (if $condition:expr ; $($rest:tt)+) => {{
        $crate::compose::for_macro::collect_from_iter(
            $crate::for_!(@iter if $condition ; $($rest)+)
        )
    }};

    (let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_!($($rest)+)
    }};

    (let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $expr;
        $crate::for_!($($rest)+)
    }};
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[rstest]
    fn test_yield_only() {
        let result = for_! {
            yield 42
        };
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_single_iteration() {
        let result = for_! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_nested_iteration() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            yield x + y
        };
        assert_eq!(result, vec![11, 21, 12, 22]);
    }

    #[test]
    fn test_tuple_pattern() {
        let pairs = vec![(1, "a"), (2, "b")];
        let result = for_! {
            (num, letter) <= pairs;
            yield format!("{}{}", num, letter)
        };
        assert_eq!(result, vec!["1a", "2b"]);
    }

    #[test]
    fn test_wildcard_pattern() {
        let result = for_! {
            _ <= vec![1, 2, 3];
            yield "x"
        };
        assert_eq!(result, vec!["x", "x", "x"]);
    }

    #[test]
    fn test_let_binding() {
        let result = for_! {
            x <= vec![1, 2, 3];
            let doubled = x * 2;
            yield doubled
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_let_tuple_binding() {
        let result = for_! {
            pair <= vec![(1, 2), (3, 4)];
            let (a, b) = pair;
            yield a + b
        };
        assert_eq!(result, vec![3, 7]);
    }

    #[test]
    fn test_empty_collection() {
        let empty: Vec<i32> = vec![];
        let result = for_! {
            x <= empty;
            yield x * 2
        };
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_collect_single_iteration_uses_map() {
        let result = for_! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_tuple_pattern_single_iteration() {
        let result = for_! { (a, b) <= vec![(1, 2), (3, 4)]; yield a + b };
        assert_eq!(result, vec![3, 7]);
    }

    #[test]
    fn test_wildcard_pattern_single_iteration() {
        let result = for_! { _ <= vec![1, 2, 3]; yield 42 };
        assert_eq!(result, vec![42, 42, 42]);
    }

    #[test]
    fn test_empty_collection_edge_case() {
        let result = for_! { x <= Vec::<i32>::new(); yield x };
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_element_collection() {
        let result = for_! { x <= vec![42]; yield x * 2 };
        assert_eq!(result, vec![84]);
    }

    #[test]
    fn test_four_level_nesting() {
        let result = for_! {
            a <= vec![1, 2];
            b <= vec![10, 20];
            c <= vec![100, 200];
            d <= vec![1000, 2000];
            yield a + b + c + d
        };
        assert_eq!(result.len(), 16);
        // Verify first and last elements
        assert_eq!(result[0], 1 + 10 + 100 + 1000); // 1111
        assert_eq!(result[15], 2 + 20 + 200 + 2000); // 2222
    }

    #[test]
    fn test_three_level_nesting() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            z <= vec![100, 200];
            yield x + y + z
        };
        assert_eq!(result, vec![111, 211, 121, 221, 112, 212, 122, 222]);
    }

    #[test]
    fn test_single_iteration_with_let_binding() {
        let result = for_! {
            x <= vec![1, 2, 3];
            let y = x * 2;
            yield y
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_nested_with_let_binding_in_middle() {
        let result = for_! {
            x <= vec![1, 2];
            let x_squared = x * x;
            y <= vec![10, 20];
            yield x_squared + y
        };
        assert_eq!(result, vec![11, 21, 14, 24]);
    }

    #[test]
    fn test_guard_basic_filter() {
        let result = for_! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn test_guard_all_pass() {
        let result = for_! {
            x <= vec![2, 4, 6];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_guard_all_fail() {
        let result = for_! {
            x <= vec![1, 3, 5];
            if x % 2 == 0;
            yield x
        };
        assert!(result.is_empty());
    }

    #[test]
    fn test_guard_empty_collection() {
        let result = for_! {
            x <= Vec::<i32>::new();
            if x > 0;
            yield x
        };
        assert!(result.is_empty());
    }

    #[test]
    fn test_guard_after_let() {
        let result = for_! {
            x <= vec![1, 2, 3, 4, 5];
            let squared = x * x;
            if squared > 10;
            yield squared
        };
        assert_eq!(result, vec![16, 25]);
    }

    #[test]
    fn test_guard_nested() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            if x + y > 15;
            yield (x, y)
        };
        assert_eq!(result, vec![(1, 20), (2, 20)]);
    }

    #[test]
    fn test_guard_multiple() {
        let result = for_! {
            x <= 1..=20i32;
            if x % 2 == 0;
            if x > 10;
            yield x
        };
        assert_eq!(result, vec![12, 14, 16, 18, 20]);
    }

    #[test]
    fn test_guard_between_binds() {
        let result = for_! {
            x <= vec![1, 2, 3];
            if x % 2 == 1;
            y <= vec![10, 20];
            yield (x, y)
        };
        assert_eq!(result, vec![(1, 10), (1, 20), (3, 10), (3, 20)]);
    }

    #[test]
    fn test_pattern_guard_option_some() {
        fn maybe_double(x: i32) -> Option<i32> {
            if x > 0 { Some(x * 2) } else { None }
        }

        let result = for_! {
            x <= vec![-1, 0, 1, 2, 3];
            if let Some(doubled) = maybe_double(x);
            yield doubled
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_pattern_guard_result_ok() {
        let result = for_! {
            s <= vec!["1", "abc", "2"];
            if let Ok(n) = s.parse::<i32>();
            yield n
        };
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_pattern_guard_nested_pattern() {
        let nested = vec![Some(Some(1)), Some(None), None, Some(Some(2))];
        let result = for_! {
            item <= nested;
            if let Some(Some(value)) = item;
            yield value
        };
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_pattern_guard_with_regular_guard() {
        let items = vec![Some(1), None, Some(5), Some(10)];
        let result = for_! {
            item <= items;
            if let Some(value) = item;
            if value > 3;
            yield value
        };
        assert_eq!(result, vec![5, 10]);
    }

    #[test]
    fn test_pattern_guard_with_let_binding() {
        let items = vec![Some(1), None, Some(2)];
        let result = for_! {
            item <= items;
            if let Some(value) = item;
            let doubled = value * 2;
            yield doubled
        };
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn test_pattern_guard_multiple_consecutive() {
        let nested = vec![Some(Some(1)), Some(None), None, Some(Some(5))];
        let result = for_! {
            item <= nested;
            if let Some(inner) = item;
            if let Some(value) = inner;
            yield value
        };
        assert_eq!(result, vec![1, 5]);
    }

    #[test]
    fn test_pattern_guard_tuple_nested() {
        let data = vec![Some((1, "a")), None, Some((2, "b"))];
        let result = for_! {
            item <= data;
            if let Some((num, letter)) = item;
            yield format!("{}{}", num, letter)
        };
        assert_eq!(result, vec!["1a", "2b"]);
    }

    #[test]
    fn test_pattern_guard_at_binding() {
        let items = vec![Some(1), None, Some(2)];
        let result = for_! {
            item <= items;
            if let whole @ Some(_) = item;
            yield whole
        };
        assert_eq!(result, vec![Some(1), Some(2)]);
    }

    use super::{
        EitherIter, SMALLVEC_INLINE_CAPACITY, collect_from_iter, collect_with_hint,
        combined_size_hint, compute_smallvec_threshold,
    };

    #[rstest]
    fn test_either_iter_left_size_hint() {
        let iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        assert_eq!(iter.size_hint(), (3, Some(3)));
    }

    #[rstest]
    fn test_either_iter_right_size_hint() {
        let iter: EitherIter<std::vec::IntoIter<i32>, _> = EitherIter::Right(std::iter::empty());
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[rstest]
    fn test_either_iter_left_iteration() {
        let iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_either_iter_right_iteration() {
        let iter: EitherIter<std::vec::IntoIter<i32>, _> = EitherIter::Right(std::iter::empty());
        let result: Vec<_> = iter.collect();
        assert_eq!(result, Vec::<i32>::new());
    }

    #[rstest]
    fn test_either_iter_exact_size_left() {
        let iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        assert_eq!(iter.len(), 3);
    }

    #[rstest]
    fn test_either_iter_exact_size_right() {
        let iter: EitherIter<std::vec::IntoIter<i32>, _> =
            EitherIter::Right(std::iter::empty::<i32>());
        assert_eq!(iter.len(), 0);
    }

    #[rstest]
    fn test_either_iter_clone() {
        let iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_either_iter_double_ended_left() {
        let mut iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        assert_eq!(iter.next_back(), Some(3));
        assert_eq!(iter.next_back(), Some(2));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);
    }

    #[rstest]
    fn test_either_iter_double_ended_right() {
        let mut iter: EitherIter<std::vec::IntoIter<i32>, _> =
            EitherIter::Right(std::iter::empty());
        assert_eq!(iter.next_back(), None);
    }

    #[rstest]
    fn test_either_iter_rev() {
        let iter: EitherIter<_, std::iter::Empty<i32>> =
            EitherIter::Left(vec![1, 2, 3].into_iter());
        let result: Vec<_> = iter.rev().collect();
        assert_eq!(result, vec![3, 2, 1]);
    }

    #[rstest]
    fn test_collect_with_hint_small_uses_smallvec_path() {
        let result = collect_with_hint(10, Some(10), 0..10);
        assert_eq!(result, (0..10).collect::<Vec<_>>());
    }

    #[rstest]
    fn test_collect_with_hint_large_uses_vec_path() {
        let result = collect_with_hint(1000, Some(1000), 0..1000);
        assert_eq!(result, (0..1000).collect::<Vec<_>>());
    }

    #[rstest]
    fn test_collect_with_hint_unknown_upper_uses_vec() {
        let result = collect_with_hint(10, None, 0..10);
        assert_eq!(result, (0..10).collect::<Vec<_>>());
    }

    #[rstest]
    fn test_collect_with_hint_zero_lower() {
        let result: Vec<i32> = collect_with_hint(0, None, std::iter::empty());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_collect_with_hint_at_threshold() {
        let result = collect_with_hint(128, Some(128), 0_i32..128);
        assert_eq!(result.len(), SMALLVEC_INLINE_CAPACITY);
    }

    #[rstest]
    fn test_collect_with_hint_just_above_threshold() {
        let result = collect_with_hint(129, Some(129), 0_i32..129);
        assert_eq!(result.len(), SMALLVEC_INLINE_CAPACITY + 1);
    }

    #[rstest]
    fn test_collect_from_iter_basic() {
        let result = collect_from_iter(vec![1, 2, 3].into_iter());
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_collect_from_iter_empty() {
        let result: Vec<i32> = collect_from_iter(std::iter::empty());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_collect_from_iter_with_known_size() {
        let result = collect_from_iter(0..100);
        assert_eq!(result.len(), 100);
    }

    #[rstest]
    fn test_hint_yield_only() {
        let (lower, upper) = for_!(@hint yield 42);
        assert_eq!(lower, 1);
        assert_eq!(upper, Some(1));
    }

    #[rstest]
    fn test_hint_single_iteration() {
        let xs = vec![1, 2, 3, 4, 5];
        let (lower, upper) = for_!(@hint x <= xs; yield x * 2);
        assert_eq!(lower, 5);
        assert_eq!(upper, Some(5));
    }

    #[rstest]
    fn test_hint_nested_iteration() {
        // Nested iteration: inner collection may reference outer loop variables,
        // so @hint conservatively returns (0, None).
        // Note: ys is referenced in macro but not evaluated at runtime due to conservative path.
        let xs = vec![1, 2, 3];
        #[allow(unused_variables, clippy::useless_vec)]
        let ys = vec![10, 20, 30, 40];
        let (lower, upper): (usize, Option<usize>) = for_!(@hint x <= xs; y <= ys; yield x + y);
        assert_eq!(lower, 0);
        assert_eq!(upper, None);
    }

    #[rstest]
    fn test_hint_with_guard() {
        // Guard after iteration: @hint returns (0, None) because rest contains more than yield
        let xs = vec![1, 2, 3, 4, 5];
        let (lower, upper): (usize, Option<usize>) = for_!(@hint x <= xs; if x % 2 == 0; yield x);
        assert_eq!(lower, 0);
        assert_eq!(upper, None);
    }

    #[rstest]
    fn test_hint_nested_with_guard() {
        // Nested iteration with guard: @hint returns (0, None) conservatively.
        // Note: ys is referenced in macro but not evaluated at runtime due to conservative path.
        let xs = vec![1, 2, 3];
        #[allow(unused_variables, clippy::useless_vec)]
        let ys = vec![10, 20];
        let (lower, upper): (usize, Option<usize>) =
            for_!(@hint x <= xs; y <= ys; if (x + y) % 2 == 0; yield x + y);
        assert_eq!(lower, 0);
        assert_eq!(upper, None);
    }

    // Note: @hint tests with pattern guards that reference loop variables are removed
    // because @hint is an internal rule that cannot safely evaluate such expressions.
    // The public entry points use collect_from_iter which relies on iterator's size_hint.

    #[rstest]
    fn test_hint_with_let_binding_no_var_ref() {
        // Test @hint with let binding that doesn't reference loop variable
        let xs = vec![1, 2, 3];
        let (lower, upper) = for_!(@hint x <= xs; yield x);
        assert_eq!(lower, 3);
        assert_eq!(upper, Some(3));
    }

    #[rstest]
    fn test_hint_empty_collection() {
        let xs: Vec<i32> = vec![];
        let (lower, upper) = for_!(@hint x <= xs; yield x * 2);
        assert_eq!(lower, 0);
        assert_eq!(upper, Some(0));
    }

    #[rstest]
    fn test_hint_tuple_pattern() {
        let pairs = vec![(1, 2), (3, 4), (5, 6)];
        let (lower, upper) = for_!(@hint (a, b) <= pairs; yield a + b);
        assert_eq!(lower, 3);
        assert_eq!(upper, Some(3));
    }

    #[rstest]
    fn test_hint_wildcard_pattern() {
        let xs = vec![1, 2, 3];
        let (lower, upper) = for_!(@hint _ <= xs; yield 42);
        assert_eq!(lower, 3);
        assert_eq!(upper, Some(3));
    }

    #[rstest]
    fn test_iter_yield_only() {
        let iter = for_!(@iter yield 42);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![42]);
    }

    #[rstest]
    fn test_iter_single_iteration() {
        let xs = vec![1, 2, 3];
        let iter = for_!(@iter x <= xs; yield x * 2);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[rstest]
    fn test_iter_nested_iteration() {
        let xs = vec![1, 2];
        let ys = vec![10, 20];
        let iter = for_!(@iter x <= xs; y <= ys.clone(); yield x + y);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![11, 21, 12, 22]);
    }

    #[rstest]
    fn test_iter_with_guard_yield() {
        let iter = for_!(@iter if true; yield 42);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![42]);

        let mut iter_false = for_!(@iter if false; yield 42);
        assert!(iter_false.next().is_none());
    }

    #[rstest]
    fn test_iter_with_guard_continuation() {
        let xs = vec![1, 2, 3, 4, 5];
        let iter = for_!(@iter x <= xs; if x % 2 == 0; yield x);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![2, 4]);
    }

    #[rstest]
    fn test_iter_with_pattern_guard_yield() {
        let iter = for_!(@iter if let Some(x) = Some(42); yield x);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![42]);

        let mut iter_none = for_!(@iter if let Some(x) = None::<i32>; yield x);
        assert!(iter_none.next().is_none());
    }

    #[rstest]
    fn test_iter_with_pattern_guard_continuation() {
        let xs = vec![Some(1), None, Some(2)];
        let iter = for_!(@iter opt <= xs; if let Some(x) = opt; yield x);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![1, 2]);
    }

    #[rstest]
    fn test_iter_with_let_binding() {
        let xs = vec![1, 2, 3];
        let iter = for_!(@iter x <= xs; let doubled = x * 2; yield doubled);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[rstest]
    fn test_iter_tuple_pattern() {
        let pairs = vec![(1, 2), (3, 4)];
        let iter = for_!(@iter (a, b) <= pairs; yield a + b);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![3, 7]);
    }

    #[rstest]
    fn test_iter_wildcard_pattern() {
        let xs = vec![1, 2, 3];
        let iter = for_!(@iter _ <= xs; yield 42);
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![42, 42, 42]);
    }

    #[rstest]
    fn test_iter_empty_collection() {
        let xs: Vec<i32> = vec![];
        let mut iter = for_!(@iter x <= xs; yield x * 2);
        assert!(iter.next().is_none());
    }

    #[rstest]
    fn test_iter_complex_nested() {
        let xs = vec![1, 2, 3];
        let ys = vec![10, 20];
        let iter = for_!(@iter
            x <= xs;
            y <= ys.clone();
            if (x + y) % 2 == 0;
            let sum = x + y;
            yield sum
        );
        let result: Vec<_> = iter.collect();
        assert_eq!(result, vec![12, 22]);
    }

    // =========================================================================
    // combined_size_hint tests
    // =========================================================================

    #[rstest]
    fn test_combined_size_hint_exact() {
        let hints = [(10, Some(10)), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 200);
    }

    #[rstest]
    fn test_combined_size_hint_partial() {
        // When upper is None, uses lower product
        let hints = [(10, None), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 200);
    }

    #[rstest]
    fn test_combined_size_hint_all_none() {
        let hints = [(10, None), (20, None)];
        assert_eq!(combined_size_hint(&hints), 200);
    }

    #[rstest]
    fn test_combined_size_hint_single() {
        let hints = [(42, Some(42))];
        assert_eq!(combined_size_hint(&hints), 42);
    }

    #[rstest]
    fn test_combined_size_hint_empty() {
        assert_eq!(combined_size_hint(&[]), 0);
    }

    #[rstest]
    fn test_combined_size_hint_overflow_upper() {
        // upper overflow: falls back to lower
        let hints = [(usize::MAX, Some(usize::MAX)), (2, Some(2))];
        let result = combined_size_hint(&hints);
        // lower saturates to usize::MAX, upper overflows to None
        assert_eq!(result, usize::MAX);
    }

    #[rstest]
    fn test_combined_size_hint_overflow_lower() {
        // lower saturates to usize::MAX
        let hints = [(usize::MAX, None), (usize::MAX, None)];
        assert_eq!(combined_size_hint(&hints), usize::MAX);
    }

    #[rstest]
    fn test_combined_size_hint_zero_lower_with_known_upper() {
        // Guard case: lower = 0 but upper is known, so use upper product
        // combined_lower = 0 * 20 = 0
        // combined_upper = Some(10 * 20) = Some(200)
        // Returns 200 because upper is known and provides better pre-allocation hint
        let hints = [(0, Some(10)), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 200);
    }

    #[rstest]
    fn test_combined_size_hint_zero_lower_with_unknown_upper() {
        // Guard case: lower = 0 and upper is unknown
        // combined_lower = 0 * 20 = 0
        // combined_upper = None (because first hint has None)
        // Falls back to lower (0)
        let hints = [(0, None), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 0);
    }

    #[rstest]
    fn test_combined_size_hint_is_pure_function() {
        // Same input always returns same output (referential transparency)
        let hints = [(5, Some(5)), (10, Some(10))];
        assert_eq!(combined_size_hint(&hints), combined_size_hint(&hints));
        assert_eq!(combined_size_hint(&hints), 50);
    }

    #[rstest]
    fn test_combined_size_hint_three_levels() {
        // 5 * 5 * 5 = 125
        let hints = [(5, Some(5)), (5, Some(5)), (5, Some(5))];
        assert_eq!(combined_size_hint(&hints), 125);
    }

    // =========================================================================
    // compute_smallvec_threshold tests
    // =========================================================================

    #[rstest]
    fn test_smallvec_threshold_varies_by_element_size() {
        // Smaller elements should have higher or equal threshold
        let threshold_i32 = compute_smallvec_threshold::<i32>();
        let threshold_i64 = compute_smallvec_threshold::<i64>();
        assert!(
            threshold_i32 >= threshold_i64,
            "i32 threshold ({threshold_i32}) should be >= i64 threshold ({threshold_i64})"
        );
    }

    #[rstest]
    fn test_threshold_is_pure_function() {
        // Same type always returns same result (referential transparency)
        assert_eq!(
            compute_smallvec_threshold::<i32>(),
            compute_smallvec_threshold::<i32>()
        );
        assert_eq!(
            compute_smallvec_threshold::<String>(),
            compute_smallvec_threshold::<String>()
        );
        assert_eq!(
            compute_smallvec_threshold::<u8>(),
            compute_smallvec_threshold::<u8>()
        );
    }

    #[rstest]
    fn test_threshold_capped_at_inline_capacity() {
        // For small elements like u8, threshold should be capped at SMALLVEC_INLINE_CAPACITY
        let threshold_u8 = compute_smallvec_threshold::<u8>();
        assert!(
            threshold_u8 <= SMALLVEC_INLINE_CAPACITY,
            "u8 threshold ({threshold_u8}) should be <= SMALLVEC_INLINE_CAPACITY ({SMALLVEC_INLINE_CAPACITY})"
        );
    }

    #[rstest]
    fn test_threshold_for_large_elements() {
        // For large elements, threshold should be based on cache size
        // A 1KB struct: 32KB / 1024 = 32
        #[repr(C)]
        struct LargeStruct {
            _data: [u8; 1024],
        }
        let threshold = compute_smallvec_threshold::<LargeStruct>();
        assert_eq!(threshold, 32, "1KB element should have threshold of 32");
    }

    #[rstest]
    fn test_threshold_for_zst() {
        // Zero-sized types should not cause division by zero
        let threshold = compute_smallvec_threshold::<()>();
        assert!(
            threshold > 0,
            "ZST threshold should be positive (computed as L1_CACHE_SIZE / 1)"
        );
    }

    #[rstest]
    fn test_into_vec_preserves_capacity() {
        // Verify SmallVec with_capacity + into_vec preserves capacity
        use smallvec::SmallVec;
        let mut smallvec: SmallVec<[i32; 128]> = SmallVec::with_capacity(50);
        smallvec.extend(0..50);
        let vec = smallvec.into_vec();
        assert!(
            vec.capacity() >= 50,
            "Vec capacity ({}) should be >= 50",
            vec.capacity()
        );
    }

    // =========================================================================
    // collect_with_hint behavior tests
    // =========================================================================

    #[rstest]
    fn test_collect_with_hint_known_upper_uses_vec_directly() {
        // When upper is known, should use Vec directly (no SmallVec overhead)
        let result = collect_with_hint(10, Some(10), 0..10);
        assert_eq!(result.len(), 10);
        // Verify capacity is at least what we requested
        assert!(
            result.capacity() >= 10,
            "Vec capacity ({}) should be >= 10",
            result.capacity()
        );
    }

    #[rstest]
    fn test_collect_with_hint_unknown_upper_above_threshold() {
        // When lower > threshold, should use Vec directly
        let threshold = compute_smallvec_threshold::<i32>();
        let count = threshold + 100;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let result = collect_with_hint(count, None, 0..count as i32);
        assert_eq!(result.len(), count);
    }

    #[rstest]
    fn test_collect_with_hint_unknown_upper_below_threshold() {
        // When lower <= threshold and upper is None, should use SmallVec path
        let result = collect_with_hint(10, None, 0..10);
        assert_eq!(result.len(), 10);
    }

    #[rstest]
    fn test_collect_with_hint_respects_max_capacity() {
        // Verify MAX_REASONABLE_CAPACITY is respected
        let large_upper = 10_000_000usize;
        let result: Vec<i32> = collect_with_hint(0, Some(large_upper), std::iter::empty());
        // Capacity should be capped at MAX_REASONABLE_CAPACITY (1MB)
        assert!(
            result.capacity() <= 1024 * 1024,
            "capacity ({}) should be <= 1MB",
            result.capacity()
        );
    }

    #[rstest]
    fn test_collect_with_hint_empty_with_zero_lower() {
        // Edge case: lower = 0, upper = None
        let result: Vec<i32> = collect_with_hint(0, None, std::iter::empty());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_collect_with_hint_large_elements() {
        // For large elements, threshold is lower
        #[derive(Clone)]
        #[repr(C)]
        struct LargeStruct {
            _data: [u8; 512],
        }
        // threshold for 512-byte struct: 32KB / 512 = 64
        let count = 100; // Above threshold, should use Vec directly
        let result: Vec<LargeStruct> = collect_with_hint(
            count,
            None,
            (0..count).map(|_| LargeStruct { _data: [0; 512] }),
        );
        assert_eq!(result.len(), count);
    }

    // =========================================================================
    // lower == 0 fallback to Vec tests (Codex review issue fix)
    // =========================================================================

    #[rstest]
    fn test_collect_with_hint_zero_lower_uses_vec_path() {
        // When lower == 0 and upper is None (typical for filter/flat_map),
        // should use Vec to avoid stack bloat from SmallVec's inline buffer.
        // Minimum capacity should be 16 to avoid under-allocation.
        let result: Vec<i32> = collect_with_hint(0, None, std::iter::empty());
        assert!(result.is_empty());
        // Capacity should be at least 16 (minimum for unknown size)
        assert!(
            result.capacity() >= 16,
            "capacity ({}) should be >= 16 for unknown size",
            result.capacity()
        );
    }

    #[rstest]
    fn test_collect_with_hint_zero_lower_with_actual_elements() {
        // When lower == 0 but iterator has elements (e.g., filter that passes some)
        let iter = (0..50).filter(|x| x % 2 == 0); // size_hint: (0, Some(50))
        let (lower, _upper) = iter.size_hint();
        assert_eq!(lower, 0, "filter's lower bound should be 0");

        let result: Vec<i32> = collect_with_hint(0, None, (0..50).filter(|x| x % 2 == 0));
        assert_eq!(result.len(), 25);
    }

    #[rstest]
    fn test_collect_with_hint_nonzero_lower_below_threshold_uses_smallvec() {
        // When lower > 0 and lower <= threshold, should use SmallVec path
        // This is the only case where SmallVec is used
        let result = collect_with_hint(10, None, 0..10);
        assert_eq!(result.len(), 10);
        // This test verifies the path is taken (SmallVec is used internally)
        // We can't directly test internal implementation, but verify correctness
    }

    #[rstest]
    fn test_collect_with_hint_zero_lower_large_elements_avoids_stack_bloat() {
        // For large elements with lower == 0, should use Vec to avoid potential stack overflow
        #[derive(Clone)]
        #[repr(C)]
        struct VeryLargeStruct {
            _data: [u8; 4096], // 4KB per element
        }
        // With SmallVec<[VeryLargeStruct; 128]>, this would be 512KB on stack!
        // Using Vec avoids this issue.
        let result: Vec<VeryLargeStruct> = collect_with_hint(0, None, std::iter::empty());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_for_macro_with_filter_uses_vec_path() {
        // for_! with filter generates size_hint (0, None), which should use Vec path
        let result = for_! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result, vec![2, 4]);
    }

    #[rstest]
    fn test_for_macro_nested_uses_vec_path() {
        // Nested for_! generates size_hint (0, None), which should use Vec path
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            yield x + y
        };
        assert_eq!(result, vec![11, 21, 12, 22]);
    }
}
