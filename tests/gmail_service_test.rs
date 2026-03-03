use email_manager::services::imap_service::ImapService;

#[tokio::test]
async fn test_imap_service_creation() {
    // Test creating an IMAP service instance
    let email = "test@gmail.com".to_string();
    let password = "test-password".to_string();

    let _service = ImapService::new(email.clone(), password);

    // Service should be created with the provided credentials
    // Actual connection testing would require valid credentials
    // This just verifies the service can be instantiated
}

#[tokio::test]
async fn test_imap_connection_with_invalid_credentials() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let email = "invalid@gmail.com".to_string();
    let password = "invalid-password".to_string();

    let service = Arc::new(Mutex::new(ImapService::new(email, password)));

    // Attempt to fetch emails with invalid credentials
    let result = service.lock().await.get_recent_emails(10).await;

    // Should return an error with invalid credentials
    assert!(result.is_err(), "Should fail with invalid credentials");
}
