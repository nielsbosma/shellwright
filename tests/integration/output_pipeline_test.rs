use shellwright::output::ring_buffer::RingBuffer;
use shellwright::output::sanitizer::Sanitizer;
use shellwright::output::vt_parser::VtParser;
use shellwright::security::redactor::SecretRedactor;

#[test]
fn test_full_output_pipeline() {
    // Simulate: raw PTY bytes -> VT parse -> sanitize -> ring buffer

    // 1. Raw PTY output with ANSI codes
    let raw = b"\x1b[32m$ \x1b[0mecho hello\r\n\x1b[32mhello\x1b[0m\r\n\x1b[32m$ \x1b[0m";

    // 2. VT parse
    let mut parser = VtParser::new(24, 80);
    parser.process(raw);
    let content = parser.screen_contents();

    // 3. Sanitize
    let sanitizer = Sanitizer::new(20_000);
    let clean = sanitizer.sanitize(&content, Some("echo hello"));

    // 4. Ring buffer
    let mut buffer = RingBuffer::new(1000);
    buffer.append_lines(&clean);

    // Verify the pipeline produced clean output
    assert!(!buffer.contents().contains("\x1b")); // No ANSI
    let contents = buffer.contents();
    assert!(contents.contains("hello") || contents.contains("$")); // Content preserved
}

#[test]
fn test_pipeline_with_progress_bar() {
    let mut parser = VtParser::new(24, 80);

    // Simulate a progress bar: CR-based overwrites
    parser.process(b"Downloading: [====      ] 40%\r");
    parser.process(b"Downloading: [========  ] 80%\r");
    parser.process(b"Downloading: [==========] 100%\r\n");
    parser.process(b"Done!\r\n");

    let content = parser.screen_contents();
    // The final state should have the 100% version
    assert!(content.contains("100%"));
    assert!(content.contains("Done!"));
}

#[test]
fn test_pipeline_with_secret_redaction() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"Config loaded: token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\r\n");

    let content = parser.screen_contents();
    let sanitizer = Sanitizer::new(20_000);
    let clean = sanitizer.sanitize(&content, None);

    let redactor = SecretRedactor::new(true);
    let redacted = redactor.redact(&clean);

    assert!(redacted.contains("[REDACTED"));
    assert!(!redacted.contains("ghp_"));
}

#[test]
fn test_pipeline_multiline_output() {
    let mut parser = VtParser::new(24, 80);

    let output = b"Line 1: Starting build\r\n\
                   Line 2: Compiling module A\r\n\
                   Line 3: Compiling module B\r\n\
                   Line 4: Build complete\r\n";

    parser.process(output);
    let content = parser.screen_contents();

    let sanitizer = Sanitizer::new(20_000);
    let clean = sanitizer.sanitize(&content, None);

    let mut buffer = RingBuffer::new(1000);
    buffer.append_lines(&clean);

    assert!(buffer.len() >= 4);
    assert!(buffer.contents().contains("Build complete"));
}

#[test]
fn test_pipeline_truncation() {
    let mut parser = VtParser::new(24, 80);

    // Generate a large amount of output
    let mut big_output = Vec::new();
    for i in 0..500 {
        big_output.extend_from_slice(format!("Line {}: {}\r\n", i, "x".repeat(50)).as_bytes());
    }
    parser.process(&big_output);

    let content = parser.screen_contents();
    let sanitizer = Sanitizer::new(1000); // Small limit
    let clean = sanitizer.sanitize(&content, None);

    // Should be truncated
    assert!(clean.len() <= 1200); // truncated + notice overhead
    if clean.len() > 1000 {
        assert!(clean.contains("[output truncated"));
    }
}
