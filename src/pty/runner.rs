use std::io::{Read, Write};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

/// Read buffer size for PTY output (bytes).
const PTY_READ_BUFFER_SIZE: usize = 4096;
/// Channel capacity for PTY read data chunks.
const PTY_READ_CHANNEL_CAPACITY: usize = 1024;
/// Channel capacity for PTY write data chunks.
const PTY_WRITE_CHANNEL_CAPACITY: usize = 256;

/// PTY runner: spawns a command in a PTY and provides async channels for I/O.
pub struct PtyRunner {
    /// Channel to send data to the PTY (write to the process stdin).
    write_tx: mpsc::Sender<Vec<u8>>,
    /// Channel to receive data from the PTY (read from the process stdout).
    read_rx: mpsc::Receiver<Vec<u8>>,
    /// The child process handle.
    child: Box<dyn Child + Send + Sync>,
    /// Keep the master PTY alive — dropping it closes the PTY and EOF's the reader.
    /// Wrapped in Arc<Mutex> for Send+Sync since MasterPty is only Send.
    _master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
    /// Process ID.
    pid: Option<u32>,
}

impl PtyRunner {
    /// Spawn a command in a new PTY.
    pub fn spawn(command: &[String], rows: u16, cols: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        if command.is_empty() {
            anyhow::bail!("Command cannot be empty");
        }

        let mut cmd = CommandBuilder::new(&command[0]);
        for arg in &command[1..] {
            cmd.arg(arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn command in PTY")?;

        let pid = child.process_id();

        // Drop the slave after spawning — the master is our I/O handle
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;

        let writer = pair
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        // Read channel: dedicated thread reads from PTY and sends to async channel
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(PTY_READ_CHANNEL_CAPACITY);
        std::thread::Builder::new()
            .name("pty-reader".to_string())
            .spawn(move || {
                let mut buf = [0u8; PTY_READ_BUFFER_SIZE];
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
            })
            .context("Failed to spawn PTY reader thread")?;

        // Write channel: dedicated thread receives from async channel and writes to PTY
        let (write_tx, mut write_rx_inner) = mpsc::channel::<Vec<u8>>(PTY_WRITE_CHANNEL_CAPACITY);
        std::thread::Builder::new()
            .name("pty-writer".to_string())
            .spawn(move || {
                let mut writer = writer;
                while let Some(data) = write_rx_inner.blocking_recv() {
                    if writer.write_all(&data).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            })
            .context("Failed to spawn PTY writer thread")?;

        Ok(Self {
            write_tx,
            read_rx,
            child,
            _master: Arc::new(StdMutex::new(pair.master)),
            pid,
        })
    }

    /// Send data to the PTY (write to the process).
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        self.write_tx
            .send(data.to_vec())
            .await
            .map_err(|_| anyhow::anyhow!("PTY writer channel closed"))
    }

    /// Send a line of text followed by Enter (carriage return).
    /// PTYs expect `\r` for Enter, not `\n` — the terminal driver handles the translation.
    pub async fn send_line(&self, text: &str) -> Result<()> {
        let mut data = text.as_bytes().to_vec();
        data.push(b'\r');
        self.send(&data).await
    }

    /// Receive a chunk of data from the PTY (non-blocking).
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.read_rx.recv().await
    }

    /// Try to receive data without blocking.
    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        self.read_rx.try_recv().ok()
    }

    /// Check if the child process has exited.
    pub fn try_wait(&mut self) -> Option<portable_pty::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    /// Wait for the child process to exit.
    pub fn wait(&mut self) -> Result<portable_pty::ExitStatus> {
        self.child
            .wait()
            .context("Failed to wait for child process")
    }

    /// Send SIGINT (Ctrl+C) to the process.
    pub async fn interrupt(&self) -> Result<()> {
        // Send Ctrl+C (ETX, 0x03) to the PTY
        self.send(&[0x03]).await
    }

    /// Kill the child process.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill child process")
    }

    /// Resize the PTY window.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self._master
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock PTY master"))?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize PTY")
    }

    /// Get the process ID.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }
}

impl std::fmt::Debug for PtyRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyRunner").field("pid", &self.pid).finish()
    }
}
