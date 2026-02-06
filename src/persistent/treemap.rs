//! Persistent (immutable) tree map based on B-Tree.
//!
//! This module provides [`PersistentTreeMap`], an immutable ordered map
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentTreeMap` is based on a persistent B-Tree, a balanced tree
//! structure that provides efficient ordered map operations with lower
//! overhead than Red-Black Trees due to fewer rebalancing operations.
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
//! The B-Tree maintains the following invariants:
//! 1. All leaf nodes are at the same depth
//! 2. Each node (except root) has between `MIN_KEYS` and `MAX_KEYS` entries
//! 3. Root has between 1 and `MAX_KEYS` entries (or is empty)
//! 4. Internal nodes have one more child than entries
//!
//! These invariants ensure the tree height is O(log B N).

use super::ReferenceCounter;
use smallvec::{SmallVec, smallvec};
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::rc::Rc;

use crate::typeclass::{Foldable, TypeConstructor};

const BRANCHING_FACTOR: usize = 16;
const MIN_KEYS: usize = BRANCHING_FACTOR - 1;
const MAX_KEYS: usize = 2 * BRANCHING_FACTOR - 1;

/// Type alias for the entry array.
/// Up to `MAX_KEYS` + 1 = 32 elements are placed on the stack.
type EntryArray<K, V> = SmallVec<[(K, V); 32]>;

/// Type alias for the child node array.
/// Since `SmallVec` only supports up to 32 elements, the child node array
/// uses a regular Vec. This incurs one heap allocation per node, but we
/// still benefit from the stack allocation of the entry array.
type ChildArray<K, V> = Vec<ReferenceCounter<BTreeNode<K, V>>>;

/// Internal node structure for the B-Tree.
///
/// This is a B-Tree (not B+Tree), so internal nodes also store key-value pairs.
#[derive(Clone)]
enum BTreeNode<K, V> {
    Leaf {
        entries: EntryArray<K, V>,
    },
    Internal {
        /// entries[i] is between children[i] and children[i+1].
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
    },
}

impl<K, V> BTreeNode<K, V> {
    fn new_leaf(key: K, value: V) -> Self {
        Self::Leaf {
            entries: smallvec![(key, value)],
        }
    }

    fn entry_count(&self) -> usize {
        match self {
            Self::Leaf { entries } | Self::Internal { entries, .. } => entries.len(),
        }
    }

    fn empty_placeholder() -> Self {
        Self::Leaf {
            entries: SmallVec::new(),
        }
    }
}

impl<K: Clone, V: Clone> BTreeNode<K, V> {
    /// Takes ownership of a child node or clones it if shared.
    ///
    /// Returns a tuple of (node, `took_ownership`) where:
    /// - `took_ownership = true`: The original reference was taken and needs to be replaced.
    /// - `took_ownership = false`: A clone was returned; the original reference is unchanged.
    fn take_or_clone_child(children: &mut ChildArray<K, V>, index: usize) -> (Self, bool) {
        // Check if we're the only owner before attempting to take ownership.
        // If there are multiple owners, clone directly without creating a placeholder.
        if ReferenceCounter::strong_count(&children[index]) > 1 {
            return ((*children[index]).clone(), false);
        }

        // We're the only owner, so we can take ownership.
        // Use empty_placeholder only for the swap; it will be replaced immediately.
        let reference = std::mem::replace(
            &mut children[index],
            ReferenceCounter::new(Self::empty_placeholder()),
        );

        // This should always succeed since we checked strong_count above,
        // but handle the error case defensively.
        match ReferenceCounter::try_unwrap(reference) {
            Ok(node) => (node, true),
            Err(reference_counter) => {
                // This case should not occur given the strong_count check above,
                // but handle it defensively by restoring the reference.
                let cloned = (*reference_counter).clone();
                children[index] = reference_counter;
                (cloned, false)
            }
        }
    }

    /// Takes ownership of a child node reference or clones it if shared.
    ///
    /// Returns a tuple of (node, `took_ownership`) where:
    /// - `took_ownership = true`: The original reference was taken and needs to be replaced.
    /// - `took_ownership = false`: A clone was returned; the original reference is unchanged.
    fn take_or_clone_child_ref(child_ref: &mut ReferenceCounter<Self>) -> (Self, bool) {
        // Check if we're the only owner before attempting to take ownership.
        // If there are multiple owners, clone directly without creating a placeholder.
        if ReferenceCounter::strong_count(child_ref) > 1 {
            return ((**child_ref).clone(), false);
        }

        // We're the only owner, so we can take ownership.
        let placeholder = ReferenceCounter::new(Self::empty_placeholder());
        let reference = std::mem::replace(child_ref, placeholder);

        // This should always succeed since we checked strong_count above.
        match ReferenceCounter::try_unwrap(reference) {
            Ok(node) => (node, true),
            Err(reference_counter) => {
                // Defensive: restore the reference if try_unwrap fails unexpectedly.
                let cloned = (*reference_counter).clone();
                *child_ref = reference_counter;
                (cloned, false)
            }
        }
    }
}

impl<K: Clone + Ord, V: Clone> BTreeNode<K, V> {
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self {
            Self::Leaf { entries } => entries
                .binary_search_by(|(k, _)| k.borrow().cmp(key))
                .ok()
                .map(|index| &entries[index].1),
            Self::Internal { entries, children } => {
                match entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
                    Ok(index) => Some(&entries[index].1),
                    Err(index) => children[index].get(key),
                }
            }
        }
    }

    fn min(&self) -> Option<(&K, &V)> {
        match self {
            Self::Leaf { entries } => entries.first().map(|(k, v)| (k, v)),
            Self::Internal { children, .. } => children.first().and_then(|c| c.min()),
        }
    }

    fn max(&self) -> Option<(&K, &V)> {
        match self {
            Self::Leaf { entries } => entries.last().map(|(k, v)| (k, v)),
            Self::Internal { children, .. } => children.last().and_then(|c| c.max()),
        }
    }
}

enum InsertResult<K, V> {
    Done {
        node: BTreeNode<K, V>,
        added: bool,
        old_value: Option<V>,
    },
    Split {
        left: BTreeNode<K, V>,
        median: (K, V),
        right: BTreeNode<K, V>,
        added: bool,
        old_value: Option<V>,
    },
}

impl<K: Clone + Ord, V: Clone> BTreeNode<K, V> {
    fn insert(self, key: K, value: V) -> InsertResult<K, V> {
        match self {
            Self::Leaf { mut entries } => match entries.binary_search_by(|(k, _)| k.cmp(&key)) {
                Ok(index) => {
                    let old_value = std::mem::replace(&mut entries[index], (key, value)).1;
                    InsertResult::Done {
                        node: Self::Leaf { entries },
                        added: false,
                        old_value: Some(old_value),
                    }
                }
                Err(index) => {
                    entries.insert(index, (key, value));
                    if entries.len() <= MAX_KEYS {
                        InsertResult::Done {
                            node: Self::Leaf { entries },
                            added: true,
                            old_value: None,
                        }
                    } else {
                        Self::split_leaf(entries)
                    }
                }
            },
            Self::Internal {
                mut entries,
                mut children,
            } => match entries.binary_search_by(|(k, _)| k.cmp(&key)) {
                Ok(index) => {
                    let old_value = std::mem::replace(&mut entries[index], (key, value)).1;
                    InsertResult::Done {
                        node: Self::Internal { entries, children },
                        added: false,
                        old_value: Some(old_value),
                    }
                }
                Err(index) => {
                    // Insert always modifies the child, so we always need to replace it.
                    let (child, _took_ownership) = Self::take_or_clone_child(&mut children, index);
                    match child.insert(key, value) {
                        InsertResult::Done {
                            node,
                            added,
                            old_value,
                        } => {
                            children[index] = ReferenceCounter::new(node);
                            InsertResult::Done {
                                node: Self::Internal { entries, children },
                                added,
                                old_value,
                            }
                        }
                        InsertResult::Split {
                            left,
                            median,
                            right,
                            added,
                            old_value,
                        } => {
                            entries.insert(index, median);
                            children[index] = ReferenceCounter::new(left);
                            children.insert(index + 1, ReferenceCounter::new(right));

                            if entries.len() <= MAX_KEYS {
                                InsertResult::Done {
                                    node: Self::Internal { entries, children },
                                    added,
                                    old_value,
                                }
                            } else {
                                Self::split_internal(entries, children, added, old_value)
                            }
                        }
                    }
                }
            },
        }
    }

    fn split_leaf(mut entries: EntryArray<K, V>) -> InsertResult<K, V> {
        let mid = entries.len() / 2;
        let right_entries: EntryArray<K, V> = entries.drain(mid + 1..).collect();
        let median = entries.pop().unwrap();

        InsertResult::Split {
            left: Self::Leaf { entries },
            median,
            right: Self::Leaf {
                entries: right_entries,
            },
            added: true,
            old_value: None,
        }
    }

    fn split_internal(
        mut entries: EntryArray<K, V>,
        mut children: ChildArray<K, V>,
        added: bool,
        old_value: Option<V>,
    ) -> InsertResult<K, V> {
        let mid = entries.len() / 2;
        let right_entries: EntryArray<K, V> = entries.drain(mid + 1..).collect();
        let median = entries.pop().unwrap();
        let right_children: ChildArray<K, V> = children.split_off(mid + 1);

        InsertResult::Split {
            left: Self::Internal { entries, children },
            median,
            right: Self::Internal {
                entries: right_entries,
                children: right_children,
            },
            added,
            old_value,
        }
    }
}

enum RemoveResult<K, V> {
    /// Key was not found; the node (potentially cloned or reconstructed) is returned.
    ///
    /// The caller should use the `took_ownership` flag from `take_or_clone_*` to determine
    /// whether to restore this node or simply discard it (if the original reference was preserved).
    NotFound(BTreeNode<K, V>),
    Done {
        node: Option<BTreeNode<K, V>>,
        removed_value: V,
    },
    Underflow {
        node: BTreeNode<K, V>,
        removed_value: V,
    },
}

/// Result of removing the maximum entry from a subtree.
enum RemoveMaxResult<K, V> {
    /// Removal succeeded without issues.
    Done,
    /// The node became empty after removal.
    Empty,
    /// The node is underflowed (has fewer than `MIN_KEYS` entries).
    Underflow(BTreeNode<K, V>),
}

/// Result of underflow handling without a removed value.
enum UnderflowHandleResult<K, V> {
    /// Underflow was resolved, node is valid.
    Done(BTreeNode<K, V>),
    /// Node became empty.
    Empty,
    /// Node still has underflow (propagate up).
    Underflow(BTreeNode<K, V>),
}

impl<K: Clone + Ord, V: Clone> BTreeNode<K, V> {
    #[allow(clippy::option_if_let_else)]
    fn remove<Q>(self, key: &Q) -> RemoveResult<K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self {
            Self::Leaf { mut entries } => {
                match entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
                    Ok(index) => {
                        let (_, removed_value) = entries.remove(index);
                        Self::make_remove_result(
                            if entries.is_empty() {
                                None
                            } else {
                                Some(Self::Leaf { entries })
                            },
                            removed_value,
                        )
                    }
                    Err(_) => RemoveResult::NotFound(Self::Leaf { entries }),
                }
            }
            Self::Internal {
                mut entries,
                mut children,
            } => match entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
                Ok(index) => {
                    let (replacement, max_result) =
                        Self::remove_max_from_child(&mut children[index]);
                    let old_value = std::mem::replace(&mut entries[index], replacement).1;

                    match max_result {
                        RemoveMaxResult::Done => {
                            Self::rebalance_after_remove(entries, children, old_value)
                        }
                        RemoveMaxResult::Empty => {
                            unreachable!("Empty should not occur for non-root nodes")
                        }
                        RemoveMaxResult::Underflow(underflowed) => {
                            children[index] = ReferenceCounter::new(underflowed);
                            Self::handle_underflow(entries, children, index, old_value)
                        }
                    }
                }
                Err(index) => {
                    let (child, took_ownership) = Self::take_or_clone_child(&mut children, index);
                    match child.remove(key) {
                        RemoveResult::NotFound(unchanged_child) => {
                            // Key was not found. Restore the child only if we took ownership.
                            // If we cloned (took_ownership == false), the original reference is unchanged.
                            if took_ownership {
                                children[index] = ReferenceCounter::new(unchanged_child);
                            }
                            // Return the whole internal node unchanged
                            RemoveResult::NotFound(Self::Internal { entries, children })
                        }
                        RemoveResult::Done {
                            node: Some(new_child),
                            removed_value,
                        } => {
                            children[index] = ReferenceCounter::new(new_child);
                            RemoveResult::Done {
                                node: Some(Self::Internal { entries, children }),
                                removed_value,
                            }
                        }
                        RemoveResult::Done {
                            node: None,
                            removed_value,
                        } => {
                            children.remove(index);
                            if entries.is_empty() {
                                RemoveResult::Done {
                                    node: children.pop().map(|c| (*c).clone()),
                                    removed_value,
                                }
                            } else {
                                RemoveResult::Done {
                                    node: Some(Self::Internal { entries, children }),
                                    removed_value,
                                }
                            }
                        }
                        RemoveResult::Underflow {
                            node: underflowed,
                            removed_value,
                        } => {
                            children[index] = ReferenceCounter::new(underflowed);
                            Self::handle_underflow(entries, children, index, removed_value)
                        }
                    }
                }
            },
        }
    }

    fn make_remove_result(node: Option<Self>, removed_value: V) -> RemoveResult<K, V> {
        match &node {
            Some(n) if n.entry_count() < MIN_KEYS => RemoveResult::Underflow {
                node: node.unwrap(),
                removed_value,
            },
            None | Some(_) => RemoveResult::Done {
                node,
                removed_value,
            },
        }
    }

    fn remove_max_from_child(
        child_ref: &mut ReferenceCounter<Self>,
    ) -> ((K, V), RemoveMaxResult<K, V>) {
        // remove_max always modifies the node, so we always need to replace it.
        let (child, _took_ownership) = Self::take_or_clone_child_ref(child_ref);
        match child {
            Self::Leaf { mut entries } => {
                let (max_key, max_value) = entries.pop().unwrap();
                if entries.is_empty() {
                    ((max_key, max_value), RemoveMaxResult::Empty)
                } else if entries.len() < MIN_KEYS {
                    let node = Self::Leaf { entries };
                    *child_ref = ReferenceCounter::new(node.clone());
                    ((max_key, max_value), RemoveMaxResult::Underflow(node))
                } else {
                    *child_ref = ReferenceCounter::new(Self::Leaf { entries });
                    ((max_key, max_value), RemoveMaxResult::Done)
                }
            }
            Self::Internal {
                entries,
                mut children,
            } => {
                let last_child_index = children.len() - 1;
                let (replacement, result) =
                    Self::remove_max_from_child(children.last_mut().unwrap());

                match result {
                    RemoveMaxResult::Done => {
                        *child_ref = ReferenceCounter::new(Self::Internal { entries, children });
                        (replacement, RemoveMaxResult::Done)
                    }
                    RemoveMaxResult::Empty => {
                        unreachable!("Empty should not occur for non-root nodes")
                    }
                    RemoveMaxResult::Underflow(underflowed_node) => {
                        children[last_child_index] = ReferenceCounter::new(underflowed_node);
                        let result = Self::handle_underflow_for_remove_max(
                            entries,
                            children,
                            last_child_index,
                        );
                        match result {
                            UnderflowHandleResult::Done(n) => {
                                *child_ref = ReferenceCounter::new(n);
                                (replacement, RemoveMaxResult::Done)
                            }
                            UnderflowHandleResult::Empty => (replacement, RemoveMaxResult::Empty),
                            UnderflowHandleResult::Underflow(node) => {
                                *child_ref = ReferenceCounter::new(node.clone());
                                (replacement, RemoveMaxResult::Underflow(node))
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_underflow_for_remove_max(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
    ) -> UnderflowHandleResult<K, V> {
        if index > 0 && children[index - 1].entry_count() > MIN_KEYS {
            return Self::borrow_from_left_no_value(entries, children, index);
        }
        if index < children.len() - 1 && children[index + 1].entry_count() > MIN_KEYS {
            return Self::borrow_from_right_no_value(entries, children, index);
        }
        if index > 0 {
            Self::merge_with_left_no_value(entries, children, index)
        } else {
            Self::merge_with_right_no_value(entries, children, index)
        }
    }

    fn borrow_from_left_no_value(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
    ) -> UnderflowHandleResult<K, V> {
        let node = Self::borrow_from_left_core(entries, children, index);
        Self::make_underflow_result(Some(node))
    }

    fn borrow_from_right_no_value(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
    ) -> UnderflowHandleResult<K, V> {
        let node = Self::borrow_from_right_core(entries, children, index);
        Self::make_underflow_result(Some(node))
    }

    fn merge_with_left_no_value(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
    ) -> UnderflowHandleResult<K, V> {
        Self::finalize_merge_core(Self::merge_with_left_core(entries, children, index))
    }

    fn merge_with_right_no_value(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
    ) -> UnderflowHandleResult<K, V> {
        Self::finalize_merge_core(Self::merge_with_right_core(entries, children, index))
    }

    fn finalize_merge_core(
        (entries, children): (EntryArray<K, V>, ChildArray<K, V>),
    ) -> UnderflowHandleResult<K, V> {
        if entries.is_empty() {
            children
                .into_iter()
                .next()
                .map_or(UnderflowHandleResult::Empty, |child| {
                    Self::make_underflow_result(Some((*child).clone()))
                })
        } else {
            Self::make_underflow_result(Some(Self::Internal { entries, children }))
        }
    }

    fn make_underflow_result(node: Option<Self>) -> UnderflowHandleResult<K, V> {
        match node {
            Some(n) if n.entry_count() < MIN_KEYS => UnderflowHandleResult::Underflow(n),
            Some(n) => UnderflowHandleResult::Done(n),
            None => UnderflowHandleResult::Empty,
        }
    }

    fn borrow_from_left_core(
        mut entries: EntryArray<K, V>,
        mut children: ChildArray<K, V>,
        index: usize,
    ) -> Self {
        // borrow operations always modify both nodes
        let (left, _) = Self::take_or_clone_child(&mut children, index - 1);
        let (current, _) = Self::take_or_clone_child(&mut children, index);

        match (left, current) {
            (
                Self::Leaf {
                    entries: mut left_entries,
                },
                Self::Leaf {
                    entries: mut current_entries,
                },
            ) => {
                let borrowed = left_entries.pop().unwrap();
                let separator = std::mem::replace(&mut entries[index - 1], borrowed);
                current_entries.insert(0, separator);

                children[index - 1] = ReferenceCounter::new(Self::Leaf {
                    entries: left_entries,
                });
                children[index] = ReferenceCounter::new(Self::Leaf {
                    entries: current_entries,
                });
                Self::Internal { entries, children }
            }
            (
                Self::Internal {
                    entries: mut left_entries,
                    children: mut left_children,
                },
                Self::Internal {
                    entries: mut current_entries,
                    children: mut current_children,
                },
            ) => {
                let borrowed_entry = left_entries.pop().unwrap();
                let borrowed_child = left_children.pop().unwrap();

                let separator = std::mem::replace(&mut entries[index - 1], borrowed_entry);
                current_entries.insert(0, separator);
                current_children.insert(0, borrowed_child);

                children[index - 1] = ReferenceCounter::new(Self::Internal {
                    entries: left_entries,
                    children: left_children,
                });
                children[index] = ReferenceCounter::new(Self::Internal {
                    entries: current_entries,
                    children: current_children,
                });
                Self::Internal { entries, children }
            }
            _ => unreachable!("Sibling nodes must be of the same type"),
        }
    }

    fn borrow_from_right_core(
        mut entries: EntryArray<K, V>,
        mut children: ChildArray<K, V>,
        index: usize,
    ) -> Self {
        // borrow operations always modify both nodes
        let (current, _) = Self::take_or_clone_child(&mut children, index);
        let (right, _) = Self::take_or_clone_child(&mut children, index + 1);

        match (current, right) {
            (
                Self::Leaf {
                    entries: mut current_entries,
                },
                Self::Leaf {
                    entries: mut right_entries,
                },
            ) => {
                let borrowed = right_entries.remove(0);
                let separator = std::mem::replace(&mut entries[index], borrowed);
                current_entries.push(separator);

                children[index] = ReferenceCounter::new(Self::Leaf {
                    entries: current_entries,
                });
                children[index + 1] = ReferenceCounter::new(Self::Leaf {
                    entries: right_entries,
                });
                Self::Internal { entries, children }
            }
            (
                Self::Internal {
                    entries: mut current_entries,
                    children: mut current_children,
                },
                Self::Internal {
                    entries: mut right_entries,
                    children: mut right_children,
                },
            ) => {
                let borrowed_entry = right_entries.remove(0);
                let borrowed_child = right_children.remove(0);

                let separator = std::mem::replace(&mut entries[index], borrowed_entry);
                current_entries.push(separator);
                current_children.push(borrowed_child);

                children[index] = ReferenceCounter::new(Self::Internal {
                    entries: current_entries,
                    children: current_children,
                });
                children[index + 1] = ReferenceCounter::new(Self::Internal {
                    entries: right_entries,
                    children: right_children,
                });
                Self::Internal { entries, children }
            }
            _ => unreachable!("Sibling nodes must be of the same type"),
        }
    }

    fn merge_with_left_core(
        mut entries: EntryArray<K, V>,
        mut children: ChildArray<K, V>,
        index: usize,
    ) -> (EntryArray<K, V>, ChildArray<K, V>) {
        // merge operations always modify both nodes
        let (left, _) = Self::take_or_clone_child(&mut children, index - 1);
        let (current, _) = Self::take_or_clone_child(&mut children, index);
        let separator = entries.remove(index - 1);
        let merged = Self::merge_nodes(left, separator, current);

        children.remove(index);
        children[index - 1] = ReferenceCounter::new(merged);
        (entries, children)
    }

    fn merge_with_right_core(
        mut entries: EntryArray<K, V>,
        mut children: ChildArray<K, V>,
        index: usize,
    ) -> (EntryArray<K, V>, ChildArray<K, V>) {
        // merge operations always modify both nodes
        let (current, _) = Self::take_or_clone_child(&mut children, index);
        let (right, _) = Self::take_or_clone_child(&mut children, index + 1);
        let separator = entries.remove(index);
        let merged = Self::merge_nodes(current, separator, right);

        children.remove(index + 1);
        children[index] = ReferenceCounter::new(merged);
        (entries, children)
    }

    fn rebalance_after_remove(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        if entries.is_empty() {
            RemoveResult::Done {
                node: children.into_iter().next().map(|c| (*c).clone()),
                removed_value,
            }
        } else {
            Self::make_remove_result(Some(Self::Internal { entries, children }), removed_value)
        }
    }

    fn handle_underflow(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        if index > 0 && children[index - 1].entry_count() > MIN_KEYS {
            return Self::borrow_from_left(entries, children, index, removed_value);
        }
        if index < children.len() - 1 && children[index + 1].entry_count() > MIN_KEYS {
            return Self::borrow_from_right(entries, children, index, removed_value);
        }
        if index > 0 {
            Self::merge_with_left(entries, children, index, removed_value)
        } else {
            Self::merge_with_right(entries, children, index, removed_value)
        }
    }

    fn borrow_from_left(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        let node = Self::borrow_from_left_core(entries, children, index);
        RemoveResult::Done {
            node: Some(node),
            removed_value,
        }
    }

    fn borrow_from_right(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        let node = Self::borrow_from_right_core(entries, children, index);
        RemoveResult::Done {
            node: Some(node),
            removed_value,
        }
    }

    fn merge_nodes(left: Self, separator: (K, V), right: Self) -> Self {
        match (left, right) {
            (
                Self::Leaf {
                    entries: mut left_entries,
                },
                Self::Leaf {
                    entries: right_entries,
                },
            ) => {
                left_entries.push(separator);
                left_entries.extend(right_entries);
                Self::Leaf {
                    entries: left_entries,
                }
            }
            (
                Self::Internal {
                    entries: mut left_entries,
                    children: mut left_children,
                },
                Self::Internal {
                    entries: right_entries,
                    children: right_children,
                },
            ) => {
                left_entries.push(separator);
                left_entries.extend(right_entries);
                left_children.extend(right_children);
                Self::Internal {
                    entries: left_entries,
                    children: left_children,
                }
            }
            _ => unreachable!("Sibling nodes must be of the same type"),
        }
    }

    fn finalize_merge(
        (entries, children): (EntryArray<K, V>, ChildArray<K, V>),
        removed_value: V,
    ) -> RemoveResult<K, V> {
        if entries.is_empty() {
            RemoveResult::Done {
                node: children.into_iter().next().map(|c| (*c).clone()),
                removed_value,
            }
        } else {
            Self::make_remove_result(Some(Self::Internal { entries, children }), removed_value)
        }
    }

    fn merge_with_left(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        Self::finalize_merge(
            Self::merge_with_left_core(entries, children, index),
            removed_value,
        )
    }

    fn merge_with_right(
        entries: EntryArray<K, V>,
        children: ChildArray<K, V>,
        index: usize,
        removed_value: V,
    ) -> RemoveResult<K, V> {
        Self::finalize_merge(
            Self::merge_with_right_core(entries, children, index),
            removed_value,
        )
    }
}

/// A persistent (immutable) ordered map based on B-Tree.
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
    root: Option<ReferenceCounter<BTreeNode<K, V>>>,
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
    /// Creates a map with a single entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::singleton(1, "one");
    /// assert_eq!(map.len(), 1);
    /// assert_eq!(map.get(&1), Some(&"one"));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(key: K, value: V) -> Self {
        Self {
            root: Some(ReferenceCounter::new(BTreeNode::new_leaf(key, value))),
            length: 1,
        }
    }

    /// Returns a reference to the value associated with the key.
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
    ///
    /// assert_eq!(map.get(&1), Some(&"one"));
    /// assert_eq!(map.get(&3), None);
    /// ```
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.root.as_ref().and_then(|node| node.get(key))
    }

    /// Returns `true` if the map contains the given key.
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
    /// let map = PersistentTreeMap::singleton(1, "one");
    /// assert!(map.contains_key(&1));
    /// assert!(!map.contains_key(&2));
    /// ```
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Inserts a key-value pair into the map, returning a new map.
    ///
    /// If the key already exists, its value is replaced.
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
    /// let map1 = PersistentTreeMap::new();
    /// let map2 = map1.insert(1, "one");
    /// let map3 = map2.insert(1, "ONE");
    ///
    /// assert!(map1.is_empty());
    /// assert_eq!(map2.get(&1), Some(&"one"));
    /// assert_eq!(map3.get(&1), Some(&"ONE"));
    /// ```
    #[must_use]
    pub fn insert(&self, key: K, value: V) -> Self {
        match &self.root {
            None => Self::singleton(key, value),
            Some(root) => {
                let root_node = (**root).clone();
                match root_node.insert(key, value) {
                    InsertResult::Done { node, added, .. } => Self {
                        root: Some(ReferenceCounter::new(node)),
                        length: if added { self.length + 1 } else { self.length },
                    },
                    InsertResult::Split {
                        left,
                        median,
                        right,
                        added,
                        ..
                    } => Self {
                        root: Some(ReferenceCounter::new(BTreeNode::Internal {
                            entries: smallvec![median],
                            children: vec![
                                ReferenceCounter::new(left),
                                ReferenceCounter::new(right),
                            ],
                        })),
                        length: if added { self.length + 1 } else { self.length },
                    },
                }
            }
        }
    }

    /// Removes a key from the map, returning a new map.
    ///
    /// If the key doesn't exist, returns a map equivalent to the original.
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
    /// let map1 = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    /// let map2 = map1.remove(&1);
    ///
    /// assert_eq!(map1.len(), 2);
    /// assert_eq!(map2.len(), 1);
    /// assert!(!map2.contains_key(&1));
    /// ```
    #[must_use]
    #[allow(clippy::option_if_let_else)]
    pub fn remove<Q>(&self, key: &Q) -> Self
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match &self.root {
            None => Self::new(),
            Some(root) => {
                let root_node = (**root).clone();
                match root_node.remove(key) {
                    RemoveResult::NotFound(_) => {
                        // Key was not found. Return a clone of self.
                        // The node returned by NotFound is discarded; it was created
                        // from cloned data anyway, so no structural sharing is broken.
                        self.clone()
                    }
                    RemoveResult::Done { node, .. } => Self {
                        root: node.map(ReferenceCounter::new),
                        length: self.length - 1,
                    },
                    RemoveResult::Underflow { node, .. } => Self {
                        root: Some(ReferenceCounter::new(node)),
                        length: self.length - 1,
                    },
                }
            }
        }
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
        self.root.as_ref().and_then(|node| node.min())
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
        self.root.as_ref().and_then(|node| node.max())
    }
}

