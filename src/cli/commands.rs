use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::cli::output;
use crate::config::Config;
use crate::daemon::client::DaemonClient;
use crate::daemon::protocol::*;

#[derive(Parser)]
#[command(
    name = "shellwright",
    about = "Universal CLI Session Broker for AI Agents — Playwright for CLIs",
    version
)]
pub struct Cli {
    /// Output as plain text instead of JSON.
    #[arg(long, global = true)]
    pub no_json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a new interactive session.
    Start {
        /// Session name (auto-generated if not provided).
        #[arg(short, long)]
        name: Option<String>,

        /// PTY rows.
        #[arg(long)]
        rows: Option<u16>,

        /// PTY columns.
        #[arg(long)]
        cols: Option<u16>,

        /// The command to run (after --).
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Read output from a session.
    Read {
        /// Session name.
        session: String,

        /// Output format: "clean" or "raw".
        #[arg(long, default_value = "clean")]
        format: String,

        /// Read only output since this cursor position.
        #[arg(long)]
        since: Option<u64>,

        /// Read only the last N lines.
        #[arg(long)]
        tail: Option<usize>,
    },

    /// Send input to a session.
    Send {
        /// Session name.
        session: String,

        /// Input text to send.
        input: String,

        /// Wait for this pattern after sending.
        #[arg(long)]
        wait_for: Option<String>,

        /// Timeout for wait-for in seconds.
        #[arg(long, default_value = "30")]
        timeout: f64,
    },

    /// Wait for a pattern in session output.
    Wait {
        /// Session name.
        session: String,

        /// Regex pattern to wait for.
        #[arg(long = "for")]
        pattern: String,

        /// Timeout in seconds.
        #[arg(long, default_value = "30")]
        timeout: f64,
    },

    /// List all active sessions.
    List,

    /// Get status of a session.
    Status {
        /// Session name.
        session: String,
    },

    /// Send Ctrl+C to a session.
    Interrupt {
        /// Session name.
        session: String,
    },

    /// Terminate a session.
    Terminate {
        /// Session name.
        session: String,
    },

    /// Confirm a dangerous command for execution.
    ConfirmDanger {
        /// The exact command string to confirm.
        command: String,

        /// Justification for why the dangerous command should be allowed.
        justification: String,
    },
}

/// Execute the CLI command.
pub async fn execute(cli: Cli) -> Result<()> {
    let config = Config::default();
    let ipc_path = config.ipc_path();

    // Ensure daemon is running
    DaemonClient::ensure_daemon(&ipc_path).await?;

    let kind = match cli.command {
        Commands::Start {
            name,
            rows,
            cols,
            command,
        } => RequestKind::Start(StartParams {
            name,
            command,
            rows,
            cols,
        }),

        Commands::Read {
            session,
            format,
            since,
            tail,
        } => RequestKind::Read(ReadParams {
            session,
            format: Some(format),
            since,
            tail,
        }),

        Commands::Send {
            session,
            input,
            wait_for,
            timeout,
        } => RequestKind::Send(SendParams {
            session,
            input,
            wait_for: wait_for.clone(),
            timeout: if wait_for.is_some() {
                Some(timeout)
            } else {
                None
            },
        }),

        Commands::Wait {
            session,
            pattern,
            timeout,
        } => RequestKind::Wait(WaitParams {
            session,
            pattern,
            timeout,
        }),

        Commands::List => RequestKind::List,

        Commands::Status { session } => RequestKind::Status(StatusParams { session }),

        Commands::Interrupt { session } => RequestKind::Interrupt(InterruptParams { session }),

        Commands::ConfirmDanger {
            command,
            justification,
        } => RequestKind::ConfirmDanger(ConfirmDangerParams {
            command,
            justification,
        }),

        Commands::Terminate { session } => RequestKind::Terminate(TerminateParams { session }),
    };

    let response = DaemonClient::request(&ipc_path, kind).await?;

    if cli.no_json {
        println!("{}", output::format_plain(&response));
    } else {
        println!("{}", output::format_json(&response));
    }

    // Exit with non-zero status on error
    if !response.success {
        std::process::exit(1);
    }

    Ok(())
}
