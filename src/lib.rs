pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod graph;
pub mod issue;
pub mod lock;
pub mod migrate;
pub mod repo;
pub mod tui;

/// Print verbose output to stderr if verbose mode is enabled.
#[macro_export]
macro_rules! verbose {
    ($cli:expr, $($arg:tt)*) => {
        if $cli.verbose {
            eprintln!("[brd] {}", format!($($arg)*));
        }
    };
}
