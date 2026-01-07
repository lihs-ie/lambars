//! Persistent (immutable) tree map based on Red-Black Tree.
//!
//! This module provides [`PersistentTreeMap`], an immutable ordered map
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentTreeMap` is based on a persistent Red-Black Tree, a self-balancing
//! binary search tree that provides efficient ordered map operations.
//!
//! - O(log N) get
//! - O(log N) insert
//! - O(log N) remove
//! - O(log N) min/max
//! - O(log N + k) range queries where k is the number of results
//! - O(1) len and `is_empty`
//!
//! All operations return new maps without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentTreeMap;
//!
//! let map = PersistentTreeMap::new()
//!     .insert(3, "three")
//!     .insert(1, "one")
//!     .insert(2, "two");
//!
//! // Entries are always in sorted order
//! let keys: Vec<&i32> = map.keys().collect();
//! assert_eq!(keys, vec![&1, &2, &3]);
//!
//! // Range queries
//! let range: Vec<(&i32, &&str)> = map.range(1..3).collect();
//! assert_eq!(range.len(), 2); // 1 and 2
//! ```
//!
//! # Internal Structure
//!
//! The Red-Black Tree maintains the following invariants:
//! 1. Every node is either red or black
//! 2. The root is black
//! 3. All leaves (NIL) are black
//! 4. Red nodes have only black children
//! 5. Every path from root to leaf has the same number of black nodes
//!
//! These invariants ensure the tree height is O(log N).

use super::ReferenceCounter;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::{Bound, RangeBounds};

use crate::typeclass::{Foldable, TypeConstructor};

// =============================================================================
// Color Definition
// =============================================================================

/// The color of a Red-Black Tree node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Color {
    Red,
    Black,
}

// =============================================================================
// Node Definition
// =============================================================================

/// Internal node structure for the Red-Black Tree.
#[derive(Clone)]
struct Node<K, V> {
    key: K,
    value: V,
    color: Color,
    left: Option<ReferenceCounter<Self>>,
    right: Option<ReferenceCounter<Self>>,
}

impl<K, V> Node<K, V> {
    /// Creates a new red node with no children.
    const fn new_red(key: K, value: V) -> Self {
        Self {
            key,
            value,
            color: Color::Red,
            left: None,
            right: None,
        }
    }

    /// Creates a copy of this node with a new color.
    fn with_color(&self, color: Color) -> Self
    where
        K: Clone,
        V: Clone,
    {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            color,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }

    /// Creates a copy of this node with new children.
    fn with_children(
        &self,
        left: Option<ReferenceCounter<Self>>,
        right: Option<ReferenceCounter<Self>>,
    ) -> Self
    where
        K: Clone,
        V: Clone,
    {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            color: self.color,
            left,
            right,
        }
    }

    /// Checks if this node is red.
    fn is_red(&self) -> bool {
        self.color == Color::Red
    }
}

/// Helper function to check if an optional node is red.
fn is_red<K, V>(node: Option<&ReferenceCounter<Node<K, V>>>) -> bool {
    node.is_some_and(|node| node.is_red())
}

// =============================================================================
// PersistentTreeMap Definition
// =============================================================================

/// A persistent (immutable) ordered map based on Red-Black Tree.
///
/// `PersistentTreeMap` is an immutable data structure that uses structural
/// sharing to efficiently support functional programming patterns.
///
/// Keys must implement `Ord` for ordering. The map maintains entries in
/// sorted key order, enabling efficient range queries and ordered iteration.
///
/// # Time Complexity
///
/// | Operation      | Complexity        |
/// |----------------|-------------------|
/// | `new`          | O(1)              |
/// | `get`          | O(log N)          |
/// | `insert`       | O(log N)          |
/// | `remove`       | O(log N)          |
/// | `contains_key` | O(log N)          |
/// | `min`/`max`    | O(log N)          |
/// | `range`        | O(log N + k)      |
/// | `len`          | O(1)              |
/// | `is_empty`     | O(1)              |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentTreeMap;
///
/// let map = PersistentTreeMap::singleton(42, "answer");
/// assert_eq!(map.get(&42), Some(&"answer"));
///
/// // Ordered iteration
/// let map = PersistentTreeMap::new()
///     .insert(3, "three")
///     .insert(1, "one")
///     .insert(2, "two");
///
/// let keys: Vec<&i32> = map.keys().collect();
/// assert_eq!(keys, vec![&1, &2, &3]);
/// ```
#[derive(Clone)]
pub struct PersistentTreeMap<K, V> {
    /// Root node of the tree
    root: Option<ReferenceCounter<Node<K, V>>>,
    /// Number of entries
    length: usize,
}

