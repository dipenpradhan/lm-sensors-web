# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.x     | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please
report it responsibly:

1. **Do not** open a public GitHub issue
2. Email: security@example.com (replace with your actual address)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
4. We will acknowledge receipt within 48 hours
5. We will respond with a timeline for resolution

### What to report

- Authentication bypass
- Unauthorized data access
- Remote code execution
- Denial of service
- Configuration file vulnerabilities
- Dependency vulnerabilities (also run `cargo audit`)

### What not to report

- Issues in third-party dependencies (report to their maintainers)
- Theoretical attacks without a realistic attack scenario
- Issues requiring physical access to the target system

## Security considerations

### Network exposure

By default, the server binds to `0.0.0.0` (all interfaces). For production:
- Bind to `127.0.0.1` and use a reverse proxy
- Configure firewall rules to restrict access
- Use TLS termination at the reverse proxy

### Authentication

The API has no built-in authentication. Add authentication at:
- Reverse proxy layer (nginx auth, Cloudflare Access)
- Reverse proxy with OAuth/OIDC (Authelia, Caddy with `forward_auth`)
- API gateway layer

### CORS

Permissive CORS (`Access-Control-Allow-Origin: *`) is the default. Restrict
in production via reverse proxy configuration.

### Webhooks

Webhook targets are specified in `config.json`. Ensure:
- Webhook endpoints use HTTPS
- Authentication headers are included
- Target URLs are trusted

### Config file

The config file may contain sensitive data (webhook URLs, API keys).
Ensure proper file permissions:

```bash
chmod 600 /etc/lm-sensors-web/config.json
chown root:root /etc/lm-sensors-web/config.json
```

### Dependencies

Run security audits regularly:

```bash
cargo install cargo-audit
cargo audit
```

## Bounties

This project does not currently offer a bug bounty program.