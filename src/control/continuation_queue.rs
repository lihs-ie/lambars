//! Common continuation queue structures for type-erased continuations.
//!
//! This module provides data structures for managing type-erased continuations,
//! used by:
//! - Freer Monad (`src/control/freer.rs`)
//! - Algebraic Effect (`src/effect/algebraic/`)
//!
//! # Design
//!
//! The "Reflection without Remorse" pattern is used to achieve O(1) `push`
//! and O(n) interpretation, avoiding the O(n^2) problem from nested `FlatMap`
//! structures.
//!
//! ## Key Components
//!
//! - [`TypeErasedArrow`]: Trait for type-erased continuations
//! - [`ContinuationQueue`]: Lazy-initialized queue with 3-state enum for O(1) push/pop
//! - [`QueueStack`]: Stack of queues to avoid O(n^2) concatenation
//!
//! # Invariants
//!
//! - **FIFO Order**: Continuations are processed in first-in-first-out order
//! - **LIFO Queue Stack**: When nested effects occur, queues are stacked in LIFO order
//! - **Mutable Operations**: Operations use `&mut self` for in-place state transitions

use std::any::Any;
use std::collections::VecDeque;

/// Internal representation of a continuation queue with three states.
///
/// This enum enables lazy initialization and optimal memory usage:
/// - `Empty`: No continuations, no allocations
/// - `Single`: One continuation, only a Box allocation
/// - `Multi`: Two or more continuations, uses `VecDeque`
///
/// # Invariants
///
/// - `Multi` always contains at least 2 elements
/// - FIFO order is maintained across all states
#[derive(Default)]
enum ContinuationQueueInner<M> {
    /// No continuations (zero allocation)
    #[default]
    Empty,
    /// Exactly one continuation (Box only, no `VecDeque`)
    Single(Box<dyn TypeErasedArrow<M>>),
    /// Two or more continuations (`VecDeque` for efficient FIFO)
    Multi(VecDeque<Box<dyn TypeErasedArrow<M>>>),
}

/// Type-erased arrow (continuation).
///
/// Converts `A -> M<B>` to `Box<dyn Any> -> M<Box<dyn Any>>`.
/// This enables storing heterogeneous continuations in a single queue.
///
/// # Type Parameters
///
/// - `M`: The monadic type that arrows produce (e.g., `Freer<I, Box<dyn Any>>`)
///
/// # Contract
///
/// - `apply` is called exactly once (enforced by `self: Box<Self>`)
/// - Input type must match the expected type, otherwise panic
/// - Output is always wrapped in `Box<dyn Any>`
pub trait TypeErasedArrow<M> {
    /// Applies this continuation to the given input.
    ///
    /// # Arguments
    ///
    /// - `input`: Type-erased input value
    ///
    /// # Returns
    ///
    /// The monadic result with type-erased output
    ///
    /// # Panics
    ///
    /// Panics if the input type does not match the expected type.
    /// This indicates a bug in the DSL design.
    fn apply(self: Box<Self>, input: Box<dyn Any>) -> M;
}

/// Continuation queue with lazy initialization.
///
/// Stores type-erased continuations with optimal memory usage:
/// - Empty state: no allocations
/// - Single element: only Box allocation
/// - Multiple elements: `VecDeque` for O(1) push/pop
///
/// # Type Parameters
///
/// - `M`: The monadic type that arrows produce
///
/// # Invariants
///
/// - Continuations are processed in FIFO order
/// - `pop` returns `None` when empty
/// - Internal `Multi` state always has 2+ elements
///
/// # Note
///
/// This type does NOT implement `Clone`. Ownership semantics ensure
/// that each queue has exactly one owner, providing logical immutability.
pub struct ContinuationQueue<M> {
    inner: ContinuationQueueInner<M>,
}

