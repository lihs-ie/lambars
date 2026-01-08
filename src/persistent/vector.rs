//! Persistent (immutable) vector based on Radix Balanced Tree.
//!
//! This module provides [`PersistentVector`], an immutable dynamic array
//! that uses structural sharing for efficient operations.
//!
//! # Overview
//!
//! `PersistentVector` is a 32-way branching trie (Radix Balanced Tree) inspired by
//! Clojure's `PersistentVector` and Scala's Vector. It provides:
//!
//! - O(log32 N) random access (effectively O(1) for practical sizes)
//! - O(log32 N) `push_back` (amortized O(1) with tail optimization)
//! - O(log32 N) update
//! - O(N) `push_front` and `pop_front` (requires rebuilding)
//! - O(1) len and `is_empty`
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
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

use super::ReferenceCounter;

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
    Branch(ReferenceCounter<[Option<ReferenceCounter<Self>>; BRANCHING_FACTOR]>),
    /// Leaf node containing actual elements
    Leaf(ReferenceCounter<[T]>),
}

impl<T> Node<T> {
    /// Creates an empty branch node.
    fn empty_branch() -> Self {
        Self::Branch(ReferenceCounter::new(std::array::from_fn(|_| None)))
    }
}

impl<T: Clone> Node<T> {
    /// Creates a leaf node by reusing an existing `ReferenceCounter<[T]>`.
    ///
    /// This avoids copying the elements and only increments the reference count.
    ///
    /// # Arguments
    ///
    /// * `elements` - An existing `ReferenceCounter<[T]>` to reuse
    ///
    /// # Returns
    ///
    /// A new Leaf node that shares the underlying storage
    #[inline]
    const fn leaf_from_reference_counter(elements: ReferenceCounter<[T]>) -> Self {
        Self::Leaf(elements)
    }
}

// =============================================================================
// TransientVector Definition
// =============================================================================

use std::marker::PhantomData;
use std::rc::Rc;

/// A transient (temporarily mutable) version of `PersistentVector`.
///
/// `TransientVector` provides efficient batch mutation operations by avoiding
/// the structural sharing overhead during construction. Once all mutations
/// are complete, call [`TransientVector::persistent()`] to convert back to an
/// immutable `PersistentVector`.
///
/// # Design
///
/// This follows the Clojure transient pattern:
/// - Convert from persistent to transient with `transient()`
/// - Perform batch mutations with `&mut self` methods
/// - Convert back with `persistent()`
///
/// # Thread Safety
///
/// `TransientVector` is intentionally **not** `Send` or `Sync`. It is designed
/// for single-threaded batch construction. Once converted to `PersistentVector`,
/// the result can be shared across threads (when the `arc` feature is enabled).
///
/// # Type Constraints
///
/// `TransientVector<T>` requires `T: Clone` because Copy-on-Write (COW)
/// semantics are used internally via `Rc::make_mut()` / `Arc::make_mut()`.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentVector;
///
/// // Efficient batch construction
/// let mut transient = PersistentVector::new().transient();
/// for i in 0..1000 {
///     transient.push_back(i);
/// }
/// let persistent = transient.persistent();
/// assert_eq!(persistent.len(), 1000);
/// ```
pub struct TransientVector<T> {
    root: ReferenceCounter<Node<T>>,
    tail: Vec<T>,
    length: usize,
    shift: usize,
    /// Marker to ensure `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

// Static assertions to verify TransientVector is not Send/Sync
static_assertions::assert_not_impl_any!(TransientVector<i32>: Send, Sync);
static_assertions::assert_not_impl_any!(TransientVector<String>: Send, Sync);

// Arc feature verification: even with Arc, TransientVector remains !Send/!Sync
#[cfg(feature = "arc")]
mod arc_send_sync_verification {
    use super::TransientVector;
    use std::sync::Arc;

    // Arc<T> where T: Send+Sync is Send+Sync, but TransientVector should still be !Send/!Sync
    static_assertions::assert_not_impl_any!(TransientVector<Arc<i32>>: Send, Sync);
    static_assertions::assert_not_impl_any!(TransientVector<Arc<String>>: Send, Sync);
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
    /// Shift amount for index calculation: (depth - 1) * `BITS_PER_LEVEL`
    shift: usize,
    /// Root node of the trie
    root: ReferenceCounter<Node<T>>,
    /// Tail buffer for efficient append (up to 32 elements)
    tail: ReferenceCounter<[T]>,
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
        Self {
            length: 0,
            shift: BITS_PER_LEVEL,
            root: ReferenceCounter::new(Node::empty_branch()),
            tail: ReferenceCounter::from(Vec::<T>::new()),
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
        Self {
            length: 1,
            shift: BITS_PER_LEVEL,
            root: ReferenceCounter::new(Node::empty_branch()),
            tail: ReferenceCounter::from(vec![element]),
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
    pub const fn len(&self) -> usize {
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
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns the starting index of the tail buffer.
    #[inline]
    const fn tail_offset(&self) -> usize {
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
                children[child_index]
                    .as_ref()
                    .and_then(|child| match child.as_ref() {
                        Node::Leaf(elements) => elements.first(),
                        Node::Branch(_) => None,
                    })
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
    /// The iterator yields elements from front to back in O(N) time.
    /// This is achieved through stack-based tree traversal, which visits
    /// each node exactly once.
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
    #[must_use]
    pub fn iter(&self) -> PersistentVectorIterator<'_, T> {
        PersistentVectorIterator::new(self)
    }

    /// Finds the index of the first element that satisfies the predicate.
    ///
    /// Returns `Some(index)` if an element is found, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns `true` for the target element
    ///
    /// # Complexity
    ///
    /// O(n) worst case
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let index = vector.find_index(|x| *x > 3);
    /// // index = Some(3)
    /// ```
    #[must_use]
    pub fn find_index<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(&T) -> bool,
    {
        self.iter().position(predicate)
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

            Self {
                length: self.length + 1,
                shift: self.shift,
                root: self.root.clone(),
                tail: ReferenceCounter::from(new_tail.as_slice()),
            }
        } else {
            // Tail is full, push tail to root and create new tail
            self.push_tail_to_root(element)
        }
    }

    /// Appends multiple elements to the back of the vector.
    ///
    /// More efficient than calling `push_back` multiple times.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator over elements to append
    ///
    /// # Returns
    ///
    /// A new vector with all elements appended (the original is unchanged)
    ///
    /// # Complexity
    ///
    /// O(M log32 N) where M = `iter.count()`, N = `self.len()`
    /// The constant factor is smaller than M individual `push_back` calls.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=3).collect();
    /// let extended = vector.push_back_many(4..=6);
    ///
    /// assert_eq!(extended.len(), 6);
    /// let collected: Vec<i32> = extended.iter().copied().collect();
    /// assert_eq!(collected, vec![1, 2, 3, 4, 5, 6]);
    /// ```
    #[must_use]
    pub fn push_back_many<I>(&self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let new_elements: Vec<T> = iter.into_iter().collect();

        if new_elements.is_empty() {
            return self.clone();
        }

        // For small additions (<=4 elements), use individual push_back
        // This avoids the overhead of collecting all elements for small cases
        if new_elements.len() <= 4 {
            let mut result = self.clone();
            for element in new_elements {
                result = result.push_back(element);
            }
            return result;
        }

        // For larger additions, collect all elements and rebuild
        let total_length = self.length + new_elements.len();
        let mut all_elements: Vec<T> = Vec::with_capacity(total_length);

        // Collect existing elements via efficient O(N) iterator
        for element in self {
            all_elements.push(element.clone());
        }
        all_elements.extend(new_elements);

        // Rebuild using the efficient batch construction
        build_persistent_vector_from_vec(all_elements)
    }

    /// Creates a `PersistentVector` from a slice.
    ///
    /// The elements are cloned from the slice.
    ///
    /// # Arguments
    ///
    /// * `slice` - The source slice
    ///
    /// # Returns
    ///
    /// A new vector containing clones of the slice elements
    ///
    /// # Complexity
    ///
    /// O(N) where N = `slice.len()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector = PersistentVector::from_slice(&[1, 2, 3, 4, 5]);
    /// assert_eq!(vector.len(), 5);
    /// assert_eq!(vector.get(0), Some(&1));
    /// ```
    #[must_use]
    pub fn from_slice(slice: &[T]) -> Self {
        if slice.is_empty() {
            return Self::new();
        }

        let elements: Vec<T> = slice.to_vec();
        build_persistent_vector_from_vec(elements)
    }

    /// Pushes the current tail into the root and creates a new tail with the element.
    fn push_tail_to_root(&self, element: T) -> Self {
        // Use leaf_from_reference_counter to avoid copying elements - only increments reference count O(1)
        let tail_leaf = Node::leaf_from_reference_counter(self.tail.clone());
        let tail_offset = self.tail_offset();

        // Check if we need to increase the tree depth
        // The tree can hold up to BRANCHING_FACTOR^(shift/BITS_PER_LEVEL + 1) elements in root
        // Root overflow occurs when tail_offset / BRANCHING_FACTOR >= capacity of current level
        let root_overflow = (tail_offset >> self.shift) >= BRANCHING_FACTOR;

        if root_overflow {
            // Create a new root level
            let mut new_root_children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            new_root_children[0] = Some(self.root.clone());
            new_root_children[1] =
                Some(ReferenceCounter::new(Self::new_path(self.shift, tail_leaf)));

            Self {
                length: self.length + 1,
                shift: self.shift + BITS_PER_LEVEL,
                root: ReferenceCounter::new(Node::Branch(ReferenceCounter::new(new_root_children))),
                tail: ReferenceCounter::from([element].as_slice()),
            }
        } else {
            // Push tail into existing root
            let new_root =
                Self::push_tail_into_node(&self.root, self.shift, tail_offset, tail_leaf);

            Self {
                length: self.length + 1,
                shift: self.shift,
                root: ReferenceCounter::new(new_root),
                tail: ReferenceCounter::from([element].as_slice()),
            }
        }
    }

    /// Creates a new path from root to the leaf.
    fn new_path(level: usize, node: Node<T>) -> Node<T> {
        if level == 0 {
            node
        } else {
            let mut children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            children[0] = Some(ReferenceCounter::new(Self::new_path(
                level - BITS_PER_LEVEL,
                node,
            )));
            Node::Branch(ReferenceCounter::new(children))
        }
    }

    /// Pushes a tail leaf into the tree at the given level.
    fn push_tail_into_node(
        node: &ReferenceCounter<Node<T>>,
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
                    new_children[subindex] = Some(ReferenceCounter::new(tail_node));
                } else {
                    // Recurse down
                    let child = match &children[subindex] {
                        Some(c) => Self::push_tail_into_node(
                            c,
                            level - BITS_PER_LEVEL,
                            tail_offset,
                            tail_node,
                        ),
                        None => Self::new_path(level - BITS_PER_LEVEL, tail_node),
                    };
                    new_children[subindex] = Some(ReferenceCounter::new(child));
                }

                Node::Branch(ReferenceCounter::new(new_children))
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
    ///
    /// # Panics
    ///
    /// This function does not panic. The internal `unwrap()` calls are safe
    /// because the code structure guarantees the values exist at those points.
    #[must_use]
    pub fn pop_back(&self) -> Option<(Self, T)> {
        if self.is_empty() {
            return None;
        }

        if self.length == 1 {
            return Some((Self::new(), self.tail[0].clone()));
        }

        if self.tail.len() > 1 {
            // Just remove from tail
            let element = self.tail.last().unwrap().clone();
            let new_tail: Vec<T> = self.tail[..self.tail.len() - 1].to_vec();

            let new_vector = Self {
                length: self.length - 1,
                shift: self.shift,
                root: self.root.clone(),
                tail: ReferenceCounter::from(new_tail.as_slice()),
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

            let new_vector = Self {
                length: self.length - 1,
                shift: new_shift,
                root: new_root,
                tail: new_tail,
            };

            Some((new_vector, element))
        }
    }

    /// Gets the leaf at the given offset.
    fn get_leaf_at(&self, offset: usize) -> ReferenceCounter<[T]> {
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
                        return ReferenceCounter::from([].as_slice());
                    }
                }
                Node::Leaf(_) => break,
            }
        }

        match node.as_ref() {
            Node::Leaf(elements) => elements.clone(),
            Node::Branch(_) => ReferenceCounter::from([].as_slice()),
        }
    }

    /// Removes the tail from the root.
    fn pop_tail_from_root(&self) -> (ReferenceCounter<Node<T>>, usize) {
        let tail_offset = self.length - 2; // Last valid index after pop
        let (new_root, _) = Self::do_pop_tail(&self.root, self.shift, tail_offset);

        // Check if we should reduce tree depth
        match new_root.as_ref() {
            Node::Branch(children) => {
                if self.shift > BITS_PER_LEVEL {
                    // Count non-None children
                    let non_none_count = children.iter().filter(|c| c.is_some()).count();
                    if non_none_count == 1
                        && let Some(only_child) = &children[0]
                    {
                        return (only_child.clone(), self.shift - BITS_PER_LEVEL);
                    }
                }
                (new_root, self.shift)
            }
            Node::Leaf(_) => (new_root, self.shift),
        }
    }

