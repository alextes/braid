pub mod cli;
pub mod commands;
pub mod config;
pub mod date;
pub mod error;
pub mod git;
pub mod graph;
pub mod issue;
pub mod lock;
pub mod migrate;
pub mod repo;
pub mod session;
#[cfg(test)]
pub mod test_utils;
pub mod tui;

use std::io::IsTerminal;

/// Check if we're running in an interactive terminal.
/// Returns false when stdin is not a TTY (e.g., running under an AI agent).
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

/// Print verbose output to stderr if verbose mode is enabled.
#[macro_export]
macro_rules! verbose {
    ($cli:expr, $($arg:tt)*) => {
        if $cli.verbose {
            eprintln!("[brd] {}", format!($($arg)*));
        }
    };
}
