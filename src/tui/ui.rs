//! TUI rendering functions.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use time::{Duration as TimeDuration, OffsetDateTime};

use crate::graph::{compute_derived, get_dependents};
use crate::issue::{Priority, Status};
use crate::session::SessionStatus;

use super::app::{App, InputMode, IssuesFocus, View};
use super::diff_panel::{DiffPanel, centered_overlay};

/// draw the entire UI.
pub fn draw(f: &mut Frame, app: &mut App) {
    if app.show_help {
        draw_help(f, f.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // main content
            Constraint::Length(1), // footer
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_footer(f, chunks[2], app);

    // draw input dialog on top if active
    if !matches!(app.input_mode, InputMode::Normal | InputMode::Filter(_)) {
        draw_input_dialog(f, app);
    }

    // draw diff panel overlay if visible
    if app.is_diff_visible() {
        draw_diff_panel(f, app);
    }

    // draw logs overlay if visible
    if app.show_logs_overlay {
        draw_logs_overlay(f, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let text = format!("brd tui — agent: {}", app.agent_id);
    let header = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let msg = app.message.as_deref().unwrap_or("");
    let help = "[1]dashboard [2]issues [3]agents [Tab]focus [\\]toggle details [a]dd [e]dit [s]tart [d]one [/]filter [?]help [q]uit";
    let text = if msg.is_empty() {
        help.to_string()
    } else {
        format!("{} │ {}", msg, help)
    };
    let footer = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    match app.view {
        View::Dashboard => draw_dashboard(f, area, app),
        View::Issues => draw_issues_view(f, area, app),
        View::Agents => draw_agents_view(f, area, app),
    }
}

fn draw_dashboard(f: &mut Frame, area: Rect, app: &App) {
    // compute all stats
    let now = OffsetDateTime::now_utc();
    let day_ago = now - TimeDuration::hours(24);

    // status counts
    let open_count = app
        .issues
        .values()
        .filter(|i| i.status() == Status::Open)
        .count();
    let doing_count = app
        .issues
        .values()
        .filter(|i| i.status() == Status::Doing)
        .count();
    let done_count = app
        .issues
        .values()
        .filter(|i| i.status() == Status::Done)
        .count();
    let skip_count = app
        .issues
        .values()
        .filter(|i| i.status() == Status::Skip)
        .count();
    let total_count = open_count + doing_count + done_count + skip_count;

    // priority counts (open + doing only)
    let active_issues: Vec<_> = app
        .issues
        .values()
        .filter(|i| matches!(i.status(), Status::Open | Status::Doing))
        .collect();
    let p0_count = active_issues
        .iter()
        .filter(|i| i.priority() == Priority::P0)
        .count();
    let p1_count = active_issues
        .iter()
        .filter(|i| i.priority() == Priority::P1)
        .count();
    let p2_count = active_issues
        .iter()
        .filter(|i| i.priority() == Priority::P2)
        .count();
    let p3_count = active_issues
        .iter()
        .filter(|i| i.priority() == Priority::P3)
        .count();
    let active_total = active_issues.len();

    // health: ready/blocked/stale
    let mut ready_count = 0;
    let mut blocked_count = 0;
    for issue in app.issues.values().filter(|i| i.status() == Status::Open) {
        let derived = compute_derived(issue, &app.issues);
        if derived.is_ready {
            ready_count += 1;
        } else if derived.is_blocked {
            blocked_count += 1;
        }
    }
    let stale_count = app
        .issues
        .values()
        .filter(|i| {
            i.status() == Status::Doing && i.frontmatter.started_at.is_some_and(|t| t < day_ago)
        })
        .count();

    // active agents
    let mut active_agents: Vec<_> = app
        .issues
        .values()
        .filter(|i| i.status() == Status::Doing && i.frontmatter.owner.is_some())
        .map(|i| {
            (
                i.frontmatter.owner.as_deref().unwrap_or("?"),
                i.id(),
                i.title(),
            )
        })
        .collect();
    active_agents.sort_by(|a, b| a.0.cmp(b.0));

    // velocity: 7-day completion and creation data
    let week_ago = now - TimeDuration::days(7);
    let two_weeks_ago = now - TimeDuration::days(14);

    // count completions per day for last 7 days
    let mut completed_by_day = [0usize; 7];
    let mut created_by_day = [0usize; 7];
    let mut completed_total = 0usize;
    let mut created_total = 0usize;
    let mut completed_prev_week = 0usize;

    for issue in app.issues.values() {
        // completions
        if let Some(completed_at) = issue.frontmatter.completed_at {
            if completed_at > week_ago {
                let days_ago = (now - completed_at).whole_days().clamp(0, 6) as usize;
                completed_by_day[6 - days_ago] += 1;
                completed_total += 1;
            } else if completed_at > two_weeks_ago {
                completed_prev_week += 1;
            }
        }
        // creations
        let created_at = issue.frontmatter.created_at;
        if created_at > week_ago {
            let days_ago = (now - created_at).whole_days().clamp(0, 6) as usize;
            created_by_day[6 - days_ago] += 1;
            created_total += 1;
        }
    }

    let completed_delta = completed_total as i32 - completed_prev_week as i32;

    // flow metrics: lead time and cycle time for completed issues
    let mut lead_times: Vec<f64> = Vec::new();
    let mut cycle_times: Vec<f64> = Vec::new();

    for issue in app.issues.values() {
        if issue.status() != Status::Done {
            continue;
        }
        let Some(completed_at) = issue.frontmatter.completed_at else {
            continue;
        };

        // lead time: completed - created
        let lead_time = (completed_at - issue.frontmatter.created_at).as_seconds_f64();
        if lead_time > 0.0 {
            lead_times.push(lead_time);
        }

        // cycle time: completed - started (if started_at exists)
        if let Some(started_at) = issue.frontmatter.started_at {
            let cycle_time = (completed_at - started_at).as_seconds_f64();
            if cycle_time > 0.0 {
                cycle_times.push(cycle_time);
            }
        }
    }

    let lead_avg = if lead_times.is_empty() {
        None
    } else {
        Some(lead_times.iter().sum::<f64>() / lead_times.len() as f64)
    };
    let lead_median = median(&lead_times);

    let cycle_avg = if cycle_times.is_empty() {
        None
    } else {
        Some(cycle_times.iter().sum::<f64>() / cycle_times.len() as f64)
    };
    let cycle_median = median(&cycle_times);

    let completed_count = lead_times.len();

    // layout: top stats row, then velocity/flow row, then git graph, then agents
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(8), // stats row (with borders)
            Constraint::Length(1), // spacing
            Constraint::Length(6), // velocity + flow metrics row
            Constraint::Length(1), // spacing
            Constraint::Length(5), // git graph
            Constraint::Length(1), // spacing
            Constraint::Min(4),    // agents (compact)
        ])
        .split(area);

    // velocity/flow row: velocity (left) and flow metrics (right)
    let metrics_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // top row: 3 columns for status/priority/health
    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[0]);

    // status box with bar
    let status_block = Block::default()
        .title(" Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let status_inner = status_block.inner(top_cols[0]);
    f.render_widget(status_block, top_cols[0]);

    let bar_width = status_inner.width.saturating_sub(2) as usize;
    let status_bar = make_stacked_bar(
        bar_width,
        total_count,
        &[
            (done_count, Color::Green),
            (doing_count, Color::Yellow),
            (open_count, Color::White),
            (skip_count, Color::DarkGray),
        ],
    );
    let status_lines = vec![
        status_bar,
        Line::from(""),
        Line::from(vec![
            Span::styled("open ", Style::default()),
            Span::styled(format!("{:>3}", open_count), Style::default()),
            Span::styled("  doing ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:>3}", doing_count),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("done ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("{:>3}", done_count),
                Style::default().fg(Color::Green),
            ),
            Span::styled("  skip  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>3}", skip_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(status_lines), status_inner);

    // priority box with bar
    let priority_block = Block::default()
        .title(" Priority ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let priority_inner = priority_block.inner(top_cols[1]);
    f.render_widget(priority_block, top_cols[1]);

    let priority_bar = make_stacked_bar(
        bar_width,
        active_total,
        &[
            (p0_count, Color::Red),
            (p1_count, Color::Yellow),
            (p2_count, Color::White),
            (p3_count, Color::DarkGray),
        ],
    );
    let priority_lines = vec![
        priority_bar,
        Line::from(""),
        Line::from(vec![
            Span::styled("P0 ", Style::default().fg(Color::Red)),
            Span::styled(format!("{:>3}", p0_count), Style::default().fg(Color::Red)),
            Span::styled("  P1 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:>3}", p1_count),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("P2 ", Style::default()),
            Span::styled(format!("{:>3}", p2_count), Style::default()),
            Span::styled("  P3 ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>3}", p3_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(priority_lines), priority_inner);

    // health box
    let health_block = Block::default()
        .title(" Health ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let health_inner = health_block.inner(top_cols[2]);
    f.render_widget(health_block, top_cols[2]);

    let health_bar = make_stacked_bar(
        bar_width,
        ready_count + blocked_count,
        &[(ready_count, Color::Green), (blocked_count, Color::Yellow)],
    );
    let mut health_lines = vec![
        health_bar,
        Line::from(""),
        Line::from(vec![
            Span::styled("ready   ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("{:>3}", ready_count),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("blocked ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:>3}", blocked_count),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];
    if stale_count > 0 {
        health_lines[3] = Line::from(vec![
            Span::styled("blocked ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:>3}", blocked_count),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("  stale ", Style::default().fg(Color::Red)),
            Span::styled(format!("{}", stale_count), Style::default().fg(Color::Red)),
        ]);
    }
    f.render_widget(Paragraph::new(health_lines), health_inner);

    // git graph box
    draw_git_graph(f, chunks[4], app);

    // agents box
    let agents_block = Block::default()
        .title(" Active Agents ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let agents_inner = agents_block.inner(chunks[6]);
    f.render_widget(agents_block, chunks[6]);

    let agent_lines: Vec<Line> = if active_agents.is_empty() {
        vec![Line::from(Span::styled(
            "(none)",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        let max_show = agents_inner.height as usize;
        let mut lines: Vec<Line> = active_agents
            .iter()
            .take(max_show)
            .map(|(owner, id, title)| {
                let max_title = agents_inner.width.saturating_sub(25) as usize;
                let truncated_title: String = title.chars().take(max_title).collect();
                Line::from(vec![
                    Span::styled(format!("{:<12}", owner), Style::default().fg(Color::Cyan)),
                    Span::styled(" → ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{} ", id)),
                    Span::styled(truncated_title, Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect();
        if active_agents.len() > max_show {
            lines.push(Line::from(Span::styled(
                format!("  (+{} more)", active_agents.len() - max_show),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines
    };
    f.render_widget(Paragraph::new(agent_lines), agents_inner);

    // velocity box (7-day sparklines)
    let velocity_block = Block::default()
        .title(" Velocity (7d) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let velocity_inner = velocity_block.inner(metrics_cols[0]);
    f.render_widget(velocity_block, metrics_cols[0]);

    let completed_spark = make_sparkline(&completed_by_day);
    let created_spark = make_sparkline(&created_by_day);

    let delta_str = if completed_delta > 0 {
        format!("(+{})", completed_delta)
    } else if completed_delta < 0 {
        format!("({})", completed_delta)
    } else {
        String::new()
    };

    let velocity_lines = vec![
        Line::from(vec![
            Span::styled("completed ", Style::default().fg(Color::Green)),
            Span::raw(completed_spark),
            Span::raw(format!("  {} total ", completed_total)),
            Span::styled(delta_str, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("created   ", Style::default().fg(Color::Cyan)),
            Span::raw(created_spark),
            Span::raw(format!("  {} total", created_total)),
        ]),
    ];
    f.render_widget(Paragraph::new(velocity_lines), velocity_inner);

    // flow metrics box (lead time, cycle time)
    let flow_block = Block::default()
        .title(" Flow Metrics ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let flow_inner = flow_block.inner(metrics_cols[1]);
    f.render_widget(flow_block, metrics_cols[1]);

    let flow_lines = if completed_count == 0 {
        vec![Line::from(Span::styled(
            "(no completed issues)",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        let lead_avg_str = lead_avg.map(format_duration_short).unwrap_or_default();
        let lead_med_str = lead_median.map(format_duration_short).unwrap_or_default();
        let cycle_avg_str = cycle_avg.map(format_duration_short).unwrap_or_default();
        let cycle_med_str = cycle_median.map(format_duration_short).unwrap_or_default();

        vec![
            Line::from(vec![
                Span::styled("lead time  ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("avg {}  med {}", lead_avg_str, lead_med_str)),
            ]),
            Line::from(vec![
                Span::styled("cycle time ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("avg {}  med {}", cycle_avg_str, cycle_med_str)),
            ]),
            Line::from(Span::styled(
                format!("({} completed issues)", completed_count),
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };
    f.render_widget(Paragraph::new(flow_lines), flow_inner);
}

/// Create a horizontal stacked bar from segments
fn make_stacked_bar(width: usize, total: usize, segments: &[(usize, Color)]) -> Line<'static> {
    if total == 0 || width == 0 {
        return Line::from(Span::styled(
            "░".repeat(width),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // use largest remainder method to distribute width fairly
    // this ensures the bar always uses its full width
    let mut widths: Vec<usize> = segments.iter().map(|_| 0).collect();
    let mut remainders: Vec<(usize, f64)> = Vec::new();

    // calculate base widths and remainders
    for (i, (count, _)) in segments.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let exact = (*count as f64 * width as f64) / total as f64;
        let base = exact.floor() as usize;
        widths[i] = base;
        remainders.push((i, exact - base as f64));
    }

    // distribute remaining width to segments with largest remainders
    let used: usize = widths.iter().sum();
    let mut remaining = width.saturating_sub(used);
    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (i, _) in remainders {
        if remaining == 0 {
            break;
        }
        widths[i] += 1;
        remaining -= 1;
    }

    // build spans
    let mut spans = Vec::new();
    for (i, (_, color)) in segments.iter().enumerate() {
        if widths[i] > 0 {
            spans.push(Span::styled(
                "█".repeat(widths[i]),
                Style::default().fg(*color),
            ));
        }
    }

    Line::from(spans)
}

fn draw_agents_view(f: &mut Frame, area: Rect, app: &App) {
    // split into worktree list (left) and file changes (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_worktree_list(f, chunks[0], app);
    draw_worktree_files(f, chunks[1], app);
}

fn draw_worktree_list(f: &mut Frame, area: Rect, app: &App) {
    use crate::tui::app::AgentsFocus;

    let is_focused = app.agents_focus == AgentsFocus::Worktrees;
    let border_color = if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Agents ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.worktrees.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No agent worktrees found",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Use 'brd agent init <name>'",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let now = time::OffsetDateTime::now_utc();

    // build list items
    let items: Vec<ListItem> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let mut spans = Vec::new();
            let is_selected = i == app.worktree_selected;

            // selection indicator (only show arrow when focused)
            if is_selected && is_focused {
                spans.push(Span::styled("▶ ", Style::default().fg(Color::Yellow)));
            } else if is_selected {
                spans.push(Span::styled("› ", Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::raw("  "));
            }

            // name
            spans.push(Span::styled(
                &wt.name,
                if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ));

            // branch
            if let Some(ref branch) = wt.branch {
                spans.push(Span::styled(
                    format!("  ({})", branch),
                    Style::default().fg(Color::Cyan),
                ));
            }

            // dirty indicator
            if wt.is_dirty {
                spans.push(Span::styled(" *", Style::default().fg(Color::Yellow)));
            }

            // session status - find session for this worktree
            if let Some(session) = app.session_for_worktree(i) {
                // status indicator
                let (status_char, status_color) = match session.status {
                    SessionStatus::Running => ("●", Color::Green),
                    SessionStatus::Waiting => ("◐", Color::Yellow),
                    SessionStatus::Completed => ("✓", Color::Green),
                    SessionStatus::Failed => ("✗", Color::Red),
                    SessionStatus::Killed => ("○", Color::DarkGray),
                    SessionStatus::Zombie => ("⚠", Color::Red),
                };
                spans.push(Span::raw(" "));
                spans.push(Span::styled(status_char, Style::default().fg(status_color)));

                // session ID (truncated)
                let short_id = &session.session_id;
                spans.push(Span::styled(
                    format!(" {}", short_id),
                    Style::default().fg(Color::DarkGray),
                ));

                // runtime for active sessions
                if matches!(
                    session.status,
                    SessionStatus::Running | SessionStatus::Waiting
                ) {
                    let runtime = now - session.started_at;
                    let runtime_str = format_runtime(runtime);
                    spans.push(Span::styled(
                        format!(" {}", runtime_str),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// format a runtime duration for display.
fn format_runtime(d: time::Duration) -> String {
    let minutes = d.whole_minutes();
    if minutes < 60 {
        format!("{}m", minutes.max(1))
    } else if minutes < 60 * 24 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}d", minutes / (60 * 24))
    }
}

fn draw_worktree_files(f: &mut Frame, area: Rect, app: &App) {
    use crate::tui::app::AgentsFocus;

    let is_focused = app.agents_focus == AgentsFocus::Files;
    let border_color = if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let Some(ref diff) = app.worktree_diff else {
        let block = Block::default()
            .title(" Changes ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No changes",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
        return;
    };

    // build title with stats
    let title = format!(
        " Changes ({}) +{} -{} [{}] ",
        diff.stat.files_changed, diff.stat.insertions, diff.stat.deletions, diff.diff_base
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if diff.files.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No file changes",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // build file list
    let items: Vec<ListItem> = diff
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let mut spans = Vec::new();
            let is_selected = i == app.worktree_file_selected;

            // selection indicator (only show arrow when focused)
            if is_selected && is_focused {
                spans.push(Span::styled("▶ ", Style::default().fg(Color::Yellow)));
            } else if is_selected {
                spans.push(Span::styled("› ", Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::raw("  "));
            }

            // status indicator
            let status_char = match file.status {
                crate::git::FileStatus::Added => "A",
                crate::git::FileStatus::Modified => "M",
                crate::git::FileStatus::Deleted => "D",
                crate::git::FileStatus::Renamed => "R",
                crate::git::FileStatus::Copied => "C",
                crate::git::FileStatus::Unknown => "?",
            };
            let status_color = match file.status {
                crate::git::FileStatus::Added => Color::Green,
                crate::git::FileStatus::Deleted => Color::Red,
                _ => Color::Yellow,
            };
            spans.push(Span::styled(
                format!("{} ", status_char),
                Style::default().fg(status_color),
            ));

            // file path
            spans.push(Span::styled(
                &file.path,
                if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ));

            // +/- counts
            if file.insertions > 0 || file.deletions > 0 {
                spans.push(Span::raw(" "));
                if file.insertions > 0 {
                    spans.push(Span::styled(
                        format!("+{}", file.insertions),
                        Style::default().fg(Color::Green),
                    ));
                }
                if file.deletions > 0 {
                    if file.insertions > 0 {
                        spans.push(Span::raw("/"));
                    }
                    spans.push(Span::styled(
                        format!("-{}", file.deletions),
                        Style::default().fg(Color::Red),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// Calculate median of a slice of f64 values.
fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

/// Format a duration in seconds to a short human-readable string.
/// Examples: "< 1h", "2.3h", "1.5d", "2.1w"
fn format_duration_short(seconds: f64) -> String {
    let hours = seconds / 3600.0;
    if hours < 1.0 {
        "< 1h".to_string()
    } else if hours < 24.0 {
        format!("{:.1}h", hours)
    } else {
        let days = hours / 24.0;
        if days < 7.0 {
            format!("{:.1}d", days)
        } else {
            let weeks = days / 7.0;
            format!("{:.1}w", weeks)
        }
    }
}

/// Create an ASCII sparkline from a slice of values.
/// Uses block characters: ▁▂▃▄▅▆▇█ (8 levels)
fn make_sparkline(values: &[usize]) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let max_val = values.iter().copied().max().unwrap_or(0);
    if max_val == 0 {
        return "▁".repeat(values.len());
    }

    values
        .iter()
        .map(|&v| {
            let idx = if v == 0 {
                0
            } else {
                // scale to 0-7 range, ensuring max value maps to index 7
                ((v * 7) / max_val).min(7)
            };
            BLOCKS[idx]
        })
        .collect()
}

fn draw_git_graph(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Git Graph ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(ref graph) = app.git_graph else {
        let text = vec![Line::from(Span::styled(
            "(no git data)",
            Style::default().fg(Color::DarkGray),
        ))];
        f.render_widget(Paragraph::new(text), inner);
        return;
    };

    if graph.main_track.is_empty() {
        let text = vec![Line::from(Span::styled(
            "(no commits)",
            Style::default().fg(Color::DarkGray),
        ))];
        f.render_widget(Paragraph::new(text), inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let max_width = inner.width as usize;
    let chars_per_commit = 3usize; // "●" + "──" between commits

    // calculate how many commits we can show
    // reserve space for labels at end
    let labels_str = graph.labels_at_head.join(", ");
    let suffix = format!("  {} ({}) ← HEAD", labels_str, graph.main_total);
    let available_for_dots = max_width.saturating_sub(suffix.len());
    let num_dots = (available_for_dots / chars_per_commit)
        .min(graph.main_track.len())
        .max(1);

    // row 0: main track with labels
    // ●──●──●──●──●  main, agent-one (510) ← HEAD
    let mut main_spans: Vec<Span> = Vec::new();
    let mut dots_str = String::new();
    for i in 0..num_dots {
        if i > 0 {
            dots_str.push_str("──");
        }
        dots_str.push('●');
    }
    main_spans.push(Span::styled(dots_str, Style::default().fg(Color::White)));
    main_spans.push(Span::styled(
        format!("  {}", labels_str),
        Style::default().fg(Color::Cyan),
    ));
    main_spans.push(Span::styled(
        format!(" ({}) ← HEAD", graph.main_total),
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::from(main_spans));

    // render behind branch labels below main track at their positions
    // show arrow pointing up to the commit and (-N) indicator
    if !graph.labels_behind.is_empty() {
        // first line: arrows pointing up to the commits
        let mut arrow_str = String::new();
        for label in &graph.labels_behind {
            let offset = label.position * chars_per_commit;
            while arrow_str.len() < offset {
                arrow_str.push(' ');
            }
            arrow_str.push('↑');
        }
        lines.push(Line::from(Span::styled(
            arrow_str,
            Style::default().fg(Color::Yellow),
        )));

        // second line: branch names with behind count
        let mut label_str = String::new();
        for label in &graph.labels_behind {
            let offset = label.position * chars_per_commit;
            // try to center the label under the arrow
            let label_text = format!("{} (-{})", label.name, label.behind);
            let label_start = offset.saturating_sub(label_text.len() / 2);
            while label_str.len() < label_start {
                label_str.push(' ');
            }
            label_str.push_str(&label_text);
            label_str.push(' ');
        }
        lines.push(Line::from(Span::styled(
            label_str,
            Style::default().fg(Color::Yellow),
        )));
    }

    // render diverged branches with connector lines and branch tracks
    // branch_tracks is sorted by fork_position (older forks first)
    for (track_idx, track) in graph.branch_tracks.iter().enumerate() {
        // connector row: show │ for this and all later branch fork points
        let mut connector_spans: Vec<Span> = Vec::new();
        let mut connector_str = String::new();

        // build connector line character by character
        for pos in 0..num_dots {
            let char_offset = pos * chars_per_commit;

            // check if any branch (this one or later) forks at this position
            let has_vertical = graph.branch_tracks[track_idx..]
                .iter()
                .any(|t| t.fork_position == pos);

            if has_vertical {
                // pad to position, then add vertical bar
                while connector_str.len() < char_offset {
                    connector_str.push(' ');
                }
                connector_str.push('│');
            }
        }

        if !connector_str.trim().is_empty() {
            connector_spans.push(Span::styled(
                connector_str,
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::from(connector_spans));
        }

        // branch row: └──●──● branch-name (+N)
        let mut branch_spans: Vec<Span> = Vec::new();

        // padding to fork position
        let fork_offset = track.fork_position * chars_per_commit;
        let padding = " ".repeat(fork_offset);
        branch_spans.push(Span::styled(padding, Style::default()));

        // fork connector and commits
        let mut branch_dots = String::from("└");
        for i in 0..track.commits.len().min(3) {
            branch_dots.push_str("──");
            if i < track.commits.len().min(3) {
                branch_dots.push('●');
            }
        }
        branch_spans.push(Span::styled(
            branch_dots,
            Style::default().fg(Color::DarkGray),
        ));

        // branch name
        branch_spans.push(Span::styled(
            format!(" {}", track.name),
            Style::default().fg(Color::Cyan),
        ));

        // commit count
        branch_spans.push(Span::styled(
            format!(" (+{})", track.commits.len()),
            Style::default().fg(Color::Yellow),
        ));

        lines.push(Line::from(branch_spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_issues_view(f: &mut Frame, area: Rect, app: &mut App) {
    if app.show_details {
        // two-pane layout: list + details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        draw_issue_list(f, chunks[0], app);
        draw_detail(f, chunks[1], app);
    } else {
        // full-width list only
        draw_issue_list(f, area, app);
    }

    // draw detail overlay on top if active
    if app.show_detail_overlay {
        draw_detail_overlay(f, area, app);
    }
}

fn draw_issue_list(f: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.issues_focus == IssuesFocus::List;
    let border_color = if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let border_style = Style::default().fg(border_color);

    // build title with filter info
    let visible = app.visible_issues();
    let filter_input = match &app.input_mode {
        InputMode::Filter(query) => Some(query.clone()),
        _ => None,
    };
    let show_filter = app.has_filter() || filter_input.is_some();
    let title = if show_filter {
        let mut filter_parts = Vec::new();
        if let Some(query) = &filter_input {
            filter_parts.push(format!("/{query}_"));
        } else if !app.filter_query.is_empty() {
            filter_parts.push(format!("\"{}\"", app.filter_query));
        }
        if !app.status_filter.is_empty() {
            let statuses: Vec<&str> = app
                .status_filter
                .iter()
                .map(|s| match s {
                    crate::issue::Status::Open => "T",
                    crate::issue::Status::Doing => "D",
                    crate::issue::Status::Done => "✓",
                    crate::issue::Status::Skip => "⊘",
                })
                .collect();
            filter_parts.push(statuses.join(""));
        }
        if app.ready_filter {
            filter_parts.push("READY".to_string());
        }
        format!(
            " Issues ({}/{}) [{}] ",
            visible.len(),
            app.sorted_issues.len(),
            filter_parts.join(" ")
        )
    } else {
        format!(" Issues ({}) ", app.sorted_issues.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // calculate available width for title
    // area - borders(2) - status_prefix(2) - id(8) - priority(2) - age(4) - owner(10) - spaces(4)
    let title_width = area.width.saturating_sub(32) as usize;
    let view_height = block.inner(area).height as usize;
    let now = OffsetDateTime::now_utc();

    // compute issues that are blocking the currently selected issue
    let blockers = app.get_blocking_selected();

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let issue = app.issues.get(id).unwrap();
            let derived = compute_derived(issue, &app.issues);
            let is_blocker = blockers.contains(id);
            // show ! for blockers, → for doing, otherwise space
            let status_prefix = if issue.status() == Status::Doing {
                "→ "
            } else if is_blocker {
                "! "
            } else {
                "  "
            };
            let age = format_age(issue.frontmatter.created_at);
            let owner = issue
                .frontmatter
                .owner
                .as_deref()
                .map(|o| truncate(o, 10))
                .unwrap_or_else(|| "-".to_string());
            let tags_width = issue_tags_width(&issue.frontmatter.tags);
            let title_part = if tags_width == 0 {
                truncate(issue.title(), title_width)
            } else {
                truncate(issue.title(), title_width.saturating_sub(tags_width + 1))
            };

            let duration = now - issue.frontmatter.created_at;
            let is_selected = i == app.selected;
            let is_blocked = derived.is_blocked;
            let base_style = if is_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                match issue.status() {
                    Status::Done | Status::Skip => Style::default().fg(Color::DarkGray),
                    Status::Doing => {
                        let age_color = age_color(duration);
                        Style::default().fg(age_color)
                    }
                    Status::Open if is_blocked => {
                        // blocked issues are slightly dimmed
                        Style::default().fg(Color::Rgb(170, 170, 170))
                    }
                    Status::Open => Style::default(),
                }
            };

            // add type-based styling (italic for design, bold for meta)
            let style = match issue.issue_type() {
                Some(crate::issue::IssueType::Design) => base_style.add_modifier(Modifier::ITALIC),
                Some(crate::issue::IssueType::Meta) => base_style.add_modifier(Modifier::BOLD),
                None => base_style,
            };

            let mut rest_spans = vec![Span::styled(
                format!(
                    "{} {} {:>4} {:<10} {}",
                    id,
                    issue.priority(),
                    age,
                    owner,
                    title_part
                ),
                style,
            )];
            push_colored_tags(&mut rest_spans, &issue.frontmatter.tags, style);

            let line = if is_blocker && !is_selected {
                // show red "!" prefix for blockers (but not when selected, as bg is yellow)
                let mut spans = vec![Span::styled("! ", Style::default().fg(Color::Red))];
                spans.extend(rest_spans);
                Line::from(spans)
            } else {
                let mut spans = vec![Span::styled(status_prefix.to_string(), style)];
                spans.extend(rest_spans);
                Line::from(spans)
            };

            ListItem::new(line)
        })
        .collect();

    let visible_len = visible.len();
    let selected = if visible_len == 0 {
        None
    } else {
        Some(app.selected)
    };
    update_offset(&mut app.offset, selected, visible_len, view_height);
    let mut state = ListState::default()
        .with_selected(selected)
        .with_offset(app.offset);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut state);

    // render scrollbar if there are more items than fit in view
    if visible_len > view_height {
        let mut scrollbar_state = ScrollbarState::new(visible_len)
            .viewport_content_length(view_height)
            .position(app.offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        // render scrollbar in the inner area (inside the border)
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn issue_tags_width(tags: &[String]) -> usize {
    if tags.is_empty() {
        return 0;
    }
    // each tag contributes "#" + tag plus one separator between tags.
    tags.iter().map(|tag| tag.len() + 1).sum::<usize>() + tags.len() - 1
}

fn push_colored_tags(spans: &mut Vec<Span<'static>>, tags: &[String], style: Style) {
    if tags.is_empty() {
        return;
    }

    spans.push(Span::styled(" ", style));
    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", style));
        }
        let color = if tag == "bug" {
            Color::Red
        } else {
            Color::Cyan
        };
        let tag_style = style.patch(Style::default().fg(color));
        spans.push(Span::styled(format!("#{}", tag), tag_style));
    }
}

fn draw_detail(f: &mut Frame, area: Rect, app: &mut App) {
    let inner_height = area.height.saturating_sub(2) as usize; // subtract borders

    let is_focused = app.issues_focus == IssuesFocus::Details;
    let border_color = if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // build the content lines first, collecting all needed data
    let lines = build_detail_lines(app, app.detail_dep_selected);

    if lines.is_empty() {
        let text = Paragraph::new("no issue selected").block(block.clone().title(" Detail "));
        f.render_widget(text, area);
        return;
    }

    let content_height = lines.len();
    let max_scroll = content_height.saturating_sub(inner_height);

    // clamp scroll to valid range
    if app.detail_scroll > max_scroll {
        app.detail_scroll = max_scroll;
    }

    // build title with scroll indicator if scrolled
    let title = if app.detail_scroll > 0 {
        format!(" Detail [{}/{}] ", app.detail_scroll + 1, content_height)
    } else {
        " Detail ".to_string()
    };

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block.clone().title(title))
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll as u16, 0));
    f.render_widget(paragraph, area);

    // render scrollbar if content overflows
    if content_height > inner_height {
        let mut scrollbar_state = ScrollbarState::new(content_height)
            .viewport_content_length(inner_height)
            .position(app.detail_scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        // render scrollbar in the inner area (inside the border)
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

/// build content lines for the detail pane, returning empty vec if no issue selected.
/// `selected_dep` enables numbered prefixes and preview for the detail pane; pass None for overlay.
fn build_detail_lines(app: &App, selected_dep: Option<usize>) -> Vec<Line<'static>> {
    let Some(issue) = app.selected_issue() else {
        return Vec::new();
    };

    let derived = app.derived_state(issue);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("ID:       ", Style::default().fg(Color::DarkGray)),
            Span::raw(issue.id().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Title:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(issue.title().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
            Span::raw(issue.priority().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Status:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                issue.status().to_string(),
                match issue.status() {
                    crate::issue::Status::Done => Style::default().fg(Color::Green),
                    crate::issue::Status::Doing => Style::default().fg(Color::Yellow),
                    crate::issue::Status::Open => Style::default(),
                    crate::issue::Status::Skip => Style::default().fg(Color::DarkGray),
                },
            ),
        ]),
    ];

    if let Some(owner) = &issue.frontmatter.owner {
        lines.push(Line::from(vec![
            Span::styled("Owner:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(owner.clone()),
        ]));
    }

    // tags
    if !issue.frontmatter.tags.is_empty() {
        let tags = issue
            .frontmatter
            .tags
            .iter()
            .map(|t| format!("#{}", t))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(Line::from(vec![
            Span::styled("Tags:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(tags, Style::default().fg(Color::Cyan)),
        ]));
    }

    // state
    let state_text = if derived.is_ready {
        Span::styled("READY", Style::default().fg(Color::Green))
    } else if derived.is_blocked {
        Span::styled("BLOCKED", Style::default().fg(Color::Red))
    } else {
        Span::raw("")
    };
    if !state_text.content.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("State:    ", Style::default().fg(Color::DarkGray)),
            state_text,
        ]));
    }

    // deps
    if !issue.deps().is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Blocked by:",
            Style::default().fg(Color::DarkGray),
        )));
        for (idx, dep_id) in issue.deps().iter().enumerate() {
            let is_selected = selected_dep == Some(idx);
            // show number for selection (1-9) when selection enabled, otherwise bullet
            let prefix = if selected_dep.is_some() {
                if idx < 9 {
                    format!("{}", idx + 1)
                } else {
                    "-".to_string()
                }
            } else {
                " ".to_string()
            };

            let (symbol, status_text, base_color) = if let Some(dep_issue) = app.issues.get(dep_id)
            {
                match dep_issue.status() {
                    Status::Done => ("✓", "done", Color::Green),
                    Status::Skip => ("⊘", "skip", Color::DarkGray),
                    Status::Doing => ("→", "doing", Color::Yellow),
                    Status::Open => ("○", "open", Color::White),
                }
            } else {
                ("?", "missing", Color::Red)
            };

            let mut style = Style::default().fg(base_color);
            if is_selected {
                style = style.add_modifier(Modifier::BOLD);
            }

            lines.push(Line::from(Span::styled(
                format!("{} {} {} ({})", prefix, symbol, dep_id, status_text),
                style,
            )));
        }

        if let Some(dep_id) = selected_dep.and_then(|idx| issue.deps().get(idx)) {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "dependency preview:",
                Style::default().fg(Color::DarkGray),
            )));
            if let Some(dep_issue) = app.issues.get(dep_id) {
                let status_style = match dep_issue.status() {
                    crate::issue::Status::Done => Style::default().fg(Color::Green),
                    crate::issue::Status::Doing => Style::default().fg(Color::Yellow),
                    crate::issue::Status::Open => Style::default(),
                    crate::issue::Status::Skip => Style::default().fg(Color::DarkGray),
                };
                lines.push(Line::from(vec![
                    Span::styled("  id:       ", Style::default().fg(Color::DarkGray)),
                    Span::raw(dep_issue.id().to_string()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  title:    ", Style::default().fg(Color::DarkGray)),
                    Span::raw(dep_issue.title().to_string()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  status:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(dep_issue.status().to_string(), status_style),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  missing dependency issue",
                    Style::default().fg(Color::Red),
                )));
            }
        }
    }

    // dependents (reverse deps - issues that depend on this one)
    let dependents = get_dependents(issue.id(), &app.issues);
    if !dependents.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Blocks:",
            Style::default().fg(Color::DarkGray),
        )));
        for dep_id in &dependents {
            let (symbol, status_text, base_color) = if let Some(dep_issue) = app.issues.get(dep_id)
            {
                match dep_issue.status() {
                    Status::Done => ("✓", "done", Color::Green),
                    Status::Skip => ("⊘", "skip", Color::DarkGray),
                    Status::Doing => ("→", "doing", Color::Yellow),
                    Status::Open => ("○", "open", Color::White),
                }
            } else {
                ("?", "missing", Color::Red)
            };

            lines.push(Line::from(Span::styled(
                format!("  {} {} ({})", symbol, dep_id, status_text),
                Style::default().fg(base_color),
            )));
        }
    }

    // acceptance
    if !issue.frontmatter.acceptance.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Acceptance:",
            Style::default().fg(Color::DarkGray),
        )));
        for ac in &issue.frontmatter.acceptance {
            lines.push(Line::from(format!("  - {}", ac)));
        }
    }

    // body
    if !issue.body.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::DarkGray),
        )));
        for line in issue.body.lines() {
            lines.push(Line::from(format!("  {}", line)));
        }
    }

    lines
}

fn draw_detail_overlay(f: &mut Frame, area: Rect, app: &App) {
    // use most of the screen for the overlay
    let overlay_area = centered_rect(80, area.height.saturating_sub(4), area);

    // clear the area behind the overlay
    f.render_widget(Clear, overlay_area);

    let block = Block::default()
        .title(" Detail (press Esc, Enter, or Tab to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let lines = build_detail_lines(app, None);
    if lines.is_empty() {
        let text = Paragraph::new("no issue selected").block(block);
        f.render_widget(text, overlay_area);
        return;
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, overlay_area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" help (press ? to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "issues view - list focused",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑ / k      move up"),
        Line::from("  ↓ / j      move down"),
        Line::from("  g          go to top"),
        Line::from("  G          go to bottom"),
        Line::from("  Ctrl+u/d   half-page scroll"),
        Line::from("  Tab        switch focus to detail pane"),
        Line::from("  Enter      switch focus to detail pane"),
        Line::from(""),
        Line::from(Span::styled(
            "issues view - detail focused",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑ / k      scroll detail content"),
        Line::from("  ↓ / j      scroll detail content"),
        Line::from("  1-9        select dependency by number"),
        Line::from("  Ctrl+u/d   half-page scroll detail"),
        Line::from("  Tab / Esc  return focus to list"),
        Line::from("  Enter      jump to selected dependency"),
        Line::from(""),
        Line::from(Span::styled(
            "actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  a / n      add new issue"),
        Line::from("  e          edit selected issue"),
        Line::from("  s          start selected issue"),
        Line::from("  d          mark selected issue as done"),
        Line::from("  r          refresh issues from disk"),
        Line::from("  S          spawn agent for issue"),
        Line::from(""),
        Line::from(Span::styled(
            "views",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  1          dashboard"),
        Line::from("  2          issues"),
        Line::from("  3          agents"),
        Line::from("  \\          toggle details pane visibility"),
        Line::from(""),
        Line::from(Span::styled(
            "agents view",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  K          kill agent session"),
        Line::from("  L          view agent logs"),
        Line::from(""),
        Line::from(Span::styled(
            "filter",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  /          enter filter mode"),
        Line::from("  R          toggle ready filter"),
        Line::from("  enter      confirm filter"),
        Line::from("  esc        clear filter / unfocus detail"),
        Line::from(""),
        Line::from(Span::styled(
            "other",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?          toggle this help"),
        Line::from("  q          quit"),
        Line::from("  Ctrl+C     quit"),
    ];

    let paragraph = Paragraph::new(help_text).block(block);
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if max_len == 1 {
        return "…".to_string();
    }
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

fn format_age(timestamp: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - timestamp;
    let minutes = duration.whole_minutes();

    if minutes < 0 {
        "0m".to_string()
    } else if minutes < 60 {
        format!("{}m", minutes.max(1))
    } else if minutes < 60 * 24 {
        format!("{}h", minutes / 60)
    } else if minutes < 60 * 24 * 7 {
        format!("{}d", minutes / (60 * 24))
    } else if minutes < 60 * 24 * 30 {
        format!("{}w", minutes / (60 * 24 * 7))
    } else if minutes < 60 * 24 * 365 {
        format!("{}mo", minutes / (60 * 24 * 30))
    } else {
        format!("{}y", minutes / (60 * 24 * 365))
    }
}

fn age_color(duration: TimeDuration) -> Color {
    if duration < TimeDuration::hours(1) {
        Color::Green
    } else if duration < TimeDuration::days(1) {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn draw_input_dialog(f: &mut Frame, app: &App) {
    // determine dialog height based on mode
    let height = match &app.input_mode {
        InputMode::Title(_) => 3,        // slim: just input
        InputMode::Priority { .. } => 7, // title + 4 options
        InputMode::Type { .. } => 7,     // title + pri + 3 options
        InputMode::Deps { .. } => 12.min(app.sorted_issues.len() as u16 + 5),
        InputMode::Filter(_) | InputMode::Normal => return,
    };

    let area = centered_rect(50, height, f.area());

    // clear the area behind the dialog
    f.render_widget(Clear, area);

    match &app.input_mode {
        InputMode::Title(title) => {
            let block = Block::default()
                .title(" New Issue - Title (Enter, Esc) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let input = Paragraph::new(format!("{}_", title))
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(input, area);
        }
        InputMode::Priority { title, selected } => {
            let block = Block::default()
                .title(" Priority (Enter, Esc) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let priorities = ["P0 (critical)", "P1 (high)", "P2 (normal)", "P3 (low)"];
            let items: Vec<ListItem> = priorities
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let style = if i == *selected {
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("  {}", p)).style(style)
                })
                .collect();

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);

            let title_line =
                Paragraph::new(format!("Title: {}", title)).style(Style::default().fg(Color::Cyan));
            f.render_widget(title_line, chunks[0]);

            let list = List::new(items);
            f.render_widget(list, chunks[1]);
        }
        InputMode::Type {
            title,
            priority,
            selected,
        } => {
            let block = Block::default()
                .title(" Type (Enter, Esc) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let priorities = ["P0", "P1", "P2", "P3"];
            let types = ["(none)", "design", "meta"];
            let items: Vec<ListItem> = types
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let style = if i == *selected {
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("  {}", t)).style(style)
                })
                .collect();

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let title_line =
                Paragraph::new(format!("Title: {}", title)).style(Style::default().fg(Color::Cyan));
            f.render_widget(title_line, chunks[0]);

            let priority_line = Paragraph::new(format!("Priority: {}", priorities[*priority]))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(priority_line, chunks[1]);

            let list = List::new(items);
            f.render_widget(list, chunks[2]);
        }
        InputMode::Deps {
            title,
            priority,
            type_idx,
            selected_deps,
            cursor,
        } => {
            let block = Block::default()
                .title(" Blocked by (Space toggle, Enter create, Esc) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let priorities = ["P0", "P1", "P2", "P3"];
            let types = ["(none)", "design", "meta"];

            let items: Vec<ListItem> = app
                .sorted_issues
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    let is_selected = selected_deps.contains(id);
                    let checkbox = if is_selected { "[x]" } else { "[ ]" };
                    let style = if i == *cursor {
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else if is_selected {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" {} {}", checkbox, id)).style(style)
                })
                .collect();

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let title_line =
                Paragraph::new(format!("Title: {}", title)).style(Style::default().fg(Color::Cyan));
            f.render_widget(title_line, chunks[0]);

            let priority_line = Paragraph::new(format!("Priority: {}", priorities[*priority]))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(priority_line, chunks[1]);

            let type_line = Paragraph::new(format!("Type: {}", types[*type_idx]))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(type_line, chunks[2]);

            if app.sorted_issues.is_empty() {
                let empty = Paragraph::new("  (no existing issues)")
                    .style(Style::default().fg(Color::DarkGray));
                f.render_widget(empty, chunks[3]);
            } else {
                let list = List::new(items);
                f.render_widget(list, chunks[3]);
            }
        }
        InputMode::Filter(_) | InputMode::Normal => {}
    }
}

fn draw_diff_panel(f: &mut Frame, app: &mut App) {
    let Some(ref content) = app.diff_content else {
        return;
    };
    let Some(ref file_path) = app.diff_file_path else {
        return;
    };
    let Some(ref mut state) = app.diff_panel_state else {
        return;
    };

    // create overlay covering 90% of screen
    let area = centered_overlay(90, 90, f.area());

    let panel = DiffPanel::new(content.clone(), file_path.clone())
        .renderer_name(app.diff_renderer.display_name());
    f.render_stateful_widget(panel, area, state);
}

fn draw_logs_overlay(f: &mut Frame, app: &App) {
    // create overlay covering 90% of screen
    let area = centered_overlay(90, 90, f.area());

    // clear the area behind the overlay
    f.render_widget(Clear, area);

    let session_id = app.logs_session_id.as_deref().unwrap_or("unknown");
    let title = format!(" Logs: {} (j/k scroll, Esc close) ", session_id);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.logs_content.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (no log content)",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, inner);
        return;
    }

    // build lines with scroll offset
    let view_height = inner.height as usize;
    let lines: Vec<Line> = app
        .logs_content
        .iter()
        .skip(app.logs_scroll)
        .take(view_height)
        .map(|s| Line::from(s.as_str()))
        .collect();

    let scroll_info = format!(
        " [{}/{} lines] ",
        app.logs_scroll + 1,
        app.logs_content.len()
    );

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);

    // show scroll position at bottom right
    if app.logs_content.len() > view_height {
        let scroll_span = Span::styled(scroll_info, Style::default().fg(Color::DarkGray));
        let scroll_rect = Rect::new(
            area.x + area.width.saturating_sub(scroll_span.width() as u16 + 2),
            area.y + area.height - 1,
            scroll_span.width() as u16,
            1,
        );
        f.render_widget(Paragraph::new(scroll_span), scroll_rect);
    }
}

fn update_offset(offset: &mut usize, selected: Option<usize>, len: usize, view_height: usize) {
    if len == 0 || view_height == 0 {
        *offset = 0;
        return;
    }
    let view_height = view_height.min(len);
    let max_offset = len.saturating_sub(view_height);
    if *offset > max_offset {
        *offset = max_offset;
    }
    let Some(selected) = selected else {
        return;
    };
    if selected < *offset {
        *offset = selected;
    } else if selected >= *offset + view_height {
        *offset = selected + 1 - view_height;
    }
}

/// create a centered rect of given percentage width and fixed height.
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
