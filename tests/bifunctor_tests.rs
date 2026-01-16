#![cfg(feature = "typeclass")]
//! Integration tests for Bifunctor type class.

use lambars::control::Either;
use lambars::typeclass::Bifunctor;
use rstest::rstest;

mod either_bifunctor {
    use super::*;

    #[rstest]
    fn bimap_transforms_left_value() {
        let either: Either<i32, String> = Either::Left(42);
        let result = either.bimap(|x| x * 2, |s: String| s.len());
        assert_eq!(result, Either::Left(84));
    }

    #[rstest]
    fn bimap_transforms_right_value() {
        let either: Either<i32, String> = Either::Right("hello".to_string());
        let result = either.bimap(|x: i32| x * 2, |s| s.len());
        assert_eq!(result, Either::Right(5));
    }

    #[rstest]
    fn first_transforms_left_value() {
        let either: Either<i32, String> = Either::Left(42);
        let result = either.first(|x| format!("value: {}", x));
        assert_eq!(result, Either::Left("value: 42".to_string()));
    }

    #[rstest]
    fn first_leaves_right_unchanged() {
        let either: Either<i32, String> = Either::Right("hello".to_string());
        let result = either.first(|x: i32| format!("value: {}", x));
        assert_eq!(result, Either::Right("hello".to_string()));
    }

    #[rstest]
    fn second_transforms_right_value() {
        let either: Either<i32, String> = Either::Right("hello".to_string());
        let result = either.second(|s| s.len());
        assert_eq!(result, Either::Right(5));
    }

    #[rstest]
    fn second_leaves_left_unchanged() {
        let either: Either<i32, String> = Either::Left(42);
        let result = either.second(|s: String| s.len());
        assert_eq!(result, Either::Left(42));
    }

    #[rstest]
    fn bimap_ref_transforms_without_consuming() {
        let either: Either<String, i32> = Either::Left("hello".to_string());
        let result = either.bimap_ref(|s| s.len(), |n| n * 2);
        assert!(either.is_left());
        assert_eq!(result, Either::Left(5));
    }

    #[rstest]
    fn bimap_ref_transforms_right_without_consuming() {
        let either: Either<String, i32> = Either::Right(42);
        let result = either.bimap_ref(|s| s.len(), |n| n * 2);
        assert!(either.is_right());
        assert_eq!(result, Either::Right(84));
    }

    #[rstest]
    fn first_ref_transforms_left_without_consuming() {
        let either: Either<String, i32> = Either::Left("hello".to_string());
        let result = either.first_ref(|s| s.len());
        assert!(either.is_left());
        assert_eq!(result, Either::Left(5));
    }

    #[rstest]
    fn first_ref_clones_right_value() {
        let either: Either<String, i32> = Either::Right(42);
        let result = either.first_ref(|s: &String| s.len());
        assert_eq!(result, Either::Right(42));
    }

    #[rstest]
    fn second_ref_transforms_right_without_consuming() {
        let either: Either<String, i32> = Either::Right(42);
        let result = either.second_ref(|n| n * 2);
        assert!(either.is_right());
        assert_eq!(result, Either::Right(84));
    }

    #[rstest]
    fn second_ref_clones_left_value() {
        let either: Either<String, i32> = Either::Left("hello".to_string());
        let result = either.second_ref(|n: &i32| n * 2);
        assert_eq!(result, Either::Left("hello".to_string()));
    }
}

mod result_bifunctor {
    use super::*;

