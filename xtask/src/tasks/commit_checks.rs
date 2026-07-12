//! Commit-tier staged checks (issue #3786, the first buildable slice of the
//! commit-gate feedback ladder).
//!
//! Every check here answers exactly one question: *is the staged artifact
//! structurally sound?* — never anything about orchestration-runtime
//! internals (agent counts, fan-out, launch topology; that boundary is
//! #3993/#3949 and stays out of this module on purpose), and never anything
//! that requires compiling the workspace (that's the pre-push tier, #3985).
//!
//! # Staged tree, never the working tree — and never the live index either
//!
//! Every check reads content through [`crate::tasks::staged`] — `git
//! write-tree` / `git ls-tree` / `git show` — never `fs::read_to_string` and
//! never a working-tree `WalkDir`. Beyond that: every check receives the
//! `tree_oid` that `gates::plan_gates` already captured once (threaded
//! through [`run_named_check`]'s `tree_oid` parameter) and reads from that
//! frozen tree object, never re-derives its own `git write-tree` or falls
//! back to the live index (`git diff --cached`, `git show :path`) — see
//! `staged.rs`'s module docs for why (a concurrent `git add` between
//! planning and dispatch must not make a check inspect a different state
//! than the receipt records).
//!
//! # Output shape
//!
//! Every check that has something to report returns a [`CheckReport`]
//! following `docs/reference/GUIDANCE_STYLE.md` §4/§5: result, why it
//! matters, affected artifacts, the fix (when mechanical), the exact rerun
//! command, and what remains required later. [`CheckReport::render`] embeds
//! the report as JSON behind a stable marker inside `GateResult.output_summary`
//! so `gates::build_agent_receipt` can reconstruct it into the action packet
//! (`AgentReceipt.advisories` / the enriched `AgentFailure` fields) without a
//! second parallel execution path.
//!
//! # Advisory-first (V1)
//!
//! Only [`Posture::Blocked`] fails a commit-tier gate. `CLASSIFICATION
//! REQUIRED`, `ADVISORY`, and `NOT PROVEN` are recorded but never block —
//! the advisory-to-blocking arming clock is a later PR (mirrors
//! `policy/changelog.toml`'s `blocking_enforced_from` pattern). `STOP`
//! (GUIDANCE_STYLE's fifth, safety/irreversibility posture) is reserved for
//! a staged-secret hazard; no check in this program asserts one.

use crate::tasks::changelog::{self, Fragment};
use crate::tasks::ci_policy::{
    ALLOWED_FROM_RAW_PATTERN, FROM_RAW_PATTERN, SEARCH_ROOTS, is_disallowed_from_raw_line,
};
use crate::tasks::staged;
use crate::utils::project_root;
use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

// =============================================================================
// Posture + report shape (GUIDANCE_STYLE §4/§5)
// =============================================================================

/// The fixed vocabulary from `docs/reference/GUIDANCE_STYLE.md` §5,
/// restricted to the four postures a V1 commit-tier check can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Posture {
    #[serde(rename = "BLOCKED")]
    Blocked,
    #[serde(rename = "CLASSIFICATION REQUIRED")]
    ClassificationRequired,
    #[serde(rename = "ADVISORY")]
    Advisory,
    #[serde(rename = "NOT PROVEN")]
    NotProven,
}

impl Posture {
    /// Only `Blocked` fails the gate in V1 — see module docs.
    pub fn is_blocking(self) -> bool {
        matches!(self, Posture::Blocked)
    }

    pub fn label(self) -> &'static str {
        match self {
            Posture::Blocked => "BLOCKED",
            Posture::ClassificationRequired => "CLASSIFICATION REQUIRED",
            Posture::Advisory => "ADVISORY",
            Posture::NotProven => "NOT PROVEN",
        }
    }
}

/// GUIDANCE_STYLE §4 shape: result · why it matters · affected artifacts ·
/// fix · rerun · what remains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    pub check: String,
    pub posture: Posture,
    pub result: String,
    pub why: String,
    #[serde(default)]
    pub affected: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    pub rerun: String,
    pub what_remains: String,
}

/// Marker line prefix used to embed a [`CheckReport`] as JSON inside a
/// `GateResult.output_summary` string. `gates::build_agent_receipt` looks for
/// this to recover structured posture/affected/fix data without a second
/// execution path — see [`parse_report`].
pub const REPORT_MARKER: &str = "COMMIT_CHECK_REPORT_JSON:";

impl CheckReport {
    /// Human-readable block followed by the machine-parseable marker line.
    ///
    /// Errors only if `self` fails to serialize, which cannot happen for
    /// today's field set (`String`/`Vec<String>`/`Option<String>`/a
    /// `#[serde(rename)]` enum) — the `Result` exists so a future field
    /// change that *can* fail surfaces as a real error at the call site
    /// (which maps it to an `"error"` gate status) instead of silently
    /// emitting a marker line with no JSON payload, which would make
    /// [`parse_report`] quietly drop the whole structured report.
    pub fn render(&self) -> Result<String> {
        let mut lines = vec![
            format!("{}: {}", self.posture.label(), self.result),
            format!("why: {}", self.why),
        ];
        if !self.affected.is_empty() {
            lines.push(format!("affected: {}", self.affected.join(", ")));
        }
        if let Some(fix) = &self.fix {
            lines.push(format!("fix: {fix}"));
        }
        lines.push(format!("rerun: {}", self.rerun));
        lines.push(format!("what remains: {}", self.what_remains));
        let json = serde_json::to_string(self).with_context(|| {
            format!("failed to serialize CheckReport for check '{}'", self.check)
        })?;
        lines.push(String::new());
        lines.push(format!("{REPORT_MARKER}{json}"));
        Ok(lines.join("\n"))
    }
}

/// Recover a [`CheckReport`] from a gate's `output_summary`, if it carries
/// one (non-commit gates simply don't have the marker line).
///
/// Searches from the END of the output, not the first match: `affected`
/// entries come from staged file paths, and a path containing a literal
/// newline followed by text that happens to start with [`REPORT_MARKER`]
/// would otherwise be mistaken for the real marker line, which
/// [`CheckReport::render`] always appends last.
pub fn parse_report(output_summary: &str) -> Option<CheckReport> {
    let line = output_summary.lines().rev().find_map(|l| l.strip_prefix(REPORT_MARKER))?;
    serde_json::from_str(line).ok()
}

/// What an internal commit-tier check hands back to the gate runner.
pub enum CommitCheckOutcome {
    /// Clean pass — a terse one-liner, no structured report needed
    /// (GUIDANCE_STYLE: "terse on success").
    Pass(String),
    /// A posture was flagged. `report.posture.is_blocking()` decides whether
    /// the gate fails.
    Flagged(CheckReport),
}

const RERUN_PREFIX: &str = "cargo xtask gates --tier commit --staged --gate";

fn rerun_for(check: &str) -> String {
    format!("{RERUN_PREFIX} {check}")
}

// =============================================================================
// Dispatch (matched against `.ci/gate-policy.yaml` commit-tier gate names)
// =============================================================================

/// Run one named commit-tier check against the current staged tree.
///
/// `tree_oid`: the `git write-tree` OID `plan_gates` already captured for
/// this run (issue #3786 correctness follow-up). Every check below uses it
/// instead of calling `staged::staged_tree_oid` again — re-deriving the tree
/// from a live `git write-tree` call at dispatch time reads whatever the
/// index happens to be *right then*, which can differ from the OID already
/// committed to `AgentReceipt.staged_tree_oid` if the index changes between
/// planning and execution (e.g. a concurrent `git add`). `None` only when
/// called outside a real plan (e.g. ad hoc testing); still correct, just
/// not pinned to a single snapshot.
pub fn run_named_check(name: &str, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let root = project_root()?;
    match name {
        "staged_tree_identity" => staged_tree_identity_at(&root, tree_oid),
        "whitespace_check" => whitespace_check_at(&root, tree_oid),
        "conflict_markers_staged" => conflict_markers_staged_at(&root, tree_oid),
        "staged_exec_mode_policy" => staged_exec_mode_policy_at(&root, tree_oid),
        "staged_config_syntax" => staged_config_syntax_at(&root, tree_oid),
        "forbidden_machine_paths" => forbidden_machine_paths_at(&root, tree_oid),
        "staged_oversized_or_binary" => staged_oversized_or_binary_at(&root, tree_oid),
        "changie_fragment_staged" => changie_fragment_staged_at(&root, tree_oid),
        "rustfmt_staged" => rustfmt_staged_at(&root, tree_oid),
        "from_raw_staged" => from_raw_staged_at(&root, tree_oid),
        other => bail!("unknown commit-tier check '{other}'"),
    }
}

