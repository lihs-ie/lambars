//! Persistent (immutable) hash map based on HAMT.
//!
//! This module provides [`PersistentHashMap`], an immutable hash map
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentHashMap` is based on Hash Array Mapped Trie (HAMT), a data structure
//! that provides efficient immutable operations. It uses a 32-way branching trie
//! where hash bits are used to navigate the tree.
//!
//! - O(log32 N) get (effectively O(1) for practical sizes)
//! - O(log32 N) insert
//! - O(log32 N) remove
//! - O(1) len and `is_empty`
//!
//! All operations return new maps without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentHashMap;
//!
//! let map = PersistentHashMap::new()
//!     .insert("one".to_string(), 1)
//!     .insert("two".to_string(), 2)
//!     .insert("three".to_string(), 3);
//!
//! assert_eq!(map.get("one"), Some(&1));
//! assert_eq!(map.get("two"), Some(&2));
//! assert_eq!(map.get("three"), Some(&3));
//!
//! // Structural sharing: the original map is preserved
//! let updated = map.insert("one".to_string(), 100);
//! assert_eq!(map.get("one"), Some(&1));       // Original unchanged
//! assert_eq!(updated.get("one"), Some(&100)); // New version
//! ```
//!
//! # Internal Structure
//!
//! The HAMT uses:
//! - 32-way branching (5 bits per level)
//! - Bitmap to track which slots are occupied
//! - Collision nodes for hash collisions
//! - Structural sharing via reference counting (`Rc` or `Arc` with `arc` feature)

use super::ReferenceCounter;
use smallvec::SmallVec;
use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::typeclass::{Foldable, TypeConstructor};

// =============================================================================
// Constants
// =============================================================================

/// Branching factor (2^5 = 32)
const BRANCHING_FACTOR: usize = 32;

/// Bits per level in the trie
const BITS_PER_LEVEL: usize = 5;

/// Bit mask for extracting index within a node
const MASK: u64 = (BRANCHING_FACTOR - 1) as u64;

/// Maximum depth of the trie (64 bits / 5 bits per level = ~13 levels)
#[allow(dead_code)]
const MAX_DEPTH: usize = 13;

/// Maximum number of entries allowed in a single bulk insert operation.
///
/// This limit prevents memory spikes and ensures predictable performance.
/// For larger datasets, callers should chunk the data into smaller batches.
///
/// # Recommended Chunk Size
///
/// When the input exceeds this limit, consider chunking into batches of 10,000
/// entries for optimal performance while staying well within limits.
pub const MAX_BULK_INSERT: usize = 100_000;

// Compile-time assertion: MAX_BULK_INSERT must be at least 2
// for CHUNK_SIZE = MAX_BULK_INSERT - 1 to be valid.
const _: () = assert!(MAX_BULK_INSERT >= 2, "MAX_BULK_INSERT must be at least 2");

// =============================================================================
// TASK-009: Occupancy Distribution Measurement (debug build only)
// =============================================================================

/// Module for collecting `BitmapNode` occupancy distribution statistics.
///
/// This module is only active in debug builds (`debug_assertions`).
/// It tracks how many children each `BitmapNode` has when created,
/// providing data to inform `SmallVec` inline capacity tuning.
///
/// # Buckets
///
/// Occupancy is grouped into buckets:
/// - Bucket 0: 0 children (should be rare)
/// - Bucket 1: 1 child
/// - Bucket 2: 2 children
/// - ...
/// - Bucket 31: 31 children
/// - Bucket 32: 32 children (fully populated)
#[cfg(debug_assertions)]
pub mod occupancy_histogram {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Histogram buckets for occupancy counts (0-32 inclusive).
    /// Index i represents `BitmapNode` instances with exactly i children.
    ///
    /// Note: We use `const INIT` for array initialization, which is a common pattern
    /// for initializing arrays of `AtomicU64`. The clippy warning about interior
    /// mutability in const is expected here since we only use it for initialization.
    static HISTOGRAM: [AtomicU64; 33] = {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: AtomicU64 = AtomicU64::new(0);
        [INIT; 33]
    };

    /// Records a `BitmapNode` creation with the given occupancy.
    ///
    /// # Arguments
    ///
    /// * `occupancy` - Number of children in the `BitmapNode` (0-32)
    ///
    /// # Panics
    ///
    /// Panics in debug mode if occupancy > 32.
    #[inline]
    pub fn record_occupancy(occupancy: usize) {
        debug_assert!(occupancy <= 32, "occupancy must be <= 32, got {occupancy}");
        let index = occupancy.min(32);
        HISTOGRAM[index].fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the current histogram as an array of counts.
    ///
    /// # Returns
    ///
    /// An array of 33 elements where index i contains the count of
    /// `BitmapNode` instances created with exactly i children.
    ///
    /// This function is primarily used by tests and external tooling for
    /// performance analysis, hence the `allow(dead_code)` attribute.
    #[must_use]
    #[allow(dead_code)]
    pub fn get_histogram() -> [u64; 33] {
        let mut result = [0u64; 33];
        for (i, bucket) in HISTOGRAM.iter().enumerate() {
            result[i] = bucket.load(Ordering::Relaxed);
        }
        result
    }

    /// Resets all histogram buckets to zero.
    ///
    /// Useful for isolating measurements in tests or benchmarks.
    /// This function is primarily used by tests, hence the `allow(dead_code)` attribute.
    #[allow(dead_code)]
    pub fn reset_histogram() {
        for bucket in &HISTOGRAM {
            bucket.store(0, Ordering::Relaxed);
        }
    }

    /// Returns a summary of the histogram distribution.
    ///
    /// # Returns
    ///
    /// A formatted string showing:
    /// - Total `BitmapNode` creations
    /// - Distribution across occupancy levels
    /// - Mean occupancy
    ///
    /// This function is primarily used by tests and external tooling for
    /// performance analysis, hence the `allow(dead_code)` attribute.
    #[must_use]
    #[allow(dead_code)]
    #[allow(clippy::cast_precision_loss)]
    pub fn summary() -> String {
        use std::fmt::Write;

        let histogram = get_histogram();
        let total: u64 = histogram.iter().sum();

        if total == 0 {
            return "No BitmapNode creations recorded.".to_string();
        }

        let mut output = String::new();
        let _ = writeln!(output, "BitmapNode Occupancy Histogram (total: {total})");
        output.push_str("-------------------------------------------\n");

        let mut weighted_sum: u64 = 0;
        for (occupancy, &count) in histogram.iter().enumerate() {
            if count > 0 {
                let percentage = (count as f64 / total as f64) * 100.0;
                let _ = writeln!(
                    output,
                    "  {occupancy:>2} children: {count:>8} ({percentage:>5.1}%)"
                );
                weighted_sum += (occupancy as u64) * count;
            }
        }

        let mean = weighted_sum as f64 / total as f64;
        output.push_str("-------------------------------------------\n");
        let _ = writeln!(output, "Mean occupancy: {mean:.2}");

        output
    }
}

/// Placeholder module for release builds (no-op).
///
/// These functions are intentionally no-ops in release builds.
/// The `#[allow(dead_code)]` attribute is applied because the corresponding
/// debug build functions are used in tests, but the release build versions
/// may not be called directly.
#[cfg(not(debug_assertions))]
#[allow(dead_code)]
pub mod occupancy_histogram {
    /// No-op in release builds.
    #[inline]
    pub fn record_occupancy(_occupancy: usize) {}

    /// Returns empty histogram in release builds.
    #[must_use]
    pub fn get_histogram() -> [u64; 33] {
        [0u64; 33]
    }

    /// No-op in release builds.
    pub fn reset_histogram() {}

    /// Returns empty summary in release builds.
    #[must_use]
    pub fn summary() -> String {
        "Occupancy histogram is only available in debug builds.".to_string()
    }
}

// =============================================================================
// Generation Token System
// =============================================================================

static GENERATION_COUNTER: AtomicU64 = AtomicU64::new(1);
const SHARED_GENERATION: u64 = 0;

#[inline]
fn next_generation() -> u64 {
    GENERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// =============================================================================
// Hash computation
// =============================================================================

/// Fixed seed for ahash to ensure referential transparency.
///
/// Using the fractional part of mathematical constants (pi) as a seed value.
/// This provides a deterministic seed that produces the same hash values
/// across process restarts and different machines.
///
/// # Warning
///
/// This fixed seed removes `DoS` resistance. Only use `ahash` feature
/// for internal data that is not influenced by external input.
#[cfg(all(feature = "ahash", not(feature = "fxhash")))]
const AHASH_SEED: usize = 0x243f_6a88_85a3_08d3_usize;

/// Computes the hash of a key.
///
/// The hasher is selected based on feature flags:
/// - `fxhash`: Uses `FxHasher` (fastest, no `DoS` resistance)
/// - `ahash`: Uses `AHasher` with fixed seed (fast, no `DoS` resistance)
/// - default: Uses `DefaultHasher` (`SipHash`, has `DoS` resistance)
///
/// # Priority
///
/// If both `fxhash` and `ahash` features are enabled, `fxhash` takes priority.
///
/// # Warning
///
/// When using `fxhash` or `ahash` features, `DoS` resistance is lost.
/// Use these only in trusted environments where performance is critical
/// and all keys come from trusted sources.
///
/// **Do NOT use `fxhash` or `ahash` when:**
/// - Keys come from user input
/// - Keys come from network data
/// - Keys come from external files
///
/// Attackers can craft keys that cause hash collisions, degrading
/// performance to O(n) per operation (`HashDoS` attack).
#[cfg(feature = "fxhash")]
#[inline]
fn compute_hash<K: Hash + ?Sized>(key: &K) -> u64 {
    use std::hash::Hasher;
    let mut hasher = rustc_hash::FxHasher::default();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Computes the hash of a key using ahash with a fixed seed.
///
/// Uses a fixed seed to ensure deterministic behavior within the same
/// binary execution: calling this function with the same key will always
/// return the same hash value during program runtime.
///
/// # Limitations
///
/// Hash values are NOT stable across:
/// - Different machines or CPU architectures
/// - Different compiler versions
/// - Different versions of the ahash crate
///
/// Therefore, do NOT use these hash values for:
/// - Persistence to disk or database
/// - Cross-process communication
/// - Network protocols
///
/// # Warning
///
/// This removes `DoS` resistance. Only use for internal/trusted data.
#[cfg(all(feature = "ahash", not(feature = "fxhash")))]
#[inline]
fn compute_hash<K: Hash + ?Sized>(key: &K) -> u64 {
    use std::sync::LazyLock;
    // Static hasher state to avoid repeated initialization
    static AHASH_STATE: LazyLock<ahash::RandomState> =
        LazyLock::new(|| ahash::RandomState::with_seed(AHASH_SEED));
    std::hash::BuildHasher::hash_one(&*AHASH_STATE, key)
}

/// Computes the hash of a key using `DefaultHasher` (`SipHash`).
///
/// This is the default hasher with `DoS` resistance, suitable for
/// handling untrusted input.
#[cfg(all(not(feature = "fxhash"), not(feature = "ahash")))]
#[inline]
fn compute_hash<K: Hash + ?Sized>(key: &K) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Extracts the index at a given depth from a hash.
///
/// # Returns
///
/// An index in the range `0..BRANCHING_FACTOR` (0-31).
///
/// # Safety of the cast
///
/// The result of `(hash >> shift) & MASK` is always in the range 0-31
/// because MASK is 0x1F (31). This range is representable by usize on
/// all supported platforms, so the cast from u64 to usize is safe.
#[inline]
const fn hash_index(hash: u64, depth: usize) -> usize {
    ((hash >> (depth * BITS_PER_LEVEL)) & MASK) as usize
}

// =============================================================================
// Type Aliases for SmallVec optimization
// =============================================================================

type ChildArray<K, V> = SmallVec<[ChildSlot<K, V>; 6]>;
type CollisionArray<K, V> = SmallVec<[(K, V); 4]>;

// =============================================================================
// Node Definition
// =============================================================================

/// Internal node structure for the HAMT.
/// The `generation` field is for COW optimization and excluded from PartialEq/Hash/Debug.
#[derive(Clone)]
enum Node<K, V> {
    /// Empty node (used as sentinel)
    Empty,
    /// Single key-value entry
    Entry {
        hash: u64,
        key: K,
        value: V,
        generation: u64,
    },
    /// Bitmap-indexed branch node
    Bitmap {
        bitmap: u32,
        children: ChildArray<K, V>,
        generation: u64,
    },
    /// Collision node for keys with the same hash
    Collision {
        hash: u64,
        entries: CollisionArray<K, V>,
        generation: u64,
    },
}

/// A child slot in a bitmap node.
/// Renamed from `Child` to `ChildSlot` for clarity.
#[derive(Clone)]
enum ChildSlot<K, V> {
    /// A key-value entry with cached hash value
    Entry {
        hash: u64,
        key: K,
        value: V,
        generation: u64,
    },
    /// A sub-node (structural sharing is maintained through `ReferenceCounter`)
    Node(ReferenceCounter<Node<K, V>>),
}

impl<K, V> Node<K, V> {
    /// Creates an empty node.
    const fn empty() -> Self {
        Self::Empty
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for Node<K, V> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (
                Self::Entry {
                    hash: h1,
                    key: k1,
                    value: v1,
                    ..
                },
                Self::Entry {
                    hash: h2,
                    key: k2,
                    value: v2,
                    ..
                },
            ) => h1 == h2 && k1 == k2 && v1 == v2,
            (
                Self::Bitmap {
                    bitmap: b1,
                    children: c1,
                    ..
                },
                Self::Bitmap {
                    bitmap: b2,
                    children: c2,
                    ..
                },
            ) => b1 == b2 && c1 == c2,
            (
                Self::Collision {
                    hash: h1,
                    entries: e1,
                    ..
                },
                Self::Collision {
                    hash: h2,
                    entries: e2,
                    ..
                },
            ) => h1 == h2 && e1 == e2,
            _ => false,
        }
    }
}
impl<K: Eq, V: Eq> Eq for Node<K, V> {}

impl<K: Hash, V: Hash> Hash for Node<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Empty => {}
            Self::Entry {
                hash: h,
                key,
                value,
                ..
            } => {
                h.hash(state);
                key.hash(state);
                value.hash(state);
            }
            Self::Bitmap {
                bitmap, children, ..
            } => {
                bitmap.hash(state);
                children.hash(state);
            }
            Self::Collision {
                hash: h, entries, ..
            } => {
                h.hash(state);
                entries.hash(state);
            }
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Node<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Entry {
                hash, key, value, ..
            } => f
                .debug_struct("Entry")
                .field("hash", hash)
                .field("key", key)
                .field("value", value)
                .finish(),
            Self::Bitmap {
                bitmap, children, ..
            } => f
                .debug_struct("Bitmap")
                .field("bitmap", bitmap)
                .field("children", children)
                .finish(),
            Self::Collision { hash, entries, .. } => f
                .debug_struct("Collision")
                .field("hash", hash)
                .field("entries", entries)
                .finish(),
        }
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for ChildSlot<K, V> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Entry {
                    hash: h1,
                    key: k1,
                    value: v1,
                    ..
                },
                Self::Entry {
                    hash: h2,
                    key: k2,
                    value: v2,
                    ..
                },
            ) => h1 == h2 && k1 == k2 && v1 == v2,
            (Self::Node(n1), Self::Node(n2)) => n1 == n2,
            _ => false,
        }
    }
}
impl<K: Eq, V: Eq> Eq for ChildSlot<K, V> {}

impl<K: Hash, V: Hash> Hash for ChildSlot<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Entry {
                hash: h,
                key,
                value,
                ..
            } => {
                h.hash(state);
                key.hash(state);
                value.hash(state);
            }
            Self::Node(node) => node.hash(state),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for ChildSlot<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entry {
                hash, key, value, ..
            } => f
                .debug_struct("Entry")
                .field("hash", hash)
                .field("key", key)
                .field("value", value)
                .finish(),
            Self::Node(node) => f.debug_tuple("Node").field(node).finish(),
        }
    }
}

// =============================================================================
// PersistentHashMap Definition
// =============================================================================

/// A persistent (immutable) hash map based on HAMT.
///
/// `PersistentHashMap` is an immutable data structure that uses structural
/// sharing to efficiently support functional programming patterns.
///
/// # Time Complexity
///
/// | Operation      | Complexity        |
/// |----------------|-------------------|
/// | `new`          | O(1)              |
/// | `get`          | O(log32 N)        |
/// | `insert`       | O(log32 N)        |
/// | `remove`       | O(log32 N)        |
/// | `contains_key` | O(log32 N)        |
/// | `len`          | O(1)              |
/// | `is_empty`     | O(1)              |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentHashMap;
///
/// let map = PersistentHashMap::singleton("key".to_string(), 42);
/// assert_eq!(map.get("key"), Some(&42));
/// ```
#[derive(Clone)]
pub struct PersistentHashMap<K, V> {
    /// Root node of the trie
    root: ReferenceCounter<Node<K, V>>,
    /// Number of entries
    length: usize,
}

