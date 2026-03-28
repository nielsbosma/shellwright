use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Main configuration for Shellwright daemon and sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Maximum number of concurrent sessions.
    pub max_sessions: usize,
    /// Idle timeout before a session is automatically cleaned up.
    pub idle_timeout: Duration,
    /// Default PTY size (rows).
    pub default_rows: u16,
    /// Default PTY size (columns).
    pub default_cols: u16,
    /// Output ring buffer capacity (number of lines).
    pub ring_buffer_capacity: usize,
    /// Maximum output size returned to agents (bytes).
    pub max_output_size: usize,
    /// Directory for session transcripts and runtime data.
    pub data_dir: PathBuf,
    /// Settle time for prompt detection (ms).
    pub settle_time_ms: u64,
    /// Double-settle confirmation time (ms).
    pub double_settle_time_ms: u64,
    /// Whether to enable dangerous command detection.
    pub enable_danger_detection: bool,
    /// Whether to enable secret redaction in output.
    pub enable_secret_redaction: bool,
    /// Daemon self-exit timeout: if no sessions exist for this duration, the daemon exits.
    /// Set to Duration::ZERO to disable.
    pub daemon_idle_exit: Duration,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .unwrap_or_else(|| {
                // Platform-appropriate fallback
                std::env::temp_dir()
            })
            .join("shellwright");

        Self {
            max_sessions: 64,
            idle_timeout: Duration::from_secs(30 * 60),
            default_rows: 24,
            default_cols: 80,
            ring_buffer_capacity: 10_000,
            max_output_size: 20_000,
            data_dir,
            settle_time_ms: 300,
            double_settle_time_ms: 300,
            enable_danger_detection: true,
            enable_secret_redaction: true,
            daemon_idle_exit: Duration::from_secs(10 * 60), // 10 minutes
        }
    }
}

impl Config {
    /// Returns the IPC socket/pipe path.
    pub fn ipc_path(&self) -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"\\.\pipe\shellwright")
        } else {
            self.data_dir.join("shellwright.sock")
        }
    }

    /// Returns the transcript directory for a session.
    pub fn transcript_path(&self, session_name: &str) -> PathBuf {
        self.data_dir
            .join("sessions")
            .join(session_name)
            .join("output.txt")
    }
}
