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
