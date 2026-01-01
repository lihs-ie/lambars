//! Persistent (immutable) vector based on Radix Balanced Tree.
//!
//! This module provides [`PersistentVector`], an immutable dynamic array
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentVector` is a 32-way branching trie (Radix Balanced Tree) inspired by
//! Clojure's PersistentVector and Scala's Vector. It provides:
//!
//! - O(log32 N) random access (effectively O(1) for practical sizes)
//! - O(log32 N) push_back (amortized O(1) with tail optimization)
//! - O(log32 N) update
//! - O(N) push_front and pop_front (requires rebuilding)
//! - O(1) len and is_empty
//!
//! All operations return new vectors without modifying the original,
//! and structural sharing ensures memory efficiency.
//!
//! # Internal Structure
//!
//! The vector consists of:
//! - A root node (32-way branching trie)
//! - A tail buffer (up to 32 elements) for efficient append
//!
//! # Examples
//!
//! ```rust
//! use lambars::persistent::PersistentVector;
//!
//! let vector = PersistentVector::new()
//!     .push_back(1)
//!     .push_back(2)
//!     .push_back(3);
//!
//! assert_eq!(vector.get(0), Some(&1));
//! assert_eq!(vector.get(1), Some(&2));
//! assert_eq!(vector.get(2), Some(&3));
//!
//! // Structural sharing: the original vector is preserved
//! let extended = vector.push_back(4);
//! assert_eq!(vector.len(), 3);     // Original unchanged
//! assert_eq!(extended.len(), 4);   // New vector
//! ```

use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;

use crate::typeclass::{Foldable, Functor, FunctorMut, Monoid, Semigroup, TypeConstructor};

// =============================================================================
// Constants
// =============================================================================

/// Branching factor (2^5 = 32)
const BRANCHING_FACTOR: usize = 32;

/// Bits per level in the trie
const BITS_PER_LEVEL: usize = 5;

/// Bit mask for extracting index within a node
const MASK: usize = BRANCHING_FACTOR - 1;

// =============================================================================
// Node Definition
// =============================================================================

/// Internal node structure for the radix balanced tree.
#[derive(Clone)]
enum Node<T> {
    /// Branch node containing child nodes
    Branch(Rc<[Option<Rc<Node<T>>>; BRANCHING_FACTOR]>),
    /// Leaf node containing actual elements
    Leaf(Rc<[T]>),
}

impl<T> Node<T> {
    /// Creates an empty branch node.
    fn empty_branch() -> Self {
        Node::Branch(Rc::new(std::array::from_fn(|_| None)))
    }
}

impl<T: Clone> Node<T> {
    /// Creates a leaf node with the given elements.
    fn leaf_from_slice(elements: &[T]) -> Self {
        Node::Leaf(Rc::from(elements))
    }
}

// =============================================================================
// PersistentVector Definition
// =============================================================================

/// A persistent (immutable) vector based on Radix Balanced Tree.
///
/// `PersistentVector` is an immutable data structure that uses structural
/// sharing to efficiently support functional programming patterns.
///
/// # Time Complexity
///
/// | Operation    | Complexity                    |
/// |--------------|-------------------------------|
/// | `new`        | O(1)                          |
/// | `get`        | O(log32 N)                    |
/// | `push_back`  | O(log32 N) amortized O(1)     |
/// | `pop_back`   | O(log32 N)                    |
/// | `push_front` | O(N)                          |
/// | `pop_front`  | O(N)                          |
/// | `update`     | O(log32 N)                    |
/// | `len`        | O(1)                          |
/// | `is_empty`   | O(1)                          |
/// | `iter`       | O(1) to create, O(N) to iterate |
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentVector;
///
/// let vector: PersistentVector<i32> = (0..100).collect();
/// assert_eq!(vector.len(), 100);
/// assert_eq!(vector.get(50), Some(&50));
/// ```
#[derive(Clone)]
pub struct PersistentVector<T> {
    /// Total number of elements
    length: usize,
    /// Shift amount for index calculation: (depth - 1) * BITS_PER_LEVEL
    shift: usize,
    /// Root node of the trie
    root: Rc<Node<T>>,
    /// Tail buffer for efficient append (up to 32 elements)
    tail: Rc<[T]>,
}

