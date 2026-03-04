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

        // Log email details for debugging
        tracing::debug!(
            "Extracting MFA codes from email: id={}, subject={:?}, sender={:?}, body_length={}",
            email_id,
            subject,
            sender,
            body.map(|b| b.len()).unwrap_or(0)
        );

        if let Some(body_text) = body {
            // Check if this looks like a verification email (supports multiple languages)
            let verification_keywords = [
                // English
                "code",
                "verification",
                "verify",
                "OTP",
                "2FA",
                "MFA",
                "authentication",
                "passcode",
                "PIN",
                // Portuguese
                "código",
                "codigo",
                "validação",
                "validacao",
                "autenticação",
                "autenticacao",
                "verificação",
                "verificacao",
                "procedimento",
                "segurança",
                "seguranca",
                // Spanish
                "código",
                "verificación",
                "autenticación",
                // Common patterns
                "token",
                "confirm",
                "validate",
            ];
            let body_lower = body_text.to_lowercase();
            let has_verification_context = verification_keywords
                .iter()
                .any(|keyword| body_lower.contains(&keyword.to_lowercase()));

            // Also check subject for verification context
            let subject_has_context = if let Some(subj) = subject {
                let subj_lower = subj.to_lowercase();
                verification_keywords
                    .iter()
                    .any(|keyword| subj_lower.contains(&keyword.to_lowercase()))
            } else {
                false
            };

            // Check if this looks like an order/shipping/receipt email (should be excluded)
            let exclusion_keywords = ["order", "invoice", "receipt", "shipping", "package", "tracking"];
            let is_excluded = exclusion_keywords.iter()
                .any(|keyword| body_lower.contains(keyword) ||
                    subject.map(|s| s.to_lowercase().contains(keyword)).unwrap_or(false));

            // Also check if there's a pattern that looks like a code even without keywords
            // But not if it's an order number (preceded by #)
            let has_code_pattern = !body_text.contains("#") &&
                (body_text.contains(": ") || body_text.contains("is "))
                && (Regex::new(r"\b\d{4,8}\b").unwrap().is_match(body_text)
                    || Regex::new(r"\b[A-Z0-9]{5,8}\b")
                        .unwrap()
                        .is_match(body_text));

            if is_excluded || (!has_verification_context && !subject_has_context && !has_code_pattern) {
                tracing::debug!(
                    "Skipping email - no verification context found. Subject: {:?}, Body preview: {:?}",
                    subject,
                    &body_text.chars().take(100).collect::<String>()
                );
                return codes;
            }

            let service = Self::detect_service(sender, subject);

            // Try to extract codes with context (like "código: 123456" or "code is 123456")
            let patterns = vec![
                // Portuguese patterns with "é"
                r"(\d{4,8})\s+(?:é|e)\s+o\s+(?:código|codigo)",
                // Brazilian government pattern: "código de validação: 275992"
                r"(?:código|codigo)\s+de\s+(?:validação|validacao):\s*(\d{4,8})",
                // General Portuguese patterns
                r"(?:código|codigo)(?:\s+de\s+validação)?(?:\s+é)?:\s*(\d{4,8})",
                r"(?:utilize|usar|use)\s+o\s+(?:código|codigo)\s+de\s+(?:validação|validacao):\s*(\d{4,8})",
                // English patterns - MFA specific
                r"(?:your\s+)?(?:mfa|MFA)\s+code\s+is:?\s*(\d{4,8})",
                // English patterns - general
                r"(?:code|token|pin)(?:\s+is)?:\s*(\d{4,8})",
                r"(?:verification|validation)\s+code:\s*(\d{4,8})",
                r"(?:use|enter)\s+(?:code|this):\s*(\d{4,8})",
                r"your\s+(?:verification\s+)?code\s+is:?\s*(\d{4,8})",
                // Hyphenated codes (like "123-456")
                r"(?:code|código)\s+is:?\s*(\d{3})-(\d{3})",
                r"(?:verification|validation)\s+code\s+is:?\s*(\d{3})-(\d{3})",
                // Generic patterns
                r"(?:código|code|token|pin)(?:\s+de\s+validação)?(?:\s+is)?:\s*(\d{4,8})",
                r"(?:verification|validação|validation)\s+(?:code|código):\s*(\d{4,8})",
                r"(?:use|utilize|usar)\s+(?:o\s+)?(?:código|code)(?:\s+de\s+validação)?:\s*(\d{4,8})",
                // Standalone 6-digit number (fallback)
                r"\b([0-9]{6})\b",
            ];

            for pattern in patterns {
                if let Ok(re) = regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                {
                    if let Some(captures) = re.captures(body_text) {
                        // Handle hyphenated codes with two capture groups
                        let extracted_code = if captures.get(2).is_some() {
                            // Combine two parts for hyphenated codes
                            format!("{}{}",
                                captures.get(1).map(|m| m.as_str()).unwrap_or(""),
                                captures.get(2).map(|m| m.as_str()).unwrap_or("")
                            )
                        } else if let Some(code_match) = captures.get(1) {
                            code_match.as_str().to_string()
                        } else {
                            continue;
                        };

                        if !extracted_code.is_empty() {
                            tracing::info!(
                                "Found MFA code '{}' using pattern '{}' in email from {:?}",
                                extracted_code,
                                pattern,
                                sender
                            );
                            codes.push(MfaCode {
                                code: extracted_code,
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
                // Check subject for more specific service
                if let Some(subj) = subject {
                    let subj_lower = subj.to_lowercase();
                    if subj_lower.contains("central de seguranca")
                        || subj_lower.contains("segurança")
                    {
                        return Some(String::from("Central de Segurança"));
                    }
                }
                return Some(String::from("Brazilian Gov"));
            }

            // Check subject for service hints even without government domain
            if let Some(subj) = subject {
                let subj_lower = subj.to_lowercase();
                if subj_lower.contains("central de seguranca") || subj_lower.contains("segurança")
                {
                    return Some(String::from("Central de Segurança"));
                }
            }

            // Match common service domains
            if sender_lower.contains("google") || sender_lower.contains("gmail") {
                return Some(String::from("Google"));
            }
            if sender_lower.contains("microsoft")
                || sender_lower.contains("outlook")
                || sender_lower.contains("hotmail")
            {
                return Some(String::from("Microsoft"));
            }
            if sender_lower.contains("facebook") || sender_lower.contains("meta") {
                return Some(String::from("Facebook"));
            }
            if sender_lower.contains("twitter") || sender_lower.contains("@x.com") {
                return Some(String::from("Twitter"));
            }
            if sender_lower.contains("github") {
                return Some(String::from("GitHub"));
            }
            if sender_lower.contains("aws") {
                return Some(String::from("AWS"));
            }
            if sender_lower.contains("amazon") {
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
