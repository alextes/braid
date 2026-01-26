//! Git command execution helpers.
//!
//! Provides a unified interface for running git commands across the codebase.

use std::path::Path;
use std::process::{Command, Output};

use crate::error::{BrdError, Result};

/// Summary of diff statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffStat {
    /// number of files changed
    pub files_changed: usize,
    /// total lines inserted
    pub insertions: usize,
    /// total lines deleted
    pub deletions: usize,
}

/// File change status in a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Unknown,
}

impl FileStatus {
    fn from_char(c: char) -> Self {
        match c {
            'A' => Self::Added,
            'M' => Self::Modified,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            _ => Self::Unknown,
        }
    }
}

/// Per-file diff statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// file path (new path for renames)
    pub path: String,
    /// old path (for renames/copies)
    pub old_path: Option<String>,
    /// change status
    pub status: FileStatus,
    /// lines inserted
    pub insertions: usize,
    /// lines deleted
    pub deletions: usize,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// context line (unchanged)
    Context(String),
    /// added line
    Add(String),
    /// removed line
    Remove(String),
}

/// A hunk in a unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// starting line in old file
    pub old_start: u32,
    /// number of lines from old file
    pub old_count: u32,
    /// starting line in new file
    pub new_start: u32,
    /// number of lines in new file
    pub new_count: u32,
    /// the actual diff lines
    pub lines: Vec<DiffLine>,
}

/// Parsed diff content for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDiff {
    /// file path
    pub path: String,
    /// hunks in the diff
    pub hunks: Vec<DiffHunk>,
}

/// Run a git command and return whether it succeeded.
pub fn run(args: &[&str], cwd: &Path) -> Result<bool> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output.status.success())
}

/// Run a git command and return stdout as a string.
pub fn output(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a git command and return the full output.
pub fn run_full(args: &[&str], cwd: &Path) -> Result<Output> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output)
}

/// Check if the working tree is clean (no uncommitted changes).
pub fn is_clean(cwd: &Path) -> Result<bool> {
    let out = output(&["status", "--porcelain"], cwd)?;
    Ok(out.is_empty())
}

/// Get the current branch name.
pub fn current_branch(cwd: &Path) -> Result<String> {
    let branch = output(&["rev-parse", "--abbrev-ref", "HEAD"], cwd)?;
    if branch.is_empty() {
        return Err(BrdError::Other("failed to get current branch".to_string()));
    }
    Ok(branch)
}

/// Check if a remote exists.
pub fn has_remote(cwd: &Path, name: &str) -> bool {
    run(&["remote", "get-url", name], cwd).unwrap_or(false)
}

/// Check if a remote branch exists.
pub fn has_remote_branch(cwd: &Path, remote: &str, branch: &str) -> bool {
    let refspec = format!("{}/{}", remote, branch);
    run(&["rev-parse", "--verify", &refspec], cwd).unwrap_or(false)
}

