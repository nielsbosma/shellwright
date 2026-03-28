use shellwright::output::sanitizer::Sanitizer;

#[test]
fn test_strip_ansi_colors() {
    let input = "\x1b[31mError\x1b[0m: something failed";
    let result = Sanitizer::strip_ansi(input);
    assert_eq!(result, "Error: something failed");
}

#[test]
fn test_strip_ansi_cursor_movement() {
    let input = "\x1b[2J\x1b[HHello";
    let result = Sanitizer::strip_ansi(input);
    assert_eq!(result, "Hello");
}

#[test]
fn test_strip_ansi_bold_underline() {
    let input = "\x1b[1mBold\x1b[0m and \x1b[4munderlined\x1b[0m";
    let result = Sanitizer::strip_ansi(input);
    assert_eq!(result, "Bold and underlined");
}

#[test]
fn test_strip_echo_exact_match() {
    let output = "npm install\ninstalling packages...\ndone";
    let result = Sanitizer::strip_echo(output, "npm install");
    assert_eq!(result, "installing packages...\ndone");
}

#[test]
fn test_strip_echo_prompt_prefixed() {
    let output = ">>> print('hi')\nhi";
    let result = Sanitizer::strip_echo(output, "print('hi')");
    assert_eq!(result, "hi");
}

#[test]
fn test_strip_echo_no_match() {
    let output = "some output\nmore output";
    let result = Sanitizer::strip_echo(output, "different command");
    assert_eq!(result, "some output\nmore output");
}

#[test]
fn test_clean_whitespace_collapse_blanks() {
    let input = "line 1\n\n\n\n\nline 2";
    let result = Sanitizer::clean_whitespace(input);
    assert_eq!(result, "line 1\n\n\nline 2");
}

#[test]
fn test_clean_whitespace_trim_trailing() {
    let input = "line 1   \nline 2  ";
    let result = Sanitizer::clean_whitespace(input);
    assert_eq!(result, "line 1\nline 2");
}

#[test]
fn test_clean_whitespace_remove_trailing_blanks() {
    let input = "content\n\n\n";
    let result = Sanitizer::clean_whitespace(input);
    assert_eq!(result, "content");
}

#[test]
fn test_truncate_within_limit() {
    let sanitizer = Sanitizer::new(1000);
    let text = "short text";
    let result = sanitizer.truncate(text);
    assert_eq!(result, "short text");
}

#[test]
fn test_truncate_at_newline() {
    let sanitizer = Sanitizer::new(50);
    let text = "a".repeat(35) + "\n" + &"b".repeat(20) + "\n" + &"c".repeat(20);
    let result = sanitizer.truncate(&text);
    assert!(result.contains("[output truncated"));
    assert!(result.len() <= 100); // truncated + notice
}

#[test]
fn test_full_sanitize_pipeline() {
    let sanitizer = Sanitizer::new(20_000);
    let input = "\x1b[32mls\x1b[0m\nfile1.txt\nfile2.txt   \n\n\n\n\nend";
    let result = sanitizer.sanitize(input, Some("ls"));
    // Should strip ANSI, strip echo, clean whitespace
    assert!(!result.contains("\x1b"));
    assert!(result.contains("file1.txt"));
    assert!(result.contains("file2.txt"));
    // The echo "ls" should be stripped (first line after ANSI strip)
}

#[test]
fn test_sanitize_empty_input() {
    let sanitizer = Sanitizer::new(20_000);
    let result = sanitizer.sanitize("", None);
    assert_eq!(result, "");
}