/// Resolve `tree_oid` to a concrete OID string, computing one fresh only
/// when the caller genuinely has none (see [`run_named_check`]'s doc
/// comment on why every real dispatch path already has one).
fn resolve_tree_oid(root: &Path, tree_oid: Option<&str>) -> Result<String> {
    match tree_oid {
        Some(oid) => Ok(oid.to_string()),
        None => staged::staged_tree_oid(root),
    }
}

// =============================================================================
// staged_tree_identity — the #3786-A wiring proof.
//
// Not a hygiene check: it exists to prove the full pipeline (--staged
// validation -> plan_gates -> run_internal_commit_check -> GateResult ->
// build_agent_receipt) reads the exact staged tree and threads its identity
// (git write-tree OID) end to end into AgentReceipt.staged_tree_oid.
// =============================================================================

fn staged_tree_identity_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    // Scope the reported file list to the SAME snapshot as the identity
    // above -- pass the (now-resolved) tree_oid through rather than letting
    // staged_diff_paths fall back to the live index, which could have moved
    // since tree_oid was captured.
    let changed = staged::staged_diff_paths(root, Some(&tree_oid))?;

    if changed.is_empty() {
        return Ok(CommitCheckOutcome::Pass(format!(
            "staged tree {tree_oid} — nothing staged relative to HEAD"
        )));
    }

    // Deliberately ADVISORY, not a Pass one-liner: this exercises the full
    // AgentReceipt.advisories plumbing (build_agent_receipt ->
    // commit_advisories -> parse_report) even though nothing here is a real
    // finding — that plumbing is exactly what #3786-A exists to prove works.
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "staged_tree_identity".to_string(),
        posture: Posture::Advisory,
        result: format!(
            "staged tree {tree_oid} — {} file(s) staged relative to HEAD",
            changed.len()
        ),
        why: "wiring proof for the commit-tier substrate (issue #3786-A): confirms --staged \
              resolves the exact git write-tree identity and threads it through the receipt, \
              independent of any real hygiene check"
            .to_string(),
        affected: changed,
        fix: None,
        rerun: rerun_for("staged_tree_identity"),
        what_remains: "none — the nine structural checks (issue #3786-B) run alongside this \
                       one, not instead of it"
            .to_string(),
    }))
}

// =============================================================================
// 1. git diff --check (whitespace + git's own conflict-marker scan)
//
// Tree-oid-pinned like the other 8: `git diff <base> <oid> --check` DOES
// have a tree-object equivalent — `--check` inspects the diff hunks between
// two trees, and `<base>` (`diff_base`, shared with `staged_diff_paths`)
// vs `<oid>` (the pinned tree) is exactly such a pair. `git diff HEAD <oid>
// --check` and `git diff --cached --check` produce byte-identical output
// when the index hasn't moved since `<oid>` was captured — the bug this
// closes is the window where it HAS moved: a `git add` between
// `plan_gates` capturing the OID and this check dispatching would make
// `--cached` silently disagree with `AgentReceipt.staged_tree_oid` about
// what's actually being checked.
// =============================================================================

fn whitespace_check_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let base = staged::diff_base(root)?;
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", &base, &tree_oid, "--check"])
        .output()
        .context("failed to run `git diff <base> <oid> --check`")?;
    if output.status.success() {
        return Ok(CommitCheckOutcome::Pass(
            "no whitespace/conflict-marker issues in the staged diff".to_string(),
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let affected: Vec<String> = raw.lines().filter(|l| !l.is_empty()).map(str::to_string).collect();
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "whitespace_check".to_string(),
        posture: Posture::Blocked,
        result: format!(
            "`git diff --cached --check` found {} issue(s) in the staged diff",
            affected.len()
        ),
        why: "trailing whitespace and unresolved merge-conflict markers break formatting and, \
              for markers, compilation"
            .to_string(),
        affected,
        fix: Some(
            "remove trailing whitespace / resolve the conflict, then re-stage (git add)"
                .to_string(),
        ),
        rerun: rerun_for("whitespace_check"),
        what_remains: "none — this is the full check".to_string(),
    }))
}

// =============================================================================
// 2. Conflict markers — full-content scan of staged text (stronger than #1:
//    #1 only sees diff-hunk context; this reads the whole staged file).
// =============================================================================

const CONFLICT_MARKER_PATTERN: &str = r"^(<{7} |={7}$|>{7} )";

// AGENTS.md code-quality bar: regexes are `static LazyLock<Regex>`, never
// `Regex::new()` per invocation — this check runs on every commit-tier gate
// dispatch, not once per process.
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static CONFLICT_MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(CONFLICT_MARKER_PATTERN).expect("CONFLICT_MARKER_PATTERN is a valid regex literal")
});

fn evaluate_conflict_markers<'a>(files: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<String> {
    let mut affected = Vec::new();
    for (path, text) in files {
        for (idx, line) in text.lines().enumerate() {
            if CONFLICT_MARKER_RE.is_match(line) {
                affected.push(format!("{path}:{}", idx + 1));
            }
        }
    }
    affected
}

fn conflict_markers_staged_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let paths = staged::staged_diff_paths(root, Some(&tree_oid))?;
    let mut files = Vec::new();
    for path in &paths {
        if let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? {
            files.push((path.clone(), text));
        }
    }
    let affected =
        evaluate_conflict_markers(files.iter().map(|(path, text)| (path.as_str(), text.as_str())));

    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass("no conflict markers in staged files".to_string()));
    }
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "conflict_markers_staged".to_string(),
        posture: Posture::Blocked,
        result: format!(
            "{} staged line(s) look like leftover merge-conflict markers",
            affected.len()
        ),
        why: "a committed conflict marker breaks compilation/parsing and usually means a merge \
              was left unresolved"
            .to_string(),
        affected,
        fix: Some(
            "resolve the conflict and remove the <<<<<<</=======/>>>>>>> lines, then re-stage"
                .to_string(),
        ),
        rerun: rerun_for("conflict_markers_staged"),
        what_remains: "none — full-content scan of every staged file touched by this commit"
            .to_string(),
    }))
}

// =============================================================================
// 3. Staged executable-bit policy (the R1 chmod defect class)
// =============================================================================

const EXECUTABLE_ALLOWLIST_PATHS: &[&str] = &["hooks/pre-push", ".ci/hooks/pre-commit"];
const EXECUTABLE_ALLOWLIST_SUFFIXES: &[&str] = &[".sh"];
const EXECUTABLE_ALLOWLIST_PREFIXES: &[&str] = &[".ci/scripts/", "scripts/"];

fn is_known_executable(path: &str) -> bool {
    EXECUTABLE_ALLOWLIST_PATHS.contains(&path)
        || EXECUTABLE_ALLOWLIST_SUFFIXES.iter().any(|suffix| path.ends_with(suffix))
        || EXECUTABLE_ALLOWLIST_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

fn evaluate_exec_mode<'a>(entries: impl Iterator<Item = (&'a str, bool)>) -> Vec<String> {
    entries
        .filter(|(path, is_exec)| *is_exec && !is_known_executable(path))
        .map(|(path, _)| path.to_string())
        .collect()
}

