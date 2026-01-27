//! Issue schema migration system.
//!
//! Migrations transform issue files from older schema versions to newer ones.
//! Each migration is a function that transforms schema version N to N+1.

use serde_yaml::Value;

use crate::error::{BrdError, Result};

/// The current schema version. All new issues are created with this version.
pub const CURRENT_SCHEMA: u32 = 9;

/// Check if a schema version needs migration.
pub fn needs_migration(schema_version: u32) -> bool {
    schema_version < CURRENT_SCHEMA
}

/// Extract the schema version from frontmatter YAML.
pub fn get_schema_version(frontmatter: &Value) -> Result<u32> {
    // Try "schema_version" first (future), then "brd" (current)
    if let Some(v) = frontmatter.get("schema_version") {
        return v.as_u64().map(|n| n as u32).ok_or_else(|| {
            BrdError::ParseError("schema_version".into(), "expected integer".into())
        });
    }

    if let Some(v) = frontmatter.get("brd") {
        return v
            .as_u64()
            .map(|n| n as u32)
            .ok_or_else(|| BrdError::ParseError("brd".into(), "expected integer".into()));
    }

    // No version field - assume schema version 0 (pre-versioning)
    Ok(0)
}

/// Migrate frontmatter from its current version to the target version.
/// Returns the migrated frontmatter and whether any migration was applied.
pub fn migrate_frontmatter(mut frontmatter: Value, target_version: u32) -> Result<(Value, bool)> {
    let current = get_schema_version(&frontmatter)?;

    if current >= target_version {
        return Ok((frontmatter, false));
    }

    // Apply migrations sequentially
    for version in current..target_version {
        frontmatter = apply_migration(frontmatter, version)?;
    }

    Ok((frontmatter, true))
}

/// Apply a single migration step from `from_version` to `from_version + 1`.
fn apply_migration(frontmatter: Value, from_version: u32) -> Result<Value> {
    match from_version {
        0 => migrate_v0_to_v1(frontmatter),
        1 => migrate_v1_to_v2(frontmatter),
        2 => migrate_v2_to_v3(frontmatter),
        3 => migrate_v3_to_v4(frontmatter),
        4 => migrate_v4_to_v5(frontmatter),
        5 => migrate_v5_to_v6(frontmatter),
        6 => migrate_v6_to_v7(frontmatter),
        7 => migrate_v7_to_v8(frontmatter),
        8 => migrate_v8_to_v9(frontmatter),
        _ => {
            // No migration needed for this version
            Ok(frontmatter)
        }
    }
}

/// Migration from v0 (pre-versioning) to v1.
/// - Adds `brd: 1` if missing
fn migrate_v0_to_v1(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        // Add brd: 1 if not present
        let brd_key = Value::String("brd".to_string());
        if !map.contains_key(&brd_key) {
            map.insert(brd_key, Value::Number(1.into()));
        }
    }
    Ok(frontmatter)
}

/// Migration from v1 to v2.
/// - Renames `brd` to `schema_version`
fn migrate_v1_to_v2(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let brd_key = Value::String("brd".to_string());
        let schema_key = Value::String("schema_version".to_string());

        // Remove brd key and add schema_version: 2
        map.remove(&brd_key);
        map.insert(schema_key, Value::Number(2.into()));
    }
    Ok(frontmatter)
}

/// Migration from v2 to v3.
/// - Adds `owner: null` if missing (owner field is now required)
fn migrate_v2_to_v3(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let owner_key = Value::String("owner".to_string());
        let schema_key = Value::String("schema_version".to_string());

        // Add owner: null if not present
        if !map.contains_key(&owner_key) {
            map.insert(owner_key, Value::Null);
        }

        // Update schema version
        map.insert(schema_key, Value::Number(3.into()));
    }
    Ok(frontmatter)
}

/// Migration from v3 to v4.
/// - Renames `labels` to `tags`
fn migrate_v3_to_v4(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let labels_key = Value::String("labels".to_string());
        let tags_key = Value::String("tags".to_string());
        let schema_key = Value::String("schema_version".to_string());

        // Rename labels to tags if present
        if let Some(labels_value) = map.remove(&labels_key) {
            map.insert(tags_key, labels_value);
        }

        // Update schema version
        map.insert(schema_key, Value::Number(4.into()));
    }
    Ok(frontmatter)
}

/// Migration from v4 to v5.
/// - No issue schema changes, just version bump for external-repo config support
fn migrate_v4_to_v5(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let schema_key = Value::String("schema_version".to_string());
        map.insert(schema_key, Value::Number(5.into()));
    }
    Ok(frontmatter)
}

/// Migration from v5 to v6.
/// - No issue schema changes, just version bump for auto_pull/auto_push config support
fn migrate_v5_to_v6(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let schema_key = Value::String("schema_version".to_string());
        map.insert(schema_key, Value::Number(6.into()));
    }
    Ok(frontmatter)
}

/// Migration from v6 to v7.
/// - Renames status 'todo' to 'open'
fn migrate_v6_to_v7(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let status_key = Value::String("status".to_string());
        let schema_key = Value::String("schema_version".to_string());

        // Rename status: todo -> open
        if let Some(Value::String(status)) = map.get(&status_key)
            && status == "todo"
        {
            map.insert(status_key, Value::String("open".to_string()));
        }

        // Update schema version
        map.insert(schema_key, Value::Number(7.into()));
    }
    Ok(frontmatter)
}

