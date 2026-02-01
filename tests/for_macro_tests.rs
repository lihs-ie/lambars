//! Unit tests for the for_! macro.
//!
//! These tests verify that the for_! macro correctly implements
//! Scala-style for-comprehension over iterators.

#![cfg(feature = "compose")]

use lambars::for_;

// =============================================================================
// Single Iteration Tests
// =============================================================================

#[test]
fn test_yield_only() {
    let result = for_! {
        yield 42
    };
    assert_eq!(result, vec![42]);
}

#[test]
fn test_single_iteration_vec() {
    let result = for_! {
        x <= vec![1, 2, 3];
        yield x * 2
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_single_iteration_array() {
    let result = for_! {
        x <= [1, 2, 3];
        yield x + 10
    };
    assert_eq!(result, vec![11, 12, 13]);
}

#[test]
fn test_single_iteration_range() {
    let result = for_! {
        x <= 1..4;
        yield x * x
    };
    assert_eq!(result, vec![1, 4, 9]);
}

// =============================================================================
// Nested Iteration Tests
// =============================================================================

#[test]
fn test_nested_iteration_two_levels() {
    let result = for_! {
        x <= vec![1, 2];
        y <= vec![10, 20];
        yield x + y
    };
    assert_eq!(result, vec![11, 21, 12, 22]);
}

#[test]
fn test_nested_iteration_three_levels() {
    let result = for_! {
        x <= vec![1, 2];
        y <= vec![10, 20];
        z <= vec![100, 200];
        yield x + y + z
    };
    assert_eq!(result, vec![111, 211, 121, 221, 112, 212, 122, 222]);
}

#[test]
fn test_nested_iteration_dependent() {
    let result = for_! {
        x <= vec![1, 2, 3];
        y <= (0..x).collect::<Vec<_>>();
        yield (x, y)
    };
    assert_eq!(result, vec![(1, 0), (2, 0), (2, 1), (3, 0), (3, 1), (3, 2)]);
}

// =============================================================================
// Empty Collection Tests
// =============================================================================

#[test]
fn test_empty_source_collection() {
    let empty: Vec<i32> = vec![];
    let result = for_! {
        x <= empty;
        yield x * 2
    };
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_empty_nested_collection() {
    let result = for_! {
        x <= vec![1, 2, 3];
        y <= if x == 2 { vec![] } else { vec![x] };
        yield y
    };
    assert_eq!(result, vec![1, 3]);
}

// =============================================================================
// Tuple Pattern Tests
// =============================================================================

#[test]
fn test_tuple_pattern_simple() {
    let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
    let result = for_! {
        (num, letter) <= pairs;
        yield format!("{}{}", num, letter)
    };
    assert_eq!(result, vec!["1a", "2b", "3c"]);
}

#[test]
fn test_tuple_pattern_nested() {
    let nested = vec![((1, 2), "a"), ((3, 4), "b")];
    let result = for_! {
        ((x, y), label) <= nested;
        yield format!("{}: ({}, {})", label, x, y)
    };
    assert_eq!(result, vec!["a: (1, 2)", "b: (3, 4)"]);
}

// =============================================================================
// Wildcard Pattern Tests
// =============================================================================

#[test]
fn test_wildcard_in_tuple() {
    let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
    let result = for_! {
        (_, letter) <= pairs;
        yield letter.to_uppercase()
    };
    assert_eq!(result, vec!["A", "B", "C"]);
}

#[test]
fn test_wildcard_full_element() {
    let result = for_! {
        _ <= vec![1, 2, 3];
        yield "x"
    };
    assert_eq!(result, vec!["x", "x", "x"]);
}

// =============================================================================
// Let Binding Tests
// =============================================================================

#[test]
fn test_let_binding_simple() {
    let result = for_! {
        x <= vec![1, 2, 3];
        let doubled = x * 2;
        yield doubled
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_let_binding_multiple() {
    let result = for_! {
        x <= vec![1, 2, 3];
        let doubled = x * 2;
        let squared = doubled * doubled;
        yield squared
    };
    assert_eq!(result, vec![4, 16, 36]);
}

#[test]
fn test_let_binding_with_nested_iteration() {
    let result = for_! {
        x <= vec![1, 2];
        let x_squared = x * x;
        y <= vec![10, 20];
        let sum = x_squared + y;
        yield sum
    };
    assert_eq!(result, vec![11, 21, 14, 24]);
}

#[test]
fn test_let_tuple_binding() {
    let result = for_! {
        pair <= vec![(1, 2), (3, 4), (5, 6)];
        let (a, b) = pair;
        yield a + b
    };
    assert_eq!(result, vec![3, 7, 11]);
}

// =============================================================================
// Scala Recommendation Feed Example
// =============================================================================

#[test]
fn test_recommendation_feed_example() {
    use std::rc::Rc;

    #[derive(Clone)]
    struct Book {
        title: Rc<str>,
        authors: Vec<Rc<str>>,
    }

    #[derive(Clone)]
    struct Movie {
        title: String,
    }

    fn book_adaptations(author: &str) -> Vec<Movie> {
        match author {
            "Author1" => vec![Movie {
                title: "Movie1".to_string(),
            }],
            "Author2" => vec![
                Movie {
                    title: "Movie2".to_string(),
                },
                Movie {
                    title: "Movie3".to_string(),
                },
            ],
            _ => vec![],
        }
    }

    let books = vec![
        Book {
            title: Rc::from("Book1"),
            authors: vec![Rc::from("Author1")],
        },
        Book {
            title: Rc::from("Book2"),
            authors: vec![Rc::from("Author2")],
        },
    ];

    // Use Rc for shared ownership to avoid FnMut move constraints
    let result = for_! {
        book <= books.clone();
        let book_title = Rc::clone(&book.title);
        let authors = book.authors.clone();
        author <= authors;
        let author_ref = Rc::clone(&author);
        let book_title_inner = Rc::clone(&book_title);
        movie <= book_adaptations(&author);
        yield format!(
            "You may like {}, because you liked {}'s {}",
            movie.title, author_ref, book_title_inner
        )
    };

    assert_eq!(
        result,
        vec![
            "You may like Movie1, because you liked Author1's Book1",
            "You may like Movie2, because you liked Author2's Book2",
            "You may like Movie3, because you liked Author2's Book2",
        ]
    );
}

// =============================================================================
// Additional Edge Case Tests
// =============================================================================

#[test]
fn test_with_string_collection() {
    let words = vec!["hello", "world"];
    let result = for_! {
        word <= words;
        yield word.to_uppercase()
    };
    assert_eq!(result, vec!["HELLO", "WORLD"]);
}

#[test]
fn test_with_option_in_yield() {
    let numbers = vec![1, 2, 3];
    let result = for_! {
        n <= numbers;
        yield Some(n * 2)
    };
    assert_eq!(result, vec![Some(2), Some(4), Some(6)]);
}

#[test]
fn test_complex_expression_in_bind() {
    let result = for_! {
        x <= (1..=3).map(|n| n * n).collect::<Vec<_>>();
        yield x + 1
    };
    assert_eq!(result, vec![2, 5, 10]); // 1+1, 4+1, 9+1
}

#[test]
fn test_reference_in_yield() {
    let data = vec![vec![1, 2], vec![3, 4]];
    let result = for_! {
        inner <= data;
        x <= inner;
        yield x
    };
    assert_eq!(result, vec![1, 2, 3, 4]);
}

// =============================================================================
// Result Identical Tests (Phase 5)
// =============================================================================
// These tests verify that the for_! macro produces results identical to
// manual iterator operations, ensuring correctness after optimization.

use rstest::rstest;

#[rstest]
fn test_result_identical_single() {
    let input = vec![1, 2, 3, 4, 5];
    let result = for_! { x <= input.clone(); yield x * 2 };
    assert_eq!(result, vec![2, 4, 6, 8, 10]);
}

#[rstest]
fn test_result_identical_nested() {
    let xs = vec![1, 2];
    let ys = vec![10, 20];
    let result = for_! { x <= xs; y <= ys.clone(); yield x + y };
    assert_eq!(result, vec![11, 21, 12, 22]);
}

#[rstest]
fn test_result_identical_with_guard() {
    let input = vec![1, 2, 3, 4, 5];
    let result = for_! { x <= input; if x % 2 == 0; yield x };
    assert_eq!(result, vec![2, 4]);
}

#[rstest]
fn test_result_identical_empty() {
    let input: Vec<i32> = vec![];
    let result = for_! { x <= input; yield x * 2 };
    assert_eq!(result, Vec::<i32>::new());
}

#[rstest]
fn test_result_identical_large_scale() {
    let input: Vec<i32> = (0..100_000).collect();
    let result = for_! { x <= input.clone(); yield x * 2 };
    let expected: Vec<i32> = input.into_iter().map(|x| x * 2).collect();
    assert_eq!(result, expected);
}

// =============================================================================
// Edge Case Tests (Phase 5)
// =============================================================================

#[rstest]
fn test_edge_case_single_element() {
    let input = vec![1];
    let result = for_! { x <= input; yield x * 2 };
    assert_eq!(result, vec![2]);
}

#[rstest]
fn test_edge_case_three_level_nesting_5x5x5() {
    let xs = vec![1, 2, 3, 4, 5];
    let ys = vec![10, 20, 30, 40, 50];
    let zs = vec![100, 200, 300, 400, 500];

    // Clone for for_! macro usage
    let ys_for = ys.clone();
    let zs_for = zs.clone();

    let result = for_! {
        x <= xs.clone();
        let ys_inner = ys_for.clone();
        let zs_for_y = zs_for.clone();
        y <= ys_inner;
        let zs_inner = zs_for_y.clone();
        z <= zs_inner;
        yield x + y + z
    };

    // 5 * 5 * 5 = 125 elements
    assert_eq!(result.len(), 125);

    // Verify against manual flat_map implementation
    let expected: Vec<i32> = xs
        .into_iter()
        .flat_map(|x| {
            let zs_inner = zs.clone();
            ys.clone()
                .into_iter()
                .flat_map(move |y| zs_inner.clone().into_iter().map(move |z| x + y + z))
        })
        .collect();

    assert_eq!(result, expected);
}

#[rstest]
fn test_edge_case_all_guard_excluded() {
    let input = vec![1, 3, 5];
    let result = for_! { x <= input; if x % 2 == 0; yield x };
    assert_eq!(result, Vec::<i32>::new());
}

#[rstest]
fn test_edge_case_boundary_value_i32_max() {
    // Use saturating_mul to avoid overflow
    let input = vec![i32::MAX];
    let result = for_! { x <= input; yield x.saturating_mul(2) };
    assert_eq!(result, vec![i32::MAX]); // saturating_mul(2) returns MAX for overflow
}

#[rstest]
fn test_edge_case_boundary_value_i32_min() {
    // Use saturating_mul to avoid overflow
    let input = vec![i32::MIN];
    let result = for_! { x <= input; yield x.saturating_mul(2) };
    assert_eq!(result, vec![i32::MIN]); // saturating_mul(2) returns MIN for underflow
}

#[rstest]
fn test_edge_case_mixed_positive_negative() {
    let input: Vec<i32> = vec![-2, -1, 0, 1, 2];
    let result = for_! { x <= input.clone(); yield x.saturating_mul(x) };
    let expected: Vec<i32> = input.into_iter().map(|x| x.saturating_mul(x)).collect();
    assert_eq!(result, expected);
}

#[rstest]
fn test_edge_case_nested_with_empty_inner() {
    // When inner collection is empty, no elements should be produced
    let xs = vec![1, 2, 3];
    let ys: Vec<i32> = vec![];

    let result = for_! {
        _x <= xs;
        y <= ys.clone();
        yield y
    };

    assert_eq!(result, Vec::<i32>::new());
}

#[rstest]
fn test_edge_case_nested_with_conditional_empty_inner() {
    // Inner collection is conditionally empty
    let xs = vec![1, 2, 3];

    let result = for_! {
        x <= xs;
        y <= if x == 2 { vec![] } else { vec![x * 10] };
        yield y
    };

    assert_eq!(result, vec![10, 30]);
}

#[rstest]
fn test_result_identical_with_let_binding() {
    let input = vec![1, 2, 3, 4, 5];
    let result = for_! {
        x <= input.clone();
        let squared = x * x;
        let doubled = squared * 2;
        yield doubled
    };
    let expected: Vec<i32> = input
        .into_iter()
        .map(|x| {
            let squared = x * x;
            squared * 2
        })
        .collect();
    assert_eq!(result, expected);
}

#[rstest]
fn test_result_identical_nested_with_guard() {
    let xs = vec![1, 2, 3, 4];
    let ys = vec![10, 20, 30];

    // Clone for for_! macro usage
    let ys_for = ys.clone();

    let result = for_! {
        x <= xs.clone();
        y <= ys_for.clone();
        if (x + y) % 2 == 0;
        yield x + y
    };

    let expected: Vec<i32> = xs
        .into_iter()
        .flat_map(|x| {
            ys.clone()
                .into_iter()
                .filter(move |&y| (x + y) % 2 == 0)
                .map(move |y| x + y)
        })
        .collect();

    assert_eq!(result, expected);
}

#[rstest]
fn test_result_identical_pattern_guard() {
    fn maybe_double(x: i32) -> Option<i32> {
        if x > 0 {
            Some(x.saturating_mul(2))
        } else {
            None
        }
    }

    let input = vec![-2, -1, 0, 1, 2, 3];

    let result = for_! {
        x <= input.clone();
        if let Some(doubled) = maybe_double(x);
        yield doubled
    };

    let expected: Vec<i32> = input.into_iter().filter_map(maybe_double).collect();

    assert_eq!(result, expected);
}

/// TEST-001: 8-element boundary (original SmallVec[T; 8] design)
mod boundary_8_element_tests {
    use lambars::for_;
    use rstest::rstest;

    #[rstest]
    fn test_7_elements_below_original_boundary() {
        let input: Vec<i32> = (1..=7).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=7).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 7);
    }

    #[rstest]
    fn test_8_elements_at_original_boundary() {
        let input: Vec<i32> = (1..=8).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=8).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 8);
    }

    #[rstest]
    fn test_9_elements_above_original_boundary() {
        let input: Vec<i32> = (1..=9).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=9).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 9);
    }

    #[rstest]
    fn test_8_elements_with_nested_iteration() {
        let xs: Vec<i32> = (1..=2).collect();
        let ys: Vec<i32> = (1..=4).collect();
        // 2 * 4 = 8 elements total
        let result = for_! {
            x <= xs;
            y <= ys.clone();
            yield x * 10 + y
        };
        assert_eq!(result.len(), 8);
        assert_eq!(result, vec![11, 12, 13, 14, 21, 22, 23, 24]);
    }

    #[rstest]
    fn test_8_elements_with_guard_filtering() {
        // Start with 16 elements, filter to 8
        let input: Vec<i32> = (1..=16).collect();
        let result = for_! {
            x <= input;
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result.len(), 8);
        assert_eq!(result, vec![2, 4, 6, 8, 10, 12, 14, 16]);
    }
}

