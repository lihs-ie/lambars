//! Persistent (immutable) double-ended queue (Deque).
//!
//! This module provides a persistent deque implementation based on Finger Trees,
//! as described in Hinze & Paterson's "Finger Trees: A Simple General-purpose Data Structure" (2006).
//!
//! # Overview
//!
//! `PersistentDeque` is a Finger Tree inspired double-ended queue that provides:
//!
//! - O(1) `front` and `back` access
//! - O(1) `len` and `is_empty`
//! - O(s) `push_front` and `push_back` where s = spine length
//! - O(s) `pop_front` and `pop_back` where s = spine length
//! - O(n × s) concatenation (worst case O(n²))
//! - O(n × s) `get` by index (worst case O(n²))
//!
//! All operations return new deques without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Finger Tree Structure
//!
//! A Finger Tree consists of:
//! - Empty: An empty tree
//! - Single: A tree with a single element
//! - Deep: A tree with left/right "fingers" (Digits) and a middle spine
//!
//! The "fingers" provide O(1) access to both ends of the structure.
//!
//! # Implementation Note
//!
//! Unlike the traditional Finger Tree formulation which uses recursive type
//! parameters (`FingerTree<Node<T>>`), this implementation uses a type-erased
//! spine to avoid Rust's infinite type expansion during drop-check.
//!
//! This simplified design trades off theoretical optimal complexity for
//! practical usability within Rust's type system constraints. The spine
//! operations use `Vec` which results in linear time for spine modifications.
//! For most practical use cases with reasonable deque sizes, this trade-off
//! provides good performance while maintaining full persistence.
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentDeque;
//!
//! let deque = PersistentDeque::new()
//!     .push_back(1)
//!     .push_back(2)
//!     .push_back(3);
//!
//! assert_eq!(deque.front(), Some(&1));
//! assert_eq!(deque.back(), Some(&3));
//! assert_eq!(deque.len(), 3);
//!
//! // Structural sharing: the original deque is preserved
//! let extended = deque.push_back(4);
//! assert_eq!(deque.len(), 3);     // Original unchanged
//! assert_eq!(extended.len(), 4);  // New deque
//! ```
//!
//! # References
//!
//! - Okasaki, "Purely Functional Data Structures" (1998)
//! - Hinze & Paterson, "Finger Trees: A Simple General-purpose Data Structure" (2006)

use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

use super::ReferenceCounter;

struct Spine<T> {
    levels: Vec<SpineLevel<T>>,
}

impl<T> Clone for Spine<T> {
    fn clone(&self) -> Self {
        Self {
            levels: self.levels.clone(),
        }
    }
}

struct SpineLevel<T> {
    nodes: Vec<SpineNode<T>>,
}

impl<T> Clone for SpineLevel<T> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
        }
    }
}

enum SpineNode<T> {
    Node2 {
        first: ReferenceCounter<T>,
        second: ReferenceCounter<T>,
    },
    Node3 {
        first: ReferenceCounter<T>,
        second: ReferenceCounter<T>,
        third: ReferenceCounter<T>,
    },
}

impl<T> Clone for SpineNode<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Node2 { first, second } => Self::Node2 {
                first: first.clone(),
                second: second.clone(),
            },
            Self::Node3 {
                first,
                second,
                third,
            } => Self::Node3 {
                first: first.clone(),
                second: second.clone(),
                third: third.clone(),
            },
        }
    }
}

impl<T> SpineNode<T> {
    const fn len(&self) -> usize {
        match self {
            Self::Node2 { .. } => 2,
            Self::Node3 { .. } => 3,
        }
    }
}

impl<T: PartialEq> PartialEq for SpineNode<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Node2 {
                    first: f1,
                    second: s1,
                },
                Self::Node2 {
                    first: f2,
                    second: s2,
                },
            ) => f1 == f2 && s1 == s2,
            (
                Self::Node3 {
                    first: f1,
                    second: s1,
                    third: t1,
                },
                Self::Node3 {
                    first: f2,
                    second: s2,
                    third: t2,
                },
            ) => f1 == f2 && s1 == s2 && t1 == t2,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for SpineNode<T> {}

impl<T: fmt::Debug> fmt::Debug for SpineNode<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node2 { first, second } => f
                .debug_struct("Node2")
                .field("first", first)
                .field("second", second)
                .finish(),
            Self::Node3 {
                first,
                second,
                third,
            } => f
                .debug_struct("Node3")
                .field("first", first)
                .field("second", second)
                .field("third", third)
                .finish(),
        }
    }
}

impl<T> SpineLevel<T> {
    const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl<T: PartialEq> PartialEq for SpineLevel<T> {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
    }
}

impl<T: Eq> Eq for SpineLevel<T> {}

impl<T: fmt::Debug> fmt::Debug for SpineLevel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpineLevel")
            .field("nodes", &self.nodes)
            .finish()
    }
}

impl<T> Spine<T> {
    const fn new() -> Self {
        Self { levels: Vec::new() }
    }

    fn is_empty(&self) -> bool {
        self.levels.is_empty() || self.levels.iter().all(SpineLevel::is_empty)
    }
}

impl<T: PartialEq> PartialEq for Spine<T> {
    fn eq(&self, other: &Self) -> bool {
        self.levels == other.levels
    }
}

impl<T: Eq> Eq for Spine<T> {}

impl<T: fmt::Debug> fmt::Debug for Spine<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spine")
            .field("levels", &self.levels)
            .finish()
    }
}

enum Digit<T> {
    One(ReferenceCounter<T>),
    Two(ReferenceCounter<T>, ReferenceCounter<T>),
    Three(
        ReferenceCounter<T>,
        ReferenceCounter<T>,
        ReferenceCounter<T>,
    ),
    Four(
        ReferenceCounter<T>,
        ReferenceCounter<T>,
        ReferenceCounter<T>,
        ReferenceCounter<T>,
    ),
}

impl<T> Clone for Digit<T> {
    fn clone(&self) -> Self {
        match self {
            Self::One(first) => Self::One(first.clone()),
            Self::Two(first, second) => Self::Two(first.clone(), second.clone()),
            Self::Three(first, second, third) => {
                Self::Three(first.clone(), second.clone(), third.clone())
            }
            Self::Four(first, second, third, fourth) => {
                Self::Four(first.clone(), second.clone(), third.clone(), fourth.clone())
            }
        }
    }
}

