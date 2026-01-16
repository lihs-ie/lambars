//! Benchmark for Alternative type class.
//!
//! Measures the performance of Alternative operations and compares them
//! with standard library equivalents to evaluate abstraction overhead
//! and practical usefulness.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::typeclass::{Alternative, AlternativeVec, Functor};
use std::hint::black_box;

// =============================================================================
// 1. Option alt vs or - Abstraction Overhead
// =============================================================================

fn benchmark_option_alt_vs_or(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("option_alt_vs_or");

    // Simple case: None.alt(Some)
    group.bench_function("alt_none_some", |bencher| {
        bencher.iter(|| {
            let first: Option<i32> = None;
            let second: Option<i32> = Some(42);
            black_box(first.alt(second))
        });
    });

    group.bench_function("or_none_some", |bencher| {
        bencher.iter(|| {
            let first: Option<i32> = None;
            let second: Option<i32> = Some(42);
            black_box(first.or(second))
        });
    });

    // Simple case: Some.alt(Some)
    group.bench_function("alt_some_some", |bencher| {
        bencher.iter(|| {
            let first: Option<i32> = Some(1);
            let second: Option<i32> = Some(2);
            black_box(first.alt(second))
        });
    });

    group.bench_function("or_some_some", |bencher| {
        bencher.iter(|| {
            let first: Option<i32> = Some(1);
            let second: Option<i32> = Some(2);
            black_box(first.or(second))
        });
    });

    // Chained alt operations
    for chain_length in [3, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("alt_chain", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut result: Option<i32> = None;
                    for iteration in 0..length {
                        result = result.alt(if iteration == length - 1 {
                            Some(iteration)
                        } else {
                            None
                        });
                    }
                    black_box(result)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("or_chain", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut result: Option<i32> = None;
                    for iteration in 0..length {
                        result = result.or(if iteration == length - 1 {
                            Some(iteration)
                        } else {
                            None
                        });
                    }
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 2. choice - Failure Distribution Tests
// =============================================================================

fn benchmark_choice_distribution(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("choice_distribution");

    // Test with different success positions
    for size in [10, 50, 100] {
        // First element succeeds (best case for short-circuit)
        let first_success: Vec<Option<i32>> = std::iter::once(Some(1))
            .chain(std::iter::repeat_n(None, size - 1))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("choice_first_success", size),
            &first_success,
            |bencher, alternatives| {
                bencher.iter(|| black_box(Option::choice(alternatives.clone())))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("find_flatten_first_success", size),
            &first_success,
            |bencher, alternatives| {
                bencher.iter(|| {
                    black_box(
                        alternatives
                            .clone()
                            .into_iter()
                            .find(Option::is_some)
                            .flatten(),
                    )
                })
            },
        );

        // Last element succeeds (worst case)
        let last_success: Vec<Option<i32>> = std::iter::repeat_n(None, size - 1)
            .chain(std::iter::once(Some(1)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("choice_last_success", size),
            &last_success,
            |bencher, alternatives| {
                bencher.iter(|| black_box(Option::choice(alternatives.clone())))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("find_flatten_last_success", size),
            &last_success,
            |bencher, alternatives| {
                bencher.iter(|| {
                    black_box(
                        alternatives
                            .clone()
                            .into_iter()
                            .find(Option::is_some)
                            .flatten(),
                    )
                })
            },
        );

        // All fail
        let all_none: Vec<Option<i32>> = std::iter::repeat_n(None, size).collect();

        group.bench_with_input(
            BenchmarkId::new("choice_all_none", size),
            &all_none,
            |bencher, alternatives| {
                bencher.iter(|| black_box(Option::choice(alternatives.clone())))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("find_flatten_all_none", size),
            &all_none,
            |bencher, alternatives| {
                bencher.iter(|| {
                    black_box(
                        alternatives
                            .clone()
                            .into_iter()
                            .find(Option::is_some)
                            .flatten(),
                    )
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// 3. Vec alt vs extend - Immutability Cost
// =============================================================================

fn benchmark_vec_alt_vs_extend(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vec_alt_vs_extend");

    for size in [100, 1000, 10000] {
        let first: Vec<i32> = (0..size).collect();
        let second: Vec<i32> = (size..size * 2).collect();

        group.bench_with_input(
            BenchmarkId::new("alt_immutable", size),
            &(first.clone(), second.clone()),
            |bencher, (first, second)| {
                bencher.iter(|| black_box(first.clone().alt(second.clone())))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("extend_mutable", size),
            &(first.clone(), second.clone()),
            |bencher, (first, second)| {
                bencher.iter(|| {
                    let mut result = first.clone();
                    result.extend(second.clone());
                    black_box(result)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("chain_collect", size),
            &(first.clone(), second.clone()),
            |bencher, (first, second)| {
                bencher.iter(|| {
                    black_box(
                        first
                            .clone()
                            .into_iter()
                            .chain(second.clone())
                            .collect::<Vec<_>>(),
                    )
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// 4. guard - Abstraction Cost and Composability
// =============================================================================

fn benchmark_guard(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("guard");

    // Single guard comparison
    group.bench_function("guard_with_fmap", |bencher| {
        bencher.iter(|| {
            let value = 42i32;
            black_box(<Option<()>>::guard(value > 0).fmap(move |()| value))
        });
    });

    group.bench_function("manual_if_else", |bencher| {
        bencher.iter(|| {
            let value = 42i32;
            black_box(if value > 0 { Some(value) } else { None })
        });
    });

    // Multi-condition guard (composability test)
    group.bench_function("guard_multi_condition", |bencher| {
        bencher.iter(|| {
            let value = 42i32;
            // Multiple guards composed
            let guard1 = <Option<()>>::guard(value > 0);
            let guard2 = <Option<()>>::guard(value < 100);
            let guard3 = <Option<()>>::guard(value % 2 == 0);
            black_box(guard1.alt(guard2).alt(guard3).fmap(move |()| value))
        });
    });

    group.bench_function("manual_multi_condition", |bencher| {
        bencher.iter(|| {
            let value = 42i32;
            black_box(if value > 0 || value < 100 || value % 2 == 0 {
                Some(value)
            } else {
                None
            })
        });
    });

    // Guard in a loop (filtering scenario)
    for size in [100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("guard_filter", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let results: Vec<Option<i32>> = (0..size)
                        .map(|number| <Option<()>>::guard(number % 2 == 0).fmap(move |()| number))
                        .collect();
                    black_box(results)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("manual_filter", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let results: Vec<Option<i32>> = (0..size)
                        .map(|number| if number % 2 == 0 { Some(number) } else { None })
                        .collect();
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 5. optional - Failure Value Pattern
// =============================================================================

fn benchmark_optional(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("optional");

    // Success case
    group.bench_function("optional_some", |bencher| {
        bencher.iter(|| {
            let value: Option<i32> = Some(42);
            black_box(value.optional())
        });
    });

    group.bench_function("manual_some_wrap", |bencher| {
        bencher.iter(|| {
            let value: Option<i32> = Some(42);
            black_box(Some(value))
        });
    });

    // Failure case
    group.bench_function("optional_none", |bencher| {
        bencher.iter(|| {
            let value: Option<i32> = None;
            black_box(value.optional())
        });
    });

    group.bench_function("manual_none_wrap", |bencher| {
        bencher.iter(|| {
            let value: Option<i32> = None;
            black_box(Some(value))
        });
    });

    // Vec optional (non-deterministic)
    for size in [10, 100] {
        group.bench_with_input(
            BenchmarkId::new("vec_optional", size),
            &size,
            |bencher, &size| {
                let vec: Vec<i32> = (0..size).collect();
                bencher.iter(|| black_box(vec.clone().optional()))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("vec_manual_optional", size),
            &size,
            |bencher, &size| {
                let vec: Vec<i32> = (0..size).collect();
                bencher.iter(|| {
                    let mut result: Vec<Option<i32>> = vec.clone().into_iter().map(Some).collect();
                    result.push(None);
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// 6. Parser Combinator Simulation
// =============================================================================

fn benchmark_parser_simulation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("parser_simulation");

    // Simulated parser functions
    fn parse_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
        input.strip_prefix(keyword)
    }

    let keywords = ["if", "else", "while", "for", "match", "let", "fn", "struct"];

    // Different input scenarios
    let inputs = [
        ("if x > 0", "first_keyword"),
        ("struct Foo", "last_keyword"),
        ("unknown_token", "no_match"),
    ];

    for (input, scenario) in inputs {
        group.bench_with_input(
            BenchmarkId::new("choice_parser", scenario),
            &input,
            |bencher, &input| {
                bencher.iter(|| {
                    let parsers: Vec<Option<&str>> =
                        keywords.iter().map(|kw| parse_keyword(input, kw)).collect();
                    black_box(Option::choice(parsers))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("manual_parser", scenario),
            &input,
            |bencher, &input| {
                bencher.iter(|| {
                    let mut result: Option<&str> = None;
                    for keyword in &keywords {
                        result = result.or_else(|| parse_keyword(input, keyword));
                        if result.is_some() {
                            break;
                        }
                    }
                    black_box(result)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("iterator_parser", scenario),
            &input,
            |bencher, &input| {
                bencher.iter(|| {
                    black_box(
                        keywords
                            .iter()
                            .find_map(|keyword| parse_keyword(input, keyword)),
                    )
                });
            },
        );
    }

    // Scaling test with more keywords
    for keyword_count in [10, 50, 100] {
        let keywords: Vec<String> = (0..keyword_count)
            .map(|keyword_number| format!("kw{}", keyword_number))
            .collect();
        let input = "kw50 rest"; // Middle keyword

        group.bench_with_input(
            BenchmarkId::new("choice_parser_scale", keyword_count),
            &(input, &keywords),
            |bencher, &(input, keywords)| {
                bencher.iter(|| {
                    let parsers: Vec<Option<&str>> =
                        keywords.iter().map(|kw| parse_keyword(input, kw)).collect();
                    black_box(Option::choice(parsers))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("iterator_parser_scale", keyword_count),
            &(input, &keywords),
            |bencher, &(input, keywords)| {
                bencher.iter(|| {
                    black_box(
                        keywords
                            .iter()
                            .find_map(|keyword| parse_keyword(input, keyword)),
                    )
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group!(
    benches,
    benchmark_option_alt_vs_or,
    benchmark_choice_distribution,
    benchmark_vec_alt_vs_extend,
    benchmark_guard,
    benchmark_optional,
    benchmark_parser_simulation,
);

criterion_main!(benches);