/// TEST-002: 128-element boundary (SMALLVEC_INLINE_CAPACITY)
mod boundary_128_element_tests {
    use lambars::compose::for_macro::SMALLVEC_INLINE_CAPACITY;
    use lambars::for_;
    use rstest::rstest;

    #[rstest]
    fn test_127_elements_below_inline_capacity() {
        let input: Vec<i32> = (1..=127).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=127).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 127);
        assert!(result.len() < SMALLVEC_INLINE_CAPACITY);
    }

    #[rstest]
    fn test_128_elements_at_inline_capacity() {
        let input: Vec<i32> = (1..=128).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=128).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 128);
        assert_eq!(result.len(), SMALLVEC_INLINE_CAPACITY);
    }

    #[rstest]
    fn test_129_elements_above_inline_capacity() {
        let input: Vec<i32> = (1..=129).collect();
        let result = for_! { x <= input.clone(); yield x * 2 };
        let expected: Vec<i32> = (1..=129).map(|x| x * 2).collect();
        assert_eq!(result, expected);
        assert_eq!(result.len(), 129);
        assert!(result.len() > SMALLVEC_INLINE_CAPACITY);
    }

    #[rstest]
    fn test_128_elements_with_nested_iteration() {
        // 8 * 16 = 128 elements total
        let xs: Vec<i32> = (1..=8).collect();
        let ys: Vec<i32> = (1..=16).collect();
        let result = for_! {
            x <= xs;
            y <= ys.clone();
            yield x * 100 + y
        };
        assert_eq!(result.len(), 128);
        // Verify first and last elements
        assert_eq!(result[0], 101);
        assert_eq!(result[127], 816);
    }

    #[rstest]
    fn test_above_inline_capacity_with_guard() {
        // Start with 256 elements, filter to ~128
        let input: Vec<i32> = (1..=256).collect();
        let result = for_! {
            x <= input;
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result.len(), 128);
        assert_eq!(result[0], 2);
        assert_eq!(result[127], 256);
    }

    #[rstest]
    fn test_capacity_hint_preserved_at_boundary() {
        // Verify that size_hint is computed correctly at the boundary
        let input: Vec<i32> = (1..=128).collect();
        let cloned = input.clone();
        let hint = cloned.into_iter().size_hint();
        assert_eq!(hint, (128, Some(128)));

        let result = for_! { x <= input; yield x };
        assert_eq!(result.len(), 128);
    }
}