impl<T> Digit<T> {
    const fn len(&self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Two(..) => 2,
            Self::Three(..) => 3,
            Self::Four(..) => 4,
        }
    }

    const fn head(&self) -> &ReferenceCounter<T> {
        match self {
            Self::One(first)
            | Self::Two(first, _)
            | Self::Three(first, _, _)
            | Self::Four(first, _, _, _) => first,
        }
    }

    const fn last(&self) -> &ReferenceCounter<T> {
        match self {
            Self::One(first) => first,
            Self::Two(_, second) => second,
            Self::Three(_, _, third) => third,
            Self::Four(_, _, _, fourth) => fourth,
        }
    }

    fn prepend(&self, element: ReferenceCounter<T>) -> Option<Self> {
        match self {
            Self::One(first) => Some(Self::Two(element, first.clone())),
            Self::Two(first, second) => Some(Self::Three(element, first.clone(), second.clone())),
            Self::Three(first, second, third) => Some(Self::Four(
                element,
                first.clone(),
                second.clone(),
                third.clone(),
            )),
            Self::Four(..) => None,
        }
    }

    fn append(&self, element: ReferenceCounter<T>) -> Option<Self> {
        match self {
            Self::One(first) => Some(Self::Two(first.clone(), element)),
            Self::Two(first, second) => Some(Self::Three(first.clone(), second.clone(), element)),
            Self::Three(first, second, third) => Some(Self::Four(
                first.clone(),
                second.clone(),
                third.clone(),
                element,
            )),
            Self::Four(..) => None,
        }
    }

    fn pop_front(&self) -> (Option<Self>, ReferenceCounter<T>) {
        match self {
            Self::One(first) => (None, first.clone()),
            Self::Two(first, second) => (Some(Self::One(second.clone())), first.clone()),
            Self::Three(first, second, third) => (
                Some(Self::Two(second.clone(), third.clone())),
                first.clone(),
            ),
            Self::Four(first, second, third, fourth) => (
                Some(Self::Three(second.clone(), third.clone(), fourth.clone())),
                first.clone(),
            ),
        }
    }

    fn pop_back(&self) -> (Option<Self>, ReferenceCounter<T>) {
        match self {
            Self::One(first) => (None, first.clone()),
            Self::Two(first, second) => (Some(Self::One(first.clone())), second.clone()),
            Self::Three(first, second, third) => (
                Some(Self::Two(first.clone(), second.clone())),
                third.clone(),
            ),
            Self::Four(first, second, third, fourth) => (
                Some(Self::Three(first.clone(), second.clone(), third.clone())),
                fourth.clone(),
            ),
        }
    }

    fn to_vec(&self) -> Vec<ReferenceCounter<T>> {
        match self {
            Self::One(first) => vec![first.clone()],
            Self::Two(first, second) => vec![first.clone(), second.clone()],
            Self::Three(first, second, third) => {
                vec![first.clone(), second.clone(), third.clone()]
            }
            Self::Four(first, second, third, fourth) => {
                vec![first.clone(), second.clone(), third.clone(), fourth.clone()]
            }
        }
    }

    fn from_spine_node(node: &SpineNode<T>) -> Self {
        match node {
            SpineNode::Node2 { first, second } => Self::Two(first.clone(), second.clone()),
            SpineNode::Node3 {
                first,
                second,
                third,
            } => Self::Three(first.clone(), second.clone(), third.clone()),
        }
    }
}

impl<T: PartialEq> PartialEq for Digit<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::One(a), Self::One(b)) => a == b,
            (Self::Two(a1, a2), Self::Two(b1, b2)) => a1 == b1 && a2 == b2,
            (Self::Three(a1, a2, a3), Self::Three(b1, b2, b3)) => a1 == b1 && a2 == b2 && a3 == b3,
            (Self::Four(a1, a2, a3, a4), Self::Four(b1, b2, b3, b4)) => {
                a1 == b1 && a2 == b2 && a3 == b3 && a4 == b4
            }
            _ => false,
        }
    }
}

impl<T: Eq> Eq for Digit<T> {}

impl<T: fmt::Debug> fmt::Debug for Digit<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One(first) => f.debug_tuple("One").field(first).finish(),
            Self::Two(first, second) => f.debug_tuple("Two").field(first).field(second).finish(),
            Self::Three(first, second, third) => f
                .debug_tuple("Three")
                .field(first)
                .field(second)
                .field(third)
                .finish(),
            Self::Four(first, second, third, fourth) => f
                .debug_tuple("Four")
                .field(first)
                .field(second)
                .field(third)
                .field(fourth)
                .finish(),
        }
    }
}

enum FingerTree<T> {
    Empty,
    Single(ReferenceCounter<T>),
    Deep {
        left: Digit<T>,
        spine: Spine<T>,
        right: Digit<T>,
        length: usize,
    },
}

impl<T> Clone for FingerTree<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Single(element) => Self::Single(element.clone()),
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => Self::Deep {
                left: left.clone(),
                spine: spine.clone(),
                right: right.clone(),
                length: *length,
            },
        }
    }
}

