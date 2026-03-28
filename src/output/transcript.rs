use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;

/// Append-only transcript logger that writes session output to disk.
#[derive(Debug)]
pub struct Transcript {
    path: PathBuf,
}

impl Transcript {
    /// Create a new transcript, ensuring the parent directory exists.
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Create or truncate the file
        fs::write(&path, "")?;
        Ok(Self { path })
    }

    /// Open an existing transcript (or create if missing).
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            fs::write(&path, "")?;
        }
        Ok(Self { path })
    }

    /// Append text to the transcript with a timestamp.
    pub fn append(&self, text: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        for line in text.lines() {
            writeln!(file, "[{}] {}", timestamp, line)?;
        }
        file.flush()?;
        Ok(())
    }

    /// Append raw text without timestamps.
    pub fn append_raw(&self, text: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        write!(file, "{}", text)?;
        file.flush()?;
        Ok(())
    }

    /// Get the transcript file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the full transcript.
    pub fn read_all(&self) -> Result<String> {
        Ok(fs::read_to_string(&self.path)?)
    }
}
