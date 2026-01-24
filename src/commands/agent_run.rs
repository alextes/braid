//! agent runtime commands: spawn, ps, logs, send, kill.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;
use crate::session::{Session, SessionStatus, find_session, load_all_sessions, next_session_id};

use super::{load_all_issues, resolve_issue_id};

/// default claude model to use.
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// spawn a claude agent to work on an issue.
pub fn cmd_agent_spawn(
    cli: &Cli,
    paths: &RepoPaths,
    issue_id: &str,
    budget: f64,
    foreground: bool,
    _worktree: bool,
    model: Option<&str>,
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;

    // ensure sessions directory exists
    let sessions_dir = paths.ensure_sessions_dir()?;

    // generate session IDs
    let session_id = next_session_id(&sessions_dir);
    let claude_session_id = Uuid::new_v4().to_string();

    // paths for this session
    let log_path = Session::log_path(&sessions_dir, &session_id);
    let state_path = Session::state_path(&sessions_dir, &session_id);

    // create log file
    let log_file = File::create(&log_path)?;

    // build the prompt
    let prompt = format!(
        "work on issue {}. run `brd show {}` to see details. \
         when done, run `brd done {}`.",
        full_id, full_id, full_id
    );

    let model_str = model.unwrap_or(DEFAULT_MODEL);

    // spawn claude
    let mut cmd = Command::new("claude");
    cmd.args([
        "-p",
        "--verbose",
        "--output-format=stream-json",
        &format!("--session-id={}", claude_session_id),
        &format!("--max-budget-usd={}", budget),
        "--model",
        model_str,
        &prompt,
    ]);
    cmd.current_dir(&paths.worktree_root);
    cmd.stdout(Stdio::from(log_file.try_clone()?));
    cmd.stderr(Stdio::from(log_file));

    if foreground {
        // run in foreground - wait and stream output
        let status = cmd.status()?;

        if cli.json {
            println!(
                r#"{{"ok": {}, "session_id": "{}", "issue_id": "{}"}}"#,
                status.success(),
                session_id,
                full_id
            );
        } else if status.success() {
            println!("agent {} completed", session_id);
        } else {
            println!(
                "agent {} failed with exit code {:?}",
                session_id,
                status.code()
            );
        }
    } else {
        // run in background
        let child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BrdError::Other(
                    "claude CLI not found - install from https://claude.ai/download".to_string(),
                )
            } else {
                BrdError::Io(e)
            }
        })?;

        let pid = child.id();

        // save session state
        let session = Session::new(
            session_id.clone(),
            claude_session_id,
            pid,
            full_id.clone(),
            Some(paths.worktree_root.clone()),
            budget,
            model_str.to_string(),
        );
        session.save(&state_path)?;

        if cli.json {
            println!(
                r#"{{"ok": true, "session_id": "{}", "pid": {}, "issue_id": "{}", "log_file": "{}"}}"#,
                session_id,
                pid,
                full_id,
                log_path.display()
            );
        } else {
            println!("spawned {} (pid {}) for {}", session_id, pid, full_id);
            println!("  logs: brd agent logs {}", session_id);
            println!("  kill: brd agent kill {}", session_id);
        }
    }

    Ok(())
}

/// list running agent sessions.
pub fn cmd_agent_ps(cli: &Cli, paths: &RepoPaths, show_all: bool) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let sessions = load_all_sessions(&sessions_dir)?;

    // filter to active sessions unless --all
    let sessions: Vec<_> = if show_all {
        sessions
    } else {
        sessions
            .into_iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SessionStatus::Running | SessionStatus::Waiting | SessionStatus::Zombie
                )
            })
            .collect()
    };

    if cli.json {
        let json: Vec<_> = sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "session_id": s.session_id,
                    "issue_id": s.issue_id,
                    "status": s.status.to_string(),
                    "pid": s.pid,
                    "started_at": s.started_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
                    "budget_usd": s.budget_usd,
                    "cost_usd": s.cost_usd,
                    "model": s.model,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else if sessions.is_empty() {
        println!("no active sessions");
    } else {
        // table header
        println!(
            "{:<12} {:<12} {:<10} {:>8}  PID",
            "SESSION", "ISSUE", "STATUS", "RUNTIME"
        );

        let now = OffsetDateTime::now_utc();
        for s in &sessions {
            let runtime = now - s.started_at;
            let runtime_str = format_duration(runtime);
            let status_str = format!("{}", s.status);

            println!(
                "{:<12} {:<12} {:<10} {:>8}  {}",
                s.session_id, s.issue_id, status_str, runtime_str, s.pid
            );
        }
    }

    Ok(())
}

