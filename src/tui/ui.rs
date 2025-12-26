//! TUI rendering functions.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use super::app::{ActivePane, App};

/// draw the entire UI.
pub fn draw(f: &mut Frame, app: &App) {
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
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let text = format!("brd tui — agent: {}", app.agent_id);
    let header = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let msg = app.message.as_deref().unwrap_or("");
    let help = "[s]tart [d]one [r]efresh [↑↓/jk]nav [Tab]switch [?]help [q]uit";
    let text = if msg.is_empty() {
        help.to_string()
    } else {
        format!("{} │ {}", msg, help)
    };
    let footer = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_lists(f, chunks[0], app);
    draw_detail(f, chunks[1], app);
}

fn draw_lists(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_ready_list(f, chunks[0], app);
    draw_all_list(f, chunks[1], app);
}

fn draw_ready_list(f: &mut Frame, area: Rect, app: &App) {
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
            let style = if is_active && i == app.ready_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_all_list(f: &mut Frame, area: Rect, app: &App) {
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
            };
            let text = format!(
                "{} {} {} {}",
                status_char,
                id,
                issue.priority(),
                truncate(issue.title(), title_width)
            );
            let style = if is_active && i == app.all_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                match issue.status() {
                    crate::issue::Status::Done => Style::default().fg(Color::DarkGray),
                    crate::issue::Status::Doing => Style::default().fg(Color::Green),
                    crate::issue::Status::Todo => Style::default(),
                }
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
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
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Dependencies:",
            Style::default().fg(Color::DarkGray),
        )));
        for dep in issue.deps() {
            let is_done = app
                .issues
                .get(dep)
                .map(|d| d.status() == crate::issue::Status::Done)
                .unwrap_or(false);
            let style = if is_done {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(Span::styled(format!("  - {}", dep), style)));
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
        Line::from("  Tab        Switch pane (Ready ↔ All)"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s          Start selected issue"),
        Line::from("  d          Mark selected issue as done"),
        Line::from("  r          Refresh issues from disk"),
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
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