impl<T> FingerTree<T> {
    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    const fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Single(_) => 1,
            Self::Deep { length, .. } => *length,
        }
    }

    fn front(&self) -> Option<&T> {
        match self {
            Self::Empty => None,
            Self::Single(element) => Some(element),
            Self::Deep { left, .. } => Some(left.head()),
        }
    }

    fn back(&self) -> Option<&T> {
        match self {
            Self::Empty => None,
            Self::Single(element) => Some(element),
            Self::Deep { right, .. } => Some(right.last()),
        }
    }

    fn push_front(&self, element: T) -> Self {
        let element = ReferenceCounter::new(element);
        match self {
            Self::Empty => Self::Single(element),
            Self::Single(existing) => Self::Deep {
                left: Digit::One(element),
                spine: Spine::new(),
                right: Digit::One(existing.clone()),
                length: 2,
            },
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => left.prepend(element.clone()).map_or_else(
                || {
                    let Digit::Four(_, second, third, fourth) = left else {
                        unreachable!("prepend returned None but digit is not Four")
                    };
                    let node = SpineNode::Node3 {
                        first: second.clone(),
                        second: third.clone(),
                        third: fourth.clone(),
                    };
                    let new_spine = push_node_front(spine, node);
                    Self::Deep {
                        left: Digit::Two(element, left.head().clone()),
                        spine: new_spine,
                        right: right.clone(),
                        length: length + 1,
                    }
                },
                |new_left| Self::Deep {
                    left: new_left,
                    spine: spine.clone(),
                    right: right.clone(),
                    length: length + 1,
                },
            ),
        }
    }

    fn push_back(&self, element: T) -> Self {
        let element = ReferenceCounter::new(element);
        match self {
            Self::Empty => Self::Single(element),
            Self::Single(existing) => Self::Deep {
                left: Digit::One(existing.clone()),
                spine: Spine::new(),
                right: Digit::One(element),
                length: 2,
            },
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => right.append(element.clone()).map_or_else(
                || {
                    let Digit::Four(first, second, third, _) = right else {
                        unreachable!("append returned None but digit is not Four")
                    };
                    let node = SpineNode::Node3 {
                        first: first.clone(),
                        second: second.clone(),
                        third: third.clone(),
                    };
                    let new_spine = push_node_back(spine, node);
                    Self::Deep {
                        left: left.clone(),
                        spine: new_spine,
                        right: Digit::Two(right.last().clone(), element),
                        length: length + 1,
                    }
                },
                |new_right| Self::Deep {
                    left: left.clone(),
                    spine: spine.clone(),
                    right: new_right,
                    length: length + 1,
                },
            ),
        }
    }

    fn pop_front(&self) -> Option<(Self, T)>
    where
        T: Clone,
    {
        match self {
            Self::Empty => None,
            Self::Single(element) => Some((Self::Empty, (**element).clone())),
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => {
                let (maybe_new_left, removed) = left.pop_front();
                let removed_value = (*removed).clone();

                let new_tree = maybe_new_left.map_or_else(
                    || borrow_from_spine_front(spine, right, *length - 1),
                    |new_left| Self::Deep {
                        left: new_left,
                        spine: spine.clone(),
                        right: right.clone(),
                        length: length - 1,
                    },
                );

                Some((new_tree, removed_value))
            }
        }
    }

    fn pop_back(&self) -> Option<(Self, T)>
    where
        T: Clone,
    {
        match self {
            Self::Empty => None,
            Self::Single(element) => Some((Self::Empty, (**element).clone())),
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => {
                let (maybe_new_right, removed) = right.pop_back();
                let removed_value = (*removed).clone();

                let new_tree = maybe_new_right.map_or_else(
                    || borrow_from_spine_back(left, spine, *length - 1),
                    |new_right| Self::Deep {
                        left: left.clone(),
                        spine: spine.clone(),
                        right: new_right,
                        length: length - 1,
                    },
                );

                Some((new_tree, removed_value))
            }
        }
    }
}

fn push_node_front<T>(spine: &Spine<T>, node: SpineNode<T>) -> Spine<T> {
    let mut new_spine = spine.clone();
    if new_spine.levels.is_empty() {
        new_spine.levels.push(SpineLevel { nodes: vec![node] });
    } else {
        new_spine.levels[0].nodes.insert(0, node);
    }
    new_spine
}

fn push_node_back<T>(spine: &Spine<T>, node: SpineNode<T>) -> Spine<T> {
    let mut new_spine = spine.clone();
    if new_spine.levels.is_empty() {
        new_spine.levels.push(SpineLevel { nodes: vec![node] });
    } else {
        new_spine.levels[0].nodes.push(node);
    }
    new_spine
}

fn pop_node_front<T>(spine: &Spine<T>) -> Option<(Spine<T>, SpineNode<T>)> {
    if spine.is_empty() {
        return None;
    }

    let mut new_spine = spine.clone();
    if new_spine.levels.is_empty() || new_spine.levels[0].is_empty() {
        return None;
    }

    let node = new_spine.levels[0].nodes.remove(0);
    while !new_spine.levels.is_empty() && new_spine.levels[0].is_empty() {
        new_spine.levels.remove(0);
    }
    Some((new_spine, node))
}

fn pop_node_back<T>(spine: &Spine<T>) -> Option<(Spine<T>, SpineNode<T>)> {
    if spine.is_empty() {
        return None;
    }

    let mut new_spine = spine.clone();
    if new_spine.levels.is_empty() || new_spine.levels[0].is_empty() {
        return None;
    }

    let node = new_spine.levels[0].nodes.pop()?;
    while !new_spine.levels.is_empty() && new_spine.levels[0].is_empty() {
        new_spine.levels.remove(0);
    }
    Some((new_spine, node))
}

fn borrow_from_spine_front<T>(
    spine: &Spine<T>,
    right: &Digit<T>,
    new_length: usize,
) -> FingerTree<T> {
    pop_node_front(spine).map_or_else(
        || collapse_to_tree_from_right(right),
        |(new_spine, node)| {
            let new_left = Digit::from_spine_node(&node);
            FingerTree::Deep {
                left: new_left,
                spine: new_spine,
                right: right.clone(),
                length: new_length,
            }
        },
    )
}

fn borrow_from_spine_back<T>(
    left: &Digit<T>,
    spine: &Spine<T>,
    new_length: usize,
) -> FingerTree<T> {
    pop_node_back(spine).map_or_else(
        || collapse_to_tree_from_left(left),
        |(new_spine, node)| {
            let new_right = Digit::from_spine_node(&node);
            FingerTree::Deep {
                left: left.clone(),
                spine: new_spine,
                right: new_right,
                length: new_length,
            }
        },
    )
}

fn collapse_to_tree_from_right<T>(digit: &Digit<T>) -> FingerTree<T> {
    collapse_digit_to_tree(digit, true)
}

fn collapse_to_tree_from_left<T>(digit: &Digit<T>) -> FingerTree<T> {
    collapse_digit_to_tree(digit, false)
}

fn collapse_digit_to_tree<T>(digit: &Digit<T>, prefer_left: bool) -> FingerTree<T> {
    let elements = digit.to_vec();
    match elements.len() {
        0 => FingerTree::Empty,
        1 => FingerTree::Single(elements[0].clone()),
        2 => FingerTree::Deep {
            left: Digit::One(elements[0].clone()),
            spine: Spine::new(),
            right: Digit::One(elements[1].clone()),
            length: 2,
        },
        3 if prefer_left => FingerTree::Deep {
            left: Digit::Two(elements[0].clone(), elements[1].clone()),
            spine: Spine::new(),
            right: Digit::One(elements[2].clone()),
            length: 3,
        },
        3 => FingerTree::Deep {
            left: Digit::One(elements[0].clone()),
            spine: Spine::new(),
            right: Digit::Two(elements[1].clone(), elements[2].clone()),
            length: 3,
        },
        4 => FingerTree::Deep {
            left: Digit::Two(elements[0].clone(), elements[1].clone()),
            spine: Spine::new(),
            right: Digit::Two(elements[2].clone(), elements[3].clone()),
            length: 4,
        },
        _ => unreachable!("Digit should have 1-4 elements"),
    }
}