/// view agent session output.
pub fn cmd_agent_logs(
    _cli: &Cli,
    paths: &RepoPaths,
    session_id: &str,
    follow: bool,
    tail: Option<usize>,
    raw: bool,
) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let session = find_session(&sessions_dir, session_id)?;
    let log_path = Session::log_path(&sessions_dir, &session.session_id);

    if !log_path.exists() {
        return Err(BrdError::Other(format!(
            "log file not found: {}",
            log_path.display()
        )));
    }

    let file = File::open(&log_path)?;
    let reader = BufReader::new(file);

    // collect lines if we need to tail
    let lines: Vec<String> = if tail.is_some() {
        reader.lines().collect::<std::io::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    let lines_to_show: Box<dyn Iterator<Item = String>> = if let Some(n) = tail {
        let start = lines.len().saturating_sub(n);
        Box::new(lines.into_iter().skip(start))
    } else {
        let file = File::open(&log_path)?;
        Box::new(
            BufReader::new(file)
                .lines()
                .map_while(std::result::Result::ok),
        )
    };

    for line in lines_to_show {
        if raw {
            println!("{}", line);
        } else {
            // parse and pretty-print the JSON event
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                print_event(&event);
            } else {
                println!("{}", line);
            }
        }
    }

    if follow {
        // TODO: implement tail -f behavior
        println!("(follow mode not yet implemented)");
    }

    Ok(())
}

/// send input to a waiting agent.
pub fn cmd_agent_send(cli: &Cli, paths: &RepoPaths, session_id: &str, message: &str) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let session = find_session(&sessions_dir, session_id)?;

    // check if process is alive
    if !session.is_process_alive() {
        return Err(BrdError::Other(format!(
            "agent {} is not running (pid {} not found)",
            session.session_id, session.pid
        )));
    }

    // write to stdin pipe
    let stdin_path = Session::stdin_path(&sessions_dir, &session.session_id);

    // create the pipe if it doesn't exist
    if !stdin_path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // create a regular file as a fallback (proper named pipe requires nix crate)
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&stdin_path)?;
        }
    }

    // for now, just note that stdin piping needs the agent to be spawned with stdin connected
    // this is a limitation of the current implementation
    if cli.json {
        println!(
            r#"{{"ok": false, "error": "stdin piping not yet implemented - agent must be spawned with connected stdin"}}"#
        );
    } else {
        println!(
            "note: stdin piping not yet fully implemented. \
             message '{}' would be sent to {}",
            message, session.session_id
        );
    }

    Ok(())
}

/// terminate a running agent.
pub fn cmd_agent_kill(cli: &Cli, paths: &RepoPaths, session_id: &str, force: bool) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let mut session = find_session(&sessions_dir, session_id)?;

    if !session.is_process_alive() {
        if cli.json {
            println!(
                r#"{{"ok": true, "message": "agent {} already dead"}}"#,
                session.session_id
            );
        } else {
            println!(
                "agent {} already dead (was pid {})",
                session.session_id, session.pid
            );
        }
        return Ok(());
    }

    // send signal
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        let result = unsafe { libc::kill(session.pid as i32, signal) };

        if result != 0 {
            let err = std::io::Error::last_os_error();
            return Err(BrdError::Other(format!(
                "failed to kill process {}: {}",
                session.pid, err
            )));
        }
    }

    #[cfg(not(unix))]
    {
        return Err(BrdError::Other(
            "kill not implemented on this platform".to_string(),
        ));
    }

    // update session state
    session.status = SessionStatus::Killed;
    let state_path = Session::state_path(&sessions_dir, &session.session_id);
    session.save(&state_path)?;

    if cli.json {
        println!(
            r#"{{"ok": true, "session_id": "{}", "signal": "{}"}}"#,
            session.session_id,
            if force { "SIGKILL" } else { "SIGTERM" }
        );
    } else {
        println!(
            "killed {} (pid {}) with {}",
            session.session_id,
            session.pid,
            if force { "SIGKILL" } else { "SIGTERM" }
        );
    }

    Ok(())
}

/// format a duration as a human-readable string.
fn format_duration(d: time::Duration) -> String {
    let minutes = d.whole_minutes();
    if minutes < 60 {
        format!("{}m", minutes.max(1))
    } else if minutes < 60 * 24 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}d", minutes / (60 * 24))
    }
}

/// pretty-print a claude stream-json event.
fn print_event(event: &serde_json::Value) {
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match event_type {
        "assistant" => {
            if let Some(message) = event.get("message")
                && let Some(content) = message.get("content").and_then(|c| c.as_array())
            {
                for item in content {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        println!("{}", text);
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Some(delta) = event.get("delta")
                && let Some(text) = delta.get("text").and_then(|t| t.as_str())
            {
                print!("{}", text);
                std::io::stdout().flush().ok();
            }
        }
        "tool_use" | "tool_result" => {
            if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                println!("[tool: {}]", name);
            }
        }
        "error" => {
            if let Some(msg) = event
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                eprintln!("error: {}", msg);
            }
        }
        "message_stop" | "message_start" => {
            // metadata events, skip
        }
        _ => {
            // unknown event type, print type for debugging
            println!("[{}]", event_type);
        }
    }
}
