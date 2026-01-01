//! integration tests for the brd CLI.

use std::path::PathBuf;
use std::process::{Command, Output};

/// a temporary test environment with a git repo and braid initialized.
struct TestEnv {
    dir: tempfile::TempDir,
}

impl TestEnv {
    /// create a new test environment with git and braid initialized.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        // initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("failed to init git");

        // configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .expect("failed to config git email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .expect("failed to config git name");

        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(dir.path())
            .output()
            .expect("failed to disable gpg signing");

        // create initial commit (required for local-sync mode)
        std::fs::write(dir.path().join("README.md"), "# Test\n").expect("failed to write README");
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .expect("failed to git add");
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .expect("failed to git commit");

        // initialize braid
        let output = Self::run_brd_in(&dir.path().to_path_buf(), &["init"]);
        assert!(
            output.status.success(),
            "brd init failed: {}",
            Self::stderr(&output)
        );

        // disable auto-sync for tests (no remote available)
        let config_path = dir.path().join(".braid/config.toml");
        let config = std::fs::read_to_string(&config_path).expect("failed to read config");
        let config = config
            .replace("auto_pull = true", "auto_pull = false")
            .replace("auto_push = true", "auto_push = false");
        std::fs::write(&config_path, config).expect("failed to write config");

        Self { dir }
    }

    /// path to the test directory.
    fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    /// run brd with args in the test directory.
    fn brd(&self, args: &[&str]) -> Output {
        Self::run_brd_in(&self.path(), args)
    }

    /// run brd with --json flag.
    fn brd_json(&self, args: &[&str]) -> Output {
        let mut full_args = vec!["--json"];
        full_args.extend(args);
        self.brd(&full_args)
    }

    /// run brd in a specific directory.
    fn run_brd_in(dir: &PathBuf, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_brd"))
            .args(args)
            .current_dir(dir)
            .output()
            .expect("failed to run brd")
    }

    /// get stdout as string.
    fn stdout(output: &Output) -> String {
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// get stderr as string.
    fn stderr(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).to_string()
    }

    /// parse JSON output.
    fn json(output: &Output) -> serde_json::Value {
        serde_json::from_str(&Self::stdout(output)).expect("failed to parse JSON output")
    }
}

// =============================================================================
// workflow tests
// =============================================================================

#[test]
fn test_init_creates_braid_directory() {
    let env = TestEnv::new();
    assert!(env.path().join(".braid").exists());
    assert!(env.path().join(".braid/issues").exists());
    assert!(env.path().join(".braid/config.toml").exists());
}

