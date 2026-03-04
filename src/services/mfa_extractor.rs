use regex::Regex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MfaCode {
    pub code: String,
    pub service: Option<String>,
    pub email_id: String,
    pub email_subject: Option<String>,
    pub email_sender: Option<String>,
    pub email_date: chrono::DateTime<chrono::Utc>,
    pub code_type: CodeType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeType {
    Numeric,
    Alphanumeric,
    Url,
}

pub struct MfaExtractor;

impl MfaExtractor {
    pub fn extract_codes(
        email_id: &str,
        subject: Option<&str>,
        sender: Option<&str>,
        body: Option<&str>,
        date: chrono::DateTime<chrono::Utc>,
    ) -> Vec<MfaCode> {
        let mut codes = Vec::new();

        if let Some(body_text) = body {
            // Check if this looks like a verification email (supports multiple languages)
            let verification_keywords = [
                // English
                "code", "verification", "verify", "OTP", "2FA", "authentication", "passcode", "PIN",
                // Portuguese
                "código", "codigo", "validação", "validacao", "autenticação", "autenticacao",
                "verificação", "verificacao", "procedimento", "segurança", "seguranca",
                // Spanish
                "código", "verificación", "autenticación",
                // Common patterns
                "token", "confirm", "validate"
            ];
            let has_verification_context = verification_keywords.iter()
                .any(|keyword| body_text.to_lowercase().contains(&keyword.to_lowercase()));

            // Also check if there's a pattern that looks like a code even without keywords
            let has_code_pattern = body_text.contains(": ") && (
                Regex::new(r"\b\d{4,8}\b").unwrap().is_match(body_text) ||
                Regex::new(r"\b[A-Z0-9]{5,8}\b").unwrap().is_match(body_text)
            );

            if !has_verification_context && !has_code_pattern {
                return codes;
            }

            let service = Self::detect_service(sender, subject);

            // Try to extract codes with context (like "código: 123456" or "code is 123456")
            let patterns = vec![
                r"(?:código|code|token|pin)(?:\s+de\s+validação)?(?:\s+is)?:\s*(\d{4,8})",
                r"(?:verification|validação|validation)\s+(?:code|código):\s*(\d{4,8})",
                r"(?:use|utilize|usar)\s+(?:o\s+)?(?:código|code)(?:\s+de\s+validação)?:\s*(\d{4,8})",
                r"\b([0-9]{6})\b", // Fallback to any 6-digit number
            ];

            for pattern in patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if let Some(captures) = re.captures(body_text) {
                        if let Some(code_match) = captures.get(1) {
                            codes.push(MfaCode {
                                code: code_match.as_str().to_string(),
                                service: service.clone(),
                                email_id: email_id.to_string(),
                                email_subject: subject.map(String::from),
                                email_sender: sender.map(String::from),
                                email_date: date,
                                code_type: CodeType::Numeric,
                            });
                            return codes;
                        }
                    }
                }
            }

            // Try to extract 4-digit codes
            if codes.is_empty() {
                if let Ok(re) = Regex::new(r"\b([0-9]{4})\b") {
                    if let Some(captures) = re.captures(body_text) {
                        if let Some(code_match) = captures.get(1) {
                            codes.push(MfaCode {
                                code: code_match.as_str().to_string(),
                                service: service.clone(),
                                email_id: email_id.to_string(),
                                email_subject: subject.map(String::from),
                                email_sender: sender.map(String::from),
                                email_date: date,
                                code_type: CodeType::Numeric,
                            });
                        }
                    }
                }
            }

            // Try to extract alphanumeric codes
            if codes.is_empty() {
                if let Ok(re) = Regex::new(r"\b([A-Z0-9]{5,8})\b") {
                    if let Some(captures) = re.captures(body_text) {
                        if let Some(code_match) = captures.get(1) {
                            codes.push(MfaCode {
                                code: code_match.as_str().to_string(),
                                service,
                                email_id: email_id.to_string(),
                                email_subject: subject.map(String::from),
                                email_sender: sender.map(String::from),
                                email_date: date,
                                code_type: CodeType::Alphanumeric,
                            });
                        }
                    }
                }
            }
        }

        codes
    }

    fn detect_service(sender: Option<&str>, subject: Option<&str>) -> Option<String> {
        if let Some(sender_email) = sender {
            let sender_lower = sender_email.to_lowercase();

            // Check for government services (Brazil)
            if sender_lower.contains(".gov.br") || sender_lower.contains("celepar") {
                return Some(String::from("Brazilian Gov"));
            }

            // Check subject for service hints
            if let Some(subj) = subject {
                let subj_lower = subj.to_lowercase();
                if subj_lower.contains("central de seguranca") || subj_lower.contains("segurança") {
                    return Some(String::from("Security Center"));
                }
            }

            // Match common service domains
            if sender_lower.contains("google") || sender_lower.contains("gmail") {
                return Some(String::from("Google"));
            }
            if sender_lower.contains("microsoft") || sender_lower.contains("outlook") || sender_lower.contains("hotmail") {
                return Some(String::from("Microsoft"));
            }
            if sender_lower.contains("facebook") || sender_lower.contains("meta") {
                return Some(String::from("Facebook"));
            }
            if sender_lower.contains("twitter") || sender_lower.contains("x.com") {
                return Some(String::from("Twitter"));
            }
            if sender_lower.contains("github") {
                return Some(String::from("GitHub"));
            }
            if sender_lower.contains("amazon") || sender_lower.contains("aws") {
                return Some(String::from("Amazon"));
            }
            if sender_lower.contains("apple") || sender_lower.contains("icloud") {
                return Some(String::from("Apple"));
            }
            if sender_lower.contains("linkedin") {
                return Some(String::from("LinkedIn"));
            }
            if sender_lower.contains("paypal") {
                return Some(String::from("PayPal"));
            }
            if sender_lower.contains("discord") {
                return Some(String::from("Discord"));
            }
            if sender_lower.contains("slack") {
                return Some(String::from("Slack"));
            }
            if sender_lower.contains("dropbox") {
                return Some(String::from("Dropbox"));
            }
            if sender_lower.contains("stripe") {
                return Some(String::from("Stripe"));
            }
            if sender_lower.contains("coinbase") {
                return Some(String::from("Coinbase"));
            }
            if sender_lower.contains("binance") {
                return Some(String::from("Binance"));
            }
            if sender_lower.contains("steam") {
                return Some(String::from("Steam"));
            }
            if sender_lower.contains("epic") {
                return Some(String::from("Epic"));
            }
            if sender_lower.contains("netflix") {
                return Some(String::from("Netflix"));
            }
            if sender_lower.contains("spotify") {
                return Some(String::from("Spotify"));
            }
            if sender_lower.contains("uber") {
                return Some(String::from("Uber"));
            }
            if sender_lower.contains("zoom") {
                return Some(String::from("Zoom"));
            }
        }

        None
    }
}