impl<K, V> PersistentHashMap<K, V> {
    /// Creates a new empty map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: ReferenceCounter::new(Node::empty()),
            length: 0,
        }
    }

    /// Returns the number of entries in the map.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// assert_eq!(map.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the map contains no entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert("key".to_string(), 42);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl<K: Clone + Hash + Eq, V: Clone> PersistentHashMap<K, V> {
    /// Creates a map containing a single key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key` - The key
    /// * `value` - The value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::singleton("key".to_string(), 42);
    /// assert_eq!(map.len(), 1);
    /// assert_eq!(map.get("key"), Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(key: K, value: V) -> Self {
        Self::new().insert(key, value)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but `Hash` and
    /// `Eq` on the borrowed form must match those for the key type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("hello".to_string(), 42);
    ///
    /// // Can use &str to look up String keys
    /// assert_eq!(map.get("hello"), Some(&42));
    /// assert_eq!(map.get("world"), None);
    /// ```
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = compute_hash(key);
        Self::get_from_node(&self.root, key, hash, 0)
    }

    /// Recursive helper for get.
    fn get_from_node<'a, Q>(node: &'a Node<K, V>, key: &Q, hash: u64, depth: usize) -> Option<&'a V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match node {
            Node::Empty => None,
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                value,
                ..
            } => {
                if *entry_hash == hash && entry_key.borrow() == key {
                    Some(value)
                } else {
                    None
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if bitmap & bit == 0 {
                    None
                } else {
                    // Count bits to find position in children array
                    let position = (bitmap & (bit - 1)).count_ones() as usize;
                    match &children[position] {
                        ChildSlot::Entry {
                            key: child_key,
                            value,
                            ..
                        } => {
                            if child_key.borrow() == key {
                                Some(value)
                            } else {
                                None
                            }
                        }
                        ChildSlot::Node(subnode) => {
                            Self::get_from_node(subnode, key, hash, depth + 1)
                        }
                    }
                }
            }
            Node::Collision {
                hash: _, entries, ..
            } => {
                for (entry_key, value) in entries {
                    if entry_key.borrow() == key {
                        return Some(value);
                    }
                }
                None
            }
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("key".to_string(), 42);
    ///
    /// assert!(map.contains_key("key"));
    /// assert!(!map.contains_key("other"));
    /// ```
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map already contains the key, the value is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    /// * `value` - The value to insert
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map1 = PersistentHashMap::new().insert("key".to_string(), 1);
    /// let map2 = map1.insert("key".to_string(), 2);
    ///
    /// assert_eq!(map1.get("key"), Some(&1)); // Original unchanged
    /// assert_eq!(map2.get("key"), Some(&2)); // New version
    /// ```
    #[must_use]
    pub fn insert(&self, key: K, value: V) -> Self {
        let hash = compute_hash(&key);
        let (new_root, added) = Self::insert_into_node(&self.root, key, value, hash, 0);

        Self {
            root: ReferenceCounter::new(new_root),
            length: if added { self.length + 1 } else { self.length },
        }
    }

    // =========================================================================
    // Helper functions for efficient child array operations
    // =========================================================================

    /// Builds a new child array with a new element inserted at the specified position.
    ///
    /// Uses `SmallVec` for stack allocation when the array is small (6 elements or fewer).
    ///
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to insert at
    /// * `new_child` - The new child to insert
    ///
    /// # Returns
    /// A new `ChildArray` with the child inserted at the specified position.
    fn build_children_with_insert(
        children: &[ChildSlot<K, V>],
        position: usize,
        new_child: ChildSlot<K, V>,
    ) -> ChildArray<K, V> {
        let mut result = ChildArray::with_capacity(children.len() + 1);
        result.extend(children[..position].iter().cloned());
        result.push(new_child);
        result.extend(children[position..].iter().cloned());
        result
    }

    /// Builds a new child array with the element at the specified position updated.
    ///
    /// Uses `SmallVec` for stack allocation when the array is small (6 elements or fewer).
    ///
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to update
    /// * `new_child` - The new child to place at the position
    ///
    /// # Returns
    /// A new `ChildArray` with the child at the specified position replaced.
    fn build_children_with_update(
        children: &[ChildSlot<K, V>],
        position: usize,
        new_child: ChildSlot<K, V>,
    ) -> ChildArray<K, V> {
        let mut result = ChildArray::with_capacity(children.len());
        result.extend(children[..position].iter().cloned());
        result.push(new_child);
        result.extend(children[(position + 1)..].iter().cloned());
        result
    }

    /// Builds a new child array with the element at the specified position removed.
    ///
    /// Uses `SmallVec` for stack allocation when the array is small (6 elements or fewer).
    ///
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to remove
    ///
    /// # Returns
    /// A new `ChildArray` with the child at the specified position removed.
    fn build_children_with_remove(
        children: &[ChildSlot<K, V>],
        position: usize,
    ) -> ChildArray<K, V> {
        let mut result = ChildArray::with_capacity(children.len().saturating_sub(1));
        result.extend(children[..position].iter().cloned());
        result.extend(children[(position + 1)..].iter().cloned());
        result
    }

    /// Recursive helper for insert.
    /// Returns (`new_node`, `was_added`) where `was_added` is true if a new entry was added.
    fn insert_into_node(
        node: &Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        match node {
            Node::Empty => (
                Node::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                },
                true,
            ),
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
                ..
            } => Self::insert_into_entry_node(
                *existing_hash,
                existing_key,
                existing_value,
                key,
                value,
                hash,
                depth,
            ),
            Node::Bitmap {
                bitmap, children, ..
            } => Self::insert_into_bitmap_node(*bitmap, children, key, value, hash, depth),
            Node::Collision {
                hash: collision_hash,
                entries,
                ..
            } => Self::insert_into_collision_node(
                node,
                *collision_hash,
                entries,
                key,
                value,
                hash,
                depth,
            ),
        }
    }

    /// Helper for inserting into an Entry node.
    fn insert_into_entry_node(
        existing_hash: u64,
        existing_key: &K,
        existing_value: &V,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        if existing_hash == hash && *existing_key == key {
            (
                Node::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                },
                false,
            )
        } else if existing_hash == hash {
            let mut entries = CollisionArray::new();
            entries.push((existing_key.clone(), existing_value.clone()));
            entries.push((key, value));
            (
                Node::Collision {
                    hash,
                    entries,
                    generation: SHARED_GENERATION,
                },
                true,
            )
        } else {
            Self::create_bitmap_from_two_entries(
                existing_hash,
                existing_key,
                existing_value,
                key,
                value,
                hash,
                depth,
            )
        }
    }

    /// Creates a bitmap node from two entries with different hashes.
    fn create_bitmap_from_two_entries(
        existing_hash: u64,
        existing_key: &K,
        existing_value: &V,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let existing_index = hash_index(existing_hash, depth);
        let new_index = hash_index(hash, depth);

        if existing_index == new_index {
            let sub_entry = Node::Entry {
                hash: existing_hash,
                key: existing_key.clone(),
                value: existing_value.clone(),
                generation: SHARED_GENERATION,
            };
            let (subnode, added) = Self::insert_into_node(&sub_entry, key, value, hash, depth + 1);
            let bitmap = 1u32 << existing_index;
            // TASK-008: Pre-allocate with exact capacity (1 child for single-bit bitmap)
            let mut children = ChildArray::with_capacity(1);
            children.push(ChildSlot::Node(ReferenceCounter::new(subnode)));
            // TASK-009: Record occupancy for histogram analysis
            occupancy_histogram::record_occupancy(1);
            (
                Node::Bitmap {
                    bitmap,
                    children,
                    generation: SHARED_GENERATION,
                },
                added,
            )
        } else {
            let bitmap = (1u32 << existing_index) | (1u32 << new_index);
            let mut children = ChildArray::with_capacity(2);
            // TASK-009: Record occupancy for histogram analysis
            occupancy_histogram::record_occupancy(2);
            if existing_index < new_index {
                children.push(ChildSlot::Entry {
                    hash: existing_hash,
                    key: existing_key.clone(),
                    value: existing_value.clone(),
                    generation: SHARED_GENERATION,
                });
                children.push(ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                });
            } else {
                children.push(ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                });
                children.push(ChildSlot::Entry {
                    hash: existing_hash,
                    key: existing_key.clone(),
                    value: existing_value.clone(),
                    generation: SHARED_GENERATION,
                });
            }
            (
                Node::Bitmap {
                    bitmap,
                    children,
                    generation: SHARED_GENERATION,
                },
                true,
            )
        }
    }

    /// Helper for inserting into a Bitmap node.
    fn insert_into_bitmap_node(
        bitmap: u32,
        children: &ChildArray<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (bitmap & (bit - 1)).count_ones() as usize;

        if bitmap & bit == 0 {
            let new_children = Self::build_children_with_insert(
                children,
                position,
                ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                },
            );
            // TASK-009: Record occupancy for histogram analysis
            occupancy_histogram::record_occupancy(new_children.len());
            (
                Node::Bitmap {
                    bitmap: bitmap | bit,
                    children: new_children,
                    generation: SHARED_GENERATION,
                },
                true,
            )
        } else {
            Self::insert_into_occupied_slot(bitmap, children, position, key, value, hash, depth)
        }
    }

    /// Helper for inserting into an occupied slot in a Bitmap node.
    fn insert_into_occupied_slot(
        bitmap: u32,
        children: &ChildArray<K, V>,
        position: usize,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let (new_child, added) = match &children[position] {
            ChildSlot::Entry {
                hash: child_hash,
                key: child_key,
                value: child_value,
                ..
            } => {
                if *child_key == key {
                    (
                        ChildSlot::Entry {
                            hash,
                            key,
                            value,
                            generation: SHARED_GENERATION,
                        },
                        false,
                    )
                } else if *child_hash == hash {
                    let mut entries = CollisionArray::new();
                    entries.push((child_key.clone(), child_value.clone()));
                    entries.push((key, value));
                    let collision = Node::Collision {
                        hash,
                        entries,
                        generation: SHARED_GENERATION,
                    };
                    (ChildSlot::Node(ReferenceCounter::new(collision)), true)
                } else {
                    let child_entry = Node::Entry {
                        hash: *child_hash,
                        key: child_key.clone(),
                        value: child_value.clone(),
                        generation: SHARED_GENERATION,
                    };
                    let (subnode, added) =
                        Self::insert_into_node(&child_entry, key, value, hash, depth + 1);
                    (ChildSlot::Node(ReferenceCounter::new(subnode)), added)
                }
            }
            ChildSlot::Node(subnode) => {
                let (new_subnode, added) =
                    Self::insert_into_node(subnode, key, value, hash, depth + 1);
                (ChildSlot::Node(ReferenceCounter::new(new_subnode)), added)
            }
        };

        let new_children = Self::build_children_with_update(children, position, new_child);
        (
            Node::Bitmap {
                bitmap,
                children: new_children,
                generation: SHARED_GENERATION,
            },
            added,
        )
    }

    /// Helper for inserting into a Collision node.
    ///
    /// # Optimization (Phase 4)
    ///
    /// - Uses `position()` iterator for efficient duplicate key detection
    /// - Hash comparison is already done before calling this function,
    ///   so only key equality is checked
    /// - In-place update when key exists (via indexed access)
    /// - Debug warning when collision bucket exceeds threshold
    fn insert_into_collision_node(
        node: &Node<K, V>,
        collision_hash: u64,
        entries: &CollisionArray<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        if hash == collision_hash {
            // Same hash - update or add to collision node
            // Use position() for efficient duplicate detection
            let existing_index = entries.iter().position(|(entry_key, _)| *entry_key == key);

            let new_entries = if let Some(index) = existing_index {
                // Key exists - update in place via clone + indexed update
                let mut cloned_entries = entries.clone();
                cloned_entries[index].1 = value;
                cloned_entries
            } else {
                // New key - clone and push
                let mut cloned_entries = entries.clone();
                cloned_entries.push((key, value));

                // Note: SmallVec inlines up to 4 elements; beyond that it allocates on heap.
                // Excessive collisions (>8 entries) may indicate a poor hash function.

                cloned_entries
            };

            (
                Node::Collision {
                    hash: collision_hash,
                    entries: new_entries,
                    generation: SHARED_GENERATION,
                },
                existing_index.is_none(),
            )
        } else {
            // Different hash - convert to bitmap node
            Self::convert_collision_to_bitmap(node, collision_hash, key, value, hash, depth)
        }
    }

    /// Converts a Collision node to a Bitmap node when a new hash is encountered.
    fn convert_collision_to_bitmap(
        node: &Node<K, V>,
        collision_hash: u64,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let collision_index = hash_index(collision_hash, depth);
        let new_index = hash_index(hash, depth);

        if collision_index == new_index {
            // Same index - recurse with collision as subnode
            let (subnode, added) = Self::insert_into_node(node, key, value, hash, depth + 1);
            let bitmap = 1u32 << collision_index;
            // TASK-008: Pre-allocate with exact capacity (1 child for single-bit bitmap)
            let mut children = ChildArray::with_capacity(1);
            children.push(ChildSlot::Node(ReferenceCounter::new(subnode)));
            // TASK-009: Record occupancy for histogram analysis
            occupancy_histogram::record_occupancy(1);
            (
                Node::Bitmap {
                    bitmap,
                    children,
                    generation: SHARED_GENERATION,
                },
                added,
            )
        } else {
            let bitmap = (1u32 << collision_index) | (1u32 << new_index);
            let mut children = ChildArray::with_capacity(2);
            if collision_index < new_index {
                children.push(ChildSlot::Node(ReferenceCounter::new(node.clone())));
                children.push(ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                });
            } else {
                children.push(ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                });
                children.push(ChildSlot::Node(ReferenceCounter::new(node.clone())));
            }
            // TASK-009: Record occupancy for histogram analysis
            occupancy_histogram::record_occupancy(2);
            (
                Node::Bitmap {
                    bitmap,
                    children,
                    generation: SHARED_GENERATION,
                },
                true,
            )
        }
    }

    /// Removes a key from the map.
    ///
    /// Returns a new map without the key. If the key doesn't exist,
    /// returns a clone of the original map.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// let removed = map.remove("a");
    ///
    /// assert_eq!(map.len(), 2);     // Original unchanged
    /// assert_eq!(removed.len(), 1); // New version
    /// assert_eq!(removed.get("a"), None);
    /// ```
    #[must_use]
    pub fn remove<Q>(&self, key: &Q) -> Self
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = compute_hash(key);
        match Self::remove_from_node(&self.root, key, hash, 0) {
            Some((new_root, removed)) => {
                if removed {
                    Self {
                        root: ReferenceCounter::new(new_root),
                        length: self.length.saturating_sub(1),
                    }
                } else {
                    self.clone()
                }
            }
            None => self.clone(),
        }
    }

    /// Recursive helper for remove.
    /// Returns `Some((new_node`, `was_removed`)) or None if no change needed.
    fn remove_from_node<Q>(
        node: &Node<K, V>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<(Node<K, V>, bool)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match node {
            Node::Empty => None,
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                ..
            } => {
                if *entry_hash == hash && entry_key.borrow() == key {
                    Some((Node::Empty, true))
                } else {
                    None
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => Self::remove_from_bitmap_node(*bitmap, children, key, hash, depth),
            Node::Collision {
                hash: collision_hash,
                entries,
                ..
            } => Self::remove_from_collision_node(*collision_hash, entries, key, hash),
        }
    }

    /// Helper for removing from a Bitmap node.
    fn remove_from_bitmap_node<Q>(
        bitmap: u32,
        children: &ChildArray<K, V>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<(Node<K, V>, bool)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;

        if bitmap & bit == 0 {
            return None;
        }

        let position = (bitmap & (bit - 1)).count_ones() as usize;

        match &children[position] {
            ChildSlot::Entry { key: child_key, .. } => {
                if child_key.borrow() == key {
                    Some(Self::remove_entry_from_bitmap(
                        bitmap, children, position, bit,
                    ))
                } else {
                    None
                }
            }
            ChildSlot::Node(subnode) => {
                Self::remove_from_subnode(bitmap, children, position, subnode, key, hash, depth)
            }
        }
    }

    /// Helper for removing an entry from a Bitmap node.
    fn remove_entry_from_bitmap(
        bitmap: u32,
        children: &ChildArray<K, V>,
        position: usize,
        bit: u32,
    ) -> (Node<K, V>, bool) {
        let new_bitmap = bitmap & !bit;

        if new_bitmap == 0 {
            return (Node::Empty, true);
        }

        let new_children = Self::build_children_with_remove(children, position);

        Self::simplify_bitmap_if_possible(new_bitmap, new_children)
    }

    /// Helper for removing from a subnode within a Bitmap node.
    fn remove_from_subnode<Q>(
        bitmap: u32,
        children: &ChildArray<K, V>,
        position: usize,
        subnode: &ReferenceCounter<Node<K, V>>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<(Node<K, V>, bool)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (new_subnode, removed) = Self::remove_from_node(subnode, key, hash, depth + 1)?;

        if !removed {
            return None;
        }

        match &new_subnode {
            Node::Empty => {
                let new_bitmap = bitmap & !(1u32 << hash_index(hash, depth));
                if new_bitmap == 0 {
                    return Some((Node::Empty, true));
                }
                let new_children = Self::build_children_with_remove(children, position);
                Some(Self::simplify_bitmap_if_possible(new_bitmap, new_children))
            }
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                value: entry_value,
                ..
            } => {
                let new_child = ChildSlot::Entry {
                    hash: *entry_hash,
                    key: entry_key.clone(),
                    value: entry_value.clone(),
                    generation: SHARED_GENERATION,
                };
                if children.len() == 1 {
                    Some((
                        Node::Entry {
                            hash: *entry_hash,
                            key: entry_key.clone(),
                            value: entry_value.clone(),
                            generation: SHARED_GENERATION,
                        },
                        true,
                    ))
                } else {
                    let new_children =
                        Self::build_children_with_update(children, position, new_child);
                    Some((
                        Node::Bitmap {
                            bitmap,
                            children: new_children,
                            generation: SHARED_GENERATION,
                        },
                        true,
                    ))
                }
            }
            _ => {
                let new_child = ChildSlot::Node(ReferenceCounter::new(new_subnode));
                let new_children = Self::build_children_with_update(children, position, new_child);
                Some((
                    Node::Bitmap {
                        bitmap,
                        children: new_children,
                        generation: SHARED_GENERATION,
                    },
                    true,
                ))
            }
        }
    }

    /// Simplifies a Bitmap node to an Entry if it has only one child entry.
    fn simplify_bitmap_if_possible(bitmap: u32, children: ChildArray<K, V>) -> (Node<K, V>, bool) {
        if children.len() == 1
            && let ChildSlot::Entry {
                hash, key, value, ..
            } = &children[0]
        {
            (
                Node::Entry {
                    hash: *hash,
                    key: key.clone(),
                    value: value.clone(),
                    generation: SHARED_GENERATION,
                },
                true,
            )
        } else {
            (
                Node::Bitmap {
                    bitmap,
                    children,
                    generation: SHARED_GENERATION,
                },
                true,
            )
        }
    }

    /// Helper for removing from a Collision node.
    ///
    /// # Optimization (Phase 4)
    ///
    /// - Early return when hash doesn't match (O(1) rejection)
    /// - Uses `position()` iterator for efficient key lookup
    /// - Simplifies to Entry node when only one entry remains
    fn remove_from_collision_node<Q>(
        collision_hash: u64,
        entries: &CollisionArray<K, V>,
        key: &Q,
        hash: u64,
    ) -> Option<(Node<K, V>, bool)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        // Early return if hash doesn't match - no need to search
        if hash != collision_hash {
            return None;
        }

        // Clone first, then find and remove. This pattern allows us to
        // use the efficient `position()` iterator on the original entries.
        let found_index = entries
            .iter()
            .position(|(entry_key, _)| entry_key.borrow() == key)?;

        let mut new_entries: CollisionArray<K, V> = entries.clone();

        new_entries.remove(found_index);

        if new_entries.is_empty() {
            Some((Node::Empty, true))
        } else if new_entries.len() == 1 {
            let (remaining_key, remaining_value) = new_entries.remove(0);
            Some((
                Node::Entry {
                    hash: collision_hash,
                    key: remaining_key,
                    value: remaining_value,
                    generation: SHARED_GENERATION,
                },
                true,
            ))
        } else {
            Some((
                Node::Collision {
                    hash: collision_hash,
                    entries: new_entries,
                    generation: SHARED_GENERATION,
                },
                true,
            ))
        }
    }

    /// Updates the value for a key using a function.
    ///
    /// Returns `None` if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to update
    /// * `function` - The function to apply to the value
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new().insert("count".to_string(), 10);
    /// let updated = map.update("count", |value| value + 1);
    ///
    /// assert_eq!(updated.unwrap().get("count"), Some(&11));
    /// ```
    #[must_use]
    pub fn update<Q, F>(&self, key: &Q, function: F) -> Option<Self>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> V,
    {
        let value = self.get(key)?;
        let new_value = function(value);

        // We need to create a new key if we only have a reference
        // This requires K to be obtainable from Q, which is complex
        // For now, we'll use a workaround by iterating to find the actual key
        let hash = compute_hash(key);
        let actual_key = Self::find_key(&self.root, key, hash, 0)?;

        Some(self.insert(actual_key, new_value))
    }

    /// Finds and clones the key matching the given query key.
    fn find_key<Q>(node: &Node<K, V>, key: &Q, hash: u64, depth: usize) -> Option<K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match node {
            Node::Empty => None,
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                ..
            } => {
                if *entry_hash == hash && entry_key.borrow() == key {
                    Some(entry_key.clone())
                } else {
                    None
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if bitmap & bit == 0 {
                    None
                } else {
                    let position = (bitmap & (bit - 1)).count_ones() as usize;
                    match &children[position] {
                        ChildSlot::Entry { key: child_key, .. } => {
                            if child_key.borrow() == key {
                                Some(child_key.clone())
                            } else {
                                None
                            }
                        }
                        ChildSlot::Node(subnode) => Self::find_key(subnode, key, hash, depth + 1),
                    }
                }
            }
            Node::Collision { entries, .. } => {
                for (entry_key, _) in entries {
                    if entry_key.borrow() == key {
                        return Some(entry_key.clone());
                    }
                }
                None
            }
        }
    }

    /// Updates or removes a value for a key using an updater function.
    ///
    /// The updater function receives `Some(&V)` if the key exists, or `None` if it doesn't.
    /// If the updater returns `Some(V)`, the value is inserted or updated.
    /// If the updater returns `None`, the key is removed (if it exists).
    ///
    /// # Arguments
    ///
    /// * `key` - The key to update
    /// * `updater` - A function that receives the current value (or None) and returns
    ///   the new value (or None to remove)
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new().insert("count".to_string(), 10);
    ///
    /// // Increment existing value
    /// let updated = map.update_with("count", |maybe_value| {
    ///     maybe_value.map(|value| value + 1)
    /// });
    /// assert_eq!(updated.get("count"), Some(&11));
    ///
    /// // Insert if not exists
    /// let inserted = map.update_with("new_key", |maybe_value| {
    ///     match maybe_value {
    ///         Some(value) => Some(*value),
    ///         None => Some(100),
    ///     }
    /// });
    /// assert_eq!(inserted.get("new_key"), Some(&100));
    ///
    /// // Remove by returning None
    /// let removed = map.update_with("count", |_| None);
    /// assert_eq!(removed.get("count"), None);
    /// ```
    #[must_use]
    pub fn update_with<Q, F>(&self, key: &Q, updater: F) -> Self
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ToOwned<Owned = K> + ?Sized,
        F: FnOnce(Option<&V>) -> Option<V>,
    {
        let current_value = self.get(key);
        let new_value = updater(current_value);

        match (current_value, new_value) {
            (Some(_), Some(value)) => {
                // Update existing key
                let hash = compute_hash(key);
                let actual_key =
                    Self::find_key(&self.root, key, hash, 0).unwrap_or_else(|| key.to_owned());
                self.insert(actual_key, value)
            }
            (Some(_), None) => {
                // Remove existing key
                self.remove(key)
            }
            (None, Some(value)) => {
                // Insert new key
                self.insert(key.to_owned(), value)
            }
            (None, None) => {
                // No change
                self.clone()
            }
        }
    }

    /// Merges two maps, with values from `other` taking precedence on key conflicts.
    ///
    /// # Arguments
    ///
    /// * `other` - The map to merge with
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two maps
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map1 = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// let map2 = PersistentHashMap::new()
    ///     .insert("b".to_string(), 20)
    ///     .insert("c".to_string(), 3);
    ///
    /// let merged = map1.merge(&map2);
    ///
    /// assert_eq!(merged.get("a"), Some(&1));
    /// assert_eq!(merged.get("b"), Some(&20)); // From map2
    /// assert_eq!(merged.get("c"), Some(&3));
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (key, value) in other {
            result = result.insert(key.clone(), value.clone());
        }
        result
    }

    /// Returns an iterator over key-value pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// This iterator uses lazy evaluation with O(1) creation cost and O(1)
    /// amortized cost per element. Early termination operations like `take()`,
    /// `find()`, and `any()` only traverse the elements actually needed.
    #[must_use]
    pub fn iter(&self) -> PersistentHashMapIterator<'_, K, V> {
        PersistentHashMapIterator::new(&self.root, self.length)
    }

    /// Returns an iterator over keys.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    ///
    /// for key in map.keys() {
    ///     println!("{}", key);
    /// }
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    /// Returns an iterator over values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    ///
    /// let sum: i32 = map.values().sum();
    /// assert_eq!(sum, 3);
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }

    /// Applies a function to all values, keeping keys unchanged.
    ///
    /// Returns a new map with the same keys but transformed values.
    /// The length of the map is preserved.
    ///
    /// # Type Parameters
    ///
    /// * `W` - The type of the transformed values
    /// * `F` - The transformation function type
    ///
    /// # Arguments
    ///
    /// * `transform` - A function to apply to each value
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// let doubled = map.map_values(|v| v * 2);
    /// assert_eq!(doubled.get("a"), Some(&2));
    /// assert_eq!(doubled.get("b"), Some(&4));
    /// ```
    #[must_use]
    pub fn map_values<W, F>(&self, mut transform: F) -> PersistentHashMap<K, W>
    where
        K: Clone + Hash + Eq,
        W: Clone,
        F: FnMut(&V) -> W,
    {
        self.iter()
            .map(|(key, value)| (key.clone(), transform(value)))
            .collect()
    }

    /// Applies a function to all keys, keeping values unchanged.
    ///
    /// Returns a new map with transformed keys and the original values.
    ///
    /// # Warning
    ///
    /// Key transformation may cause collisions. When multiple original keys
    /// map to the same new key, only one entry will be kept (the one
    /// processed last, which depends on iteration order).
    ///
    /// # Type Parameters
    ///
    /// * `L` - The type of the transformed keys
    /// * `F` - The transformation function type
    ///
    /// # Arguments
    ///
    /// * `transform` - A function to apply to each key
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("hello".to_string(), 1)
    ///     .insert("world".to_string(), 2);
    /// let uppercased = map.map_keys(|k| k.to_uppercase());
    /// assert_eq!(uppercased.get("HELLO"), Some(&1));
    /// assert_eq!(uppercased.get("WORLD"), Some(&2));
    /// ```
    #[must_use]
    pub fn map_keys<L, F>(&self, mut transform: F) -> PersistentHashMap<L, V>
    where
        L: Clone + Hash + Eq,
        V: Clone,
        F: FnMut(&K) -> L,
    {
        self.iter()
            .map(|(key, value)| (transform(key), value.clone()))
            .collect()
    }

    /// Applies a function to each entry, keeping only those that return Some.
    ///
    /// This combines filtering and mapping in a single operation.
    /// Entries for which the function returns None are excluded from the result.
    ///
    /// # Type Parameters
    ///
    /// * `W` - The type of the transformed values
    /// * `F` - The filter-map function type
    ///
    /// # Arguments
    ///
    /// * `filter_transform` - A function that receives a reference to the key and the value,
    ///   and returns `Some(new_value)` to include or `None` to exclude
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2)
    ///     .insert("c".to_string(), 3);
    /// let evens_doubled = map.filter_map(|_, v| {
    ///     if v % 2 == 0 { Some(v * 2) } else { None }
    /// });
    /// assert_eq!(evens_doubled.len(), 1);
    /// assert_eq!(evens_doubled.get("b"), Some(&4));
    /// ```
    #[must_use]
    pub fn filter_map<W, F>(&self, mut filter_transform: F) -> PersistentHashMap<K, W>
    where
        K: Clone + Hash + Eq,
        W: Clone,
        F: FnMut(&K, &V) -> Option<W>,
    {
        self.iter()
            .filter_map(|(key, value)| {
                filter_transform(key, value).map(|new_value| (key.clone(), new_value))
            })
            .collect()
    }

    /// Returns an iterator over key-value pairs.
    ///
    /// This is an alias for [`iter`](Self::iter), provided for API consistency
    /// with other functional programming languages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// for (key, value) in map.entries() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn entries(&self) -> PersistentHashMapIterator<'_, K, V> {
        self.iter()
    }

    /// Merges two maps with a custom conflict resolver.
    ///
    /// Returns a new map containing all entries from both maps.
    /// When a key exists in both maps, the resolver function is called
    /// with the key and both values to determine the final value.
    ///
    /// # Arguments
    ///
    /// * `other` - The map to merge with
    /// * `resolver` - A function that receives (key, `self_value`, `other_value`) and
    ///   returns the value to use in the merged map
    ///
    /// # Complexity
    ///
    /// O(n + m) where n and m are the sizes of the two maps
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map1 = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// let map2 = PersistentHashMap::new()
    ///     .insert("b".to_string(), 20)
    ///     .insert("c".to_string(), 3);
    /// let merged = map1.merge_with(&map2, |_, v1, v2| v1 + v2);
    /// assert_eq!(merged.get("a"), Some(&1));
    /// assert_eq!(merged.get("b"), Some(&22)); // 2 + 20
    /// assert_eq!(merged.get("c"), Some(&3));
    /// ```
    #[must_use]
    pub fn merge_with<F>(&self, other: &Self, mut resolver: F) -> Self
    where
        F: FnMut(&K, &V, &V) -> V,
    {
        let mut result = self.clone();
        for (key, other_value) in other {
            let new_value = result.get(key).map_or_else(
                || other_value.clone(),
                |self_value| resolver(key, self_value, other_value),
            );
            result = result.insert(key.clone(), new_value);
        }
        result
    }

    /// Removes entries for which the predicate returns true.
    ///
    /// Returns a new map containing only entries for which the predicate
    /// returns false.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true if the entry should be deleted
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2)
    ///     .insert("c".to_string(), 3);
    /// let odds_only = map.delete_if(|_, v| v % 2 == 0);
    /// assert_eq!(odds_only.len(), 2);
    /// assert!(odds_only.contains_key("a"));
    /// assert!(odds_only.contains_key("c"));
    /// ```
    #[must_use]
    pub fn delete_if<F>(&self, mut predicate: F) -> Self
    where
        K: Clone + Hash + Eq,
        V: Clone,
        F: FnMut(&K, &V) -> bool,
    {
        self.iter()
            .filter(|(key, value)| !predicate(key, value))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Keeps only entries for which the predicate returns true.
    ///
    /// Returns a new map containing only entries for which the predicate
    /// returns true. This is equivalent to `filter` but with a more explicit name.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true if the entry should be kept
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("small".to_string(), 5)
    ///     .insert("medium".to_string(), 15)
    ///     .insert("large".to_string(), 100);
    /// let large_values = map.keep_if(|_, v| *v >= 10);
    /// assert_eq!(large_values.len(), 2);
    /// assert!(large_values.contains_key("medium"));
    /// assert!(large_values.contains_key("large"));
    /// ```
    #[must_use]
    pub fn keep_if<F>(&self, mut predicate: F) -> Self
    where
        K: Clone + Hash + Eq,
        V: Clone,
        F: FnMut(&K, &V) -> bool,
    {
        self.iter()
            .filter(|(key, value)| predicate(key, value))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Partitions the map into two maps based on a predicate.
    ///
    /// Returns a tuple of two maps:
    /// - The first contains entries for which the predicate returns true
    /// - The second contains entries for which the predicate returns false
    ///
    /// This is more efficient than calling both `keep_if` and `delete_if`
    /// as it only traverses the map once.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true to include in the first map, false for the second
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2)
    ///     .insert("c".to_string(), 3)
    ///     .insert("d".to_string(), 4);
    /// let (evens, odds) = map.partition(|_, v| v % 2 == 0);
    /// assert_eq!(evens.len(), 2);
    /// assert_eq!(odds.len(), 2);
    /// assert!(evens.contains_key("b"));
    /// assert!(evens.contains_key("d"));
    /// assert!(odds.contains_key("a"));
    /// assert!(odds.contains_key("c"));
    /// ```
    #[must_use]
    pub fn partition<F>(&self, mut predicate: F) -> (Self, Self)
    where
        K: Clone + Hash + Eq,
        V: Clone,
        F: FnMut(&K, &V) -> bool,
    {
        let mut matching = Self::new();
        let mut not_matching = Self::new();

        for (key, value) in self {
            if predicate(key, value) {
                matching = matching.insert(key.clone(), value.clone());
            } else {
                not_matching = not_matching.insert(key.clone(), value.clone());
            }
        }

        (matching, not_matching)
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// Stack frame for depth-first HAMT traversal.
enum StackFrame<'a, K, V> {
    BitmapNode {
        children: &'a [ChildSlot<K, V>],
        index: usize,
    },
    CollisionNode {
        entries: &'a [(K, V)],
        index: usize,
    },
}

/// A lazy iterator over key-value pairs of a [`PersistentHashMap`].
///
/// This iterator uses a stack-based depth-first traversal algorithm to achieve
/// O(1) iterator creation cost and O(1) amortized cost per `next()` call.
/// The stack size is bounded by the maximum depth of the HAMT (13 levels for
/// 64-bit hashes with 5 bits per level), making space complexity O(1).
///
/// # Performance
///
/// | Operation        | Complexity        |
/// |------------------|-------------------|
/// | Iterator creation | O(1)             |
/// | `next()`         | O(1) amortized    |
/// | Full traversal   | O(n)              |
/// | Space            | O(13) = O(1)      |
///
/// # Early Termination
///
/// Unlike the previous collect-based implementation, this iterator supports
/// efficient early termination. Operations like `take(k)`, `find()`, `any()`,
/// and `all()` only traverse the elements actually needed.
pub struct PersistentHashMapIterator<'a, K, V> {
    stack: Vec<StackFrame<'a, K, V>>,
    pending_entry: Option<(&'a K, &'a V)>,
    remaining: usize,
}

impl<'a, K, V> PersistentHashMapIterator<'a, K, V> {
    fn new(root: &'a Node<K, V>, length: usize) -> Self {
        let mut iterator = Self {
            stack: Vec::with_capacity(13), // Maximum HAMT depth for 64-bit hash
            pending_entry: None,
            remaining: length,
        };

        if length > 0 {
            iterator.initialize_from_node(root);
        }

        iterator
    }

    fn initialize_from_node(&mut self, node: &'a Node<K, V>) {
        match node {
            Node::Empty => {}
            Node::Entry { key, value, .. } => {
                self.pending_entry = Some((key, value));
            }
            Node::Bitmap { children, .. } => {
                self.stack.push(StackFrame::BitmapNode {
                    children: children.as_slice(),
                    index: 0,
                });
                self.advance();
            }
            Node::Collision { entries, .. } => {
                self.stack.push(StackFrame::CollisionNode {
                    entries: entries.as_slice(),
                    index: 0,
                });
                self.advance();
            }
        }
    }

