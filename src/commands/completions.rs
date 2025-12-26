//! brd completions command.

use crate::cli::Cli;
use crate::error::Result;

pub fn cmd_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(shell, &mut Cli::command(), "brd", &mut std::io::stdout());
    Ok(())
}