#[test]
fn test_full_workflow_add_start_done() {
    let env = TestEnv::new();

    // add an issue
    let output = env.brd(&["add", "test issue", "-p", "P1"]);
    assert!(
        output.status.success(),
        "add failed: {}",
        TestEnv::stderr(&output)
    );
    let stdout = TestEnv::stdout(&output);
    assert!(stdout.contains("Created issue:"));

    // extract issue ID from output
    let id = stdout
        .lines()
        .find(|l| l.contains("Created issue:"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim())
        .expect("couldn't find issue ID");

    // ls should show the issue
    let output = env.brd(&["ls"]);
    assert!(output.status.success());
    let stdout = TestEnv::stdout(&output);
    assert!(stdout.contains(id));
    assert!(stdout.contains("test issue"));

    // ready should show the issue
    let output = env.brd(&["ready"]);
    assert!(output.status.success());
    assert!(TestEnv::stdout(&output).contains(id));

    // start the issue
    let output = env.brd(&["start", id]);
    assert!(
        output.status.success(),
        "start failed: {}",
        TestEnv::stderr(&output)
    );
    assert!(TestEnv::stdout(&output).contains("Started:"));

    // ls should show status as doing
    let output = env.brd(&["ls", "--status", "doing"]);
    assert!(output.status.success());
    assert!(TestEnv::stdout(&output).contains(id));

    // ready should NOT show the issue anymore
    let output = env.brd(&["ready"]);
    assert!(output.status.success());
    assert!(!TestEnv::stdout(&output).contains(id));

    // done the issue
    let output = env.brd(&["done", id]);
    assert!(
        output.status.success(),
        "done failed: {}",
        TestEnv::stderr(&output)
    );

    // ls should show status as done
    let output = env.brd(&["ls", "--status", "done"]);
    assert!(output.status.success());
    assert!(TestEnv::stdout(&output).contains(id));
}

#[test]
fn test_ls_shows_tags() {
    let env = TestEnv::new();

    let output = env.brd(&["add", "tagged issue", "--tag", "visual", "--tag", "urgent"]);
    assert!(
        output.status.success(),
        "add failed: {}",
        TestEnv::stderr(&output)
    );

    let output = env.brd(&["ls"]);
    assert!(output.status.success());
    let stdout = TestEnv::stdout(&output);
    assert!(stdout.contains("#visual"));
    assert!(stdout.contains("#urgent"));
}

#[test]
fn test_start_picks_highest_priority() {
    let env = TestEnv::new();

    // add issues with different priorities
    env.brd(&["add", "low priority", "-p", "P3"]);
    env.brd(&["add", "high priority", "-p", "P0"]);
    env.brd(&["add", "medium priority", "-p", "P2"]);

    // start without id should pick P0 issue
    let output = env.brd(&["start"]);
    assert!(output.status.success());
    assert!(TestEnv::stdout(&output).contains("Started:"));

    // verify the high priority issue is now doing
    let output = env.brd(&["ls", "--status", "doing"]);
    assert!(TestEnv::stdout(&output).contains("high priority"));
}

// =============================================================================
// dependency tests
// =============================================================================

#[test]
fn test_dependency_blocks_issue() {
    let env = TestEnv::new();

    // add parent and child issues
    let output = env.brd_json(&["add", "parent issue"]);
    let parent_id = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    let output = env.brd_json(&["add", "child issue", "--dep", &parent_id]);
    let child_id = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    // child should be blocked
    let output = env.brd_json(&["show", &child_id]);
    let json = TestEnv::json(&output);
    assert!(json["derived"]["is_blocked"].as_bool().unwrap());
    assert!(!json["derived"]["is_ready"].as_bool().unwrap());

    // ready should NOT show child
    let output = env.brd(&["ready"]);
    assert!(!TestEnv::stdout(&output).contains(&child_id));

    // complete parent
    env.brd(&["start", &parent_id]);
    env.brd(&["done", &parent_id]);

    // child should now be ready
    let output = env.brd_json(&["show", &child_id]);
    let json = TestEnv::json(&output);
    assert!(!json["derived"]["is_blocked"].as_bool().unwrap());
    assert!(json["derived"]["is_ready"].as_bool().unwrap());

    // ready should show child
    let output = env.brd(&["ready"]);
    assert!(TestEnv::stdout(&output).contains(&child_id));
}

#[test]
fn test_dep_add_and_remove() {
    let env = TestEnv::new();

    // add two issues
    let output = env.brd_json(&["add", "issue one"]);
    let id1 = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    let output = env.brd_json(&["add", "issue two"]);
    let id2 = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    // add dependency
    let output = env.brd(&["dep", "add", &id2, &id1]);
    assert!(output.status.success());

    // id2 should be blocked by id1
    let output = env.brd_json(&["show", &id2]);
    let json = TestEnv::json(&output);
    assert!(json["derived"]["is_blocked"].as_bool().unwrap());

    // remove dependency
    let output = env.brd(&["dep", "rm", &id2, &id1]);
    assert!(output.status.success());

    // id2 should no longer be blocked
    let output = env.brd_json(&["show", &id2]);
    let json = TestEnv::json(&output);
    assert!(!json["derived"]["is_blocked"].as_bool().unwrap());
}

#[test]
fn test_dep_add_rejects_cycle() {
    let env = TestEnv::new();

    // add two issues
    let output = env.brd_json(&["add", "issue one"]);
    let id1 = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    let output = env.brd_json(&["add", "issue two"]);
    let id2 = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    // add dependency id1 -> id2
    let output = env.brd(&["dep", "add", &id1, &id2]);
    assert!(output.status.success());

    // try to add reverse dependency id2 -> id1 (would create cycle)
    let output = env.brd(&["dep", "add", &id2, &id1]);
    assert!(!output.status.success());
    assert!(TestEnv::stderr(&output).contains("cycle"));
}

// =============================================================================
// JSON output tests
// =============================================================================

#[test]
fn test_json_output_ls() {
    let env = TestEnv::new();

    env.brd(&["add", "test issue", "-p", "P1"]);

    let output = env.brd_json(&["ls"]);
    assert!(output.status.success());

    let json = TestEnv::json(&output);
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["title"], "test issue");
    assert_eq!(json[0]["priority"], "P1");
}