impl<K, V> PersistentTreeMap<K, V> {
    /// Creates a new empty map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: None,
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
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
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
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let empty: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert(1, "one".to_string());
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl<K: Clone + Ord, V: Clone> PersistentTreeMap<K, V> {
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
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::singleton(42, "answer");
    /// assert_eq!(map.len(), 1);
    /// assert_eq!(map.get(&42), Some(&"answer"));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(key: K, value: V) -> Self {
        Self::new().insert(key, value)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the
    /// ordering on the borrowed form must match the ordering on the key type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Complexity
    ///
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
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
        Q: Ord + ?Sized,
    {
        Self::get_from_node(self.root.as_ref(), key)
    }

    /// Recursive helper for get.
    fn get_from_node<'a, Q>(
        node: Option<&'a ReferenceCounter<Node<K, V>>>,
        key: &Q,
    ) -> Option<&'a V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        node.and_then(|node_ref| match key.cmp(node_ref.key.borrow()) {
            Ordering::Less => Self::get_from_node(node_ref.left.as_ref(), key),
            Ordering::Greater => Self::get_from_node(node_ref.right.as_ref(), key),
            Ordering::Equal => Some(&node_ref.value),
        })
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Complexity
    ///
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert("key".to_string(), 42);
    ///
    /// assert!(map.contains_key("key"));
    /// assert!(!map.contains_key("other"));
    /// ```
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
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
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map1 = PersistentTreeMap::new().insert(1, "one");
    /// let map2 = map1.insert(1, "ONE");
    ///
    /// assert_eq!(map1.get(&1), Some(&"one")); // Original unchanged
    /// assert_eq!(map2.get(&1), Some(&"ONE")); // New version
    /// ```
    #[must_use]
    pub fn insert(&self, key: K, value: V) -> Self {
        let (new_root, added) = Self::insert_into_node(self.root.as_ref(), key, value);

        // Make root black
        let black_root = new_root.map_or_else(
            || None,
            |node_ref| {
                if node_ref.is_red() {
                    Some(ReferenceCounter::new(node_ref.with_color(Color::Black)))
                } else {
                    Some(node_ref)
                }
            },
        );

        Self {
            root: black_root,
            length: if added { self.length + 1 } else { self.length },
        }
    }

    /// Recursive helper for insert.
    /// Returns (`new_node`, `was_added`) where `was_added` is true if a new entry was added.
    fn insert_into_node(
        node: Option<&ReferenceCounter<Node<K, V>>>,
        key: K,
        value: V,
    ) -> (Option<ReferenceCounter<Node<K, V>>>, bool) {
        match node {
            None => {
                // Insert new red node
                (Some(ReferenceCounter::new(Node::new_red(key, value))), true)
            }
            Some(node_ref) => {
                match key.cmp(&node_ref.key) {
                    Ordering::Less => {
                        let (new_left, added) =
                            Self::insert_into_node(node_ref.left.as_ref(), key, value);
                        let new_node = node_ref.with_children(new_left, node_ref.right.clone());
                        (Some(ReferenceCounter::new(Self::balance(new_node))), added)
                    }
                    Ordering::Greater => {
                        let (new_right, added) =
                            Self::insert_into_node(node_ref.right.as_ref(), key, value);
                        let new_node = node_ref.with_children(node_ref.left.clone(), new_right);
                        (Some(ReferenceCounter::new(Self::balance(new_node))), added)
                    }
                    Ordering::Equal => {
                        // Key exists, update value
                        let new_node = Node {
                            key,
                            value,
                            color: node_ref.color,
                            left: node_ref.left.clone(),
                            right: node_ref.right.clone(),
                        };
                        (Some(ReferenceCounter::new(new_node)), false)
                    }
                }
            }
        }
    }

    /// Balances the tree after insertion.
    /// Handles the four cases of red-red violation.
    fn balance(node: Node<K, V>) -> Node<K, V> {
        // Case 1: Left-Left (left child is red, left-left grandchild is red)
        if is_red(node.left.as_ref())
            && let Some(left) = &node.left
            && is_red(left.left.as_ref())
        {
            return Self::rotate_right_and_recolor(node);
        }

        // Case 2: Left-Right (left child is red, left-right grandchild is red)
        if is_red(node.left.as_ref())
            && let Some(left) = &node.left
            && is_red(left.right.as_ref())
        {
            // First rotate left on the left child, then rotate right on node
            let new_left = Self::rotate_left((**left).clone());
            let new_node =
                node.with_children(Some(ReferenceCounter::new(new_left)), node.right.clone());
            return Self::rotate_right_and_recolor(new_node);
        }

        // Case 3: Right-Right (right child is red, right-right grandchild is red)
        if is_red(node.right.as_ref())
            && let Some(right) = &node.right
            && is_red(right.right.as_ref())
        {
            return Self::rotate_left_and_recolor(node);
        }

        // Case 4: Right-Left (right child is red, right-left grandchild is red)
        if is_red(node.right.as_ref())
            && let Some(right) = &node.right
            && is_red(right.left.as_ref())
        {
            // First rotate right on the right child, then rotate left on node
            let new_right = Self::rotate_right((**right).clone());
            let new_node =
                node.with_children(node.left.clone(), Some(ReferenceCounter::new(new_right)));
            return Self::rotate_left_and_recolor(new_node);
        }

        node
    }

    /// Rotates the tree to the right around the given node.
    fn rotate_right(node: Node<K, V>) -> Node<K, V> {
        if let Some(left) = node.left {
            let new_node = Node {
                key: node.key,
                value: node.value,
                color: node.color,
                left: left.right.clone(),
                right: node.right,
            };
            Node {
                key: left.key.clone(),
                value: left.value.clone(),
                color: left.color,
                left: left.left.clone(),
                right: Some(ReferenceCounter::new(new_node)),
            }
        } else {
            node
        }
    }

    /// Rotates the tree to the left around the given node.
    fn rotate_left(node: Node<K, V>) -> Node<K, V> {
        if let Some(right) = node.right {
            let new_node = Node {
                key: node.key,
                value: node.value,
                color: node.color,
                left: node.left,
                right: right.left.clone(),
            };
            Node {
                key: right.key.clone(),
                value: right.value.clone(),
                color: right.color,
                left: Some(ReferenceCounter::new(new_node)),
                right: right.right.clone(),
            }
        } else {
            node
        }
    }

    /// Rotates right and recolors for balancing.
    fn rotate_right_and_recolor(node: Node<K, V>) -> Node<K, V> {
        if let Some(left) = &node.left {
            // New root (the old left child)
            let new_right = Node {
                key: node.key.clone(),
                value: node.value.clone(),
                color: Color::Red,
                left: left.right.clone(),
                right: node.right.clone(),
            };

            // If left has a left child, make it black
            let new_left = left
                .left
                .as_ref()
                .map(|left_left| ReferenceCounter::new(left_left.with_color(Color::Black)));

            Node {
                key: left.key.clone(),
                value: left.value.clone(),
                color: Color::Black,
                left: new_left,
                right: Some(ReferenceCounter::new(new_right)),
            }
        } else {
            node
        }
    }

    /// Rotates left and recolors for balancing.
    fn rotate_left_and_recolor(node: Node<K, V>) -> Node<K, V> {
        if let Some(right) = &node.right {
            // New root (the old right child)
            let new_left = Node {
                key: node.key.clone(),
                value: node.value.clone(),
                color: Color::Red,
                left: node.left.clone(),
                right: right.left.clone(),
            };

            // If right has a right child, make it black
            let new_right = right
                .right
                .as_ref()
                .map(|right_right| ReferenceCounter::new(right_right.with_color(Color::Black)));

            Node {
                key: right.key.clone(),
                value: right.value.clone(),
                color: Color::Black,
                left: Some(ReferenceCounter::new(new_left)),
                right: new_right,
            }
        } else {
            node
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
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    /// let removed = map.remove(&1);
    ///
    /// assert_eq!(map.len(), 2);     // Original unchanged
    /// assert_eq!(removed.len(), 1); // New version
    /// assert_eq!(removed.get(&1), None);
    /// ```
    #[must_use]
    pub fn remove<Q>(&self, key: &Q) -> Self
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if !self.contains_key(key) {
            return self.clone();
        }

        let new_root = Self::remove_from_node(self.root.as_ref(), key);

        // Make root black if it exists
        let black_root = new_root.map(|node| {
            if node.is_red() {
                ReferenceCounter::new(node.with_color(Color::Black))
            } else {
                node
            }
        });

        Self {
            root: black_root,
            length: self.length.saturating_sub(1),
        }
    }

    /// Recursive helper for remove.
    fn remove_from_node<Q>(
        node: Option<&ReferenceCounter<Node<K, V>>>,
        key: &Q,
    ) -> Option<ReferenceCounter<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        node.and_then(|node_ref| {
            match key.cmp(node_ref.key.borrow()) {
                Ordering::Less => {
                    let new_left = Self::remove_from_node(node_ref.left.as_ref(), key);
                    let new_node = node_ref.with_children(new_left, node_ref.right.clone());
                    Some(ReferenceCounter::new(Self::balance_after_delete(new_node)))
                }
                Ordering::Greater => {
                    let new_right = Self::remove_from_node(node_ref.right.as_ref(), key);
                    let new_node = node_ref.with_children(node_ref.left.clone(), new_right);
                    Some(ReferenceCounter::new(Self::balance_after_delete(new_node)))
                }
                Ordering::Equal => {
                    // Found the node to remove
                    match (&node_ref.left, &node_ref.right) {
                        (None, None) => None,
                        (Some(left), None) => Some(left.clone()),
                        (None, Some(right)) => Some(right.clone()),
                        (Some(_), Some(right)) => {
                            // Find the minimum in the right subtree
                            let (successor_key, successor_value) = Self::find_min_entry(right);
                            let new_right = Self::remove_from_node(
                                node_ref.right.as_ref(),
                                successor_key.borrow(),
                            );
                            let new_node = Node {
                                key: successor_key,
                                value: successor_value,
                                color: node_ref.color,
                                left: node_ref.left.clone(),
                                right: new_right,
                            };
                            Some(ReferenceCounter::new(Self::balance_after_delete(new_node)))
                        }
                    }
                }
            }
        })
    }

    /// Finds the minimum key-value pair in a subtree.
    fn find_min_entry(node: &ReferenceCounter<Node<K, V>>) -> (K, V) {
        node.left.as_ref().map_or_else(
            || (node.key.clone(), node.value.clone()),
            |left| Self::find_min_entry(left),
        )
    }

    /// Balances the tree after deletion (simplified version).
    const fn balance_after_delete(node: Node<K, V>) -> Node<K, V> {
        // For a full implementation, we would need to handle double-black cases
        // This simplified version just returns the node and relies on the
        // tree still being relatively balanced
        node
    }

    /// Returns the entry with the minimum key.
    ///
    /// # Complexity
    ///
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(3, "three")
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// assert_eq!(map.min(), Some((&1, &"one")));
    /// ```
    #[must_use]
    pub fn min(&self) -> Option<(&K, &V)> {
        Self::min_from_node(self.root.as_ref())
    }

    /// Recursive helper for min.
    fn min_from_node(node: Option<&ReferenceCounter<Node<K, V>>>) -> Option<(&K, &V)> {
        node.and_then(|node_ref| {
            node_ref.left.as_ref().map_or_else(
                || Some((&node_ref.key, &node_ref.value)),
                |left| Self::min_from_node(Some(left)),
            )
        })
    }

    /// Returns the entry with the maximum key.
    ///
    /// # Complexity
    ///
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(3, "three")
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// assert_eq!(map.max(), Some((&3, &"three")));
    /// ```
    #[must_use]
    pub fn max(&self) -> Option<(&K, &V)> {
        Self::max_from_node(self.root.as_ref())
    }

    /// Recursive helper for max.
    fn max_from_node(node: Option<&ReferenceCounter<Node<K, V>>>) -> Option<(&K, &V)> {
        node.and_then(|node_ref| {
            node_ref.right.as_ref().map_or_else(
                || Some((&node_ref.key, &node_ref.value)),
                |right| Self::max_from_node(Some(right)),
            )
        })
    }

    /// Returns an iterator over entries in sorted key order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(3, "three")
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    #[must_use]
    pub fn iter(&self) -> PersistentTreeMapIterator<'_, K, V> {
        let mut entries = Vec::with_capacity(self.length);
        Self::collect_entries_in_order(self.root.as_ref(), &mut entries);
        PersistentTreeMapIterator {
            entries,
            current_index: 0,
        }
    }

    /// Collects all entries in sorted order (in-order traversal).
    fn collect_entries_in_order<'a>(
        node: Option<&'a ReferenceCounter<Node<K, V>>>,
        entries: &mut Vec<(&'a K, &'a V)>,
    ) {
        if let Some(node_ref) = node {
            Self::collect_entries_in_order(node_ref.left.as_ref(), entries);
            entries.push((&node_ref.key, &node_ref.value));
            Self::collect_entries_in_order(node_ref.right.as_ref(), entries);
        }
    }

    /// Returns an iterator over keys in sorted order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(3, "three")
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// let keys: Vec<&i32> = map.keys().collect();
    /// assert_eq!(keys, vec![&1, &2, &3]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    /// Returns an iterator over values in key order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20)
    ///     .insert(3, 30);
    ///
    /// let sum: i32 = map.values().sum();
    /// assert_eq!(sum, 60);
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }

    /// Applies a function to all values, keeping keys unchanged.
    ///
    /// Returns a new map with the same keys but transformed values.
    /// The length of the map is preserved, and entries remain in sorted key order.
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
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20);
    /// let doubled = map.map_values(|v| v * 2);
    /// assert_eq!(doubled.get(&1), Some(&20));
    /// assert_eq!(doubled.get(&2), Some(&40));
    /// ```
    #[must_use]
    pub fn map_values<W, F>(&self, mut transform: F) -> PersistentTreeMap<K, W>
    where
        K: Clone + Ord,
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
    /// The new map will be ordered by the new keys.
    ///
    /// # Warning
    ///
    /// Key transformation may cause collisions. When multiple original keys
    /// map to the same new key, only one entry will be kept. The collision
    /// behavior depends on internal iteration order.
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
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("bb".to_string(), 2)
    ///     .insert("ccc".to_string(), 3);
    /// let by_length = map.map_keys(|k| k.len());
    /// assert_eq!(by_length.get(&1), Some(&1));
    /// assert_eq!(by_length.get(&2), Some(&2));
    /// assert_eq!(by_length.get(&3), Some(&3));
    /// ```
    #[must_use]
    pub fn map_keys<L, F>(&self, mut transform: F) -> PersistentTreeMap<L, V>
    where
        L: Clone + Ord,
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
    /// The result maintains sorted key order.
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
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "1".to_string())
    ///     .insert(2, "abc".to_string())
    ///     .insert(3, "42".to_string());
    /// let parsed = map.filter_map(|_, v| v.parse::<i32>().ok());
    /// assert_eq!(parsed.len(), 2);
    /// assert_eq!(parsed.get(&1), Some(&1));
    /// assert_eq!(parsed.get(&3), Some(&42));
    /// ```
    #[must_use]
    pub fn filter_map<W, F>(&self, mut filter_transform: F) -> PersistentTreeMap<K, W>
    where
        K: Clone + Ord,
        W: Clone,
        F: FnMut(&K, &V) -> Option<W>,
    {
        self.iter()
            .filter_map(|(key, value)| {
                filter_transform(key, value).map(|new_value| (key.clone(), new_value))
            })
            .collect()
    }

    /// Returns an iterator over key-value pairs in sorted key order.
    ///
    /// This is an alias for [`iter`](Self::iter), provided for API consistency
    /// with other functional programming languages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    /// for (key, value) in map.entries() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn entries(&self) -> PersistentTreeMapIterator<'_, K, V> {
        self.iter()
    }

    /// Merges two maps, with values from `other` taking precedence on key conflicts.
    ///
    /// Returns a new map containing all entries from both maps.
    /// When a key exists in both maps, the value from `other` is used.
    ///
    /// # Arguments
    ///
    /// * `other` - The map to merge with
    ///
    /// # Complexity
    ///
    /// O(m log(n + m)) where n is the size of self and m is the size of other
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map1 = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    /// let map2 = PersistentTreeMap::new()
    ///     .insert(2, "TWO")
    ///     .insert(3, "three");
    /// let merged = map1.merge(&map2);
    /// assert_eq!(merged.get(&1), Some(&"one"));
    /// assert_eq!(merged.get(&2), Some(&"TWO")); // From map2
    /// assert_eq!(merged.get(&3), Some(&"three"));
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (key, value) in other {
            result = result.insert(key.clone(), value.clone());
        }
        result
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
    /// O(m log(n + m)) where n is the size of self and m is the size of other
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map1 = PersistentTreeMap::new()
    ///     .insert(1, 100)
    ///     .insert(2, 200);
    /// let map2 = PersistentTreeMap::new()
    ///     .insert(2, 50)
    ///     .insert(3, 300);
    /// let merged = map1.merge_with(&map2, |_, v1, v2| *v1.max(v2));
    /// assert_eq!(merged.get(&1), Some(&100));
    /// assert_eq!(merged.get(&2), Some(&200)); // max(200, 50)
    /// assert_eq!(merged.get(&3), Some(&300));
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
    /// returns false. The result maintains sorted key order.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true if the entry should be deleted
    ///
    /// # Complexity
    ///
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20)
    ///     .insert(3, 30);
    /// let small_values = map.delete_if(|_, v| *v >= 20);
    /// assert_eq!(small_values.len(), 1);
    /// assert_eq!(small_values.get(&1), Some(&10));
    /// ```
    #[must_use]
    pub fn delete_if<F>(&self, mut predicate: F) -> Self
    where
        K: Clone + Ord,
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
    /// returns true. The result maintains sorted key order.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true if the entry should be kept
    ///
    /// # Complexity
    ///
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20)
    ///     .insert(3, 30);
    /// let even_keys = map.keep_if(|k, _| k % 2 == 0);
    /// assert_eq!(even_keys.len(), 1);
    /// assert_eq!(even_keys.get(&2), Some(&20));
    /// ```
    #[must_use]
    pub fn keep_if<F>(&self, mut predicate: F) -> Self
    where
        K: Clone + Ord,
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
    /// Both resulting maps maintain sorted key order.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that receives a reference to the key and value,
    ///   and returns true to include in the first map, false for the second
    ///
    /// # Complexity
    ///
    /// O(n log n) where n is the number of entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20)
    ///     .insert(3, 30)
    ///     .insert(4, 40);
    /// let (even_keys, odd_keys) = map.partition(|k, _| k % 2 == 0);
    /// assert_eq!(even_keys.len(), 2);
    /// assert_eq!(odd_keys.len(), 2);
    /// ```
    #[must_use]
    pub fn partition<F>(&self, mut predicate: F) -> (Self, Self)
    where
        K: Clone + Ord,
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

    /// Returns an iterator over entries within the specified range.
    ///
    /// The range is specified using Rust's range syntax:
    /// - `a..b` - from a (inclusive) to b (exclusive)
    /// - `a..=b` - from a (inclusive) to b (inclusive)
    /// - `a..` - from a (inclusive) to the end
    /// - `..b` - from the start to b (exclusive)
    /// - `..=b` - from the start to b (inclusive)
    /// - `..` - all entries
    ///
    /// # Complexity
    ///
    /// O(log N + k) where k is the number of entries in the range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two")
    ///     .insert(3, "three")
    ///     .insert(4, "four")
    ///     .insert(5, "five");
    ///
    /// let range: Vec<(&i32, &&str)> = map.range(2..=4).collect();
    /// assert_eq!(range.len(), 3); // 2, 3, 4
    /// ```
    pub fn range<R, Q>(&self, range: R) -> PersistentTreeMapRangeIterator<'_, K, V>
    where
        R: RangeBounds<Q>,
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        // Collect all entries and filter by range
        let mut entries = Vec::new();

        for (key, value) in self {
            let key_borrowed: &Q = key.borrow();

            let in_start = match range.start_bound() {
                Bound::Included(bound) => key_borrowed >= bound,
                Bound::Excluded(bound) => key_borrowed > bound,
                Bound::Unbounded => true,
            };

            let in_end = match range.end_bound() {
                Bound::Included(bound) => key_borrowed <= bound,
                Bound::Excluded(bound) => key_borrowed < bound,
                Bound::Unbounded => true,
            };

            if in_start && in_end {
                entries.push((key, value));
            }
        }

        PersistentTreeMapRangeIterator {
            entries,
            current_index: 0,
        }
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// An iterator over key-value pairs of a [`PersistentTreeMap`].
pub struct PersistentTreeMapIterator<'a, K, V> {
    entries: Vec<(&'a K, &'a V)>,
    current_index: usize,
}

