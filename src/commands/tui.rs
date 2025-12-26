//! brd tui command.

use crate::cli::Cli;
use crate::error::Result;
use crate::repo::RepoPaths;
use crate::tui;

pub fn cmd_tui(_cli: &Cli, paths: &RepoPaths) -> Result<()> {
    tui::run(paths)
}
