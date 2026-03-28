use shellwright::security::redactor::SecretRedactor;

#[test]
fn test_redact_aws_key() {
    let redactor = SecretRedactor::new(true);
    let input = "Found key: AKIAIOSFODNN7EXAMPLE in config";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:AWS_KEY]"));
    assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn test_redact_github_token() {
    let redactor = SecretRedactor::new(true);
    let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let result = redactor.redact(input);
    // The TOKEN generic pattern matches "token: ghp_..." before the GITHUB_TOKEN pattern
    assert!(result.contains("[REDACTED"));
    assert!(!result.contains("ghp_ABCDEF"));
}

#[test]
fn test_redact_github_oauth_token() {
    let redactor = SecretRedactor::new(true);
    let input = "gho_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:GITHUB_TOKEN]"));
}

#[test]
fn test_redact_api_key_sk_prefix() {
    let redactor = SecretRedactor::new(true);
    let input = "OPENAI_API_KEY=sk-proj-abc123def456ghi789jkl012mno";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED"));
    assert!(!result.contains("sk-proj-abc123"));
}

#[test]
fn test_redact_pem_header() {
    let redactor = SecretRedactor::new(true);
    let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIB...";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:PRIVATE_KEY]"));
}

#[test]
fn test_redact_connection_string() {
    let redactor = SecretRedactor::new(true);
    let input = "postgres://admin:s3cr3tP@ss@db.example.com:5432/mydb";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:CONNECTION_STRING]"));
    assert!(!result.contains("s3cr3tP@ss"));
}

#[test]
fn test_redact_bearer_token() {
    let redactor = SecretRedactor::new(true);
    let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.abc";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:BEARER_TOKEN]"));
}

#[test]
fn test_no_redaction_safe_text() {
    let redactor = SecretRedactor::new(true);
    let input = "Hello world, this is normal output with no secrets.";
    let result = redactor.redact(input);
    assert_eq!(result, input);
}

#[test]
fn test_disabled_redactor() {
    let redactor = SecretRedactor::new(false);
    let input = "AKIAIOSFODNN7EXAMPLE";
    let result = redactor.redact(input);
    assert_eq!(result, input); // No redaction when disabled
}

#[test]
fn test_contains_secrets() {
    let redactor = SecretRedactor::new(true);
    assert!(redactor.contains_secrets("token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij"));
    assert!(!redactor.contains_secrets("Hello, world!"));
}

#[test]
fn test_multiple_secrets_in_one_line() {
    let redactor = SecretRedactor::new(true);
    let input = "AWS: AKIAIOSFODNN7EXAMPLE, GH: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let result = redactor.redact(input);
    assert!(result.contains("[REDACTED:AWS_KEY]"));
    assert!(result.contains("[REDACTED:GITHUB_TOKEN]"));
}
