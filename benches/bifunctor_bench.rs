//! Benchmark for Bifunctor type class operations.
//!
//! Compares Bifunctor methods against manual alternatives to evaluate
//! the performance overhead (if any) of the abstraction.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lambars::control::Either;
use lambars::typeclass::Bifunctor;
use std::hint::black_box;

// =============================================================================
// Result Bifunctor Benchmarks
// =============================================================================

fn benchmark_result_bimap(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("result_bimap");

    // Compare bimap vs manual map + map_err chain
    group.bench_function("bifunctor_bimap_ok", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(42);
            black_box(result.bimap(|e| e.len(), |x| x * 2))
        });
    });

    group.bench_function("manual_map_ok", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(42);
            let mapped: Result<i32, usize> = match result {
                Ok(value) => Ok(value * 2),
                Err(error) => Err(error.len()),
            };
            black_box(mapped)
        });
    });

    group.bench_function("bifunctor_bimap_err", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Err("error".to_string());
            black_box(result.bimap(|e| e.len(), |x| x * 2))
        });
    });

    group.bench_function("manual_map_err", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Err("error".to_string());
            let mapped: Result<i32, usize> = match result {
                Ok(value) => Ok(value * 2),
                Err(error) => Err(error.len()),
            };
            black_box(mapped)
        });
    });

    group.finish();
}

fn benchmark_result_first_second(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("result_first_second");

    // first (error transform) vs map_err
    group.bench_function("bifunctor_first", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Err("error".to_string());
            black_box(result.first(|e| e.len()))
        });
    });

    group.bench_function("std_map_err", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Err("error".to_string());
            black_box(result.map_err(|e| e.len()))
        });
    });

    // second (success transform) vs map
    group.bench_function("bifunctor_second", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(42);
            black_box(result.second(|x| x * 2))
        });
    });

    group.bench_function("std_map", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(42);
            black_box(result.map(|x| x * 2))
        });
    });

    group.finish();
}

// =============================================================================
// Either Bifunctor Benchmarks
// =============================================================================

fn benchmark_either_bimap(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("either_bimap");

    group.bench_function("bifunctor_bimap_left", |bencher| {
        bencher.iter(|| {
            let either: Either<i32, String> = Either::Left(42);
            black_box(either.bimap(|x| x * 2, |s: String| s.len()))
        });
    });

    group.bench_function("manual_match_left", |bencher| {
        bencher.iter(|| {
            let either: Either<i32, String> = Either::Left(42);
            let mapped: Either<i32, usize> = match either {
                Either::Left(left) => Either::Left(left * 2),
                Either::Right(right) => Either::Right(right.len()),
            };
            black_box(mapped)
        });
    });

    group.bench_function("bifunctor_bimap_right", |bencher| {
        bencher.iter(|| {
            let either: Either<i32, String> = Either::Right("hello".to_string());
            black_box(either.bimap(|x: i32| x * 2, |s| s.len()))
        });
    });

    group.bench_function("manual_match_right", |bencher| {
        bencher.iter(|| {
            let either: Either<i32, String> = Either::Right("hello".to_string());
            let mapped: Either<i32, usize> = match either {
                Either::Left(left) => Either::Left(left * 2),
                Either::Right(right) => Either::Right(right.len()),
            };
            black_box(mapped)
        });
    });

    group.finish();
}

// =============================================================================
// Tuple Bifunctor Benchmarks
// =============================================================================

fn benchmark_tuple_bimap(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("tuple_bimap");

    group.bench_function("bifunctor_bimap", |bencher| {
        bencher.iter(|| {
            let tuple = (42, "hello".to_string());
            black_box(tuple.bimap(|x| x * 2, |s| s.len()))
        });
    });

    group.bench_function("manual_transform", |bencher| {
        bencher.iter(|| {
            let tuple = (42, "hello".to_string());
            black_box((tuple.0 * 2, tuple.1.len()))
        });
    });

    group.bench_function("bifunctor_first", |bencher| {
        bencher.iter(|| {
            let tuple = (42, "hello".to_string());
            black_box(tuple.first(|x| x * 2))
        });
    });

    group.bench_function("manual_first", |bencher| {
        bencher.iter(|| {
            let tuple = (42, "hello".to_string());
            black_box((tuple.0 * 2, tuple.1))
        });
    });

    group.finish();
}

// =============================================================================
// Reference-based Operations (bimap_ref)
// =============================================================================