    #[rstest]
    fn bimap_transforms_ok_value() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.bimap(|e| e.len(), |x| x * 2);
        assert_eq!(mapped, Ok(84));
    }

    #[rstest]
    fn bimap_transforms_err_value() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.bimap(|e| e.len(), |x| x * 2);
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn first_transforms_error_value() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.first(|e| e.len());
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn first_leaves_ok_unchanged() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.first(|e: String| e.len());
        assert_eq!(mapped, Ok(42));
    }

    #[rstest]
    fn second_transforms_success_value() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.second(|x| x.to_string());
        assert_eq!(mapped, Ok("42".to_string()));
    }

    #[rstest]
    fn second_leaves_err_unchanged() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.second(|x: i32| x.to_string());
        assert_eq!(mapped, Err("error".to_string()));
    }

    #[rstest]
    fn bimap_ref_transforms_ok_without_consuming() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.bimap_ref(|e| e.len(), |x| x * 2);
        assert_eq!(result, Ok(42));
        assert_eq!(mapped, Ok(84));
    }

    #[rstest]
    fn bimap_ref_transforms_err_without_consuming() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.bimap_ref(|e| e.len(), |x| x * 2);
        assert_eq!(result, Err("error".to_string())); // still usable
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn first_ref_transforms_error_without_consuming() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.first_ref(|e| e.len());
        assert_eq!(result, Err("error".to_string()));
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn first_ref_on_ok_clones_success_value() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.first_ref(|e: &String| e.len());
        assert_eq!(result, Ok(42));
        assert_eq!(mapped, Ok(42));
    }

    #[rstest]
    fn second_ref_transforms_success_without_consuming() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.second_ref(|x| x * 2);
        assert_eq!(result, Ok(42));
        assert_eq!(mapped, Ok(84));
    }

    #[rstest]
    fn second_ref_on_err_clones_error_value() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.second_ref(|x: &i32| x * 2);
        assert_eq!(result, Err("error".to_string()));
        assert_eq!(mapped, Err("error".to_string()));
    }

    #[rstest]
    fn second_is_equivalent_to_functor_fmap() {
        use lambars::typeclass::Functor;

        let result: Result<i32, String> = Ok(42);
        let by_second = result.clone().second(|x| x * 2);
        let by_fmap = result.fmap(|x| x * 2);
        assert_eq!(by_second, by_fmap);
    }
}

mod tuple_bifunctor {
    use super::*;

    #[rstest]
    fn bimap_transforms_both_elements() {
        let tuple = (42, "hello".to_string());
        let result = tuple.bimap(|x| x * 2, |s| s.len());
        assert_eq!(result, (84, 5));
    }

    #[rstest]
    fn first_transforms_first_element() {
        let tuple = (42, "hello".to_string());
        let result = tuple.first(|x| x.to_string());
        assert_eq!(result, ("42".to_string(), "hello".to_string()));
    }

    #[rstest]
    fn second_transforms_second_element() {
        let tuple = (42, "hello".to_string());
        let result = tuple.second(|s| s.len());
        assert_eq!(result, (42, 5));
    }

    #[rstest]
    fn bimap_ref_transforms_without_consuming() {
        let tuple = (42, "hello".to_string());
        let result = tuple.bimap_ref(|x| x * 2, |s| s.len());
        assert_eq!(tuple.0, 42);
        assert_eq!(result, (84, 5));
    }

    #[rstest]
    fn first_ref_transforms_first_without_consuming() {
        let tuple = (42, "hello".to_string());
        let result = tuple.first_ref(|x| x * 2);
        assert_eq!(tuple.0, 42);
        assert_eq!(result, (84, "hello".to_string()));
    }

    #[rstest]
    fn second_ref_transforms_second_without_consuming() {
        let tuple = (42, "hello".to_string());
        let result = tuple.second_ref(|s| s.len());
        assert_eq!(tuple.1, "hello".to_string());
        assert_eq!(result, (42, 5));
    }
}

mod bifunctor_laws {
    use super::*;

    mod identity_law {
        use super::*;

        #[rstest]
        fn either_left_identity() {
            let either: Either<i32, String> = Either::Left(42);
            let result = either.clone().bimap(|x| x, |y| y);
            assert_eq!(result, either);
        }

        #[rstest]
        fn either_right_identity() {
            let either: Either<i32, String> = Either::Right("hello".to_string());
            let result = either.clone().bimap(|x| x, |y| y);
            assert_eq!(result, either);
        }

        #[rstest]
        fn result_ok_identity() {
            let result: Result<i32, String> = Ok(42);
            let mapped = result.clone().bimap(|e| e, |x| x);
            assert_eq!(mapped, result);
        }

        #[rstest]
        fn result_err_identity() {
            let result: Result<i32, String> = Err("error".to_string());
            let mapped = result.clone().bimap(|e| e, |x| x);
            assert_eq!(mapped, result);
        }

        #[rstest]
        fn tuple_identity() {
            let tuple = (42, "hello".to_string());
            let result = tuple.clone().bimap(|x| x, |y| y);
            assert_eq!(result, tuple);
        }
    }

    mod composition_law {
        use super::*;

        #[rstest]
        fn either_left_composition() {
            let either: Either<i32, String> = Either::Left(5);
            let function1 = |x: i32| x + 1;
            let function2 = |x: i32| x * 2;
            let function3 = |s: String| s.len();
            let function4 = |n: usize| n + 10;

            // bimap(f2 . f1, f4 . f3)
            let left = either
                .clone()
                .bimap(|x| function2(function1(x)), |s| function4(function3(s)));

            // bimap(f1, f3).bimap(f2, f4)
            let right = either
                .bimap(function1, function3)
                .bimap(function2, function4);

            assert_eq!(left, right);
            assert_eq!(left, Either::Left(12)); // (5 + 1) * 2 = 12
        }

