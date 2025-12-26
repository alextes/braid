//! claim management for multi-agent coordination.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{BrdError, Result};

/// default lease duration in seconds (10 minutes).
pub const DEFAULT_LEASE_SECONDS: u64 = 600;

/// a claim on an issue by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub issue_id: String,
    pub agent_id: String,
    pub pid: u32,
    pub worktree: String,
    pub branch: String,
    pub claimed_at: u64,
    pub lease_until: u64,
}

impl Claim {
    /// create a new claim for the given issue.
    pub fn new(issue_id: String, agent_id: String, worktree: String, branch: String) -> Self {
        let now = current_unix_time();
        Self {
            issue_id,
            agent_id,
            pid: std::process::id(),
            worktree,
            branch,
            claimed_at: now,
            lease_until: now + DEFAULT_LEASE_SECONDS,
        }
    }

    /// check if this claim is expired.
    pub fn is_expired(&self) -> bool {
        current_unix_time() > self.lease_until
    }

    /// check if this claim is active (not expired).
    pub fn is_active(&self) -> bool {
        !self.is_expired()
    }

    /// renew the lease for another DEFAULT_LEASE_SECONDS.
    pub fn renew(&mut self) {
        self.lease_until = current_unix_time() + DEFAULT_LEASE_SECONDS;
    }

    /// load a claim from a file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let claim: Claim = toml::from_str(&content)
            .map_err(|e| BrdError::ParseError(path.display().to_string(), e.to_string()))?;
        Ok(claim)
    }

    /// save claim to a file atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| BrdError::Other(format!("failed to serialize claim: {e}")))?;

        // write atomically via temp file
        let tmp_path = path.with_extension(format!("toml.tmp.{}", std::process::id()));
        std::fs::write(&tmp_path, &content)?;
        std::fs::rename(&tmp_path, path)?;

        Ok(())
    }
}

/// claim state for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    Unclaimed,
    ClaimedByMe,
    ClaimedByOther,
    Expired,
}

/// get the current agent ID:
/// 1. BRD_AGENT_ID env var
/// 2. .braid/agent.toml in worktree
/// 3. fallback: $USER
pub fn get_agent_id(worktree_root: &Path) -> String {
    // 1. check env var
    if let Ok(id) = std::env::var("BRD_AGENT_ID") {
        return id;
    }

    // 2. check .braid/agent.toml
    let agent_toml = worktree_root.join(".braid/agent.toml");
    if let Ok(content) = std::fs::read_to_string(&agent_toml)
        && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
        && let Some(id) = parsed.get("agent_id").and_then(|v| v.as_str())
    {
        return id.to_string();
    }

    // 3. fallback to $USER
    match std::env::var("USER") {
        Ok(user) => user,
        Err(_) => {
            eprintln!("warning: $USER not set, using 'default-user' as agent_id");
            "default-user".to_string()
        }
    }
}

/// get current unix timestamp.
fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_expiry() {
        let mut claim = Claim::new(
            "test-123".to_string(),
            "agent".to_string(),
            "/path".to_string(),
            "main".to_string(),
        );

        assert!(claim.is_active());

        // manually expire it
        claim.lease_until = 0;
        assert!(claim.is_expired());
    }
}
