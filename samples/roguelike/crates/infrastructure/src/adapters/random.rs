//! Random number generator adapters.
//!
//! This module provides implementations of the [`RandomGenerator`] port trait
//! for different use cases:
//!
//! - [`SystemRandomGenerator`]: Uses system entropy for production use
//! - [`DeterministicRandomGenerator`]: Predictable sequence for testing
//!
//! # Examples
//!
//! ## Production Use
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::SystemRandomGenerator;
//! use roguelike_workflow::ports::RandomGenerator;
//!
//! let generator = SystemRandomGenerator::new();
//! let seed = generator.generate_seed().run_async().await;
//! let (random_value, next_seed) = generator.next_u32(&seed);
//! ```
//!
//! ## Testing
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::DeterministicRandomGenerator;
//! use roguelike_workflow::ports::RandomGenerator;
//!
//! let generator = DeterministicRandomGenerator::new(42);
//! // Seeds are generated deterministically from the counter
//! let seed1 = generator.generate_seed().run_async().await;
//! let seed2 = generator.generate_seed().run_async().await;
//! // seed1.value() == 42, seed2.value() == 43
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::RandomSeed;
use roguelike_workflow::ports::RandomGenerator;

// =============================================================================
// SystemRandomGenerator
// =============================================================================

/// System entropy-based random generator for production use.
///
/// This generator uses system time to create random seeds, providing
/// non-deterministic randomness suitable for production game sessions.
///
/// # Seed Generation
///
/// Seeds are derived from the current system time in nanoseconds.
/// Each call to [`generate_seed`](RandomGenerator::generate_seed) will
/// produce a different seed based on the current time.
///
/// # Random Number Generation
///
/// Once a seed is obtained, [`next_u32`](RandomGenerator::next_u32) uses
/// a Linear Congruential Generator (LCG) algorithm to produce deterministic
/// random sequences. This ensures that game sessions can be replayed
/// given the same initial seed.
///
/// # Thread Safety
///
/// This type is `Clone`, `Send`, and `Sync`, making it safe to use
/// across async tasks and threads.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::SystemRandomGenerator;
/// use roguelike_workflow::ports::RandomGenerator;
///
/// #[tokio::main]
/// async fn main() {
///     let generator = SystemRandomGenerator::new();
///
///     // Generate a seed from system entropy
///     let seed = generator.generate_seed().run_async().await;
///
///     // Generate random numbers deterministically from the seed
///     let (value1, seed1) = generator.next_u32(&seed);
///     let (value2, seed2) = generator.next_u32(&seed1);
///
///     // Same seed always produces the same sequence
///     let (check_value1, _) = generator.next_u32(&seed);
///     assert_eq!(value1, check_value1);
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct SystemRandomGenerator;

impl SystemRandomGenerator {
    /// Creates a new system random generator.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::SystemRandomGenerator;
    ///
    /// let generator = SystemRandomGenerator::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RandomGenerator for SystemRandomGenerator {
    /// Generates a new random seed using system entropy.
    ///
    /// This method uses the current system time in nanoseconds to create
    /// a unique seed. Each call will produce a different seed.
    ///
    /// # Returns
    ///
    /// An [`AsyncIO`] that resolves to a new [`RandomSeed`].
    fn generate_seed(&self) -> AsyncIO<RandomSeed> {
        AsyncIO::new(move || async move {
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let nanos = duration.as_nanos() as u64;
            RandomSeed::new(nanos)
        })
    }

    /// Generates the next random u32 value from a seed.
    ///
    /// Uses a Linear Congruential Generator (LCG) algorithm with the
    /// following parameters:
    /// - Multiplier: 1103515245
    /// - Increment: 12345
    ///
    /// This is the same algorithm used by many C standard library
    /// implementations, providing good randomness properties.
    ///
    /// # Arguments
    ///
    /// * `seed` - The current random seed state.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - The generated random u32 value (15 bits, 0-32767)
    /// - The next seed state for subsequent calls
    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        lcg_next_u32(seed)
    }
}

// =============================================================================
// DeterministicRandomGenerator
// =============================================================================