/// An iterator over the entries of a `PersistentTreeMap`.
pub struct PersistentTreeMapIterator<'a, K, V> {
    stack: Vec<IterState<'a, K, V>>,
    remaining: usize,
}

enum IterState<'a, K, V> {
    Node {
        node: &'a BTreeNode<K, V>,
        entry_index: usize,
    },
}

impl<'a, K, V> PersistentTreeMapIterator<'a, K, V> {
    fn new(root: Option<&'a ReferenceCounter<BTreeNode<K, V>>>, length: usize) -> Self {
        let mut iter = Self {
            stack: Vec::new(),
            remaining: length,
        };
        if let Some(root) = root {
            iter.push_leftmost(root.as_ref());
        }
        iter
    }

    fn push_leftmost(&mut self, mut node: &'a BTreeNode<K, V>) {
        loop {
            match node {
                BTreeNode::Leaf { .. } => {
                    self.stack.push(IterState::Node {
                        node,
                        entry_index: 0,
                    });
                    break;
                }
                BTreeNode::Internal { children, .. } => {
                    self.stack.push(IterState::Node {
                        node,
                        entry_index: 0,
                    });
                    node = children[0].as_ref();
                }
            }
        }
    }
}

impl<'a, K: Ord, V> PersistentTreeMapIterator<'a, K, V> {
    /// Creates an iterator that starts at the first entry >= the given key.
    /// This is O(log N) instead of O(N) for seeking to a position.
    fn new_from_lower_bound<Q>(
        root: Option<&'a ReferenceCounter<BTreeNode<K, V>>>,
        key: &Q,
        inclusive: bool,
    ) -> Self
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut iter = Self {
            stack: Vec::new(),
            remaining: 0, // Will be inaccurate but not used for range iteration
        };
        if let Some(root) = root {
            iter.seek_to_lower_bound(root.as_ref(), key, inclusive);
        }
        iter
    }

    /// Seeks to the first entry that satisfies the lower bound condition.
    /// For inclusive: first entry >= key
    /// For exclusive: first entry > key
    fn seek_to_lower_bound<Q>(&mut self, mut node: &'a BTreeNode<K, V>, key: &Q, inclusive: bool)
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord + ?Sized,
    {
        loop {
            match node {
                BTreeNode::Leaf { entries } => {
                    // Binary search for the first entry that satisfies the condition
                    let index = entries
                        .binary_search_by(|(k, _)| {
                            let cmp = k.borrow().cmp(key);
                            if inclusive {
                                // For >= comparison, Less stays Less, Equal/Greater become Greater
                                if cmp == std::cmp::Ordering::Less {
                                    std::cmp::Ordering::Less
                                } else {
                                    std::cmp::Ordering::Greater
                                }
                            } else {
                                // For > comparison, Less/Equal stays Less, Greater becomes Greater
                                if cmp == std::cmp::Ordering::Greater {
                                    std::cmp::Ordering::Greater
                                } else {
                                    std::cmp::Ordering::Less
                                }
                            }
                        })
                        .unwrap_or_else(|index| index);

                    if index < entries.len() {
                        self.stack.push(IterState::Node {
                            node,
                            entry_index: index,
                        });
                    }
                    break;
                }
                BTreeNode::Internal { entries, children } => {
                    // Find the appropriate child to descend into
                    let search_result = entries.binary_search_by(|(k, _)| k.borrow().cmp(key));

                    match search_result {
                        Ok(index) => {
                            if inclusive {
                                // Key found and we want >=, so this entry is the answer
                                // Push with this entry index
                                self.stack.push(IterState::Node {
                                    node,
                                    entry_index: index,
                                });
                                // Since this is exact match and inclusive, we return this
                                break;
                            }
                            // Key found but we want >, so go to the right subtree
                            self.stack.push(IterState::Node {
                                node,
                                entry_index: index + 1,
                            });
                            if index + 1 < children.len() {
                                node = children[index + 1].as_ref();
                            } else {
                                break;
                            }
                        }
                        Err(index) => {
                            // Key not found, index is where it would be inserted
                            // Descend into children[index] but also remember entries[index] and beyond
                            self.stack.push(IterState::Node {
                                node,
                                entry_index: index,
                            });
                            node = children[index].as_ref();
                        }
                    }
                }
            }
        }
    }
}

