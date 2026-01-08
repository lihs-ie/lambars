//! Benchmark for parallel iteration with rayon.
//!
//! These benchmarks focus on **CPU-intensive operations** where parallel
//! processing provides significant benefits. Simple operations like sum or
//! basic arithmetic are intentionally excluded as they are dominated by
//! thread synchronization overhead.
//!
//! ## When to use parallel iteration
//!
//! Parallel iteration is beneficial when:
//! - Each element requires significant computation (>1µs per element)
//! - The collection has enough elements to amortize thread overhead
//! - The operation is CPU-bound, not I/O-bound
//!
//! ## Benchmarks included
//!
//! 1. **Cryptographic hashing**: SHA-256-like computation per element
//! 2. **Prime factorization**: Finding all prime factors of large numbers
//! 3. **Matrix operations**: Small matrix multiplications per element
//! 4. **String processing**: Complex regex-like pattern matching
//! 5. **Numerical integration**: Monte Carlo simulation per element
//!
//! Requires the `rayon` feature to be enabled.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::{PersistentHashMap, PersistentHashSet, PersistentVector};
use rayon::prelude::*;
use std::hint::black_box;

// =============================================================================
// CPU-Intensive Computation Functions
// =============================================================================

/// Simulates a cryptographic hash computation (SHA-256-like iterations).
/// Each call performs ~1000 mixing operations.
#[inline(never)]
fn crypto_hash_simulation(input: i64) -> u64 {
    let mut state = [
        input as u64,
        0x6a09e667bb67ae85,
        0x3c6ef372a54ff53a,
        0x510e527f9b05688c,
    ];

    for round in 0..64 {
        let t1 = state[3]
            .wrapping_add(state[1].rotate_right(6) ^ state[1].rotate_right(11))
            .wrapping_add(state[1] & state[2] ^ !state[1] & state[3])
            .wrapping_add(round as u64);

        let t2 = (state[0].rotate_right(2) ^ state[0].rotate_right(13))
            .wrapping_add(state[0] & state[1] ^ state[0] & state[2] ^ state[1] & state[2]);

        state[3] = state[2];
        state[2] = state[1];
        state[1] = state[0].wrapping_add(t1);
        state[0] = t1.wrapping_add(t2);
    }

    state[0] ^ state[1] ^ state[2] ^ state[3]
}

/// Finds all prime factors of a number using trial division.
/// Computational complexity varies with input but averages O(sqrt(n)).
#[inline(never)]
fn prime_factorization(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();

    while n.is_multiple_of(2) {
        factors.push(2);
        n /= 2;
    }

    let mut i = 3u64;
    while i * i <= n {
        while n.is_multiple_of(i) {
            factors.push(i);
            n /= i;
        }
        i += 2;
    }

    if n > 1 {
        factors.push(n);
    }

    factors
}

/// Performs a small 4x4 matrix multiplication.
/// Each call does 64 multiply-add operations.
#[inline(never)]
fn matrix_multiply_4x4(seed: i64) -> [[f64; 4]; 4] {
    let s = seed as f64;
    let a = [
        [s, s + 1.0, s + 2.0, s + 3.0],
        [s + 4.0, s + 5.0, s + 6.0, s + 7.0],
        [s + 8.0, s + 9.0, s + 10.0, s + 11.0],
        [s + 12.0, s + 13.0, s + 14.0, s + 15.0],
    ];

    let b = [
        [s + 16.0, s + 17.0, s + 18.0, s + 19.0],
        [s + 20.0, s + 21.0, s + 22.0, s + 23.0],
        [s + 24.0, s + 25.0, s + 26.0, s + 27.0],
        [s + 28.0, s + 29.0, s + 30.0, s + 31.0],
    ];

    let mut result = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }

    // Additional iterations to increase computation time
    for _ in 0..10 {
        let temp = result;
        for i in 0..4 {
            for j in 0..4 {
                result[i][j] = 0.0;
                for k in 0..4 {
                    result[i][j] += temp[i][k] * a[k][j];
                }
            }
        }
    }

    result
}

/// Simulates complex string pattern matching with backtracking.
/// Performs multiple passes over an internal state machine.
#[inline(never)]
fn pattern_matching_simulation(input: i64) -> u32 {
    let pattern_length = 16;
    let mut state = input as u64;
    let mut match_count = 0u32;

    for _ in 0..100 {
        let mut pattern_state = 0u64;
        for bit in 0..pattern_length {
            let current_bit = (state >> bit) & 1;
            let pattern_bit = (pattern_state.wrapping_mul(31).wrapping_add(17)) & 1;

            if current_bit == pattern_bit {
                match_count += 1;
                pattern_state = pattern_state.wrapping_add(current_bit);
            } else {
                // Backtrack simulation
                pattern_state = pattern_state.rotate_right(3);
            }

            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        }
    }

    match_count
}

/// Performs Monte Carlo integration to estimate pi.
/// Uses a deterministic PRNG seeded with the input.
#[inline(never)]
fn monte_carlo_pi_estimation(seed: i64, samples: u32) -> f64 {
    let mut state = seed as u64;
    let mut inside_circle = 0u32;

    for _ in 0..samples {
        // LCG random number generator
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = (state >> 33) as f64 / (u32::MAX as f64);

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = (state >> 33) as f64 / (u32::MAX as f64);

        if x * x + y * y <= 1.0 {
            inside_circle += 1;
        }
    }

    4.0 * (inside_circle as f64) / (samples as f64)
}

