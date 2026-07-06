# Contributing to lm-sensors-web

Thank you for your interest in contributing! This document provides guidelines
for contributing to the project.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Requests](#pull-requests)
- [Documentation](#documentation)
- [Release Process](#release-process)

---

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
Be respectful, constructive, and assume good faith.

---

## Getting Started

### Prerequisites

- **Rust** 1.70+ (via [rustup](https://rustup.rs/))
- **libsensors** (lm-sensors) installed on your system
- **Git**

### Setup

```bash
# Clone
git clone https://github.com/your-org/lm-sensors-web.git
cd lm-sensors-web

# Install dependencies
sudo apt install lm-sensors libsensors-dev  # Debian/Ubuntu
# or
sudo dnf install lm_sensors lm_sensors-devel  # RHEL/Fedora

# Build
cargo build

# Run tests
cargo test

# Run
cargo run -- --help
```

---

## Development Workflow

### Branch strategy

- `main` — stable, production-ready code
- `develop` — integration branch for upcoming features
- `feature/*` — new features
- `fix/*` — bug fixes
- `docs/*` — documentation changes

### Local development

```bash
# Create feature branch
git checkout -b feature/my-feature

# Make changes
# ...

# Run tests
cargo test

# Check formatting and linting
cargo fmt
cargo clippy -- -D warnings

# Commit and push
git add -A
git commit -m "feat: add new feature"
git push origin feature/my-feature

# Open PR on GitHub
```

---

## Coding Standards

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt
```

The CI will reject unformatted code.

### Linting

All code must pass `clippy` with no warnings:

```bash
cargo clippy -- -D warnings
```

### Conventions

1. **Module-level doc comments**: Every file starts with `//! # Module name` and a description
2. **Public items**: Documented with `///` doc comments
3. **Function behavior**: Document return values, errors, and side effects
4. **Error handling**: Use `Result<T, E>` — avoid `unwrap()` outside tests
5. **Naming**: Follow Rust conventions (snake_case for functions/vars, CamelCase for types)
6. **Line length**: Prefer 100 characters or less
7. **Comments**: Explain *why*, not *what*

### Example

```rust
/// Read all sensor data from all devices.
///
/// Primary method used by the WebSocket broadcast loop.
/// Returns a complete snapshot of all devices, features, and readings.
pub fn read_all(&self) -> SensorReadings {
    // Implementation
}
```

---

## Testing

### Requirements

- Every new feature must have tests
- Every bug fix must include a regression test
- Tests must pass before merging

### Running tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Specific test file
cargo test --test api_endpoints

# With verbose output
cargo test -- --nocapture
```

### Test types

| Type | Location | Purpose |
|------|----------|---------|
| Unit tests | `src/*.rs` (inline) | Test individual functions |
| Integration tests | `tests/` | Test cross-module behavior |

### Coverage

Aim for 80%+ line coverage. Check coverage with:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

---

## Pull Requests

### Checklist

Before submitting a PR, ensure:

- [ ] Code is formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] All tests pass: `cargo test`
- [ ] Documentation updated (if applicable)
- [ ] Changelog entry added (for user-visible changes)
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)

### Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

feat(api): add device filtering endpoint
fix(webhook): handle temperature threshold edge case
docs(README): update installation instructions
test(config): add partial config loading tests
chore(ci): update GitHub Actions runners
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### PR template

When opening a PR, include:

1. **Description**: What does this PR do?
2. **Motivation**: Why is this change needed?
3. **Testing**: What tests were added/changed?
4. **Breaking changes**: Any API changes?

---

## Documentation

### What to document

- New public APIs (endpoints, config options, CLI flags)
- Behavior changes
- New dependencies
- Deployment changes

### Where to document

| Change | Document |
|--------|----------|
| New feature | `docs/USER_GUIDE.md` + README |
| API change | `docs/API_REFERENCE.md` |
| Deployment option | `docs/DEPLOYMENT.md` |
| Common issue | `docs/TROUBLESHOOTING.md` |
| Testing change | `docs/TESTING.md` |
| Bug fix | `CHANGELOG.md` |

---

## Release Process

### Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

### Creating a release

```bash
# Update changelog
# Update version in Cargo.toml
# Commit
git add -A
git commit -m "chore: bump version to 0.2.0"

# Tag and push
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin main --tags

# GitHub Actions will build binaries, Docker images, and publish
```

---

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: Open a GitHub Issue
- **Security**: See [SECURITY.md](SECURITY.md)
- **Chat**: Join our community (Discord/Matrix link)