impl<'a, K, V> Iterator for PersistentTreeMapIterator<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(state) = self.stack.pop() {
            match state {
                IterState::Node { node, entry_index } => match node {
                    BTreeNode::Leaf { entries } => {
                        if entry_index < entries.len() {
                            let (k, v) = &entries[entry_index];
                            if entry_index + 1 < entries.len() {
                                self.stack.push(IterState::Node {
                                    node,
                                    entry_index: entry_index + 1,
                                });
                            }
                            self.remaining = self.remaining.saturating_sub(1);
                            return Some((k, v));
                        }
                    }
                    BTreeNode::Internal { entries, children } => {
                        if entry_index < entries.len() {
                            let (k, v) = &entries[entry_index];
                            self.stack.push(IterState::Node {
                                node,
                                entry_index: entry_index + 1,
                            });
                            self.push_leftmost(children[entry_index + 1].as_ref());
                            self.remaining = self.remaining.saturating_sub(1);
                            return Some((k, v));
                        }
                    }
                },
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for PersistentTreeMapIterator<'_, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<K: Clone + Ord, V: Clone> PersistentTreeMap<K, V> {
    /// Returns an iterator over the entries in key order.
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
    /// let entries: Vec<_> = map.iter().collect();
    /// assert_eq!(entries, vec![(&1, &"one"), (&2, &"two"), (&3, &"three")]);
    /// ```
    #[must_use]
    pub fn iter(&self) -> PersistentTreeMapIterator<'_, K, V> {
        PersistentTreeMapIterator::new(self.root.as_ref(), self.length)
    }

    /// Returns an iterator over the keys in order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(3, "three")
    ///     .insert(1, "one");
    ///
    /// let keys: Vec<_> = map.keys().collect();
    /// assert_eq!(keys, vec![&1, &3]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over the values in key order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// let values: Vec<_> = map.values().collect();
    /// assert_eq!(values, vec![&"one", &"two"]);
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over the entries in key order.
    ///
    /// This is the same as `iter()` but with a different return type.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.iter()
    }
}

