//! Unit tests for Traversal optics.
//!
//! Tests cover:
//! - Basic Traversal operations (get_all, modify_all, set_all, fold, length, for_all, exists, head_option)
//! - VecTraversal for Vec<A>
//! - OptionTraversal for Option<A>
//! - ResultTraversal for Result<A, E>
//! - ComposedTraversal for nested structures
//! - LensAsTraversal and PrismAsTraversal integration

#![forbid(unsafe_code)]

use lambars::lens;
use lambars::optics::{Lens, Prism, Traversal};
use lambars::prism;
use rstest::rstest;

// =============================================================================
// Test Data Structures
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
struct Container {
    items: Vec<i32>,
}

#[derive(Clone, PartialEq, Debug)]
enum MyOption<T> {
    Some(T),
    None,
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug)]
enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

// =============================================================================
// VecTraversal Tests
// =============================================================================

mod vec_traversal_tests {
    use super::*;
    use lambars::optics::VecTraversal;

    #[test]
    fn test_vec_traversal_get_all_non_empty() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3, 4, 5];

        let collected: Vec<&i32> = traversal.get_all(&numbers).collect();
        assert_eq!(collected, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn test_vec_traversal_get_all_empty() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let empty: Vec<i32> = vec![];

        let collected: Vec<&i32> = traversal.get_all(&empty).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_vec_traversal_get_all_owned() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3];

        let owned = traversal.get_all_owned(numbers);
        assert_eq!(owned, vec![1, 2, 3]);
    }

    #[test]
    fn test_vec_traversal_modify_all() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3];

        let doubled = traversal.modify_all(numbers, |x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[test]
    fn test_vec_traversal_modify_all_empty() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let empty: Vec<i32> = vec![];

        let result = traversal.modify_all(empty, |x| x * 2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_vec_traversal_set_all() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3, 4, 5];

        let all_zeros = traversal.set_all(numbers, 0);
        assert_eq!(all_zeros, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_vec_traversal_fold() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let numbers = vec![1, 2, 3, 4, 5];

        let sum = traversal.fold(&numbers, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_vec_traversal_fold_with_string() {
        let traversal: VecTraversal<String> = VecTraversal::new();
        let words = vec!["hello".to_string(), " ".to_string(), "world".to_string()];

        let concatenated = traversal.fold(&words, String::new(), |accumulator, element| {
            accumulator + element
        });
        assert_eq!(concatenated, "hello world");
    }

    #[test]
    fn test_vec_traversal_length() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        assert_eq!(traversal.length(&vec![1, 2, 3, 4, 5]), 5);
        assert_eq!(traversal.length(&vec![1]), 1);
        assert_eq!(traversal.length(&Vec::<i32>::new()), 0);
    }

    #[test]
    fn test_vec_traversal_for_all() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        let positive = vec![1, 2, 3, 4, 5];
        assert!(traversal.for_all(&positive, |x| *x > 0));

        let mixed = vec![1, -2, 3];
        assert!(!traversal.for_all(&mixed, |x| *x > 0));

        let empty: Vec<i32> = vec![];
        assert!(traversal.for_all(&empty, |x| *x > 0)); // vacuously true
    }

    #[test]
    fn test_vec_traversal_exists() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        let numbers = vec![1, 2, 3, 4, 5];
        assert!(traversal.exists(&numbers, |x| *x == 3));
        assert!(!traversal.exists(&numbers, |x| *x == 10));

        let empty: Vec<i32> = vec![];
        assert!(!traversal.exists(&empty, |x| *x == 1));
    }

    #[test]
    fn test_vec_traversal_head_option() {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        let numbers = vec![1, 2, 3];
        assert_eq!(traversal.head_option(&numbers), Some(&1));

        let empty: Vec<i32> = vec![];
        assert_eq!(traversal.head_option(&empty), None);
    }

    #[rstest]
    #[case(vec![1, 2, 3], 6)]
    #[case(vec![10], 10)]
    #[case(vec![], 0)]
    fn test_vec_traversal_get_all_sum(#[case] input: Vec<i32>, #[case] expected_sum: i32) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let sum: i32 = traversal.get_all(&input).sum();
        assert_eq!(sum, expected_sum);
    }
}