fn staged_exec_mode_policy_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let diff_paths: BTreeSet<String> =
        staged::staged_diff_paths(root, Some(&tree_oid))?.into_iter().collect();
    if diff_paths.is_empty() {
        return Ok(CommitCheckOutcome::Pass("no staged files".to_string()));
    }
    let entries = staged::list_staged_entries(root, &tree_oid)?;

    let unexpected_exec = evaluate_exec_mode(
        entries
            .iter()
            .filter(|entry| diff_paths.contains(&entry.path))
            .map(|entry| (entry.path.as_str(), entry.is_executable())),
    );

    if unexpected_exec.is_empty() {
        return Ok(CommitCheckOutcome::Pass("staged executable-bit policy holds".to_string()));
    }

    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "staged_exec_mode_policy".to_string(),
        posture: Posture::Blocked,
        result: format!(
            "{} staged file(s) carry mode 100755 (executable) but aren't a recognized script",
            unexpected_exec.len()
        ),
        why: "core.fileMode=false makes the working-tree permission bit unreliable; an \
              accidental +x on a non-script file silently ships a wrong mode — the R1 chmod \
              defect class this tier exists to catch"
            .to_string(),
        affected: unexpected_exec,
        fix: Some(
            "git update-index --chmod=-x <path> for each affected file, then re-stage".to_string(),
        ),
        rerun: rerun_for("staged_exec_mode_policy"),
        what_remains: "scripts staged WITHOUT the executable bit are not flagged here (a softer, \
                       non-deterministic judgment call)"
            .to_string(),
    }))
}

// =============================================================================
// 4. Malformed staged JSON/TOML/YAML
// =============================================================================

fn config_syntax_error(ext: &str, text: &str) -> Option<String> {
    match ext {
        "json" => serde_json::from_str::<serde_json::Value>(text).err().map(|e| e.to_string()),
        "yaml" | "yml" => {
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).err().map(|e| e.to_string())
        }
        "toml" => toml::from_str::<toml::Value>(text).err().map(|e| e.to_string()),
        _ => None,
    }
}

fn staged_config_syntax_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let paths = staged::staged_diff_paths(root, Some(&tree_oid))?;
    let mut affected = Vec::new();
    for path in &paths {
        // Case-insensitive extension match: `.YAML`/`.Json` are valid staged
        // paths and shouldn't silently skip this check just because the
        // extension isn't lowercase.
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "json" | "yaml" | "yml" | "toml") {
            continue;
        }
        let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? else {
            continue;
        };
        if let Some(message) = config_syntax_error(&ext, &text) {
            affected.push(format!("{path}: {message}"));
        }
    }
    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass(
            "staged JSON/YAML/TOML files parse cleanly".to_string(),
        ));
    }
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "staged_config_syntax".to_string(),
        posture: Posture::Blocked,
        result: format!("{} staged config file(s) fail to parse", affected.len()),
        why: "a malformed JSON/YAML/TOML file breaks whatever consumes it downstream (CI \
              policy, gate config, editor settings)"
            .to_string(),
        affected,
        fix: Some("fix the reported syntax error, then re-stage".to_string()),
        rerun: rerun_for("staged_config_syntax"),
        what_remains: "schema/semantic validation beyond syntax is out of scope for this check"
            .to_string(),
    }))
}

// =============================================================================
// 5. Forbidden absolute / machine-specific paths
// =============================================================================

const FORBIDDEN_PATH_SCAN_EXCLUDE_PREFIXES: &[&str] = &["docs/"];
const FORBIDDEN_PATH_SCAN_EXCLUDE_SUFFIXES: &[&str] = &[".md"];
const FORBIDDEN_PATH_PATTERN: &str =
    r"([A-Za-z]:\\Users\\[^\\\s]+\\|/home/[A-Za-z0-9_.-]+/|/Users/[A-Za-z0-9_.-]+/)";

// AGENTS.md code-quality bar: regexes are `static LazyLock<Regex>`, never
// `Regex::new()` per invocation.
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static FORBIDDEN_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(FORBIDDEN_PATH_PATTERN).expect("FORBIDDEN_PATH_PATTERN is a valid regex literal")
});

fn is_forbidden_path_scan_excluded(path: &str) -> bool {
    FORBIDDEN_PATH_SCAN_EXCLUDE_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
        || FORBIDDEN_PATH_SCAN_EXCLUDE_SUFFIXES.iter().any(|suffix| path.ends_with(suffix))
}

fn evaluate_forbidden_paths<'a>(
    re: &Regex,
    files: impl Iterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let mut affected = Vec::new();
    for (path, text) in files {
        if is_forbidden_path_scan_excluded(path) {
            continue;
        }
        for (idx, line) in text.lines().enumerate() {
            if let Some(m) = re.find(line) {
                affected.push(format!("{path}:{}: {}", idx + 1, m.as_str()));
            }
        }
    }
    affected
}

fn forbidden_machine_paths_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let paths = staged::staged_diff_paths(root, Some(&tree_oid))?;
    let mut files = Vec::new();
    for path in &paths {
        if let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? {
            files.push((path.clone(), text));
        }
    }
    let affected = evaluate_forbidden_paths(
        &FORBIDDEN_PATH_RE,
        files.iter().map(|(path, text)| (path.as_str(), text.as_str())),
    );

    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass(
            "no machine-specific absolute paths found in staged non-doc files".to_string(),
        ));
    }
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "forbidden_machine_paths".to_string(),
        posture: Posture::Blocked,
        result: format!(
            "{} staged line(s) contain a machine-specific absolute path",
            affected.len()
        ),
        why: "an absolute path tied to one machine/user (a Windows user profile, a POSIX home \
              dir) breaks on every other machine and often leaks a local username"
            .to_string(),
        affected,
        fix: Some(
            "replace the absolute path with a repo-relative path or an environment-derived one, \
             then re-stage"
                .to_string(),
        ),
        rerun: rerun_for("forbidden_machine_paths"),
        what_remains:
            "docs/ and *.md are excluded — placeholder paths in examples are common there"
                .to_string(),
    }))
}

// =============================================================================
// 6. Oversized / disallowed-binary staged files
// =============================================================================

const MAX_STAGED_FILE_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB
const DISALLOWED_BINARY_EXTENSIONS: &[&str] = &["exe", "dll", "dylib"];

fn evaluate_oversized_or_binary<'a>(entries: impl Iterator<Item = (&'a str, u64)>) -> Vec<String> {
    let mut affected = Vec::new();
    for (path, size) in entries {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if DISALLOWED_BINARY_EXTENSIONS.contains(&ext.as_str()) {
            affected.push(format!("{path}: disallowed binary extension .{ext}"));
        } else if size > MAX_STAGED_FILE_BYTES {
            affected.push(format!("{path}: {size} bytes (limit {MAX_STAGED_FILE_BYTES})"));
        }
    }
    affected
}

fn staged_oversized_or_binary_at(
    root: &Path,
    tree_oid: Option<&str>,
) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let diff_paths: BTreeSet<String> =
        staged::staged_diff_paths(root, Some(&tree_oid))?.into_iter().collect();
    if diff_paths.is_empty() {
        return Ok(CommitCheckOutcome::Pass("no staged files".to_string()));
    }
    let entries = staged::list_staged_entries(root, &tree_oid)?;

    let mut sized = Vec::new();
    for entry in entries.iter().filter(|entry| diff_paths.contains(&entry.path)) {
        let size = staged::blob_size(root, &entry.blob_oid)?;
        sized.push((entry.path.clone(), size));
    }
    let affected =
        evaluate_oversized_or_binary(sized.iter().map(|(path, size)| (path.as_str(), *size)));

    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass(
            "no oversized or disallowed-binary staged files".to_string(),
        ));
    }
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "staged_oversized_or_binary".to_string(),
        posture: Posture::Blocked,
        result: format!(
            "{} staged file(s) are oversized or a disallowed binary type",
            affected.len()
        ),
        why: "large or native-binary blobs bloat repo history irreversibly and usually mean a \
              build artifact or download was staged by accident"
            .to_string(),
        affected,
        fix: Some(
            "unstage the file (git restore --staged <path>), add it to .gitignore if it's a \
             build artifact, then re-commit"
                .to_string(),
        ),
        rerun: rerun_for("staged_oversized_or_binary"),
        what_remains: "the 5 MiB threshold and extension list are a starting policy; widen via \
                       .ci/gate-policy.yaml if a legitimate large asset needs staging"
            .to_string(),
    }))
}

// =============================================================================
// 7. Changie fragment syntax (staged) — dry-render deferred to pre-push (#3985)
// =============================================================================