/// Run git rev-parse with the given arguments.
/// Arguments are split on whitespace to support multi-arg calls like "--abbrev-ref HEAD".
pub fn rev_parse(cwd: &Path, args: &str) -> Result<String> {
    let mut cmd_args = vec!["rev-parse"];
    cmd_args.extend(args.split_whitespace());

    let out = Command::new("git")
        .args(&cmd_args)
        .current_dir(cwd)
        .output()?;

    if !out.status.success() {
        return Err(BrdError::NotGitRepo);
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Check if a branch exists.
pub fn branch_exists(cwd: &Path, branch: &str) -> bool {
    run(&["rev-parse", "--verify", branch], cwd).unwrap_or(false)
}

/// Count entries in the git stash.
pub fn stash_count(cwd: &Path) -> Result<usize> {
    let out = output(&["stash", "list"], cwd)?;
    if out.is_empty() {
        Ok(0)
    } else {
        Ok(out.lines().count())
    }
}

/// Stash changes with a message. Returns true if a stash was created.
pub fn stash_push(cwd: &Path, message: &str) -> Result<bool> {
    let before = stash_count(cwd)?;
    if !run(
        &["stash", "push", "--include-untracked", "-m", message],
        cwd,
    )? {
        return Err(BrdError::Other("failed to stash changes".to_string()));
    }
    let after = stash_count(cwd)?;
    Ok(after > before)
}

/// Pop the most recent stash. Returns true if successful.
pub fn stash_pop(cwd: &Path) -> Result<bool> {
    run(&["stash", "pop"], cwd)
}

/// Get diff statistics between two refs, or for uncommitted changes.
///
/// # arguments
/// * `cwd` - working directory
/// * `base` - base ref (e.g., "main"), or None for uncommitted changes
/// * `head` - head ref (e.g., "HEAD"), or None for uncommitted changes
///
/// # examples
/// ```ignore
/// // uncommitted changes
/// diff_stat(cwd, None, None)?;
/// // branch diff
/// diff_stat(cwd, Some("main"), Some("HEAD"))?;
/// ```
pub fn diff_stat(cwd: &Path, base: Option<&str>, head: Option<&str>) -> Result<DiffStat> {
    let files = diff_files(cwd, base, head)?;

    let (insertions, deletions) = files.iter().fold((0, 0), |(ins, del), f| {
        (ins + f.insertions, del + f.deletions)
    });

    Ok(DiffStat {
        files_changed: files.len(),
        insertions,
        deletions,
    })
}

/// Get per-file diff statistics between two refs, or for uncommitted changes.
///
/// # arguments
/// * `cwd` - working directory
/// * `base` - base ref (e.g., "main"), or None for uncommitted changes
/// * `head` - head ref (e.g., "HEAD"), or None for uncommitted changes
pub fn diff_files(cwd: &Path, base: Option<&str>, head: Option<&str>) -> Result<Vec<FileDiff>> {
    // use --numstat for machine-readable output: insertions deletions path
    // use --find-renames to detect renames
    let mut args = vec!["diff", "--numstat", "--find-renames"];

    let range;
    match (base, head) {
        (Some(b), Some(h)) => {
            range = format!("{}..{}", b, h);
            args.push(&range);
        }
        (Some(b), None) => {
            args.push(b);
        }
        (None, Some(h)) => {
            args.push(h);
        }
        (None, None) => {
            // uncommitted changes - no additional args needed
        }
    }

    let numstat_output = output(&args, cwd)?;

    // also get status info for file status (A/M/D/R)
    args[1] = "--name-status";
    let status_output = output(&args, cwd)?;

    parse_diff_output(&numstat_output, &status_output)
}

/// Parse git diff --numstat and --name-status output into FileDiff structs.
fn parse_diff_output(numstat: &str, name_status: &str) -> Result<Vec<FileDiff>> {
    let mut files = Vec::new();

    // build a map of path -> status from name-status output
    let mut status_map: std::collections::HashMap<String, (FileStatus, Option<String>)> =
        std::collections::HashMap::new();

    for line in name_status.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }

        let status_str = parts[0];
        let status_char = status_str.chars().next().unwrap_or('?');
        let status = FileStatus::from_char(status_char);

        match parts.len() {
            2 => {
                // normal: status\tpath
                status_map.insert(parts[1].to_string(), (status, None));
            }
            3 => {
                // rename/copy: R100\told_path\tnew_path
                status_map.insert(parts[2].to_string(), (status, Some(parts[1].to_string())));
            }
            _ => {}
        }
    }

    // parse numstat output: insertions\tdeletions\tpath
    for line in numstat.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }

        // binary files show "-" for insertions/deletions
        let insertions = parts[0].parse().unwrap_or(0);
        let deletions = parts[1].parse().unwrap_or(0);

        // for renames, numstat shows: ins\tdel\told_path => new_path
        // or with -z: ins\tdel\told_path\0new_path
        let path_part = parts[2..].join("\t");
        let (path, old_path) = if path_part.contains(" => ") {
            // rename format: old => new or {prefix/}old => new{/suffix}
            let arrow_parts: Vec<&str> = path_part.split(" => ").collect();
            if arrow_parts.len() == 2 {
                (arrow_parts[1].to_string(), Some(arrow_parts[0].to_string()))
            } else {
                (path_part.clone(), None)
            }
        } else {
            (path_part.clone(), None)
        };

        // look up status, default to Modified if not found
        let (status, status_old_path) = status_map
            .get(&path)
            .cloned()
            .unwrap_or((FileStatus::Modified, None));

        files.push(FileDiff {
            path,
            old_path: old_path.or(status_old_path),
            status,
            insertions,
            deletions,
        });
    }

    Ok(files)
}