/// Fibonacci with memoization simulation (iterative).
/// Computes fibonacci and performs additional work.
#[inline(never)]
fn fibonacci_with_work(n: u64) -> u64 {
    let n = n % 90; // Prevent overflow
    let mut memo = vec![0u64; (n + 1) as usize];

    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    memo[1] = 1;
    for i in 2..=n as usize {
        memo[i] = memo[i - 1].wrapping_add(memo[i - 2]);

        // Additional computation per step
        for j in 0..i.min(10) {
            memo[i] = memo[i].wrapping_add(memo[j].wrapping_mul(3));
        }
    }

    memo[n as usize]
}

// =============================================================================
// PersistentVector Benchmarks - CPU Intensive
// =============================================================================

fn benchmark_crypto_hash(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("crypto_hash");
    group.sample_size(50);

    for size in [10_000, 50_000, 100_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = vector.iter().map(|x| crypto_hash_simulation(*x)).collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = vector
                    .par_iter()
                    .map(|x| crypto_hash_simulation(*x))
                    .collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_prime_factorization(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("prime_factorization");
    group.sample_size(50);

    for size in [1_000, 5_000, 10_000] {
        // Use larger numbers for more computation
        let vector: PersistentVector<u64> = (1_000_000..1_000_000 + size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<Vec<u64>> =
                    vector.iter().map(|x| prime_factorization(*x)).collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<Vec<u64>> =
                    vector.par_iter().map(|x| prime_factorization(*x)).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_matrix_operations(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("matrix_operations");
    group.sample_size(50);

    for size in [5_000, 10_000, 20_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<[[f64; 4]; 4]> =
                    vector.iter().map(|x| matrix_multiply_4x4(*x)).collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<[[f64; 4]; 4]> =
                    vector.par_iter().map(|x| matrix_multiply_4x4(*x)).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_pattern_matching(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("pattern_matching");
    group.sample_size(50);

    for size in [10_000, 50_000, 100_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: u32 = vector.iter().map(|x| pattern_matching_simulation(*x)).sum();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: u32 = vector
                    .par_iter()
                    .map(|x| pattern_matching_simulation(*x))
                    .sum();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_monte_carlo(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("monte_carlo");
    group.sample_size(30);

    for size in [1_000, 5_000, 10_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: f64 = vector
                    .iter()
                    .map(|x| monte_carlo_pi_estimation(*x, 1000))
                    .sum::<f64>()
                    / size as f64;
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: f64 = vector
                    .par_iter()
                    .map(|x| monte_carlo_pi_estimation(*x, 1000))
                    .sum::<f64>()
                    / size as f64;
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashMap Benchmarks - CPU Intensive
// =============================================================================

fn benchmark_hashmap_crypto(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashmap_crypto");
    group.sample_size(50);

    for size in [5_000, 10_000, 20_000] {
        let map: PersistentHashMap<i64, i64> = (0..size).map(|i| (i, i * 7 + 13)).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = map
                    .iter()
                    .map(|(k, v)| crypto_hash_simulation(*k ^ *v))
                    .collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = map
                    .par_iter()
                    .map(|(k, v)| crypto_hash_simulation(k ^ v))
                    .collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_hashmap_fibonacci(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashmap_fibonacci");
    group.sample_size(50);

    for size in [1_000, 5_000, 10_000] {
        let map: PersistentHashMap<u64, u64> = (0..size).map(|i| (i, i % 80 + 10)).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = map.iter().map(|(_, v)| fibonacci_with_work(*v)).collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = map
                    .par_iter()
                    .map(|(_, v)| fibonacci_with_work(*v))
                    .collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashSet Benchmarks - CPU Intensive
// =============================================================================

fn benchmark_hashset_crypto(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashset_crypto");
    group.sample_size(50);

    for size in [5_000, 10_000, 20_000] {
        let set: PersistentHashSet<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = set.iter().map(|x| crypto_hash_simulation(*x)).collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<u64> = set.par_iter().map(|x| crypto_hash_simulation(*x)).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Filter with Heavy Predicate
// =============================================================================

fn benchmark_filter_heavy_predicate(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("filter_heavy_predicate");
    group.sample_size(50);

    for size in [10_000, 50_000, 100_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Filter with expensive predicate (check if crypto hash is even)
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector
                    .iter()
                    .filter(|x| crypto_hash_simulation(**x).is_multiple_of(2))
                    .copied()
                    .collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector
                    .par_iter()
                    .filter(|x| crypto_hash_simulation(**x).is_multiple_of(2))
                    .copied()
                    .collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Reduce with Heavy Computation
// =============================================================================

fn benchmark_reduce_heavy(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reduce_heavy");
    group.sample_size(50);

    for size in [5_000, 10_000, 20_000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: u64 = vector
                    .iter()
                    .map(|x| crypto_hash_simulation(*x))
                    .fold(0u64, |acc, x| acc.wrapping_add(x));
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: u64 = vector
                    .par_iter()
                    .map(|x| crypto_hash_simulation(*x))
                    .reduce(|| 0u64, |acc, x| acc.wrapping_add(x));
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    // Vector - CPU intensive operations
    benchmark_crypto_hash,
    benchmark_prime_factorization,
    benchmark_matrix_operations,
    benchmark_pattern_matching,
    benchmark_monte_carlo,
    // HashMap - CPU intensive operations
    benchmark_hashmap_crypto,
    benchmark_hashmap_fibonacci,
    // HashSet - CPU intensive operations
    benchmark_hashset_crypto,
    // Filter/Reduce with heavy predicates
    benchmark_filter_heavy_predicate,
    benchmark_reduce_heavy,
);

criterion_main!(benches);
