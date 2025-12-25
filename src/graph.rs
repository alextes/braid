//! dependency graph operations: ready computation, cycle detection.

use std::collections::{HashMap, HashSet};

use crate::issue::{Issue, Status};

/// derived information about an issue's dependency state.
#[derive(Debug, Clone)]
pub struct DerivedState {
    /// whether the issue is ready to work on
    pub is_ready: bool,
    /// dependencies that are not yet done
    pub open_deps: Vec<String>,
    /// dependencies that don't exist
    pub missing_deps: Vec<String>,
    /// whether the issue is blocked (not ready due to deps)
    pub is_blocked: bool,
}

/// compute derived state for a single issue given a lookup of all issues.
pub fn compute_derived(issue: &Issue, all_issues: &HashMap<String, Issue>) -> DerivedState {
    let mut open_deps = Vec::new();
    let mut missing_deps = Vec::new();

    for dep_id in issue.deps() {
        match all_issues.get(dep_id) {
            Some(dep_issue) => {
                if dep_issue.status() != Status::Done {
                    open_deps.push(dep_id.clone());
                }
            }
            None => {
                missing_deps.push(dep_id.clone());
            }
        }
    }

    let is_ready =
        issue.status() == Status::Todo && open_deps.is_empty() && missing_deps.is_empty();

    let is_blocked =
        issue.status() == Status::Todo && (!open_deps.is_empty() || !missing_deps.is_empty());

    DerivedState {
        is_ready,
        open_deps,
        missing_deps,
        is_blocked,
    }
}

/// find all cycles in the dependency graph.
/// returns a list of cycles, where each cycle is a list of issue IDs.
pub fn find_cycles(issues: &HashMap<String, Issue>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for id in issues.keys() {
        if !visited.contains(id) {
            find_cycles_dfs(
                id,
                issues,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn find_cycles_dfs(
    id: &str,
    issues: &HashMap<String, Issue>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(id.to_string());
    rec_stack.insert(id.to_string());
    path.push(id.to_string());

    if let Some(issue) = issues.get(id) {
        for dep_id in issue.deps() {
            if !visited.contains(dep_id) {
                find_cycles_dfs(dep_id, issues, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(dep_id) {
                // found a cycle - extract it from the path
                if let Some(start_idx) = path.iter().position(|x| x == dep_id) {
                    let mut cycle: Vec<String> = path[start_idx..].to_vec();
                    cycle.push(dep_id.clone()); // close the cycle
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    rec_stack.remove(id);
}

/// get all ready issues, sorted by priority, created_at, then id.
pub fn get_ready_issues(issues: &HashMap<String, Issue>) -> Vec<&Issue> {
    let mut ready: Vec<&Issue> = issues
        .values()
        .filter(|issue| {
            let derived = compute_derived(issue, issues);
            derived.is_ready
        })
        .collect();

    // sort by priority (P0 first), then created_at (oldest first), then id (lexicographic)
    ready.sort_by(|a, b| {
        a.priority()
            .cmp(&b.priority())
            .then_with(|| a.frontmatter.created_at.cmp(&b.frontmatter.created_at))
            .then_with(|| a.id().cmp(b.id()))
    });

    ready
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Priority;

    fn make_issue(id: &str, status: Status, deps: Vec<&str>) -> Issue {
        let mut issue = Issue::new(
            id.to_string(),
            format!("Issue {}", id),
            Priority::P1,
            deps.into_iter().map(String::from).collect(),
        );
        issue.frontmatter.status = status;
        issue
    }

    #[test]
    fn test_compute_derived_ready() {
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec![]));

        let derived = compute_derived(&issues["a"], &issues);
        assert!(derived.is_ready);
        assert!(!derived.is_blocked);
    }

    #[test]
    fn test_compute_derived_blocked() {
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec!["b"]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec![]));

        let derived = compute_derived(&issues["a"], &issues);
        assert!(!derived.is_ready);
        assert!(derived.is_blocked);
        assert_eq!(derived.open_deps, vec!["b"]);
    }

    #[test]
    fn test_find_cycles() {
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec!["b"]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec!["a"]));

        let cycles = find_cycles(&issues);
        assert!(!cycles.is_empty());
    }
}
