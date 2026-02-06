//! Pure string building functions with capacity pre-estimation and buffer reuse.
//!
//! This module provides pure functions for building formatted strings
//! with pre-estimated capacity to minimize reallocations. It supports
//! two usage patterns:
//!
//! 1. **Convenience functions** (`build_*`): Return new strings, simple API
//! 2. **Buffer-reuse functions** (`build_*_into`): Write to provided buffer, zero allocation
//!
//! # Design Principles
//!
//! - **Referential Transparency**: Same inputs always produce same outputs
//! - **Pure Functions**: No hidden state or side effects
//! - **Immutability**: Inputs are never modified (buffer is output only)
//! - **Capacity Estimation**: Pre-allocate based on expected output size
//! - **Buffer Reuse**: Support `&mut String` + `clear` pattern for hot paths

use std::fmt::Write;

/// Estimates the decimal digit count for a usize value.
///
/// This pure function calculates the number of digits needed to represent
/// a usize value in decimal notation.
#[inline]
const fn digit_count(value: usize) -> usize {
    if value == 0 {
        return 1;
    }
    // log10(value) + 1, but computed without floating point
    let mut count = 0;
    let mut remaining = value;
    while remaining > 0 {
        count += 1;
        remaining /= 10;
    }
    count
}

/// Builds a task ID string with format "{prefix}-{index}" into a provided buffer.
///
/// This function clears the buffer and writes the result, enabling buffer reuse
/// across multiple calls to minimize allocations in hot paths.
///
/// # Arguments
///
/// * `prefix` - The prefix string for the task ID
/// * `index` - The numeric index to append
/// * `buffer` - Mutable string buffer to write into (will be cleared first)
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_task_id_into;
/// let mut buffer = String::with_capacity(32);
/// build_task_id_into("task", 42, &mut buffer);
/// assert_eq!(buffer, "task-42");
///
/// // Reuse the same buffer
/// build_task_id_into("other", 99, &mut buffer);
/// assert_eq!(buffer, "other-99");
/// ```
#[inline]
pub fn build_task_id_into(prefix: &str, index: usize, buffer: &mut String) {
    buffer.clear();
    // Reserve if needed (buffer may already have sufficient capacity)
    // After clear(), len = 0, so reserve(required) ensures capacity >= required
    let required = prefix.len() + 1 + digit_count(index);
    if buffer.capacity() < required {
        buffer.reserve(required);
    }
    // write! to String is infallible
    let _ = write!(buffer, "{prefix}-{index}");
}

/// Builds a task ID string with format "{prefix}-{index}".
///
/// This is a convenience wrapper that allocates a new string.
/// For hot paths, prefer [`build_task_id_into`] with buffer reuse.
///
/// # Arguments
///
/// * `prefix` - The prefix string for the task ID
/// * `index` - The numeric index to append
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_task_id;
/// let id = build_task_id("task", 42);
/// assert_eq!(id, "task-42");
/// ```
#[inline]
pub fn build_task_id(prefix: &str, index: usize) -> String {
    // Capacity: prefix + "-" + digits
    let capacity = prefix.len() + 1 + digit_count(index);
    let mut result = String::with_capacity(capacity);
    // write! to String is infallible
    let _ = write!(result, "{prefix}-{index}");
    result
}

/// Builds a subtask title with format "Subtask at depth {depth}, index {index}" into a buffer.
///
/// This function clears the buffer and writes the result, enabling buffer reuse.
///
/// # Arguments
///
/// * `depth` - The depth level of the subtask
/// * `index` - The index of the subtask at this depth
/// * `buffer` - Mutable string buffer to write into (will be cleared first)
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_subtask_title_into;
/// let mut buffer = String::with_capacity(64);
/// build_subtask_title_into(3, 5, &mut buffer);
/// assert_eq!(buffer, "Subtask at depth 3, index 5");
/// ```
#[inline]
pub fn build_subtask_title_into(depth: usize, index: usize, buffer: &mut String) {
    const PREFIX_LEN: usize = 17; // "Subtask at depth "
    const MIDDLE_LEN: usize = 8; // ", index "
    buffer.clear();
    let required = PREFIX_LEN + digit_count(depth) + MIDDLE_LEN + digit_count(index);
    if buffer.capacity() < required {
        buffer.reserve(required);
    }
    let _ = write!(buffer, "Subtask at depth {depth}, index {index}");
}