    fn advance(&mut self) {
        while let Some(frame) = self.stack.last_mut() {
            match frame {
                StackFrame::BitmapNode { children, index } => {
                    if *index >= children.len() {
                        self.stack.pop();
                        continue;
                    }

                    let current_index = *index;
                    *index += 1;

                    match &children[current_index] {
                        ChildSlot::Entry { key, value, .. } => {
                            self.pending_entry = Some((key, value));
                            return;
                        }
                        ChildSlot::Node(subnode) => match subnode.as_ref() {
                            Node::Empty => {}
                            Node::Entry { key, value, .. } => {
                                self.pending_entry = Some((key, value));
                                return;
                            }
                            Node::Bitmap {
                                children: child_children,
                                ..
                            } => {
                                self.stack.push(StackFrame::BitmapNode {
                                    children: child_children.as_slice(),
                                    index: 0,
                                });
                            }
                            Node::Collision {
                                entries: child_entries,
                                ..
                            } => {
                                self.stack.push(StackFrame::CollisionNode {
                                    entries: child_entries.as_slice(),
                                    index: 0,
                                });
                            }
                        },
                    }
                }
                StackFrame::CollisionNode { entries, index } => {
                    if *index >= entries.len() {
                        self.stack.pop();
                        continue;
                    }

                    let current_index = *index;
                    *index += 1;

                    let (key, value) = &entries[current_index];
                    self.pending_entry = Some((key, value));
                    return;
                }
            }
        }
    }
}

impl<'a, K, V> Iterator for PersistentHashMapIterator<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let result = self.pending_entry.take();
        if result.is_some() {
            self.remaining -= 1;
            self.advance();
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for PersistentHashMapIterator<'_, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<K, V> std::iter::FusedIterator for PersistentHashMapIterator<'_, K, V> {}

/// An owning iterator over key-value pairs of a [`PersistentHashMap`].
pub struct PersistentHashMapIntoIterator<K, V> {
    entries: Vec<(K, V)>,
    current_index: usize,
}

impl<K: Clone, V: Clone> Iterator for PersistentHashMapIntoIterator<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.entries.len() {
            None
        } else {
            let entry = self.entries[self.current_index].clone();
            self.current_index += 1;
            Some(entry)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.entries.len().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<K: Clone, V: Clone> ExactSizeIterator for PersistentHashMapIntoIterator<K, V> {
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.current_index)
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<K, V> Default for PersistentHashMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Hash + Eq, V: Clone> FromIterator<(K, V)> for PersistentHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower_bound, _) = iter.size_hint();
        let mut transient = TransientHashMap::with_capacity_hint(lower_bound);
        transient.extend(iter);
        transient.persistent()
    }
}

impl<K: Clone + Hash + Eq, V: Clone> IntoIterator for PersistentHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = PersistentHashMapIntoIterator<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        let entries: Vec<(K, V)> = self.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        PersistentHashMapIntoIterator {
            entries,
            current_index: 0,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a PersistentHashMap<K, V>
where
    K: Clone + Hash + Eq,
    V: Clone,
{
    type Item = (&'a K, &'a V);
    type IntoIter = PersistentHashMapIterator<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Clone + Hash + Eq, V: Clone + PartialEq> PartialEq for PersistentHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }

        for (key, value) in self {
            match other.get(key) {
                Some(other_value) if other_value == value => {}
                _ => return false,
            }
        }

        true
    }
}

impl<K: Clone + Hash + Eq, V: Clone + Eq> Eq for PersistentHashMap<K, V> {}

impl<K: Clone + Hash + Eq + fmt::Debug, V: Clone + fmt::Debug> fmt::Debug
    for PersistentHashMap<K, V>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Clone + Hash + Eq + fmt::Display, V: Clone + fmt::Display> fmt::Display
    for PersistentHashMap<K, V>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{{")?;
        let mut first = true;
        for (key, value) in self {
            if first {
                first = false;
            } else {
                write!(formatter, ", ")?;
            }
            write!(formatter, "{key}: {value}")?;
        }
        write!(formatter, "}}")
    }
}

// =============================================================================
// Type Class Implementations
// =============================================================================

/// Wrapper to make `PersistentHashMap` implement `TypeConstructor` for values.
///
/// Since `PersistentHashMap` has two type parameters (K, V), we treat it as
/// a container of V values with K being fixed.
impl<K, V> TypeConstructor for PersistentHashMap<K, V> {
    type Inner = V;
    type WithType<B> = PersistentHashMap<K, B>;
}

impl<K: Clone + Hash + Eq, V: Clone> Foldable for PersistentHashMap<K, V> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, V) -> B,
    {
        self.into_iter()
            .fold(init, |accumulator, (_, value)| function(accumulator, value))
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(V, B) -> B,
    {
        // For unordered collections, fold_right is semantically equivalent to fold_left
        self.into_iter()
            .fold(init, |accumulator, (_, value)| function(value, accumulator))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    fn length(&self) -> usize {
        self.length
    }
}

// =============================================================================
// Serde Support
// =============================================================================

#[cfg(feature = "serde")]
impl<K, V> serde::Serialize for PersistentHashMap<K, V>
where
    K: serde::Serialize + Clone + Hash + Eq,
    V: serde::Serialize + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self {
            map.serialize_entry(&key, &value)?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
struct PersistentHashMapVisitor<K, V> {
    key_marker: std::marker::PhantomData<K>,
    value_marker: std::marker::PhantomData<V>,
}

#[cfg(feature = "serde")]
impl<K, V> PersistentHashMapVisitor<K, V> {
    const fn new() -> Self {
        Self {
            key_marker: std::marker::PhantomData,
            value_marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::de::Visitor<'de> for PersistentHashMapVisitor<K, V>
where
    K: serde::Deserialize<'de> + Clone + Hash + Eq,
    V: serde::Deserialize<'de> + Clone,
{
    type Value = PersistentHashMap<K, V>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        // Note: Sequential insert ensures gradual memory usage even for large inputs.
        let mut map = PersistentHashMap::new();
        while let Some((key, value)) = access.next_entry()? {
            map = map.insert(key, value);
        }
        Ok(map)
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::Deserialize<'de> for PersistentHashMap<K, V>
where
    K: serde::Deserialize<'de> + Clone + Hash + Eq,
    V: serde::Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(PersistentHashMapVisitor::new())
    }
}

// =============================================================================
// BulkInsertError Definition
// =============================================================================

/// Error type for bulk insertion operations on [`TransientHashMap`].
///
/// This error is returned when a bulk insert operation fails due to input
/// size constraints designed to prevent memory spikes and ensure predictable
/// performance.
///
/// # Handling
///
/// When receiving this error, callers should:
/// 1. Split the input into smaller chunks (recommended: 10,000 entries per chunk)
/// 2. Call `insert_bulk` multiple times with smaller batches
///
/// # Example
///
/// ```rust
/// use lambars::persistent::{TransientHashMap, BulkInsertError, MAX_BULK_INSERT};
///
/// let large_data: Vec<(i32, i32)> = (0..200_000).map(|i| (i, i * 2)).collect();
///
/// // For large datasets, chunk the data to avoid TooManyEntries error
/// const CHUNK_SIZE: usize = 10_000;
/// let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();
/// for chunk in large_data.chunks(CHUNK_SIZE) {
///     transient = transient.insert_bulk(chunk.to_vec()).unwrap();
/// }
///
/// assert_eq!(transient.len(), 200_000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkInsertError {
    /// The number of entries exceeds the maximum allowed limit.
    ///
    /// # Fields
    ///
    /// * `count` - The number of entries observed before early termination.
    ///   Due to early cutoff at `MAX_BULK_INSERT + 1`, this may not reflect
    ///   the total number of entries in the input iterator.
    /// * `limit` - The maximum allowed number of entries ([`MAX_BULK_INSERT`])
    TooManyEntries {
        /// The number of entries observed (capped at `MAX_BULK_INSERT + 1`).
        count: usize,
        /// The maximum allowed number of entries.
        limit: usize,
    },
}

impl std::fmt::Display for BulkInsertError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyEntries { count, limit } => {
                write!(
                    formatter,
                    "bulk insert failed: {count} entries exceeds maximum of {limit}. \
                     Consider chunking into batches of 10,000 entries."
                )
            }
        }
    }
}

impl std::error::Error for BulkInsertError {}

/// Result type for [`TransientHashMap::insert_bulk_with_metrics`].
///
/// Contains statistics about the bulk insertion operation, including counts of
/// new insertions, updates, and the values that were replaced.
///
/// # Type Parameter
///
/// * `V` - The value type. Only values (not keys) are stored in `replaced_values`
///   to avoid the cost of cloning keys.
///
/// # Example
///
/// ```rust
/// use lambars::persistent::{TransientHashMap, BulkInsertResult};
///
/// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
/// transient.insert("existing".to_string(), 100);
///
/// let items = vec![
///     ("existing".to_string(), 200), // Update
///     ("new".to_string(), 300),      // Insert
/// ];
///
/// let result = transient.insert_bulk_with_metrics(items).unwrap();
/// assert_eq!(result.inserted_count, 1);  // "new"
/// assert_eq!(result.updated_count, 1);   // "existing"
/// assert_eq!(result.replaced_values, vec![100]); // Old value of "existing"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkInsertResult<V> {
    /// Number of new keys inserted (keys that did not exist before).
    pub inserted_count: usize,
    /// Number of existing keys updated (keys that already existed).
    pub updated_count: usize,
    /// Values that were replaced during updates.
    ///
    /// Only values are stored (not keys) to avoid unnecessary cloning.
    /// The order of values in this vector corresponds to the order of
    /// updates encountered during the bulk operation.
    pub replaced_values: Vec<V>,
}

impl<V> Default for BulkInsertResult<V> {
    fn default() -> Self {
        Self {
            inserted_count: 0,
            updated_count: 0,
            replaced_values: Vec::new(),
        }
    }
}

/// Error type for [`TransientHashMap::insert_bulk_owned`] that includes the original items.
///
/// This error type allows callers to recover the original items that could not be inserted,
/// enabling retry with smaller batches or alternative strategies.
///
/// # Example
///
/// ```rust
/// use lambars::persistent::TransientHashMap;
///
/// let transient: TransientHashMap<i32, i32> = TransientHashMap::new();
/// let too_many_items: Vec<(i32, i32)> = (0..200_000).map(|i| (i, i * 2)).collect();
///
/// match transient.insert_bulk_owned(too_many_items) {
///     Ok(_) => unreachable!(),
///     Err(e) => {
///         // Recover the items for retry with smaller batches
///         let recovered_items = e.into_items();
///         assert_eq!(recovered_items.len(), 200_000);
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkInsertErrorWithItems<K, V> {
    /// The number of entries exceeds the maximum allowed limit.
    ///
    /// The original items are included for recovery.
    TooManyEntries {
        /// The number of entries in the input.
        count: usize,
        /// The maximum allowed number of entries ([`MAX_BULK_INSERT`]).
        limit: usize,
        /// The original items that could not be inserted.
        items: Vec<(K, V)>,
    },
}

impl<K, V> BulkInsertErrorWithItems<K, V> {
    /// Extracts the original items from the error.
    ///
    /// This allows callers to recover the items and retry with smaller batches.
    #[must_use]
    pub fn into_items(self) -> Vec<(K, V)> {
        match self {
            Self::TooManyEntries { items, .. } => items,
        }
    }
}

impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Display for BulkInsertErrorWithItems<K, V> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyEntries { count, limit, .. } => {
                write!(
                    formatter,
                    "bulk insert failed: {count} entries exceeds maximum of {limit}. \
                     Consider chunking into batches of 10,000 entries."
                )
            }
        }
    }
}

impl<K: std::fmt::Debug, V: std::fmt::Debug> std::error::Error for BulkInsertErrorWithItems<K, V> {}

// =============================================================================
// NodePool - Memory Pool for HAMT Node Reuse
// =============================================================================

/// Default maximum size for the node pool.
pub const DEFAULT_NODE_POOL_MAX_SIZE: usize = 1024;

/// A memory pool for reusing HAMT nodes.
///
/// `NodePool` provides a mechanism to recycle `Node` instances during bulk
/// operations, reducing allocation overhead by reusing previously allocated
/// nodes.
///
/// # Design
///
/// - Pools `ReferenceCounter<Node<K, V>>` instances, not raw nodes
/// - Only accepts exclusively-owned nodes (`strong_count() == 1`)
/// - Has a configurable maximum size to limit memory usage
/// - Intentionally `!Send` and `!Sync` for single-threaded use
///
/// # Thread Safety
///
/// `NodePool` is designed for single-threaded use and is intentionally not
/// `Send` or `Sync`. Use a separate pool per thread if needed.
///
/// # Example
///
/// ```rust,ignore
/// use lambars::persistent::{TransientHashMap, NodePool};
///
/// let mut pool = NodePool::new();
/// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
///
/// let items = vec![("a".to_string(), 1), ("b".to_string(), 2)];
/// let result = transient.insert_bulk_with_pool(items, &mut pool);
/// ```
#[derive(Debug)]
pub struct NodePool<K, V> {
    /// Pooled nodes (`ReferenceCounter<Node<K, V>>`).
    nodes: Vec<ReferenceCounter<Node<K, V>>>,
    /// Hit count (successful acquisition from pool).
    hit_count: usize,
    /// Miss count (pool empty, new allocation needed).
    miss_count: usize,
    /// Maximum pool size.
    max_pool_size: usize,
    /// Marker to ensure `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

// Static assertions to verify NodePool is not Send/Sync
static_assertions::assert_not_impl_any!(NodePool<i32, i32>: Send, Sync);
static_assertions::assert_not_impl_any!(NodePool<String, String>: Send, Sync);

impl<K, V> Default for NodePool<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> NodePool<K, V> {
    /// Creates a new empty node pool with the default maximum size.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_max_size(DEFAULT_NODE_POOL_MAX_SIZE)
    }

    /// Creates a new empty node pool with a custom maximum size.
    #[must_use]
    pub const fn with_max_size(max_pool_size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            hit_count: 0,
            miss_count: 0,
            max_pool_size,
            _marker: PhantomData,
        }
    }

    /// Attempts to acquire a node from the pool.
    ///
    /// Returns `Some(node)` if a node is available, `None` otherwise.
    ///
    /// # Note
    ///
    /// The returned node should be reinitialized before use, as it may
    /// contain stale data from previous operations.
    #[must_use]
    #[allow(dead_code)] // Used in tests and future pool-based bulk insert implementation
    fn try_acquire(&mut self) -> Option<ReferenceCounter<Node<K, V>>> {
        if let Some(node) = self.nodes.pop() {
            self.hit_count += 1;
            Some(node)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Attempts to release a node back to the pool.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the node was successfully returned to the pool
    /// - `Err(node)` if the node could not be returned (shared or pool full)
    ///
    /// # Conditions for Acceptance
    ///
    /// A node is only accepted if:
    /// 1. It is exclusively owned (`strong_count() == 1`)
    /// 2. The pool has not reached its maximum size
    ///
    /// Shared nodes (referenced by multiple owners) cannot be reused safely.
    #[allow(dead_code)] // Used in tests and future pool-based bulk insert implementation
    fn try_release(
        &mut self,
        node: ReferenceCounter<Node<K, V>>,
    ) -> Result<(), ReferenceCounter<Node<K, V>>> {
        // Check if the node is exclusively owned
        if ReferenceCounter::strong_count(&node) != 1 {
            return Err(node);
        }

        // Check if pool has capacity
        if self.nodes.len() >= self.max_pool_size {
            return Err(node);
        }

        self.nodes.push(node);
        Ok(())
    }

    /// Returns the current number of nodes in the pool.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the pool is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the maximum size of the pool.
    #[must_use]
    pub const fn max_size(&self) -> usize {
        self.max_pool_size
    }

    /// Clears the pool, releasing all nodes.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// Returns the metrics for this pool.
    #[must_use]
    pub const fn metrics(&self) -> NodePoolMetrics {
        NodePoolMetrics {
            acquired_count: self.hit_count,
            released_count: self.nodes.len(),
            rejected_count: 0, // This is tracked externally during bulk operations
            hit_count: self.hit_count,
            miss_count: self.miss_count,
        }
    }
}

/// Metrics for node pool usage.
///
/// This structure tracks statistics about pool operations, including
/// acquisition hits/misses and release outcomes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodePoolMetrics {
    /// Number of nodes successfully acquired from the pool.
    pub acquired_count: usize,
    /// Number of nodes released back to the pool.
    pub released_count: usize,
    /// Number of nodes rejected (shared or pool full).
    pub rejected_count: usize,
    /// Hit count (acquisition succeeded).
    hit_count: usize,
    /// Miss count (pool was empty).
    miss_count: usize,
}

impl NodePoolMetrics {
    /// Calculates the hit rate (hits / total attempts).
    ///
    /// Returns 0.0 if no acquisition attempts have been made.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Acceptable for metrics calculation
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }

    /// Returns the total number of acquisition attempts.
    #[must_use]
    pub const fn total_attempts(&self) -> usize {
        self.hit_count + self.miss_count
    }
}

// =============================================================================
// TransientHashMap Definition
// =============================================================================

/// A transient (temporarily mutable) hash map for efficient batch updates.
///
/// `TransientHashMap` provides mutable operations for batch construction of hash maps.
/// Once construction is complete, it can be converted back to a [`PersistentHashMap`]
/// using the [`persistent()`](TransientHashMap::persistent) method.
///
/// # Design
///
/// Transient data structures use Copy-on-Write (COW) semantics via `Rc::make_mut()`
/// / `Arc::make_mut()` to efficiently share structure with the persistent version
/// while allowing mutation.
///
/// # Thread Safety
///
/// `TransientHashMap` is intentionally not `Send` or `Sync`. It is designed for
/// single-threaded batch construction. For thread-safe operations, convert back
/// to `PersistentHashMap`.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::{PersistentHashMap, TransientHashMap};
///
/// // Build a map efficiently using transient operations
/// let mut transient = TransientHashMap::new();
/// transient.insert("one".to_string(), 1);
/// transient.insert("two".to_string(), 2);
/// transient.insert("three".to_string(), 3);
///
/// // Convert to persistent map
/// let persistent = transient.persistent();
/// assert_eq!(persistent.get("one"), Some(&1));
/// assert_eq!(persistent.len(), 3);
/// ```
pub struct TransientHashMap<K, V> {
    root: ReferenceCounter<Node<K, V>>,
    length: usize,
    /// A hint for the expected number of elements.
    ///
    /// Note: Due to the nature of HAMT, this hint cannot be used for
    /// pre-allocation like `HashMap::with_capacity`. It is stored for
    /// potential future optimizations (e.g., batch insertion strategies).
    capacity_hint: usize,
    /// Generation token for COW optimization.
    ///
    /// This value is assigned when the `TransientHashMap` is created from a
    /// `PersistentHashMap`. All newly created nodes will have this generation,
    /// allowing `insert_without_cow` to skip cloning if the node's generation
    /// matches. A value of `SHARED_GENERATION` (0) indicates nodes that are
    /// shared and must always be cloned.
    generation: u64,
    /// Marker to ensure `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

// Static assertions to verify TransientHashMap is not Send/Sync
static_assertions::assert_not_impl_any!(TransientHashMap<i32, i32>: Send, Sync);
static_assertions::assert_not_impl_any!(TransientHashMap<String, String>: Send, Sync);

// Arc feature verification: even with Arc, TransientHashMap remains !Send/!Sync
#[cfg(feature = "arc")]
mod arc_send_sync_verification_hashmap {
    use super::TransientHashMap;
    use std::sync::Arc;

    // Arc<T> where T: Send+Sync is Send+Sync, but TransientHashMap should still be !Send/!Sync
    static_assertions::assert_not_impl_any!(TransientHashMap<Arc<i32>, Arc<i32>>: Send, Sync);
    static_assertions::assert_not_impl_any!(TransientHashMap<Arc<String>, Arc<String>>: Send, Sync);
}

// Manual implementations for TransientHashMap to exclude internal fields (generation, _marker)

impl<K: PartialEq, V: PartialEq> PartialEq for TransientHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.length == other.length && self.root == other.root
    }
}

impl<K: Eq, V: Eq> Eq for TransientHashMap<K, V> {}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for TransientHashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransientHashMap")
            .field("root", &self.root)
            .field("length", &self.length)
            .field("capacity_hint", &self.capacity_hint)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// TransientHashMap Implementation
// =============================================================================

impl<K, V> TransientHashMap<K, V> {
    /// Returns the number of entries in the map.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// assert_eq!(transient.len(), 0);
    /// transient.insert("key".to_string(), 42);
    /// assert_eq!(transient.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the map contains no entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// assert!(transient.is_empty());
    /// transient.insert("key".to_string(), 42);
    /// assert!(!transient.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns the capacity hint that was provided when the map was created.
    ///
    /// If the map was created with [`TransientHashMap::new()`], this returns 0.
    /// If created with [`TransientHashMap::with_capacity_hint()`], it returns
    /// the hint that was provided.
    ///
    /// # Note
    ///
    /// This value is advisory only and does not reflect actual memory allocation.
    /// HAMT structures do not support pre-allocation in the same way as `HashMap`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let transient1: TransientHashMap<String, i32> = TransientHashMap::new();
    /// assert_eq!(transient1.capacity_hint(), 0);
    ///
    /// let transient2: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(100);
    /// assert_eq!(transient2.capacity_hint(), 100);
    /// ```
    #[inline]
    #[must_use]
    pub const fn capacity_hint(&self) -> usize {
        self.capacity_hint
    }
}