impl<T> PersistentVector<T> {
    /// Creates a new empty vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = PersistentVector::new();
    /// assert!(vector.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        PersistentVector {
            length: 0,
            shift: BITS_PER_LEVEL,
            root: Rc::new(Node::empty_branch()),
            tail: Rc::from(Vec::<T>::new()),
        }
    }

    /// Creates a vector containing a single element.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to store in the vector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector = PersistentVector::singleton(42);
    /// assert_eq!(vector.len(), 1);
    /// assert_eq!(vector.get(0), Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn singleton(element: T) -> Self {
        PersistentVector {
            length: 1,
            shift: BITS_PER_LEVEL,
            root: Rc::new(Node::empty_branch()),
            tail: Rc::from(vec![element]),
        }
    }

    /// Returns the number of elements in the vector.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// assert_eq!(vector.len(), 5);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let empty: PersistentVector<i32> = PersistentVector::new();
    /// assert!(empty.is_empty());
    ///
    /// let non_empty = empty.push_back(1);
    /// assert!(!non_empty.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns the starting index of the tail buffer.
    #[inline]
    fn tail_offset(&self) -> usize {
        if self.length < BRANCHING_FACTOR {
            0
        } else {
            ((self.length - 1) >> BITS_PER_LEVEL) << BITS_PER_LEVEL
        }
    }

    /// Returns a reference to the element at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index of the element
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// assert_eq!(vector.get(0), Some(&1));
    /// assert_eq!(vector.get(4), Some(&5));
    /// assert_eq!(vector.get(10), None);
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.length {
            return None;
        }

        let tail_offset = self.tail_offset();

        if index >= tail_offset {
            // Element is in the tail
            self.tail.get(index - tail_offset)
        } else {
            // Element is in the root tree
            self.get_from_root(index)
        }
    }

    /// Gets an element from the root tree.
    fn get_from_root(&self, index: usize) -> Option<&T> {
        let mut node = &self.root;
        let mut level = self.shift;

        while level > 0 {
            match node.as_ref() {
                Node::Branch(children) => {
                    let child_index = (index >> level) & MASK;
                    match &children[child_index] {
                        Some(child) => {
                            node = child;
                            level -= BITS_PER_LEVEL;
                        }
                        None => return None,
                    }
                }
                Node::Leaf(_) => break,
            }
        }

        match node.as_ref() {
            Node::Leaf(elements) => elements.get(index & MASK),
            Node::Branch(children) => {
                let child_index = index & MASK;
                match &children[child_index] {
                    Some(child) => match child.as_ref() {
                        Node::Leaf(elements) => elements.first(),
                        _ => None,
                    },
                    None => None,
                }
            }
        }
    }

    /// Returns a reference to the first element.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// assert_eq!(vector.first(), Some(&1));
    ///
    /// let empty: PersistentVector<i32> = PersistentVector::new();
    /// assert_eq!(empty.first(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    /// Returns a reference to the last element.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// # Complexity
    ///
    /// O(1) - the last element is always in the tail
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// assert_eq!(vector.last(), Some(&5));
    ///
    /// let empty: PersistentVector<i32> = PersistentVector::new();
    /// assert_eq!(empty.last(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn last(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            self.tail.last()
        }
    }

    /// Returns an iterator over references to the elements.
    ///
    /// The iterator yields elements from front to back.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let collected: Vec<&i32> = vector.iter().collect();
    /// assert_eq!(collected, vec![&1, &2, &3, &4, &5]);
    /// ```
    pub fn iter(&self) -> PersistentVectorIterator<'_, T> {
        PersistentVectorIterator {
            vector: self,
            current_index: 0,
        }
    }
}

