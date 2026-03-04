use crate::errors::ApiError;
use crate::models::EmailSummary;
use crate::services::connection_pool::ImapConnectionPool;
use crate::services::email_cache::EmailCache;
use crate::services::scoring::EmailScorer;
use anyhow::Result;
use chrono::{DateTime, Utc};
use imap::Session;
use native_tls::TlsStream;
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ImapService {
    pool: Arc<ImapConnectionPool>,
    cache: Arc<EmailCache>,
    scorer: Arc<Mutex<EmailScorer>>,
}

impl ImapService {
    pub fn new(email: String, password: String) -> Self {
        let pool = Arc::new(ImapConnectionPool::new(email, password));
        let cache = Arc::new(EmailCache::new(300)); // 5 minute TTL

        Self {
            pool,
            cache,
            scorer: Arc::new(Mutex::new(EmailScorer::new())),
        }
    }

    pub async fn get_recent_emails(&self, limit: u32) -> Result<Vec<EmailSummary>, ApiError> {
        // Check if we should use cache or fetch new emails
        // Only use cache if we have very recent data (less than 30 seconds old)
        let cache_max_age_seconds = 30; // Only trust cache for 30 seconds for recent emails

        // Check if cache has recent enough data
        if self
            .cache
            .has_recent_data(limit as usize, cache_max_age_seconds)
            .await
        {
            let cached_emails = self.cache.get_recent(limit as usize).await;
            if cached_emails.len() >= limit as usize {
                tracing::debug!("Returning {} emails from cache", cached_emails.len());
                return Ok(cached_emails);
            }
        }

        tracing::info!("Cache miss or stale, fetching fresh emails from IMAP");

        // Get a connection from the pool
        let mut session = self.pool.get().await?;

        // Calculate date for recent emails (e.g., last 30 days)
        let days_back = 30;
        let since_date = (Utc::now() - chrono::Duration::days(days_back))
            .format("%d-%b-%Y")
            .to_string();

        // Search for messages from the last 30 days
        let search_query = format!("SINCE {}", since_date);
        let messages = session
            .search(&search_query)
            .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?;

        // If we have too few messages, expand the search
        let messages = if messages.len() < limit as usize {
            // Try getting more messages with a broader search
            let search_query = "ALL";
            session
                .search(search_query)
                .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?
        } else {
            messages
        };

        // Convert to vector and reverse to get most recent first (higher UIDs are more recent)
        let mut messages: Vec<_> = messages.into_iter().collect();
        messages.sort_by(|a, b| b.cmp(a)); // Sort UIDs in descending order

        // We need to fetch more than requested to ensure we get enough after sorting by date
        let fetch_count = (limit * 2).min(messages.len() as u32);
        let messages: Vec<_> = messages.into_iter().take(fetch_count as usize).collect();

        let mut emails = Vec::new();
        for uid in messages {
            if let Ok(email) = self.fetch_email(&mut session, uid).await {
                emails.push(email);
            }
        }

        // Sort emails by date, most recent first
        emails.sort_by(|a, b| b.date.cmp(&a.date));

        // Take only the requested limit after sorting
        emails.truncate(limit as usize);

        // Cache the fetched emails
        self.cache.put_many(emails.clone()).await;

        // Return connection to pool instead of logging out
        self.pool.return_connection(session).await;

        Ok(emails)
    }

    pub async fn get_emails_by_date(
        &self,
        date: DateTime<Utc>,
    ) -> Result<Vec<EmailSummary>, ApiError> {
        let mut session = self.pool.get().await?;

        // Format date for IMAP search
        let date_str = date.format("%d-%b-%Y").to_string();
        let search_query = format!("ON {}", date_str);

        let messages = session
            .search(&search_query)
            .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?;

        let mut emails = Vec::new();
        for uid in messages {
            if let Ok(email) = self.fetch_email(&mut session, uid).await {
                emails.push(email);
            }
        }

        // Sort emails by date, most recent first
        emails.sort_by(|a, b| b.date.cmp(&a.date));

        // Cache the fetched emails
        self.cache.put_many(emails.clone()).await;

        // Return connection to pool
        self.pool.return_connection(session).await;

        Ok(emails)
    }