// =============================================================================
// OptionTraversal Tests
// =============================================================================

mod option_traversal_tests {
    use super::*;
    use lambars::optics::OptionTraversal;

    #[test]
    fn test_option_traversal_get_all_some() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let some_value = Some(42);

        let collected: Vec<&i32> = traversal.get_all(&some_value).collect();
        assert_eq!(collected, vec![&42]);
    }

    #[test]
    fn test_option_traversal_get_all_none() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let none_value: Option<i32> = None;

        let collected: Vec<&i32> = traversal.get_all(&none_value).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_option_traversal_get_all_owned() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        assert_eq!(traversal.get_all_owned(Some(42)), vec![42]);
        assert_eq!(traversal.get_all_owned(None::<i32>), Vec::<i32>::new());
    }

    #[test]
    fn test_option_traversal_modify_all_some() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let some_value = Some(10);

        let doubled = traversal.modify_all(some_value, |x| x * 2);
        assert_eq!(doubled, Some(20));
    }

    #[test]
    fn test_option_traversal_modify_all_none() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let none_value: Option<i32> = None;

        let result = traversal.modify_all(none_value, |x| x * 2);
        assert_eq!(result, None);
    }

    #[test]
    fn test_option_traversal_set_all() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        let some_value = Some(42);
        assert_eq!(traversal.set_all(some_value, 100), Some(100));

        let none_value: Option<i32> = None;
        assert_eq!(traversal.set_all(none_value, 100), None);
    }

    #[test]
    fn test_option_traversal_length() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        assert_eq!(traversal.length(&Some(42)), 1);
        assert_eq!(traversal.length(&None::<i32>), 0);
    }

    #[test]
    fn test_option_traversal_for_all() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        assert!(traversal.for_all(&Some(10), |x| *x > 5));
        assert!(!traversal.for_all(&Some(3), |x| *x > 5));
        assert!(traversal.for_all(&None::<i32>, |x| *x > 5)); // vacuously true
    }

    #[test]
    fn test_option_traversal_exists() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        assert!(traversal.exists(&Some(10), |x| *x > 5));
        assert!(!traversal.exists(&Some(3), |x| *x > 5));
        assert!(!traversal.exists(&None::<i32>, |x| *x > 5));
    }

    #[test]
    fn test_option_traversal_head_option() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();

        assert_eq!(traversal.head_option(&Some(42)), Some(&42));
        assert_eq!(traversal.head_option(&None::<i32>), None);
    }
}

// =============================================================================
// ResultTraversal Tests
// =============================================================================

mod result_traversal_tests {
    use super::*;
    use lambars::optics::ResultTraversal;

    #[test]
    fn test_result_traversal_get_all_ok() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let ok_value: Result<i32, String> = Ok(42);

