# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Comprehensive documentation suite (User Guide, API Reference, Deployment, Troubleshooting, Testing)
- Enhanced CI/CD workflows with linting, unit tests, integration tests, coverage, Docker builds, and security audits
- Expanded unit tests across all source modules (CLI, config, sensors, webhooks, WebSocket)
- New integration test files for API endpoints, config, CLI, sensor data, webhook engine, and WebSocket
- Production-ready `config.example.json` with all webhook trigger types demonstrated
- Release workflow with multi-platform binary builds and Docker image publishing

### Improved
- Better error handling in `main.rs` startup sequence
- Enhanced unit tests in `sensors.rs` with serialization, round-trip, and edge case coverage
- Enhanced CLI tests with combined flags, edge cases (IPv6, port 0/65535)
- Improved webhook trigger test coverage (above/below thresholds, on-change logic)
- WebSocket broadcast tests for concurrent sends, late subscribers, and channel closure

### Documentation
- Added `docs/USER_GUIDE.md` — complete user guide with all features
- Added `docs/API_REFERENCE.md` — full endpoint documentation with schemas
- Added `docs/DEPLOYMENT.md` — deployment strategies (systemd, Docker, K8s, reverse proxy)
- Added `docs/TROUBLESHOOTING.md` — troubleshooting guide with diagnostics
- Added `docs/TESTING.md` — testing guide with best practices
- Updated `README.md` — cleaner summary with doc links

---

## [0.1.0] - 2024-01-01

### Added
- Initial release of lm-sensors-web
- REST API for sensor device listing, details, and readings
- WebSocket live feed for real-time sensor broadcast
- Webhook engine with always/temperature/on-change triggers
- Dark-mode web dashboard with real-time filtering
- CLI with host/port/log-level/config flags and service management subcommands
- Systemd service install/uninstall/start/stop/restart/status
- Docker multi-stage build with health checks
- Config hot-reload via `POST /api/reload`
- CORS middleware for cross-origin dashboard access
- Graceful shutdown on SIGTERM/SIGINT
- Unit tests for core modules
- Integration tests for API models, WebSocket, and webhooks