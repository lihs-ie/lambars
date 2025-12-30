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
//! - O(1) len and is_empty
//!
//! All operations return new maps without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Examples
//!
//! ```rust
//! use functional_rusty::persistent::PersistentHashMap;
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
//! - Structural sharing via `Rc`

use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
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

/// Computes the hash of a key using DefaultHasher.
fn compute_hash<K: Hash + ?Sized>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Extracts the index at a given depth from a hash.
#[inline]
fn hash_index(hash: u64, depth: usize) -> usize {
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
        children: Rc<[Child<K, V>]>,
    },
    /// Collision node for keys with the same hash
    Collision { hash: u64, entries: Rc<[(K, V)]> },
}

/// A child in a bitmap node.
#[derive(Clone)]
enum Child<K, V> {
    /// A key-value entry
    Entry { key: K, value: V },
    /// A sub-node
    Node(Rc<Node<K, V>>),
}

impl<K, V> Node<K, V> {
    /// Creates an empty node.
    fn empty() -> Self {
        Node::Empty
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
/// use functional_rusty::persistent::PersistentHashMap;
///
/// let map = PersistentHashMap::singleton("key".to_string(), 42);
/// assert_eq!(map.get("key"), Some(&42));
/// ```
#[derive(Clone)]
pub struct PersistentHashMap<K, V> {
    /// Root node of the trie
    root: Rc<Node<K, V>>,
    /// Number of entries
    length: usize,
}

impl<K, V> PersistentHashMap<K, V> {
    /// Creates a new empty map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::persistent::PersistentHashMap;
    ///
    /// let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        PersistentHashMap {
            root: Rc::new(Node::empty()),
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
    /// use functional_rusty::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    /// assert_eq!(map.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the map contains no entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::persistent::PersistentHashMap;
    ///
    /// let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.insert("key".to_string(), 42);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
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
    /// use functional_rusty::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::singleton("key".to_string(), 42);
    /// assert_eq!(map.len(), 1);
    /// assert_eq!(map.get("key"), Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(key: K, value: V) -> Self {
        PersistentHashMap::new().insert(key, value)
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
        self.get_from_node(&self.root, key, hash, 0)
    }

    /// Recursive helper for get.
    fn get_from_node<'a, Q>(
        &'a self,
        node: &'a Node<K, V>,
        key: &Q,
        hash: u64,
        depth: usize,
    ) -> Option<&'a V>
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
                        } => {
                            if child_key.borrow() == key {
                                Some(value)
                            } else {
                                None
                            }
                        }
                        Child::Node(subnode) => self.get_from_node(subnode, key, hash, depth + 1),
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
        let (new_root, added) = self.insert_into_node(&self.root, key, value, hash, 0);

        PersistentHashMap {
            root: Rc::new(new_root),
            length: if added { self.length + 1 } else { self.length },
        }
    }

    /// Recursive helper for insert.
    /// Returns (new_node, was_added) where was_added is true if a new entry was added.
    fn insert_into_node(
        &self,
        node: &Node<K, V>,
        key: K,
        value: V,
        hash: u64,
        depth: usize,
    ) -> (Node<K, V>, bool) {
        match node {
            Node::Empty => {
                // Create a new entry node
                (Node::Entry { hash, key, value }, true)
            }
            Node::Entry {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
            } => {
                if *existing_hash == hash && *existing_key == key {
                    // Same key, replace value
                    (Node::Entry { hash, key, value }, false)
                } else if *existing_hash == hash {
                    // Hash collision - create collision node
                    let entries = Rc::from(vec![
                        (existing_key.clone(), existing_value.clone()),
                        (key, value),
                    ]);
                    (Node::Collision { hash, entries }, true)
                } else {
                    // Different hash - need to create a bitmap node
                    let existing_index = hash_index(*existing_hash, depth);
                    let new_index = hash_index(hash, depth);

                    if existing_index == new_index {
                        // Same index at this level - recurse
                        let sub_entry = Node::Entry {
                            hash: *existing_hash,
                            key: existing_key.clone(),
                            value: existing_value.clone(),
                        };
                        let (subnode, added) =
                            self.insert_into_node(&sub_entry, key, value, hash, depth + 1);

                        let bitmap = 1u32 << existing_index;
                        let children = Rc::from(vec![Child::Node(Rc::new(subnode))]);

                        (Node::Bitmap { bitmap, children }, added)
                    } else {
                        // Different indices - create bitmap with two children
                        let bitmap = (1u32 << existing_index) | (1u32 << new_index);

                        let children: Vec<Child<K, V>> = if existing_index < new_index {
                            vec![
                                Child::Entry {
                                    key: existing_key.clone(),
                                    value: existing_value.clone(),
                                },
                                Child::Entry { key, value },
                            ]
                        } else {
                            vec![
                                Child::Entry { key, value },
                                Child::Entry {
                                    key: existing_key.clone(),
                                    value: existing_value.clone(),
                                },
                            ]
                        };

                        (
                            Node::Bitmap {
                                bitmap,
                                children: Rc::from(children),
                            },
                            true,
                        )
                    }
                }
            }
            Node::Bitmap { bitmap, children } => {
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;
                let position = (bitmap & (bit - 1)).count_ones() as usize;

                if bitmap & bit == 0 {
                    // Slot is empty - add new entry
                    let mut new_children = children.to_vec();
                    new_children.insert(position, Child::Entry { key, value });

                    (
                        Node::Bitmap {
                            bitmap: bitmap | bit,
                            children: Rc::from(new_children),
                        },
                        true,
                    )
                } else {
                    // Slot is occupied
                    let mut new_children = children.to_vec();

                    match &children[position] {
                        Child::Entry {
                            key: child_key,
                            value: child_value,
                        } => {
                            let child_hash = compute_hash(child_key);
                            if *child_key == key {
                                // Same key - replace
                                new_children[position] = Child::Entry { key, value };
                                (
                                    Node::Bitmap {
                                        bitmap: *bitmap,
                                        children: Rc::from(new_children),
                                    },
                                    false,
                                )
                            } else if child_hash == hash {
                                // Hash collision
                                let collision = Node::Collision {
                                    hash,
                                    entries: Rc::from(vec![
                                        (child_key.clone(), child_value.clone()),
                                        (key, value),
                                    ]),
                                };
                                new_children[position] = Child::Node(Rc::new(collision));
                                (
                                    Node::Bitmap {
                                        bitmap: *bitmap,
                                        children: Rc::from(new_children),
                                    },
                                    true,
                                )
                            } else {
                                // Need to create subnode
                                let child_entry = Node::Entry {
                                    hash: child_hash,
                                    key: child_key.clone(),
                                    value: child_value.clone(),
                                };
                                let (subnode, added) = self.insert_into_node(
                                    &child_entry,
                                    key,
                                    value,
                                    hash,
                                    depth + 1,
                                );
                                new_children[position] = Child::Node(Rc::new(subnode));

                                (
                                    Node::Bitmap {
                                        bitmap: *bitmap,
                                        children: Rc::from(new_children),
                                    },
                                    added,
                                )
                            }
                        }
                        Child::Node(subnode) => {
                            let (new_subnode, added) =
                                self.insert_into_node(subnode, key, value, hash, depth + 1);
                            new_children[position] = Child::Node(Rc::new(new_subnode));

                            (
                                Node::Bitmap {
                                    bitmap: *bitmap,
                                    children: Rc::from(new_children),
                                },
                                added,
                            )
                        }
                    }
                }
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if hash == *collision_hash {
                    // Same hash - update or add to collision node
                    let mut new_entries = entries.to_vec();
                    let mut found = false;

                    for entry in new_entries.iter_mut() {
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
                            hash: *collision_hash,
                            entries: Rc::from(new_entries),
                        },
                        !found,
                    )
                } else {
                    // Different hash - need to convert to bitmap node
                    // This is a complex case, handle by creating a bitmap node
                    let collision_index = hash_index(*collision_hash, depth);
                    let new_index = hash_index(hash, depth);

                    if collision_index == new_index {
                        // Same index - recurse with collision as subnode
                        let (subnode, added) =
                            self.insert_into_node(node, key, value, hash, depth + 1);
                        let bitmap = 1u32 << collision_index;
                        let children = Rc::from(vec![Child::Node(Rc::new(subnode))]);
                        (Node::Bitmap { bitmap, children }, added)
                    } else {
                        let bitmap = (1u32 << collision_index) | (1u32 << new_index);
                        let children: Vec<Child<K, V>> = if collision_index < new_index {
                            vec![
                                Child::Node(Rc::new(node.clone())),
                                Child::Entry { key, value },
                            ]
                        } else {
                            vec![
                                Child::Entry { key, value },
                                Child::Node(Rc::new(node.clone())),
                            ]
                        };

                        (
                            Node::Bitmap {
                                bitmap,
                                children: Rc::from(children),
                            },
                            true,
                        )
                    }
                }
            }
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
        match self.remove_from_node(&self.root, key, hash, 0) {
            Some((new_root, removed)) => {
                if removed {
                    PersistentHashMap {
                        root: Rc::new(new_root),
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
    /// Returns Some((new_node, was_removed)) or None if no change needed.
    fn remove_from_node<Q>(
        &self,
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
                let index = hash_index(hash, depth);
                let bit = 1u32 << index;

                if bitmap & bit == 0 {
                    // Key not in this subtree
                    None
                } else {
                    let position = (bitmap & (bit - 1)).count_ones() as usize;

                    match &children[position] {
                        Child::Entry { key: child_key, .. } => {
                            if child_key.borrow() == key {
                                // Found the entry to remove
                                let new_bitmap = bitmap & !bit;

                                if new_bitmap == 0 {
                                    // No children left
                                    Some((Node::Empty, true))
                                } else {
                                    let mut new_children = children.to_vec();
                                    new_children.remove(position);

                                    // If only one child left and it's an entry, we might want to simplify
                                    if new_children.len() == 1 {
                                        if let Child::Entry { key, value } = &new_children[0] {
                                            // Can simplify to single entry
                                            let entry_hash = compute_hash(key);
                                            return Some((
                                                Node::Entry {
                                                    hash: entry_hash,
                                                    key: key.clone(),
                                                    value: value.clone(),
                                                },
                                                true,
                                            ));
                                        }
                                    }

                                    Some((
                                        Node::Bitmap {
                                            bitmap: new_bitmap,
                                            children: Rc::from(new_children),
                                        },
                                        true,
                                    ))
                                }
                            } else {
                                None
                            }
                        }
                        Child::Node(subnode) => {
                            if let Some((new_subnode, removed)) =
                                self.remove_from_node(subnode, key, hash, depth + 1)
                            {
                                if !removed {
                                    return None;
                                }

                                let mut new_children = children.to_vec();

                                match &new_subnode {
                                    Node::Empty => {
                                        // Subnode became empty, remove it
                                        let new_bitmap = bitmap & !bit;

                                        if new_bitmap == 0 {
                                            return Some((Node::Empty, true));
                                        }

                                        new_children.remove(position);

                                        // Simplify if only one child left
                                        if new_children.len() == 1 {
                                            if let Child::Entry { key, value } = &new_children[0] {
                                                let entry_hash = compute_hash(key);
                                                return Some((
                                                    Node::Entry {
                                                        hash: entry_hash,
                                                        key: key.clone(),
                                                        value: value.clone(),
                                                    },
                                                    true,
                                                ));
                                            }
                                        }

                                        Some((
                                            Node::Bitmap {
                                                bitmap: new_bitmap,
                                                children: Rc::from(new_children),
                                            },
                                            true,
                                        ))
                                    }
                                    Node::Entry {
                                        hash: entry_hash,
                                        key: entry_key,
                                        value: entry_value,
                                    } => {
                                        // Subnode collapsed to single entry
                                        new_children[position] = Child::Entry {
                                            key: entry_key.clone(),
                                            value: entry_value.clone(),
                                        };

                                        // Check if we can simplify further
                                        if new_children.len() == 1 {
                                            return Some((
                                                Node::Entry {
                                                    hash: *entry_hash,
                                                    key: entry_key.clone(),
                                                    value: entry_value.clone(),
                                                },
                                                true,
                                            ));
                                        }

                                        Some((
                                            Node::Bitmap {
                                                bitmap: *bitmap,
                                                children: Rc::from(new_children),
                                            },
                                            true,
                                        ))
                                    }
                                    _ => {
                                        new_children[position] = Child::Node(Rc::new(new_subnode));
                                        Some((
                                            Node::Bitmap {
                                                bitmap: *bitmap,
                                                children: Rc::from(new_children),
                                            },
                                            true,
                                        ))
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    }
                }
            }
            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if hash != *collision_hash {
                    return None;
                }

                let mut new_entries = entries.to_vec();
                let mut found_index = None;

                for (index, (entry_key, _)) in new_entries.iter().enumerate() {
                    if entry_key.borrow() == key {
                        found_index = Some(index);
                        break;
                    }
                }

                if let Some(index) = found_index {
                    new_entries.remove(index);

                    if new_entries.is_empty() {
                        Some((Node::Empty, true))
                    } else if new_entries.len() == 1 {
                        let (remaining_key, remaining_value) = new_entries.remove(0);
                        Some((
                            Node::Entry {
                                hash: *collision_hash,
                                key: remaining_key,
                                value: remaining_value,
                            },
                            true,
                        ))
                    } else {
                        Some((
                            Node::Collision {
                                hash: *collision_hash,
                                entries: Rc::from(new_entries),
                            },
                            true,
                        ))
                    }
                } else {
                    None
                }
            }
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
        let actual_key = self.find_key(&self.root, key, hash, 0)?;

        Some(self.insert(actual_key, new_value))
    }

    /// Finds and clones the key matching the given query key.
    fn find_key<Q>(&self, node: &Node<K, V>, key: &Q, hash: u64, depth: usize) -> Option<K>
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
                        Child::Node(subnode) => self.find_key(subnode, key, hash, depth + 1),
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
    ///               the new value (or None to remove)
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::persistent::PersistentHashMap;
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
                let actual_key = self
                    .find_key(&self.root, key, hash, 0)
                    .unwrap_or_else(|| key.to_owned());
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
        for (key, value) in other.iter() {
            result = result.insert(key.clone(), value.clone());
        }
        result
    }

    /// Returns an iterator over key-value pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::persistent::PersistentHashMap;
    ///
    /// let map = PersistentHashMap::new()
    ///     .insert("a".to_string(), 1)
    ///     .insert("b".to_string(), 2);
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> PersistentHashMapIterator<'_, K, V> {
        let mut entries = Vec::new();
        self.collect_entries(&self.root, &mut entries);
        PersistentHashMapIterator {
            entries,
            current_index: 0,
        }
    }

    /// Collects all entries from a node into a vector.
    fn collect_entries<'a>(&'a self, node: &'a Node<K, V>, entries: &mut Vec<(&'a K, &'a V)>) {
        match node {
            Node::Empty => {}
            Node::Entry { key, value, .. } => {
                entries.push((key, value));
            }
            Node::Bitmap { children, .. } => {
                for child in children.iter() {
                    match child {
                        Child::Entry { key, value } => {
                            entries.push((key, value));
                        }
                        Child::Node(subnode) => {
                            self.collect_entries(subnode, entries);
                        }
                    }
                }
            }
            Node::Collision {
                entries: collision_entries,
                ..
            } => {
                for (key, value) in collision_entries.iter() {
                    entries.push((key, value));
                }
            }
        }
    }

    /// Returns an iterator over keys.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::persistent::PersistentHashMap;
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
    /// use functional_rusty::persistent::PersistentHashMap;
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
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// An iterator over key-value pairs of a [`PersistentHashMap`].
pub struct PersistentHashMapIterator<'a, K, V> {
    entries: Vec<(&'a K, &'a V)>,
    current_index: usize,
}

impl<'a, K, V> Iterator for PersistentHashMapIterator<'a, K, V> {
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

impl<K, V> ExactSizeIterator for PersistentHashMapIterator<'_, K, V> {
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.current_index)
    }
}

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
        PersistentHashMap::new()
    }
}

impl<K: Clone + Hash + Eq, V: Clone> FromIterator<(K, V)> for PersistentHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = PersistentHashMap::new();
        for (key, value) in iter {
            map = map.insert(key, value);
        }
        map
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

        for (key, value) in self.iter() {
            match other.get(key) {
                Some(other_value) if other_value == value => continue,
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

// =============================================================================
// Type Class Implementations
// =============================================================================

/// Wrapper to make PersistentHashMap implement TypeConstructor for values.
///
/// Since PersistentHashMap has two type parameters (K, V), we treat it as
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
        let entries: Vec<_> = self.into_iter().collect();
        entries
            .into_iter()
            .rev()
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
}
