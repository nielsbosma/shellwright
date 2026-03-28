use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::daemon::protocol::*;
use crate::security::danger::DangerDetector;
use crate::session::manager::SessionManager;

/// The Shellwright daemon server.
pub struct DaemonServer {
    manager: Arc<Mutex<SessionManager>>,
    danger: Arc<Mutex<DangerDetector>>,
    config: Config,
}

impl DaemonServer {
    pub fn new(config: Config) -> Self {
        let danger_enabled = config.enable_danger_detection;
        Self {
            manager: Arc::new(Mutex::new(SessionManager::new(config.clone()))),
            danger: Arc::new(Mutex::new(DangerDetector::new(danger_enabled))),
            config,
        }
    }

    /// Run the daemon, listening for IPC connections.
    pub async fn run(&self) -> Result<()> {
        // Ensure data directory exists
        std::fs::create_dir_all(&self.config.data_dir)?;

        let ipc_path = self.config.ipc_path();
        tracing::info!("Shellwright daemon starting on {:?}", ipc_path);

        // Start background tasks: session processing, idle cleanup, daemon self-exit
        let manager = Arc::clone(&self.manager);
        let daemon_idle_exit = self.config.daemon_idle_exit;
        tokio::spawn(async move {
            let mut empty_since: Option<tokio::time::Instant> = None;

            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let mut mgr = manager.lock().await;
                mgr.process_all().await;
                mgr.cleanup_idle();

                // Daemon self-exit: if no sessions for daemon_idle_exit duration, shut down
                if daemon_idle_exit > Duration::ZERO {
                    if mgr.session_count() == 0 {
                        let since = empty_since.get_or_insert_with(tokio::time::Instant::now);
                        if since.elapsed() >= daemon_idle_exit {
                            tracing::info!(
                                "No sessions for {:?} — daemon shutting down",
                                daemon_idle_exit
                            );
                            std::process::exit(0);
                        }
                    } else {
                        empty_since = None;
                    }
                }
            }
        });

