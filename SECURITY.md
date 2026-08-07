# Security policy

## Supported versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Previous release | Critical fixes only |
| Older versions | No |

## Reporting a vulnerability

If you discover a security vulnerability in Titen, please report it responsibly.

**Do not** open a public issue for security vulnerabilities.

### How to report

Use [GitHub Private Vulnerability Reporting](https://github.com/codecoradev/titen/security/advisories/new). This keeps the report confidential and visible only to maintainers.

Include as much detail as possible:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### What to expect

- **Acknowledgment** within 48 hours
- **Initial assessment** within 5 business days
- **Fix timeline** depends on severity:
  - Critical: 7 days
  - High: 14 days
  - Medium: 30 days
  - Low: next minor release

## Security measures in the development process

Titen uses multiple automated security checks on every PR:

- **Cargo Audit**: dependency vulnerability scanning
- **Trivy FS Scan**: filesystem security scanning
- **CodeCora review**: AI-powered code review with security rules

These run via CI and block merge on findings.

## Token encryption

`access_token` and `app_secret` are encrypted at rest with AES-256-GCM (since PR #48). Production deployments **must** set `TITEN_ENCRYPTION_KEY` (generate with `openssl rand -hex 32`).

> **Warning:** If `TITEN_ENCRYPTION_KEY` is missing, the store runs in plaintext mode. This is intended for local development only. Running without encryption in production is a security risk. Verify the key is set before deploying.

See `.env.example` for configuration details.