    /// Recursively pops the tail from the tree.
    fn do_pop_tail(
        node: &ReferenceCounter<Node<T>>,
        level: usize,
        offset: usize,
    ) -> (ReferenceCounter<Node<T>>, bool) {
        let subindex = (offset >> level) & MASK;

        match node.as_ref() {
            Node::Branch(children) => {
                if level == BITS_PER_LEVEL {
                    // At bottom level, remove the child
                    let mut new_children = children.as_ref().clone();
                    new_children[subindex] = None;

                    let all_none = new_children.iter().all(|c| c.is_none());
                    (
                        ReferenceCounter::new(Node::Branch(ReferenceCounter::new(new_children))),
                        all_none,
                    )
                } else if let Some(child) = &children[subindex] {
                    let (new_child, is_empty) =
                        Self::do_pop_tail(child, level - BITS_PER_LEVEL, offset);
                    let mut new_children = children.as_ref().clone();

                    if is_empty {
                        new_children[subindex] = None;
                    } else {
                        new_children[subindex] = Some(new_child);
                    }

                    let all_none = new_children.iter().all(|c| c.is_none());
                    (
                        ReferenceCounter::new(Node::Branch(ReferenceCounter::new(new_children))),
                        all_none,
                    )
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
        for item in self {
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
        let new_vector: Self = self.iter().skip(1).cloned().collect();

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

            Some(Self {
                length: self.length,
                shift: self.shift,
                root: self.root.clone(),
                tail: ReferenceCounter::from(new_tail.as_slice()),
            })
        } else {
            // Element is in the root
            let new_root = Self::update_in_root(&self.root, self.shift, index, element);

            Some(Self {
                length: self.length,
                shift: self.shift,
                root: ReferenceCounter::new(new_root),
                tail: self.tail.clone(),
            })
        }
    }

    /// Updates an element in the root tree.
    fn update_in_root(
        node: &ReferenceCounter<Node<T>>,
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
                        new_children[subindex] = Some(ReferenceCounter::new(Self::update_in_root(
                            child,
                            level - BITS_PER_LEVEL,
                            index,
                            element,
                        )));
                    }
                } else if let Some(child) = &children[subindex] {
                    new_children[subindex] = Some(ReferenceCounter::new(Self::update_in_root(
                        child, 0, index, element,
                    )));
                }

                Node::Branch(ReferenceCounter::new(new_children))
            }
            Node::Leaf(elements) => {
                let leaf_index = index & MASK;
                let mut new_elements = elements.to_vec();
                if leaf_index < new_elements.len() {
                    new_elements[leaf_index] = element;
                }
                Node::Leaf(ReferenceCounter::from(new_elements.as_slice()))
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
        for element in other {
            result = result.push_back(element.clone());
        }
        result
    }

    /// Returns a new vector containing the first `count` elements.
    ///
    /// If `count` exceeds the vector's length, returns a copy of the entire vector.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements to take from the front
    ///
    /// # Complexity
    ///
    /// O(min(n, count))
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let taken = vector.take(3);
    /// // taken = [1, 2, 3]
    ///
    /// let over = vector.take(10);
    /// // over = [1, 2, 3, 4, 5] (entire vector)
    ///
    /// let zero = vector.take(0);
    /// // zero = []
    /// ```
    #[must_use]
    pub fn take(&self, count: usize) -> Self {
        let actual_count = count.min(self.len());
        self.slice(0, actual_count)
    }

    /// Returns a new vector with the first `count` elements removed.
    ///
    /// If `count` exceeds the vector's length, returns an empty vector.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements to skip from the front
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let dropped = vector.drop_first(2);
    /// // dropped = [3, 4, 5]
    ///
    /// let all_dropped = vector.drop_first(10);
    /// // all_dropped = []
    ///
    /// let none_dropped = vector.drop_first(0);
    /// // none_dropped = [1, 2, 3, 4, 5]
    /// ```
    #[must_use]
    pub fn drop_first(&self, count: usize) -> Self {
        if count >= self.len() {
            Self::new()
        } else {
            self.slice(count, self.len())
        }
    }

    /// Splits the vector at the given index.
    ///
    /// Returns a tuple of two vectors: the first contains elements before the index,
    /// and the second contains elements from the index onward.
    ///
    /// This is equivalent to `(self.take(index), self.drop_first(index))`.
    ///
    /// # Arguments
    ///
    /// * `index` - The position at which to split the vector
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let (left, right) = vector.split_at(2);
    /// // left = [1, 2]
    /// // right = [3, 4, 5]
    ///
    /// let (empty_left, all) = vector.split_at(0);
    /// // empty_left = []
    /// // all = [1, 2, 3, 4, 5]
    /// ```
    #[must_use]
    pub fn split_at(&self, index: usize) -> (Self, Self) {
        (self.take(index), self.drop_first(index))
    }

    /// Folds the vector using the first element as the initial accumulator.
    ///
    /// Returns `None` if the vector is empty, otherwise returns `Some(result)`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that combines the accumulator with each element
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let sum = vector.fold_left1(|accumulator, x| accumulator + x);
    /// // sum = Some(15)
    ///
    /// let empty: PersistentVector<i32> = PersistentVector::new();
    /// let result = empty.fold_left1(|accumulator, x| accumulator + x);
    /// // result = None
    /// ```
    #[must_use]
    pub fn fold_left1<F>(&self, mut function: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        let mut iter = self.iter();
        let first = iter.next()?.clone();
        Some(iter.fold(first, |accumulator, x| function(accumulator, x.clone())))
    }

    /// Folds the vector from the right using the last element as the initial accumulator.
    ///
    /// Returns `None` if the vector is empty, otherwise returns `Some(result)`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that combines each element with the accumulator
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=5).collect();
    /// let sum = vector.fold_right1(|x, accumulator| x + accumulator);
    /// // sum = Some(15)
    ///
    /// let vector2: PersistentVector<i32> = (1..=4).collect();
    /// let result = vector2.fold_right1(|x, accumulator| x - accumulator);
    /// // result = Some(1 - (2 - (3 - 4))) = Some(-2)
    /// ```
    #[must_use]
    pub fn fold_right1<F>(&self, mut function: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        let elements: Vec<T> = self.iter().cloned().collect();
        let mut iter = elements.into_iter().rev();
        let last = iter.next()?;
        Some(iter.fold(last, |accumulator, x| function(x, accumulator)))
    }

    /// Returns a vector of intermediate accumulator values from a left fold.
    ///
    /// The returned vector starts with the initial value and includes each
    /// intermediate result of applying the function.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial accumulator value
    /// * `function` - A function that combines the accumulator with each element
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=4).collect();
    /// let scanned = vector.scan_left(0, |accumulator, x| accumulator + x);
    /// // scanned = [0, 1, 3, 6, 10]
    ///
    /// let empty: PersistentVector<i32> = PersistentVector::new();
    /// let scanned_empty = empty.scan_left(0, |accumulator, x| accumulator + x);
    /// // scanned_empty = [0]
    /// ```
    #[must_use]
    pub fn scan_left<B, F>(&self, initial: B, mut function: F) -> PersistentVector<B>
    where
        B: Clone,
        F: FnMut(B, &T) -> B,
    {
        let mut results = Vec::with_capacity(self.len() + 1);
        let mut accumulator = initial;
        results.push(accumulator.clone());

        for element in self {
            accumulator = function(accumulator, element);
            results.push(accumulator.clone());
        }

        results.into_iter().collect()
    }

    /// Partitions the vector into two vectors based on a predicate.
    ///
    /// Returns a tuple where the first vector contains elements for which the
    /// predicate returns `true`, and the second vector contains elements for
    /// which it returns `false`. Order is preserved in both vectors.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns `true` for elements to include
    ///   in the first vector
    ///
    /// # Complexity
    ///
    /// O(n)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=6).collect();
    /// let (evens, odds) = vector.partition(|x| x % 2 == 0);
    /// // evens = [2, 4, 6]
    /// // odds = [1, 3, 5]
    /// ```
    #[must_use]
    pub fn partition<P>(&self, predicate: P) -> (Self, Self)
    where
        P: Fn(&T) -> bool,
    {
        let mut pass = Vec::new();
        let mut fail = Vec::new();

        for element in self {
            if predicate(element) {
                pass.push(element.clone());
            } else {
                fail.push(element.clone());
            }
        }

        (pass.into_iter().collect(), fail.into_iter().collect())
    }

    /// Zips this vector with another vector into a vector of pairs.
    ///
    /// The resulting vector has the length of the shorter input vector.
    /// If either vector is empty, returns an empty vector.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to zip with
    ///
    /// # Complexity
    ///
    /// O(min(n, m)) where n and m are the lengths of the two vectors
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector1: PersistentVector<i32> = (1..=3).collect();
    /// let vector2: PersistentVector<char> = vec!['a', 'b', 'c'].into_iter().collect();
    /// let zipped = vector1.zip(&vector2);
    /// // zipped = [(1, 'a'), (2, 'b'), (3, 'c')]
    ///
    /// // Different lengths
    /// let short: PersistentVector<i32> = (1..=2).collect();
    /// let zipped_short = short.zip(&vector2);
    /// // zipped_short = [(1, 'a'), (2, 'b')]
    /// ```
    #[must_use]
    pub fn zip<U: Clone>(&self, other: &PersistentVector<U>) -> PersistentVector<(T, U)> {
        self.iter()
            .zip(other.iter())
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect()
    }

    /// Returns a new vector with the separator inserted between each element.
    ///
    /// # Arguments
    ///
    /// * `separator` - The element to insert between each pair of elements
    ///
    /// # Returns
    ///
    /// A new vector with separators inserted between elements. Returns an empty vector
    /// if the original vector is empty, and returns a single-element vector unchanged.
    ///
    /// # Complexity
    ///
    /// O(n) time and space
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let vector: PersistentVector<i32> = (1..=4).collect();
    /// let result = vector.intersperse(0);
    ///
    /// let collected: Vec<i32> = result.iter().cloned().collect();
    /// assert_eq!(collected, vec![1, 0, 2, 0, 3, 0, 4]);
    /// ```
    #[must_use]
    pub fn intersperse(&self, separator: T) -> Self {
        let mut iter = self.iter();
        let Some(first) = iter.next() else {
            return Self::new();
        };

        let result_length = self.len() * 2 - 1;
        let mut result = Vec::with_capacity(result_length);
        result.push(first.clone());

        for element in iter {
            result.push(separator.clone());
            result.push(element.clone());
        }

        result.into_iter().collect()
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
            return Self::new();
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
// Specialized Methods for Tuple Elements
// =============================================================================

impl<A: Clone, B: Clone> PersistentVector<(A, B)> {
    /// Separates a vector of pairs into two vectors.
    ///
    /// This is the inverse operation of [`zip`].
    ///
    /// # Returns
    ///
    /// A tuple containing two vectors: one with all first elements and one with all
    /// second elements.
    ///
    /// # Complexity
    ///
    /// O(n) time and space
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let pairs: PersistentVector<(i32, char)> =
    ///     vec![(1, 'a'), (2, 'b'), (3, 'c')].into_iter().collect();
    /// let (numbers, chars) = pairs.unzip();
    ///
    /// let numbers_collected: Vec<i32> = numbers.iter().cloned().collect();
    /// let chars_collected: Vec<char> = chars.iter().cloned().collect();
    /// assert_eq!(numbers_collected, vec![1, 2, 3]);
    /// assert_eq!(chars_collected, vec!['a', 'b', 'c']);
    /// ```
    ///
    /// [`zip`]: PersistentVector::zip
    #[must_use]
    pub fn unzip(&self) -> (PersistentVector<A>, PersistentVector<B>) {
        let mut first_elements = Vec::with_capacity(self.len());
        let mut second_elements = Vec::with_capacity(self.len());
        for (a, b) in self {
            first_elements.push(a.clone());
            second_elements.push(b.clone());
        }
        (
            first_elements.into_iter().collect(),
            second_elements.into_iter().collect(),
        )
    }
}

// =============================================================================
// Specialized Methods for Nested Vectors
// =============================================================================

impl<T: Clone> PersistentVector<PersistentVector<T>> {
    /// Inserts a separator vector between each inner vector and flattens the result.
    ///
    /// This is equivalent to `intersperse` followed by `flatten`.
    ///
    /// # Arguments
    ///
    /// * `separator` - The vector to insert between each pair of inner vectors
    ///
    /// # Returns
    ///
    /// A flattened vector with separators inserted between the original inner vectors.
    ///
    /// # Complexity
    ///
    /// O(n * m) time and space, where n is the number of inner vectors and m is
    /// the average length of inner vectors
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let inner1: PersistentVector<i32> = vec![1, 2].into_iter().collect();
    /// let inner2: PersistentVector<i32> = vec![3, 4].into_iter().collect();
    /// let outer: PersistentVector<PersistentVector<i32>> =
    ///     vec![inner1, inner2].into_iter().collect();
    /// let separator: PersistentVector<i32> = vec![0].into_iter().collect();
    /// let result = outer.intercalate(&separator);
    ///
    /// let collected: Vec<i32> = result.iter().cloned().collect();
    /// assert_eq!(collected, vec![1, 2, 0, 3, 4]);
    /// ```
    #[must_use]
    pub fn intercalate(&self, separator: &PersistentVector<T>) -> PersistentVector<T> {
        let mut iter = self.iter();
        let Some(first) = iter.next() else {
            return PersistentVector::new();
        };

        let mut result: Vec<T> = first.iter().cloned().collect();

        for inner in iter {
            result.extend(separator.iter().cloned());
            result.extend(inner.iter().cloned());
        }

        result.into_iter().collect()
    }
}

// =============================================================================
// Iterator Implementation
// =============================================================================

/// The processing state of the iterator.
///
/// Tracks whether the iterator is traversing the tree, processing the tail,
/// or has finished iterating.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IteratorState {
    /// Currently traversing the tree (root) structure
    TraversingTree,
    /// Currently processing elements in the tail buffer
    ProcessingTail,
    /// All elements have been consumed
    Exhausted,
}

/// A stack entry for tree traversal.
///
/// Holds a reference to a branch node's children array and tracks
/// which child index to process next. This enables depth-first traversal
/// with efficient backtracking.
struct TraversalStackEntry<'a, T> {
    /// Reference to the branch node's children array
    children: &'a [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR],
    /// Index of the next child to process
    child_index: usize,
}

/// An iterator over references to elements of a [`PersistentVector`].
///
/// This iterator uses a stack-based tree traversal algorithm to achieve
/// O(N) iteration complexity instead of O(N log32 N). It maintains a cache
/// of the current leaf node for efficient sequential access.
pub struct PersistentVectorIterator<'a, T> {
    /// Reference to the original vector (for lifetime and metadata)
    vector: &'a PersistentVector<T>,
    /// Stack for tree traversal (maximum depth is 7 for practical sizes)
    traversal_stack: Vec<TraversalStackEntry<'a, T>>,
    /// Currently cached leaf node elements
    current_leaf: Option<&'a [T]>,
    /// Current position within the cached leaf
    leaf_index: usize,
    /// Current processing state
    state: IteratorState,
    /// Current position within the tail buffer
    tail_index: usize,
    /// Number of elements already returned (for `ExactSizeIterator`)
    elements_returned: usize,
}

