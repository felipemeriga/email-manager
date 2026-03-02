# Gmail Manager API

[![Rust CI](https://github.com/felipemeriga/email-manager/actions/workflows/rust.yml/badge.svg)](https://github.com/felipemeriga/email-manager/actions/workflows/rust.yml)
[![Docker Hub](https://img.shields.io/docker/v/felipemeriga1/email-manager?label=docker&sort=semver)](https://hub.docker.com/r/felipemeriga1/email-manager)

A Rust-based REST API for managing Gmail emails with intelligent importance scoring.

## Features

- 📧 Read Gmail emails (recent, today, by date)
- 🔍 Search and filter emails
- ✅ Mark emails as read/unread
- 🗑️ Delete emails (single or bulk)
- ⭐ Automatic importance scoring (1-3 scale)
- 🔐 Secure Service Account authentication

## Setup

### 1. Google Cloud Setup

First, enable the Gmail API:
1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Create or select a project
3. Enable the Gmail API in "APIs & Services" → "Library"

### 2. Authentication Setup

> **⚠️ IMPORTANT**: Service accounts CANNOT access personal Gmail (gmail.com) accounts!

#### For Google Workspace Accounts (Company/Organization Email)

If you have a Google Workspace account (e.g., user@yourcompany.com):

1. **Create Service Account:**
   - In Google Cloud Console → "IAM & Admin" → "Service Accounts"
   - Create a service account
   - Download the JSON key as `service-account.json`

2. **Enable Domain-Wide Delegation:**
   - In Google Admin Console → Security → API controls
   - Add the service account with these scopes:
     ```
     https://www.googleapis.com/auth/gmail.readonly
     https://www.googleapis.com/auth/gmail.modify
     ```

3. **Configure the app:**
   ```bash
   export GMAIL_SERVICE_ACCOUNT_PATH=service-account.json
   export GMAIL_USER_EMAIL=user@yourdomain.com  # Required for impersonation
   export RUST_LOG=info
   ```

#### For Personal Gmail Accounts (gmail.com)

**Container deployment with personal Gmail is challenging** because:
- Service accounts don't work with personal Gmail
- OAuth2 requires browser interaction (not available in containers)

**Workarounds:**
1. **Use Google Workspace instead** (recommended)
2. **Pre-authorize locally and mount token**:
   - Run the app locally with OAuth2 first
   - Authorize in browser and generate `tokencache.json`
   - Mount this token file in your container
3. **Consider using IMAP/SMTP** instead of Gmail API for personal accounts

### 3. Configuration

Create a `.env` file:

```bash
# For Google Workspace (Service Account)
GMAIL_SERVICE_ACCOUNT_PATH=service-account.json
GMAIL_USER_EMAIL=user@yourcompany.com  # Required for Workspace
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

A complete Postman collection is available in [`postman_collection.json`](./postman_collection.json) for easy API testing.

### Email Operations

- `GET /emails/recent?limit=50` - Get recent emails
- `GET /emails/today?min_score=2` - Get today's emails
- `GET /emails/by-date/{YYYY-MM-DD}?min_score=2` - Get emails by date
- `POST /emails/search` - Search emails with query
- `POST /emails/{id}/read` - Mark as read
- `POST /emails/{id}/unread` - Mark as unread
- `DELETE /emails/{id}` - Delete single email
- `POST /emails/bulk-delete` - Delete multiple emails

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
docker build -t gmail-manager .

# Run container
docker run -p 8080:8080 -v $(pwd)/config:/app/config felipemeriga1/email-manager:latest
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
