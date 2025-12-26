use std::collections::HashMap;
use std::path::PathBuf;

use braid::cli::{AgentAction, Cli, Command, DepAction};
use braid::config::Config;
use braid::error::{BrdError, Result};
use braid::graph::{compute_derived, get_ready_issues};
use braid::issue::{Issue, Priority, Status};
use braid::lock::LockGuard;
use braid::repo::{self, RepoPaths};
use clap::Parser;
use rand::Rng;

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
    // handle init specially - it doesn't require existing repo
    if matches!(cli.command, Command::Init) {
        return cmd_init(cli);
    }

    // all other commands need repo discovery
    let paths = repo::discover(cli.repo.as_deref())?;

    match &cli.command {
        Command::Init => unreachable!(),
        Command::Add {
            title,
            priority,
            dep,
            ac,
        } => cmd_add(cli, &paths, title, priority, dep, ac),
        Command::Ls {
            status,
            priority,
            ready,
            blocked,
        } => cmd_ls(
            cli,
            &paths,
            status.as_deref(),
            priority.as_deref(),
            *ready,
            *blocked,
        ),
        Command::Show { id } => cmd_show(cli, &paths, id),
        Command::Ready { include_claimed } => cmd_ready(cli, &paths, *include_claimed),
        Command::Next {
            claim,
            include_claimed,
        } => cmd_next(cli, &paths, *claim, *include_claimed),
        Command::Dep { action } => match action {
            DepAction::Add { child, parent } => cmd_dep_add(cli, &paths, child, parent),
            DepAction::Rm { child, parent } => cmd_dep_rm(cli, &paths, child, parent),
        },
        Command::Claim { id } => cmd_claim(cli, &paths, id),
        Command::Release { id, force } => cmd_release(cli, &paths, id, *force),
        Command::Reclaim { id, force } => cmd_reclaim(cli, &paths, id, *force),
        Command::Claims { all } => cmd_claims(cli, &paths, *all),
        Command::Start { id, force } => cmd_start(cli, &paths, id.as_deref(), *force),
        Command::Done { id, force } => cmd_done(cli, &paths, id, *force),
        Command::Agent { action } => match action {
            AgentAction::Init { name, base } => cmd_agent_init(cli, &paths, name, base.as_deref()),
        },
        Command::Doctor => cmd_doctor(cli, &paths),
        Command::Completions { shell } => cmd_completions(*shell),
    }
}

// =============================================================================
// command implementations
// =============================================================================

fn cmd_init(cli: &Cli) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // discover git info
    let worktree_root = git_rev_parse(&cwd, "--show-toplevel")?;
    let git_common_dir_str = git_rev_parse(&cwd, "--git-common-dir")?;
    let git_common_dir = if git_common_dir_str.is_absolute() {
        git_common_dir_str
    } else {
        cwd.join(&git_common_dir_str)
            .canonicalize()
            .unwrap_or(git_common_dir_str)
    };

    let braid_dir = worktree_root.join(".braid");
    let issues_dir = braid_dir.join("issues");
    let config_path = braid_dir.join("config.toml");
    let gitignore_path = braid_dir.join(".gitignore");

    let brd_common_dir = git_common_dir.join("brd");
    let claims_dir = brd_common_dir.join("claims");
    let control_root_file = brd_common_dir.join("control_root");

    // create directories
    std::fs::create_dir_all(&issues_dir)?;
    std::fs::create_dir_all(&claims_dir)?;

    // create config if missing
    if !config_path.exists() {
        let repo_name = worktree_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("brd");
        let config = Config::with_derived_prefix(repo_name);
        config.save(&config_path)?;
    }

    // create/update .gitignore
    let gitignore_content = "agent.toml\nruntime/\n";
    std::fs::write(&gitignore_path, gitignore_content)?;

    // create agent.toml if missing (with $USER as default agent_id)
    let agent_toml_path = braid_dir.join("agent.toml");
    if !agent_toml_path.exists() {
        let user = match std::env::var("USER") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("warning: $USER not set, using 'default-user' as agent_id");
                "default-user".to_string()
            }
        };
        let agent_toml_content = format!("agent_id = \"{}\"\n", user);
        std::fs::write(&agent_toml_path, agent_toml_content)?;
    }

    // set control root if not already set
    if !control_root_file.exists() {
        std::fs::write(&control_root_file, worktree_root.to_string_lossy().as_ref())?;
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "braid_dir": braid_dir.to_string_lossy(),
            "control_root": worktree_root.to_string_lossy(),
            "common_dir": brd_common_dir.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Initialized braid in {}", braid_dir.display());
        println!("  control root: {}", worktree_root.display());
        println!("  common dir:   {}", brd_common_dir.display());
    }

    Ok(())
}

