use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use regex::Regex;

use crate::config::Config;
use crate::output::ring_buffer::RingBuffer;
use crate::output::sanitizer::Sanitizer;
use crate::output::transcript::Transcript;
use crate::output::vt_parser::VtParser;
use crate::prompt::detector::{PromptDetection, PromptDetector};
use crate::pty::runner::PtyRunner;
use crate::security::redactor::SecretRedactor;
use crate::session::state::{ActivityTracker, SessionInfo, SessionState};

/// Maximum number of cached compiled regexes per session.
const REGEX_CACHE_CAPACITY: usize = 32;

/// An individual session managing a PTY process and its output pipeline.
pub struct Session {
    pub name: String,
    pub command: Vec<String>,
    pub state: SessionState,
    pub exit_code: Option<i32>,
    pub created_at: chrono::DateTime<Utc>,
    pub last_activity: chrono::DateTime<Utc>,

    pty: PtyRunner,
    vt_parser: VtParser,
    ring_buffer: RingBuffer,
    sanitizer: Sanitizer,
    transcript: Transcript,
    prompt_detector: PromptDetector,
    redactor: SecretRedactor,
    activity: ActivityTracker,
    last_sent_command: Option<String>,
    /// LRU cache of compiled regex patterns for wait_for_pattern.
    regex_cache: HashMap<String, Regex>,
}

impl Session {
    /// Create and start a new session.
    pub fn new(name: String, command: Vec<String>, config: &Config) -> Result<Self> {
        let pty = PtyRunner::spawn(&command, config.default_rows, config.default_cols)
            .context("Failed to spawn PTY session")?;

        let transcript_path = config.transcript_path(&name);
        let transcript = Transcript::new(transcript_path)?;

        let now = Utc::now();

        Ok(Self {
            name: name.clone(),
            command,
            state: SessionState::Spawning,
            exit_code: None,
            created_at: now,
            last_activity: now,
            pty,
            vt_parser: VtParser::new(config.default_rows, config.default_cols),
            ring_buffer: RingBuffer::new(config.ring_buffer_capacity),
            sanitizer: Sanitizer::new(config.max_output_size),
            transcript,
            prompt_detector: PromptDetector::new(
                config.settle_time_ms,
                config.double_settle_time_ms,
            ),
            redactor: SecretRedactor::new(config.enable_secret_redaction),
            activity: ActivityTracker::default(),
            last_sent_command: None,
            regex_cache: HashMap::new(),
        })
    }

    /// Process any available PTY output through the pipeline.
    /// Returns the number of new bytes processed.
    pub async fn process_output(&mut self) -> usize {
        let mut total = 0;

        while let Some(data) = self.pty.try_recv() {
            total += data.len();

            // 1. Feed VT parser for screen state (prompt detection, cursor tracking)
            self.vt_parser.process(&data);

            // 2. Convert raw bytes to text, strip ANSI directly from raw output
            //    This is more reliable than tracking VT parser screen deltas,
            //    especially on Windows ConPTY where screen rewrites are common.
            let raw_text = String::from_utf8_lossy(&data);
            let stripped = Sanitizer::strip_ansi(&raw_text);
            if stripped.trim().is_empty() {
                continue;
            }

            // 3. Echo-strip if we just sent a command
            let clean = if let Some(ref cmd) = self.last_sent_command {
                let result = Sanitizer::strip_echo(&stripped, cmd);
                self.last_sent_command = None;
                result
            } else {
                stripped
            };

            // 4. Truncate if necessary
            let truncated = self.sanitizer.truncate(&clean);

            // 5. Redact secrets
            let redacted = self.redactor.redact(&truncated);

            // 5. Append to ring buffer
            self.ring_buffer.append_lines(&redacted);

            // 6. Write to transcript (log but don't fail on write errors)
            if let Err(e) = self.transcript.append_raw(&redacted) {
                tracing::warn!("Failed to write to transcript: {}", e);
            }

            // 7. Signal prompt detector
            self.prompt_detector.on_output();

            // 8. Update activity tracking
            self.activity.last_output = std::time::Instant::now();
            self.last_activity = Utc::now();
        }

        // Transition from Spawning to Running on first output
        if self.state == SessionState::Spawning && total > 0 {
            self.state = SessionState::Running;
        }

        // Check for process exit
        if let Some(status) = self.pty.try_wait() {
            self.state = SessionState::Exited;
            self.exit_code = status.success().then_some(0).or(Some(1));
        }

        total
    }

    /// Detect if the process is waiting for input.
    pub fn detect_prompt(&self) -> Option<PromptDetection> {
        let last_lines = self.vt_parser.last_non_empty_lines(3);
        let cursor_at_end = self.vt_parser.cursor_at_line_end();
        self.prompt_detector.detect(&last_lines, cursor_at_end)
    }

