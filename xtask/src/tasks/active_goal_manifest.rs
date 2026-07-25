//! Validate the active swarm goal manifest.
//!
//! Schema 2 (the "goal-state split", #3612) remains a compatibility shape:
//! `active.toml` was a small
//! pointer that names the active program and lane, and points at a durable
//! program manifest (`.perl-lsp/goals/programs/<program>.toml`) plus a
//! board file. The program manifest owns the durable outcome, claim
//! boundary, lane ownership, WIP caps, and the (small) set of currently
//! active work items. Each routing lane it owns has a full definition
//! (may-change / must-route-elsewhere / board / proof policy) in
//! `.perl-lsp/goals/lanes/<lane>.toml`. Completed work items live under
//! `.perl-lsp/goals/archive/` and are relocated only, never re-validated
//! against the active-lane contract.
//!
//! Schema 3 makes the repository-global surface a portfolio of enabled
//! programs. It deliberately has no active/default program authority; a
//! session claims one issue and worktree separately.

use crate::tasks::goals::manifest as goals_manifest;
use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use toml::{Table, Value};

const ACTIVE_GOAL_PATH: &str = ".perl-lsp/goals/active.toml";
const EXPECTED_LANES: &[&str] = &["trust", "substrate", "reliability"];
const ALLOWED_WORK_ITEM_STATUSES: &[&str] =
    &["active", "ready", "planned", "completed", "blocked", "deferred"];

const PROGRAM_REQUIRED_TOP_LEVEL_STRINGS: &[&str] = &["id", "title", "owner", "created"];
const PROGRAM_REQUIRED_PATH_FIELDS: &[&str] =
    &["proposal", "plan", "status_pointer", "operating_model"];
const PROGRAM_REQUIRED_TEXT_ARRAYS: &[&str] = &["end_state", "claim_boundaries"];
const PROGRAM_REQUIRED_PATH_ARRAYS: &[&str] = &["status_docs", "specs"];
const PORTFOLIO_PROGRAM_KINDS: [&str; 2] = ["lane_routing", "milestone_ledger"];
const EXPECTED_REPOSITORY: &str = "perl-lsp-swarm";

const LANE_REQUIRED_TOP_LEVEL_STRINGS: &[&str] = &["id", "program", "proof_policy"];
const LANE_REQUIRED_TEXT_ARRAYS: &[&str] = &["may_change", "next_items"];

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
    program: String,
    programs: usize,
    enabled_programs: usize,
    warnings: Vec<String>,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate(&root)?;

    println!(
        "active goal manifest check passed: repo={}, lane={}, program={}, {} programs ({} enabled), {} lanes, {} work items ({} active, {} completed), {} path references, {} proof commands",
        stats.repo,
        stats.lane,
        stats.program,
        stats.programs,
        stats.enabled_programs,
        stats.lanes,
        stats.work_items,
        stats.active_work_items,
        stats.completed_work_items,
        stats.path_references,
        stats.proof_commands,
    );
    for warning in &stats.warnings {
        eprintln!("active goal manifest warning: {warning}");
    }

    Ok(())
}

fn validate(root: &Path) -> Result<ManifestStats> {
    let pointer_text = fs::read_to_string(root.join(ACTIVE_GOAL_PATH))
        .with_context(|| format!("failed to read {ACTIVE_GOAL_PATH}"))?;
    let pointer: Value = toml::from_str(&pointer_text)
        .with_context(|| format!("failed to parse {ACTIVE_GOAL_PATH}"))?;
    let Some(pointer_table) = pointer.as_table() else {
        bail!("{ACTIVE_GOAL_PATH}: expected TOML table");
    };

    let mut stats = ManifestStats::default();
    let mut violations = Vec::new();

    if pointer_table.get("schema").and_then(Value::as_integer) == Some(3) {
        validate_portfolio(root, pointer_table, &mut stats, &mut violations);
        if !violations.is_empty() {
            eprintln!("active goal manifest violations:");
            for violation in &violations {
                eprintln!("  - {violation}");
            }
            bail!("active goal manifest check failed with {} violation(s)", violations.len());
        }
        return Ok(stats);
    }

    // Captured from the pointer before validate_pointer/validate_program_manifest
    // run, so the program-manifest and lane-manifest cross-checks below can fail
    // loud on dangling/mistyped pointer-to-program and pointer-to-lane references
    // (see #3612 M2 review: factory-droid P1, sourcery, cubic).
    let expected_program = string_field(pointer_table, "active_program").unwrap_or_default();
    let expected_lane = string_field(pointer_table, "active_lane").unwrap_or_default();

    let resolved = validate_pointer(root, pointer_table, &mut stats, &mut violations);

    if let Some((program_path, _board_path)) = resolved {
        match load_table(root, &program_path) {
            Ok(program_table) => {
                let lane_ownership = validate_program_manifest(
                    root,
                    &program_table,
                    expected_lane,
                    &mut stats,
                    &mut violations,
                );
                for owned in &lane_ownership {
                    match load_table(root, &owned.manifest) {
                        Ok(lane_table) => {
                            validate_lane_manifest(
                                root,
                                &lane_table,
                                owned,
                                expected_program,
                                &mut stats,
                                &mut violations,
                            );
                        }
                        Err(err) => violations.push(format!(
                            "{ACTIVE_GOAL_PATH}: [program].manifest lane_ownership[{}]: failed to load {}: {err}",
                            owned.lane, owned.manifest
                        )),
                    }
                }
                stats.lanes = lane_ownership.len();
            }
            Err(err) => violations.push(format!(
                "{ACTIVE_GOAL_PATH}: [program].manifest: failed to load {program_path}: {err}"
            )),
        }
    }

    validate_default_program(root, pointer_table, expected_program, &mut violations);

    if !violations.is_empty() {
        eprintln!("active goal manifest violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("active goal manifest check failed with {} violation(s)", violations.len());
    }

    Ok(stats)
}

fn validate_portfolio(
    root: &Path,
    table: &Table,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    if table.get("mode").and_then(Value::as_str) != Some("portfolio") {
        violations.push(format!("{ACTIVE_GOAL_PATH}: schema 3 requires mode = \"portfolio\""));
    }

    let Some(authority) = table.get("authority").and_then(Value::as_table) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [authority] table is required"));
        return;
    };
    for (field, expected) in [("work_items", "github"), ("specs", "allow"), ("receipts", "machine")]
    {
        match authority.get(field).and_then(Value::as_str) {
            Some(value) if value == expected => {}
            Some(value) => violations.push(format!(
                "{ACTIVE_GOAL_PATH}: [authority].{field} must be {expected:?}, got {value:?}"
            )),
            None => {
                violations.push(format!("{ACTIVE_GOAL_PATH}: [authority].{field} must be a string"))
            }
        }
    }

    let Some(selection) = table.get("selection").and_then(Value::as_table) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [selection] table is required"));
        return;
    };
    if selection.get("strategy").and_then(Value::as_str) != Some("eligible_portfolio") {
        violations.push(format!(
            "{ACTIVE_GOAL_PATH}: [selection].strategy must be \"eligible_portfolio\""
        ));
    }
    for field in [
        "require_explicit_claim",
        "respect_lane_caps",
        "respect_dependencies",
        "respect_conflict_surfaces",
    ] {
        if selection.get(field).and_then(Value::as_bool) != Some(true) {
            violations.push(format!("{ACTIVE_GOAL_PATH}: [selection].{field} must be true"));
        }
    }

    for field in ["active_program", "active_lane", "default_program"] {
        if table.contains_key(field) {
            stats.warnings.push(format!(
                "{ACTIVE_GOAL_PATH}: legacy {field} is ignored in schema 3 portfolio selection"
            ));
        }
    }

    let Some(program_value) = table.get("program") else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [[program]] entries are required"));
        return;
    };
    let Some(programs) = program_value.as_array() else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: program must be an array of tables"));
        return;
    };
    if programs.is_empty() {
        violations.push(format!("{ACTIVE_GOAL_PATH}: at least one [[program]] entry is required"));
        return;
    }

    let mut ids = BTreeSet::new();
    for (index, value) in programs.iter().enumerate() {
        let doc = format!("{ACTIVE_GOAL_PATH}: program[{index}]");
        let Some(program) = value.as_table() else {
            violations.push(format!("{doc} must be a table"));
            continue;
        };
        let Some(id) = string_field(program, "id") else {
            violations.push(format!("{doc}.id must be a string"));
            continue;
        };
        if !is_bare_program_id(id) || id.trim().is_empty() {
            violations.push(format!("{doc}.id must be a non-empty bare program id, got {id:?}"));
        }
        if !ids.insert(id.to_owned()) {
            violations.push(format!("{doc}.id duplicates program {id:?}"));
        }
        let Some(manifest) = string_field(program, "manifest") else {
            violations.push(format!("{doc}.manifest must be a string path"));
            continue;
        };
        let expected_manifest = goals_manifest::program_manifest_path(id);
        if manifest != expected_manifest {
            violations
                .push(format!("{doc}.manifest must be {expected_manifest:?}, got {manifest:?}"));
        }
        if !validate_relative_existing_path(root, &doc, "manifest", manifest, stats, violations) {
            continue;
        }
        let enabled = match program.get("enabled").and_then(Value::as_bool) {
            Some(value) => value,
            None => {
                violations.push(format!("{doc}.enabled must be a boolean"));
                false
            }
        };
        stats.programs += 1;
        if enabled {
            stats.enabled_programs += 1;
        }

        let Some(kind) = string_field(program, "kind") else {
            violations.push(format!("{doc}.kind must be one of {PORTFOLIO_PROGRAM_KINDS:?}"));
            continue;
        };
        if !PORTFOLIO_PROGRAM_KINDS.contains(&kind) {
            violations.push(format!(
                "{doc}.kind must be one of {PORTFOLIO_PROGRAM_KINDS:?}, got {kind:?}"
            ));
            continue;
        }

        let text = match fs::read_to_string(root.join(manifest)) {
            Ok(text) => text,
            Err(err) => {
                violations.push(format!("{doc}.manifest failed to load {manifest}: {err}"));
                continue;
            }
        };
        let Ok(program_table) = load_table(root, manifest) else {
            violations.push(format!("{doc}.manifest failed to parse {manifest}"));
            continue;
        };
        let actual_kind = if goals_manifest::is_milestone_ledger(&text) {
            "milestone_ledger"
        } else {
            "lane_routing"
        };
        if actual_kind != kind {
            violations
                .push(format!("{doc}.kind {kind:?} does not match manifest shape {actual_kind:?}"));
            continue;
        }

        match kind {
            "milestone_ledger" => match goals_manifest::load_milestone_ledger(root, manifest) {
                Ok(ledger) => {
                    if ledger.id != id {
                        violations
                            .push(format!("{doc}.manifest id must be {id:?}, got {:?}", ledger.id));
                    }
                    if enabled {
                        violations.extend(goals_manifest::validate_milestone_ledger(&ledger));
                    }
                }
                Err(err) => violations.push(format!("{doc}.manifest milestone ledger: {err}")),
            },
            "lane_routing" => {
                if enabled {
                    let ownership =
                        validate_program_manifest(root, &program_table, "", stats, violations);
                    stats.lanes += ownership.len();
                    for owned in ownership {
                        match load_table(root, &owned.manifest) {
                            Ok(lane_table) => validate_lane_manifest(
                                root,
                                &lane_table,
                                &owned,
                                id,
                                stats,
                                violations,
                            ),
                            Err(err) => violations.push(format!(
                                "{doc}.manifest lane_ownership[{}]: failed to load {}: {err}",
                                owned.lane, owned.manifest
                            )),
                        }
                    }
                }
            }
            _ => {}
        }
    }
    stats.program = "portfolio".to_owned();
    stats.lane = "portfolio".to_owned();
    if stats.enabled_programs == 0 {
        violations.push(format!("{ACTIVE_GOAL_PATH}: portfolio must enable at least one program"));
    }
    if stats.programs > 0 {
        stats.repo = EXPECTED_REPOSITORY.to_owned();
    }
}

