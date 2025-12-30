//! configuration parsing for `.braid/config.toml`.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{BrdError, Result};
use crate::migrate::CURRENT_SCHEMA;

/// the braid configuration stored in `.braid/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// schema version for the repo
    pub schema_version: u32,
    /// prefix for issue IDs (e.g., "mevx")
    pub id_prefix: String,
    /// length of the random suffix (default 4, range 4-10)
    pub id_len: u32,
    /// optional branch for issue tracking (if set, issues live on this branch via shared worktree)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues_branch: Option<String>,
    /// optional external repo for issue tracking (path to another braid repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues_repo: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            id_prefix: "brd".to_string(),
            id_len: 4,
            issues_branch: None,
            issues_repo: None,
        }
    }
}

impl Config {
    /// create a config with a derived prefix from the repo directory name.
    pub fn with_derived_prefix(repo_name: &str) -> Self {
        let prefix = derive_prefix(repo_name);
        Self {
            id_prefix: prefix,
            ..Default::default()
        }
    }

    /// returns true if issues branch mode is enabled (local-sync).
    pub fn is_issues_branch_mode(&self) -> bool {
        self.issues_branch.is_some()
    }

    /// returns true if external repo mode is enabled.
    pub fn is_external_repo_mode(&self) -> bool {
        self.issues_repo.is_some()
    }

    /// load config from a file path, applying migrations if needed.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // parse as generic TOML value to check for migrations
        let mut value: toml::Value = toml::from_str(&content)
            .map_err(|e| BrdError::ParseError(path.display().to_string(), e.to_string()))?;

        // apply config migrations if needed
        let migrated = migrate_config(&mut value);

        // deserialize into Config
        let config: Config = value.try_into().map_err(|e: toml::de::Error| {
            BrdError::ParseError(path.display().to_string(), e.to_string())
        })?;

        // save if migrations were applied
        if migrated {
            config.save(path)?;
        }

        Ok(config)
    }

    /// save config to a file path.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| BrdError::Other(format!("failed to serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// validate the config.
    /// fails if the repo uses a newer schema than this brd version supports.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version > CURRENT_SCHEMA {
            return Err(BrdError::Other(format!(
                "this repo uses schema v{}, but this brd only supports up to v{}. please upgrade brd.",
                self.schema_version, CURRENT_SCHEMA
            )));
        }
        if self.id_len < 4 || self.id_len > 10 {
            return Err(BrdError::ParseError(
                "config".to_string(),
                format!("id_len must be between 4 and 10, got {}", self.id_len),
            ));
        }
        if self.id_prefix.len() < 2 || self.id_prefix.len() > 12 {
            return Err(BrdError::ParseError(
                "config".to_string(),
                format!(
                    "id_prefix must be 2-12 chars, got {} chars",
                    self.id_prefix.len()
                ),
            ));
        }
        Ok(())
    }
}

/// apply config file migrations. returns true if any migrations were applied.
fn migrate_config(value: &mut toml::Value) -> bool {
    let mut migrated = false;

    if let toml::Value::Table(table) = value {
        // get current schema version
        let schema_version = table
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;

        // v4 -> v5: rename sync_branch to issues_branch
        if schema_version < 5 {
            if let Some(sync_branch) = table.remove("sync_branch") {
                table.insert("issues_branch".to_string(), sync_branch);
            }
            table.insert(
                "schema_version".to_string(),
                toml::Value::Integer(CURRENT_SCHEMA as i64),
            );
            migrated = true;
        }
    }

    migrated
}

/// derive the id_prefix from the repo directory name per spec section 6.1.
/// takes first 4 alphanumeric chars, lowercased, pads with 'x' if needed.
fn derive_prefix(name: &str) -> String {
    let chars: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .collect::<String>()
        .to_lowercase();

    if chars.len() >= 4 {
        chars
    } else {
        format!("{:x<4}", chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_prefix() {
        assert_eq!(derive_prefix("my-repo"), "myre");
        assert_eq!(derive_prefix("A!"), "axxx");
        assert_eq!(derive_prefix("braid"), "brai");
        assert_eq!(derive_prefix("X"), "xxxx");
        assert_eq!(derive_prefix("123abc"), "123a");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        // schema_version at current is ok
        config.schema_version = CURRENT_SCHEMA;
        assert!(config.validate().is_ok());

        // schema_version below current is ok (old repo)
        config.schema_version = 1;
        assert!(config.validate().is_ok());

        // schema_version above current should fail (repo newer than brd)
        config.schema_version = CURRENT_SCHEMA + 1;
        assert!(config.validate().is_err());

        config.schema_version = CURRENT_SCHEMA;
        config.id_len = 3;
        assert!(config.validate().is_err());

        config.id_len = 4;
        config.id_prefix = "x".to_string();
        assert!(config.validate().is_err());
    }
}
