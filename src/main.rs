use braid::cli::{AgentAction, Cli, Command, DepAction};
use braid::commands::{
    cmd_add, cmd_agent_init, cmd_completions, cmd_dep_add, cmd_dep_rm, cmd_doctor, cmd_done,
    cmd_init, cmd_ls, cmd_migrate, cmd_next, cmd_ready, cmd_ship, cmd_show, cmd_start, cmd_tui,
};
use braid::error::Result;
use braid::repo;
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let result = run(&cli);

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            if cli.json {
                let json = serde_json::json!({
                    "ok": false,
                    "code": e.code_str(),
                    "message": e.to_string(),
                    "exit": i32::from(e.exit_code()),
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                eprintln!("error: {}", e);
            }
            std::process::exit(e.exit_code().into());
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    // handle commands that don't require existing repo
    if matches!(cli.command, Command::Init) {
        return cmd_init(cli);
    }
    if let Command::Completions { shell } = &cli.command {
        return cmd_completions(*shell);
    }

    // all other commands need repo discovery
    let paths = repo::discover(cli.repo.as_deref())?;

    match &cli.command {
        Command::Init => unreachable!(),
        Command::Add(args) => cmd_add(cli, &paths, args),
        Command::Ls {
            status,
            priority,
            ready,
            blocked,
            label,
            all,
        } => cmd_ls(
            cli,
            &paths,
            status.as_deref(),
            priority.as_deref(),
            *ready,
            *blocked,
            label,
            *all,
        ),
        Command::Show { id } => cmd_show(cli, &paths, id),
        Command::Ready => cmd_ready(cli, &paths),
        Command::Next => cmd_next(cli, &paths),
        Command::Dep { action } => match action {
            DepAction::Add { child, parent } => cmd_dep_add(cli, &paths, child, parent),
            DepAction::Rm { child, parent } => cmd_dep_rm(cli, &paths, child, parent),
        },
        Command::Start { id, force } => cmd_start(cli, &paths, id.as_deref(), *force),
        Command::Done { id, force } => cmd_done(cli, &paths, id, *force),
        Command::Agent { action } => match action {
            AgentAction::Init { name, base } => cmd_agent_init(cli, &paths, name, base.as_deref()),
            AgentAction::Ship => cmd_ship(cli, &paths),
        },
        Command::Doctor => cmd_doctor(cli, &paths),
        Command::Completions { .. } => unreachable!(),
        Command::Tui => cmd_tui(cli, &paths),
        Command::Migrate { dry_run } => cmd_migrate(cli, &paths, *dry_run),
    }
}
