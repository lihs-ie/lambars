//! Integration tests for pattern guard (if let) support in for_! and for_async! macros.

#![cfg(feature = "compose")]
#![allow(deprecated)]

use lambars::for_;

// =============================================================================
// Basic Pattern Guard Tests
// =============================================================================

#[test]
fn test_pattern_guard_option_some() {
    fn maybe_double(x: i32) -> Option<i32> {
        if x > 0 { Some(x * 2) } else { None }
    }

    let result = for_! {
        x <= vec![-1, 0, 1, 2, 3];
        if let Some(doubled) = maybe_double(x);
        yield doubled
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_pattern_guard_option_none() {
    let items = vec![Some(1), None, Some(2), None, None];
    let result = for_! {
        item <= items;
        if let None = item;
        yield "none"
    };
    assert_eq!(result, vec!["none", "none", "none"]);
}

#[test]
fn test_pattern_guard_result_ok() {
    let result = for_! {
        s <= vec!["1", "abc", "2", "xyz", "3"];
        if let Ok(n) = s.parse::<i32>();
        yield n
    };
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_pattern_guard_result_err() {
    let result = for_! {
        s <= vec!["1", "abc", "2", "xyz"];
        if let Err(_) = s.parse::<i32>();
        yield s
    };
    assert_eq!(result, vec!["abc", "xyz"]);
}

#[test]
fn test_pattern_guard_all_match() {
    let items = vec![Some(1), Some(2), Some(3)];
    let result = for_! {
        item <= items;
        if let Some(value) = item;
        yield value
    };
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_pattern_guard_none_match() {
    let items: Vec<Option<i32>> = vec![None, None, None];
    let result = for_! {
        item <= items;
        if let Some(value) = item;
        yield value
    };
    assert!(result.is_empty());
}

// =============================================================================
// Complex Pattern Tests
// =============================================================================

#[test]
fn test_pattern_guard_tuple() {
    let pairs = vec![(1, 2), (3, 4), (5, 6)];
    let result = for_! {
        pair <= pairs;
        if let (a, b) = pair;
        yield a + b
    };
    assert_eq!(result, vec![3, 7, 11]);
}

#[test]
fn test_pattern_guard_struct() {
    #[derive(Clone)]
    struct Point {
        x: i32,
        y: i32,
    }

    let points = vec![Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let result = for_! {
        p <= points;
        if let Point { x, y } = p;
        yield x + y
    };
    assert_eq!(result, vec![3, 7]);
}

#[test]
fn test_pattern_guard_enum_variant() {
    #[derive(Clone)]
    #[allow(dead_code)]
    enum Event {
        Click { x: i32, y: i32 },
        KeyPress(char),
        Resize(u32, u32),
    }

    let events = vec![
        Event::Click { x: 10, y: 20 },
        Event::KeyPress('a'),
        Event::Resize(800, 600),
        Event::Click { x: 30, y: 40 },
    ];

    let result = for_! {
        event <= events;
        if let Event::Click { x, y } = event;
        yield (x, y)
    };
    assert_eq!(result, vec![(10, 20), (30, 40)]);
}

#[test]
fn test_pattern_guard_at_binding() {
    let items = vec![Some(1), None, Some(2)];
    let result = for_! {
        item <= items;
        if let whole @ Some(value) = item;
        yield (whole, value)
    };
    assert_eq!(result, vec![(Some(1), 1), (Some(2), 2)]);
}

#[test]
fn test_pattern_guard_nested() {
    let nested = vec![Some(Some(1)), Some(None), None, Some(Some(2))];
    let result = for_! {
        item <= nested;
        if let Some(Some(value)) = item;
        yield value
    };
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn test_pattern_guard_wildcard() {
    let items = vec![Some(1), None, Some(2)];
    let result = for_! {
        item <= items.clone();
        if let Some(_) = item;
        yield "matched"
    };
    assert_eq!(result, vec!["matched", "matched"]);
}

// =============================================================================
// Combination Tests
// =============================================================================

#[test]
fn test_pattern_guard_with_regular_guard() {
    let items = vec![Some(1), None, Some(5), Some(10)];
    let result = for_! {
        item <= items;
        if let Some(value) = item;
        if value > 3;
        yield value
    };
    assert_eq!(result, vec![5, 10]);
}

#[test]
fn test_pattern_guard_with_let_binding() {
    let items = vec![Some(1), None, Some(2), Some(3)];
    let result = for_! {
        item <= items;
        if let Some(value) = item;
        let doubled = value * 2;
        yield doubled
    };
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_pattern_guard_multiple_consecutive() {
    fn maybe_parse(s: &str) -> Option<i32> {
        s.parse().ok()
    }

    fn maybe_double(n: i32) -> Option<i32> {
        if n > 0 { Some(n * 2) } else { None }
    }

    let result = for_! {
        s <= vec!["1", "abc", "-5", "10"];
        if let Some(n) = maybe_parse(s);
        if let Some(doubled) = maybe_double(n);
        yield doubled
    };
    assert_eq!(result, vec![2, 20]);
}

#[test]
fn test_pattern_guard_with_nested_iteration() {
    let outer = vec![Some(vec![1, 2]), None, Some(vec![3, 4])];
    let result = for_! {
        opt <= outer;
        if let Some(inner) = opt;
        x <= inner;
        yield x * 10
    };
    assert_eq!(result, vec![10, 20, 30, 40]);
}

// =============================================================================
// Async Pattern Guard Tests
// =============================================================================

#[cfg(feature = "async")]
mod async_tests {
    use lambars::effect::AsyncIO;
    use lambars::for_async;

    #[tokio::test]
    async fn test_async_pattern_guard_option_some() {
        fn maybe_double(x: i32) -> Option<i32> {
            if x > 0 { Some(x * 2) } else { None }
        }

        let result = for_async! {
            x <= vec![-1, 0, 1, 2, 3];
            if let Some(doubled) = maybe_double(x);
            yield doubled
        };
        assert_eq!(result.run_async().await, vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_with_async_bind() {
        let result = for_async! {
            x <= vec![1, 2, 3];
            opt <~ AsyncIO::pure(if x > 1 { Some(x * 10) } else { None });
            if let Some(value) = opt;
            yield value
        };
        assert_eq!(result.run_async().await, vec![20, 30]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_multiple() {
        let items = vec![Some(Some(1)), Some(None), None, Some(Some(5))];
        let result = for_async! {
            item <= items;
            if let Some(inner) = item;
            if let Some(value) = inner;
            yield value
        };
        assert_eq!(result.run_async().await, vec![1, 5]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_with_regular_guard() {
        let items = vec![Some(1), None, Some(5), Some(10)];
        let result = for_async! {
            item <= items;
            if let Some(value) = item;
            if value > 3;
            yield value
        };
        assert_eq!(result.run_async().await, vec![5, 10]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_result_ok() {
        let result = for_async! {
            s <= vec!["1", "abc", "2"];
            if let Ok(n) = s.parse::<i32>();
            yield n
        };
        assert_eq!(result.run_async().await, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_with_let() {
        let items = vec![Some(1), None, Some(2)];
        let result = for_async! {
            item <= items;
            if let Some(value) = item;
            let doubled = value * 2;
            yield doubled
        };
        assert_eq!(result.run_async().await, vec![2, 4]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_nested_pattern() {
        let data = vec![Some((1, "a")), None, Some((2, "b"))];
        let result = for_async! {
            item <= data;
            if let Some((num, letter)) = item;
            yield format!("{}{}", num, letter)
        };
        assert_eq!(result.run_async().await, vec!["1a", "2b"]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_struct() {
        #[derive(Clone)]
        struct Person {
            name: String,
            age: Option<u32>,
        }

        let people = vec![
            Person {
                name: "Alice".into(),
                age: Some(30),
            },
            Person {
                name: "Bob".into(),
                age: None,
            },
            Person {
                name: "Charlie".into(),
                age: Some(25),
            },
        ];

        let result = for_async! {
            person <= people;
            if let Person { name, age: Some(a) } = person;
            yield (name, a)
        };
        assert_eq!(
            result.run_async().await,
            vec![("Alice".to_string(), 30), ("Charlie".to_string(), 25),]
        );
    }
}
