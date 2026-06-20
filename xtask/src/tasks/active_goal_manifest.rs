//! Validate the active swarm goal manifest.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use toml::{Table, Value};

const ACTIVE_GOAL_PATH: &str = ".perl-lsp/goals/active.toml";
const REQUIRED_TOP_LEVEL_STRINGS: &[&str] = &["id", "title", "status", "owner", "created"];
const REQUIRED_PATH_FIELDS: &[&str] = &["proposal", "plan", "status_pointer", "operating_model"];
const REQUIRED_TEXT_ARRAYS: &[&str] = &["end_state", "claim_boundaries"];
const REQUIRED_PATH_ARRAYS: &[&str] = &["status_docs", "specs"];
const EXPECTED_LANES: &[&str] = &["trust", "substrate", "reliability"];
const ALLOWED_WORK_ITEM_STATUSES: &[&str] =
    &["active", "ready", "planned", "completed", "blocked", "deferred"];

#[derive(Debug, Default, Eq, PartialEq)]
struct ManifestStats {
    path_references: usize,
    proof_commands: usize,
    lanes: usize,
    work_items: usize,
    active_work_items: usize,
    completed_work_items: usize,
    repo: String,
    lane: String,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate(&root)?;

    println!(
        "active goal manifest check passed: repo={}, lane={}, {} lanes, {} work items ({} active, {} completed), {} path references, {} proof commands",
        stats.repo,
        stats.lane,
        stats.lanes,
        stats.work_items,
        stats.active_work_items,
        stats.completed_work_items,
        stats.path_references,
        stats.proof_commands,
    );

    Ok(())
}

fn validate(root: &Path) -> Result<ManifestStats> {
    let manifest_text = fs::read_to_string(root.join(ACTIVE_GOAL_PATH))
        .with_context(|| format!("failed to read {ACTIVE_GOAL_PATH}"))?;
    let manifest: Value = toml::from_str(&manifest_text)
        .with_context(|| format!("failed to parse {ACTIVE_GOAL_PATH}"))?;
    let Some(table) = manifest.as_table() else {
        bail!("{ACTIVE_GOAL_PATH}: expected TOML table");
    };

    let (stats, violations) = validate_manifest_table(root, table);
    if !violations.is_empty() {
        eprintln!("active goal manifest violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("active goal manifest check failed with {} violation(s)", violations.len());
    }

    Ok(stats)
}

fn validate_manifest_table(root: &Path, table: &Table) -> (ManifestStats, Vec<String>) {
    let mut stats = ManifestStats::default();
    let mut violations = Vec::new();

    validate_top_level(root, table, &mut stats, &mut violations);
    validate_current(table, &mut stats, &mut violations);
    let limits = validate_limits(table, &mut violations);
    let lanes = validate_lanes(table, &limits, &mut stats, &mut violations);
    validate_next_queues(table, &lanes, &mut violations);
    validate_work_items(root, table, &lanes, &mut stats, &mut violations);

    (stats, violations)
}

fn validate_top_level(
    root: &Path,
    table: &Table,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    for field in REQUIRED_TOP_LEVEL_STRINGS {
        require_non_empty_string(ACTIVE_GOAL_PATH, table, field, violations);
    }

    if string_field(table, "status") != Some("active") {
        violations.push(format!("{ACTIVE_GOAL_PATH}: status must be \"active\""));
    }

    require_non_empty_string(ACTIVE_GOAL_PATH, table, "objective", violations);

    for field in REQUIRED_TEXT_ARRAYS {
        validate_non_empty_string_array(ACTIVE_GOAL_PATH, table, field, violations);
    }

    for field in REQUIRED_PATH_FIELDS {
        validate_path_field(root, table, field, stats, violations);
    }

    for field in REQUIRED_PATH_ARRAYS {
        validate_path_array(root, table, field, stats, violations);
    }
}

fn validate_current(table: &Table, stats: &mut ManifestStats, violations: &mut Vec<String>) {
    let Some(current) = table.get("current").and_then(Value::as_table) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [current] table is required"));
        return;
    };

    for field in ["lane", "repo", "release_lineage_repo", "status"] {
        require_non_empty_string("[current]", current, field, violations);
    }

    if let Some(repo) = string_field(current, "repo") {
        stats.repo = repo.to_owned();
        if repo != "perl-lsp-swarm" {
            violations
                .push(format!("{ACTIVE_GOAL_PATH}: [current].repo must be \"perl-lsp-swarm\""));
        }
    }

    if let Some(lineage_repo) = string_field(current, "release_lineage_repo")
        && lineage_repo != "perl-lsp"
    {
        violations.push(format!(
            "{ACTIVE_GOAL_PATH}: [current].release_lineage_repo must be \"perl-lsp\""
        ));
    }

    if let Some(lane) = string_field(current, "lane") {
        stats.lane = lane.to_owned();
    }
}

