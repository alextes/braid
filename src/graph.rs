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
                // skip and done both count as "resolved" for dependency purposes
                if !matches!(dep_issue.status(), Status::Done | Status::Skip) {
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

/// check if adding a dependency from child to parent would create a cycle.
/// returns Some(path) if it would create a cycle, where path shows the cycle.
/// returns None if it's safe to add.
pub fn would_create_cycle(
    child_id: &str,
    parent_id: &str,
    issues: &HashMap<String, Issue>,
) -> Option<Vec<String>> {
    // adding child -> parent creates a cycle if parent can already reach child
    // i.e., there's a path from parent to child through existing deps
    let mut visited = HashSet::new();
    let mut path = vec![child_id.to_string(), parent_id.to_string()];

    if can_reach(parent_id, child_id, issues, &mut visited, &mut path) {
        Some(path)
    } else {
        None
    }
}

/// check if `from` can reach `to` via dependencies, building the path along the way.
fn can_reach(
    from: &str,
    to: &str,
    issues: &HashMap<String, Issue>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> bool {
    if from == to {
        return true;
    }

    if visited.contains(from) {
        return false;
    }
    visited.insert(from.to_string());

    if let Some(issue) = issues.get(from) {
        for dep_id in issue.deps() {
            path.push(dep_id.clone());
            if can_reach(dep_id, to, issues, visited, path) {
                return true;
            }
            path.pop();
        }
    }

    false
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
    ready.sort_by(|a, b| a.cmp_by_priority(b));

    ready
}

/// get all issues that depend on the given issue (reverse deps).
pub fn get_dependents(issue_id: &str, all_issues: &HashMap<String, Issue>) -> Vec<String> {
    let mut dependents: Vec<String> = all_issues
        .iter()
        .filter(|(_, issue)| issue.deps().contains(&issue_id.to_string()))
        .map(|(id, _)| id.clone())
        .collect();
    dependents.sort();
    dependents
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

    #[test]
    fn test_would_create_cycle_detects_direct() {
        // a -> b exists, adding b -> a would create cycle
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec!["b"]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec![]));

        let result = would_create_cycle("b", "a", &issues);
        assert!(result.is_some());
    }

    #[test]
    fn test_would_create_cycle_detects_indirect() {
        // a -> b -> c exists, adding c -> a would create cycle
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec!["b"]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec!["c"]));
        issues.insert("c".to_string(), make_issue("c", Status::Todo, vec![]));

        let result = would_create_cycle("c", "a", &issues);
        assert!(result.is_some());
    }

    #[test]
    fn test_would_create_cycle_allows_valid() {
        // a -> b exists, adding c -> a is fine
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec!["b"]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec![]));
        issues.insert("c".to_string(), make_issue("c", Status::Todo, vec![]));

        let result = would_create_cycle("c", "a", &issues);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_dependents() {
        // a depends on nothing, b and c depend on a
        let mut issues = HashMap::new();
        issues.insert("a".to_string(), make_issue("a", Status::Todo, vec![]));
        issues.insert("b".to_string(), make_issue("b", Status::Todo, vec!["a"]));
        issues.insert("c".to_string(), make_issue("c", Status::Todo, vec!["a"]));
        issues.insert("d".to_string(), make_issue("d", Status::Todo, vec!["b"]));

        let dependents = get_dependents("a", &issues);
        assert_eq!(dependents, vec!["b", "c"]);

        let dependents_b = get_dependents("b", &issues);
        assert_eq!(dependents_b, vec!["d"]);

        let dependents_d = get_dependents("d", &issues);
        assert!(dependents_d.is_empty());
    }
}
