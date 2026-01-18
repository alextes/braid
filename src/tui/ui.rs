//! TUI rendering functions.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::app::{App, InputMode};

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
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let text = format!("brd tui — agent: {}", app.agent_id);
    let header = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let msg = app.message.as_deref().unwrap_or("");
    let help =
        "[a]dd [e]dit [s]tart [d]one [r]efresh [/]filter [1-4]status [g/G]top/bot [?]help [q]uit";
    let text = if msg.is_empty() {
        help.to_string()
    } else {
        format!("{} │ {}", msg, help)
    };
    let footer = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_issue_list(f, chunks[0], app);
    draw_detail(f, chunks[1], app);
}

fn draw_issue_list(f: &mut Frame, area: Rect, app: &mut App) {
    let border_style = Style::default().fg(Color::Yellow);

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
    // area - borders(2) - status(1) - id(8) - priority(2) - age(4) - owner(12) - spaces(6)
    let title_width = area.width.saturating_sub(35) as usize;
    let view_height = block.inner(area).height as usize;
    let now = OffsetDateTime::now_utc();

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let issue = app.issues.get(id).unwrap();
            let status_char = match issue.status() {
                crate::issue::Status::Open => ' ',
                crate::issue::Status::Doing => '→',
                crate::issue::Status::Done => '✓',
                crate::issue::Status::Skip => '⊘',
            };
            let age = format_age(issue.frontmatter.created_at);
            let owner = issue
                .frontmatter
                .owner
                .as_deref()
                .map(|o| truncate(o, 10))
                .unwrap_or_else(|| "-".to_string());
            let tags = issue
                .frontmatter
                .tags
                .iter()
                .map(|t| format!("#{}", t))
                .collect::<Vec<_>>()
                .join(" ");
            let title_and_tags = if tags.is_empty() {
                truncate(issue.title(), title_width)
            } else {
                let title_part =
                    truncate(issue.title(), title_width.saturating_sub(tags.len() + 1));
                format!("{} {}", title_part, tags)
            };

            let text = format!(
                "{} {} {} {:>4} {:<10} {}",
                status_char,
                id,
                issue.priority(),
                age,
                owner,
                title_and_tags
            );

            let duration = now - issue.frontmatter.created_at;
            let mut style = if i == app.selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                match issue.status() {
                    crate::issue::Status::Done | crate::issue::Status::Skip => {
                        Style::default().fg(Color::DarkGray)
                    }
                    crate::issue::Status::Doing => {
                        let age_color = age_color(duration);
                        Style::default().fg(age_color)
                    }
                    crate::issue::Status::Open => Style::default(),
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
                    crate::issue::Status::Open => Style::default(),
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
                    crate::issue::Status::Open => Style::default(),
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
        .title(" help (press ? to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑ / k      move up"),
        Line::from("  ↓ / j      move down"),
        Line::from("  g          go to top"),
        Line::from("  G          go to bottom"),
        Line::from("  ← / h      select previous dependency"),
        Line::from("  → / l      select next dependency"),
        Line::from(""),
        Line::from(Span::styled(
            "actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  a / n      add new issue"),
        Line::from("  e          edit selected issue"),
        Line::from("  s          start selected issue"),
        Line::from("  d          mark selected issue as done"),
        Line::from("  enter      open selected dependency"),
        Line::from("  r          refresh issues from disk"),
        Line::from(""),
        Line::from(Span::styled(
            "filter",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  /          enter filter mode"),
        Line::from("  enter      confirm filter"),
        Line::from("  esc        clear filter"),
        Line::from("  1-4        toggle status (1=open 2=doing 3=done 4=skip)"),
        Line::from(""),
        Line::from(Span::styled(
            "other",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?          toggle this help"),
        Line::from("  q          quit"),
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
                .title(" Dependencies (Space toggle, Enter create, Esc) ")
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