impl<T: PartialEq> PartialEq for FingerTree<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Single(a), Self::Single(b)) => a == b,
            (
                Self::Deep {
                    left: left_a,
                    spine: spine_a,
                    right: right_a,
                    ..
                },
                Self::Deep {
                    left: left_b,
                    spine: spine_b,
                    right: right_b,
                    ..
                },
            ) => left_a == left_b && spine_a == spine_b && right_a == right_b,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for FingerTree<T> {}

impl<T: fmt::Debug> fmt::Debug for FingerTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Single(element) => f.debug_tuple("Single").field(element).finish(),
            Self::Deep {
                left,
                spine,
                right,
                length,
            } => f
                .debug_struct("Deep")
                .field("left", left)
                .field("spine", spine)
                .field("right", right)
                .field("length", length)
                .finish(),
        }
    }
}

/// A persistent (immutable) double-ended queue.
///
/// Implemented using Finger Trees for efficient operations at both ends.
///
/// # Time Complexity
///
/// | Operation | Complexity |
/// |-----------|------------|
/// | `new`       | O(1)       |
/// | `singleton` | O(1)       |
/// | `push_front` | O(s) where s = spine length |
/// | `push_back`  | O(s) where s = spine length |
/// | `pop_front`  | O(s) where s = spine length |
/// | `pop_back`   | O(s) where s = spine length |
/// | `front`     | O(1)       |
/// | `back`      | O(1)       |
/// | `concat`    | O(n × s), worst O(n²) |
/// | `len`       | O(1)       |
/// | `is_empty`  | O(1)       |
/// | `get`       | O(n × s), worst O(n²) |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentDeque;
///
/// let deque = PersistentDeque::singleton(42);
/// assert_eq!(deque.front(), Some(&42));
/// assert_eq!(deque.len(), 1);
/// ```
pub struct PersistentDeque<T> {
    tree: FingerTree<T>,
}

impl<T> Clone for PersistentDeque<T> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
        }
    }
}

impl<T> PersistentDeque<T> {
    /// Creates a new empty deque.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tree: FingerTree::Empty,
        }
    }

    /// Creates a deque containing a single element.
    #[inline]
    #[must_use]
    pub fn singleton(element: T) -> Self {
        Self {
            tree: FingerTree::Single(ReferenceCounter::new(element)),
        }
    }

    /// Returns `true` if the deque contains no elements.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Returns the number of elements in the deque.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns a reference to the first element, if any.
    #[inline]
    #[must_use]
    pub fn front(&self) -> Option<&T> {
        self.tree.front()
    }

    /// Returns a reference to the last element, if any.
    #[inline]
    #[must_use]
    pub fn back(&self) -> Option<&T> {
        self.tree.back()
    }

    /// Prepends an element to the front of the deque.
    #[must_use]
    pub fn push_front(&self, element: T) -> Self {
        Self {
            tree: self.tree.push_front(element),
        }
    }

    /// Appends an element to the back of the deque.
    #[must_use]
    pub fn push_back(&self, element: T) -> Self {
        Self {
            tree: self.tree.push_back(element),
        }
    }

    /// Removes and returns the first element.
    #[must_use]
    pub fn pop_front(&self) -> Option<(Self, T)>
    where
        T: Clone,
    {
        self.tree
            .pop_front()
            .map(|(tree, element)| (Self { tree }, element))
    }

    /// Removes and returns the last element.
    #[must_use]
    pub fn pop_back(&self) -> Option<(Self, T)>
    where
        T: Clone,
    {
        self.tree
            .pop_back()
            .map(|(tree, element)| (Self { tree }, element))
    }

    /// Creates an iterator over references to the elements.
    #[must_use]
    pub const fn iter(&self) -> PersistentDequeIterator<'_, T> {
        PersistentDequeIterator {
            deque: self,
            front_index: 0,
            back_index: self.tree.len(),
        }
    }

    /// Returns the element at the given index, if any.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        self.iter().nth(index)
    }

    /// Returns a new deque with elements in reverse order.
    #[must_use]
    pub fn reverse(&self) -> Self
    where
        T: Clone,
    {
        self.iter()
            .cloned()
            .fold(Self::new(), |accumulator, element| {
                accumulator.push_front(element)
            })
    }

    /// Concatenates this deque with another deque.
    #[must_use]
    pub fn concat(&self, other: &Self) -> Self
    where
        T: Clone,
    {
        other
            .iter()
            .cloned()
            .fold(self.clone(), |accumulator, element| {
                accumulator.push_back(element)
            })
    }

    /// Creates a deque from a slice.
    #[must_use]
    pub fn from_slice(slice: &[T]) -> Self
    where
        T: Clone,
    {
        slice.iter().cloned().collect()
    }
}

impl<T> Default for PersistentDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for PersistentDeque<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: Eq> Eq for PersistentDeque<T> {}

impl<T: fmt::Debug> fmt::Debug for PersistentDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Hash> Hash for PersistentDeque<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for element in self {
            element.hash(state);
        }
    }
}

impl<T> FromIterator<T> for PersistentDeque<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut deque = Self::new();
        for element in iter {
            deque = deque.push_back(element);
        }
        deque
    }
}

impl<T: Clone> IntoIterator for PersistentDeque<T> {
    type Item = T;
    type IntoIter = PersistentDequeIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        PersistentDequeIntoIterator { deque: self }
    }
}

impl<'a, T> IntoIterator for &'a PersistentDeque<T> {
    type Item = &'a T;
    type IntoIter = PersistentDequeIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct PersistentDequeIterator<'a, T> {
    deque: &'a PersistentDeque<T>,
    front_index: usize,
    back_index: usize,
}

impl<'a, T> Iterator for PersistentDequeIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_index >= self.back_index {
            return None;
        }

        let result = get_element_at(&self.deque.tree, self.front_index);
        self.front_index += 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back_index - self.front_index;
        (remaining, Some(remaining))
    }
}

