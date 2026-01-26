//! pluggable diff rendering strategies.
//!
//! provides a trait for rendering unified diff content to ratatui `Text`,
//! with multiple implementations for different rendering approaches.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

use crate::error::Result;

/// trait for rendering diff content to ratatui Text.
pub trait DiffRenderer {
    /// render diff content to styled ratatui Text.
    ///
    /// # arguments
    /// * `diff` - the raw unified diff content
    /// * `width` - available width for rendering (useful for external tools)
    fn render(&self, diff: &str, width: u16) -> Result<Text<'static>>;

    /// name of this renderer (for UI/config display).
    fn name(&self) -> &'static str;
}

/// native renderer that parses diff and applies inline styling.
///
/// styles lines based on their prefix:
/// - `+` lines: green (additions)
/// - `-` lines: red (deletions)
/// - `@@` lines: cyan (hunk headers)
/// - `diff --git`, `index`, `---`, `+++`: bold (file headers)
/// - other lines: default (context)
pub struct NativeRenderer;

impl DiffRenderer for NativeRenderer {
    fn render(&self, diff: &str, _width: u16) -> Result<Text<'static>> {
        let lines: Vec<Line<'static>> = diff.lines().map(style_diff_line).collect();
        Ok(Text::from(lines))
    }

    fn name(&self) -> &'static str {
        "native"
    }
}

/// style a single diff line based on its content.
fn style_diff_line(line: &str) -> Line<'static> {
    let owned_line = line.to_string();

    if owned_line.starts_with('+') && !owned_line.starts_with("+++") {
        // addition line - green
        Line::from(Span::styled(owned_line, Style::default().fg(Color::Green)))
    } else if owned_line.starts_with('-') && !owned_line.starts_with("---") {
        // deletion line - red
        Line::from(Span::styled(owned_line, Style::default().fg(Color::Red)))
    } else if owned_line.starts_with("@@") {
        // hunk header - cyan
        Line::from(Span::styled(owned_line, Style::default().fg(Color::Cyan)))
    } else if owned_line.starts_with("diff --git")
        || owned_line.starts_with("index ")
        || owned_line.starts_with("---")
        || owned_line.starts_with("+++")
    {
        // file header - bold
        Line::from(Span::styled(
            owned_line,
            Style::default().add_modifier(Modifier::BOLD),
        ))
    } else {
        // context line - default
        Line::from(Span::raw(owned_line))
    }
}

/// external tool renderer that pipes diff through a command and converts ANSI output.
///
/// uses tools like `delta` or `diff-so-fancy` for enhanced diff rendering.
/// the `{width}` placeholder in the command is replaced with the actual width.
pub struct ExternalRenderer {
    /// command template, e.g. "delta --width={width} --paging=never"
    pub command: String,
}

impl ExternalRenderer {
    /// create a new external renderer with the given command template.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// check if the external tool is available on the system.
    pub fn is_available(&self) -> bool {
        // extract the tool name from the command (first word)
        let tool = self
            .command
            .split_whitespace()
            .next()
            .unwrap_or(&self.command);

        std::process::Command::new("which")
            .arg(tool)
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

impl DiffRenderer for ExternalRenderer {
    fn render(&self, diff: &str, width: u16) -> Result<Text<'static>> {
        use ansi_to_tui::IntoText;
        use std::io::Write;
        use std::process::{Command, Stdio};

        // replace {width} placeholder
        let cmd = self.command.replace("{width}", &width.to_string());

        // spawn the external tool
        let mut child = Command::new("sh")
            .args(["-c", &cmd])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                crate::error::BrdError::Other(format!("failed to spawn {}: {}", cmd, e))
            })?;

        // write diff to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(diff.as_bytes()).ok();
        }

        // read output
        let output = child.wait_with_output().map_err(|e| {
            crate::error::BrdError::Other(format!("failed to read output from {}: {}", cmd, e))
        })?;

        // convert ANSI to ratatui Text
        output.stdout.into_text().map_err(|e| {
            crate::error::BrdError::Other(format!("failed to parse ANSI output: {}", e))
        })
    }

    fn name(&self) -> &'static str {
        "external"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, world!");
+    println!("Goodbye");
 }
"#;

    #[test]
    fn test_native_renderer_creates_text() {
        let renderer = NativeRenderer;
        let text = renderer.render(SAMPLE_DIFF, 80).expect("render failed");
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_native_renderer_name() {
        let renderer = NativeRenderer;
        assert_eq!(renderer.name(), "native");
    }

    #[test]
    fn test_style_diff_line_addition() {
        let line = style_diff_line("+    println!(\"new line\");");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_style_diff_line_deletion() {
        let line = style_diff_line("-    println!(\"old line\");");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_style_diff_line_hunk_header() {
        let line = style_diff_line("@@ -1,5 +1,6 @@");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_diff_line_file_header() {
        let line = style_diff_line("diff --git a/foo.rs b/foo.rs");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_style_diff_line_context() {
        let line = style_diff_line(" fn main() {");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, None);
    }

    #[test]
    fn test_plus_plus_plus_is_not_addition() {
        // +++ should be styled as header, not addition
        let line = style_diff_line("+++ b/src/main.rs");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_ne!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_minus_minus_minus_is_not_deletion() {
        // --- should be styled as header, not deletion
        let line = style_diff_line("--- a/src/main.rs");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_ne!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_external_renderer_name() {
        let renderer = ExternalRenderer::new("delta");
        assert_eq!(renderer.name(), "external");
    }
}
