//! brd migrate command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::issue::Issue;
use crate::migrate::{self, CURRENT_SCHEMA};
use crate::repo::RepoPaths;

pub fn cmd_migrate(cli: &Cli, paths: &RepoPaths, dry_run: bool) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues_dir = paths.issues_dir(&config);

    if !issues_dir.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "migrated": 0,
                    "issues": []
                })
            );
        } else {
            println!("No issues to migrate.");
        }
        return Ok(());
    }

    let mut migrated = Vec::new();
    let mut results = Vec::new();

    for entry in std::fs::read_dir(&issues_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }

        // Read and parse the raw file to check version
        let content = std::fs::read_to_string(&path)?;
        let (frontmatter_str, _body) = parse_frontmatter(&content)?;
        let frontmatter: serde_yaml::Value = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| crate::error::BrdError::ParseError("yaml".into(), e.to_string()))?;

        let version = migrate::get_schema_version(&frontmatter)?;
        if !migrate::needs_migration(version) {
            continue;
        }

        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let summaries = migrate::migration_summary(version, CURRENT_SCHEMA);

        if dry_run {
            results.push(serde_json::json!({
                "id": id,
                "from_version": version,
                "to_version": CURRENT_SCHEMA,
                "migrations": summaries
            }));

            if !cli.json {
                println!(
                    "{}: v{} → v{} ({})",
                    id,
                    version,
                    CURRENT_SCHEMA,
                    summaries.join(", ")
                );
            }
        } else {
            // Perform actual migration by re-loading and saving
            let issue = Issue::load(&path)?;
            issue.save(&path)?;
            migrated.push(id.clone());

            results.push(serde_json::json!({
                "id": id,
                "from_version": version,
                "to_version": CURRENT_SCHEMA,
                "migrations": summaries
            }));

            if !cli.json {
                println!("migrated {}: v{} → v{}", id, version, CURRENT_SCHEMA);
            }
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "dry_run": dry_run,
            "migrated": if dry_run { 0 } else { migrated.len() },
            "would_migrate": if dry_run { results.len() } else { 0 },
            "issues": results
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if results.is_empty() {
        println!("All issues are up to date (schema v{}).", CURRENT_SCHEMA);
    } else if dry_run {
        println!(
            "\n{} issue(s) would be migrated. Run without --dry-run to apply.",
            results.len()
        );
    } else {
        println!("\nMigrated {} issue(s).", migrated.len());
    }

    Ok(())
}

/// Parse frontmatter from markdown content.
fn parse_frontmatter(content: &str) -> Result<(String, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(crate::error::BrdError::ParseError(
            "frontmatter".into(),
            "missing opening ---".into(),
        ));
    }

    let rest = &content[3..];
    let end = rest.find("\n---").ok_or_else(|| {
        crate::error::BrdError::ParseError("frontmatter".into(), "missing closing ---".into())
    })?;

    let frontmatter = rest[..end].trim().to_string();
    let body = rest[end + 4..].trim().to_string();

    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Priority, Status};
    use std::fs;
    use tempfile::tempdir;

    fn create_test_repo() -> (tempfile::TempDir, RepoPaths, Config) {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
        let config = Config::default();
        config.save(&paths.config_path()).unwrap();
        (dir, paths, config)
    }

    fn write_issue(paths: &RepoPaths, config: &Config, issue: &Issue) -> std::path::PathBuf {
        let issue_path = paths.issues_dir(config).join(format!("{}.md", issue.id()));
        issue.save(&issue_path).unwrap();
        issue_path
    }

    fn make_cli(json: bool) -> Cli {
        Cli {
            json,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    fn read_schema_version(path: &std::path::Path) -> u32 {
        let content = fs::read_to_string(path).unwrap();
        let (frontmatter, _) = parse_frontmatter(&content).unwrap();
        let frontmatter: serde_yaml::Value = serde_yaml::from_str(&frontmatter).unwrap();
        migrate::get_schema_version(&frontmatter).unwrap()
    }

    #[test]
    fn test_cmd_migrate_dry_run_does_not_modify_issue() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.status = Status::Open;
        issue.frontmatter.schema_version = CURRENT_SCHEMA - 1;
        let issue_path = write_issue(&paths, &config, &issue);

        let cli = make_cli(false);
        cmd_migrate(&cli, &paths, true).unwrap();

        assert_eq!(read_schema_version(&issue_path), CURRENT_SCHEMA - 1);
    }

    #[test]
    fn test_cmd_migrate_updates_issue_schema() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.status = Status::Open;
        issue.frontmatter.schema_version = CURRENT_SCHEMA - 1;
        let issue_path = write_issue(&paths, &config, &issue);

        let cli = make_cli(true);
        cmd_migrate(&cli, &paths, false).unwrap();

        assert_eq!(read_schema_version(&issue_path), CURRENT_SCHEMA);
    }
}