/// TEST-003: Large element (4KB struct) stack safety
mod large_element_stack_tests {
    use lambars::compose::for_macro::compute_smallvec_threshold;
    use lambars::for_;
    use rstest::rstest;

    #[derive(Clone, Debug, PartialEq)]
    #[repr(C)]
    struct LargeStruct4KB {
        data: [u8; 4096],
    }

    impl LargeStruct4KB {
        fn new(value: u8) -> Self {
            Self {
                data: [value; 4096],
            }
        }

        fn value(&self) -> u8 {
            self.data[0]
        }
    }

    #[rstest]
    fn test_large_struct_threshold_calculation() {
        let threshold = compute_smallvec_threshold::<LargeStruct4KB>();
        assert_eq!(threshold, 8); // 32KB / 4KB = 8
    }

    #[rstest]
    fn test_large_struct_single_element() {
        let input = vec![LargeStruct4KB::new(42)];
        let result = for_! {
            x <= input;
            yield x.value()
        };
        assert_eq!(result, vec![42]);
    }

    #[rstest]
    fn test_large_struct_below_threshold() {
        // 7 elements (below threshold of 8)
        let input: Vec<LargeStruct4KB> = (0..7).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            yield x.value()
        };
        assert_eq!(result.len(), 7);
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn test_large_struct_at_threshold() {
        // 8 elements (at threshold)
        let input: Vec<LargeStruct4KB> = (0..8).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            yield x.value()
        };
        assert_eq!(result.len(), 8);
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[rstest]
    fn test_large_struct_above_threshold() {
        // 10 elements (above threshold of 8)
        let input: Vec<LargeStruct4KB> = (0..10).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            yield x.value()
        };
        assert_eq!(result.len(), 10);
    }

    #[rstest]
    fn test_large_struct_with_nested_iteration() {
        let xs: Vec<LargeStruct4KB> = (0..2).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let ys: Vec<LargeStruct4KB> = (0..3).map(|i| LargeStruct4KB::new(i as u8 * 10)).collect();
        let result = for_! {
            x <= xs;
            y <= ys.clone();
            yield x.value() as u16 + y.value() as u16
        };
        assert_eq!(result.len(), 6);
        assert_eq!(result, vec![0, 10, 20, 1, 11, 21]);
    }

    #[rstest]
    fn test_large_struct_with_guard() {
        // Filter half of the elements
        let input: Vec<LargeStruct4KB> = (0..8).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            if x.value() % 2 == 0;
            yield x.value()
        };
        assert_eq!(result.len(), 4);
        assert_eq!(result, vec![0, 2, 4, 6]);
    }

    #[rstest]
    fn test_large_struct_preserves_data_integrity() {
        // Ensure data is not corrupted during collection
        let input: Vec<LargeStruct4KB> = (0..5).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            yield x
        };
        for (i, item) in result.iter().enumerate() {
            assert_eq!(item.value(), i as u8);
            // Verify entire data array
            for byte in &item.data {
                assert_eq!(*byte, i as u8);
            }
        }
    }

    #[rstest]
    fn test_no_stack_overflow_with_many_large_elements() {
        let input: Vec<LargeStruct4KB> = (0..100).map(|i| LargeStruct4KB::new(i as u8)).collect();
        let result = for_! {
            x <= input;
            yield x.value()
        };
        assert_eq!(result.len(), 100);
    }
}