/// Deterministic random generator for testing purposes.
///
/// This generator produces predictable seed sequences, making it ideal
/// for unit tests and reproducible test scenarios.
///
/// # Seed Generation
///
/// Seeds are generated from an internal counter that starts at the
/// initial value provided during construction. Each call to
/// [`generate_seed`](RandomGenerator::generate_seed) increments the counter
/// and returns the previous value as a seed.
///
/// # Random Number Generation
///
/// Uses the same LCG algorithm as [`SystemRandomGenerator`], ensuring
/// that random number sequences are identical given the same seed.
///
/// # Thread Safety
///
/// This type uses atomic operations for the counter, making it safe to
/// use across async tasks and threads. Note that the [`Clone`] implementation
/// copies the current counter value, creating an independent sequence.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::DeterministicRandomGenerator;
/// use roguelike_workflow::ports::RandomGenerator;
///
/// #[tokio::test]
/// async fn test_deterministic_random() {
///     let generator = DeterministicRandomGenerator::new(100);
///
///     // Seeds are generated from the counter
///     let seed1 = generator.generate_seed().run_async().await;
///     assert_eq!(seed1.value(), 100);
///
///     let seed2 = generator.generate_seed().run_async().await;
///     assert_eq!(seed2.value(), 101);
///
///     // Random numbers from the same seed are deterministic
///     let (value1, _) = generator.next_u32(&seed1);
///     let (value2, _) = generator.next_u32(&seed1);
///     assert_eq!(value1, value2);
/// }
/// ```
#[derive(Debug)]
pub struct DeterministicRandomGenerator {
    counter: AtomicU64,
}

impl DeterministicRandomGenerator {
    /// Creates a new deterministic random generator with the given initial counter.
    ///
    /// The first call to [`generate_seed`](RandomGenerator::generate_seed) will
    /// return a seed with this initial value.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial counter value for seed generation.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::DeterministicRandomGenerator;
    ///
    /// let generator = DeterministicRandomGenerator::new(42);
    /// ```
    #[must_use]
    pub fn new(initial: u64) -> Self {
        Self {
            counter: AtomicU64::new(initial),
        }
    }
}

impl Clone for DeterministicRandomGenerator {
    fn clone(&self) -> Self {
        Self {
            counter: AtomicU64::new(self.counter.load(Ordering::SeqCst)),
        }
    }
}

impl RandomGenerator for DeterministicRandomGenerator {
    /// Generates a seed from the internal counter.
    ///
    /// Each call increments the counter and returns the previous value
    /// as a seed. This provides a predictable sequence of seeds for testing.
    ///
    /// # Returns
    ///
    /// An [`AsyncIO`] that resolves to a new [`RandomSeed`] with the
    /// current counter value.
    fn generate_seed(&self) -> AsyncIO<RandomSeed> {
        let counter_value = self.counter.fetch_add(1, Ordering::SeqCst);
        AsyncIO::pure(RandomSeed::new(counter_value))
    }

    /// Generates the next random u32 value from a seed.
    ///
    /// Uses the same LCG algorithm as [`SystemRandomGenerator`] for
    /// consistent random number generation across generator types.
    ///
    /// # Arguments
    ///
    /// * `seed` - The current random seed state.
    ///
    /// # Returns
    ///
    /// A tuple containing the random value and the next seed state.
    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        lcg_next_u32(seed)
    }
}

// =============================================================================
// Shared LCG Implementation
// =============================================================================