fn benchmark_bimap_ref(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bimap_ref");

    // Result bimap_ref
    let result: Result<i32, String> = Ok(42);
    group.bench_function("result_bimap_ref", |bencher| {
        bencher.iter(|| black_box(result.bimap_ref(|e| e.len(), |x| x * 2)));
    });

    group.bench_function("result_as_ref_map", |bencher| {
        bencher.iter(|| {
            let mapped: Result<i32, usize> = match &result {
                Ok(value) => Ok(*value * 2),
                Err(error) => Err(error.len()),
            };
            black_box(mapped)
        });
    });

    // Either bimap_ref
    let either: Either<String, i32> = Either::Right(42);
    group.bench_function("either_bimap_ref", |bencher| {
        bencher.iter(|| black_box(either.bimap_ref(|s| s.len(), |n| n * 2)));
    });

    group.bench_function("either_manual_ref", |bencher| {
        bencher.iter(|| {
            let mapped: Either<usize, i32> = match &either {
                Either::Left(left) => Either::Left(left.len()),
                Either::Right(right) => Either::Right(*right * 2),
            };
            black_box(mapped)
        });
    });

    // Tuple bimap_ref
    let tuple = (42, "hello".to_string());
    group.bench_function("tuple_bimap_ref", |bencher| {
        bencher.iter(|| black_box(tuple.bimap_ref(|x| x * 2, |s| s.len())));
    });

    group.bench_function("tuple_manual_ref", |bencher| {
        bencher.iter(|| black_box((tuple.0 * 2, tuple.1.len())));
    });

    group.finish();
}

// =============================================================================
// Chained Operations (composition performance)
// =============================================================================

fn benchmark_chained_operations(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("chained_operations");

    // Multiple bimap calls vs single bimap with composed functions
    group.bench_function("single_bimap_composed", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(5);
            black_box(result.bimap(|e| e.len() + 100, |x| (x + 1) * 2))
        });
    });

    group.bench_function("chained_bimap", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(5);
            black_box(
                result
                    .bimap(|e| e.len(), |x| x + 1)
                    .bimap(|e| e + 100, |x| x * 2),
            )
        });
    });

    group.bench_function("first_then_second", |bencher| {
        bencher.iter(|| {
            let result: Result<i32, String> = Ok(5);
            black_box(
                result
                    .first(|e| e.len())
                    .second(|x| x + 1)
                    .first(|e| e + 100)
                    .second(|x| x * 2),
            )
        });
    });

    group.finish();
}

// =============================================================================
// Throughput Benchmarks (larger data)
// =============================================================================

fn benchmark_throughput(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("throughput");

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("bifunctor_iter", size),
            &size,
            |bencher, &size| {
                let items: Vec<Result<i32, String>> = (0..size).map(Ok).collect();
                bencher.iter(|| {
                    let results: Vec<_> = items
                        .iter()
                        .map(|r| r.clone().bimap(|e| e.len(), |x| x * 2))
                        .collect();
                    black_box(results)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("manual_iter", size),
            &size,
            |bencher, &size| {
                let items: Vec<Result<i32, String>> = (0..size).map(Ok).collect();
                bencher.iter(|| {
                    let results: Vec<_> = items
                        .iter()
                        .map(|r| match r.clone() {
                            Ok(value) => Ok(value * 2),
                            Err(error) => Err(error.len()),
                        })
                        .collect();
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Complex Transformation Benchmarks
// =============================================================================

fn benchmark_complex_transforms(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("complex_transforms");

    // String parsing and error handling
    group.bench_function("bifunctor_parse_transform", |bencher| {
        bencher.iter(|| {
            let result: Result<&str, &str> = Ok("42");
            black_box(result.bimap(
                |e| format!("Error: {}", e),
                |s| s.parse::<i32>().unwrap_or(0) * 2,
            ))
        });
    });

    group.bench_function("manual_parse_transform", |bencher| {
        bencher.iter(|| {
            let result: Result<&str, &str> = Ok("42");
            let mapped: Result<i32, String> = match result {
                Ok(value) => Ok(value.parse::<i32>().unwrap_or(0) * 2),
                Err(error) => Err(format!("Error: {}", error)),
            };
            black_box(mapped)
        });
    });

    // Nested structure transformation
    group.bench_function("bifunctor_nested", |bencher| {
        bencher.iter(|| {
            let outer: Result<(i32, String), (String, i32)> = Ok((42, "hello".to_string()));
            black_box(outer.bimap(|e| (e.0.len(), e.1 * 2), |ok| (ok.0 * 2, ok.1.len())))
        });
    });

    group.bench_function("manual_nested", |bencher| {
        bencher.iter(|| {
            let outer: Result<(i32, String), (String, i32)> = Ok((42, "hello".to_string()));
            let mapped: Result<(i32, usize), (usize, i32)> = match outer {
                Ok((num, text)) => Ok((num * 2, text.len())),
                Err((text, num)) => Err((text.len(), num * 2)),
            };
            black_box(mapped)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_result_bimap,
    benchmark_result_first_second,
    benchmark_either_bimap,
    benchmark_tuple_bimap,
    benchmark_bimap_ref,
    benchmark_chained_operations,
    benchmark_throughput,
    benchmark_complex_transforms,
);

criterion_main!(benches);
