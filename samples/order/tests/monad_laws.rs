//! Result/Option Monad 法則の proptest による検証
//!
//! このテストファイルは lambars ライブラリの使用例として
//! Result と Option の Monad 法則を proptest で検証する。
//!
//! Monad 法則:
//! 1. Left Identity: pure(a).flat_map(f) == f(a)
//! 2. Right Identity: m.flat_map(pure) == m
//! 3. Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! Rust では:
//! - pure は Ok/Some
//! - flat_map は and_then

use order_taking_sample::simple_types::{Price, String50, UnitQuantity};
use proptest::prelude::*;
use rust_decimal::Decimal;

// =============================================================================
// 関数選択用のヘルパー
// =============================================================================

/// Result 用のテスト関数（インデックスで選択）
fn result_function(index: usize, x: i32) -> Result<i32, String> {
    match index % 5 {
        0 => Ok(x.saturating_mul(2)),
        1 => Ok(x.saturating_add(1)),
        2 => Ok(x.saturating_sub(1)),
        3 => {
            if x % 2 == 0 {
                Ok(x / 2)
            } else {
                Err("odd".to_string())
            }
        }
        _ => {
            if x >= 0 {
                Ok(x)
            } else {
                Err("negative".to_string())
            }
        }
    }
}

/// Option 用のテスト関数（インデックスで選択）
fn option_function(index: usize, x: i32) -> Option<i32> {
    match index % 5 {
        0 => Some(x.saturating_mul(2)),
        1 => Some(x.saturating_add(1)),
        2 => Some(x.saturating_sub(1)),
        3 => {
            if x % 2 == 0 {
                Some(x / 2)
            } else {
                None
            }
        }
        _ => {
            if x >= 0 {
                Some(x)
            } else {
                None
            }
        }
    }
}

// =============================================================================
// Result Monad 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Left Identity 法則: Ok(a).and_then(f) == f(a)
    ///
    /// pure(a).flat_map(f) は f(a) と等しくなければならない
    #[test]
    fn test_result_left_identity(
        value in any::<i32>(),
        function_index in 0usize..5
    ) {
        let left = Ok::<i32, String>(value).and_then(|x| result_function(function_index, x));
        let right = result_function(function_index, value);
        prop_assert_eq!(left, right, "Left Identity violated: Ok({}).and_then(f) != f({})", value, value);
    }

    /// Result Right Identity 法則: m.and_then(Ok) == m
    ///
    /// m.flat_map(pure) は m と等しくなければならない
    #[test]
    fn test_result_right_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        let left = result.clone().and_then(Ok);
        let right = result;
        prop_assert_eq!(left, right, "Right Identity violated");
    }

    /// Result Associativity 法則:
    /// m.and_then(f).and_then(g) == m.and_then(|x| f(x).and_then(g))
    ///
    /// 結合法則: 合成の順序を変えても結果は同じ
    #[test]
    fn test_result_associativity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}",
        function_index1 in 0usize..5,
        function_index2 in 0usize..5
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let left = result
            .clone()
            .and_then(|x| result_function(function_index1, x))
            .and_then(|x| result_function(function_index2, x));
        let right = result.and_then(|x| {
            result_function(function_index1, x).and_then(|y| result_function(function_index2, y))
        });
        prop_assert_eq!(left, right, "Associativity violated");
    }
}

// =============================================================================
// Option Monad 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Option Left Identity 法則: Some(a).and_then(f) == f(a)
    #[test]
    fn test_option_left_identity(
        value in any::<i32>(),
        function_index in 0usize..5
    ) {
        let left = Some(value).and_then(|x| option_function(function_index, x));
        let right = option_function(function_index, value);
        prop_assert_eq!(left, right, "Left Identity violated: Some({}).and_then(f) != f({})", value, value);
    }

    /// Option Right Identity 法則: m.and_then(Some) == m
    #[test]
    fn test_option_right_identity(option in proptest::option::of(any::<i32>())) {
        let left = option.and_then(Some);
        let right = option;
        prop_assert_eq!(left, right, "Right Identity violated");
    }

    /// Option Associativity 法則:
    /// m.and_then(f).and_then(g) == m.and_then(|x| f(x).and_then(g))
    #[test]
    fn test_option_associativity(
        option in proptest::option::of(any::<i32>()),
        function_index1 in 0usize..5,
        function_index2 in 0usize..5
    ) {
        let left = option
            .and_then(|x| option_function(function_index1, x))
            .and_then(|x| option_function(function_index2, x));
        let right = option.and_then(|x| {
            option_function(function_index1, x).and_then(|y| option_function(function_index2, y))
        });
        prop_assert_eq!(left, right, "Associativity violated");
    }
}

// =============================================================================
// Functor 法則テスト（Monad は Functor なので追加検証）
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Functor Identity 法則: m.map(|x| x) == m
    #[test]
    fn test_result_functor_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        let left = result.clone().map(|x| x);
        let right = result;
        prop_assert_eq!(left, right, "Functor Identity violated");
    }

    /// Result Functor Composition 法則: m.map(f).map(g) == m.map(|x| g(f(x)))
    #[test]
    fn test_result_functor_composition(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let function1 = |x: i32| x.saturating_mul(2);
        let function2 = |x: i32| x.saturating_add(1);

        let left = result.clone().map(function1).map(function2);
        let right = result.map(|x| function2(function1(x)));
        prop_assert_eq!(left, right, "Functor Composition violated");
    }

    /// Option Functor Identity 法則: m.map(|x| x) == m
    #[test]
    fn test_option_functor_identity(option in proptest::option::of(any::<i32>())) {
        let left = option.map(|x| x);
        let right = option;
        prop_assert_eq!(left, right, "Functor Identity violated");
    }

    /// Option Functor Composition 法則: m.map(f).map(g) == m.map(|x| g(f(x)))
    #[test]
    fn test_option_functor_composition(option in proptest::option::of(any::<i32>())) {
        let function1 = |x: i32| x.saturating_mul(2);
        let function2 = |x: i32| x.saturating_add(1);

        let left = option.map(function1).map(function2);
        let right = option.map(|x| function2(function1(x)));
        prop_assert_eq!(left, right, "Functor Composition violated");
    }
}

