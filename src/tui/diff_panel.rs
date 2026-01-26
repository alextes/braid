//! diff panel component for displaying unified diff content.
//!
//! provides a scrollable panel that renders diff content as a modal overlay.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget},
};

/// state for the diff panel, tracking scroll position.
#[derive(Debug, Default)]
pub struct DiffPanelState {
    /// vertical scroll offset (line number at top of viewport)
    pub scroll: u16,
}

impl DiffPanelState {
    /// create a new diff panel state with scroll at top.
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    /// scroll up by the given number of lines.
    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    /// scroll down by the given number of lines, clamped to content height.
    pub fn scroll_down(&mut self, lines: u16, content_height: u16, viewport_height: u16) {
        let max_scroll = content_height.saturating_sub(viewport_height);
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    /// scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    /// scroll to bottom.
    pub fn scroll_to_bottom(&mut self, content_height: u16, viewport_height: u16) {
        self.scroll = content_height.saturating_sub(viewport_height);
    }

    /// page up (scroll by viewport height).
    pub fn page_up(&mut self, viewport_height: u16) {
        self.scroll_up(viewport_height.saturating_sub(1));
    }

    /// page down (scroll by viewport height).
    pub fn page_down(&mut self, content_height: u16, viewport_height: u16) {
        self.scroll_down(
            viewport_height.saturating_sub(1),
            content_height,
            viewport_height,
        );
    }
}

/// diff panel widget for displaying diff content.
pub struct DiffPanel<'a> {
    /// pre-styled diff content
    content: Text<'a>,
    /// file path being displayed
    file_path: String,
    /// block style
    block_style: Style,
}

impl<'a> DiffPanel<'a> {
    /// create a new diff panel with the given content and file path.
    pub fn new(content: Text<'a>, file_path: impl Into<String>) -> Self {
        Self {
            content,
            file_path: file_path.into(),
            block_style: Style::default().fg(Color::Yellow),
        }
    }

    /// set the border style.
    pub fn block_style(mut self, style: Style) -> Self {
        self.block_style = style;
        self
    }

    /// get the content height (number of lines).
    pub fn content_height(&self) -> u16 {
        self.content.lines.len() as u16
    }
}

impl StatefulWidget for DiffPanel<'_> {
    type State = DiffPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // clear the area first (for overlay effect)
        Clear.render(area, buf);

        // build title with file path and scroll info
        let content_height = self.content_height();
        let viewport_height = area.height.saturating_sub(2); // minus borders
        let scroll_info = if content_height > viewport_height {
            format!(
                " [{}/{}] ",
                state.scroll + 1,
                content_height.saturating_sub(viewport_height) + 1
            )
        } else {
            String::new()
        };
        let title = format!(
            " {} {}[j/k scroll, q/Esc close] ",
            self.file_path, scroll_info
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.block_style);

        // create scrolled paragraph
        let paragraph = Paragraph::new(self.content)
            .block(block)
            .scroll((state.scroll, 0));

        paragraph.render(area, buf);
    }
}

/// convenience function to create a centered rect for the diff overlay.
pub fn centered_overlay(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    fn make_text(lines: usize) -> Text<'static> {
        let lines: Vec<Line<'static>> = (0..lines)
            .map(|i| Line::raw(format!("line {}", i)))
            .collect();
        Text::from(lines)
    }

    #[test]
    fn test_diff_panel_state_new() {
        let state = DiffPanelState::new();
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_scroll_up_from_zero() {
        let mut state = DiffPanelState::new();
        state.scroll_up(5);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = DiffPanelState { scroll: 10 };
        state.scroll_up(3);
        assert_eq!(state.scroll, 7);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = DiffPanelState::new();
        state.scroll_down(5, 100, 20);
        assert_eq!(state.scroll, 5);
    }

    #[test]
    fn test_scroll_down_clamped() {
        let mut state = DiffPanelState::new();
        // content: 30 lines, viewport: 20 lines, max scroll: 10
        state.scroll_down(50, 30, 20);
        assert_eq!(state.scroll, 10);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut state = DiffPanelState { scroll: 50 };
        state.scroll_to_top();
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut state = DiffPanelState::new();
        state.scroll_to_bottom(100, 20);
        assert_eq!(state.scroll, 80);
    }

    #[test]
    fn test_page_up() {
        let mut state = DiffPanelState { scroll: 50 };
        state.page_up(20);
        assert_eq!(state.scroll, 31); // 50 - 19
    }

    #[test]
    fn test_page_down() {
        let mut state = DiffPanelState::new();
        state.page_down(100, 20);
        assert_eq!(state.scroll, 19); // page is viewport - 1
    }

    #[test]
    fn test_diff_panel_content_height() {
        let text = make_text(50);
        let panel = DiffPanel::new(text, "test.rs");
        assert_eq!(panel.content_height(), 50);
    }

    #[test]
    fn test_centered_overlay() {
        let area = Rect::new(0, 0, 100, 50);
        let overlay = centered_overlay(80, 80, area);

        // should be centered and 80% of original
        assert_eq!(overlay.width, 80);
        assert_eq!(overlay.height, 40);
        assert_eq!(overlay.x, 10); // (100 - 80) / 2
        assert_eq!(overlay.y, 5); // (50 - 40) / 2
    }
}