/// TEST-004: Nested size_hint composition
mod nested_size_hint_tests {
    use lambars::compose::for_macro::combined_size_hint;
    use lambars::for_;
    use rstest::rstest;

    #[rstest]
    fn test_combined_hint_two_exact_iterators() {
        let hints = [(10, Some(10)), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 200); // 10 * 20
    }

    #[rstest]
    fn test_combined_hint_three_exact_iterators() {
        let hints = [(5, Some(5)), (5, Some(5)), (5, Some(5))];
        assert_eq!(combined_size_hint(&hints), 125); // 5^3
    }

    #[rstest]
    fn test_combined_hint_with_unknown_upper() {
        let hints = [(10, None), (20, Some(20))];
        assert_eq!(combined_size_hint(&hints), 200); // falls back to lower product
    }

    #[rstest]
    fn test_combined_hint_all_unknown_upper() {
        let hints = [(5, None), (10, None), (20, None)];
        assert_eq!(combined_size_hint(&hints), 1000);
    }

    #[rstest]
    fn test_combined_hint_with_zero_lower() {
        let hints = [(0, Some(10)), (5, Some(5))];
        assert_eq!(combined_size_hint(&hints), 50); // uses upper product
    }

    #[rstest]
    fn test_combined_hint_with_zero_lower_unknown_upper() {
        let hints = [(0, None), (5, Some(5))];
        assert_eq!(combined_size_hint(&hints), 0); // falls back to 0
    }

