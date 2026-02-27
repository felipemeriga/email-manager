use email_manager::services::gmail::GmailService;

#[tokio::test]
async fn test_gmail_service_creation() {
    let service_account_path = "config/test-account.json";
    let result = GmailService::new(service_account_path).await;

    // Should fail if no service account exists
    assert!(result.is_err());
}