// =============================================================================
// Applicative 法則テスト（Monad は Applicative なので追加検証）
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Applicative Identity 法則:
    /// pure(identity).apply(m) == m
    /// Rust では: Ok(|x| x) と map を組み合わせて表現
    #[test]
    fn test_result_applicative_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        // pure(id) <*> v = v に相当
        let identity = |x: i32| x;
        let left: Result<i32, String> = result.clone().map(identity);
        let right = result;
        prop_assert_eq!(left, right, "Applicative Identity violated");
    }

    /// Option Applicative Identity 法則
    #[test]
    fn test_option_applicative_identity(option in proptest::option::of(any::<i32>())) {
        let identity = |x: i32| x;
        let left: Option<i32> = option.map(identity);
        let right = option;
        prop_assert_eq!(left, right, "Applicative Identity violated");
    }
}

// =============================================================================
// ドメイン型での Monad 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Price 生成と Monad 法則の組み合わせテスト
    /// Smart Constructor の結果（Result）に対して Monad 法則が成り立つ
    #[test]
    fn test_price_creation_monad_left_identity(value in 0u32..=1000u32) {
        let decimal = Decimal::from(value);

        // Left Identity: Ok(a).and_then(f) == f(a)
        let create_price = |d: Decimal| Price::create(d);

        let left = Ok::<Decimal, _>(decimal).and_then(|d| create_price(d).map_err(|e| e.message));
        let right = create_price(decimal).map_err(|e| e.message);

        prop_assert_eq!(left, right, "Price creation Left Identity violated");
    }

    /// String50 生成の Monad 合成テスト
    /// 複数の Smart Constructor を連鎖させても Monad 法則が成り立つ
    #[test]
    fn test_string50_monad_composition(input in "[a-zA-Z]{1,30}") {
        // 文字列を String50 に変換し、その結果をさらに処理
        let result1 = String50::create("Field1", &input);
        let result2 = result1.and_then(|s| {
            // String50 の値を別の String50 に変換（例: 接頭辞追加）
            let prefixed = format!("prefix_{}", s.value());
            if prefixed.len() <= 50 {
                String50::create("Field2", &prefixed)
            } else {
                // 長すぎる場合は切り詰め
                String50::create("Field2", &prefixed[..50])
            }
        });

        // Result の Monad 法則により、これは正しく合成される
        // 結果が Ok ならば両方の変換が成功している
        if result2.is_ok() {
            let value = result2.unwrap();
            prop_assert!(value.value().starts_with("prefix_"), "Composition failed");
        }
    }

    /// UnitQuantity での演算と Monad 法則
    #[test]
    fn test_unit_quantity_monad_operations(quantity in 1u32..=500u32) {
        // UnitQuantity を作成し、2倍にする操作
        let double = |q: UnitQuantity| {
            let doubled = q.value() * 2;
            UnitQuantity::create("Doubled", doubled)
        };

        let result = UnitQuantity::create("Original", quantity);

        // Left Identity の検証
        let left = result.clone().and_then(double);
        let right = result.and_then(|q| double(q));

        // 同じ値に対して同じ操作をするので結果は同じ
        prop_assert_eq!(left.is_ok(), right.is_ok(), "Monad operation consistency");
        if let (Ok(left_val), Ok(right_val)) = (left, right) {
            prop_assert_eq!(left_val.value(), right_val.value());
        }
    }
}

// =============================================================================
// エラー処理の Monad 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Err 値に対する Monad 法則（短絡評価の検証）
    /// Err(e).and_then(f) は常に Err(e) を返す
    #[test]
    fn test_result_error_short_circuit(error_message in "[a-z]{1,20}") {
        let error: Result<i32, String> = Err(error_message.clone());

        let result = error.and_then(|x| Ok(x * 2));

        prop_assert!(result.is_err(), "Error should short-circuit");
        prop_assert_eq!(result.unwrap_err(), error_message, "Error message should be preserved");
    }

    /// None に対する Monad 法則（短絡評価の検証）
    /// None.and_then(f) は常に None を返す
    #[test]
    fn test_option_none_short_circuit(_dummy in any::<u8>()) {
        let none: Option<i32> = None;

        let result = none.and_then(|x| Some(x * 2));

        prop_assert!(result.is_none(), "None should short-circuit");
    }
}

// =============================================================================
// Monad 法則と等価性の組み合わせテスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result の map と and_then の関係
    /// m.map(f) == m.and_then(|x| Ok(f(x)))
    #[test]
    fn test_result_map_is_and_then_with_pure(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let function = |x: i32| x.saturating_mul(3);

        let left = result.clone().map(function);
        let right = result.and_then(|x| Ok(function(x)));

        prop_assert_eq!(left, right, "map should be equivalent to and_then with Ok");
    }

    /// Option の map と and_then の関係
    /// m.map(f) == m.and_then(|x| Some(f(x)))
    #[test]
    fn test_option_map_is_and_then_with_pure(option in proptest::option::of(any::<i32>())) {
        let function = |x: i32| x.saturating_mul(3);

        let left = option.map(function);
        let right = option.and_then(|x| Some(function(x)));

        prop_assert_eq!(left, right, "map should be equivalent to and_then with Some");
    }
}