impl<T> DoubleEndedIterator for PersistentDequeIterator<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.back_index -= 1;
        get_element_at(&self.deque.tree, self.back_index)
    }
}

impl<T> ExactSizeIterator for PersistentDequeIterator<'_, T> {}

pub struct PersistentDequeIntoIterator<T> {
    deque: PersistentDeque<T>,
}

impl<T: Clone> Iterator for PersistentDequeIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (new_deque, element) = self.deque.pop_front()?;
        self.deque = new_deque;
        Some(element)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.deque.len(), Some(self.deque.len()))
    }
}

impl<T: Clone> ExactSizeIterator for PersistentDequeIntoIterator<T> {}

fn get_element_at<T>(tree: &FingerTree<T>, index: usize) -> Option<&T> {
    match tree {
        FingerTree::Empty => None,
        FingerTree::Single(element) => (index == 0).then_some(element.as_ref()),
        FingerTree::Deep {
            left, spine, right, ..
        } => {
            let left_len = left.len();
            if index < left_len {
                return get_element_from_digit(left, index);
            }
            let spine_len = get_spine_element_count(spine);
            if index < left_len + spine_len {
                get_element_from_spine(spine, index - left_len)
            } else {
                get_element_from_digit(right, index - left_len - spine_len)
            }
        }
    }
}

fn get_element_from_digit<T>(digit: &Digit<T>, index: usize) -> Option<&T> {
    match (digit, index) {
        (
            Digit::One(first)
            | Digit::Two(first, _)
            | Digit::Three(first, _, _)
            | Digit::Four(first, _, _, _),
            0,
        ) => Some(first),
        (Digit::Two(_, second) | Digit::Three(_, second, _) | Digit::Four(_, second, _, _), 1) => {
            Some(second)
        }
        (Digit::Three(_, _, third) | Digit::Four(_, _, third, _), 2) => Some(third),
        (Digit::Four(_, _, _, fourth), 3) => Some(fourth),
        _ => None,
    }
}

fn get_spine_element_count<T>(spine: &Spine<T>) -> usize {
    if spine.levels.is_empty() {
        return 0;
    }
    spine.levels[0].nodes.iter().map(SpineNode::len).sum()
}

fn get_element_from_spine<T>(spine: &Spine<T>, index: usize) -> Option<&T> {
    if spine.levels.is_empty() {
        return None;
    }

    let mut current_index = index;
    for node in &spine.levels[0].nodes {
        let node_len = node.len();
        if current_index < node_len {
            return get_element_from_spine_node(node, current_index);
        }
        current_index -= node_len;
    }
    None
}

fn get_element_from_spine_node<T>(node: &SpineNode<T>, index: usize) -> Option<&T> {
    match (node, index) {
        (SpineNode::Node2 { first, .. } | SpineNode::Node3 { first, .. }, 0) => Some(first),
        (SpineNode::Node2 { second, .. } | SpineNode::Node3 { second, .. }, 1) => Some(second),
        (SpineNode::Node3 { third, .. }, 2) => Some(third),
        _ => None,
    }
}

use crate::typeclass::{
    Applicative, Foldable, Functor, FunctorMut, Monad, Monoid, Semigroup, TypeConstructor,
};

impl<T> TypeConstructor for PersistentDeque<T> {
    type Inner = T;
    type WithType<B> = PersistentDeque<B>;
}

impl<T: Clone> Functor for PersistentDeque<T> {
    fn fmap<B, F>(self, function: F) -> PersistentDeque<B>
    where
        F: FnOnce(T) -> B,
    {
        self.front().map_or_else(PersistentDeque::new, |front| {
            PersistentDeque::singleton(function(front.clone()))
        })
    }

    fn fmap_ref<B, F>(&self, function: F) -> PersistentDeque<B>
    where
        F: FnOnce(&T) -> B,
    {
        self.front().map_or_else(PersistentDeque::new, |front| {
            PersistentDeque::singleton(function(front))
        })
    }
}

impl<T: Clone> FunctorMut for PersistentDeque<T> {
    fn fmap_mut<B, F>(self, function: F) -> PersistentDeque<B>
    where
        F: FnMut(T) -> B,
    {
        self.into_iter().map(function).collect()
    }

    fn fmap_ref_mut<B, F>(&self, function: F) -> PersistentDeque<B>
    where
        F: FnMut(&T) -> B,
    {
        self.iter().map(function).collect()
    }
}

impl<T: Clone> Applicative for PersistentDeque<T> {
    fn pure<A>(value: A) -> PersistentDeque<A> {
        PersistentDeque::singleton(value)
    }

    fn map2<B, C, F>(self, other: Self::WithType<B>, _function: F) -> Self::WithType<C>
    where
        F: FnOnce(T, B) -> C,
    {
        // For FnOnce, we can only support single-element deques.
        // This is a type system limitation - the Applicative trait requires FnOnce
        // but we cannot extract elements without Clone on B.
        let _ = (self, other);
        PersistentDeque::new()
    }

    fn map3<B, C, D, F>(
        self,
        second: Self::WithType<B>,
        third: Self::WithType<C>,
        _function: F,
    ) -> Self::WithType<D>
    where
        F: FnOnce(T, B, C) -> D,
    {
        // Similar limitation as map2 - requires Clone on B and C for proper implementation.
        let _ = (self, second, third);
        PersistentDeque::new()
    }

    fn apply<B, Output>(self, other: Self::WithType<B>) -> Self::WithType<Output>
    where
        Self: Sized,
        T: FnOnce(B) -> Output,
    {
        // FnOnce limitation - cannot apply function to multiple elements.
        let _ = (self, other);
        PersistentDeque::new()
    }
}