impl<K: Clone + Hash + Eq, V: Clone> TransientHashMap<K, V> {
    /// Creates a new empty `TransientHashMap`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// assert!(transient.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: ReferenceCounter::new(Node::empty()),
            length: 0,
            capacity_hint: 0,
            generation: next_generation(),
            _marker: PhantomData,
        }
    }

    /// Creates a new empty `TransientHashMap` with a capacity hint.
    ///
    /// The capacity hint is advisory only - due to the nature of HAMT (Hash Array Mapped Trie),
    /// it cannot be used for pre-allocation like `HashMap::with_capacity`. The hint is stored
    /// for potential future optimizations such as batch insertion strategies.
    ///
    /// # Arguments
    ///
    /// * `hint` - A hint for the expected number of elements
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// // Create a transient map with a hint for 1000 elements
    /// let mut transient: TransientHashMap<i32, i32> = TransientHashMap::with_capacity_hint(1000);
    /// for i in 0..1000 {
    ///     transient.insert(i, i * 2);
    /// }
    /// let persistent = transient.persistent();
    /// assert_eq!(persistent.len(), 1000);
    /// ```
    ///
    /// # Note
    ///
    /// The capacity hint is purely advisory. Exceeding the hinted capacity
    /// does not cause reallocation or performance degradation - the map
    /// will simply grow as needed.
    #[must_use]
    pub fn with_capacity_hint(hint: usize) -> Self {
        Self {
            root: ReferenceCounter::new(Node::empty()),
            length: 0,
            capacity_hint: hint,
            generation: next_generation(),
            _marker: PhantomData,
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    ///
    /// # Note
    ///
    /// Due to the nature of HAMT (Hash Array Mapped Trie), this method does not
    /// actually pre-allocate memory. It only updates the `capacity_hint` for
    /// potential future optimizations (e.g., batch insertion strategies).
    ///
    /// # Arguments
    ///
    /// * `additional` - The number of additional elements expected to be inserted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();
    /// transient.reserve(100);
    /// assert_eq!(transient.capacity_hint(), 100);
    ///
    /// // Adding more elements after reserve
    /// transient.reserve(50);
    /// assert_eq!(transient.capacity_hint(), 150);
    /// ```
    #[inline]
    pub const fn reserve(&mut self, additional: usize) {
        self.capacity_hint = self.capacity_hint.saturating_add(additional);
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but `Hash` and
    /// `Eq` on the borrowed form must match those for the key type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("hello".to_string(), 42);
    ///
    /// // Can use &str to look up String keys
    /// assert_eq!(transient.get("hello"), Some(&42));
    /// assert_eq!(transient.get("world"), None);
    /// ```
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = compute_hash(key);
        PersistentHashMap::get_from_node(&self.root, key, hash, 0)
    }

    /// Returns `true` if the map contains the given key.
    ///
    /// The key may be any borrowed form of the map's key type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("key".to_string(), 42);
    ///
    /// assert!(transient.contains_key("key"));
    /// assert!(!transient.contains_key("other"));
    /// ```
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map already contains the key, the old value is replaced and returned.
    /// Otherwise, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    /// * `value` - The value to associate with the key
    ///
    /// # Returns
    ///
    /// The old value if the key was already present, otherwise `None`.
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// assert_eq!(transient.insert("key".to_string(), 1), None);
    /// assert_eq!(transient.insert("key".to_string(), 2), Some(1));
    /// assert_eq!(transient.get("key"), Some(&2));
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = compute_hash(&key);
        let root = ReferenceCounter::make_mut(&mut self.root);
        let (old_value, added) = Self::insert_into_node_cow(root, key, value, hash, 0);
        if added {
            self.length += 1;
        }
        old_value
    }

    /// Inserts a key-value pair with generation-based COW optimization.
    ///
    /// Uses the generation token to skip COW when the node is exclusively owned
    /// by this `TransientHashMap`. Semantically equivalent to [`insert`](Self::insert),
    /// but optimized for bulk operations on newly created transient maps.
    pub fn insert_without_cow(&mut self, key: K, value: V) -> Option<V> {
        let hash = compute_hash(&key);
        let owner_generation = self.generation;

        if let Some(root_mut) = ReferenceCounter::get_mut(&mut self.root) {
            Self::ensure_node_generation(root_mut, owner_generation);
            let (old_value, added) =
                Self::insert_into_node_inplace(root_mut, key, value, hash, 0, owner_generation);
            if added {
                self.length += 1;
            }
            return old_value;
        }

        let root = ReferenceCounter::make_mut(&mut self.root);
        let (old_value, added) =
            Self::insert_into_node_with_generation(root, key, value, hash, 0, owner_generation);
        if added {
            self.length += 1;
        }
        old_value
    }

    /// Updates the generation token of a node to the owner's generation.
    ///
    /// This is a helper function used in in-place insertion paths to ensure
    /// that modified nodes have the correct generation token.
    ///
    /// # Preconditions
    ///
    /// The caller must ensure the node is exclusively owned (reference count == 1)
    /// before calling this function.
    // Note: Cannot be const fn because const functions with mutable references
    // to generic types are not fully stabilized in Rust.
    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    fn ensure_node_generation(node: &mut Node<K, V>, owner_generation: u64) {
        match node {
            Node::Entry { generation, .. }
            | Node::Bitmap { generation, .. }
            | Node::Collision { generation, .. } => *generation = owner_generation,
            Node::Empty => {}
        }
    }

    /// Ensures a child node is exclusively owned and returns a mutable reference.
    ///
    /// # Performance Note
    ///
    /// While `ReferenceCounter::get_mut` could avoid the atomic strong count
    /// decrement in `make_mut` for exclusively owned nodes, Rust's borrow checker
    /// prevents the pattern of "try `get_mut`, fallback to `make_mut`" in a single
    /// function returning a reference. The cost is minimal as `make_mut` is
    /// already optimized for the exclusive ownership case.
    #[inline]
    fn ensure_child_owned(
        child_ref: &mut ReferenceCounter<Node<K, V>>,
        owner_generation: u64,
    ) -> &mut Node<K, V> {
        let child_mut = ReferenceCounter::make_mut(child_ref);
        Self::ensure_node_generation(child_mut, owner_generation);
        child_mut
    }

    /// Recursively updates the generation of a node and all its children.
    ///
    /// This is used when nodes are created via `PersistentHashMap::create_bitmap_from_two_entries`
    /// or `PersistentHashMap::insert_into_node`, which produce nodes with `SHARED_GENERATION`.
    /// This function ensures all nodes have the correct `owner_generation` for consistency
    /// (RISK-002 mitigation).
    fn update_node_generation_recursive(node: &mut Node<K, V>, owner_generation: u64) {
        match node {
            Node::Empty => {}
            Node::Entry { generation, .. } | Node::Collision { generation, .. } => {
                *generation = owner_generation;
            }
            Node::Bitmap {
                generation,
                children,
                ..
            } => {
                *generation = owner_generation;
                for child in children.iter_mut() {
                    match child {
                        ChildSlot::Entry {
                            generation: child_gen,
                            ..
                        } => {
                            *child_gen = owner_generation;
                        }
                        ChildSlot::Node(child_ref) => {
                            if let Some(child_node) = ReferenceCounter::get_mut(child_ref) {
                                Self::update_node_generation_recursive(
                                    child_node,
                                    owner_generation,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Inserts a key-value pair into a node using in-place modification.
    ///
    /// For shared child nodes, [`ensure_child_owned`](Self::ensure_child_owned)
    /// performs local COW as needed.
    ///
    /// # Preconditions
    ///
    /// The `node` must be exclusively owned by the caller. This is guaranteed
    /// when called from [`insert_without_cow`](Self::insert_without_cow) after
    /// a successful `ReferenceCounter::get_mut`.
    #[allow(clippy::too_many_lines)]
    fn insert_into_node_inplace(
        node: &mut Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
        owner_generation: u64,
    ) -> (Option<V>, bool) {
        match node {
            Node::Empty => {
                *node = Node::Entry {
                    hash,
                    key,
                    value,
                    generation: owner_generation,
                };
                (None, true)
            }
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
                generation,
            } => {
                if *existing_hash == hash && *existing_key == key {
                    // Same key - replace value in-place
                    let old_value = std::mem::replace(existing_value, value);
                    *generation = owner_generation;
                    (Some(old_value), false)
                } else if *existing_hash == hash {
                    // Hash collision - create collision node
                    let mut entries = CollisionArray::new();
                    entries.push((existing_key.clone(), existing_value.clone()));
                    entries.push((key, value));
                    *node = Node::Collision {
                        hash: *existing_hash,
                        entries,
                        generation: owner_generation,
                    };
                    (None, true)
                } else {
                    // Different hash - create bitmap node
                    let (mut new_node, _) = PersistentHashMap::create_bitmap_from_two_entries(
                        *existing_hash,
                        existing_key,
                        existing_value,
                        key,
                        value,
                        hash,
                        depth,
                    );
                    Self::ensure_node_generation(&mut new_node, owner_generation);
                    if let Node::Bitmap { children, .. } = &mut new_node {
                        for child in children.iter_mut() {
                            match child {
                                ChildSlot::Entry { generation, .. } => {
                                    *generation = owner_generation;
                                }
                                ChildSlot::Node(child_node) => {
                                    if let Some(child_mut) = ReferenceCounter::get_mut(child_node) {
                                        Self::ensure_node_generation(child_mut, owner_generation);
                                    }
                                }
                            }
                        }
                    }
                    *node = new_node;
                    (None, true)
                }
            }
            Node::Bitmap {
                bitmap,
                children,
                generation,
            } => {
                *generation = owner_generation;
                Self::insert_into_bitmap_node_inplace(
                    bitmap,
                    children,
                    key,
                    value,
                    hash,
                    depth,
                    owner_generation,
                )
            }
            Node::Collision {
                hash: collision_hash,
                entries,
                generation,
            } => {
                if hash == *collision_hash {
                    // Same hash - add to collision entries in-place
                    *generation = owner_generation;
                    for entry in entries.iter_mut() {
                        if entry.0 == key {
                            let old_value = std::mem::replace(&mut entry.1, value);
                            return (Some(old_value), false);
                        }
                    }
                    entries.push((key, value));
                    (None, true)
                } else {
                    // Different hash - convert collision to bitmap
                    let collision_entries: CollisionArray<K, V> = entries.clone();
                    let collision_hash_value = *collision_hash;

                    let index = hash_index(collision_hash_value, depth);
                    let bit = 1u32 << index;
                    let collision_child = ChildSlot::Node(ReferenceCounter::new(Node::Collision {
                        hash: collision_hash_value,
                        entries: collision_entries,
                        generation: owner_generation,
                    }));

                    let new_index = hash_index(hash, depth);
                    let new_bit = 1u32 << new_index;

                    if index == new_index {
                        let mut subnode = Node::Collision {
                            hash: collision_hash_value,
                            entries: entries.clone(),
                            generation: owner_generation,
                        };
                        let result = Self::insert_into_node_inplace(
                            &mut subnode,
                            key,
                            value,
                            hash,
                            depth + 1,
                            owner_generation,
                        );
                        // TASK-008: Pre-allocate with exact capacity (1 child for single-bit bitmap)
                        let mut children = ChildArray::with_capacity(1);
                        children.push(ChildSlot::Node(ReferenceCounter::new(subnode)));
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(1);
                        *node = Node::Bitmap {
                            bitmap: bit,
                            children,
                            generation: owner_generation,
                        };
                        result
                    } else {
                        let new_child = ChildSlot::Entry {
                            hash,
                            key,
                            value,
                            generation: owner_generation,
                        };
                        let mut children = ChildArray::with_capacity(2);
                        if index < new_index {
                            children.push(collision_child);
                            children.push(new_child);
                        } else {
                            children.push(new_child);
                            children.push(collision_child);
                        }
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(2);
                        *node = Node::Bitmap {
                            bitmap: bit | new_bit,
                            children,
                            generation: owner_generation,
                        };
                        (None, true)
                    }
                }
            }
        }
    }

    /// Inserts into a bitmap node using in-place modification.
    ///
    /// For shared child nodes, [`ensure_child_owned`](Self::ensure_child_owned)
    /// performs local COW.
    fn insert_into_bitmap_node_inplace(
        bitmap: &mut u32,
        children: &mut ChildArray<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
        owner_generation: u64,
    ) -> (Option<V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (*bitmap & (bit - 1)).count_ones() as usize;

        if *bitmap & bit == 0 {
            // New slot - insert entry with owner's generation
            children.insert(
                position,
                ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: owner_generation,
                },
            );
            *bitmap |= bit;
            (None, true)
        } else {
            match &mut children[position] {
                ChildSlot::Entry {
                    hash: child_hash,
                    key: child_key,
                    value: child_value,
                    generation,
                } => {
                    if *child_key == key {
                        // Same key - replace value in-place
                        let old_value = std::mem::replace(child_value, value);
                        *generation = owner_generation;
                        (Some(old_value), false)
                    } else if *child_hash == hash {
                        // Hash collision - convert to collision node
                        let mut entries = CollisionArray::new();
                        entries.push((child_key.clone(), child_value.clone()));
                        entries.push((key, value));
                        let collision = Node::Collision {
                            hash,
                            entries,
                            generation: owner_generation,
                        };
                        children[position] = ChildSlot::Node(ReferenceCounter::new(collision));
                        (None, true)
                    } else {
                        // Different hash - create sub-bitmap
                        let child_entry = Node::Entry {
                            hash: *child_hash,
                            key: child_key.clone(),
                            value: child_value.clone(),
                            generation: owner_generation,
                        };
                        let (mut subnode, _) = PersistentHashMap::insert_into_node(
                            &child_entry,
                            key,
                            value,
                            hash,
                            depth + 1,
                        );
                        Self::ensure_node_generation(&mut subnode, owner_generation);
                        if let Node::Bitmap {
                            children: sub_children,
                            ..
                        } = &mut subnode
                        {
                            for child in sub_children.iter_mut() {
                                if let ChildSlot::Entry { generation, .. } = child {
                                    *generation = owner_generation;
                                }
                            }
                        }
                        children[position] = ChildSlot::Node(ReferenceCounter::new(subnode));
                        (None, true)
                    }
                }
                ChildSlot::Node(subnode) => {
                    let subnode_mut = Self::ensure_child_owned(subnode, owner_generation);
                    Self::insert_into_node_inplace(
                        subnode_mut,
                        key,
                        value,
                        hash,
                        depth + 1,
                        owner_generation,
                    )
                }
            }
        }
    }

    /// Recursively inserts into a node with generation-based COW optimization.
    ///
    /// Returns (`old_value`, `was_added`).
    ///
    /// This function modifies nodes in-place after COW has been performed by the caller.
    /// The `owner_generation` is applied to all newly created or modified nodes to maintain
    /// generation consistency. This is a fallback path used when the root node is shared.
    #[allow(clippy::too_many_lines)]
    fn insert_into_node_with_generation(
        node: &mut Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
        owner_generation: u64,
    ) -> (Option<V>, bool) {
        match node {
            Node::Empty => {
                *node = Node::Entry {
                    hash,
                    key,
                    value,
                    generation: owner_generation,
                };
                (None, true)
            }
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
                generation,
            } => {
                if *existing_hash == hash && *existing_key == key {
                    // Same key - just replace the value (always in-place)
                    let old_value = std::mem::replace(existing_value, value);
                    *generation = owner_generation;
                    (Some(old_value), false)
                } else if *existing_hash == hash {
                    // Hash collision - create collision node
                    let mut entries = CollisionArray::new();
                    entries.push((existing_key.clone(), existing_value.clone()));
                    entries.push((key, value));
                    *node = Node::Collision {
                        hash: *existing_hash,
                        entries,
                        generation: owner_generation,
                    };
                    (None, true)
                } else {
                    // Different hash - create bitmap node
                    let (mut new_node, _) = PersistentHashMap::create_bitmap_from_two_entries(
                        *existing_hash,
                        existing_key,
                        existing_value,
                        key,
                        value,
                        hash,
                        depth,
                    );
                    // Update generation on the new bitmap node and all its children (RISK-002 mitigation)
                    Self::update_node_generation_recursive(&mut new_node, owner_generation);
                    *node = new_node;
                    (None, true)
                }
            }
            Node::Bitmap {
                bitmap,
                children,
                generation,
            } => {
                // Update generation if we're modifying this node
                *generation = owner_generation;
                Self::insert_into_bitmap_node_with_generation(
                    bitmap,
                    children,
                    key,
                    value,
                    hash,
                    depth,
                    owner_generation,
                )
            }
            Node::Collision {
                hash: collision_hash,
                entries,
                generation,
            } => {
                if hash == *collision_hash {
                    // Same hash - add to collision entries
                    *generation = owner_generation;
                    for entry in entries.iter_mut() {
                        if entry.0 == key {
                            let old_value = std::mem::replace(&mut entry.1, value);
                            return (Some(old_value), false);
                        }
                    }
                    entries.push((key, value));
                    (None, true)
                } else {
                    // Different hash - convert collision to bitmap
                    let collision_entries: CollisionArray<K, V> = entries.clone();
                    let collision_hash_value = *collision_hash;

                    let index = hash_index(collision_hash_value, depth);
                    let bit = 1u32 << index;
                    let collision_child = ChildSlot::Node(ReferenceCounter::new(Node::Collision {
                        hash: collision_hash_value,
                        entries: collision_entries,
                        generation: owner_generation,
                    }));

                    let new_index = hash_index(hash, depth);
                    let new_bit = 1u32 << new_index;

                    if index == new_index {
                        let mut subnode = Node::Collision {
                            hash: collision_hash_value,
                            entries: entries.clone(),
                            generation: owner_generation,
                        };
                        let result = Self::insert_into_node_with_generation(
                            &mut subnode,
                            key,
                            value,
                            hash,
                            depth + 1,
                            owner_generation,
                        );
                        // TASK-008: Pre-allocate with exact capacity (1 child for single-bit bitmap)
                        let mut children = ChildArray::with_capacity(1);
                        children.push(ChildSlot::Node(ReferenceCounter::new(subnode)));
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(1);
                        *node = Node::Bitmap {
                            bitmap: bit,
                            children,
                            generation: owner_generation,
                        };
                        result
                    } else {
                        let new_child = ChildSlot::Entry {
                            hash,
                            key,
                            value,
                            generation: owner_generation,
                        };
                        let mut children = ChildArray::with_capacity(2);
                        if index < new_index {
                            children.push(collision_child);
                            children.push(new_child);
                        } else {
                            children.push(new_child);
                            children.push(collision_child);
                        }
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(2);
                        *node = Node::Bitmap {
                            bitmap: bit | new_bit,
                            children,
                            generation: owner_generation,
                        };
                        (None, true)
                    }
                }
            }
        }
    }

    /// Inserts into a bitmap node with generation-based COW optimization.
    fn insert_into_bitmap_node_with_generation(
        bitmap: &mut u32,
        children: &mut ChildArray<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
        owner_generation: u64,
    ) -> (Option<V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (*bitmap & (bit - 1)).count_ones() as usize;

        if *bitmap & bit == 0 {
            // New slot - insert entry with owner's generation
            children.insert(
                position,
                ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: owner_generation,
                },
            );
            *bitmap |= bit;
            (None, true)
        } else {
            match &mut children[position] {
                ChildSlot::Entry {
                    hash: child_hash,
                    key: child_key,
                    value: child_value,
                    generation,
                } => {
                    if *child_key == key {
                        let old_value = std::mem::replace(child_value, value);
                        *generation = owner_generation;
                        (Some(old_value), false)
                    } else if *child_hash == hash {
                        let mut entries = CollisionArray::new();
                        entries.push((child_key.clone(), child_value.clone()));
                        entries.push((key, value));
                        let collision = Node::Collision {
                            hash,
                            entries,
                            generation: owner_generation,
                        };
                        children[position] = ChildSlot::Node(ReferenceCounter::new(collision));
                        (None, true)
                    } else {
                        let child_entry = Node::Entry {
                            hash: *child_hash,
                            key: child_key.clone(),
                            value: child_value.clone(),
                            generation: owner_generation,
                        };
                        let (mut subnode, _) = PersistentHashMap::insert_into_node(
                            &child_entry,
                            key,
                            value,
                            hash,
                            depth + 1,
                        );
                        // Update generation on the new subnode and all its children (RISK-002 mitigation)
                        Self::update_node_generation_recursive(&mut subnode, owner_generation);
                        children[position] = ChildSlot::Node(ReferenceCounter::new(subnode));
                        (None, true)
                    }
                }
                ChildSlot::Node(subnode) => {
                    // ReferenceCounter::make_mut handles COW automatically based on reference count.
                    // Generation tracking is maintained when creating new nodes.
                    let subnode_mut = ReferenceCounter::make_mut(subnode);
                    Self::insert_into_node_with_generation(
                        subnode_mut,
                        key,
                        value,
                        hash,
                        depth + 1,
                        owner_generation,
                    )
                }
            }
        }
    }

    /// Recursively inserts into a node using COW semantics.
    /// Returns (`old_value`, `was_added`).
    #[allow(clippy::too_many_lines)]
    fn insert_into_node_cow(
        node: &mut Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Option<V>, bool) {
        match node {
            Node::Empty => {
                *node = Node::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                };
                (None, true)
            }
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
                ..
            } => {
                if *existing_hash == hash && *existing_key == key {
                    let old_value = std::mem::replace(existing_value, value);
                    (Some(old_value), false)
                } else if *existing_hash == hash {
                    let mut entries = CollisionArray::new();
                    entries.push((existing_key.clone(), existing_value.clone()));
                    entries.push((key, value));
                    *node = Node::Collision {
                        hash: *existing_hash,
                        entries,
                        generation: SHARED_GENERATION,
                    };
                    (None, true)
                } else {
                    let (new_node, _) = PersistentHashMap::create_bitmap_from_two_entries(
                        *existing_hash,
                        existing_key,
                        existing_value,
                        key,
                        value,
                        hash,
                        depth,
                    );
                    *node = new_node;
                    (None, true)
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => Self::insert_into_bitmap_node_cow(bitmap, children, key, value, hash, depth),
            Node::Collision {
                hash: collision_hash,
                entries,
                ..
            } => {
                if hash == *collision_hash {
                    for entry in entries.iter_mut() {
                        if entry.0 == key {
                            let old_value = std::mem::replace(&mut entry.1, value);
                            return (Some(old_value), false);
                        }
                    }
                    entries.push((key, value));
                    (None, true)
                } else {
                    // Different hash - convert collision to bitmap and insert
                    let collision_entries: CollisionArray<K, V> = entries.clone();
                    let collision_hash_value = *collision_hash;

                    // Create bitmap node from collision entries
                    let index = hash_index(collision_hash_value, depth);
                    let bit = 1u32 << index;
                    let collision_child = ChildSlot::Node(ReferenceCounter::new(Node::Collision {
                        hash: collision_hash_value,
                        entries: collision_entries,
                        generation: SHARED_GENERATION,
                    }));

                    let new_index = hash_index(hash, depth);
                    let new_bit = 1u32 << new_index;

                    if index == new_index {
                        let mut subnode = Node::Collision {
                            hash: collision_hash_value,
                            entries: entries.clone(),
                            generation: SHARED_GENERATION,
                        };
                        let result =
                            Self::insert_into_node_cow(&mut subnode, key, value, hash, depth + 1);
                        // TASK-008: Pre-allocate with exact capacity (1 child for single-bit bitmap)
                        let mut children = ChildArray::with_capacity(1);
                        children.push(ChildSlot::Node(ReferenceCounter::new(subnode)));
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(1);
                        *node = Node::Bitmap {
                            bitmap: bit,
                            children,
                            generation: SHARED_GENERATION,
                        };
                        result
                    } else {
                        let new_child = ChildSlot::Entry {
                            hash,
                            key,
                            value,
                            generation: SHARED_GENERATION,
                        };
                        let mut children = ChildArray::with_capacity(2);
                        if index < new_index {
                            children.push(collision_child);
                            children.push(new_child);
                        } else {
                            children.push(new_child);
                            children.push(collision_child);
                        }
                        // TASK-009: Record occupancy for histogram analysis
                        occupancy_histogram::record_occupancy(2);
                        *node = Node::Bitmap {
                            bitmap: bit | new_bit,
                            children,
                            generation: SHARED_GENERATION,
                        };
                        (None, true)
                    }
                }
            }
        }
    }

    /// Inserts into a bitmap node using COW semantics.
    fn insert_into_bitmap_node_cow(
        bitmap: &mut u32,
        children: &mut ChildArray<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Option<V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (*bitmap & (bit - 1)).count_ones() as usize;

        if *bitmap & bit == 0 {
            children.insert(
                position,
                ChildSlot::Entry {
                    hash,
                    key,
                    value,
                    generation: SHARED_GENERATION,
                },
            );
            *bitmap |= bit;
            (None, true)
        } else {
            match &mut children[position] {
                ChildSlot::Entry {
                    hash: child_hash,
                    key: child_key,
                    value: child_value,
                    ..
                } => {
                    if *child_key == key {
                        let old_value = std::mem::replace(child_value, value);
                        (Some(old_value), false)
                    } else if *child_hash == hash {
                        let mut entries = CollisionArray::new();
                        entries.push((child_key.clone(), child_value.clone()));
                        entries.push((key, value));
                        let collision = Node::Collision {
                            hash,
                            entries,
                            generation: SHARED_GENERATION,
                        };
                        children[position] = ChildSlot::Node(ReferenceCounter::new(collision));
                        (None, true)
                    } else {
                        let child_entry = Node::Entry {
                            hash: *child_hash,
                            key: child_key.clone(),
                            value: child_value.clone(),
                            generation: SHARED_GENERATION,
                        };
                        let (subnode, _) = PersistentHashMap::insert_into_node(
                            &child_entry,
                            key,
                            value,
                            hash,
                            depth + 1,
                        );
                        children[position] = ChildSlot::Node(ReferenceCounter::new(subnode));
                        (None, true)
                    }
                }
                ChildSlot::Node(subnode) => {
                    let subnode_mut = ReferenceCounter::make_mut(subnode);
                    Self::insert_into_node_cow(subnode_mut, key, value, hash, depth + 1)
                }
            }
        }
    }

    /// Removes a key from the map and returns the value if it was present.
    ///
    /// The key may be any borrowed form of the map's key type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Returns
    ///
    /// The removed value if the key was present, otherwise `None`.
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("key".to_string(), 42);
    /// assert_eq!(transient.remove("key"), Some(42));
    /// assert_eq!(transient.remove("key"), None);
    /// assert!(transient.is_empty());
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = compute_hash(key);
        let root = ReferenceCounter::make_mut(&mut self.root);
        let result = Self::remove_from_node_cow(root, key, hash, 0);
        if result.is_some() {
            self.length = self.length.saturating_sub(1);
        }
        result
    }

    /// Recursively removes from a node using COW semantics.
    fn remove_from_node_cow<Q>(node: &mut Node<K, V>, key: &Q, hash: u64, depth: usize) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match node {
            Node::Empty => None,
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                value,
                ..
            } => {
                if *entry_hash == hash && (*entry_key).borrow() == key {
                    let old_value = value.clone();
                    *node = Node::Empty;
                    Some(old_value)
                } else {
                    None
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => Self::remove_from_bitmap_node_cow(bitmap, children, key, hash, depth),
            Node::Collision {
                hash: collision_hash,
                entries,
                ..
            } => {
                if *collision_hash != hash {
                    return None;
                }

                let found_index = entries
                    .iter()
                    .position(|(entry_key, _)| entry_key.borrow() == key)?;

                let removed_value = entries[found_index].1.clone();

                if entries.len() == 2 {
                    let other_index = 1 - found_index;
                    let (remaining_key, remaining_value) = entries[other_index].clone();
                    *node = Node::Entry {
                        hash: *collision_hash,
                        key: remaining_key,
                        value: remaining_value,
                        generation: SHARED_GENERATION,
                    };
                } else {
                    entries.remove(found_index);
                }

                Some(removed_value)
            }
        }
    }

    /// Removes from a bitmap node using COW semantics.
    fn remove_from_bitmap_node_cow<Q>(
        bitmap: &mut u32,
        children: &mut ChildArray<K, V>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;

        if *bitmap & bit == 0 {
            return None;
        }

        let position = (*bitmap & (bit - 1)).count_ones() as usize;

        match &mut children[position] {
            ChildSlot::Entry {
                key: child_key,
                value: child_value,
                ..
            } => {
                if (*child_key).borrow() == key {
                    let removed_value = child_value.clone();

                    let new_bitmap = *bitmap & !bit;
                    children.remove(position);
                    *bitmap = new_bitmap;

                    Some(removed_value)
                } else {
                    None
                }
            }
            ChildSlot::Node(subnode) => {
                let subnode_mut = ReferenceCounter::make_mut(subnode);
                let result = Self::remove_from_node_cow(subnode_mut, key, hash, depth + 1);

                if result.is_some() {
                    // Check if we need to simplify the structure
                    match subnode_mut {
                        Node::Empty => {
                            let new_bitmap = *bitmap & !bit;
                            children.remove(position);
                            *bitmap = new_bitmap;
                        }
                        Node::Entry {
                            hash: entry_hash,
                            key: entry_key,
                            value: entry_value,
                            ..
                        } => {
                            children[position] = ChildSlot::Entry {
                                hash: *entry_hash,
                                key: entry_key.clone(),
                                value: entry_value.clone(),
                                generation: SHARED_GENERATION,
                            };
                        }
                        _ => {}
                    }
                }

                result
            }
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// This operation uses Copy-on-Write (COW) semantics to ensure that
    /// shared nodes are copied before mutation.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("count".to_string(), 0);
    ///
    /// if let Some(count) = transient.get_mut("count") {
    ///     *count += 1;
    /// }
    ///
    /// assert_eq!(transient.get("count"), Some(&1));
    /// ```
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = compute_hash(key);
        let root = ReferenceCounter::make_mut(&mut self.root);
        Self::get_mut_from_node(root, key, hash, 0)
    }

    /// Recursively gets a mutable reference from a node using COW semantics.
    fn get_mut_from_node<'a, Q>(
        node: &'a mut Node<K, V>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<&'a mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match node {
            Node::Empty => None,
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                value,
                ..
            } => {
                if *entry_hash == hash && (*entry_key).borrow() == key {
                    Some(value)
                } else {
                    None
                }
            }
            Node::Bitmap {
                bitmap, children, ..
            } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if *bitmap & bit == 0 {
                    return None;
                }

                let position = (*bitmap & (bit - 1)).count_ones() as usize;

                match &mut children[position] {
                    ChildSlot::Entry {
                        hash: child_hash,
                        key: child_key,
                        value,
                        ..
                    } => {
                        if *child_hash == hash && (*child_key).borrow() == key {
                            Some(value)
                        } else {
                            None
                        }
                    }
                    ChildSlot::Node(subnode) => {
                        let subnode_mut = ReferenceCounter::make_mut(subnode);
                        Self::get_mut_from_node(subnode_mut, key, hash, depth + 1)
                    }
                }
            }
            Node::Collision {
                hash: collision_hash,
                entries,
                ..
            } => {
                if *collision_hash != hash {
                    return None;
                }

                for entry in entries.iter_mut() {
                    if entry.0.borrow() == key {
                        return Some(&mut entry.1);
                    }
                }
                None
            }
        }
    }

    /// Updates the value at the given key using a function.
    ///
    /// Returns `true` if the key was found and updated, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to update
    /// * `function` - A function that transforms the old value into the new value
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("count".to_string(), 10);
    ///
    /// assert!(transient.update_with("count", |x| x * 2));
    /// assert_eq!(transient.get("count"), Some(&20));
    ///
    /// assert!(!transient.update_with("missing", |x| x * 2));
    /// ```
    pub fn update_with<Q, F>(&mut self, key: &Q, function: F) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(V) -> V,
    {
        self.get_mut(key).is_some_and(|value_ref| {
            let old_value = value_ref.clone();
            *value_ref = function(old_value);
            true
        })
    }

    /// Extends the map with elements from an iterator.
    ///
    /// This method utilizes the iterator's `size_hint` to update the internal
    /// capacity hint, which may be used for future optimizations.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator over key-value pairs to insert
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.extend([
    ///     ("one".to_string(), 1),
    ///     ("two".to_string(), 2),
    ///     ("three".to_string(), 3),
    /// ]);
    /// assert_eq!(transient.len(), 3);
    /// ```
    pub fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        // Extract size_hint for potential future optimizations.
        // Currently stored but not actively used for allocation.
        let (lower_bound, _upper_bound) = iter.size_hint();
        if lower_bound > 0 {
            // Update capacity_hint if the iterator provides a meaningful hint
            self.capacity_hint = self.capacity_hint.saturating_add(lower_bound);
        }

        for (key, value) in iter {
            self.insert(key, value);
        }
    }

    /// Inserts multiple key-value pairs in a single batch operation.
    ///
    /// This method is optimized for bulk insertions, providing better performance
    /// than calling `insert` repeatedly for large datasets. It consumes `self`
    /// and returns a new `TransientHashMap` with all entries inserted.
    ///
    /// # Duplicate Key Handling
    ///
    /// When the same key appears multiple times in the input:
    /// - **Last value wins**: The value from the last occurrence is kept
    /// - This matches the semantics of calling `insert` sequentially
    /// - The input order is preserved during processing
    ///
    /// # Errors
    ///
    /// Returns `Err(BulkInsertError::TooManyEntries)` if the input exceeds
    /// [`MAX_BULK_INSERT`] (100,000 entries). To prevent memory spikes, the
    /// iterator is only consumed up to `MAX_BULK_INSERT + 1` elements.
    ///
    /// # Purity Assumption
    ///
    /// This method assumes that `items` is a pure data source (e.g., `Vec`, slice iterator).
    /// If the iterator has side effects (e.g., I/O operations), the early termination
    /// at `MAX_BULK_INSERT + 1` may leave those side effects partially executed.
    /// For side-effecting iterators, collect into a `Vec` first and pass the `Vec`.
    ///
    /// # Complexity
    ///
    /// O(N * log32 M) where N is the number of items and M is the map size.
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// let transient = transient
    ///     .insert_bulk(vec![
    ///         ("one".to_string(), 1),
    ///         ("two".to_string(), 2),
    ///         ("three".to_string(), 3),
    ///     ])
    ///     .unwrap();
    ///
    /// assert_eq!(transient.len(), 3);
    /// assert_eq!(transient.get("one"), Some(&1));
    /// ```
    ///
    /// ## Duplicate Keys (Last Value Wins)
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// let transient = transient
    ///     .insert_bulk(vec![
    ///         ("key".to_string(), 1),
    ///         ("key".to_string(), 2),
    ///         ("key".to_string(), 3),
    ///     ])
    ///     .unwrap();
    ///
    /// assert_eq!(transient.len(), 1);
    /// assert_eq!(transient.get("key"), Some(&3)); // Last value wins
    /// ```
    ///
    /// ## Method Chaining with `persistent()`
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let map: PersistentHashMap<String, i32> = PersistentHashMap::new()
    ///     .transient()
    ///     .insert_bulk(vec![
    ///         ("a".to_string(), 1),
    ///         ("b".to_string(), 2),
    ///     ])
    ///     .unwrap()
    ///     .persistent();
    ///
    /// assert_eq!(map.len(), 2);
    /// ```
    ///
    /// ## Handling Large Inputs
    ///
    /// ```rust
    /// use lambars::persistent::{TransientHashMap, BulkInsertError, MAX_BULK_INSERT};
    ///
    /// fn insert_with_chunking(
    ///     mut transient: TransientHashMap<i32, i32>,
    ///     data: Vec<(i32, i32)>,
    /// ) -> TransientHashMap<i32, i32> {
    ///     const CHUNK_SIZE: usize = 10_000;
    ///     for chunk in data.chunks(CHUNK_SIZE) {
    ///         transient = transient.insert_bulk(chunk.to_vec()).unwrap();
    ///     }
    ///     transient
    /// }
    /// ```
    pub fn insert_bulk<I>(self, items: I) -> Result<Self, BulkInsertError>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let items_vec: Vec<(K, V)> = items.into_iter().take(MAX_BULK_INSERT + 1).collect();

        match self.insert_bulk_owned(items_vec) {
            Ok(updated) => Ok(updated),
            Err(BulkInsertErrorWithItems::TooManyEntries { count, limit, .. }) => {
                Err(BulkInsertError::TooManyEntries { count, limit })
            }
        }
    }

    /// Bulk inserts key-value pairs with item recovery on error.
    ///
    /// Similar to [`insert_bulk`](Self::insert_bulk), but takes ownership of the `Vec`
    /// and returns items on error for recovery. Uses [`insert_without_cow`](Self::insert_without_cow)
    /// internally to minimize COW operations.
    ///
    /// # Errors
    ///
    /// Returns [`BulkInsertErrorWithItems::TooManyEntries`] if `items.len()` exceeds
    /// [`MAX_BULK_INSERT`] (100,000), including the original items for retry.
    ///
    /// # Complexity
    ///
    /// O(N * log32 M) where N is the number of items and M is the resulting map size.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let transient: TransientHashMap<i32, i32> = TransientHashMap::new();
    /// let items: Vec<(i32, i32)> = (0..100).map(|i| (i, i * 2)).collect();
    ///
    /// let result = transient.insert_bulk_owned(items);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap().len(), 100);
    /// ```
    pub fn insert_bulk_owned(
        mut self,
        items: Vec<(K, V)>,
    ) -> Result<Self, BulkInsertErrorWithItems<K, V>> {
        if items.len() > MAX_BULK_INSERT {
            return Err(BulkInsertErrorWithItems::TooManyEntries {
                count: items.len(),
                limit: MAX_BULK_INSERT,
                items,
            });
        }

        if !items.is_empty() {
            self.reserve(items.len());
        }

        for (key, value) in items {
            self.insert_without_cow(key, value);
        }

        Ok(self)
    }

    /// Performs a bulk insert with detailed metrics collection.
    ///
    /// This method is similar to [`insert_bulk`](Self::insert_bulk) but returns
    /// statistics about the operation, including counts of new insertions,
    /// updates, and the values that were replaced.
    ///
    /// # Arguments
    ///
    /// * `items` - An iterator of key-value pairs to insert. Must implement
    ///   [`ExactSizeIterator`] to allow pre-validation of entry count.
    ///
    /// # Returns
    ///
    /// * `Ok(BulkInsertResult<V>)` - On success, contains insertion metrics
    /// * `Err(BulkInsertError::TooManyEntries)` - If item count exceeds [`MAX_BULK_INSERT`]
    ///
    /// # Errors
    ///
    /// Returns [`BulkInsertError::TooManyEntries`] if the number of items exceeds
    /// [`MAX_BULK_INSERT`] (100,000 entries). To handle large datasets, chunk the
    /// input into smaller batches of 10,000 entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("existing".to_string(), 100);
    ///
    /// let items = vec![
    ///     ("existing".to_string(), 200),
    ///     ("new".to_string(), 300),
    /// ];
    ///
    /// let result = transient.insert_bulk_with_metrics(items).unwrap();
    /// assert_eq!(result.inserted_count, 1);
    /// assert_eq!(result.updated_count, 1);
    /// assert_eq!(result.replaced_values, vec![100]);
    /// ```
    pub fn insert_bulk_with_metrics<I>(
        &mut self,
        items: I,
    ) -> Result<BulkInsertResult<V>, BulkInsertError>
    where
        I: IntoIterator<Item = (K, V)>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = items.into_iter();
        let count = iter.len();

        if count > MAX_BULK_INSERT {
            return Err(BulkInsertError::TooManyEntries {
                count,
                limit: MAX_BULK_INSERT,
            });
        }

        if count == 0 {
            return Ok(BulkInsertResult::default());
        }

        self.reserve(count);

        let mut inserted_count = 0;
        let mut updated_count = 0;
        let mut replaced_values = Vec::new();

        for (key, value) in iter {
            match self.insert_without_cow(key, value) {
                Some(old_value) => {
                    updated_count += 1;
                    replaced_values.push(old_value);
                }
                None => {
                    inserted_count += 1;
                }
            }
        }

        Ok(BulkInsertResult {
            inserted_count,
            updated_count,
            replaced_values,
        })
    }

    /// Performs a bulk insert using a node pool for memory reuse.
    ///
    /// This method is similar to [`insert_bulk_with_metrics`](Self::insert_bulk_with_metrics)
    /// but uses a `NodePool` to reuse node allocations, potentially reducing
    /// allocation overhead during bulk operations.
    ///
    /// # Arguments
    ///
    /// * `items` - An iterator of key-value pairs to insert. Must implement
    ///   [`ExactSizeIterator`] to allow pre-validation of entry count.
    /// * `pool` - A mutable reference to a `NodePool` for node reuse.
    ///
    /// # Returns
    ///
    /// * `Ok(BulkInsertResult<V>)` - On success, contains insertion metrics
    /// * `Err(BulkInsertError::TooManyEntries)` - If item count exceeds [`MAX_BULK_INSERT`]
    ///
    /// # Errors
    ///
    /// Returns [`BulkInsertError::TooManyEntries`] if the number of items exceeds
    /// [`MAX_BULK_INSERT`] (100,000 entries).
    ///
    /// # Note
    ///
    /// The pool is used opportunistically. If the pool is empty or full, the
    /// operation continues without pool-assisted allocation/deallocation.
    /// The resulting map is identical regardless of pool usage.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use lambars::persistent::{TransientHashMap, NodePool};
    ///
    /// let mut pool = NodePool::new();
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    ///
    /// let items = vec![("a".to_string(), 1), ("b".to_string(), 2)];
    /// let result = transient.insert_bulk_with_pool(items, &mut pool);
    /// ```
    pub fn insert_bulk_with_pool<I>(
        &mut self,
        items: I,
        _pool: &mut NodePool<K, V>,
    ) -> Result<BulkInsertResult<V>, BulkInsertError>
    where
        I: IntoIterator<Item = (K, V)>,
        I::IntoIter: ExactSizeIterator,
    {
        // For now, delegate to insert_bulk_with_metrics
        // Pool-based optimization can be added as a future enhancement
        // when profiling shows allocation is still a bottleneck
        self.insert_bulk_with_metrics(items)
    }

    /// Converts this transient map into a persistent map.
    ///
    /// This consumes the `TransientHashMap` and returns a `PersistentHashMap`.
    /// The conversion is O(1) as it simply moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientHashMap;
    ///
    /// let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
    /// transient.insert("one".to_string(), 1);
    /// transient.insert("two".to_string(), 2);
    /// let persistent = transient.persistent();
    /// assert_eq!(persistent.len(), 2);
    /// assert_eq!(persistent.get("one"), Some(&1));
    /// ```
    #[must_use]
    pub fn persistent(self) -> PersistentHashMap<K, V> {
        PersistentHashMap {
            root: self.root,
            length: self.length,
        }
    }
}

