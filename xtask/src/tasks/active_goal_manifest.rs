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

#[derive(Debug, Default)]
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

    let mut stats = ManifestStats::default();
    let mut violations = Vec::new();

    validate_top_level(root, table, &mut stats, &mut violations);
    validate_current(table, &mut stats, &mut violations);
    let limits = validate_limits(table, &mut violations);
    let lanes = validate_lanes(table, &limits, &mut stats, &mut violations);
    validate_next_queues(table, &lanes, &mut violations);
    validate_work_items(root, table, &lanes, &mut stats, &mut violations);

    if !violations.is_empty() {
        eprintln!("active goal manifest violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("active goal manifest check failed with {} violation(s)", violations.len());
    }

    Ok(stats)
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