    #[rstest]
    fn test_combined_hint_single_iterator() {
        let hints = [(42, Some(42))];
        assert_eq!(combined_size_hint(&hints), 42);
    }

    #[rstest]
    fn test_combined_hint_empty_hints() {
        assert_eq!(combined_size_hint(&[]), 0);
    }

    #[rstest]
    fn test_combined_hint_overflow_protection() {
        let hints = [(usize::MAX, Some(usize::MAX)), (2, Some(2))];
        assert_eq!(combined_size_hint(&hints), usize::MAX); // saturates
    }

    #[rstest]
    fn test_combined_hint_large_product() {
        let hints = [(1000, Some(1000)), (1000, Some(1000))];
        assert_eq!(combined_size_hint(&hints), 1_000_000); // 1000^2
    }

    #[rstest]
    fn test_nested_for_produces_correct_count() {
        let xs = vec![1, 2, 3];
        let ys = vec![10, 20, 30, 40];
        let result = for_! {
            x <= xs;
            y <= ys.clone();
            yield x + y
        };
        assert_eq!(result.len(), 12); // 3 * 4
    }

    #[rstest]
    fn test_triple_nested_for_produces_correct_count() {
        let xs = vec![1, 2];
        let ys = vec![10, 20, 30];
        let zs = vec![100, 200, 300, 400];
        let result = for_! {
            x <= xs;
            let ys_inner = ys.clone();
            let zs_outer = zs.clone();
            y <= ys_inner;
            let zs_inner = zs_outer.clone();
            z <= zs_inner;
            yield x + y + z
        };
        assert_eq!(result.len(), 24); // 2 * 3 * 4
        assert_eq!(result[0], 111); // 1 + 10 + 100
        assert_eq!(result[23], 432); // 2 + 30 + 400
    }