impl<'a, T> PersistentVectorIterator<'a, T> {
    /// Creates a new optimized iterator for the given vector.
    fn new(vector: &'a PersistentVector<T>) -> Self {
        if vector.is_empty() {
            return Self {
                vector,
                traversal_stack: Vec::new(),
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::Exhausted,
                tail_index: 0,
                elements_returned: 0,
            };
        }

        // Calculate the number of elements in the tree (excluding tail)
        let tail_offset = vector.tail_offset();

        if tail_offset == 0 {
            // All elements are in the tail
            Self {
                vector,
                traversal_stack: Vec::new(),
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::ProcessingTail,
                tail_index: 0,
                elements_returned: 0,
            }
        } else {
            // Elements exist in the tree, start traversal from root
            let mut iterator = Self {
                vector,
                traversal_stack: Vec::with_capacity(7), // Maximum depth is 7
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::TraversingTree,
                tail_index: 0,
                elements_returned: 0,
            };
            iterator.initialize_from_root();
            iterator
        }
    }

    /// Initializes the iterator from the root node.
    ///
    /// Pushes the root branch onto the stack and descends to the first leaf.
    fn initialize_from_root(&mut self) {
        match self.vector.root.as_ref() {
            Node::Branch(children) => {
                self.traversal_stack.push(TraversalStackEntry {
                    children: children.as_ref(),
                    child_index: 0,
                });
                self.descend_to_first_leaf();
            }
            Node::Leaf(elements) => {
                // Root is directly a leaf (unusual but handle for safety)
                self.current_leaf = Some(elements.as_ref());
                self.leaf_index = 0;
            }
        }
    }

    /// Descends from the current stack top to the first leaf node.
    ///
    /// Traverses the tree depth-first, skipping None children, until
    /// a leaf node is found.
    fn descend_to_first_leaf(&mut self) {
        loop {
            let stack_len = self.traversal_stack.len();
            if stack_len == 0 {
                break;
            }

            // Get current entry information
            let entry = &mut self.traversal_stack[stack_len - 1];

            // Find the first valid child in the current branch
            let mut found_branch: Option<
                &'a [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR],
            > = None;
            let mut found_leaf: Option<&'a [T]> = None;

            while entry.child_index < BRANCHING_FACTOR {
                let index = entry.child_index;
                entry.child_index += 1;

                if let Some(child) = &entry.children[index] {
                    match child.as_ref() {
                        Node::Branch(child_children) => {
                            found_branch = Some(child_children.as_ref());
                            break;
                        }
                        Node::Leaf(elements) => {
                            found_leaf = Some(elements.as_ref());
                            break;
                        }
                    }
                }
            }

            if let Some(leaf) = found_leaf {
                self.current_leaf = Some(leaf);
                self.leaf_index = 0;
                return;
            }

            if let Some(branch) = found_branch {
                self.traversal_stack.push(TraversalStackEntry {
                    children: branch,
                    child_index: 0,
                });
                continue;
            }

            // All children processed, pop this entry
            self.traversal_stack.pop();
        }
    }

    /// Advances to the next leaf node.
    ///
    /// Called when the current leaf is exhausted. Backtracks through the
    /// stack to find the next unvisited subtree and descends to its first leaf.
    /// Transitions to tail processing if no more leaves exist in the tree.
    fn advance_to_next_leaf(&mut self) {
        self.current_leaf = None;
        self.leaf_index = 0;

        // Use the same pattern as descend_to_first_leaf
        self.descend_to_first_leaf();

        // If no leaf was found, transition to tail
        if self.current_leaf.is_none() {
            self.state = IteratorState::ProcessingTail;
            self.tail_index = 0;
        }
    }
}

impl<'a, T> Iterator for PersistentVectorIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                IteratorState::TraversingTree => {
                    if let Some(leaf) = self.current_leaf {
                        if self.leaf_index < leaf.len() {
                            let element = &leaf[self.leaf_index];
                            self.leaf_index += 1;
                            self.elements_returned += 1;
                            return Some(element);
                        }
                        // Current leaf is exhausted, move to next leaf
                        self.advance_to_next_leaf();
                    } else {
                        // No leaf available (empty tree or initialization issue)
                        self.state = IteratorState::ProcessingTail;
                        self.tail_index = 0;
                    }
                }
                IteratorState::ProcessingTail => {
                    if self.tail_index < self.vector.tail.len() {
                        let element = &self.vector.tail[self.tail_index];
                        self.tail_index += 1;
                        self.elements_returned += 1;
                        return Some(element);
                    }
                    // Tail is also exhausted
                    self.state = IteratorState::Exhausted;
                    return None;
                }
                IteratorState::Exhausted => {
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vector.length.saturating_sub(self.elements_returned);
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for PersistentVectorIterator<'_, T> {
    fn len(&self) -> usize {
        self.vector.length.saturating_sub(self.elements_returned)
    }
}

/// A stack entry for tree traversal in the owning iterator.
///
/// Unlike `TraversalStackEntry`, this holds an `ReferenceCounter<Node<T>>` directly
/// to avoid lifetime issues with owned data.
struct IntoIteratorStackEntry<T> {
    /// The branch node (held via reference counting)
    node: ReferenceCounter<Node<T>>,
    /// Index of the next child to process
    child_index: usize,
}

/// An owning iterator over elements of a [`PersistentVector`].
///
/// This iterator uses a stack-based tree traversal algorithm to achieve
/// O(N) iteration complexity. Elements are cloned from the tree as they
/// are returned.
pub struct PersistentVectorIntoIterator<T> {
    /// The original vector (for accessing the tail)
    vector: PersistentVector<T>,
    /// Stack for tree traversal
    traversal_stack: Vec<IntoIteratorStackEntry<T>>,
    /// Currently cached leaf node (held via reference counting)
    current_leaf: Option<ReferenceCounter<[T]>>,
    /// Current position within the cached leaf
    leaf_index: usize,
    /// Current processing state
    state: IteratorState,
    /// Current position within the tail buffer
    tail_index: usize,
    /// Number of elements already returned
    elements_returned: usize,
}

impl<T: Clone> PersistentVectorIntoIterator<T> {
    /// Creates a new optimized owning iterator for the given vector.
    fn new(vector: PersistentVector<T>) -> Self {
        if vector.is_empty() {
            return Self {
                vector,
                traversal_stack: Vec::new(),
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::Exhausted,
                tail_index: 0,
                elements_returned: 0,
            };
        }

        let tail_offset = vector.tail_offset();

        if tail_offset == 0 {
            // All elements are in the tail
            Self {
                vector,
                traversal_stack: Vec::new(),
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::ProcessingTail,
                tail_index: 0,
                elements_returned: 0,
            }
        } else {
            // Elements exist in the tree
            let root_clone = vector.root.clone();
            let mut iterator = Self {
                vector,
                traversal_stack: Vec::with_capacity(7),
                current_leaf: None,
                leaf_index: 0,
                state: IteratorState::TraversingTree,
                tail_index: 0,
                elements_returned: 0,
            };
            iterator.initialize_from_root(root_clone);
            iterator
        }
    }

    /// Initializes the iterator from the root node.
    fn initialize_from_root(&mut self, root: ReferenceCounter<Node<T>>) {
        match root.as_ref() {
            Node::Branch(_) => {
                self.traversal_stack.push(IntoIteratorStackEntry {
                    node: root,
                    child_index: 0,
                });
                self.descend_to_first_leaf();
            }
            Node::Leaf(elements) => {
                self.current_leaf = Some(elements.clone());
                self.leaf_index = 0;
            }
        }
    }

    /// Descends from the current stack top to the first leaf node.
    fn descend_to_first_leaf(&mut self) {
        loop {
            let stack_len = self.traversal_stack.len();
            if stack_len == 0 {
                break;
            }

            let entry = &mut self.traversal_stack[stack_len - 1];

            let children = match entry.node.as_ref() {
                Node::Branch(c) => c,
                Node::Leaf(_) => {
                    self.traversal_stack.pop();
                    continue;
                }
            };

            let mut found_branch: Option<ReferenceCounter<Node<T>>> = None;
            let mut found_leaf: Option<ReferenceCounter<[T]>> = None;

            while entry.child_index < BRANCHING_FACTOR {
                let index = entry.child_index;
                entry.child_index += 1;

                if let Some(child) = &children[index] {
                    match child.as_ref() {
                        Node::Branch(_) => {
                            found_branch = Some(child.clone());
                            break;
                        }
                        Node::Leaf(elements) => {
                            found_leaf = Some(elements.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(leaf) = found_leaf {
                self.current_leaf = Some(leaf);
                self.leaf_index = 0;
                return;
            }

            if let Some(branch) = found_branch {
                self.traversal_stack.push(IntoIteratorStackEntry {
                    node: branch,
                    child_index: 0,
                });
                continue;
            }

            // All children processed, pop this entry
            self.traversal_stack.pop();
        }
    }

    /// Advances to the next leaf node.
    fn advance_to_next_leaf(&mut self) {
        self.current_leaf = None;
        self.leaf_index = 0;

        // Use the same pattern as descend_to_first_leaf
        self.descend_to_first_leaf();

        // If no leaf was found, transition to tail
        if self.current_leaf.is_none() {
            self.state = IteratorState::ProcessingTail;
            self.tail_index = 0;
        }
    }
}

impl<T: Clone> Iterator for PersistentVectorIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                IteratorState::TraversingTree => {
                    if let Some(ref leaf) = self.current_leaf {
                        if self.leaf_index < leaf.len() {
                            let element = leaf[self.leaf_index].clone();
                            self.leaf_index += 1;
                            self.elements_returned += 1;
                            return Some(element);
                        }
                        self.advance_to_next_leaf();
                    } else {
                        self.state = IteratorState::ProcessingTail;
                        self.tail_index = 0;
                    }
                }
                IteratorState::ProcessingTail => {
                    if self.tail_index < self.vector.tail.len() {
                        let element = self.vector.tail[self.tail_index].clone();
                        self.tail_index += 1;
                        self.elements_returned += 1;
                        return Some(element);
                    }
                    self.state = IteratorState::Exhausted;
                    return None;
                }
                IteratorState::Exhausted => {
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vector.length.saturating_sub(self.elements_returned);
        (remaining, Some(remaining))
    }
}

impl<T: Clone> ExactSizeIterator for PersistentVectorIntoIterator<T> {
    fn len(&self) -> usize {
        self.vector.length.saturating_sub(self.elements_returned)
    }
}

// =============================================================================
// Standard Trait Implementations
// =============================================================================

impl<T> Default for PersistentVector<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> FromIterator<T> for PersistentVector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let elements: Vec<T> = iter.into_iter().collect();
        build_persistent_vector_from_vec(elements)
    }
}

impl<T: Clone> IntoIterator for PersistentVector<T> {
    type Item = T;
    type IntoIter = PersistentVectorIntoIterator<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        PersistentVectorIntoIterator::new(self)
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

/// Compares two vectors lexicographically for partial ordering.
///
/// This implementation enables comparison operators (`<`, `<=`, `>`, `>=`) for
/// `PersistentVector<T>` when the element type `T` supports partial ordering.
///
/// The comparison follows lexicographic ordering:
/// 1. Compare elements pairwise from the beginning
/// 2. The first non-equal pair determines the ordering
/// 3. If all compared elements are equal:
///    - A shorter vector is less than a longer vector
///    - Vectors of equal length are equal
///
/// Returns `None` if any element comparison returns `None` (e.g., when
/// comparing `NaN` values in floating-point types).
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentVector;
/// use std::cmp::Ordering;
///
/// let vector1: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
/// let vector2: PersistentVector<i32> = vec![1, 2, 4].into_iter().collect();
/// assert_eq!(vector1.partial_cmp(&vector2), Some(Ordering::Less));
///
/// let prefix: PersistentVector<i32> = vec![1, 2].into_iter().collect();
/// let extended: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
/// assert_eq!(prefix.partial_cmp(&extended), Some(Ordering::Less));
/// ```
impl<T: PartialOrd> PartialOrd for PersistentVector<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

/// Compares two vectors lexicographically for total ordering.
///
/// This implementation enables `PersistentVector<T>` to be used as keys in
/// `BTreeMap`/`BTreeSet` and to be sorted when the element type `T` supports
/// total ordering.
///
/// The comparison follows lexicographic ordering, identical to `PartialOrd`,
/// but always returns a definite `Ordering` since `T: Ord` guarantees total
/// ordering for all elements.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentVector;
/// use std::collections::BTreeSet;
///
/// let mut set: BTreeSet<PersistentVector<i32>> = BTreeSet::new();
/// set.insert(vec![1, 2, 3].into_iter().collect());
/// set.insert(vec![1, 2, 2].into_iter().collect());
/// set.insert(vec![1, 2, 4].into_iter().collect());
///
/// let ordered: Vec<PersistentVector<i32>> = set.iter().cloned().collect();
/// assert_eq!(ordered[0], vec![1, 2, 2].into_iter().collect());
/// assert_eq!(ordered[1], vec![1, 2, 3].into_iter().collect());
/// assert_eq!(ordered[2], vec![1, 2, 4].into_iter().collect());
/// ```
impl<T: Ord> Ord for PersistentVector<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.iter().cmp(other.iter())
    }
}

/// Computes a hash value for this vector.
///
/// The hash is computed by first hashing the length, then hashing each
/// element in order using the O(N) iterator. This ensures that:
///
/// - Vectors with different lengths have different hashes (with high probability)
/// - The order of elements affects the hash value
/// - Equal vectors produce equal hash values (Hash-Eq consistency)
///
/// # Complexity
///
/// O(N) where N is the number of elements, using the optimized stack-based
/// iterator implemented in Phase 8.1.
///
/// # Examples
///
/// ```rust
/// use lambars::persistent::PersistentVector;
/// use std::collections::HashMap;
///
/// let mut map: HashMap<PersistentVector<i32>, &str> = HashMap::new();
/// let key: PersistentVector<i32> = (1..=3).collect();
/// map.insert(key.clone(), "value");
/// assert_eq!(map.get(&key), Some(&"value"));
/// ```
impl<T: Hash> Hash for PersistentVector<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the length first to distinguish vectors of different lengths
        self.length.hash(state);
        // Hash each element in order (using O(N) iterator)
        for element in self {
            element.hash(state);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for PersistentVector<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl<T: fmt::Display> fmt::Display for PersistentVector<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[")?;
        let mut first = true;
        for element in self {
            if first {
                first = false;
            } else {
                write!(formatter, ", ")?;
            }
            write!(formatter, "{element}")?;
        }
        write!(formatter, "]")
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
        self.get(0).map_or_else(PersistentVector::new, |first| {
            PersistentVector::singleton(function(first.clone()))
        })
    }

    fn fmap_ref<B, F>(&self, function: F) -> PersistentVector<B>
    where
        F: FnOnce(&T) -> B,
    {
        self.get(0).map_or_else(PersistentVector::new, |first| {
            PersistentVector::singleton(function(first))
        })
    }
}

impl<T: Clone> FunctorMut for PersistentVector<T> {
    fn fmap_mut<B, F>(self, function: F) -> PersistentVector<B>
    where
        F: FnMut(T) -> B,
    {
        build_persistent_vector_from_iter(self.into_iter().map(function))
    }