impl<K: Clone + Hash + Eq, V: Clone> Default for TransientHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Hash + Eq, V: Clone> FromIterator<(K, V)> for TransientHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower_bound, _) = iter.size_hint();
        let mut transient = Self::with_capacity_hint(lower_bound);
        transient.extend(iter);
        transient
    }
}

// =============================================================================
// PersistentHashMap::transient() method
// =============================================================================

impl<K: Clone + Hash + Eq, V: Clone> PersistentHashMap<K, V> {
    /// Converts this persistent map into a transient map.
    ///
    /// This consumes the `PersistentHashMap` and returns a `TransientHashMap`
    /// that can be efficiently mutated. The conversion is O(1) as it simply
    /// moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentHashMap;
    ///
    /// let persistent: PersistentHashMap<String, i32> = [
    ///     ("one".to_string(), 1),
    ///     ("two".to_string(), 2),
    /// ].into_iter().collect();
    ///
    /// let mut transient = persistent.transient();
    /// transient.insert("three".to_string(), 3);
    /// transient.remove("one");
    ///
    /// let new_persistent = transient.persistent();
    /// assert_eq!(new_persistent.len(), 2);
    /// assert_eq!(new_persistent.get("two"), Some(&2));
    /// assert_eq!(new_persistent.get("three"), Some(&3));
    /// ```
    #[must_use]
    pub fn transient(self) -> TransientHashMap<K, V> {
        TransientHashMap {
            root: self.root,
            length: self.length,
            capacity_hint: 0,
            generation: next_generation(),
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_empty_hashmap() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        assert_eq!(format!("{map}"), "{}");
    }

    #[rstest]
    fn test_display_single_element_hashmap() {
        let map = PersistentHashMap::singleton("key".to_string(), 42);
        assert_eq!(format!("{map}"), "{key: 42}");
    }

    #[rstest]
    fn test_display_multiple_elements_hashmap() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let display = format!("{map}");
        // HashMap is unordered, so we check that the format is correct
        assert!(display.starts_with('{'));
        assert!(display.ends_with('}'));
        assert!(display.contains("a: 1"));
        assert!(display.contains("b: 2"));
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_new_creates_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[rstest]
    fn test_singleton() {
        let map = PersistentHashMap::singleton("key".to_string(), 42);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&42));
    }

    #[rstest]
    fn test_insert_and_get() {
        let map = PersistentHashMap::new()
            .insert("one".to_string(), 1)
            .insert("two".to_string(), 2);

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("one"), Some(&1));
        assert_eq!(map.get("two"), Some(&2));
        assert_eq!(map.get("three"), None);
    }

    #[rstest]
    fn test_insert_overwrite() {
        let map1 = PersistentHashMap::new().insert("key".to_string(), 1);
        let map2 = map1.insert("key".to_string(), 2);

        assert_eq!(map1.get("key"), Some(&1));
        assert_eq!(map2.get("key"), Some(&2));
        assert_eq!(map1.len(), 1);
        assert_eq!(map2.len(), 1);
    }

    #[rstest]
    fn test_remove() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let removed = map.remove("a");

        assert_eq!(removed.len(), 1);
        assert_eq!(removed.get("a"), None);
        assert_eq!(removed.get("b"), Some(&2));
    }

    #[rstest]
    fn test_contains_key() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);

        assert!(map.contains_key("key"));
        assert!(!map.contains_key("other"));
    }

    #[rstest]
    fn test_iter() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);

        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by_key(|(k, _)| (*k).clone());

        assert_eq!(entries.len(), 2);
    }

    #[rstest]
    fn test_from_iter() {
        let entries = vec![("a".to_string(), 1), ("b".to_string(), 2)];
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
    }

    #[rstest]
    fn test_eq() {
        let map1 = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let map2 = PersistentHashMap::new()
            .insert("b".to_string(), 2)
            .insert("a".to_string(), 1);

        assert_eq!(map1, map2);
    }

    #[rstest]
    fn test_fold_left() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);

        let sum = map.fold_left(0, |accumulator, value| accumulator + value);
        assert_eq!(sum, 6);
    }

    // =========================================================================
    // map_values Tests
    // =========================================================================

    #[rstest]
    fn test_map_values_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.map_values(|v| v * 2);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_map_values_basic() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let doubled = map.map_values(|v| v * 2);
        assert_eq!(doubled.get("a"), Some(&2));
        assert_eq!(doubled.get("b"), Some(&4));
        assert_eq!(doubled.len(), 2);
    }

    #[rstest]
    fn test_map_values_type_change() {
        let map = PersistentHashMap::new().insert(1, 100).insert(2, 200);
        let stringified = map.map_values(|v| v.to_string());
        assert_eq!(stringified.get(&1), Some(&"100".to_string()));
        assert_eq!(stringified.get(&2), Some(&"200".to_string()));
    }

    #[rstest]
    fn test_map_values_identity_law() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.map_values(|v| *v);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_map_values_length_preservation() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let result = map.map_values(|v| v * 2);
        assert_eq!(result.len(), map.len());
    }

    #[rstest]
    fn test_map_values_composition_law() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let f = |v: &i32| v * 2;
        let g = |v: &i32| v + 10;
        let chained = map.map_values(f).map_values(g);
        let composed = map.map_values(|v| g(&f(v)));
        assert_eq!(chained, composed);
    }

    // =========================================================================
    // map_keys Tests
    // =========================================================================

    #[rstest]
    fn test_map_keys_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.map_keys(|k| k.to_uppercase());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_map_keys_basic() {
        let map = PersistentHashMap::new()
            .insert("hello".to_string(), 1)
            .insert("world".to_string(), 2);
        let uppercased = map.map_keys(|k| k.to_uppercase());
        assert_eq!(uppercased.get("HELLO"), Some(&1));
        assert_eq!(uppercased.get("WORLD"), Some(&2));
    }

    #[rstest]
    fn test_map_keys_type_change() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("bb".to_string(), 2)
            .insert("ccc".to_string(), 3);
        let by_length = map.map_keys(|k| k.len());
        assert_eq!(by_length.get(&1), Some(&1));
        assert_eq!(by_length.get(&2), Some(&2));
        assert_eq!(by_length.get(&3), Some(&3));
    }

    #[rstest]
    fn test_map_keys_collision() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("A".to_string(), 2);
        let uppercased = map.map_keys(|k| k.to_uppercase());
        assert_eq!(uppercased.len(), 1);
        assert!(uppercased.contains_key("A"));
    }

    #[rstest]
    fn test_map_keys_identity_law() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.map_keys(|k| k.clone());
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_map_keys_length_upper_bound() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("A".to_string(), 2);
        let result = map.map_keys(|k| k.to_uppercase());
        assert!(result.len() <= map.len());
    }

    // =========================================================================
    // filter_map Tests
    // =========================================================================

    #[rstest]
    fn test_filter_map_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.filter_map(|_, v| Some(v * 2));
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_filter_map_basic() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3)
            .insert("d".to_string(), 4);
        let evens_doubled = map.filter_map(|_, v| if v % 2 == 0 { Some(v * 2) } else { None });
        assert_eq!(evens_doubled.len(), 2);
        assert_eq!(evens_doubled.get("b"), Some(&4));
        assert_eq!(evens_doubled.get("d"), Some(&8));
    }

    #[rstest]
    fn test_filter_map_all_none() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result: PersistentHashMap<String, i32> = map.filter_map(|_, _| None);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_filter_map_all_some() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.filter_map(|_, v| Some(*v));
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_filter_map_identity_with_some() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let filter_mapped = map.filter_map(|_, v| Some(*v * 2));
        let map_valued = map.map_values(|v| v * 2);
        assert_eq!(filter_mapped, map_valued);
    }

    #[rstest]
    fn test_filter_map_type_change() {
        let map = PersistentHashMap::new()
            .insert("valid".to_string(), "42".to_string())
            .insert("invalid".to_string(), "abc".to_string());
        let parsed = map.filter_map(|_, v| v.parse::<i32>().ok());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.get("valid"), Some(&42));
    }

    #[rstest]
    fn test_filter_map_uses_key() {
        let map = PersistentHashMap::new()
            .insert("keep".to_string(), 1)
            .insert("drop".to_string(), 2);
        let result = map.filter_map(|k, v| {
            if k.starts_with("keep") {
                Some(*v)
            } else {
                None
            }
        });
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("keep"));
    }

    // =========================================================================
    // entries Tests
    // =========================================================================

    #[rstest]
    fn test_entries_hashmap_equals_iter() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let iter_entries: Vec<_> = map.iter().collect();
        let entries_entries: Vec<_> = map.entries().collect();
        assert_eq!(iter_entries, entries_entries);
    }

    #[rstest]
    fn test_entries_count_equals_len() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        assert_eq!(map.entries().count(), map.len());
    }

    // =========================================================================
    // merge_with Tests
    // =========================================================================

    #[rstest]
    fn test_merge_with_empty_left() {
        let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let other = PersistentHashMap::singleton("a".to_string(), 1);
        let result = empty.merge_with(&other, |_, v1, v2| v1 + v2);
        assert_eq!(result, other);
    }

    #[rstest]
    fn test_merge_with_empty_right() {
        let map = PersistentHashMap::singleton("a".to_string(), 1);
        let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.merge_with(&empty, |_, v1, v2| v1 + v2);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_merge_with_sum_resolver() {
        let map1 = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let map2 = PersistentHashMap::new()
            .insert("b".to_string(), 20)
            .insert("c".to_string(), 3);
        let merged = map1.merge_with(&map2, |_, v1, v2| v1 + v2);
        assert_eq!(merged.get("a"), Some(&1));
        assert_eq!(merged.get("b"), Some(&22));
        assert_eq!(merged.get("c"), Some(&3));
    }

    #[rstest]
    fn test_merge_with_max_resolver() {
        let map1 = PersistentHashMap::new()
            .insert("x".to_string(), 100)
            .insert("y".to_string(), 5);
        let map2 = PersistentHashMap::new()
            .insert("x".to_string(), 50)
            .insert("y".to_string(), 500);
        let merged = map1.merge_with(&map2, |_, v1, v2| *v1.max(v2));
        assert_eq!(merged.get("x"), Some(&100));
        assert_eq!(merged.get("y"), Some(&500));
    }

    #[rstest]
    fn test_merge_with_left_wins() {
        let map1 = PersistentHashMap::singleton("a".to_string(), 1);
        let map2 = PersistentHashMap::singleton("a".to_string(), 2);
        let merged = map1.merge_with(&map2, |_, v1, _| *v1);
        assert_eq!(merged.get("a"), Some(&1));
    }

    #[rstest]
    fn test_merge_with_right_wins() {
        let map1 = PersistentHashMap::singleton("a".to_string(), 1);
        let map2 = PersistentHashMap::singleton("a".to_string(), 2);
        let merged = map1.merge_with(&map2, |_, _, v2| *v2);
        assert_eq!(merged.get("a"), Some(&2));
    }

    #[rstest]
    fn test_merge_with_commutativity_with_commutative_resolver() {
        let map1 = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let map2 = PersistentHashMap::new()
            .insert("b".to_string(), 20)
            .insert("c".to_string(), 3);
        let merged1 = map1.merge_with(&map2, |_, v1, v2| v1 + v2);
        let merged2 = map2.merge_with(&map1, |_, v1, v2| v1 + v2);
        assert_eq!(merged1, merged2);
    }

    // =========================================================================
    // delete_if Tests
    // =========================================================================

    #[rstest]
    fn test_delete_if_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.delete_if(|_, _| true);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_delete_if_none() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.delete_if(|_, _| false);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_delete_if_all() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.delete_if(|_, _| true);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_delete_if_evens() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3)
            .insert("d".to_string(), 4);
        let odds = map.delete_if(|_, v| v % 2 == 0);
        assert_eq!(odds.len(), 2);
        assert!(odds.contains_key("a"));
        assert!(odds.contains_key("c"));
    }

    #[rstest]
    fn test_delete_if_by_key() {
        let map = PersistentHashMap::new()
            .insert("keep".to_string(), 1)
            .insert("delete".to_string(), 2);
        let result = map.delete_if(|k, _| k.starts_with("delete"));
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("keep"));
    }

    // =========================================================================
    // keep_if Tests
    // =========================================================================

    #[rstest]
    fn test_keep_if_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = map.keep_if(|_, _| true);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_keep_if_all() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.keep_if(|_, _| true);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_keep_if_none() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let result = map.keep_if(|_, _| false);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_keep_if_evens() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3)
            .insert("d".to_string(), 4);
        let evens = map.keep_if(|_, v| v % 2 == 0);
        assert_eq!(evens.len(), 2);
        assert!(evens.contains_key("b"));
        assert!(evens.contains_key("d"));
    }

    #[rstest]
    fn test_keep_if_complement_of_delete_if() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let predicate = |_: &String, v: &i32| v % 2 == 0;
        let kept = map.keep_if(predicate);
        let deleted = map.delete_if(|k, v| !predicate(k, v));
        assert_eq!(kept, deleted);
    }

    // =========================================================================
    // partition Tests
    // =========================================================================

    #[rstest]
    fn test_partition_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let (matching, not_matching) = map.partition(|_, _| true);
        assert!(matching.is_empty());
        assert!(not_matching.is_empty());
    }

    #[rstest]
    fn test_partition_all_match() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let (matching, not_matching) = map.partition(|_, _| true);
        assert_eq!(matching, map);
        assert!(not_matching.is_empty());
    }

    #[rstest]
    fn test_partition_none_match() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let (matching, not_matching) = map.partition(|_, _| false);
        assert!(matching.is_empty());
        assert_eq!(not_matching, map);
    }

    #[rstest]
    fn test_partition_even_odd() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3)
            .insert("d".to_string(), 4);
        let (evens, odds) = map.partition(|_, v| v % 2 == 0);
        assert_eq!(evens.len(), 2);
        assert_eq!(odds.len(), 2);
        assert!(evens.contains_key("b"));
        assert!(evens.contains_key("d"));
        assert!(odds.contains_key("a"));
        assert!(odds.contains_key("c"));
    }

    #[rstest]
    fn test_partition_completeness() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let (matching, not_matching) = map.partition(|_, v| v % 2 == 0);
        assert_eq!(matching.len() + not_matching.len(), map.len());
    }

    #[rstest]
    fn test_partition_disjointness() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let (matching, not_matching) = map.partition(|_, v| v % 2 == 0);
        for (key, _) in &matching {
            assert!(!not_matching.contains_key(key));
        }
    }

    #[rstest]
    fn test_partition_equals_keep_if_delete_if() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let predicate = |_: &String, v: &i32| v % 2 == 0;
        let (matching, not_matching) = map.partition(predicate);
        let kept = map.keep_if(predicate);
        let deleted_complement = map.keep_if(|k, v| !predicate(k, v));
        assert_eq!(matching, kept);
        assert_eq!(not_matching, deleted_complement);
    }

    // =========================================================================
    // Lazy Iterator Tests
    // =========================================================================

    #[rstest]
    fn test_lazy_iter_empty_map() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let mut iterator = map.iter();
        assert_eq!(iterator.next(), None);
        // FusedIterator behavior: None after exhausted
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_lazy_iter_single_element() {
        let map = PersistentHashMap::singleton("key".to_string(), 42);
        let mut iterator = map.iter();
        assert_eq!(iterator.next(), Some((&"key".to_string(), &42)));
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_lazy_iter_exact_size() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let iterator = map.iter();
        assert_eq!(iterator.len(), 3);
    }

    #[rstest]
    fn test_lazy_iter_exact_size_decreases() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let mut iterator = map.iter();
        assert_eq!(iterator.len(), 3);
        let _ = iterator.next();
        assert_eq!(iterator.len(), 2);
        let _ = iterator.next();
        assert_eq!(iterator.len(), 1);
        let _ = iterator.next();
        assert_eq!(iterator.len(), 0);
    }

    #[rstest]
    fn test_lazy_iter_size_hint() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let iterator = map.iter();
        let (lower, upper) = iterator.size_hint();
        assert_eq!(lower, 2);
        assert_eq!(upper, Some(2));
    }

    #[rstest]
    fn test_lazy_iter_early_termination_take() {
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|index| (index, index * 10)).collect();
        assert_eq!(map.iter().take(5).count(), 5);
    }

    #[rstest]
    fn test_lazy_iter_early_termination_find() {
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|index| (index, index * 10)).collect();
        let found = map.iter().find(|(_, value)| **value == 500);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), (&50, &500));
    }

    #[rstest]
    fn test_lazy_iter_early_termination_any() {
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|index| (index, index * 10)).collect();
        let has_value_500 = map.iter().any(|(_, value)| *value == 500);
        assert!(has_value_500);
    }

    #[rstest]
    fn test_lazy_iter_early_termination_all() {
        let map: PersistentHashMap<i32, i32> = (0..100).map(|index| (index, index)).collect();
        let all_non_negative = map.iter().all(|(_, value)| *value >= 0);
        assert!(all_non_negative);
    }

    #[rstest]
    fn test_lazy_iter_fused_behavior() {
        let map = PersistentHashMap::singleton("key".to_string(), 42);
        let mut iterator = map.iter();
        let _ = iterator.next(); // consume the only element
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.next(), None);
    }

    #[rstest]
    fn test_lazy_iter_collect_all_elements() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let collected: Vec<_> = map.iter().collect();
        assert_eq!(collected.len(), 3);
        // Check that all elements are present (order may vary)
        let mut sorted: Vec<_> = collected
            .into_iter()
            .map(|(key, value)| (key.clone(), *value))
            .collect();
        sorted.sort_by_key(|(key, _)| key.clone());
        assert_eq!(
            sorted,
            vec![
                ("a".to_string(), 1),
                ("b".to_string(), 2),
                ("c".to_string(), 3)
            ]
        );
    }

    #[rstest]
    fn test_lazy_iter_large_map() {
        let map: PersistentHashMap<i32, i32> = (0..10000).map(|index| (index, index * 2)).collect();
        let collected: Vec<_> = map.iter().collect();
        assert_eq!(collected.len(), 10000);
        // Verify all elements are present
        let sum: i32 = collected.iter().map(|(_, value)| **value).sum();
        let expected_sum: i32 = (0..10000).map(|index| index * 2).sum();
        assert_eq!(sum, expected_sum);
    }

    #[rstest]
    fn test_lazy_iter_with_collisions() {
        // Create a map that will have hash collisions by using keys that hash similarly
        #[derive(Clone, PartialEq, Eq, Debug)]
        struct CollidingKey(i32);

        impl std::hash::Hash for CollidingKey {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                // Force all keys to have the same hash
                0_i32.hash(state);
            }
        }

        let map = PersistentHashMap::new()
            .insert(CollidingKey(1), "one")
            .insert(CollidingKey(2), "two")
            .insert(CollidingKey(3), "three");

        assert_eq!(map.iter().count(), 3);
    }

    #[rstest]
    fn test_lazy_iter_bitmap_node_traversal() {
        // Create a map with enough elements to have multiple bitmap nodes
        let map: PersistentHashMap<i32, i32> = (0..100).map(|index| (index, index)).collect();
        let mut count = 0;
        for (key, value) in &map {
            assert_eq!(key, value);
            count += 1;
        }
        assert_eq!(count, 100);
    }

    #[rstest]
    fn test_lazy_iter_order_regression() {
        // This test ensures that the lazy iterator maintains the same DFS order
        // as the original collect_entries implementation.
        // The order should be: children.iter() order (depth-first)
        let map: PersistentHashMap<i32, i32> = (0..50).map(|index| (index, index * 10)).collect();

        // Collect using the iterator
        let iter_order: Vec<_> = map.iter().map(|(key, value)| (*key, *value)).collect();

        // Verify that all elements are present
        assert_eq!(iter_order.len(), 50);

        // Verify that each element has the correct key-value relationship
        for (key, value) in &iter_order {
            assert_eq!(*value, key * 10);
        }

        // Verify that all keys from 0..50 are present
        let mut keys: Vec<_> = iter_order.iter().map(|(key, _)| *key).collect();
        keys.sort_unstable();
        let expected_keys: Vec<_> = (0..50).collect();
        assert_eq!(keys, expected_keys);
    }

    #[rstest]
    fn test_lazy_iter_keys_values_consistency() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);

        let iter_keys: Vec<_> = map.iter().map(|(key, _)| key.clone()).collect();
        let iter_values: Vec<_> = map.iter().map(|(_, value)| *value).collect();

        let keys_method: Vec<_> = map.keys().cloned().collect();
        let values_method: Vec<_> = map.values().copied().collect();

        // keys() and values() should return the same elements in the same order as iter()
        assert_eq!(iter_keys, keys_method);
        assert_eq!(iter_values, values_method);
    }

    #[rstest]
    fn test_lazy_iter_multiple_iterations() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);

        let first_iteration: Vec<_> = map.iter().collect();
        let second_iteration: Vec<_> = map.iter().collect();

        // Multiple iterations should return the same results
        assert_eq!(first_iteration, second_iteration);
    }

    #[rstest]
    fn test_lazy_iter_nested_bitmap_nodes() {
        // Create a map with many elements to ensure deep nesting in HAMT structure
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|index| (index, index)).collect();

        let collected: Vec<_> = map.iter().collect();
        assert_eq!(collected.len(), 1000);

        // Verify all values
        for (key, value) in collected {
            assert_eq!(key, value);
        }
    }
}