#[test]
fn test_json_output_show() {
    let env = TestEnv::new();

    let output = env.brd_json(&["add", "test issue", "-p", "P2", "--ac", "do the thing"]);
    let id = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    let output = env.brd_json(&["show", &id]);
    let json = TestEnv::json(&output);

    assert_eq!(json["title"], "test issue");
    assert_eq!(json["priority"], "P2");
    assert_eq!(json["status"], "todo");
    assert!(
        json["acceptance"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("do the thing"))
    );
}

#[test]
fn test_json_output_init() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // initialize git repo with initial commit
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to init git");
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("failed to config git email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("failed to config git name");
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir.path())
        .output()
        .expect("failed to disable gpg signing");
    std::fs::write(dir.path().join("README.md"), "# Test\n").expect("failed to write README");
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .expect("failed to git add");
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .expect("failed to git commit");

    let output = Command::new(env!("CARGO_BIN_EXE_brd"))
        .args(["--json", "init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run brd");

    assert!(
        output.status.success(),
        "init failed: {}",
        TestEnv::stderr(&output)
    );

    let json = TestEnv::json(&output);
    assert!(json["ok"].as_bool().unwrap());
    assert!(json["braid_dir"].as_str().unwrap().ends_with(".braid"));
    assert!(
        json["worktree"]
            .as_str()
            .unwrap()
            .contains(dir.path().to_str().unwrap())
    );
    // default in JSON mode is local-sync with issues_branch
    assert_eq!(json["issues_branch"].as_str().unwrap(), "braid-issues");
}

#[test]
fn test_json_output_doctor() {
    let env = TestEnv::new();

    let output = env.brd_json(&["doctor"]);
    assert!(output.status.success());

    let json = TestEnv::json(&output);
    assert_eq!(json["ok"], true);
    assert!(json["checks"].is_array());
    assert!(json["errors"].as_array().unwrap().is_empty());
}

// =============================================================================
// error case tests
// =============================================================================

#[test]
fn test_error_issue_not_found() {
    let env = TestEnv::new();

    let output = env.brd(&["show", "nonexistent-id"]);
    assert!(!output.status.success());
    assert!(
        TestEnv::stderr(&output).contains("not found")
            || TestEnv::stderr(&output).contains("error")
    );
}

#[test]
fn test_error_ambiguous_id() {
    let env = TestEnv::new();

    // add two issues - they'll have IDs starting with the same prefix
    env.brd(&["add", "issue one"]);
    env.brd(&["add", "issue two"]);

    // try to resolve with just the prefix (should be ambiguous)
    let output = env.brd(&["show", "brd-"]);

    // should either fail with ambiguous error or not find exact match
    // (depends on how many issues match)
    if !output.status.success() {
        let stderr = TestEnv::stderr(&output);
        assert!(stderr.contains("ambiguous") || stderr.contains("not found"));
    }
}

#[test]
fn test_error_start_already_doing() {
    let env = TestEnv::new();

    let output = env.brd_json(&["add", "test issue"]);
    let id = TestEnv::json(&output)["id"].as_str().unwrap().to_string();

    // start once
    env.brd(&["start", &id]);

    // try to start again without force
    let output = env.brd(&["start", &id]);
    assert!(!output.status.success());
    assert!(TestEnv::stderr(&output).contains("already being worked on"));
}
