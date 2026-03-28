use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Session lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is being initialized and PTY is spawning.
    Spawning,
    /// Process is running and producing output.
    Running,
    /// Process appears to be waiting for user input (prompt detected).
    AwaitingInput,
    /// A dangerous command was detected; waiting for confirmation.
    AwaitingConfirmation,
    /// Process has exited.
    Exited,
}

impl SessionState {
    /// Returns whether this state transition is valid.
    pub fn can_transition_to(&self, next: SessionState) -> bool {
        use SessionState::*;
        matches!(
            (self, next),
            (Spawning, Running)
                | (Running, AwaitingInput)
                | (Running, AwaitingConfirmation)
                | (Running, Exited)
                | (AwaitingInput, Running)
                | (AwaitingInput, Exited)
                | (AwaitingConfirmation, Running)
                | (AwaitingConfirmation, Exited)
        )
    }
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawning => write!(f, "spawning"),
            Self::Running => write!(f, "running"),
            Self::AwaitingInput => write!(f, "awaiting_input"),
            Self::AwaitingConfirmation => write!(f, "awaiting_confirmation"),
            Self::Exited => write!(f, "exited"),
        }
    }
}

/// Information about a session, returned in status/list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name (user-provided or auto-generated).
    pub name: String,
    /// Current state.
    pub state: SessionState,
    /// The command that was started.
    pub command: Vec<String>,
    /// Process exit code (only set when state is Exited).
    pub exit_code: Option<i32>,
    /// Number of output lines captured.
    pub output_lines: usize,
    /// Path to the transcript file on disk.
    pub transcript_path: String,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session last had activity.
    pub last_activity: DateTime<Utc>,
    /// Prompt detection confidence (0.0 - 1.0), if in awaiting_input state.
    pub prompt_confidence: Option<f64>,
    /// The detected prompt text, if any.
    pub prompt_text: Option<String>,
    /// Process ID of the spawned command.
    pub pid: Option<u32>,
}

/// Tracks timing for idle detection (not serialized — runtime only).
#[derive(Debug, Clone)]
pub struct ActivityTracker {
    pub last_output: Instant,
    pub last_input: Instant,
    pub created: Instant,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_output: now,
            last_input: now,
            created: now,
        }
    }
}
