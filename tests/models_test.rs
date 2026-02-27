use chrono::Utc;
use email_manager::models::{EmailSummary, ImportanceScore};

#[test]
fn test_email_summary_creation() {
    let email = EmailSummary {
        id: "test123".to_string(),
        subject: "Test Subject".to_string(),
        sender: "John Doe".to_string(),
        sender_email: "john@example.com".to_string(),
        date: Utc::now(),
        snippet: "This is a test...".to_string(),
        is_read: false,
        labels: vec!["INBOX".to_string()],
        importance_score: 2,
    };

    assert_eq!(email.importance_score, 2);
    assert_eq!(email.sender, "John Doe");
}

#[test]
fn test_importance_score_values() {
    assert_eq!(ImportanceScore::Low as u8, 1);
    assert_eq!(ImportanceScore::Normal as u8, 2);
    assert_eq!(ImportanceScore::High as u8, 3);
}
