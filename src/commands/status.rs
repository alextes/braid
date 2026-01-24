//! brd status command.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use time::OffsetDateTime;

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
    dirty_count: usize,
}

#[derive(Clone, Debug)]
struct ActiveAgent {
    name: String,
    issue_id: String,
    issue_title: String,
    duration_mins: Option<i64>,
}

#[derive(Clone, Debug)]
struct AgentInfo {
    total: usize,
    active: Vec<ActiveAgent>,
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
            Status::Open => counts.open += 1,
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
    let dirty_suffix = if sync.dirty_count > 0 {
        format!(" ({} dirty)", sync.dirty_count)
    } else {
        String::new()
    };

    match &sync.state {
        SyncState::UpToDate => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("up to date with {}{}", upstream, dirty_suffix)
            } else {
                format!("up to date{}", dirty_suffix)
            }
        }
        SyncState::Ahead(count) => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("ahead of {} by {}{}", upstream, count, dirty_suffix)
            } else {
                format!("ahead by {}{}", count, dirty_suffix)
            }
        }
        SyncState::Behind(count) => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!("behind {} by {}{}", upstream, count, dirty_suffix)
            } else {
                format!("behind by {}{}", count, dirty_suffix)
            }
        }
        SyncState::Diverged { ahead, behind } => {
            if let Some(upstream) = sync.upstream.as_deref() {
                format!(
                    "diverged from {} (ahead {}, behind {}){}",
                    upstream, ahead, behind, dirty_suffix
                )
            } else {
                format!(
                    "diverged (ahead {}, behind {}){}",
                    ahead, behind, dirty_suffix
                )
            }
        }
        SyncState::NoUpstream => format!("no upstream{}", dirty_suffix),
        SyncState::MissingWorktree => "issues worktree missing".to_string(),
        SyncState::Unknown => format!("unknown{}", dirty_suffix),
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
        "dirty_count": sync.dirty_count,
    })
}

fn format_duration(mins: i64) -> String {
    if mins < 60 {
        format!("{}m", mins)
    } else if mins < 60 * 24 {
        format!("{}h", mins / 60)
    } else {
        format!("{}d", mins / (60 * 24))
    }
}

fn format_agent_line(agents: &AgentInfo) -> String {
    if agents.total == 0 && agents.active.is_empty() {
        return "none".to_string();
    }

    let active_count = agents.active.len();
    let summary = format!("{} total, {} active", agents.total, active_count);

    if active_count == 0 {
        return summary;
    }

    if active_count == 1 {
        let agent = &agents.active[0];
        let duration = agent
            .duration_mins
            .map(|m| format!(" for {}", format_duration(m)))
            .unwrap_or_default();
        return format!(
            "{} ({} on {}{})",
            summary, agent.name, agent.issue_id, duration
        );
    }

    // Multiple active agents - just show summary, details in multi-line or JSON
    summary
}

fn agents_to_json(agents: &AgentInfo) -> serde_json::Value {
    serde_json::json!({
        "total": agents.total,
        "active": agents.active.iter().map(|a| {
            serde_json::json!({
                "name": a.name,
                "issue_id": a.issue_id,
                "issue_title": a.issue_title,
                "duration_mins": a.duration_mins,
            })
        }).collect::<Vec<_>>(),
    })
}

struct StatusInfo<'a> {
    mode: &'a str,
    branch: Option<&'a str>,
    agent: &'a str,
    prefix: &'a str,
    counts: IssueCounts,
    sync: Option<&'a SyncInfo>,
    agents: &'a AgentInfo,
}