/// An iterator over a range of entries in a `PersistentTreeMap`.
pub struct PersistentTreeMapRangeIterator<'a, K, V> {
    inner: PersistentTreeMapIterator<'a, K, V>,
    end_bound: Bound<K>,
}

impl<'a, K: Clone + Ord, V> Iterator for PersistentTreeMapRangeIterator<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;
        let in_range = match &self.end_bound {
            Bound::Included(end) => item.0 <= end,
            Bound::Excluded(end) => item.0 < end,
            Bound::Unbounded => true,
        };
        if in_range { Some(item) } else { None }
    }
}

impl<K: Clone + Ord, V: Clone> PersistentTreeMap<K, V> {
    /// Returns an iterator over a range of entries.
    ///
    /// # Complexity
    ///
    /// O(log N + k) where k is the number of entries in the range.
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
    ///     .insert(4, "four");
    ///
    /// let range: Vec<_> = map.range(2..4).collect();
    /// assert_eq!(range, vec![(&2, &"two"), (&3, &"three")]);
    /// ```
    pub fn range<R, Q>(&self, range: R) -> PersistentTreeMapRangeIterator<'_, K, V>
    where
        R: RangeBounds<Q>,
        K: Borrow<Q>,
        Q: Ord + ?Sized + ToOwned<Owned = K>,
    {
        // Create an iterator that starts at the lower bound in O(log N)
        let inner = match range.start_bound() {
            Bound::Included(start) => {
                PersistentTreeMapIterator::new_from_lower_bound(self.root.as_ref(), start, true)
            }
            Bound::Excluded(start) => {
                PersistentTreeMapIterator::new_from_lower_bound(self.root.as_ref(), start, false)
            }
            Bound::Unbounded => PersistentTreeMapIterator::new(self.root.as_ref(), self.length),
        };

        let end_bound = match range.end_bound() {
            Bound::Included(end) => Bound::Included(end.to_owned()),
            Bound::Excluded(end) => Bound::Excluded(end.to_owned()),
            Bound::Unbounded => Bound::Unbounded,
        };

        PersistentTreeMapRangeIterator { inner, end_bound }
    }
}

/// State for the owning iterator's stack-based traversal.
enum IntoIterState<K, V> {
    /// A node with entries to process, starting from the given index.
    Node {
        entries: EntryArray<K, V>,
        children: Option<ChildArray<K, V>>,
        entry_index: usize,
    },
}

/// An owning iterator over the entries of a `PersistentTreeMap`.
///
/// This iterator consumes the map and returns owned key-value pairs.
/// It uses stack-based traversal for O(n) total complexity.
pub struct PersistentTreeMapIntoIterator<K, V> {
    stack: Vec<IntoIterState<K, V>>,
    remaining: usize,
}

impl<K: Clone, V: Clone> PersistentTreeMapIntoIterator<K, V> {
    fn new(root: Option<ReferenceCounter<BTreeNode<K, V>>>, length: usize) -> Self {
        let mut iter = Self {
            stack: Vec::new(),
            remaining: length,
        };
        if let Some(root) = root {
            iter.push_leftmost(root);
        }
        iter
    }