fn validate_limits(table: &Table, violations: &mut Vec<String>) -> BTreeMap<String, i64> {
    let mut limits = BTreeMap::new();
    let Some(limit_table) = table.get("limits").and_then(Value::as_table) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [limits] table is required"));
        return limits;
    };

    for (field, lane) in
        [("trust_prs", "trust"), ("substrate_prs", "substrate"), ("reliability_prs", "reliability")]
    {
        match limit_table.get(field).and_then(Value::as_integer) {
            Some(value) if value > 0 => {
                limits.insert(lane.to_owned(), value);
            }
            Some(_) => {
                violations.push(format!("{ACTIVE_GOAL_PATH}: [limits].{field} must be positive"))
            }
            None => {
                violations.push(format!("{ACTIVE_GOAL_PATH}: [limits].{field} must be an integer"))
            }
        }
    }

    limits
}

fn validate_lanes(
    table: &Table,
    limits: &BTreeMap<String, i64>,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut lanes = BTreeSet::new();
    let Some(lane_items) = table.get("lanes").and_then(Value::as_array) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [[lanes]] entries are required"));
        return lanes;
    };

    for (index, item) in lane_items.iter().enumerate() {
        let doc = format!("{ACTIVE_GOAL_PATH}: lanes[{index}]");
        let Some(lane_table) = item.as_table() else {
            violations.push(format!("{doc} must be a TOML table"));
            continue;
        };

        require_non_empty_string(&doc, lane_table, "id", violations);
        require_non_empty_string(&doc, lane_table, "rule", violations);
        validate_non_empty_string_array(&doc, lane_table, "owns", violations);

        let Some(id) = string_field(lane_table, "id") else {
            continue;
        };
        if !EXPECTED_LANES.contains(&id) {
            violations.push(format!("{doc}: unknown lane id {id:?}"));
        }
        if !lanes.insert(id.to_owned()) {
            violations.push(format!("{doc}: duplicate lane id {id:?}"));
        }

        match lane_table.get("pr_cap").and_then(Value::as_integer) {
            Some(value) if value > 0 => {
                if let Some(expected) = limits.get(id)
                    && value != *expected
                {
                    violations.push(format!(
                        "{doc}: pr_cap {value} does not match [limits] value {expected}"
                    ));
                }
            }
            Some(_) => violations.push(format!("{doc}: pr_cap must be positive")),
            None => violations.push(format!("{doc}: pr_cap must be an integer")),
        }
    }

    for lane in EXPECTED_LANES {
        if !lanes.contains(*lane) {
            violations.push(format!("{ACTIVE_GOAL_PATH}: missing {lane:?} lane"));
        }
    }

    stats.lanes = lanes.len();
    lanes
}

fn validate_next_queues(table: &Table, lanes: &BTreeSet<String>, violations: &mut Vec<String>) {
    for lane in lanes {
        let key = format!("{lane}.next");
        let Some(next_table) = table
            .get(lane)
            .and_then(Value::as_table)
            .and_then(|lane_table| lane_table.get("next").and_then(Value::as_table))
        else {
            violations.push(format!("{ACTIVE_GOAL_PATH}: [{key}] table is required"));
            continue;
        };
        validate_non_empty_string_array(&format!("[{key}]"), next_table, "items", violations);
    }
}

