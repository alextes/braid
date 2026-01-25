//! brd tui command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::is_interactive;
use crate::repo::RepoPaths;
use crate::tui;

pub fn cmd_tui(_cli: &Cli, paths: &RepoPaths, force: bool) -> Result<()> {
    // check for interactive terminal
    if !force && !is_interactive() {
        return Err(BrdError::Other(
            "brd tui requires an interactive terminal.\n\
             hint: use `brd ls` or `brd show <id>` instead\n\
             hint: use --force to override this check"
                .to_string(),
        ));
    }

    tui::run(paths)
}
