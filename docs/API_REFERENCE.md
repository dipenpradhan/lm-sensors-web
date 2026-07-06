# API Reference — lm-sensors-web

Complete reference for all REST API endpoints, WebSocket protocol, and
request/response schemas.

---

## Table of Contents

- [Base URL](#base-url)
- [Authentication](#authentication)
- [Endpoints](#endpoints)
  - [Health Check](#health-check)
  - [Config Reload](#config-reload)
  - [List Devices](#list-devices)
  - [Get Device](#get-device)
  - [Get Device Features](#get-device-features)
- [WebSocket](#websocket)
- [Response Schemas](#response-schemas)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [CORS](#cors)

---

## Base URL

```
http://<host>:<port>
```

Default: `http://0.0.0.0:47890`

---

## Authentication

The API has no authentication by default. For production deployments, place
behind an authenticated reverse proxy (nginx, Caddy, Cloudflare) or add
authentication middleware to the Axum router.

---

## Endpoints

### Health Check

**`GET /api/health`**

Liveness probe. Returns server status and current timestamp.

**Request**

```
GET /api/health HTTP/1.1
Host: localhost:47890
```

**Response** `200 OK`

```json
{
  "status": "ok",
  "timestamp": "2024-01-15T10:30:00.000000000+00:00"
}
```

**Use cases**
- Load balancer health checks
- Kubernetes liveness/readiness probes
- Uptime monitoring scripts

**curl example**

```bash
curl -f http://localhost:47890/api/health
# Returns: {"status":"ok","timestamp":"2024-01-15T10:30:00+00:00"}
```

---

### Config Reload

**`POST /api/reload`**

Hot-reload the configuration file without restarting the server.

**Request**

```
POST /api/reload HTTP/1.1
Host: localhost:47890
```

No request body required.

**Response** `200 OK` (success)

```json
{
  "status": "ok",
  "message": "Config reloaded successfully"
}
```

**Response** `500 Internal Server Error` (failure)

```json
{
  "status": "error",
  "message": "Failed to reload config: No such file or directory (os error 2)"
}
```

**Behavior**
1. Reads the config file from the path specified at startup (`-c` flag or working directory)
2. Parses and validates the JSON
3. Atomically replaces the shared `Config` behind `RwLock`
4. WebSocket broadcast interval updates on next tick
5. New webhooks spawn; removed webhooks continue until next config poll

**curl example**

```bash
curl -X POST http://localhost:47890/api/reload
# Returns: {"status":"ok","message":"Config reloaded successfully"}
```

---

### List Devices

**`GET /api/devices`**

List all detected sensor devices (metadata only, no readings).

**Request**

```
GET /api/devices HTTP/1.1
Host: localhost:47890
Accept: application/json
```

**Response** `200 OK`

```json
{
  "devices": [
    {
      "name": "coretemp",
      "bus": "ISA",
      "path": "/sys/class/hwmon/hwmon0"
    },
    {
      "name": "acpitz",
      "bus": "ISA",
      "path": null
    },
    {
      "name": "it8792",
      "bus": "ISA",
      "path": "/sys/class/hwmon/hwmon1"
    }
  ]
}
```

**Device schema**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Device name (e.g. "coretemp", "acpitz") |
| `bus` | `string` | Bus type (e.g. "ISA", "SMBus", "PCI") |
| `path` | `string` or `null` | sysfs path, or `null` if unavailable |

**Performance**
- Metadata only — no sensor reads
- Fast response (~1ms)
- Suitable for frequent polling

**curl example**

```bash
curl http://localhost:47890/api/devices | jq
```

---

### Get Device

**`GET /api/devices/{device_id}`**

Get a specific device by partial name match (case-sensitive).

**Request**

```
GET /api/devices/coretemp HTTP/1.1
Host: localhost:47890
Accept: application/json
```

**Path parameter**

| Parameter | Type | Description |
|-----------|------|-------------|
| `device_id` | `string` | Partial device name (searches all devices) |

**Response** `200 OK`

```json
{
  "device": {
    "name": "coretemp-isa-0000",
    "bus": "ISA",
    "path": "/sys/class/hwmon/hwmon0"
  }
}
```

**Response** `404 Not Found`

```json
{
  "error": "Device 'nonexistent' not found"
}
```

**Behavior**
- Searches all devices and returns the first whose name contains `device_id`
- Case-sensitive partial match
- Metadata only — use `/api/devices/{id}/features` for readings

**curl example**

```bash
curl http://localhost:47890/api/devices/coretemp
# Returns: {"device":{"name":"coretemp-isa-0000","bus":"ISA","path":"..."}}
```

---

### Get Device Features

**`GET /api/devices/{device_id}/features`**

Get a specific device with all feature readings and current values.

**Request**

```
GET /api/devices/coretemp/features HTTP/1.1
Host: localhost:47890
Accept: application/json
```

**Path parameter**

| Parameter | Type | Description |
|-----------|------|-------------|
| `device_id` | `string` | Partial device name |

**Response** `200 OK`

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
        {
          "name": "temp1_input",
          "value": 55.0,
          "unit": "°C"
        },
        {
          "name": "temp1_max",
          "value": 100.0,
          "unit": "°C"
        },
        {
          "name": "temp1_crit",
          "value": 105.0,
          "unit": "°C"
        }
      ]
    },
    {
      "name": "temp2",
      "sub_features": [
        {
          "name": "temp2_input",
          "value": 42.5,
          "unit": "°C"
        }
      ]
    }
  ]
}
```

**Response** `404 Not Found`

```json
{
  "error": "Device 'nonexistent' not found"
}
```

**Feature schema**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Feature name (e.g. "temp1", "fan1", "in1") |
| `sub_features` | `array` | Array of sub-feature readings |

**Sub-feature schema**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Sub-feature name (e.g. "temp1_input") |
| `value` | `number` or `null` | Current reading value, or `null` if unreadable |
| `unit` | `string` | Unit of measurement (°C, V, A, W, RPM, %, s, J) |

**Units reference**

| Unit | Symbol | Description |
|------|--------|-------------|
| Temperature | °C | Degrees Celsius |
| Voltage | V | Volts |
| Current | A | Amperes |
| Power | W | Watts |
| Energy | J | Joules |
| Rotation | RPM | Revolutions per minute |
| Percentage | % | Percentage |
| Time | s | Seconds |
| None | — | Unitless |

**Performance**
- Triggers a full sensor read for the matching device
- Typical latency: 1-10ms
- More expensive than `/api/devices` (metadata-only)

**curl example**

```bash
curl http://localhost:47890/api/devices/coretemp/features | jq
```

---

## WebSocket

### Endpoint

```
ws://<host>:<port>/ws/sensors
```

Default path: `/ws/sensors` (configurable via `websocket.path` in config).

### Protocol

1. **Upgrade**: Client sends `GET` with `Upgrade: websocket` header
2. **Connect**: Server accepts and begins broadcasting
3. **Messages**: Server pushes `SensorReadings` JSON on every broadcast tick
4. **Disconnect**: Client closes connection or server shuts down

### Broadcast payload

Every message is a JSON-serialized `SensorReadings`:

```json
{
  "devices": [
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
  ]
}
```

### Broadcast interval

Configurable via `websocket.broadcast_interval_ms` (default: 2000ms).

### Client behavior

- **Slow clients**: If a client can't consume messages fast enough, it receives
  a `Lagged` error. Messages are dropped, not queued.
- **Reconnection**: Server does not reconnect — clients must implement their own
  reconnection logic.
- **Capacity**: Broadcast channel has capacity of 100 messages.

### JavaScript client example

```javascript
function connect() {
  const ws = new WebSocket('ws://localhost:47890/ws/sensors');

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    // Process sensor readings
    console.log(data.devices);
  };

  ws.onerror = (error) => {
    console.error('WebSocket error:', error);
  };

  ws.onclose = () => {
    // Reconnect after a delay
    setTimeout(connect, 1000);
  };
}

connect();
```

---

## Response Schemas

### Device

```json
{
  "name": "string",    // Device name
  "bus": "string",     // Bus type
  "path": "string"     // sysfs path or null
}
```

### FeatureInfo

```json
{
  "name": "string",           // Feature name (e.g. "temp1")
  "sub_features": [           // Array of sub-features
    {
      "name": "string",       // Sub-feature name
      "value": "number",      // Reading value or null
      "unit": "string"        // Unit symbol
    }
  ]
}
```

### DeviceReadings

```json
{
  "device": { /* Device schema */ },
  "features": [ /* FeatureInfo array */ ]
}
```

### SensorReadings

```json
{
  "devices": [ /* DeviceReadings array */ ]
}
```

### Health

```json
{
  "status": "string",     // Always "ok"
  "timestamp": "string"   // RFC 3339 timestamp
}
```

### Error

```json
{
  "error": "string"       // Human-readable error message
}
```

---

## Error Handling

### HTTP status codes

| Code | Meaning | Endpoint |
|------|---------|----------|
| `200` | Success | All endpoints |
| `404` | Device not found | `/api/devices/{id}`, `/api/devices/{id}/features` |
| `500` | Config reload failed | `/api/reload` |

### Error response format

All error responses use a consistent JSON format:

```json
{
  "error": "Human-readable error message"
}
```

### Config reload errors

```json
{
  "status": "error",
  "message": "Failed to reload config: <reason>"
}
```

---

## Rate Limiting

The API does not implement rate limiting. For production deployments:

- Use a reverse proxy (nginx, Cloudflare) for rate limiting
- Typical sensor polling: 1-5 requests/second is safe
- WebSocket replaces REST polling for real-time updates

---

## CORS

The server includes permissive CORS middleware by default:

```
Access-Control-Allow-Origin: *
```

**For production:**
- Restrict origins in your reverse proxy
- Or modify the router to use `CorsLayer::new().allow_origin("https://your-domain.com")`
- WebSocket connections bypass CORS (they use a separate upgrade protocol)