fn load_table(root: &Path, relative_path: &str) -> Result<Table> {
    let text = fs::read_to_string(root.join(relative_path))
        .with_context(|| format!("failed to read {relative_path}"))?;
    let value: Value =
        toml::from_str(&text).with_context(|| format!("failed to parse {relative_path}"))?;
    match value {
        Value::Table(table) => Ok(table),
        _ => Err(color_eyre::eyre::eyre!("expected TOML table")),
    }
}

/// Validates the small `active.toml` pointer. Returns the resolved
/// `(program manifest path, board path)` when both are well-formed
/// repo-relative existing paths.
fn validate_pointer(
    root: &Path,
    table: &Table,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) -> Option<(String, String)> {
    match table.get("schema").and_then(Value::as_integer) {
        Some(2) => {}
        Some(other) => {
            violations.push(format!("{ACTIVE_GOAL_PATH}: schema must be 2, got {other}"))
        }
        None => violations.push(format!("{ACTIVE_GOAL_PATH}: schema must be an integer")),
    }

    require_non_empty_string(ACTIVE_GOAL_PATH, table, "active_program", violations);
    require_non_empty_string(ACTIVE_GOAL_PATH, table, "active_lane", violations);

    if let Some(program) = string_field(table, "active_program") {
        stats.program = program.to_owned();
        // `active_program` was never bare-id-checked (see #3692 defect
        // 4): it flows into `validate_default_program`'s
        // `default_program == expected_program` comparison below, and a
        // path-shaped value there previously bypassed the path-injection
        // check entirely when both fields shared the same value.
        if !program.trim().is_empty() && !is_bare_program_id(program) {
            violations.push(format!(
                "{ACTIVE_GOAL_PATH}: active_program must be a bare program id (no path separators or \"..\"), got {program:?}"
            ));
        }
    }

    // Pointer-only invariant: active.toml must not carry work items itself
    // (schema 1 mixed program definition, routing, and ~65 completed work
    // items into this file; schema 2 relocates all of that).
    if table.contains_key("work_item") {
        violations.push(format!(
            "{ACTIVE_GOAL_PATH}: must not contain [[work_item]] entries (pointer-only in schema 2)"
        ));
    }

    let Some(program) = table.get("program").and_then(Value::as_table) else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: [program] table is required"));
        return None;
    };

    let manifest_path = match string_field(program, "manifest") {
        Some(path) => {
            validate_relative_existing_path(
                root,
                ACTIVE_GOAL_PATH,
                "manifest",
                path,
                stats,
                violations,
            );
            Some(path.to_owned())
        }
        None => {
            violations
                .push(format!("{ACTIVE_GOAL_PATH}: [program].manifest must be a string path"));
            None
        }
    };

    let board_path = match string_field(program, "board") {
        Some(path) => {
            validate_relative_existing_path(
                root,
                ACTIVE_GOAL_PATH,
                "board",
                path,
                stats,
                violations,
            );
            Some(path.to_owned())
        }
        None => {
            violations.push(format!("{ACTIVE_GOAL_PATH}: [program].board must be a string path"));
            None
        }
    };

    match table.get("authority").and_then(Value::as_table) {
        Some(authority) => {
            match string_field(authority, "work_items") {
                Some("github") => {}
                Some(other) => violations.push(format!(
                    "{ACTIVE_GOAL_PATH}: [authority].work_items must be \"github\", got {other:?}"
                )),
                None => violations
                    .push(format!("{ACTIVE_GOAL_PATH}: [authority].work_items must be a string")),
            }
            match string_field(authority, "receipts") {
                Some("machine") => {}
                Some(other) => violations.push(format!(
                    "{ACTIVE_GOAL_PATH}: [authority].receipts must be \"machine\", got {other:?}"
                )),
                None => violations
                    .push(format!("{ACTIVE_GOAL_PATH}: [authority].receipts must be a string")),
            }
        }
        None => violations.push(format!("{ACTIVE_GOAL_PATH}: [authority] table is required")),
    }

    manifest_path.zip(board_path)
}