fn validate_work_items(
    root: &Path,
    table: &Table,
    lanes: &BTreeSet<String>,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let Some(items) = table.get("work_item").and_then(Value::as_array) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [[work_item]] entries are required"));
        return;
    };

    let mut ids = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let doc = format!("{ACTIVE_GOAL_PATH}: work_item[{index}]");
        let Some(item_table) = item.as_table() else {
            violations.push(format!("{doc} must be a TOML table"));
            continue;
        };

        stats.work_items += 1;
        for field in ["id", "status", "lane", "claim_boundary"] {
            require_non_empty_string(&doc, item_table, field, violations);
        }

        if let Some(id) = string_field(item_table, "id")
            && !ids.insert(id.to_owned())
        {
            violations.push(format!("{doc}: duplicate work item id {id:?}"));
        }

        if let Some(status) = string_field(item_table, "status") {
            if !ALLOWED_WORK_ITEM_STATUSES.contains(&status) {
                violations.push(format!("{doc}: unsupported status {status:?}"));
            }
            if status == "active" {
                stats.active_work_items += 1;
            }
            if status == "completed" {
                stats.completed_work_items += 1;
            }
        }

        if let Some(lane) = string_field(item_table, "lane")
            && !lanes.contains(lane)
        {
            violations.push(format!("{doc}: lane {lane:?} is not declared in [[lanes]]"));
        }

        validate_optional_path_array(root, &doc, item_table, "files", stats, violations);
        validate_optional_command_array(&doc, item_table, "commands", stats, violations);
    }

    if stats.active_work_items == 0 {
        violations.push(format!("{ACTIVE_GOAL_PATH}: at least one work_item must be active"));
    }
}

fn validate_path_field(
    root: &Path,
    table: &Table,
    field: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    match string_field(table, field) {
        Some(path) => {
            validate_relative_existing_path(root, ACTIVE_GOAL_PATH, field, path, stats, violations)
        }
        None => violations.push(format!("{ACTIVE_GOAL_PATH}: {field} must be a string path")),
    }
}

fn validate_path_array(
    root: &Path,
    table: &Table,
    field: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let Some(values) = table.get(field).and_then(Value::as_array) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: {field} must be a non-empty array"));
        return;
    };

    if values.is_empty() {
        violations.push(format!("{ACTIVE_GOAL_PATH}: {field} must not be empty"));
    }

    for (index, value) in values.iter().enumerate() {
        let Some(path) = value.as_str() else {
            violations.push(format!("{ACTIVE_GOAL_PATH}: {field}[{index}] must be a string"));
            continue;
        };
        validate_relative_existing_path(
            root,
            ACTIVE_GOAL_PATH,
            &format!("{field}[{index}]"),
            path,
            stats,
            violations,
        );
    }
}

fn validate_optional_path_array(
    root: &Path,
    doc: &str,
    table: &Table,
    field: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let Some(value) = table.get(field) else {
        return;
    };
    let Some(values) = value.as_array() else {
        violations.push(format!("{doc}: {field} must be an array when present"));
        return;
    };
    for (index, value) in values.iter().enumerate() {
        let Some(path) = value.as_str() else {
            violations.push(format!("{doc}: {field}[{index}] must be a string"));
            continue;
        };
        validate_relative_existing_path(
            root,
            doc,
            &format!("{field}[{index}]"),
            path,
            stats,
            violations,
        );
    }
}

fn validate_optional_command_array(
    doc: &str,
    table: &Table,
    field: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let Some(value) = table.get(field) else {
        return;
    };
    let Some(values) = value.as_array() else {
        violations.push(format!("{doc}: {field} must be an array when present"));
        return;
    };
    for (index, value) in values.iter().enumerate() {
        let Some(command) = value.as_str() else {
            violations.push(format!("{doc}: {field}[{index}] must be a string"));
            continue;
        };
        if command.trim().is_empty() {
            violations.push(format!("{doc}: {field}[{index}] must not be empty"));
            continue;
        }
        if !command.starts_with("rtk ") {
            violations.push(format!("{doc}: {field}[{index}] must start with \"rtk \""));
        }
        stats.proof_commands += 1;
    }
}