impl<T: Clone> PersistentVector<T> {
    /// Appends an element to the back of the vector.
    ///
    /// Returns a new vector with the element at the end.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to append
    ///
    /// # Complexity
    ///
    /// O(log32 N) amortized O(1) due to tail optimization
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector = PersistentVector::new()
    ///     .push_back(1)
    ///     .push_back(2)
    ///     .push_back(3);
    ///
    /// assert_eq!(vector.len(), 3);
    /// assert_eq!(vector.get(2), Some(&3));
    /// ```
    #[must_use]
    pub fn push_back(&self, element: T) -> Self {
        if self.tail.len() < BRANCHING_FACTOR {
            // Tail has space, just add to tail
            let mut new_tail = self.tail.to_vec();
            new_tail.push(element);

            PersistentVector {
                length: self.length + 1,
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail.as_slice()),
            }
        } else {
            // Tail is full, push tail to root and create new tail
            self.push_tail_to_root(element)
        }
    }

    /// Pushes the current tail into the root and creates a new tail with the element.
    fn push_tail_to_root(&self, element: T) -> Self {
        let tail_leaf = Node::leaf_from_slice(&self.tail);
        let tail_offset = self.tail_offset();

        // Check if we need to increase the tree depth
        // The tree can hold up to BRANCHING_FACTOR^(shift/BITS_PER_LEVEL + 1) elements in root
        // Root overflow occurs when tail_offset / BRANCHING_FACTOR >= capacity of current level
        let root_overflow = (tail_offset >> self.shift) >= BRANCHING_FACTOR;

        if root_overflow {
            // Create a new root level
            let mut new_root_children: [Option<Rc<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            new_root_children[0] = Some(self.root.clone());
            new_root_children[1] = Some(Rc::new(self.new_path(self.shift, tail_leaf)));

            PersistentVector {
                length: self.length + 1,
                shift: self.shift + BITS_PER_LEVEL,
                root: Rc::new(Node::Branch(Rc::new(new_root_children))),
                tail: Rc::from([element].as_slice()),
            }
        } else {
            // Push tail into existing root
            let new_root = self.push_tail_into_node(&self.root, self.shift, tail_offset, tail_leaf);

            PersistentVector {
                length: self.length + 1,
                shift: self.shift,
                root: Rc::new(new_root),
                tail: Rc::from([element].as_slice()),
            }
        }
    }

    /// Creates a new path from root to the leaf.
    fn new_path(&self, level: usize, node: Node<T>) -> Node<T> {
        if level == 0 {
            node
        } else {
            let mut children: [Option<Rc<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            children[0] = Some(Rc::new(self.new_path(level - BITS_PER_LEVEL, node)));
            Node::Branch(Rc::new(children))
        }
    }

    /// Pushes a tail leaf into the tree at the given level.
    fn push_tail_into_node(
        &self,
        node: &Rc<Node<T>>,
        level: usize,
        tail_offset: usize,
        tail_node: Node<T>,
    ) -> Node<T> {
        let subindex = (tail_offset >> level) & MASK;

        match node.as_ref() {
            Node::Branch(children) => {
                let mut new_children = children.as_ref().clone();

                if level == BITS_PER_LEVEL {
                    // We're at the bottom branch level, insert the tail leaf
                    new_children[subindex] = Some(Rc::new(tail_node));
                } else {
                    // Recurse down
                    let child = match &children[subindex] {
                        Some(c) => self.push_tail_into_node(
                            c,
                            level - BITS_PER_LEVEL,
                            tail_offset,
                            tail_node,
                        ),
                        None => self.new_path(level - BITS_PER_LEVEL, tail_node),
                    };
                    new_children[subindex] = Some(Rc::new(child));
                }

                Node::Branch(Rc::new(new_children))
            }
            Node::Leaf(_) => {
                // This shouldn't happen in a well-formed tree
                tail_node
            }
        }
    }

    /// Removes the last element from the vector.
    ///
    /// Returns `None` if the vector is empty, otherwise returns the new vector
    /// and the removed element.
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let (remaining, element) = vector.pop_back().unwrap();
    ///
    /// assert_eq!(element, 5);
    /// assert_eq!(remaining.len(), 4);
    /// ```
    #[must_use]
    pub fn pop_back(&self) -> Option<(Self, T)> {
        if self.is_empty() {
            return None;
        }

        if self.length == 1 {
            return Some((PersistentVector::new(), self.tail[0].clone()));
        }

        if self.tail.len() > 1 {
            // Just remove from tail
            let element = self.tail.last().unwrap().clone();
            let new_tail: Vec<T> = self.tail[..self.tail.len() - 1].to_vec();

            let new_vector = PersistentVector {
                length: self.length - 1,
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail.as_slice()),
            };

            Some((new_vector, element))
        } else {
            // Tail has only 1 element, need to pop from root
            let element = self.tail[0].clone();
            let new_tail_offset = self.length - BRANCHING_FACTOR - 1;

            // Get the new tail from the root
            let new_tail = self.get_leaf_at(new_tail_offset);

            // Remove the last leaf from the root
            let (new_root, new_shift) = self.pop_tail_from_root();

            let new_vector = PersistentVector {
                length: self.length - 1,
                shift: new_shift,
                root: new_root,
                tail: new_tail,
            };

            Some((new_vector, element))
        }
    }

    /// Gets the leaf at the given offset.
    fn get_leaf_at(&self, offset: usize) -> Rc<[T]> {
        let mut node = &self.root;
        let mut level = self.shift;

        while level > 0 {
            match node.as_ref() {
                Node::Branch(children) => {
                    let child_index = (offset >> level) & MASK;
                    if let Some(child) = &children[child_index] {
                        node = child;
                        level -= BITS_PER_LEVEL;
                    } else {
                        return Rc::from([].as_slice());
                    }
                }
                Node::Leaf(_) => break,
            }
        }

        match node.as_ref() {
            Node::Leaf(elements) => elements.clone(),
            _ => Rc::from([].as_slice()),
        }
    }

    /// Removes the tail from the root.
    fn pop_tail_from_root(&self) -> (Rc<Node<T>>, usize) {
        let tail_offset = self.length - 2; // Last valid index after pop
        let (new_root, _) = self.do_pop_tail(&self.root, self.shift, tail_offset);

        // Check if we should reduce tree depth
        match new_root.as_ref() {
            Node::Branch(children) => {
                if self.shift > BITS_PER_LEVEL {
                    // Count non-None children
                    let non_none_count = children.iter().filter(|c| c.is_some()).count();
                    if non_none_count == 1 {
                        if let Some(only_child) = &children[0] {
                            return (only_child.clone(), self.shift - BITS_PER_LEVEL);
                        }
                    }
                }
                (new_root, self.shift)
            }
            _ => (new_root, self.shift),
        }
    }

    /// Recursively pops the tail from the tree.
    fn do_pop_tail(&self, node: &Rc<Node<T>>, level: usize, offset: usize) -> (Rc<Node<T>>, bool) {
        let subindex = (offset >> level) & MASK;

        match node.as_ref() {
            Node::Branch(children) => {
                if level == BITS_PER_LEVEL {
                    // At bottom level, remove the child
                    let mut new_children = children.as_ref().clone();
                    new_children[subindex] = None;

                    let all_none = new_children.iter().all(|c| c.is_none());
                    (Rc::new(Node::Branch(Rc::new(new_children))), all_none)
                } else if let Some(child) = &children[subindex] {
                    let (new_child, is_empty) =
                        self.do_pop_tail(child, level - BITS_PER_LEVEL, offset);
                    let mut new_children = children.as_ref().clone();

                    if is_empty {
                        new_children[subindex] = None;
                    } else {
                        new_children[subindex] = Some(new_child);
                    }

                    let all_none = new_children.iter().all(|c| c.is_none());
                    (Rc::new(Node::Branch(Rc::new(new_children))), all_none)
                } else {
                    (node.clone(), false)
                }
            }
            Node::Leaf(_) => (node.clone(), true),
        }
    }

    /// Prepends an element to the front of the vector.
    ///
    /// Returns a new vector with the element at the front.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to prepend
    ///
    /// # Complexity
    ///
    /// O(N) - requires rebuilding the vector with all elements shifted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=3).collect();
    /// let with_zero = vector.push_front(0);
    ///
    /// assert_eq!(with_zero.get(0), Some(&0));
    /// assert_eq!(with_zero.get(1), Some(&1));
    /// ```
    #[must_use]
    pub fn push_front(&self, element: T) -> Self {
        // Rebuild the vector with the new element at the front
        // This is O(N) but maintains the correct structure
        let mut elements: Vec<T> = Vec::with_capacity(self.length + 1);
        elements.push(element);
        for item in self.iter() {
            elements.push(item.clone());
        }
        elements.into_iter().collect()
    }

    /// Removes the first element from the vector.
    ///
    /// Returns `None` if the vector is empty, otherwise returns the new vector
    /// and the removed element.
    ///
    /// # Complexity
    ///
    /// O(N) - requires rebuilding the vector without the first element
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let (remaining, element) = vector.pop_front().unwrap();
    ///
    /// assert_eq!(element, 1);
    /// assert_eq!(remaining.len(), 4);
    /// assert_eq!(remaining.get(0), Some(&2));
    /// ```
    #[must_use]
    pub fn pop_front(&self) -> Option<(Self, T)> {
        if self.is_empty() {
            return None;
        }

        let first = self.get(0)?.clone();

        // Rebuild without the first element
        let new_vector: PersistentVector<T> = self.iter().skip(1).cloned().collect();

        Some((new_vector, first))
    }

    /// Updates the element at the given index.
    ///
    /// Returns `None` if the index is out of bounds, otherwise returns a new
    /// vector with the updated element.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index to update
    /// * `element` - The new element value
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let updated = vector.update(2, 100).unwrap();
    ///
    /// assert_eq!(updated.get(2), Some(&100));
    /// assert_eq!(vector.get(2), Some(&3)); // Original unchanged
    /// ```
    #[must_use]
    pub fn update(&self, index: usize, element: T) -> Option<Self> {
        if index >= self.length {
            return None;
        }

        let tail_offset = self.tail_offset();

        if index >= tail_offset {
            // Element is in the tail
            let tail_index = index - tail_offset;
            let mut new_tail = self.tail.to_vec();
            new_tail[tail_index] = element;

            Some(PersistentVector {
                length: self.length,
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail.as_slice()),
            })
        } else {
            // Element is in the root
            let new_root = self.update_in_root(&self.root, self.shift, index, element);

            Some(PersistentVector {
                length: self.length,
                shift: self.shift,
                root: Rc::new(new_root),
                tail: self.tail.clone(),
            })
        }
    }

    /// Updates an element in the root tree.
    fn update_in_root(
        &self,
        node: &Rc<Node<T>>,
        level: usize,
        index: usize,
        element: T,
    ) -> Node<T> {
        match node.as_ref() {
            Node::Branch(children) => {
                let subindex = (index >> level) & MASK;
                let mut new_children = children.as_ref().clone();

                if level > 0 {
                    if let Some(child) = &children[subindex] {
                        new_children[subindex] = Some(Rc::new(self.update_in_root(
                            child,
                            level - BITS_PER_LEVEL,
                            index,
                            element,
                        )));
                    }
                } else if let Some(child) = &children[subindex] {
                    new_children[subindex] =
                        Some(Rc::new(self.update_in_root(child, 0, index, element)));
                }

                Node::Branch(Rc::new(new_children))
            }
            Node::Leaf(elements) => {
                let leaf_index = index & MASK;
                let mut new_elements = elements.to_vec();
                if leaf_index < new_elements.len() {
                    new_elements[leaf_index] = element;
                }
                Node::Leaf(Rc::from(new_elements.as_slice()))
            }
        }
    }

    /// Appends another vector to this vector.
    ///
    /// Returns a new vector containing all elements from this vector
    /// followed by all elements from the other vector.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to append
    ///
    /// # Complexity
    ///
    /// O(M log32 N) where M is the length of other
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector1: PersistentVector<i32> = (1..=3).collect();
    /// let vector2: PersistentVector<i32> = (4..=6).collect();
    /// let combined = vector1.append(&vector2);
    ///
    /// assert_eq!(combined.len(), 6);
    /// ```
    #[must_use]
    pub fn append(&self, other: &Self) -> Self {
        if self.is_empty() {
            return other.clone();
        }
        if other.is_empty() {
            return self.clone();
        }

        let mut result = self.clone();
        for element in other.iter() {
            result = result.push_back(element.clone());
        }
        result
    }

    /// Returns a new vector containing elements from index `start` (inclusive)
    /// to index `end` (exclusive).
    ///
    /// If `start` is greater than or equal to `end`, or `start` is out of bounds,
    /// returns an empty vector. If `end` exceeds the vector's length, it is
    /// clamped to the length.
    ///
    /// # Arguments
    ///
    /// * `start` - The starting index (inclusive)
    /// * `end` - The ending index (exclusive)
    ///
    /// # Complexity
    ///
    /// O((end - start) log32 N) for building the slice
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (0..10).collect();
    /// let sliced = vector.slice(2, 5);
    ///
    /// assert_eq!(sliced.len(), 3);
    /// assert_eq!(sliced.get(0), Some(&2));
    /// assert_eq!(sliced.get(1), Some(&3));
    /// assert_eq!(sliced.get(2), Some(&4));
    /// ```
    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> Self {
        // Handle invalid range cases
        if start >= self.length || start >= end {
            return PersistentVector::new();
        }

        // Clamp end to the vector's length
        let clamped_end = end.min(self.length);

        // Build a new vector from the slice range
        self.iter()
            .skip(start)
            .take(clamped_end - start)
            .cloned()
            .collect()
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// An iterator over references to elements of a [`PersistentVector`].
pub struct PersistentVectorIterator<'a, T> {
    vector: &'a PersistentVector<T>,
    current_index: usize,
}