    fn fmap_ref_mut<B, F>(&self, function: F) -> PersistentVector<B>
    where
        F: FnMut(&T) -> B,
    {
        build_persistent_vector_from_iter(self.iter().map(function))
    }
}

/// Helper function to build a `PersistentVector` from an iterator without requiring Clone.
fn build_persistent_vector_from_iter<T, I>(iter: I) -> PersistentVector<T>
where
    I: Iterator<Item = T>,
{
    let elements: Vec<T> = iter.collect();
    build_persistent_vector_from_vec(elements)
}

/// Helper function to build a `PersistentVector` from a Vec without requiring Clone.
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
            root: ReferenceCounter::new(Node::empty_branch()),
            tail: ReferenceCounter::from(elements),
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
        tail: ReferenceCounter::from(tail_elements),
    }
}

/// Build the root tree from a vector of elements.
fn build_root_from_elements<T>(elements: Vec<T>) -> (ReferenceCounter<Node<T>>, usize) {
    if elements.is_empty() {
        return (ReferenceCounter::new(Node::empty_branch()), BITS_PER_LEVEL);
    }

    // Split into chunks of BRANCHING_FACTOR
    let mut leaves: Vec<ReferenceCounter<Node<T>>> = Vec::new();
    let mut iter = elements.into_iter();

    loop {
        let chunk: Vec<T> = iter.by_ref().take(BRANCHING_FACTOR).collect();
        if chunk.is_empty() {
            break;
        }
        leaves.push(ReferenceCounter::new(Node::Leaf(ReferenceCounter::from(
            chunk,
        ))));
    }

    // If there's only one leaf, wrap it in a branch
    if leaves.len() == 1 {
        let mut children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
            std::array::from_fn(|_| None);
        children[0] = Some(leaves.remove(0));
        return (
            ReferenceCounter::new(Node::Branch(ReferenceCounter::new(children))),
            BITS_PER_LEVEL,
        );
    }

    // Build tree bottom-up
    let mut current_level = leaves;
    let mut shift = BITS_PER_LEVEL;

    while current_level.len() > BRANCHING_FACTOR {
        let mut next_level: Vec<ReferenceCounter<Node<T>>> = Vec::new();

        for chunk in current_level.chunks(BRANCHING_FACTOR) {
            let mut children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            for (index, node) in chunk.iter().enumerate() {
                children[index] = Some(node.clone());
            }
            next_level.push(ReferenceCounter::new(Node::Branch(ReferenceCounter::new(
                children,
            ))));
        }

        current_level = next_level;
        shift += BITS_PER_LEVEL;
    }

    // Wrap the remaining nodes in the root branch
    let mut root_children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
        std::array::from_fn(|_| None);
    for (index, node) in current_level.into_iter().enumerate() {
        root_children[index] = Some(node);
    }

    (
        ReferenceCounter::new(Node::Branch(ReferenceCounter::new(root_children))),
        shift,
    )
}

// =============================================================================
// TransientVector Implementation
// =============================================================================

impl<T: Clone> TransientVector<T> {
    /// Creates a new empty `TransientVector`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let transient: TransientVector<i32> = TransientVector::new();
    /// assert!(transient.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: ReferenceCounter::new(Node::empty_branch()),
            tail: Vec::new(),
            length: 0,
            shift: BITS_PER_LEVEL,
            _marker: PhantomData,
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
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// assert_eq!(transient.len(), 0);
    /// transient.push_back(1);
    /// assert_eq!(transient.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// assert!(transient.is_empty());
    /// transient.push_back(1);
    /// assert!(!transient.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns the starting index of the tail buffer.
    #[inline]
    const fn tail_offset(&self) -> usize {
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
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// assert_eq!(transient.get(0), Some(&1));
    /// assert_eq!(transient.get(1), Some(&2));
    /// assert_eq!(transient.get(10), None);
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
                children[child_index]
                    .as_ref()
                    .and_then(|child| match child.as_ref() {
                        Node::Leaf(elements) => elements.first(),
                        Node::Branch(_) => None,
                    })
            }
        }
    }

    /// Appends an element to the back of the vector.
    ///
    /// This method mutates `self` in place, unlike `PersistentVector::push_back`
    /// which returns a new vector.
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
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// transient.push_back(3);
    /// assert_eq!(transient.len(), 3);
    /// ```
    pub fn push_back(&mut self, element: T) {
        if self.tail.len() < BRANCHING_FACTOR {
            // Tail has space, just add to tail
            self.tail.push(element);
        } else {
            // Tail is full, push tail to root and create new tail
            self.push_tail_to_root();
            self.tail = vec![element];
        }
        self.length += 1;
    }

    /// Pushes the current tail into the root tree.
    fn push_tail_to_root(&mut self) {
        let tail_leaf = Node::Leaf(ReferenceCounter::from(std::mem::take(&mut self.tail)));
        let tail_offset = self.tail_offset();

        // Check if we need to increase the tree depth
        let root_overflow = (tail_offset >> self.shift) >= BRANCHING_FACTOR;

        if root_overflow {
            // Create a new root level
            let mut new_root_children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            new_root_children[0] = Some(self.root.clone());
            new_root_children[1] =
                Some(ReferenceCounter::new(Self::new_path(self.shift, tail_leaf)));

            self.root =
                ReferenceCounter::new(Node::Branch(ReferenceCounter::new(new_root_children)));
            self.shift += BITS_PER_LEVEL;
        } else {
            // Push tail into existing root using COW
            self.push_tail_into_root_cow(tail_offset, tail_leaf);
        }
    }

    /// Creates a new path from root to the leaf.
    fn new_path(level: usize, node: Node<T>) -> Node<T> {
        if level == 0 {
            node
        } else {
            let mut children: [Option<ReferenceCounter<Node<T>>>; BRANCHING_FACTOR] =
                std::array::from_fn(|_| None);
            children[0] = Some(ReferenceCounter::new(Self::new_path(
                level - BITS_PER_LEVEL,
                node,
            )));
            Node::Branch(ReferenceCounter::new(children))
        }
    }

    /// Pushes a tail leaf into the tree using Copy-on-Write semantics.
    fn push_tail_into_root_cow(&mut self, tail_offset: usize, tail_node: Node<T>) {
        // Use Rc::make_mut / Arc::make_mut for COW
        let root = ReferenceCounter::make_mut(&mut self.root);
        Self::push_tail_into_node_cow(root, self.shift, tail_offset, tail_node);
    }

    /// Recursively pushes a tail node into the tree with COW.
    fn push_tail_into_node_cow(
        node: &mut Node<T>,
        level: usize,
        tail_offset: usize,
        tail_node: Node<T>,
    ) {
        let subindex = (tail_offset >> level) & MASK;

        match node {
            Node::Branch(children) => {
                let children_mut = ReferenceCounter::make_mut(children);

                if level == BITS_PER_LEVEL {
                    // We're at the bottom branch level, insert the tail leaf
                    children_mut[subindex] = Some(ReferenceCounter::new(tail_node));
                } else {
                    // Recurse down
                    match &mut children_mut[subindex] {
                        Some(child) => {
                            let child_mut = ReferenceCounter::make_mut(child);
                            Self::push_tail_into_node_cow(
                                child_mut,
                                level - BITS_PER_LEVEL,
                                tail_offset,
                                tail_node,
                            );
                        }
                        None => {
                            children_mut[subindex] = Some(ReferenceCounter::new(Self::new_path(
                                level - BITS_PER_LEVEL,
                                tail_node,
                            )));
                        }
                    }
                }
            }
            Node::Leaf(_) => {
                // This shouldn't happen in a well-formed tree
                *node = tail_node;
            }
        }
    }

    /// Removes the last element from the vector and returns it.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// transient.push_back(3);
    /// assert_eq!(transient.pop_back(), Some(3));
    /// assert_eq!(transient.pop_back(), Some(2));
    /// assert_eq!(transient.len(), 1);
    /// ```
    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        if self.tail.len() > 1 {
            // Tail has more than one element, just pop from tail
            self.length -= 1;
            self.tail.pop()
        } else if !self.tail.is_empty() {
            // Tail has exactly one element
            if self.length == 1 {
                // This is the last element
                self.length = 0;
                self.tail.pop()
            } else {
                // Need to get a new tail from root
                let popped = self.tail.pop();
                let new_tail_offset = self.tail_offset().saturating_sub(BRANCHING_FACTOR);
                let new_tail = self.get_leaf_at(new_tail_offset);
                self.pop_tail_from_root_cow();
                self.tail = new_tail.to_vec();
                self.length -= 1;
                popped
            }
        } else {
            // Tail is empty (shouldn't happen in normal use, but handle defensively)
            None
        }
    }

    /// Gets the leaf at the given offset.
    fn get_leaf_at(&self, offset: usize) -> ReferenceCounter<[T]> {
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
                        return ReferenceCounter::from(Vec::<T>::new());
                    }
                }
                Node::Leaf(_) => break,
            }
        }

        match node.as_ref() {
            Node::Leaf(elements) => elements.clone(),
            Node::Branch(_) => ReferenceCounter::from(Vec::<T>::new()),
        }
    }

    /// Removes the tail from the root using COW.
    fn pop_tail_from_root_cow(&mut self) {
        let tail_offset = self.length - 2; // Last valid index after pop

        // Use COW to modify the root
        let root = ReferenceCounter::make_mut(&mut self.root);
        let is_empty = Self::do_pop_tail_cow(root, self.shift, tail_offset);

        // Check if we should reduce tree depth
        if is_empty && self.shift > BITS_PER_LEVEL {
            match &*self.root {
                Node::Branch(children) => {
                    let non_none_count = children.iter().filter(|c| c.is_some()).count();
                    if non_none_count == 1
                        && let Some(only_child) = &children[0]
                    {
                        self.root = only_child.clone();
                        self.shift -= BITS_PER_LEVEL;
                    }
                }
                Node::Leaf(_) => {}
            }
        }
    }

    /// Recursively pops the tail from the tree with COW.
    fn do_pop_tail_cow(node: &mut Node<T>, level: usize, offset: usize) -> bool {
        let subindex = (offset >> level) & MASK;

        match node {
            Node::Branch(children) => {
                let children_mut = ReferenceCounter::make_mut(children);

                if level == BITS_PER_LEVEL {
                    // At bottom level, remove the child
                    children_mut[subindex] = None;
                    children_mut.iter().all(|c| c.is_none())
                } else if let Some(child) = &mut children_mut[subindex] {
                    let child_mut = ReferenceCounter::make_mut(child);
                    let is_empty = Self::do_pop_tail_cow(child_mut, level - BITS_PER_LEVEL, offset);

                    if is_empty {
                        children_mut[subindex] = None;
                    }

                    children_mut.iter().all(|c| c.is_none())
                } else {
                    false
                }
            }
            Node::Leaf(_) => true,
        }
    }

    /// Updates the element at the given index.
    ///
    /// Returns `Some(old_element)` if the index is valid, `None` if out of bounds.
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
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// transient.push_back(3);
    /// let old = transient.update(1, 20);
    /// assert_eq!(old, Some(2));
    /// assert_eq!(transient.get(1), Some(&20));
    /// ```
    pub fn update(&mut self, index: usize, element: T) -> Option<T> {
        if index >= self.length {
            return None;
        }

        let tail_offset = self.tail_offset();

        if index >= tail_offset {
            // Element is in the tail
            let tail_index = index - tail_offset;
            let old = std::mem::replace(&mut self.tail[tail_index], element);
            Some(old)
        } else {
            // Element is in the root tree
            Some(self.update_in_root_cow(index, element))
        }
    }

    /// Updates an element in the root tree using COW.
    fn update_in_root_cow(&mut self, index: usize, element: T) -> T {
        let root = ReferenceCounter::make_mut(&mut self.root);
        Self::do_update_in_root_cow(root, self.shift, index, element)
    }

    /// Recursively updates an element in the tree with COW.
    fn do_update_in_root_cow(node: &mut Node<T>, level: usize, index: usize, element: T) -> T {
        match node {
            Node::Branch(children) => {
                let subindex = (index >> level) & MASK;
                let children_mut = ReferenceCounter::make_mut(children);

                if let Some(child) = &mut children_mut[subindex] {
                    let child_mut = ReferenceCounter::make_mut(child);
                    Self::do_update_in_root_cow(child_mut, level - BITS_PER_LEVEL, index, element)
                } else {
                    debug_assert!(
                        false,
                        "TransientVector internal invariant violation: missing child node at index {index}, level {level}"
                    );
                    element
                }
            }
            Node::Leaf(elements) => {
                let leaf_index = index & MASK;
                let elements_mut = ReferenceCounter::make_mut(elements);
                std::mem::replace(&mut elements_mut[leaf_index], element)
            }
        }
    }

    /// Updates the element at the given index using a function.
    ///
    /// Returns `true` if the update was successful, `false` if the index was out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index to update
    /// * `function` - A function that transforms the old element into the new element
    ///
    /// # Complexity
    ///
    /// O(log32 N)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// transient.push_back(3);
    /// assert!(transient.update_with(1, |x| x * 10));
    /// assert_eq!(transient.get(1), Some(&20));
    /// assert!(!transient.update_with(10, |x| x * 10));
    /// ```
    pub fn update_with<F>(&mut self, index: usize, function: F) -> bool
    where
        F: FnOnce(T) -> T,
    {
        if index >= self.length {
            return false;
        }

        let tail_offset = self.tail_offset();

        if index >= tail_offset {
            // Element is in the tail
            let tail_index = index - tail_offset;
            // Use clone to get the old value since T: Clone is already required
            let old = self.tail[tail_index].clone();
            self.tail[tail_index] = function(old);
        } else {
            // Element is in the root tree
            self.update_with_in_root_cow(index, function);
        }
        true
    }

    /// Updates an element in the root tree using a function with COW.
    fn update_with_in_root_cow<F>(&mut self, index: usize, function: F)
    where
        F: FnOnce(T) -> T,
    {
        let root = ReferenceCounter::make_mut(&mut self.root);
        Self::do_update_with_in_root_cow(root, self.shift, index, function);
    }

    /// Recursively updates an element in the tree using a function with COW.
    fn do_update_with_in_root_cow<F>(node: &mut Node<T>, level: usize, index: usize, function: F)
    where
        F: FnOnce(T) -> T,
    {
        match node {
            Node::Branch(children) => {
                let subindex = (index >> level) & MASK;
                let children_mut = ReferenceCounter::make_mut(children);

                if let Some(child) = &mut children_mut[subindex] {
                    let child_mut = ReferenceCounter::make_mut(child);
                    Self::do_update_with_in_root_cow(
                        child_mut,
                        level - BITS_PER_LEVEL,
                        index,
                        function,
                    );
                }
            }
            Node::Leaf(elements) => {
                let leaf_index = index & MASK;
                let elements_mut = ReferenceCounter::make_mut(elements);
                // Use clone to get the old value since T: Clone is already required
                let old = elements_mut[leaf_index].clone();
                elements_mut[leaf_index] = function(old);
            }
        }
    }

    /// Extends the vector with elements from an iterator.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator over elements to append
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.extend(0..1000);
    /// assert_eq!(transient.len(), 1000);
    /// ```
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for element in iter {
            self.push_back(element);
        }
    }

    /// Converts this transient vector into a persistent vector.
    ///
    /// This consumes the `TransientVector` and returns a `PersistentVector`.
    /// The conversion is O(1) as it simply moves the internal data.
    ///
    /// # Complexity
    ///
    /// O(1) - only moves fields and wraps the tail
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::TransientVector;
    ///
    /// let mut transient: TransientVector<i32> = TransientVector::new();
    /// transient.push_back(1);
    /// transient.push_back(2);
    /// transient.push_back(3);
    /// let persistent = transient.persistent();
    /// assert_eq!(persistent.len(), 3);
    /// assert_eq!(persistent.get(0), Some(&1));
    /// ```
    #[must_use]
    pub fn persistent(self) -> PersistentVector<T> {
        PersistentVector {
            length: self.length,
            shift: self.shift,
            root: self.root,
            tail: ReferenceCounter::from(self.tail),
        }
    }
}