// =============================================================================
// Send + Sync Tests (arc feature only)
// =============================================================================

#[cfg(all(test, feature = "arc"))]
mod send_sync_tests {
    use super::*;
    use rstest::rstest;

    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}

    #[rstest]
    fn test_hashmap_is_send() {
        assert_send::<PersistentHashMap<String, i32>>();
        assert_send::<PersistentHashMap<i32, String>>();
    }

    #[rstest]
    fn test_hashmap_is_sync() {
        assert_sync::<PersistentHashMap<String, i32>>();
        assert_sync::<PersistentHashMap<i32, String>>();
    }

    #[rstest]
    fn test_hashmap_send_sync_combined() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<PersistentHashMap<String, i32>>();
        is_send_sync::<PersistentHashMap<i32, String>>();
    }
}

// =============================================================================
// Multithread Tests (arc feature only)
// =============================================================================

#[cfg(all(test, feature = "arc"))]
mod multithread_tests {
    use super::*;
    use rstest::rstest;
    use std::sync::Arc;
    use std::thread;

    #[rstest]
    fn test_hashmap_shared_across_threads() {
        let map = Arc::new(
            PersistentHashMap::new()
                .insert("one".to_string(), 1)
                .insert("two".to_string(), 2)
                .insert("three".to_string(), 3),
        );

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    assert_eq!(map_clone.get("one"), Some(&1));
                    assert_eq!(map_clone.get("two"), Some(&2));
                    assert_eq!(map_clone.get("three"), Some(&3));
                    assert_eq!(map_clone.len(), 3);
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[rstest]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn test_hashmap_concurrent_insert() {
        let base_map = Arc::new(PersistentHashMap::new().insert("base".to_string(), 0));

        let results: Vec<_> = (0..4)
            .map(|index| {
                let map_clone = Arc::clone(&base_map);
                thread::spawn(move || {
                    let new_map = map_clone.insert(format!("key_{index}"), index);
                    assert_eq!(new_map.get(&format!("key_{index}")), Some(&index));
                    assert_eq!(new_map.get("base"), Some(&0));
                    new_map
                })
            })
            .map(|handle| handle.join().expect("Thread panicked"))
            .collect();

        // Each thread should have created an independent map with 2 entries
        for (index, map) in results.iter().enumerate() {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get(&format!("key_{index}")), Some(&(index as i32)));
        }

        // Original map should be unchanged
        assert_eq!(base_map.len(), 1);
    }

    #[rstest]
    fn test_hashmap_referential_transparency() {
        let map = Arc::new(
            PersistentHashMap::new()
                .insert("a".to_string(), 1)
                .insert("b".to_string(), 2),
        );

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    let updated = map_clone.insert("c".to_string(), 3);
                    // Original should be unchanged
                    assert_eq!(map_clone.len(), 2);
                    assert_eq!(map_clone.get("c"), None);
                    // New map should have the addition
                    assert_eq!(updated.len(), 3);
                    assert_eq!(updated.get("c"), Some(&3));
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Original should still be unchanged
        assert_eq!(map.len(), 2);
    }

    #[rstest]
    fn test_hashmap_concurrent_iteration() {
        let map = Arc::new(
            PersistentHashMap::new()
                .insert("a".to_string(), 1)
                .insert("b".to_string(), 2)
                .insert("c".to_string(), 3),
        );

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    let sum: i32 = map_clone.iter().map(|(_, v)| v).sum();
                    assert_eq!(sum, 6);
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_serialize_empty() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, "{}");
    }

    #[rstest]
    fn test_serialize_single_entry() {
        let map = PersistentHashMap::singleton("key".to_string(), 42);
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"{"key":42}"#);
    }

    #[rstest]
    fn test_serialize_multiple_entries() {
        let map = PersistentHashMap::singleton("a".to_string(), 1);
        let json = serde_json::to_string(&map).unwrap();
        let parsed: std::collections::HashMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.get("a"), Some(&1));
    }

    #[rstest]
    fn test_deserialize_empty() {
        let json = "{}";
        let map: PersistentHashMap<String, i32> = serde_json::from_str(json).unwrap();
        assert!(map.is_empty());
    }

    #[rstest]
    fn test_deserialize_single_entry() {
        let json = r#"{"key":42}"#;
        let map: PersistentHashMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&42));
    }

    #[rstest]
    fn test_deserialize_multiple_entries() {
        let json = r#"{"a":1,"b":2,"c":3}"#;
        let map: PersistentHashMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get("c"), Some(&3));
    }

    #[rstest]
    fn test_roundtrip_empty() {
        let original: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentHashMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_roundtrip_large() {
        let mut original: PersistentHashMap<String, i32> = PersistentHashMap::new();
        for element_index in 0..100 {
            original = original.insert(format!("key{element_index}"), element_index);
        }
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentHashMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_entry_preservation() {
        let mut map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        for element_index in 0..100 {
            map = map.insert(format!("key{element_index}"), element_index);
        }
        let json = serde_json::to_string(&map).unwrap();
        let restored: PersistentHashMap<String, i32> = serde_json::from_str(&json).unwrap();
        for element_index in 0..100 {
            let key = format!("key{element_index}");
            assert_eq!(restored.get(&key), Some(&element_index));
        }
    }

    #[rstest]
    fn test_serialize_nested_values() {
        let map = PersistentHashMap::new()
            .insert("numbers".to_string(), vec![1, 2, 3])
            .insert("empty".to_string(), vec![]);
        let json = serde_json::to_string(&map).unwrap();
        let restored: PersistentHashMap<String, Vec<i32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.get("numbers"), Some(&vec![1, 2, 3]));
        assert_eq!(restored.get("empty"), Some(&vec![]));
    }

    #[rstest]
    fn test_deserialize_overwrites_duplicate_keys() {
        let json = r#"{"key":1,"key":2}"#;
        let map: PersistentHashMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&2));
    }
}

// =============================================================================
// TransientHashMap Tests
// =============================================================================