fn validate_relative_existing_path(
    root: &Path,
    doc: &str,
    field: &str,
    path: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    if path.trim().is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
        return;
    }
    if Path::new(path).is_absolute() || path.contains(':') || path.contains('\\') {
        violations.push(format!("{doc}: {field} must be a repo-relative slash path: {path}"));
        return;
    }
    if !root.join(path).exists() {
        violations.push(format!("{doc}: {field} points to missing path {path}"));
        return;
    }
    stats.path_references += 1;
}

fn validate_non_empty_string_array(
    doc: &str,
    table: &Table,
    field: &str,
    violations: &mut Vec<String>,
) {
    let Some(values) = table.get(field).and_then(Value::as_array) else {
        violations.push(format!("{doc}: {field} must be a non-empty array"));
        return;
    };
    if values.is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
    }
    for (index, value) in values.iter().enumerate() {
        match value.as_str() {
            Some(text) if !text.trim().is_empty() => {}
            Some(_) => violations.push(format!("{doc}: {field}[{index}] must not be empty")),
            None => violations.push(format!("{doc}: {field}[{index}] must be a string")),
        }
    }
}

fn require_non_empty_string(doc: &str, table: &Table, field: &str, violations: &mut Vec<String>) {
    match string_field(table, field) {
        Some(value) if !value.trim().is_empty() => {}
        Some(_) => violations.push(format!("{doc}: {field} must not be empty")),
        None => violations.push(format!("{doc}: {field} must be a string")),
    }
}

