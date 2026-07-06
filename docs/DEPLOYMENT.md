# Deployment Guide — lm-sensors-web

Production deployment strategies for lm-sensors-web, covering bare metal,
containerized, and managed environments.

---

## Table of Contents

- [Deployment Options](#deployment-options)
- [Systemd Service](#systemd-service)
- [Docker](#docker)
- [Docker Compose](#docker-compose)
- [Reverse Proxy](#reverse-proxy)
- [Kubernetes](#kubernetes)
- [Configuration Management](#configuration-management)
- [Monitoring](#monitoring)
- [Backups](#backups)
- [Security Checklist](#security-checklist)
- [Scaling](#scaling)

---

## Deployment Options

| Option | Use case | Complexity |
|--------|----------|------------|
| **Binary + systemd** | Single server, simple setup | Low |
| **Docker** | Containerized, reproducible | Low |
| **Docker Compose** | Multi-service with proxy | Medium |
| **Kubernetes** | Clustered, self-healing | High |
| **Reverse proxy** | TLS, auth, CDN | Medium |

---

## Systemd Service

### Installation

```bash
# Build and install
cargo build --release
sudo cp target/release/lm-sensors-web /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/lm-sensors-web
sudo cp config.example.json /etc/lm-sensors-web/config.json

# Install systemd service
sudo /usr/local/bin/lm-sensors-web install-service \
    --binary /usr/local/bin/lm-sensors-web \
    --config /etc/lm-sensors-web/config.json
```

### Generated unit file

```ini
[Unit]
Description=LM Sensors Web API Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/lm-sensors-web
Environment=RUST_LOG=info
Environment=CONFIG_PATH=/etc/lm-sensors-web/config.json
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### Enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable lm-sensors-web
sudo systemctl start lm-sensors-web
```

### Verify

```bash
# Check status
sudo systemctl status lm-sensors-web

# Check logs
sudo journalctl -u lm-sensors-web -f

# Health check
curl -f http://localhost:47890/api/health
```

### Update

```bash
# Stop service
sudo systemctl stop lm-sensors-web

# Replace binary
sudo cp target/release/lm-sensors-web /usr/local/bin/

# Restart
sudo systemctl start lm-sensors-web
```

---

## Docker

### Build

```bash
docker build -t lm-sensors-web:latest .
```

### Run

```bash
docker run -d \
    --name lm-sensors-web \
    --restart unless-stopped \
    -p 47890:47890 \
    -v $(pwd)/config.json:/app/config.json:ro \
    -e RUST_LOG=info \
    lm-sensors-web:latest
```

### Dockerfile (multi-stage)

```dockerfile
# Build stage
FROM rust:bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsensors5 \
    curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/lm-sensors-web /usr/local/bin/
COPY static/ /app/static/
COPY config.example.json /app/config.json
WORKDIR /app
EXPOSE 47890
ENTRYPOINT ["lm-sensors-web"]
```

---

## Docker Compose

### docker-compose.yml

```yaml
version: "3.8"

services:
  lm-sensors-web:
    build: .
    container_name: lm-sensors-web
    ports:
      - "47890:47890"
    volumes:
      - ./config.json:/app/config.json:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:47890/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"

  # Optional: nginx reverse proxy with TLS
  # nginx:
  #   image: nginx:alpine
  #   ports:
  #     - "80:80"
  #     - "443:443"
  #   volumes:
  #     - ./nginx.conf:/etc/nginx/nginx.conf
  #     - ./certs:/etc/nginx/certs:ro
  #   depends_on:
  #     - lm-sensors-web
```

### Start

```bash
docker compose up -d
docker compose logs -f
```

---

## Reverse Proxy

### nginx configuration

```nginx
# /etc/nginx/conf.d/lm-sensors-web.conf

upstream lm_sensors {
    server 127.0.0.1:47890;
}

server {
    listen 443 ssl http2;
    server_name sensors.example.com;

    ssl_certificate     /etc/nginx/certs/sensors.crt;
    ssl_certificate_key /etc/nginx/certs/sensors.key;
    ssl_protocols TLSv1.2 TLSv1.3;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";

    # Dashboard
    location / {
        proxy_pass http://lm_sensors;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket support
    location /ws/sensors {
        proxy_pass http://lm_sensors;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;  # Keep alive for WebSocket
    }

    # API endpoints
    location /api/ {
        proxy_pass http://lm_sensors;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # Rate limiting
    # limit_req_zone $binary_remote_addr zone=api:10m rate=30r/s;
    # location /api/ {
    #     limit_req zone=api burst=20;
    #     ...
    # }
}

# HTTP → HTTPS redirect
server {
    listen 80;
    server_name sensors.example.com;
    return 301 https://$host$request_uri;
}
```

### Caddy configuration

```caddy
sensors.example.com {
    tls admin@example.com

    # Security headers
    header {
        X-Frame-Options DENY
        X-Content-Type-Options nosniff
    }

    # Dashboard and API
    reverse_proxy 127.0.0.1:47890

    # WebSocket support (automatic in Caddy 2)
}
```

---

## Kubernetes

### Deployment manifest

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lm-sensors-web
  labels:
    app: lm-sensors-web
spec:
  replicas: 1
  selector:
    matchLabels:
      app: lm-sensors-web
  template:
    metadata:
      labels:
        app: lm-sensors-web
    spec:
      containers:
        - name: lm-sensors-web
          image: lm-sensors-web:latest
          ports:
            - containerPort: 47890
          env:
            - name: RUST_LOG
              value: "info"
          volumeMounts:
            - name: config
              mountPath: /app/config.json
              subPath: config.json
          livenessProbe:
            httpGet:
              path: /api/health
              port: 47890
            initialDelaySeconds: 5
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/health
              port: 47890
            initialDelaySeconds: 3
            periodSeconds: 10
      volumes:
        - name: config
          configMap:
            name: lm-sensors-web-config
---
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: lm-sensors-web
spec:
  selector:
    app: lm-sensors-web
  ports:
    - protocol: TCP
      port: 47890
      targetPort: 47890
  type: ClusterIP
---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: lm-sensors-web-config
data:
  config.json: |
    {
      "server": {"host": "0.0.0.0", "port": 47890, "log_level": "info"},
      "websocket": {"enabled": true, "path": "/ws/sensors", "broadcast_interval_ms": 2000},
      "webhooks": [],
      "sensors": {"refresh_interval_ms": 5000}
    }
```

### Apply

```bash
kubectl apply -f k8s/
kubectl get pods -l app=lm-sensors-web
kubectl logs -l app=lm-sensors-web -f
```

---

## Configuration Management

### Config file structure

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
      "url": "https://alerts.example.com/hooks/sensors",
      "method": "POST",
      "content_type": "application/json",
      "trigger": "temperature",
      "condition": {"above_celsius": 80},
      "interval_seconds": 30,
      "headers": {"X-API-Key": "secret"}
    }
  ],
  "sensors": {
    "refresh_interval_ms": 5000
  }
}
```

### Hot-reload

```bash
# Update the config file
cp config.new.json /etc/lm-sensors-web/config.json

# Trigger reload via API
curl -X POST http://localhost:47890/api/reload
```

### Config validation

```bash
# Validate JSON syntax
cat config.json | jq . > /dev/null && echo "Valid JSON" || echo "Invalid JSON"

# Check with python
python3 -c "import json; json.load(open('config.json'))" && echo "Valid" || echo "Invalid"
```

---

## Monitoring

### Health endpoint

```bash
# Continuous health monitoring
while true; do
    status=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:47890/api/health)
    if [ "$status" != "200" ]; then
        echo "$(date): Health check failed (HTTP $status)"
        # Alert logic here
    fi
    sleep 30
done
```

### Systemd service monitoring

```bash
# Check service status
systemctl is-active lm-sensors-web

# Check recent logs
journalctl -u lm-sensors-web --since "1 hour ago"

# Check for errors
journalctl -u lm-sensors-web -p err --since "1 day ago"
```

### Prometheus-style metrics

The health endpoint can be scraped for availability monitoring:

```bash
# Custom health script for monitoring tools
#!/bin/bash
response=$(curl -s -w "\n%{http_code}" http://localhost:47890/api/health)
status=$(echo "$response" | tail -1)
if [ "$status" = "200" ]; then
    echo "1"  # Healthy
else
    echo "0"  # Unhealthy
fi
```

### Resource monitoring

```bash
# Memory usage
ps aux | grep lm-sensors-web | grep -v grep

# Open connections (if using ss)
ss -tlnp | grep 47890
```

---

## Backups

### Config backup

```bash
# Backup config
cp /etc/lm-sensors-web/config.json /etc/lm-sensors-web/config.json.bak.$(date +%Y%m%d)

# Restore
cp /etc/lm-sensors-web/config.json.bak.20240115 /etc/lm-sensors-web/config.json
systemctl restart lm-sensors-web
```

### systemd service backup

```bash
# Backup unit file
cp /etc/systemd/system/lm-sensors-web.service /etc/systemd/system/lm-sensors-web.service.bak
```

---

## Security Checklist

Before deploying to production:

- [ ] **Bind address**: Use `127.0.0.1` or restrict via firewall if not behind proxy
- [ ] **TLS**: Terminate TLS at reverse proxy (nginx, Caddy, Cloudflare)
- [ ] **Authentication**: Add auth layer at reverse proxy (basic auth, OAuth, API keys)
- [ ] **CORS**: Restrict `Access-Control-Allow-Origin` to your domain
- [ ] **Webhook URLs**: Use HTTPS endpoints for webhooks
- [ ] **Webhook auth**: Include authentication headers in webhook config
- [ ] **Config file permissions**: `600` or `640` on config.json
- [ ] **Log rotation**: Configure journal log rotation (`/etc/systemd/journald.conf`)
- [ ] **Firewall**: Restrict port access to trusted networks
- [ ] **Updates**: Subscribe to security advisories

### Config file permissions

```bash
sudo chmod 600 /etc/lm-sensors-web/config.json
sudo chown root:root /etc/lm-sensors-web/config.json
```

### Firewall rules

```bash
# Allow only from specific networks
sudo ufw allow from 10.0.0.0/8 to any port 47890
sudo ufw allow from 192.168.0.0/16 to any port 47890
```

---

## Scaling

### Single instance

For most use cases, a single instance handles all traffic:
- REST API: ~1000 req/s
- WebSocket: ~100 concurrent connections
- Webhooks: ~10 concurrent dispatches

### Horizontal scaling

If you need more than one instance:

1. **Shared nothing**: Each instance runs independently
2. **Load balancer**: Distribute REST traffic across instances
3. **WebSocket**: Sticky sessions required (can't be load-balanced easily)
4. **Webhooks**: Each instance dispatches independently (may cause duplicates)

### Vertical scaling

For higher loads on a single instance:

- Increase `websocket.broadcast_interval_ms` (reduces CPU)
- Increase sensor `refresh_interval_ms` (reduces I/O)
- Limit concurrent WebSocket clients
- Use `TOKIO_WORKER_THREADS=8` for more parallelism