    pub async fn search_emails(&self, query: &str) -> Result<Vec<EmailSummary>, ApiError> {
        let mut session = self.pool.get().await?;

        // Convert Gmail-style query to IMAP
        // Simple conversion - in production you'd want more sophisticated parsing
        let imap_query = if query.starts_with("from:") {
            format!("FROM \"{}\"", query.trim_start_matches("from:"))
        } else if query.starts_with("subject:") {
            format!("SUBJECT \"{}\"", query.trim_start_matches("subject:"))
        } else {
            format!("TEXT \"{}\"", query)
        };

        let messages = session
            .search(&imap_query)
            .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?;

        // Sort UIDs in descending order (most recent first)
        let mut messages: Vec<_> = messages.into_iter().collect();
        messages.sort_by(|a, b| b.cmp(a));

        // Fetch more messages to ensure we have enough after date sorting
        let fetch_limit = 100.min(messages.len());
        let mut emails = Vec::new();
        for uid in messages.into_iter().take(fetch_limit) {
            if let Ok(email) = self.fetch_email(&mut session, uid).await {
                emails.push(email);
            }
        }

        // Sort emails by date, most recent first
        emails.sort_by(|a, b| b.date.cmp(&a.date));

        // Return up to 50 most recent emails
        emails.truncate(50);

        // Cache the fetched emails
        self.cache.put_many(emails.clone()).await;

        // Return connection to pool
        self.pool.return_connection(session).await;

        Ok(emails)
    }

    pub async fn mark_as_read(&self, message_id: &str) -> Result<(), ApiError> {
        let mut session = self.pool.get().await?;

        let uid: u32 = message_id
            .parse()
            .map_err(|_| ApiError::ValidationError("Invalid message ID".to_string()))?;

        session
            .store(format!("{}", uid), "+FLAGS (\\Seen)")
            .map_err(|e| ApiError::InternalError(format!("Failed to mark as read: {}", e)))?;

        self.pool.return_connection(session).await;
        Ok(())
    }

    pub async fn mark_as_unread(&self, message_id: &str) -> Result<(), ApiError> {
        let mut session = self.pool.get().await?;

        let uid: u32 = message_id
            .parse()
            .map_err(|_| ApiError::ValidationError("Invalid message ID".to_string()))?;

        session
            .store(format!("{}", uid), "-FLAGS (\\Seen)")
            .map_err(|e| ApiError::InternalError(format!("Failed to mark as unread: {}", e)))?;

        self.pool.return_connection(session).await;
        Ok(())
    }

    pub async fn delete_email(&self, message_id: &str) -> Result<(), ApiError> {
        let mut session = self.pool.get().await?;

        let uid: u32 = message_id
            .parse()
            .map_err(|_| ApiError::ValidationError("Invalid message ID".to_string()))?;

        // Mark as deleted
        session
            .store(format!("{}", uid), "+FLAGS (\\Deleted)")
            .map_err(|e| ApiError::InternalError(format!("Failed to delete: {}", e)))?;

        // Expunge to actually delete
        session
            .expunge()
            .map_err(|e| ApiError::InternalError(format!("Failed to expunge: {}", e)))?;

        self.pool.return_connection(session).await;
        Ok(())
    }

    pub async fn mark_multiple_as_read(&self, count: u32) -> Result<usize, ApiError> {
        let mut session = self.pool.get().await?;

        // Get the most recent unread messages
        let search_query = "UNSEEN";
        let messages = session
            .search(search_query)
            .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?;

        // Sort UIDs in descending order (most recent first)
        let mut messages_vec: Vec<_> = messages.into_iter().collect();
        messages_vec.sort_by(|a, b| b.cmp(a));
        let messages_to_mark: Vec<_> = messages_vec.into_iter().take(count as usize).collect();

        let total_marked = messages_to_mark.len();

        // Mark each message as read
        for uid in &messages_to_mark {
            let _ = session.store(format!("{}", uid), "+FLAGS (\\Seen)");
        }

        self.pool.return_connection(session).await;
        Ok(total_marked)
    }

    pub async fn delete_multiple(&self, ids: Vec<String>) -> Result<usize, ApiError> {
        let mut session = self.pool.get().await?;
        let mut deleted = 0;

        for id in ids {
            if let Ok(uid) = id.parse::<u32>() {
                if session
                    .store(format!("{}", uid), "+FLAGS (\\Deleted)")
                    .is_ok()
                {
                    deleted += 1;
                }
            }
        }

        session
            .expunge()
            .map_err(|e| ApiError::InternalError(format!("Failed to expunge: {}", e)))?;

        self.pool.return_connection(session).await;
        Ok(deleted)
    }

    async fn fetch_email(
        &self,
        session: &mut Session<TlsStream<TcpStream>>,
        uid: u32,
    ) -> Result<EmailSummary, ApiError> {
        let messages = session
            .fetch(format!("{}", uid), "RFC822")
            .map_err(|e| ApiError::InternalError(format!("Fetch failed: {}", e)))?;

        let message = messages
            .iter()
            .next()
            .ok_or_else(|| ApiError::InternalError("No message found".to_string()))?;

        let body = message
            .body()
            .ok_or_else(|| ApiError::InternalError("No message body".to_string()))?;

        // Parse the email
        let parsed = mailparse::parse_mail(body)
            .map_err(|e| ApiError::InternalError(format!("Failed to parse email: {}", e)))?;

        // Extract headers using proper mailparse API
        let from = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("From"))
            .map(|h| h.get_value())
            .unwrap_or_else(String::new);