    fn push_leftmost(&mut self, node: ReferenceCounter<BTreeNode<K, V>>) {
        let mut current = node;
        loop {
            let node_owned = (*current).clone();
            match node_owned {
                BTreeNode::Leaf { entries } => {
                    self.stack.push(IntoIterState::Node {
                        entries,
                        children: None,
                        entry_index: 0,
                    });
                    break;
                }
                BTreeNode::Internal { entries, children } => {
                    let first_child = children[0].clone();
                    self.stack.push(IntoIterState::Node {
                        entries,
                        children: Some(children),
                        entry_index: 0,
                    });
                    current = first_child;
                }
            }
        }
    }
}

impl<K: Clone, V: Clone> Iterator for PersistentTreeMapIntoIterator<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(state) = self.stack.pop() {
            match state {
                IntoIterState::Node {
                    entries,
                    children,
                    entry_index,
                } => {
                    if entry_index < entries.len() {
                        // Get the current entry
                        let (key, value) = entries[entry_index].clone();

                        // Push back this node with incremented index
                        self.stack.push(IntoIterState::Node {
                            entries,
                            children: children.clone(),
                            entry_index: entry_index + 1,
                        });

                        // If this is an internal node, push the right child
                        if let Some(ref children_vec) = children
                            && entry_index + 1 < children_vec.len()
                        {
                            self.push_leftmost(children_vec[entry_index + 1].clone());
                        }

                        self.remaining -= 1;
                        return Some((key, value));
                    }
                    // No more entries in this node, continue to parent
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K: Clone, V: Clone> ExactSizeIterator for PersistentTreeMapIntoIterator<K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<K: Clone + Ord, V: Clone> IntoIterator for PersistentTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = PersistentTreeMapIntoIterator<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        PersistentTreeMapIntoIterator::new(self.root, self.length)
    }
}

impl<'a, K: Clone + Ord, V: Clone> IntoIterator for &'a PersistentTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = PersistentTreeMapIterator<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Clone + Ord, V: Clone> PersistentTreeMap<K, V> {
    /// Transforms all values using the given function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20);
    ///
    /// let doubled = map.map_values(|v| v * 2);
    /// assert_eq!(doubled.get(&1), Some(&20));
    /// assert_eq!(doubled.get(&2), Some(&40));
    /// ```
    pub fn map_values<U: Clone, F>(&self, mut f: F) -> PersistentTreeMap<K, U>
    where
        F: FnMut(&V) -> U,
    {
        self.iter().map(|(k, v)| (k.clone(), f(v))).collect()
    }

    /// Transforms all keys using the given function.
    ///
    /// # Note
    ///
    /// The function must preserve key ordering, otherwise the resulting
    /// map may have incorrect behavior.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// let negated = map.map_keys(|k| -k);
    /// assert_eq!(negated.get(&-1), Some(&"one"));
    /// ```
    pub fn map_keys<L: Clone + Ord, F>(&self, mut f: F) -> PersistentTreeMap<L, V>
    where
        F: FnMut(&K) -> L,
    {
        self.iter().map(|(k, v)| (f(k), v.clone())).collect()
    }

    /// Filters and transforms entries.
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
    /// let filtered = map.filter_map(|_k, v| if *v > 15 { Some(v * 2) } else { None });
    /// assert_eq!(filtered.len(), 2);
    /// assert_eq!(filtered.get(&2), Some(&40));
    /// ```
    pub fn filter_map<U: Clone, F>(&self, mut f: F) -> PersistentTreeMap<K, U>
    where
        F: FnMut(&K, &V) -> Option<U>,
    {
        self.iter()
            .filter_map(|(k, v)| f(k, v).map(|new_v| (k.clone(), new_v)))
            .collect()
    }

    /// Merges two maps, preferring values from `other` on key collision.
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
    ///
    /// let merged = map1.merge(&map2);
    /// assert_eq!(merged.get(&1), Some(&"one"));
    /// assert_eq!(merged.get(&2), Some(&"TWO"));
    /// assert_eq!(merged.get(&3), Some(&"three"));
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        other
            .iter()
            .fold(self.clone(), |map, (k, v)| map.insert(k.clone(), v.clone()))
    }

    /// Merges two maps using a custom function for key collisions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let map1 = PersistentTreeMap::new()
    ///     .insert(1, 10)
    ///     .insert(2, 20);
    /// let map2 = PersistentTreeMap::new()
    ///     .insert(2, 5)
    ///     .insert(3, 30);
    ///
    /// let merged = map1.merge_with(&map2, |_k, v1, v2| v1 + v2);
    /// assert_eq!(merged.get(&2), Some(&25));
    /// ```
    #[must_use]
    pub fn merge_with<F>(&self, other: &Self, mut f: F) -> Self
    where
        F: FnMut(&K, &V, &V) -> V,
    {
        other.iter().fold(self.clone(), |map, (k, v)| {
            let new_value = map
                .get(k)
                .map_or_else(|| v.clone(), |existing| f(k, existing, v));
            map.insert(k.clone(), new_value)
        })
    }

    /// Removes entries that satisfy the predicate.
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
    /// let filtered = map.delete_if(|_k, v| *v > 15);
    /// assert_eq!(filtered.len(), 1);
    /// assert!(filtered.contains_key(&1));
    /// ```
    #[must_use]
    pub fn delete_if<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&K, &V) -> bool,
    {
        self.iter()
            .filter(|(k, v)| !predicate(k, v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Keeps only entries that satisfy the predicate.
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
    /// let kept = map.keep_if(|_k, v| *v > 15);
    /// assert_eq!(kept.len(), 2);
    /// ```
    #[must_use]
    pub fn keep_if<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&K, &V) -> bool,
    {
        self.iter()
            .filter(|(k, v)| predicate(k, v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Partitions the map by a predicate.
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
    /// let (small, large) = map.partition(|_k, v| *v <= 15);
    /// assert_eq!(small.len(), 1);
    /// assert_eq!(large.len(), 2);
    /// ```
    pub fn partition<F>(&self, mut predicate: F) -> (Self, Self)
    where
        F: FnMut(&K, &V) -> bool,
    {
        let mut left = Self::new();
        let mut right = Self::new();
        for (k, v) in self {
            if predicate(k, v) {
                left = left.insert(k.clone(), v.clone());
            } else {
                right = right.insert(k.clone(), v.clone());
            }
        }
        (left, right)
    }
}

impl<K, V> Default for PersistentTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Ord, V: Clone> FromIterator<(K, V)> for PersistentTreeMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        iter.into_iter()
            .fold(Self::new(), |map, (k, v)| map.insert(k, v))
    }
}

impl<K: Clone + Ord + PartialEq, V: Clone + PartialEq> PartialEq for PersistentTreeMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<K: Clone + Ord + Eq, V: Clone + Eq> Eq for PersistentTreeMap<K, V> {}

impl<K: Clone + Ord + Hash, V: Clone + Hash> Hash for PersistentTreeMap<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.length.hash(state);
        for (k, v) in self {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl<K: Clone + Ord + fmt::Debug, V: Clone + fmt::Debug> fmt::Debug for PersistentTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Clone + Ord + fmt::Display, V: Clone + fmt::Display> fmt::Display
    for PersistentTreeMap<K, V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in self {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl<K, V> TypeConstructor for PersistentTreeMap<K, V> {
    type Inner = V;
    type WithType<B> = PersistentTreeMap<K, B>;
}

impl<K: Clone + Ord, V: Clone> Foldable for PersistentTreeMap<K, V> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, Self::Inner) -> B,
    {
        self.into_iter()
            .fold(init, |accumulator, (_, value)| function(accumulator, value))
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(Self::Inner, B) -> B,
    {
        let entries: Vec<_> = self.into_iter().collect();
        entries
            .into_iter()
            .rev()
            .fold(init, |accumulator, (_, value)| function(value, accumulator))
    }

    fn is_empty(&self) -> bool
    where
        Self: Clone,
    {
        self.length == 0
    }

    fn length(&self) -> usize
    where
        Self: Clone,
    {
        self.length
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::PersistentTreeMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<K, V> Serialize for PersistentTreeMap<K, V>
    where
        K: Clone + Ord + Serialize,
        V: Clone + Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(Some(self.length))?;
            for (k, v) in self {
                map.serialize_entry(k, v)?;
            }
            map.end()
        }
    }

    struct PersistentTreeMapVisitor<K, V> {
        key_marker: std::marker::PhantomData<K>,
        value_marker: std::marker::PhantomData<V>,
    }

    impl<K, V> PersistentTreeMapVisitor<K, V> {
        const fn new() -> Self {
            Self {
                key_marker: std::marker::PhantomData,
                value_marker: std::marker::PhantomData,
            }
        }
    }

    impl<'de, K, V> serde::de::Visitor<'de> for PersistentTreeMapVisitor<K, V>
    where
        K: Clone + Ord + Deserialize<'de>,
        V: Clone + Deserialize<'de>,
    {
        type Value = PersistentTreeMap<K, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut map = PersistentTreeMap::new();
            while let Some((key, value)) = access.next_entry()? {
                map = map.insert(key, value);
            }
            Ok(map)
        }
    }

    impl<'de, K, V> Deserialize<'de> for PersistentTreeMap<K, V>
    where
        K: Clone + Ord + Deserialize<'de>,
        V: Clone + Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_map(PersistentTreeMapVisitor::new())
        }
    }
}

#[cfg(feature = "rayon")]
mod rayon_impl {
    use super::PersistentTreeMap;
    use rayon::iter::{
        FromParallelIterator, IndexedParallelIterator, IntoParallelIterator, ParallelIterator,
    };