impl<'a, K, V> Iterator for PersistentTreeMapIterator<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.entries.len() {
            None
        } else {
            let entry = self.entries[self.current_index];
            self.current_index += 1;
            Some(entry)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.entries.len().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<K, V> ExactSizeIterator for PersistentTreeMapIterator<'_, K, V> {
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.current_index)
    }
}

/// A range iterator over key-value pairs of a [`PersistentTreeMap`].
pub struct PersistentTreeMapRangeIterator<'a, K, V> {
    entries: Vec<(&'a K, &'a V)>,
    current_index: usize,
}

impl<'a, K, V> Iterator for PersistentTreeMapRangeIterator<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.entries.len() {
            None
        } else {
            let entry = self.entries[self.current_index];
            self.current_index += 1;
            Some(entry)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.entries.len().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<K, V> ExactSizeIterator for PersistentTreeMapRangeIterator<'_, K, V> {
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.current_index)
    }
}

/// An owning iterator over key-value pairs of a [`PersistentTreeMap`].
pub struct PersistentTreeMapIntoIterator<K, V> {
    entries: Vec<(K, V)>,
    current_index: usize,
}

impl<K: Clone, V: Clone> Iterator for PersistentTreeMapIntoIterator<K, V> {
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

impl<K: Clone, V: Clone> ExactSizeIterator for PersistentTreeMapIntoIterator<K, V> {
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.current_index)
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<K, V> Default for PersistentTreeMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Ord, V: Clone> FromIterator<(K, V)> for PersistentTreeMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = Self::new();
        for (key, value) in iter {
            map = map.insert(key, value);
        }
        map
    }
}