        let subject = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("Subject"))
            .map(|h| h.get_value())
            .unwrap_or_else(String::new);

        let date_str = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("Date"))
            .map(|h| h.get_value())
            .unwrap_or_else(String::new);

        // Parse date
        let date = if !date_str.is_empty() {
            match chrono::DateTime::parse_from_rfc2822(&date_str) {
                Ok(dt) => dt.with_timezone(&Utc),
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse date '{}': {}, using current time",
                        date_str,
                        e
                    );
                    Utc::now()
                }
            }
        } else {
            tracing::warn!("Email has no date header, using current time");
            Utc::now()
        };

        // Extract body text
        let body_text = parsed.get_body().unwrap_or_else(|_| String::new());
        let snippet = body_text.chars().take(200).collect::<String>();

        // Keep full body for MFA extraction (limit to 5000 chars to avoid huge emails)
        let body = if !body_text.is_empty() {
            Some(body_text.chars().take(5000).collect::<String>())
        } else {
            None
        };

        // Parse sender email
        let sender_email = if from.contains('<') && from.contains('>') {
            from.split('<')
                .nth(1)
                .unwrap_or(&from)
                .trim_end_matches('>')
                .to_string()
        } else {
            from.clone()
        };

        // Parse sender name
        let sender = if from.contains('<') {
            from.split('<').next().unwrap_or(&from).trim().to_string()
        } else {
            from.clone()
        };

        // Check if read - for simplicity, we'll assume all messages in INBOX are unread unless explicitly marked
        // In production, you'd properly handle the flag lifetime issue
        let is_read = false; // Default to unread for now

        // Get labels (folders)
        let labels = vec!["INBOX".to_string()];
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();

        // Score the email
        let scorer = self.scorer.lock().await;
        let importance_score = scorer.calculate_score(&sender_email, &subject, &label_refs);

        Ok(EmailSummary {
            id: uid.to_string(),
            sender,
            sender_email,
            subject,
            snippet,
            body,
            date,
            is_read,
            labels,
            importance_score,
        })
    }

    pub async fn get_today_emails(&self) -> Result<Vec<EmailSummary>, ApiError> {
        let today = Utc::now();
        self.get_emails_by_date(today).await
    }

    /// Get recent emails with forced refresh (bypasses cache)
    /// Used for time-sensitive operations like MFA code retrieval
    pub async fn get_recent_emails_fresh(&self, limit: u32) -> Result<Vec<EmailSummary>, ApiError> {
        tracing::info!("Force fetching fresh emails from IMAP (bypassing cache)");

        // Get a connection from the pool
        let mut session = self.pool.get().await?;

        // Calculate date for recent emails (last 7 days for MFA)
        let days_back = 7;
        let since_date = (Utc::now() - chrono::Duration::days(days_back))
            .format("%d-%b-%Y")
            .to_string();

        // Search for messages from the last 7 days
        let search_query = format!("SINCE {}", since_date);
        let messages = session
            .search(&search_query)
            .map_err(|e| ApiError::InternalError(format!("Search failed: {}", e)))?;

        // Convert to vector and reverse to get most recent first
        let mut messages: Vec<_> = messages.into_iter().collect();
        messages.sort_by(|a, b| b.cmp(a));

        // Fetch requested number of emails
        let fetch_count = limit.min(messages.len() as u32);
        let messages: Vec<_> = messages.into_iter().take(fetch_count as usize).collect();

        let mut emails = Vec::new();
        for uid in messages {
            if let Ok(email) = self.fetch_email(&mut session, uid).await {
                emails.push(email);
            }
        }

        // Sort emails by date, most recent first
        emails.sort_by(|a, b| b.date.cmp(&a.date));

        // Update cache with fresh data
        self.cache.put_many(emails.clone()).await;

        // Return connection to pool
        self.pool.return_connection(session).await;

        Ok(emails)
    }

    pub async fn get_email_by_id(&self, id: &str) -> Result<EmailSummary, ApiError> {
        // First check cache
        if let Some(cached) = self.cache.get(id).await {
            return Ok(cached);
        }

        let mut session = self.pool.get().await?;
        let uid: u32 = id
            .parse()
            .map_err(|_| ApiError::ValidationError("Invalid message ID".to_string()))?;

        let email = self.fetch_email(&mut session, uid).await?;

        // Cache the single email
        self.cache.put_many(vec![email.clone()]).await;

        self.pool.return_connection(session).await;

        Ok(email)
    }
}
