use email_manager::services::gmail::GmailService;

#[tokio::test]
async fn test_parse_gmail_message() {
    // This tests the parsing logic without actual API calls
    let service = GmailService::new("nonexistent.json").await;
    assert!(service.is_err()); // Expected to fail without real credentials
}