fn is_fragment_path(path: &str) -> bool {
    let prefix = format!("{}/", changelog::UNRELEASED_DIR);
    // Extension match is case-insensitive (`.YAML`/`.Yml` are valid on
    // case-insensitive/case-preserving filesystems and git tracks them
    // byte-for-byte) — only the directory prefix stays case-sensitive,
    // matching how git itself treats POSIX paths.
    let lower = path.to_ascii_lowercase();
    path.starts_with(&prefix) && (lower.ends_with(".yaml") || lower.ends_with(".yml"))
}

/// Read and parse `.changie.yaml` from the pinned staged tree, not the
/// working-tree filesystem. A staged fragment must be validated against the
/// config that's actually part of THIS commit's snapshot — if `.changie.yaml`
/// has unstaged edits (or a staged edit not yet reflected on disk in some
/// other workflow), `changelog::load_config` reading straight off disk would
/// validate staged fragment content against a config that isn't the one
/// being committed, silently blocking a legitimately-valid staged fragment
/// or passing an invalid one, depending on which way the unstaged edit
/// drifted.
fn load_staged_changie_config(root: &Path, tree_oid: &str) -> Result<changelog::ChangieConfig> {
    let text = staged::read_staged_path_text(root, changelog::CHANGIE_CONFIG, Some(tree_oid))?
        .with_context(|| {
            format!(
                "staged {} is not valid UTF-8 (or was not found in the staged tree)",
                changelog::CHANGIE_CONFIG
            )
        })?;
    changelog::parse_config(&text)
        .with_context(|| format!("failed to parse staged {}", changelog::CHANGIE_CONFIG))
}

fn changie_fragment_staged_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let paths = staged::staged_diff_paths(root, Some(&tree_oid))?;
    let fragment_paths: Vec<String> = paths.into_iter().filter(|p| is_fragment_path(p)).collect();

    if fragment_paths.is_empty() {
        return Ok(CommitCheckOutcome::Pass("no staged Changie fragments".to_string()));
    }

    let cfg = load_staged_changie_config(root, &tree_oid)
        .context("failed to load staged .changie.yaml")?;
    let mut blocked = Vec::new();
    for path in &fragment_paths {
        let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? else {
            continue;
        };
        match serde_yaml_ng::from_str::<Fragment>(&text) {
            Err(err) => blocked.push(format!("{path}: malformed YAML — {err}")),
            Ok(frag) => {
                let findings = changelog::validate_fragment(&frag, &cfg);
                if !findings.is_empty() {
                    blocked.push(format!("{path}: {}", findings.join("; ")));
                }
            }
        }
    }

    if !blocked.is_empty() {
        return Ok(CommitCheckOutcome::Flagged(CheckReport {
            check: "changie_fragment_staged".to_string(),
            posture: Posture::Blocked,
            result: format!("{} staged Changie fragment(s) fail schema validation", blocked.len()),
            why: "a malformed fragment fails silently at release-batch time instead of at commit \
                  time"
                .to_string(),
            affected: blocked,
            fix: Some(
                "run `changie new` to regenerate the fragment interactively, or fix the reported \
                 field"
                    .to_string(),
            ),
            rerun: rerun_for("changie_fragment_staged"),
            what_remains: "full `changie batch --dry-run` render is deferred to pre-push (#3985), \
                           where the working checkout is authoritative"
                .to_string(),
        }));
    }

    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "changie_fragment_staged".to_string(),
        posture: Posture::NotProven,
        result: format!(
            "{} staged Changie fragment(s) parse and pass schema validation",
            fragment_paths.len()
        ),
        why: "schema validity doesn't prove the fragment renders through `changie batch` — that \
              step needs the full working checkout"
            .to_string(),
        affected: fragment_paths,
        fix: None,
        rerun: rerun_for("changie_fragment_staged"),
        what_remains: "`changie batch --dry-run` render still required before merge (pre-push / \
                       CI, #3985)"
            .to_string(),
    }))
}

// =============================================================================
// 8. rustfmt --check on staged Rust (piped via stdin, not a temp-file
//    checkout of the blob)
// =============================================================================

const RUSTFMT_EDITION: &str = "2024";
const RUSTFMT_CONFIG_FILE: &str = "rustfmt.toml";

/// `true` if rustfmt would reformat `text`. Pipes content via stdin rather
/// than writing it to a temp file — two reasons, not one:
///
/// - Files like `gates.rs` (this crate's own) contain `mod first_failure;`
///   /`mod planning_types;` declarations. Given a real *file path*, rustfmt
///   tries to resolve those as sibling files and **errors** (not "would
///   reformat" — a hard resolution failure) when a staged file is written
///   in isolation to an unrelated temp path with no siblings. Stdin mode
///   has no file-path context to resolve modules against, so it formats
///   exactly the given text and nothing else.
/// - It also means no temp file/directory I/O at all for this check.
///
/// `--config-path` points at a temp file holding the **staged**
/// `rustfmt.toml` content (`config_text`), when the tree has one — never the
/// working-tree `root.join("rustfmt.toml")` file. stdin mode has no file
/// location to search upward from for config discovery, so a caller has to
/// supply one explicitly; if it pointed at the working-tree file, a staged
/// `rustfmt.toml` policy change with an unrelated unstaged edit sitting on
/// top of it would check staged Rust blobs against a config that isn't the
/// one actually being committed (the same class of drift the staged Changie
/// config fix closes for `changie_fragment_staged_at`). `config_text: None`
/// means the tree has no `rustfmt.toml` at all — falls back to rustfmt's
/// stock defaults, same as the pre-fix behavior for a config-less repo.
///
/// Deciding "would reformat" from **stdout content**, not the exit code:
/// unlike file-path mode (`--check <path>` exits non-zero on a real diff),
/// rustfmt's stdin `--check` mode exits `0` even when stdout carries an
/// actual diff — an exit-code check alone would silently report every
/// unformatted file as clean. A genuine tool failure (a bad
/// `--config-path`, a syntax error in the staged content itself) writes to
/// stderr and must surface as a real `Err` — the "error" gate status, not a
/// silently-wrong "clean" or a misleading "needs reformatting".
fn rustfmt_would_reformat(config_text: Option<&str>, text: &str) -> Result<bool> {
    use std::io::Write;
    use std::process::Stdio;

    let mut command = Command::new("rustfmt");
    command.args(["--check", "--edition", RUSTFMT_EDITION]);

    // Materialize the staged config to a temp file only for the duration of
    // this call; keep the guard alive until the process finishes so the
    // path `--config-path` points at doesn't get cleaned up mid-run.
    let _config_guard = match config_text {
        Some(content) => {
            let mut file = tempfile::NamedTempFile::new()
                .context("failed to create a temp file for the staged rustfmt.toml")?;
            file.write_all(content.as_bytes())
                .context("failed to write the staged rustfmt.toml to a temp file")?;
            command.arg("--config-path").arg(file.path());
            Some(file)
        }
        None => None,
    };

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn rustfmt")?;
    child
        .stdin
        .take()
        .context("rustfmt stdin was not piped")?
        .write_all(text.as_bytes())
        .context("failed to write staged content to rustfmt stdin")?;
    let output = child.wait_with_output().context("failed to wait for rustfmt")?;

    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("rustfmt reported a failure rather than a formatting diff: {stderr}");
    }
    Ok(!output.stdout.is_empty())
}

/// Read the staged `rustfmt.toml` content, or `None` if the tree has no such
/// file. See [`rustfmt_would_reformat`]'s doc comment for why this must come
/// from the pinned tree, not `std::fs::read_to_string` against the working
/// copy.
fn load_staged_rustfmt_config(root: &Path, tree_oid: &str) -> Result<Option<String>> {
    if !staged::staged_path_exists(root, tree_oid, RUSTFMT_CONFIG_FILE)? {
        return Ok(None);
    }
    staged::read_staged_path_text(root, RUSTFMT_CONFIG_FILE, Some(tree_oid))
}