impl<M> ContinuationQueue<M> {
    /// Creates a new empty continuation queue.
    ///
    /// This operation is zero-cost: no heap allocations occur.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            inner: ContinuationQueueInner::Empty,
        }
    }

    /// Returns `true` if the queue is empty.
    #[inline]
    pub(crate) const fn is_empty(&self) -> bool {
        matches!(self.inner, ContinuationQueueInner::Empty)
    }

    /// Removes and returns the first continuation from the queue.
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # State Transitions
    ///
    /// - `Empty` -> `Empty` (returns `None`)
    /// - `Single(a)` -> `Empty` (returns `Some(a)`)
    /// - `Multi([a, b])` -> `Single(b)` (returns `Some(a)`)
    /// - `Multi([a, b, c, ...])` -> `Multi([b, c, ...])` (returns `Some(a)`)
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<Box<dyn TypeErasedArrow<M>>> {
        let inner = std::mem::take(&mut self.inner);

        match inner {
            ContinuationQueueInner::Empty => None,
            ContinuationQueueInner::Single(arrow) => Some(arrow),
            ContinuationQueueInner::Multi(mut deque) => {
                let arrow = deque
                    .pop_front()
                    .expect("Multi invariant: at least 2 elements");

                self.inner = if deque.len() == 1 {
                    ContinuationQueueInner::Single(deque.pop_front().expect("checked len == 1"))
                } else {
                    debug_assert!(deque.len() >= 2, "Multi invariant: at least 2 elements");
                    ContinuationQueueInner::Multi(deque)
                };
                Some(arrow)
            }
        }
    }

    /// Adds a continuation to the end of the queue.
    ///
    /// This is an O(1) amortized operation.
    ///
    /// # State Transitions
    ///
    /// - `Empty` -> `Single(a)`
    /// - `Single(a)` -> `Multi([a, b])`
    /// - `Multi(xs)` -> `Multi(xs ++ [a])`
    #[inline]
    pub(crate) fn push_arrow(&mut self, arrow: Box<dyn TypeErasedArrow<M>>) {
        let inner = std::mem::take(&mut self.inner);

        self.inner = match inner {
            ContinuationQueueInner::Empty => ContinuationQueueInner::Single(arrow),
            ContinuationQueueInner::Single(existing) => {
                let mut deque = VecDeque::with_capacity(4);
                deque.push_back(existing);
                deque.push_back(arrow);
                ContinuationQueueInner::Multi(deque)
            }
            ContinuationQueueInner::Multi(mut deque) => {
                deque.push_back(arrow);
                ContinuationQueueInner::Multi(deque)
            }
        };
    }

    /// Returns the number of continuations in the queue.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        match &self.inner {
            ContinuationQueueInner::Empty => 0,
            ContinuationQueueInner::Single(_) => 1,
            ContinuationQueueInner::Multi(deque) => deque.len(),
        }
    }
}

impl<M> Default for ContinuationQueue<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// A stack of continuation queues.
///
/// Used during interpretation to avoid O(n^2) from repeated queue concatenation.
/// Instead of merging queues, we maintain a stack of queues and process them
/// in LIFO order.
///
/// # Transition Rules
///
/// 1. Initial state: `current = operation.queue`, `pending = []`
/// 2. On continuation application:
///    - `current.pop()` to get continuation
///    - If result is `Impure`: push `current` to `pending`, set new queue as `current`
///    - If result is `Pure`: apply next continuation
/// 3. When `current` is empty: `pending.pop()` to get next queue
/// 4. When both `current` and `pending` are empty: processing complete
///
/// # Note
///
/// This type does NOT implement `Clone`. Ownership semantics ensure
/// that each stack has exactly one owner.
pub struct QueueStack<M> {
    current: ContinuationQueue<M>,
    pending: Vec<ContinuationQueue<M>>,
}

impl<M> QueueStack<M> {
    /// Creates a new `QueueStack` with the given initial queue.
    #[inline]
    pub(crate) const fn new(initial: ContinuationQueue<M>) -> Self {
        Self {
            current: initial,
            pending: Vec::new(),
        }
    }

    /// Pushes a new queue onto the stack.
    ///
    /// The current queue is moved to `pending` (if non-empty),
    /// and the new queue becomes `current`.
    #[inline]
    pub(crate) fn push_queue(&mut self, queue: ContinuationQueue<M>) {
        let old = std::mem::replace(&mut self.current, queue);
        if !old.is_empty() {
            self.pending.push(old);
        }
    }

    /// Pops the next continuation from the stack.
    ///
    /// First tries to pop from `current`. If `current` is empty,
    /// pops the next queue from `pending` and continues.
    ///
    /// Returns `None` when all queues are exhausted.
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<Box<dyn TypeErasedArrow<M>>> {
        loop {
            if let Some(arrow) = self.current.pop() {
                return Some(arrow);
            }
            self.current = self.pending.pop()?;
        }
    }