    /// Update session state based on prompt detection.
    pub fn update_prompt_state(&mut self) {
        if self.state != SessionState::Running && self.state != SessionState::AwaitingInput {
            return;
        }

        if let Some(detection) = self.detect_prompt() {
            if detection.confidence >= 0.5 && self.state == SessionState::Running {
                self.state = SessionState::AwaitingInput;
            }
        } else if self.state == SessionState::AwaitingInput {
            self.state = SessionState::Running;
        }
    }

    /// Send input to the session.
    pub async fn send_input(&mut self, input: &str) -> Result<()> {
        if self.state == SessionState::Exited {
            bail!("Session '{}' has already exited", self.name);
        }

        self.last_sent_command = Some(input.to_string());
        self.pty.send_line(input).await?;
        self.prompt_detector.reset();

        if self.state == SessionState::AwaitingInput
            || self.state == SessionState::AwaitingConfirmation
        {
            self.state = SessionState::Running;
        }

        self.activity.last_input = std::time::Instant::now();
        self.last_activity = Utc::now();
        Ok(())
    }

    /// Read output from the ring buffer.
    pub fn read_output(&self, since: Option<u64>, tail: Option<usize>) -> (String, u64) {
        if let Some(n) = tail {
            let text = self.ring_buffer.tail_text(n);
            let cursor = self.ring_buffer.cursor();
            (text, cursor)
        } else {
            let cursor = since.unwrap_or(0);
            let (lines, new_cursor) = self.ring_buffer.read_since(cursor);
            let text = lines
                .iter()
                .map(|l| l.line.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (text, new_cursor)
        }
    }

    /// Wait for a regex pattern to appear in output.
    /// Uses an LRU cache for compiled regexes to avoid recompilation.
    pub async fn wait_for_pattern(
        &mut self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<Option<String>> {
        let re = self.get_or_compile_regex(pattern)?;
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            // Process any pending output
            self.process_output().await;

            // Check existing content
            let content = self.ring_buffer.contents();
            if let Some(m) = re.find(&content) {
                return Ok(Some(m.as_str().to_string()));
            }

            // Check timeout
            if tokio::time::Instant::now() >= deadline {
                return Ok(None);
            }

            // Check if process has exited
            if self.state == SessionState::Exited {
                // Final check
                let content = self.ring_buffer.contents();
                return Ok(re.find(&content).map(|m| m.as_str().to_string()));
            }

            // Wait briefly before checking again
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Send Ctrl+C to the process.
    pub async fn interrupt(&self) -> Result<()> {
        self.pty.interrupt().await
    }

    /// Terminate the session.
    pub fn terminate(&mut self) -> Result<()> {
        self.pty.kill()?;
        self.state = SessionState::Exited;
        Ok(())
    }

    /// Get session info for status/list responses.
    pub fn info(&self) -> SessionInfo {
        let detection = self.detect_prompt();

        SessionInfo {
            name: self.name.clone(),
            state: self.state,
            command: self.command.clone(),
            exit_code: self.exit_code,
            output_lines: self.ring_buffer.len(),
            transcript_path: self.transcript.path().to_string_lossy().to_string(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            prompt_confidence: detection.as_ref().map(|d| d.confidence),
            prompt_text: detection.and_then(|d| d.prompt_text),
            pid: self.pty.pid(),
        }
    }

    /// Calibrate prompt detection from current screen state.
    pub fn calibrate_prompt(&mut self) {
        let lines = self.vt_parser.last_non_empty_lines(3);
        self.prompt_detector.calibrator_mut().calibrate(&lines);
    }

    /// Resize the PTY window.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.pty.resize(rows, cols)
    }

    /// Get a compiled regex from cache, or compile and cache it.
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<Regex> {
        if let Some(re) = self.regex_cache.get(pattern) {
            return Ok(re.clone());
        }

        let re = Regex::new(pattern).context("Invalid regex pattern")?;

        // Evict oldest if cache is full (simple eviction — clear all)
        if self.regex_cache.len() >= REGEX_CACHE_CAPACITY {
            self.regex_cache.clear();
        }
        self.regex_cache.insert(pattern.to_string(), re.clone());
        Ok(re)
    }

    /// Check if the session has been idle longer than the given duration.
    pub fn is_idle(&self, timeout: Duration) -> bool {
        self.activity.last_output.elapsed() > timeout
            && self.activity.last_input.elapsed() > timeout
    }

    /// Get the output file path.
    pub fn output_file(&self) -> String {
        self.transcript.path().to_string_lossy().to_string()
    }

    /// Get the tail of the output.
    pub fn output_tail(&self, n: usize) -> String {
        self.ring_buffer.tail_text(n)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("name", &self.name)
            .field("state", &self.state)
            .field("command", &self.command)
            .finish()
    }
}
