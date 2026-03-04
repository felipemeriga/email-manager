use crate::models::EmailSummary;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache for emails to avoid repeated IMAP fetches
pub struct EmailCache {
    /// Map of email ID to email data
    emails: Arc<RwLock<HashMap<String, CachedEmail>>>,
    /// Cache TTL in seconds
    ttl_seconds: i64,
}

#[derive(Clone)]
struct CachedEmail {
    email: EmailSummary,
    fetched_at: DateTime<Utc>,
}

impl EmailCache {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            emails: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    /// Get an email from cache if it's still valid
    pub async fn get(&self, email_id: &str) -> Option<EmailSummary> {
        let cache = self.emails.read().await;

        if let Some(cached) = cache.get(email_id) {
            let age = Utc::now() - cached.fetched_at;
            if age.num_seconds() < self.ttl_seconds {
                return Some(cached.email.clone());
            }
        }

        None
    }

    /// Store emails in cache
    pub async fn put_many(&self, emails: Vec<EmailSummary>) {
        let mut cache = self.emails.write().await;
        let now = Utc::now();

        for email in emails {
            let cached = CachedEmail {
                email: email.clone(),
                fetched_at: now,
            };
            cache.insert(email.id.clone(), cached);
        }
    }

    /// Get multiple cached emails that are still valid
    pub async fn get_recent(&self, limit: usize) -> Vec<EmailSummary> {
        let cache = self.emails.read().await;
        let now = Utc::now();

        let mut valid_emails: Vec<_> = cache
            .values()
            .filter(|cached| {
                let age = now - cached.fetched_at;
                age.num_seconds() < self.ttl_seconds
            })
            .map(|cached| cached.email.clone())
            .collect();

        // Sort by date, most recent first
        valid_emails.sort_by(|a, b| b.date.cmp(&a.date));
        valid_emails.truncate(limit);

        valid_emails
    }

    /// Clear expired entries from cache
    pub async fn clean_expired(&self) {
        let mut cache = self.emails.write().await;
        let now = Utc::now();

        cache.retain(|_, cached| {
            let age = now - cached.fetched_at;
            age.num_seconds() < self.ttl_seconds
        });
    }

    /// Check if we have recent enough cached data
    pub async fn has_recent_data(&self, required_count: usize, max_age_seconds: i64) -> bool {
        let cache = self.emails.read().await;
        let now = Utc::now();

        let recent_count = cache
            .values()
            .filter(|cached| {
                let age = now - cached.fetched_at;
                age.num_seconds() < max_age_seconds
            })
            .count();

        recent_count >= required_count
    }
}
