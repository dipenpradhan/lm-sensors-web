# Testing Guide — lm-sensors-web

Comprehensive guide for running, writing, and understanding tests.

---

## Table of Contents

- [Test Suite Overview](#test-suite-overview)
- [Running Tests](#running-tests)
- [Test Structure](#test-structure)
- [Unit Tests](#unit-tests)
- [Integration Tests](#integration-tests)
- [Test Coverage](#test-coverage)
- [Writing Tests](#writing-tests)
- [CI/CD](#cicd)

---

## Test Suite Overview

| Category | Location | Tests | Coverage |
|----------|----------|-------|----------|
| **Unit tests** | `src/*.rs` (inline) | 30+ | Config, CLI, sensors, webhooks, WebSocket |
| **Integration tests** | `tests/` | 50+ | API, config, CLI, webhooks, WebSocket, sensor data |

### Test files

```
src/
  cli.rs          — CLI parsing tests (inline)
  config.rs       — Config loading/defaults tests (inline)
  sensors.rs      — Data model serialization tests (inline)
  service.rs      — Systemd unit generation tests (inline)
  webhook.rs      — Trigger logic tests (inline)
  websocket.rs    — Broadcast channel tests (inline)

tests/
  api_test.rs         — Legacy API model tests
  api_endpoints.rs    — HTTP endpoint response schema tests
  cli_integration.rs  — CLI argument parsing tests
  config_integration.rs — Config loading/validation tests
  sensor_data.rs      — Sensor data model tests
  webhook_engine.rs   — Webhook trigger/condition tests
  webhook_test.rs     — Legacy webhook model tests
  ws_test.rs          — Legacy WebSocket broadcast tests
  websocket_integration.rs — WebSocket broadcast tests
```

---

## Running Tests

### All tests

```bash
cargo test
```

### Unit tests only (inline `#[test]` functions)

```bash
cargo test --lib
```

### Integration tests only

```bash
cargo test --test '*'
```

### Specific test file

```bash
cargo test --test api_endpoints
cargo test --test config_integration
cargo test --test webhook_engine
```

### Specific test function

```bash
cargo test test_health_response_schema
cargo test test_broadcast_multiple_subscribers
```

### Verbose output

```bash
cargo test -- --nocapture
cargo test -- --verbose
```

### With debug logging

```bash
RUST_LOG=debug cargo test
```

### Test coverage (requires `grcov`)

```bash
# Install grcov
cargo install grcov

# Run tests with coverage
RUSTFLAGS="-Zprofile" RUSTDOCFLAGS="-Zprofile" cargo test
grcov . -s . -t html --branch --ignore-not-implemented \
    --ignore "tests/*" --ignore "/*" -o target/coverage/
```

---

## Test Structure

### Unit test pattern

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_function_name() {
        // Arrange: set up test data
        let input = ...;

        // Act: call the function under test
        let result = function_under_test(&input);

        // Assert: verify the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}
```

### Integration test pattern

```rust
use lm_sensors_web::config::Config;

#[test]
fn test_config_default_values() {
    // Arrange: nothing needed
    // Act
    let config = Config::default();
    // Assert
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 47890);
}
```

### Async test pattern

```rust
#[tokio::test]
async fn test_broadcast_send_recv() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();
    tx.send("hello".into()).unwrap();
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        rx.recv()
    ).await;
    assert_eq!(result.unwrap().unwrap(), "hello");
}
```

---

## Unit Tests

### Modules with inline tests

| Module | Tests | What's tested |
|--------|-------|---------------|
| `cli.rs` | 15+ | Argument parsing, subcommands, flags |
| `config.rs` | 8 | JSON loading, defaults, serialization |
| `sensors.rs` | 12 | Data model serialization, round-trips |
| `service.rs` | 4 | Unit file generation, paths |
| `webhook.rs` | 6 | Trigger logic, temperature checks |
| `websocket.rs` | 4 | Broadcast channel, subscribers |

### Key unit test categories

#### Config tests
- Default values for all config structs
- JSON loading from file
- Partial config (missing keys use defaults)
- Invalid JSON handling
- Serde round-trip

#### CLI tests
- Default argument values
- Short and long flag variants
- Subcommand parsing
- Combined flags
- Edge cases (IPv6, port 0, port 65535)

#### Sensor data tests
- Empty readings serialization
- Multiple devices/features serialization
- None value handling
- Round-trip serialization
- Debug formatting

---

## Integration Tests

### Test files

#### `tests/api_endpoints.rs`

Tests HTTP response schemas:
- Health endpoint response structure
- Device listing response format
- Device detail response format
- Device features response format
- Error response format
- WebSocket payload format

#### `tests/config_integration.rs`

Tests config loading:
- Minimal JSON (all defaults)
- Partial overrides
- Full overrides
- Invalid JSON error handling
- Non-existent file error handling
- Webhook config loading
- Serde round-trips

#### `tests/cli_integration.rs`

Tests CLI parsing:
- Default values
- Host/port overrides
- Log level variants
- Config path flag
- All subcommands
- Combined flags
- Edge cases

#### `tests/webhook_engine.rs`

Tests webhook triggers:
- Always trigger
- Temperature trigger (above/below)
- On-change trigger
- Payload structure
- Header handling
- Serialization round-trips

#### `tests/sensor_data.rs`

Tests data model:
- Empty readings
- Single/multiple devices
- Mixed feature types
- None values
- Special characters in paths
- Round-trip serialization
- Trait implementations

#### `tests/websocket_integration.rs`

Tests broadcast:
- Single subscriber
- Multiple subscribers
- Slow subscriber (lag)
- Channel closure
- Late subscriber behavior
- Concurrent sends
- Large payloads

---

## Test Coverage

### Current coverage targets

| Module | Target | Status |
|--------|--------|--------|
| `config.rs` | 90%+ | ✅ |
| `cli.rs` | 85%+ | ✅ |
| `sensors.rs` (data) | 95%+ | ✅ |
| `service.rs` | 70%+ | ✅ |
| `webhook.rs` | 80%+ | ✅ |
| `websocket.rs` | 75%+ | ✅ |
| `server.rs` | 60%+ | ⚠️ (hard to test without real server) |
| `api/*.rs` | 60%+ | ⚠️ (requires Axum test harness) |
| `main.rs` | 20% | ⚠️ (orchestration, tested by integration tests) |

### What's not fully tested

- **SensorManager**: Requires live `libsensors` environment
- **Server routes**: Require Axum `TestClient` setup
- **WebSocket upgrade**: Requires HTTP upgrade handshake
- **Signal handling**: SIGTERM/SIGINT handling
- **File I/O edge cases**: Race conditions, permissions

---

## Writing Tests

### Best practices

1. **Descriptive names**: `test_broadcast_multiple_subscribers` not `test_ws`
2. **One assertion per concept**: Group related assertions, don't mix unrelated checks
3. **Document why**: Add a doc comment explaining what the test verifies
4. **Use fixtures**: Build test data with helper functions, not inline
5. **Isolate tests**: Each test should be independent, no shared state

### Example: Testing a new config field

```rust
#[test]
fn test_new_field_default() {
    let config = Config::default();
    // New field should have sensible default
    assert_eq!(config.server.max_connections, 1000);
}

#[test]
fn test_new_field_override() {
    let json = r#"{"server":{"max_connections":500},"websocket":{},"webhooks":[],"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let config = Config::load(f.path()).unwrap();
    assert_eq!(config.server.max_connections, 500);
}
```

### Example: Testing a new webhook trigger

```rust
#[test]
fn test_new_trigger_fires() {
    let readings = SensorReadings { ... };
    let fires = should_fire(&webhook, &readings, &last);
    assert!(fires);
}

#[test]
fn test_new_trigger_no_fire() {
    // Condition not met
    assert!(!fires);
}
```

---

## CI/CD

### GitHub Actions (test.yaml)

```yaml
name: Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-lang@v1
        with:
          rust-version: stable

      - name: Install libsensors
        run: |
          sudo apt update
          sudo apt install -y lm-sensors libsensors-dev

      - name: Run tests
        run: cargo test --all-targets

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt --check
```

### Pre-commit hooks

```bash
# .husky/pre-commit
cargo fmt
cargo clippy -- -D warnings
cargo test --all-targets
```

---

## Test Troubleshooting

### "No such file" errors in tests

Temp files from `tempfile` crate should auto-cleanup. If files accumulate:
- Check test runner is completing (not hanging)
- Use `NamedTempFile` instead of manual file creation

### Race conditions in async tests

- Use `tokio::time::timeout` for all async operations
- Don't rely on timing (use channels/semaphores for synchronization)
- Use `tokio::sync::broadcast` for reliable message delivery

### Intermittent test failures

- Check for shared mutable state (use `Arc` + `RwLock` or avoid sharing)
- Ensure each test creates its own fixtures
- Use `tempfile` for unique temp paths