impl<'a, T> Iterator for PersistentVectorIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.vector.length {
            return None;
        }

        let item = self.vector.get(self.current_index);
        self.current_index += 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vector.length.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for PersistentVectorIterator<'_, T> {
    fn len(&self) -> usize {
        self.vector.length.saturating_sub(self.current_index)
    }
}

/// An owning iterator over elements of a [`PersistentVector`].
pub struct PersistentVectorIntoIterator<T> {
    vector: PersistentVector<T>,
    current_index: usize,
}

impl<T: Clone> Iterator for PersistentVectorIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.vector.length {
            return None;
        }

        let item = self.vector.get(self.current_index).cloned();
        self.current_index += 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vector.length.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<T: Clone> ExactSizeIterator for PersistentVectorIntoIterator<T> {
    fn len(&self) -> usize {
        self.vector.length.saturating_sub(self.current_index)
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<T> Default for PersistentVector<T> {
    #[inline]
    fn default() -> Self {
        PersistentVector::new()
    }
}

impl<T: Clone> FromIterator<T> for PersistentVector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vector = PersistentVector::new();
        for element in iter {
            vector = vector.push_back(element);
        }
        vector
    }
}

impl<T: Clone> IntoIterator for PersistentVector<T> {
    type Item = T;
    type IntoIter = PersistentVectorIntoIterator<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        PersistentVectorIntoIterator {
            vector: self,
            current_index: 0,
        }
    }
}

