# Gmail Manager API

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

1. Create a project in Google Cloud Console
2. Enable Gmail API
3. Create a Service Account
4. Download the JSON key file
5. Place it in `config/service-account.json`

### 2. Configuration

Copy `.env.example` to `.env` and update:

```bash
SERVICE_ACCOUNT_PATH=config/service-account.json
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

### Email Operations

- `GET /emails/recent?limit=50` - Get recent emails
- `GET /emails/today?min_score=2` - Get today's emails
- `GET /emails/date/{YYYY-MM-DD}?min_score=2` - Get emails by date
- `GET /emails/search?query=from:john&min_score=1` - Search emails
- `POST /emails/{id}/mark-read` - Mark as read
- `POST /emails/{id}/mark-unread` - Mark as unread
- `DELETE /emails/{id}` - Delete single email
- `DELETE /emails/bulk` - Delete multiple emails

### Health Check

- `GET /health` - Service health status

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
# Build image
docker build -t gmail-manager .

# Run container
docker run -p 8080:8080 -v $(pwd)/config:/app/config gmail-manager
```

## License

MIT