/// Builds a subtask title with format "Subtask at depth {depth}, index {index}".
///
/// This is a convenience wrapper that allocates a new string.
/// For hot paths, prefer [`build_subtask_title_into`] with buffer reuse.
///
/// # Arguments
///
/// * `depth` - The depth level of the subtask
/// * `index` - The index of the subtask at this depth
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_subtask_title;
/// let title = build_subtask_title(3, 5);
/// assert_eq!(title, "Subtask at depth 3, index 5");
/// ```
#[inline]
pub fn build_subtask_title(depth: usize, index: usize) -> String {
    // "Subtask at depth " (17) + depth_digits + ", index " (8) + index_digits
    const PREFIX_LEN: usize = 17; // "Subtask at depth "
    const MIDDLE_LEN: usize = 8; // ", index "
    let capacity = PREFIX_LEN + digit_count(depth) + MIDDLE_LEN + digit_count(index);
    let mut result = String::with_capacity(capacity);
    let _ = write!(result, "Subtask at depth {depth}, index {index}");
    result
}

/// Builds a child task ID with format "{parent_id}-child-{index}" into a buffer.
///
/// This function clears the buffer and writes the result, enabling buffer reuse.
///
/// # Arguments
///
/// * `parent_id` - The parent task's ID
/// * `index` - The child index
/// * `buffer` - Mutable string buffer to write into (will be cleared first)
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_child_task_id_into;
/// let mut buffer = String::with_capacity(64);
/// build_child_task_id_into("parent-123", 7, &mut buffer);
/// assert_eq!(buffer, "parent-123-child-7");
/// ```
#[inline]
pub fn build_child_task_id_into(parent_id: &str, index: usize, buffer: &mut String) {
    const SUFFIX_LEN: usize = 7; // "-child-"
    buffer.clear();
    let required = parent_id.len() + SUFFIX_LEN + digit_count(index);
    if buffer.capacity() < required {
        buffer.reserve(required);
    }
    let _ = write!(buffer, "{parent_id}-child-{index}");
}

/// Builds a child task ID with format "{parent_id}-child-{index}".
///
/// This is a convenience wrapper that allocates a new string.
/// For hot paths, prefer [`build_child_task_id_into`] with buffer reuse.
///
/// # Arguments
///
/// * `parent_id` - The parent task's ID
/// * `index` - The child index
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_child_task_id;
/// let id = build_child_task_id("parent-123", 7);
/// assert_eq!(id, "parent-123-child-7");
/// ```
#[inline]
pub fn build_child_task_id(parent_id: &str, index: usize) -> String {
    // Capacity: parent_id + "-child-" (7) + digits
    const SUFFIX_LEN: usize = 7; // "-child-"
    let capacity = parent_id.len() + SUFFIX_LEN + digit_count(index);
    let mut result = String::with_capacity(capacity);
    let _ = write!(result, "{parent_id}-child-{index}");
    result
}

/// Builds a task title with level suffix: "{title} (level {level})" into a buffer.
///
/// This function clears the buffer and writes the result, enabling buffer reuse.
///
/// # Arguments
///
/// * `title` - The base title string
/// * `level` - The level number to append
/// * `buffer` - Mutable string buffer to write into (will be cleared first)
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_task_title_with_level_into;
/// let mut buffer = String::with_capacity(64);
/// build_task_title_with_level_into("My Task", 2, &mut buffer);
/// assert_eq!(buffer, "My Task (level 2)");
/// ```
#[inline]
pub fn build_task_title_with_level_into(title: &str, level: usize, buffer: &mut String) {
    const PREFIX_LEN: usize = 8; // " (level "
    const SUFFIX_LEN: usize = 1; // ")"
    buffer.clear();
    let required = title.len() + PREFIX_LEN + digit_count(level) + SUFFIX_LEN;
    if buffer.capacity() < required {
        buffer.reserve(required);
    }
    let _ = write!(buffer, "{title} (level {level})");
}