    /// A parallel iterator over entries of a `PersistentTreeMap`.
    pub struct PersistentTreeMapParallelIterator<K, V> {
        elements: Vec<(K, V)>,
    }

    impl<K: Send, V: Send> ParallelIterator for PersistentTreeMapParallelIterator<K, V> {
        type Item = (K, V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
        {
            self.elements.into_par_iter().drive_unindexed(consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.elements.len())
        }
    }

    impl<K: Send, V: Send> IndexedParallelIterator for PersistentTreeMapParallelIterator<K, V> {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            self.elements.into_par_iter().drive(consumer)
        }

        fn with_producer<CB: rayon::iter::plumbing::ProducerCallback<Self::Item>>(
            self,
            callback: CB,
        ) -> CB::Output {
            self.elements.into_par_iter().with_producer(callback)
        }
    }

    /// A parallel iterator over references to entries of a `PersistentTreeMap`.
    pub struct PersistentTreeMapParallelRefIterator<K, V> {
        elements: Vec<(K, V)>,
    }

    impl<K: Send + Sync, V: Send + Sync> ParallelIterator
        for PersistentTreeMapParallelRefIterator<K, V>
    {
        type Item = (K, V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
        {
            self.elements.into_par_iter().drive_unindexed(consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.elements.len())
        }
    }

    impl<K: Send + Sync, V: Send + Sync> IndexedParallelIterator
        for PersistentTreeMapParallelRefIterator<K, V>
    {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            self.elements.into_par_iter().drive(consumer)
        }

        fn with_producer<CB: rayon::iter::plumbing::ProducerCallback<Self::Item>>(
            self,
            callback: CB,
        ) -> CB::Output {
            self.elements.into_par_iter().with_producer(callback)
        }
    }

    impl<K: Clone + Ord + Send, V: Clone + Send> IntoParallelIterator for PersistentTreeMap<K, V> {
        type Item = (K, V);
        type Iter = PersistentTreeMapParallelIterator<K, V>;

        fn into_par_iter(self) -> Self::Iter {
            PersistentTreeMapParallelIterator {
                elements: self.into_iter().collect(),
            }
        }
    }

    impl<K: Clone + Ord + Send, V: Clone + Send> FromParallelIterator<(K, V)>
        for PersistentTreeMap<K, V>
    {
        fn from_par_iter<I>(par_iter: I) -> Self
        where
            I: IntoParallelIterator<Item = (K, V)>,
        {
            let vec: Vec<_> = par_iter.into_par_iter().collect();
            vec.into_iter().collect()
        }
    }

    impl<K: Clone + Ord + Send + Sync, V: Clone + Send + Sync> PersistentTreeMap<K, V> {
        /// Returns a parallel iterator over the entries.
        #[must_use]
        pub fn par_iter(&self) -> PersistentTreeMapParallelRefIterator<K, V> {
            PersistentTreeMapParallelRefIterator {
                elements: self.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            }
        }
    }
}

#[cfg(feature = "rayon")]
pub use rayon_impl::{PersistentTreeMapParallelIterator, PersistentTreeMapParallelRefIterator};

// =============================================================================
// TransientTreeMap Definition
// =============================================================================

/// A transient (mutable) version of [`PersistentTreeMap`] for batch construction.
///
/// `TransientTreeMap` provides mutable operations for efficient batch construction
/// of tree maps. Once construction is complete, it can be converted back to a
/// [`PersistentTreeMap`] using the [`persistent()`](TransientTreeMap::persistent) method.
///
/// # Design
///
/// Transient data structures use Copy-on-Write (COW) semantics via
/// `ReferenceCounter::try_unwrap()` to efficiently share structure with the
/// persistent version while allowing mutation.
///
/// # Thread Safety
///
/// `TransientTreeMap` is intentionally not `Send` or `Sync`. It is designed for
/// single-threaded batch construction. For thread-safe operations, convert back
/// to `PersistentTreeMap`.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::{PersistentTreeMap, TransientTreeMap};
///
/// // Build a map efficiently using transient operations
/// let mut transient = TransientTreeMap::new();
/// transient.insert(3, "three");
/// transient.insert(1, "one");
/// transient.insert(2, "two");
///
/// // Convert to persistent map
/// let persistent = transient.persistent();
/// assert_eq!(persistent.get(&1), Some(&"one"));
/// assert_eq!(persistent.len(), 3);
///
/// // Keys are in sorted order
/// let keys: Vec<&i32> = persistent.keys().collect();
/// assert_eq!(keys, vec![&1, &2, &3]);
/// ```
pub struct TransientTreeMap<K, V> {
    root: Option<ReferenceCounter<BTreeNode<K, V>>>,
    length: usize,
    /// Marker to ensure `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

// Static assertions to verify TransientTreeMap is not Send/Sync
static_assertions::assert_not_impl_any!(TransientTreeMap<i32, i32>: Send, Sync);
static_assertions::assert_not_impl_any!(TransientTreeMap<String, String>: Send, Sync);

// Arc feature verification: even with Arc, TransientTreeMap remains !Send/!Sync
#[cfg(feature = "arc")]
mod arc_send_sync_verification_transient_treemap {
    use super::TransientTreeMap;
    use std::sync::Arc;

