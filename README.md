# Email Manager API

[![Rust CI](https://github.com/felipemeriga/email-manager/actions/workflows/rust.yml/badge.svg)](https://github.com/felipemeriga/email-manager/actions/workflows/rust.yml)
[![Docker Hub](https://img.shields.io/docker/v/felipemeriga1/email-manager?label=docker&sort=semver)](https://hub.docker.com/r/felipemeriga1/email-manager)

A Rust-based REST API for managing Gmail emails using IMAP with intelligent importance scoring.

## Features

- 📧 Read Gmail emails (recent, today, by date)
- 🔍 Search and filter emails
- ✅ Mark emails as read/unread (single or bulk)
- 🗑️ Delete emails (single or bulk)
- ⭐ Automatic importance scoring (1-3 scale)
- 🔐 Secure IMAP authentication with App Passwords
- 🔑 API token authentication for all endpoints
- 🔢 MFA/2FA code extraction from verification emails

## Setup

### 1. Gmail App Password Setup

This API uses IMAP to access Gmail, which works with both personal Gmail accounts and Google Workspace accounts.

1. **Enable 2-Factor Authentication:**
   - Go to your [Google Account settings](https://myaccount.google.com/security)
   - Enable 2-Step Verification if not already enabled

2. **Create an App Password:**
   - Go to [App passwords page](https://myaccount.google.com/apppasswords)
   - Select "Mail" as the app
   - Generate a 16-character app password
   - Save this password securely

> **Note**: App Passwords are the recommended way to authenticate with Gmail via IMAP, especially for containerized deployments where browser-based OAuth is not feasible.

### 2. Configuration

Create a `.env` file:

```bash
# Gmail credentials
GMAIL_EMAIL=your-email@gmail.com
GMAIL_APP_PASSWORD=your-16-char-app-password

# API Token for authentication
API_TOKEN=your-secure-api-token

# Server configuration
RUST_LOG=info
PORT=8080
```

### 3. Build and Run

```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run the server
cargo run
```

## API Endpoints

All endpoints except `/health` require authentication via Bearer token in the `Authorization` header:

```
Authorization: Bearer <your-api-token>
```

A complete Postman collection is available in [`postman_collection.json`](./postman_collection.json) for easy API testing.

### Email Operations

- `GET /emails/recent?limit=50&fresh=true` - Get recent emails
  - `limit`: Number of emails to return (default: 10)
  - `fresh`: Skip cache and fetch directly from IMAP (default: false)
- `GET /emails/today?min_score=2` - Get today's emails
- `GET /emails/by-date/{YYYY-MM-DD}?min_score=2` - Get emails by date
- `POST /emails/search` - Search emails with query
- `POST /emails/{id}/read` - Mark single email as read
- `POST /emails/{id}/unread` - Mark single email as unread
- `POST /emails/bulk-mark-read?count=50` - Mark multiple emails as read (default: 50, max: 500)
- `DELETE /emails/{id}` - Delete single email
- `POST /emails/bulk-delete` - Delete multiple emails

### MFA Code Extraction

- `GET /mfa/codes?minutes=5&service=Google` - Extract MFA codes from recent emails
  - `minutes`: Time window to search (default: 5)
  - `service`: Optional filter by service name
  - `limit`: Maximum codes to return (default: 20)
- `GET /mfa/latest?service=GitHub` - Get the most recent MFA code
  - Returns the latest verification code found in emails

### Health Check

- `GET /health` - Service health status

### Testing with Postman

1. Import `postman_collection.json` into Postman
2. Update the `base_url` variable if needed (default: `http://localhost:8080`)
3. Start testing the endpoints!

## Importance Scoring

Emails are automatically scored on a 1-3 scale:

- **1 (Low)**: Promotional, newsletters, noreply addresses
- **2 (Normal)**: Regular correspondence
- **3 (High)**: Important contacts, urgent keywords

## Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin

# Run specific test
cargo test test_name
```

## Docker

```bash
# Pull from Docker Hub
docker pull felipemeriga1/email-manager:latest

# Build locally
docker build -t email-manager .

# Run container with environment variables
docker run -p 8080:8080 \
  -e GMAIL_EMAIL=your-email@gmail.com \
  -e GMAIL_APP_PASSWORD=your-app-password \
  -e API_TOKEN=your-secure-api-token \
  -e RUST_LOG=info \
  felipemeriga1/email-manager:latest
```

## CI/CD

This project uses GitHub Actions for continuous integration and deployment to Docker Hub.

### Required GitHub Secrets

To enable automatic Docker Hub deployment, configure these secrets in your GitHub repository settings:

1. **`DOCKER_USERNAME`** - Your Docker Hub username (e.g., `felipemeriga1`)
2. **`DOCKER_PASSWORD`** - Your Docker Hub password or access token
   - To create an access token: Docker Hub → Account Settings → Security → Access Tokens

### CI Pipeline

The CI pipeline automatically:
1. **Format Check** - Ensures code is properly formatted
2. **Clippy Lint** - Runs Rust linter for code quality
3. **Tests** - Runs all unit and integration tests
4. **Build** - Creates release binary
5. **Docker** - Builds and pushes multi-platform images (amd64/arm64)
6. **Security Audit** - Checks for known vulnerabilities

Docker images are tagged as:
- `latest` - Latest main branch build
- `main-{sha}` - Specific commit on main branch

## License

MIT
