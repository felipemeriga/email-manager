use crate::errors::ApiError;
use crate::models::{BulkDeleteRequest, SearchQuery};
use crate::services::gmail::GmailService;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedGmailService = Arc<Mutex<GmailService>>;

pub async fn get_recent_emails(
    gmail_service: web::Data<SharedGmailService>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(10);

    let service = gmail_service.lock().await;
    let emails = service.get_recent_emails(limit).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len()
    })))
}

pub async fn get_today_emails(
    gmail_service: web::Data<SharedGmailService>,
) -> Result<HttpResponse, ApiError> {
    let today = Utc::now().date_naive();
    let today_utc = today
        .and_hms_opt(0, 0, 0)
        .ok_or(ApiError::ValidationError("Invalid date".to_string()))?
        .and_utc();

    let service = gmail_service.lock().await;
    let emails = service.get_emails_by_date(today_utc).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len(),
        "date": today.to_string()
    })))
}

pub async fn get_emails_by_date(
    gmail_service: web::Data<SharedGmailService>,
    date_str: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|_| ApiError::ValidationError("Invalid date format. Use YYYY-MM-DD".to_string()))?
        .and_hms_opt(0, 0, 0)
        .ok_or(ApiError::ValidationError("Invalid date".to_string()))?
        .and_utc();

    let service = gmail_service.lock().await;
    let emails = service.get_emails_by_date(date).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len(),
        "date": date_str.into_inner()
    })))
}

pub async fn search_emails(
    gmail_service: web::Data<SharedGmailService>,
    query: web::Json<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
    if query.query.is_empty() {
        return Err(ApiError::ValidationError(
            "Search query cannot be empty".to_string(),
        ));
    }

    let service = gmail_service.lock().await;
    let mut emails = service.search_emails(&query.query).await?;

    // Filter by minimum score if specified
    if query.min_score > 1 {
        emails.retain(|email| email.importance_score >= query.min_score);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len(),
        "query": query.query
    })))
}

pub async fn mark_as_read(
    gmail_service: web::Data<SharedGmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = gmail_service.lock().await;
    service.mark_as_read(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email marked as read",
        "email_id": email_id.into_inner()
    })))
}

pub async fn mark_as_unread(
    gmail_service: web::Data<SharedGmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = gmail_service.lock().await;
    service.mark_as_unread(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email marked as unread",
        "email_id": email_id.into_inner()
    })))
}

pub async fn delete_email(
    gmail_service: web::Data<SharedGmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = gmail_service.lock().await;
    service.delete_email(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email deleted",
        "email_id": email_id.into_inner()
    })))
}

pub async fn bulk_delete(
    gmail_service: web::Data<SharedGmailService>,
    request: web::Json<BulkDeleteRequest>,
) -> Result<HttpResponse, ApiError> {
    if request.ids.is_empty() {
        return Err(ApiError::ValidationError(
            "No email IDs provided for deletion".to_string(),
        ));
    }

    let service = gmail_service.lock().await;
    let mut deleted_count = 0;
    let mut failed_ids = Vec::new();

    for email_id in &request.ids {
        match service.delete_email(email_id).await {
            Ok(_) => deleted_count += 1,
            Err(_) => failed_ids.push(email_id.clone()),
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "deleted": deleted_count,
        "failed": failed_ids.len(),
        "failed_ids": failed_ids
    })))
}