/// Builds a task title with level suffix: "{title} (level {level})".
///
/// This is a convenience wrapper that allocates a new string.
/// For hot paths, prefer [`build_task_title_with_level_into`] with buffer reuse.
///
/// # Arguments
///
/// * `title` - The base title string
/// * `level` - The level number to append
///
/// # Examples
///
/// ```
/// use task_management_benchmark_api::api::string_buffer::build_task_title_with_level;
/// let title = build_task_title_with_level("My Task", 2);
/// assert_eq!(title, "My Task (level 2)");
/// ```
#[inline]
pub fn build_task_title_with_level(title: &str, level: usize) -> String {
    // Capacity: title + " (level " (8) + digits + ")" (1)
    const PREFIX_LEN: usize = 8; // " (level "
    const SUFFIX_LEN: usize = 1; // ")"
    let capacity = title.len() + PREFIX_LEN + digit_count(level) + SUFFIX_LEN;
    let mut result = String::with_capacity(capacity);
    let _ = write!(result, "{title} (level {level})");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // digit_count tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(9, 1)]
    #[case(10, 2)]
    #[case(99, 2)]
    #[case(100, 3)]
    #[case(999, 3)]
    #[case(1000, 4)]
    #[case(12345, 5)]
    fn test_digit_count(#[case] value: usize, #[case] expected: usize) {
        assert_eq!(digit_count(value), expected);
    }

    #[rstest]
    fn test_digit_count_max() {
        // Handle both 32-bit and 64-bit platforms
        let expected = usize::MAX.to_string().len();
        assert_eq!(digit_count(usize::MAX), expected);
    }

    // -------------------------------------------------------------------------
    // build_task_id tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task_id_basic() {
        let result = build_task_id("task", 42);
        assert_eq!(result, "task-42");
    }

    #[rstest]
    fn test_build_task_id_zero_index() {
        let result = build_task_id("prefix", 0);
        assert_eq!(result, "prefix-0");
    }

    #[rstest]
    fn test_build_task_id_large_index() {
        let result = build_task_id("id", 123_456_789);
        assert_eq!(result, "id-123456789");
    }

    #[rstest]
    fn test_build_task_id_empty_prefix() {
        let result = build_task_id("", 5);
        assert_eq!(result, "-5");
    }

    #[rstest]
    fn test_build_task_id_capacity_preallocation() {
        // Verify pre-allocation prevents reallocation during write
        // Expected length: "task" (4) + "-" (1) + "42" (2) = 7
        let prefix = "task";
        let index = 42;
        let expected_len = prefix.len() + 1 + digit_count(index);

        let result = build_task_id(prefix, index);

        // The result should match expected length
        assert_eq!(result.len(), expected_len);
        // Pre-allocated capacity should match what we requested
        // (String::with_capacity(n).capacity() may round up, but should be >= n)
        let preallocated_capacity = String::with_capacity(expected_len).capacity();
        assert_eq!(result.capacity(), preallocated_capacity);
    }

    // -------------------------------------------------------------------------
    // build_subtask_title tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_subtask_title_basic() {
        let result = build_subtask_title(3, 5);
        assert_eq!(result, "Subtask at depth 3, index 5");
    }

    #[rstest]
    fn test_build_subtask_title_zeros() {
        let result = build_subtask_title(0, 0);
        assert_eq!(result, "Subtask at depth 0, index 0");
    }

    #[rstest]
    fn test_build_subtask_title_large_values() {
        let result = build_subtask_title(100, 999);
        assert_eq!(result, "Subtask at depth 100, index 999");
    }

    #[rstest]
    fn test_build_subtask_title_capacity_preallocation() {
        // Verify pre-allocation prevents reallocation during write
        // Expected: "Subtask at depth " (17) + "3" (1) + ", index " (8) + "5" (1) = 27
        let depth = 3;
        let index = 5;
        let expected_len = 17 + digit_count(depth) + 8 + digit_count(index);

        let result = build_subtask_title(depth, index);

        assert_eq!(result.len(), expected_len);
        let preallocated_capacity = String::with_capacity(expected_len).capacity();
        assert_eq!(result.capacity(), preallocated_capacity);
    }

    // -------------------------------------------------------------------------
    // build_child_task_id tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_child_task_id_basic() {
        let result = build_child_task_id("parent-123", 7);
        assert_eq!(result, "parent-123-child-7");
    }

    #[rstest]
    fn test_build_child_task_id_zero_index() {
        let result = build_child_task_id("root", 0);
        assert_eq!(result, "root-child-0");
    }

    #[rstest]
    fn test_build_child_task_id_nested() {
        let result = build_child_task_id("task-1-child-2", 3);
        assert_eq!(result, "task-1-child-2-child-3");
    }

    #[rstest]
    fn test_build_child_task_id_capacity_preallocation() {
        // Verify pre-allocation prevents reallocation during write
        // Expected: "parent" (6) + "-child-" (7) + "7" (1) = 14
        let parent_id = "parent";
        let index = 7;
        let expected_len = parent_id.len() + 7 + digit_count(index);

        let result = build_child_task_id(parent_id, index);

        assert_eq!(result.len(), expected_len);
        let preallocated_capacity = String::with_capacity(expected_len).capacity();
        assert_eq!(result.capacity(), preallocated_capacity);
    }

    // -------------------------------------------------------------------------
    // build_task_title_with_level tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task_title_with_level_basic() {
        let result = build_task_title_with_level("My Task", 2);
        assert_eq!(result, "My Task (level 2)");
    }

    #[rstest]
    fn test_build_task_title_with_level_zero() {
        let result = build_task_title_with_level("Root", 0);
        assert_eq!(result, "Root (level 0)");
    }

    #[rstest]
    fn test_build_task_title_with_level_large() {
        let result = build_task_title_with_level("Deep Task", 999);
        assert_eq!(result, "Deep Task (level 999)");
    }

    #[rstest]
    fn test_build_task_title_with_level_capacity_preallocation() {
        // Verify pre-allocation prevents reallocation during write
        // Expected: "Task" (4) + " (level " (8) + "5" (1) + ")" (1) = 14
        let title = "Task";
        let level = 5;
        let expected_len = title.len() + 8 + digit_count(level) + 1;

        let result = build_task_title_with_level(title, level);

        assert_eq!(result.len(), expected_len);
        let preallocated_capacity = String::with_capacity(expected_len).capacity();
        assert_eq!(result.capacity(), preallocated_capacity);
    }

    // -------------------------------------------------------------------------
    // Referential transparency tests (PL-001)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_referential_transparency_build_task_id() {
        // Same inputs must produce identical outputs
        let result1 = build_task_id("test", 42);
        let result2 = build_task_id("test", 42);
        assert_eq!(result1, result2);
    }

    #[rstest]
    fn test_referential_transparency_build_subtask_title() {
        let result1 = build_subtask_title(5, 10);
        let result2 = build_subtask_title(5, 10);
        assert_eq!(result1, result2);
    }

    #[rstest]
    fn test_referential_transparency_build_child_task_id() {
        let result1 = build_child_task_id("parent", 3);
        let result2 = build_child_task_id("parent", 3);
        assert_eq!(result1, result2);
    }

    #[rstest]
    fn test_referential_transparency_build_task_title_with_level() {
        let result1 = build_task_title_with_level("Task", 7);
        let result2 = build_task_title_with_level("Task", 7);
        assert_eq!(result1, result2);
    }

    // -------------------------------------------------------------------------
    // Independence tests (pure function output isolation)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_independent_outputs() {
        // Multiple calls produce independent strings that don't share memory
        let id1 = build_task_id("first", 1);
        let id2 = build_task_id("second", 2);

        // Values are independent
        assert_eq!(id1, "first-1");
        assert_eq!(id2, "second-2");

        // Pointers are different (no shared backing memory)
        assert_ne!(id1.as_ptr(), id2.as_ptr());
    }

    #[rstest]
    fn test_independent_title_outputs() {
        let title1 = build_subtask_title(0, 0);
        let title2 = build_task_title_with_level("Test", 5);

        assert_eq!(title1, "Subtask at depth 0, index 0");
        assert_eq!(title2, "Test (level 5)");
        assert_ne!(title1.as_ptr(), title2.as_ptr());
    }

    // -------------------------------------------------------------------------
    // Buffer reuse tests (*_into functions) - PL-002
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task_id_into_basic() {
        let mut buffer = String::with_capacity(32);
        build_task_id_into("task", 42, &mut buffer);
        assert_eq!(buffer, "task-42");
    }

    #[rstest]
    fn test_build_task_id_into_buffer_reuse() {
        let mut buffer = String::with_capacity(32);

        // First call
        build_task_id_into("first", 1, &mut buffer);
        assert_eq!(buffer, "first-1");
        let ptr1 = buffer.as_ptr();

        // Second call reuses buffer (no new allocation if capacity sufficient)
        build_task_id_into("second", 2, &mut buffer);
        assert_eq!(buffer, "second-2");
        let ptr2 = buffer.as_ptr();

        // Same backing allocation (pointer should be the same)
        assert_eq!(ptr1, ptr2, "Buffer should reuse same allocation");
    }

    #[rstest]
    fn test_build_task_id_into_clears_previous_content() {
        let mut buffer = String::from("previous content that should be cleared");
        build_task_id_into("new", 99, &mut buffer);
        assert_eq!(buffer, "new-99");
    }

    #[rstest]
    fn test_build_subtask_title_into_basic() {
        let mut buffer = String::with_capacity(64);
        build_subtask_title_into(3, 5, &mut buffer);
        assert_eq!(buffer, "Subtask at depth 3, index 5");
    }

    #[rstest]
    fn test_build_subtask_title_into_buffer_reuse() {
        let mut buffer = String::with_capacity(64);

        build_subtask_title_into(1, 2, &mut buffer);
        let ptr1 = buffer.as_ptr();

        build_subtask_title_into(3, 4, &mut buffer);
        let ptr2 = buffer.as_ptr();

        assert_eq!(ptr1, ptr2, "Buffer should reuse same allocation");
    }

    #[rstest]
    fn test_build_child_task_id_into_basic() {
        let mut buffer = String::with_capacity(64);
        build_child_task_id_into("parent-123", 7, &mut buffer);
        assert_eq!(buffer, "parent-123-child-7");
    }

    #[rstest]
    fn test_build_child_task_id_into_buffer_reuse() {
        let mut buffer = String::with_capacity(64);

        build_child_task_id_into("parent", 1, &mut buffer);
        let ptr1 = buffer.as_ptr();

        build_child_task_id_into("other", 2, &mut buffer);
        let ptr2 = buffer.as_ptr();

        assert_eq!(ptr1, ptr2, "Buffer should reuse same allocation");
    }

    #[rstest]
    fn test_build_child_task_id_into_empty_parent() {
        let mut buffer = String::with_capacity(32);
        build_child_task_id_into("", 5, &mut buffer);
        assert_eq!(buffer, "-child-5");
    }

    #[rstest]
    fn test_build_task_title_with_level_into_basic() {
        let mut buffer = String::with_capacity(64);
        build_task_title_with_level_into("My Task", 2, &mut buffer);
        assert_eq!(buffer, "My Task (level 2)");
    }

    #[rstest]
    fn test_build_task_title_with_level_into_buffer_reuse() {
        let mut buffer = String::with_capacity(64);

        build_task_title_with_level_into("Task A", 1, &mut buffer);
        let ptr1 = buffer.as_ptr();

        build_task_title_with_level_into("Task B", 2, &mut buffer);
        let ptr2 = buffer.as_ptr();

        assert_eq!(ptr1, ptr2, "Buffer should reuse same allocation");
    }

    #[rstest]
    fn test_build_task_title_with_level_into_empty_title() {
        let mut buffer = String::with_capacity(32);
        build_task_title_with_level_into("", 5, &mut buffer);
        assert_eq!(buffer, " (level 5)");
    }

    // -------------------------------------------------------------------------
    // Edge case tests - empty strings
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_child_task_id_empty_parent_id() {
        let result = build_child_task_id("", 0);
        assert_eq!(result, "-child-0");
    }

    #[rstest]
    fn test_build_task_title_with_level_empty_title() {
        let result = build_task_title_with_level("", 0);
        assert_eq!(result, " (level 0)");
    }

    // -------------------------------------------------------------------------
    // Referential transparency for *_into functions
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_referential_transparency_build_task_id_into() {
        let mut buffer1 = String::with_capacity(32);
        let mut buffer2 = String::with_capacity(32);

        build_task_id_into("test", 42, &mut buffer1);
        build_task_id_into("test", 42, &mut buffer2);

        assert_eq!(buffer1, buffer2);
    }

    #[rstest]
    fn test_referential_transparency_build_subtask_title_into() {
        let mut buffer1 = String::with_capacity(64);
        let mut buffer2 = String::with_capacity(64);

        build_subtask_title_into(5, 10, &mut buffer1);
        build_subtask_title_into(5, 10, &mut buffer2);

        assert_eq!(buffer1, buffer2);
    }

    #[rstest]
    fn test_referential_transparency_build_child_task_id_into() {
        let mut buffer1 = String::with_capacity(64);
        let mut buffer2 = String::with_capacity(64);

        build_child_task_id_into("parent", 3, &mut buffer1);
        build_child_task_id_into("parent", 3, &mut buffer2);

        assert_eq!(buffer1, buffer2);
    }

    #[rstest]
    fn test_referential_transparency_build_task_title_with_level_into() {
        let mut buffer1 = String::with_capacity(64);
        let mut buffer2 = String::with_capacity(64);

        build_task_title_with_level_into("Task", 7, &mut buffer1);
        build_task_title_with_level_into("Task", 7, &mut buffer2);

        assert_eq!(buffer1, buffer2);
    }
}