        // Platform-specific listener
        self.listen(&ipc_path).await
    }

    #[cfg(unix)]
    async fn listen(&self, ipc_path: &std::path::Path) -> Result<()> {
        // Check if another daemon is already listening on this socket
        if ipc_path.exists() {
            match tokio::net::UnixStream::connect(ipc_path).await {
                Ok(_) => {
                    anyhow::bail!(
                        "Another shellwrightd instance is already running on {:?}",
                        ipc_path
                    );
                }
                Err(_) => {
                    // Stale socket — safe to remove
                    let _ = std::fs::remove_file(ipc_path);
                }
            }
        }

        let listener =
            tokio::net::UnixListener::bind(ipc_path).context("Failed to bind Unix socket")?;

        tracing::info!("Listening on Unix socket: {:?}", ipc_path);

        loop {
            let (stream, _) = listener.accept().await?;
            let manager = Arc::clone(&self.manager);
            let danger = Arc::clone(&self.danger);
            tokio::spawn(async move {
                let (reader, writer) = stream.into_split();
                if let Err(e) = Self::handle_connection(reader, writer, manager, danger).await {
                    tracing::error!("Connection error: {}", e);
                }
            });
        }
    }

    #[cfg(windows)]
    async fn listen(&self, ipc_path: &std::path::Path) -> Result<()> {
        use tokio::net::windows::named_pipe::ServerOptions;

        let pipe_name = ipc_path.to_string_lossy().to_string();
        tracing::info!("Listening on named pipe: {}", pipe_name);

        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&pipe_name)
                .context("Failed to create named pipe")?;

            server
                .connect()
                .await
                .context("Failed to accept pipe connection")?;

            let manager = Arc::clone(&self.manager);
            let danger = Arc::clone(&self.danger);

            tokio::spawn(async move {
                let (reader, writer) = tokio::io::split(server);
                if let Err(e) = Self::handle_connection(reader, writer, manager, danger).await {
                    tracing::error!("Connection error: {}", e);
                }
            });
        }
    }

    /// Handle a single IPC connection.
    async fn handle_connection<R, W>(
        reader: R,
        mut writer: W,
        manager: Arc<Mutex<SessionManager>>,
        danger: Arc<Mutex<DangerDetector>>,
    ) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // Connection closed
            }

            let request: Request = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let resp =
                        Response::error("unknown".to_string(), format!("Invalid request: {}", e));
                    let json = serde_json::to_string(&resp)? + "\n";
                    writer.write_all(json.as_bytes()).await?;
                    continue;
                }
            };

            let response =
                Self::handle_request(request, Arc::clone(&manager), Arc::clone(&danger)).await;

            let json = serde_json::to_string(&response)? + "\n";
            writer.write_all(json.as_bytes()).await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Handle a single request.
    async fn handle_request(
        request: Request,
        manager: Arc<Mutex<SessionManager>>,
        danger: Arc<Mutex<DangerDetector>>,
    ) -> Response {
        let id = request.id.clone();

        match request.kind {
            RequestKind::Start(params) => {
                // Check for dangerous commands
                let cmd_str = params.command.join(" ");
                {
                    let det = danger.lock().await;
                    if let Some(detection) = det.check(&cmd_str) {
                        if !det.is_confirmed(&cmd_str) {
                            return Response::error(
                                id,
                                format!(
                                    "Dangerous command detected ({}:{}): {}. Confirm with justification first.",
                                    detection.category, detection.pattern_name, cmd_str
                                ),
                            );
                        }
                    }
                }

                let mut mgr = manager.lock().await;
                match mgr.start(params.name, params.command, params.rows, params.cols) {
                    Ok(info) => Response::success(id, ResponseData::Session(info)),
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::Read(params) => {
                let mut mgr = manager.lock().await;
                // Process output before reading
                if let Ok(session) = mgr.get_mut(&params.session) {
                    session.process_output().await;
                }
                match mgr.read_output(&params.session, params.since, params.tail) {
                    Ok((text, cursor)) => {
                        let session = match mgr.get(&params.session) {
                            Ok(s) => s,
                            Err(e) => return Response::error(id, e.to_string()),
                        };
                        let output_tail = session.output_tail(5);
                        let lines = session.info().output_lines;
                        let output_file = session.output_file();
                        Response::success(
                            id,
                            ResponseData::Output(OutputData {
                                session: params.session,
                                text,
                                cursor,
                                lines,
                                output_file,
                                output_tail,
                            }),
                        )
                    }
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::Send(params) => {
                let mut mgr = manager.lock().await;
                if let Err(e) = mgr.send_input(&params.session, &params.input).await {
                    return Response::error(id, e.to_string());
                }
                drop(mgr);

                // If wait_for is specified, wait for the pattern
                if let Some(pattern) = params.wait_for {
                    let timeout = Duration::from_secs_f64(params.timeout.unwrap_or(30.0));
                    let mut mgr = manager.lock().await;
                    match mgr.wait_for(&params.session, &pattern, timeout).await {
                        Ok(Some(match_text)) => Response::success(
                            id,
                            ResponseData::WaitResult(WaitResult {
                                session: params.session,
                                matched: true,
                                pattern,
                                match_text: Some(match_text),
                                timed_out: false,
                            }),
                        ),
                        Ok(None) => Response::success(
                            id,
                            ResponseData::WaitResult(WaitResult {
                                session: params.session,
                                matched: false,
                                pattern,
                                match_text: None,
                                timed_out: true,
                            }),
                        ),
                        Err(e) => Response::error(id, e.to_string()),
                    }
                } else {
                    Response::success(
                        id,
                        ResponseData::Ok(OkData {
                            session: params.session,
                            message: "Input sent".to_string(),
                        }),
                    )
                }
            }

            RequestKind::Wait(params) => {
                let timeout = Duration::from_secs_f64(params.timeout);
                let mut mgr = manager.lock().await;
                match mgr
                    .wait_for(&params.session, &params.pattern, timeout)
                    .await
                {
                    Ok(Some(match_text)) => Response::success(
                        id,
                        ResponseData::WaitResult(WaitResult {
                            session: params.session,
                            matched: true,
                            pattern: params.pattern,
                            match_text: Some(match_text),
                            timed_out: false,
                        }),
                    ),
                    Ok(None) => Response::success(
                        id,
                        ResponseData::WaitResult(WaitResult {
                            session: params.session,
                            matched: false,
                            pattern: params.pattern,
                            match_text: None,
                            timed_out: true,
                        }),
                    ),
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::List => {
                let mgr = manager.lock().await;
                let sessions = mgr.list();
                Response::success(id, ResponseData::SessionList(sessions))
            }

            RequestKind::Status(params) => {
                let mgr = manager.lock().await;
                match mgr.status(&params.session) {
                    Ok(info) => Response::success(id, ResponseData::Session(info)),
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::Interrupt(params) => {
                let mgr = manager.lock().await;
                match mgr.interrupt(&params.session).await {
                    Ok(()) => Response::success(
                        id,
                        ResponseData::Ok(OkData {
                            session: params.session,
                            message: "Interrupt sent".to_string(),
                        }),
                    ),
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::Terminate(params) => {
                let mut mgr = manager.lock().await;
                match mgr.terminate(&params.session) {
                    Ok(()) => Response::success(
                        id,
                        ResponseData::Ok(OkData {
                            session: params.session,
                            message: "Session terminated".to_string(),
                        }),
                    ),
                    Err(e) => Response::error(id, e.to_string()),
                }
            }

            RequestKind::ConfirmDanger(params) => {
                let mut det = danger.lock().await;
                match det.confirm(&params.command, &params.justification) {
                    Ok(()) => {
                        tracing::warn!(
                            "Dangerous command confirmed: '{}' — justification: '{}'",
                            params.command,
                            params.justification
                        );
                        Response::success(
                            id,
                            ResponseData::Ok(OkData {
                                session: "system".to_string(),
                                message: format!(
                                    "Command confirmed. You may now start it: {}",
                                    params.command
                                ),
                            }),
                        )
                    }
                    Err(e) => Response::error(id, e),
                }
            }

            RequestKind::Shutdown => {
                tracing::info!("Shutdown requested");
                std::process::exit(0);
            }
        }
    }
}