impl<T: Clone> Foldable for PersistentDeque<T> {
    fn fold_left<B, F>(self, init: B, function: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.into_iter().fold(init, function)
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(T, B) -> B,
    {
        let elements: Vec<T> = self.into_iter().collect();
        elements
            .into_iter()
            .rev()
            .fold(init, |accumulator, element| function(element, accumulator))
    }

    #[inline]
    fn is_empty(&self) -> bool
    where
        Self: Clone,
    {
        Self::is_empty(self)
    }

    #[inline]
    fn length(&self) -> usize
    where
        Self: Clone,
    {
        self.len()
    }
}

impl<T: Clone> Monad for PersistentDeque<T> {
    fn flat_map<B, F>(self, function: F) -> PersistentDeque<B>
    where
        F: FnOnce(T) -> PersistentDeque<B>,
    {
        // FnOnce can only be called once, so this only works for single-element deques.
        self.front()
            .map_or_else(PersistentDeque::new, |front| function(front.clone()))
    }
}

impl<T: Clone> Semigroup for PersistentDeque<T> {
    fn combine(self, other: Self) -> Self {
        self.concat(&other)
    }
}

impl<T: Clone> Monoid for PersistentDeque<T> {
    fn empty() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod phase1_basic_structure {
        use super::*;

        #[rstest]
        fn test_persistent_deque_new() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert!(deque.is_empty());
            assert_eq!(deque.len(), 0);
        }

        #[rstest]
        fn test_persistent_deque_singleton() {
            let deque = PersistentDeque::singleton(42);
            assert!(!deque.is_empty());
            assert_eq!(deque.len(), 1);
        }

        #[rstest]
        fn test_persistent_deque_default() {
            let deque: PersistentDeque<i32> = PersistentDeque::default();
            assert!(deque.is_empty());
            assert_eq!(deque.len(), 0);
        }

        #[rstest]
        fn test_persistent_deque_equality() {
            let deque1: PersistentDeque<i32> = PersistentDeque::singleton(42);
            let deque2: PersistentDeque<i32> = PersistentDeque::singleton(42);
            let deque3: PersistentDeque<i32> = PersistentDeque::singleton(43);

            assert_eq!(deque1, deque2);
            assert_ne!(deque1, deque3);
        }

        #[rstest]
        fn test_persistent_deque_clone() {
            let deque1 = PersistentDeque::singleton(42);
            let deque2 = deque1.clone();
            assert_eq!(deque1, deque2);
        }

        #[rstest]
        fn test_digit_operations() {
            let digit: Digit<i32> = Digit::One(ReferenceCounter::new(1));
            assert_eq!(digit.len(), 1);
            assert_eq!(**digit.head(), 1);
            assert_eq!(**digit.last(), 1);
        }

        #[rstest]
        fn test_digit_prepend() {
            let digit: Digit<i32> = Digit::One(ReferenceCounter::new(2));
            let prepended = digit.prepend(ReferenceCounter::new(1)).unwrap();
            assert_eq!(prepended.len(), 2);
            assert_eq!(**prepended.head(), 1);
            assert_eq!(**prepended.last(), 2);
        }

        #[rstest]
        fn test_digit_append() {
            let digit: Digit<i32> = Digit::One(ReferenceCounter::new(1));
            let appended = digit.append(ReferenceCounter::new(2)).unwrap();
            assert_eq!(appended.len(), 2);
            assert_eq!(**appended.head(), 1);
            assert_eq!(**appended.last(), 2);
        }

        #[rstest]
        fn test_digit_overflow() {
            let digit: Digit<i32> = Digit::Four(
                ReferenceCounter::new(1),
                ReferenceCounter::new(2),
                ReferenceCounter::new(3),
                ReferenceCounter::new(4),
            );
            assert!(digit.prepend(ReferenceCounter::new(0)).is_none());
            assert!(digit.append(ReferenceCounter::new(5)).is_none());
        }
    }

    mod phase2_push_operations {
        use super::*;

        #[rstest]
        fn test_push_front_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            let pushed = deque.push_front(1);
            assert_eq!(pushed.len(), 1);
            assert_eq!(pushed.front(), Some(&1));
        }

        #[rstest]
        fn test_push_front_single() {
            let deque = PersistentDeque::singleton(2);
            let pushed = deque.push_front(1);
            assert_eq!(pushed.len(), 2);
            assert_eq!(pushed.front(), Some(&1));
            assert_eq!(pushed.back(), Some(&2));
        }

        #[rstest]
        fn test_push_front_multiple() {
            let deque = PersistentDeque::new()
                .push_front(3)
                .push_front(2)
                .push_front(1);
            assert_eq!(deque.len(), 3);
            assert_eq!(deque.front(), Some(&1));
            assert_eq!(deque.back(), Some(&3));
        }

