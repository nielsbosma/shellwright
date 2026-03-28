/// VT/ANSI terminal emulator wrapper using the `vt100` crate.
///
/// Processes raw PTY bytes through a full terminal emulator to produce
/// clean text output. Handles cursor movement, carriage returns,
/// progress bars, and all escape sequences correctly.
pub struct VtParser {
    parser: vt100::Parser,
    /// Track the last known content length for incremental reads.
    last_content_len: usize,
}

impl VtParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 1000),
            last_content_len: 0,
        }
    }

    /// Process raw bytes from the PTY.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// Get the full terminal screen contents as clean text.
    pub fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    /// Get only the new content since the last call to `new_content()`.
    pub fn new_content(&mut self) -> String {
        let full = self.screen_contents();
        let new = if full.len() > self.last_content_len {
            full[self.last_content_len..].to_string()
        } else if full.len() < self.last_content_len {
            // Screen was cleared or rewound — return full content
            full.clone()
        } else {
            String::new()
        };
        self.last_content_len = full.len();
        new
    }

    /// Get the contents of a specific row (0-indexed).
    pub fn row_contents(&self, row: u16) -> String {
        let screen = self.parser.screen();
        let mut s = String::new();
        for col in 0..screen.size().1 {
            let cell = screen.cell(row, col);
            if let Some(cell) = cell {
                s.push(cell.contents().chars().next().unwrap_or(' '));
            }
        }
        s.trim_end().to_string()
    }

    /// Get the last non-empty line on the screen (useful for prompt detection).
    pub fn last_non_empty_line(&self) -> Option<String> {
        let screen = self.parser.screen();
        let rows = screen.size().0;
        for row in (0..rows).rev() {
            let content = self.row_contents(row);
            if !content.is_empty() {
                return Some(content);
            }
        }
        None
    }

    /// Get the last N non-empty lines.
    pub fn last_non_empty_lines(&self, n: usize) -> Vec<String> {
        let screen = self.parser.screen();
        let rows = screen.size().0;
        let mut result = Vec::new();
        for row in (0..rows).rev() {
            let content = self.row_contents(row);
            if !content.is_empty() {
                result.push(content);
                if result.len() >= n {
                    break;
                }
            }
        }
        result.reverse();
        result
    }

    /// Get the cursor position (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        let screen = self.parser.screen();
        let pos = screen.cursor_position();
        (pos.0, pos.1)
    }

    /// Check if cursor is at the end of a line (heuristic for prompt state).
    pub fn cursor_at_line_end(&self) -> bool {
        let (row, col) = self.cursor_position();
        let content = self.row_contents(row);
        col as usize >= content.len().saturating_sub(1)
    }
}

impl std::fmt::Debug for VtParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VtParser")
            .field("last_content_len", &self.last_content_len)
            .finish()
    }
}