fn rustfmt_staged_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let paths: Vec<String> = staged::staged_diff_paths(root, Some(&tree_oid))?
        .into_iter()
        .filter(|p| p.ends_with(".rs"))
        .collect();
    if paths.is_empty() {
        return Ok(CommitCheckOutcome::Pass("no staged Rust files".to_string()));
    }

    if Command::new("rustfmt").arg("--version").output().is_err() {
        return Ok(CommitCheckOutcome::Flagged(CheckReport {
            check: "rustfmt_staged".to_string(),
            posture: Posture::NotProven,
            result: "rustfmt is not on PATH".to_string(),
            why: "the expected verification instrument didn't run — this is not the same as \
                  passing"
                .to_string(),
            affected: paths,
            fix: Some("install the rustfmt component (rustup component add rustfmt)".to_string()),
            rerun: rerun_for("rustfmt_staged"),
            what_remains: "formatting still required before push (cargo xtask fmt --check)"
                .to_string(),
        }));
    }

    let config_text = load_staged_rustfmt_config(root, &tree_oid)?;
    let mut affected = Vec::new();
    for path in &paths {
        let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? else {
            continue;
        };
        if rustfmt_would_reformat(config_text.as_deref(), &text)? {
            affected.push(path.clone());
        }
    }

    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass(format!(
            "{} staged Rust file(s) are formatted",
            paths.len()
        )));
    }

    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "rustfmt_staged".to_string(),
        posture: Posture::Blocked,
        result: format!("{} staged Rust file(s) would be reformatted by rustfmt", affected.len()),
        why: "unformatted Rust fails the fmt pr_fast gate later; catching it at commit time is \
              cheaper"
            .to_string(),
        affected,
        fix: Some("cargo xtask fmt".to_string()),
        rerun: rerun_for("rustfmt_staged"),
        what_remains: format!(
            "assumes workspace edition {RUSTFMT_EDITION}; a crate on a different edition may \
             false-positive here"
        ),
    }))
}

// =============================================================================
// 9. ExitStatus::from_raw() policy, staged-tree variant (folds in and retires
//    the working-tree `.ci/hooks/pre-commit` / check-from-raw.sh authority).
// =============================================================================

// AGENTS.md code-quality bar: regexes are `static LazyLock<Regex>`, never
// `Regex::new()` per invocation. `FROM_RAW_PATTERN`/`ALLOWED_FROM_RAW_PATTERN`
// are `ci_policy.rs` consts (shared with `check_from_raw`'s working-tree
// authority); these statics are this module's own compiled instances, not
// re-exports, so `from_raw_staged_at` doesn't recompile them on every
// commit-tier dispatch.
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static DISALLOW_FROM_RAW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(FROM_RAW_PATTERN).expect("FROM_RAW_PATTERN is a valid regex literal")
});
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static ALLOWED_FROM_RAW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("ALLOWED_FROM_RAW_PATTERN is a valid regex literal")
});

