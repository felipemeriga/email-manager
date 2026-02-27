use crate::errors::ApiError;
use crate::models::EmailSummary;
use anyhow::Result;
use chrono::{DateTime, Utc};
use google_gmail1::{
    hyper, hyper_rustls,
    oauth2::{read_service_account_key, ServiceAccountAuthenticator},
    Gmail,
};

pub struct GmailService {
    hub: Gmail<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl GmailService {
    pub async fn new(service_account_path: &str) -> Result<Self, ApiError> {
        let secret = read_service_account_key(service_account_path)
            .await
            .map_err(|e| ApiError::AuthenticationError(format!("Failed to read service account: {}", e)))?;

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
            .map_err(|e| ApiError::AuthenticationError(format!("Failed to create authenticator: {}", e)))?;

        let hub = Gmail::new(client, auth);

        Ok(Self { hub })
    }

    pub async fn get_recent_emails(&self, _limit: u32) -> Result<Vec<EmailSummary>, ApiError> {
        // Implementation in next task
        Ok(Vec::new())
    }

    pub async fn get_emails_by_date(&self, _date: DateTime<Utc>) -> Result<Vec<EmailSummary>, ApiError> {
        // Implementation in next task
        Ok(Vec::new())
    }

    pub async fn mark_as_read(&self, _email_id: &str) -> Result<(), ApiError> {
        // Implementation in next task
        Ok(())
    }

    pub async fn mark_as_unread(&self, _email_id: &str) -> Result<(), ApiError> {
        // Implementation in next task
        Ok(())
    }

    pub async fn delete_email(&self, _email_id: &str) -> Result<(), ApiError> {
        // Implementation in next task
        Ok(())
    }
}
