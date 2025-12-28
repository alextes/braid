//! TUI rendering functions.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::app::{ActivePane, App, InputMode, ViewMode};

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
    if !matches!(app.input_mode, InputMode::Normal) {
        draw_input_dialog(f, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let text = if app.view_mode == ViewMode::Live {
        format!("brd tui — live — agent: {}", app.agent_id)
    } else {
        format!("brd tui — agent: {}", app.agent_id)
    };
    let header = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let msg = app.message.as_deref().unwrap_or("");
    let help = "[a]dd [e]dit [s]tart [d]one [r]efresh [v]live [↑↓/jk]nav [Tab]switch [h/l]dep [enter]open dep [?]help [q]uit";
    let text = if msg.is_empty() {
        help.to_string()
    } else {
        format!("{} │ {}", msg, help)
    };
    let footer = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    match app.view_mode {
        ViewMode::Normal => draw_normal_main(f, area, app),
        ViewMode::Live => draw_live_main(f, area, app),
    }
}

fn draw_normal_main(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_lists(f, chunks[0], app);
    draw_detail(f, chunks[1], app);
}

fn draw_lists(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_ready_list(f, chunks[0], app);
    draw_all_list(f, chunks[1], app);
}

fn draw_live_main(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_live_lists(f, chunks[0], app);
    draw_detail(f, chunks[1], app);
}

fn draw_live_lists(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    draw_in_progress_list(f, chunks[0], app);
    draw_recent_done_list(f, chunks[1], app);
}

fn draw_in_progress_list(f: &mut Frame, area: Rect, app: &mut App) {
    let border_style = Style::default().fg(Color::Yellow);
    let title = format!(" In Progress ({}) ", app.in_progress_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let owner_width = 12usize;
    let fixed_width = 4 + 2 + 8 + 1 + 2 + 1 + owner_width + 1;
    let title_width = usize::from(area.width.saturating_sub(fixed_width as u16)).max(1);
    let view_height = block.inner(area).height as usize;
    let now = OffsetDateTime::now_utc();

    let items: Vec<ListItem> = app
        .in_progress_issues
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let issue = app.issues.get(id).unwrap();
            let age = format_age(issue.frontmatter.updated_at);
            let owner = issue.frontmatter.owner.as_deref().unwrap_or("unknown");
            let owner = truncate(owner, owner_width);
            let title = truncate(issue.title(), title_width);
            let text = format!(
                "{:>4}  {} {} {:<width$} {}",
                age,
                id,
                issue.priority(),
                owner,
                title,
                width = owner_width
            );
            let duration = now - issue.frontmatter.updated_at;
            let age_color = age_color(duration);
            let style = if !app.in_progress_issues.is_empty() && i == app.in_progress_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(age_color)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let selected = if app.in_progress_issues.is_empty() {
        None
    } else {
        Some(app.in_progress_selected)
    };
    update_offset(
        &mut app.in_progress_offset,
        selected,
        app.in_progress_issues.len(),
        view_height,
    );
    let mut state = ListState::default()
        .with_selected(selected)
        .with_offset(app.in_progress_offset);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_recent_done_list(f: &mut Frame, area: Rect, app: &mut App) {
    let title = format!(" Recently Done ({}) ", app.recent_done_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let fixed_width = 4 + 1 + 1 + 8 + 1;
    let title_width = usize::from(area.width.saturating_sub(fixed_width as u16)).max(1);

    let items: Vec<ListItem> = app
        .recent_done_issues
        .iter()
        .map(|id| {
            let issue = app.issues.get(id).unwrap();
            let age = format_age(issue.frontmatter.updated_at);
            let status_char = match issue.status() {
                crate::issue::Status::Done => '✓',
                crate::issue::Status::Skip => '⊘',
                _ => ' ',
            };
            let title = truncate(issue.title(), title_width);
            let text = format!("{:>4} {} {} {}", age, status_char, id, title);
            let style = match issue.status() {
                crate::issue::Status::Done => Style::default().fg(Color::Green),
                crate::issue::Status::Skip => Style::default().fg(Color::DarkGray),
                _ => Style::default(),
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_ready_list(f: &mut Frame, area: Rect, app: &mut App) {
    let is_active = app.active_pane == ActivePane::Ready;
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" Ready ({}) ", app.ready_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // calculate available width for title: area - borders(2) - id(8) - priority(2) - spaces(2)
    let title_width = area.width.saturating_sub(14) as usize;
    let view_height = block.inner(area).height as usize;

    let items: Vec<ListItem> = app
        .ready_issues
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let issue = app.issues.get(id).unwrap();
            let text = format!(
                "{} {} {}",
                id,
                issue.priority(),
                truncate(issue.title(), title_width)
            );
            let mut style = if is_active && i == app.ready_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            // add type-based styling (italic for design, bold for meta)
            match issue.issue_type() {
                Some(crate::issue::IssueType::Design) => {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                Some(crate::issue::IssueType::Meta) => {
                    style = style.add_modifier(Modifier::BOLD);
                }
                None => {}
            }
            ListItem::new(text).style(style)
        })
        .collect();

    let selected = if app.ready_issues.is_empty() {
        None
    } else {
        Some(app.ready_selected)
    };
    update_offset(
        &mut app.ready_offset,
        selected,
        app.ready_issues.len(),
        view_height,
    );
    let mut state = ListState::default()
        .with_selected(selected)
        .with_offset(app.ready_offset);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_all_list(f: &mut Frame, area: Rect, app: &mut App) {
    let is_active = app.active_pane == ActivePane::All;
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" All ({}) ", app.all_issues.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // calculate available width for title: area - borders(2) - status(1) - id(8) - priority(2) - spaces(3)
    let title_width = area.width.saturating_sub(16) as usize;
    let view_height = block.inner(area).height as usize;

    let items: Vec<ListItem> = app
        .all_issues
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let issue = app.issues.get(id).unwrap();
            let status_char = match issue.status() {
                crate::issue::Status::Todo => ' ',
                crate::issue::Status::Doing => '→',
                crate::issue::Status::Done => '✓',
                crate::issue::Status::Skip => '⊘',
            };
            let text = format!(
                "{} {} {} {}",
                status_char,
                id,
                issue.priority(),
                truncate(issue.title(), title_width)
            );
            let mut style = if is_active && i == app.all_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                match issue.status() {
                    crate::issue::Status::Done | crate::issue::Status::Skip => {
                        Style::default().fg(Color::DarkGray)
                    }
                    crate::issue::Status::Doing => Style::default().fg(Color::Green),
                    crate::issue::Status::Todo => Style::default(),
                }
            };
            // add type-based styling (italic for design, bold for meta)
            match issue.issue_type() {
                Some(crate::issue::IssueType::Design) => {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                Some(crate::issue::IssueType::Meta) => {
                    style = style.add_modifier(Modifier::BOLD);
                }
                None => {}
            }
            ListItem::new(text).style(style)
        })
        .collect();

    let selected = if app.all_issues.is_empty() {
        None
    } else {
        Some(app.all_selected)
    };
    update_offset(
        &mut app.all_offset,
        selected,
        app.all_issues.len(),
        view_height,
    );
    let mut state = ListState::default()
        .with_selected(selected)
        .with_offset(app.all_offset);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let Some(issue) = app.selected_issue() else {
        let text = Paragraph::new("no issue selected").block(block);
        f.render_widget(text, area);
        return;
    };

    let derived = app.derived_state(issue);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("ID:       ", Style::default().fg(Color::DarkGray)),
            Span::raw(issue.id()),
        ]),
        Line::from(vec![
            Span::styled("Title:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(issue.title()),
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
                    crate::issue::Status::Todo => Style::default(),
                    crate::issue::Status::Skip => Style::default().fg(Color::DarkGray),
                },
            ),
        ]),
    ];

    if let Some(owner) = &issue.frontmatter.owner {
        lines.push(Line::from(vec![
            Span::styled("Owner:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(owner.as_str()),
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
        let selected_dep = app.detail_dep_selected;
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Dependencies:",
            Style::default().fg(Color::DarkGray),
        )));
        for (idx, dep) in issue.deps().iter().enumerate() {
            let is_resolved = app
                .issues
                .get(dep)
                .map(|d| {
                    matches!(
                        d.status(),
                        crate::issue::Status::Done | crate::issue::Status::Skip
                    )
                })
                .unwrap_or(false);
            let mut style = if is_resolved {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            let prefix = if Some(idx) == selected_dep { ">" } else { "-" };
            if Some(idx) == selected_dep {
                style = style.add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(
                format!("  {} {}", prefix, dep),
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
                    crate::issue::Status::Todo => Style::default(),
                    crate::issue::Status::Skip => Style::default().fg(Color::DarkGray),
                };
                lines.push(Line::from(vec![
                    Span::styled("  id:       ", Style::default().fg(Color::DarkGray)),
                    Span::raw(dep_issue.id()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  title:    ", Style::default().fg(Color::DarkGray)),
                    Span::raw(dep_issue.title()),
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

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Help (press ? to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑ / k      Move up"),
        Line::from("  ↓ / j      Move down"),
        Line::from("  ← / h      select previous dependency"),
        Line::from("  → / l      select next dependency"),
        Line::from("  Tab        Switch pane (Ready ↔ All)"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  a / n      Add new issue"),
        Line::from("  e          Edit selected issue"),
        Line::from("  s          Start selected issue"),
        Line::from("  d          Mark selected issue as done"),
        Line::from("  enter      open selected dependency"),
        Line::from("  r          Refresh issues from disk"),
        Line::from("  v          Toggle live view"),
        Line::from(""),
        Line::from(Span::styled(
            "Other",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?          Toggle this help"),
        Line::from("  q          Quit"),
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

fn format_age(updated_at: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - updated_at;
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
    let area = centered_rect(50, 8, f.area());

    // clear the area behind the dialog
    f.render_widget(Clear, area);

    match &app.input_mode {
        InputMode::Title(title) => {
            let block = Block::default()
                .title(" New Issue - Title (Enter to confirm, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let input = Paragraph::new(format!("{}_", title))
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(input, area);
        }
        InputMode::Priority { title, selected } => {
            let block = Block::default()
                .title(" New Issue - Priority (Enter to create, Esc to cancel) ")
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

            // show title at top of dialog
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
        InputMode::EditSelect { issue_id, selected } => {
            let block = Block::default()
                .title(" Edit Issue - Select Field (Enter to edit, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let fields = ["Title", "Priority", "Status"];
            let items: Vec<ListItem> = fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let style = if i == *selected {
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("  {}", field)).style(style)
                })
                .collect();

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);

            let id_line = Paragraph::new(format!("Editing: {}", issue_id))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(id_line, chunks[0]);

            let list = List::new(items);
            f.render_widget(list, chunks[1]);
        }
        InputMode::EditTitle { issue_id, current } => {
            let block = Block::default()
                .title(" Edit Title (Enter to save, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);

            let id_line = Paragraph::new(format!("Issue: {}", issue_id))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(id_line, chunks[0]);

            let input =
                Paragraph::new(format!("{}_", current)).style(Style::default().fg(Color::White));
            f.render_widget(input, chunks[1]);
        }
        InputMode::EditPriority { issue_id, selected } => {
            let block = Block::default()
                .title(" Edit Priority (Enter to save, Esc to cancel) ")
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

            let id_line = Paragraph::new(format!("Issue: {}", issue_id))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(id_line, chunks[0]);

            let list = List::new(items);
            f.render_widget(list, chunks[1]);
        }
        InputMode::EditStatus { issue_id, selected } => {
            let block = Block::default()
                .title(" Edit Status (Enter to save, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            let statuses = ["Todo", "Doing", "Done", "Skip"];
            let items: Vec<ListItem> = statuses
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let style = if i == *selected {
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("  {}", s)).style(style)
                })
                .collect();

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);

            let id_line = Paragraph::new(format!("Issue: {}", issue_id))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(id_line, chunks[0]);

            let list = List::new(items);
            f.render_widget(list, chunks[1]);
        }
        InputMode::Normal => {}
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
