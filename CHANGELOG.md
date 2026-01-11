# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Breaking Changes

- **typeclass**: Add `'static` bound to `Functor::fmap`, `Applicative::pure`/`map2`, and `Monad::flat_map` type parameters. This is required for IO monad implementation. External crates implementing these traits may need to update their implementations.

### Bug Fixes

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

- AsyncIO
- Sample application
- Basic APIs