        let collected: Vec<&i32> = traversal.get_all(&ok_value).collect();
        assert_eq!(collected, vec![&42]);
    }

    #[test]
    fn test_result_traversal_get_all_err() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let err_value: Result<i32, String> = Err("error".to_string());

        let collected: Vec<&i32> = traversal.get_all(&err_value).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_result_traversal_get_all_owned() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();

        assert_eq!(traversal.get_all_owned(Ok(42)), vec![42]);
        assert_eq!(
            traversal.get_all_owned(Err::<i32, String>("error".to_string())),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn test_result_traversal_modify_all_ok() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let ok_value: Result<i32, String> = Ok(10);

        let doubled = traversal.modify_all(ok_value, |x| x * 2);
        assert_eq!(doubled, Ok(20));
    }

    #[test]
    fn test_result_traversal_modify_all_err() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let err_value: Result<i32, String> = Err("error".to_string());

        let result = traversal.modify_all(err_value, |x| x * 2);
        assert_eq!(result, Err("error".to_string()));
    }

    #[test]
    fn test_result_traversal_set_all() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();

        let ok_value: Result<i32, String> = Ok(42);
        assert_eq!(traversal.set_all(ok_value, 100), Ok(100));

        let err_value: Result<i32, String> = Err("error".to_string());
        assert_eq!(traversal.set_all(err_value, 100), Err("error".to_string()));
    }

    #[test]
    fn test_result_traversal_length() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();

        assert_eq!(traversal.length(&Ok::<i32, String>(42)), 1);
        assert_eq!(
            traversal.length(&Err::<i32, String>("error".to_string())),
            0
        );
    }

    #[test]
    fn test_result_traversal_for_all_and_exists() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();

        let ok_value: Result<i32, String> = Ok(10);
        assert!(traversal.for_all(&ok_value, |x| *x > 5));
        assert!(traversal.exists(&ok_value, |x| *x > 5));

        let err_value: Result<i32, String> = Err("error".to_string());
        assert!(traversal.for_all(&err_value, |x| *x > 5)); // vacuously true
        assert!(!traversal.exists(&err_value, |x| *x > 5));
    }
}

// =============================================================================
// ComposedTraversal Tests
// =============================================================================

mod composed_traversal_tests {
    use super::*;
    use lambars::optics::VecTraversal;

    #[test]
    fn test_composed_traversal_nested_vec() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];

        // get_all returns nested iterators - need to handle accordingly
        let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_composed_traversal_nested_vec_modify() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let doubled = composed.modify_all(data, |x| x * 2);

        assert_eq!(doubled, vec![vec![2, 4], vec![6, 8, 10]]);
    }

    #[test]
    fn test_composed_traversal_nested_vec_empty() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let empty: Vec<Vec<i32>> = vec![];
        assert_eq!(composed.length(&empty), 0);

        let partial_empty = vec![vec![], vec![1, 2], vec![]];
        assert_eq!(composed.length(&partial_empty), 2);
    }

    #[test]
    fn test_composed_traversal_vec_option() {
        use lambars::optics::OptionTraversal;

        let vec_traversal: VecTraversal<Option<i32>> = VecTraversal::new();
        let option_traversal: OptionTraversal<i32> = OptionTraversal::new();
        let composed = vec_traversal.compose(option_traversal);

        let data = vec![Some(1), None, Some(3), None, Some(5)];

        let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 9);
    }

    #[test]
    fn test_composed_traversal_three_levels() {
        let first: VecTraversal<Vec<Vec<i32>>> = VecTraversal::new();
        let second: VecTraversal<Vec<i32>> = VecTraversal::new();
        let third: VecTraversal<i32> = VecTraversal::new();

        let composed = first.compose(second).compose(third);

        let data = vec![vec![vec![1, 2], vec![3]], vec![vec![4, 5, 6]]];

        let sum: i32 = composed.fold(&data, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 21);
    }
}

// =============================================================================
// LensAsTraversal Integration Tests
// =============================================================================

mod lens_as_traversal_tests {
    use super::*;
    use lambars::optics::VecTraversal;

    #[test]
    fn test_lens_to_traversal_basic() {
        let items_lens = lens!(Container, items);
        let lens_traversal = items_lens.to_traversal();

        let container = Container {
            items: vec![1, 2, 3],
        };

        let collected: Vec<&Vec<i32>> = lens_traversal.get_all(&container).collect();
        assert_eq!(collected, vec![&vec![1, 2, 3]]);
    }

    #[test]
    fn test_lens_to_traversal_modify() {
        let items_lens = lens!(Container, items);
        let lens_traversal = items_lens.to_traversal();

        let container = Container {
            items: vec![1, 2, 3],
        };

        let modified = lens_traversal.modify_all(container, |items| {
            items.into_iter().map(|x| x * 2).collect()
        });

        assert_eq!(modified.items, vec![2, 4, 6]);
    }