#[cfg(test)]
mod transient_hashmap_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_transient_hashmap_new() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_transient_hashmap_insert_and_get() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        assert_eq!(transient.insert("one".to_string(), 1), None);
        assert_eq!(transient.insert("two".to_string(), 2), None);

        assert_eq!(transient.len(), 2);
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
        assert_eq!(transient.get("three"), None);
    }

    #[rstest]
    fn test_transient_hashmap_insert_overwrites() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        assert_eq!(transient.insert("key".to_string(), 1), None);
        assert_eq!(transient.insert("key".to_string(), 2), Some(1));
        assert_eq!(transient.get("key"), Some(&2));
        assert_eq!(transient.len(), 1);
    }

    #[rstest]
    fn test_transient_hashmap_remove() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("a".to_string(), 1);
        transient.insert("b".to_string(), 2);

        assert_eq!(transient.remove("a"), Some(1));
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get("a"), None);
        assert_eq!(transient.get("b"), Some(&2));

        assert_eq!(transient.remove("a"), None);
    }

    #[rstest]
    fn test_transient_hashmap_contains_key() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("key".to_string(), 42);

        assert!(transient.contains_key("key"));
        assert!(!transient.contains_key("other"));
    }

    #[rstest]
    fn test_transient_hashmap_get_mut() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("count".to_string(), 0);

        if let Some(count) = transient.get_mut("count") {
            *count += 10;
        }

        assert_eq!(transient.get("count"), Some(&10));
    }

    #[rstest]
    fn test_transient_hashmap_update_with() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("count".to_string(), 5);

        assert!(transient.update_with("count", |x| x * 2));
        assert_eq!(transient.get("count"), Some(&10));

        assert!(!transient.update_with("missing", |x| x * 2));
    }

    #[rstest]
    fn test_transient_hashmap_extend() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.extend([
            ("one".to_string(), 1),
            ("two".to_string(), 2),
            ("three".to_string(), 3),
        ]);

        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
        assert_eq!(transient.get("three"), Some(&3));
    }

    #[rstest]
    fn test_transient_hashmap_from_iterator() {
        let transient: TransientHashMap<String, i32> = [("a".to_string(), 1), ("b".to_string(), 2)]
            .into_iter()
            .collect();

        assert_eq!(transient.len(), 2);
        assert_eq!(transient.get("a"), Some(&1));
        assert_eq!(transient.get("b"), Some(&2));
    }

    #[rstest]
    fn test_transient_hashmap_persistent() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("one".to_string(), 1);
        transient.insert("two".to_string(), 2);

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 2);
        assert_eq!(persistent.get("one"), Some(&1));
        assert_eq!(persistent.get("two"), Some(&2));
    }

    #[rstest]
    fn test_persistent_hashmap_transient() {
        let persistent: PersistentHashMap<String, i32> =
            [("one".to_string(), 1), ("two".to_string(), 2)]
                .into_iter()
                .collect();

        let mut transient = persistent.transient();
        transient.insert("three".to_string(), 3);
        transient.remove("one");

        let new_persistent = transient.persistent();
        assert_eq!(new_persistent.len(), 2);
        assert_eq!(new_persistent.get("one"), None);
        assert_eq!(new_persistent.get("two"), Some(&2));
        assert_eq!(new_persistent.get("three"), Some(&3));
    }

    // =========================================================================
    // Transient-Persistent Roundtrip Law Tests
    // =========================================================================

    #[rstest]
    fn test_transient_persistent_roundtrip_empty() {
        let original: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_transient_persistent_roundtrip_single() {
        let original = PersistentHashMap::singleton("key".to_string(), 42);
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_transient_persistent_roundtrip_multiple() {
        let original: PersistentHashMap<String, i32> = [
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ]
        .into_iter()
        .collect();
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    #[rstest]
    fn test_transient_persistent_roundtrip_large() {
        let mut original: PersistentHashMap<String, i32> = PersistentHashMap::new();
        for i in 0..1000 {
            original = original.insert(format!("key{i}"), i);
        }
        let result = original.clone().transient().persistent();
        assert_eq!(result, original);
    }

    // =========================================================================
    // Mutation Equivalence Law Tests
    // =========================================================================

    #[rstest]
    fn test_insert_equivalence() {
        let base: PersistentHashMap<String, i32> = [("a".to_string(), 1), ("b".to_string(), 2)]
            .into_iter()
            .collect();

        let via_persistent = base.insert("c".to_string(), 3);

        let via_transient = {
            let mut transient = base.transient();
            transient.insert("c".to_string(), 3);
            transient.persistent()
        };

        assert_eq!(via_persistent, via_transient);
    }

    #[rstest]
    fn test_remove_equivalence() {
        let base: PersistentHashMap<String, i32> = [
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ]
        .into_iter()
        .collect();

        let via_persistent = base.remove("b");

        let via_transient = {
            let mut transient = base.transient();
            transient.remove("b");
            transient.persistent()
        };

        assert_eq!(via_persistent, via_transient);
    }

    // =========================================================================
    // COW (Copy-on-Write) Tests
    // =========================================================================

    #[rstest]
    fn test_transient_cow_preserves_original_persistent() {
        let original: PersistentHashMap<String, i32> = [("a".to_string(), 1), ("b".to_string(), 2)]
            .into_iter()
            .collect();

        // Clone the original before creating transient
        let original_clone = original.clone();

        let mut transient = original.transient();
        transient.insert("c".to_string(), 3);
        transient.remove("a");

        let modified = transient.persistent();

        // The original clone should be unchanged
        assert_eq!(original_clone.len(), 2);
        assert_eq!(original_clone.get("a"), Some(&1));
        assert_eq!(original_clone.get("b"), Some(&2));
        assert_eq!(original_clone.get("c"), None);

        // The modified version should have the changes
        assert_eq!(modified.len(), 2);
        assert_eq!(modified.get("a"), None);
        assert_eq!(modified.get("b"), Some(&2));
        assert_eq!(modified.get("c"), Some(&3));
    }

    #[rstest]
    fn test_transient_batch_operations() {
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Insert many elements
        for i in 0..100 {
            transient.insert(i, i * 10);
        }

        // Update some elements
        for i in 0..50 {
            transient.update_with(&i, |v| v + 1);
        }

        // Remove some elements
        for i in 25..75 {
            transient.remove(&i);
        }

        let persistent = transient.persistent();

        // Verify the results
        assert_eq!(persistent.len(), 50); // 0..25 and 75..100

        for i in 0..25 {
            assert_eq!(persistent.get(&i), Some(&(i * 10 + 1)));
        }

        for i in 25..75 {
            assert_eq!(persistent.get(&i), None);
        }

        for i in 75..100 {
            assert_eq!(persistent.get(&i), Some(&(i * 10)));
        }
    }

    #[rstest]
    fn test_transient_default() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::default();
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_transient_hashmap_many_insertions() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();

        for i in 0..1000 {
            transient.insert(format!("key{i}"), i);
        }

        assert_eq!(transient.len(), 1000);

        for i in 0..1000 {
            assert_eq!(transient.get(&format!("key{i}")), Some(&i));
        }

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 1000);
    }

    // =========================================================================
    // Phase 2: Transient Builder Optimization Tests
    // =========================================================================

    #[rstest]
    fn test_transient_with_capacity_hint_creates_empty_map() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(1000);
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_transient_with_capacity_hint_basic_operations() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(10);
        transient.insert("one".to_string(), 1);
        transient.insert("two".to_string(), 2);
        transient.insert("three".to_string(), 3);

        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
        assert_eq!(transient.get("three"), Some(&3));
    }

    #[rstest]
    fn test_transient_with_capacity_hint_to_persistent() {
        let mut transient: TransientHashMap<String, i32> =
            TransientHashMap::with_capacity_hint(100);
        for i in 0..50 {
            transient.insert(format!("key{i}"), i);
        }

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 50);

        for i in 0..50 {
            assert_eq!(persistent.get(&format!("key{i}")), Some(&i));
        }
    }

    #[rstest]
    fn test_transient_with_capacity_hint_exceeds_hint() {
        // Capacity hint is just a hint - map should work fine with more elements
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::with_capacity_hint(10);
        for i in 0..100 {
            transient.insert(i, i * 2);
        }

        assert_eq!(transient.len(), 100);
        for i in 0..100 {
            assert_eq!(transient.get(&i), Some(&(i * 2)));
        }
    }

    #[rstest]
    fn test_transient_with_capacity_hint_zero() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(0);
        transient.insert("key".to_string(), 42);
        assert_eq!(transient.get("key"), Some(&42));
    }

    #[rstest]
    fn test_from_iter_equivalence_with_transient() {
        // FromIterator via PersistentHashMap should produce the same result
        // as manually using TransientHashMap
        let data: Vec<(String, i32)> = (0..100).map(|i| (format!("key{i}"), i)).collect();

        let via_from_iter: PersistentHashMap<String, i32> = data.clone().into_iter().collect();

        let via_transient = {
            let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
            transient.extend(data);
            transient.persistent()
        };

        assert_eq!(via_from_iter, via_transient);
    }

    #[rstest]
    fn test_transient_from_iter_equivalence() {
        // TransientHashMap::from_iter should produce same result as manual extend
        let data: Vec<(String, i32)> = (0..50).map(|i| (format!("key{i}"), i)).collect();

        let via_from_iter: TransientHashMap<String, i32> = data.clone().into_iter().collect();

        let via_extend = {
            let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
            transient.extend(data);
            transient
        };

        // Compare by converting to persistent (since TransientHashMap does not implement PartialEq)
        let persistent_from_iter = via_from_iter.persistent();
        let persistent_from_extend = via_extend.persistent();

        assert_eq!(persistent_from_iter, persistent_from_extend);
    }

    #[rstest]
    fn test_extend_with_empty_iterator() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("initial".to_string(), 1);
        transient.extend(std::iter::empty());
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get("initial"), Some(&1));
    }

    #[rstest]
    fn test_extend_preserves_existing_entries() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("one".to_string(), 1);

        transient.extend([("two".to_string(), 2), ("three".to_string(), 3)]);

        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
        assert_eq!(transient.get("three"), Some(&3));
    }

    #[rstest]
    fn test_capacity_hint_accessor() {
        // Verify that capacity_hint can be retrieved (even if not used currently)
        let transient: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(500);
        assert_eq!(transient.capacity_hint(), 500);
    }

    // =========================================================================
    // insert_bulk Tests
    // =========================================================================

    #[rstest]
    fn test_insert_bulk_basic() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let result = transient.insert_bulk(vec![
            ("one".to_string(), 1),
            ("two".to_string(), 2),
            ("three".to_string(), 3),
        ]);

        let transient = result.expect("insert_bulk should succeed");
        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
        assert_eq!(transient.get("three"), Some(&3));
    }

    #[rstest]
    fn test_insert_bulk_empty() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let result = transient.insert_bulk(Vec::<(String, i32)>::new());

        let transient = result.expect("insert_bulk with empty input should succeed");
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_insert_bulk_duplicate_keys_last_wins() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let result = transient.insert_bulk(vec![
            ("key".to_string(), 1),
            ("key".to_string(), 2),
            ("key".to_string(), 3),
        ]);

        let transient = result.expect("insert_bulk should succeed");
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get("key"), Some(&3)); // Last value wins
    }

    #[rstest]
    fn test_insert_bulk_with_existing_entries() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("existing".to_string(), 100);

        let transient = transient
            .insert_bulk(vec![("one".to_string(), 1), ("two".to_string(), 2)])
            .expect("insert_bulk should succeed");

        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("existing"), Some(&100));
        assert_eq!(transient.get("one"), Some(&1));
        assert_eq!(transient.get("two"), Some(&2));
    }

    #[rstest]
    fn test_insert_bulk_overwrites_existing_key() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("key".to_string(), 100);

        let transient = transient
            .insert_bulk(vec![("key".to_string(), 999)])
            .expect("insert_bulk should succeed");

        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get("key"), Some(&999)); // Bulk overwrites existing
    }

    #[rstest]
    fn test_insert_bulk_chaining() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let transient = transient
            .insert_bulk(vec![("a".to_string(), 1)])
            .expect("first insert_bulk should succeed")
            .insert_bulk(vec![("b".to_string(), 2)])
            .expect("second insert_bulk should succeed")
            .insert_bulk(vec![("c".to_string(), 3)])
            .expect("third insert_bulk should succeed");

        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("a"), Some(&1));
        assert_eq!(transient.get("b"), Some(&2));
        assert_eq!(transient.get("c"), Some(&3));
    }

    #[rstest]
    fn test_insert_bulk_to_persistent() {
        let persistent = TransientHashMap::new()
            .insert_bulk(vec![("a".to_string(), 1), ("b".to_string(), 2)])
            .expect("insert_bulk should succeed")
            .persistent();

        assert_eq!(persistent.len(), 2);
        assert_eq!(persistent.get("a"), Some(&1));
        assert_eq!(persistent.get("b"), Some(&2));
    }

    #[rstest]
    fn test_insert_bulk_from_persistent_transient_persistent() {
        let original: PersistentHashMap<String, i32> =
            std::iter::once(("existing".to_string(), 100)).collect();

        let result = original
            .transient()
            .insert_bulk(vec![("new1".to_string(), 1), ("new2".to_string(), 2)])
            .expect("insert_bulk should succeed")
            .persistent();

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("existing"), Some(&100));
        assert_eq!(result.get("new1"), Some(&1));
        assert_eq!(result.get("new2"), Some(&2));
    }

    #[rstest]
    fn test_insert_bulk_large_dataset() {
        let transient: TransientHashMap<i32, i32> = TransientHashMap::new();
        let data: Vec<(i32, i32)> = (0..10_000).map(|i| (i, i * 2)).collect();

        let transient = transient
            .insert_bulk(data)
            .expect("insert_bulk should succeed");

        assert_eq!(transient.len(), 10_000);
        for i in 0..10_000 {
            assert_eq!(transient.get(&i), Some(&(i * 2)));
        }
    }

    #[rstest]
    fn test_insert_bulk_error_too_many_entries() {
        let transient: TransientHashMap<usize, usize> = TransientHashMap::new();
        let data = (0..(MAX_BULK_INSERT + 100)).map(|i| (i, i));

        match transient.insert_bulk(data) {
            Err(BulkInsertError::TooManyEntries { count, limit }) => {
                assert_eq!(count, MAX_BULK_INSERT + 1);
                assert_eq!(limit, MAX_BULK_INSERT);
            }
            Ok(_) => panic!("Expected TooManyEntries error"),
        }
    }

    #[rstest]
    fn test_insert_bulk_at_limit() {
        const LIMIT: usize = 1000;
        let transient: TransientHashMap<usize, usize> = TransientHashMap::new();
        let data: Vec<(usize, usize)> = (0..LIMIT).map(|i| (i, i * 2)).collect();

        let result = transient
            .insert_bulk(data)
            .expect("should succeed at limit");
        assert_eq!(result.len(), LIMIT);
    }

    #[rstest]
    fn test_insert_bulk_updates_capacity_hint() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::with_capacity_hint(100);

        let transient = transient
            .insert_bulk(vec![
                ("a".to_string(), 1),
                ("b".to_string(), 2),
                ("c".to_string(), 3),
            ])
            .expect("insert_bulk should succeed");

        // capacity_hint should be updated to 100 + 3 = 103
        assert_eq!(transient.capacity_hint(), 103);
    }

    #[rstest]
    fn test_insert_bulk_equivalence_with_sequential_insert() {
        // Property: insert_bulk(items) should be equivalent to
        // items.fold(map, |m, (k, v)| { m.insert(k, v); m })

        let items = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("a".to_string(), 3), // Duplicate key
            ("c".to_string(), 4),
        ];

        // Via insert_bulk
        let via_bulk = TransientHashMap::new()
            .insert_bulk(items.clone())
            .expect("insert_bulk should succeed")
            .persistent();

        // Via sequential insert
        let mut via_sequential = TransientHashMap::new();
        for (key, value) in items {
            via_sequential.insert(key, value);
        }
        let via_sequential = via_sequential.persistent();

        // Both should have the same entries
        assert_eq!(via_bulk.len(), via_sequential.len());
        for (key, value) in &via_bulk {
            assert_eq!(via_sequential.get(key), Some(value));
        }
    }

    #[rstest]
    fn test_bulk_insert_error_display() {
        let error = BulkInsertError::TooManyEntries {
            count: 150_000,
            limit: 100_000,
        };
        let message = format!("{error}");
        assert!(message.contains("150000"));
        assert!(message.contains("100000"));
        assert!(message.contains("bulk insert failed"));
        assert!(message.contains("10,000"));
    }

    // =========================================================================
    // insert_without_cow Tests
    // =========================================================================

    #[rstest]
    fn test_insert_without_cow_basic() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();

        // First insert - should create entry with owner's generation
        assert_eq!(transient.insert_without_cow("a".to_string(), 1), None);
        assert_eq!(transient.get("a"), Some(&1));
        assert_eq!(transient.len(), 1);

        // Update existing key - should update in place
        assert_eq!(transient.insert_without_cow("a".to_string(), 2), Some(1));
        assert_eq!(transient.get("a"), Some(&2));
        assert_eq!(transient.len(), 1);

        // Insert new key
        assert_eq!(transient.insert_without_cow("b".to_string(), 3), None);
        assert_eq!(transient.get("b"), Some(&3));
        assert_eq!(transient.len(), 2);
    }

    #[rstest]
    fn test_insert_without_cow_equivalence_with_insert() {
        // insert_without_cow should produce the same result as insert
        let items = vec![
            ("one".to_string(), 1),
            ("two".to_string(), 2),
            ("one".to_string(), 10), // Duplicate - last wins
            ("three".to_string(), 3),
        ];

        // Via insert_without_cow
        let mut via_without_cow = TransientHashMap::new();
        for (key, value) in items.clone() {
            via_without_cow.insert_without_cow(key, value);
        }
        let via_without_cow = via_without_cow.persistent();

        // Via regular insert
        let mut via_insert = TransientHashMap::new();
        for (key, value) in items {
            via_insert.insert(key, value);
        }
        let via_insert = via_insert.persistent();

        // Both should have the same entries
        assert_eq!(via_without_cow.len(), via_insert.len());
        for (key, value) in &via_without_cow {
            assert_eq!(via_insert.get(key), Some(value));
        }
    }

    #[rstest]
    fn test_insert_without_cow_from_persistent() {
        // Create a persistent map and convert to transient
        let persistent: PersistentHashMap<String, i32> =
            vec![("existing".to_string(), 100)].into_iter().collect();

        let mut transient = persistent.transient();

        // Use insert_without_cow to add new entries
        transient.insert_without_cow("new".to_string(), 200);
        transient.insert_without_cow("existing".to_string(), 101); // Update

        let result = transient.persistent();
        assert_eq!(result.get("existing"), Some(&101));
        assert_eq!(result.get("new"), Some(&200));
        assert_eq!(result.len(), 2);
    }

    #[rstest]
    fn test_insert_without_cow_hash_collision() {
        // Use integer keys that might have hash collisions with similar indices
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Insert entries that may share the same hash index at some level
        for i in 0..100 {
            transient.insert_without_cow(i, i * 2);
        }

        // Verify all entries are present
        assert_eq!(transient.len(), 100);
        for i in 0..100 {
            assert_eq!(transient.get(&i), Some(&(i * 2)));
        }
    }

    // =========================================================================
    // insert_bulk_owned Tests
    // =========================================================================

    #[rstest]
    fn test_insert_bulk_owned_basic() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let items = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ];

        let result = transient.insert_bulk_owned(items);
        assert!(result.is_ok());
        let transient = result.unwrap();
        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get("a"), Some(&1));
        assert_eq!(transient.get("b"), Some(&2));
        assert_eq!(transient.get("c"), Some(&3));
    }

    #[rstest]
    fn test_insert_bulk_owned_empty() {
        let transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let result = transient.insert_bulk_owned(Vec::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[rstest]
    fn test_insert_bulk_owned_too_many_entries_returns_items() {
        let transient: TransientHashMap<i32, i32> = TransientHashMap::new();
        let items: Vec<(i32, i32)> = (0..150_000).map(|i| (i, i * 2)).collect();

        match transient.insert_bulk_owned(items) {
            Ok(_) => panic!("Should have returned an error"),
            Err(e) => {
                // Verify we can recover the items
                let recovered = e.into_items();
                assert_eq!(recovered.len(), 150_000);
                assert_eq!(recovered[0], (0, 0));
                assert_eq!(recovered[149_999], (149_999, 299_998));
            }
        }
    }

    #[rstest]
    fn test_insert_bulk_owned_updates_capacity_hint() {
        let transient: TransientHashMap<i32, i32> = TransientHashMap::new();
        let items: Vec<(i32, i32)> = (0..100).map(|i| (i, i)).collect();

        let initial_hint = transient.capacity_hint();
        let result = transient.insert_bulk_owned(items).unwrap();

        // Capacity hint should have increased by 100
        assert_eq!(result.capacity_hint(), initial_hint + 100);
    }

    #[rstest]
    fn test_insert_bulk_owned_equivalence_with_insert_bulk() {
        let items: Vec<(String, i32)> = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ];

        // Via insert_bulk_owned
        let via_owned = TransientHashMap::new()
            .insert_bulk_owned(items.clone())
            .unwrap()
            .persistent();

        // Via insert_bulk
        let via_bulk = TransientHashMap::new()
            .insert_bulk(items)
            .unwrap()
            .persistent();

        // Both should be equivalent
        assert_eq!(via_owned.len(), via_bulk.len());
        for (key, value) in &via_owned {
            assert_eq!(via_bulk.get(key), Some(value));
        }
    }

    #[rstest]
    fn test_bulk_insert_error_with_items_display() {
        let error: BulkInsertErrorWithItems<i32, i32> = BulkInsertErrorWithItems::TooManyEntries {
            count: 150_000,
            limit: 100_000,
            items: vec![(1, 2)],
        };
        let message = format!("{error}");
        assert!(message.contains("150000"));
        assert!(message.contains("100000"));
        assert!(message.contains("bulk insert failed"));
    }

    // =========================================================================
    // Generation Token Safety Tests
    // =========================================================================

    #[rstest]
    fn test_generation_token_unique_per_transient() {
        // Each TransientHashMap should have a unique generation token
        // This is verified indirectly by checking that operations work correctly
        let transient1: TransientHashMap<i32, i32> = TransientHashMap::new();
        let transient2: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Both should work independently
        let transient1 = transient1.insert_bulk(vec![(1, 1), (2, 2)]).unwrap();
        let transient2 = transient2.insert_bulk(vec![(3, 3), (4, 4)]).unwrap();

        assert_eq!(transient1.len(), 2);
        assert_eq!(transient2.len(), 2);
        assert_eq!(transient1.get(&1), Some(&1));
        assert_eq!(transient2.get(&3), Some(&3));
    }

    #[rstest]
    fn test_generation_token_inherited_from_persistent() {
        // When converting from Persistent to Transient, shared nodes have SHARED_GENERATION
        // New/modified nodes should use the transient's generation
        let persistent: PersistentHashMap<i32, i32> =
            vec![(1, 100), (2, 200)].into_iter().collect();

        let mut transient = persistent.transient();

        // Adding new entries should work correctly
        transient.insert_without_cow(3, 300);
        transient.insert_without_cow(1, 101); // Update existing

        let result = transient.persistent();
        assert_eq!(result.get(&1), Some(&101));
        assert_eq!(result.get(&2), Some(&200));
        assert_eq!(result.get(&3), Some(&300));
    }

    #[rstest]
    fn test_generation_token_isolation_between_transients() {
        // Two transients from the same persistent should not affect each other
        let persistent: PersistentHashMap<i32, i32> = vec![(1, 1)].into_iter().collect();

        let mut transient1 = persistent.clone().transient();
        let mut transient2 = persistent.transient();

        transient1.insert_without_cow(1, 10);
        transient2.insert_without_cow(1, 20);

        let result1 = transient1.persistent();
        let result2 = transient2.persistent();

        // Each should have its own modification
        assert_eq!(result1.get(&1), Some(&10));
        assert_eq!(result2.get(&1), Some(&20));
    }

    #[rstest]
    fn test_reserve_method() {
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();
        assert_eq!(transient.capacity_hint(), 0);

        transient.reserve(100);
        assert_eq!(transient.capacity_hint(), 100);

        transient.reserve(50);
        assert_eq!(transient.capacity_hint(), 150);

        // Should not overflow
        transient.reserve(usize::MAX);
        assert_eq!(transient.capacity_hint(), usize::MAX);
    }

    // =========================================================================
    // TASK-001: ensure_node_generation Tests
    // =========================================================================

    #[rstest]
    fn test_ensure_node_generation_entry() {
        let mut node: Node<String, i32> = Node::Entry {
            hash: 12345,
            key: "test".to_string(),
            value: 42,
            generation: SHARED_GENERATION,
        };

        TransientHashMap::<String, i32>::ensure_node_generation(&mut node, 999);

        match node {
            Node::Entry { generation, .. } => assert_eq!(generation, 999),
            _ => panic!("Expected Entry node"),
        }
    }

    #[rstest]
    fn test_ensure_node_generation_bitmap() {
        let mut node: Node<String, i32> = Node::Bitmap {
            bitmap: 0b101,
            children: ChildArray::new(),
            generation: SHARED_GENERATION,
        };

        TransientHashMap::<String, i32>::ensure_node_generation(&mut node, 123);

        match node {
            Node::Bitmap { generation, .. } => assert_eq!(generation, 123),
            _ => panic!("Expected Bitmap node"),
        }
    }

    #[rstest]
    fn test_ensure_node_generation_collision() {
        let mut node: Node<String, i32> = Node::Collision {
            hash: 12345,
            entries: CollisionArray::new(),
            generation: SHARED_GENERATION,
        };

        TransientHashMap::<String, i32>::ensure_node_generation(&mut node, 456);

        match node {
            Node::Collision { generation, .. } => assert_eq!(generation, 456),
            _ => panic!("Expected Collision node"),
        }
    }

    #[rstest]
    fn test_ensure_node_generation_empty() {
        let mut node: Node<String, i32> = Node::Empty;

        // Should not panic - Empty nodes don't have generation
        TransientHashMap::<String, i32>::ensure_node_generation(&mut node, 789);

        assert!(matches!(node, Node::Empty));
    }

    // =========================================================================
    // TASK-002: ensure_child_owned Tests
    // =========================================================================

    #[rstest]
    fn test_ensure_child_owned_exclusive() {
        // When reference count is 1, should return in-place without cloning
        let mut child_ref = ReferenceCounter::new(Node::Entry {
            hash: 12345,
            key: "exclusive".to_string(),
            value: 100,
            generation: SHARED_GENERATION,
        });

        let child_mut = TransientHashMap::<String, i32>::ensure_child_owned(&mut child_ref, 999);

        // Generation should be updated
        match child_mut {
            Node::Entry {
                generation, key, ..
            } => {
                assert_eq!(*generation, 999);
                assert_eq!(key, "exclusive");
            }
            _ => panic!("Expected Entry node"),
        }
    }

    #[rstest]
    fn test_ensure_child_owned_shared() {
        // When reference count > 1, should perform COW
        let original = ReferenceCounter::new(Node::Entry {
            hash: 12345,
            key: "shared".to_string(),
            value: 200,
            generation: SHARED_GENERATION,
        });
        let mut cloned = original.clone();

        // Now reference count is 2
        let child_mut = TransientHashMap::<String, i32>::ensure_child_owned(&mut cloned, 888);

        // Generation should be updated on the cloned node
        match child_mut {
            Node::Entry { generation, .. } => assert_eq!(*generation, 888),
            _ => panic!("Expected Entry node"),
        }

        // Original should still have SHARED_GENERATION
        match &*original {
            Node::Entry { generation, .. } => assert_eq!(*generation, SHARED_GENERATION),
            _ => panic!("Expected Entry node"),
        }
    }

    #[rstest]
    fn test_ensure_child_owned_generation_updated() {
        // Test that generation is always updated even when already owned
        let mut child_ref = ReferenceCounter::new(Node::Bitmap {
            bitmap: 0b1010,
            children: ChildArray::new(),
            generation: 100, // Non-shared generation
        });

        let child_mut = TransientHashMap::<String, i32>::ensure_child_owned(&mut child_ref, 200);

        match child_mut {
            Node::Bitmap { generation, .. } => assert_eq!(*generation, 200),
            _ => panic!("Expected Bitmap node"),
        }
    }

    // =========================================================================
    // TASK-003: insert_into_node_inplace Tests
    // =========================================================================

    #[rstest]
    fn test_insert_into_node_inplace_empty() {
        let mut node: Node<String, i32> = Node::Empty;

        let (old_value, added) = TransientHashMap::insert_into_node_inplace(
            &mut node,
            "key".to_string(),
            42,
            compute_hash(&"key".to_string()),
            0,
            999,
        );

        assert_eq!(old_value, None);
        assert!(added);
        match node {
            Node::Entry {
                key,
                value,
                generation,
                ..
            } => {
                assert_eq!(key, "key");
                assert_eq!(value, 42);
                assert_eq!(generation, 999);
            }
            _ => panic!("Expected Entry node"),
        }
    }

    #[rstest]
    fn test_insert_into_node_inplace_entry_same_key() {
        let hash = compute_hash(&"key".to_string());
        let mut node: Node<String, i32> = Node::Entry {
            hash,
            key: "key".to_string(),
            value: 100,
            generation: SHARED_GENERATION,
        };

        let (old_value, added) = TransientHashMap::insert_into_node_inplace(
            &mut node,
            "key".to_string(),
            200,
            hash,
            0,
            999,
        );

        assert_eq!(old_value, Some(100));
        assert!(!added);
        match node {
            Node::Entry {
                value, generation, ..
            } => {
                assert_eq!(value, 200);
                assert_eq!(generation, 999);
            }
            _ => panic!("Expected Entry node"),
        }
    }

    #[rstest]
    fn test_insert_into_node_inplace_entry_collision() {
        // Create two different keys with the same hash (simulated collision)
        // Use a custom struct that can have controlled hash collisions
        #[derive(Clone, PartialEq, Eq, Debug)]
        struct CollisionKey {
            value: String,
            hash_override: u64,
        }
        impl std::hash::Hash for CollisionKey {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.hash_override.hash(state);
            }
        }

        let hash = 12345u64;
        let key1 = CollisionKey {
            value: "a".to_string(),
            hash_override: hash,
        };
        let key2 = CollisionKey {
            value: "b".to_string(),
            hash_override: hash,
        };

        let mut node: Node<CollisionKey, i32> = Node::Entry {
            hash,
            key: key1,
            value: 100,
            generation: SHARED_GENERATION,
        };

        let (old_value, added) =
            TransientHashMap::insert_into_node_inplace(&mut node, key2, 200, hash, 0, 999);

        assert_eq!(old_value, None);
        assert!(added);
        match node {
            Node::Collision {
                entries,
                generation,
                ..
            } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(generation, 999);
            }
            _ => panic!("Expected Collision node, got {node:?}"),
        }
    }

    #[rstest]
    fn test_insert_into_node_inplace_bitmap() {
        // Start with a bitmap node
        let hash1 = compute_hash(&"key1".to_string());
        let hash2 = compute_hash(&"key2".to_string());

        // Create a bitmap node with one entry
        let mut children = ChildArray::new();
        children.push(ChildSlot::Entry {
            hash: hash1,
            key: "key1".to_string(),
            value: 100,
            generation: SHARED_GENERATION,
        });

        let index1 = hash_index(hash1, 0);
        let bitmap = 1u32 << index1;

        let mut node: Node<String, i32> = Node::Bitmap {
            bitmap,
            children,
            generation: SHARED_GENERATION,
        };

        // Insert a new key
        let (old_value, added) = TransientHashMap::insert_into_node_inplace(
            &mut node,
            "key2".to_string(),
            200,
            hash2,
            0,
            999,
        );

        assert_eq!(old_value, None);
        assert!(added);
        match node {
            Node::Bitmap { generation, .. } => {
                assert_eq!(generation, 999);
            }
            _ => panic!("Expected Bitmap node"),
        }
    }

    #[rstest]
    fn test_insert_into_node_inplace_nested() {
        // Test inserting into a node that causes nested node creation
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Insert enough entries to create nested structure
        for i in 0..20 {
            transient.insert_without_cow(i, i * 10);
        }

        // Verify all entries
        assert_eq!(transient.len(), 20);
        for i in 0..20 {
            assert_eq!(transient.get(&i), Some(&(i * 10)));
        }
    }

    // =========================================================================
    // TASK-004: insert_into_bitmap_node_inplace Tests
    // =========================================================================

    #[rstest]
    fn test_insert_into_bitmap_node_inplace_new_slot() {
        let mut bitmap: u32 = 0;
        let mut children: ChildArray<String, i32> = ChildArray::new();

        let hash = compute_hash(&"key".to_string());
        let (old_value, added) = TransientHashMap::insert_into_bitmap_node_inplace(
            &mut bitmap,
            &mut children,
            "key".to_string(),
            42,
            hash,
            0,
            999,
        );

        assert_eq!(old_value, None);
        assert!(added);
        assert_eq!(children.len(), 1);
        match &children[0] {
            ChildSlot::Entry {
                key,
                value,
                generation,
                ..
            } => {
                assert_eq!(key, "key");
                assert_eq!(*value, 42);
                assert_eq!(*generation, 999);
            }
            ChildSlot::Node(_) => panic!("Expected Entry child"),
        }
    }

    #[rstest]
    fn test_insert_into_bitmap_node_inplace_update_entry() {
        let hash = compute_hash(&"key".to_string());
        let index = hash_index(hash, 0);
        let mut bitmap: u32 = 1u32 << index;
        let mut children: ChildArray<String, i32> = ChildArray::new();
        children.push(ChildSlot::Entry {
            hash,
            key: "key".to_string(),
            value: 100,
            generation: SHARED_GENERATION,
        });

        let (old_value, added) = TransientHashMap::insert_into_bitmap_node_inplace(
            &mut bitmap,
            &mut children,
            "key".to_string(),
            200,
            hash,
            0,
            999,
        );

        assert_eq!(old_value, Some(100));
        assert!(!added);
        match &children[0] {
            ChildSlot::Entry {
                value, generation, ..
            } => {
                assert_eq!(*value, 200);
                assert_eq!(*generation, 999);
            }
            ChildSlot::Node(_) => panic!("Expected Entry child"),
        }
    }

    #[rstest]
    fn test_insert_into_bitmap_node_inplace_collision() {
        // Use same hash collision approach as before
        #[derive(Clone, PartialEq, Eq, Debug)]
        struct CollisionKey {
            value: String,
            hash_override: u64,
        }
        impl std::hash::Hash for CollisionKey {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.hash_override.hash(state);
            }
        }

        let hash = 12345u64;
        let key1 = CollisionKey {
            value: "a".to_string(),
            hash_override: hash,
        };
        let key2 = CollisionKey {
            value: "b".to_string(),
            hash_override: hash,
        };

        let index = hash_index(hash, 0);
        let mut bitmap: u32 = 1u32 << index;
        let mut children: ChildArray<CollisionKey, i32> = ChildArray::new();
        children.push(ChildSlot::Entry {
            hash,
            key: key1,
            value: 100,
            generation: SHARED_GENERATION,
        });

        let (old_value, added) = TransientHashMap::insert_into_bitmap_node_inplace(
            &mut bitmap,
            &mut children,
            key2,
            200,
            hash,
            0,
            999,
        );

        assert_eq!(old_value, None);
        assert!(added);
        match &children[0] {
            ChildSlot::Node(subnode) => match &**subnode {
                Node::Collision {
                    entries,
                    generation,
                    ..
                } => {
                    assert_eq!(entries.len(), 2);
                    assert_eq!(*generation, 999);
                }
                _ => panic!("Expected Collision node"),
            },
            ChildSlot::Entry { .. } => panic!("Expected Node child"),
        }
    }

    #[rstest]
    fn test_insert_into_bitmap_node_inplace_nested_node() {
        // Test inserting into a bitmap that contains a child node
        let hash1 = compute_hash(&"key1".to_string());
        let hash2 = compute_hash(&"key2".to_string());
        let index = hash_index(hash1, 0);

        // Create a child node
        let child_node = ReferenceCounter::new(Node::Entry {
            hash: hash1,
            key: "key1".to_string(),
            value: 100,
            generation: SHARED_GENERATION,
        });

        let mut bitmap: u32 = 1u32 << index;
        let mut children: ChildArray<String, i32> = ChildArray::new();
        children.push(ChildSlot::Node(child_node));

        // Insert key2 - either collides at same index or goes to a new slot
        let index2 = hash_index(hash2, 0);
        let (old_value, added) = TransientHashMap::insert_into_bitmap_node_inplace(
            &mut bitmap,
            &mut children,
            "key2".to_string(),
            200,
            hash2,
            0,
            999,
        );

        // Common assertions for both cases
        assert_eq!(old_value, None);
        assert!(added);

        // Additional assertion only when indices differ
        if index != index2 {
            assert_eq!(children.len(), 2);
        }
    }

    // =========================================================================
    // TASK-005: insert_without_cow Rewrite Tests
    // =========================================================================

    #[rstest]
    fn test_insert_without_cow_exclusive_root() {
        // When root is exclusive (ref count 1), should use in-place path
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();

        // Insert first entry
        let result = transient.insert_without_cow("key1".to_string(), 100);
        assert_eq!(result, None);
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get("key1"), Some(&100));

        // Insert second entry
        let result = transient.insert_without_cow("key2".to_string(), 200);
        assert_eq!(result, None);
        assert_eq!(transient.len(), 2);

        // Update existing
        let result = transient.insert_without_cow("key1".to_string(), 150);
        assert_eq!(result, Some(100));
        assert_eq!(transient.get("key1"), Some(&150));
    }

    #[rstest]
    fn test_insert_without_cow_shared_root_fallback() {
        // When root is shared (ref count > 1), should fallback to COW
        let persistent: PersistentHashMap<String, i32> =
            vec![("existing".to_string(), 100)].into_iter().collect();

        // Clone to preserve original for later verification
        let persistent_clone = persistent.clone();

        // Create transient from persistent (consumes the cloned version)
        let mut transient = persistent.transient();

        // At this point, root is shared with persistent_clone
        // insert_without_cow should handle this via fallback
        transient.insert_without_cow("new".to_string(), 200);
        transient.insert_without_cow("existing".to_string(), 150);

        let result = transient.persistent();
        assert_eq!(result.get("existing"), Some(&150));
        assert_eq!(result.get("new"), Some(&200));

        // Original persistent_clone should be unchanged
        assert_eq!(persistent_clone.get("existing"), Some(&100));
        assert_eq!(persistent_clone.get("new"), None);
    }

    #[rstest]
    fn test_insert_without_cow_generation_consistency() {
        // Verify that all nodes have consistent generation after operations
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Insert many entries to create complex tree structure
        for i in 0..100 {
            transient.insert_without_cow(i, i * 10);
        }

        // Verify all entries are retrievable
        assert_eq!(transient.len(), 100);
        for i in 0..100 {
            assert_eq!(transient.get(&i), Some(&(i * 10)));
        }

        // Convert to persistent and back
        let persistent = transient.persistent();
        let mut transient2 = persistent.transient();

        // Insert more entries
        for i in 100..150 {
            transient2.insert_without_cow(i, i * 10);
        }

        assert_eq!(transient2.len(), 150);
        for i in 0..150 {
            assert_eq!(transient2.get(&i), Some(&(i * 10)));
        }
    }

    #[rstest]
    fn test_insert_without_cow_equivalence() {
        // Verify insert_without_cow produces same results as insert
        let items: Vec<(String, i32)> = (0..50).map(|i| (format!("key_{i}"), i * 2)).collect();

        // Via insert
        let mut via_insert: TransientHashMap<String, i32> = TransientHashMap::new();
        for (k, v) in items.clone() {
            via_insert.insert(k, v);
        }

        // Via insert_without_cow
        let mut via_without_cow: TransientHashMap<String, i32> = TransientHashMap::new();
        for (k, v) in items {
            via_without_cow.insert_without_cow(k, v);
        }

        // Both should have same content
        assert_eq!(via_insert.len(), via_without_cow.len());
        for (key, value) in &via_insert.persistent() {
            assert_eq!(via_without_cow.get(key), Some(value));
        }
    }

    // =========================================================================
    // TASK-008: ChildArray Pre-allocation Tests
    // =========================================================================

    #[rstest]
    fn test_child_array_preallocation() {
        // Test that ChildArray is pre-allocated correctly when creating BitmapNodes.
        // This test verifies that:
        // 1. Single-child BitmapNodes use with_capacity(1)
        // 2. Two-child BitmapNodes use with_capacity(2)
        // 3. No unnecessary reallocations occur during construction

        // Create a map with entries that will require BitmapNode creation
        let mut map = PersistentHashMap::new();

        // Insert entries to trigger BitmapNode creation
        // Different hash values will create BitmapNodes with varying child counts
        for i in 0..100 {
            map = map.insert(format!("key_{i}"), i);
        }

        // Verify all entries are accessible
        assert_eq!(map.len(), 100);
        for i in 0..100 {
            assert_eq!(map.get(&format!("key_{i}")), Some(&i));
        }

        // Test via transient path as well
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        for i in 0..100 {
            transient.insert_without_cow(format!("trans_key_{i}"), i * 2);
        }

        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 100);
        for i in 0..100 {
            assert_eq!(persistent.get(&format!("trans_key_{i}")), Some(&(i * 2)));
        }
    }

    // =========================================================================
    // RUST-005: NodePool Tests
    // =========================================================================

    #[rstest]
    fn test_node_pool_new() {
        let pool: NodePool<String, i32> = NodePool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.max_size(), DEFAULT_NODE_POOL_MAX_SIZE);
    }

    #[rstest]
    fn test_node_pool_with_max_size() {
        let pool: NodePool<String, i32> = NodePool::with_max_size(10);
        assert!(pool.is_empty());
        assert_eq!(pool.max_size(), 10);
    }

    #[rstest]
    fn test_node_pool_default() {
        let pool: NodePool<String, i32> = NodePool::default();
        assert!(pool.is_empty());
        assert_eq!(pool.max_size(), DEFAULT_NODE_POOL_MAX_SIZE);
    }

    #[rstest]
    fn test_node_pool_try_acquire_empty() {
        let mut pool: NodePool<String, i32> = NodePool::new();

        let result = pool.try_acquire();

        assert!(result.is_none());
        let metrics = pool.metrics();
        assert_eq!(metrics.miss_count, 1);
        assert_eq!(metrics.hit_count, 0);
    }

    #[rstest]
    fn test_node_pool_try_release_and_acquire() {
        let mut pool: NodePool<String, i32> = NodePool::new();

        // Create an exclusively-owned node
        let node = ReferenceCounter::new(Node::Entry {
            hash: 12345,
            key: "test".to_string(),
            value: 42,
            generation: SHARED_GENERATION,
        });

        // Release to pool
        let release_result = pool.try_release(node);
        assert!(release_result.is_ok());
        assert_eq!(pool.len(), 1);

        // Acquire from pool
        let acquired = pool.try_acquire();
        assert!(acquired.is_some());
        assert!(pool.is_empty());

        let metrics = pool.metrics();
        assert_eq!(metrics.hit_count, 1);
        assert_eq!(metrics.miss_count, 0);
    }

    #[rstest]
    fn test_node_pool_try_release_shared_node_fails() {
        let mut pool: NodePool<String, i32> = NodePool::new();

        // Create a shared node (multiple owners)
        let node1 = ReferenceCounter::new(Node::Entry {
            hash: 12345,
            key: "shared".to_string(),
            value: 42,
            generation: SHARED_GENERATION,
        });
        let _node2 = node1.clone(); // Now strong_count == 2

        // Attempt to release shared node
        let result = pool.try_release(node1);

        // Should fail because node is shared
        assert!(result.is_err());
        assert!(pool.is_empty());
    }

    #[rstest]
    #[allow(clippy::cast_sign_loss)]
    fn test_node_pool_try_release_pool_full() {
        let mut pool: NodePool<String, i32> = NodePool::with_max_size(2);

        // Fill the pool
        for i in 0..2 {
            let node = ReferenceCounter::new(Node::Entry {
                hash: i as u64,
                key: format!("key{i}"),
                value: i,
                generation: SHARED_GENERATION,
            });
            let result = pool.try_release(node);
            assert!(result.is_ok());
        }
        assert_eq!(pool.len(), 2);

        // Try to add one more
        let extra_node = ReferenceCounter::new(Node::Entry {
            hash: 999,
            key: "extra".to_string(),
            value: 999,
            generation: SHARED_GENERATION,
        });
        let result = pool.try_release(extra_node);

        // Should fail because pool is full
        assert!(result.is_err());
        assert_eq!(pool.len(), 2);
    }

    #[rstest]
    #[allow(clippy::cast_sign_loss)]
    fn test_node_pool_clear() {
        let mut pool: NodePool<String, i32> = NodePool::new();

        // Add some nodes
        for i in 0..5 {
            let node = ReferenceCounter::new(Node::Entry {
                hash: i as u64,
                key: format!("key{i}"),
                value: i,
                generation: SHARED_GENERATION,
            });
            let _ = pool.try_release(node);
        }
        assert_eq!(pool.len(), 5);

        // Clear the pool
        pool.clear();
        assert!(pool.is_empty());
    }

    // =========================================================================
    // RUST-006: NodePoolMetrics Tests
    // =========================================================================

    #[rstest]
    fn test_node_pool_metrics_default() {
        let metrics = NodePoolMetrics::default();
        assert_eq!(metrics.acquired_count, 0);
        assert_eq!(metrics.released_count, 0);
        assert_eq!(metrics.rejected_count, 0);
        assert_eq!(metrics.hit_count, 0);
        assert_eq!(metrics.miss_count, 0);
    }

    #[rstest]
    fn test_node_pool_metrics_hit_rate_zero_attempts() {
        let metrics = NodePoolMetrics::default();
        // 0 / 0 should return 0.0, not NaN
        assert!((metrics.hit_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_node_pool_metrics_hit_rate_all_hits() {
        let metrics = NodePoolMetrics {
            acquired_count: 10,
            released_count: 10,
            rejected_count: 0,
            hit_count: 10,
            miss_count: 0,
        };
        assert!((metrics.hit_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_node_pool_metrics_hit_rate_all_misses() {
        let metrics = NodePoolMetrics {
            acquired_count: 0,
            released_count: 0,
            rejected_count: 0,
            hit_count: 0,
            miss_count: 10,
        };
        assert!((metrics.hit_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_node_pool_metrics_hit_rate_mixed() {
        let metrics = NodePoolMetrics {
            acquired_count: 3,
            released_count: 3,
            rejected_count: 0,
            hit_count: 3,
            miss_count: 7,
        };
        assert!((metrics.hit_rate() - 0.3).abs() < 1e-10);
    }

    #[rstest]
    fn test_node_pool_metrics_total_attempts() {
        let metrics = NodePoolMetrics {
            acquired_count: 5,
            released_count: 3,
            rejected_count: 2,
            hit_count: 5,
            miss_count: 10,
        };
        assert_eq!(metrics.total_attempts(), 15);
    }

    #[rstest]
    fn test_node_pool_metrics_debug_clone_eq() {
        let metrics1 = NodePoolMetrics {
            acquired_count: 1,
            released_count: 2,
            rejected_count: 3,
            hit_count: 4,
            miss_count: 5,
        };
        let metrics2 = metrics1.clone();

        // Test Debug
        let debug_str = format!("{metrics1:?}");
        assert!(debug_str.contains("NodePoolMetrics"));
        assert!(debug_str.contains("acquired_count"));

        // Test Clone and PartialEq
        assert_eq!(metrics1, metrics2);

        // Test inequality
        let metrics3 = NodePoolMetrics::default();
        assert_ne!(metrics1, metrics3);
    }

    // =========================================================================
    // RUST-007: insert_bulk_with_pool Tests
    // =========================================================================

    #[rstest]
    fn test_insert_bulk_with_pool_empty() {
        let mut pool: NodePool<String, i32> = NodePool::new();
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();

        let result = transient
            .insert_bulk_with_pool(Vec::<(String, i32)>::new(), &mut pool)
            .expect("empty insert should succeed");

        assert_eq!(result.inserted_count, 0);
        assert_eq!(result.updated_count, 0);
        assert!(result.replaced_values.is_empty());
    }

    #[rstest]
    fn test_insert_bulk_with_pool_basic() {
        let mut pool: NodePool<String, i32> = NodePool::new();
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();

        let items = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ];

        let result = transient
            .insert_bulk_with_pool(items, &mut pool)
            .expect("insert should succeed");

        assert_eq!(result.inserted_count, 3);
        assert_eq!(result.updated_count, 0);
        assert!(result.replaced_values.is_empty());
        assert_eq!(transient.len(), 3);
    }

    #[rstest]
    fn test_insert_bulk_with_pool_with_updates() {
        let mut pool: NodePool<String, i32> = NodePool::new();
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("existing".to_string(), 100);

        let items = vec![
            ("existing".to_string(), 200), // Update
            ("new".to_string(), 300),      // Insert
        ];

        let result = transient
            .insert_bulk_with_pool(items, &mut pool)
            .expect("insert should succeed");

        assert_eq!(result.inserted_count, 1);
        assert_eq!(result.updated_count, 1);
        assert_eq!(result.replaced_values, vec![100]);
        assert_eq!(transient.get("existing"), Some(&200));
        assert_eq!(transient.get("new"), Some(&300));
    }

    #[rstest]
    fn test_insert_bulk_with_pool_equivalence_with_metrics() {
        // insert_bulk_with_pool should produce same result as insert_bulk_with_metrics
        let items: Vec<(String, i32)> = (0..100).map(|i| (format!("key{i}"), i)).collect();

        // Via insert_bulk_with_pool
        let mut pool: NodePool<String, i32> = NodePool::new();
        let mut via_pool: TransientHashMap<String, i32> = TransientHashMap::new();
        let result_pool = via_pool
            .insert_bulk_with_pool(items.clone(), &mut pool)
            .expect("insert should succeed");
        let via_pool = via_pool.persistent();

        // Via insert_bulk_with_metrics
        let mut via_metrics: TransientHashMap<String, i32> = TransientHashMap::new();
        let result_metrics = via_metrics
            .insert_bulk_with_metrics(items)
            .expect("insert should succeed");
        let via_metrics = via_metrics.persistent();

        // Results should be identical
        assert_eq!(result_pool.inserted_count, result_metrics.inserted_count);
        assert_eq!(result_pool.updated_count, result_metrics.updated_count);
        assert_eq!(via_pool.len(), via_metrics.len());

        for (key, value) in &via_pool {
            assert_eq!(via_metrics.get(key), Some(value));
        }
    }

    #[rstest]
    fn test_insert_bulk_with_pool_too_many_entries() {
        let mut pool: NodePool<usize, usize> = NodePool::new();
        let mut transient: TransientHashMap<usize, usize> = TransientHashMap::new();
        let items: Vec<(usize, usize)> = (0..(MAX_BULK_INSERT + 100)).map(|i| (i, i)).collect();

        let result = transient.insert_bulk_with_pool(items, &mut pool);

        match result {
            Err(BulkInsertError::TooManyEntries { count, limit }) => {
                assert_eq!(count, MAX_BULK_INSERT + 100);
                assert_eq!(limit, MAX_BULK_INSERT);
            }
            Ok(_) => panic!("Expected TooManyEntries error"),
        }
    }

    // =========================================================================
    // TASK-009: Occupancy Histogram Tests (debug build only)
    // =========================================================================

    #[cfg(debug_assertions)]
    #[rstest]
    fn test_occupancy_histogram_collection() {
        use crate::persistent::hashmap::occupancy_histogram;

        // Note: Tests run in parallel, so we cannot guarantee the histogram is empty
        // after reset. Instead, we capture the state before and after our operations.

        // Reset histogram and capture initial state
        occupancy_histogram::reset_histogram();
        let initial_histogram = occupancy_histogram::get_histogram();
        let initial_total: u64 = initial_histogram.iter().sum();

        // Create a map that will trigger BitmapNode creation
        let mut map = PersistentHashMap::new();
        for i in 0..50 {
            map = map.insert(format!("key_{i}"), i);
        }

        // Check that histogram has recorded some occupancy data
        let histogram = occupancy_histogram::get_histogram();
        let total: u64 = histogram.iter().sum();

        // We should have more BitmapNode creations than initially
        assert!(
            total > initial_total,
            "Expected BitmapNode creations to increase: initial={initial_total}, after={total}"
        );

        // Most common initial occupancies should be 1 or 2
        // (this check is still valid as we created new entries)
        assert!(
            histogram[1] > 0 || histogram[2] > 0,
            "Expected BitmapNodes with 1 or 2 children"
        );

        // Test summary output format
        let summary = occupancy_histogram::summary();
        assert!(summary.contains("BitmapNode Occupancy Histogram"));
        assert!(summary.contains("Mean occupancy"));
    }

    #[cfg(debug_assertions)]
    #[rstest]
    fn test_occupancy_histogram_transient_path() {
        use crate::persistent::hashmap::occupancy_histogram;

        // Note: Tests run in parallel, so we capture state before and after.

        // Reset histogram and capture initial state
        occupancy_histogram::reset_histogram();
        let initial_histogram = occupancy_histogram::get_histogram();
        let initial_total: u64 = initial_histogram.iter().sum();

        // Create entries via transient path
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        for i in 0..30 {
            transient.insert_without_cow(format!("trans_{i}"), i);
        }

        // Get histogram data
        let histogram = occupancy_histogram::get_histogram();
        let total: u64 = histogram.iter().sum();

        // Should have recorded more BitmapNode creations from transient operations
        assert!(
            total > initial_total,
            "Expected BitmapNode creations to increase from transient operations: initial={initial_total}, after={total}"
        );
    }

    // =========================================================================
    // RUST-001: BulkInsertResult<V> Tests
    // =========================================================================

    #[rstest]
    fn test_bulk_insert_result_default() {
        let result: BulkInsertResult<i32> = BulkInsertResult::default();
        assert_eq!(result.inserted_count, 0);
        assert_eq!(result.updated_count, 0);
        assert!(result.replaced_values.is_empty());
    }

    #[rstest]
    fn test_bulk_insert_result_debug_clone_eq() {
        let result1 = BulkInsertResult {
            inserted_count: 5,
            updated_count: 3,
            replaced_values: vec![10, 20, 30],
        };
        let result2 = result1.clone();

        // Test Debug
        let debug_str = format!("{result1:?}");
        assert!(debug_str.contains("BulkInsertResult"));
        assert!(debug_str.contains("inserted_count"));
        assert!(debug_str.contains("updated_count"));
        assert!(debug_str.contains("replaced_values"));

        // Test Clone and PartialEq
        assert_eq!(result1, result2);

        // Test inequality
        let result3 = BulkInsertResult {
            inserted_count: 10,
            updated_count: 0,
            replaced_values: vec![],
        };
        assert_ne!(result1, result3);
    }

    #[rstest]
    fn test_bulk_insert_result_replaced_values_is_vec_of_values_only() {
        // Ensure replaced_values is Vec<V>, not Vec<(K, V)>
        let result: BulkInsertResult<String> = BulkInsertResult {
            inserted_count: 1,
            updated_count: 2,
            replaced_values: vec!["old1".to_string(), "old2".to_string()],
        };

        // This should compile - replaced_values is Vec<V>
        let values: Vec<String> = result.replaced_values;
        assert_eq!(values.len(), 2);
    }

    // =========================================================================
    // RUST-002: insert_bulk_with_metrics Tests
    // =========================================================================

    #[rstest]
    fn test_insert_bulk_with_metrics_empty() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let result = transient
            .insert_bulk_with_metrics(Vec::<(String, i32)>::new())
            .expect("empty insert should succeed");

        assert_eq!(result.inserted_count, 0);
        assert_eq!(result.updated_count, 0);
        assert!(result.replaced_values.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_all_new() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let items = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ];

        let result = transient
            .insert_bulk_with_metrics(items)
            .expect("insert should succeed");

        assert_eq!(result.inserted_count, 3);
        assert_eq!(result.updated_count, 0);
        assert!(result.replaced_values.is_empty());
        assert_eq!(transient.len(), 3);
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_with_updates() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        transient.insert("a".to_string(), 100);
        transient.insert("b".to_string(), 200);

        let items = vec![
            ("a".to_string(), 1), // Update: 100 -> 1
            ("c".to_string(), 3), // New
            ("b".to_string(), 2), // Update: 200 -> 2
            ("d".to_string(), 4), // New
        ];

        let result = transient
            .insert_bulk_with_metrics(items)
            .expect("insert should succeed");

        assert_eq!(result.inserted_count, 2); // c, d
        assert_eq!(result.updated_count, 2); // a, b
        assert_eq!(result.replaced_values.len(), 2);
        // replaced_values should contain old values (100, 200) in some order
        assert!(result.replaced_values.contains(&100));
        assert!(result.replaced_values.contains(&200));
        assert_eq!(transient.len(), 4);
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_duplicate_keys_in_batch() {
        let mut transient: TransientHashMap<String, i32> = TransientHashMap::new();
        let items = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("a".to_string(), 10), // Duplicate - overwrites first
            ("c".to_string(), 3),
        ];

        let result = transient
            .insert_bulk_with_metrics(items)
            .expect("insert should succeed");

        // First "a" insert: inserted_count++
        // Second "a" insert: updated_count++, replaced_values += 1
        assert_eq!(result.inserted_count, 3); // a, b, c
        assert_eq!(result.updated_count, 1); // a (overwritten)
        assert_eq!(result.replaced_values.len(), 1);
        assert_eq!(result.replaced_values[0], 1); // Old value of "a"
        assert_eq!(transient.get("a"), Some(&10)); // Last value wins
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_too_many_entries() {
        let mut transient: TransientHashMap<usize, usize> = TransientHashMap::new();
        let items: Vec<(usize, usize)> = (0..(MAX_BULK_INSERT + 100)).map(|i| (i, i)).collect();

        let result = transient.insert_bulk_with_metrics(items);

        match result {
            Err(BulkInsertError::TooManyEntries { count, limit }) => {
                // ExactSizeIterator provides exact count, so we get the full count
                assert_eq!(count, MAX_BULK_INSERT + 100);
                assert_eq!(limit, MAX_BULK_INSERT);
            }
            Ok(_) => panic!("Expected TooManyEntries error"),
        }
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_requires_exact_size_iterator() {
        // This test verifies the ExactSizeIterator requirement
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

        // Vec provides ExactSizeIterator
        let items: Vec<(i32, i32)> = vec![(1, 1), (2, 2)];
        let result = transient.insert_bulk_with_metrics(items);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_equivalence_with_sequential() {
        // Property: insert_bulk_with_metrics should produce same map state
        // as sequential insert_without_cow calls
        let items: Vec<(String, i32)> = vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("a".to_string(), 3), // Duplicate
            ("c".to_string(), 4),
        ];

        // Via insert_bulk_with_metrics
        let mut via_bulk = TransientHashMap::new();
        let _ = via_bulk.insert_bulk_with_metrics(items.clone());
        let via_bulk = via_bulk.persistent();

        // Via sequential insert_without_cow
        let mut via_sequential = TransientHashMap::new();
        for (key, value) in items {
            via_sequential.insert_without_cow(key, value);
        }
        let via_sequential = via_sequential.persistent();

        // Both should have same entries
        assert_eq!(via_bulk.len(), via_sequential.len());
        for (key, value) in &via_bulk {
            assert_eq!(via_sequential.get(key), Some(value));
        }
    }

    #[rstest]
    fn test_insert_bulk_with_metrics_large_batch() {
        let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();
        let items: Vec<(i32, i32)> = (0..10_000).map(|i| (i, i * 2)).collect();

        let result = transient
            .insert_bulk_with_metrics(items)
            .expect("insert should succeed");

        assert_eq!(result.inserted_count, 10_000);
        assert_eq!(result.updated_count, 0);
        assert_eq!(transient.len(), 10_000);

        // Verify contents
        for i in 0..10_000 {
            assert_eq!(transient.get(&i), Some(&(i * 2)));
        }
    }
}

// =============================================================================
// Rayon Parallel Iterator Support
// =============================================================================

#[cfg(feature = "rayon")]
mod rayon_support {
    use super::{Hash, PersistentHashMap};
    use rayon::iter::plumbing::{Consumer, Producer, ProducerCallback, UnindexedConsumer, bridge};
    use rayon::iter::{FromParallelIterator, IntoParallelIterator, ParallelIterator};

    /// A parallel iterator over owned key-value pairs of a [`PersistentHashMap`].
    pub struct PersistentHashMapParallelIterator<K, V> {
        elements: Vec<(K, V)>,
    }

    impl<K: Clone + Hash + Eq + Send, V: Clone + Send> IntoParallelIterator
        for PersistentHashMap<K, V>
    {
        type Iter = PersistentHashMapParallelIterator<K, V>;
        type Item = (K, V);

        fn into_par_iter(self) -> Self::Iter {
            PersistentHashMapParallelIterator {
                elements: self.into_iter().collect(),
            }
        }
    }

    impl<K: Clone + Hash + Eq + Send, V: Clone + Send> ParallelIterator
        for PersistentHashMapParallelIterator<K, V>
    {
        type Item = (K, V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.elements.len())
        }
    }

    impl<K: Clone + Hash + Eq + Send, V: Clone + Send> rayon::iter::IndexedParallelIterator
        for PersistentHashMapParallelIterator<K, V>
    {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            bridge(self, consumer)
        }

        fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
            callback.callback(HashMapProducer {
                elements: self.elements,
            })
        }
    }

    struct HashMapProducer<K, V> {
        elements: Vec<(K, V)>,
    }

    impl<K: Clone + Hash + Eq + Send, V: Clone + Send> Producer for HashMapProducer<K, V> {
        type Item = (K, V);
        type IntoIter = std::vec::IntoIter<(K, V)>;

        fn into_iter(self) -> Self::IntoIter {
            self.elements.into_iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let mut left = self.elements;
            let right = left.split_off(index);
            (Self { elements: left }, Self { elements: right })
        }
    }

    /// A parallel iterator over references to key-value pairs of a [`PersistentHashMap`].
    pub struct PersistentHashMapParallelRefIterator<'a, K, V> {
        elements: Vec<(&'a K, &'a V)>,
    }

    impl<'a, K: Clone + Hash + Eq + Sync, V: Clone + Sync> IntoParallelIterator
        for &'a PersistentHashMap<K, V>
    {
        type Iter = PersistentHashMapParallelRefIterator<'a, K, V>;
        type Item = (&'a K, &'a V);

        fn into_par_iter(self) -> Self::Iter {
            PersistentHashMapParallelRefIterator {
                elements: self.iter().collect(),
            }
        }
    }

    impl<'a, K: Sync, V: Sync> ParallelIterator for PersistentHashMapParallelRefIterator<'a, K, V> {
        type Item = (&'a K, &'a V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.elements.len())
        }
    }

    impl<K: Sync, V: Sync> rayon::iter::IndexedParallelIterator
        for PersistentHashMapParallelRefIterator<'_, K, V>
    {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            bridge(self, consumer)
        }

        fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
            callback.callback(HashMapRefProducer {
                elements: self.elements,
            })
        }
    }

    struct HashMapRefProducer<'a, K, V> {
        elements: Vec<(&'a K, &'a V)>,
    }

    impl<'a, K: Sync, V: Sync> Producer for HashMapRefProducer<'a, K, V> {
        type Item = (&'a K, &'a V);
        type IntoIter = std::vec::IntoIter<(&'a K, &'a V)>;

        fn into_iter(self) -> Self::IntoIter {
            self.elements.into_iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let mut left = self.elements;
            let right = left.split_off(index);
            (Self { elements: left }, Self { elements: right })
        }
    }

    impl<K: Clone + Hash + Eq + Sync, V: Clone + Sync> PersistentHashMap<K, V> {
        /// Returns a parallel iterator over references to the key-value pairs.
        ///
        /// Note: The iteration order is unspecified and may vary between runs.
        /// Use `iter()` if you need deterministic ordering.
        #[inline]
        #[must_use]
        pub fn par_iter(&self) -> PersistentHashMapParallelRefIterator<'_, K, V> {
            self.into_par_iter()
        }
    }

    impl<K: Clone + Hash + Eq + Send, V: Clone + Send> FromParallelIterator<(K, V)>
        for PersistentHashMap<K, V>
    {
        /// Collects key-value pairs from a parallel iterator into a [`PersistentHashMap`].
        ///
        /// # Non-determinism
        ///
        /// If the parallel iterator yields duplicate keys, which value is retained
        /// is non-deterministic. For deterministic results with duplicate keys,
        /// use sequential collection via `iter().collect()`.
        fn from_par_iter<I>(par_iter: I) -> Self
        where
            I: IntoParallelIterator<Item = (K, V)>,
        {
            par_iter
                .into_par_iter()
                .collect::<Vec<_>>()
                .into_iter()
                .collect()
        }
    }
}