impl<'a, T> IntoIterator for &'a PersistentVector<T> {
    type Item = &'a T;
    type IntoIter = PersistentVectorIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: PartialEq> PartialEq for PersistentVector<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: Eq> Eq for PersistentVector<T> {}

impl<T: fmt::Debug> fmt::Debug for PersistentVector<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

// =============================================================================
// Type Class Implementations
// =============================================================================

impl<T> TypeConstructor for PersistentVector<T> {
    type Inner = T;
    type WithType<B> = PersistentVector<B>;
}

impl<T: Clone> Functor for PersistentVector<T> {
    fn fmap<B, F>(self, function: F) -> PersistentVector<B>
    where
        F: FnOnce(T) -> B,
    {
        // FnOnce can only be called once, so this only works for single-element vectors
        if let Some(first) = self.get(0) {
            PersistentVector::singleton(function(first.clone()))
        } else {
            PersistentVector::new()
        }
    }

    fn fmap_ref<B, F>(&self, function: F) -> PersistentVector<B>
    where
        F: FnOnce(&T) -> B,
    {
        if let Some(first) = self.get(0) {
            PersistentVector::singleton(function(first))
        } else {
            PersistentVector::new()
        }
    }
}

impl<T: Clone> FunctorMut for PersistentVector<T> {
    fn fmap_mut<B, F>(self, mut function: F) -> PersistentVector<B>
    where
        F: FnMut(T) -> B,
    {
        build_persistent_vector_from_iter(self.into_iter().map(|element| function(element)))
    }