impl<T: Clone> Default for TransientVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> FromIterator<T> for TransientVector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut transient = Self::new();
        transient.extend(iter);
        transient
    }
}

// =============================================================================
// PersistentVector::transient() method
// =============================================================================

impl<T: Clone> PersistentVector<T> {
    /// Converts this persistent vector into a transient vector.
    ///
    /// This consumes the `PersistentVector` and returns a `TransientVector`
    /// that can be efficiently mutated.
    ///
    /// # Complexity
    ///
    /// O(1) - moves root and copies tail (max 32 elements)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::persistent::PersistentVector;
    ///
    /// let persistent: PersistentVector<i32> = (1..=3).collect();
    /// let mut transient = persistent.transient();
    /// transient.push_back(4);
    /// transient.push_back(5);
    /// let new_persistent = transient.persistent();
    /// assert_eq!(new_persistent.len(), 5);
    /// ```
    #[must_use]
    pub fn transient(self) -> TransientVector<T> {
        TransientVector {
            root: self.root,
            tail: self.tail.to_vec(),
            length: self.length,
            shift: self.shift,
            _marker: PhantomData,
        }
    }
}

impl<T: Clone> Foldable for PersistentVector<T> {
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
        Self::new()
    }
}

// =============================================================================
// Serde Support
// =============================================================================

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for PersistentVector<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for element in self {
            seq.serialize_element(element)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
struct PersistentVectorVisitor<T> {
    marker: std::marker::PhantomData<T>,
}

#[cfg(feature = "serde")]
impl<T> PersistentVectorVisitor<T> {
    const fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::de::Visitor<'de> for PersistentVectorVisitor<T>
where
    T: serde::Deserialize<'de> + Clone,
{
    type Value = PersistentVector<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        const MAX_PREALLOCATE: usize = 4096;
        let capacity = seq.size_hint().unwrap_or(0).min(MAX_PREALLOCATE);
        let mut elements = Vec::with_capacity(capacity);
        while let Some(element) = seq.next_element()? {
            elements.push(element);
        }
        Ok(elements.into_iter().collect())
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for PersistentVector<T>
where
    T: serde::Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(PersistentVectorVisitor::new())
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
    fn test_display_empty_vector() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        assert_eq!(format!("{vector}"), "[]");
    }

    #[rstest]
    fn test_display_single_element_vector() {
        let vector = PersistentVector::singleton(42);
        assert_eq!(format!("{vector}"), "[42]");
    }

    #[rstest]
    fn test_display_multiple_elements_vector() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        assert_eq!(format!("{vector}"), "[1, 2, 3]");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

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
        for index in 0..1000_usize {
            let expected = i32::try_from(index).expect("Test index exceeds i32::MAX");
            assert_eq!(vector.get(index), Some(&expected));
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
        let sum = vector.fold_left(0, |accumulator, x| accumulator + x);
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

    // =========================================================================
    // take Tests
    // =========================================================================

    #[rstest]
    fn test_take_basic() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let taken = vector.take(3);
        let collected: Vec<&i32> = taken.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);
        assert_eq!(taken.len(), 3);
    }

    #[rstest]
    fn test_take_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let taken = vector.take(5);
        assert!(taken.is_empty());
        assert_eq!(taken.len(), 0);
    }

    #[rstest]
    fn test_take_zero() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let taken = vector.take(0);
        assert!(taken.is_empty());
        assert_eq!(taken.len(), 0);
    }

    #[rstest]
    fn test_take_exceeds_length() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let taken = vector.take(10);
        let collected: Vec<&i32> = taken.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);
        assert_eq!(taken.len(), 3);
    }

    #[rstest]
    fn test_take_exact_length() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let taken = vector.take(5);
        assert_eq!(vector, taken);
    }

    // =========================================================================
    // drop_first Tests
    // =========================================================================

    #[rstest]
    fn test_drop_first_basic() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let dropped = vector.drop_first(2);
        let collected: Vec<&i32> = dropped.iter().collect();
        assert_eq!(collected, vec![&3, &4, &5]);
        assert_eq!(dropped.len(), 3);
    }

    #[rstest]
    fn test_drop_first_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let dropped = vector.drop_first(5);
        assert!(dropped.is_empty());
        assert_eq!(dropped.len(), 0);
    }

    #[rstest]
    fn test_drop_first_zero() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let dropped = vector.drop_first(0);
        assert_eq!(vector, dropped);
    }

    #[rstest]
    fn test_drop_first_exceeds_length() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let dropped = vector.drop_first(10);
        assert!(dropped.is_empty());
        assert_eq!(dropped.len(), 0);
    }

    #[rstest]
    fn test_drop_first_exact_length() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let dropped = vector.drop_first(5);
        assert!(dropped.is_empty());
    }

    // =========================================================================
    // split_at Tests
    // =========================================================================

    #[rstest]
    fn test_split_at_basic() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let (left, right) = vector.split_at(2);
        let left_collected: Vec<&i32> = left.iter().collect();
        let right_collected: Vec<&i32> = right.iter().collect();
        assert_eq!(left_collected, vec![&1, &2]);
        assert_eq!(right_collected, vec![&3, &4, &5]);
    }

    #[rstest]
    fn test_split_at_zero() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let (left, right) = vector.split_at(0);
        assert!(left.is_empty());
        assert_eq!(right, vector);
    }

    #[rstest]
    fn test_split_at_length() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let (left, right) = vector.split_at(5);
        assert_eq!(left, vector);
        assert!(right.is_empty());
    }

    #[rstest]
    fn test_split_at_exceeds_length() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let (left, right) = vector.split_at(10);
        assert_eq!(left, vector);
        assert!(right.is_empty());
    }

    #[rstest]
    fn test_split_at_law() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let (left, right) = vector.split_at(3);
        assert_eq!(left, vector.take(3));
        assert_eq!(right, vector.drop_first(3));
    }

    #[rstest]
    fn test_split_at_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let (left, right) = vector.split_at(2);
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    // =========================================================================
    // find_index Tests
    // =========================================================================

    #[rstest]
    fn test_find_index_found() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let index = vector.find_index(|x| *x > 3);
        assert_eq!(index, Some(3));
    }

    #[rstest]
    fn test_find_index_not_found() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let index = vector.find_index(|x| *x > 10);
        assert_eq!(index, None);
    }

    #[rstest]
    fn test_find_index_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let index = vector.find_index(|x| *x > 0);
        assert_eq!(index, None);
    }

    #[rstest]
    fn test_find_index_first_match() {
        let vector: PersistentVector<i32> = vec![1, 3, 3, 3, 5].into_iter().collect();
        let index = vector.find_index(|x| *x == 3);
        assert_eq!(index, Some(1));
    }

    #[rstest]
    fn test_find_index_at_start() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let index = vector.find_index(|x| *x == 1);
        assert_eq!(index, Some(0));
    }

    #[rstest]
    fn test_find_index_at_end() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let index = vector.find_index(|x| *x == 5);
        assert_eq!(index, Some(4));
    }

    // =========================================================================
    // fold_left1 Tests
    // =========================================================================

    #[rstest]
    fn test_fold_left1_basic() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let sum = vector.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(sum, Some(15));
    }

    #[rstest]
    fn test_fold_left1_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let result = vector.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_fold_left1_single_element() {
        let vector: PersistentVector<i32> = vec![42].into_iter().collect();
        let result = vector.fold_left1(|accumulator, x| accumulator + x);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn test_fold_left1_subtraction() {
        let vector: PersistentVector<i32> = (1..=4).collect();
        let result = vector.fold_left1(|accumulator, x| accumulator - x);
        assert_eq!(result, Some(1 - 2 - 3 - 4));
    }

    #[rstest]
    fn test_fold_left1_max() {
        let vector: PersistentVector<i32> = vec![3, 1, 4, 1, 5, 9, 2, 6].into_iter().collect();
        let result =
            vector.fold_left1(|accumulator, x| if accumulator > x { accumulator } else { x });
        assert_eq!(result, Some(9));
    }

    // =========================================================================
    // fold_right1 Tests
    // =========================================================================

    #[rstest]
    fn test_fold_right1_basic() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let sum = vector.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(sum, Some(15));
    }

    #[rstest]
    fn test_fold_right1_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let result = vector.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_fold_right1_single_element() {
        let vector: PersistentVector<i32> = vec![42].into_iter().collect();
        let result = vector.fold_right1(|x, accumulator| x + accumulator);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn test_fold_right1_subtraction() {
        let vector: PersistentVector<i32> = (1..=4).collect();
        let result = vector.fold_right1(|x, accumulator| x - accumulator);
        assert_eq!(result, Some(1 - (2 - (3 - 4))));
    }

    #[rstest]
    fn test_fold_right1_list_construction() {
        let vector: PersistentVector<String> =
            vec!["a", "b", "c"].into_iter().map(String::from).collect();
        let result = vector.fold_right1(|x, accumulator| format!("({x} {accumulator})"));
        assert_eq!(result, Some("(a (b c))".to_string()));
    }

    // =========================================================================
    // scan_left Tests
    // =========================================================================

    #[rstest]
    fn test_scan_left_basic() {
        let vector: PersistentVector<i32> = (1..=4).collect();
        let scanned = vector.scan_left(0, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![0, 1, 3, 6, 10]);
    }

    #[rstest]
    fn test_scan_left_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let scanned = vector.scan_left(0, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![0]);
    }

    #[rstest]
    fn test_scan_left_single_element() {
        let vector: PersistentVector<i32> = vec![5].into_iter().collect();
        let scanned = vector.scan_left(10, |accumulator, x| accumulator + x);
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![10, 15]);
    }

    #[rstest]
    fn test_scan_left_type_change() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let scanned = vector.scan_left(String::new(), |accumulator, x| format!("{accumulator}{x}"));
        let collected: Vec<String> = scanned.iter().cloned().collect();
        assert_eq!(
            collected,
            vec![
                String::new(),
                "1".to_string(),
                "12".to_string(),
                "123".to_string()
            ]
        );
    }

    #[rstest]
    fn test_scan_left_running_max() {
        let vector: PersistentVector<i32> = vec![3, 1, 4, 1, 5, 9, 2, 6].into_iter().collect();
        let scanned = vector.scan_left(i32::MIN, |accumulator, x| accumulator.max(*x));
        let collected: Vec<i32> = scanned.iter().copied().collect();
        assert_eq!(collected, vec![i32::MIN, 3, 3, 4, 4, 5, 9, 9, 9]);
    }

    // =========================================================================
    // partition Tests
    // =========================================================================

    #[rstest]
    fn test_partition_basic() {
        let vector: PersistentVector<i32> = (1..=6).collect();
        let (evens, odds) = vector.partition(|x| x % 2 == 0);
        let evens_collected: Vec<i32> = evens.iter().copied().collect();
        let odds_collected: Vec<i32> = odds.iter().copied().collect();
        assert_eq!(evens_collected, vec![2, 4, 6]);
        assert_eq!(odds_collected, vec![1, 3, 5]);
    }

    #[rstest]
    fn test_partition_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let (pass, fail) = vector.partition(|x| x % 2 == 0);
        assert!(pass.is_empty());
        assert!(fail.is_empty());
    }

    #[rstest]
    fn test_partition_all_pass() {
        let vector: PersistentVector<i32> = (2..=8).step_by(2).collect();
        let (pass, fail) = vector.partition(|x| x % 2 == 0);
        let pass_collected: Vec<i32> = pass.iter().copied().collect();
        assert_eq!(pass_collected, vec![2, 4, 6, 8]);
        assert!(fail.is_empty());
    }

    #[rstest]
    fn test_partition_all_fail() {
        let vector: PersistentVector<i32> = (1..=7).step_by(2).collect();
        let (pass, fail) = vector.partition(|x| x % 2 == 0);
        assert!(pass.is_empty());
        let fail_collected: Vec<i32> = fail.iter().copied().collect();
        assert_eq!(fail_collected, vec![1, 3, 5, 7]);
    }

    #[rstest]
    fn test_partition_preserves_order() {
        let vector: PersistentVector<i32> = (1..=10).collect();
        let (pass, fail) = vector.partition(|x| x % 3 == 0);
        let pass_collected: Vec<i32> = pass.iter().copied().collect();
        let fail_collected: Vec<i32> = fail.iter().copied().collect();
        assert_eq!(pass_collected, vec![3, 6, 9]);
        assert_eq!(fail_collected, vec![1, 2, 4, 5, 7, 8, 10]);
    }

    // =========================================================================
    // zip Tests
    // =========================================================================

    #[rstest]
    fn test_zip_basic() {
        let vector1: PersistentVector<i32> = (1..=3).collect();
        let vector2: PersistentVector<char> = vec!['a', 'b', 'c'].into_iter().collect();
        let zipped = vector1.zip(&vector2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
    }

    #[rstest]
    fn test_zip_empty_first() {
        let vector1: PersistentVector<i32> = PersistentVector::new();
        let vector2: PersistentVector<char> = vec!['a', 'b', 'c'].into_iter().collect();
        let zipped = vector1.zip(&vector2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_empty_second() {
        let vector1: PersistentVector<i32> = (1..=3).collect();
        let vector2: PersistentVector<char> = PersistentVector::new();
        let zipped = vector1.zip(&vector2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_both_empty() {
        let vector1: PersistentVector<i32> = PersistentVector::new();
        let vector2: PersistentVector<char> = PersistentVector::new();
        let zipped = vector1.zip(&vector2);
        assert!(zipped.is_empty());
    }

    #[rstest]
    fn test_zip_different_lengths_first_shorter() {
        let vector1: PersistentVector<i32> = (1..=2).collect();
        let vector2: PersistentVector<char> = vec!['a', 'b', 'c', 'd'].into_iter().collect();
        let zipped = vector1.zip(&vector2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b')]);
    }

    #[rstest]
    fn test_zip_different_lengths_second_shorter() {
        let vector1: PersistentVector<i32> = (1..=5).collect();
        let vector2: PersistentVector<char> = vec!['a', 'b'].into_iter().collect();
        let zipped = vector1.zip(&vector2);
        let collected: Vec<(i32, char)> = zipped.iter().copied().collect();
        assert_eq!(collected, vec![(1, 'a'), (2, 'b')]);
    }

    // =========================================================================
    // unzip Tests
    // =========================================================================

    #[rstest]
    fn test_unzip_basic() {
        let vector: PersistentVector<(i32, char)> =
            vec![(1, 'a'), (2, 'b'), (3, 'c')].into_iter().collect();
        let (first, second) = vector.unzip();
        let first_collected: Vec<i32> = first.iter().copied().collect();
        let second_collected: Vec<char> = second.iter().copied().collect();
        assert_eq!(first_collected, vec![1, 2, 3]);
        assert_eq!(second_collected, vec!['a', 'b', 'c']);
    }

    #[rstest]
    fn test_unzip_empty() {
        let vector: PersistentVector<(i32, char)> = PersistentVector::new();
        let (first, second) = vector.unzip();
        assert!(first.is_empty());
        assert!(second.is_empty());
    }

    #[rstest]
    fn test_unzip_single_element() {
        let vector: PersistentVector<(i32, char)> = vec![(42, 'x')].into_iter().collect();
        let (first, second) = vector.unzip();
        let first_collected: Vec<i32> = first.iter().copied().collect();
        let second_collected: Vec<char> = second.iter().copied().collect();
        assert_eq!(first_collected, vec![42]);
        assert_eq!(second_collected, vec!['x']);
    }

    #[rstest]
    fn test_unzip_roundtrip_with_zip() {
        let vector1: PersistentVector<i32> = (1..=5).collect();
        let vector2: PersistentVector<char> = vec!['a', 'b', 'c', 'd', 'e'].into_iter().collect();
        let zipped = vector1.zip(&vector2);
        let (unzipped1, unzipped2) = zipped.unzip();
        let collected1: Vec<i32> = unzipped1.iter().copied().collect();
        let collected2: Vec<char> = unzipped2.iter().copied().collect();
        assert_eq!(collected1, vec![1, 2, 3, 4, 5]);
        assert_eq!(collected2, vec!['a', 'b', 'c', 'd', 'e']);
    }

    // =========================================================================
    // intersperse Tests
    // =========================================================================

    #[rstest]
    fn test_intersperse_basic() {
        let vector: PersistentVector<i32> = (1..=4).collect();
        let result = vector.intersperse(0);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 0, 2, 0, 3, 0, 4]);
    }

    #[rstest]
    fn test_intersperse_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let result = vector.intersperse(0);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_intersperse_single_element() {
        let vector: PersistentVector<i32> = vec![42].into_iter().collect();
        let result = vector.intersperse(0);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![42]);
    }

    #[rstest]
    fn test_intersperse_two_elements() {
        let vector: PersistentVector<char> = vec!['a', 'b'].into_iter().collect();
        let result = vector.intersperse('-');
        let collected: Vec<char> = result.iter().copied().collect();
        assert_eq!(collected, vec!['a', '-', 'b']);
    }

    #[rstest]
    fn test_intersperse_strings() {
        let vector: PersistentVector<String> =
            vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
                .into_iter()
                .collect();
        let result = vector.intersperse(",".to_string());
        let collected: Vec<String> = result.iter().cloned().collect();
        assert_eq!(
            collected,
            vec![
                "foo".to_string(),
                ",".to_string(),
                "bar".to_string(),
                ",".to_string(),
                "baz".to_string()
            ]
        );
    }

    // =========================================================================
    // intercalate Tests
    // =========================================================================

    #[rstest]
    fn test_intercalate_basic() {
        let inner1: PersistentVector<i32> = vec![1, 2].into_iter().collect();
        let inner2: PersistentVector<i32> = vec![3, 4].into_iter().collect();
        let inner3: PersistentVector<i32> = vec![5, 6].into_iter().collect();
        let outer: PersistentVector<PersistentVector<i32>> =
            vec![inner1, inner2, inner3].into_iter().collect();
        let separator: PersistentVector<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 0, 3, 4, 0, 5, 6]);
    }

    #[rstest]
    fn test_intercalate_empty_outer() {
        let outer: PersistentVector<PersistentVector<i32>> = PersistentVector::new();
        let separator: PersistentVector<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_intercalate_single_inner() {
        let inner: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
        let outer: PersistentVector<PersistentVector<i32>> = vec![inner].into_iter().collect();
        let separator: PersistentVector<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_intercalate_empty_separator() {
        let inner1: PersistentVector<i32> = vec![1, 2].into_iter().collect();
        let inner2: PersistentVector<i32> = vec![3, 4].into_iter().collect();
        let outer: PersistentVector<PersistentVector<i32>> =
            vec![inner1, inner2].into_iter().collect();
        let separator: PersistentVector<i32> = PersistentVector::new();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn test_intercalate_empty_inner_vectors() {
        let inner1: PersistentVector<i32> = PersistentVector::new();
        let inner2: PersistentVector<i32> = PersistentVector::new();
        let outer: PersistentVector<PersistentVector<i32>> =
            vec![inner1, inner2].into_iter().collect();
        let separator: PersistentVector<i32> = vec![0].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<i32> = result.iter().copied().collect();
        assert_eq!(collected, vec![0]);
    }

    #[rstest]
    fn test_intercalate_multi_element_separator() {
        let inner1: PersistentVector<char> = vec!['a', 'b'].into_iter().collect();
        let inner2: PersistentVector<char> = vec!['c', 'd'].into_iter().collect();
        let outer: PersistentVector<PersistentVector<char>> =
            vec![inner1, inner2].into_iter().collect();
        let separator: PersistentVector<char> = vec!['-', '-'].into_iter().collect();
        let result = outer.intercalate(&separator);
        let collected: Vec<char> = result.iter().copied().collect();
        assert_eq!(collected, vec!['a', 'b', '-', '-', 'c', 'd']);
    }

    // =========================================================================
    // Ord / PartialOrd Tests
    // =========================================================================

    mod ord_tests {
        use super::*;
        use std::cmp::Ordering;
        use std::collections::{BTreeMap, BTreeSet};

        #[rstest]
        fn test_ord_empty_equals_empty() {
            let empty1: PersistentVector<i32> = PersistentVector::new();
            let empty2: PersistentVector<i32> = PersistentVector::new();
            assert_eq!(empty1.cmp(&empty2), Ordering::Equal);
            assert_eq!(empty1.partial_cmp(&empty2), Some(Ordering::Equal));
        }

        #[rstest]
        fn test_ord_empty_less_than_non_empty() {
            let empty: PersistentVector<i32> = PersistentVector::new();
            let non_empty = PersistentVector::singleton(1);
            assert_eq!(empty.cmp(&non_empty), Ordering::Less);
            assert_eq!(non_empty.cmp(&empty), Ordering::Greater);
        }

        #[rstest]
        fn test_ord_single_element_comparison() {
            let vector1 = PersistentVector::singleton(1);
            let vector2 = PersistentVector::singleton(2);
            let vector3 = PersistentVector::singleton(1);
            assert_eq!(vector1.cmp(&vector2), Ordering::Less);
            assert_eq!(vector2.cmp(&vector1), Ordering::Greater);
            assert_eq!(vector1.cmp(&vector3), Ordering::Equal);
        }

        #[rstest]
        fn test_ord_lexicographic_comparison() {
            let vector1: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
            let vector2: PersistentVector<i32> = vec![1, 2, 4].into_iter().collect();
            let vector3: PersistentVector<i32> = vec![1, 3, 2].into_iter().collect();
            assert_eq!(vector1.cmp(&vector2), Ordering::Less);
            assert_eq!(vector1.cmp(&vector3), Ordering::Less);
            assert_eq!(vector2.cmp(&vector3), Ordering::Less);
        }

        #[rstest]
        fn test_ord_prefix_is_less() {
            let prefix: PersistentVector<i32> = vec![1, 2].into_iter().collect();
            let extended: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
            assert_eq!(prefix.cmp(&extended), Ordering::Less);
            assert_eq!(extended.cmp(&prefix), Ordering::Greater);
        }

        #[rstest]
        fn test_ord_first_difference_determines_order() {
            let vector1: PersistentVector<i32> = vec![1, 2, 9].into_iter().collect();
            let vector2: PersistentVector<i32> = vec![1, 3, 0].into_iter().collect();
            assert_eq!(vector1.cmp(&vector2), Ordering::Less);
        }

        #[rstest]
        fn test_partial_cmp_with_nan() {
            let vector1: PersistentVector<f64> = vec![1.0, f64::NAN, 3.0].into_iter().collect();
            let vector2: PersistentVector<f64> = vec![1.0, 2.0, 3.0].into_iter().collect();
            assert_eq!(vector1.partial_cmp(&vector2), None);

            let empty1: PersistentVector<f64> = PersistentVector::new();
            let empty2: PersistentVector<f64> = PersistentVector::new();
            assert_eq!(empty1.partial_cmp(&empty2), Some(Ordering::Equal));

            let vector3: PersistentVector<f64> = vec![1.0, 2.0].into_iter().collect();
            let vector4: PersistentVector<f64> = vec![1.0, 3.0].into_iter().collect();
            assert_eq!(vector3.partial_cmp(&vector4), Some(Ordering::Less));
        }

        #[rstest]
        fn test_partial_cmp_nan_not_reached_due_to_earlier_difference() {
            let vector1: PersistentVector<f64> = vec![1.0, f64::NAN].into_iter().collect();
            let vector2: PersistentVector<f64> = vec![2.0, f64::NAN].into_iter().collect();
            assert_eq!(vector1.partial_cmp(&vector2), Some(Ordering::Less));
        }

        #[rstest]
        fn test_ord_comparison_operators() {
            let vector1: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
            let vector2: PersistentVector<i32> = vec![1, 2, 4].into_iter().collect();
            let vector3: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();

            assert!(vector1 < vector2);
            assert!(vector2 > vector1);
            assert!(vector1 <= vector2);
            assert!(vector2 >= vector1);
            assert!(vector1 <= vector3);
            assert!(vector1 >= vector3);
        }

        #[rstest]
        fn test_ord_in_btreeset() {
            let mut set: BTreeSet<PersistentVector<i32>> = BTreeSet::new();
            set.insert(vec![1, 2, 3].into_iter().collect());
            set.insert(vec![1, 2, 2].into_iter().collect());
            set.insert(vec![1, 2, 4].into_iter().collect());

            let ordered: Vec<PersistentVector<i32>> = set.iter().cloned().collect();
            assert_eq!(ordered.len(), 3);
            assert_eq!(ordered[0], vec![1, 2, 2].into_iter().collect());
            assert_eq!(ordered[1], vec![1, 2, 3].into_iter().collect());
            assert_eq!(ordered[2], vec![1, 2, 4].into_iter().collect());
        }

        #[rstest]
        fn test_ord_in_btreemap_key() {
            let mut map: BTreeMap<PersistentVector<i32>, &str> = BTreeMap::new();
            map.insert(vec![1, 2, 3].into_iter().collect(), "middle");
            map.insert(vec![1, 2, 2].into_iter().collect(), "first");
            map.insert(vec![1, 2, 4].into_iter().collect(), "last");

            let keys: Vec<PersistentVector<i32>> = map.keys().cloned().collect();
            assert_eq!(keys.len(), 3);
            assert_eq!(keys[0], vec![1, 2, 2].into_iter().collect());
            assert_eq!(keys[1], vec![1, 2, 3].into_iter().collect());
            assert_eq!(keys[2], vec![1, 2, 4].into_iter().collect());

            assert_eq!(
                map.get(&vec![1, 2, 2].into_iter().collect()),
                Some(&"first")
            );
        }

        #[rstest]
        fn test_ord_vec_sort() {
            let vector1: PersistentVector<i32> = vec![1, 2, 4].into_iter().collect();
            let vector2: PersistentVector<i32> = vec![1, 2, 2].into_iter().collect();
            let vector3: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();

            let mut vectors = [vector1.clone(), vector2.clone(), vector3.clone()];
            vectors.sort();

            assert_eq!(vectors[0], vector2);
            assert_eq!(vectors[1], vector3);
            assert_eq!(vectors[2], vector1);
        }
    }
}

// =============================================================================
// Thread Safety Tests (arc feature only)
// =============================================================================

#[cfg(all(test, feature = "arc"))]
mod send_sync_tests {
    use super::*;
    use rstest::rstest;

    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}

    #[rstest]
    fn test_vector_is_send() {
        assert_send::<PersistentVector<i32>>();
        assert_send::<PersistentVector<String>>();
    }

    #[rstest]
    fn test_vector_is_sync() {
        assert_sync::<PersistentVector<i32>>();
        assert_sync::<PersistentVector<String>>();
    }

    #[rstest]
    fn test_vector_send_sync_combined() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<PersistentVector<i32>>();
        is_send_sync::<PersistentVector<String>>();
    }
}

#[cfg(all(test, feature = "arc"))]
mod multithread_tests {
    use super::*;
    use rstest::rstest;
    use std::thread;

    #[rstest]
    fn test_vector_shared_across_threads() {
        let vector: PersistentVector<i32> = (0..10000).collect();

        let vector1 = vector.clone();
        let vector2 = vector;

        let handle1 = thread::spawn(move || vector1.iter().sum::<i32>());

        let handle2 = thread::spawn(move || vector2.iter().sum::<i32>());

        let sum1 = handle1.join().unwrap();
        let sum2 = handle2.join().unwrap();

        assert_eq!(sum1, sum2);
        assert_eq!(sum1, (0..10000).sum::<i32>());
    }

    #[rstest]
    fn test_vector_concurrent_push_back() {
        let vector: PersistentVector<i32> = PersistentVector::new();

        let vector1 = vector.clone();
        let vector2 = vector;

        let handle1 = thread::spawn(move || vector1.push_back(1).push_back(2).push_back(3));

        let handle2 = thread::spawn(move || vector2.push_back(4).push_back(5).push_back(6));

        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        assert_eq!(result1.len(), 3);
        assert_eq!(result2.len(), 3);
    }

    #[rstest]
    fn test_vector_concurrent_random_access() {
        let vector: PersistentVector<i32> = (0..10000).collect();

        let total: i32 = (0..4)
            .map(|thread_id| {
                let vector_clone = vector.clone();
                thread::spawn(move || {
                    let start = thread_id * 2500;
                    let end = start + 2500;
                    (start..end)
                        .map(|i| *vector_clone.get(i).unwrap())
                        .sum::<i32>()
                })
            })
            .map(|handle| handle.join().unwrap())
            .sum();

        assert_eq!(total, (0..10000).sum::<i32>());
    }

    #[rstest]
    fn test_vector_referential_transparency() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let vector_clone = vector.clone();

        let handle1 = thread::spawn(move || vector.iter().sum::<i32>());

        let handle2 = thread::spawn(move || vector_clone.iter().sum::<i32>());

        // Same input always produces same output (referential transparency)
        assert_eq!(handle1.join().unwrap(), handle2.join().unwrap());
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_serialize_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let json = serde_json::to_string(&vector).unwrap();
        assert_eq!(json, "[]");
    }

    #[rstest]
    fn test_serialize_single_element() {
        let vector = PersistentVector::singleton(42);
        let json = serde_json::to_string(&vector).unwrap();
        assert_eq!(json, "[42]");
    }

    #[rstest]
    fn test_serialize_multiple_elements() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let json = serde_json::to_string(&vector).unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[rstest]
    fn test_deserialize_empty() {
        let json = "[]";
        let vector: PersistentVector<i32> = serde_json::from_str(json).unwrap();
        assert!(vector.is_empty());
    }

    #[rstest]
    fn test_deserialize_single_element() {
        let json = "[42]";
        let vector: PersistentVector<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(vector.len(), 1);
        assert_eq!(vector.get(0), Some(&42));
    }

    #[rstest]
    fn test_deserialize_multiple_elements() {
        let json = "[1,2,3]";
        let vector: PersistentVector<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(vector.len(), 3);
        assert_eq!(vector.get(0), Some(&1));
        assert_eq!(vector.get(1), Some(&2));
        assert_eq!(vector.get(2), Some(&3));
    }

    #[rstest]
    fn test_roundtrip_empty() {
        let original: PersistentVector<i32> = PersistentVector::new();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentVector<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_roundtrip_large() {
        let original: PersistentVector<i32> = (1..=100).collect();
        let json = serde_json::to_string(&original).unwrap();
        let restored: PersistentVector<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[rstest]
    fn test_order_preservation() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let json = serde_json::to_string(&vector).unwrap();
        let restored: PersistentVector<i32> = serde_json::from_str(&json).unwrap();
        for element_index in 0..100 {
            assert_eq!(vector.get(element_index), restored.get(element_index));
        }
    }

    #[rstest]
    fn test_serialize_strings() {
        let vector: PersistentVector<String> = vec!["hello".to_string(), "world".to_string()]
            .into_iter()
            .collect();
        let json = serde_json::to_string(&vector).unwrap();
        assert_eq!(json, r#"["hello","world"]"#);
    }

    #[rstest]
    fn test_deserialize_strings() {
        let json = r#"["hello","world"]"#;
        let vector: PersistentVector<String> = serde_json::from_str(json).unwrap();
        assert_eq!(vector.len(), 2);
        assert_eq!(vector.get(0), Some(&"hello".to_string()));
        assert_eq!(vector.get(1), Some(&"world".to_string()));
    }
}

// =============================================================================
// TransientVector Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::cast_sign_loss)]
mod transient_vector_tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Basic Construction Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_new_creates_empty_transient() {
        let transient: TransientVector<i32> = TransientVector::new();
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    #[rstest]
    fn test_default_creates_empty_transient() {
        let transient: TransientVector<i32> = TransientVector::default();
        assert!(transient.is_empty());
        assert_eq!(transient.len(), 0);
    }

    // -------------------------------------------------------------------------
    // push_back Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_push_back_single_element() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(42);
        assert_eq!(transient.len(), 1);
        assert_eq!(transient.get(0), Some(&42));
    }

    #[rstest]
    fn test_push_back_multiple_elements() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_value in 0i32..100 {
            transient.push_back(element_value);
        }
        assert_eq!(transient.len(), 100);
        for element_value in 0i32..100 {
            assert_eq!(transient.get(element_value as usize), Some(&element_value));
        }
    }

    #[rstest]
    fn test_push_back_fills_tail_and_root() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        // Push more than BRANCHING_FACTOR elements to test root tree creation
        for element_value in 0i32..100 {
            transient.push_back(element_value);
        }
        assert_eq!(transient.len(), 100);
        // Verify all elements
        for element_value in 0i32..100 {
            assert_eq!(transient.get(element_value as usize), Some(&element_value));
        }
    }

    #[rstest]
    fn test_push_back_large_vector() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_index in 0..10_000 {
            transient.push_back(element_index);
        }
        assert_eq!(transient.len(), 10_000);
        // Spot check some elements
        assert_eq!(transient.get(0), Some(&0));
        assert_eq!(transient.get(5_000), Some(&5_000));
        assert_eq!(transient.get(9_999), Some(&9_999));
    }

    // -------------------------------------------------------------------------
    // pop_back Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_pop_back_empty_returns_none() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        assert_eq!(transient.pop_back(), None);
    }

    #[rstest]
    fn test_pop_back_single_element() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(42);
        assert_eq!(transient.pop_back(), Some(42));
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_pop_back_multiple_elements() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        transient.push_back(2);
        transient.push_back(3);
        assert_eq!(transient.pop_back(), Some(3));
        assert_eq!(transient.pop_back(), Some(2));
        assert_eq!(transient.pop_back(), Some(1));
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_pop_back_from_root() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        // Push more than BRANCHING_FACTOR elements
        for element_index in 0..50 {
            transient.push_back(element_index);
        }
        // Pop all elements
        for element_index in (0..50).rev() {
            assert_eq!(transient.pop_back(), Some(element_index));
        }
        assert!(transient.is_empty());
    }

    // -------------------------------------------------------------------------
    // get Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_get_out_of_bounds() {
        let transient: TransientVector<i32> = TransientVector::new();
        assert_eq!(transient.get(0), None);
        assert_eq!(transient.get(100), None);
    }

    #[rstest]
    fn test_get_from_tail() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        transient.push_back(2);
        transient.push_back(3);
        assert_eq!(transient.get(0), Some(&1));
        assert_eq!(transient.get(1), Some(&2));
        assert_eq!(transient.get(2), Some(&3));
    }

    #[rstest]
    fn test_get_from_root() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_index in 0..100 {
            transient.push_back(element_index);
        }
        // Elements in the root tree
        assert_eq!(transient.get(0), Some(&0));
        assert_eq!(transient.get(31), Some(&31));
        // Elements in tail
        assert_eq!(transient.get(99), Some(&99));
    }

    // -------------------------------------------------------------------------
    // update Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_update_out_of_bounds() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        assert_eq!(transient.update(10, 100), None);
    }

    #[rstest]
    fn test_update_in_tail() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        transient.push_back(2);
        transient.push_back(3);
        let old = transient.update(1, 20);
        assert_eq!(old, Some(2));
        assert_eq!(transient.get(1), Some(&20));
    }

    #[rstest]
    fn test_update_in_root() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_index in 0..100 {
            transient.push_back(element_index);
        }
        let old = transient.update(50, 500);
        assert_eq!(old, Some(50));
        assert_eq!(transient.get(50), Some(&500));
    }

    // -------------------------------------------------------------------------
    // update_with Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_update_with_out_of_bounds() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        assert!(!transient.update_with(10, |x| x * 10));
    }

    #[rstest]
    fn test_update_with_in_tail() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        transient.push_back(2);
        transient.push_back(3);
        assert!(transient.update_with(1, |x| x * 10));
        assert_eq!(transient.get(1), Some(&20));
    }

    #[rstest]
    fn test_update_with_in_root() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_index in 0..100 {
            transient.push_back(element_index);
        }
        assert!(transient.update_with(50, |x| x * 10));
        assert_eq!(transient.get(50), Some(&500));
    }

    // -------------------------------------------------------------------------
    // extend Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_extend_empty_iterator() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.extend(std::iter::empty());
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_extend_from_iterator() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.extend(0i32..1000);
        assert_eq!(transient.len(), 1000);
        for element_value in 0i32..1000 {
            assert_eq!(transient.get(element_value as usize), Some(&element_value));
        }
    }

    // -------------------------------------------------------------------------
    // FromIterator Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_from_iterator() {
        let transient: TransientVector<i32> = (0i32..100).collect();
        assert_eq!(transient.len(), 100);
        for element_value in 0i32..100 {
            assert_eq!(transient.get(element_value as usize), Some(&element_value));
        }
    }

    // -------------------------------------------------------------------------
    // persistent Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_persistent_empty() {
        let transient: TransientVector<i32> = TransientVector::new();
        let persistent = transient.persistent();
        assert!(persistent.is_empty());
    }

    #[rstest]
    fn test_persistent_with_elements() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        transient.push_back(1);
        transient.push_back(2);
        transient.push_back(3);
        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 3);
        assert_eq!(persistent.get(0), Some(&1));
        assert_eq!(persistent.get(1), Some(&2));
        assert_eq!(persistent.get(2), Some(&3));
    }

    #[rstest]
    fn test_persistent_large_vector() {
        let mut transient: TransientVector<i32> = TransientVector::new();
        for element_value in 0i32..10_000 {
            transient.push_back(element_value);
        }
        let persistent = transient.persistent();
        assert_eq!(persistent.len(), 10_000);
        for element_value in 0i32..10_000 {
            assert_eq!(persistent.get(element_value as usize), Some(&element_value));
        }
    }

    // -------------------------------------------------------------------------
    // transient Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_transient_empty() {
        let persistent: PersistentVector<i32> = PersistentVector::new();
        let transient = persistent.transient();
        assert!(transient.is_empty());
    }

    #[rstest]
    fn test_transient_with_elements() {
        let persistent: PersistentVector<i32> = (1..=3).collect();
        let transient = persistent.transient();
        assert_eq!(transient.len(), 3);
        assert_eq!(transient.get(0), Some(&1));
        assert_eq!(transient.get(1), Some(&2));
        assert_eq!(transient.get(2), Some(&3));
    }

    #[rstest]
    fn test_transient_modify_and_persistent() {
        let persistent: PersistentVector<i32> = (1..=3).collect();
        let mut transient = persistent.transient();
        transient.push_back(4);
        transient.push_back(5);
        let new_persistent = transient.persistent();
        assert_eq!(new_persistent.len(), 5);
        assert_eq!(new_persistent.get(3), Some(&4));
        assert_eq!(new_persistent.get(4), Some(&5));
    }

    // -------------------------------------------------------------------------
    // Roundtrip Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_roundtrip_empty() {
        let original: PersistentVector<i32> = PersistentVector::new();
        let result = original.clone().transient().persistent();
        assert_eq!(original, result);
    }

    #[rstest]
    fn test_roundtrip_small() {
        let original: PersistentVector<i32> = (1..=10).collect();
        let result = original.clone().transient().persistent();
        assert_eq!(original, result);
    }

    #[rstest]
    fn test_roundtrip_large() {
        let original: PersistentVector<i32> = (0..10_000).collect();
        let result = original.clone().transient().persistent();
        assert_eq!(original, result);
    }

    // -------------------------------------------------------------------------
    // Mutation Equivalence Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_mutation_equivalence_push_back() {
        let persistent: PersistentVector<i32> = (1..=3).collect();
        let element = 42;

        // Via transient
        let via_transient = {
            let mut transient = persistent.clone().transient();
            transient.push_back(element);
            transient.persistent()
        };

        // Via persistent
        let via_persistent = persistent.push_back(element);

        assert_eq!(via_transient, via_persistent);
    }

    #[rstest]
    fn test_mutation_equivalence_update() {
        let persistent: PersistentVector<i32> = (1..=10).collect();
        let index = 5;
        let element = 500;

        // Via transient
        let via_transient = {
            let mut transient = persistent.clone().transient();
            transient.update(index, element);
            transient.persistent()
        };

        // Via persistent
        let via_persistent = persistent.update(index, element).unwrap();

        assert_eq!(via_transient, via_persistent);
    }

    #[rstest]
    fn test_mutation_equivalence_multiple_operations() {
        let persistent: PersistentVector<i32> = (0..100).collect();

        // Via transient
        let via_transient = {
            let mut transient = persistent.clone().transient();
            for element_value in 100i32..200 {
                transient.push_back(element_value);
            }
            transient.update(50, 5000);
            transient.persistent()
        };

        // Via persistent (need a mutable copy)
        let mut via_persistent = persistent;
        for element_value in 100i32..200 {
            via_persistent = via_persistent.push_back(element_value);
        }
        via_persistent = via_persistent.update(50, 5000).unwrap();

        assert_eq!(via_transient, via_persistent);
    }

    // -------------------------------------------------------------------------
    // COW (Copy-on-Write) Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cow_shared_structure() {
        // Create a persistent vector
        let persistent1: PersistentVector<i32> = (0..100).collect();

        // Clone it (shares structure)
        let persistent2 = persistent1.clone();

        // Convert first to transient and modify
        let mut transient = persistent1.transient();
        transient.update(50, 5000);
        let modified = transient.persistent();

        // Original clone should be unchanged
        assert_eq!(persistent2.get(50), Some(&50));
        assert_eq!(modified.get(50), Some(&5000));
    }

    #[rstest]
    fn test_cow_push_back_shared() {
        // Create a persistent vector
        let persistent1: PersistentVector<i32> = (0..100).collect();

        // Clone it (shares structure)
        let persistent2 = persistent1.clone();

        // Convert first to transient and push
        let mut transient = persistent1.transient();
        transient.push_back(100);
        transient.push_back(101);
        let modified = transient.persistent();

        // Original clone should be unchanged
        assert_eq!(persistent2.len(), 100);
        assert_eq!(modified.len(), 102);
    }
}