fn cmd_add(
    cli: &Cli,
    paths: &RepoPaths,
    title: &str,
    priority_str: &str,
    deps: &[String],
    acceptance: &[String],
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let priority: Priority = priority_str.parse()?;

    // resolve deps to full IDs
    let all_issues = load_all_issues(paths)?;
    let resolved_deps: Vec<String> = deps
        .iter()
        .map(|d| resolve_issue_id(d, &all_issues))
        .collect::<Result<Vec<_>>>()?;

    // generate ID
    let id = generate_issue_id(&config, &paths.issues_dir())?;

    // create issue
    let mut issue = Issue::new(id.clone(), title.to_string(), priority, resolved_deps);
    issue.frontmatter.acceptance = acceptance.to_vec();

    // save with lock
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let issue_path = paths.issues_dir().join(format!("{}.md", id));
    issue.save(&issue_path)?;

    if cli.json {
        let json = issue_to_json(&issue, &all_issues, None);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created issue: {}", id);
        println!("  {}", issue_path.display());
    }

    Ok(())
}

fn cmd_ls(
    cli: &Cli,
    paths: &RepoPaths,
    status_filter: Option<&str>,
    priority_filter: Option<&str>,
    ready_only: bool,
    blocked_only: bool,
) -> Result<()> {
    let issues = load_all_issues(paths)?;

    let status_filter: Option<Status> = status_filter.map(|s| s.parse()).transpose()?;
    let priority_filter: Option<Priority> = priority_filter.map(|p| p.parse()).transpose()?;

    let mut filtered: Vec<&Issue> = issues
        .values()
        .filter(|issue| {
            if let Some(s) = status_filter {
                if issue.status() != s {
                    return false;
                }
            }
            if let Some(p) = priority_filter {
                if issue.priority() != p {
                    return false;
                }
            }
            if ready_only {
                let derived = compute_derived(issue, &issues);
                if !derived.is_ready {
                    return false;
                }
            }
            if blocked_only {
                let derived = compute_derived(issue, &issues);
                if !derived.is_blocked {
                    return false;
                }
            }
            true
        })
        .collect();

    // sort by priority, created_at, id
    filtered.sort_by(|a, b| {
        a.priority()
            .cmp(&b.priority())
            .then_with(|| a.frontmatter.created_at.cmp(&b.frontmatter.created_at))
            .then_with(|| a.id().cmp(b.id()))
    });

    if cli.json {
        let json: Vec<_> = filtered
            .iter()
            .map(|issue| issue_to_json(issue, &issues, None))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        if filtered.is_empty() {
            println!("No issues found.");
        } else {
            for issue in filtered {
                let derived = compute_derived(issue, &issues);
                let deps_info = if issue.deps().is_empty() {
                    String::new()
                } else {
                    format!(
                        " (deps:{} open:{})",
                        issue.deps().len(),
                        derived.open_deps.len()
                    )
                };
                println!(
                    "{}  {}  {}  {}{}",
                    issue.id(),
                    issue.priority(),
                    issue.status(),
                    issue.title(),
                    deps_info
                );
            }
        }
    }

    Ok(())
}

fn cmd_show(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let full_id = resolve_issue_id(id, &issues)?;
    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

    if cli.json {
        let json = issue_to_json(issue, &issues, None);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("ID:       {}", issue.id());
        println!("Title:    {}", issue.title());
        println!("Priority: {}", issue.priority());
        println!("Status:   {}", issue.status());

        if !issue.deps().is_empty() {
            println!("Deps:     {}", issue.deps().join(", "));
        }

        if let Some(owner) = &issue.frontmatter.owner {
            println!("Owner:    {}", owner);
        }

        let derived = compute_derived(issue, &issues);
        if derived.is_ready {
            println!("State:    READY");
        } else if derived.is_blocked {
            println!("State:    BLOCKED");
            if !derived.open_deps.is_empty() {
                println!("  open:   {}", derived.open_deps.join(", "));
            }
            if !derived.missing_deps.is_empty() {
                println!("  missing: {}", derived.missing_deps.join(", "));
            }
        }

        if !issue.frontmatter.acceptance.is_empty() {
            println!("\nAcceptance:");
            for ac in &issue.frontmatter.acceptance {
                println!("  - {}", ac);
            }
        }

        if !issue.body.is_empty() {
            println!("\n{}", issue.body);
        }
    }

    Ok(())
}

