//! Integration tests for the `for_async!` macro.
//!
//! These tests verify that `for_async!` works correctly with other
//! lambars components and in real-world scenarios.

#![cfg(feature = "async")]

use lambars::eff_async;
use lambars::effect::AsyncIO;
use lambars::for_async;
use lambars::persistent::PersistentVector;

// =============================================================================
// Integration with eff_async!
// =============================================================================

#[tokio::test]
async fn test_with_eff_async() {
    let result = eff_async! {
        data_list <= for_async! {
            x <= vec![1, 2, 3];
            data <~ AsyncIO::pure(x * 10);
            yield data
        };
        let sum: i32 = data_list.iter().sum();
        AsyncIO::pure(sum)
    };
    assert_eq!(result.run_async().await, 60);
}

#[tokio::test]
async fn test_for_async_in_eff_async_chain() {
    let result = eff_async! {
        list1 <= for_async! {
            x <= vec![1, 2];
            yield x
        };
        list2 <= for_async! {
            x <= list1;
            yield x * 10
        };
        AsyncIO::pure(list2)
    };
    assert_eq!(result.run_async().await, vec![10, 20]);
}

#[tokio::test]
async fn test_nested_for_async_in_eff_async() {
    let result = eff_async! {
        outer <= for_async! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            yield x + y
        };
        let total: i32 = outer.iter().sum();
        AsyncIO::pure(total)
    };
    // 11 + 21 + 12 + 22 = 66
    assert_eq!(result.run_async().await, 66);
}

// =============================================================================
// Integration with PersistentVector
// =============================================================================

#[tokio::test]
async fn test_with_persistent_vector() {
    let vector = PersistentVector::from_iter([1, 2, 3]);
    let items: Vec<_> = vector.iter().cloned().collect();
    let result = for_async! {
        x <= items;
        yield x * 2
    };
    assert_eq!(result.run_async().await, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_persistent_vector_iteration() {
    let vector = PersistentVector::from_iter([1, 2, 3]);
    // Convert to Vec for iteration in for_async!
    let collected: Vec<_> = vector.into_iter().collect();
    let result = for_async! {
        x <= collected;
        doubled <~ AsyncIO::pure(x * 2);
        yield doubled
    };
    assert_eq!(result.run_async().await, vec![2, 4, 6]);
}

// =============================================================================
// Chained for_async! calls
// =============================================================================

#[tokio::test]
async fn test_chained_for_async() {
    let result = eff_async! {
        list1 <= for_async! {
            x <= vec![1, 2];
            yield x
        };
        list2 <= for_async! {
            x <= list1;
            yield x * 10
        };
        AsyncIO::pure(list2)
    };
    assert_eq!(result.run_async().await, vec![10, 20]);
}

#[tokio::test]
async fn test_multiple_for_async_independent() {
    let result = eff_async! {
        list1 <= for_async! {
            x <= vec![1, 2, 3];
            yield x
        };
        list2 <= for_async! {
            x <= vec![10, 20, 30];
            yield x
        };
        let combined: Vec<_> = list1.into_iter().chain(list2.into_iter()).collect();
        AsyncIO::pure(combined)
    };
    assert_eq!(result.run_async().await, vec![1, 2, 3, 10, 20, 30]);
}

// =============================================================================
// Real-world Scenarios
// =============================================================================

#[tokio::test]
async fn test_data_processing_pipeline() {
    // Simulate fetching and processing data
    fn fetch_data(id: i32) -> AsyncIO<String> {
        AsyncIO::pure(format!("data_{}", id))
    }

    fn process_data(data: String) -> AsyncIO<String> {
        AsyncIO::pure(data.to_uppercase())
    }

    let result = for_async! {
        id <= vec![1, 2, 3];
        raw <~ fetch_data(id);
        processed <~ process_data(raw);
        yield processed
    };

    assert_eq!(result.run_async().await, vec!["DATA_1", "DATA_2", "DATA_3"]);
}

#[tokio::test]
async fn test_conditional_async_processing() {
    fn process_if_positive(x: i32) -> AsyncIO<Option<i32>> {
        AsyncIO::pure(if x > 0 { Some(x * 2) } else { None })
    }

    let result = for_async! {
        x <= vec![-1, 2, -3, 4, 5];
        maybe_processed <~ process_if_positive(x);
        processed <= maybe_processed.into_iter();
        yield processed
    };

    assert_eq!(result.run_async().await, vec![4, 8, 10]);
}

#[tokio::test]
async fn test_complex_data_transformation() {
    #[derive(Clone)]
    struct User {
        id: i32,
        name: String,
    }

    fn fetch_user(id: i32) -> AsyncIO<User> {
        AsyncIO::pure(User {
            id,
            name: format!("User{}", id),
        })
    }

    fn fetch_posts(user_id: i32) -> AsyncIO<Vec<String>> {
        AsyncIO::pure(vec![
            format!("Post1 by {}", user_id),
            format!("Post2 by {}", user_id),
        ])
    }

    let result = for_async! {
        user_id <= vec![1, 2];
        user <~ fetch_user(user_id);
        posts <~ fetch_posts(user.id);
        post <= posts;
        yield format!("{}: {}", user.name, post)
    };

    assert_eq!(
        result.run_async().await,
        vec![
            "User1: Post1 by 1",
            "User1: Post2 by 1",
            "User2: Post1 by 2",
            "User2: Post2 by 2"
        ]
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_single_element_collection() {
    let result = for_async! {
        x <= vec![42];
        yield x
    };
    assert_eq!(result.run_async().await, vec![42]);
}

#[tokio::test]
async fn test_deeply_nested_iteration() {
    let result = for_async! {
        a <= vec![1, 2];
        b <= vec![10, 20];
        c <= vec![100, 200];
        d <= vec![1000];
        yield a + b + c + d
    };
    assert_eq!(
        result.run_async().await,
        vec![1111, 1211, 1121, 1221, 1112, 1212, 1122, 1222]
    );
}

#[tokio::test]
async fn test_fmap_after_for_async() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield x * 2
    }
    .fmap(|vec| {
        vec.into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    });

    assert_eq!(result.run_async().await, "2, 4, 6");
}

#[tokio::test]
async fn test_and_then_after_for_async() {
    let result = for_async! {
        x <= vec![1, 2, 3];
        yield x * 2
    }
    .and_then(|vec| {
        for_async! {
            x <= vec;
            yield x + 1
        }
    });

    assert_eq!(result.run_async().await, vec![3, 5, 7]);
}