    /// Returns `true` if all queues are exhausted.
    #[inline]
    #[allow(dead_code)]
    pub(crate) const fn is_exhausted(&self) -> bool {
        self.current.is_empty() && self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ==========================================================================
    // Test helper: Simple arrow that wraps a function
    // ==========================================================================

    struct SimpleArrow<F> {
        function: F,
    }

    impl<F> SimpleArrow<F> {
        fn new(function: F) -> Self {
            Self { function }
        }
    }

    // For testing, we use Box<dyn Any> as the monadic type
    impl<F> TypeErasedArrow<Box<dyn Any>> for SimpleArrow<F>
    where
        F: FnOnce(Box<dyn Any>) -> Box<dyn Any> + 'static,
    {
        fn apply(self: Box<Self>, input: Box<dyn Any>) -> Box<dyn Any> {
            (self.function)(input)
        }
    }

    fn make_add_one_arrow() -> Box<dyn TypeErasedArrow<Box<dyn Any>>> {
        Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 1) as Box<dyn Any>
        }))
    }

    fn make_multiply_two_arrow() -> Box<dyn TypeErasedArrow<Box<dyn Any>>> {
        Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value * 2) as Box<dyn Any>
        }))
    }

    // ==========================================================================
    // ContinuationQueue Tests
    // ==========================================================================

    #[rstest]
    fn continuation_queue_new_creates_empty_queue() {
        let queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[rstest]
    fn continuation_queue_default_creates_empty_queue() {
        let queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::default();
        assert!(queue.is_empty());
    }

    #[rstest]
    fn continuation_queue_push_increases_length() {
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);

        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 2);
    }

    #[rstest]
    fn continuation_queue_pop_returns_arrows_in_fifo_order() {
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());

        // First pop: +1
        let arrow1 = queue.pop().expect("should have first arrow");
        let result1 = arrow1.apply(Box::new(10i32));
        assert_eq!(*result1.downcast::<i32>().unwrap(), 11); // 10 + 1

        // Second pop: *2
        let arrow2 = queue.pop().expect("should have second arrow");
        let result2 = arrow2.apply(Box::new(10i32));
        assert_eq!(*result2.downcast::<i32>().unwrap(), 20); // 10 * 2

        // Third pop: None
        assert!(queue.pop().is_none());
        assert!(queue.is_empty());
    }

    #[rstest]
    fn continuation_queue_pop_from_empty_returns_none() {
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        assert!(queue.pop().is_none());
    }

    // ==========================================================================
    // QueueStack Tests
    // ==========================================================================

    #[rstest]
    fn queue_stack_new_creates_stack_with_initial_queue() {
        let queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        let stack = QueueStack::new(queue);
        assert!(stack.is_exhausted());
    }

    #[rstest]
    fn queue_stack_pop_from_single_queue() {
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        let mut stack = QueueStack::new(queue);

        assert!(!stack.is_exhausted());
        let arrow = stack.pop().expect("should have arrow");
        let result = arrow.apply(Box::new(5i32));
        assert_eq!(*result.downcast::<i32>().unwrap(), 6);

        assert!(stack.is_exhausted());
        assert!(stack.pop().is_none());
    }

    #[rstest]
    fn queue_stack_push_queue_switches_current() {
        let mut queue1: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue1.push_arrow(make_add_one_arrow()); // +1

        let mut queue2: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue2.push_arrow(make_multiply_two_arrow()); // *2

        let mut stack = QueueStack::new(queue1);
        stack.push_queue(queue2);

        // queue2 should be current now (LIFO: new queue becomes current)
        // First pop from queue2: *2
        let arrow1 = stack.pop().expect("should have arrow from queue2");
        let result1 = arrow1.apply(Box::new(5i32));
        assert_eq!(*result1.downcast::<i32>().unwrap(), 10); // 5 * 2

        // queue2 is exhausted, switch to queue1: +1
        let arrow2 = stack.pop().expect("should have arrow from queue1");
        let result2 = arrow2.apply(Box::new(5i32));
        assert_eq!(*result2.downcast::<i32>().unwrap(), 6); // 5 + 1

        assert!(stack.is_exhausted());
    }

    #[rstest]
    fn queue_stack_push_empty_queue_skips_pending() {
        let mut queue1: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue1.push_arrow(make_add_one_arrow());

        let queue2: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new(); // empty

        let mut stack = QueueStack::new(queue1);
        stack.push_queue(queue2);

        // queue2 is empty, should switch to queue1 immediately
        let arrow = stack.pop().expect("should have arrow from queue1");
        let result = arrow.apply(Box::new(5i32));
        assert_eq!(*result.downcast::<i32>().unwrap(), 6);

        assert!(stack.is_exhausted());
    }

    #[rstest]
    fn queue_stack_multiple_queues_lifo_order() {
        let mut queue1: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue1.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 1) as Box<dyn Any>
        })));

        let mut queue2: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue2.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 10) as Box<dyn Any>
        })));

        let mut queue3: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue3.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 100) as Box<dyn Any>
        })));

        let mut stack = QueueStack::new(queue1);
        stack.push_queue(queue2);
        stack.push_queue(queue3);

        // Order should be: queue3 -> queue2 -> queue1 (LIFO)
        let result1 = stack
            .pop()
            .unwrap()
            .apply(Box::new(0i32))
            .downcast::<i32>()
            .unwrap();
        assert_eq!(*result1, 100); // +100 from queue3

        let result2 = stack
            .pop()
            .unwrap()
            .apply(Box::new(0i32))
            .downcast::<i32>()
            .unwrap();
        assert_eq!(*result2, 10); // +10 from queue2

        let result3 = stack
            .pop()
            .unwrap()
            .apply(Box::new(0i32))
            .downcast::<i32>()
            .unwrap();
        assert_eq!(*result3, 1); // +1 from queue1

        assert!(stack.is_exhausted());
    }

    // ==========================================================================
    // ContinuationQueue State Transition Tests (3-state enum)
    // ==========================================================================

    #[rstest]
    fn continuation_queue_state_empty_to_single() {
        // Empty -> Single transition on first push
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.push_arrow(make_add_one_arrow());
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);
    }

    #[rstest]
    fn continuation_queue_state_single_to_multi() {
        // Single -> Multi transition on second push
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 1);

        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn continuation_queue_state_multi_grows() {
        // Multi state grows with more pushes
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 3);

        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 4);
    }

    #[rstest]
    fn continuation_queue_state_single_to_empty() {
        // Single -> Empty transition on pop
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 1);

        let arrow = queue.pop();
        assert!(arrow.is_some());
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[rstest]
    fn continuation_queue_state_multi_to_single() {
        // Multi (2 elements) -> Single transition on pop
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 2);

        let arrow = queue.pop();
        assert!(arrow.is_some());
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[rstest]
    fn continuation_queue_state_multi_shrinks() {
        // Multi (3+ elements) -> Multi (2+ elements) on pop
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 3);

        let arrow = queue.pop();
        assert!(arrow.is_some());
        assert_eq!(queue.len(), 2);
    }

    #[rstest]
    fn continuation_queue_state_multi_to_single_to_empty() {
        // Full cycle: Multi -> Single -> Empty
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 2);

        queue.pop(); // Multi -> Single
        assert_eq!(queue.len(), 1);

        queue.pop(); // Single -> Empty
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[rstest]
    fn continuation_queue_fifo_order_with_three_elements() {
        // Verify FIFO order: first pushed is first popped
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();

        // Push: +1, *2, +1 (in this order)
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 1) as Box<dyn Any>
        })));
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value * 2) as Box<dyn Any>
        })));
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 100) as Box<dyn Any>
        })));

        // Pop order should be: +1, *2, +100
        let result1 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*result1.downcast::<i32>().unwrap(), 11); // 10 + 1

        let result2 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*result2.downcast::<i32>().unwrap(), 20); // 10 * 2

        let result3 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*result3.downcast::<i32>().unwrap(), 110); // 10 + 100

        assert!(queue.is_empty());
    }

    #[rstest]
    fn continuation_queue_interleaved_push_pop() {
        // Test interleaved push and pop operations
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();

        // Push +1, then pop
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 1) as Box<dyn Any>
        })));
        let r1 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*r1.downcast::<i32>().unwrap(), 11);
        assert!(queue.is_empty());

        // Push *2 and +100, then pop both
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value * 2) as Box<dyn Any>
        })));
        queue.push_arrow(Box::new(SimpleArrow::new(|input: Box<dyn Any>| {
            let value = *input.downcast::<i32>().expect("expected i32");
            Box::new(value + 100) as Box<dyn Any>
        })));
        assert_eq!(queue.len(), 2);

        let r2 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*r2.downcast::<i32>().unwrap(), 20); // *2

        let r3 = queue.pop().unwrap().apply(Box::new(10i32));
        assert_eq!(*r3.downcast::<i32>().unwrap(), 110); // +100

        assert!(queue.is_empty());
    }

    // ==========================================================================
    // QueueStack Tests (single queue path)
    // ==========================================================================

    #[rstest]
    fn queue_stack_single_queue_simple_path() {
        // When only using a single queue, no entries in pending
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue.push_arrow(make_add_one_arrow());
        let mut stack = QueueStack::new(queue);

        // Pop all from single queue
        let arrow = stack.pop().expect("should have arrow");
        let result = arrow.apply(Box::new(5i32));
        assert_eq!(*result.downcast::<i32>().unwrap(), 6);

        assert!(stack.is_exhausted());
        assert!(stack.pop().is_none());
    }

    #[rstest]
    fn queue_stack_fifo_within_queue_lifo_between_queues() {
        // Test that within a queue, arrows are processed FIFO
        // But between queues, they are processed LIFO
        let mut queue1: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue1.push_arrow(Box::new(SimpleArrow::new(|_| {
            Box::new("q1_first") as Box<dyn Any>
        })));
        queue1.push_arrow(Box::new(SimpleArrow::new(|_| {
            Box::new("q1_second") as Box<dyn Any>
        })));

        let mut queue2: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();
        queue2.push_arrow(Box::new(SimpleArrow::new(|_| {
            Box::new("q2_first") as Box<dyn Any>
        })));
        queue2.push_arrow(Box::new(SimpleArrow::new(|_| {
            Box::new("q2_second") as Box<dyn Any>
        })));

        let mut stack = QueueStack::new(queue1);
        stack.push_queue(queue2);

        // queue2 is now current (LIFO between queues)
        let r1 = *stack
            .pop()
            .unwrap()
            .apply(Box::new(()))
            .downcast::<&str>()
            .unwrap();
        assert_eq!(r1, "q2_first"); // FIFO within queue2

        let r2 = *stack
            .pop()
            .unwrap()
            .apply(Box::new(()))
            .downcast::<&str>()
            .unwrap();
        assert_eq!(r2, "q2_second"); // FIFO within queue2

        // Now queue1
        let r3 = *stack
            .pop()
            .unwrap()
            .apply(Box::new(()))
            .downcast::<&str>()
            .unwrap();
        assert_eq!(r3, "q1_first"); // FIFO within queue1

        let r4 = *stack
            .pop()
            .unwrap()
            .apply(Box::new(()))
            .downcast::<&str>()
            .unwrap();
        assert_eq!(r4, "q1_second"); // FIFO within queue1

        assert!(stack.is_exhausted());
    }

    #[rstest]
    fn continuation_queue_state_multi_single_multi_roundtrip() {
        // Test roundtrip: Multi -> Single -> Multi -> Single -> Empty
        let mut queue: ContinuationQueue<Box<dyn Any>> = ContinuationQueue::new();

        // Build up to Multi (3 elements)
        queue.push_arrow(make_add_one_arrow());
        queue.push_arrow(make_multiply_two_arrow());
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 3);

        // Multi -> Multi (3 -> 2)
        queue.pop();
        assert_eq!(queue.len(), 2);

        // Multi -> Single (2 -> 1)
        queue.pop();
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        // Single -> Multi (1 -> 2): add two more
        queue.push_arrow(make_multiply_two_arrow());
        assert_eq!(queue.len(), 2);
        queue.push_arrow(make_add_one_arrow());
        assert_eq!(queue.len(), 3);

        // Multi -> Single -> Empty
        queue.pop();
        assert_eq!(queue.len(), 2);
        queue.pop();
        assert_eq!(queue.len(), 1);
        queue.pop();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }
}
