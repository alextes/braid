//! integration tests for the brd ship command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

struct ShipEnv {
    repo: TempDir,
    remote: TempDir,
}

impl ShipEnv {
    fn new() -> Self {
        let repo = tempfile::tempdir().expect("failed to create temp repo");
        let remote = tempfile::tempdir().expect("failed to create temp remote");
        let env = Self { repo, remote };
        env.init_repo();
        env
    }

    fn path(&self) -> PathBuf {
        self.repo.path().to_path_buf()
    }

    fn remote_path(&self) -> PathBuf {
        self.remote.path().to_path_buf()
    }

    fn git(&self, args: &[&str]) -> Output {
        run_git_in(&self.path(), args)
    }

    fn git_ok(&self, args: &[&str]) -> Output {
        let output = self.git(args);
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            stderr(&output)
        );
        output
    }

    fn git_stdout(&self, args: &[&str]) -> String {
        let output = self.git_ok(args);
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn git_remote_stdout(&self, args: &[&str]) -> String {
        let output = run_git_in_bare(&self.remote_path(), args);
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            stderr(&output)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn brd(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_brd"))
            .args(args)
            .current_dir(self.path())
            .output()
            .expect("failed to run brd")
    }

    fn write_file(&self, path: &str, contents: &str) {
        fs::write(self.path().join(path), contents).expect("failed to write file");
    }

    fn commit_all(&self, message: &str) {
        self.git_ok(&["add", "-A"]);
        self.git_ok(&["commit", "-m", message]);
    }

    fn init_repo(&self) {
        self.git_ok(&["init"]);
        self.git_ok(&["config", "user.email", "test@test.com"]);
        self.git_ok(&["config", "user.name", "test user"]);
        self.git_ok(&["checkout", "-b", "main"]);

        self.write_file("README.md", "test repo\n");
        self.commit_all("init");

        let remote_init = run_git_in(&self.remote_path(), &["init", "--bare"]);
        assert!(
            remote_init.status.success(),
            "git init --bare failed: {}",
            stderr(&remote_init)
        );

        let remote = self.remote_path();
        let remote_str = remote.to_str().expect("remote path is not utf-8");
        self.git_ok(&["remote", "add", "origin", remote_str]);
        self.git_ok(&["push", "-u", "origin", "main"]);

        let output = self.brd(&["init"]);
        assert!(
            output.status.success(),
            "brd init failed: {}",
            stderr(&output)
        );
        self.git_ok(&["add", ".braid"]);
        self.git_ok(&["commit", "-m", "init braid"]);
        self.git_ok(&["push", "origin", "main"]);
    }

    fn reject_remote_pushes(&self) {
        let hook_path = self.remote_path().join("hooks").join("pre-receive");
        let script = "#!/bin/sh\necho \"rejected\" 1>&2\nexit 1\n";
        fs::write(&hook_path, script).expect("failed to write hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)
                .expect("failed to read hook metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms).expect("failed to set hook perms");
        }
    }
}

fn run_git_in(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git")
}

fn run_git_in_bare(git_dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .output()
        .expect("failed to run git")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn test_ship_success_flow() {
    let env = ShipEnv::new();

    env.git_ok(&["checkout", "-b", "agent-test"]);
    env.write_file("work.txt", "agent work\n");
    env.commit_all("agent work");

    let output = env.brd(&["agent", "ship"]);
    assert!(output.status.success(), "ship failed: {}", stderr(&output));

    let branch = env.git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(branch, "agent-test");

    let head = env.git_stdout(&["rev-parse", "HEAD"]);
    let origin_main = env.git_stdout(&["rev-parse", "origin/main"]);
    assert_eq!(head, origin_main);

    let remote_main = env.git_remote_stdout(&["rev-parse", "main"]);
    assert_eq!(origin_main, remote_main);
}

#[test]
fn test_ship_dirty_worktree() {
    let env = ShipEnv::new();

    env.git_ok(&["checkout", "-b", "agent-test"]);
    env.write_file("dirty.txt", "dirty\n");

    let output = env.brd(&["agent", "ship"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("working tree is dirty"));
}

#[test]
fn test_ship_rebase_conflict() {
    let env = ShipEnv::new();

    env.write_file("conflict.txt", "base\n");
    env.commit_all("add conflict base");
    env.git_ok(&["push", "origin", "main"]);

    env.git_ok(&["checkout", "-b", "agent-test"]);
    env.write_file("conflict.txt", "agent\n");
    env.commit_all("agent change");

    env.git_ok(&["checkout", "main"]);
    env.write_file("conflict.txt", "main\n");
    env.commit_all("main change");
    env.git_ok(&["push", "origin", "main"]);

    env.git_ok(&["checkout", "agent-test"]);
    let output = env.brd(&["agent", "ship"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("rebase failed - resolve conflicts manually"));
}

#[test]
fn test_ship_push_rejected() {
    let env = ShipEnv::new();

    env.reject_remote_pushes();

    env.git_ok(&["checkout", "-b", "agent-test"]);
    env.write_file("work.txt", "agent work\n");
    env.commit_all("agent work");

    let output = env.brd(&["agent", "ship"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("push rejected"));
}
