//! brd doctor command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::migrate::{self, CURRENT_SCHEMA};
use crate::repo::RepoPaths;

use super::{
    AGENTS_BLOCK_VERSION, AgentsBlockMode, check_agents_block, extract_mode, has_issues_symlink,
    load_all_issues,
};

/// Parse frontmatter from markdown content.
fn parse_frontmatter(content: &str) -> Result<(String, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(BrdError::ParseError(
            "frontmatter".into(),
            "missing opening ---".into(),
        ));
    }

    let rest = &content[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| BrdError::ParseError("frontmatter".into(), "missing closing ---".into()))?;

    let frontmatter = rest[..end].trim().to_string();
    let body = rest[end + 4..].trim().to_string();

    Ok((frontmatter, body))
}

pub fn cmd_doctor(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();

    // helper to record a check result
    let mut record_check = |name: &str, description: &str, passed: bool| {
        checks.push(serde_json::json!({
            "name": name,
            "description": description,
            "passed": passed
        }));
        if !cli.json {
            if passed {
                println!("✓ {}", description);
            } else {
                println!("✗ {}", description);
            }
        }
    };

    // check 1: .braid directory exists
    let braid_exists = paths.braid_dir().exists();
    record_check("braid_dir", ".braid directory exists", braid_exists);
    if !braid_exists {
        errors.push(serde_json::json!({
            "code": "missing_braid_dir",
            "message": ".braid directory not found"
        }));
    }

    // check 2: config.toml is valid
    let config_valid =
        paths.config_path().exists() && crate::config::Config::load(&paths.config_path()).is_ok();
    record_check("config_valid", "config.toml is valid", config_valid);
    if !config_valid {
        errors.push(serde_json::json!({
            "code": "invalid_config",
            "message": "config.toml is missing or invalid"
        }));
    }

    // load config for issue operations
    let config = crate::config::Config::load(&paths.config_path()).unwrap_or_default();

    // check 3: external repo config version (if in external-repo mode)
    if let Some(ref external_path) = config.issues_repo {
        match check_external_config(paths, external_path) {
            Ok((ext_version, supported)) => {
                if supported {
                    record_check(
                        "external_config_version",
                        &format!(
                            "external repo config at schema v{} (supported)",
                            ext_version
                        ),
                        true,
                    );
                } else {
                    record_check(
                        "external_config_version",
                        &format!(
                            "external repo uses schema v{}, this brd supports up to v{}",
                            ext_version, CURRENT_SCHEMA
                        ),
                        false,
                    );
                    errors.push(serde_json::json!({
                        "code": "external_schema_unsupported",
                        "message": format!(
                            "external repo uses schema v{}, please upgrade brd",
                            ext_version
                        )
                    }));
                }
            }
            Err(msg) => {
                record_check("external_config_version", &msg, false);
                errors.push(serde_json::json!({
                    "code": "external_config_error",
                    "message": msg
                }));
            }
        }
    }

    // check 4: issues worktree config version (if in local-sync mode)
    if config.is_issues_branch_mode() {
        match check_worktree_config(paths) {
            Ok(Some((wt_version, supported))) => {
                if supported {
                    record_check(
                        "worktree_config_version",
                        &format!(
                            "issues worktree config at schema v{} (supported)",
                            wt_version
                        ),
                        true,
                    );
                } else {
                    record_check(
                        "worktree_config_version",
                        &format!(
                            "issues worktree uses schema v{}, this brd supports up to v{}",
                            wt_version, CURRENT_SCHEMA
                        ),
                        false,
                    );
                    errors.push(serde_json::json!({
                        "code": "worktree_schema_unsupported",
                        "message": format!(
                            "issues worktree uses schema v{}, please upgrade brd",
                            wt_version
                        )
                    }));
                }
            }
            Ok(None) => {
                // worktree doesn't exist yet, that's ok
                record_check(
                    "worktree_config_version",
                    "issues worktree not yet created",
                    true,
                );
            }
            Err(msg) => {
                record_check("worktree_config_version", &msg, false);
                errors.push(serde_json::json!({
                    "code": "worktree_config_error",
                    "message": msg
                }));
            }
        }

        // check 5: issues symlink for editor visibility (informational)
        let has_symlink = has_issues_symlink(paths);
        record_check(
            "issues_symlink",
            "issues symlink for editor visibility",
            has_symlink,
        );
        if !has_symlink && !cli.json {
            eprintln!("  hint: use `brd edit <id>` to open issues, or recreate symlink manually:");
            eprintln!("    ln -s {} .braid/issues", paths.issues_dir(&config).display());
        }
    }

    // check 6: all issue files parse correctly
    let issues = load_all_issues(paths, &config)?;
    record_check(
        "issues_parse",
        "all issue files parse correctly",
        true, // load_all_issues already warns on parse errors
    );

    // check 6: all issues at current schema version (check raw files, not migrated structs)
    let mut needs_migration = Vec::new();
    let issues_dir = paths.issues_dir(&config);
    if issues_dir.exists() {
        for entry in std::fs::read_dir(&issues_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }

            let content = std::fs::read_to_string(&path)?;
            if let Ok((frontmatter_str, _)) = parse_frontmatter(&content)
                && let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&frontmatter_str)
            {
                let version = migrate::get_schema_version(&yaml).unwrap_or(0);
                if migrate::needs_migration(version) {
                    let id = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    needs_migration.push(id.to_string());
                }
            }
        }
    }
    let schema_ok = needs_migration.is_empty();
    record_check(
        "schema_current",
        &format!("all issues at schema v{}", CURRENT_SCHEMA),
        schema_ok,
    );
    if !schema_ok {
        // This is a warning, not an error - issues still work
        if !cli.json {
            eprintln!(
                "  warning: {} issue(s) need migration, run `brd migrate`",
                needs_migration.len()
            );
        }
    }

    // check 7: no missing dependencies
    let mut missing_deps = Vec::new();
    for (id, issue) in &issues {
        for dep in issue.deps() {
            if !issues.contains_key(dep) {
                missing_deps.push((id.clone(), dep.clone()));
                errors.push(serde_json::json!({
                    "code": "missing_dep",
                    "issue": id,
                    "dep": dep
                }));
            }
        }
    }
    record_check(
        "no_missing_deps",
        "no missing dependencies",
        missing_deps.is_empty(),
    );

    // check 8: no dependency cycles
    let cycles = crate::graph::find_cycles(&issues);
    for cycle in &cycles {
        errors.push(serde_json::json!({
            "code": "cycle",
            "cycle": cycle
        }));
    }
    record_check("no_cycles", "no dependency cycles", cycles.is_empty());

    // check 9: AGENTS.md block version (informational)
    let agents_block_version = check_agents_block(paths);
    match agents_block_version {
        Some(version) if version >= AGENTS_BLOCK_VERSION => {
            record_check(
                "agents_block",
                &format!("AGENTS.md braid block at v{}", AGENTS_BLOCK_VERSION),
                true,
            );
        }
        Some(version) => {
            record_check(
                "agents_block",
                &format!(
                    "AGENTS.md braid block outdated (v{} < v{})",
                    version, AGENTS_BLOCK_VERSION
                ),
                false,
            );
            if !cli.json {
                eprintln!("  hint: run `brd agent inject` to update");
            }
        }
        None => {
            record_check("agents_block", "AGENTS.md braid block not found", false);
            if !cli.json {
                eprintln!("  hint: run `brd agent inject` to add");
            }
        }
    }

    // check 10: AGENTS.md block mode matches config mode (informational)
    let agents_path = paths.worktree_root.join("AGENTS.md");
    if agents_path.exists()
        && let Ok(content) = std::fs::read_to_string(&agents_path)
    {
        let block_mode = extract_mode(&content);
        let config_mode = if config.is_issues_branch_mode() {
            AgentsBlockMode::LocalSync
        } else {
            AgentsBlockMode::GitNative
        };

        match block_mode {
            Some(mode) if mode == config_mode => {
                record_check(
                    "agents_block_mode",
                    &format!("AGENTS.md block mode matches config ({})", config_mode),
                    true,
                );
            }
            Some(mode) => {
                record_check(
                    "agents_block_mode",
                    &format!(
                        "AGENTS.md block mode mismatch ({} != {})",
                        mode, config_mode
                    ),
                    false,
                );
                if !cli.json {
                    eprintln!("  current mode: {}", config_mode);
                    eprintln!("  AGENTS.md block: {}", mode);
                    eprintln!("  run `brd agent inject` to update");
                }
            }
            None => {
                // block exists but no mode detected - already handled by version check
            }
        }
    }

    let ok = errors.is_empty();

    if cli.json {
        let json = serde_json::json!({
            "ok": ok,
            "checks": checks,
            "errors": errors
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if !ok {
        println!();
        for e in &errors {
            if let Some(code) = e.get("code").and_then(|c| c.as_str()) {
                match code {
                    "missing_dep" => {
                        let issue = e.get("issue").and_then(|i| i.as_str()).unwrap_or("?");
                        let dep = e.get("dep").and_then(|d| d.as_str()).unwrap_or("?");
                        eprintln!("  error: {} depends on missing issue {}", issue, dep);
                    }
                    "cycle" => {
                        if let Some(cycle) = e.get("cycle") {
                            eprintln!("  error: dependency cycle: {}", cycle);
                        }
                    }
                    _ => {
                        if let Some(msg) = e.get("message").and_then(|m| m.as_str()) {
                            eprintln!("  error: {}", msg);
                        }
                    }
                }
            }
        }
    }

    if ok {
        Ok(())
    } else {
        Err(BrdError::Other("doctor found errors".to_string()))
    }
}

/// Check external repo config and return (schema_version, is_supported).
fn check_external_config(
    paths: &RepoPaths,
    external_path: &str,
) -> std::result::Result<(u32, bool), String> {
    use std::path::Path;

    // resolve path
    let resolved = if Path::new(external_path).is_absolute() {
        std::path::PathBuf::from(external_path)
    } else {
        paths.worktree_root.join(external_path)
    };

    let resolved = resolved
        .canonicalize()
        .map_err(|_| format!("external repo not found: {}", external_path))?;

    let external_paths = crate::repo::discover(Some(&resolved))
        .map_err(|_| format!("external path is not a git repo: {}", resolved.display()))?;

    let config = crate::config::Config::load(&external_paths.config_path())
        .map_err(|e| format!("failed to load external config: {}", e))?;

    Ok((
        config.schema_version,
        config.schema_version <= CURRENT_SCHEMA,
    ))
}

/// Check issues worktree config and return Some((schema_version, is_supported)) or None if not created.
fn check_worktree_config(paths: &RepoPaths) -> std::result::Result<Option<(u32, bool)>, String> {
    let wt_config_path = paths
        .issues_worktree_dir()
        .join(".braid")
        .join("config.toml");

    if !wt_config_path.exists() {
        return Ok(None);
    }

    let config = crate::config::Config::load(&wt_config_path)
        .map_err(|e| format!("failed to load worktree config: {}", e))?;

    Ok(Some((
        config.schema_version,
        config.schema_version <= CURRENT_SCHEMA,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // parse_frontmatter tests
    // =========================================================================

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
id: test-123
title: Test Issue
---
Body content here."#;
        let (frontmatter, body) = parse_frontmatter(content).unwrap();
        assert!(frontmatter.contains("id: test-123"));
        assert!(frontmatter.contains("title: Test Issue"));
        assert_eq!(body, "Body content here.");
    }

    #[test]
    fn test_parse_frontmatter_with_leading_whitespace() {
        let content = r#"
---
id: test
---
Body"#;
        let (frontmatter, body) = parse_frontmatter(content).unwrap();
        assert!(frontmatter.contains("id: test"));
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_parse_frontmatter_missing_opening() {
        let content = "no frontmatter here";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing opening ---"));
    }

    #[test]
    fn test_parse_frontmatter_missing_closing() {
        let content = r#"---
id: test
title: No closing"#;
        let result = parse_frontmatter(content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing closing ---"));
    }

    #[test]
    fn test_parse_frontmatter_empty_body() {
        let content = r#"---
id: test
---"#;
        let (frontmatter, body) = parse_frontmatter(content).unwrap();
        assert!(frontmatter.contains("id: test"));
        assert_eq!(body, "");
    }

    #[test]
    fn test_parse_frontmatter_multiline_body() {
        let content = r#"---
id: test
---
Line 1
Line 2
Line 3"#;
        let (_, body) = parse_frontmatter(content).unwrap();
        assert!(body.contains("Line 1"));
        assert!(body.contains("Line 2"));
        assert!(body.contains("Line 3"));
    }

    // =========================================================================
    // Integration-style tests using tempfile
    // =========================================================================

    use std::fs;
    use tempfile::tempdir;

    fn create_test_repo() -> (tempfile::TempDir, crate::repo::RepoPaths) {
        let dir = tempdir().unwrap();
        let paths = crate::repo::RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        (dir, paths)
    }

    fn create_braid_dir(paths: &crate::repo::RepoPaths) {
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
    }

    fn create_valid_config(paths: &crate::repo::RepoPaths) {
        let config = crate::config::Config::default();
        config.save(&paths.config_path()).unwrap();
    }

    fn create_issue(paths: &crate::repo::RepoPaths, id: &str, deps: &[&str]) {
        let config = crate::config::Config::default();
        let deps_yaml = if deps.is_empty() {
            "deps: []".to_string()
        } else {
            format!(
                "deps:\n{}",
                deps.iter()
                    .map(|d| format!("  - {}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let content = format!(
            r#"---
schema_version: {}
id: {}
title: Test Issue {}
priority: P2
status: todo
{}
tags: []
owner: ~
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-01T00:00:00Z
acceptance: []
---
Test body."#,
            migrate::CURRENT_SCHEMA,
            id,
            id,
            deps_yaml
        );
        let issue_path = paths.issues_dir(&config).join(format!("{}.md", id));
        fs::write(issue_path, content).unwrap();
    }

    fn make_cli() -> crate::cli::Cli {
        crate::cli::Cli {
            json: true,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    #[test]
    fn test_doctor_missing_braid_dir() {
        let (_dir, paths) = create_test_repo();
        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_missing_config() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        // Don't create config.toml

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_healthy_repo() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);

        let cli = make_cli();

        // Should pass (only AGENTS.md check fails, which is not an error)
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_missing_dependency() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        // Create an issue that depends on a non-existent issue
        create_issue(&paths, "child", &["nonexistent"]);

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_valid_dependency() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "parent", &[]);
        create_issue(&paths, "child", &["parent"]);

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_dependency_cycle() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        // Create a cycle: a -> b -> a
        create_issue(&paths, "issue-a", &["issue-b"]);
        create_issue(&paths, "issue-b", &["issue-a"]);

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_schema_needs_migration() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);

        // Create an issue with old schema version
        let config = crate::config::Config::default();
        let content = r#"---
brd: 1
id: old-issue
title: Old Schema Issue
priority: P2
status: todo
deps: []
tags: []
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-01T00:00:00Z
---
Old issue."#;
        let issue_path = paths.issues_dir(&config).join("old-issue.md");
        fs::write(issue_path, content).unwrap();

        let cli = make_cli();

        // Schema migration warning doesn't cause failure
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_invalid_config_toml() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        // Create an invalid config.toml
        fs::write(paths.config_path(), "this is not valid toml {{{{").unwrap();

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_agents_block_present() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);

        // Create AGENTS.md with current version block
        let config = crate::config::Config::default();
        let block = crate::commands::agent::generate_block(&config);
        fs::write(
            paths.worktree_root.join("AGENTS.md"),
            format!("# Agents\n\n{}", block),
        )
        .unwrap();

        let cli = make_cli();

        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_agents_block_outdated() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);

        // Create AGENTS.md with old version block
        let old_block = r#"<!-- braid:agents:start v1 -->
Old content
<!-- braid:agents:end -->"#;
        fs::write(paths.worktree_root.join("AGENTS.md"), old_block).unwrap();

        let cli = make_cli();

        // Outdated agents block is not an error
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_agents_block_mode_match() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);

        // Create AGENTS.md with git-native mode (matches default config)
        let config = crate::config::Config::default();
        let block = crate::commands::agent::generate_block(&config);
        fs::write(
            paths.worktree_root.join("AGENTS.md"),
            format!("# Agents\n\n{}", block),
        )
        .unwrap();

        let cli = make_cli();

        // Mode matches, should pass
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_agents_block_mode_mismatch() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);

        // Create config with local-sync mode
        let config = crate::config::Config {
            issues_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };
        config.save(&paths.config_path()).unwrap();

        // Create AGENTS.md with git-native mode (mismatches config)
        let git_native_config = crate::config::Config::default();
        let block = crate::commands::agent::generate_block(&git_native_config);
        fs::write(
            paths.worktree_root.join("AGENTS.md"),
            format!("# Agents\n\n{}", block),
        )
        .unwrap();

        let cli = make_cli();

        // Mode mismatch is informational, not an error
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_multiple_errors() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        // Create issues with missing deps AND cycles
        create_issue(&paths, "issue-a", &["issue-b", "nonexistent"]);
        create_issue(&paths, "issue-b", &["issue-a"]);

        let cli = make_cli();

        // Should fail and aggregate multiple errors
        let result = cmd_doctor(&cli, &paths);
        assert!(result.is_err());
    }
}