/// Migration from v7 to v8.
/// - Removes `updated_at`, adds `started_at` and `completed_at`
/// - If status is 'doing', set `started_at` to old `updated_at`
/// - If status is 'done' or 'skip', set both to old `updated_at`
fn migrate_v7_to_v8(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let updated_at_key = Value::String("updated_at".to_string());
        let started_at_key = Value::String("started_at".to_string());
        let completed_at_key = Value::String("completed_at".to_string());
        let status_key = Value::String("status".to_string());
        let schema_key = Value::String("schema_version".to_string());

        // Get the old updated_at value and status
        let updated_at = map.remove(&updated_at_key);
        let status = map.get(&status_key).and_then(|v| v.as_str()).unwrap_or("");

        // Set started_at and completed_at based on status
        match status {
            "doing" => {
                if let Some(ts) = updated_at {
                    map.insert(started_at_key, ts);
                }
            }
            "done" | "skip" => {
                if let Some(ts) = updated_at {
                    map.insert(started_at_key.clone(), ts.clone());
                    map.insert(completed_at_key, ts);
                }
            }
            _ => {
                // open status: no timestamps to set
            }
        }

        // Update schema version
        map.insert(schema_key, Value::Number(8.into()));
    }
    Ok(frontmatter)
}

/// Migration from v8 to v9.
/// - Adds optional `scheduled_for` field (no data changes needed)
fn migrate_v8_to_v9(mut frontmatter: Value) -> Result<Value> {
    if let Value::Mapping(ref mut map) = frontmatter {
        let schema_key = Value::String("schema_version".to_string());
        map.insert(schema_key, Value::Number(9.into()));
    }
    Ok(frontmatter)
}

/// Summary of what migrations would be applied to get from one version to another.
pub fn migration_summary(from_version: u32, to_version: u32) -> Vec<String> {
    let mut summaries = Vec::new();

    for version in from_version..to_version {
        match version {
            0 => summaries.push("v0→v1: add schema version field".to_string()),
            1 => summaries.push("v1→v2: rename 'brd' to 'schema_version'".to_string()),
            2 => summaries.push("v2→v3: add required 'owner' field".to_string()),
            3 => summaries.push("v3→v4: rename 'labels' to 'tags'".to_string()),
            4 => summaries.push("v4→v5: external-repo config support".to_string()),
            5 => summaries.push("v5→v6: auto_pull/auto_push config support".to_string()),
            6 => summaries.push("v6→v7: rename status 'todo' to 'open'".to_string()),
            7 => {
                summaries.push("v7→v8: replace updated_at with started_at/completed_at".to_string())
            }
            8 => summaries.push("v8→v9: add scheduled_for field".to_string()),
            _ => {}
        }
    }

    summaries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_schema_version_brd() {
        let yaml: Value = serde_yaml::from_str("brd: 1\nid: test").unwrap();
        assert_eq!(get_schema_version(&yaml).unwrap(), 1);
    }

    #[test]
    fn test_get_schema_version_schema_version() {
        let yaml: Value = serde_yaml::from_str("schema_version: 2\nid: test").unwrap();
        assert_eq!(get_schema_version(&yaml).unwrap(), 2);
    }

    #[test]
    fn test_get_schema_version_missing() {
        let yaml: Value = serde_yaml::from_str("id: test").unwrap();
        assert_eq!(get_schema_version(&yaml).unwrap(), 0);
    }

    #[test]
    fn test_migrate_v0_to_v1() {
        let yaml: Value = serde_yaml::from_str("id: test\ntitle: Test").unwrap();
        let (migrated, changed) = migrate_frontmatter(yaml, 1).unwrap();
        assert!(changed);
        assert_eq!(get_schema_version(&migrated).unwrap(), 1);
    }

    #[test]
    fn test_migrate_v1_to_v2() {
        let yaml: Value = serde_yaml::from_str("brd: 1\nid: test").unwrap();
        let (migrated, changed) = migrate_frontmatter(yaml, 2).unwrap();
        assert!(changed);
        assert_eq!(get_schema_version(&migrated).unwrap(), 2);
        // Verify brd key is gone and schema_version exists
        assert!(migrated.get("brd").is_none());
        assert!(migrated.get("schema_version").is_some());
    }

    #[test]
    fn test_migrate_v0_to_v2() {
        // Full migration path from v0 to current
        let yaml: Value = serde_yaml::from_str("id: test\ntitle: Test").unwrap();
        let (migrated, changed) = migrate_frontmatter(yaml, 2).unwrap();
        assert!(changed);
        assert_eq!(get_schema_version(&migrated).unwrap(), 2);
        assert!(migrated.get("schema_version").is_some());
    }

    #[test]
    fn test_no_migration_needed() {
        let yaml: Value = serde_yaml::from_str("schema_version: 2\nid: test").unwrap();
        let (_migrated, changed) = migrate_frontmatter(yaml, 2).unwrap();
        assert!(!changed);
    }
}