fn cmd_ready(cli: &Cli, paths: &RepoPaths, _include_claimed: bool) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let ready = get_ready_issues(&issues);

    // TODO: filter by claims when include_claimed is false

    if cli.json {
        let json: Vec<_> = ready
            .iter()
            .map(|issue| issue_to_json(issue, &issues, None))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        if ready.is_empty() {
            println!("No ready issues.");
        } else {
            for issue in ready {
                println!("{}  {}  {}", issue.id(), issue.priority(), issue.title());
            }
        }
    }

    Ok(())
}

fn cmd_next(cli: &Cli, paths: &RepoPaths, claim: bool, _include_claimed: bool) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let ready = get_ready_issues(&issues);

    // TODO: filter by claims

    let next_issue = ready.first();

    if claim {
        if let Some(issue) = next_issue {
            let _lock = LockGuard::acquire(&paths.lock_path())?;
            // TODO: actually create claim
            if !cli.json {
                println!("Claimed: {}", issue.id());
            }
        }
    }

    if cli.json {
        match next_issue {
            Some(issue) => {
                let json = issue_to_json(issue, &issues, None);
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
            None => println!("null"),
        }
    } else if let Some(issue) = next_issue {
        if !claim {
            println!("{}  {}  {}", issue.id(), issue.priority(), issue.title());
        }
    } else {
        println!("No ready issues.");
    }

    Ok(())
}

fn cmd_dep_add(cli: &Cli, paths: &RepoPaths, child_id: &str, parent_id: &str) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let child_full = resolve_issue_id(child_id, &issues)?;
    let parent_full = resolve_issue_id(parent_id, &issues)?;

    // check not self-dep
    if child_full == parent_full {
        return Err(BrdError::Other("cannot add self-dependency".to_string()));
    }

    let child = issues
        .get_mut(&child_full)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;

    if !child.frontmatter.deps.contains(&parent_full) {
        child.frontmatter.deps.push(parent_full.clone());
        child.touch();
        let issue_path = paths.issues_dir().join(format!("{}.md", child_full));
        child.save(&issue_path)?;
    }

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("Added dependency: {} -> {}", child_full, parent_full);
    }

    Ok(())
}

fn cmd_dep_rm(cli: &Cli, paths: &RepoPaths, child_id: &str, parent_id: &str) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let child_full = resolve_issue_id(child_id, &issues)?;
    let parent_full = resolve_issue_id(parent_id, &issues)?;

    let child = issues
        .get_mut(&child_full)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;

    child.frontmatter.deps.retain(|d| d != &parent_full);
    child.touch();
    let issue_path = paths.issues_dir().join(format!("{}.md", child_full));
    child.save(&issue_path)?;

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("Removed dependency: {} -> {}", child_full, parent_full);
    }

    Ok(())
}

fn cmd_claim(_cli: &Cli, _paths: &RepoPaths, _id: &str) -> Result<()> {
    // TODO: implement claims
    println!("claim not yet implemented");
    Ok(())
}

fn cmd_release(_cli: &Cli, _paths: &RepoPaths, _id: &str, _force: bool) -> Result<()> {
    // TODO: implement claims
    println!("release not yet implemented");
    Ok(())
}

fn cmd_reclaim(_cli: &Cli, _paths: &RepoPaths, _id: &str, _force: bool) -> Result<()> {
    // TODO: implement claims
    println!("reclaim not yet implemented");
    Ok(())
}

fn cmd_claims(_cli: &Cli, _paths: &RepoPaths, _all: bool) -> Result<()> {
    // TODO: implement claims
    println!("claims not yet implemented");
    Ok(())
}

