use crate::errors::ApiError;
use crate::models::{BulkDeleteRequest, SearchQuery};
use crate::services::imap_service::ImapService;
use crate::services::mfa_extractor::{MfaCode, MfaExtractor};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedEmailService = Arc<Mutex<ImapService>>;

pub async fn get_recent_emails(
    email_service: web::Data<SharedEmailService>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(10);

    let service = email_service.lock().await;

    // Add logging for debugging
    tracing::info!("Fetching {} recent emails", limit);

    let emails = match service.get_recent_emails(limit).await {
        Ok(emails) => emails,
        Err(e) => {
            tracing::error!("Failed to get recent emails: {:?}", e);
            return Err(e);
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len()
    })))
}

pub async fn get_today_emails(
    email_service: web::Data<SharedEmailService>,
) -> Result<HttpResponse, ApiError> {
    let today = Utc::now().date_naive();
    let today_utc = today
        .and_hms_opt(0, 0, 0)
        .ok_or(ApiError::ValidationError("Invalid date".to_string()))?
        .and_utc();

    let service = email_service.lock().await;
    let emails = service.get_emails_by_date(today_utc).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len(),
        "date": today.to_string()
    })))
}

pub async fn get_emails_by_date(
    email_service: web::Data<SharedEmailService>,
    date_str: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|_| ApiError::ValidationError("Invalid date format. Use YYYY-MM-DD".to_string()))?
        .and_hms_opt(0, 0, 0)
        .ok_or(ApiError::ValidationError("Invalid date".to_string()))?
        .and_utc();

    let service = email_service.lock().await;
    let emails = service.get_emails_by_date(date).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "emails": emails,
        "count": emails.len(),
        "date": date_str.into_inner()
    })))
}

pub async fn search_emails(
    email_service: web::Data<SharedEmailService>,
    query: web::Json<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
    if query.query.is_empty() {
        return Err(ApiError::ValidationError(
            "Search query cannot be empty".to_string(),
        ));
    }

    let service = email_service.lock().await;
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
    email_service: web::Data<SharedEmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = email_service.lock().await;
    service.mark_as_read(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email marked as read",
        "email_id": email_id.into_inner()
    })))
}

pub async fn mark_as_unread(
    email_service: web::Data<SharedEmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = email_service.lock().await;
    service.mark_as_unread(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email marked as unread",
        "email_id": email_id.into_inner()
    })))
}

pub async fn delete_email(
    email_service: web::Data<SharedEmailService>,
    email_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let service = email_service.lock().await;
    service.delete_email(&email_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email deleted",
        "email_id": email_id.into_inner()
    })))
}

pub async fn bulk_mark_as_read(
    email_service: web::Data<SharedEmailService>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    // Get count from query params (default to 50)
    let count = query
        .get("count")
        .and_then(|c| c.parse::<u32>().ok())
        .unwrap_or(50);

    // Limit to max 500 for safety
    let count = count.min(500);

    let service = email_service.lock().await;
    let marked_count = service.mark_multiple_as_read(count).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "marked_count": marked_count,
        "requested_count": count,
        "message": format!("Successfully marked {} emails as read", marked_count)
    })))
}

pub async fn bulk_delete(
    email_service: web::Data<SharedEmailService>,
    request: web::Json<BulkDeleteRequest>,
) -> Result<HttpResponse, ApiError> {
    if request.ids.is_empty() {
        return Err(ApiError::ValidationError(
            "No email IDs provided for deletion".to_string(),
        ));
    }

    let service = email_service.lock().await;
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

#[derive(Deserialize)]
pub struct MfaQueryParams {
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default = "default_minutes")]
    minutes: u32,
    service: Option<String>,
}

fn default_limit() -> u32 {
    20
}

fn default_minutes() -> u32 {
    5
}

pub async fn get_mfa_codes(
    email_service: web::Data<SharedEmailService>,
    query: web::Query<MfaQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let service = email_service.lock().await;

    // Get recent emails (look at more emails to find MFA codes)
    let search_limit = query.limit.clamp(20, 100);
    tracing::info!("Searching for MFA codes in {} recent emails", search_limit);

    let emails = service.get_recent_emails(search_limit).await?;

    // Filter emails by time window (only look at emails from the last X minutes)
    let cutoff_time = Utc::now() - chrono::Duration::minutes(query.minutes as i64);
    let recent_emails: Vec<_> = emails
        .into_iter()
        .filter(|email| email.date >= cutoff_time)
        .collect();

    tracing::info!("Found {} emails within the last {} minutes", recent_emails.len(), query.minutes);

    // Extract MFA codes from emails
    let mut all_codes = Vec::new();

    for email in recent_emails {
        // Use full body if available, otherwise fall back to snippet
        let text_to_search = email.body.as_deref().unwrap_or(&email.snippet);

        let codes = MfaExtractor::extract_codes(
            &email.id,
            Some(&email.subject),
            Some(&email.sender_email),
            Some(text_to_search),
            email.date,
        );

        // If we found codes and there's a service filter, apply it
        if let Some(ref filter_service) = query.service {
            let filter_lower = filter_service.to_lowercase();
            for code in codes {
                if let Some(ref service) = code.service {
                    if service.to_lowercase().contains(&filter_lower) {
                        all_codes.push(code);
                    }
                }
            }
        } else {
            all_codes.extend(codes);
        }
    }

    // Sort by date (newest first)
    all_codes.sort_by(|a, b| b.email_date.cmp(&a.email_date));

    // Limit the number of codes returned
    all_codes.truncate(query.limit as usize);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "codes": all_codes,
        "count": all_codes.len(),
        "search_window_minutes": query.minutes,
        "service_filter": query.service
    })))
}

pub async fn get_latest_mfa_code(
    email_service: web::Data<SharedEmailService>,
    query: web::Query<MfaQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let service = email_service.lock().await;

    // Get recent emails
    let search_limit = 50; // Look at up to 50 emails to find the latest code
    let emails = service.get_recent_emails(search_limit).await?;

    // Filter emails by time window
    let cutoff_time = Utc::now() - chrono::Duration::minutes(query.minutes as i64);

    // Look for the first (most recent) email with an MFA code
    for email in emails {
        if email.date < cutoff_time {
            break; // Stop if we've gone past the time window
        }

        // Use full body if available, otherwise fall back to snippet
        let text_to_search = email.body.as_deref().unwrap_or(&email.snippet);

        let codes = MfaExtractor::extract_codes(
            &email.id,
            Some(&email.subject),
            Some(&email.sender_email),
            Some(text_to_search),
            email.date,
        );

        if !codes.is_empty() {
            // If there's a service filter, check if it matches
            if let Some(ref filter_service) = query.service {
                let filter_lower = filter_service.to_lowercase();
                for code in codes {
                    if let Some(ref service) = code.service {
                        if service.to_lowercase().contains(&filter_lower) {
                            return Ok(HttpResponse::Ok().json(code));
                        }
                    }
                }
            } else {
                // Return the first code found
                return Ok(HttpResponse::Ok().json(&codes[0]));
            }
        }
    }

    // No MFA code found
    Err(ApiError::NotFound(format!(
        "No MFA code found in emails from the last {} minutes{}",
        query.minutes,
        if let Some(ref s) = query.service {
            format!(" for service: {}", s)
        } else {
            String::new()
        }
    )))
}
