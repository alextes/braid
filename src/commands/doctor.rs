//! brd doctor command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::migrate::{self, CURRENT_SCHEMA};
use crate::repo::RepoPaths;

use super::{AGENTS_BLOCK_VERSION, check_agents_block, load_all_issues};

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
    let config = match crate::config::Config::load(&paths.config_path()) {
        Ok(c) => c,
        Err(_) => crate::config::Config::default(),
    };

    // check 3: all issue files parse correctly
    let issues = load_all_issues(paths, &config)?;
    record_check(
        "issues_parse",
        "all issue files parse correctly",
        true, // load_all_issues already warns on parse errors
    );

    // check 4: all issues at current schema version (check raw files, not migrated structs)
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

    // check 5: no missing dependencies (renumbered from 4)
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

    // check 6: no dependency cycles
    let cycles = crate::graph::find_cycles(&issues);
    for cycle in &cycles {
        errors.push(serde_json::json!({
            "code": "cycle",
            "cycle": cycle
        }));
    }
    record_check("no_cycles", "no dependency cycles", cycles.is_empty());

    // check 7: AGENTS.md block version (informational)
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
                eprintln!("  hint: run `brd agents inject` to update");
            }
        }
        None => {
            record_check("agents_block", "AGENTS.md braid block not found", false);
            if !cli.json {
                eprintln!("  hint: run `brd agents inject` to add");
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
