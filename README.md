# lm-sensors-web

Hardware sensor monitoring web application built in Rust. Exposes real-time sensor data (temperatures, voltages, fan speeds, etc.) from Linux `libsensors` via a REST API, WebSocket live-feed, and a lightweight dark-mode web dashboard.

![lm-sensors-web](static/app.png)

## Features

- **REST API** — `/api/sensors`, `/api/devices`, `/api/devices/{id}`, `/api/devices/{id}/features`, `/api/health`
- **WebSocket** — live sensor broadcast on `/ws/sensors`
- **Webhooks** — scheduled sensor push with triggers (`always`, `temperature`, `on-change`)
- **Web Dashboard** — dark-mode UI with pastel-coloured sensor cards, real-time filter, collapsible grid
- **CLI** — `--host`, `--port`, `--log-level`, `--config`, service management subcommands
- **Docker** — multi-stage build + `docker-compose.yml`
- **Linux Service** — systemd install/uninstall/start/stop/restart/status

## Architecture

### Component diagram

```dot
// Render with: dot -Tsvg components.dot > components.svg
// Or paste into https://dreampuf.github.io/GraphvizOnline

digraph lm_sensors_web {
  rankdir=LR;
  node [fontname="Helvetica", fontsize=11];
  edge [color="#7eb8da"];

  // Styling
  subgraph cluster_app {
    label="lm-sensors-web";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    CLI       [shape=box, fillcolor="#242640", fontcolor=white, label="CLI\n(clap)"];
    Config    [shape=box, fillcolor="#242640", fontcolor=white, label="Config\n(JSON)" ];
    Sensor    [shape=box, fillcolor="#242640", fontcolor=white, label="SensorManager\n(lm-sensors)"];
    Router    [shape=box, fillcolor="#242640", fontcolor=white, label="Axum Router"];
    WebSocket [shape=box, fillcolor="#242640", fontcolor=white, label="WebSocket\nbroadcast"];
    Webhook   [shape=box, fillcolor="#242640", fontcolor=white, label="Webhook\nengine"];
    Static    [shape=box, fillcolor="#242640", fontcolor=white, label="Static Files"];
  }

  subgraph cluster_deps {
    label="External";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    LibSensors [shape=cds, fillcolor="#2e3050", fontcolor=white, label="libsensors\n(sysfs/hwmon)"];
    Browser    [shape=ellipse, fillcolor="#2e3050", fontcolor=white, label="Browser"];
  }

  CLI -> Config       [label="loads"];
  CLI -> Router       [label="starts"];
  Router -> Sensor   [label="queries"];
  Router -> WebSocket [label="subscribes"];
  Router -> Webhook   [label="triggers"];
  Router -> Static    [label="serves"];
  Router -> Config    [label="reloads"];
  Sensor -> LibSensors [label="reads"];
  Browser -> Router  [label="HTTP / WS"];
  Browser -> Static  [label="GET"];
}
```

### Request flow

```dot
// Render with: dot -Tsvg flow.dot > flow.svg
// Or paste into https://dreampuf.github.io/GraphvizOnline

digraph request_flow {
  rankdir=TB;
  node [shape=box, fontname="Helvetica", fontsize=10];
  edge [color="#7eb8da"];

  // REST flow
  subgraph cluster_rest {
    label="REST: GET /api/sensors";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    R1 [fillcolor="#242640", fontcolor=white, label="Browser\nGET /api/sensors"];
    R2 [fillcolor="#242640", fontcolor=white, label="Axum Router\nget_devices()"];
    R3 [fillcolor="#242640", fontcolor=white, label="SensorManager\nlist_devices()"];
    R4 [fillcolor="#2e3050", fontcolor=white, label="libsensors\nchip_iter()"];
    R5 [fillcolor="#242640", fontcolor=white, label="200 JSON\n{ devices: [...] }"];

    R1 -> R2 -> R3 -> R4;
    R4 -> R3 -> R2 -> R5;
  }

  // WebSocket flow
  subgraph cluster_ws {
    label="WebSocket: /ws/sensors";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    W1 [fillcolor="#242640", fontcolor=white, label="Browser\nWS upgrade"];
    W2 [fillcolor="#242640", fontcolor=white, label="broadcast::channel\nsubscribe()"];
    W3 [fillcolor="#242640", fontcolor=white, label="broadcast loop\n(tick every 2s)"];
    W4 [fillcolor="#242640", fontcolor=white, label="SensorManager\nread_all()"];
    W5 [fillcolor="#242640", fontcolor=white, label="browser\nJSON frame"];

    W1 -> W2 -> W3 -> W4;
    W4 -> W3 -> W2 -> W5;
  }

  // Auto-reconnect note
  subgraph cluster_note {
    label="Dashboard resilience";
    style=filled;
    fillcolor="#2e3050";
    color="#7eb8da";
    fontcolor="#c8c9e0";

    N1 [fillcolor="#242640", fontcolor=white, label="WS disconnect\n→ exponential backoff"];
    N2 [fillcolor="#242640", fontcolor=white, label="REST fallback\nGET /api/sensors every 30s"];
  }
}
```

