use std::io::{Read, Write};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use tokio::sync::mpsc;

/// Pipe-based fallback for when PTY is unavailable.
///
/// Falls back to standard child_process with pipes. Auto-injects
/// interactive flags for known interpreters:
/// - Python: `-u -i` (unbuffered + interactive)
/// - Node: `--interactive`
/// - Bash/zsh: `-i` (but NOT if `-c` is present)
pub struct PipeFallback {
    write_tx: mpsc::Sender<Vec<u8>>,
    read_rx: mpsc::Receiver<Vec<u8>>,
    child: std::process::Child,
}

impl PipeFallback {
    pub fn spawn(command: &[String]) -> Result<Self> {
        if command.is_empty() {
            anyhow::bail!("Command cannot be empty");
        }
        let (program, args) = Self::inject_interactive_flags(command);

        let mut child = Command::new(&program)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn: {}", program))?;

        let stdout = child.stdout.take().context("No stdout")?;
        let stderr = child.stderr.take().context("No stderr")?;
        let stdin = child.stdin.take().context("No stdin")?;

        // Merge stdout and stderr into a single read channel
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(1024);

        let read_tx2 = read_tx.clone();
        std::thread::Builder::new()
            .name("pipe-stdout".to_string())
            .spawn(move || {
                let mut reader = stdout;
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if read_tx.blocking_send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })?;

        std::thread::Builder::new()
            .name("pipe-stderr".to_string())
            .spawn(move || {
                let mut reader = stderr;
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if read_tx2.blocking_send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })?;

        let (write_tx, mut write_rx_inner) = mpsc::channel::<Vec<u8>>(256);
        std::thread::Builder::new()
            .name("pipe-stdin".to_string())
            .spawn(move || {
                let mut writer = stdin;
                while let Some(data) = write_rx_inner.blocking_recv() {
                    if writer.write_all(&data).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            })?;

        Ok(Self {
            write_tx,
            read_rx,
            child,
        })
    }

    /// Auto-inject interactive flags for known interpreters.
    fn inject_interactive_flags(command: &[String]) -> (String, Vec<String>) {
        let program = &command[0];
        let args = &command[1..];
        let base = program.rsplit(['/', '\\']).next().unwrap_or(program);

        // Normalize base name: strip .exe suffix for matching
        let base_lower = base.to_lowercase();
        let base_norm = base_lower.strip_suffix(".exe").unwrap_or(&base_lower);

        match base_norm {
            "python" | "python3" | "python3.11" | "python3.12" | "python3.13" | "pypy"
            | "pypy3" => {
                let mut new_args = vec!["-u".to_string(), "-i".to_string()];
                new_args.extend(args.iter().cloned());
                (program.clone(), new_args)
            }
            "node" | "bun" | "deno" => {
                let mut new_args = vec!["--interactive".to_string()];
                new_args.extend(args.iter().cloned());
                (program.clone(), new_args)
            }
            "bash" | "zsh" | "sh" | "fish" | "dash" | "ksh" => {
                // Don't add -i if -c is present (running a script)
                if args.iter().any(|a| a == "-c") {
                    (program.clone(), args.to_vec())
                } else {
                    let mut new_args = vec!["-i".to_string()];
                    new_args.extend(args.iter().cloned());
                    (program.clone(), new_args)
                }
            }
            _ => (program.clone(), args.to_vec()),
        }
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        self.write_tx
            .send(data.to_vec())
            .await
            .map_err(|_| anyhow::anyhow!("Pipe writer channel closed"))
    }

    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.read_rx.recv().await
    }

    pub fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill child process")
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl std::fmt::Debug for PipeFallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeFallback")
            .field("pid", &self.child.id())
            .finish()
    }
}