    // Arc<T> where T: Send+Sync is Send+Sync, but TransientTreeMap should still be !Send/!Sync
    static_assertions::assert_not_impl_any!(TransientTreeMap<Arc<i32>, Arc<i32>>: Send, Sync);
    static_assertions::assert_not_impl_any!(TransientTreeMap<Arc<String>, Arc<String>>: Send, Sync);
}

// =============================================================================
// TransientTreeMap Implementation
// =============================================================================

impl<K, V> TransientTreeMap<K, V> {
    /// Returns the number of entries in the map.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// assert_eq!(transient.len(), 0);
    /// transient.insert(1, "one");
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
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// assert!(transient.is_empty());
    /// transient.insert(1, "one");
    /// assert!(!transient.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl<K: Clone + Ord, V: Clone> TransientTreeMap<K, V> {
    /// Creates a new empty `TransientTreeMap`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// assert!(transient.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: None,
            length: 0,
            _marker: PhantomData,
        }
    }

    /// Creates a `TransientTreeMap` from a `PersistentTreeMap`.
    ///
    /// This consumes the persistent map and allows efficient batch modifications.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::{PersistentTreeMap, TransientTreeMap};
    ///
    /// let persistent = PersistentTreeMap::new()
    ///     .insert(1, "one")
    ///     .insert(2, "two");
    ///
    /// let mut transient = TransientTreeMap::from_persistent(persistent);
    /// transient.insert(3, "three");
    /// assert_eq!(transient.len(), 3);
    /// ```
    #[must_use]
    pub fn from_persistent(map: PersistentTreeMap<K, V>) -> Self {
        Self {
            root: map.root,
            length: map.length,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but `Ord`
    /// on the borrowed form must match that for the key type.
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
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<String, i32> = TransientTreeMap::new();
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
        Q: Ord + ?Sized,
    {
        self.root.as_ref().and_then(|node| node.get(key))
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
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// transient.insert(1, "one");
    ///
    /// assert!(transient.contains_key(&1));
    /// assert!(!transient.contains_key(&2));
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
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// assert_eq!(transient.insert(1, "one"), None);
    /// assert_eq!(transient.insert(1, "ONE"), Some("one"));
    /// assert_eq!(transient.get(&1), Some(&"ONE"));
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match &mut self.root {
            None => {
                self.root = Some(ReferenceCounter::new(BTreeNode::new_leaf(key, value)));
                self.length = 1;
                None
            }
            Some(root) => {
                // Insert always modifies the tree, so we always need to replace the root.
                // The `took_ownership` flag is not needed here.
                let (root_node, _took_ownership) = Self::take_or_clone_root(root);
                match root_node.insert(key, value) {
                    InsertResult::Done {
                        node,
                        added,
                        old_value,
                    } => {
                        *root = ReferenceCounter::new(node);
                        if added {
                            self.length += 1;
                        }
                        old_value
                    }
                    InsertResult::Split {
                        left,
                        median,
                        right,
                        added,
                        old_value,
                    } => {
                        *root = ReferenceCounter::new(BTreeNode::Internal {
                            entries: smallvec![median],
                            children: vec![
                                ReferenceCounter::new(left),
                                ReferenceCounter::new(right),
                            ],
                        });
                        if added {
                            self.length += 1;
                        }
                        old_value
                    }
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
    /// O(log N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// transient.insert(1, "one");
    /// assert_eq!(transient.remove(&1), Some("one"));
    /// assert_eq!(transient.remove(&1), None);
    /// assert!(transient.is_empty());
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let root = self.root.as_mut()?;
        let (root_node, took_ownership) = Self::take_or_clone_root(root);
        match root_node.remove(key) {
            RemoveResult::NotFound(node) => {
                // Key was not found. Restore the root only if we took ownership.
                // If we cloned (took_ownership == false), the original reference is unchanged.
                if took_ownership {
                    *root = ReferenceCounter::new(node);
                }
                // If !took_ownership, the original root reference is still valid,
                // so we don't need to do anything.
                None
            }
            RemoveResult::Done {
                node,
                removed_value,
            } => {
                self.length -= 1;
                self.root = node.map(ReferenceCounter::new);
                Some(removed_value)
            }
            RemoveResult::Underflow {
                node,
                removed_value,
            } => {
                self.length -= 1;
                self.root = Some(ReferenceCounter::new(node));
                Some(removed_value)
            }
        }
    }

    /// Helper function to take ownership of a root node using COW semantics.
    ///
    /// Returns a tuple of (node, `took_ownership`) where:
    /// - `took_ownership = true`: The original reference was taken and needs to be replaced.
    /// - `took_ownership = false`: A clone was returned; the original reference is unchanged.
    ///
    /// This distinction is important for operations like `remove` where a `NotFound`
    /// result should not disturb the original structure when shared.
    fn take_or_clone_root(root: &mut ReferenceCounter<BTreeNode<K, V>>) -> (BTreeNode<K, V>, bool) {
        // Check if we're the only owner before attempting to take ownership.
        // If there are multiple owners, clone directly without creating a placeholder.
        if ReferenceCounter::strong_count(root) > 1 {
            return ((**root).clone(), false);
        }

        // We're the only owner, so we can take ownership.
        let placeholder = ReferenceCounter::new(BTreeNode::empty_placeholder());
        let reference = std::mem::replace(root, placeholder);

        // This should always succeed since we checked strong_count above.
        match ReferenceCounter::try_unwrap(reference) {
            Ok(node) => (node, true),
            Err(reference_counter) => {
                // Defensive: restore the reference if try_unwrap fails unexpectedly.
                let cloned = (*reference_counter).clone();
                *root = reference_counter;
                (cloned, false)
            }
        }
    }

    /// Converts this transient map into a persistent map.
    ///
    /// This consumes the `TransientTreeMap` and returns a `PersistentTreeMap`.
    /// The conversion is O(1) as it simply moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientTreeMap;
    ///
    /// let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
    /// transient.insert(1, "one");
    /// transient.insert(2, "two");
    /// let persistent = transient.persistent();
    /// assert_eq!(persistent.len(), 2);
    /// assert_eq!(persistent.get(&1), Some(&"one"));
    /// ```
    #[must_use]
    pub fn persistent(self) -> PersistentTreeMap<K, V> {
        PersistentTreeMap {
            root: self.root,
            length: self.length,
        }
    }
}

impl<K: Clone + Ord, V: Clone> Default for TransientTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Ord, V: Clone> FromIterator<(K, V)> for TransientTreeMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut transient = Self::new();
        for (key, value) in iter {
            transient.insert(key, value);
        }
        transient
    }
}

// =============================================================================
// PersistentTreeMap::transient() method
// =============================================================================

impl<K: Clone + Ord, V: Clone> PersistentTreeMap<K, V> {
    /// Converts this persistent map into a transient map.
    ///
    /// This consumes the `PersistentTreeMap` and returns a `TransientTreeMap`
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
    /// use lambars::persistent::PersistentTreeMap;
    ///
    /// let persistent: PersistentTreeMap<i32, &str> = [
    ///     (1, "one"),
    ///     (2, "two"),
    /// ].into_iter().collect();
    ///
    /// let mut transient = persistent.transient();
    /// transient.insert(3, "three");
    /// transient.remove(&1);
    ///
    /// let new_persistent = transient.persistent();
    /// assert_eq!(new_persistent.len(), 2);
    /// assert_eq!(new_persistent.get(&2), Some(&"two"));
    /// assert_eq!(new_persistent.get(&3), Some(&"three"));
    /// ```
    #[must_use]
    pub fn transient(self) -> TransientTreeMap<K, V> {
        TransientTreeMap::from_persistent(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let map: PersistentTreeMap<i32, &str> = PersistentTreeMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_singleton() {
        let map = PersistentTreeMap::singleton(1, "one");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn test_insert_and_get() {
        let map = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
        assert_eq!(map.get(&3), Some(&"three"));
        assert_eq!(map.get(&4), None);
    }

    #[test]
    fn test_insert_updates_existing() {
        let map = PersistentTreeMap::new().insert(1, "one").insert(1, "ONE");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1), Some(&"ONE"));
    }

    #[test]
    fn test_remove() {
        let map = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");
        let map2 = map.remove(&2);
        assert_eq!(map.len(), 3);
        assert_eq!(map2.len(), 2);
        assert!(map2.contains_key(&1));
        assert!(!map2.contains_key(&2));
        assert!(map2.contains_key(&3));
    }

    #[test]
    fn test_min_max() {
        let map = PersistentTreeMap::new()
            .insert(3, "three")
            .insert(1, "one")
            .insert(2, "two");
        assert_eq!(map.min(), Some((&1, &"one")));
        assert_eq!(map.max(), Some((&3, &"three")));
    }

    #[test]
    fn test_iter_ordered() {
        let map = PersistentTreeMap::new()
            .insert(3, "three")
            .insert(1, "one")
            .insert(2, "two");
        let keys: Vec<_> = map.keys().collect();
        assert_eq!(keys, vec![&1, &2, &3]);
    }

    #[test]
    fn test_large_insert() {
        let mut map = PersistentTreeMap::new();
        for i in 0..1000 {
            map = map.insert(i, i * 2);
        }
        assert_eq!(map.len(), 1000);
        for i in 0..1000 {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }
    }

    /// Copy-on-Write correctness test: verifies results remain correct after insert
    #[test]
    fn test_cow_insert_correctness() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "a")
            .insert(2, "b")
            .insert(3, "c");

        let map2 = map1.clone().insert(4, "d");
        let map3 = map1.clone().insert(2, "B");

        // map1 is unchanged
        assert_eq!(map1.len(), 3);
        assert_eq!(map1.get(&1), Some(&"a"));
        assert_eq!(map1.get(&2), Some(&"b"));
        assert_eq!(map1.get(&3), Some(&"c"));
        assert_eq!(map1.get(&4), None);

        // map2 contains the new entry
        assert_eq!(map2.len(), 4);
        assert_eq!(map2.get(&4), Some(&"d"));

        // map3 contains the updated entry
        assert_eq!(map3.len(), 3);
        assert_eq!(map3.get(&2), Some(&"B"));
    }

    /// Copy-on-Write correctness test: verifies results remain correct after remove
    #[test]
    fn test_cow_remove_correctness() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "a")
            .insert(2, "b")
            .insert(3, "c");

        let map2 = map1.clone().remove(&2);
        let map3 = map1.clone().remove(&99);

        // map1 is unchanged
        assert_eq!(map1.len(), 3);
        assert_eq!(map1.get(&2), Some(&"b"));

        // map2 does not contain the removed entry
        assert_eq!(map2.len(), 2);
        assert_eq!(map2.get(&2), None);
        assert_eq!(map2.get(&1), Some(&"a"));
        assert_eq!(map2.get(&3), Some(&"c"));

        // map3 is unchanged (removal of a non-existent key)
        assert_eq!(map3.len(), 3);
    }

    /// Structural sharing test: verifies the original map is not modified by operations after clone
    #[test]
    fn test_structural_sharing_after_clone() {
        let map1 = PersistentTreeMap::new().insert(1, "a").insert(2, "b");
        let map1_clone = map1.clone();
        let _map2 = map1.insert(3, "c");

        // map1_clone is unchanged
        assert_eq!(map1_clone.len(), 2);
        assert_eq!(map1_clone.get(&3), None);
        assert_eq!(map1_clone.get(&1), Some(&"a"));
        assert_eq!(map1_clone.get(&2), Some(&"b"));
    }

    /// Copy-on-Write correctness test with large data
    #[test]
    fn test_cow_large_data() {
        let mut map = PersistentTreeMap::new();
        for i in 0..500 {
            map = map.insert(i, i * 2);
        }

        // Clone then operate on the clone
        let map_clone = map.clone();
        let mut map2 = map;
        for i in 500..1000 {
            map2 = map2.insert(i, i * 2);
        }

        // map_clone still has 500 entries
        assert_eq!(map_clone.len(), 500);
        for i in 0..500 {
            assert_eq!(map_clone.get(&i), Some(&(i * 2)));
        }
        for i in 500..1000 {
            assert_eq!(map_clone.get(&i), None);
        }

        // map2 has 1000 entries
        assert_eq!(map2.len(), 1000);
        for i in 0..1000 {
            assert_eq!(map2.get(&i), Some(&(i * 2)));
        }
    }

    /// Copy-on-Write correctness test for remove with large data
    #[test]
    fn test_cow_remove_large_data() {
        let mut map = PersistentTreeMap::new();
        for i in 0..500 {
            map = map.insert(i, i * 2);
        }

        let map_clone = map.clone();
        let mut map2 = map;

        // Remove even keys
        for i in (0..500).step_by(2) {
            map2 = map2.remove(&i);
        }

        // map_clone still has 500 entries
        assert_eq!(map_clone.len(), 500);
        for i in 0..500 {
            assert_eq!(map_clone.get(&i), Some(&(i * 2)));
        }

        // map2 has 250 entries (odd keys only)
        assert_eq!(map2.len(), 250);
        for i in (0..500).step_by(2) {
            assert_eq!(map2.get(&i), None);
        }
        for i in (1..500).step_by(2) {
            assert_eq!(map2.get(&i), Some(&(i * 2)));
        }
    }

    /// Verify that remove on a non-existent key maintains structure sharing
    /// for `PersistentTreeMap`.
    #[test]
    fn test_persistent_remove_nonexistent_maintains_structure_sharing() {
        let map1 = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");

        // Clone the map to share structure
        let map2 = map1.clone();

        // Get a reference to the root before the remove operation
        let root_before = map1.root.as_ref().unwrap();

        // Remove a non-existent key
        let map3 = map2.remove(&99);

        // Verify data is preserved
        assert_eq!(map3.len(), 3);
        assert_eq!(map3.get(&1), Some(&"one"));
        assert_eq!(map3.get(&2), Some(&"two"));
        assert_eq!(map3.get(&3), Some(&"three"));

        // Verify structure sharing: the root should be the same because
        // map2.clone() shares the root with map1, and remove(&99) on map2
        // returns a clone of map2, which still shares the same root.
        // Note: PersistentTreeMap::remove returns self.clone() for NotFound,
        // so the new map shares structure with the original.
        let root_after = map3.root.as_ref().unwrap();
        assert!(
            ReferenceCounter::ptr_eq(root_before, root_after),
            "Structure sharing should be maintained when removing a non-existent key"
        );
    }
}

