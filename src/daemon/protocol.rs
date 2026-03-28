use serde::{Deserialize, Serialize};

use crate::session::state::SessionInfo;

/// A request from the CLI to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    #[serde(flatten)]
    pub kind: RequestKind,
}

/// The specific command being requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "params", rename_all = "snake_case")]
pub enum RequestKind {
    Start(StartParams),
    Read(ReadParams),
    Send(SendParams),
    Wait(WaitParams),
    List,
    Status(StatusParams),
    Interrupt(InterruptParams),
    Terminate(TerminateParams),
    ConfirmDanger(ConfirmDangerParams),
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartParams {
    pub name: Option<String>,
    pub command: Vec<String>,
    pub rows: Option<u16>,
    pub cols: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadParams {
    pub session: String,
    /// Output format: "clean" (default) or "raw".
    pub format: Option<String>,
    /// Only return output since this cursor position.
    pub since: Option<u64>,
    /// Return only the last N lines.
    pub tail: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendParams {
    pub session: String,
    pub input: String,
    /// If set, wait for this pattern after sending input.
    pub wait_for: Option<String>,
    /// Timeout for wait_for, in seconds.
    pub timeout: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitParams {
    pub session: String,
    /// Regex pattern to wait for in output.
    pub pattern: String,
    /// Timeout in seconds.
    pub timeout: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusParams {
    pub session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptParams {
    pub session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateParams {
    pub session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDangerParams {
    /// The exact command string to confirm.
    pub command: String,
    /// Justification for why the dangerous command should be allowed (min 10 chars).
    pub justification: String,
}

/// A response from the daemon to the CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
}

/// Typed response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
    Session(SessionInfo),
    Output(OutputData),
    SessionList(Vec<SessionInfo>),
    WaitResult(WaitResult),
    Ok(OkData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {
    pub session: String,
    pub text: String,
    pub cursor: u64,
    pub lines: usize,
    pub output_file: String,
    pub output_tail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResult {
    pub session: String,
    pub matched: bool,
    pub pattern: String,
    pub match_text: Option<String>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkData {
    pub session: String,
    pub message: String,
}

impl Response {
    pub fn success(id: String, data: ResponseData) -> Self {
        Self {
            id,
            success: true,
            error: None,
            data: Some(data),
        }
    }

    pub fn error(id: String, msg: impl Into<String>) -> Self {
        Self {
            id,
            success: false,
            error: Some(msg.into()),
            data: None,
        }
    }
}