        #[rstest]
        fn either_right_composition() {
            let either: Either<i32, String> = Either::Right("hello".to_string());
            let function1 = |x: i32| x + 1;
            let function2 = |x: i32| x * 2;
            let function3 = |s: String| s.len();
            let function4 = |n: usize| n + 10;

            let left = either
                .clone()
                .bimap(|x| function2(function1(x)), |s| function4(function3(s)));
            let right = either
                .bimap(function1, function3)
                .bimap(function2, function4);

            assert_eq!(left, right);
            assert_eq!(left, Either::Right(15)); // 5 + 10 = 15
        }

        #[rstest]
        fn result_ok_composition() {
            let result: Result<i32, String> = Ok(5);
            let error_transform1 = |e: String| e.len();
            let error_transform2 = |n: usize| n + 100;
            let success_transform1 = |x: i32| x + 1;
            let success_transform2 = |x: i32| x * 2;

            let left = result.clone().bimap(
                |e| error_transform2(error_transform1(e)),
                |x| success_transform2(success_transform1(x)),
            );
            let right = result
                .bimap(error_transform1, success_transform1)
                .bimap(error_transform2, success_transform2);

            assert_eq!(left, right);
            assert_eq!(left, Ok(12)); // (5 + 1) * 2 = 12
        }

        #[rstest]
        fn result_err_composition() {
            let result: Result<i32, String> = Err("error".to_string());
            let error_transform1 = |e: String| e.len();
            let error_transform2 = |n: usize| n + 100;
            let success_transform1 = |x: i32| x + 1;
            let success_transform2 = |x: i32| x * 2;

            let left = result.clone().bimap(
                |e| error_transform2(error_transform1(e)),
                |x| success_transform2(success_transform1(x)),
            );
            let right = result
                .bimap(error_transform1, success_transform1)
                .bimap(error_transform2, success_transform2);

            assert_eq!(left, right);
            assert_eq!(left, Err(105)); // 5 + 100 = 105
        }

        #[rstest]
        fn tuple_composition() {
            let tuple = (5, "hello".to_string());
            let function1 = |x: i32| x + 1;
            let function2 = |x: i32| x * 2;
            let function3 = |s: String| s.len();
            let function4 = |n: usize| n + 10;

            let left = tuple
                .clone()
                .bimap(|x| function2(function1(x)), |s| function4(function3(s)));
            let right = tuple
                .bimap(function1, function3)
                .bimap(function2, function4);

            assert_eq!(left, right);
            assert_eq!(left, (12, 15)); // (5+1)*2=12, 5+10=15
        }
    }

    mod first_second_consistency {
        use super::*;

        #[rstest]
        fn either_first_second_consistency() {
            let either: Either<i32, String> = Either::Left(42);
            let function_first = |x: i32| x * 2;
            let function_second = |s: String| s.len();

            let by_bimap = either.clone().bimap(function_first, function_second);
            let by_first_second = either.clone().first(function_first).second(function_second);
            let by_second_first = either.second(function_second).first(function_first);

            assert_eq!(by_bimap, by_first_second);
            assert_eq!(by_first_second, by_second_first);
        }

        #[rstest]
        fn result_first_second_consistency() {
            let result: Result<i32, String> = Ok(42);
            let function_first = |e: String| e.len();
            let function_second = |x: i32| x * 2;

            let by_bimap = result.clone().bimap(function_first, function_second);
            let by_first_second = result.clone().first(function_first).second(function_second);
            let by_second_first = result.second(function_second).first(function_first);

            assert_eq!(by_bimap, by_first_second);
            assert_eq!(by_first_second, by_second_first);
        }

        #[rstest]
        fn tuple_first_second_consistency() {
            let tuple = (42, "hello".to_string());
            let function_first = |x: i32| x * 2;
            let function_second = |s: String| s.len();

            let by_bimap = tuple.clone().bimap(function_first, function_second);
            let by_first_second = tuple.clone().first(function_first).second(function_second);
            let by_second_first = tuple.second(function_second).first(function_first);

            assert_eq!(by_bimap, by_first_second);
            assert_eq!(by_first_second, by_second_first);
        }
    }
}