/// Validates the M3 (#3624) `default_program` extension to the schema-2
/// pointer. Optional: absent `default_program` is not itself a violation
/// (older schema-2 `active.toml` files without it remain valid; callers
/// fall back to `active_program`). When present and naming a program other
/// than `active_program`, that sibling manifest must exist and, if it is a
/// milestone-ledger program (contains `[[milestone]]`), must pass the
/// shared `goals::manifest::validate_milestone_ledger` structural checks —
/// keeping this validator and the `goals next` selector from drifting on
/// what a valid ledger looks like.
fn validate_default_program(
    root: &Path,
    pointer_table: &Table,
    expected_program: &str,
    violations: &mut Vec<String>,
) {
    let Some(raw) = pointer_table.get("default_program") else {
        return;
    };
    let Some(default_program) = raw.as_str() else {
        violations.push(format!("{ACTIVE_GOAL_PATH}: default_program must be a string"));
        return;
    };
    if default_program.trim().is_empty() {
        violations.push(format!("{ACTIVE_GOAL_PATH}: default_program must not be empty"));
        return;
    }

    // Bare-id + on-disk-existence check, shared with `goals next`'s live
    // resolution of `default_program` (`snapshot::build_snapshot`) via
    // `goals_manifest::validate_program_id` — the ONE place this check
    // lives, so the static validator here and the live selector can never
    // drift on what counts as a safe, known program id (#3647 follow-up,
    // #3697).
    //
    // This MUST run before the equality-with-`active_program` shortcut
    // below (see #3692 defect 4): `default_program` is interpolated
    // directly into `program_manifest_path` further down, so a
    // path-shaped value could otherwise escape
    // `.perl-lsp/goals/programs/`. Previously, when `default_program`
    // happened to equal `active_program` (both reader-controlled
    // fields), the equality check returned early and this validation
    // never ran for EITHER field — `active_program` itself was never
    // bare-id-checked at all (closed separately below, in
    // `validate_pointer`). Running this first closes that bypass
    // regardless of which field(s) the hostile value came from.
    if let Err(reason) = goals_manifest::validate_program_id(root, default_program) {
        violations.push(format!("{ACTIVE_GOAL_PATH}: default_program {reason}"));
        return;
    }

    if default_program == expected_program {
        return;
    }

    let manifest_path = goals_manifest::program_manifest_path(default_program);
    match fs::read_to_string(root.join(&manifest_path)) {
        Ok(text) if goals_manifest::is_milestone_ledger(&text) => {
            match goals_manifest::load_milestone_ledger(root, &manifest_path) {
                Ok(ledger) => {
                    violations.extend(goals_manifest::validate_milestone_ledger(&ledger));
                }
                Err(err) => violations.push(format!(
                    "{ACTIVE_GOAL_PATH}: default_program {default_program:?} manifest failed to parse as a milestone ledger: {err}"
                )),
            }
        }
        Ok(_) => {
            // A non-milestone-ledger sibling program (e.g. another
            // lane-routing program). Existence is already confirmed above;
            // full structural validation of that shape is
            // `validate_program_manifest`'s job when it IS `active_program`.
        }
        Err(err) => violations.push(format!(
            "{ACTIVE_GOAL_PATH}: default_program {default_program:?}: failed to read {manifest_path}: {err}"
        )),
    }
}

struct LaneOwnership {
    lane: String,
    pr_cap: i64,
    manifest: String,
}

/// Validates the durable program manifest and returns its declared lane
/// ownership entries.
fn validate_program_manifest(
    root: &Path,
    table: &Table,
    expected_lane: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) -> Vec<LaneOwnership> {
    let doc = "program manifest";

    for field in PROGRAM_REQUIRED_TOP_LEVEL_STRINGS {
        require_non_empty_string(doc, table, field, violations);
    }
    require_non_empty_string(doc, table, "objective", violations);
    for field in PROGRAM_REQUIRED_TEXT_ARRAYS {
        validate_non_empty_string_array(doc, table, field, violations);
    }
    for field in PROGRAM_REQUIRED_PATH_FIELDS {
        match string_field(table, field) {
            Some(path) => {
                validate_relative_existing_path(root, doc, field, path, stats, violations);
            }
            None => violations.push(format!("{doc}: {field} must be a string path")),
        }
    }
    for field in PROGRAM_REQUIRED_PATH_ARRAYS {
        validate_path_array(root, doc, table, field, stats, violations);
    }

    validate_current(doc, table, expected_lane, stats, violations);
    let limits = validate_limits(doc, table, violations);
    let lane_ownership = validate_lane_ownership(root, doc, table, &limits, stats, violations);
    validate_selection_priority(doc, table, &lane_ownership, violations);

    let declared_lanes: BTreeSet<String> =
        lane_ownership.iter().map(|owned| owned.lane.clone()).collect();
    validate_work_items(root, doc, table, &declared_lanes, stats, violations, false);

    lane_ownership
}

fn validate_current(
    doc: &str,
    table: &Table,
    expected_lane: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let Some(current) = table.get("current").and_then(Value::as_table) else {
        violations.push(format!("{doc}: [current] table is required"));
        return;
    };

    for field in ["lane", "repo", "release_lineage_repo", "status"] {
        require_non_empty_string("[current]", current, field, violations);
    }

    if let Some(repo) = string_field(current, "repo") {
        stats.repo = repo.to_owned();
        if repo != EXPECTED_REPOSITORY {
            violations.push(format!("{doc}: [current].repo must be \"{EXPECTED_REPOSITORY}\""));
        }
    }

    if let Some(lineage_repo) = string_field(current, "release_lineage_repo")
        && lineage_repo != "perl-lsp"
    {
        violations.push(format!("{doc}: [current].release_lineage_repo must be \"perl-lsp\""));
    }

    if let Some(lane) = string_field(current, "lane") {
        stats.lane = lane.to_owned();
        // Cross-check the pointer's active_lane against the durable program
        // manifest's own idea of the active lane, so a stale/mistyped
        // active.toml.active_lane fails loudly instead of silently drifting
        // from [current].lane (#3612 M2 review: sourcery, cubic).
        if !expected_lane.is_empty() && lane != expected_lane {
            violations.push(format!(
                "{doc}: [current].lane {lane:?} does not match {ACTIVE_GOAL_PATH} active_lane {expected_lane:?}"
            ));
        }
    }
}

fn validate_limits(
    doc: &str,
    table: &Table,
    violations: &mut Vec<String>,
) -> BTreeMap<String, i64> {
    let mut limits = BTreeMap::new();
    let Some(limit_table) = table.get("limits").and_then(Value::as_table) else {
        violations.push(format!("{doc}: [limits] table is required"));
        return limits;
    };

    for (field, lane) in
        [("trust_prs", "trust"), ("substrate_prs", "substrate"), ("reliability_prs", "reliability")]
    {
        match limit_table.get(field).and_then(Value::as_integer) {
            Some(value) if value > 0 => {
                limits.insert(lane.to_owned(), value);
            }
            Some(_) => violations.push(format!("{doc}: [limits].{field} must be positive")),
            None => violations.push(format!("{doc}: [limits].{field} must be an integer")),
        }
    }

    limits
}

fn validate_lane_ownership(
    root: &Path,
    doc: &str,
    table: &Table,
    limits: &BTreeMap<String, i64>,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) -> Vec<LaneOwnership> {
    let mut owned = Vec::new();
    let mut seen = BTreeSet::new();

    let Some(items) = table.get("lane_ownership").and_then(Value::as_array) else {
        violations.push(format!("{doc}: [[lane_ownership]] entries are required"));
        return owned;
    };

    for (index, item) in items.iter().enumerate() {
        let entry_doc = format!("{doc}: lane_ownership[{index}]");
        let Some(entry) = item.as_table() else {
            violations.push(format!("{entry_doc} must be a TOML table"));
            continue;
        };

        require_non_empty_string(&entry_doc, entry, "lane", violations);
        // Validated as a repo-relative existing path (not just a non-empty
        // string) so an absolute or out-of-repo lane manifest path is
        // rejected before load_table ever reads it as trusted schema input
        // (#3612 M2 review: chatgpt-codex, cubic).
        match string_field(entry, "manifest") {
            Some(path) => {
                validate_relative_existing_path(
                    root, &entry_doc, "manifest", path, stats, violations,
                );
            }
            None => violations.push(format!("{entry_doc}: manifest must be a string path")),
        }

        let Some(lane) = string_field(entry, "lane") else {
            continue;
        };
        if !EXPECTED_LANES.contains(&lane) {
            violations.push(format!("{entry_doc}: unknown lane id {lane:?}"));
        }
        if !seen.insert(lane.to_owned()) {
            violations.push(format!("{entry_doc}: duplicate lane id {lane:?}"));
        }

        let pr_cap = match entry.get("pr_cap").and_then(Value::as_integer) {
            Some(value) if value > 0 => {
                if let Some(expected) = limits.get(lane)
                    && value != *expected
                {
                    violations.push(format!(
                        "{entry_doc}: pr_cap {value} does not match [limits] value {expected}"
                    ));
                }
                value
            }
            Some(_) => {
                violations.push(format!("{entry_doc}: pr_cap must be positive"));
                0
            }
            None => {
                violations.push(format!("{entry_doc}: pr_cap must be an integer"));
                0
            }
        };

        let Some(manifest) = string_field(entry, "manifest") else {
            continue;
        };

        owned.push(LaneOwnership { lane: lane.to_owned(), pr_cap, manifest: manifest.to_owned() });
    }

    for lane in EXPECTED_LANES {
        if !seen.contains(*lane) {
            violations.push(format!("{doc}: missing {lane:?} lane_ownership entry"));
        }
    }

    owned
}

