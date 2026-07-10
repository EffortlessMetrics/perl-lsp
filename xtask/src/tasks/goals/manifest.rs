//! Shared typed loader for the goal-selection schema surface introduced by
//! M3 (#3624): `active.toml`'s `default_program` pointer and milestone
//! ledgers (`.perl-lsp/goals/programs/<id>.toml` files shaped as
//! `[[milestone]]` entries rather than the lane-routing `[[work_item]]`
//! shape).
//!
//! This is the ONE typed parser for milestone-ledger schema, used by both
//! `active_goal_manifest::run()` (structural validation, extended for M3)
//! and `super::snapshot::build_snapshot` (live selection). Sharing it here
//! is what keeps the validator and the selector from drifting apart on
//! what a valid ledger looks like (the same failure class M2's
//! `active_goal_manifest.rs` guards against for the pointer/program/lane
//! chain).
//!
//! The pre-existing schema-2 pointer/program/lane chain
//! (`active_program`/`active_lane`/`[program]`/`[authority]`,
//! `[[work_item]]`, `[[lane_ownership]]`) is intentionally NOT re-typed
//! here: `active_goal_manifest.rs` already owns that surface with its own
//! extensive, already-passing test suite, and duplicating it would be the
//! exact "two sources of truth" drift #3612 exists to eliminate. Only the
//! genuinely new M3 surface (`default_program`, milestone ledgers) lives
//! in this module.

use color_eyre::eyre::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const ACTIVE_GOAL_PATH: &str = ".perl-lsp/goals/active.toml";

/// Minimal typed view of `active.toml` sufficient to resolve the governed
/// default program. Fields already validated structurally by
/// `active_goal_manifest::validate_pointer` are re-read here (not
/// re-validated) so `goals next` doesn't need to shell out to the
/// validator to learn `active_program`/`default_program`.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivePointerFile {
    #[serde(default)]
    pub active_program: String,
    /// Governed default program for `cargo xtask goals next` when no
    /// `--program` flag is given. Optional so schema-2 `active.toml` files
    /// written before M3 still parse; when absent, callers fall back to
    /// `active_program`.
    #[serde(default)]
    pub default_program: Option<String>,
}

pub fn load_active_pointer(root: &Path) -> Result<ActivePointerFile> {
    let text = fs::read_to_string(root.join(ACTIVE_GOAL_PATH))
        .with_context(|| format!("failed to read {ACTIVE_GOAL_PATH}"))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {ACTIVE_GOAL_PATH}"))
}

/// Repo-relative manifest path for a program id, by the same convention
/// `active.toml`'s `[program].manifest` already uses for
/// `real_perl_editor_trust`: `.perl-lsp/goals/programs/<id>.toml`.
pub fn program_manifest_path(id: &str) -> String {
    format!(".perl-lsp/goals/programs/{id}.toml")
}

/// The ONE program-id validator shared by `active_goal_manifest::run()`'s
/// static `validate_default_program` check and
/// `super::snapshot::build_snapshot`'s live resolution of `default_program`
/// (#3647 follow-up: the two had drifted, and the live selector path was
/// fail-open — a `default_program` value could reach `resolved_program`
/// unvalidated whenever no explicit `--program` was given).
///
/// Rejects any id containing a path separator (`/`, `\\`, `:`) or
/// parent-dir traversal (`..`): such a value would otherwise escape
/// `.perl-lsp/goals/programs/` when joined onto [`program_manifest_path`].
/// Then requires the resulting manifest path to actually exist on disk —
/// the same definition of "known program" `snapshot::discover_known_programs`
/// uses (it lists `.toml` files under that same directory), so this check
/// and known-programs membership can never disagree.
///
/// Returns `Ok(())` when `id` is safe to resolve as a program, `Err(reason)`
/// with a human-readable violation string otherwise (not a display value —
/// callers decide how to surface it: a validation violation line for the
/// static checker, or a fail-closed `None` for the live selector).
pub fn validate_program_id(root: &Path, id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("program id must not be empty".to_owned());
    }
    if id.contains('/') || id.contains('\\') || id.contains(':') || id.contains("..") {
        return Err(format!(
            "must be a bare program id (no path separators or \"..\"), got {id:?}"
        ));
    }
    let manifest_path = program_manifest_path(id);
    if !root.join(&manifest_path).exists() {
        return Err(format!("program {id:?} manifest not found at {manifest_path}"));
    }
    Ok(())
}