// =============================================================================
// Rayon Parallel Iterator Support
// =============================================================================

#[cfg(feature = "rayon")]
mod rayon_support {
    use super::PersistentVector;
    use rayon::iter::plumbing::{Consumer, Producer, ProducerCallback, UnindexedConsumer, bridge};
    use rayon::iter::{
        FromParallelIterator, IndexedParallelIterator, IntoParallelIterator, ParallelIterator,
    };

    /// A parallel iterator over owned elements of a [`PersistentVector`].
    pub struct PersistentVectorParallelIterator<T> {
        elements: Vec<T>,
    }

    impl<T: Clone + Send> IntoParallelIterator for PersistentVector<T> {
        type Iter = PersistentVectorParallelIterator<T>;
        type Item = T;

        fn into_par_iter(self) -> Self::Iter {
            PersistentVectorParallelIterator {
                elements: self.into_iter().collect(),
            }
        }
    }

    impl<T: Clone + Send> ParallelIterator for PersistentVectorParallelIterator<T> {
        type Item = T;

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

    impl<T: Clone + Send> IndexedParallelIterator for PersistentVectorParallelIterator<T> {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            bridge(self, consumer)
        }

        fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
            callback.callback(VectorProducer {
                elements: self.elements,
            })
        }
    }

    struct VectorProducer<T> {
        elements: Vec<T>,
    }

    impl<T: Clone + Send> Producer for VectorProducer<T> {
        type Item = T;
        type IntoIter = std::vec::IntoIter<T>;

        fn into_iter(self) -> Self::IntoIter {
            self.elements.into_iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let mut left = self.elements;
            let right = left.split_off(index);
            (Self { elements: left }, Self { elements: right })
        }
    }

    /// A parallel iterator over references to elements of a [`PersistentVector`].
    pub struct PersistentVectorParallelRefIterator<'a, T> {
        elements: Vec<&'a T>,
    }

    impl<'a, T: Sync> IntoParallelIterator for &'a PersistentVector<T> {
        type Iter = PersistentVectorParallelRefIterator<'a, T>;
        type Item = &'a T;

        fn into_par_iter(self) -> Self::Iter {
            PersistentVectorParallelRefIterator {
                elements: self.iter().collect(),
            }
        }
    }

    impl<'a, T: Sync> ParallelIterator for PersistentVectorParallelRefIterator<'a, T> {
        type Item = &'a T;

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

    impl<T: Sync> IndexedParallelIterator for PersistentVectorParallelRefIterator<'_, T> {
        fn len(&self) -> usize {
            self.elements.len()
        }

        fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            bridge(self, consumer)
        }

        fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
            callback.callback(VectorRefProducer {
                elements: self.elements,
            })
        }
    }

    struct VectorRefProducer<'a, T> {
        elements: Vec<&'a T>,
    }

    impl<'a, T: Sync> Producer for VectorRefProducer<'a, T> {
        type Item = &'a T;
        type IntoIter = std::vec::IntoIter<&'a T>;

        fn into_iter(self) -> Self::IntoIter {
            self.elements.into_iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let mut left = self.elements;
            let right = left.split_off(index);
            (Self { elements: left }, Self { elements: right })
        }
    }

    impl<T: Sync> PersistentVector<T> {
        /// Returns a parallel iterator over references to the elements.
        ///
        /// This method preserves the original vector, allowing continued use
        /// after iteration.
        ///
        /// # Performance Note
        ///
        /// The iterator collects element references into a `Vec` for efficient
        /// parallel splitting. This adds O(n) memory overhead but enables
        /// work-stealing parallelism.
        #[inline]
        #[must_use]
        pub fn par_iter(&self) -> PersistentVectorParallelRefIterator<'_, T> {
            self.into_par_iter()
        }
    }

    impl<T: Clone + Send> FromParallelIterator<T> for PersistentVector<T> {
        fn from_par_iter<I>(par_iter: I) -> Self
        where
            I: IntoParallelIterator<Item = T>,
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
pub use rayon_support::PersistentVectorParallelIterator;
#[cfg(feature = "rayon")]
pub use rayon_support::PersistentVectorParallelRefIterator;

// =============================================================================
// Rayon Tests
// =============================================================================

#[cfg(all(test, feature = "rayon"))]
mod rayon_tests {
    use super::PersistentVector;
    use rayon::prelude::*;
    use rstest::rstest;

    // =========================================================================
    // IntoParallelIterator Tests
    // =========================================================================

    #[rstest]
    fn test_into_par_iter_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let result: Vec<i32> = vector.into_par_iter().collect();
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_into_par_iter_single_element() {
        let vector: PersistentVector<i32> = PersistentVector::singleton(42);
        let result: Vec<i32> = vector.into_par_iter().collect();
        assert_eq!(result, vec![42]);
    }

    #[rstest]
    fn test_into_par_iter_multiple_elements() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let mut result: Vec<i32> = vector.into_par_iter().collect();
        result.sort_unstable();
        assert_eq!(result, (0..100).collect::<Vec<_>>());
    }

    #[rstest]
    fn test_into_par_iter_map() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        let mut result: Vec<i32> = vector.into_par_iter().map(|x| x * 2).collect();
        result.sort_unstable();
        let expected: Vec<i32> = (0..1000).map(|x| x * 2).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_into_par_iter_filter() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        let mut result: Vec<i32> = vector.into_par_iter().filter(|&x| x % 2 == 0).collect();
        result.sort_unstable();
        let expected: Vec<i32> = (0..1000).filter(|&x| x % 2 == 0).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_into_par_iter_sum() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let sum: i32 = vector.into_par_iter().sum();
        let expected: i32 = (0..10000).sum();
        assert_eq!(sum, expected);
    }

    #[rstest]
    fn test_into_par_iter_reduce() {
        let vector: PersistentVector<i32> = (1..=100).collect();
        let product: i32 = vector
            .into_par_iter()
            .reduce(|| 1, |a, b| a.wrapping_mul(b));
        let expected: i32 = (1..=100).fold(1, |a, b| a.wrapping_mul(b));
        assert_eq!(product, expected);
    }

    // =========================================================================
    // par_iter (Reference) Tests
    // =========================================================================

    #[rstest]
    fn test_par_iter_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let result: Vec<&i32> = vector.par_iter().collect();
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_par_iter_single_element() {
        let vector: PersistentVector<i32> = PersistentVector::singleton(42);
        let result: Vec<&i32> = vector.par_iter().collect();
        assert_eq!(result, vec![&42]);
    }

    #[rstest]
    fn test_par_iter_preserves_original() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let sum: i32 = vector.par_iter().sum();
        // Vector should still be usable
        assert_eq!(vector.len(), 100);
        assert_eq!(sum, (0..100).sum::<i32>());
    }

    #[rstest]
    fn test_par_iter_map_cloned() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        let mut result: Vec<i32> = vector.par_iter().map(|&x| x * 2).collect();
        result.sort_unstable();
        let expected: Vec<i32> = (0..1000).map(|x| x * 2).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_par_iter_filter_cloned() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        let mut result: Vec<i32> = vector
            .par_iter()
            .filter(|&&x| x % 2 == 0)
            .copied()
            .collect();
        result.sort_unstable();
        let expected: Vec<i32> = (0..1000).filter(|&x| x % 2 == 0).collect();
        assert_eq!(result, expected);
    }

    // =========================================================================
    // IndexedParallelIterator Tests
    // =========================================================================

    #[rstest]
    fn test_indexed_par_iter_enumerate() {
        let vector: PersistentVector<char> = "hello world".chars().collect();
        let mut result: Vec<(usize, char)> = vector.into_par_iter().enumerate().collect();
        result.sort_unstable_by_key(|(index, _)| *index);
        let expected: Vec<(usize, char)> = "hello world".chars().enumerate().collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_indexed_par_iter_zip() {
        let vector1: PersistentVector<i32> = (0..100).collect();
        let vector2: PersistentVector<i32> = (100..200).collect();

        let mut result: Vec<(i32, i32)> = vector1
            .into_par_iter()
            .zip(vector2.into_par_iter())
            .collect();
        result.sort_unstable();

        let expected: Vec<(i32, i32)> = (0..100).zip(100..200).collect();
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_indexed_par_iter_take() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let mut result: Vec<i32> = vector.into_par_iter().take(10).collect();
        result.sort_unstable();
        // take with IndexedParallelIterator preserves order
        assert_eq!(result.len(), 10);
        // Elements should be from 0..10
        for expected_value in 0..10 {
            assert!(result.contains(&expected_value));
        }
    }

    #[rstest]
    fn test_indexed_par_iter_skip() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let mut result: Vec<i32> = vector.into_par_iter().skip(90).collect();
        result.sort_unstable();
        assert_eq!(result.len(), 10);
        // Elements should be from 90..100
        for expected_value in 90..100 {
            assert!(result.contains(&expected_value));
        }
    }

    // =========================================================================
    // FromParallelIterator Tests
    // =========================================================================

    #[rstest]
    fn test_from_par_iter_vec() {
        let source: Vec<i32> = (0..1000).collect();
        let vector: PersistentVector<i32> = source.into_par_iter().collect();
        assert_eq!(vector.len(), 1000);
    }

    #[rstest]
    fn test_from_par_iter_map() {
        let source: Vec<i32> = (0..1000).collect();
        let vector: PersistentVector<i32> = source.into_par_iter().map(|x| x * 2).collect();
        assert_eq!(vector.len(), 1000);
        // Check some values
        for element_index in 0_i32..1000 {
            #[allow(clippy::cast_sign_loss)]
            let index = element_index as usize;
            assert_eq!(vector.get(index), Some(&(element_index * 2)));
        }
    }

    #[rstest]
    fn test_from_par_iter_filter() {
        let source: Vec<i32> = (0..1000).collect();
        let vector: PersistentVector<i32> =
            source.into_par_iter().filter(|&x| x % 2 == 0).collect();
        assert_eq!(vector.len(), 500);
    }

    // =========================================================================
    // Parallel-Sequential Equivalence Tests
    // =========================================================================

    #[rstest]
    fn test_parallel_sequential_sum_equivalence() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let parallel_sum: i32 = vector.par_iter().sum();
        let sequential_sum: i32 = vector.iter().sum();
        assert_eq!(parallel_sum, sequential_sum);
    }

    #[rstest]
    fn test_parallel_sequential_map_equivalence() {
        let vector: PersistentVector<i32> = (0..1000).collect();

        let mut parallel_result: Vec<i32> = vector.par_iter().map(|&x| x * 2).collect();
        parallel_result.sort_unstable();

        let mut sequential_result: Vec<i32> = vector.iter().map(|&x| x * 2).collect();
        sequential_result.sort_unstable();

        assert_eq!(parallel_result, sequential_result);
    }

    #[rstest]
    fn test_parallel_sequential_filter_equivalence() {
        let vector: PersistentVector<i32> = (0..1000).collect();

        let mut parallel_result: Vec<i32> = vector
            .par_iter()
            .filter(|&&x| x % 3 == 0)
            .copied()
            .collect();
        parallel_result.sort_unstable();

        let mut sequential_result: Vec<i32> =
            vector.iter().filter(|&&x| x % 3 == 0).copied().collect();
        sequential_result.sort_unstable();

        assert_eq!(parallel_result, sequential_result);
    }

    #[rstest]
    fn test_parallel_sequential_count_equivalence() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let parallel_count = vector.par_iter().filter(|&&x| x % 7 == 0).count();
        let sequential_count = vector.iter().filter(|&&x| x % 7 == 0).count();
        assert_eq!(parallel_count, sequential_count);
    }

    #[rstest]
    fn test_parallel_sequential_any_equivalence() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let parallel_any = vector.par_iter().any(|&x| x == 5000);
        let sequential_any = vector.iter().any(|&x| x == 5000);
        assert_eq!(parallel_any, sequential_any);
    }

    #[rstest]
    fn test_parallel_sequential_all_equivalence() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        let parallel_all = vector.par_iter().all(|&x| x >= 0);
        let sequential_all = vector.iter().all(|&x| x >= 0);
        assert_eq!(parallel_all, sequential_all);
    }

    // =========================================================================
    // Large Data Tests
    // =========================================================================

    #[rstest]
    fn test_large_parallel_map() {
        let vector: PersistentVector<i32> = (0..100_000).collect();
        let result: PersistentVector<i32> = vector.into_par_iter().map(|x| x * 2).collect();
        assert_eq!(result.len(), 100_000);
    }

    #[rstest]
    fn test_large_parallel_sum() {
        let vector: PersistentVector<i64> = (0..100_000_i64).collect();
        let sum: i64 = vector.into_par_iter().sum();
        let expected: i64 = (0..100_000_i64).sum();
        assert_eq!(sum, expected);
    }
}
