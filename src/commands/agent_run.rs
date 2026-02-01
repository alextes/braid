//! agent runtime commands: spawn, ps, logs, send, kill.

use std::collections::HashMap;
use std::fs::File;
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

    if foreground {
        // foreground mode: inherit stdio so output goes to terminal
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

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
        // background mode: redirect output to log file
        let log_file = File::create(&log_path)?;
        cmd.stdout(Stdio::from(log_file.try_clone()?));
        cmd.stderr(Stdio::from(log_file));
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

    // track tool_use events to correlate with tool_result
    let mut tool_map: HashMap<String, ToolInfo> = HashMap::new();

    for line in lines_to_show {
        if raw {
            println!("{}", line);
        } else {
            // parse and pretty-print the JSON event
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                print_event(&event, &mut tool_map);
            } else {
                println!("{}", line);
            }
        }
    }

    if follow {
        use std::io::{Seek, SeekFrom};
        use std::thread::sleep;
        use std::time::Duration;

        let mut file = File::open(&log_path)?;
        file.seek(SeekFrom::End(0))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // no new data - check if process still alive
                    let session = find_session(&sessions_dir, session_id)?;
                    if !session.is_process_alive() {
                        break;
                    }
                    sleep(Duration::from_millis(100));
                }
                Ok(_) => {
                    if raw {
                        print!("{}", line);
                    } else if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                        print_event(&event, &mut tool_map);
                    } else {
                        print!("{}", line);
                    }
                    std::io::stdout().flush().ok();
                }
                Err(e) => return Err(BrdError::Io(e)),
            }
        }
    }

    Ok(())
}

/// send a message to an agent session using claude's --resume.
///
/// the agent must not be running - use `brd agent kill` first if needed.
/// this starts a new claude process that continues the conversation.
pub fn cmd_agent_send(cli: &Cli, paths: &RepoPaths, session_id: &str, message: &str) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let session = find_session(&sessions_dir, session_id)?;

    // agent must NOT be running to send a message via --resume
    if session.is_process_alive() {
        return Err(BrdError::Other(format!(
            "agent {} is still running (pid {}). use `brd agent kill {}` first.",
            session.session_id, session.pid, session.session_id
        )));
    }

    // determine working directory (use worktree if set, otherwise repo root)
    let working_dir = session.worktree.as_ref().unwrap_or(&paths.worktree_root);

    // append output to the session log file
    let log_path = Session::log_path(&sessions_dir, &session.session_id);
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // spawn claude with --resume to continue the conversation
    let status = Command::new("claude")
        .args(["-p", "--verbose", message])
        .args(["--resume", &session.claude_session_id])
        .args(["--output-format", "stream-json"])
        .current_dir(working_dir)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BrdError::Other(
                    "claude CLI not found - install from https://claude.ai/download".to_string(),
                )
            } else {
                BrdError::Io(e)
            }
        })?;

    if cli.json {
        println!(
            r#"{{"ok": {}, "session_id": "{}"}}"#,
            status.success(),
            session.session_id
        );
    } else if !status.success() {
        eprintln!("claude exited with code {:?}", status.code().unwrap_or(-1));
    }

    Ok(())
}

/// attach to an agent session interactively using claude's --resume.
///
/// the agent must not be running - use `brd agent kill` first if needed.
/// this replaces the current process with an interactive claude session.
#[cfg(unix)]
pub fn cmd_agent_attach(_cli: &Cli, paths: &RepoPaths, session_id: &str) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let sessions_dir = paths.sessions_dir();
    let session = find_session(&sessions_dir, session_id)?;

    // agent must NOT be running to attach via --resume
    if session.is_process_alive() {
        return Err(BrdError::Other(format!(
            "agent {} is still running (pid {}). use `brd agent kill {}` first.",
            session.session_id, session.pid, session.session_id
        )));
    }

    // determine working directory (use worktree if set, otherwise repo root)
    let working_dir = session.worktree.as_ref().unwrap_or(&paths.worktree_root);

    // exec replaces the current process - user gets full interactive claude
    let err = Command::new("claude")
        .args(["--resume", &session.claude_session_id])
        .current_dir(working_dir)
        .exec();

    // exec() only returns on error
    Err(BrdError::Io(err))
}