    #[test]
    fn test_lens_to_traversal_compose_with_vec_traversal() {
        let items_lens = lens!(Container, items);
        let vec_traversal: VecTraversal<i32> = VecTraversal::new();
        let composed = items_lens.to_traversal().compose(vec_traversal);

        let container = Container {
            items: vec![1, 2, 3],
        };

        let sum: i32 = composed.fold(&container, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_lens_to_traversal_compose_modify() {
        let items_lens = lens!(Container, items);
        let vec_traversal: VecTraversal<i32> = VecTraversal::new();
        let composed = items_lens.to_traversal().compose(vec_traversal);

        let container = Container {
            items: vec![1, 2, 3],
        };

        let modified = composed.modify_all(container, |x| x * 10);
        assert_eq!(modified.items, vec![10, 20, 30]);
    }
}

// =============================================================================
// PrismAsTraversal Integration Tests
// =============================================================================

mod prism_as_traversal_tests {
    use super::*;

    #[test]
    fn test_prism_to_traversal_matching_variant() {
        let some_prism = prism!(MyOption<i32>, Some);
        let prism_traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(42);
        let collected: Vec<&i32> = prism_traversal.get_all(&some_value).collect();
        assert_eq!(collected, vec![&42]);
    }

    #[test]
    fn test_prism_to_traversal_non_matching_variant() {
        let some_prism = prism!(MyOption<i32>, Some);
        let prism_traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        let collected: Vec<&i32> = prism_traversal.get_all(&none_value).collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_prism_to_traversal_modify_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let prism_traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(10);
        let doubled = prism_traversal.modify_all(some_value, |x| x * 2);
        assert_eq!(doubled, MyOption::Some(20));
    }

    #[test]
    fn test_prism_to_traversal_modify_non_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let prism_traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        let result = prism_traversal.modify_all(none_value, |x| x * 2);
        assert_eq!(result, MyOption::None);
    }
}

// =============================================================================
// Traversal Trait Implementors Test
// =============================================================================

mod traversal_trait_tests {
    #[allow(unused_imports)]
    use super::*;
    use lambars::optics::{OptionTraversal, ResultTraversal, Traversal, VecTraversal};

    fn assert_traversal<S, A, T: Traversal<S, A>>(_traversal: &T) {}

    #[test]
    fn test_vec_traversal_implements_traversal_trait() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        assert_traversal::<Vec<i32>, i32, _>(&traversal);
    }

    #[test]
    fn test_option_traversal_implements_traversal_trait() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        assert_traversal::<Option<i32>, i32, _>(&traversal);
    }

    #[test]
    fn test_result_traversal_implements_traversal_trait() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        assert_traversal::<Result<i32, String>, i32, _>(&traversal);
    }
}

// =============================================================================
// Debug, Clone, Default Trait Tests
// =============================================================================

mod trait_implementation_tests {
    use lambars::optics::{OptionTraversal, ResultTraversal, Traversal, VecTraversal};

    #[test]
    fn test_vec_traversal_debug() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let debug_str = format!("{:?}", traversal);
        assert!(debug_str.contains("VecTraversal"));
    }

    #[test]
    fn test_vec_traversal_clone() {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let cloned = traversal.clone();
        let numbers = vec![1, 2, 3];
        assert_eq!(cloned.length(&numbers), 3);
    }

    #[test]
    fn test_vec_traversal_default() {
        let traversal: VecTraversal<i32> = VecTraversal::default();
        let numbers = vec![1, 2, 3];
        assert_eq!(traversal.length(&numbers), 3);
    }

    #[test]
    fn test_option_traversal_debug() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let debug_str = format!("{:?}", traversal);
        assert!(debug_str.contains("OptionTraversal"));
    }

    #[test]
    fn test_option_traversal_clone() {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let cloned = traversal.clone();
        assert_eq!(cloned.length(&Some(42)), 1);
    }

    #[test]
    fn test_option_traversal_default() {
        let traversal: OptionTraversal<i32> = OptionTraversal::default();
        assert_eq!(traversal.length(&Some(42)), 1);
    }

    #[test]
    fn test_result_traversal_debug() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let debug_str = format!("{:?}", traversal);
        assert!(debug_str.contains("ResultTraversal"));
    }

    #[test]
    fn test_result_traversal_clone() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let cloned = traversal.clone();
        let ok_value: Result<i32, String> = Ok(42);
        assert_eq!(cloned.length(&ok_value), 1);
    }

    #[test]
    fn test_result_traversal_default() {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::default();
        let ok_value: Result<i32, String> = Ok(42);
        assert_eq!(traversal.length(&ok_value), 1);
    }
}

