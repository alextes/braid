//! brd status command.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::issue::{Issue, Status};
use crate::repo::{self, RepoPaths};

use super::load_all_issues;

const LABEL_WIDTH: usize = 10;

#[derive(Clone, Copy, Debug)]
struct IssueCounts {
    open: usize,
    doing: usize,
    done: usize,
    skip: usize,
}

#[derive(Clone, Debug)]
enum SyncState {
    UpToDate,
    Ahead(usize),
    Behind(usize),
    Diverged { ahead: usize, behind: usize },
    NoUpstream,
    MissingWorktree,
    Unknown,
}

#[derive(Clone, Debug)]
struct SyncInfo {
    upstream: Option<String>,
    state: SyncState,
}

fn count_issues(issues: &HashMap<String, Issue>) -> IssueCounts {
    let mut counts = IssueCounts {
        open: 0,
        doing: 0,
        done: 0,
        skip: 0,
    };

    for issue in issues.values() {
        match issue.status() {
            Status::Todo => counts.open += 1,
            Status::Doing => counts.doing += 1,
            Status::Done => counts.done += 1,
            Status::Skip => counts.skip += 1,
        }
    }

    counts
}

fn format_issue_counts(counts: IssueCounts) -> String {
    let mut parts = vec![
        format!("{} open", counts.open),
        format!("{} doing", counts.doing),
        format!("{} done", counts.done),
    ];
    if counts.skip > 0 {
        parts.push(format!("{} skip", counts.skip));
    }
    parts.join(", ")
}

fn format_sync_line(sync: &SyncInfo) -> String {
    match &sync.state {
        SyncState::UpToDate => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("up to date with {}", upstream)
            } else {
                "up to date".to_string()
            }
        }
        SyncState::Ahead(count) => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("ahead of {} by {}", upstream, count)
            } else {
                format!("ahead by {}", count)
            }
        }
        SyncState::Behind(count) => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("behind {} by {}", upstream, count)
            } else {
                format!("behind by {}", count)
            }
        }
        SyncState::Diverged { ahead, behind } => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!(
                    "diverged from {} (ahead {}, behind {})",
                    upstream, ahead, behind
                )
            } else {
                format!("diverged (ahead {}, behind {})", ahead, behind)
            }
        }
        SyncState::NoUpstream => "no upstream".to_string(),
        SyncState::MissingWorktree => "issues worktree missing".to_string(),
        SyncState::Unknown => "unknown".to_string(),
    }
}

fn sync_status_label(state: &SyncState) -> &'static str {
    match state {
        SyncState::UpToDate => "up-to-date",
        SyncState::Ahead(_) => "ahead",
        SyncState::Behind(_) => "behind",
        SyncState::Diverged { .. } => "diverged",
        SyncState::NoUpstream => "no-upstream",
        SyncState::MissingWorktree => "missing-worktree",
        SyncState::Unknown => "unknown",
    }
}

fn sync_counts(state: &SyncState) -> (Option<usize>, Option<usize>) {
    match state {
        SyncState::UpToDate => (Some(0), Some(0)),
        SyncState::Ahead(ahead) => (Some(*ahead), Some(0)),
        SyncState::Behind(behind) => (Some(0), Some(*behind)),
        SyncState::Diverged { ahead, behind } => (Some(*ahead), Some(*behind)),
        SyncState::NoUpstream | SyncState::MissingWorktree | SyncState::Unknown => (None, None),
    }
}

fn sync_to_json(sync: &SyncInfo) -> serde_json::Value {
    let (ahead, behind) = sync_counts(&sync.state);
    serde_json::json!({
        "status": sync_status_label(&sync.state),
        "upstream": sync.upstream,
        "ahead": ahead,
        "behind": behind,
    })
}

fn format_status_output(
    mode: &str,
    branch: Option<&str>,
    agent: &str,
    prefix: &str,
    counts: IssueCounts,
    sync: Option<&SyncInfo>,
    json: bool,
) -> String {
    if json {
        let mut root = serde_json::json!({
            "mode": mode,
            "agent": agent,
            "prefix": prefix,
            "issues": {
                "open": counts.open,
                "doing": counts.doing,
                "done": counts.done,
                "skip": counts.skip,
            },
        });

        if let Some(branch) = branch {
            root["branch"] = serde_json::Value::String(branch.to_string());
        }
        if let Some(sync) = sync {
            root["sync"] = sync_to_json(sync);
        }

        let mut output = serde_json::to_string_pretty(&root).unwrap();
        output.push('\n');
        return output;
    }

    let mut output = String::new();
    let header = "braid status";
    let underline = "-".repeat(header.len());

    let mode_line = match branch {
        Some(branch) => format!("local-sync (branch: {})", branch),
        None => "git-native".to_string(),
    };

    let _ = writeln!(output, "{header}");
    let _ = writeln!(output, "{underline}");
    let _ = writeln!(
        output,
        "{:<width$}{}",
        "Mode:",
        mode_line,
        width = LABEL_WIDTH
    );
    let _ = writeln!(output, "{:<width$}{}", "Agent:", agent, width = LABEL_WIDTH);
    let _ = writeln!(
        output,
        "{:<width$}{}",
        "Prefix:",
        prefix,
        width = LABEL_WIDTH
    );
    let _ = writeln!(
        output,
        "{:<width$}{}",
        "Issues:",
        format_issue_counts(counts),
        width = LABEL_WIDTH
    );

    if let Some(sync) = sync {
        let _ = writeln!(
            output,
            "{:<width$}{}",
            "Sync:",
            format_sync_line(sync),
            width = LABEL_WIDTH
        );
    }

    output
}

