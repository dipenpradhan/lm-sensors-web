# User Guide — lm-sensors-web

A production-ready hardware sensor monitoring web application built in Rust.
Exposes real-time sensor data via REST API, WebSocket live-feed, webhooks,
and a dark-mode web dashboard.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CLI Reference](#cli-reference)
- [Web Dashboard](#web-dashboard)
- [REST API](#rest-api)
- [WebSocket Live Feed](#websocket-live-feed)
- [Webhooks](#webhooks)
- [Systemd Service](#systemd-service)
- [Docker](#docker)
- [Security](#security)
- [Performance](#performance)
- [Upgrading](#upgrading)

---

## Installation

### Prerequisites

- **Linux** with `libsensors` (lm-sensors) installed:
  ```bash
  # Debian/Ubuntu
  sudo apt install lm-sensors

  # RHEL/Fedora
  sudo dnf install lm_sensors
  ```
- **Rust toolchain** (1.70+):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Build from source

```bash
git clone https://github.com/your-org/lm-sensors-web.git
cd lm-sensors-web
cargo build --release
```

The binary is at `target/release/lm-sensors-web`.

### Install system-wide

```bash
sudo cp target/release/lm-sensors-web /usr/local/bin/
sudo lm-sensors-web install-service --binary /usr/local/bin/lm-sensors-web --config /etc/lm-sensors-web/config.json
```

---

## Quick Start

```bash
# Build
cargo build --release

# Run (binds to 0.0.0.0:47890)
./target/release/lm-sensors-web

# Open dashboard in browser
open http://localhost:47890
```

The server exposes:
- **Dashboard** at `http://localhost:47890`
- **REST API** at `http://localhost:47890/api/devices`
- **WebSocket** at `ws://localhost:47890/ws/sensors`
- **Health check** at `http://localhost:47890/api/health`

---

## Configuration

### Config file (`config.json`)

Create a `config.json` in the current directory or specify with `-c`:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 47890,
    "log_level": "info"
  },
  "websocket": {
    "enabled": true,
    "path": "/ws/sensors",
    "broadcast_interval_ms": 2000
  },
  "webhooks": [],
  "sensors": {
    "refresh_interval_ms": 5000
  }
}
```

### Configuration reference

| Setting | Default | Description |
|---------|---------|-------------|
| `server.host` | `0.0.0.0` | Bind address (`127.0.0.1` for local-only) |
| `server.port` | `47890` | Listen port |
| `server.log_level` | `info` | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `websocket.enabled` | `true` | Enable/disable WebSocket broadcast |
| `websocket.path` | `/ws/sensors` | WebSocket endpoint URL path |
| `websocket.broadcast_interval_ms` | `2000` | Milliseconds between broadcasts |
| `sensors.refresh_interval_ms` | `5000` | Milliseconds between sensor reads |
| `webhooks` | `[]` | Array of webhook definitions |

### Hot-reload configuration

Without restarting the server, reload config:

```bash
curl -X POST http://localhost:47890/api/reload
```

Or via the web dashboard settings panel.

---

## CLI Reference

### Main flags

```
lm-sensors-web [FLAGS] [SUBCOMMAND]

Flags:
  -H, --host <HOST>        Bind address (default: 0.0.0.0)
  -p, --port <PORT>        Listen port (default: 47890)
  -l, --log-level <LEVEL>  Logging level: trace|debug|info|warn|error
  -c, --config <PATH>      Path to config.json
  -h, --help               Print help
  -V, --version            Print version
```

### Examples

```bash
# Run on a custom port
lm-sensors-web -p 8080

# Run with custom config and verbose logging
lm-sensors-web -c /etc/lm-sensors-web/config.json --log-level debug

# Local-only binding
lm-sensors-web -H 127.0.0.1
```

### Subcommands

#### Service management

```bash
# Install as systemd service (system-wide)
lm-sensors-web install-service \
    --binary /usr/local/bin/lm-sensors-web \
    --config /etc/lm-sensors-web/config.json

# Install as user-level service
lm-sensors-web install-service --user --binary ~/.local/bin/lm-sensors-web

# Manage the service
lm-sensors-web start-service
lm-sensors-web stop-service
lm-sensors-web restart-service
lm-sensors-web status-service
lm-sensors-web uninstall-service
```

---

## Web Dashboard

The built-in web dashboard provides real-time sensor visualization:

### Features

- **Real-time updates** via WebSocket with automatic reconnection
- **Collapsible sensor cards** — click to expand/collapse device details
- **Live filtering** — type to filter devices by name
- **Dark mode** — pastel-colored sensor cards optimized for monitoring
- **REST fallback** — if WebSocket is unavailable, polls `/api/devices` every 30s

### Access

1. Start the server: `lm-sensors-web`
2. Open `http://localhost:47890` in your browser
3. Sensor cards appear automatically with live updates

### Dashboard URL patterns

| URL | Purpose |
|-----|---------|
| `/` | Dashboard (redirected to `/static/index.html`) |
| `/static/index.html` | Dashboard HTML |
| `/static/app.css` | Stylesheet |
| `/static/app.js` | Frontend JavaScript |

---

## REST API

### Endpoints

| Method | Path | Description | Status Codes |
|--------|------|-------------|-------------|
| `GET` | `/api/health` | Health check / liveness probe | `200` |
| `POST` | `/api/reload` | Hot-reload configuration | `200`, `500` |
| `GET` | `/api/devices` | List all sensor devices | `200` |
| `GET` | `/api/devices/{id}` | Get device by name (partial match) | `200`, `404` |
| `GET` | `/api/devices/{id}/features` | Get device with readings | `200`, `404` |

### Health check

```bash
$ curl http://localhost:47890/api/health
{"status":"ok","timestamp":"2024-01-15T10:30:00.000+00:00"}
```

### List devices

```bash
$ curl http://localhost:47890/api/devices
{"devices":[{"name":"coretemp","bus":"ISA","path":"/sys/class/hwmon/hwmon0"},...]}
```

### Get device details

```bash
$ curl http://localhost:47890/api/devices/coretemp
{"device":{"name":"coretemp-isa-0000","bus":"ISA","path":"/sys/class/hwmon/hwmon0"}}
```

### Get device readings

```bash
$ curl http://localhost:47890/api/devices/coretemp/features
{
  "device": {"name": "coretemp-isa-0000", "bus": "ISA", "path": "..."},
  "features": [
    {"name": "temp1", "sub_features": [
      {"name": "temp1_input", "value": 55.0, "unit": "°C"},
      {"name": "temp1_max", "value": 100.0, "unit": "°C"}
    ]}
  ]
}
```

### Reload config

```bash
$ curl -X POST http://localhost:47890/api/reload
{"status":"ok","message":"Config reloaded successfully"}
```

### API response schemas

#### `GET /api/devices`
```json
{
  "devices": [
    {
      "name": "coretemp",
      "bus": "ISA",
      "path": "/sys/class/hwmon/hwmon0"
    }
  ]
}
```

#### `GET /api/devices/{id}/features`
```json
{
  "device": {
    "name": "coretemp-isa-0000",
    "bus": "ISA",
    "path": "/sys/class/hwmon/hwmon0"
  },
  "features": [
    {
      "name": "temp1",
      "sub_features": [
        {"name": "temp1_input", "value": 55.0, "unit": "°C"}
      ]
    }
  ]
}
```

#### Error response
```json
{"error": "Device 'nonexistent' not found"}
```

---

## WebSocket Live Feed

### Endpoint

```
ws://localhost:47890/ws/sensors
```

### Protocol

1. Client connects via standard WebSocket upgrade
2. Server pushes full `SensorReadings` JSON on every broadcast tick
3. Configurable broadcast interval (default: 2000ms)
4. Slow clients receive `Lagged` errors (messages are dropped)

### Example: JavaScript client

```javascript
const ws = new WebSocket('ws://localhost:47890/ws/sensors');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Sensor readings:', data.devices);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Connection closed');
};
```

### Example: curl (with websocat)

```bash
# Install websocat
cargo install websocat

# Connect to the WebSocket feed
websocat ws://localhost:47890/ws/sensors
```

---

## Webhooks

Webhooks push sensor data to arbitrary HTTP endpoints with configurable triggers.

### Configuration

Add webhooks to `config.json`:

```json
{
  "webhooks": [
    {
      "name": "temp-alert",
      "url": "http://monitoring.example.com/alerts",
      "method": "POST",
      "content_type": "application/json",
      "trigger": "temperature",
      "condition": {"above_celsius": 80},
      "interval_seconds": 30,
      "headers": {"X-API-Key": "secret"}
    },
    {
      "name": "heartbeat",
      "url": "http://monitoring.example.com/heartbeat",
      "trigger": "always",
      "interval_seconds": 60
    },
    {
      "name": "temp-changes",
      "url": "http://monitoring.example.com/changes",
      "trigger": "on_change",
      "interval_seconds": 15
    }
  ]
}
```

### Trigger types

| Trigger | Description |
|---------|-------------|
| `always` | Fire on every interval tick |
| `temperature` | Fire when temp crosses threshold |
| `on_change` | Fire when average temp changes by > 0.1°C |

### Webhook payload

Every webhook sends a JSON payload:

```json
{
  "webhook": "temp-alert",
  "timestamp": "2024-01-15T10:30:00+00:00",
  "readings": {
    "devices": [
      {
        "device": {"name": "coretemp", "bus": "ISA", "path": "..."},
        "features": [...]
      }
    ]
  }
}
```

### Webhook options

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | — | Human-readable name (used in logs) |
| `url` | Yes | — | Target URL for HTTP request |
| `method` | No | `POST` | HTTP method |
| `content_type` | No | `application/json` | Content-Type header |
| `trigger` | No | `always` | Trigger type |
| `condition` | No | — | Temperature thresholds (for `temperature` trigger) |
| `interval_seconds` | No | `30` | Seconds between attempts |
| `headers` | No | `{}` | Additional HTTP headers |

---

## Systemd Service

### Installation

```bash
# System-wide service
sudo lm-sensors-web install-service \
    --binary /usr/local/bin/lm-sensors-web \
    --config /etc/lm-sensors-web/config.json

# User-level service
lm-sensors-web install-service --user \
    --binary ~/.local/bin/lm-sensors-web \
    --config ~/.config/lm-sensors-web/config.json
```

### Service unit

The generated unit file includes:

- **Auto-restart**: `Restart=on-failure` with 5s delay
- **Journal logging**: Standard output/error → `journalctl`
- **Networking**: Starts after `network.target`
- **User/Group**: Runs as root (system) or current user (`--user`)

### Management

```bash
# Using systemd directly
sudo systemctl start lm-sensors-web
sudo systemctl status lm-sensors-web
sudo journalctl -u lm-sensors-web -f

# Using the CLI
lm-sensors-web start-service
lm-sensors-web status-service
lm-sensors-web stop-service
```

### Logs

```bash
# View logs
journalctl -u lm-sensors-web -f

# View with timestamp
journalctl -u lm-sensors-web --output=cat
```

---

## Docker

### Docker Compose

```bash
docker compose up -d
```

### Dockerfile

Multi-stage build:
1. Build stage: `rust:bookworm` with `libsensors-dev`
2. Runtime stage: `debian:bookworm-slim`

### docker-compose.yml

```yaml
services:
  lm-sensors-web:
    build: .
    ports:
      - "47890:47890"
    volumes:
      - ./config.json:/app/config.json:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:47890/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
```

### Environment

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level (default: `info`) |
| `CONFIG_PATH` | Config file path (default: `/app/config.json`) |

---

## Security

### Network exposure

- Default binding is `0.0.0.0` (all interfaces) — bind to `127.0.0.1` for local-only access
- Place behind a reverse proxy (nginx, Caddy) for TLS termination
- Use firewall rules to restrict access to the sensor port

### CORS

The default configuration allows all origins (permissive CORS). In production:
- Configure your reverse proxy to restrict `Access-Control-Allow-Origin`
- Or modify the router to use `CorsLayer::new().allow_origin("https://your-domain.com")`

### Webhook URLs

- Webhook targets are specified in `config.json` — keep this file restricted
- Use `headers` for authentication tokens
- Prefer HTTPS endpoints

### Sensor data sensitivity

Sensor readings are generally non-sensitive, but:
- Hardware fingerprints (device names, paths) may reveal system topology
- Temperature patterns may reveal workload characteristics
- Restrict dashboard access in production environments

---

## Performance

### Resource usage

- **Memory**: ~15-30 MB (Rust + tokio runtime)
- **CPU**: Near-zero idle; sensor reads are I/O-bound
- **Threads**: tokio multi-threaded runtime (adjustable via `TOKIO_WORKER_THREADS`)

### Tuning

| Setting | Impact | Recommendation |
|---------|--------|----------------|
| `broadcast_interval_ms` | Higher = less CPU, slower updates | 2000ms for most use cases |
| `refresh_interval_ms` | Higher = less I/O | 5000ms for CPU monitoring |
| Webhook `interval_seconds` | Higher = fewer HTTP requests | 30s minimum recommended |
| WebSocket channel capacity | Higher = more memory, handles slow clients | Default (100) is adequate |

### Monitoring

```bash
# Check server health
curl http://localhost:47890/api/health

# Check systemd service status
systemctl status lm-sensors-web

# Check logs for errors
journalctl -u lm-sensors-web --since "1 hour ago" | grep ERROR
```

---

## Upgrading

### From source

```bash
git pull
cargo build --release
sudo cp target/release/lm-sensors-web /usr/local/bin/
sudo systemctl restart lm-sensors-web
```

### Config migration

Configuration format is stable across versions. No migration needed for:
- `0.x` → `0.y` (patch/minor within 0.x series)

Always backup `config.json` before upgrading.

---

## Getting Help

- **Issues**: https://github.com/your-org/lm-sensors-web/issues
- **Logs**: `journalctl -u lm-sensors-web -f` (systemd) or check process stdout
- **Debug mode**: Run with `--log-level trace` for detailed diagnostics