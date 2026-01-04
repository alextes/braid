//! brd completions command.

use crate::cli::Cli;
use crate::error::Result;

pub fn cmd_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(shell, &mut Cli::command(), "brd", &mut std::io::stdout());
    Ok(())
}

/// Generate completions to a writer (for testing).
#[cfg(test)]
fn generate_completions_to<W: std::io::Write>(
    shell: clap_complete::Shell,
    writer: &mut W,
) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(shell, &mut Cli::command(), "brd", writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap_complete::Shell;

    #[test]
    fn test_generate_bash_completions() {
        let mut output = Vec::new();
        let result = generate_completions_to(Shell::Bash, &mut output);
        assert!(result.is_ok());
        assert!(!output.is_empty());
        let content = String::from_utf8_lossy(&output);
        assert!(content.contains("brd"));
    }

    #[test]
    fn test_generate_zsh_completions() {
        let mut output = Vec::new();
        let result = generate_completions_to(Shell::Zsh, &mut output);
        assert!(result.is_ok());
        assert!(!output.is_empty());
        let content = String::from_utf8_lossy(&output);
        assert!(content.contains("brd"));
    }

    #[test]
    fn test_generate_fish_completions() {
        let mut output = Vec::new();
        let result = generate_completions_to(Shell::Fish, &mut output);
        assert!(result.is_ok());
        assert!(!output.is_empty());
        let content = String::from_utf8_lossy(&output);
        assert!(content.contains("brd"));
    }

    #[test]
    fn test_completions_contain_subcommands() {
        let mut output = Vec::new();
        generate_completions_to(Shell::Bash, &mut output).unwrap();
        let content = String::from_utf8_lossy(&output);

        // Check that common subcommands are in the completions
        assert!(content.contains("add"), "completions should include 'add'");
        assert!(content.contains("ls"), "completions should include 'ls'");
        assert!(
            content.contains("done"),
            "completions should include 'done'"
        );
        assert!(
            content.contains("start"),
            "completions should include 'start'"
        );
    }
}
