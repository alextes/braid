//! agent session management for tracking spawned claude agents.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

/// status of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// actively processing
    Running,
    /// waiting for user input
    Waiting,
    /// finished successfully
    Completed,
    /// exited with error
    Failed,
    /// terminated by user
    Killed,
    /// process died unexpectedly
    Zombie,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Waiting => write!(f, "waiting"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Killed => write!(f, "killed"),
            SessionStatus::Zombie => write!(f, "zombie"),
        }
    }
}

/// agent session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// human-friendly session ID (agent-1, agent-2, etc.)
    pub session_id: String,
    /// claude's internal session ID (UUID)
    pub claude_session_id: String,
    /// process ID of the claude process
    pub pid: u32,
    /// issue being worked on
    pub issue_id: String,
    /// path to the worktree (if any)
    pub worktree: Option<PathBuf>,
    /// current status
    pub status: SessionStatus,
    /// when the session started
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    /// budget limit in USD
    pub budget_usd: f64,
    /// cost spent so far in USD
    pub cost_usd: f64,
    /// model being used
    pub model: String,
}

impl Session {
    /// create a new session.
    pub fn new(
        session_id: String,
        claude_session_id: String,
        pid: u32,
        issue_id: String,
        worktree: Option<PathBuf>,
        budget_usd: f64,
        model: String,
    ) -> Self {
        Self {
            session_id,
            claude_session_id,
            pid,
            issue_id,
            worktree,
            status: SessionStatus::Running,
            started_at: OffsetDateTime::now_utc(),
            budget_usd,
            cost_usd: 0.0,
            model,
        }
    }

    /// load a session from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// save the session to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// path to the session state file.
    pub fn state_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
        sessions_dir.join(format!("{}.json", session_id))
    }

    /// path to the session log file.
    pub fn log_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
        sessions_dir.join(format!("{}.log", session_id))
    }

    /// path to the session stdin pipe.
    pub fn stdin_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
        sessions_dir.join(format!("{}.stdin", session_id))
    }

    /// check if the process is still alive.
    pub fn is_process_alive(&self) -> bool {
        is_pid_alive(self.pid)
    }

    /// update status based on process state.
    pub fn refresh_status(&mut self) {
        if (self.status == SessionStatus::Running || self.status == SessionStatus::Waiting)
            && !self.is_process_alive()
        {
            self.status = SessionStatus::Zombie;
        }
    }
}

/// check if a process with the given PID is alive.
#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    // kill with signal 0 checks if process exists without sending a signal
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    // fallback for non-unix: assume alive
    true
}

/// generate the next sequential session ID.
pub fn next_session_id(sessions_dir: &Path) -> String {
    let mut max_num = 0;

    if let Ok(entries) = fs::read_dir(sessions_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // parse "agent-N.json" pattern
            if let Some(rest) = name_str.strip_prefix("agent-")
                && let Some(num_str) = rest.strip_suffix(".json")
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_num = max_num.max(num);
            }
        }
    }

    format!("agent-{}", max_num + 1)
}

/// load all sessions from the sessions directory.
pub fn load_all_sessions(sessions_dir: &Path) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();

    if !sessions_dir.exists() {
        return Ok(sessions);
    }

    for entry in fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "json") {
            match Session::load(&path) {
                Ok(mut session) => {
                    session.refresh_status();
                    sessions.push(session);
                }
                Err(e) => {
                    eprintln!("warning: failed to load session {}: {}", path.display(), e);
                }
            }
        }
    }

    // sort by started_at, newest first
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(sessions)
}

/// find a session by ID (supports partial matching).
pub fn find_session(sessions_dir: &Path, id: &str) -> Result<Session> {
    let sessions = load_all_sessions(sessions_dir)?;

    let matches: Vec<_> = sessions
        .iter()
        .filter(|s| s.session_id == id || s.session_id.contains(id))
        .collect();

    match matches.len() {
        0 => Err(BrdError::SessionNotFound(id.to_string())),
        1 => Ok(matches[0].clone()),
        _ => {
            let ids: Vec<_> = matches.iter().map(|s| s.session_id.as_str()).collect();
            Err(BrdError::Other(format!(
                "ambiguous session ID '{}', matches: {}",
                id,
                ids.join(", ")
            )))
        }
    }
}

impl RepoPaths {
    /// path to the sessions directory.
    pub fn sessions_dir(&self) -> PathBuf {
        self.brd_common_dir.join("sessions")
    }

    /// ensure the sessions directory exists.
    pub fn ensure_sessions_dir(&self) -> Result<PathBuf> {
        let dir = self.sessions_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_next_session_id_empty() {
        let dir = tempdir().unwrap();
        let id = next_session_id(dir.path());
        assert_eq!(id, "agent-1");
    }

    #[test]
    fn test_next_session_id_sequential() {
        let dir = tempdir().unwrap();

        // create some session files
        fs::write(dir.path().join("agent-1.json"), "{}").unwrap();
        fs::write(dir.path().join("agent-2.json"), "{}").unwrap();
        fs::write(dir.path().join("agent-5.json"), "{}").unwrap();

        let id = next_session_id(dir.path());
        assert_eq!(id, "agent-6");
    }

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Running.to_string(), "running");
        assert_eq!(SessionStatus::Waiting.to_string(), "waiting");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_session_paths() {
        let dir = PathBuf::from("/tmp/sessions");
        assert_eq!(
            Session::state_path(&dir, "agent-1"),
            PathBuf::from("/tmp/sessions/agent-1.json")
        );
        assert_eq!(
            Session::log_path(&dir, "agent-1"),
            PathBuf::from("/tmp/sessions/agent-1.log")
        );
        assert_eq!(
            Session::stdin_path(&dir, "agent-1"),
            PathBuf::from("/tmp/sessions/agent-1.stdin")
        );
    }

    #[test]
    fn test_session_save_load() {
        let dir = tempdir().unwrap();
        let session = Session::new(
            "agent-1".to_string(),
            "uuid-123".to_string(),
            12345,
            "brd-xyz".to_string(),
            Some(PathBuf::from("/tmp/worktree")),
            1.0,
            "claude-sonnet-4-20250514".to_string(),
        );

        let path = dir.path().join("agent-1.json");
        session.save(&path).unwrap();

        let loaded = Session::load(&path).unwrap();
        assert_eq!(loaded.session_id, "agent-1");
        assert_eq!(loaded.issue_id, "brd-xyz");
        assert_eq!(loaded.pid, 12345);
    }
}