fn validate_selection_priority(
    doc: &str,
    table: &Table,
    lane_ownership: &[LaneOwnership],
    violations: &mut Vec<String>,
) {
    let known: BTreeSet<&str> = lane_ownership.iter().map(|owned| owned.lane.as_str()).collect();
    validate_non_empty_string_array(doc, table, "selection_priority", violations);
    let Some(values) = table.get("selection_priority").and_then(Value::as_array) else {
        return;
    };
    for (index, value) in values.iter().enumerate() {
        if let Some(lane) = value.as_str()
            && !lane.trim().is_empty()
            && !known.contains(lane)
        {
            violations
                .push(format!("{doc}: selection_priority[{index}] {lane:?} is not an owned lane"));
        }
    }
}

fn validate_lane_manifest(
    root: &Path,
    table: &Table,
    owned: &LaneOwnership,
    expected_program: &str,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
) {
    let doc = format!("lane manifest \"{}\"", owned.lane);

    for field in LANE_REQUIRED_TOP_LEVEL_STRINGS {
        require_non_empty_string(&doc, table, field, violations);
    }
    require_non_empty_string(&doc, table, "must_route_elsewhere", violations);
    for field in LANE_REQUIRED_TEXT_ARRAYS {
        validate_non_empty_string_array(&doc, table, field, violations);
    }

    match string_field(table, "board") {
        Some(path) => {
            validate_relative_existing_path(root, &doc, "board", path, stats, violations);
        }
        None => violations.push(format!("{doc}: board must be a string path")),
    }

    if let Some(id) = string_field(table, "id") {
        if id != owned.lane {
            violations.push(format!(
                "{doc}: id {id:?} does not match lane_ownership lane {:?}",
                owned.lane
            ));
        }
        if !EXPECTED_LANES.contains(&id) {
            violations.push(format!("{doc}: unknown lane id {id:?}"));
        }
    }

    // Cross-check the lane manifest's own "program" field against the
    // pointer's active_program. This is the program manifest's short slug
    // (distinct from the program manifest's globally-namespaced "id"), and
    // is already required to be populated consistently across every lane a
    // program owns, so a mismatch here is a real dangling/mistyped
    // pointer-to-program reference (#3612 M2 review: factory-droid P1).
    if let Some(program) = string_field(table, "program")
        && !expected_program.is_empty()
        && program != expected_program
    {
        violations.push(format!(
            "{doc}: program {program:?} does not match {ACTIVE_GOAL_PATH} active_program {expected_program:?}"
        ));
    }

    match table.get("pr_cap").and_then(Value::as_integer) {
        Some(value) if value > 0 => {
            if value != owned.pr_cap {
                violations.push(format!(
                    "{doc}: pr_cap {value} does not match program lane_ownership pr_cap {}",
                    owned.pr_cap
                ));
            }
        }
        Some(_) => violations.push(format!("{doc}: pr_cap must be positive")),
        None => violations.push(format!("{doc}: pr_cap must be an integer")),
    }
}

// Each parameter is a distinct manifest contract point (root for path
// resolution, lanes/stats/violations accumulators, and the allow_completed
// flag distinguishing active vs. archived work-item validation); splitting
// this into a struct would obscure the single validate_work_items call site
// rather than clarify it (#3612 M2 review: chatgpt-codex).
#[allow(clippy::too_many_arguments)]
fn validate_work_items(
    root: &Path,
    doc: &str,
    table: &Table,
    lanes: &BTreeSet<String>,
    stats: &mut ManifestStats,
    violations: &mut Vec<String>,
    allow_completed: bool,
) {
    let Some(items) = table.get("work_item").and_then(Value::as_array) else {
        violations.push(format!("{doc}: [[work_item]] entries are required"));
        return;
    };

    let mut ids = BTreeSet::new();
    let mut program_active_work_items = 0;
    for (index, item) in items.iter().enumerate() {
        let item_doc = format!("{doc}: work_item[{index}]");
        let Some(item_table) = item.as_table() else {
            violations.push(format!("{item_doc} must be a TOML table"));
            continue;
        };

        stats.work_items += 1;
        for field in ["id", "status", "lane", "claim_boundary"] {
            require_non_empty_string(&item_doc, item_table, field, violations);
        }

        if let Some(id) = string_field(item_table, "id")
            && !ids.insert(id.to_owned())
        {
            violations.push(format!("{item_doc}: duplicate work item id {id:?}"));
        }

        if let Some(status) = string_field(item_table, "status") {
            if !ALLOWED_WORK_ITEM_STATUSES.contains(&status) {
                violations.push(format!("{item_doc}: unsupported status {status:?}"));
            }
            if status == "active" {
                stats.active_work_items += 1;
                program_active_work_items += 1;
            }
            if status == "completed" {
                stats.completed_work_items += 1;
                if !allow_completed {
                    violations.push(format!(
                        "{item_doc}: completed work items must live under .perl-lsp/goals/archive/, not the active program manifest"
                    ));
                }
            }
        }

        if let Some(lane) = string_field(item_table, "lane")
            && !lanes.contains(lane)
        {
            violations.push(format!("{item_doc}: lane {lane:?} is not an owned lane"));
        }

        validate_optional_path_array(root, &item_doc, item_table, "files", stats, violations);
        validate_optional_command_array(&item_doc, item_table, "commands", stats, violations);
    }

    if !allow_completed && program_active_work_items == 0 {
        violations.push(format!("{doc}: at least one work_item must be active"));
    }
}