fn git_output(args: &[&str], cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_upstream(branch: &str, cwd: &Path) -> Option<String> {
    let upstream = git_output(
        &["rev-parse", "--abbrev-ref", &format!("{branch}@{{u}}")],
        cwd,
    )?;
    if upstream.is_empty() {
        None
    } else {
        Some(upstream)
    }
}

fn get_ahead_behind(upstream: &str, cwd: &Path) -> Option<(usize, usize)> {
    let output = git_output(
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{upstream}...HEAD"),
        ],
        cwd,
    )?;
    let mut parts = output.split_whitespace();
    let behind = parts.next()?.parse().ok()?;
    let ahead = parts.next()?.parse().ok()?;
    Some((ahead, behind))
}

fn get_sync_info(paths: &RepoPaths, branch: &str) -> SyncInfo {
    let issues_wt = paths.issues_worktree_dir();
    if !issues_wt.exists() {
        return SyncInfo {
            upstream: None,
            state: SyncState::MissingWorktree,
        };
    }

    let upstream = get_upstream(branch, &issues_wt);
    let state = match upstream.as_deref() {
        None => SyncState::NoUpstream,
        Some(upstream) => match get_ahead_behind(upstream, &issues_wt) {
            Some((ahead, behind)) => {
                if ahead == 0 && behind == 0 {
                    SyncState::UpToDate
                } else if ahead > 0 && behind == 0 {
                    SyncState::Ahead(ahead)
                } else if ahead == 0 && behind > 0 {
                    SyncState::Behind(behind)
                } else {
                    SyncState::Diverged { ahead, behind }
                }
            }
            None => SyncState::Unknown,
        },
    };

    SyncInfo { upstream, state }
}

pub fn cmd_status(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let counts = count_issues(&issues);
    let agent_id = repo::get_agent_id(&paths.worktree_root);

    let (mode, branch, sync) = match config.sync_branch.as_deref() {
        Some(branch) => (
            "local-sync",
            Some(branch),
            Some(get_sync_info(paths, branch)),
        ),
        None => ("git-native", None, None),
    };

    let output = format_status_output(
        mode,
        branch,
        &agent_id,
        &config.id_prefix,
        counts,
        sync.as_ref(),
        cli.json,
    );
    print!("{output}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_status_output_git_native() {
        let counts = IssueCounts {
            open: 2,
            doing: 1,
            done: 3,
            skip: 0,
        };
        let output =
            format_status_output("git-native", None, "agent-one", "brd", counts, None, false);

        assert!(output.contains("Mode:     git-native"));
        assert!(output.contains("Agent:    agent-one"));
        assert!(output.contains("Prefix:   brd"));
        assert!(output.contains("Issues:   2 open, 1 doing, 3 done"));
        assert!(!output.contains("Sync:"));
    }

    #[test]
    fn test_format_status_output_local_sync() {
        let counts = IssueCounts {
            open: 1,
            doing: 0,
            done: 0,
            skip: 1,
        };
        let sync = SyncInfo {
            upstream: Some("origin/braid-issues".to_string()),
            state: SyncState::UpToDate,
        };
        let output = format_status_output(
            "local-sync",
            Some("braid-issues"),
            "agent-one",
            "brd",
            counts,
            Some(&sync),
            false,
        );

        assert!(output.contains("Mode:     local-sync (branch: braid-issues)"));
        assert!(output.contains("Issues:   1 open, 0 doing, 0 done, 1 skip"));
        assert!(output.contains("Sync:     up to date with origin/braid-issues"));
    }

    #[test]
    fn test_format_status_output_json() {
        let counts = IssueCounts {
            open: 0,
            doing: 2,
            done: 4,
            skip: 0,
        };
        let sync = SyncInfo {
            upstream: None,
            state: SyncState::NoUpstream,
        };
        let output = format_status_output(
            "local-sync",
            Some("braid-issues"),
            "agent-one",
            "brd",
            counts,
            Some(&sync),
            true,
        );
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(json["mode"], "local-sync");
        assert_eq!(json["branch"], "braid-issues");
        assert_eq!(json["agent"], "agent-one");
        assert_eq!(json["prefix"], "brd");
        assert_eq!(json["issues"]["doing"], 2);
        assert_eq!(json["sync"]["status"], "no-upstream");
    }
}