fn cmd_start(cli: &Cli, paths: &RepoPaths, id: Option<&str>, force: bool) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;

    // resolve issue id: either from argument or pick next ready
    let full_id = match id {
        Some(partial) => resolve_issue_id(partial, &issues)?,
        None => {
            let ready = get_ready_issues(&issues);
            ready
                .first()
                .map(|i| i.id().to_string())
                .ok_or_else(|| BrdError::Other("no ready issues".to_string()))?
        }
    };

    let agent_id = braid::claims::get_agent_id(&paths.worktree_root);

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;

        // check if already being worked on
        if issue.status() == Status::Doing && !force {
            let owner = issue.frontmatter.owner.as_deref().unwrap_or("unknown");
            return Err(BrdError::Other(format!(
                "issue {} is already being worked on by '{}' (use --force to reassign)",
                full_id, owner
            )));
        }

        issue.frontmatter.status = Status::Doing;
        issue.frontmatter.owner = Some(agent_id.clone());
        issue.touch();

        let issue_path = paths.issues_dir().join(format!("{}.md", full_id));
        issue.save(&issue_path)?;

        // dual-write: also save to local worktree if different from control root
        if paths.worktree_root != paths.control_root {
            let local_issue_path = paths
                .worktree_root
                .join(".braid/issues")
                .join(format!("{}.md", full_id));
            issue.save(&local_issue_path)?;
        }
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues, None);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Started: {} (owner: {})", full_id, agent_id);
    }

    Ok(())
}

fn cmd_done(cli: &Cli, paths: &RepoPaths, id: &str, _force: bool) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let full_id = resolve_issue_id(id, &issues)?;

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        // TODO: check claim ownership unless force

        issue.frontmatter.status = Status::Done;
        issue.frontmatter.owner = None;
        issue.touch();

        let issue_path = paths.issues_dir().join(format!("{}.md", full_id));
        issue.save(&issue_path)?;

        // dual-write: also save to local worktree if different from control root
        if paths.worktree_root != paths.control_root {
            let local_issue_path = paths
                .worktree_root
                .join(".braid/issues")
                .join(format!("{}.md", full_id));
            issue.save(&local_issue_path)?;
        }
    }

    // TODO: release claim

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues, None);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Done: {}", full_id);
    }

    Ok(())
}

fn cmd_agent_init(cli: &Cli, paths: &RepoPaths, name: &str, base: Option<&str>) -> Result<()> {
    // validate agent name (alphanumeric + hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(BrdError::Other(format!(
            "invalid agent name '{}': use only alphanumeric, hyphens, underscores",
            name
        )));
    }

    // determine worktree path (~/.braid/worktrees/<repo-name>/<agent-name>)
    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| BrdError::Other("cannot determine home directory".to_string()))?;
    let repo_name = paths
        .worktree_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| BrdError::Other("cannot determine repo name".to_string()))?;
    let worktrees_dir = home_dir.join(".braid").join("worktrees").join(repo_name);
    let worktree_path = worktrees_dir.join(name);

    // ensure parent directories exist
    std::fs::create_dir_all(&worktrees_dir)?;

    // check if worktree already exists
    if worktree_path.exists() {
        return Err(BrdError::Other(format!(
            "directory already exists: {}",
            worktree_path.display()
        )));
    }

    // get base branch (default to current branch)
    let base_branch = match base {
        Some(b) => b.to_string(),
        None => {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&paths.worktree_root)
                .output()?;
            if !output.status.success() {
                return Err(BrdError::Other("failed to get current branch".to_string()));
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    // create worktree with new branch
    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            name,
            worktree_path.to_str().unwrap(),
            &base_branch,
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create worktree: {}",
            stderr.trim()
        )));
    }

    // create .braid directory in new worktree (for agent.toml)
    let new_braid_dir = worktree_path.join(".braid");
    std::fs::create_dir_all(&new_braid_dir)?;

    // create agent.toml
    let agent_toml_path = new_braid_dir.join("agent.toml");
    let agent_toml_content = format!("agent_id = \"{}\"\n", name);
    std::fs::write(&agent_toml_path, agent_toml_content)?;

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "agent_id": name,
            "worktree": worktree_path.to_string_lossy(),
            "branch": name,
            "base": base_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created agent worktree: {}", name);
        println!("  path:   {}", worktree_path.display());
        println!("  branch: {} (from {})", name, base_branch);
        println!();
        println!("To use this agent:");
        println!("  cd {}", worktree_path.display());
        println!("  brd next  # get next issue to work on");
    }

    Ok(())
}