fn validate_path_array(
    root: &Path,
    doc: &str,
    table: &Table,
    field: &str,
    stats: &mut ManifestStats,
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
        if command.split_whitespace().next() == Some("rtk") {
            violations.push(format!("{doc}: {field}[{index}] must be a direct repository command"));
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
) -> bool {
    if path.trim().is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
        return false;
    }
    if Path::new(path).is_absolute()
        || path.contains(':')
        || path.contains('\\')
        || Path::new(path).components().any(|component| component == Component::ParentDir)
    {
        violations.push(format!("{doc}: {field} must be a repo-relative slash path: {path}"));
        return false;
    }
    if !root.join(path).exists() {
        violations.push(format!("{doc}: {field} points to missing path {path}"));
        return false;
    }
    stats.path_references += 1;
    true
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

/// A bare program id must not contain path separators or `..`: it is
/// interpolated unvalidated into `goals::manifest::program_manifest_path`
/// (`.perl-lsp/goals/programs/<id>.toml`) by both `active_program` and
/// `default_program` call sites, so either field accepting a path-shaped
/// value would let it escape that directory (see #3692 defect 4).
fn is_bare_program_id(value: &str) -> bool {
    !value.contains('/') && !value.contains('\\') && !value.contains(':') && !value.contains("..")
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

    fn portfolio_table(programs: Vec<Value>) -> Table {
        let mut table = Table::new();
        table.insert("schema".to_owned(), Value::Integer(3));
        table.insert("mode".to_owned(), Value::String("portfolio".to_owned()));

        let mut authority = Table::new();
        authority.insert("work_items".to_owned(), Value::String("github".to_owned()));
        authority.insert("specs".to_owned(), Value::String("allow".to_owned()));
        authority.insert("receipts".to_owned(), Value::String("machine".to_owned()));
        table.insert("authority".to_owned(), Value::Table(authority));

        let mut selection = Table::new();
        selection.insert("strategy".to_owned(), Value::String("eligible_portfolio".to_owned()));
        for field in [
            "require_explicit_claim",
            "respect_lane_caps",
            "respect_dependencies",
            "respect_conflict_surfaces",
        ] {
            selection.insert(field.to_owned(), Value::Boolean(true));
        }
        table.insert("selection".to_owned(), Value::Table(selection));
        table.insert("program".to_owned(), Value::Array(programs));
        table
    }

    fn portfolio_program(id: &str, kind: &str, enabled: bool) -> Value {
        let mut program = Table::new();
        program.insert("id".to_owned(), Value::String(id.to_owned()));
        program.insert(
            "manifest".to_owned(),
            Value::String(goals_manifest::program_manifest_path(id)),
        );
        program.insert("kind".to_owned(), Value::String(kind.to_owned()));
        program.insert("enabled".to_owned(), Value::Boolean(enabled));
        Value::Table(program)
    }

    #[test]
    fn active_goal_manifest_accepts_current_contract() -> Result<()> {
        let stats = validate(&project_root()?)?;

        assert_eq!(stats.repo, EXPECTED_REPOSITORY);
        assert_eq!(stats.lane, "portfolio");
        assert_eq!(stats.program, "portfolio");
        assert_eq!(stats.programs, 2);
        assert_eq!(stats.enabled_programs, 2);
        assert_eq!(stats.lanes, 3);
        assert_eq!(stats.active_work_items, 1);
        assert_eq!(stats.completed_work_items, 0);

        Ok(())
    }

    #[test]
    fn portfolio_warns_when_legacy_selection_fields_are_present() -> Result<()> {
        let root = fixture_root(&[".perl-lsp/goals/programs/p.toml"])?;
        fs::write(
            root.path().join(".perl-lsp/goals/programs/p.toml"),
            "id = \"p\"\n\n[[milestone]]\nid = \"M1\"\ntitle = \"one\"\nstatus = \"completed\"\nexit_criteria = \"done\"\n",
        )?;
        let mut table = Table::new();
        table.insert("schema".to_owned(), Value::Integer(3));
        table.insert("mode".to_owned(), Value::String("portfolio".to_owned()));
        table.insert("active_program".to_owned(), Value::String("p".to_owned()));
        table.insert("active_lane".to_owned(), Value::String("legacy".to_owned()));
        table.insert("default_program".to_owned(), Value::String("p".to_owned()));
        let mut authority = Table::new();
        authority.insert("work_items".to_owned(), Value::String("github".to_owned()));
        authority.insert("specs".to_owned(), Value::String("allow".to_owned()));
        authority.insert("receipts".to_owned(), Value::String("machine".to_owned()));
        table.insert("authority".to_owned(), Value::Table(authority));
        let mut selection = Table::new();
        selection.insert("strategy".to_owned(), Value::String("eligible_portfolio".to_owned()));
        for field in [
            "require_explicit_claim",
            "respect_lane_caps",
            "respect_dependencies",
            "respect_conflict_surfaces",
        ] {
            selection.insert(field.to_owned(), Value::Boolean(true));
        }
        table.insert("selection".to_owned(), Value::Table(selection));
        let mut program = Table::new();
        program.insert("id".to_owned(), Value::String("p".to_owned()));
        program.insert(
            "manifest".to_owned(),
            Value::String(".perl-lsp/goals/programs/p.toml".to_owned()),
        );
        program.insert("kind".to_owned(), Value::String("milestone_ledger".to_owned()));
        program.insert("enabled".to_owned(), Value::Boolean(true));
        table.insert("program".to_owned(), Value::Array(vec![Value::Table(program)]));

        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();
        validate_portfolio(root.path(), &table, &mut stats, &mut violations);

        assert!(violations.is_empty(), "legacy fields should warn, not fail: {violations:?}");
        assert_eq!(stats.warnings.len(), 3);
        assert!(stats.warnings.iter().all(|warning| warning.contains("ignored")));
        Ok(())
    }

    #[test]
    fn portfolio_requires_unique_ids_and_known_program_kinds() -> Result<()> {
        let root = fixture_root(&[".perl-lsp/goals/programs/p.toml"])?;
        fs::write(
            root.path().join(".perl-lsp/goals/programs/p.toml"),
            "id = \"p\"\n\n[[milestone]]\nid = \"M1\"\ntitle = \"one\"\nstatus = \"completed\"\nexit_criteria = \"done\"\n",
        )?;
        let table = portfolio_table(vec![
            portfolio_program("p", "milestone_ledger", true),
            portfolio_program("p", "unknown", true),
        ]);
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_portfolio(root.path(), &table, &mut stats, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("duplicates program")),
            "expected duplicate id violation, got {violations:?}"
        );
        assert!(
            violations.iter().any(|violation| violation.contains("kind must be one of")),
            "expected unknown kind violation, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn portfolio_rejects_a_kind_that_does_not_match_manifest_shape() -> Result<()> {
        let root = fixture_root(&[".perl-lsp/goals/programs/p.toml"])?;
        fs::write(
            root.path().join(".perl-lsp/goals/programs/p.toml"),
            "id = \"p\"\n\n[[milestone]]\nid = \"M1\"\ntitle = \"one\"\nstatus = \"completed\"\nexit_criteria = \"done\"\n",
        )?;
        let table = portfolio_table(vec![portfolio_program("p", "lane_routing", true)]);
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_portfolio(root.path(), &table, &mut stats, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("does not match manifest shape")),
            "expected kind mismatch violation, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn disabled_programs_do_not_contribute_active_totals() -> Result<()> {
        let root = fixture_root(&[".perl-lsp/goals/programs/p.toml"])?;
        fs::write(root.path().join(".perl-lsp/goals/programs/p.toml"), "id = \"p\"\n")?;
        let table = portfolio_table(vec![portfolio_program("p", "lane_routing", false)]);
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_portfolio(root.path(), &table, &mut stats, &mut violations);

        assert_eq!(stats.programs, 1);
        assert_eq!(stats.enabled_programs, 0);
        assert_eq!(stats.lanes, 0);
        assert_eq!(stats.work_items, 0);
        assert_eq!(stats.active_work_items, 0);
        assert_eq!(stats.completed_work_items, 0);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("must enable at least one program")),
            "expected zero-enabled violation, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn empty_portfolio_reports_one_missing_program_violation() -> Result<()> {
        let root = fixture_root(&[])?;
        let table = portfolio_table(Vec::new());
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_portfolio(root.path(), &table, &mut stats, &mut violations);

        let missing_program_count = violations
            .iter()
            .filter(|violation| violation.contains("at least one [[program]] entry is required"))
            .count();
        assert_eq!(
            missing_program_count, 1,
            "expected one missing-program violation, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn portfolio_manifest_parent_path_is_rejected_before_opening() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("root");
        fs::create_dir_all(root.join(".perl-lsp/goals/programs"))?;
        fs::write(temp.path().join("outside.toml"), "not = [")?;

        let mut table = portfolio_table(vec![portfolio_program("p", "milestone_ledger", true)]);
        let program = table
            .get_mut("program")
            .and_then(Value::as_array_mut)
            .and_then(|programs| programs.first_mut())
            .and_then(Value::as_table_mut)
            .ok_or_else(|| color_eyre::eyre::eyre!("portfolio fixture missing program"))?;
        program.insert("manifest".to_owned(), Value::String("../outside.toml".to_owned()));

        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();
        validate_portfolio(&root, &table, &mut stats, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("repo-relative slash path")),
            "expected parent path rejection, got {violations:?}"
        );
        assert!(
            !violations
                .iter()
                .any(|violation| violation.contains("failed to parse ../outside.toml")),
            "parent path must not be opened, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_missing_schema_and_work_items_in_pointer() -> Result<()> {
        let root = fixture_root(&["programs/p.toml", "docs/board.md"])?;
        let mut table = Table::new();
        table.insert("active_program".to_owned(), Value::String(" ".to_owned()));
        table.insert("work_item".to_owned(), Value::Array(Vec::new()));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        let resolved = validate_pointer(root.path(), &table, &mut stats, &mut violations);

        assert!(resolved.is_none());
        for expected in [
            ".perl-lsp/goals/active.toml: schema must be an integer",
            ".perl-lsp/goals/active.toml: active_program must not be empty",
            ".perl-lsp/goals/active.toml: active_lane must be a string",
            ".perl-lsp/goals/active.toml: must not contain [[work_item]] entries (pointer-only in schema 2)",
            ".perl-lsp/goals/active.toml: [program] table is required",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_pointer_program_and_authority_contracts() -> Result<()> {
        let root = fixture_root(&["programs/p.toml"])?;
        let mut table = Table::new();
        table.insert("schema".to_owned(), Value::Integer(2));
        table.insert("active_program".to_owned(), Value::String("p".to_owned()));
        table.insert("active_lane".to_owned(), Value::String("p_lane".to_owned()));
        let mut program = Table::new();
        program.insert("manifest".to_owned(), Value::String("programs/p.toml".to_owned()));
        program.insert("board".to_owned(), Value::String("docs/missing-board.md".to_owned()));
        table.insert("program".to_owned(), Value::Table(program));
        let mut authority = Table::new();
        authority.insert("work_items".to_owned(), Value::String("spreadsheet".to_owned()));
        authority.insert("receipts".to_owned(), Value::Integer(1));
        table.insert("authority".to_owned(), Value::Table(authority));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        let resolved = validate_pointer(root.path(), &table, &mut stats, &mut violations);

        assert_eq!(
            resolved,
            Some(("programs/p.toml".to_owned(), "docs/missing-board.md".to_owned()))
        );
        for expected in [
            ".perl-lsp/goals/active.toml: board points to missing path docs/missing-board.md",
            ".perl-lsp/goals/active.toml: [authority].work_items must be \"github\", got \"spreadsheet\"",
            ".perl-lsp/goals/active.toml: [authority].receipts must be a string",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_program_manifest_shape_contracts() -> Result<()> {
        let root = fixture_root(&["docs/proposal.md", "docs/status.md"])?;
        let mut table = Table::new();
        for field in PROGRAM_REQUIRED_TOP_LEVEL_STRINGS {
            table.insert((*field).to_owned(), Value::String("present".to_owned()));
        }
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

        let lane_ownership =
            validate_program_manifest(root.path(), &table, "", &mut stats, &mut violations);

        ensure!(
            lane_ownership.is_empty(),
            "expected no lane ownership, got {} entries",
            lane_ownership.len()
        );
        for expected in [
            "program manifest: objective must not be empty",
            "program manifest: end_state[0] must not be empty",
            "program manifest: end_state[1] must be a string",
            "program manifest: claim_boundaries must be a non-empty array",
            "program manifest: plan points to missing path plans/missing.md",
            "program manifest: status_pointer must be a string path",
            "program manifest: operating_model must be a repo-relative slash path: docs\\operating.md",
            "program manifest: status_docs[1] must be a string",
            "program manifest: specs must not be empty",
            "program manifest: [current] table is required",
            "program manifest: [limits] table is required",
            "program manifest: [[lane_ownership]] entries are required",
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

        validate_current("program manifest", &table, "lane-a", &mut stats, &mut violations);

        ensure!(stats.repo == "wrong-repo", "got stats: {stats:?}");
        ensure!(stats.lane == "lane-a", "got stats: {stats:?}");
        ensure!(
            violations
                == vec![
                    "program manifest: [current].repo must be \"perl-lsp-swarm\"".to_owned(),
                    "program manifest: [current].release_lineage_repo must be \"perl-lsp\""
                        .to_owned(),
                ],
            "got violations: {violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_current_lane_mismatched_with_pointer_active_lane() -> Result<()>
    {
        let mut table = Table::new();
        let mut current = Table::new();
        current.insert("lane".to_owned(), Value::String("stale-lane".to_owned()));
        current.insert("repo".to_owned(), Value::String(EXPECTED_REPOSITORY.to_owned()));
        current.insert("release_lineage_repo".to_owned(), Value::String("perl-lsp".to_owned()));
        current.insert("status".to_owned(), Value::String("active".to_owned()));
        table.insert("current".to_owned(), Value::Table(current));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_current("program manifest", &table, "pointer-lane", &mut stats, &mut violations);

        assert_eq!(
            violations,
            vec![
                "program manifest: [current].lane \"stale-lane\" does not match .perl-lsp/goals/active.toml active_lane \"pointer-lane\"".to_owned()
            ]
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_lane_ownership_identity_and_cap_contracts() -> Result<()> {
        let limits = BTreeMap::from([
            ("trust".to_owned(), 1),
            ("substrate".to_owned(), 2),
            ("reliability".to_owned(), 3),
        ]);
        let mut missing_lane = Table::new();
        missing_lane.insert("manifest".to_owned(), Value::String("lanes/x.toml".to_owned()));
        missing_lane.insert("pr_cap".to_owned(), Value::Integer(1));
        let mut unknown = Table::new();
        unknown.insert("lane".to_owned(), Value::String("unknown".to_owned()));
        unknown.insert("manifest".to_owned(), Value::String("lanes/unknown.toml".to_owned()));
        unknown.insert("pr_cap".to_owned(), Value::Integer(1));
        let mut mismatched_trust = Table::new();
        mismatched_trust.insert("lane".to_owned(), Value::String("trust".to_owned()));
        mismatched_trust
            .insert("manifest".to_owned(), Value::String("lanes/trust.toml".to_owned()));
        mismatched_trust.insert("pr_cap".to_owned(), Value::Integer(2));
        let mut duplicate_trust = Table::new();
        duplicate_trust.insert("lane".to_owned(), Value::String("trust".to_owned()));
        duplicate_trust
            .insert("manifest".to_owned(), Value::String("lanes/trust2.toml".to_owned()));
        duplicate_trust.insert("pr_cap".to_owned(), Value::Integer(1));
        let mut bad_cap = Table::new();
        bad_cap.insert("lane".to_owned(), Value::String("substrate".to_owned()));
        bad_cap.insert("manifest".to_owned(), Value::String("lanes/substrate.toml".to_owned()));
        bad_cap.insert("pr_cap".to_owned(), Value::Integer(0));
        let mut missing_cap = Table::new();
        missing_cap.insert("lane".to_owned(), Value::String("reliability".to_owned()));
        missing_cap
            .insert("manifest".to_owned(), Value::String("lanes/reliability.toml".to_owned()));
        let mut table = Table::new();
        table.insert(
            "lane_ownership".to_owned(),
            Value::Array(vec![
                Value::Table(missing_lane),
                Value::Table(unknown),
                Value::Table(mismatched_trust),
                Value::Table(duplicate_trust),
                Value::Table(bad_cap),
                Value::Table(missing_cap),
            ]),
        );
        let root = fixture_root(&[
            "lanes/x.toml",
            "lanes/unknown.toml",
            "lanes/trust.toml",
            "lanes/trust2.toml",
            "lanes/substrate.toml",
            "lanes/reliability.toml",
        ])?;
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        let owned = validate_lane_ownership(
            root.path(),
            "program manifest",
            &table,
            &limits,
            &mut stats,
            &mut violations,
        );

        let owned_lanes: BTreeSet<String> = owned.iter().map(|o| o.lane.clone()).collect();
        ensure!(
            owned_lanes
                == BTreeSet::from([
                    "reliability".to_owned(),
                    "substrate".to_owned(),
                    "trust".to_owned(),
                    "unknown".to_owned(),
                ]),
            "got lanes: {owned_lanes:?}"
        );
        for expected in [
            "program manifest: lane_ownership[0]: lane must be a string",
            "program manifest: lane_ownership[1]: unknown lane id \"unknown\"",
            "program manifest: lane_ownership[2]: pr_cap 2 does not match [limits] value 1",
            "program manifest: lane_ownership[3]: duplicate lane id \"trust\"",
            "program manifest: lane_ownership[4]: pr_cap must be positive",
            "program manifest: lane_ownership[5]: pr_cap must be an integer",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing lane violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_out_of_repo_lane_ownership_manifest_path() -> Result<()> {
        let limits = BTreeMap::from([("trust".to_owned(), 1)]);
        let mut absolute = Table::new();
        absolute.insert("lane".to_owned(), Value::String("trust".to_owned()));
        absolute.insert("manifest".to_owned(), Value::String("C:/etc/lane.toml".to_owned()));
        absolute.insert("pr_cap".to_owned(), Value::Integer(1));
        let mut table = Table::new();
        table.insert("lane_ownership".to_owned(), Value::Array(vec![Value::Table(absolute)]));
        let root = fixture_root(&[])?;
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        let owned = validate_lane_ownership(
            root.path(),
            "program manifest",
            &table,
            &limits,
            &mut stats,
            &mut violations,
        );

        ensure!(
            owned.len() == 1,
            "expected the entry to still be recorded, got {} entries",
            owned.len()
        );
        ensure!(
            violations.iter().any(|violation| violation
                == "program manifest: lane_ownership[0]: manifest must be a repo-relative slash path: C:/etc/lane.toml"),
            "missing out-of-repo path violation; got {violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_lane_manifest_shape_contracts() -> Result<()> {
        let root = fixture_root(&[])?;
        let mut table = Table::new();
        table.insert("id".to_owned(), Value::String("substrate".to_owned()));
        table.insert("program".to_owned(), Value::String("p".to_owned()));
        table.insert("proof_policy".to_owned(), Value::String("policy".to_owned()));
        table.insert("must_route_elsewhere".to_owned(), Value::String(" ".to_owned()));
        table.insert("may_change".to_owned(), Value::Array(Vec::new()));
        table.insert("pr_cap".to_owned(), Value::Integer(5));
        let owned = LaneOwnership {
            lane: "trust".to_owned(),
            pr_cap: 2,
            manifest: "lanes/trust.toml".to_owned(),
        };
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_lane_manifest(root.path(), &table, &owned, "p", &mut stats, &mut violations);

        for expected in [
            "lane manifest \"trust\": must_route_elsewhere must not be empty",
            "lane manifest \"trust\": may_change must not be empty",
            "lane manifest \"trust\": next_items must be a non-empty array",
            "lane manifest \"trust\": board must be a string path",
            "lane manifest \"trust\": id \"substrate\" does not match lane_ownership lane \"trust\"",
            "lane manifest \"trust\": pr_cap 5 does not match program lane_ownership pr_cap 2",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_lane_manifest_program_mismatch_and_counts_board_stat()
    -> Result<()> {
        let root = fixture_root(&["docs/board.md"])?;
        let mut table = Table::new();
        table.insert("id".to_owned(), Value::String("trust".to_owned()));
        table.insert("program".to_owned(), Value::String("wrong_program".to_owned()));
        table.insert("proof_policy".to_owned(), Value::String("policy".to_owned()));
        table.insert("must_route_elsewhere".to_owned(), Value::String("elsewhere".to_owned()));
        table.insert("may_change".to_owned(), Value::Array(vec![Value::String("x".to_owned())]));
        table.insert("next_items".to_owned(), Value::Array(vec![Value::String("y".to_owned())]));
        table.insert("board".to_owned(), Value::String("docs/board.md".to_owned()));
        table.insert("pr_cap".to_owned(), Value::Integer(2));
        let owned = LaneOwnership {
            lane: "trust".to_owned(),
            pr_cap: 2,
            manifest: "lanes/trust.toml".to_owned(),
        };
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_lane_manifest(
            root.path(),
            &table,
            &owned,
            "real_perl_editor_trust",
            &mut stats,
            &mut violations,
        );

        // The lane manifest's board path must be counted into the aggregate
        // stats, not silently dropped by a locally-scoped ManifestStats
        // (#3612 M2 review: gemini, cubic).
        assert_eq!(stats.path_references, 1, "got stats: {stats:?}");
        assert_eq!(
            violations,
            vec![
                "lane manifest \"trust\": program \"wrong_program\" does not match .perl-lsp/goals/active.toml active_program \"real_perl_editor_trust\"".to_owned()
            ]
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_collection_shape_contracts() -> Result<()> {
        let root = fixture_root(&[])?;
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_optional_path_array(
            root.path(),
            "program manifest: work_item[0]",
            &Table::new(),
            "files",
            &mut stats,
            &mut violations,
        );
        validate_optional_command_array(
            "program manifest: work_item[0]",
            &Table::new(),
            "commands",
            &mut stats,
            &mut violations,
        );
        ensure!(violations.is_empty(), "got violations: {violations:?}");

        let mut path_table = Table::new();
        path_table.insert("status_docs".to_owned(), Value::String("docs/status.md".to_owned()));
        validate_path_array(
            root.path(),
            "program manifest",
            &path_table,
            "status_docs",
            &mut stats,
            &mut violations,
        );
        let mut optional_paths = Table::new();
        optional_paths.insert("files".to_owned(), Value::String("docs/status.md".to_owned()));
        validate_optional_path_array(
            root.path(),
            "program manifest: work_item[0]",
            &optional_paths,
            "files",
            &mut stats,
            &mut violations,
        );
        let mut optional_commands = Table::new();
        optional_commands.insert("commands".to_owned(), Value::String("rtk cargo test".to_owned()));
        validate_optional_command_array(
            "program manifest: work_item[0]",
            &optional_commands,
            "commands",
            &mut stats,
            &mut violations,
        );
        validate_relative_existing_path(
            root.path(),
            "program manifest",
            "proposal",
            " ",
            &mut stats,
            &mut violations,
        );
        let mut text_table = Table::new();
        text_table.insert("items".to_owned(), Value::Array(Vec::new()));
        validate_non_empty_string_array("doc", &text_table, "items", &mut violations);
        require_non_empty_string("doc", &Table::new(), "missing", &mut violations);

        for expected in [
            "program manifest: status_docs must be a non-empty array",
            "program manifest: work_item[0]: files must be an array when present",
            "program manifest: work_item[0]: commands must be an array when present",
            "program manifest: proposal must not be empty",
            "doc: items must not be empty",
            "doc: missing must be a string",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing collection violation {expected:?}; got {violations:?}"
            );
        }

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
            "program manifest: work_item[0]",
            &path_table,
            "files",
            &mut stats,
            &mut path_violations,
        );

        ensure!(stats.path_references == 1, "got stats: {stats:?}");
        ensure!(
            path_violations
                == vec![
                    "program manifest: work_item[0]: files[1] points to missing path docs/missing.md"
                        .to_owned(),
                    "program manifest: work_item[0]: files[2] must be a string".to_owned(),
                ],
            "got path violations: {path_violations:?}"
        );

        let mut command_table = Table::new();
        command_table.insert(
            "commands".to_owned(),
            Value::Array(vec![
                Value::String("cargo test -p xtask".to_owned()),
                Value::String(" ".to_owned()),
                Value::Integer(5),
            ]),
        );
        let mut command_violations = Vec::new();

        validate_optional_command_array(
            "program manifest: work_item[0]",
            &command_table,
            "commands",
            &mut stats,
            &mut command_violations,
        );

        ensure!(stats.proof_commands == 1, "got stats: {stats:?}");
        ensure!(
            command_violations
                == vec![
                    "program manifest: work_item[0]: commands[1] must not be empty".to_owned(),
                    "program manifest: work_item[0]: commands[2] must be a string".to_owned(),
                ],
            "got command violations: {command_violations:?}"
        );

        Ok(())
    }

    #[test]
    fn validate_default_program_rejects_path_like_ids() -> Result<()> {
        let root = fixture_root(&[])?;
        for hostile in ["../../etc/passwd", "programs/../secret", "a/b", "a\\b", "C:evil"] {
            let mut pointer_table = Table::new();
            pointer_table.insert("default_program".to_owned(), Value::String(hostile.to_owned()));
            let mut violations = Vec::new();

            validate_default_program(root.path(), &pointer_table, "expected", &mut violations);

            assert!(
                violations.iter().any(|v| v.contains("bare program id")),
                "expected a bare-program-id violation for {hostile:?}, got {violations:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn validate_default_program_rejects_path_like_id_even_when_it_equals_active_program()
    -> Result<()> {
        // Regression for #3692 defect 4: the bare-id/path-injection check
        // previously ran AFTER the `default_program == expected_program`
        // equality shortcut, so a hostile value shared by BOTH fields
        // bypassed validation entirely. Passing the same hostile string
        // as `expected_program` here reproduces exactly that shape.
        let root = fixture_root(&[])?;
        for hostile in ["../../etc/passwd", "programs/../secret", "a/b"] {
            let mut pointer_table = Table::new();
            pointer_table.insert("default_program".to_owned(), Value::String(hostile.to_owned()));
            let mut violations = Vec::new();

            validate_default_program(root.path(), &pointer_table, hostile, &mut violations);

            assert!(
                violations.iter().any(|v| v.contains("bare program id")),
                "expected a bare-program-id violation for {hostile:?} even when default_program == active_program, got {violations:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn validate_pointer_rejects_a_path_like_active_program() -> Result<()> {
        // Regression for #3692 defect 4: `active_program` was never
        // bare-id-checked at all prior to this fix.
        let root = fixture_root(&[])?;
        let mut table = Table::new();
        table.insert("schema".to_owned(), Value::Integer(2));
        table.insert("active_program".to_owned(), Value::String("../escape".to_owned()));
        table.insert("active_lane".to_owned(), Value::String("trust".to_owned()));
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_pointer(root.path(), &table, &mut stats, &mut violations);

        assert!(
            violations
                .iter()
                .any(|v| v.contains("active_program") && v.contains("bare program id")),
            "expected an active_program bare-id violation, got {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn validate_default_program_accepts_a_bare_known_milestone_ledger_id() -> Result<()> {
        // The real repo tree has a genuine milestone-ledger program
        // (agent_loop_enablement) — exercise the parsed-TOML
        // `is_milestone_ledger` classification against it end to end.
        let root = project_root()?;
        let mut pointer_table = Table::new();
        pointer_table.insert(
            "default_program".to_owned(),
            Value::String("agent_loop_enablement".to_owned()),
        );
        let mut violations = Vec::new();

        validate_default_program(&root, &pointer_table, "expected", &mut violations);

        assert_eq!(violations, Vec::<String>::new(), "got violations: {violations:?}");
        Ok(())
    }

    #[test]
    fn active_goal_manifest_reports_non_table_and_duplicate_work_items() -> Result<()> {
        let root = project_root()?;
        let lanes = BTreeSet::from(["trust".to_owned()]);
        let mut unsupported = Table::new();
        unsupported.insert("id".to_owned(), Value::String("wi-1".to_owned()));
        unsupported.insert("status".to_owned(), Value::String("surprise".to_owned()));
        unsupported.insert("lane".to_owned(), Value::String("trust".to_owned()));
        unsupported.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut duplicate = Table::new();
        duplicate.insert("id".to_owned(), Value::String("wi-1".to_owned()));
        duplicate.insert("status".to_owned(), Value::String("active".to_owned()));
        duplicate.insert("lane".to_owned(), Value::String("trust".to_owned()));
        duplicate.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut table = Table::new();
        table.insert(
            "work_item".to_owned(),
            Value::Array(vec![
                Value::String("not a table".to_owned()),
                Value::Table(unsupported),
                Value::Table(duplicate),
            ]),
        );
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_work_items(
            &root,
            "program manifest",
            &table,
            &lanes,
            &mut stats,
            &mut violations,
            false,
        );

        ensure!(stats.work_items == 2, "got stats: {stats:?}");
        ensure!(stats.active_work_items == 1, "got stats: {stats:?}");
        for expected in [
            "program manifest: work_item[0] must be a TOML table",
            "program manifest: work_item[1]: unsupported status \"surprise\"",
            "program manifest: work_item[2]: duplicate work item id \"wi-1\"",
        ] {
            ensure!(
                violations.iter().any(|violation| violation == expected),
                "missing work item violation {expected:?}; got {violations:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn active_work_requirement_is_checked_per_program() -> Result<()> {
        let root = project_root()?;
        let lanes = BTreeSet::from(["trust".to_owned()]);

        let mut active_item = Table::new();
        active_item.insert("id".to_owned(), Value::String("active".to_owned()));
        active_item.insert("status".to_owned(), Value::String("active".to_owned()));
        active_item.insert("lane".to_owned(), Value::String("trust".to_owned()));
        active_item.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut active_program = Table::new();
        active_program
            .insert("work_item".to_owned(), Value::Array(vec![Value::Table(active_item)]));

        let mut ready_item = Table::new();
        ready_item.insert("id".to_owned(), Value::String("ready".to_owned()));
        ready_item.insert("status".to_owned(), Value::String("ready".to_owned()));
        ready_item.insert("lane".to_owned(), Value::String("trust".to_owned()));
        ready_item.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut ready_program = Table::new();
        ready_program.insert("work_item".to_owned(), Value::Array(vec![Value::Table(ready_item)]));

        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();
        validate_work_items(
            &root,
            "program A",
            &active_program,
            &lanes,
            &mut stats,
            &mut violations,
            false,
        );
        validate_work_items(
            &root,
            "program B",
            &ready_program,
            &lanes,
            &mut stats,
            &mut violations,
            false,
        );

        assert_eq!(stats.active_work_items, 1);
        assert!(
            violations.iter().any(|v| { v == "program B: at least one work_item must be active" })
        );
        assert!(
            !violations.iter().any(|v| v == "program A: at least one work_item must be active")
        );
        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_completed_work_items_outside_archive() -> Result<()> {
        let root = project_root()?;
        let lanes = BTreeSet::from(["trust".to_owned()]);
        let mut completed = Table::new();
        completed.insert("id".to_owned(), Value::String("wi-1".to_owned()));
        completed.insert("status".to_owned(), Value::String("completed".to_owned()));
        completed.insert("lane".to_owned(), Value::String("trust".to_owned()));
        completed.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut active = Table::new();
        active.insert("id".to_owned(), Value::String("wi-2".to_owned()));
        active.insert("status".to_owned(), Value::String("active".to_owned()));
        active.insert("lane".to_owned(), Value::String("trust".to_owned()));
        active.insert("claim_boundary".to_owned(), Value::String("fixture".to_owned()));
        let mut table = Table::new();
        table.insert(
            "work_item".to_owned(),
            Value::Array(vec![Value::Table(completed), Value::Table(active)]),
        );
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_work_items(
            &root,
            "program manifest",
            &table,
            &lanes,
            &mut stats,
            &mut violations,
            false,
        );

        ensure!(stats.completed_work_items == 1, "got stats: {stats:?}");
        ensure!(
            violations.iter().any(|violation| violation
                == "program manifest: work_item[0]: completed work items must live under .perl-lsp/goals/archive/, not the active program manifest"),
            "got violations: {violations:?}"
        );

        Ok(())
    }

    #[test]
    fn active_goal_manifest_rejects_rtk_wrapped_proof_commands() -> Result<()> {
        let mut table = Table::new();
        table.insert(
            "commands".to_owned(),
            Value::Array(vec![
                Value::String("rtk cargo test -p xtask".to_owned()),
                Value::String("rtk\tgit diff --check".to_owned()),
            ]),
        );
        let mut stats = ManifestStats::default();
        let mut violations = Vec::new();

        validate_optional_command_array(
            "program manifest: work_item[0]",
            &table,
            "commands",
            &mut stats,
            &mut violations,
        );

        assert_eq!(stats.proof_commands, 2);
        assert_eq!(
            violations,
            vec![
                "program manifest: work_item[0]: commands[0] must be a direct repository command"
                    .to_owned(),
                "program manifest: work_item[0]: commands[1] must be a direct repository command"
                    .to_owned(),
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
            "program manifest",
            "proposal",
            "C:/tmp/proposal.md",
            &mut stats,
            &mut violations,
        );

        assert_eq!(
            violations,
            vec![
                "program manifest: proposal must be a repo-relative slash path: C:/tmp/proposal.md"
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

        validate_work_items(
            &root,
            "program manifest",
            &table,
            &lanes,
            &mut stats,
            &mut violations,
            true,
        );

        assert_eq!(
            violations,
            vec![
                "program manifest: work_item[0]: lane \"unknown\" is not an owned lane".to_owned(),
            ]
        );

        Ok(())
    }
}
