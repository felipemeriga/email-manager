# Gmail Manager API Design

## Overview
A Rust-based REST API for managing Gmail emails with intelligent importance scoring and comprehensive email operations.

## Core Requirements
- Read Gmail emails (recent, today, by date range)
- Mark emails as read/unread
- Search and filter emails
- Delete emails
- Simple importance scoring (1-3 scale)
- Single Gmail account via Service Account auth
- Test-Driven Development approach

## Architecture

### Technology Stack
- **Framework**: Actix-web (async, production-ready)
- **Gmail Integration**: google-gmail1 crate (official Google API)
- **Authentication**: Service Account with JWT
- **Testing**: Built-in Rust testing with mockall for mocks
- **Error Handling**: anyhow + thiserror for type-safe errors
- **Logging**: tracing + tracing-subscriber
- **Configuration**: config crate with TOML files

### Project Structure
```
email-manager/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Configuration management
│   ├── handlers/         # REST endpoint handlers
│   ├── services/         # Business logic
│   │   ├── gmail.rs     # Gmail API integration
│   │   └── scoring.rs   # Email importance scoring
│   ├── models/          # Data structures
│   └── errors/          # Error types
├── tests/               # Integration tests
├── config/             # Configuration files
│   └── service-account.json
└── Cargo.toml
```

## API Endpoints

### Email Operations

#### Get Recent Emails
```
GET /emails/recent?limit=50
```
Returns the most recent emails (default: 50)

#### Get Today's Emails
```
GET /emails/today?min_score=2
```
Returns all emails from today, optionally filtered by minimum importance score

#### Get Emails by Date
```
GET /emails/date/{date}?min_score=2
```
Returns emails from a specific date (YYYY-MM-DD format)

#### Get Emails by Date Range
```
GET /emails/range?from={date}&to={date}&min_score=1
```
Returns emails within a date range

#### Search Emails
```
GET /emails/search?query={string}&min_score=1
```
Search emails by sender, subject, or content

#### Mark as Read
```
POST /emails/{id}/mark-read
```
Marks a specific email as read

#### Mark as Unread
```
POST /emails/{id}/mark-unread
```
Marks a specific email as unread

#### Delete Single Email
```
DELETE /emails/{id}
```
Moves an email to trash

#### Delete Multiple Emails
```
DELETE /emails/bulk
Body: {"ids": ["id1", "id2", "id3"]}
```
Deletes multiple emails at once

## Data Models

### EmailSummary
```rust
pub struct EmailSummary {
    pub id: String,
    pub subject: String,
    pub sender: String,
    pub sender_email: String,
    pub date: DateTime<Utc>,
    pub snippet: String,
    pub is_read: bool,
    pub labels: Vec<String>,
    pub importance_score: u8,
}
```

### ImportanceScore
```rust
pub enum ImportanceScore {
    Low = 1,    // Spam, promotions, newsletters
    Normal = 2, // Regular correspondence
    High = 3,   // Important contacts, urgent keywords
}
```

## Email Importance Scoring

### Simple Rule-Based System

The scoring system uses a straightforward rule hierarchy:

1. **Score 1 (Low Priority)**
   - Emails in SPAM or PROMOTIONS categories
   - Senders containing: "noreply", "newsletter", "marketing", "promo"
   - Automated notifications

2. **Score 2 (Normal Priority)**
   - Default score for regular emails
   - Personal or business correspondence
   - Emails not matching other rules

3. **Score 3 (High Priority)**
   - Senders in contacts/important domains list (configured)
   - Subject contains: "urgent", "important", "action required", "asap"
   - Emails marked as important by Gmail

### Configuration
Important domains and keywords are configurable via `config/scoring.toml`:
```toml
[scoring]
important_domains = ["work.com", "client.com"]
important_keywords = ["urgent", "important", "asap"]
spam_indicators = ["noreply", "newsletter", "marketing"]
```

## Authentication Setup

### Service Account Configuration
1. Create service account in Google Cloud Console
2. Enable Gmail API
3. Download JSON key file
4. Grant domain-wide delegation (if using Google Workspace)
5. Store key in `config/service-account.json`

### Required Scopes
- `https://www.googleapis.com/auth/gmail.readonly`
- `https://www.googleapis.com/auth/gmail.modify`

## Testing Strategy (TDD)

### Test Levels
1. **Unit Tests**
   - Scoring logic validation
   - Date filtering functions
   - Model serialization

2. **Integration Tests**
   - Gmail API client with mocked responses
   - Endpoint handlers with test fixtures
   - Error handling scenarios

3. **E2E Tests**
   - Real Gmail test account (optional)
   - Full request/response cycle
   - Performance benchmarks

### Test Coverage Goals
- Minimum 80% code coverage
- 100% coverage for scoring logic
- All error paths tested

## Error Handling

### Error Types
- `AuthenticationError` - Service account issues
- `ApiError` - Gmail API failures
- `ValidationError` - Invalid input data
- `NotFoundError` - Email not found
- `RateLimitError` - API quota exceeded

### Error Responses
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid date format",
    "details": "Expected YYYY-MM-DD"
  }
}
```

## Security Considerations

1. **Service Account Security**
   - Never commit service account JSON
   - Use environment variables for production
   - Rotate keys periodically

2. **API Security**
   - Optional API key authentication
   - Rate limiting per endpoint
   - Request validation

3. **Data Privacy**
   - No email content caching
   - Audit logging for all operations
   - Minimal data in responses

## Performance Optimizations

1. **Batch Operations**
   - Batch API requests where possible
   - Bulk delete endpoint

2. **Connection Pooling**
   - Reuse HTTP clients
   - Connection keep-alive

3. **Async Processing**
   - All handlers fully async
   - Non-blocking I/O

## Deployment

### Docker Support
- Multi-stage Dockerfile
- Minimal runtime image
- Health check endpoint

### Environment Variables
```bash
SERVICE_ACCOUNT_PATH=/config/service-account.json
RUST_LOG=info
PORT=8080
```

## Future Enhancements
- Webhook support for real-time updates
- Email attachment handling
- Advanced filtering options
- Caching layer for frequently accessed data
- Machine learning-based importance scoring