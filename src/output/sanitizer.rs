use regex::Regex;

use once_cell::sync::Lazy;

static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| {
    // Comprehensive ANSI/VT escape sequence regex covering:
    // - CSI sequences with DEC private mode markers: \x1b[?25h, \x1b[2J, etc.
    // - CSI with intermediate bytes: \x1b[2 q (cursor style)
    // - OSC sequences: \x1b]...\x07
    // - Character set designation: \x1b(A, \x1b)0, etc.
    // - Mode set/reset: \x1b>, \x1b=, \x1b<
    // - Save/restore cursor: \x1b7, \x1b8
    // - DCS strings: \x1bP...\x1b\\
    // - Bell character: \x07
    // - Carriage return (bare \r without \n for progress bar overwrites)
    Regex::new(
        r"(?x)
        \x1b\[ [?>=!]? [0-9;]* [ -/]* [A-Za-z@`] |  # CSI sequences
        \x1b\] [^\x07]* \x07 |                        # OSC sequences
        \x1b[()][AB012] |                              # Character sets
        \x1b[>=<78DEHM] |                              # Simple escape sequences
        \x1bP [^\x1b]* \x1b\\ |                       # DCS strings
        \x07                                           # Bell
        ",
    )
    .unwrap()
});

/// Output sanitizer: strips ANSI codes, removes echoed commands,
/// cleans whitespace, and truncates with newline-aware breaking.
#[derive(Debug)]
pub struct Sanitizer {
    max_output_size: usize,
}

impl Sanitizer {
    pub fn new(max_output_size: usize) -> Self {
        Self { max_output_size }
    }

    /// Strip ANSI escape sequences from text.
    pub fn strip_ansi(text: &str) -> String {
        ANSI_REGEX.replace_all(text, "").to_string()
    }

    /// Remove the echoed command from output.
    ///
    /// Terminals echo input back. If the first line of output matches
    /// the sent command (with or without a prompt prefix), strip it.
    pub fn strip_echo(output: &str, sent_command: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.is_empty() {
            return output.to_string();
        }

        let first = lines[0].trim();
        let cmd = sent_command.trim();

        // Exact match or prompt-prefixed match (e.g., ">>> print('hi')")
        if first == cmd || first.ends_with(cmd) {
            lines[1..].join("\n")
        } else {
            output.to_string()
        }
    }

    /// Clean up whitespace: collapse multiple blank lines, trim trailing whitespace.
    pub fn clean_whitespace(text: &str) -> String {
        let mut result = Vec::new();
        let mut blank_count = 0;

        for line in text.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                blank_count += 1;
                if blank_count <= 2 {
                    result.push("");
                }
            } else {
                blank_count = 0;
                result.push(trimmed);
            }
        }

        // Remove trailing blank lines
        while result.last() == Some(&"") {
            result.pop();
        }

        result.join("\n")
    }

    /// Truncate output with newline-aware breaking.
    ///
    /// If the output exceeds `max_chars`, break at a newline boundary
    /// past 80% of the limit, and append a truncation notice.
    pub fn truncate(&self, text: &str) -> String {
        if text.len() <= self.max_output_size {
            return text.to_string();
        }

        let break_point = (self.max_output_size * 80) / 100;

        // Find the next newline at or after the break point
        let cut = text[break_point..]
            .find('\n')
            .map(|i| break_point + i)
            .unwrap_or(self.max_output_size);

        let cut = cut.min(self.max_output_size);
        let truncated = &text[..cut];
        format!(
            "{}\n[output truncated at {} chars, total {} chars]",
            truncated,
            cut,
            text.len()
        )
    }

    /// Full sanitization pipeline: strip ANSI, clean whitespace, truncate.
    pub fn sanitize(&self, text: &str, sent_command: Option<&str>) -> String {
        let mut result = Self::strip_ansi(text);
        if let Some(cmd) = sent_command {
            result = Self::strip_echo(&result, cmd);
        }
        result = Self::clean_whitespace(&result);
        self.truncate(&result)
    }
}
