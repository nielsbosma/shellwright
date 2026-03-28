use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

use crate::daemon::protocol::*;

/// IPC client for communicating with the Shellwright daemon.
pub struct DaemonClient;

impl DaemonClient {
    /// Send a request to the daemon and get a response.
    pub async fn request(ipc_path: &Path, kind: RequestKind) -> Result<Response> {
        let request = Request {
            id: Uuid::new_v4().to_string(),
            kind,
        };

        let request_json = serde_json::to_string(&request)? + "\n";

        Self::send_raw(ipc_path, &request_json).await
    }

    #[cfg(unix)]
    async fn send_raw(ipc_path: &Path, request_json: &str) -> Result<Response> {
        let stream = tokio::net::UnixStream::connect(ipc_path)
            .await
            .context("Failed to connect to daemon. Is shellwrightd running?")?;

        let (reader, mut writer) = stream.into_split();
        writer.write_all(request_json.as_bytes()).await?;
        writer.flush().await?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;

        let response: Response =
            serde_json::from_str(line.trim()).context("Failed to parse daemon response")?;
        Ok(response)
    }

    #[cfg(windows)]
    async fn send_raw(ipc_path: &Path, request_json: &str) -> Result<Response> {
        use tokio::net::windows::named_pipe::ClientOptions;

        let pipe_name = ipc_path.to_string_lossy().to_string();

        let client = ClientOptions::new()
            .open(&pipe_name)
            .context("Failed to connect to daemon. Is shellwrightd running?")?;

        let (reader, mut writer) = tokio::io::split(client);
        writer.write_all(request_json.as_bytes()).await?;
        writer.flush().await?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;

        let response: Response =
            serde_json::from_str(line.trim()).context("Failed to parse daemon response")?;
        Ok(response)
    }

    /// Check if the daemon is running by trying to connect.
    pub async fn is_daemon_running(ipc_path: &Path) -> bool {
        Self::request(ipc_path, RequestKind::List).await.is_ok()
    }

    /// Start the daemon as a background process and wait for it to be ready.
    pub async fn ensure_daemon(ipc_path: &Path) -> Result<()> {
        if Self::is_daemon_running(ipc_path).await {
            return Ok(());
        }

        tracing::info!("Starting shellwrightd daemon...");

        // Start the daemon as a detached process
        let exe = std::env::current_exe().context("Failed to get current exe path")?;
        let daemon_exe = exe
            .parent()
            .context("Failed to determine exe parent directory")?
            .join(if cfg!(windows) {
                "shellwrightd.exe"
            } else {
                "shellwrightd"
            });

        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new(&daemon_exe)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("Failed to start shellwrightd")?;
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            Command::new(&daemon_exe)
                .creation_flags(0x00000008) // DETACHED_PROCESS
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("Failed to start shellwrightd")?;
        }

        // Wait for daemon to become ready (up to 5 seconds)
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if Self::is_daemon_running(ipc_path).await {
                tracing::info!("Daemon is ready");
                return Ok(());
            }
        }

        anyhow::bail!("Daemon failed to start within 5 seconds")
    }
}
