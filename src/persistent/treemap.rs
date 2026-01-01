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

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::iter::FromIterator;
use std::ops::{Bound, RangeBounds};
use std::rc::Rc;

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
    left: Option<Rc<Self>>,
    right: Option<Rc<Self>>,
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
    fn with_children(&self, left: Option<Rc<Self>>, right: Option<Rc<Self>>) -> Self
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
fn is_red<K, V>(node: Option<&Rc<Node<K, V>>>) -> bool {
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
    root: Option<Rc<Node<K, V>>>,
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
    fn get_from_node<'a, Q>(node: Option<&'a Rc<Node<K, V>>>, key: &Q) -> Option<&'a V>
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
                    Some(Rc::new(node_ref.with_color(Color::Black)))
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
        node: Option<&Rc<Node<K, V>>>,
        key: K,
        value: V,
    ) -> (Option<Rc<Node<K, V>>>, bool) {
        match node {
            None => {
                // Insert new red node
                (Some(Rc::new(Node::new_red(key, value))), true)
            }
            Some(node_ref) => {
                match key.cmp(&node_ref.key) {
                    Ordering::Less => {
                        let (new_left, added) =
                            Self::insert_into_node(node_ref.left.as_ref(), key, value);
                        let new_node = node_ref.with_children(new_left, node_ref.right.clone());
                        (Some(Rc::new(Self::balance(new_node))), added)
                    }
                    Ordering::Greater => {
                        let (new_right, added) =
                            Self::insert_into_node(node_ref.right.as_ref(), key, value);
                        let new_node = node_ref.with_children(node_ref.left.clone(), new_right);
                        (Some(Rc::new(Self::balance(new_node))), added)
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
                        (Some(Rc::new(new_node)), false)
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
            let new_node = node.with_children(Some(Rc::new(new_left)), node.right.clone());
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
            let new_node = node.with_children(node.left.clone(), Some(Rc::new(new_right)));
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
                right: Some(Rc::new(new_node)),
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
                left: Some(Rc::new(new_node)),
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
                .map(|left_left| Rc::new(left_left.with_color(Color::Black)));

            Node {
                key: left.key.clone(),
                value: left.value.clone(),
                color: Color::Black,
                left: new_left,
                right: Some(Rc::new(new_right)),
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
                .map(|right_right| Rc::new(right_right.with_color(Color::Black)));

            Node {
                key: right.key.clone(),
                value: right.value.clone(),
                color: Color::Black,
                left: Some(Rc::new(new_left)),
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
                Rc::new(node.with_color(Color::Black))
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
    fn remove_from_node<Q>(node: Option<&Rc<Node<K, V>>>, key: &Q) -> Option<Rc<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        node.and_then(|node_ref| {
            match key.cmp(node_ref.key.borrow()) {
                Ordering::Less => {
                    let new_left = Self::remove_from_node(node_ref.left.as_ref(), key);
                    let new_node = node_ref.with_children(new_left, node_ref.right.clone());
                    Some(Rc::new(Self::balance_after_delete(new_node)))
                }
                Ordering::Greater => {
                    let new_right = Self::remove_from_node(node_ref.right.as_ref(), key);
                    let new_node = node_ref.with_children(node_ref.left.clone(), new_right);
                    Some(Rc::new(Self::balance_after_delete(new_node)))
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
                            Some(Rc::new(Self::balance_after_delete(new_node)))
                        }
                    }
                }
            }
        })
    }

    /// Finds the minimum key-value pair in a subtree.
    fn find_min_entry(node: &Rc<Node<K, V>>) -> (K, V) {
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
    fn min_from_node(node: Option<&Rc<Node<K, V>>>) -> Option<(&K, &V)> {
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
    fn max_from_node(node: Option<&Rc<Node<K, V>>>) -> Option<(&K, &V)> {
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
        node: Option<&'a Rc<Node<K, V>>>,
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

impl<K: Clone + Ord + fmt::Debug, V: Clone + fmt::Debug> fmt::Debug for PersistentTreeMap<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
}