### Sensor wrapper safety

```dot
// Render with: dot -Tsvg safety.dot > safety.svg
// Or paste into https://dreampuf.github.io/GraphvizOnline

digraph sensor_safety {
  rankdir=TB;
  node [fontname="Helvetica", fontsize=10];
  edge [color="#7eb8da"];

  // std::sync primitives
  subgraph cluster_std {
    label="std::sync";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    Arc    [shape=doublecircle, fillcolor="#242640", fontcolor=white, label="Arc<T>"];
    RwLock [shape=doublecircle, fillcolor="#242640", fontcolor=white, label="RwLock<T>"];
  }

  // External crate
  subgraph cluster_external {
    label="lm_sensors crate";
    style=filled;
    fillcolor="#2e3050";
    color="#7eb8da";
    fontcolor=white;

    LMSensors [shape=box, fillcolor="#242640", fontcolor=white,
      label="LMSensors\n(not Send + Sync)"];
  }

  // Our wrapper
  subgraph cluster_ours {
    label="lm-sensors-web";
    style=filled;
    fillcolor="#1a1b2e";
    color="#7eb8da";
    fontcolor=white;

    Safe [shape=box, fillcolor="#242640", fontcolor=white,
      label="SafeLMSensors\n(unsafe impl Send + Sync)"];

    Manager [shape=box, fillcolor="#242640", fontcolor=white,
      label="SensorManager\n(Arc<RwLock<SafeLMSensors>>)"];
  }

  // Safety invariant
  Invariant [shape=note, fillcolor="#2e3050", fontcolor="#c8c9e0",
    label="Safety invariant:\n• RwLock guard = exclusive read\n• chip_iter() never mutates\n• Multiple tasks share Arc"];

  Manager -> Arc  [label="owns"];
  Arc   -> RwLock [label="wraps"];
  RwLock -> Safe [label="protects"];
  Safe  -> LMSensors [label="contains"];
  Manager -> Invariant;
}
```

## Quick Start

```bash
# Build
cargo build --release

# Run (binds to 0.0.0.0:47890)
./target/release/lm-sensors-web

# Open dashboard
open http://localhost:47890
```

## CLI

```
lm-sensors-web [FLAGS] [SUBCOMMAND]

Flags:
  -H, --host <HOST>        Bind address (default: 0.0.0.0)
  -p, --port <PORT>        Listen port (default: 47890)
  -l, --log-level <LEVEL>  Logging level (info|debug|trace|warn|error)
  -c, --config <PATH>      Path to config.json
  -h, --help               Print help

Subcommands:
  install-service          Install as systemd service
  uninstall-service        Remove systemd service
  start-service            Start service
  stop-service             Stop service
  restart-service          Restart service
  status-service           Show service status
```

## Config (`config.json`)

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
  "webhooks": [
    {
      "name": "temp-alert",
      "url": "http://localhost:9090/alerts",
      "method": "POST",
      "content_type": "application/json",
      "trigger": "temperature",
      "condition": { "above_celsius": 80 },
      "interval_seconds": 30
    }
  ],
  "sensors": {
    "refresh_interval_ms": 5000
  }
}
```

## Docker

```bash
docker compose up -d
```

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Health check |
| `POST` | `/api/reload` | Hot-reload config |
| `GET` | `/api/devices` | List all devices |
| `GET` | `/api/devices/{device_id}` | Device details |
| `GET` | `/api/devices/{device_id}/features` | Device readings |
| `GET` | `/ws/sensors` | WebSocket live feed |

## Testing

```bash
cargo test
```

## License

MIT