    fn fmap_ref_mut<B, F>(&self, mut function: F) -> PersistentVector<B>
    where
        F: FnMut(&T) -> B,
    {
        build_persistent_vector_from_iter(self.iter().map(|element| function(element)))
    }
}

/// Helper function to build a PersistentVector from an iterator without requiring Clone.
fn build_persistent_vector_from_iter<T, I>(iter: I) -> PersistentVector<T>
where
    I: Iterator<Item = T>,
{
    let elements: Vec<T> = iter.collect();
    build_persistent_vector_from_vec(elements)
}

/// Helper function to build a PersistentVector from a Vec without requiring Clone.
fn build_persistent_vector_from_vec<T>(elements: Vec<T>) -> PersistentVector<T> {
    if elements.is_empty() {
        return PersistentVector::new();
    }

    let length = elements.len();

    // For small vectors, just put everything in the tail
    if length <= BRANCHING_FACTOR {
        return PersistentVector {
            length,
            shift: BITS_PER_LEVEL,
            root: Rc::new(Node::empty_branch()),
            tail: Rc::from(elements),
        };
    }

    // Calculate how many elements go in the tail
    let tail_size = length % BRANCHING_FACTOR;
    let tail_size = if tail_size == 0 {
        BRANCHING_FACTOR
    } else {
        tail_size
    };
    let root_size = length - tail_size;

    // Split elements into root and tail portions
    let mut elements = elements;
    let tail_elements = elements.split_off(root_size);
    let root_elements = elements;

    // Build the root tree
    let (root, shift) = build_root_from_elements(root_elements);

    PersistentVector {
        length,
        shift,
        root,
        tail: Rc::from(tail_elements),
    }
}

/// Build the root tree from a vector of elements.
fn build_root_from_elements<T>(elements: Vec<T>) -> (Rc<Node<T>>, usize) {
    if elements.is_empty() {
        return (Rc::new(Node::empty_branch()), BITS_PER_LEVEL);
    }

    // Split into chunks of BRANCHING_FACTOR
    let mut leaves: Vec<Rc<Node<T>>> = Vec::new();
    let mut iter = elements.into_iter();

    loop {
        let chunk: Vec<T> = iter.by_ref().take(BRANCHING_FACTOR).collect();
        if chunk.is_empty() {
            break;
        }
        leaves.push(Rc::new(Node::Leaf(Rc::from(chunk))));
    }

    // If there's only one leaf, wrap it in a branch
    if leaves.len() == 1 {
        let mut children: [Option<Rc<Node<T>>>; BRANCHING_FACTOR] = std::array::from_fn(|_| None);
        children[0] = Some(leaves.remove(0));
        return (Rc::new(Node::Branch(Rc::new(children))), BITS_PER_LEVEL);
    }

    // Build tree bottom-up
    let mut current_level = leaves;
    let mut shift = BITS_PER_LEVEL;

    while current_level.len() > BRANCHING_FACTOR {
        let mut next_level: Vec<Rc<Node<T>>> = Vec::new();

        for chunk in current_level.chunks(BRANCHING_FACTOR) {
            let mut children: [Option<Rc<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            for (index, node) in chunk.iter().enumerate() {
                children[index] = Some(node.clone());
            }
            next_level.push(Rc::new(Node::Branch(Rc::new(children))));
        }

        current_level = next_level;
        shift += BITS_PER_LEVEL;
    }

    // Wrap the remaining nodes in the root branch
    let mut root_children: [Option<Rc<Node<T>>>; BRANCHING_FACTOR] = std::array::from_fn(|_| None);
    for (index, node) in current_level.into_iter().enumerate() {
        root_children[index] = Some(node);
    }

    (Rc::new(Node::Branch(Rc::new(root_children))), shift)
}

impl<T: Clone> Foldable for PersistentVector<T> {
    fn fold_left<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.into_iter()
            .fold(init, |accumulator, element| function(accumulator, element))
    }

    fn fold_right<B, F>(self, init: B, mut function: F) -> B
    where
        F: FnMut(T, B) -> B,
    {
        // Collect and reverse for right fold
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
        self.length == 0
    }

    #[inline]
    fn length(&self) -> usize
    where
        Self: Clone,
    {
        self.length
    }
}

impl<T: Clone> Semigroup for PersistentVector<T> {
    fn combine(self, other: Self) -> Self {
        self.append(&other)
    }
}

impl<T: Clone> Monoid for PersistentVector<T> {
    fn empty() -> Self {
        PersistentVector::new()
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
        let vector: PersistentVector<i32> = PersistentVector::new();
        assert!(vector.is_empty());
        assert_eq!(vector.len(), 0);
    }

