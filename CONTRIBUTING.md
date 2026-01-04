# Contributing to lambars

Thank you for your interest in contributing to lambars! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.92.0 or later (nightly)
- Git

### Setting Up the Development Environment

1. Fork and clone the repository:

```bash
git clone https://github.com/your-username/lambars.git
cd lambars
```

2. Build the project:

```bash
cargo build
```

3. Run tests:

```bash
cargo test
```

## Development Workflow

### Creating a Branch

Create a branch from `main` for your changes:

```bash
git checkout -b feature/your-feature-name
```

### Making Changes

1. Write your code following the coding guidelines below
2. Add tests for new functionality
3. Ensure all tests pass

### Pre-Commit Checklist

Before committing, run the following commands:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all-features --all-targets -- -D warnings

# Test without features
cargo test --no-default-features

# Test with all features
cargo test --all-features

# Build documentation
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

## Coding Guidelines

### Naming Conventions

- Use full, descriptive names instead of abbreviations
- Acceptable abbreviations: `URL`, `UUID`, `ULID`, `IO`, `API`
- Avoid abbreviations like: `req` (use `request`), `res` (use `response`), `cfg` (use `config`)

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- No `unsafe` code (enforced by `#![forbid(unsafe_code)]`)
- Aim for 100% test coverage

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

**Types:**

| Type       | Description            |
| ---------- | ---------------------- |
| `feat`     | New feature            |
| `fix`      | Bug fix                |
| `docs`     | Documentation          |
| `refactor` | Code refactoring       |
| `perf`     | Performance improvement|
| `test`     | Adding/fixing tests    |
| `chore`    | Miscellaneous tasks    |
| `ci`       | CI/CD changes          |
| `deps`     | Dependency updates     |

**Scopes:**

| Scope        |
| ------------ |
| `typeclass`  |
| `compose`    |
| `control`    |
| `persistent` |
| `optics`     |
| `effect`     |
| `derive`     |

**Examples:**

```bash
feat(typeclass): add Bifunctor trait
fix(persistent): resolve memory leak in Vector
docs: update README examples
```

## Submitting Changes

### Pull Request Process

1. Push your branch to your fork
2. Create a pull request against `main`
3. Fill out the PR template completely
4. Wait for CI checks to pass
5. Address any review feedback

### Pull Request Guidelines

- Keep PRs focused and reasonably sized
- Link related issues using "Fixes #123" or "Closes #123"
- Update documentation if needed
- Add tests for new functionality

## Reporting Issues

### Bug Reports

Use the bug report template and include:

- Clear description of the bug
- Steps to reproduce
- Expected vs actual behavior
- Minimal reproducible example
- Rust and lambars versions

### Feature Requests

Use the feature request template and include:

- Problem statement
- Proposed solution
- Proposed API (if applicable)
- Related module

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive community.

## Questions?

If you have questions, feel free to:

- Open a discussion on GitHub
- Ask in an issue

Thank you for contributing!
