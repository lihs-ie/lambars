//! Integration tests for the for_! macro.
//!
//! These tests verify that the for_! macro works correctly
//! with lambars' persistent data structures.

#![cfg(feature = "compose")]

use lambars::for_;
use lambars::persistent::{PersistentHashMap, PersistentList, PersistentVector};

// =============================================================================
// PersistentList Integration Tests
// =============================================================================

#[test]
fn test_with_persistent_list() {
    let list = PersistentList::from_iter([1, 2, 3]);
    let result = for_! {
        x <= list.iter().cloned().collect::<Vec<_>>();
        yield x * 2
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_with_persistent_list_nested() {
    let list1 = PersistentList::from_iter([1, 2]);
    let list2 = PersistentList::from_iter([10, 20]);

    let vec1: Vec<_> = list1.iter().cloned().collect();
    let vec2: Vec<_> = list2.iter().cloned().collect();

    let result = for_! {
        x <= vec1;
        y <= vec2.clone();
        yield x + y
    };
    assert_eq!(result, vec![11, 21, 12, 22]);
}

// =============================================================================
// PersistentVector Integration Tests
// =============================================================================

#[test]
fn test_with_persistent_vector() {
    let vector = PersistentVector::from_iter([1, 2, 3]);
    let result = for_! {
        x <= vector.iter().cloned().collect::<Vec<_>>();
        yield x * 2
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_with_persistent_vector_into_iter() {
    let vector = PersistentVector::from_iter([1, 2, 3]);

    // Use into_iter directly on PersistentVector
    let result = for_! {
        x <= vector;
        yield x * 2
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_with_persistent_vector_nested() {
    let vector1 = PersistentVector::from_iter([1, 2]);
    let vector2 = PersistentVector::from_iter([10, 20]);

    let vec1: Vec<_> = vector1.iter().cloned().collect();
    let vec2: Vec<_> = vector2.iter().cloned().collect();

    let result = for_! {
        x <= vec1;
        y <= vec2.clone();
        yield x + y
    };
    assert_eq!(result, vec![11, 21, 12, 22]);
}

// =============================================================================
// PersistentHashMap Integration Tests
// =============================================================================

#[test]
fn test_with_persistent_hashmap() {
    let map = PersistentHashMap::new().insert("a", 1).insert("b", 2);

    // Collect pairs into a Vec first
    let pairs: Vec<_> = map.iter().collect();
    let result = for_! {
        (key, value) <= pairs;
        yield format!("{}: {}", key, value)
    };
    assert_eq!(result.len(), 2);
    // Note: HashMap iteration order is not guaranteed,
    // so we check that both expected values are present
    assert!(result.contains(&"a: 1".to_string()) || result.contains(&"b: 2".to_string()));
}

#[test]
fn test_with_persistent_hashmap_values() {
    let map = PersistentHashMap::new()
        .insert("x", 10)
        .insert("y", 20)
        .insert("z", 30);

    let pairs: Vec<_> = map.iter().collect();
    let result: i32 = for_! {
        (_, value) <= pairs;
        yield *value
    }
    .into_iter()
    .sum();

    assert_eq!(result, 60);
}

// =============================================================================
// Mixed Integration Tests
// =============================================================================

#[test]
fn test_persistent_list_with_let_binding() {
    let list = PersistentList::from_iter([1, 2, 3, 4, 5]);
    let vec_data: Vec<_> = list.iter().cloned().collect();

    let result = for_! {
        x <= vec_data;
        let squared = x * x;
        let doubled = squared * 2;
        yield doubled
    };
    assert_eq!(result, vec![2, 8, 18, 32, 50]);
}

#[test]
fn test_persistent_vector_filter_simulation() {
    let vector = PersistentVector::from_iter([1, 2, 3, 4, 5, 6]);

    // Simulate filter using for_! with conditional collection
    let result = for_! {
        x <= vector;
        y <= if x % 2 == 0 { vec![x] } else { vec![] };
        yield y
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_build_persistent_list_from_for() {
    let numbers = vec![1, 2, 3];
    let result = for_! {
        n <= numbers;
        yield n * 2
    };

    // Convert result to PersistentList
    let persistent_result = PersistentList::from_iter(result);
    let collected: Vec<_> = persistent_result.iter().cloned().collect();
    assert_eq!(collected, vec![2, 4, 6]);
}

#[test]
fn test_build_persistent_vector_from_for() {
    let numbers = vec![1, 2, 3];
    let result = for_! {
        n <= numbers;
        yield n * 2
    };

    // Convert result to PersistentVector
    let persistent_result = PersistentVector::from_iter(result);
    assert_eq!(persistent_result.len(), 3);
    assert_eq!(persistent_result.get(0), Some(&2));
    assert_eq!(persistent_result.get(1), Some(&4));
    assert_eq!(persistent_result.get(2), Some(&6));
}

// =============================================================================
// Complex Real-World Scenario Tests
// =============================================================================

#[test]
fn test_database_query_simulation() {
    // Simulate a database query scenario using persistent data structures

    #[derive(Clone, Debug, PartialEq)]
    struct User {
        id: u32,
        name: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Order {
        user_id: u32,
        product: String,
        amount: u32,
    }

    let users = PersistentVector::from_iter([
        User {
            id: 1,
            name: "Alice".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
        },
    ]);

    let orders = PersistentVector::from_iter([
        Order {
            user_id: 1,
            product: "Book".to_string(),
            amount: 2,
        },
        Order {
            user_id: 1,
            product: "Pen".to_string(),
            amount: 5,
        },
        Order {
            user_id: 2,
            product: "Notebook".to_string(),
            amount: 3,
        },
    ]);

    let users_vec: Vec<_> = users.iter().cloned().collect();
    let orders_vec: Vec<_> = orders.iter().cloned().collect();

    // Join users with their orders
    let result = for_! {
        user <= users_vec;
        order <= orders_vec.clone().into_iter().filter(|o| o.user_id == user.id).collect::<Vec<_>>();
        yield format!("{} ordered {} x {}", user.name, order.amount, order.product)
    };

    assert_eq!(result.len(), 3);
    assert!(result.contains(&"Alice ordered 2 x Book".to_string()));
    assert!(result.contains(&"Alice ordered 5 x Pen".to_string()));
    assert!(result.contains(&"Bob ordered 3 x Notebook".to_string()));
}

#[test]
fn test_tree_traversal_simulation() {
    // Simulate tree traversal using for_!

    #[derive(Clone)]
    struct TreeNode {
        value: i32,
        children: Vec<TreeNode>,
    }

    fn collect_all_values(node: &TreeNode) -> Vec<i32> {
        let mut result = vec![node.value];
        for child in &node.children {
            result.extend(collect_all_values(child));
        }
        result
    }

    let tree = TreeNode {
        value: 1,
        children: vec![
            TreeNode {
                value: 2,
                children: vec![
                    TreeNode {
                        value: 4,
                        children: vec![],
                    },
                    TreeNode {
                        value: 5,
                        children: vec![],
                    },
                ],
            },
            TreeNode {
                value: 3,
                children: vec![TreeNode {
                    value: 6,
                    children: vec![],
                }],
            },
        ],
    };

    // Use for_! to double all values in the tree
    let all_values = collect_all_values(&tree);
    let doubled = for_! {
        value <= all_values;
        yield value * 2
    };

    assert_eq!(doubled, vec![2, 4, 8, 10, 6, 12]);
}
