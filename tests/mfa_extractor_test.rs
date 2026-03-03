use chrono::Utc;
use email_manager::services::mfa_extractor::{CodeType, MfaExtractor};

#[test]
fn test_extract_google_verification_code() {
    let body = "Your Google verification code is 123456. Don't share this code with anyone.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Sign in to Google"),
        Some("noreply@google.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "123456");
    assert_eq!(codes[0].service, Some("Google".to_string()));
    assert!(matches!(codes[0].code_type, CodeType::Numeric));
}

#[test]
fn test_extract_microsoft_code() {
    let body = "Use this code for Microsoft verification: 987654";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Microsoft account security code"),
        Some("account-security-noreply@microsoft.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "987654");
    assert_eq!(codes[0].service, Some("Microsoft".to_string()));
}

#[test]
fn test_extract_github_code() {
    let body =
        "Your GitHub authentication code is 456789. Enter this code to verify your identity.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("[GitHub] Please verify your device"),
        Some("noreply@github.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "456789");
    assert_eq!(codes[0].service, Some("GitHub".to_string()));
}

#[test]
fn test_extract_aws_code() {
    let body =
        "Your AWS verification code is: 234567\n\nIf you didn't request this, please ignore.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("AWS Verification Code"),
        Some("no-reply@aws.amazon.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "234567");
    assert_eq!(codes[0].service, Some("AWS".to_string()));
}

#[test]
fn test_extract_hyphenated_code() {
    let body = "Your verification code is 123-456. Enter this code to continue.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Verification Required"),
        Some("security@example.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "123456"); // Hyphen should be removed
}

#[test]
fn test_extract_alphanumeric_code() {
    let body = "Your Discord verification code is: ABC123";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Discord Verification"),
        Some("noreply@discord.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "ABC123");
    assert_eq!(codes[0].service, Some("Discord".to_string()));
    assert!(matches!(codes[0].code_type, CodeType::Alphanumeric));
}

#[test]
fn test_extract_four_digit_pin() {
    let body = "Your PIN code is 1234. This code will expire in 10 minutes.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Security PIN"),
        Some("security@bank.com"),
        Some(body),
        Utc::now(),
    );

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].code, "1234");
}

#[test]
fn test_extract_with_label_patterns() {
    let test_cases = vec![
        ("Code is 555666", "555666"),
        ("Your code: 777888", "777888"),
        ("Enter this code: 999000", "999000"),
        ("Verification code: 111222", "111222"),
        ("Use code 333444", "333444"),
    ];

    for (body, expected_code) in test_cases {
        let codes = MfaExtractor::extract_codes(
            "test-id",
            Some("Verification"),
            Some("noreply@service.com"),
            Some(body),
            Utc::now(),
        );

        assert_eq!(codes.len(), 1, "Failed for body: {}", body);
        assert_eq!(codes[0].code, expected_code, "Failed for body: {}", body);
    }
}

#[test]
fn test_no_code_in_non_verification_email() {
    let body = "Thank you for your order #123456. Your package will arrive soon.";
    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("Order Confirmation"),
        Some("orders@shop.com"),
        Some(body),
        Utc::now(),
    );

    // Should not extract order numbers as verification codes
    assert_eq!(codes.len(), 0);
}

#[test]
fn test_multiple_services_detection() {
    let services = vec![
        ("noreply@facebook.com", "Facebook"),
        ("security@twitter.com", "Twitter"),
        ("no-reply@linkedin.com", "LinkedIn"),
        ("noreply@apple.com", "Apple"),
        ("team@slack.com", "Slack"),
        ("noreply@paypal.com", "PayPal"),
        ("support@coinbase.com", "Coinbase"),
        ("noreply@netflix.com", "Netflix"),
        ("no-reply@spotify.com", "Spotify"),
        ("verify@uber.com", "Uber"),
        ("noreply@zoom.us", "Zoom"),
    ];

    for (sender, expected_service) in services {
        let codes = MfaExtractor::extract_codes(
            "test-id",
            Some("Verification Code"),
            Some(sender),
            Some("Your verification code is 654321"),
            Utc::now(),
        );

        assert_eq!(codes.len(), 1, "Failed for sender: {}", sender);
        assert_eq!(
            codes[0].service,
            Some(expected_service.to_string()),
            "Failed for sender: {}",
            sender
        );
    }
}
