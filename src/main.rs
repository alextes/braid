use braid::cli::{AgentAction, Cli, Command, ConfigAction, DepAction};
use braid::commands::{
    cmd_add, cmd_agent_branch, cmd_agent_init, cmd_agent_pr, cmd_agents_inject, cmd_agents_show,
    cmd_commit, cmd_completions, cmd_config_auto_sync, cmd_config_external_repo,
    cmd_config_issues_branch, cmd_config_show, cmd_dep_add, cmd_dep_rm, cmd_doctor, cmd_done,
    cmd_edit, cmd_init, cmd_ls, cmd_merge, cmd_migrate, cmd_ready, cmd_rm, cmd_search, cmd_set,
    cmd_show, cmd_skip, cmd_start, cmd_status, cmd_sync, cmd_tui,
};
use braid::config::Config;
use braid::error::{BrdError, Result};
use braid::repo;
use braid::verbose;
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
    if let Command::Init(args) = &cli.command {
        return cmd_init(cli, args);
    }
    if let Command::Completions { shell } = &cli.command {
        return cmd_completions(*shell);
    }

    // all other commands need repo discovery
    let paths = repo::discover(cli.repo.as_deref())?;

    // check if braid is initialized
    if !paths.braid_dir().exists() {
        return Err(BrdError::NotInitialized);
    }
    verbose!(cli, "found .braid at {}", paths.braid_dir().display());

    // validate config schema version early to prevent old brd from modifying upgraded repos
    let config = Config::load(&paths.config_path())?;
    config.validate(Some(&paths.worktree_root))?;
    // also validate external/worktree configs if in those modes
    paths.validate_resolved_config(&config)?;
    verbose!(
        cli,
        "config: prefix={}, id_len={}, schema=v{}",
        config.id_prefix,
        config.id_len,
        config.schema_version
    );

    match &cli.command {
        Command::Init(_) => unreachable!(),
        Command::Add(args) => cmd_add(cli, &paths, args),
        Command::Ls {
            status,
            priority,
            ready,
            blocked,
            tag,
            all,
        } => cmd_ls(
            cli,
            &paths,
            status.as_deref(),
            priority.as_deref(),
            *ready,
            *blocked,
            tag,
            *all,
        ),
        Command::Show { id, context } => cmd_show(cli, &paths, id, *context),
        Command::Edit { id } => cmd_edit(cli, &paths, id.as_deref()),
        Command::Set { id, field, value } => cmd_set(cli, &paths, id, field, value),
        Command::Ready => cmd_ready(cli, &paths),
        Command::Status => cmd_status(cli, &paths),
        Command::Dep { action } => match action {
            DepAction::Add { blocked, blocker } => cmd_dep_add(cli, &paths, blocked, blocker),
            DepAction::Rm { blocked, blocker } => cmd_dep_rm(cli, &paths, blocked, blocker),
        },
        Command::Start {
            id,
            force,
            no_sync,
            no_push,
            stash,
        } => cmd_start(cli, &paths, id.as_deref(), *force, *no_sync, *no_push, *stash),
        Command::Done {
            id,
            force,
            result,
            no_push,
        } => cmd_done(cli, &paths, id, *force, result, *no_push),
        Command::Skip { id } => cmd_skip(cli, &paths, id),
        Command::Rm { id, force } => cmd_rm(cli, &paths, id, *force),
        Command::Agent { action } => match action {
            AgentAction::Init { name, base } => cmd_agent_init(cli, &paths, name, base.as_deref()),
            AgentAction::Branch { id } => cmd_agent_branch(cli, &paths, id),
            AgentAction::Pr => cmd_agent_pr(cli, &paths),
            AgentAction::Merge => cmd_merge(cli, &paths),
            AgentAction::Inject { file } => cmd_agents_inject(&paths, file.as_deref()),
            AgentAction::Instructions => cmd_agents_show(),
        },
        Command::Doctor => cmd_doctor(cli, &paths),
        Command::Completions { .. } => unreachable!(),
        Command::Tui => cmd_tui(cli, &paths),
        Command::Migrate { dry_run } => cmd_migrate(cli, &paths, *dry_run),
        Command::Search => cmd_search(cli, &paths),
        Command::Commit { message } => cmd_commit(cli, &paths, message.as_deref()),
        Command::Sync { push } => cmd_sync(cli, &paths, *push),
        Command::Config { action } => match action {
            None => cmd_config_show(cli, &paths),
            Some(ConfigAction::IssuesBranch { name, clear, yes }) => {
                cmd_config_issues_branch(cli, &paths, name.as_deref(), *clear, *yes)
            }
            Some(ConfigAction::ExternalRepo { path, clear, yes }) => {
                cmd_config_external_repo(cli, &paths, path.as_deref(), *clear, *yes)
            }
            Some(ConfigAction::AutoSync { enabled }) => cmd_config_auto_sync(cli, &paths, *enabled),
        },
    }
}