    #[rstest]
    fn test_nested_with_filter_reduces_count() {
        let xs = vec![1, 2, 3, 4, 5];
        let ys = vec![1, 2, 3, 4, 5];
        let result = for_! {
            x <= xs;
            y <= ys.clone();
            if (x + y) % 2 == 0;
            yield x + y
        };
        assert_eq!(result.len(), 13); // even sums from 5x5 grid
    }

    #[rstest]
    fn test_hint_with_let_binding_preserves_count() {
        let xs = vec![1, 2, 3, 4, 5];
        let result = for_! {
            x <= xs;
            let squared = x * x;
            yield squared
        };
        assert_eq!(result.len(), 5);
        assert_eq!(result, vec![1, 4, 9, 16, 25]);
    }

    #[rstest]
    fn test_combined_hint_referential_transparency() {
        let hints = [(10, Some(10)), (5, Some(5))];
        let result1 = combined_size_hint(&hints);
        let result2 = combined_size_hint(&hints);
        let result3 = combined_size_hint(&hints);
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
        assert_eq!(result1, 50);
    }
}

/// TEST-005: Capacity strategy verification tests
/// Verifies that collect_small and collect_with_capacity use correct allocation strategies
mod capacity_strategy_tests {
    use lambars::compose::for_macro::{
        L1_CACHE_SIZE, SMALLVEC_INLINE_CAPACITY, collect_small, collect_with_capacity,
        compute_smallvec_threshold, should_use_smallvec,
    };
    use rstest::rstest;

    // =========================================================================
    // should_use_smallvec pure function tests
    // =========================================================================

    #[rstest]
    fn test_should_use_smallvec_known_upper_returns_false() {
        // When upper is known, always use Vec directly
        assert!(!should_use_smallvec::<i32>(10, Some(10)));
        assert!(!should_use_smallvec::<i32>(0, Some(0)));
        assert!(!should_use_smallvec::<i32>(100, Some(100)));
    }

    #[rstest]
    fn test_should_use_smallvec_zero_lower_returns_false() {
        // When lower is 0 (from filter/flat_map), use Vec
        assert!(!should_use_smallvec::<i32>(0, None));
    }

    #[rstest]
    fn test_should_use_smallvec_small_element_small_lower_returns_true() {
        // Small element (i32) with small lower bound: use SmallVec
        assert!(should_use_smallvec::<i32>(10, None));
        assert!(should_use_smallvec::<i32>(50, None));
        assert!(should_use_smallvec::<i32>(128, None)); // at threshold
    }

    #[rstest]
    fn test_should_use_smallvec_small_element_large_lower_returns_false() {
        // Small element but lower > threshold: use Vec
        assert!(!should_use_smallvec::<i32>(129, None)); // above threshold
        assert!(!should_use_smallvec::<i32>(1000, None));
    }

    #[rstest]
    fn test_should_use_smallvec_large_element_returns_false() {
        // Large element (4KB): stack unsafe, always use Vec
        #[repr(C)]
        struct Large4KB {
            _data: [u8; 4096],
        }

        // Even with lower=1 (below threshold of 8), stack safety fails
        assert!(!should_use_smallvec::<Large4KB>(1, None));
        assert!(!should_use_smallvec::<Large4KB>(5, None));
        assert!(!should_use_smallvec::<Large4KB>(8, None));
    }

    #[rstest]
    fn test_should_use_smallvec_boundary_256_byte_element() {
        // 256-byte element: exactly at L1 cache boundary
        #[repr(C)]
        struct Boundary256 {
            _data: [u8; 256],
        }

        // 256 * 128 = 32KB = L1_CACHE_SIZE (stack safe)
        let inline_size = std::mem::size_of::<Boundary256>() * SMALLVEC_INLINE_CAPACITY;
        assert_eq!(inline_size, L1_CACHE_SIZE);

        // Should use SmallVec when lower is small
        assert!(should_use_smallvec::<Boundary256>(10, None));
        assert!(should_use_smallvec::<Boundary256>(128, None)); // threshold = 128
    }