#[cfg(not(unix))]
pub fn cmd_agent_attach(_cli: &Cli, _paths: &RepoPaths, _session_id: &str) -> Result<()> {
    Err(BrdError::Other(
        "agent attach is not supported on this platform".to_string(),
    ))
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

/// remove stale agent session files.
pub fn cmd_agent_clean(cli: &Cli, paths: &RepoPaths, all: bool, force: bool) -> Result<()> {
    let sessions_dir = paths.sessions_dir();
    let sessions = load_all_sessions(&sessions_dir)?;

    // find sessions to clean
    let to_clean: Vec<_> = sessions
        .iter()
        .filter(|s| {
            if all {
                true
            } else {
                // stale = not running or waiting
                matches!(
                    s.status,
                    SessionStatus::Completed
                        | SessionStatus::Failed
                        | SessionStatus::Killed
                        | SessionStatus::Zombie
                )
            }
        })
        .collect();

    if to_clean.is_empty() {
        if cli.json {
            println!(r#"{{"ok": true, "cleaned": 0, "message": "no sessions to clean"}}"#);
        } else {
            println!("no stale sessions to clean");
        }
        return Ok(());
    }

    // show what will be cleaned
    if !cli.json && !force {
        println!("found {} session(s) to clean:", to_clean.len());
        let now = time::OffsetDateTime::now_utc();
        for s in &to_clean {
            let age = format_duration(now - s.started_at);
            println!("  {} ({}) - {} old", s.session_id, s.status, age);
        }
        println!();

        // ask for confirmation
        eprint!("remove these? [y/N] ");
        use std::io::Write;
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("aborted");
            return Ok(());
        }
    }

    // remove session files
    let mut cleaned = 0;
    for s in &to_clean {
        let state_path = Session::state_path(&sessions_dir, &s.session_id);
        let log_path = Session::log_path(&sessions_dir, &s.session_id);
        let stdin_path = Session::stdin_path(&sessions_dir, &s.session_id);

        // remove each file if it exists
        for path in [&state_path, &log_path, &stdin_path] {
            if path.exists()
                && let Err(e) = std::fs::remove_file(path)
            {
                eprintln!("warning: failed to remove {}: {}", path.display(), e);
            }
        }
        cleaned += 1;

        if !cli.json {
            println!("removed {}", s.session_id);
        }
    }

    if cli.json {
        println!(r#"{{"ok": true, "cleaned": {}}}"#, cleaned);
    } else {
        println!("\ncleaned {} session(s)", cleaned);
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

/// info about a tool call, used to correlate results back to their calls.
#[derive(Debug, Clone)]
struct ToolInfo {
    name: String,
    #[allow(dead_code)] // reserved for future use (e.g., --verbose mode)
    summary: String,
}

/// extract a summary of what a tool is doing from its input.
fn tool_summary(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Bash" => input
            .get("command")
            .and_then(|c| c.as_str())
            .map(|cmd| truncate_str(cmd, 50))
            .unwrap_or_default(),
        "Read" => input
            .get("file_path")
            .and_then(|p| p.as_str())
            .map(short_path)
            .unwrap_or_default(),
        "Write" => input
            .get("file_path")
            .and_then(|p| p.as_str())
            .map(short_path)
            .unwrap_or_default(),
        "Edit" => input
            .get("file_path")
            .and_then(|p| p.as_str())
            .map(short_path)
            .unwrap_or_default(),
        "Glob" => input
            .get("pattern")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        "Grep" => input
            .get("pattern")
            .and_then(|p| p.as_str())
            .map(|s| format!("\"{}\"", truncate_str(s, 30)))
            .unwrap_or_default(),
        "Task" => {
            let subagent = input
                .get("subagent_type")
                .and_then(|s| s.as_str())
                .unwrap_or("?");
            let desc = input
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| truncate_str(s, 30))
                .unwrap_or_default();
            format!("{}: {}", subagent, desc)
        }
        "TodoWrite" => "updating todos".to_string(),
        _ => String::new(),
    }
}

/// truncate a string to max length, adding "..." if truncated.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// shorten a file path to just filename or last two components.
fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        path.to_string()
    } else {
        parts[parts.len() - 2..].join("/")
    }
}