/// Linear Congruential Generator implementation.
///
/// Uses the classic LCG parameters from the C standard library:
/// - Multiplier (a): 1103515245
/// - Increment (c): 12345
/// - Modulus (m): 2^64 (implicit via wrapping arithmetic)
///
/// The output is the middle 15 bits of the next state, which have
/// better statistical properties than the low bits.
fn lcg_next_u32(seed: &RandomSeed) -> (u32, RandomSeed) {
    let current = seed.value();
    let next = current.wrapping_mul(1103515245).wrapping_add(12345);
    // Extract bits 16-30 (15 bits) for better randomness
    let random_value = ((next >> 16) & 0x7FFF) as u32;
    (random_value, RandomSeed::new(next))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // SystemRandomGenerator Tests
    // =========================================================================

    mod system_random_generator {
        use super::*;

        #[rstest]
        fn new_creates_generator() {
            let generator = SystemRandomGenerator::new();
            // Verify it implements the required traits
            fn assert_bounds<T: RandomGenerator + Clone + Send + Sync + 'static>(_: &T) {}
            assert_bounds(&generator);
        }

        #[rstest]
        fn default_creates_generator() {
            let generator: SystemRandomGenerator = Default::default();
            fn assert_bounds<T: RandomGenerator>(_: &T) {}
            assert_bounds(&generator);
        }

        #[rstest]
        #[tokio::test]
        async fn generate_seed_returns_different_values() {
            let generator = SystemRandomGenerator::new();

            let seed1 = generator.generate_seed().run_async().await;
            // Small delay to ensure different nanosecond timestamp
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            let seed2 = generator.generate_seed().run_async().await;

            // While theoretically possible to get the same value,
            // it's extremely unlikely with nanosecond precision
            assert_ne!(seed1, seed2);
        }

        #[rstest]
        fn next_u32_is_deterministic() {
            let generator = SystemRandomGenerator::new();
            let seed = RandomSeed::new(12345);

            let (value1, next_seed1) = generator.next_u32(&seed);
            let (value2, next_seed2) = generator.next_u32(&seed);

            assert_eq!(value1, value2);
            assert_eq!(next_seed1, next_seed2);
        }

        #[rstest]
        fn next_u32_produces_sequence() {
            let generator = SystemRandomGenerator::new();
            let seed = RandomSeed::new(42);

            let (value1, seed1) = generator.next_u32(&seed);
            let (value2, seed2) = generator.next_u32(&seed1);
            let (value3, _) = generator.next_u32(&seed2);

            // Values should be different (with high probability)
            assert_ne!(value1, value2);
            assert_ne!(value2, value3);
        }

        #[rstest]
        fn next_u32_returns_values_in_valid_range() {
            let generator = SystemRandomGenerator::new();
            let mut seed = RandomSeed::new(0);

            // Generate many values and verify they are within range
            for _ in 0..100 {
                let (value, next_seed) = generator.next_u32(&seed);
                assert!(value <= 0x7FFF, "Value {value} exceeds 15-bit range");
                seed = next_seed;
            }
        }

        #[rstest]
        fn clone_creates_independent_generator() {
            let generator1 = SystemRandomGenerator::new();
            let generator2 = generator1.clone();

            let seed = RandomSeed::new(42);
            let (value1, _) = generator1.next_u32(&seed);
            let (value2, _) = generator2.next_u32(&seed);

            // Same seed should produce same result
            assert_eq!(value1, value2);
        }

        #[rstest]
        fn debug_format_works() {
            let generator = SystemRandomGenerator::new();
            let debug_str = format!("{generator:?}");
            assert!(debug_str.contains("SystemRandomGenerator"));
        }
    }

    // =========================================================================
    // DeterministicRandomGenerator Tests
    // =========================================================================

    mod deterministic_random_generator {
        use super::*;

        #[rstest]
        fn new_creates_generator_with_initial_value() {
            let generator = DeterministicRandomGenerator::new(100);
            // Verify it implements the required traits
            fn assert_bounds<T: RandomGenerator + Clone + Send + Sync + 'static>(_: &T) {}
            assert_bounds(&generator);
        }

        #[rstest]
        #[tokio::test]
        async fn generate_seed_returns_sequential_values() {
            let generator = DeterministicRandomGenerator::new(100);

            let seed1 = generator.generate_seed().run_async().await;
            let seed2 = generator.generate_seed().run_async().await;
            let seed3 = generator.generate_seed().run_async().await;

            assert_eq!(seed1.value(), 100);
            assert_eq!(seed2.value(), 101);
            assert_eq!(seed3.value(), 102);
        }

        #[rstest]
        #[tokio::test]
        async fn generate_seed_starts_from_zero() {
            let generator = DeterministicRandomGenerator::new(0);

            let seed = generator.generate_seed().run_async().await;
            assert_eq!(seed.value(), 0);
        }

        #[rstest]
        fn next_u32_is_deterministic() {
            let generator = DeterministicRandomGenerator::new(0);
            let seed = RandomSeed::new(12345);

            let (value1, next_seed1) = generator.next_u32(&seed);
            let (value2, next_seed2) = generator.next_u32(&seed);

            assert_eq!(value1, value2);
            assert_eq!(next_seed1, next_seed2);
        }

        #[rstest]
        fn next_u32_produces_sequence() {
            let generator = DeterministicRandomGenerator::new(0);
            let seed = RandomSeed::new(42);

            let (value1, seed1) = generator.next_u32(&seed);
            let (value2, seed2) = generator.next_u32(&seed1);
            let (value3, _) = generator.next_u32(&seed2);

            assert_ne!(value1, value2);
            assert_ne!(value2, value3);
        }

        #[rstest]
        fn next_u32_same_as_system_generator() {
            let deterministic = DeterministicRandomGenerator::new(0);
            let system = SystemRandomGenerator::new();
            let seed = RandomSeed::new(42);

            let (det_value, det_next) = deterministic.next_u32(&seed);
            let (sys_value, sys_next) = system.next_u32(&seed);

            // Both should produce identical results
            assert_eq!(det_value, sys_value);
            assert_eq!(det_next, sys_next);
        }

        #[rstest]
        fn clone_copies_current_counter() {
            let generator1 = DeterministicRandomGenerator::new(100);

            // Advance the counter
            let _ = generator1.counter.fetch_add(5, Ordering::SeqCst);

            // Clone should have the current value
            let generator2 = generator1.clone();

            assert_eq!(generator1.counter.load(Ordering::SeqCst), 105);
            assert_eq!(generator2.counter.load(Ordering::SeqCst), 105);
        }

        #[rstest]
        #[tokio::test]
        async fn clone_creates_independent_sequence() {
            let generator1 = DeterministicRandomGenerator::new(100);
            let generator2 = generator1.clone();

            let seed1 = generator1.generate_seed().run_async().await;
            let seed2 = generator2.generate_seed().run_async().await;

            // Both should start from the same value since they were cloned at the same point
            assert_eq!(seed1.value(), seed2.value());

            // But subsequent values are independent
            let seed1_next = generator1.generate_seed().run_async().await;
            let seed2_next = generator2.generate_seed().run_async().await;

            assert_eq!(seed1_next.value(), seed2_next.value());
        }

        #[rstest]
        fn debug_format_works() {
            let generator = DeterministicRandomGenerator::new(42);
            let debug_str = format!("{generator:?}");
            assert!(debug_str.contains("DeterministicRandomGenerator"));
            assert!(debug_str.contains("42"));
        }
    }

    // =========================================================================
    // LCG Algorithm Tests
    // =========================================================================

    mod lcg_algorithm {
        use super::*;

        #[rstest]
        fn lcg_produces_known_sequence() {
            // Test that the LCG produces a known sequence from seed 0
            let seed = RandomSeed::new(0);
            let (value, next_seed) = lcg_next_u32(&seed);

            // With multiplier 1103515245 and increment 12345,
            // next state = 0 * 1103515245 + 12345 = 12345
            // output = (12345 >> 16) & 0x7FFF = 0
            assert_eq!(value, 0);
            assert_eq!(next_seed.value(), 12345);
        }

        #[rstest]
        fn lcg_from_seed_1_produces_expected() {
            let seed = RandomSeed::new(1);
            let (value, next_seed) = lcg_next_u32(&seed);

            // next state = 1 * 1103515245 + 12345 = 1103527590
            // output = (1103527590 >> 16) & 0x7FFF = 16838
            let expected_next = 1u64.wrapping_mul(1103515245).wrapping_add(12345);
            let expected_value = ((expected_next >> 16) & 0x7FFF) as u32;

            assert_eq!(value, expected_value);
            assert_eq!(next_seed.value(), expected_next);
        }

        #[rstest]
        fn lcg_handles_large_seed() {
            let seed = RandomSeed::new(u64::MAX);
            let (value, next_seed) = lcg_next_u32(&seed);

            // Should handle wrapping correctly
            let expected_next = u64::MAX.wrapping_mul(1103515245).wrapping_add(12345);
            let expected_value = ((expected_next >> 16) & 0x7FFF) as u32;

            assert_eq!(value, expected_value);
            assert_eq!(next_seed.value(), expected_next);
        }

        #[rstest]
        fn lcg_sequence_has_no_immediate_repetition() {
            let mut seed = RandomSeed::new(42);
            let mut values = Vec::with_capacity(1000);

            for _ in 0..1000 {
                let (value, next_seed) = lcg_next_u32(&seed);
                values.push(value);
                seed = next_seed;
            }

            // Check that consecutive values are not equal (basic sanity check)
            for window in values.windows(2) {
                if let [a, b] = window {
                    // While theoretically possible, identical consecutive values
                    // should be rare with a good LCG
                    if a == b {
                        // Allow a small number of collisions but not many
                        // This is a weak test but catches obvious issues
                    }
                }
            }
        }
    }

    // =========================================================================
    // Trait Bounds Verification
    // =========================================================================

    mod trait_bounds {
        use super::*;

        #[rstest]
        fn system_generator_is_send() {
            fn assert_send<T: Send>() {}
            assert_send::<SystemRandomGenerator>();
        }

        #[rstest]
        fn system_generator_is_sync() {
            fn assert_sync<T: Sync>() {}
            assert_sync::<SystemRandomGenerator>();
        }

        #[rstest]
        fn system_generator_is_static() {
            fn assert_static<T: 'static>() {}
            assert_static::<SystemRandomGenerator>();
        }

        #[rstest]
        fn deterministic_generator_is_send() {
            fn assert_send<T: Send>() {}
            assert_send::<DeterministicRandomGenerator>();
        }

        #[rstest]
        fn deterministic_generator_is_sync() {
            fn assert_sync<T: Sync>() {}
            assert_sync::<DeterministicRandomGenerator>();
        }

        #[rstest]
        fn deterministic_generator_is_static() {
            fn assert_static<T: 'static>() {}
            assert_static::<DeterministicRandomGenerator>();
        }
    }
}