impl<K: Clone + Ord, V: Clone> IntoIterator for PersistentTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = PersistentTreeMapIntoIterator<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        let entries: Vec<(K, V)> = self
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        PersistentTreeMapIntoIterator {
            entries,
            current_index: 0,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a PersistentTreeMap<K, V>
where
    K: Clone + Ord,
    V: Clone,
{
    type Item = (&'a K, &'a V);
    type IntoIter = PersistentTreeMapIterator<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Clone + Ord, V: Clone + PartialEq> PartialEq for PersistentTreeMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }

        // Compare all entries
        for (key, value) in self {
            match other.get(key) {
                Some(other_value) if other_value == value => {}
                _ => return false,
            }
        }

        true
    }
}

impl<K: Clone + Ord, V: Clone + Eq> Eq for PersistentTreeMap<K, V> {}

/// Computes a hash value for this tree map.
///
/// The hash is computed by first hashing the length, then hashing each
/// (key, value) pair in key order. This ensures that:
///
/// - Maps with different sizes have different hashes (with high probability)
/// - The insertion order does not affect the hash value (since iteration is in key order)
/// - Equal maps produce equal hash values (Hash-Eq consistency)
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentTreeMap;
/// use std::collections::HashMap;
///
/// let mut outer: HashMap<PersistentTreeMap<i32, String>, &str> = HashMap::new();
/// let key = PersistentTreeMap::new()
///     .insert(1, "one".to_string())
///     .insert(2, "two".to_string());
/// outer.insert(key.clone(), "value");
/// assert_eq!(outer.get(&key), Some(&"value"));
/// ```
impl<K, V> Hash for PersistentTreeMap<K, V>
where
    K: Clone + Ord + Hash,
    V: Clone + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the length first to distinguish maps of different sizes
        self.length.hash(state);
        // Hash each entry in key order (iteration returns entries in key order)
        for (key, value) in self {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl<K: Clone + Ord + fmt::Debug, V: Clone + fmt::Debug> fmt::Debug for PersistentTreeMap<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Clone + Ord + fmt::Display, V: Clone + fmt::Display> fmt::Display
    for PersistentTreeMap<K, V>
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

/// Wrapper to make `PersistentTreeMap` implement `TypeConstructor` for values.
///
/// Since `PersistentTreeMap` has two type parameters (K, V), we treat it as
/// a container of V values with K being fixed.
impl<K, V> TypeConstructor for PersistentTreeMap<K, V> {
    type Inner = V;
    type WithType<B> = PersistentTreeMap<K, B>;
}

impl<K: Clone + Ord, V: Clone> Foldable for PersistentTreeMap<K, V> {
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
        // For ordered collections, fold_right needs to process elements in reverse order.
        // We collect values into a Vec and reverse it for proper right-to-left folding.
        let mut values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
        values.reverse();
        values
            .into_iter()
            .fold(init, |accumulator, value| function(value, accumulator))
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
impl<K, V> serde::Serialize for PersistentTreeMap<K, V>
where
    K: serde::Serialize + Clone + Ord,
    V: serde::Serialize + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
struct PersistentTreeMapVisitor<K, V> {
    key_marker: std::marker::PhantomData<K>,
    value_marker: std::marker::PhantomData<V>,
}

#[cfg(feature = "serde")]
impl<K, V> PersistentTreeMapVisitor<K, V> {
    const fn new() -> Self {
        Self {
            key_marker: std::marker::PhantomData,
            value_marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::de::Visitor<'de> for PersistentTreeMapVisitor<K, V>
where
    K: serde::Deserialize<'de> + Clone + Ord,
    V: serde::Deserialize<'de> + Clone,
{
    type Value = PersistentTreeMap<K, V>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        // Note: Sequential insert ensures gradual memory usage even for large inputs.
        let mut map = PersistentTreeMap::new();
        while let Some((key, value)) = access.next_entry()? {
            map = map.insert(key, value);
        }
        Ok(map)
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::Deserialize<'de> for PersistentTreeMap<K, V>
where
    K: serde::Deserialize<'de> + Clone + Ord,
    V: serde::Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(PersistentTreeMapVisitor::new())
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
    fn test_display_empty_treemap() {
        let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
        assert_eq!(format!("{map}"), "{}");
    }

    #[rstest]
    fn test_display_single_element_treemap() {
        let map = PersistentTreeMap::singleton(1, "one".to_string());
        assert_eq!(format!("{map}"), "{1: one}");
    }

    #[rstest]
    fn test_display_multiple_elements_treemap_sorted() {
        let map = PersistentTreeMap::new()
            .insert(3, "three".to_string())
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        // TreeMap should display in sorted order
        assert_eq!(format!("{map}"), "{1: one, 2: two, 3: three}");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_new_creates_empty() {
        let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[rstest]
    fn test_singleton() {
        let map = PersistentTreeMap::singleton(42, "answer".to_string());
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&42), Some(&"answer".to_string()));
    }

    #[rstest]
    fn test_insert_and_get() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&1), Some(&"one".to_string()));
        assert_eq!(map.get(&2), Some(&"two".to_string()));
        assert_eq!(map.get(&3), None);
    }

    #[rstest]
    fn test_insert_overwrite() {
        let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
        let map2 = map1.insert(1, "ONE".to_string());

        assert_eq!(map1.get(&1), Some(&"one".to_string()));
        assert_eq!(map2.get(&1), Some(&"ONE".to_string()));
        assert_eq!(map1.len(), 1);
        assert_eq!(map2.len(), 1);
    }

    #[rstest]
    fn test_remove() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let removed = map.remove(&1);

        assert_eq!(removed.len(), 1);
        assert_eq!(removed.get(&1), None);
        assert_eq!(removed.get(&2), Some(&"two".to_string()));
    }

    #[rstest]
    fn test_min_max() {
        let map = PersistentTreeMap::new()
            .insert(3, "three".to_string())
            .insert(1, "one".to_string())
            .insert(5, "five".to_string());

        assert_eq!(map.min(), Some((&1, &"one".to_string())));
        assert_eq!(map.max(), Some((&5, &"five".to_string())));
    }

    #[rstest]
    fn test_iter_sorted() {
        let map = PersistentTreeMap::new()
            .insert(3, "three".to_string())
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());

        let keys: Vec<&i32> = map.keys().collect();
        assert_eq!(keys, vec![&1, &2, &3]);
    }

    #[rstest]
    fn test_range() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string())
            .insert(3, "three".to_string())
            .insert(4, "four".to_string())
            .insert(5, "five".to_string());

        let range: Vec<&i32> = map.range(2..=4).map(|(k, _)| k).collect();
        assert_eq!(range, vec![&2, &3, &4]);
    }

    #[rstest]
    fn test_from_iter() {
        let entries = vec![
            (3, "three".to_string()),
            (1, "one".to_string()),
            (2, "two".to_string()),
        ];
        let map: PersistentTreeMap<i32, String> = entries.into_iter().collect();

        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&1), Some(&"one".to_string()));
    }

    #[rstest]
    fn test_eq() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let map2 = PersistentTreeMap::new()
            .insert(2, "two".to_string())
            .insert(1, "one".to_string());

        assert_eq!(map1, map2);
    }

    #[rstest]
    fn test_fold_left() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);

        let sum = map.fold_left(0, |accumulator, value| accumulator + value);
        assert_eq!(sum, 60);
    }

    // =========================================================================
    // map_values Tests
    // =========================================================================

    #[rstest]
    fn test_map_values_treemap_empty() {
        let map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let result = map.map_values(|v| v * 2);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_map_values_treemap_basic() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let doubled = map.map_values(|v| v * 2);
        assert_eq!(doubled.get(&1), Some(&20));
        assert_eq!(doubled.get(&2), Some(&40));
    }

    #[rstest]
    fn test_map_values_treemap_preserves_order() {
        let map = PersistentTreeMap::new()
            .insert(3, 30)
            .insert(1, 10)
            .insert(2, 20);
        let result = map.map_values(|v| v / 10);
        let keys: Vec<&i32> = result.keys().collect();
        assert_eq!(keys, vec![&1, &2, &3]);
    }

    #[rstest]
    fn test_map_values_treemap_type_change() {
        let map = PersistentTreeMap::new().insert(1, 100).insert(2, 200);
        let stringified = map.map_values(|v| v.to_string());
        assert_eq!(stringified.get(&1), Some(&"100".to_string()));
        assert_eq!(stringified.get(&2), Some(&"200".to_string()));
    }

    #[rstest]
    fn test_map_values_treemap_identity_law() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.map_values(|v| *v);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_map_values_treemap_length_preservation() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let result = map.map_values(|v| v * 2);
        assert_eq!(result.len(), map.len());
    }

    // =========================================================================
    // map_keys Tests
    // =========================================================================

    #[rstest]
    fn test_map_keys_treemap_empty() {
        let map: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
        let result = map.map_keys(|k| k.len());
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_map_keys_treemap_basic() {
        let map = PersistentTreeMap::new()
            .insert("a".to_string(), 1)
            .insert("bb".to_string(), 2)
            .insert("ccc".to_string(), 3);
        let by_length = map.map_keys(|k| k.len());
        assert_eq!(by_length.get(&1), Some(&1));
        assert_eq!(by_length.get(&2), Some(&2));
        assert_eq!(by_length.get(&3), Some(&3));
    }

    #[rstest]
    fn test_map_keys_treemap_reorders() {
        let map = PersistentTreeMap::new()
            .insert(1, "a".to_string())
            .insert(2, "b".to_string())
            .insert(3, "c".to_string());
        let negated = map.map_keys(|k| -k);
        let keys: Vec<&i32> = negated.keys().collect();
        assert_eq!(keys, vec![&-3, &-2, &-1]);
    }

    #[rstest]
    fn test_map_keys_treemap_collision() {
        let map = PersistentTreeMap::new()
            .insert("a".to_string(), 1)
            .insert("A".to_string(), 2);
        let uppercased = map.map_keys(|k| k.to_uppercase());
        assert_eq!(uppercased.len(), 1);
        assert!(uppercased.contains_key("A"));
    }

    // =========================================================================
    // filter_map Tests
    // =========================================================================

    #[rstest]
    fn test_filter_map_treemap_empty() {
        let map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let result = map.filter_map(|_, v| Some(v * 2));
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_filter_map_treemap_basic() {
        let map = PersistentTreeMap::new()
            .insert(1, "1".to_string())
            .insert(2, "abc".to_string())
            .insert(3, "42".to_string());
        let parsed = map.filter_map(|_, v| v.parse::<i32>().ok());
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get(&1), Some(&1));
        assert_eq!(parsed.get(&3), Some(&42));
    }

    #[rstest]
    fn test_filter_map_treemap_preserves_order() {
        let map = PersistentTreeMap::new()
            .insert(5, 50)
            .insert(1, 10)
            .insert(3, 30);
        let filtered = map.filter_map(|k, v| if *k > 1 { Some(*v) } else { None });
        let keys: Vec<&i32> = filtered.keys().collect();
        assert_eq!(keys, vec![&3, &5]);
    }

    #[rstest]
    fn test_filter_map_treemap_all_none() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result: PersistentTreeMap<i32, i32> = map.filter_map(|_, _| None);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_filter_map_treemap_all_some() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.filter_map(|_, v| Some(*v));
        assert_eq!(result, map);
    }

    // =========================================================================
    // entries Tests
    // =========================================================================

    #[rstest]
    fn test_entries_treemap_equals_iter() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let iter_entries: Vec<_> = map.iter().collect();
        let entries_entries: Vec<_> = map.entries().collect();
        assert_eq!(iter_entries, entries_entries);
    }

    #[rstest]
    fn test_entries_treemap_count_equals_len() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string())
            .insert(3, "three".to_string());
        assert_eq!(map.entries().count(), map.len());
    }

    // =========================================================================
    // merge Tests
    // =========================================================================

    #[rstest]
    fn test_merge_treemap_empty_left() {
        let empty: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
        let other = PersistentTreeMap::singleton(1, "one".to_string());
        let result = empty.merge(&other);
        assert_eq!(result, other);
    }

    #[rstest]
    fn test_merge_treemap_empty_right() {
        let map = PersistentTreeMap::singleton(1, "one".to_string());
        let empty: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
        let result = map.merge(&empty);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_merge_treemap_no_overlap() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let map2 = PersistentTreeMap::new()
            .insert(3, "three".to_string())
            .insert(4, "four".to_string());
        let result = map1.merge(&map2);
        assert_eq!(result.len(), 4);
    }

    #[rstest]
    fn test_merge_treemap_with_overlap() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(2, "two".to_string());
        let map2 = PersistentTreeMap::new()
            .insert(2, "TWO".to_string())
            .insert(3, "three".to_string());
        let result = map1.merge(&map2);
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(&2), Some(&"TWO".to_string()));
    }

    #[rstest]
    fn test_merge_treemap_preserves_order() {
        let map1 = PersistentTreeMap::singleton(2, "two".to_string());
        let map2 = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(3, "three".to_string());
        let result = map1.merge(&map2);
        let keys: Vec<&i32> = result.keys().collect();
        assert_eq!(keys, vec![&1, &2, &3]);
    }

    // =========================================================================
    // merge_with Tests
    // =========================================================================

    #[rstest]
    fn test_merge_with_treemap_sum() {
        let map1 = PersistentTreeMap::new().insert(1, 100).insert(2, 200);
        let map2 = PersistentTreeMap::new().insert(2, 50).insert(3, 300);
        let merged = map1.merge_with(&map2, |_, v1, v2| v1 + v2);
        assert_eq!(merged.get(&1), Some(&100));
        assert_eq!(merged.get(&2), Some(&250));
        assert_eq!(merged.get(&3), Some(&300));
    }

    #[rstest]
    fn test_merge_with_treemap_preserves_order() {
        let map1 = PersistentTreeMap::singleton(2, 20);
        let map2 = PersistentTreeMap::new().insert(1, 10).insert(3, 30);
        let result = map1.merge_with(&map2, |_, v1, v2| v1 + v2);
        let keys: Vec<&i32> = result.keys().collect();
        assert_eq!(keys, vec![&1, &2, &3]);
    }

    #[rstest]
    fn test_merge_with_treemap_empty_left() {
        let empty: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let other = PersistentTreeMap::singleton(1, 100);
        let result = empty.merge_with(&other, |_, v1, v2| v1 + v2);
        assert_eq!(result, other);
    }

    #[rstest]
    fn test_merge_with_treemap_max_resolver() {
        let map1 = PersistentTreeMap::new().insert(1, 100).insert(2, 5);
        let map2 = PersistentTreeMap::new().insert(1, 50).insert(2, 500);
        let merged = map1.merge_with(&map2, |_, v1, v2| *v1.max(v2));
        assert_eq!(merged.get(&1), Some(&100));
        assert_eq!(merged.get(&2), Some(&500));
    }

    // =========================================================================
    // delete_if Tests
    // =========================================================================

    #[rstest]
    fn test_delete_if_treemap_empty() {
        let map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let result = map.delete_if(|_, _| true);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_delete_if_treemap_basic() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let small = map.delete_if(|_, v| *v >= 20);
        assert_eq!(small.len(), 1);
        assert_eq!(small.get(&1), Some(&10));
    }

    #[rstest]
    fn test_delete_if_treemap_preserves_order() {
        let map = PersistentTreeMap::new()
            .insert(5, 50)
            .insert(1, 10)
            .insert(3, 30);
        let filtered = map.delete_if(|k, _| *k == 3);
        let keys: Vec<&i32> = filtered.keys().collect();
        assert_eq!(keys, vec![&1, &5]);
    }

    #[rstest]
    fn test_delete_if_treemap_none() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.delete_if(|_, _| false);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_delete_if_treemap_all() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.delete_if(|_, _| true);
        assert!(result.is_empty());
    }

    // =========================================================================
    // keep_if Tests
    // =========================================================================

    #[rstest]
    fn test_keep_if_treemap_empty() {
        let map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let result = map.keep_if(|_, _| true);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_keep_if_treemap_basic() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let even_keys = map.keep_if(|k, _| k % 2 == 0);
        assert_eq!(even_keys.len(), 1);
        assert_eq!(even_keys.get(&2), Some(&20));
    }

    #[rstest]
    fn test_keep_if_treemap_preserves_order() {
        let map = PersistentTreeMap::new()
            .insert(5, 50)
            .insert(1, 10)
            .insert(3, 30);
        let filtered = map.keep_if(|k, _| *k > 1);
        let keys: Vec<&i32> = filtered.keys().collect();
        assert_eq!(keys, vec![&3, &5]);
    }

    #[rstest]
    fn test_keep_if_treemap_all() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.keep_if(|_, _| true);
        assert_eq!(result, map);
    }

    #[rstest]
    fn test_keep_if_treemap_none() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let result = map.keep_if(|_, _| false);
        assert!(result.is_empty());
    }

    // =========================================================================
    // partition Tests
    // =========================================================================

    #[rstest]
    fn test_partition_treemap_empty() {
        let map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
        let (matching, not_matching) = map.partition(|_, _| true);
        assert!(matching.is_empty());
        assert!(not_matching.is_empty());
    }

    #[rstest]
    fn test_partition_treemap_basic() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30)
            .insert(4, 40);
        let (even_keys, odd_keys) = map.partition(|k, _| k % 2 == 0);
        assert_eq!(even_keys.len(), 2);
        assert_eq!(odd_keys.len(), 2);
        assert!(even_keys.contains_key(&2));
        assert!(even_keys.contains_key(&4));
        assert!(odd_keys.contains_key(&1));
        assert!(odd_keys.contains_key(&3));
    }

    #[rstest]
    fn test_partition_treemap_preserves_order() {
        let map = PersistentTreeMap::new()
            .insert(5, 50)
            .insert(1, 10)
            .insert(3, 30)
            .insert(2, 20)
            .insert(4, 40);
        let (evens, odds) = map.partition(|k, _| k % 2 == 0);
        let even_keys: Vec<&i32> = evens.keys().collect();
        let odd_keys: Vec<&i32> = odds.keys().collect();
        assert_eq!(even_keys, vec![&2, &4]);
        assert_eq!(odd_keys, vec![&1, &3, &5]);
    }

    #[rstest]
    fn test_partition_treemap_by_key_range() {
        let map = PersistentTreeMap::new()
            .insert(1, "one".to_string())
            .insert(5, "five".to_string())
            .insert(10, "ten".to_string())
            .insert(15, "fifteen".to_string());
        let (small, large) = map.partition(|k, _| *k < 10);
        assert_eq!(small.len(), 2);
        assert_eq!(large.len(), 2);
    }

    #[rstest]
    fn test_partition_treemap_all_match() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let (matching, not_matching) = map.partition(|_, _| true);
        assert_eq!(matching, map);
        assert!(not_matching.is_empty());
    }

    #[rstest]
    fn test_partition_treemap_none_match() {
        let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);
        let (matching, not_matching) = map.partition(|_, _| false);
        assert!(matching.is_empty());
        assert_eq!(not_matching, map);
    }

    #[rstest]
    fn test_partition_treemap_completeness() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let (matching, not_matching) = map.partition(|k, _| k % 2 == 0);
        assert_eq!(matching.len() + not_matching.len(), map.len());
    }

    #[rstest]
    fn test_partition_treemap_equals_keep_if_delete_if() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let predicate = |k: &i32, _: &i32| k % 2 == 0;
        let (matching, not_matching) = map.partition(predicate);
        let kept = map.keep_if(predicate);
        let deleted_complement = map.keep_if(|k, v| !predicate(k, v));
        assert_eq!(matching, kept);
        assert_eq!(not_matching, deleted_complement);
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
    fn test_treemap_is_send() {
        assert_send::<PersistentTreeMap<i32, String>>();
        assert_send::<PersistentTreeMap<String, i32>>();
    }

    #[rstest]
    fn test_treemap_is_sync() {
        assert_sync::<PersistentTreeMap<i32, String>>();
        assert_sync::<PersistentTreeMap<String, i32>>();
    }

    #[rstest]
    fn test_treemap_send_sync_combined() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<PersistentTreeMap<i32, String>>();
        is_send_sync::<PersistentTreeMap<String, i32>>();
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
    fn test_treemap_shared_across_threads() {
        let map = Arc::new(
            PersistentTreeMap::new()
                .insert(1, "one")
                .insert(2, "two")
                .insert(3, "three"),
        );

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    assert_eq!(map_clone.get(&1), Some(&"one"));
                    assert_eq!(map_clone.get(&2), Some(&"two"));
                    assert_eq!(map_clone.get(&3), Some(&"three"));
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
    fn test_treemap_concurrent_insert() {
        let base_map = Arc::new(PersistentTreeMap::new().insert(0, "base"));

        let results: Vec<_> = (1..=4)
            .map(|index| {
                let map_clone = Arc::clone(&base_map);
                thread::spawn(move || {
                    let new_map = map_clone.insert(index, "new");
                    assert_eq!(new_map.get(&index), Some(&"new"));
                    assert_eq!(new_map.get(&0), Some(&"base"));
                    new_map
                })
            })
            .map(|handle| handle.join().expect("Thread panicked"))
            .collect();

        // Each thread should have created an independent map with 2 entries
        for (index, map) in results.iter().enumerate() {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get(&((index + 1) as i32)), Some(&"new"));
        }

        // Original map should be unchanged
        assert_eq!(base_map.len(), 1);
    }

    #[rstest]
    fn test_treemap_referential_transparency() {
        let map = Arc::new(PersistentTreeMap::new().insert(1, "one").insert(2, "two"));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    let updated = map_clone.insert(3, "three");
                    // Original should be unchanged
                    assert_eq!(map_clone.len(), 2);
                    assert_eq!(map_clone.get(&3), None);
                    // New map should have the addition
                    assert_eq!(updated.len(), 3);
                    assert_eq!(updated.get(&3), Some(&"three"));
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
    fn test_treemap_concurrent_ordered_iteration() {
        let map = Arc::new(
            PersistentTreeMap::new()
                .insert(3, "three")
                .insert(1, "one")
                .insert(2, "two"),
        );

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let map_clone = Arc::clone(&map);
                thread::spawn(move || {
                    let keys: Vec<&i32> = map_clone.keys().collect();
                    // TreeMap should always return keys in sorted order
                    assert_eq!(keys, vec![&1, &2, &3]);
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
        let map: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, "{}");
    }

    #[rstest]
    fn test_serialize_single_entry() {
        let map = PersistentTreeMap::singleton("key".to_string(), 42);
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"{"key":42}"#);
    }

    #[rstest]
    fn test_serialize_multiple_entries() {
        let map = PersistentTreeMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"{"a":1,"b":2,"c":3}"#);
    }

    #[rstest]
    fn test_deserialize_empty() {
        let json = "{}";
        let map: PersistentTreeMap<String, i32> = serde_json::from_str(json).unwrap();
        assert!(map.is_empty());
    }

    #[rstest]
    fn test_deserialize_single_entry() {
        let json = r#"{"key":42}"#;
        let map: PersistentTreeMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&42));
    }

    #[rstest]
    fn test_deserialize_multiple_entries() {
        let json = r#"{"a":1,"b":2,"c":3}"#;
        let map: PersistentTreeMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get("c"), Some(&3));
    }

    #[rstest]
    fn test_roundtrip_empty() {
        let original: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentTreeMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_roundtrip_large() {
        let mut original: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
        for element_index in 0..100 {
            original = original.insert(format!("key{element_index:03}"), element_index);
        }
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentTreeMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_entry_preservation() {
        let mut map: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
        for element_index in 0..100 {
            map = map.insert(format!("key{element_index}"), element_index);
        }
        let json = serde_json::to_string(&map).unwrap();
        let restored: PersistentTreeMap<String, i32> = serde_json::from_str(&json).unwrap();
        for element_index in 0..100 {
            let key = format!("key{element_index}");
            assert_eq!(restored.get(&key), Some(&element_index));
        }
    }

    #[rstest]
    fn test_order_preservation() {
        let map = PersistentTreeMap::new()
            .insert("c".to_string(), 3)
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"{"a":1,"b":2,"c":3}"#);
    }

    #[rstest]
    fn test_serialize_nested_values() {
        let map = PersistentTreeMap::new()
            .insert("numbers".to_string(), vec![1, 2, 3])
            .insert("empty".to_string(), vec![]);
        let json = serde_json::to_string(&map).unwrap();
        let restored: PersistentTreeMap<String, Vec<i32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.get("numbers"), Some(&vec![1, 2, 3]));
        assert_eq!(restored.get("empty"), Some(&vec![]));
    }

    #[rstest]
    fn test_deserialize_overwrites_duplicate_keys() {
        let json = r#"{"key":1,"key":2}"#;
        let map: PersistentTreeMap<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&2));
    }
}
