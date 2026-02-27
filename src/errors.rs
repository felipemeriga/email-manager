use actix_web::{error::ResponseError, HttpResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Gmail API error: {0}")]
    GmailApiError(String),

    #[error("Email not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    ValidationError(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Internal server error")]
    InternalError,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(_) => HttpResponse::NotFound().json(serde_json::json!({
                "error": {
                    "code": "NOT_FOUND",
                    "message": self.to_string()
                }
            })),
            ApiError::ValidationError(_) => HttpResponse::BadRequest().json(serde_json::json!({
                "error": {
                    "code": "VALIDATION_ERROR",
                    "message": self.to_string()
                }
            })),
            ApiError::AuthenticationError(_) => HttpResponse::Unauthorized().json(serde_json::json!({
                "error": {
                    "code": "AUTHENTICATION_ERROR",
                    "message": self.to_string()
                }
            })),
            ApiError::RateLimitError => HttpResponse::TooManyRequests().json(serde_json::json!({
                "error": {
                    "code": "RATE_LIMIT_ERROR",
                    "message": self.to_string()
                }
            })),
            _ => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "An internal error occurred"
                }
            })),
        }
    }
}