// =============================================================================
// ComposedTraversal Additional Tests
// =============================================================================

mod composed_traversal_advanced_tests {
    use lambars::optics::{Traversal, VecTraversal};

    #[test]
    fn test_composed_traversal_get_all_owned() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let owned = composed.get_all_owned(data);
        assert_eq!(owned, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_composed_traversal_debug() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let debug_str = format!("{:?}", composed);
        assert!(debug_str.contains("ComposedTraversal"));
    }

    #[test]
    fn test_composed_traversal_clone() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);
        let cloned = composed.clone();

        let data = vec![vec![1, 2], vec![3]];
        assert_eq!(cloned.length(&data), 3);
    }

    #[test]
    fn test_composed_traversal_set_all() {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let data = vec![vec![1, 2], vec![3, 4, 5]];
        let all_zeros = composed.set_all(data, 0);
        assert_eq!(all_zeros, vec![vec![0, 0], vec![0, 0, 0]]);
    }
}

// =============================================================================
// LensAsTraversal Additional Tests
// =============================================================================

mod lens_as_traversal_advanced_tests {
    use super::*;
    use lambars::optics::Traversal;

    #[test]
    fn test_lens_as_traversal_get_all_owned() {
        let items_lens = lens!(Container, items);
        let traversal = items_lens.to_traversal();

        let container = Container {
            items: vec![1, 2, 3],
        };

        let owned = traversal.get_all_owned(container);
        assert_eq!(owned, vec![vec![1, 2, 3]]);
    }

    #[test]
    fn test_lens_as_traversal_length() {
        let items_lens = lens!(Container, items);
        let traversal = items_lens.to_traversal();

        let container = Container {
            items: vec![1, 2, 3],
        };

        // A lens always focuses on exactly one element
        assert_eq!(traversal.length(&container), 1);
    }

    #[test]
    fn test_lens_as_traversal_debug() {
        let items_lens = lens!(Container, items);
        let traversal = items_lens.to_traversal();

        let debug_str = format!("{:?}", traversal);
        assert!(debug_str.contains("LensAsTraversal"));
    }

    #[test]
    fn test_lens_as_traversal_clone() {
        let items_lens = lens!(Container, items);
        let traversal = items_lens.to_traversal();
        let cloned = traversal.clone();

        let container = Container {
            items: vec![1, 2, 3],
        };
        assert_eq!(cloned.length(&container), 1);
    }
}

// =============================================================================
// PrismAsTraversal Additional Tests
// =============================================================================

mod prism_as_traversal_advanced_tests {
    use super::*;
    use lambars::optics::Traversal;

    #[test]
    fn test_prism_as_traversal_get_all_owned_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(42);
        let owned = traversal.get_all_owned(some_value);
        assert_eq!(owned, vec![42]);
    }

    #[test]
    fn test_prism_as_traversal_get_all_owned_non_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        let owned = traversal.get_all_owned(none_value);
        assert!(owned.is_empty());
    }

    #[test]
    fn test_prism_as_traversal_debug() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let debug_str = format!("{:?}", traversal);
        assert!(debug_str.contains("PrismAsTraversal"));
    }

    #[test]
    fn test_prism_as_traversal_clone() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();
        let cloned = traversal.clone();

        let some_value = MyOption::Some(42);
        let collected: Vec<&i32> = cloned.get_all(&some_value).collect();
        assert_eq!(collected, vec![&42]);
    }
}
