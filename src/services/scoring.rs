use std::collections::HashSet;

pub struct EmailScorer {
    important_domains: HashSet<String>,
    urgent_keywords: Vec<String>,
    spam_indicators: Vec<String>,
}

impl EmailScorer {
    pub fn new() -> Self {
        Self {
            important_domains: HashSet::new(),
            urgent_keywords: vec![
                "urgent".to_string(),
                "important".to_string(),
                "asap".to_string(),
                "action required".to_string(),
                "critical".to_string(),
            ],
            spam_indicators: vec![
                "noreply".to_string(),
                "newsletter".to_string(),
                "marketing".to_string(),
                "promo".to_string(),
                "unsubscribe".to_string(),
            ],
        }
    }

    pub fn add_important_domain(&mut self, domain: &str) {
        self.important_domains.insert(domain.to_string());
    }

    pub fn calculate_score(&self, sender_email: &str, subject: &str, labels: &[&str]) -> u8 {
        let sender_lower = sender_email.to_lowercase();
        let subject_lower = subject.to_lowercase();

        // Check for spam/promotional indicators first (Score 1)
        if labels.contains(&"SPAM") || labels.contains(&"PROMOTIONS") {
            return 1;
        }

        for indicator in &self.spam_indicators {
            if sender_lower.contains(indicator) {
                return 1;
            }
        }

        // Check for high priority indicators (Score 3)
        // Check important domains
        if let Some(domain) = sender_email.split('@').nth(1) {
            if self.important_domains.contains(domain) {
                return 3;
            }
        }

        // Check urgent keywords
        for keyword in &self.urgent_keywords {
            if subject_lower.contains(keyword) {
                return 3;
            }
        }

        // Check if marked important by Gmail
        if labels.contains(&"IMPORTANT") {
            return 3;
        }

        // Default to normal priority (Score 2)
        2
    }
}

impl Default for EmailScorer {
    fn default() -> Self {
        Self::new()
    }
}