/// Detects whether a program manifest's TOML text declares a `[[milestone]]`
/// array-of-tables (a milestone ledger, `load_milestone_ledger`'s shape) as
/// opposed to the lane-routing `[[work_item]]` shape. Parses the TOML and
/// checks for a top-level `milestone` key rather than doing a raw substring
/// scan on the source text, so a `[[milestone]]` mention inside a comment or
/// string literal elsewhere in an unrelated manifest can't mis-classify it
/// as a ledger (and conversely a real ledger with unusual formatting can't
/// be missed). Malformed TOML is treated as "not a ledger" — the caller's
/// subsequent `load_milestone_ledger`/lane-routing parse will surface the
/// real parse error with better context.
pub fn is_milestone_ledger(text: &str) -> bool {
    toml::from_str::<toml::Value>(text)
        .ok()
        .and_then(|value| value.as_table().cloned())
        .is_some_and(|table| table.contains_key("milestone"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct MilestoneLedger {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tracker_issue: Option<u64>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(rename = "milestone", default)]
    pub milestones: Vec<MilestoneEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MilestoneEntry {
    pub id: String,
    pub title: String,
    /// `completed | in_progress | pending | blocked | deferred`.
    pub status: String,
    #[serde(default)]
    pub issue: Option<u64>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub exit_criteria: String,
}

pub const ALLOWED_MILESTONE_STATUSES: &[&str] =
    &["completed", "in_progress", "pending", "blocked", "deferred"];

pub fn load_milestone_ledger(root: &Path, relative_path: &str) -> Result<MilestoneLedger> {
    let text = fs::read_to_string(root.join(relative_path))
        .with_context(|| format!("failed to read {relative_path}"))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {relative_path}"))
}

/// Structural validation shared by `check-active-goal-manifest` and
/// `goals next`: non-empty/unique ids, known statuses, non-empty
/// `exit_criteria`, `depends_on` referencing an existing milestone id, and
/// an acyclic `depends_on` graph. Returns plain violation strings so
/// `active_goal_manifest.rs` can fold them into its existing
/// violations-accumulator pattern without a new error type.
pub fn validate_milestone_ledger(ledger: &MilestoneLedger) -> Vec<String> {
    let mut violations = Vec::new();
    let doc_prefix = if ledger.id.is_empty() {
        "milestone ledger".to_owned()
    } else {
        format!("milestone ledger {:?}", ledger.id)
    };

    if ledger.milestones.is_empty() {
        violations.push(format!("{doc_prefix}: must declare at least one [[milestone]]"));
        return violations;
    }

    let ids: BTreeSet<&str> = ledger.milestones.iter().map(|m| m.id.as_str()).collect();
    let mut seen = BTreeSet::new();

    for (index, milestone) in ledger.milestones.iter().enumerate() {
        let doc = format!("{doc_prefix}: milestone[{index}]");
        if milestone.id.trim().is_empty() {
            violations.push(format!("{doc}: id must not be empty"));
        } else if !seen.insert(milestone.id.as_str()) {
            violations.push(format!("{doc}: duplicate milestone id {:?}", milestone.id));
        }
        if milestone.title.trim().is_empty() {
            violations.push(format!("{doc}: title must not be empty"));
        }
        if !ALLOWED_MILESTONE_STATUSES.contains(&milestone.status.as_str()) {
            violations.push(format!("{doc}: unsupported status {:?}", milestone.status));
        }
        if milestone.status == "in_progress" && milestone.issue.is_none() {
            // The selector's active-work guard (select_next's precedence
            // rule 2) matches live open PRs against in-progress milestones
            // by issue number; an in-progress milestone with no issue is
            // invisible to that guard, silently breaking the single-flight
            // guarantee for exactly the manifest shape most likely to
            // trigger it (a milestone marked in_progress before its
            // tracking issue is filed). Fail closed at validation time
            // rather than let it reach the selector.
            violations.push(format!(
                "{doc}: status \"in_progress\" requires an issue number (the active-work guard cannot detect a live PR for in-progress work with no issue)"
            ));
        }
        if milestone.exit_criteria.trim().is_empty() {
            violations.push(format!("{doc}: exit_criteria must not be empty"));
        }
        for dep in &milestone.depends_on {
            if !ids.contains(dep.as_str()) {
                violations
                    .push(format!("{doc}: depends_on {dep:?} references unknown milestone id"));
            }
        }
    }

    if let Some(cycle) = detect_cycle(&ledger.milestones) {
        violations
            .push(format!("{doc_prefix}: depends_on graph has a cycle: {}", cycle.join(" -> ")));
    }

    violations
}

fn detect_cycle(milestones: &[MilestoneEntry]) -> Option<Vec<String>> {
    let deps: BTreeMap<&str, &[String]> =
        milestones.iter().map(|m| (m.id.as_str(), m.depends_on.as_slice())).collect();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Visiting,
        Done,
    }
    let mut marks: BTreeMap<&str, Mark> = BTreeMap::new();

    fn visit<'a>(
        node: &'a str,
        deps: &BTreeMap<&'a str, &'a [String]>,
        marks: &mut BTreeMap<&'a str, Mark>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        match marks.get(node) {
            Some(Mark::Done) => return None,
            Some(Mark::Visiting) => {
                stack.push(node.to_owned());
                return Some(stack.clone());
            }
            None => {}
        }
        marks.insert(node, Mark::Visiting);
        stack.push(node.to_owned());
        if let Some(children) = deps.get(node) {
            for child in children.iter() {
                if let Some(cycle) = visit(child.as_str(), deps, marks, stack) {
                    return Some(cycle);
                }
            }
        }
        stack.pop();
        marks.insert(node, Mark::Done);
        None
    }

    for id in deps.keys() {
        let mut stack = Vec::new();
        if let Some(cycle) = visit(id, &deps, &mut marks, &mut stack) {
            return Some(cycle);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger(milestones: Vec<MilestoneEntry>) -> MilestoneLedger {
        MilestoneLedger {
            id: "test-ledger".to_owned(),
            title: None,
            tracker_issue: None,
            non_goals: Vec::new(),
            milestones,
        }
    }

    fn entry(id: &str, status: &str, depends_on: &[&str]) -> MilestoneEntry {
        MilestoneEntry {
            id: id.to_owned(),
            title: format!("title-{id}"),
            status: status.to_owned(),
            issue: None,
            depends_on: depends_on.iter().map(|s| (*s).to_owned()).collect(),
            exit_criteria: "exit".to_owned(),
        }
    }

    fn entry_with_issue(id: &str, status: &str, issue: u64, depends_on: &[&str]) -> MilestoneEntry {
        MilestoneEntry { issue: Some(issue), ..entry(id, status, depends_on) }
    }

    #[test]
    fn accepts_a_well_formed_ledger() {
        let l = ledger(vec![
            entry("M2", "completed", &[]),
            entry_with_issue("M3", "in_progress", 3624, &["M2"]),
            entry("M4", "pending", &["M2"]),
        ]);
        assert_eq!(validate_milestone_ledger(&l), Vec::<String>::new());
    }

    #[test]
    fn rejects_in_progress_milestone_with_no_issue() {
        let l = ledger(vec![entry("M2", "in_progress", &[])]);
        let violations = validate_milestone_ledger(&l);
        assert!(
            violations.iter().any(|v| v.contains("in_progress") && v.contains("issue number")),
            "expected an in_progress/issue-number violation, got {violations:?}"
        );
    }

    #[test]
    fn rejects_unknown_status() {
        let l = ledger(vec![entry("M2", "surprise", &[])]);
        let violations = validate_milestone_ledger(&l);
        assert!(violations.iter().any(|v| v.contains("unsupported status")));
    }

    #[test]
    fn rejects_dangling_depends_on() {
        let l = ledger(vec![entry("M2", "pending", &["ghost"])]);
        let violations = validate_milestone_ledger(&l);
        assert!(violations.iter().any(|v| v.contains("references unknown milestone id")));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let l = ledger(vec![entry("M2", "pending", &[]), entry("M2", "pending", &[])]);
        let violations = validate_milestone_ledger(&l);
        assert!(violations.iter().any(|v| v.contains("duplicate milestone id")));
    }

    #[test]
    fn rejects_cycles() {
        let l = ledger(vec![entry("M2", "pending", &["M3"]), entry("M3", "pending", &["M2"])]);
        let violations = validate_milestone_ledger(&l);
        assert!(violations.iter().any(|v| v.contains("cycle")));
    }

    #[test]
    fn rejects_empty_ledger() {
        let l = ledger(Vec::new());
        let violations = validate_milestone_ledger(&l);
        assert!(violations.iter().any(|v| v.contains("at least one")));
    }

    #[test]
    fn is_milestone_ledger_detects_real_milestone_table() {
        let text = "id = \"x\"\n\n[[milestone]]\nid = \"M2\"\n";
        assert!(is_milestone_ledger(text));
    }

    #[test]
    fn is_milestone_ledger_ignores_mention_in_a_comment() {
        // A lane-routing program that merely *mentions* "[[milestone]]" in a
        // comment (e.g. explaining why it is NOT a milestone ledger) must
        // not be mis-classified as one.
        let text =
            "# this program does not use [[milestone]] entries\n\n[[work_item]]\nid = \"wi-1\"\n";
        assert!(!is_milestone_ledger(text));
    }

    #[test]
    fn is_milestone_ledger_false_for_malformed_toml() {
        assert!(!is_milestone_ledger("not valid toml {{{"));
    }

    #[test]
    fn validate_program_id_accepts_a_known_bare_id() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(programs_dir.join("known.toml"), "id = \"known\"\n")?;

        assert!(validate_program_id(temp.path(), "known").is_ok());
        Ok(())
    }

    #[test]
    fn validate_program_id_rejects_an_unknown_id() -> Result<()> {
        let temp = tempfile::tempdir()?;
        fs::create_dir_all(temp.path().join(".perl-lsp/goals/programs"))?;

        let err = validate_program_id(temp.path(), "not-a-real-program")
            .expect_err("unknown program id must fail validation");
        assert!(
            err.contains("not-a-real-program"),
            "expected the id to be named in the error, got {err}"
        );
        Ok(())
    }

    #[test]
    fn validate_program_id_rejects_path_traversal_tokens() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(programs_dir.join("known.toml"), "id = \"known\"\n")?;

        for bad in ["../../../etc/passwd", "sub/dir", "a\\b", "a:b", ".."] {
            let err = validate_program_id(temp.path(), bad)
                .expect_err("path-traversal-shaped id must fail validation");
            assert!(
                err.contains("bare program id"),
                "expected a bare-program-id violation for {bad:?}, got {err}"
            );
        }
        Ok(())
    }

    #[test]
    fn validate_program_id_rejects_empty_id() {
        let err = validate_program_id(Path::new("."), "").expect_err("empty id must fail");
        assert!(err.contains("must not be empty"));
    }
}