// =============================================================================
// TransientTreeMap Tests
// =============================================================================

#[cfg(test)]
mod transient_treemap_tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // new() Tests
    // =========================================================================

    #[rstest]
    fn test_transient_new_creates_empty() {
        let transient: TransientTreeMap<i32, String> = TransientTreeMap::new();
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    // =========================================================================
    // from_persistent() and persistent() Tests
    // =========================================================================

    #[rstest]
    fn test_transient_from_persistent_empty() {
        let persistent: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
        let transient = TransientTreeMap::from_persistent(persistent);
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_transient_from_persistent_with_data() {
        let persistent = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");
        let transient = TransientTreeMap::from_persistent(persistent);
        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get(&1), Some(&"one"));
        assert_eq!(transient.get(&2), Some(&"two"));
        assert_eq!(transient.get(&3), Some(&"three"));
    }

    #[rstest]
    fn test_transient_persistent_roundtrip() {
        let persistent1 = PersistentTreeMap::new()
            .insert(10, "ten")
            .insert(20, "twenty");
        let mut transient = TransientTreeMap::from_persistent(persistent1);
        transient.insert(30, "thirty");
        let persistent2 = transient.persistent();

        assert_eq!(persistent2.len(), 3);
        assert_eq!(persistent2.get(&10), Some(&"ten"));
        assert_eq!(persistent2.get(&20), Some(&"twenty"));
        assert_eq!(persistent2.get(&30), Some(&"thirty"));
    }

    #[rstest]
    fn test_persistent_treemap_transient_method() {
        let persistent = PersistentTreeMap::new().insert("a", 1).insert("b", 2);
        let mut transient = persistent.transient();
        transient.insert("c", 3);
        let new_persistent = transient.persistent();

        assert_eq!(new_persistent.len(), 3);
        assert_eq!(new_persistent.get(&"a"), Some(&1));
        assert_eq!(new_persistent.get(&"b"), Some(&2));
        assert_eq!(new_persistent.get(&"c"), Some(&3));
    }

    // =========================================================================
    // insert() Tests
    // =========================================================================

    #[rstest]
    fn test_transient_insert_new_key() {
        let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
        let old_value = transient.insert(1, "one");
        assert_eq!(old_value, None);
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get(&1), Some(&"one"));
    }

    #[rstest]
    fn test_transient_insert_replace_existing() {
        let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
        transient.insert(1, "one");
        let old_value = transient.insert(1, "ONE");
        assert_eq!(old_value, Some("one"));
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get(&1), Some(&"ONE"));
    }

    #[rstest]
    fn test_transient_insert_multiple() {
        let mut transient: TransientTreeMap<i32, i32> = TransientTreeMap::new();
        for i in 0..100 {
            transient.insert(i, i * 2);
        }
        assert_eq!(transient.len(), 100);
        for i in 0..100 {
            assert_eq!(transient.get(&i), Some(&(i * 2)));
        }
    }

    // =========================================================================
    // remove() Tests
    // =========================================================================

    #[rstest]
    fn test_transient_remove_existing() {
        let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
        transient.insert(1, "one");
        transient.insert(2, "two");
        let removed = transient.remove(&1);
        assert_eq!(removed, Some("one"));
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get(&1), None);
        assert_eq!(transient.get(&2), Some(&"two"));
    }

    #[rstest]
    fn test_transient_remove_nonexistent() {
        let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
        transient.insert(1, "one");
        transient.insert(2, "two");
        transient.insert(3, "three");

        let removed = transient.remove(&99);
        assert_eq!(removed, None);
        assert_eq!(transient.len(), 3);

        // Verify all existing data is preserved
        assert_eq!(transient.get(&1), Some(&"one"));
        assert_eq!(transient.get(&2), Some(&"two"));
        assert_eq!(transient.get(&3), Some(&"three"));
    }

    #[rstest]
    fn test_transient_remove_nonexistent_preserves_structure_sharing() {
        // Create a persistent map
        let persistent = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");

        // Clone the persistent map to share structure
        let persistent_clone = persistent.clone();

        // Convert the clone to transient
        let mut transient = persistent_clone.transient();

        // Get a reference to the root before the remove operation
        // (both persistent and transient should share the same root initially)
        let root_before = persistent.root.as_ref().unwrap();

        // Remove a nonexistent key - this should not break structure sharing
        let removed = transient.remove(&99);
        assert_eq!(removed, None);

        // Verify all existing data is preserved
        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get(&1), Some(&"one"));
        assert_eq!(transient.get(&2), Some(&"two"));
        assert_eq!(transient.get(&3), Some(&"three"));

        // Convert back to persistent and verify
        let new_persistent = transient.persistent();
        assert_eq!(new_persistent.len(), 3);
        assert_eq!(new_persistent.get(&1), Some(&"one"));
        assert_eq!(new_persistent.get(&2), Some(&"two"));
        assert_eq!(new_persistent.get(&3), Some(&"three"));

        // Verify structure sharing is maintained:
        // The new_persistent's root should be the same as the original's root
        // because nothing was changed.
        let root_after = new_persistent.root.as_ref().unwrap();
        assert!(
            ReferenceCounter::ptr_eq(root_before, root_after),
            "Structure sharing should be maintained when removing a nonexistent key"
        );
    }

    #[rstest]
    fn test_transient_remove_all() {
        let mut transient: TransientTreeMap<i32, i32> = TransientTreeMap::new();
        for i in 0..10 {
            transient.insert(i, i * 2);
        }
        for i in 0..10 {
            transient.remove(&i);
        }
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    // =========================================================================
    // get() and contains_key() Tests
    // =========================================================================

    #[rstest]
    fn test_transient_get_existing() {
        let mut transient: TransientTreeMap<String, i32> = TransientTreeMap::new();
        transient.insert("key".to_string(), 42);
        assert_eq!(transient.get("key"), Some(&42));
    }

    #[rstest]
    fn test_transient_get_nonexistent() {
        let transient: TransientTreeMap<String, i32> = TransientTreeMap::new();
        assert_eq!(transient.get("key"), None);
    }

    #[rstest]
    fn test_transient_contains_key() {
        let mut transient: TransientTreeMap<i32, &str> = TransientTreeMap::new();
        transient.insert(1, "one");
        assert!(transient.contains_key(&1));
        assert!(!transient.contains_key(&2));
    }

    // =========================================================================
    // COW (Copy-on-Write) Tests
    // =========================================================================

    #[rstest]
    fn test_transient_cow_original_unchanged() {
        let persistent = PersistentTreeMap::new().insert(1, "one").insert(2, "two");
        let persistent_clone = persistent.clone();

        let mut transient = TransientTreeMap::from_persistent(persistent);
        transient.insert(3, "three");
        transient.remove(&1);

        // The original (cloned) persistent map should be unchanged
        assert_eq!(persistent_clone.len(), 2);
        assert_eq!(persistent_clone.get(&1), Some(&"one"));
        assert_eq!(persistent_clone.get(&2), Some(&"two"));
        assert_eq!(persistent_clone.get(&3), None);
    }

    #[rstest]
    fn test_transient_cow_batch_operations() {
        let persistent = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let original = persistent.clone();

        let mut transient = TransientTreeMap::from_persistent(persistent);

        // Batch operations
        for i in 4..=100 {
            transient.insert(i, i * 10);
        }
        for i in (1..=50).step_by(2) {
            transient.remove(&i);
        }

        let new_persistent = transient.persistent();

        // Original should be unchanged
        assert_eq!(original.len(), 3);
        assert_eq!(original.get(&1), Some(&10));
        assert_eq!(original.get(&50), None);

        // New map should have the batch changes
        assert_eq!(new_persistent.get(&1), None); // Removed
        assert_eq!(new_persistent.get(&2), Some(&20)); // Kept
        assert_eq!(new_persistent.get(&100), Some(&1000)); // Added
    }

    // =========================================================================
    // Large Data Tests
    // =========================================================================

    #[rstest]
    fn test_transient_large_batch_insert() {
        let mut transient: TransientTreeMap<i32, i32> = TransientTreeMap::new();
        for i in 0..1000 {
            transient.insert(i, i * 2);
        }
        let persistent = transient.persistent();

        assert_eq!(persistent.len(), 1000);
        for i in 0..1000 {
            assert_eq!(persistent.get(&i), Some(&(i * 2)));
        }
    }

    #[rstest]
    fn test_transient_large_batch_mixed_operations() {
        let initial: PersistentTreeMap<i32, i32> = (0..500).map(|i| (i, i)).collect();
        let mut transient = TransientTreeMap::from_persistent(initial);

        // Add new entries
        for i in 500..1000 {
            transient.insert(i, i);
        }

        // Update some existing entries
        for i in (0..500).step_by(10) {
            transient.insert(i, i * 100);
        }

        // Remove some entries
        for i in (0..500).step_by(5) {
            transient.remove(&i);
        }

        let result = transient.persistent();

        // Verify removals (every 5th from 0..500)
        for i in (0..500).step_by(5) {
            assert_eq!(result.get(&i), None);
        }

        // Verify additions (500..1000)
        for i in 500..1000 {
            assert_eq!(result.get(&i), Some(&i));
        }
    }
}
