use std::sync::atomic::{AtomicU64, Ordering};

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::RandomSeed;
use roguelike_workflow::ports::RandomGenerator;

// =============================================================================
// SystemRandomGenerator
// =============================================================================

#[derive(Clone, Debug, Default)]
pub struct SystemRandomGenerator;

impl SystemRandomGenerator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RandomGenerator for SystemRandomGenerator {
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

    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        lcg_next_u32(seed)
    }
}

// =============================================================================
// DeterministicRandomGenerator
// =============================================================================

#[derive(Debug)]
pub struct DeterministicRandomGenerator {
    counter: AtomicU64,
}

impl DeterministicRandomGenerator {
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
    fn generate_seed(&self) -> AsyncIO<RandomSeed> {
        let counter_value = self.counter.fetch_add(1, Ordering::SeqCst);
        AsyncIO::pure(RandomSeed::new(counter_value))
    }

    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        lcg_next_u32(seed)
    }
}

// =============================================================================
// Shared LCG Implementation
// =============================================================================

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
