use crate::errors::ApiError;
use crate::models::EmailSummary;
use crate::services::scoring::EmailScorer;
use anyhow::Result;
use chrono::{DateTime, Utc};
use google_gmail1::api::ModifyMessageRequest;
use google_gmail1::{
    hyper, hyper_rustls,
    oauth2::{read_service_account_key, ServiceAccountAuthenticator},
    Gmail,
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GmailService {
    hub: Gmail<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    scorer: Arc<Mutex<EmailScorer>>,
}

impl GmailService {
    pub async fn new(service_account_path: &str) -> Result<Self, ApiError> {
        let secret = read_service_account_key(service_account_path)
            .await
            .map_err(|e| {
                ApiError::AuthenticationError(format!("Failed to read service account: {}", e))
            })?;

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_only()
            .enable_http1()
            .build();

        let client = hyper::Client::builder().build(https);

        let auth = ServiceAccountAuthenticator::builder(secret)
            .build()
            .await
            .map_err(|e| {
                ApiError::AuthenticationError(format!("Failed to create authenticator: {}", e))
            })?;

        let hub = Gmail::new(client, auth);
        let scorer = Arc::new(Mutex::new(EmailScorer::new()));

        Ok(Self { hub, scorer })
    }

    pub async fn get_recent_emails(&self, limit: u32) -> Result<Vec<EmailSummary>, ApiError> {
        let messages = self
            .hub
            .users()
            .messages_list("me")
            .max_results(limit)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to list messages: {}", e)))?
            .1;

        let mut emails = Vec::new();

        if let Some(message_list) = messages.messages {
            for message in message_list {
                if let Some(id) = message.id {
                    if let Ok(email) = self.get_email_by_id(&id).await {
                        emails.push(email);
                    }
                }
            }
        }

        Ok(emails)
    }

    pub async fn get_emails_by_date(
        &self,
        date: DateTime<Utc>,
    ) -> Result<Vec<EmailSummary>, ApiError> {
        let date_str = date.format("%Y/%m/%d").to_string();
        let query = format!("after:{}", date_str);

        let messages = self
            .hub
            .users()
            .messages_list("me")
            .q(&query)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to search messages: {}", e)))?
            .1;

        let mut emails = Vec::new();

        if let Some(message_list) = messages.messages {
            for message in message_list {
                if let Some(id) = message.id {
                    if let Ok(email) = self.get_email_by_id(&id).await {
                        emails.push(email);
                    }
                }
            }
        }

        Ok(emails)
    }

    pub async fn search_emails(&self, query: &str) -> Result<Vec<EmailSummary>, ApiError> {
        let messages = self
            .hub
            .users()
            .messages_list("me")
            .q(query)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to search messages: {}", e)))?
            .1;

        let mut emails = Vec::new();

        if let Some(message_list) = messages.messages {
            for message in message_list {
                if let Some(id) = message.id {
                    if let Ok(email) = self.get_email_by_id(&id).await {
                        emails.push(email);
                    }
                }
            }
        }

        Ok(emails)
    }

    async fn get_email_by_id(&self, message_id: &str) -> Result<EmailSummary, ApiError> {
        let message = self
            .hub
            .users()
            .messages_get("me", message_id)
            .format("full")
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to get message: {}", e)))?
            .1;

        self.parse_message(message).await
    }

    async fn parse_message(
        &self,
        message: google_gmail1::api::Message,
    ) -> Result<EmailSummary, ApiError> {
        let message_id = message.id.clone().unwrap_or_default();
        let snippet = message.snippet.clone().unwrap_or_default();
        let label_ids = message.label_ids.clone().unwrap_or_default();

        // Extract headers
        let mut subject = String::new();
        let mut sender = String::new();
        let mut sender_email = String::new();
        let mut date_str = String::new();

        if let Some(payload) = &message.payload {
            if let Some(headers) = &payload.headers {
                for header in headers {
                    match header.name.as_deref() {
                        Some("Subject") => subject = header.value.clone().unwrap_or_default(),
                        Some("From") => {
                            let from_value = header.value.clone().unwrap_or_default();
                            // Parse "Name <email@domain.com>" format
                            if let Some(email_start) = from_value.find('<') {
                                if let Some(email_end) = from_value.find('>') {
                                    sender_email =
                                        from_value[email_start + 1..email_end].to_string();
                                    sender = from_value[..email_start].trim().to_string();
                                    if sender.is_empty() {
                                        sender = sender_email.clone();
                                    }
                                } else {
                                    sender_email = from_value.clone();
                                    sender = from_value;
                                }
                            } else {
                                sender_email = from_value.clone();
                                sender = from_value;
                            }
                        }
                        Some("Date") => date_str = header.value.clone().unwrap_or_default(),
                        _ => {}
                    }
                }
            }
        }

        // Parse date
        let date = chrono::DateTime::parse_from_rfc2822(&date_str)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        // Check if read
        let is_read = !label_ids.contains(&"UNREAD".to_string());

        // Calculate importance score
        let scorer = self.scorer.lock().await;
        let label_strs: Vec<&str> = label_ids.iter().map(|s| s.as_str()).collect();
        let importance_score = scorer.calculate_score(&sender_email, &subject, &label_strs);

        Ok(EmailSummary {
            id: message_id,
            subject,
            sender,
            sender_email,
            date,
            snippet,
            is_read,
            labels: label_ids,
            importance_score,
        })
    }

    pub async fn mark_as_read(&self, email_id: &str) -> Result<(), ApiError> {
        let modify_request = ModifyMessageRequest {
            remove_label_ids: Some(vec!["UNREAD".to_string()]),
            ..Default::default()
        };

        self.hub
            .users()
            .messages_modify(modify_request, "me", email_id)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to mark as read: {}", e)))?;

        Ok(())
    }

    pub async fn mark_as_unread(&self, email_id: &str) -> Result<(), ApiError> {
        let modify_request = ModifyMessageRequest {
            add_label_ids: Some(vec!["UNREAD".to_string()]),
            ..Default::default()
        };

        self.hub
            .users()
            .messages_modify(modify_request, "me", email_id)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to mark as unread: {}", e)))?;

        Ok(())
    }

    pub async fn delete_email(&self, email_id: &str) -> Result<(), ApiError> {
        self.hub
            .users()
            .messages_delete("me", email_id)
            .doit()
            .await
            .map_err(|e| ApiError::GmailApiError(format!("Failed to delete message: {}", e)))?;

        Ok(())
    }
}