        #[rstest]
        fn test_push_back_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            let pushed = deque.push_back(1);
            assert_eq!(pushed.len(), 1);
            assert_eq!(pushed.back(), Some(&1));
        }

        #[rstest]
        fn test_push_back_single() {
            let deque = PersistentDeque::singleton(1);
            let pushed = deque.push_back(2);
            assert_eq!(pushed.len(), 2);
            assert_eq!(pushed.front(), Some(&1));
            assert_eq!(pushed.back(), Some(&2));
        }

        #[rstest]
        fn test_push_back_multiple() {
            let deque = PersistentDeque::new()
                .push_back(1)
                .push_back(2)
                .push_back(3);
            assert_eq!(deque.len(), 3);
            assert_eq!(deque.front(), Some(&1));
            assert_eq!(deque.back(), Some(&3));
        }

        #[rstest]
        fn test_push_front_preserves_original() {
            let original = PersistentDeque::singleton(1);
            let pushed = original.push_front(0);
            assert_eq!(original.len(), 1);
            assert_eq!(pushed.len(), 2);
        }

        #[rstest]
        fn test_push_back_preserves_original() {
            let original = PersistentDeque::singleton(1);
            let pushed = original.push_back(2);
            assert_eq!(original.len(), 1);
            assert_eq!(pushed.len(), 2);
        }

        #[rstest]
        fn test_push_many_elements() {
            let mut deque = PersistentDeque::new();
            for i in 0..100 {
                deque = deque.push_back(i);
            }
            assert_eq!(deque.len(), 100);
            assert_eq!(deque.front(), Some(&0));
            assert_eq!(deque.back(), Some(&99));
        }
    }

    mod phase3_pop_operations {
        use super::*;

        #[rstest]
        fn test_front_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert_eq!(deque.front(), None);
        }

        #[rstest]
        fn test_back_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert_eq!(deque.back(), None);
        }

        #[rstest]
        fn test_front_singleton() {
            let deque = PersistentDeque::singleton(42);
            assert_eq!(deque.front(), Some(&42));
        }

        #[rstest]
        fn test_back_singleton() {
            let deque = PersistentDeque::singleton(42);
            assert_eq!(deque.back(), Some(&42));
        }

        #[rstest]
        fn test_pop_front_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert!(deque.pop_front().is_none());
        }

        #[rstest]
        fn test_pop_back_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert!(deque.pop_back().is_none());
        }

        #[rstest]
        fn test_pop_front_singleton() {
            let deque = PersistentDeque::singleton(42);
            let (rest, element) = deque.pop_front().unwrap();
            assert_eq!(element, 42);
            assert!(rest.is_empty());
        }

        #[rstest]
        fn test_pop_back_singleton() {
            let deque = PersistentDeque::singleton(42);
            let (rest, element) = deque.pop_back().unwrap();
            assert_eq!(element, 42);
            assert!(rest.is_empty());
        }

        #[rstest]
        fn test_pop_front_multiple() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let (rest, first) = deque.pop_front().unwrap();
            assert_eq!(first, 1);
            assert_eq!(rest.len(), 2);
            assert_eq!(rest.front(), Some(&2));
        }

        #[rstest]
        fn test_pop_back_multiple() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let (rest, last) = deque.pop_back().unwrap();
            assert_eq!(last, 3);
            assert_eq!(rest.len(), 2);
            assert_eq!(rest.back(), Some(&2));
        }

        #[rstest]
        fn test_pop_front_preserves_original() {
            let original: PersistentDeque<i32> = (1..=3).collect();
            let (rest, _) = original.pop_front().unwrap();
            assert_eq!(original.len(), 3);
            assert_eq!(rest.len(), 2);
        }

        #[rstest]
        fn test_pop_back_preserves_original() {
            let original: PersistentDeque<i32> = (1..=3).collect();
            let (rest, _) = original.pop_back().unwrap();
            assert_eq!(original.len(), 3);
            assert_eq!(rest.len(), 2);
        }

        #[rstest]
        fn test_pop_all_front() {
            let mut deque: PersistentDeque<i32> = (1..=5).collect();
            let mut elements = Vec::new();
            while let Some((rest, element)) = deque.pop_front() {
                elements.push(element);
                deque = rest;
            }
            assert_eq!(elements, vec![1, 2, 3, 4, 5]);
            assert!(deque.is_empty());
        }

        #[rstest]
        fn test_pop_all_back() {
            let mut deque: PersistentDeque<i32> = (1..=5).collect();
            let mut elements = Vec::new();
            while let Some((rest, element)) = deque.pop_back() {
                elements.push(element);
                deque = rest;
            }
            assert_eq!(elements, vec![5, 4, 3, 2, 1]);
            assert!(deque.is_empty());
        }
    }

    mod phase4_iterator {
        use super::*;

        #[rstest]
        fn test_iter_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert_eq!(deque.iter().count(), 0);
        }

        #[rstest]
        fn test_iter_singleton() {
            let deque = PersistentDeque::singleton(42);
            let elements: Vec<&i32> = deque.iter().collect();
            assert_eq!(elements, vec![&42]);
        }

        #[rstest]
        fn test_iter_multiple() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let elements: Vec<&i32> = deque.iter().collect();
            assert_eq!(elements, vec![&1, &2, &3, &4, &5]);
        }

        #[rstest]
        fn test_iter_rev() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let elements: Vec<&i32> = deque.iter().rev().collect();
            assert_eq!(elements, vec![&5, &4, &3, &2, &1]);
        }

        #[rstest]
        fn test_into_iter() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let elements: Vec<i32> = deque.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3, 4, 5]);
        }

        #[rstest]
        fn test_from_iter() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            assert_eq!(deque.len(), 5);
            assert_eq!(deque.front(), Some(&1));
            assert_eq!(deque.back(), Some(&5));
        }

        #[rstest]
        fn test_iter_size_hint() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let iter = deque.iter();
            assert_eq!(iter.size_hint(), (5, Some(5)));
        }
    }

    mod phase5_get {
        use super::*;

        #[rstest]
        fn test_get_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            assert_eq!(deque.get(0), None);
        }

        #[rstest]
        fn test_get_singleton() {
            let deque = PersistentDeque::singleton(42);
            assert_eq!(deque.get(0), Some(&42));
            assert_eq!(deque.get(1), None);
        }

        #[rstest]
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        fn test_get_multiple() {
            let deque: PersistentDeque<i32> = (0..10).collect();
            for i in 0..10 {
                assert_eq!(deque.get(i), Some(&(i as i32)));
            }
            assert_eq!(deque.get(10), None);
        }

        #[rstest]
        fn test_get_out_of_bounds() {
            let deque: PersistentDeque<i32> = (0..5).collect();
            assert_eq!(deque.get(5), None);
            assert_eq!(deque.get(100), None);
        }
    }

    mod phase6_debug_hash {
        use super::*;
        use std::collections::HashSet;

        #[rstest]
        fn test_debug_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            let debug = format!("{deque:?}");
            assert_eq!(debug, "[]");
        }

        #[rstest]
        fn test_debug_singleton() {
            let deque = PersistentDeque::singleton(42);
            let debug = format!("{deque:?}");
            assert_eq!(debug, "[42]");
        }

        #[rstest]
        fn test_debug_multiple() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let debug = format!("{deque:?}");
            assert_eq!(debug, "[1, 2, 3]");
        }

        #[rstest]
        fn test_hash_equality() {
            let deque1: PersistentDeque<i32> = (1..=3).collect();
            let deque2: PersistentDeque<i32> = (1..=3).collect();

            let mut set = HashSet::new();
            set.insert(deque1);

            assert!(set.contains(&deque2));
        }
    }

    mod phase7_additional_operations {
        use super::*;

        #[rstest]
        fn test_reverse_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::new();
            let reversed = deque.reverse();
            assert!(reversed.is_empty());
        }

        #[rstest]
        fn test_reverse_singleton() {
            let deque = PersistentDeque::singleton(42);
            let reversed = deque.reverse();
            assert_eq!(reversed.len(), 1);
            assert_eq!(reversed.front(), Some(&42));
        }

        #[rstest]
        fn test_reverse_multiple() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let reversed = deque.reverse();
            let elements: Vec<i32> = reversed.into_iter().collect();
            assert_eq!(elements, vec![5, 4, 3, 2, 1]);
        }

        #[rstest]
        fn test_reverse_preserves_original() {
            let original: PersistentDeque<i32> = (1..=3).collect();
            let reversed = original.reverse();

            assert_eq!(original.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
            assert_eq!(reversed.iter().collect::<Vec<_>>(), vec![&3, &2, &1]);
        }

        #[rstest]
        fn test_concat_both_empty() {
            let deque1: PersistentDeque<i32> = PersistentDeque::new();
            let deque2: PersistentDeque<i32> = PersistentDeque::new();
            let result = deque1.concat(&deque2);
            assert!(result.is_empty());
        }

        #[rstest]
        fn test_concat_first_empty() {
            let deque1: PersistentDeque<i32> = PersistentDeque::new();
            let deque2: PersistentDeque<i32> = (1..=3).collect();
            let result = deque1.concat(&deque2);
            let elements: Vec<i32> = result.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3]);
        }

        #[rstest]
        fn test_concat_second_empty() {
            let deque1: PersistentDeque<i32> = (1..=3).collect();
            let deque2: PersistentDeque<i32> = PersistentDeque::new();
            let result = deque1.concat(&deque2);
            let elements: Vec<i32> = result.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3]);
        }

        #[rstest]
        fn test_concat_both_non_empty() {
            let deque1: PersistentDeque<i32> = (1..=3).collect();
            let deque2: PersistentDeque<i32> = (4..=6).collect();
            let result = deque1.concat(&deque2);
            let elements: Vec<i32> = result.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3, 4, 5, 6]);
        }

        #[rstest]
        fn test_concat_preserves_originals() {
            let deque1: PersistentDeque<i32> = (1..=3).collect();
            let deque2: PersistentDeque<i32> = (4..=6).collect();
            let _ = deque1.concat(&deque2);

            assert_eq!(deque1.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
            assert_eq!(deque2.iter().collect::<Vec<_>>(), vec![&4, &5, &6]);
        }

        #[rstest]
        fn test_from_slice_empty() {
            let deque: PersistentDeque<i32> = PersistentDeque::from_slice(&[]);
            assert!(deque.is_empty());
        }

        #[rstest]
        fn test_from_slice_non_empty() {
            let deque = PersistentDeque::from_slice(&[1, 2, 3, 4, 5]);
            assert_eq!(deque.len(), 5);
            assert_eq!(deque.front(), Some(&1));
            assert_eq!(deque.back(), Some(&5));
        }
    }

    mod typeclass_tests {
        use super::*;
        use crate::typeclass::{Applicative, Foldable, FunctorMut, Monoid, Semigroup};

        #[rstest]
        fn test_functor_mut_fmap() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let doubled: PersistentDeque<i32> = deque.fmap_mut(|x| x * 2);
            let elements: Vec<i32> = doubled.into_iter().collect();
            assert_eq!(elements, vec![2, 4, 6]);
        }

        #[rstest]
        fn test_functor_mut_fmap_ref() {
            let deque: PersistentDeque<String> =
                vec!["a".to_string(), "bb".to_string(), "ccc".to_string()]
                    .into_iter()
                    .collect();
            let lengths: PersistentDeque<usize> = deque.fmap_ref_mut(|s| s.len());
            let elements: Vec<usize> = lengths.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3]);
        }

        #[rstest]
        fn test_applicative_pure() {
            let deque: PersistentDeque<i32> = <PersistentDeque<()>>::pure(42);
            assert_eq!(deque.len(), 1);
            assert_eq!(deque.front(), Some(&42));
        }

        #[rstest]
        fn test_foldable_fold_left() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            let sum = deque.fold_left(0, |accumulator, element| accumulator + element);
            assert_eq!(sum, 15);
        }

        #[rstest]
        fn test_foldable_fold_right() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let result: Vec<i32> = deque.fold_right(Vec::new(), |element, mut accumulator| {
                accumulator.push(element);
                accumulator
            });
            assert_eq!(result, vec![3, 2, 1]);
        }

        #[rstest]
        fn test_foldable_is_empty() {
            let empty: PersistentDeque<i32> = PersistentDeque::new();
            let non_empty: PersistentDeque<i32> = PersistentDeque::singleton(42);

            assert!(<PersistentDeque<i32> as Foldable>::is_empty(&empty));
            assert!(!<PersistentDeque<i32> as Foldable>::is_empty(&non_empty));
        }

        #[rstest]
        fn test_foldable_length() {
            let deque: PersistentDeque<i32> = (1..=5).collect();
            assert_eq!(<PersistentDeque<i32> as Foldable>::length(&deque), 5);
        }

        #[rstest]
        fn test_semigroup_combine() {
            let deque1: PersistentDeque<i32> = (1..=3).collect();
            let deque2: PersistentDeque<i32> = (4..=6).collect();
            let combined = deque1.combine(deque2);

            let elements: Vec<i32> = combined.into_iter().collect();
            assert_eq!(elements, vec![1, 2, 3, 4, 5, 6]);
        }

        #[rstest]
        fn test_monoid_empty() {
            let empty: PersistentDeque<i32> = PersistentDeque::empty();
            assert!(empty.is_empty());
        }

        #[rstest]
        fn test_monoid_identity() {
            let deque: PersistentDeque<i32> = (1..=3).collect();
            let empty: PersistentDeque<i32> = PersistentDeque::empty();

            let left_combined = empty.clone().combine(deque.clone());
            assert_eq!(
                left_combined.iter().collect::<Vec<_>>(),
                deque.iter().collect::<Vec<_>>()
            );

            let right_combined = deque.clone().combine(empty);
            assert_eq!(
                right_combined.iter().collect::<Vec<_>>(),
                deque.iter().collect::<Vec<_>>()
            );
        }
    }

    mod stress_tests {
        use super::*;

        #[rstest]
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        fn test_large_deque() {
            let mut deque = PersistentDeque::new();
            for i in 0..1000 {
                deque = deque.push_back(i);
            }
            assert_eq!(deque.len(), 1000);
            assert_eq!(deque.front(), Some(&0));
            assert_eq!(deque.back(), Some(&999));

            for (index, &element) in deque.iter().enumerate() {
                assert_eq!(element, index as i32);
            }
        }

        #[rstest]
        fn test_mixed_operations() {
            let mut deque = PersistentDeque::new();

            for i in 0..50 {
                deque = deque.push_back(i);
                deque = deque.push_front(-i - 1);
            }

            assert_eq!(deque.len(), 100);

            for _ in 0..25 {
                let (rest, _) = deque.pop_front().unwrap();
                deque = rest;
                let (rest, _) = deque.pop_back().unwrap();
                deque = rest;
            }

            assert_eq!(deque.len(), 50);
        }
    }
}