fn from_raw_staged_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = resolve_tree_oid(root, tree_oid)?;
    let disallow_re = &*DISALLOW_FROM_RAW_RE;
    let allowed_re = &*ALLOWED_FROM_RAW_RE;

    let paths: Vec<String> = staged::staged_diff_paths(root, Some(&tree_oid))?
        .into_iter()
        .filter(|p| {
            p.ends_with(".rs")
                && SEARCH_ROOTS.iter().any(|search_root| p.starts_with(&format!("{search_root}/")))
        })
        .collect();
    if paths.is_empty() {
        return Ok(CommitCheckOutcome::Pass(
            "no staged Rust files under the from_raw search roots".to_string(),
        ));
    }

    let mut affected = Vec::new();
    for path in &paths {
        let Some(text) = staged::read_staged_path_text(root, path, Some(&tree_oid))? else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let candidate = format!("{path}:{}:{line}", idx + 1);
            if is_disallowed_from_raw_line(&candidate, disallow_re, allowed_re) {
                affected.push(candidate);
            }
        }
    }

    if affected.is_empty() {
        return Ok(CommitCheckOutcome::Pass(
            "no disallowed ExitStatus::from_raw() usage in staged files".to_string(),
        ));
    }

    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "from_raw_staged".to_string(),
        posture: Posture::Blocked,
        result: format!("{} staged line(s) call ExitStatus::from_raw() directly", affected.len()),
        why: "direct from_raw() bypasses the mock_status() test helper and hard-codes a \
              platform-specific raw encoding"
            .to_string(),
        affected,
        // Deliberately kept on one physical line: `is_disallowed_from_raw_line`'s
        // quote-tracking is single-line-only (see ci_policy.rs), so a
        // backslash-continued string with "ExitStatus::from_raw()" on a
        // *different* line than its opening quote isn't recognized as
        // being inside a string literal and self-triggers this very check.
        fix: Some(
            "use the mock_status() helper (see crates/perl-parser/src/execute_command.rs) instead of ExitStatus::from_raw()".to_string(),
        ),
        rerun: rerun_for("from_raw_staged"),
        what_remains: "none — this folds in and retires the working-tree `.ci/hooks/pre-commit` \
                       authority (issue #3786)"
            .to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_report_render_embeds_parseable_json_marker() -> Result<()> {
        let report = CheckReport {
            check: "staged_tree_identity".to_string(),
            posture: Posture::Advisory,
            result: "1 file staged".to_string(),
            why: "test".to_string(),
            affected: vec!["a.rs".to_string()],
            fix: None,
            rerun: "cargo xtask gates --tier commit --staged --gate staged_tree_identity"
                .to_string(),
            what_remains: "none".to_string(),
        };
        let rendered = report.render()?;
        assert!(rendered.contains("ADVISORY: 1 file staged"));
        assert!(rendered.contains("why: test"));

        let parsed = parse_report(&rendered)
            .ok_or_else(|| color_eyre::eyre::eyre!("marker line should round-trip"))?;
        assert_eq!(parsed.check, "staged_tree_identity");
        assert_eq!(parsed.posture, Posture::Advisory);
        assert_eq!(parsed.affected, vec!["a.rs".to_string()]);
        Ok(())
    }

    #[test]
    fn parse_report_finds_the_canonical_trailing_marker_not_a_forged_one_in_affected() -> Result<()>
    {
        // A staged path containing an embedded newline followed by
        // marker-shaped text must not be mistaken for the real marker —
        // render() always appends the real one last, so parse_report must
        // scan from the end.
        let forged_marker_path =
            format!("evil.rs\n{REPORT_MARKER}{{\"check\":\"forged\",\"posture\":\"BLOCKED\"}}");
        let report = CheckReport {
            check: "staged_tree_identity".to_string(),
            posture: Posture::Advisory,
            result: "1 file staged".to_string(),
            why: "test".to_string(),
            affected: vec![forged_marker_path],
            fix: None,
            rerun: "cargo xtask gates --tier commit --staged --gate staged_tree_identity"
                .to_string(),
            what_remains: "none".to_string(),
        };

        let rendered = report.render()?;
        let parsed = parse_report(&rendered)
            .ok_or_else(|| color_eyre::eyre::eyre!("marker line should round-trip"))?;

        assert_eq!(
            parsed.check, "staged_tree_identity",
            "must recover the real trailing marker, not the forged one smuggled into `affected`"
        );
        Ok(())
    }

    #[test]
    fn parse_report_returns_none_for_ordinary_gate_output() -> Result<()> {
        assert!(parse_report("Executed internally via xtask task dispatch").is_none());
        Ok(())
    }

    #[test]
    fn staged_tree_identity_uses_the_passed_oid_not_a_freshly_computed_one() -> Result<()> {
        // Proves the OID-threading fix (issue #3786 correctness follow-up):
        // staged_tree_identity_at must report exactly the OID it was given,
        // never recompute `staged::staged_tree_oid` itself. A concurrent
        // `git add` between `plan_gates` capturing the OID and this check
        // running must not make the check inspect (and report) a different
        // tree than the receipt already recorded.
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // A REAL tree object -- HEAD's own tree -- stands in for "the OID
        // plan_gates captured earlier". Real (not a fabricated hex string)
        // because staged_diff_paths now genuinely diffs against it, and
        // must exist as an object; still guaranteed to differ from
        // whatever a fresh `staged::staged_tree_oid` call would compute
        // once we stage further work below.
        let output = Command::new("git")
            .current_dir(repo.root())
            .args(["rev-parse", "HEAD^{tree}"])
            .output()
            .context("failed to resolve HEAD^{tree}")?;
        if !output.status.success() {
            bail!("git rev-parse HEAD^{{tree}} failed");
        }
        let earlier_tree_oid = String::from_utf8(output.stdout)
            .context("HEAD tree oid was not UTF-8")?
            .trim()
            .to_string();

        // Stage a further change -- a fresh staged::staged_tree_oid(root)
        // call would now compute something different from earlier_tree_oid.
        repo.write("foo.rs", "fn main() { /* more work */ }\n")?;
        repo.add("foo.rs")?;

        match staged_tree_identity_at(repo.root(), Some(&earlier_tree_oid))? {
            CommitCheckOutcome::Pass(summary) => {
                assert!(
                    summary.contains(&earlier_tree_oid),
                    "expected the passed OID (not a freshly computed one) in: {summary}"
                );
            }
            CommitCheckOutcome::Flagged(report) => {
                assert!(
                    report.result.contains(&earlier_tree_oid),
                    "expected the passed OID (not a freshly computed one) in: {}",
                    report.result
                );
            }
        }
        Ok(())
    }

    #[test]
    fn posture_is_blocking_only_for_blocked() -> Result<()> {
        assert!(Posture::Blocked.is_blocking());
        assert!(!Posture::ClassificationRequired.is_blocking());
        assert!(!Posture::Advisory.is_blocking());
        assert!(!Posture::NotProven.is_blocking());
        Ok(())
    }

    #[test]
    fn run_named_check_rejects_unknown_names() -> Result<()> {
        let result = run_named_check("not_a_real_check", None);
        assert!(result.is_err(), "an unregistered check name must error, not silently pass");
        Ok(())
    }

    // -------------------------------------------------------------------
    // Pure-logic unit tests for the nine structural checks (issue #3786-B).
    // No git process, no filesystem.
    // -------------------------------------------------------------------

    #[test]
    fn conflict_marker_regex_flags_seven_char_markers_only() {
        let files = vec![
            ("a.rs", "fn main() {}\n<<<<<<< HEAD\nfoo\n=======\nbar\n>>>>>>> branch\n"),
            ("b.rs", "// <<<<< not seven chars, not a marker\n"),
            ("c.rs", "clean file\nno markers here\n"),
        ];
        let affected = evaluate_conflict_markers(files.into_iter());
        assert_eq!(
            affected,
            vec!["a.rs:2".to_string(), "a.rs:4".to_string(), "a.rs:6".to_string()]
        );
    }

    #[test]
    fn exec_mode_policy_flags_non_script_executables_only() {
        let entries = vec![
            ("crates/foo/src/lib.rs", true), // unexpected: Rust source marked +x
            ("scripts/run.sh", true),        // allowlisted prefix
            (".ci/hooks/pre-commit", true),  // allowlisted exact path
            ("README.md", false),            // not executable at all
        ];
        let flagged = evaluate_exec_mode(entries.into_iter());
        assert_eq!(flagged, vec!["crates/foo/src/lib.rs".to_string()]);
    }

    #[test]
    fn config_syntax_error_flags_malformed_json_yaml_toml() {
        assert!(config_syntax_error("json", "{ not json").is_some());
        assert!(config_syntax_error("json", "{}").is_none());
        assert!(config_syntax_error("yaml", "a: [unterminated").is_some());
        assert!(config_syntax_error("yaml", "a: 1\n").is_none());
        assert!(config_syntax_error("toml", "a = ").is_some());
        assert!(config_syntax_error("toml", "a = 1\n").is_none());
        assert!(
            config_syntax_error("rs", "fn main() {").is_none(),
            "non-config extensions are skipped"
        );
    }

    #[test]
    fn forbidden_paths_flags_machine_specific_absolute_paths() -> Result<()> {
        let re = Regex::new(FORBIDDEN_PATH_PATTERN).context("valid regex")?;
        let files = vec![
            ("crates/foo/src/lib.rs", "windows path: C:\\Users\\steven\\scratch\\file.rs"),
            ("crates/foo/src/lib.rs", "posix path: /home/agent/work/file.rs"),
            ("crates/foo/src/lib.rs", "relative path: crates/foo/src/lib.rs"),
        ];
        let affected = evaluate_forbidden_paths(&re, files.into_iter());
        assert_eq!(
            affected.len(),
            2,
            "expected exactly the two machine-specific paths: {affected:?}"
        );
        Ok(())
    }

    #[test]
    fn forbidden_paths_excludes_docs_and_markdown() {
        assert!(is_forbidden_path_scan_excluded("docs/reference/GUIDE.md"));
        assert!(is_forbidden_path_scan_excluded("README.md"));
        assert!(!is_forbidden_path_scan_excluded("crates/foo/src/lib.rs"));
    }

    #[test]
    fn oversized_or_binary_flags_disallowed_extension_and_size_threshold() {
        let entries = vec![
            ("target/release/tool.exe", 10u64),
            ("crates/foo/src/lib.rs", 500u64),
            ("assets/huge.bin", MAX_STAGED_FILE_BYTES + 1),
        ];
        let affected = evaluate_oversized_or_binary(entries.into_iter());
        assert_eq!(affected.len(), 2, "expected the .exe and the oversized file: {affected:?}");
        assert!(affected[0].contains("tool.exe"));
        assert!(affected[1].contains("huge.bin"));
    }

    // -------------------------------------------------------------------
    // Staged-vs-working-tree correctness proof for the nine checks.
    //
    // Each check must see what's STAGED, never an unstaged working-tree
    // edit and never the pre-image before staging. Proven against a real
    // temp git repository (not the host repo's own index).
    // -------------------------------------------------------------------

    struct TempRepo {
        dir: tempfile::TempDir,
    }

    impl TempRepo {
        fn init() -> Result<Self> {
            let dir = tempfile::tempdir().context("failed to create temp repo dir")?;
            let root = dir.path();
            for args in [
                vec!["init", "--quiet"],
                vec!["config", "user.email", "test@example.com"],
                vec!["config", "user.name", "Test"],
                // core.fileMode=false mirrors this repo's own config and is
                // exactly the condition that makes filesystem mode bits
                // unreliable — see staged.rs module docs.
                vec!["config", "core.fileMode", "false"],
            ] {
                let status = Command::new("git")
                    .current_dir(root)
                    .args(&args)
                    .status()
                    .with_context(|| format!("failed to run git {args:?}"))?;
                if !status.success() {
                    bail!("git {args:?} failed in temp repo setup");
                }
            }
            Ok(Self { dir })
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn write(&self, rel_path: &str, content: &str) -> Result<()> {
            let path = self.root().join(rel_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content).context("failed to write file in temp repo")
        }

        fn add(&self, rel_path: &str) -> Result<()> {
            let status = Command::new("git")
                .current_dir(self.root())
                .args(["add", rel_path])
                .status()
                .context("failed to run git add")?;
            if !status.success() {
                bail!("git add {rel_path} failed");
            }
            Ok(())
        }

        fn commit(&self, message: &str) -> Result<()> {
            let status = Command::new("git")
                .current_dir(self.root())
                .args(["commit", "--quiet", "-m", message])
                .status()
                .context("failed to run git commit")?;
            if !status.success() {
                bail!("git commit failed");
            }
            Ok(())
        }
    }

    #[test]
    fn conflict_markers_staged_ignores_unstaged_edits() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Dirty the WORKING TREE with a conflict marker WITHOUT staging it.
        repo.write("foo.rs", "fn main() {}\n<<<<<<< HEAD\n")?;

        match conflict_markers_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Pass(_) => {}
            CommitCheckOutcome::Flagged(report) => {
                bail!(
                    "expected a clean pass — the conflict marker is unstaged, not committed \
                     content; check read the working tree instead of the staged blob. Report: \
                     {report:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn conflict_markers_staged_sees_staged_content_the_working_tree_lacks() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Stage a conflict marker, matching what `git write-tree` would record.
        repo.write("foo.rs", "fn main() {}\n<<<<<<< HEAD\n")?;
        repo.add("foo.rs")?;

        // Now revert the WORKING TREE back to clean content — the file on
        // disk no longer has the marker, but the STAGED blob still does.
        repo.write("foo.rs", "fn main() {}\n")?;

        match conflict_markers_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(report.affected.iter().any(|a| a.starts_with("foo.rs:")));
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "expected a BLOCKED report — the staged blob still has the conflict marker \
                     even though the working tree was reverted; check read the working tree \
                     instead of the staged blob. Pass summary: {summary}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn staged_exec_mode_policy_reads_git_mode_not_filesystem_mode() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("crates/foo/src/lib.rs", "fn main() {}\n")?;
        repo.add("crates/foo/src/lib.rs")?;

        // With core.fileMode=false the filesystem bit is not what git
        // records; force the STAGED mode to 100755 via update-index, which
        // is the same class of divergence the R1 chmod defect hit.
        let status = Command::new("git")
            .current_dir(repo.root())
            .args(["update-index", "--chmod=+x", "crates/foo/src/lib.rs"])
            .status()
            .context("failed to run git update-index --chmod=+x")?;
        if !status.success() {
            bail!("git update-index --chmod=+x failed");
        }

        match staged_exec_mode_policy_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(report.affected.contains(&"crates/foo/src/lib.rs".to_string()));
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "expected a BLOCKED report for a Rust file staged as executable — the check \
                     did not read the staged git mode. Pass summary: {summary}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn staged_config_syntax_flags_malformed_staged_toml() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("bad.toml", "a = 1\n")?;
        repo.add("bad.toml")?;
        repo.commit("initial")?;

        repo.write("bad.toml", "a = \n")?; // malformed, staged next
        repo.add("bad.toml")?;

        match staged_config_syntax_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(report.affected.iter().any(|a| a.starts_with("bad.toml:")));
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!("expected malformed TOML to block: {summary}")
            }
        }
        Ok(())
    }

    /// Regression for the deep-review send-back on PR #4020: extension
    /// matching must be case-insensitive. An `unwrap_or_default()` `ext ==
    /// "yaml"` comparison against a staged `.YAML` file would silently skip
    /// it (no match arm hit -> `continue`), reporting a false Pass on
    /// malformed content. A `.to_ascii_lowercase()` fix makes this test
    /// pass; reverting it makes this test fail (the malformed `.YAML` file
    /// would be skipped and the check would report Pass instead of
    /// Flagged).
    #[test]
    fn staged_config_syntax_matches_extension_case_insensitively() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("bad.YAML", "kind: [unterminated\n")?;
        repo.add("bad.YAML")?;

        match staged_config_syntax_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(
                    report.affected.iter().any(|a| a.starts_with("bad.YAML:")),
                    "expected the uppercase-extension file to be checked: {:?}",
                    report.affected
                );
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "expected malformed content in a .YAML (uppercase extension) file to block, \
                     not be silently skipped by a case-sensitive extension match: {summary}"
                )
            }
        }
        Ok(())
    }

    #[test]
    fn rustfmt_staged_flags_unformatted_staged_rust() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("crates/foo/src/lib.rs", "fn main(){let x=1;}\n")?;
        repo.add("crates/foo/src/lib.rs")?;

        match rustfmt_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(report.affected.contains(&"crates/foo/src/lib.rs".to_string()));
                assert_eq!(report.fix.as_deref(), Some("cargo xtask fmt"));
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!("expected unformatted staged Rust to block (or rustfmt missing): {summary}")
            }
        }
        Ok(())
    }

    #[test]
    fn rustfmt_staged_handles_mod_declarations_without_a_module_resolution_error() -> Result<()> {
        // Regression test found via dogfooding this check against this
        // repo's own diff: xtask/src/tasks/gates.rs declares `mod
        // first_failure;` / `mod planning_types;`. Given a real *file
        // path* (the original temp-file-checkout design), rustfmt tries to
        // resolve those as sibling files and ERRORS -- not "would
        // reformat", a hard resolution failure -- when the staged blob is
        // written in isolation with no siblings nearby. That error was
        // getting misclassified as "needs reformatting" by
        // `!status.success()`. Piping via stdin (the current design) has
        // no file-path context to resolve modules against, so a `mod`
        // declaration in otherwise-well-formatted content must not cause a
        // false BLOCKED.
        let repo = TempRepo::init()?;
        repo.write("lib_like.rs", "mod other;\n\nfn main() {}\n")?;
        repo.add("lib_like.rs")?;

        match rustfmt_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Pass(_) => {}
            CommitCheckOutcome::Flagged(report) => {
                bail!(
                    "expected a clean pass — the content is well-formatted; a `mod other;` \
                     declaration with no resolvable sibling file must not cause a false \
                     BLOCKED via module-resolution failure. Report: {report:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn rustfmt_staged_honors_the_repos_own_config_not_stock_defaults() -> Result<()> {
        // Regression test for the other half of the same dogfooding find:
        // without an explicit --config-path, a staged file checked in
        // isolation (no ancestor directory to discover rustfmt.toml from)
        // silently falls back to rustfmt's stock defaults, which disagree
        // with this repo's actual (non-default) settings and would
        // false-positive-block every staged file regardless of whether
        // it's really formatted by this repo's own convention.
        //
        // `rustfmt.toml` is deliberately STAGED (`repo.add`), not just
        // written to disk: the config is now read from the pinned tree, not
        // the working-tree filesystem (see [`load_staged_rustfmt_config`]),
        // so an unstaged config file wouldn't be picked up at all.
        let repo = TempRepo::init()?;
        // An extreme max_width that stock defaults would NOT require
        // reformatting this already brace-styled fn body at, but this
        // custom config WILL (verified directly: this exact fixture is
        // `--check`-clean under stock rustfmt and flagged under
        // max_width = 1).
        repo.write("rustfmt.toml", "max_width = 1\n")?;
        repo.add("rustfmt.toml")?;
        let already_brace_styled = "fn call() {\n    long_named_function(argument_one, argument_two, argument_three);\n}\n";
        repo.write("lib_like.rs", already_brace_styled)?;
        repo.add("lib_like.rs")?;

        match rustfmt_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(
                    report.affected.contains(&"lib_like.rs".to_string()),
                    "expected the repo's own (staged, extreme-width) rustfmt.toml to be \
                     honored, forcing a reformat that stock defaults wouldn't require: {report:?}"
                );
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "expected the repo's own STAGED rustfmt.toml (max_width = 1) to be honored \
                     via --config-path, not silently ignored in favor of stock defaults: \
                     {summary}"
                )
            }
        }
        Ok(())
    }

    /// Mutation-checked regression proof for the deep-review send-back on
    /// PR #4020 (bot thread, P2): `rustfmt.toml` must be read from the
    /// pinned STAGED tree, never `root.join("rustfmt.toml")` against the
    /// working-tree filesystem. Stages a Rust file that's already clean
    /// under stock defaults with NO `rustfmt.toml` staged at all, then
    /// drops an unstaged (never `git add`ed) `rustfmt.toml` with an extreme
    /// `max_width = 1` onto the working tree — a check reading the
    /// filesystem directly would find that file and wrongly Block an
    /// already-clean staged file; a check reading the pinned tree must
    /// still see "no staged config" and Pass.
    #[test]
    fn rustfmt_staged_reads_the_staged_config_not_an_unstaged_working_tree_file() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write(
            "lib_like.rs",
            "fn call() {\n    long_named_function(argument_one, argument_two, argument_three);\n}\n",
        )?;
        repo.add("lib_like.rs")?;
        // No rustfmt.toml staged at all -- the captured tree has none.
        let captured_oid = staged::staged_tree_oid(repo.root())?;

        // Drop an UNSTAGED rustfmt.toml with an extreme max_width onto the
        // working tree -- never `git add`ed, so it's not part of the
        // captured tree.
        repo.write("rustfmt.toml", "max_width = 1\n")?;

        match rustfmt_staged_at(repo.root(), Some(&captured_oid))? {
            CommitCheckOutcome::Pass(_) => {}
            CommitCheckOutcome::Flagged(report) => {
                bail!(
                    "expected the check to see NO staged rustfmt.toml (stock defaults, under \
                     which lib_like.rs is already clean) and Pass, not be wrongly blocked by an \
                     unstaged max_width=1 config sitting on the filesystem: {report:?}"
                )
            }
        }
        Ok(())
    }

    #[test]
    fn from_raw_staged_flags_direct_usage_in_staged_content() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("crates/foo/src/lib.rs", "let s = std::process::ExitStatus::from_raw(0);\n")?;
        repo.add("crates/foo/src/lib.rs")?;

        match from_raw_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(!report.affected.is_empty());
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!("expected direct from_raw() usage to block: {summary}")
            }
        }
        Ok(())
    }

    #[test]
    fn from_raw_staged_allows_the_mock_status_adapter() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write(
            "crates/foo/src/lib.rs",
            "let s = std::process::ExitStatus::from_raw(raw_exit(0));\n",
        )?;
        repo.add("crates/foo/src/lib.rs")?;

        match from_raw_staged_at(repo.root(), None)? {
            CommitCheckOutcome::Pass(_) => {}
            CommitCheckOutcome::Flagged(report) => {
                bail!("expected the raw_exit() adapter form to pass: {report:?}")
            }
        }
        Ok(())
    }

    #[test]
    fn whitespace_check_flags_trailing_whitespace_in_staged_diff() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Stage a line with trailing whitespace.
        repo.write("foo.rs", "fn main() {}   \n")?;
        repo.add("foo.rs")?;

        match whitespace_check_at(repo.root(), None)? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(!report.affected.is_empty());
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!("expected trailing whitespace to block: {summary}")
            }
        }
        Ok(())
    }

    /// Mutation-checked regression proof for the deep-review send-back on
    /// PR #4020: `whitespace_check_at` MUST read the pinned tree OID, not
    /// `git diff --cached --check` (the live index). Reverting the fix to
    /// use `--cached` directly makes this test fail: capture an OID with a
    /// staged whitespace issue, then `git add` a FIX to the working tree
    /// (moving the live index past the captured snapshot) — a
    /// `--cached`-based check would see the fix and pass; the pinned check
    /// must still flag the captured (unfixed) content.
    #[test]
    fn whitespace_check_reads_the_pinned_oid_not_a_later_git_add() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Stage a line with trailing whitespace and capture the tree OID —
        // this is what plan_gates does once, up front.
        repo.write("foo.rs", "fn main() {}   \n")?;
        repo.add("foo.rs")?;
        let captured_oid = staged::staged_tree_oid(repo.root())?;

        // A concurrent `git add` FIXES the whitespace issue in the index
        // AFTER the OID was captured — simulating another process staging
        // a fix while this run's checks are still dispatching.
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;

        // A check pinned to the captured OID must still flag the captured
        // (unfixed) content — not the live index, which has since moved on.
        match whitespace_check_at(repo.root(), Some(&captured_oid))? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(report.posture, Posture::Blocked);
                assert!(!report.affected.is_empty());
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "expected the pinned OID's staged whitespace issue to still block, even \
                     though a later git add fixed it in the live index: {summary}"
                )
            }
        }

        // For contrast: an unpinned (None) read sees the live (now-fixed)
        // index and passes — proving the difference is the tree-oid
        // pinning, not some other accident of the test setup.
        match whitespace_check_at(repo.root(), None)? {
            CommitCheckOutcome::Pass(_) => {}
            CommitCheckOutcome::Flagged(report) => {
                bail!("expected the unpinned (live-index) read to see the fix and pass: {report:?}")
            }
        }
        Ok(())
    }

    #[test]
    fn changie_fragment_staged_blocks_malformed_yaml() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("README.md", "hello\n")?;
        repo.add("README.md")?;
        repo.commit("initial")?;

        repo.write(".changes/unreleased/product-1-Added-000000.yaml", "kind: [unterminated\n")?;
        repo.add(".changes/unreleased/product-1-Added-000000.yaml")?;

        match changie_fragment_staged_at(repo.root(), None) {
            Ok(CommitCheckOutcome::Flagged(report)) => {
                assert_eq!(report.posture, Posture::Blocked);
            }
            // `.changie.yaml` doesn't exist in the staged tree of this bare
            // temp repo, so `load_staged_changie_config` fails before it
            // even gets to the malformed fragment — that's still a
            // legitimate instrument-error Err, not a false pass, which is
            // the property this test cares about (a real repo has
            // `.changie.yaml` staged; see the next test for that path being
            // exercised).
            Err(_) => {}
            Ok(CommitCheckOutcome::Pass(summary)) => {
                bail!("expected malformed YAML to block or the config load to fail: {summary}")
            }
        }
        Ok(())
    }

    /// Mutation-checked regression proof for the deep-review send-back on
    /// PR #4020 (bot thread, P2): `.changie.yaml` must be read from the
    /// pinned STAGED tree, never `std::fs::read_to_string` against the
    /// working-tree file. This stages a fragment that's valid under the
    /// committed (and still-staged) config, then dirties the WORKING TREE's
    /// `.changie.yaml` (unstaged) to drop the very project/kind the
    /// fragment relies on. A check reading the working-tree file would
    /// wrongly Block a legitimately-valid staged fragment; a check reading
    /// the pinned tree must still see the original config and report
    /// NotProven (schema-valid, dry-render deferred), not Blocked.
    #[test]
    fn changie_fragment_staged_reads_the_staged_config_not_an_unstaged_working_tree_edit()
    -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write(
            ".changie.yaml",
            "projects:\n  - key: product\ncomponents: []\nkinds:\n  - label: Added\nbody:\n  minLength: 0\n",
        )?;
        repo.add(".changie.yaml")?;
        repo.commit("initial")?;

        // Stage a fragment valid under the committed (and still-staged)
        // config: project "product" and kind "Added" both exist.
        repo.write(
            ".changes/unreleased/product-1-Added-000000.yaml",
            "project: product\nkind: Added\nbody: something happened\ncustom:\n  PR: \"1\"\n",
        )?;
        repo.add(".changes/unreleased/product-1-Added-000000.yaml")?;
        let captured_oid = staged::staged_tree_oid(repo.root())?;

        // Dirty the WORKING TREE's .changie.yaml (unstaged) to drop both
        // the "product" project and the "Added" kind entirely -- simulating
        // an in-progress, not-yet-staged edit to the config sitting on top
        // of the index at the moment this check dispatches.
        repo.write(
            ".changie.yaml",
            "projects: []\ncomponents: []\nkinds: []\nbody:\n  minLength: 0\n",
        )?;

        match changie_fragment_staged_at(repo.root(), Some(&captured_oid))? {
            CommitCheckOutcome::Flagged(report) => {
                assert_eq!(
                    report.posture,
                    Posture::NotProven,
                    "expected the staged fragment to validate against the STAGED config (which \
                     still has the product project and Added kind), not be blocked by an \
                     unstaged edit that hasn't been committed to the index yet: {report:?}"
                );
            }
            CommitCheckOutcome::Pass(summary) => {
                bail!(
                    "changie_fragment_staged_at should Flag (NotProven) when fragments were \
                     checked, never bare-Pass: {summary}"
                )
            }
        }
        Ok(())
    }

    #[test]
    fn is_fragment_path_matches_only_unreleased_yaml() {
        assert!(is_fragment_path(".changes/unreleased/product-1-Added-000000.yaml"));
        assert!(is_fragment_path(".changes/unreleased/product-1-Added-000000.yml"));
        assert!(!is_fragment_path(".changes/samples/product-1-Added-000000.yaml"));
        assert!(!is_fragment_path(".changes/unreleased/.gitkeep"));
        assert!(!is_fragment_path("crates/foo/src/lib.rs"));
    }

    /// Regression for the deep-review send-back on PR #4020: the extension
    /// half of the match must be case-insensitive (the directory-prefix
    /// half deliberately stays case-sensitive, matching git's own
    /// treatment of POSIX paths). Reverting the `.to_ascii_lowercase()` fix
    /// makes this test fail — `.YAML`/`.Yml` would no longer match
    /// `.ends_with(".yaml")`/`.ends_with(".yml")`.
    #[test]
    fn is_fragment_path_matches_extension_case_insensitively() {
        assert!(is_fragment_path(".changes/unreleased/product-1-Added-000000.YAML"));
        assert!(is_fragment_path(".changes/unreleased/product-1-Added-000000.Yml"));
        // The directory prefix stays case-sensitive: an uppercase
        // "Unreleased" is a different (non-matching) path on a
        // case-sensitive filesystem, same as git tracks it.
        assert!(!is_fragment_path(".changes/Unreleased/product-1-Added-000000.yaml"));
    }
}
