use once_cell::sync::Lazy;
use regex::Regex;

/// A secret pattern with its label.
struct SecretPattern {
    label: &'static str,
    regex: Regex,
}

static SECRET_PATTERNS: Lazy<Vec<SecretPattern>> = Lazy::new(|| {
    vec![
        // AWS Access Key IDs
        SecretPattern {
            label: "AWS_KEY",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        // AWS Secret Keys (base64-like, 40 chars)
        SecretPattern {
            label: "AWS_SECRET",
            regex: Regex::new(r"(?i)(aws_secret_access_key|secret_key)\s*[=:]\s*\S{20,}").unwrap(),
        },
        // GitHub tokens
        SecretPattern {
            label: "GITHUB_TOKEN",
            regex: Regex::new(r"gh[ps]_[A-Za-z0-9_]{36,}|gho_[A-Za-z0-9_]{36,}").unwrap(),
        },
        // Generic API keys (sk- prefix, common for OpenAI, Stripe, etc.)
        SecretPattern {
            label: "API_KEY",
            regex: Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap(),
        },
        // PEM private key headers
        SecretPattern {
            label: "PRIVATE_KEY",
            regex: Regex::new(r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----").unwrap(),
        },
        // Connection strings with passwords
        SecretPattern {
            label: "CONNECTION_STRING",
            regex: Regex::new(r"://[^:]+:[^@]+@[^/\s]+").unwrap(),
        },
        // Generic token= patterns
        SecretPattern {
            label: "TOKEN",
            regex: Regex::new(r"(?i)(token|api_key|apikey|secret|password)\s*[=:]\s*\S{8,}")
                .unwrap(),
        },
        // Bearer tokens
        SecretPattern {
            label: "BEARER_TOKEN",
            regex: Regex::new(r"(?i)bearer\s+[A-Za-z0-9\-._~+/]+=*").unwrap(),
        },
        // Azure/GCP keys
        SecretPattern {
            label: "AZURE_KEY",
            regex: Regex::new(r"(?i)(azure|subscription)[_-]?(key|secret|password)\s*[=:]\s*\S+")
                .unwrap(),
        },
    ]
});

/// Secret redactor that replaces sensitive patterns in output.
#[derive(Debug)]
pub struct SecretRedactor {
    enabled: bool,
}

impl SecretRedactor {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Redact secrets from the given text.
    /// Returns the text with secrets replaced by `[REDACTED:LABEL]`.
    pub fn redact(&self, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }

        let mut result = text.to_string();
        for pattern in SECRET_PATTERNS.iter() {
            result = pattern
                .regex
                .replace_all(&result, format!("[REDACTED:{}]", pattern.label))
                .to_string();
        }
        result
    }

    /// Check if text contains any secrets (without modifying it).
    pub fn contains_secrets(&self, text: &str) -> bool {
        if !self.enabled {
            return false;
        }
        SECRET_PATTERNS.iter().any(|p| p.regex.is_match(text))
    }
}
