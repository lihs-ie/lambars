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
use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::rc::Rc;

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

// =============================================================================
// Hash computation
// =============================================================================

/// Computes the hash of a key using `DefaultHasher`.
fn compute_hash<K: Hash + ?Sized>(key: &K) -> u64 {
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
// Node Definition
// =============================================================================

/// Internal node structure for the HAMT.
#[derive(Clone)]
enum Node<K, V> {
    /// Empty node (used as sentinel)
    Empty,
    /// Single key-value entry
    Entry { hash: u64, key: K, value: V },
    /// Bitmap-indexed branch node
    Bitmap {
        /// Bitmap indicating which slots are occupied
        bitmap: u32,
        /// Children (entries or subnodes), compressed
        children: ReferenceCounter<[Child<K, V>]>,
    },
    /// Collision node for keys with the same hash
    Collision {
        hash: u64,
        entries: ReferenceCounter<[(K, V)]>,
    },
}

/// A child in a bitmap node.
#[derive(Clone)]
enum Child<K, V> {
    /// A key-value entry with cached hash value
    Entry {
        /// Cached hash value for the key (avoids recomputation)
        hash: u64,
        /// The key
        key: K,
        /// The value
        value: V,
    },
    /// A sub-node
    Node(ReferenceCounter<Node<K, V>>),
}

impl<K, V> Node<K, V> {
    /// Creates an empty node.
    const fn empty() -> Self {
        Self::Empty
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
            } => {
                if *entry_hash == hash && entry_key.borrow() == key {
                    Some(value)
                } else {
                    None
                }
            }
            Node::Bitmap { bitmap, children } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if bitmap & bit == 0 {
                    // Slot is empty
                    None
                } else {
                    // Count bits to find position in children array
                    let position = (bitmap & (bit - 1)).count_ones() as usize;
                    match &children[position] {
                        Child::Entry {
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
                        Child::Node(subnode) => Self::get_from_node(subnode, key, hash, depth + 1),
                    }
                }
            }
            Node::Collision { hash: _, entries } => {
                for (entry_key, value) in entries.iter() {
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
    /// Uses `Iterator::collect()` to construct `ReferenceCounter<[Child]>` in a single expression,
    /// avoiding the pattern of manually creating a `Vec`, mutating it, and converting
    /// it to `ReferenceCounter<[Child]>`.
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to insert at
    /// * `new_child` - The new child to insert
    ///
    /// # Returns
    /// A new `ReferenceCounter<[Child]>` with the child inserted at the specified position.
    fn build_children_with_insert(
        children: &[Child<K, V>],
        position: usize,
        new_child: &Child<K, V>,
    ) -> ReferenceCounter<[Child<K, V>]> {
        (0..=children.len())
            .map(|index| match index.cmp(&position) {
                std::cmp::Ordering::Less => children[index].clone(),
                std::cmp::Ordering::Equal => new_child.clone(),
                std::cmp::Ordering::Greater => children[index - 1].clone(),
            })
            .collect()
    }

    /// Builds a new child array with the element at the specified position updated.
    ///
    /// Uses `Iterator::collect()` to construct `ReferenceCounter<[Child]>` in a single step,
    /// avoiding the pattern of manually creating a `Vec`, mutating it, and then
    /// converting it to a reference-counted pointer.
    ///
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to update
    /// * `new_child` - The new child to place at the position
    ///
    /// # Returns
    /// A new `ReferenceCounter<[Child]>` with the child at the specified position replaced.
    fn build_children_with_update(
        children: &[Child<K, V>],
        position: usize,
        new_child: &Child<K, V>,
    ) -> ReferenceCounter<[Child<K, V>]> {
        children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                if index == position {
                    new_child.clone()
                } else {
                    child.clone()
                }
            })
            .collect()
    }

    /// Builds a new child array with the element at the specified position removed.
    ///
    /// Uses `Iterator::collect()` to directly construct `ReferenceCounter<[Child]>` without
    /// intermediate `Vec` allocation.
    ///
    /// # Arguments
    /// * `children` - The current child array
    /// * `position` - The position to remove
    ///
    /// # Returns
    /// A new `ReferenceCounter<[Child]>` with the child at the specified position removed.
    fn build_children_with_remove(
        children: &[Child<K, V>],
        position: usize,
    ) -> ReferenceCounter<[Child<K, V>]> {
        children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| {
                if index == position {
                    None
                } else {
                    Some(child.clone())
                }
            })
            .collect()
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
            Node::Empty => (Node::Entry { hash, key, value }, true),
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
            } => Self::insert_into_entry_node(
                *existing_hash,
                existing_key,
                existing_value,
                key,
                value,
                hash,
                depth,
            ),
            Node::Bitmap { bitmap, children } => {
                Self::insert_into_bitmap_node(*bitmap, children, key, value, hash, depth)
            }
            Node::Collision {
                hash: collision_hash,
                entries,
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
            // Same key, replace value
            (Node::Entry { hash, key, value }, false)
        } else if existing_hash == hash {
            // Hash collision - create collision node
            let entries = ReferenceCounter::from(vec![
                (existing_key.clone(), existing_value.clone()),
                (key, value),
            ]);
            (Node::Collision { hash, entries }, true)
        } else {
            // Different hash - need to create a bitmap node
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
            // Same index at this level - recurse
            let sub_entry = Node::Entry {
                hash: existing_hash,
                key: existing_key.clone(),
                value: existing_value.clone(),
            };
            let (subnode, added) = Self::insert_into_node(&sub_entry, key, value, hash, depth + 1);
            let bitmap = 1u32 << existing_index;
            // Use std::iter::once() for single element
            let children: ReferenceCounter<[Child<K, V>]> =
                std::iter::once(Child::Node(ReferenceCounter::new(subnode))).collect();
            (Node::Bitmap { bitmap, children }, added)
        } else {
            // Different indices - create bitmap with two children
            // Use array literal + collect() for efficient small array construction
            let bitmap = (1u32 << existing_index) | (1u32 << new_index);
            let children: ReferenceCounter<[Child<K, V>]> = if existing_index < new_index {
                [
                    Child::Entry {
                        hash: existing_hash,
                        key: existing_key.clone(),
                        value: existing_value.clone(),
                    },
                    Child::Entry { hash, key, value },
                ]
                .into_iter()
                .collect()
            } else {
                [
                    Child::Entry { hash, key, value },
                    Child::Entry {
                        hash: existing_hash,
                        key: existing_key.clone(),
                        value: existing_value.clone(),
                    },
                ]
                .into_iter()
                .collect()
            };
            (Node::Bitmap { bitmap, children }, true)
        }
    }

    /// Helper for inserting into a Bitmap node.
    fn insert_into_bitmap_node(
        bitmap: u32,
        children: &ReferenceCounter<[Child<K, V>]>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (bitmap & (bit - 1)).count_ones() as usize;

        if bitmap & bit == 0 {
            // Slot is empty - add new entry using Iterator::collect()
            let new_children = Self::build_children_with_insert(
                children,
                position,
                &Child::Entry { hash, key, value },
            );
            (
                Node::Bitmap {
                    bitmap: bitmap | bit,
                    children: new_children,
                },
                true,
            )
        } else {
            // Slot is occupied
            Self::insert_into_occupied_slot(bitmap, children, position, key, value, hash, depth)
        }
    }

    /// Helper for inserting into an occupied slot in a Bitmap node.
    fn insert_into_occupied_slot(
        bitmap: u32,
        children: &ReferenceCounter<[Child<K, V>]>,
        position: usize,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        let (new_child, added) = match &children[position] {
            Child::Entry {
                hash: child_hash,
                key: child_key,
                value: child_value,
            } => {
                // child_hash is already cached - no recomputation needed
                if *child_key == key {
                    (Child::Entry { hash, key, value }, false)
                } else if *child_hash == hash {
                    let collision = Node::Collision {
                        hash,
                        entries: ReferenceCounter::from(vec![
                            (child_key.clone(), child_value.clone()),
                            (key, value),
                        ]),
                    };
                    (Child::Node(ReferenceCounter::new(collision)), true)
                } else {
                    let child_entry = Node::Entry {
                        hash: *child_hash,
                        key: child_key.clone(),
                        value: child_value.clone(),
                    };
                    let (subnode, added) =
                        Self::insert_into_node(&child_entry, key, value, hash, depth + 1);
                    (Child::Node(ReferenceCounter::new(subnode)), added)
                }
            }
            Child::Node(subnode) => {
                let (new_subnode, added) =
                    Self::insert_into_node(subnode, key, value, hash, depth + 1);
                (Child::Node(ReferenceCounter::new(new_subnode)), added)
            }
        };

        // Use Iterator::collect() to build new children array
        let new_children = Self::build_children_with_update(children, position, &new_child);
        (
            Node::Bitmap {
                bitmap,
                children: new_children,
            },
            added,
        )
    }

    /// Helper for inserting into a Collision node.
    fn insert_into_collision_node(
        node: &Node<K, V>,
        collision_hash: u64,
        entries: &ReferenceCounter<[(K, V)]>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        if hash == collision_hash {
            // Same hash - update or add to collision node
            let mut new_entries = entries.to_vec();
            let mut found = false;

            for entry in &mut new_entries {
                if entry.0 == key {
                    entry.1 = value.clone();
                    found = true;
                    break;
                }
            }

            if !found {
                new_entries.push((key, value));
            }

            (
                Node::Collision {
                    hash: collision_hash,
                    entries: ReferenceCounter::from(new_entries),
                },
                !found,
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
            // Use std::iter::once() for single element
            let children: ReferenceCounter<[Child<K, V>]> =
                std::iter::once(Child::Node(ReferenceCounter::new(subnode))).collect();
            (Node::Bitmap { bitmap, children }, added)
        } else {
            // Use array literal + collect() for efficient small array construction
            let bitmap = (1u32 << collision_index) | (1u32 << new_index);
            let children: ReferenceCounter<[Child<K, V>]> = if collision_index < new_index {
                [
                    Child::Node(ReferenceCounter::new(node.clone())),
                    Child::Entry { hash, key, value },
                ]
                .into_iter()
                .collect()
            } else {
                [
                    Child::Entry { hash, key, value },
                    Child::Node(ReferenceCounter::new(node.clone())),
                ]
                .into_iter()
                .collect()
            };
            (Node::Bitmap { bitmap, children }, true)
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
            Node::Bitmap { bitmap, children } => {
                Self::remove_from_bitmap_node(*bitmap, children, key, hash, depth)
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => Self::remove_from_collision_node(*collision_hash, entries, key, hash),
        }
    }

    /// Helper for removing from a Bitmap node.
    fn remove_from_bitmap_node<Q>(
        bitmap: u32,
        children: &ReferenceCounter<[Child<K, V>]>,
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
            Child::Entry { key: child_key, .. } => {
                if child_key.borrow() == key {
                    Some(Self::remove_entry_from_bitmap(
                        bitmap, children, position, bit,
                    ))
                } else {
                    None
                }
            }
            Child::Node(subnode) => {
                Self::remove_from_subnode(bitmap, children, position, subnode, key, hash, depth)
            }
        }
    }

    /// Helper for removing an entry from a Bitmap node.
    fn remove_entry_from_bitmap(
        bitmap: u32,
        children: &ReferenceCounter<[Child<K, V>]>,
        position: usize,
        bit: u32,
    ) -> (Node<K, V>, bool) {
        let new_bitmap = bitmap & !bit;

        if new_bitmap == 0 {
            return (Node::Empty, true);
        }

        // Use Iterator::collect() to build new children array
        let new_children = Self::build_children_with_remove(children, position);

        Self::simplify_bitmap_if_possible(new_bitmap, new_children)
    }

    /// Helper for removing from a subnode within a Bitmap node.
    fn remove_from_subnode<Q>(
        bitmap: u32,
        children: &ReferenceCounter<[Child<K, V>]>,
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
                // Use Iterator::collect() to build new children array
                let new_children = Self::build_children_with_remove(children, position);
                Some(Self::simplify_bitmap_if_possible(new_bitmap, new_children))
            }
            Node::Entry {
                hash: entry_hash,
                key: entry_key,
                value: entry_value,
            } => {
                let new_child = Child::Entry {
                    hash: *entry_hash,
                    key: entry_key.clone(),
                    value: entry_value.clone(),
                };
                if children.len() == 1 {
                    Some((
                        Node::Entry {
                            hash: *entry_hash,
                            key: entry_key.clone(),
                            value: entry_value.clone(),
                        },
                        true,
                    ))
                } else {
                    // Use Iterator::collect() to build new children array
                    let new_children =
                        Self::build_children_with_update(children, position, &new_child);
                    Some((
                        Node::Bitmap {
                            bitmap,
                            children: new_children,
                        },
                        true,
                    ))
                }
            }
            _ => {
                let new_child = Child::Node(ReferenceCounter::new(new_subnode));
                // Use Iterator::collect() to build new children array
                let new_children = Self::build_children_with_update(children, position, &new_child);
                Some((
                    Node::Bitmap {
                        bitmap,
                        children: new_children,
                    },
                    true,
                ))
            }
        }
    }

    /// Simplifies a Bitmap node to an Entry if it has only one child entry.
    fn simplify_bitmap_if_possible(
        bitmap: u32,
        children: ReferenceCounter<[Child<K, V>]>,
    ) -> (Node<K, V>, bool) {
        if children.len() == 1
            && let Child::Entry { hash, key, value } = &children[0]
        {
            // hash is already cached - no recomputation needed
            (
                Node::Entry {
                    hash: *hash,
                    key: key.clone(),
                    value: value.clone(),
                },
                true,
            )
        } else {
            (Node::Bitmap { bitmap, children }, true)
        }
    }

    /// Helper for removing from a Collision node.
    fn remove_from_collision_node<Q>(
        collision_hash: u64,
        entries: &ReferenceCounter<[(K, V)]>,
        key: &Q,
        hash: u64,
    ) -> Option<(Node<K, V>, bool)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if hash != collision_hash {
            return None;
        }

        let mut new_entries = entries.to_vec();
        let found_index = new_entries
            .iter()
            .position(|(entry_key, _)| entry_key.borrow() == key)?;

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
                },
                true,
            ))
        } else {
            Some((
                Node::Collision {
                    hash: collision_hash,
                    entries: ReferenceCounter::from(new_entries),
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
            Node::Bitmap { bitmap, children } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if bitmap & bit == 0 {
                    None
                } else {
                    let position = (bitmap & (bit - 1)).count_ones() as usize;
                    match &children[position] {
                        Child::Entry { key: child_key, .. } => {
                            if child_key.borrow() == key {
                                Some(child_key.clone())
                            } else {
                                None
                            }
                        }
                        Child::Node(subnode) => Self::find_key(subnode, key, hash, depth + 1),
                    }
                }
            }
            Node::Collision { entries, .. } => {
                for (entry_key, _) in entries.iter() {
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
        children: &'a [Child<K, V>],
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
                    children: children.as_ref(),
                    index: 0,
                });
                self.advance();
            }
            Node::Collision { entries, .. } => {
                self.stack.push(StackFrame::CollisionNode {
                    entries: entries.as_ref(),
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
                        Child::Entry { key, value, .. } => {
                            self.pending_entry = Some((key, value));
                            return;
                        }
                        Child::Node(subnode) => match subnode.as_ref() {
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
                                    children: child_children.as_ref(),
                                    index: 0,
                                });
                            }
                            Node::Collision {
                                entries: child_entries,
                                ..
                            } => {
                                self.stack.push(StackFrame::CollisionNode {
                                    entries: child_entries.as_ref(),
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
        let mut transient = TransientHashMap::new();
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
            _marker: PhantomData,
        }
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

    /// Recursively inserts into a node using COW semantics.
    /// Returns (`old_value`, `was_added`).
    fn insert_into_node_cow(
        node: &mut Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Option<V>, bool) {
        match node {
            Node::Empty => {
                *node = Node::Entry { hash, key, value };
                (None, true)
            }
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
            } => {
                if *existing_hash == hash && *existing_key == key {
                    // Same key, replace value
                    let old_value = std::mem::replace(existing_value, value);
                    (Some(old_value), false)
                } else if *existing_hash == hash {
                    // Hash collision - create collision node
                    let entries = ReferenceCounter::from(vec![
                        (existing_key.clone(), existing_value.clone()),
                        (key, value),
                    ]);
                    *node = Node::Collision {
                        hash: *existing_hash,
                        entries,
                    };
                    (None, true)
                } else {
                    // Different hash - need to create a bitmap node
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
            Node::Bitmap { bitmap, children } => {
                Self::insert_into_bitmap_node_cow(bitmap, children, key, value, hash, depth)
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if hash == *collision_hash {
                    // Same hash - update or add to collision node
                    let entries_mut = ReferenceCounter::make_mut(entries);
                    for entry in entries_mut.iter_mut() {
                        if entry.0 == key {
                            let old_value = std::mem::replace(&mut entry.1, value);
                            return (Some(old_value), false);
                        }
                    }
                    // Key not found, add new entry
                    let mut new_entries = entries_mut.to_vec();
                    new_entries.push((key, value));
                    *entries = ReferenceCounter::from(new_entries);
                    (None, true)
                } else {
                    // Different hash - convert collision to bitmap and insert
                    let collision_entries = entries.to_vec();
                    let collision_hash_value = *collision_hash;

                    // Create bitmap node from collision entries
                    let index = hash_index(collision_hash_value, depth);
                    let bit = 1u32 << index;
                    let collision_child = Child::Node(ReferenceCounter::new(Node::Collision {
                        hash: collision_hash_value,
                        entries: ReferenceCounter::from(collision_entries),
                    }));

                    let new_index = hash_index(hash, depth);
                    let new_bit = 1u32 << new_index;

                    if index == new_index {
                        // Same index, create subnode
                        let mut subnode = Node::Collision {
                            hash: collision_hash_value,
                            entries: entries.clone(),
                        };
                        let result =
                            Self::insert_into_node_cow(&mut subnode, key, value, hash, depth + 1);
                        *node = Node::Bitmap {
                            bitmap: bit,
                            children: ReferenceCounter::from(vec![Child::Node(
                                ReferenceCounter::new(subnode),
                            )]),
                        };
                        result
                    } else {
                        // Different index
                        let new_child = Child::Entry { hash, key, value };
                        let (first, second) = if index < new_index {
                            (collision_child, new_child)
                        } else {
                            (new_child, collision_child)
                        };
                        *node = Node::Bitmap {
                            bitmap: bit | new_bit,
                            children: ReferenceCounter::from(vec![first, second]),
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
        children: &mut ReferenceCounter<[Child<K, V>]>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Option<V>, bool) {
        let index = hash_index(hash, depth);
        let bit = 1u32 << index;
        let position = (*bitmap & (bit - 1)).count_ones() as usize;
        let children_mut = ReferenceCounter::make_mut(children);

        if *bitmap & bit == 0 {
            // Slot is empty - add new entry
            let mut new_children = children_mut.to_vec();
            new_children.insert(position, Child::Entry { hash, key, value });
            *children = ReferenceCounter::from(new_children);
            *bitmap |= bit;
            (None, true)
        } else {
            // Slot is occupied
            match &mut children_mut[position] {
                Child::Entry {
                    hash: child_hash,
                    key: child_key,
                    value: child_value,
                } => {
                    if *child_key == key {
                        // Same key, replace value
                        let old_value = std::mem::replace(child_value, value);
                        (Some(old_value), false)
                    } else if *child_hash == hash {
                        // Hash collision - create collision node
                        let collision = Node::Collision {
                            hash,
                            entries: ReferenceCounter::from(vec![
                                (child_key.clone(), child_value.clone()),
                                (key, value),
                            ]),
                        };
                        children_mut[position] = Child::Node(ReferenceCounter::new(collision));
                        (None, true)
                    } else {
                        // Different hash - create subnode
                        let child_entry = Node::Entry {
                            hash: *child_hash,
                            key: child_key.clone(),
                            value: child_value.clone(),
                        };
                        let (subnode, _) = PersistentHashMap::insert_into_node(
                            &child_entry,
                            key,
                            value,
                            hash,
                            depth + 1,
                        );
                        children_mut[position] = Child::Node(ReferenceCounter::new(subnode));
                        (None, true)
                    }
                }
                Child::Node(subnode) => {
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
            } => {
                if *entry_hash == hash && (*entry_key).borrow() == key {
                    let old_value = value.clone();
                    *node = Node::Empty;
                    Some(old_value)
                } else {
                    None
                }
            }
            Node::Bitmap { bitmap, children } => {
                Self::remove_from_bitmap_node_cow(bitmap, children, key, hash, depth)
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if *collision_hash != hash {
                    return None;
                }

                let entries_mut = ReferenceCounter::make_mut(entries);
                let found_index = entries_mut
                    .iter()
                    .position(|(entry_key, _)| entry_key.borrow() == key)?;

                let removed_value = entries_mut[found_index].1.clone();

                if entries_mut.len() == 2 {
                    // Convert back to single entry
                    let other_index = 1 - found_index;
                    let (remaining_key, remaining_value) = entries_mut[other_index].clone();
                    *node = Node::Entry {
                        hash: *collision_hash,
                        key: remaining_key,
                        value: remaining_value,
                    };
                } else {
                    let mut new_entries = entries_mut.to_vec();
                    new_entries.remove(found_index);
                    *entries = ReferenceCounter::from(new_entries);
                }

                Some(removed_value)
            }
        }
    }

    /// Removes from a bitmap node using COW semantics.
    fn remove_from_bitmap_node_cow<Q>(
        bitmap: &mut u32,
        children: &mut ReferenceCounter<[Child<K, V>]>,
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
        let children_mut = ReferenceCounter::make_mut(children);

        match &mut children_mut[position] {
            Child::Entry {
                key: child_key,
                value: child_value,
                ..
            } => {
                if (*child_key).borrow() == key {
                    let removed_value = child_value.clone();

                    // Remove this entry from the bitmap
                    let new_bitmap = *bitmap & !bit;
                    if new_bitmap == 0 {
                        // This would make the bitmap empty - not expected at root
                        // but handle it gracefully
                    }

                    let mut new_children = children_mut.to_vec();
                    new_children.remove(position);
                    *children = ReferenceCounter::from(new_children);
                    *bitmap = new_bitmap;

                    Some(removed_value)
                } else {
                    None
                }
            }
            Child::Node(subnode) => {
                let subnode_mut = ReferenceCounter::make_mut(subnode);
                let result = Self::remove_from_node_cow(subnode_mut, key, hash, depth + 1);

                if result.is_some() {
                    // Check if we need to simplify the structure
                    match subnode_mut {
                        Node::Empty => {
                            // Remove this child
                            let new_bitmap = *bitmap & !bit;
                            let mut new_children = children_mut.to_vec();
                            new_children.remove(position);
                            *children = ReferenceCounter::from(new_children);
                            *bitmap = new_bitmap;
                        }
                        Node::Entry {
                            hash: entry_hash,
                            key: entry_key,
                            value: entry_value,
                        } => {
                            // Promote entry up
                            children_mut[position] = Child::Entry {
                                hash: *entry_hash,
                                key: entry_key.clone(),
                                value: entry_value.clone(),
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
            } => {
                if *entry_hash == hash && (*entry_key).borrow() == key {
                    Some(value)
                } else {
                    None
                }
            }
            Node::Bitmap { bitmap, children } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if *bitmap & bit == 0 {
                    return None;
                }

                let position = (*bitmap & (bit - 1)).count_ones() as usize;
                let children_mut = ReferenceCounter::make_mut(children);

                match &mut children_mut[position] {
                    Child::Entry {
                        hash: child_hash,
                        key: child_key,
                        value,
                    } => {
                        if *child_hash == hash && (*child_key).borrow() == key {
                            Some(value)
                        } else {
                            None
                        }
                    }
                    Child::Node(subnode) => {
                        let subnode_mut = ReferenceCounter::make_mut(subnode);
                        Self::get_mut_from_node(subnode_mut, key, hash, depth + 1)
                    }
                }
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if *collision_hash != hash {
                    return None;
                }

                let entries_mut = ReferenceCounter::make_mut(entries);
                for entry in entries_mut.iter_mut() {
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
        for (key, value) in iter {
            self.insert(key, value);
        }
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
        let mut transient = Self::new();
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