/// Get raw diff content for a specific file.
///
/// # arguments
/// * `cwd` - working directory
/// * `base` - base ref (e.g., "main"), or None for uncommitted changes
/// * `head` - head ref (e.g., "HEAD"), or None for uncommitted changes
/// * `file` - optional file path to limit diff to
pub fn diff_content(
    cwd: &Path,
    base: Option<&str>,
    head: Option<&str>,
    file: Option<&str>,
) -> Result<String> {
    let mut args = vec!["diff", "-U3"]; // unified format with 3 lines of context

    let range;
    match (base, head) {
        (Some(b), Some(h)) => {
            range = format!("{}..{}", b, h);
            args.push(&range);
        }
        (Some(b), None) => {
            args.push(b);
        }
        (None, Some(h)) => {
            args.push(h);
        }
        (None, None) => {
            // uncommitted changes - no additional args needed
        }
    }

    // add -- separator before file path
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    output(&args, cwd)
}

/// Parse raw unified diff content into structured form.
pub fn parse_diff(diff: &str) -> Vec<ParsedDiff> {
    let mut results = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_hunks: Vec<DiffHunk> = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;

    for line in diff.lines() {
        // new file header: diff --git a/path b/path
        if line.starts_with("diff --git ") {
            // save previous file if any
            if let Some(path) = current_path.take() {
                if let Some(hunk) = current_hunk.take() {
                    current_hunks.push(hunk);
                }
                if !current_hunks.is_empty() {
                    results.push(ParsedDiff {
                        path,
                        hunks: std::mem::take(&mut current_hunks),
                    });
                }
            }

            // extract path from "diff --git a/path b/path"
            // take the b/path part (after last space that starts with b/)
            if let Some(b_path) = line.rsplit_once(" b/") {
                current_path = Some(b_path.1.to_string());
            }
            continue;
        }

        // skip other header lines
        if line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("similarity")
            || line.starts_with("rename ")
            || line.starts_with("old mode")
            || line.starts_with("new mode")
        {
            continue;
        }

        // hunk header: @@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@") {
            // save previous hunk
            if let Some(hunk) = current_hunk.take() {
                current_hunks.push(hunk);
            }

            // parse hunk header
            if let Some(hunk) = parse_hunk_header(line) {
                current_hunk = Some(hunk);
            }
            continue;
        }

        // diff lines (only if we have an active hunk)
        if let Some(ref mut hunk) = current_hunk {
            if let Some(rest) = line.strip_prefix('+') {
                hunk.lines.push(DiffLine::Add(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix('-') {
                hunk.lines.push(DiffLine::Remove(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine::Context(rest.to_string()));
            } else if line.is_empty() {
                // empty context line
                hunk.lines.push(DiffLine::Context(String::new()));
            }
            // ignore "\ No newline at end of file" and other metadata
        }
    }

    // save final file/hunk
    if let Some(path) = current_path {
        if let Some(hunk) = current_hunk {
            current_hunks.push(hunk);
        }
        if !current_hunks.is_empty() {
            results.push(ParsedDiff {
                path,
                hunks: current_hunks,
            });
        }
    }

    results
}

/// Parse a hunk header line like "@@ -1,5 +1,6 @@ optional context"
fn parse_hunk_header(line: &str) -> Option<DiffHunk> {
    // format: @@ -old_start[,old_count] +new_start[,new_count] @@
    let line = line.strip_prefix("@@ ")?;
    let end = line.find(" @@")?;
    let range_part = &line[..end];

    let mut parts = range_part.split_whitespace();
    let old_part = parts.next()?.strip_prefix('-')?;
    let new_part = parts.next()?.strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old_part);
    let (new_start, new_count) = parse_range(new_part);

    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

/// Parse a range like "1,5" or "1" into (start, count)
fn parse_range(s: &str) -> (u32, u32) {
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}

/// Test helpers that panic on failure (for use in tests only).
#[cfg(test)]
pub mod test {
    use super::*;

    /// Run a git command, panicking if it fails.
    pub fn run_ok(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Run a git command and return stdout, panicking if it fails.
    pub fn output(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        test::run_ok(dir.path(), &["init"]);
        test::run_ok(dir.path(), &["config", "user.email", "test@test.com"]);
        test::run_ok(dir.path(), &["config", "user.name", "test"]);
        test::run_ok(dir.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.path().join("README.md"), "test\n").unwrap();
        test::run_ok(dir.path(), &["add", "."]);
        test::run_ok(dir.path(), &["commit", "-m", "init"]);
        dir
    }

    #[test]
    fn test_run_success() {
        let dir = create_test_repo();
        assert!(run(&["status"], dir.path()).unwrap());
    }

    #[test]
    fn test_run_failure() {
        let dir = create_test_repo();
        assert!(!run(&["checkout", "nonexistent"], dir.path()).unwrap());
    }

    #[test]
    fn test_output() {
        let dir = create_test_repo();
        let out = output(&["rev-parse", "--abbrev-ref", "HEAD"], dir.path()).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn test_is_clean() {
        let dir = create_test_repo();
        assert!(is_clean(dir.path()).unwrap());

        std::fs::write(dir.path().join("dirty.txt"), "dirty").unwrap();
        assert!(!is_clean(dir.path()).unwrap());
    }

    #[test]
    fn test_current_branch() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_has_remote() {
        let dir = create_test_repo();
        assert!(!has_remote(dir.path(), "origin"));
    }

    #[test]
    fn test_branch_exists() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch_exists(dir.path(), &branch));
        assert!(!branch_exists(dir.path(), "nonexistent-branch"));
    }

    #[test]
    fn test_rev_parse() {
        let dir = create_test_repo();
        let branch = rev_parse(dir.path(), "--abbrev-ref HEAD").unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_rev_parse_not_repo() {
        let dir = tempdir().unwrap();
        let err = rev_parse(dir.path(), "--show-toplevel").unwrap_err();
        assert!(matches!(err, BrdError::NotGitRepo));
    }

    #[test]
    fn test_stash_count_empty() {
        let dir = create_test_repo();
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_push_creates_stash() {
        let dir = create_test_repo();

        // Create uncommitted changes
        std::fs::write(dir.path().join("new_file.txt"), "content").unwrap();

        // Stash should succeed and return true (stash created)
        let created = stash_push(dir.path(), "test stash").unwrap();
        assert!(created);

        // Working tree should be clean now
        assert!(is_clean(dir.path()).unwrap());

        // Stash count should be 1
        assert_eq!(stash_count(dir.path()).unwrap(), 1);
    }

    #[test]
    fn test_stash_push_clean_tree_returns_false() {
        let dir = create_test_repo();

        // No changes to stash - should return false
        let created = stash_push(dir.path(), "empty stash").unwrap();
        assert!(!created);

        // Stash count should still be 0
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_pop_restores_changes() {
        let dir = create_test_repo();

        // Create and stash changes
        let file_path = dir.path().join("new_file.txt");
        std::fs::write(&file_path, "content").unwrap();
        stash_push(dir.path(), "test stash").unwrap();

        // File should be gone
        assert!(!file_path.exists());

        // Pop should succeed
        let success = stash_pop(dir.path()).unwrap();
        assert!(success);

        // File should be restored
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "content");

        // Stash count should be 0
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_pop_empty_stash_returns_false() {
        let dir = create_test_repo();

        // Pop with no stash should return false
        let success = stash_pop(dir.path()).unwrap();
        assert!(!success);
    }

    #[test]
    fn test_diff_stat_uncommitted_changes() {
        let dir = create_test_repo();

        // create uncommitted changes (must be staged to show in diff)
        std::fs::write(dir.path().join("new_file.txt"), "line1\nline2\nline3\n").unwrap();
        std::fs::write(dir.path().join("README.md"), "modified\n").unwrap();
        test::run_ok(dir.path(), &["add", "."]);

        // diff HEAD shows staged changes
        let stat = diff_stat(dir.path(), Some("HEAD"), None).unwrap();

        assert_eq!(stat.files_changed, 2);
        // new_file: 3 insertions, README: 1 insertion 1 deletion
        assert_eq!(stat.insertions, 4);
        assert_eq!(stat.deletions, 1);
    }

    #[test]
    fn test_diff_stat_branch_diff() {
        let dir = create_test_repo();

        // create a branch with changes
        test::run_ok(dir.path(), &["checkout", "-b", "feature"]);
        std::fs::write(dir.path().join("feature.txt"), "new feature\n").unwrap();
        test::run_ok(dir.path(), &["add", "feature.txt"]);
        test::run_ok(dir.path(), &["commit", "-m", "add feature"]);

        // get main branch name
        test::run_ok(dir.path(), &["checkout", "-"]);
        let main = current_branch(dir.path()).unwrap();
        test::run_ok(dir.path(), &["checkout", "feature"]);

        let stat = diff_stat(dir.path(), Some(&main), Some("HEAD")).unwrap();

        assert_eq!(stat.files_changed, 1);
        assert_eq!(stat.insertions, 1);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn test_diff_stat_clean_tree() {
        let dir = create_test_repo();

        let stat = diff_stat(dir.path(), None, None).unwrap();

        assert_eq!(stat.files_changed, 0);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn test_diff_files_uncommitted() {
        let dir = create_test_repo();

        // create new file
        std::fs::write(dir.path().join("added.txt"), "new\n").unwrap();
        // modify existing
        std::fs::write(dir.path().join("README.md"), "changed\n").unwrap();

        // stage all changes to show in diff
        test::run_ok(dir.path(), &["add", "."]);

        // diff HEAD shows staged changes
        let files = diff_files(dir.path(), Some("HEAD"), None).unwrap();

        assert_eq!(files.len(), 2);

        // find each file
        let added = files.iter().find(|f| f.path == "added.txt");
        let modified = files.iter().find(|f| f.path == "README.md");

        assert!(added.is_some());
        assert!(modified.is_some());

        let added = added.unwrap();
        assert_eq!(added.insertions, 1);
        assert_eq!(added.deletions, 0);

        let modified = modified.unwrap();
        assert_eq!(modified.insertions, 1);
        assert_eq!(modified.deletions, 1);
    }

    #[test]
    fn test_diff_files_with_deletion() {
        let dir = create_test_repo();

        // delete the README
        std::fs::remove_file(dir.path().join("README.md")).unwrap();

        let files = diff_files(dir.path(), None, None).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "README.md");
        assert_eq!(files[0].status, FileStatus::Deleted);
        assert_eq!(files[0].insertions, 0);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn test_diff_files_branch_diff() {
        let dir = create_test_repo();

        // create feature branch with multiple changes
        test::run_ok(dir.path(), &["checkout", "-b", "feature"]);

        std::fs::write(dir.path().join("a.txt"), "aaa\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb\nbbb\n").unwrap();
        test::run_ok(dir.path(), &["add", "."]);
        test::run_ok(dir.path(), &["commit", "-m", "add files"]);

        // get main branch name
        test::run_ok(dir.path(), &["checkout", "-"]);
        let main = current_branch(dir.path()).unwrap();
        test::run_ok(dir.path(), &["checkout", "feature"]);

        let files = diff_files(dir.path(), Some(&main), Some("HEAD")).unwrap();

        assert_eq!(files.len(), 2);

        let a = files.iter().find(|f| f.path == "a.txt").unwrap();
        assert_eq!(a.insertions, 1);

        let b = files.iter().find(|f| f.path == "b.txt").unwrap();
        assert_eq!(b.insertions, 2);
    }

    #[test]
    fn test_parse_diff_output_empty() {
        let files = parse_diff_output("", "").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_output_simple() {
        let numstat = "3\t1\tfile.txt\n";
        let name_status = "M\tfile.txt\n";

        let files = parse_diff_output(numstat, name_status).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "file.txt");
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].insertions, 3);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn test_parse_diff_output_binary_file() {
        // binary files show "-" for stats
        let numstat = "-\t-\timage.png\n";
        let name_status = "A\timage.png\n";

        let files = parse_diff_output(numstat, name_status).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "image.png");
        assert_eq!(files[0].status, FileStatus::Added);
        assert_eq!(files[0].insertions, 0);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn test_file_status_from_char() {
        assert_eq!(FileStatus::from_char('A'), FileStatus::Added);
        assert_eq!(FileStatus::from_char('M'), FileStatus::Modified);
        assert_eq!(FileStatus::from_char('D'), FileStatus::Deleted);
        assert_eq!(FileStatus::from_char('R'), FileStatus::Renamed);
        assert_eq!(FileStatus::from_char('C'), FileStatus::Copied);
        assert_eq!(FileStatus::from_char('?'), FileStatus::Unknown);
    }

    #[test]
    fn test_diff_content_uncommitted() {
        let dir = create_test_repo();

        // modify a file
        std::fs::write(dir.path().join("README.md"), "changed content\n").unwrap();

        let content = diff_content(dir.path(), None, None, None).unwrap();

        assert!(content.contains("diff --git"));
        assert!(content.contains("-test"));
        assert!(content.contains("+changed content"));
    }

    #[test]
    fn test_diff_content_specific_file() {
        let dir = create_test_repo();

        // create multiple changes
        std::fs::write(dir.path().join("README.md"), "changed\n").unwrap();
        std::fs::write(dir.path().join("other.txt"), "other\n").unwrap();
        test::run_ok(dir.path(), &["add", "other.txt"]);

        // get diff for just README
        let content = diff_content(dir.path(), None, None, Some("README.md")).unwrap();

        assert!(content.contains("README.md"));
        assert!(!content.contains("other.txt"));
    }

    #[test]
    fn test_diff_content_branch_diff() {
        let dir = create_test_repo();

        // create feature branch with changes
        test::run_ok(dir.path(), &["checkout", "-b", "feature"]);
        std::fs::write(dir.path().join("feature.txt"), "new feature\n").unwrap();
        test::run_ok(dir.path(), &["add", "feature.txt"]);
        test::run_ok(dir.path(), &["commit", "-m", "add feature"]);

        // get main branch name
        test::run_ok(dir.path(), &["checkout", "-"]);
        let main = current_branch(dir.path()).unwrap();
        test::run_ok(dir.path(), &["checkout", "feature"]);

        let content = diff_content(dir.path(), Some(&main), Some("HEAD"), None).unwrap();

        assert!(content.contains("diff --git"));
        assert!(content.contains("feature.txt"));
        assert!(content.contains("+new feature"));
    }

    #[test]
    fn test_parse_diff_empty() {
        let diffs = parse_diff("");
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_parse_diff_simple() {
        let diff = r#"diff --git a/file.txt b/file.txt
index abc123..def456 100644
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,4 @@
 line1
-old line
+new line
+added line
 line3
"#;

        let diffs = parse_diff(diff);

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "file.txt");
        assert_eq!(diffs[0].hunks.len(), 1);

        let hunk = &diffs[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);

        assert_eq!(hunk.lines.len(), 5);
        assert_eq!(hunk.lines[0], DiffLine::Context("line1".to_string()));
        assert_eq!(hunk.lines[1], DiffLine::Remove("old line".to_string()));
        assert_eq!(hunk.lines[2], DiffLine::Add("new line".to_string()));
        assert_eq!(hunk.lines[3], DiffLine::Add("added line".to_string()));
        assert_eq!(hunk.lines[4], DiffLine::Context("line3".to_string()));
    }

    #[test]
    fn test_parse_diff_multiple_hunks() {
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
-old1
+new1
@@ -10,2 +10,2 @@
-old2
+new2
"#;

        let diffs = parse_diff(diff);

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].hunks.len(), 2);

        assert_eq!(diffs[0].hunks[0].old_start, 1);
        assert_eq!(diffs[0].hunks[1].old_start, 10);
    }

    #[test]
    fn test_parse_diff_multiple_files() {
        let diff = r#"diff --git a/file1.txt b/file1.txt
--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-old
+new
diff --git a/file2.txt b/file2.txt
--- a/file2.txt
+++ b/file2.txt
@@ -1 +1 @@
-foo
+bar
"#;

        let diffs = parse_diff(diff);

        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].path, "file1.txt");
        assert_eq!(diffs[1].path, "file2.txt");
    }

    #[test]
    fn test_parse_hunk_header_with_counts() {
        let hunk = parse_hunk_header("@@ -10,5 +20,8 @@ some context").unwrap();
        assert_eq!(hunk.old_start, 10);
        assert_eq!(hunk.old_count, 5);
        assert_eq!(hunk.new_start, 20);
        assert_eq!(hunk.new_count, 8);
    }

    #[test]
    fn test_parse_hunk_header_without_counts() {
        // single line changes don't have counts
        let hunk = parse_hunk_header("@@ -1 +1 @@").unwrap();
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 1);
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("10,5"), (10, 5));
        assert_eq!(parse_range("1"), (1, 1));
        assert_eq!(parse_range("42,100"), (42, 100));
    }
}
