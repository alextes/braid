//! brd search command - prints instructions for searching issues.

use crate::cli::Cli;
use crate::error::Result;
use crate::repo::RepoPaths;

pub fn cmd_search(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let issues_dir = paths.issues_dir();

    if cli.json {
        let json = serde_json::json!({
            "issues_dir": issues_dir.to_string_lossy(),
            "hint": "use grep or rg to search issue files"
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("braid issues are plain markdown files. use grep or rg to search:");
        println!();
        println!("  rg <pattern> {}", issues_dir.display());
        println!("  grep -r <pattern> {}", issues_dir.display());
        println!();
        println!("examples:");
        println!(
            "  rg 'authentication' {}   # search for 'authentication'",
            issues_dir.display()
        );
        println!(
            "  rg -l 'P0' {}             # list P0 issues",
            issues_dir.display()
        );
    }

    Ok(())
}