/// pretty-print a claude stream-json event.
///
/// `tool_map` tracks tool_use events by ID so we can correlate results.
fn print_event(event: &serde_json::Value, tool_map: &mut HashMap<String, ToolInfo>) {
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
                    let item_type = item.get("type").and_then(|t| t.as_str());
                    match item_type {
                        Some("text") => {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                println!("{}", text);
                            }
                        }
                        Some("tool_use") => {
                            // tool_use embedded in assistant message content
                            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                let input = item.get("input").unwrap_or(&serde_json::Value::Null);
                                let summary = tool_summary(name, input);

                                // store for later lookup
                                if !id.is_empty() {
                                    tool_map.insert(
                                        id.to_string(),
                                        ToolInfo {
                                            name: name.to_string(),
                                            summary: summary.clone(),
                                        },
                                    );
                                }

                                // print tool call with context
                                if summary.is_empty() {
                                    println!("[{}]", name);
                                } else {
                                    println!("[{}] {}", name, summary);
                                }
                            }
                        }
                        _ => {}
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
        "tool_use" => {
            // track tool call for later correlation with result
            if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                let id = event.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let input = event.get("input").unwrap_or(&serde_json::Value::Null);
                let summary = tool_summary(name, input);

                // store for later lookup
                if !id.is_empty() {
                    tool_map.insert(
                        id.to_string(),
                        ToolInfo {
                            name: name.to_string(),
                            summary: summary.clone(),
                        },
                    );
                }

                // print tool call with context
                if summary.is_empty() {
                    println!("[{}]", name);
                } else {
                    println!("[{}] {}", name, summary);
                }
            }
        }
        "tool_result" => {
            // standalone tool_result events (rare, but handle them)
            if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                println!("[{}]", name);
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
        "result" => {
            // end of turn - add blank line for readability
            println!();
        }
        "user" => {
            // tool results returned to the model - show brief summary
            if let Some(message) = event.get("message")
                && let Some(content) = message.get("content").and_then(|c| c.as_array())
            {
                for item in content {
                    if item.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                        let tool_use_id = item
                            .get("tool_use_id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        let is_error = item
                            .get("is_error")
                            .and_then(|e| e.as_bool())
                            .unwrap_or(false);
                        let content_len = item
                            .get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.len())
                            .unwrap_or(0);

                        // look up tool name from earlier tool_use event
                        let tool_name = tool_map
                            .get(tool_use_id)
                            .map(|info| info.name.as_str())
                            .unwrap_or("?");

                        if is_error {
                            println!("  → {}: error ({} bytes)", tool_name, content_len);
                        } else {
                            println!("  → {}: ok ({} bytes)", tool_name, content_len);
                        }
                    }
                }
                // blank line after tool results for visual separation
                println!();
            }
        }
        "system" => {
            // system init event, skip (or could show session info)
        }
        _ => {
            // unknown event type, print type for debugging
            println!("[{}]", event_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestRepo, test_cli};
    use std::fs;
    use std::path::PathBuf;

    // =========================================================================
    // format_duration tests
    // =========================================================================

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(time::Duration::minutes(0)), "1m");
        assert_eq!(format_duration(time::Duration::minutes(1)), "1m");
        assert_eq!(format_duration(time::Duration::minutes(30)), "30m");
        assert_eq!(format_duration(time::Duration::minutes(59)), "59m");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(time::Duration::minutes(60)), "1h");
        assert_eq!(format_duration(time::Duration::minutes(90)), "1h");
        assert_eq!(format_duration(time::Duration::minutes(120)), "2h");
        assert_eq!(format_duration(time::Duration::minutes(60 * 23)), "23h");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(time::Duration::minutes(60 * 24)), "1d");
        assert_eq!(format_duration(time::Duration::minutes(60 * 48)), "2d");
        assert_eq!(format_duration(time::Duration::minutes(60 * 24 * 7)), "7d");
    }

    // =========================================================================
    // print_event tests (via format_event helper)
    // =========================================================================

    /// format an event to a string for testing (captures what print_event would output).
    /// this version takes a tool_map for stateful correlation.
    fn format_event_with_map(
        event: &serde_json::Value,
        tool_map: &mut HashMap<String, ToolInfo>,
    ) -> String {
        let event_type = event
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "assistant" => {
                let mut output = String::new();
                if let Some(message) = event.get("message")
                    && let Some(content) = message.get("content").and_then(|c| c.as_array())
                {
                    for item in content {
                        let item_type = item.get("type").and_then(|t| t.as_str());
                        match item_type {
                            Some("text") => {
                                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                    output.push_str(text);
                                    output.push('\n');
                                }
                            }
                            Some("tool_use") => {
                                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                    let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                    let input =
                                        item.get("input").unwrap_or(&serde_json::Value::Null);
                                    let summary = tool_summary(name, input);

                                    if !id.is_empty() {
                                        tool_map.insert(
                                            id.to_string(),
                                            ToolInfo {
                                                name: name.to_string(),
                                                summary: summary.clone(),
                                            },
                                        );
                                    }

                                    if summary.is_empty() {
                                        output.push_str(&format!("[{}]\n", name));
                                    } else {
                                        output.push_str(&format!("[{}] {}\n", name, summary));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                output
            }
            "content_block_delta" => {
                if let Some(delta) = event.get("delta")
                    && let Some(text) = delta.get("text").and_then(|t| t.as_str())
                {
                    text.to_string()
                } else {
                    String::new()
                }
            }
            "tool_use" => {
                if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                    let id = event.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let input = event.get("input").unwrap_or(&serde_json::Value::Null);
                    let summary = tool_summary(name, input);

                    if !id.is_empty() {
                        tool_map.insert(
                            id.to_string(),
                            ToolInfo {
                                name: name.to_string(),
                                summary: summary.clone(),
                            },
                        );
                    }

                    if summary.is_empty() {
                        format!("[{}]\n", name)
                    } else {
                        format!("[{}] {}\n", name, summary)
                    }
                } else {
                    String::new()
                }
            }
            "tool_result" => {
                if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                    format!("[{}]\n", name)
                } else {
                    String::new()
                }
            }
            "error" => {
                if let Some(msg) = event
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    format!("error: {}\n", msg)
                } else {
                    String::new()
                }
            }
            "message_stop" | "message_start" | "system" => String::new(),
            "user" => {
                let mut output = String::new();
                if let Some(message) = event.get("message")
                    && let Some(content) = message.get("content").and_then(|c| c.as_array())
                {
                    for item in content {
                        if item.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                            let tool_use_id = item
                                .get("tool_use_id")
                                .and_then(|i| i.as_str())
                                .unwrap_or("");
                            let is_error = item
                                .get("is_error")
                                .and_then(|e| e.as_bool())
                                .unwrap_or(false);
                            let content_len = item
                                .get("content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.len())
                                .unwrap_or(0);

                            let tool_name = tool_map
                                .get(tool_use_id)
                                .map(|info| info.name.as_str())
                                .unwrap_or("?");

                            if is_error {
                                output.push_str(&format!(
                                    "  → {}: error ({} bytes)\n",
                                    tool_name, content_len
                                ));
                            } else {
                                output.push_str(&format!(
                                    "  → {}: ok ({} bytes)\n",
                                    tool_name, content_len
                                ));
                            }
                        }
                    }
                    // blank line after tool results
                    output.push('\n');
                }
                output
            }
            _ => format!("[{}]\n", event_type),
        }
    }

    /// simple helper for tests that don't need tool correlation.
    fn format_event(event: &serde_json::Value) -> String {
        let mut tool_map = HashMap::new();
        format_event_with_map(event, &mut tool_map)
    }

    #[test]
    fn test_format_event_assistant() {
        let event = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "hello world"}
                ]
            }
        });
        assert_eq!(format_event(&event), "hello world\n");
    }

    #[test]
    fn test_format_event_assistant_multiple_texts() {
        let event = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "line one"},
                    {"type": "text", "text": "line two"}
                ]
            }
        });
        assert_eq!(format_event(&event), "line one\nline two\n");
    }

    #[test]
    fn test_format_event_content_block_delta() {
        let event = serde_json::json!({
            "type": "content_block_delta",
            "delta": {"text": "streaming text"}
        });
        assert_eq!(format_event(&event), "streaming text");
    }

    #[test]
    fn test_format_event_tool_use() {
        let event = serde_json::json!({
            "type": "tool_use",
            "name": "Bash"
        });
        assert_eq!(format_event(&event), "[Bash]\n");
    }

    #[test]
    fn test_format_event_tool_use_with_summary() {
        let event = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_123",
            "name": "Bash",
            "input": {"command": "git status"}
        });
        assert_eq!(format_event(&event), "[Bash] git status\n");
    }

    #[test]
    fn test_format_event_tool_use_read() {
        let event = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_456",
            "name": "Read",
            "input": {"file_path": "/Users/test/code/project/src/main.rs"}
        });
        assert_eq!(format_event(&event), "[Read] src/main.rs\n");
    }

    #[test]
    fn test_format_event_tool_use_grep() {
        let event = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_789",
            "name": "Grep",
            "input": {"pattern": "fn main"}
        });
        assert_eq!(format_event(&event), "[Grep] \"fn main\"\n");
    }

    #[test]
    fn test_format_event_error() {
        let event = serde_json::json!({
            "type": "error",
            "error": {"message": "something went wrong"}
        });
        assert_eq!(format_event(&event), "error: something went wrong\n");
    }

    #[test]
    fn test_format_event_message_stop_silent() {
        let event = serde_json::json!({"type": "message_stop"});
        assert_eq!(format_event(&event), "");
    }

    #[test]
    fn test_format_event_unknown_type() {
        let event = serde_json::json!({"type": "custom_event"});
        assert_eq!(format_event(&event), "[custom_event]\n");
    }

    #[test]
    fn test_format_event_user_tool_result_unknown_tool() {
        // when there's no prior tool_use event, tool name is "?"
        let event = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "tool_use_id": "toolu_unknown",
                        "type": "tool_result",
                        "content": "command output here",
                        "is_error": false
                    }
                ]
            }
        });
        assert_eq!(format_event(&event), "  → ?: ok (19 bytes)\n\n");
    }

    #[test]
    fn test_format_event_user_tool_result_with_correlation() {
        // test tool_use -> tool_result correlation
        let mut tool_map = HashMap::new();
        let tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_123",
            "name": "Bash",
            "input": {"command": "ls"}
        });
        format_event_with_map(&tool_use, &mut tool_map);

        let tool_result = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "tool_use_id": "toolu_123",
                        "type": "tool_result",
                        "content": "file1.txt\nfile2.txt",
                        "is_error": false
                    }
                ]
            }
        });
        assert_eq!(
            format_event_with_map(&tool_result, &mut tool_map),
            "  → Bash: ok (19 bytes)\n\n"
        );
    }

    #[test]
    fn test_format_event_user_tool_result_error() {
        let event = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "tool_use_id": "toolu_123",
                        "type": "tool_result",
                        "content": "error message",
                        "is_error": true
                    }
                ]
            }
        });
        assert_eq!(format_event(&event), "  → ?: error (13 bytes)\n\n");
    }

    #[test]
    fn test_format_event_system_silent() {
        let event = serde_json::json!({"type": "system", "subtype": "init"});
        assert_eq!(format_event(&event), "");
    }

    // =========================================================================
    // cmd_agent_ps tests
    // =========================================================================

    #[test]
    fn test_agent_ps_empty_sessions() {
        let repo = TestRepo::default();
        let cli = test_cli();

        // ensure sessions dir exists but is empty
        let sessions_dir = repo.paths.sessions_dir();
        fs::create_dir_all(&sessions_dir).unwrap();

        let result = cmd_agent_ps(&cli, &repo.paths, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_ps_with_session() {
        let repo = TestRepo::default();
        let cli = test_cli();

        // create a session file
        let sessions_dir = repo.paths.ensure_sessions_dir().unwrap();
        let session = Session::new(
            "agent-1".to_string(),
            "uuid-123".to_string(),
            99999, // fake pid that won't exist
            "brd-test".to_string(),
            Some(PathBuf::from("/tmp")),
            1.0,
            "claude-test".to_string(),
        );
        session
            .save(&Session::state_path(&sessions_dir, "agent-1"))
            .unwrap();

        let result = cmd_agent_ps(&cli, &repo.paths, true);
        assert!(result.is_ok());
    }

    // =========================================================================
    // cmd_agent_logs tests
    // =========================================================================

    #[test]
    fn test_agent_logs_session_not_found() {
        let repo = TestRepo::default();
        let cli = test_cli();

        let result = cmd_agent_logs(&cli, &repo.paths, "nonexistent", false, None, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("session not found")
        );
    }

    #[test]
    fn test_agent_logs_reads_log_file() {
        let repo = TestRepo::default();
        let cli = test_cli();

        // create session and log file
        let sessions_dir = repo.paths.ensure_sessions_dir().unwrap();
        let session = Session::new(
            "agent-1".to_string(),
            "uuid-123".to_string(),
            99999,
            "brd-test".to_string(),
            None,
            1.0,
            "claude-test".to_string(),
        );
        session
            .save(&Session::state_path(&sessions_dir, "agent-1"))
            .unwrap();

        // create log file with some content
        let log_path = Session::log_path(&sessions_dir, "agent-1");
        fs::write(&log_path, r#"{"type": "message_start"}"#).unwrap();

        let result = cmd_agent_logs(&cli, &repo.paths, "agent-1", false, None, true);
        assert!(result.is_ok());
    }

    // =========================================================================
    // cmd_agent_kill tests
    // =========================================================================

    #[test]
    fn test_agent_kill_session_not_found() {
        let repo = TestRepo::default();
        let cli = test_cli();

        let result = cmd_agent_kill(&cli, &repo.paths, "nonexistent", false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("session not found")
        );
    }

    #[test]
    fn test_agent_kill_already_dead() {
        let repo = TestRepo::default();
        let cli = test_cli();

        // create session with fake pid
        let sessions_dir = repo.paths.ensure_sessions_dir().unwrap();
        let session = Session::new(
            "agent-1".to_string(),
            "uuid-123".to_string(),
            99999, // fake pid
            "brd-test".to_string(),
            None,
            1.0,
            "claude-test".to_string(),
        );
        session
            .save(&Session::state_path(&sessions_dir, "agent-1"))
            .unwrap();

        // should succeed (reports already dead)
        let result = cmd_agent_kill(&cli, &repo.paths, "agent-1", false);
        assert!(result.is_ok());
    }

    // =========================================================================
    // cmd_agent_spawn tests
    // =========================================================================

    #[test]
    fn test_agent_spawn_issue_not_found() {
        let repo = TestRepo::default();
        let cli = test_cli();

        let result = cmd_agent_spawn(&cli, &repo.paths, "nonexistent", 1.0, false, false, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("issue not found"));
    }
}
