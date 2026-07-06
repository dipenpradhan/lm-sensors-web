# Troubleshooting Guide — lm-sensors-web

Common issues, diagnostics, and solutions for lm-sensors-web.

---

## Table of Contents

- [Diagnostics](#diagnostics)
- [Startup Issues](#startup-issues)
- [Sensor Issues](#sensor-issues)
- [API Issues](#api-issues)
- [WebSocket Issues](#websocket-issues)
- [Webhook Issues](#webhook-issues)
- [Performance Issues](#performance-issues)
- [Systemd Issues](#systemd-issues)
- [Docker Issues](#docker-issues)
- [Logs](#logs)
- [Debug Mode](#debug-mode)
- [Common Errors](#common-errors)

---

## Diagnostics

### Quick health check

```bash
# Check if the server is running
curl -s -o /dev/null -w "%{http_code}" http://localhost:47890/api/health

# Check device listing
curl -s http://localhost:47890/api/devices | jq

# Check a specific device
curl -s http://localhost:47890/api/devices/coretemp/features | jq
```

### Process check

```bash
# Check if the process is running
ps aux | grep lm-sensors-web

# Check open ports
ss -tlnp | grep 47890
# or
lsof -i :47890
```

### Log check

```bash
# Systemd logs
journalctl -u lm-sensors-web --since "1 hour ago"

# Docker logs
docker logs lm-sensors-web --tail 100

# Stdout/stderr (if running directly)
# Check the process output or redirect to a file
```

---

## Startup Issues

### "Failed to initialize lm-sensors"

**Symptom**: Server fails to start with sensor initialization error.

**Causes**:
- `libsensors` not installed
- `sensors-detect` not run
- No sensor hardware detected

**Fix**:

```bash
# Install libsensors
sudo apt install lm-sensors

# Run sensor detection (answer YES to all prompts for a quick scan)
sudo sensors-detect

# Verify sensors work
sensors
```

### "Address already in use"

**Symptom**: `Bind failed: address already in use`

**Causes**:
- Another process is using the port
- Previous instance didn't shut down cleanly

**Fix**:

```bash
# Find what's using the port
sudo lsof -i :47890
sudo ss -tlnp | grep 47890

# Kill the process
sudo kill -9 <PID>

# Or use a different port
lm-sensors-web -p 8080
```

### "Config load failed"

**Symptom**: Server starts but uses default config.

**Causes**:
- Config file path is wrong
- File permissions issue
- Invalid JSON

**Fix**:

```bash
# Check file exists
ls -la /path/to/config.json

# Check permissions
chmod 644 /path/to/config.json

# Validate JSON
cat config.json | jq . > /dev/null && echo "Valid" || echo "Invalid"

# Specify config explicitly
lm-sensors-web -c /path/to/config.json
```

---

## Sensor Issues

### Empty device list

**Symptom**: `/api/devices` returns `{"devices":[]}`

**Causes**:
- No sensor hardware detected
- `libsensors` not properly configured
- Running in a container without host sysfs access

**Fix**:

```bash
# Check sensors directly
sensors

# Run detection
sudo sensors-detect

# For containers, mount sysfs
docker run -v /sys/class/hwmon:/sys/class/hwmon lm-sensors-web
```

### Missing temperature readings

**Symptom**: Device shows up but features are empty.

**Causes**:
- Sensor driver not loaded
- Permission issue reading sysfs
- Hardware sensor temporarily unavailable

**Fix**:

```bash
# Check sysfs access
ls -la /sys/class/hwmon/

# Check specific sensor
cat /sys/class/hwmon/hwmon0/temp1_input

# Load kernel module (example)
sudo modprobe coretemp
```

### Stale readings (values don't change)

**Symptom**: Sensor values stay the same even when conditions change.

**Causes**:
- `broadcast_interval_ms` is too high
- Sensor driver is caching values
- Hardware limitation (some sensors only update periodically)

**Fix**:

```bash
# Check broadcast interval
grep broadcast_interval_ms config.json

# Reduce interval
# "broadcast_interval_ms": 1000  (1 second)

# Verify sysfs values update
watch -n 1 'cat /sys/class/hwmon/hwmon0/temp1_input'
```

---

## API Issues

### 404 for device lookup

**Symptom**: `GET /api/devices/{id}` returns 404.

**Causes**:
- Device name doesn't match (partial match is case-sensitive)
- Device was removed or renamed
- Typo in the device ID

**Fix**:

```bash
# List all devices to find the correct name
curl http://localhost:47890/api/devices | jq '.devices[].name'

# Use partial match
curl http://localhost:47890/api/devices/coretemp  # matches "coretemp-isa-0000"
```

### Config reload fails

**Symptom**: `POST /api/reload` returns 500.

**Causes**:
- Config file path is wrong
- File was deleted or moved
- Invalid JSON in config

**Fix**:

```bash
# Check the config path used by the server
# It's set by the -c flag at startup

# Verify config exists
ls -la /path/to/config.json

# Validate JSON
cat config.json | jq . > /dev/null

# Reload again
curl -X POST http://localhost:47890/api/reload
```

---

## WebSocket Issues

### Connection refused

**Symptom**: WebSocket upgrade fails with connection error.

**Causes**:
- WebSocket is disabled in config
- Wrong endpoint path
- Reverse proxy not configured for WebSocket

**Fix**:

```bash
# Check config
grep -A3 '"websocket"' config.json

# Ensure WebSocket is enabled
# "enabled": true

# Check reverse proxy WebSocket config
# nginx: proxy_set_header Upgrade $http_upgrade;
#         proxy_set_header Connection "upgrade";
```

### WebSocket messages not received

**Symptom**: Client connects but receives no messages.

**Causes**:
- `broadcast_interval_ms` is very high
- Broadcast channel is full (client is too slow)
- Server error in broadcast loop

**Fix**:

```bash
# Check logs for errors
journalctl -u lm-sensors-web --since "5 minutes ago"

# Reduce broadcast interval
# "broadcast_interval_ms": 1000

# Check client is processing messages fast enough
# Look for "Lagged" warnings in logs
```

### High memory usage from WebSocket

**Symptom**: Memory grows over time.

**Causes**:
- Many slow subscribers accumulating messages
- Broadcast channel capacity too high

**Fix**:

```bash
# Slow clients are automatically dropped
# Check for "Lagged" warnings in logs

# Reduce broadcast channel capacity in code (default: 100)
# Or reduce broadcast interval to flush messages faster
```

---

## Webhook Issues

### Webhook not firing

**Symptom**: Webhook is configured but no requests are sent.

**Causes**:
- Trigger condition not met
- Webhook interval hasn't elapsed yet
- Target URL is unreachable
- Webhook was added but config wasn't reloaded

**Fix**:

```bash
# Check logs for webhook activity
journalctl -u lm-sensors-web | grep -i webhook

# Force config reload
curl -X POST http://localhost:47890/api/reload

# Check trigger condition
# For temperature: verify sensor values cross the threshold
# For on_change: verify temperature changed by > 0.1°C
# For always: wait for interval_seconds to elapse
```

### Webhook HTTP errors

**Symptom**: Webhook fires but gets errors.

**Causes**:
- Target URL is wrong
- Authentication failed
- Target server is down
- Rate limiting on target

**Fix**:

```bash
# Check logs for error details
journalctl -u lm-sensors-web | grep "Webhook.*error"

# Test the webhook URL manually
curl -X POST -H "Content-Type: application/json" \
    -d '{"test": true}' \
    http://target-url/hook

# Check webhook headers and auth
grep -A5 '"webhooks"' config.json
```

---

## Performance Issues

### High CPU usage

**Causes**:
- `broadcast_interval_ms` is very low (< 500ms)
- Many WebSocket clients
- Webhooks dispatching frequently

**Fix**:

```bash
# Increase broadcast interval
# "broadcast_interval_ms": 3000

# Check number of WebSocket connections
ss -an | grep 47890 | grep ESTAB | wc -l

# Reduce webhook frequency
# "interval_seconds": 60
```

### High memory usage

**Causes**:
- Many WebSocket clients with slow connections
- Log accumulation
- Leak in dependencies (unlikely in Rust)

**Fix**:

```bash
# Check memory usage
ps aux | grep lm-sensors-web

# Check for slow WebSocket clients in logs
journalctl -u lm-sensors-web | grep -i lagged

# Configure log rotation
# /etc/systemd/journald.conf
# SystemMaxUse=100M
```

### Slow API responses

**Causes**:
- Sensor reads are slow (hardware I/O)
- Too many concurrent requests
- System under load

**Fix**:

```bash
# Check system load
uptime

# Check I/O wait
iostat -x 1

# The API is I/O bound — sensor reads are inherently slow
# Cache responses at the client side instead
```

---

## Systemd Issues

### Service fails to start

**Symptom**: `systemctl status lm-sensors-web` shows failed.

**Causes**:
- Binary path is wrong
- Config file path is wrong
- Permission denied

**Fix**:

```bash
# Check status
systemctl status lm-sensors-web

# Check logs
journalctl -u lm-sensors-web -n 50

# Verify binary exists
ls -la /usr/local/bin/lm-sensors-web

# Test binary directly
/usr/local/bin/lm-sensors-web -c /etc/lm-sensors-web/config.json
```

### Service restarts constantly

**Symptom**: Service restarts every 5 seconds (RestartSec).

**Causes**:
- Server crashes on startup
- Config file invalid
- Port already in use

**Fix**:

```bash
# Check restart count
systemctl show lm-sensors-web | grep NRestarts

# Check logs for crash reason
journalctl -u lm-sensors-web --since "5 minutes ago"

# Run manually to see errors
/usr/local/bin/lm-sensors-web -c /etc/lm-sensors-web/config.json
```

---

## Docker Issues

### Container exits immediately

**Causes**:
- Missing `libsensors` library
- Config file not mounted
- Sensor initialization fails

**Fix**:

```bash
# Check logs
docker logs lm-sensors-web

# Ensure libsensors is in the image
docker run --rm lm-sensors-web:latest apt list --installed 2>/dev/null | grep sensors

# Mount config file
docker run -v $(pwd)/config.json:/app/config.json:ro lm-sensors-web:latest
```

### "sensor init failed" in container

**Causes**:
- Container doesn't have access to host sysfs
- `libsensors` library not installed

**Fix**:

```bash
# Mount sysfs
docker run -v /sys/class/hwmon:/sys/class/hwmon:ro lm-sensors-web:latest

# Or run with --privileged (less secure)
docker run --privileged lm-sensors-web:latest
```

---

## Logs

### Systemd logs

```bash
# Follow logs in real-time
journalctl -u lm-sensors-web -f

# Show last 100 lines
journalctl -u lm-sensors-web -n 100

# Show errors only
journalctl -u lm-sensors-web -p err

# Show since specific time
journalctl -u lm-sensors-web --since "1 hour ago"

# Show specific log level
journalctl -u lm-sensors-web -p warning
```

### Log levels

| Level | When to use |
|-------|-------------|
| `trace` | Detailed debugging (verbose) |
| `debug` | Development debugging |
| `info` | Production default |
| `warn` | Production (suppress info) |
| `error` | Only critical issues |

```bash
# Change log level
RUST_LOG=debug lm-sensors-web
# or
lm-sensors-web --log-level debug
```

### Docker logs

```bash
# Follow logs
docker logs -f lm-sensors-web

# Last 100 lines
docker logs --tail 100 lm-sensors-web

# Since specific time
docker logs --since 1h lm-sensors-web
```

---

## Debug Mode

Enable debug mode for detailed diagnostics:

```bash
# Via CLI flag
lm-sensors-web --log-level debug

# Via environment variable
RUST_LOG=debug lm-sensors-web

# Via config
{
  "server": {
    "log_level": "debug"
  }
}
```

Debug logs include:
- Sensor read details
- WebSocket client connections/disconnections
- Webhook dispatch details
- Config reload events
- Request processing traces

---

## Common Errors

### Error: "Failed to initialize lm-sensors"

```bash
# Install libsensors
sudo apt install lm-sensors
sudo sensors-detect

# Verify
sensors
```

### Error: "Bind failed: address already in use"

```bash
# Find the process
sudo lsof -i :47890
sudo kill -9 <PID>
```

### Error: "Failed to reload config"

```bash
# Validate JSON
cat config.json | jq . > /dev/null

# Check file exists
ls -la /path/to/config.json
```

### Error: "WebSocket client lagged"

This is a warning, not an error. The client is consuming messages too slowly.
Old messages are dropped. Consider:
- Increasing `broadcast_interval_ms`
- Optimizing client message processing
- Accepting occasional data loss (non-critical for monitoring)

---

## Getting Help

1. **Check logs**: `journalctl -u lm-sensors-web -f`
2. **Enable debug**: `--log-level debug`
3. **Verify sensors**: `sensors` (command-line)
4. **Test API**: `curl http://localhost:47890/api/health`
5. **Check docs**: `docs/USER_GUIDE.md`, `docs/API_REFERENCE.md`
6. **Report issues**: https://github.com/your-org/lm-sensors-web/issues