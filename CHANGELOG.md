# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Breaking Changes

- **typeclass**: Add `'static` bound to `Functor::fmap`, `Applicative::pure`/`map2`, and `Monad::flat_map` type parameters. This is required for IO monad implementation. External crates implementing these traits may need to update their implementations.

### Deprecated

- **effect/async_io**: `AsyncIO::run_async()` is now deprecated in favor of direct `await`. See [Migration Guide](#asyncio-run_async-migration-guide) below.

### Migration Guides

#### AsyncIO run_async Migration Guide

`AsyncIO::run_async()` is deprecated since version 0.2.0. Use direct `await` instead for better performance (avoids unnecessary `Box::pin` heap allocation).

##### In async context

```rust
// Before (deprecated)
let result = AsyncIO::pure(42).run_async().await;

// After (recommended)
let result = AsyncIO::pure(42).await;
```

##### In sync context

```rust
use lambars::effect::async_io::runtime;

// Before (deprecated)
let result = runtime::run_blocking(AsyncIO::pure(42).run_async());

// After (recommended) - AsyncIO implements Future, so it can be passed directly
let result = runtime::run_blocking(AsyncIO::pure(42));
```

##### Suppressing the warning

If you need to suppress this warning temporarily during migration:

```rust
#[allow(deprecated)]
let result = AsyncIO::pure(42).run_async().await;
```

For projects using `deny(warnings)`, add `#[allow(deprecated)]` to the specific call site or module during the migration period.

### Bug Fixes

- **benches**: Fix HTTP status collection not reflected in meta.json
- **benches**: Fix thread-local status aggregation in result_collector
- Documents directory
- Clippy
- Tests
- Clippy
- For macro performance
- Ci
- Test
- Ci applys all branch
- Persistence performance
- README
- Running test os only ubuntu
- Ci
- README
- Clippy

### Features

- **benches**: Add lua_metrics.json generation and phase merging pipeline
- **benches**: Add HTTP status distribution to summary.txt
- **benches**: Add raw_wrk.txt for done handler output preservation
- **benches**: Add test_http_status_pipeline.sh for pipeline validation
- AsyncIO
- Sample application
- Basic APIs