fn format_status_output(info: &StatusInfo, json: bool) -> String {
    let StatusInfo {
        mode,
        branch,
        agent,
        prefix,
        counts,
        sync,
        agents,
    } = info;
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
            "agents": agents_to_json(agents),
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
        format_issue_counts(*counts),
        width = LABEL_WIDTH
    );
    let _ = writeln!(
        output,
        "{:<width$}{}",
        "Agents:",
        format_agent_line(agents),
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

fn count_dirty_files(cwd: &Path) -> usize {
    // Count uncommitted changes (staged or unstaged)
    git_output(&["status", "--porcelain"], cwd)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

fn get_worktrees_dir(paths: &RepoPaths) -> Option<PathBuf> {
    let home_dir = std::env::var("HOME").ok().map(PathBuf::from)?;
    let braid_worktrees = home_dir.join(".braid").join("worktrees");

    // Check if we're already inside a braid worktree (~/.braid/worktrees/<repo>/<agent>)
    if paths.worktree_root.starts_with(&braid_worktrees) {
        // Extract repo name from the path: ~/.braid/worktrees/<repo-name>/<agent-name>
        let relative = paths.worktree_root.strip_prefix(&braid_worktrees).ok()?;
        let repo_name = relative.components().next()?.as_os_str().to_str()?;
        return Some(braid_worktrees.join(repo_name));
    }

    // Otherwise, use the worktree root's directory name as repo name
    let repo_name = paths.worktree_root.file_name()?.to_str()?;
    Some(braid_worktrees.join(repo_name))
}

fn count_agent_worktrees(paths: &RepoPaths) -> usize {
    let Some(worktrees_dir) = get_worktrees_dir(paths) else {
        return 0;
    };

    if !worktrees_dir.exists() {
        return 0;
    }

    std::fs::read_dir(&worktrees_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0)
}

fn get_active_agents(issues: &HashMap<String, Issue>) -> Vec<ActiveAgent> {
    let now = OffsetDateTime::now_utc();

    issues
        .values()
        .filter(|issue| issue.status() == Status::Doing)
        .filter_map(|issue| {
            let name = issue.frontmatter.owner.clone()?;
            let duration_mins = issue
                .frontmatter
                .started_at
                .map(|started| (now - started).whole_minutes());

            Some(ActiveAgent {
                name,
                issue_id: issue.id().to_string(),
                issue_title: issue.title().to_string(),
                duration_mins,
            })
        })
        .collect()
}

fn get_agent_info(paths: &RepoPaths, issues: &HashMap<String, Issue>) -> AgentInfo {
    AgentInfo {
        total: count_agent_worktrees(paths),
        active: get_active_agents(issues),
    }
}

fn get_sync_info(paths: &RepoPaths, branch: &str) -> SyncInfo {
    let issues_wt = paths.issues_worktree_dir();
    if !issues_wt.exists() {
        return SyncInfo {
            upstream: None,
            state: SyncState::MissingWorktree,
            dirty_count: 0,
        };
    }

    let upstream = get_upstream(branch, &issues_wt);
    let dirty_count = count_dirty_files(&issues_wt);
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

    SyncInfo {
        upstream,
        state,
        dirty_count,
    }
}

pub fn cmd_status(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let counts = count_issues(&issues);
    let agent_id = repo::get_agent_id(&paths.worktree_root);
    let agents = get_agent_info(paths, &issues);

    let (mode, branch, sync) = match config.issues_branch.as_deref() {
        Some(branch) => (
            "local-sync",
            Some(branch),
            Some(get_sync_info(paths, branch)),
        ),
        None => ("git-native", None, None),
    };

    let info = StatusInfo {
        mode,
        branch,
        agent: &agent_id,
        prefix: &config.id_prefix,
        counts,
        sync: sync.as_ref(),
        agents: &agents,
    };
    let output = format_status_output(&info, cli.json);
    print!("{output}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_agents() -> AgentInfo {
        AgentInfo {
            total: 0,
            active: vec![],
        }
    }

    #[test]
    fn test_format_status_output_git_native() {
        let counts = IssueCounts {
            open: 2,
            doing: 1,
            done: 3,
            skip: 0,
        };
        let info = StatusInfo {
            mode: "git-native",
            branch: None,
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: None,
            agents: &empty_agents(),
        };
        let output = format_status_output(&info, false);

        assert!(output.contains("Mode:     git-native"));
        assert!(output.contains("Agent:    agent-one"));
        assert!(output.contains("Prefix:   brd"));
        assert!(output.contains("Issues:   2 open, 1 doing, 3 done"));
        assert!(output.contains("Agents:   none"));
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
            dirty_count: 0,
        };
        let info = StatusInfo {
            mode: "local-sync",
            branch: Some("braid-issues"),
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: Some(&sync),
            agents: &empty_agents(),
        };
        let output = format_status_output(&info, false);

        assert!(output.contains("Mode:     local-sync (branch: braid-issues)"));
        assert!(output.contains("Issues:   1 open, 0 doing, 0 done, 1 skip"));
        assert!(output.contains("Sync:     up to date with origin/braid-issues"));
    }

    #[test]
    fn test_format_status_output_dirty() {
        let counts = IssueCounts {
            open: 1,
            doing: 0,
            done: 0,
            skip: 0,
        };
        let sync = SyncInfo {
            upstream: Some("origin/braid-issues".to_string()),
            state: SyncState::UpToDate,
            dirty_count: 3,
        };
        let info = StatusInfo {
            mode: "local-sync",
            branch: Some("braid-issues"),
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: Some(&sync),
            agents: &empty_agents(),
        };
        let output = format_status_output(&info, false);

        assert!(output.contains("Sync:     up to date with origin/braid-issues (3 dirty)"));
    }

    #[test]
    fn test_format_status_output_with_agents() {
        let counts = IssueCounts {
            open: 1,
            doing: 1,
            done: 0,
            skip: 0,
        };
        let agents = AgentInfo {
            total: 2,
            active: vec![ActiveAgent {
                name: "agent-one".to_string(),
                issue_id: "brd-abc1".to_string(),
                issue_title: "test issue".to_string(),
                duration_mins: Some(15),
            }],
        };
        let info = StatusInfo {
            mode: "git-native",
            branch: None,
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: None,
            agents: &agents,
        };
        let output = format_status_output(&info, false);

        assert!(output.contains("Agents:   2 total, 1 active (agent-one on brd-abc1 for 15m)"));
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
            dirty_count: 0,
        };
        let info = StatusInfo {
            mode: "local-sync",
            branch: Some("braid-issues"),
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: Some(&sync),
            agents: &empty_agents(),
        };
        let output = format_status_output(&info, true);
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(json["mode"], "local-sync");
        assert_eq!(json["branch"], "braid-issues");
        assert_eq!(json["agent"], "agent-one");
        assert_eq!(json["prefix"], "brd");
        assert_eq!(json["issues"]["doing"], 2);
        assert_eq!(json["sync"]["status"], "no-upstream");
        assert_eq!(json["sync"]["dirty_count"], 0);
        assert_eq!(json["agents"]["total"], 0);
    }

    #[test]
    fn test_format_status_output_json_dirty() {
        let counts = IssueCounts {
            open: 1,
            doing: 0,
            done: 0,
            skip: 0,
        };
        let sync = SyncInfo {
            upstream: Some("origin/issues".to_string()),
            state: SyncState::UpToDate,
            dirty_count: 5,
        };
        let info = StatusInfo {
            mode: "local-sync",
            branch: Some("issues"),
            agent: "agent-one",
            prefix: "brd",
            counts,
            sync: Some(&sync),
            agents: &empty_agents(),
        };
        let output = format_status_output(&info, true);
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(json["sync"]["status"], "up-to-date");
        assert_eq!(json["sync"]["dirty_count"], 5);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(5), "5m");
        assert_eq!(format_duration(59), "59m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(120), "2h");
        assert_eq!(format_duration(60 * 24), "1d");
        assert_eq!(format_duration(60 * 24 * 3), "3d");
    }
}
