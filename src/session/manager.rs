use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Result};
use uuid::Uuid;

use crate::config::Config;
use crate::session::session::Session;
use crate::session::state::SessionInfo;

/// Manages the lifecycle of all sessions.
#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    config: Config,
}

impl SessionManager {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: HashMap::new(),
            config,
        }
    }

    /// Start a new session.
    pub fn start(
        &mut self,
        name: Option<String>,
        command: Vec<String>,
        rows: Option<u16>,
        cols: Option<u16>,
    ) -> Result<SessionInfo> {
        // Check session cap
        if self.sessions.len() >= self.config.max_sessions {
            bail!(
                "Maximum session limit ({}) reached. Terminate an existing session first.",
                self.config.max_sessions
            );
        }

        // Generate name if not provided
        let name = name.unwrap_or_else(|| {
            let short_id = &Uuid::new_v4().to_string()[..8];
            format!("session-{}", short_id)
        });

        // Check for duplicate name
        if self.sessions.contains_key(&name) {
            bail!("Session '{}' already exists", name);
        }

        // Apply custom size if provided
        let mut session_config = self.config.clone();
        if let Some(r) = rows {
            session_config.default_rows = r;
        }
        if let Some(c) = cols {
            session_config.default_cols = c;
        }

        let session = Session::new(name.clone(), command, &session_config)?;
        let info = session.info();
        self.sessions.insert(name, session);
        Ok(info)
    }

    /// Get a session by name.
    pub fn get(&self, name: &str) -> Result<&Session> {
        self.sessions
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", name))
    }

    /// Get a mutable session by name.
    pub fn get_mut(&mut self, name: &str) -> Result<&mut Session> {
        self.sessions
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", name))
    }

    /// List all sessions.
    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions.values().map(|s| s.info()).collect()
    }

    /// Get status of a specific session.
    pub fn status(&self, name: &str) -> Result<SessionInfo> {
        Ok(self.get(name)?.info())
    }

    /// Send input to a session.
    pub async fn send_input(&mut self, name: &str, input: &str) -> Result<()> {
        let session = self.get_mut(name)?;
        session.send_input(input).await
    }

    /// Read output from a session.
    pub fn read_output(
        &self,
        name: &str,
        since: Option<u64>,
        tail: Option<usize>,
    ) -> Result<(String, u64)> {
        let session = self.get(name)?;
        Ok(session.read_output(since, tail))
    }

    /// Wait for a pattern in a session's output.
    pub async fn wait_for(
        &mut self,
        name: &str,
        pattern: &str,
        timeout: Duration,
    ) -> Result<Option<String>> {
        let session = self.get_mut(name)?;
        session.wait_for_pattern(pattern, timeout).await
    }

    /// Interrupt a session (send Ctrl+C).
    pub async fn interrupt(&self, name: &str) -> Result<()> {
        let session = self.get(name)?;
        session.interrupt().await
    }

    /// Terminate and remove a session.
    pub fn terminate(&mut self, name: &str) -> Result<()> {
        let mut session = self
            .sessions
            .remove(name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", name))?;
        session.terminate()?;
        Ok(())
    }

    /// Process output for all active sessions.
    pub async fn process_all(&mut self) {
        for session in self.sessions.values_mut() {
            session.process_output().await;
            session.update_prompt_state();
        }
    }

    /// Clean up idle sessions that have exceeded the timeout.
    pub fn cleanup_idle(&mut self) -> Vec<String> {
        let timeout = self.config.idle_timeout;
        let idle_names: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_idle(timeout))
            .map(|(name, _)| name.clone())
            .collect();

        for name in &idle_names {
            if let Some(mut session) = self.sessions.remove(name) {
                let _ = session.terminate();
                tracing::info!("Cleaned up idle session: {}", name);
            }
        }

        idle_names
    }

    /// Number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}