    #[rstest]
    fn test_should_use_smallvec_boundary_257_byte_element() {
        // 257-byte element: just over L1 cache boundary
        #[repr(C)]
        struct JustOver257 {
            _data: [u8; 257],
        }

        // 257 * 128 = 32896 > L1_CACHE_SIZE (stack unsafe)
        let inline_size = std::mem::size_of::<JustOver257>() * SMALLVEC_INLINE_CAPACITY;
        assert!(inline_size > L1_CACHE_SIZE);

        // Should NOT use SmallVec even with small lower
        assert!(!should_use_smallvec::<JustOver257>(1, None));
        assert!(!should_use_smallvec::<JustOver257>(10, None));
    }

    #[rstest]
    fn test_should_use_smallvec_referential_transparency() {
        // Same inputs always produce same output (pure function)
        assert_eq!(
            should_use_smallvec::<i32>(10, None),
            should_use_smallvec::<i32>(10, None)
        );
        assert_eq!(
            should_use_smallvec::<i32>(10, Some(10)),
            should_use_smallvec::<i32>(10, Some(10))
        );
    }

    #[rstest]
    fn test_should_use_smallvec_zst() {
        // Zero-sized types (ZST) should work correctly
        // ZST has size 0, which is treated as 1 in compute_smallvec_threshold
        // inline_buffer_size = 0 * 128 = 0 <= L1_CACHE_SIZE (stack safe)
        // threshold = L1_CACHE_SIZE / 1 = 32KB, but capped at 128
        assert!(should_use_smallvec::<()>(10, None));
        assert!(should_use_smallvec::<()>(128, None)); // at threshold

        // ZST still respects other conditions
        assert!(!should_use_smallvec::<()>(10, Some(10))); // upper known
        assert!(!should_use_smallvec::<()>(0, None)); // lower is 0
        assert!(!should_use_smallvec::<()>(129, None)); // above threshold
    }

