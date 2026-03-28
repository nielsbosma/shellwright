use shellwright::output::vt_parser::VtParser;

#[test]
fn test_plain_text() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"Hello, world!\r\n");
    let content = parser.screen_contents();
    assert!(content.contains("Hello, world!"));
}

#[test]
fn test_ansi_color_codes() {
    let mut parser = VtParser::new(24, 80);
    // Red text: ESC[31m ... ESC[0m
    parser.process(b"\x1b[31mError\x1b[0m: something failed\r\n");
    let content = parser.screen_contents();
    assert!(content.contains("Error: something failed"));
}

#[test]
fn test_cursor_movement() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"Line 1\r\nLine 2\r\nLine 3\r\n");
    let content = parser.screen_contents();
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Line 3"));
}

#[test]
fn test_carriage_return_overwrite() {
    let mut parser = VtParser::new(24, 80);
    // Progress bar pattern: write, CR, overwrite
    parser.process(b"Progress: 50%\rProgress: 100%\r\n");
    let content = parser.screen_contents();
    assert!(content.contains("Progress: 100%"));
    // The 50% should be overwritten
    assert!(!content.contains("50%"));
}

#[test]
fn test_last_non_empty_line() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"line 1\r\nline 2\r\n$ \r\n");
    let last = parser.last_non_empty_line();
    assert!(last.is_some());
}

#[test]
fn test_last_non_empty_lines() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"line 1\r\nline 2\r\nline 3\r\n");
    let lines = parser.last_non_empty_lines(2);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_new_content_incremental() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"first\r\n");
    let c1 = parser.new_content();
    assert!(c1.contains("first"));

    parser.process(b"second\r\n");
    let c2 = parser.new_content();
    assert!(c2.contains("second"));
    // Should not re-include first
    assert!(!c2.contains("first"));
}

#[test]
fn test_cursor_position() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"prompt> ");
    let (row, col) = parser.cursor_position();
    assert_eq!(row, 0);
    assert!(col > 0);
}

#[test]
fn test_row_contents() {
    let mut parser = VtParser::new(24, 80);
    parser.process(b"row zero content\r\nsecond row\r\n");
    let row0 = parser.row_contents(0);
    assert_eq!(row0, "row zero content");
    let row1 = parser.row_contents(1);
    assert_eq!(row1, "second row");
}
