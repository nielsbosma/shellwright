use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A timestamped line of output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLine {
    pub line: String,
    pub timestamp: DateTime<Utc>,
    pub index: u64,
}

/// Bounded ring buffer for session output with cursor-based reads.
///
/// Lines are appended with monotonically increasing indices. When capacity
/// is exceeded, oldest lines are evicted. Readers use cursors (the index
/// of the last line they read) to get only new output.
#[derive(Debug)]
pub struct RingBuffer {
    lines: VecDeque<OutputLine>,
    capacity: usize,
    next_index: u64,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            next_index: 0,
        }
    }

    /// Append a line to the buffer.
    pub fn append(&mut self, line: String) {
        let entry = OutputLine {
            line,
            timestamp: Utc::now(),
            index: self.next_index,
        };
        self.next_index += 1;

        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(entry);
    }

    /// Append multiple lines (e.g. from splitting a chunk on newlines).
    pub fn append_lines(&mut self, text: &str) {
        for line in text.lines() {
            self.append(line.to_string());
        }
    }

    /// Read all lines since the given cursor (exclusive).
    /// Returns the lines and the new cursor position.
    pub fn read_since(&self, cursor: u64) -> (Vec<&OutputLine>, u64) {
        let lines: Vec<_> = self.lines.iter().filter(|l| l.index >= cursor).collect();
        let new_cursor = self.next_index;
        (lines, new_cursor)
    }

    /// Read the last N lines.
    pub fn tail(&self, n: usize) -> Vec<&OutputLine> {
        let skip = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(skip).collect()
    }

    /// Total number of lines that have ever been appended (including evicted).
    pub fn total_lines(&self) -> u64 {
        self.next_index
    }

    /// Current number of lines in the buffer.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Current cursor position (next index to be written).
    pub fn cursor(&self) -> u64 {
        self.next_index
    }

    /// Get the text of the last N lines joined with newlines.
    pub fn tail_text(&self, n: usize) -> String {
        let lines = self.tail(n);
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&line.line);
        }
        result
    }

    /// Get all current content as a single string.
    pub fn contents(&self) -> String {
        let mut result = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&line.line);
        }
        result
    }
}