fn cmd_doctor(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let mut errors: Vec<serde_json::Value> = Vec::new();
    let warnings: Vec<serde_json::Value> = Vec::new();

    // check .braid exists
    if !paths.braid_dir().exists() {
        errors.push(serde_json::json!({
            "code": "missing_braid_dir",
            "message": ".braid directory not found"
        }));
    }

    // load and validate all issues
    let issues = load_all_issues(paths)?;

    for (id, issue) in &issues {
        // check for missing deps
        for dep in issue.deps() {
            if !issues.contains_key(dep) {
                errors.push(serde_json::json!({
                    "code": "missing_dep",
                    "issue": id,
                    "dep": dep
                }));
            }
        }
    }

    // check for cycles
    let cycles = braid::graph::find_cycles(&issues);
    for cycle in cycles {
        errors.push(serde_json::json!({
            "code": "cycle",
            "cycle": cycle
        }));
    }

    let ok = errors.is_empty();

    if cli.json {
        let json = serde_json::json!({
            "ok": ok,
            "errors": errors,
            "warnings": warnings
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        if ok && warnings.is_empty() {
            println!("✓ All checks passed");
        } else {
            for e in &errors {
                eprintln!("error: {}", e);
            }
            for w in &warnings {
                eprintln!("warning: {}", w);
            }
        }
    }

    if ok {
        Ok(())
    } else {
        Err(BrdError::Other("doctor found errors".to_string()))
    }
}

fn cmd_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(shell, &mut Cli::command(), "brd", &mut std::io::stdout());
    Ok(())
}

// =============================================================================
// helper functions
// =============================================================================

fn git_rev_parse(cwd: &std::path::Path, arg: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg(arg)
        .current_dir(cwd)
        .output()?;

    if !output.status.success() {
        return Err(BrdError::NotGitRepo);
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path_str))
}

fn load_all_issues(paths: &RepoPaths) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    let issues_dir = paths.issues_dir();

    if !issues_dir.exists() {
        return Ok(issues);
    }

    for entry in std::fs::read_dir(&issues_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "md") {
            match Issue::load(&path) {
                Ok(issue) => {
                    issues.insert(issue.id().to_string(), issue);
                }
                Err(e) => {
                    eprintln!("warning: failed to load {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(issues)
}

fn resolve_issue_id(partial: &str, issues: &HashMap<String, Issue>) -> Result<String> {
    // exact match
    if issues.contains_key(partial) {
        return Ok(partial.to_string());
    }

    // partial match
    let matches: Vec<&str> = issues
        .keys()
        .filter(|id| id.contains(partial) || id.ends_with(partial))
        .map(|s| s.as_str())
        .collect();

    match matches.len() {
        0 => Err(BrdError::IssueNotFound(partial.to_string())),
        1 => Ok(matches[0].to_string()),
        _ => Err(BrdError::AmbiguousId(
            partial.to_string(),
            matches.into_iter().map(String::from).collect(),
        )),
    }
}

fn generate_issue_id(config: &Config, issues_dir: &std::path::Path) -> Result<String> {
    let charset: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();

    for _ in 0..20 {
        let suffix: String = (0..config.id_len)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        let id = format!("{}-{}", config.id_prefix, suffix);
        let path = issues_dir.join(format!("{}.md", id));

        if !path.exists() {
            return Ok(id);
        }
    }

    Err(BrdError::Other(
        "failed to generate unique ID after 20 attempts".to_string(),
    ))
}

fn issue_to_json(
    issue: &Issue,
    all_issues: &HashMap<String, Issue>,
    _claim: Option<&braid::claims::Claim>,
) -> serde_json::Value {
    let derived = compute_derived(issue, all_issues);

    serde_json::json!({
        "id": issue.id(),
        "title": issue.title(),
        "priority": issue.priority().to_string(),
        "status": issue.status().to_string(),
        "deps": issue.deps(),
        "owner": issue.frontmatter.owner,
        "created_at": issue.frontmatter.created_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "updated_at": issue.frontmatter.updated_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "acceptance": issue.frontmatter.acceptance,
        "derived": {
            "is_ready": derived.is_ready,
            "open_deps": derived.open_deps,
            "missing_deps": derived.missing_deps,
            "is_blocked": derived.is_blocked
        },
        "claim": {
            "state": "unclaimed",
            "agent_id": null,
            "lease_until": null
        }
    })
}