    #[rstest]
    fn test_collect_small_with_known_upper_uses_vec_directly() {
        // When upper is known (e.g., from Vec), should use Vec directly
        let input = vec![1, 2, 3, 4, 5];
        let result = collect_small(input.into_iter());
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn test_collect_small_with_unknown_upper_small_collection() {
        // When upper is unknown but lower is small, may use SmallVec path
        // Filter has size_hint (0, Some(n)), but we test with map which preserves hints
        let result = collect_small((0..10).map(|x| x * 2));
        assert_eq!(result.len(), 10);
    }

    #[rstest]
    fn test_collect_small_with_filter_uses_vec() {
        // Filter produces (0, Some(n)), so lower == 0 means Vec is used
        let result = collect_small((0..100).filter(|x| x % 2 == 0));
        assert_eq!(result.len(), 50);
    }

    #[rstest]
    fn test_collect_small_large_element_avoids_smallvec() {
        // Large elements (4KB) should use Vec to avoid stack overflow
        #[derive(Clone)]
        #[repr(C)]
        struct LargeStruct4KB {
            data: [u8; 4096],
        }

        // Verify that 4KB * 128 = 512KB exceeds L1 cache (32KB)
        let inline_buffer_size = std::mem::size_of::<LargeStruct4KB>() * SMALLVEC_INLINE_CAPACITY;
        assert!(
            inline_buffer_size > L1_CACHE_SIZE,
            "4KB * 128 = {} should exceed L1_CACHE_SIZE ({})",
            inline_buffer_size,
            L1_CACHE_SIZE
        );

        // This should use Vec directly due to stack safety check
        let result: Vec<LargeStruct4KB> = collect_small((0..10).map(|i| LargeStruct4KB {
            data: [i as u8; 4096],
        }));
        assert_eq!(result.len(), 10);
    }

    #[rstest]
    fn test_collect_small_small_element_may_use_smallvec() {
        // Small elements (i32) with unknown upper may use SmallVec
        let inline_buffer_size = std::mem::size_of::<i32>() * SMALLVEC_INLINE_CAPACITY;
        assert!(
            inline_buffer_size <= L1_CACHE_SIZE,
            "i32 * 128 = {} should fit in L1_CACHE_SIZE ({})",
            inline_buffer_size,
            L1_CACHE_SIZE
        );

        // This may use SmallVec path (internal detail, but result should be correct)
        let result = collect_small((0..50).map(|x| x * 2));
        assert_eq!(result.len(), 50);
    }

    #[rstest]
    fn test_collect_with_capacity_respects_capacity() {
        let result = collect_with_capacity(100, 0..100);
        assert_eq!(result.len(), 100);
        assert!(
            result.capacity() >= 100,
            "capacity ({}) should be >= 100",
            result.capacity()
        );
    }

    #[rstest]
    fn test_collect_with_capacity_capped_at_max() {
        // Very large capacity should be capped at MAX_REASONABLE_CAPACITY
        let result: Vec<i32> = collect_with_capacity(10_000_000, std::iter::empty());
        assert!(
            result.capacity() <= 1024 * 1024,
            "capacity ({}) should be capped at 1MB",
            result.capacity()
        );
    }

    #[rstest]
    fn test_compute_threshold_for_various_sizes() {
        // i32: 4 bytes -> 32KB / 4 = 8192, capped at 128
        let threshold_i32 = compute_smallvec_threshold::<i32>();
        assert_eq!(threshold_i32, SMALLVEC_INLINE_CAPACITY);

        // 256-byte struct -> 32KB / 256 = 128
        #[repr(C)]
        struct Medium256 {
            _data: [u8; 256],
        }
        let threshold_medium = compute_smallvec_threshold::<Medium256>();
        assert_eq!(threshold_medium, 128);

        // 1KB struct -> 32KB / 1024 = 32
        #[repr(C)]
        struct Large1KB {
            _data: [u8; 1024],
        }
        let threshold_large = compute_smallvec_threshold::<Large1KB>();
        assert_eq!(threshold_large, 32);

        // 4KB struct -> 32KB / 4096 = 8
        #[repr(C)]
        struct VeryLarge4KB {
            _data: [u8; 4096],
        }
        let threshold_very_large = compute_smallvec_threshold::<VeryLarge4KB>();
        assert_eq!(threshold_very_large, 8);
    }

    #[rstest]
    fn test_stack_safety_check_boundary() {
        // Test that 256-byte elements are stack-safe (256 * 128 = 32KB = L1_CACHE_SIZE)
        #[repr(C)]
        struct Boundary256 {
            _data: [u8; 256],
        }
        let inline_size = std::mem::size_of::<Boundary256>() * SMALLVEC_INLINE_CAPACITY;
        assert_eq!(inline_size, L1_CACHE_SIZE);

        // 257-byte elements should not be stack-safe
        #[repr(C)]
        struct JustOver257 {
            _data: [u8; 257],
        }
        let inline_size_over = std::mem::size_of::<JustOver257>() * SMALLVEC_INLINE_CAPACITY;
        assert!(inline_size_over > L1_CACHE_SIZE);
    }

    #[rstest]
    fn test_collect_small_referential_transparency() {
        // Same input always produces same output
        let result1 = collect_small(0..25);
        let result2 = collect_small(0..25);
        let result3 = collect_small(0..25);
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[rstest]
    fn test_collect_with_capacity_referential_transparency() {
        // Same input always produces same output
        let result1 = collect_with_capacity(50, 0..50);
        let result2 = collect_with_capacity(50, 0..50);
        assert_eq!(result1, result2);
    }

    #[rstest]
    fn test_collect_with_hint_large_element_with_small_lower_uses_vec() {
        // Critical test: upper=None, lower <= threshold, but large element type
        // This should use Vec path due to stack safety check, not SmallVec
        use lambars::compose::for_macro::collect_with_hint;

        #[derive(Clone)]
        #[repr(C)]
        struct LargeStruct4KB {
            data: [u8; 4096],
        }

        // Verify threshold: 32KB / 4KB = 8
        let threshold = compute_smallvec_threshold::<LargeStruct4KB>();
        assert_eq!(threshold, 8);

        // Verify stack unsafe: 4KB * 128 = 512KB > 32KB
        let inline_buffer_size = std::mem::size_of::<LargeStruct4KB>() * SMALLVEC_INLINE_CAPACITY;
        assert!(
            inline_buffer_size > L1_CACHE_SIZE,
            "4KB * 128 = {} should exceed L1_CACHE_SIZE ({})",
            inline_buffer_size,
            L1_CACHE_SIZE
        );

        // Test case: lower=1 (below threshold of 8), upper=None
        // Without stack safety check, this would use SmallVec (512KB on stack!)
        // With stack safety check, this uses Vec
        let result: Vec<LargeStruct4KB> = collect_with_hint(
            1,    // lower = 1 (below threshold of 8)
            None, // upper = None (unknown)
            std::iter::once(LargeStruct4KB { data: [42; 4096] }),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].data[0], 42);
    }

    #[rstest]
    fn test_collect_from_iter_large_element_uses_vec() {
        // Test that collect_from_iter also respects stack safety
        use lambars::compose::for_macro::collect_from_iter;

        #[derive(Clone)]
        #[repr(C)]
        struct LargeStruct4KB {
            data: [u8; 4096],
        }

        let items: Vec<LargeStruct4KB> = (0..5)
            .map(|i| LargeStruct4KB {
                data: [i as u8; 4096],
            })
            .collect();

        // map preserves size_hint, so (5, Some(5)) -> uses Vec directly
        let result: Vec<LargeStruct4KB> = collect_from_iter(items.into_iter().map(|mut x| {
            x.data[0] += 100;
            x
        }));
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].data[0], 100);
        assert_eq!(result[4].data[0], 104);
    }
}