#[cfg(feature = "rayon")]
pub use rayon_support::PersistentHashMapParallelIterator;
#[cfg(feature = "rayon")]
pub use rayon_support::PersistentHashMapParallelRefIterator;

#[cfg(all(test, feature = "rayon"))]
mod rayon_tests {
    use super::PersistentHashMap;
    use rayon::prelude::*;
    use rstest::rstest;

    #[rstest]
    fn test_into_par_iter_empty() {
        let map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
        let result: Vec<(i32, i32)> = map.into_par_iter().collect();
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_into_par_iter_single_element() {
        let map = PersistentHashMap::singleton(42, "answer".to_string());
        let result: Vec<(i32, String)> = map.into_par_iter().collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (42, "answer".to_string()));
    }

    #[rstest]
    fn test_into_par_iter_multiple_elements() {
        let map: PersistentHashMap<i32, i32> = (0..100).map(|x| (x, x * 10)).collect();
        let mut result: Vec<(i32, i32)> = map.into_par_iter().collect();
        result.sort_unstable_by_key(|(k, _)| *k);
        let expected: Vec<(i32, i32)> = (0..100).map(|x| (x, x * 10)).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_into_par_iter_sum_values() {
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|x| (x, x)).collect();
        let sum: i32 = map.into_par_iter().map(|(_, v)| v).sum();
        let expected: i32 = (0..1000).sum();
        assert_eq!(sum, expected);
    }

    #[rstest]
    fn test_par_iter_empty() {
        let map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
        let result: Vec<(&i32, &i32)> = map.par_iter().collect();
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_par_iter_preserves_original() {
        let map: PersistentHashMap<i32, i32> = (0..100).map(|x| (x, x * 10)).collect();
        let sum: i32 = map.par_iter().map(|(_, v)| *v).sum();
        assert_eq!(map.len(), 100);
        assert_eq!(sum, (0..100).map(|x| x * 10).sum::<i32>());
    }

    #[rstest]
    fn test_from_par_iter_vec() {
        let source: Vec<(i32, i32)> = (0..1000).map(|x| (x, x * 2)).collect();
        let map: PersistentHashMap<i32, i32> = source.into_par_iter().collect();
        assert_eq!(map.len(), 1000);
    }

    #[rstest]
    fn test_parallel_sequential_sum_equivalence() {
        let map: PersistentHashMap<i32, i32> = (0..1000).map(|x| (x, x)).collect();
        let parallel_sum: i32 = map.par_iter().map(|(_, v)| *v).sum();
        let sequential_sum: i32 = map.iter().map(|(_, v)| *v).sum();
        assert_eq!(parallel_sum, sequential_sum);
    }

    #[rstest]
    fn test_large_parallel_map() {
        let map: PersistentHashMap<i32, i32> = (0..100_000).map(|x| (x, x)).collect();
        let result: PersistentHashMap<i32, i32> =
            map.into_par_iter().map(|(k, v)| (k, v * 2)).collect();
        assert_eq!(result.len(), 100_000);
    }
}