fn string_field<'a>(table: &'a Table, field: &str) -> Option<&'a str> {
    table.get(field).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::ensure;
    use tempfile::TempDir;

    fn fixture_root(paths: &[&str]) -> Result<TempDir> {
        let temp = tempfile::tempdir()?;
        for path in paths {
            let full_path = temp.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, "fixture")?;
        }
        Ok(temp)
    }

    #[test]
    fn active_goal_manifest_accepts_current_contract() -> Result<()> {
        let stats = validate(&project_root()?)?;

        assert_eq!(stats.repo, "perl-lsp-swarm");
        assert_eq!(stats.lane, "real_perl_editor_trust_v1");
        assert_eq!(stats.lanes, 3);
        assert_eq!(stats.active_work_items, 1);

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_top_level_status_and_path_contracts() -> Result<()> {
        let root = fixture_root(&["docs/proposal.md", "docs/status.md"])?;
        let mut table = Table::new();
        for field in REQUIRED_TOP_LEVEL_STRINGS {
            table.insert((*field).to_owned(), Value::String("present".to_owned()));
        }
        table.insert("status".to_owned(), Value::String("paused".to_owned()));
        table.insert("objective".to_owned(), Value::String(" ".to_owned()));
        table.insert(
            "end_state".to_owned(),
            Value::Array(vec![Value::String(String::new()), Value::Integer(7)]),
        );
        table.insert("claim_boundaries".to_owned(), Value::String("not an array".to_owned()));
        table.insert("proposal".to_owned(), Value::String("docs/proposal.md".to_owned()));
        table.insert("plan".to_owned(), Value::String("plans/missing.md".to_owned()));
        table.insert("status_pointer".to_owned(), Value::Integer(1));
        table.insert("operating_model".to_owned(), Value::String("docs\\operating.md".to_owned()));
        table.insert(
            "status_docs".to_owned(),
            Value::Array(vec![Value::String("docs/status.md".to_owned()), Value::Integer(9)]),
        );
        table.insert("specs".to_owned(), Value::Array(Vec::new()));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_top_level(root.path(), &table, &mut stats, &mut violations);

        ensure!(stats.path_references == 2, "got stats: {stats:?}");
        for expected in [
            ".perl-lsp/goals/active.toml: status must be \"active\"",
            ".perl-lsp/goals/active.toml: objective must not be empty",
            ".perl-lsp/goals/active.toml: end_state[0] must not be empty",
            ".perl-lsp/goals/active.toml: end_state[1] must be a string",
            ".perl-lsp/goals/active.toml: claim_boundaries must be a non-empty array",
            ".perl-lsp/goals/active.toml: plan points to missing path plans/missing.md",
            ".perl-lsp/goals/active.toml: status_pointer must be a string path",
            ".perl-lsp/goals/active.toml: operating_model must be a repo-relative slash path: docs\\operating.md",
            ".perl-lsp/goals/active.toml: status_docs[1] must be a string",
            ".perl-lsp/goals/active.toml: specs must not be empty",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_wrong_current_repo_and_release_lineage() -> Result<()> {
        let mut table = Table::new();
        let mut current = Table::new();
        current.insert("lane".to_owned(), Value::String("lane-a".to_owned()));
        current.insert("repo".to_owned(), Value::String("wrong-repo".to_owned()));
        current
            .insert("release_lineage_repo".to_owned(), Value::String("wrong-lineage".to_owned()));
        current.insert("status".to_owned(), Value::String("active".to_owned()));
        table.insert("current".to_owned(), Value::Table(current));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_current(&table, &mut stats, &mut violations);

        ensure!(stats.repo == "wrong-repo", "got stats: {stats:?}");
        ensure!(stats.lane == "lane-a", "got stats: {stats:?}");
        ensure!(
            violations
                == vec![
                    ".perl-lsp/goals/active.toml: [current].repo must be \"perl-lsp-swarm\""
                        .to_owned(),
                    ".perl-lsp/goals/active.toml: [current].release_lineage_repo must be \"perl-lsp\""
                        .to_owned(),
                ],
            "got violations: {violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_limits_lanes_and_next_queue_shape() -> Result<()> {
        let mut limits_table = Table::new();
        limits_table.insert("trust_prs".to_owned(), Value::Integer(0));
        limits_table.insert("substrate_prs".to_owned(), Value::String("two".to_owned()));
        limits_table.insert("reliability_prs".to_owned(), Value::Integer(4));
        let mut table = Table::new();
        table.insert("limits".to_owned(), Value::Table(limits_table));

        let mut violations = Vec::new();
        let limits = validate_limits(&table, &mut violations);

        ensure!(limits.len() == 1, "got limits: {limits:?}");
        ensure!(limits.get("reliability") == Some(&4), "got limits: {limits:?}");
        ensure!(
            violations
                == vec![
                    ".perl-lsp/goals/active.toml: [limits].trust_prs must be positive".to_owned(),
                    ".perl-lsp/goals/active.toml: [limits].substrate_prs must be an integer"
                        .to_owned(),
                ],
            "got violations: {violations:?}"
        );

        let mut trust_lane = Table::new();
        trust_lane.insert("id".to_owned(), Value::String("trust".to_owned()));
        trust_lane.insert("rule".to_owned(), Value::String("rule".to_owned()));
        trust_lane
            .insert("owns".to_owned(), Value::Array(vec![Value::String("policy".to_owned())]));
        trust_lane.insert("pr_cap".to_owned(), Value::Integer(1));
        table.insert(
            "lanes".to_owned(),
            Value::Array(vec![Value::String("bad".to_owned()), Value::Table(trust_lane)]),
        );
        let mut stats = ManifestStats::default();
        let mut lane_violations = Vec::new();

        let lanes = validate_lanes(&table, &limits, &mut stats, &mut lane_violations);

        ensure!(lanes == BTreeSet::from(["trust".to_owned()]), "got lanes: {lanes:?}");
        for expected in [
            ".perl-lsp/goals/active.toml: lanes[0] must be a TOML table",
            ".perl-lsp/goals/active.toml: missing \"substrate\" lane",
            ".perl-lsp/goals/active.toml: missing \"reliability\" lane",
        ] {
            ensure!(
                lane_violations.iter().any(|violation| violation == expected),
                "missing lane violation {expected:?}; got {lane_violations:?}"
            );
        }

        let mut next_violations = Vec::new();
        validate_next_queues(&table, &lanes, &mut next_violations);
        ensure!(
            next_violations
                == vec![".perl-lsp/goals/active.toml: [trust.next] table is required".to_owned()],
            "got next violations: {next_violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_optional_path_and_command_entry_contracts() -> Result<()> {
        let root = fixture_root(&["docs/kept.md"])?;
        let mut stats = ManifestStats::default();
        let mut path_violations = Vec::new();
        let mut path_table = Table::new();
        path_table.insert(
            "files".to_owned(),
            Value::Array(vec![
                Value::String("docs/kept.md".to_owned()),
                Value::String("docs/missing.md".to_owned()),
                Value::Integer(3),
            ]),
        );

        validate_optional_path_array(
            root.path(),
            ".perl-lsp/goals/active.toml: work_item[0]",
            &path_table,
            "files",
            &mut stats,
            &mut path_violations,
        );

        ensure!(stats.path_references == 1, "got stats: {stats:?}");
        ensure!(
            path_violations
                == vec![
                    ".perl-lsp/goals/active.toml: work_item[0]: files[1] points to missing path docs/missing.md"
                        .to_owned(),
                    ".perl-lsp/goals/active.toml: work_item[0]: files[2] must be a string"
                        .to_owned(),
                ],
            "got path violations: {path_violations:?}"
        );

        let mut command_table = Table::new();
        command_table.insert(
            "commands".to_owned(),
            Value::Array(vec![
                Value::String("rtk cargo test -p xtask".to_owned()),
                Value::String(" ".to_owned()),
                Value::Integer(5),
            ]),
        );
        let mut command_violations = Vec::new();

        validate_optional_command_array(
            ".perl-lsp/goals/active.toml: work_item[0]",
            &command_table,
            "commands",
            &mut stats,
            &mut command_violations,
        );

        ensure!(stats.proof_commands == 1, "got stats: {stats:?}");
        ensure!(
            command_violations
                == vec![
                    ".perl-lsp/goals/active.toml: work_item[0]: commands[1] must not be empty"
                        .to_owned(),
                    ".perl-lsp/goals/active.toml: work_item[0]: commands[2] must be a string"
                        .to_owned(),
                ],
            "got command violations: {command_violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_non_rtk_proof_commands() -> Result<()> {
        let mut table = Table::new();
        table.insert(
            "commands".to_owned(),
            Value::Array(vec![Value::String("cargo test -p xtask".to_owned())]),
        );
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_optional_command_array(
            ".perl-lsp/goals/active.toml: work_item[0]",
            &table,
            "commands",
            &mut stats,
            &mut violations,
        );

        assert_eq!(stats.proof_commands, 1);
        assert_eq!(
            violations,
            vec![
                ".perl-lsp/goals/active.toml: work_item[0]: commands[0] must start with \"rtk \""
                    .to_owned()
            ]
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_non_repo_relative_paths() -> Result<()> {
        let root = project_root()?;
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_relative_existing_path(
            &root,
            ACTIVE_GOAL_PATH,
            "proposal",
            "C:/tmp/proposal.md",
            &mut stats,
            &mut violations,
        );

        assert_eq!(
            violations,
            vec![
                ".perl-lsp/goals/active.toml: proposal must be a repo-relative slash path: C:/tmp/proposal.md"
                    .to_owned()
            ]
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_requires_declared_lane_and_active_work() -> Result<()> {
        let root = project_root()?;
        let mut table = Table::new();
        let mut work_item = Table::new();
        work_item.insert("id".to_owned(), Value::String("wi-1".to_owned()));
        work_item.insert("status".to_owned(), Value::String("completed".to_owned()));
        work_item.insert("lane".to_owned(), Value::String("unknown".to_owned()));
        work_item.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        table.insert("work_item".to_owned(), Value::Array(vec![Value::Table(work_item)]));
        let lanes = BTreeSet::from(["trust".to_owned()]);
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_work_items(&root, &table, &lanes, &mut stats, &mut violations);

        assert_eq!(
            violations,
            vec![
                ".perl-lsp/goals/active.toml: work_item[0]: lane \"unknown\" is not declared in [[lanes]]"
                    .to_owned(),
                ".perl-lsp/goals/active.toml: at least one work_item must be active".to_owned(),
            ]
        );

        Ok(())
    }
}
