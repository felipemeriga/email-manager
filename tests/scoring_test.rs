use email_manager::services::scoring::EmailScorer;

#[test]
fn test_score_promotional_email() {
    let scorer = EmailScorer::new();

    let score = scorer.calculate_score("newsletter@company.com", "50% off sale!", &["PROMOTIONS"]);
    assert_eq!(score, 1);
}

#[test]
fn test_score_noreply_email() {
    let scorer = EmailScorer::new();

    let score =
        scorer.calculate_score("noreply@service.com", "Your order confirmation", &["INBOX"]);
    assert_eq!(score, 1);
}

#[test]
fn test_score_urgent_email() {
    let scorer = EmailScorer::new();

    let score = scorer.calculate_score("boss@work.com", "URGENT: Need this by EOD", &["INBOX"]);
    assert_eq!(score, 3);
}

#[test]
fn test_score_important_domain() {
    let mut scorer = EmailScorer::new();
    scorer.add_important_domain("work.com");

    let score = scorer.calculate_score("colleague@work.com", "Meeting notes", &["INBOX"]);
    assert_eq!(score, 3);
}

#[test]
fn test_score_regular_email() {
    let scorer = EmailScorer::new();

    let score = scorer.calculate_score("friend@gmail.com", "Hey, how are you?", &["INBOX"]);
    assert_eq!(score, 2);
}
