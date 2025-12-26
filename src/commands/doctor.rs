//! brd doctor command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

use super::load_all_issues;

pub fn cmd_doctor(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let mut errors: Vec<serde_json::Value> = Vec::new();
    let warnings: Vec<serde_json::Value> = Vec::new();

    // check .braid exists
    if !paths.braid_dir().exists() {
        errors.push(serde_json::json!({
            "code": "missing_braid_dir",
            "message": ".braid directory not found"
        }));
    }

    // load and validate all issues
    let issues = load_all_issues(paths)?;

    for (id, issue) in &issues {
        // check for missing deps
        for dep in issue.deps() {
            if !issues.contains_key(dep) {
                errors.push(serde_json::json!({
                    "code": "missing_dep",
                    "issue": id,
                    "dep": dep
                }));
            }
        }
    }

    // check for cycles
    let cycles = crate::graph::find_cycles(&issues);
    for cycle in cycles {
        errors.push(serde_json::json!({
            "code": "cycle",
            "cycle": cycle
        }));
    }

    let ok = errors.is_empty();

    if cli.json {
        let json = serde_json::json!({
            "ok": ok,
            "errors": errors,
            "warnings": warnings
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if ok && warnings.is_empty() {
        println!("✓ All checks passed");
    } else {
        for e in &errors {
            eprintln!("error: {}", e);
        }
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }

    if ok {
        Ok(())
    } else {
        Err(BrdError::Other("doctor found errors".to_string()))
    }
}