    #[rstest]
    fn test_singleton() {
        let vector = PersistentVector::singleton(42);
        assert_eq!(vector.len(), 1);
        assert_eq!(vector.get(0), Some(&42));
    }

    #[rstest]
    fn test_push_back_and_get() {
        let vector = PersistentVector::new()
            .push_back(1)
            .push_back(2)
            .push_back(3);
        assert_eq!(vector.len(), 3);
        assert_eq!(vector.get(0), Some(&1));
        assert_eq!(vector.get(1), Some(&2));
        assert_eq!(vector.get(2), Some(&3));
    }

    #[rstest]
    fn test_large_vector() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        assert_eq!(vector.len(), 1000);
        for index in 0..1000 {
            assert_eq!(vector.get(index), Some(&(index as i32)));
        }
    }

    #[rstest]
    fn test_update() {
        let vector: PersistentVector<i32> = (0..10).collect();
        let updated = vector.update(5, 100).unwrap();
        assert_eq!(updated.get(5), Some(&100));
        assert_eq!(vector.get(5), Some(&5));
    }

    #[rstest]
    fn test_pop_back() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let (remaining, element) = vector.pop_back().unwrap();
        assert_eq!(element, 5);
        assert_eq!(remaining.len(), 4);
    }

    #[rstest]
    fn test_iter() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let collected: Vec<&i32> = vector.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3, &4, &5]);
    }

    #[rstest]
    fn test_append() {
        let vector1: PersistentVector<i32> = (1..=3).collect();
        let vector2: PersistentVector<i32> = (4..=6).collect();
        let combined = vector1.append(&vector2);
        assert_eq!(combined.len(), 6);
        let collected: Vec<_> = combined.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn test_eq() {
        let vector1: PersistentVector<i32> = (1..=5).collect();
        let vector2: PersistentVector<i32> = (1..=5).collect();
        assert_eq!(vector1, vector2);
    }

    #[rstest]
    fn test_fmap_mut() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let doubled: PersistentVector<i32> = vector.fmap_mut(|x| x * 2);
        let collected: Vec<_> = doubled.iter().copied().collect();
        assert_eq!(collected, vec![2, 4, 6, 8, 10]);
    }

    #[rstest]
    fn test_fold_left() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let sum = vector.fold_left(0, |acc, x| acc + x);
        assert_eq!(sum, 15);
    }

    #[rstest]
    fn test_semigroup_combine() {
        let vector1: PersistentVector<i32> = (1..=3).collect();
        let vector2: PersistentVector<i32> = (4..=6).collect();
        let combined = vector1.combine(vector2);
        assert_eq!(combined.len(), 6);
    }

    #[rstest]
    fn test_monoid_empty() {
        let empty: PersistentVector<i32> = PersistentVector::empty();
        assert!(empty.is_empty());
    }
}
