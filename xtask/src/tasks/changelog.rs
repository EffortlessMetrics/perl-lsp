//! Changelog-ledger checks for the Changie PR-time fragment flow.
//!
//! FOUNDATION / ADVISORY (tracking issue #3784; #3768 is the retrospective
//! baseline artifact only). This task validates that a PR carries an explicit
//! changelog *disposition* — a Changie fragment under `.changes/unreleased/`
//! OR an exemption-with-reason — and that any added fragment is schema-valid
//! and (when `changie` is on `PATH`) renders through `changie batch --dry-run`.
//!
//! This PR does not change release execution. A later reviewed cutover makes
//! Changie batching the changelog-preparation authority; git-cliff remains an
//! audit lens; Cargo/release tooling remains the versioning/tag/publish
//! authority.
//!
//! ## Exit codes (`cargo xtask changelog check`)
//!
//! This checker separates the *policy verdict* from *whether the instrument
//! itself worked* — an advisory checker must fail open on the former, never
//! on the latter:
//!
//! - **0** — policy satisfied, OR an advisory finding was reported (a missing
//!   disposition during the soak window between `advisory_expected_from` and
//!   `blocking_enforced_from`). Both are non-fatal.
//! - **1** — a *blocking* policy violation. Only reachable once
//!   `policy/changelog.toml`'s `blocking_enforced_from` is set AND the PR's
//!   base is at/after that commit. With `blocking_enforced_from` empty (the
//!   state shipped by this PR), this path is unreachable.
//! - **2** — an instrument/config failure: `.changie.yaml` or
//!   `policy/changelog.toml` fails to parse, the changed-file list cannot be
//!   resolved (unreadable `--changed-files`, unspawnable/failing `git diff`),
//!   or `changie` crashes while rendering a fragment. These are tooling
//!   problems, not policy findings, and are never silently downgraded to a
//!   passing exit.
//!
//! ## The three-clock cutoff model
//!
//! See the comment block in `policy/changelog.toml` for the full rationale.
//! In short: `retrospective_covered_through` is the conservative floor the
//! #3768 manual catalog audit covered, `advisory_expected_from` is this PR's
//! (#3775) own merge SHA (empty until it merges — the soak boundary is not
//! yet armed), and `blocking_enforced_from` is a future SHA that gates the
//! only path that can return exit code 1.
//!
//! ## Single policy source
//!
//! `policy/changelog.toml` is deserialized into [`ChangelogPolicy`] and is the
//! *only* place changelog paths, project keys, and exemption categories are
//! declared for this checker — nothing here hand-duplicates it.
//! `.changie.yaml` remains Changie's own render config (kinds, components,
//! body-length schema) and is read separately via [`ChangieConfig`].

use color_eyre::eyre::{Result, eyre};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `pub(crate)`: also read by `commit_checks::changie_fragment_staged_at`,
/// which needs the path (not the on-disk-reading [`load_config`]) so it can
/// read the *staged* config via `staged::read_staged_path_text` instead —
/// see [`parse_config`].
pub(crate) const CHANGIE_CONFIG: &str = ".changie.yaml";
const POLICY_FILE: &str = "policy/changelog.toml";
pub(crate) const UNRELEASED_DIR: &str = ".changes/unreleased";
const SAMPLES_DIR: &str = ".changes/samples";

/// Parsed subset of `.changie.yaml` needed for validation. This is Changie's
/// own render config (kinds/components/body schema) — a different authority
/// from `policy/changelog.toml` ([`ChangelogPolicy`]), which owns repo policy
/// (changelog paths, exemption categories, enforcement clocks).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ChangieConfig {
    #[serde(default)]
    projects: Vec<ChangieProject>,
    #[serde(default)]
    components: Vec<String>,
    #[serde(default)]
    kinds: Vec<ChangieKind>,
    #[serde(default)]
    body: BodyConfig,
}

#[derive(Debug, Deserialize)]
struct ChangieProject {
    key: String,
}

#[derive(Debug, Deserialize)]
struct ChangieKind {
    label: String,
    #[serde(default)]
    key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct BodyConfig {
    #[serde(default, rename = "minLength")]
    min_length: Option<u64>,
}

impl ChangieConfig {
    fn project_keys(&self) -> Vec<&str> {
        self.projects.iter().map(|p| p.key.as_str()).collect()
    }

    /// Kinds are matched case-insensitively against either the label or the key.
    fn has_kind(&self, kind: &str) -> bool {
        self.kinds.iter().any(|k| {
            k.label.eq_ignore_ascii_case(kind)
                || k.key.as_deref().is_some_and(|key| key.eq_ignore_ascii_case(kind))
        })
    }

    fn min_body(&self) -> usize {
        self.body.min_length.unwrap_or(0) as usize
    }
}

/// The sole authority for repo changelog policy: `policy/changelog.toml`,
/// deserialized. Project keys, changelog output paths, and exemption
/// categories are derived from this struct — never hand-duplicated as Rust
/// consts.
#[derive(Debug, Deserialize)]
struct ChangelogPolicy {
    #[allow(dead_code)] // reserved for future schema-migration checks
    schema_version: u32,
    enforcement: Enforcement,
    projects: Vec<ProjectPolicy>,
    exemption_categories: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)] // documents the retrospective floor; not consumed by this checker
    retrospective_covered_through: Option<String>,
    #[serde(default)]
    advisory_expected_from: Option<String>,
    #[serde(default)]
    blocking_enforced_from: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectPolicy {
    #[allow(dead_code)] // reserved for future project-configuration checks
    key: String,
    #[allow(dead_code)] // reserved for changelog header rendering
    label: String,
    changelog: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Enforcement {
    Advisory,
    Blocking,
}

impl ChangelogPolicy {
    /// Repo-relative paths of every project's generated changelog file, e.g.
    /// `CHANGELOG.md`, `vscode-extension/CHANGELOG.md`.
    fn changelog_paths(&self) -> Vec<&str> {
        self.projects.iter().map(|p| p.changelog.as_str()).collect()
    }

    /// Exemption categories accepted in a PR-body marker, including the
    /// syntactic `none` keyword (`changelog: none — ...`), which is not a
    /// policy category but a recognized marker value.
    fn exemption_marker_categories(&self) -> Vec<String> {
        let mut cats = self.exemption_categories.clone();
        if !cats.iter().any(|c| c == "none") {
            cats.push("none".to_string());
        }
        cats
    }
}

/// A parsed Changie fragment (the fields this check cares about).
#[derive(Debug, Deserialize)]
pub(crate) struct Fragment {
    #[serde(default)]
    project: String,
    #[serde(default)]
    component: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    custom: BTreeMap<String, String>,
}

/// Validate one fragment against the config; return human-readable findings
/// (empty vec => valid).
///
/// `pub(crate)`: reused by the commit-tier staged Changie-fragment check
/// (`commit_checks::changie_fragment_staged`, issue #3786) so fragment-schema
/// validation has exactly one implementation, not a second copy at commit
/// time.
pub(crate) fn validate_fragment(frag: &Fragment, cfg: &ChangieConfig) -> Vec<String> {
    let mut findings = Vec::new();

    let keys = cfg.project_keys();
    if frag.project.is_empty() {
        findings.push("missing `project` field".to_string());
    } else if !keys.iter().any(|k| *k == frag.project) {
        findings.push(format!(
            "unknown project `{}` (expected one of: {})",
            frag.project,
            keys.join(", ")
        ));
    }

    if frag.kind.is_empty() {
        findings.push("missing `kind` field".to_string());
    } else if !cfg.has_kind(&frag.kind) {
        findings.push(format!("unknown kind `{}`", frag.kind));
    }

    if !frag.component.is_empty() && !cfg.components.iter().any(|c| c == &frag.component) {
        findings.push(format!("unknown component `{}`", frag.component));
    }

    let min = cfg.min_body();
    if frag.body.trim().chars().count() < min {
        findings.push(format!("body too short (need >= {min} chars): {:?}", frag.body.trim()));
    }

    match frag.custom.get("PR") {
        None => findings.push("missing custom `PR` metadata".to_string()),
        Some(pr) => match pr.parse::<u64>() {
            Ok(n) if n >= 1 => {}
            _ => findings.push(format!("custom `PR` must be a positive integer, got {pr:?}")),
        },
    }

    if let Some(breaking) = frag.custom.get("Breaking")
        && breaking != "no"
        && breaking != "yes"
    {
        findings.push(format!("custom `Breaking` must be `no` or `yes`, got {breaking:?}"));
    }

    findings
}

/// A PR's changelog disposition.
#[derive(Debug, PartialEq, Eq)]
enum Disposition {
    /// One or more fragment files were added under `.changes/unreleased/`.
    Fragment(Vec<String>),
    /// An explicit exemption (PR-body marker or `.changes/exemptions/*` file).
    Exemption(String),
    /// A recognized release-prep PR (version bump + changelog batch).
    ReleasePrep,
    /// No explicit disposition found.
    Missing,
}

/// The overall policy verdict for a `check()` run. This is distinct from
/// instrument failure (an `Err` from `check()`): every variant here means the
/// instrument worked and produced a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    /// A disposition was found, or none was expected yet (boundary not armed).
    PolicySatisfied,
    /// A disposition was expected (advisory boundary armed) and missing.
    /// Reported as a finding; still exits 0.
    AdvisoryFinding,
    /// A disposition was required (blocking boundary reached) and missing.
    /// The only outcome that should map to a non-zero (1) exit code.
    BlockingViolation,
}

/// Where a PR's base sits relative to the three-clock policy boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Boundary {
    /// Neither the advisory nor the blocking boundary has been reached (or
    /// they are unset / unresolvable) — a missing disposition is not (yet) a
    /// reportable finding.
    NotArmed,
    /// The advisory boundary is armed: a missing disposition is a reported
    /// finding, but never fails the check.
    Advisory,
    /// The blocking boundary is reached: a missing disposition is a blocking
    /// violation.
    Blocking,
}

/// Parse a PR-body exemption marker.
///
/// Accepts, case-insensitively, a line of the form:
/// `changelog-exempt: <category> — <reason>` or `changelog: none — <reason>`
/// (the separator may be `—`, `-`, or `:`). `categories` is the policy's
/// exemption categories plus the syntactic `none` keyword.
fn parse_exemption_marker(pr_body: &str, categories: &[String]) -> Option<(String, String)> {
    for raw in pr_body.lines() {
        // Strip Markdown list/quote decoration and surrounding whitespace.
        let line = raw.trim().trim_start_matches(['>', '*', '-', ' ']).trim();
        let lower = line.to_ascii_lowercase();
        // `lower` has the same byte length as `line` (ASCII-only case folding),
        // so slicing by a suffix length stays on a char boundary.
        let tail = if let Some(t) = lower.strip_prefix("changelog-exempt:") {
            &line[line.len() - t.len()..]
        } else if let Some(t) = lower.strip_prefix("changelog:") {
            &line[line.len() - t.len()..]
        } else {
            continue;
        };
        let tail = tail.trim();
        let tail_lower = tail.to_ascii_lowercase();
        for cat in categories {
            let Some(after) = tail_lower.strip_prefix(cat.as_str()) else {
                continue;
            };
            // The category must be a whole token, not a prefix of a longer word.
            let next = after.chars().next();
            if matches!(next, None | Some(' ') | Some('-') | Some('—') | Some(':')) {
                let reason_start = tail.len() - after.len();
                let reason = tail[reason_start..]
                    .trim_start_matches([' ', '-', '—', ':'])
                    .trim()
                    .to_string();
                return Some((cat.clone(), reason));
            }
        }
    }
    None
}

/// Files under `.changes/unreleased/` that are fragments (exclude `.gitkeep`).
fn fragment_files(changed: &[String]) -> Vec<String> {
    changed
        .iter()
        .filter(|f| {
            let norm = f.replace('\\', "/");
            norm.starts_with(&format!("{UNRELEASED_DIR}/")) && norm.ends_with(".yaml")
        })
        .cloned()
        .collect()
}

/// Heuristic: is this a release-prep PR (version bump + changelog batch)?
fn is_release_prep(changed: &[String], changelog_paths: &[&str]) -> bool {
    if changed.is_empty() {
        return false;
    }
    changed.iter().all(|f| {
        let n = f.replace('\\', "/");
        changelog_paths.contains(&n.as_str())
            || n == "Cargo.toml"
            || n == "Cargo.lock"
            || n.ends_with("/Cargo.toml")
            || n == "features.toml"
            || n == "vscode-extension/package.json"
            || n == "vscode-extension/package-lock.json"
            || n.starts_with(".changes/")
    })
}

/// Best-effort mapping of a changed path to an exemption category (for hints).
fn exempt_category_for_path(path: &str, changelog_paths: &[&str]) -> Option<&'static str> {
    let n = path.replace('\\', "/");
    if changelog_paths.contains(&n.as_str()) {
        return None; // handled by direct-edit detection
    }
    if n == ".changie.yaml"
        || n.starts_with(".changes/")
        || n == "cliff.toml"
        || n == "docs/CHANGELOG_WORKFLOW.md"
        || n == "policy/changelog.toml"
    {
        return Some("changelog-tooling");
    }
    if n == "docs/project/CURRENT_STATUS.md" || n.starts_with("docs/project/status/") {
        return Some("generated-status");
    }
    if n == "Cargo.lock" {
        return Some("deps");
    }
    if n.starts_with(".github/")
        || n.starts_with(".ci/")
        || n.starts_with("scripts/ci/")
        || n.starts_with("policy/")
    {
        return Some("ci");
    }
    if n.contains("/tests/") || n.ends_with("_test.rs") {
        return Some("tests");
    }
    if n.starts_with("docs/") || n.ends_with(".md") {
        return Some("docs-no-contract-change");
    }
    None
}

/// Detect the PR's disposition from its changed files and PR body.
fn detect_disposition(changed: &[String], pr_body: &str, policy: &ChangelogPolicy) -> Disposition {
    let frags = fragment_files(changed);
    if !frags.is_empty() {
        return Disposition::Fragment(frags);
    }
    let categories = policy.exemption_marker_categories();
    if let Some((cat, reason)) = parse_exemption_marker(pr_body, &categories) {
        let reason = if reason.is_empty() { "(no reason given)".to_string() } else { reason };
        return Disposition::Exemption(format!("{cat}: {reason}"));
    }
    if changed.iter().any(|f| {
        let n = f.replace('\\', "/");
        n.starts_with(".changes/exemptions/") && n.ends_with(".md")
    }) {
        return Disposition::Exemption("file-based (.changes/exemptions/)".to_string());
    }
    let changelog_paths = policy.changelog_paths();
    if is_release_prep(changed, &changelog_paths) {
        return Disposition::ReleasePrep;
    }
    Disposition::Missing
}

/// Direct-edit warning: feature PRs should not hand-edit the generated
/// changelogs. Returns a warning message when applicable.
fn direct_changelog_edit_warning(
    changed: &[String],
    is_release_prep: bool,
    changelog_paths: &[&str],
) -> Option<String> {
    if is_release_prep {
        return None;
    }
    let edited: Vec<&str> = changed
        .iter()
        .map(|f| f.as_str())
        .filter(|f| {
            let n = f.replace('\\', "/");
            changelog_paths.contains(&n.as_str())
        })
        .collect();
    if edited.is_empty() {
        None
    } else {
        Some(format!(
            "directly edits generated changelog(s): {}. Feature PRs should add a \
             Changie fragment (`changie new`) instead of hand-editing these files.",
            edited.join(", ")
        ))
    }
}

/// Is `changie` available on `PATH`?
fn changie_available() -> bool {
    Command::new("changie").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Pure decision of what a `changie` invocation's result means. Split out of
/// [`render_project`] so the failure path (instrument crash) is unit-testable
/// without spawning a process.
fn render_outcome(
    success: bool,
    code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    project: &str,
) -> std::result::Result<Option<String>, String> {
    if success {
        Ok(Some(String::from_utf8_lossy(stdout).into_owned()))
    } else {
        Err(format!(
            "`changie batch --project {project} --dry-run` failed (exit {code:?}): {}",
            String::from_utf8_lossy(stderr).trim()
        ))
    }
}

/// Render a project's unreleased fragments via `changie batch --dry-run --keep`.
///
/// Returns `Ok(None)` when changie ran successfully but produced no output;
/// returns `Err` when changie fails to spawn or exits non-zero — a genuine
/// instrument failure, not a policy finding. Callers propagate `Err` as an
/// instrument failure (exit 2), never silently degrade it.
fn render_project(root: &Path, project: &str) -> std::result::Result<Option<String>, String> {
    let out = Command::new("changie")
        .current_dir(root)
        .args(["batch", "v0.0.0-advisory-selftest", "--project", project, "--dry-run", "--keep"])
        .output()
        .map_err(|e| format!("failed to spawn `changie` for project `{project}`: {e}"))?;
    render_outcome(out.status.success(), out.status.code(), &out.stdout, &out.stderr, project)
}

/// Read the changed-files list, either from an explicit file or via git diff.
///
/// Instrument failure: an unreadable `--changed-files` list, an unspawnable
/// `git`, or a failing `git diff` are all genuine "cannot resolve changed
/// files" instrument failures and are surfaced as `Err` — never silently
/// downgraded to an empty list. An empty *result* (the diff legitimately has
/// no changed files) is a normal `Ok(vec![])`.
fn read_changed_files(
    root: &Path,
    changed_files: Option<&Path>,
    base: &str,
) -> std::result::Result<Vec<String>, String> {
    if let Some(list) = changed_files {
        return std::fs::read_to_string(list)
            .map(|content| {
                content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .map_err(|e| format!("failed to read changed-files list {}: {e}", list.display()));
    }
    let out = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .map_err(|e| format!("failed to spawn `git diff`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`git diff {base}...HEAD` failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub(crate) fn load_config(root: &Path) -> Result<ChangieConfig> {
    let path = root.join(CHANGIE_CONFIG);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;
    parse_config(&content).map_err(|e| eyre!("failed to parse {}: {e}", path.display()))
}

/// Parse already-read `.changie.yaml` content. Split out from [`load_config`]
/// so a caller that already has the config text from somewhere other than
/// the working-tree filesystem — `commit_checks::changie_fragment_staged_at`
/// reads it from the *staged* tree via `staged::read_staged_path_text`, not
/// `std::fs` — doesn't have to duplicate the deserialization step (or worse,
/// silently validate staged fragments against a stale/unstaged on-disk
/// config).
pub(crate) fn parse_config(text: &str) -> Result<ChangieConfig> {
    serde_yaml_ng::from_str(text).map_err(|e| eyre!("failed to parse Changie config: {e}"))
}

fn load_policy(root: &Path) -> Result<ChangelogPolicy> {
    let path = root.join(POLICY_FILE);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| eyre!("failed to parse {}: {e}", path.display()))
}

/// Load a fragment from disk and validate it; append findings to `report`.
/// A malformed fragment is a POLICY finding (the author's input is invalid),
/// not an instrument failure — the checker itself works fine.
fn check_fragment_file(root: &Path, rel: &str, cfg: &ChangieConfig, report: &mut Report) {
    let path = root.join(rel);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            report.warn(format!("could not read fragment {rel}: {e}"));
            return;
        }
    };
    match serde_yaml_ng::from_str::<Fragment>(&content) {
        Ok(frag) => {
            let findings = validate_fragment(&frag, cfg);
            if findings.is_empty() {
                report.ok(format!("fragment {rel} is schema-valid"));
            } else {
                for f in findings {
                    report.warn(format!("{rel}: {f}"));
                }
            }
        }
        Err(e) => report.warn(format!("fragment {rel} does not parse as YAML: {e}")),
    }
}

/// Accumulates advisory findings for a single run.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
}

impl Report {
    fn ok(&mut self, msg: impl Into<String>) {
        self.lines.push(format!("  OK   {}", msg.into()));
    }
    fn info(&mut self, msg: impl Into<String>) {
        self.lines.push(format!("  INFO {}", msg.into()));
    }
    fn warn(&mut self, msg: impl Into<String>) {
        self.lines.push(format!("  WARN {}", msg.into()));
    }
    fn emit(&self) {
        for l in &self.lines {
            println!("{l}");
        }
    }
}

/// Entry point for `cargo xtask changelog check`.
///
/// Returns `Err` only for instrument/config failures (exit 2 at the CLI
/// layer); every reachable policy verdict — satisfied, advisory finding, or
/// blocking violation — is an `Ok(CheckOutcome)`. See the module docs for the
/// full exit-code contract.
pub fn check(
    base: Option<String>,
    changed_files: Option<PathBuf>,
    pr_body_file: Option<PathBuf>,
    self_test: bool,
    root: Option<PathBuf>,
) -> Result<CheckOutcome> {
    let root = match root {
        Some(r) => r,
        None => crate::utils::project_root()?,
    };

    println!("changelog check (tracking issue #3784; foundation PR #3775)");

    let policy = load_policy(&root)?;
    let cfg = load_config(&root)?;
    report_policy(&policy);

    let mut report = Report::default();

    if self_test {
        run_self_test(&root, &cfg, &mut report).map_err(|e| eyre!(e))?;
        report.emit();
        return Ok(CheckOutcome::PolicySatisfied);
    }

    let base = base.unwrap_or_else(|| "origin/main".to_string());
    let changed =
        read_changed_files(&root, changed_files.as_deref(), &base).map_err(|e| eyre!(e))?;
    let pr_body = match pr_body_file {
        Some(p) => std::fs::read_to_string(&p).unwrap_or_default(),
        None => std::env::var("CHANGELOG_PR_BODY").unwrap_or_default(),
    };

    if changed.is_empty() {
        report.info("no changed files resolved (nothing to check)");
        report.emit();
        return Ok(CheckOutcome::PolicySatisfied);
    }

    let changelog_paths = policy.changelog_paths();
    let disposition = detect_disposition(&changed, &pr_body, &policy);
    let outcome = match &disposition {
        Disposition::Fragment(frags) => {
            report.ok(format!("disposition: {} Changie fragment(s) added", frags.len()));
            for rel in frags {
                check_fragment_file(&root, rel, &cfg, &mut report);
            }
            // Render each project that has unreleased fragments.
            render_disposition_projects(&root, &mut report).map_err(|e| eyre!(e))?;
            CheckOutcome::PolicySatisfied
        }
        Disposition::Exemption(reason) => {
            report.ok(format!("disposition: exemption — {reason}"));
            CheckOutcome::PolicySatisfied
        }
        Disposition::ReleasePrep => {
            report.ok("disposition: recognized release-prep PR (version bump + changelog batch)");
            CheckOutcome::PolicySatisfied
        }
        Disposition::Missing => {
            let boundary = boundary_state(&root, &policy, &base);
            match boundary {
                Boundary::NotArmed => {
                    report.info(
                        "no explicit changelog disposition found, but the advisory boundary \
                         (`advisory_expected_from`) is not yet armed for this PR's base — not \
                         reported as a finding.",
                    );
                    report_category_hint(&changed, &changelog_paths, &mut report);
                    CheckOutcome::PolicySatisfied
                }
                Boundary::Advisory => {
                    report.warn(
                        "no explicit changelog disposition found. Add a Changie fragment \
                         (`changie new`) OR a PR-body marker: `changelog-exempt: <category> — \
                         <reason>`.",
                    );
                    report_category_hint(&changed, &changelog_paths, &mut report);
                    CheckOutcome::AdvisoryFinding
                }
                Boundary::Blocking => {
                    report.warn(
                        "BLOCKING: no explicit changelog disposition found and the enforcement \
                         boundary (`blocking_enforced_from`) has been reached for this PR's \
                         base. Add a Changie fragment (`changie new`) OR a PR-body marker: \
                         `changelog-exempt: <category> — <reason>`.",
                    );
                    report_category_hint(&changed, &changelog_paths, &mut report);
                    CheckOutcome::BlockingViolation
                }
            }
        }
    };

    if let Some(warning) = direct_changelog_edit_warning(
        &changed,
        disposition == Disposition::ReleasePrep,
        &changelog_paths,
    ) {
        report.warn(warning);
    }

    report.emit();
    Ok(outcome)
}

/// Print the policy's enforcement mode and the three cutoff clocks.
fn report_policy(policy: &ChangelogPolicy) {
    println!(
        "  policy: enforcement={}",
        match policy.enforcement {
            Enforcement::Advisory => "advisory",
            Enforcement::Blocking => "blocking",
        }
    );
    match non_empty(&policy.retrospective_covered_through) {
        Some(sha) => println!("  policy: retrospective_covered_through={sha}"),
        None => println!("  policy: retrospective_covered_through not set"),
    }
    match non_empty(&policy.advisory_expected_from) {
        Some(sha) => println!("  policy: advisory_expected_from={sha}"),
        None => {
            println!("  policy: advisory_expected_from not set — advisory boundary not yet armed")
        }
    }
    match non_empty(&policy.blocking_enforced_from) {
        Some(sha) => println!("  policy: blocking_enforced_from={sha}"),
        None => println!(
            "  policy: blocking_enforced_from not set — no blocking exit path is reachable"
        ),
    }
}

fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.is_empty())
}

/// Is `sha` an ancestor of `base` in `root`'s git history? `None` means the
/// check was inconclusive (e.g. `sha` unresolvable in a shallow clone) — a
/// soft degrade to "not confirmed" (treated as `false` by callers), not an
/// instrument failure. This is a soak-period safety gate, not a correctness
/// check on user input, so failing open here (never escalating a boundary
/// during an unresolvable ancestry check) is the advisory-correct choice.
fn is_ancestor(root: &Path, sha: &str, base: &str) -> Option<bool> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["merge-base", "--is-ancestor", sha, base])
        .output()
        .ok()?;
    match out.status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// Resolve which policy boundary (if any) applies to a PR whose diff base is
/// `base`. Blocking is only reachable when `enforcement` is `Blocking` AND
/// `blocking_enforced_from` is set AND that commit is at/before `base`.
fn boundary_state(root: &Path, policy: &ChangelogPolicy, base: &str) -> Boundary {
    if policy.enforcement == Enforcement::Blocking
        && let Some(sha) = non_empty(&policy.blocking_enforced_from)
        && is_ancestor(root, sha, base) == Some(true)
    {
        return Boundary::Blocking;
    }
    if let Some(sha) = non_empty(&policy.advisory_expected_from)
        && is_ancestor(root, sha, base) == Some(true)
    {
        return Boundary::Advisory;
    }
    Boundary::NotArmed
}

/// Render every project that has at least one unreleased fragment.
fn render_disposition_projects(
    root: &Path,
    report: &mut Report,
) -> std::result::Result<(), String> {
    if !changie_available() {
        report.info("changie not on PATH; skipping render validation (advisory)");
        return Ok(());
    }
    let unreleased = root.join(UNRELEASED_DIR);
    let projects = fragment_projects_on_disk(&unreleased);
    if projects.is_empty() {
        report.info("no unreleased fragments on disk to render");
        return Ok(());
    }
    for project in projects {
        match render_project(root, &project)? {
            Some(rendered) if !rendered.trim().is_empty() => {
                report.ok(format!("`changie batch --project {project} --dry-run` rendered OK"));
            }
            _ => report.info(format!("render for project `{project}` produced no output")),
        }
    }
    Ok(())
}

/// Distinct project keys present in the on-disk unreleased fragments.
fn fragment_projects_on_disk(unreleased: &Path) -> Vec<String> {
    let mut projects = Vec::new();
    let Ok(entries) = std::fs::read_dir(unreleased) else {
        return projects;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(frag) = serde_yaml_ng::from_str::<Fragment>(&content)
            && !frag.project.is_empty()
            && !projects.contains(&frag.project)
        {
            projects.push(frag.project);
        }
    }
    projects
}

/// For a Missing disposition, hint the likely exemption category.
fn report_category_hint(changed: &[String], changelog_paths: &[&str], report: &mut Report) {
    let cats: Vec<&str> =
        changed.iter().filter_map(|f| exempt_category_for_path(f, changelog_paths)).collect();
    if !cats.is_empty() && cats.len() == changed.len() {
        let mut uniq: Vec<&str> = cats.clone();
        uniq.sort_unstable();
        uniq.dedup();
        report.info(format!(
            "all changed files look like `{}` — an exemption likely applies",
            uniq.join(", ")
        ));
    }
}

/// `--self-test`: validate sample fragments and (if changie is present) render
/// them in a throwaway workspace to prove the config + render pipeline.
fn run_self_test(
    root: &Path,
    cfg: &ChangieConfig,
    report: &mut Report,
) -> std::result::Result<(), String> {
    let samples = root.join(SAMPLES_DIR);
    let Ok(entries) = std::fs::read_dir(&samples) else {
        report.warn(format!("no samples directory at {}", samples.display()));
        return Ok(());
    };
    let mut sample_files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    sample_files.sort();

    if sample_files.is_empty() {
        report.warn("no sample fragments found");
        return Ok(());
    }

    let mut valid_by_project: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in &sample_files {
        let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml_ng::from_str::<Fragment>(&content) {
                Ok(frag) => {
                    let findings = validate_fragment(&frag, cfg);
                    if findings.is_empty() {
                        report.ok(format!("sample {rel} is schema-valid"));
                        valid_by_project
                            .entry(frag.project.clone())
                            .or_default()
                            .push(path.clone());
                    } else {
                        for f in findings {
                            report.warn(format!("{rel}: {f}"));
                        }
                    }
                }
                Err(e) => report.warn(format!("sample {rel} does not parse: {e}")),
            },
            Err(e) => report.warn(format!("could not read sample {rel}: {e}")),
        }
    }

    if !changie_available() {
        report.info("changie not on PATH; skipping sample render (advisory)");
        return Ok(());
    }
    render_samples_in_tempdir(root, &valid_by_project, report)
}

/// Copy the config + samples into a temp workspace and render each project.
fn render_samples_in_tempdir(
    root: &Path,
    by_project: &BTreeMap<String, Vec<PathBuf>>,
    report: &mut Report,
) -> std::result::Result<(), String> {
    let tmp = match tempfile::tempdir() {
        Ok(t) => t,
        Err(e) => {
            report.info(format!("could not create temp dir for render self-test: {e}"));
            return Ok(());
        }
    };
    let tmp_root = tmp.path();
    let setup = || -> std::io::Result<()> {
        std::fs::copy(root.join(CHANGIE_CONFIG), tmp_root.join(CHANGIE_CONFIG))?;
        let changes = tmp_root.join(".changes");
        std::fs::create_dir_all(changes.join("unreleased"))?;
        std::fs::copy(root.join(".changes/header.tpl.md"), changes.join("header.tpl.md"))?;
        Ok(())
    };
    if let Err(e) = setup() {
        report.info(format!("temp workspace setup failed: {e}"));
        return Ok(());
    }

    for (project, files) in by_project {
        if project.is_empty() {
            continue;
        }
        // Refresh the unreleased dir with just this project's samples.
        let unreleased = tmp_root.join(".changes/unreleased");
        let _ = clear_dir(&unreleased);
        for (i, src) in files.iter().enumerate() {
            let dest = unreleased.join(format!("{project}-sample-{i}.yaml"));
            if let Err(e) = std::fs::copy(src, &dest) {
                report.info(format!("could not stage sample for `{project}`: {e}"));
            }
        }
        match render_project(tmp_root, project)? {
            Some(rendered) if rendered.contains("pull/") => {
                report
                    .ok(format!("sample render for `{project}` produced a PR-linked change line"));
            }
            Some(rendered) if !rendered.trim().is_empty() => {
                report.ok(format!("sample render for `{project}` produced output"));
            }
            _ => report.info(format!("sample render for `{project}` produced no output")),
        }
    }
    Ok(())
}

fn clear_dir(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return std::fs::create_dir_all(dir);
    }
    for entry in std::fs::read_dir(dir)?.flatten() {
        let _ = std::fs::remove_file(entry.path());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ChangieConfig {
        let yaml = r#"
projects:
  - key: product
  - key: vscode
components:
  - Developer experience
  - Editor behavior
kinds:
  - label: Added
  - label: Fixed
body:
  minLength: 12
"#;
        serde_yaml_ng::from_str(yaml).expect("test config parses")
    }

    fn test_policy(advisory_expected_from: &str, blocking_enforced_from: &str) -> ChangelogPolicy {
        ChangelogPolicy {
            schema_version: 1,
            enforcement: Enforcement::Advisory,
            projects: vec![
                ProjectPolicy {
                    key: "product".to_string(),
                    label: "Product".to_string(),
                    changelog: "CHANGELOG.md".to_string(),
                },
                ProjectPolicy {
                    key: "vscode".to_string(),
                    label: "VS Code extension".to_string(),
                    changelog: "vscode-extension/CHANGELOG.md".to_string(),
                },
            ],
            exemption_categories: vec![
                "tests".to_string(),
                "ci".to_string(),
                "refactor".to_string(),
                "generated-status".to_string(),
                "docs-no-contract-change".to_string(),
                "deps".to_string(),
                "release-prep".to_string(),
                "changelog-tooling".to_string(),
            ],
            retrospective_covered_through: Some("da86c123a".to_string()),
            advisory_expected_from: non_empty_owned(advisory_expected_from),
            blocking_enforced_from: non_empty_owned(blocking_enforced_from),
        }
    }

    fn non_empty_owned(s: &str) -> Option<String> {
        if s.is_empty() { None } else { Some(s.to_string()) }
    }

    fn good_fragment() -> Fragment {
        let yaml = r#"
project: product
component: Developer experience
kind: Added
body: A sufficiently long changelog body line.
custom:
  PR: "3768"
  Breaking: "no"
"#;
        serde_yaml_ng::from_str(yaml).expect("fragment parses")
    }

    #[test]
    fn valid_fragment_has_no_findings() {
        let cfg = test_config();
        assert!(validate_fragment(&good_fragment(), &cfg).is_empty());
    }

    #[test]
    fn unknown_project_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.project = "nope".to_string();
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("unknown project")), "{findings:?}");
    }

    #[test]
    fn unknown_kind_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.kind = "Frobnicated".to_string();
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("unknown kind")), "{findings:?}");
    }

    #[test]
    fn short_body_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.body = "short".to_string();
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("body too short")), "{findings:?}");
    }

    #[test]
    fn missing_pr_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.custom.remove("PR");
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("PR")), "{findings:?}");
    }

    #[test]
    fn non_numeric_pr_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.custom.insert("PR".to_string(), "abc".to_string());
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("positive integer")), "{findings:?}");
    }

    #[test]
    fn invalid_breaking_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.custom.insert("Breaking".to_string(), "maybe".to_string());
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("Breaking")), "{findings:?}");
    }

    #[test]
    fn unknown_component_is_flagged() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.component = "Nonexistent area".to_string();
        let findings = validate_fragment(&frag, &cfg);
        assert!(findings.iter().any(|f| f.contains("unknown component")), "{findings:?}");
    }

    #[test]
    fn kind_matches_case_insensitively() {
        let cfg = test_config();
        let mut frag = good_fragment();
        frag.kind = "added".to_string();
        assert!(validate_fragment(&frag, &cfg).is_empty());
    }

    #[test]
    fn fragment_files_filters_unreleased_yaml() {
        let changed = vec![
            ".changes/unreleased/product-1-Added-101010.yaml".to_string(),
            ".changes/unreleased/.gitkeep".to_string(),
            "src/lib.rs".to_string(),
            ".changes/samples/product-example.yaml".to_string(),
        ];
        let frags = fragment_files(&changed);
        assert_eq!(frags, vec![".changes/unreleased/product-1-Added-101010.yaml".to_string()]);
    }

    #[test]
    fn disposition_prefers_fragment() {
        let policy = test_policy("", "");
        let changed = vec![".changes/unreleased/product-1-Added-101010.yaml".to_string()];
        assert!(matches!(detect_disposition(&changed, "", &policy), Disposition::Fragment(_)));
    }

    #[test]
    fn disposition_reads_pr_body_marker() {
        let policy = test_policy("", "");
        let changed = vec!["src/lib.rs".to_string()];
        let body = "This PR does X.\nchangelog-exempt: refactor — internal cleanup only\n";
        match detect_disposition(&changed, body, &policy) {
            Disposition::Exemption(reason) => {
                assert!(reason.starts_with("refactor:"), "{reason}");
                assert!(reason.contains("internal cleanup"), "{reason}");
            }
            other => panic!("expected exemption, got {other:?}"),
        }
    }

    #[test]
    fn disposition_recognizes_release_prep() {
        let policy = test_policy("", "");
        let changed = vec!["CHANGELOG.md".to_string(), "Cargo.toml".to_string()];
        assert_eq!(detect_disposition(&changed, "", &policy), Disposition::ReleasePrep);
    }

    #[test]
    fn disposition_recognizes_exemption_file() {
        let policy = test_policy("", "");
        let changed = vec![".changes/exemptions/my-note.md".to_string()];
        assert!(matches!(detect_disposition(&changed, "", &policy), Disposition::Exemption(_)));
    }

    #[test]
    fn disposition_missing_when_nothing_declared() {
        let policy = test_policy("", "");
        let changed = vec!["crates/perl-parser/src/lib.rs".to_string()];
        assert_eq!(detect_disposition(&changed, "", &policy), Disposition::Missing);
    }

    #[test]
    fn none_marker_is_accepted() {
        let policy = test_policy("", "");
        let body = "changelog: none — trivial typo fix";
        let categories = policy.exemption_marker_categories();
        let parsed = parse_exemption_marker(body, &categories);
        assert!(parsed.is_some());
        let (cat, reason) = parsed.expect("parsed");
        assert_eq!(cat, "none");
        assert!(reason.contains("typo"), "{reason}");
    }

    #[test]
    fn direct_edit_warns_for_feature_pr() {
        let paths = ["CHANGELOG.md", "vscode-extension/CHANGELOG.md"];
        let changed = vec!["CHANGELOG.md".to_string(), "crates/perl-parser/src/lib.rs".to_string()];
        assert!(direct_changelog_edit_warning(&changed, false, &paths).is_some());
    }

    #[test]
    fn direct_edit_silent_for_release_prep() {
        let paths = ["CHANGELOG.md", "vscode-extension/CHANGELOG.md"];
        let changed = vec!["CHANGELOG.md".to_string()];
        assert!(direct_changelog_edit_warning(&changed, true, &paths).is_none());
    }

    #[test]
    fn exempt_category_for_path_maps_known_paths() {
        let paths = ["CHANGELOG.md", "vscode-extension/CHANGELOG.md"];
        assert_eq!(exempt_category_for_path(".github/workflows/x.yml", &paths), Some("ci"));
        assert_eq!(exempt_category_for_path("crates/x/tests/a.rs", &paths), Some("tests"));
        assert_eq!(exempt_category_for_path("Cargo.lock", &paths), Some("deps"));
        assert_eq!(exempt_category_for_path(".changie.yaml", &paths), Some("changelog-tooling"));
        assert_eq!(
            exempt_category_for_path("docs/project/status/lsp.md", &paths),
            Some("generated-status")
        );
        assert_eq!(exempt_category_for_path("CHANGELOG.md", &paths), None);
    }

    // --- render_outcome: instrument-failure unit coverage (no process spawn) ---

    #[test]
    fn render_outcome_success_yields_output() {
        let r = render_outcome(true, Some(0), b"rendered output", b"", "product");
        assert_eq!(r, Ok(Some("rendered output".to_string())));
    }

    #[test]
    fn render_outcome_failure_is_instrument_error() {
        let r = render_outcome(false, Some(1), b"", b"boom: schema mismatch", "product");
        let msg = r.expect_err("nonzero exit must be Err, not a silent None");
        assert!(msg.contains("product"), "{msg}");
        assert!(msg.contains("boom"), "{msg}");
    }

    // --- boundary_state: three-clock cutoff coverage ---

    #[test]
    fn boundary_not_armed_when_advisory_expected_from_empty() {
        let policy = test_policy("", "");
        let boundary = boundary_state(Path::new("."), &policy, "HEAD");
        assert_eq!(boundary, Boundary::NotArmed);
    }

    #[test]
    fn boundary_not_armed_when_ancestry_unresolvable() -> std::result::Result<(), String> {
        // A non-git directory makes `git merge-base` fail (inconclusive), which
        // must degrade to NotArmed, not escalate. Each test gets its own
        // tempfile::tempdir() (not a fixed name under the shared temp
        // namespace) so parallel test threads / concurrent agents on the same
        // machine can never race on the same path (see #3435).
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let policy = test_policy("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef", "");
        let boundary = boundary_state(tmp.path(), &policy, "HEAD");
        assert_eq!(boundary, Boundary::NotArmed);
        Ok(())
    }

    #[test]
    fn boundary_never_blocking_when_blocking_enforced_from_empty() {
        // Even with enforcement=Blocking conceptually reachable, an empty
        // blocking_enforced_from must never produce Boundary::Blocking.
        let mut policy = test_policy("", "");
        policy.enforcement = Enforcement::Blocking;
        policy.blocking_enforced_from = None;
        let boundary = boundary_state(Path::new("."), &policy, "HEAD");
        assert_ne!(boundary, Boundary::Blocking);
    }

    // --- read_changed_files: instrument-failure (not silent-empty) coverage ---

    #[test]
    fn read_changed_files_missing_list_is_instrument_failure() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let missing = tmp.path().join("does-not-exist.txt");
        let result = read_changed_files(Path::new("."), Some(&missing), "origin/main");
        result.expect_err("an unreadable --changed-files path must be Err, not silent-empty");
        Ok(())
    }

    #[test]
    fn read_changed_files_unspawnable_git_is_instrument_failure() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let result = read_changed_files(tmp.path(), None, "origin/main");
        result.expect_err("a failing `git diff` must be Err, not silent-empty");
        Ok(())
    }

    #[test]
    fn read_changed_files_valid_list_parses() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let list = tmp.path().join("changed.txt");
        std::fs::write(&list, "src/lib.rs\n\n  crates/foo/src/bar.rs  \n")
            .map_err(|e| e.to_string())?;
        let changed = read_changed_files(Path::new("."), Some(&list), "origin/main")?;
        assert_eq!(changed, vec!["src/lib.rs".to_string(), "crates/foo/src/bar.rs".to_string()]);
        Ok(())
    }

    // --- load_config / load_policy: malformed config is an instrument failure ---

    #[test]
    fn load_config_missing_file_is_err() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(load_config(tmp.path()).is_err());
        Ok(())
    }

    #[test]
    fn load_policy_missing_file_is_err() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(load_policy(tmp.path()).is_err());
        Ok(())
    }

    #[test]
    fn load_config_malformed_yaml_is_err() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        std::fs::write(tmp.path().join(CHANGIE_CONFIG), "projects: [this is not valid: yaml: :")
            .map_err(|e| e.to_string())?;
        assert!(load_config(tmp.path()).is_err());
        Ok(())
    }

    // --- check(): end-to-end exit-path coverage (mutation-check style) ---

    fn write_policy(dir: &Path) -> std::result::Result<(), String> {
        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join(POLICY_FILE),
            r#"
schema_version = 1
enforcement = "advisory"
retrospective_covered_through = "da86c123a"
advisory_expected_from = ""
blocking_enforced_from = ""
exemption_categories = ["tests", "ci", "refactor", "generated-status", "docs-no-contract-change", "deps", "release-prep", "changelog-tooling"]

[[projects]]
key = "product"
label = "Product"
changelog = "CHANGELOG.md"

[[projects]]
key = "vscode"
label = "VS Code extension"
changelog = "vscode-extension/CHANGELOG.md"
"#,
        )
        .map_err(|e| e.to_string())
    }

    fn write_valid_changie(dir: &Path) -> std::result::Result<(), String> {
        std::fs::write(
            dir.join(CHANGIE_CONFIG),
            r#"
projects:
  - key: product
  - key: vscode
components:
  - Developer experience
kinds:
  - label: Added
body:
  minLength: 5
"#,
        )
        .map_err(|e| e.to_string())
    }

    fn write_changed_files(dir: &Path, files: &[&str]) -> std::result::Result<PathBuf, String> {
        let list = dir.join("changed-files.txt");
        std::fs::write(&list, files.join("\n")).map_err(|e| e.to_string())?;
        Ok(list)
    }

    /// Regression test for a real bug this PR's Correction 2 surfaced: TOML
    /// scopes bare `key = value` lines to the most recently opened table, so
    /// `exemption_categories` declared AFTER `[[projects]]` silently nests
    /// under the last project entry instead of the document root. The old
    /// ad-hoc `toml::Value` lookups never touched `exemption_categories` (it
    /// was a hardcoded Rust const), so this was invisible until
    /// `ChangelogPolicy` started deserializing the whole file. Pins both the
    /// real repo file AND the fix (field must precede any `[[projects]]`).
    #[test]
    fn real_policy_toml_deserializes_with_all_fields() -> std::result::Result<(), String> {
        let root = crate::utils::project_root().map_err(|e| e.to_string())?;
        let policy = load_policy(&root).map_err(|e| e.to_string())?;
        assert!(
            !policy.exemption_categories.is_empty(),
            "exemption_categories must deserialize at the document root, not nest under the \
             last [[projects]] table"
        );
        assert_eq!(policy.projects.len(), 2, "expected product + vscode projects");
        assert!(policy.changelog_paths().contains(&"CHANGELOG.md"));
        assert!(policy.changelog_paths().contains(&"vscode-extension/CHANGELOG.md"));
        Ok(())
    }

    #[test]
    fn check_malformed_changie_yaml_is_instrument_failure_exit2() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir)?;
        std::fs::write(dir.join(CHANGIE_CONFIG), "not: [valid, yaml: :")
            .map_err(|e| e.to_string())?;
        let result = check(None, None, None, false, Some(dir.to_path_buf()));
        assert!(result.is_err(), "malformed .changie.yaml must be an instrument failure (exit 2)");
        Ok(())
    }

    #[test]
    fn check_malformed_policy_toml_is_instrument_failure_exit2() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(POLICY_FILE), "this = [is not : valid toml")
            .map_err(|e| e.to_string())?;
        write_valid_changie(dir)?;
        let result = check(None, None, None, false, Some(dir.to_path_buf()));
        assert!(
            result.is_err(),
            "malformed policy/changelog.toml must be an instrument failure (exit 2)"
        );
        Ok(())
    }

    #[test]
    fn check_unresolvable_changed_files_is_instrument_failure_exit2()
    -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir)?;
        write_valid_changie(dir)?;
        let missing = dir.join("does-not-exist.txt");
        let result = check(None, Some(missing), None, false, Some(dir.to_path_buf()));
        assert!(
            result.is_err(),
            "an unresolvable --changed-files path must be an instrument failure (exit 2)"
        );
        Ok(())
    }

    #[test]
    fn check_missing_disposition_not_armed_is_policy_satisfied_exit0()
    -> std::result::Result<(), String> {
        // advisory_expected_from is empty in write_policy() => NotArmed.
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir)?;
        write_valid_changie(dir)?;
        let list = write_changed_files(dir, &["crates/perl-parser/src/lib.rs"])?;
        let outcome = check(None, Some(list), None, false, Some(dir.to_path_buf()))
            .map_err(|e| e.to_string())?;
        assert_eq!(outcome, CheckOutcome::PolicySatisfied);
        Ok(())
    }

    #[test]
    fn check_missing_disposition_during_soak_is_advisory_finding_exit0()
    -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        // Make this dir a tiny git repo with one commit, then set
        // advisory_expected_from to that commit and diff base to HEAD, so the
        // ancestry check resolves true (armed, but not blocking).
        run_git(dir, &["init", "-q"])?;
        run_git(dir, &["config", "user.email", "test@example.com"])?;
        run_git(dir, &["config", "user.name", "test"])?;
        std::fs::write(dir.join("seed.txt"), "seed").map_err(|e| e.to_string())?;
        run_git(dir, &["add", "seed.txt"])?;
        run_git(dir, &["commit", "-q", "-m", "seed"])?;
        let sha = run_git_output(dir, &["rev-parse", "HEAD"])?;

        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join(POLICY_FILE),
            format!(
                r#"
schema_version = 1
enforcement = "advisory"
retrospective_covered_through = "da86c123a"
advisory_expected_from = "{sha}"
blocking_enforced_from = ""
exemption_categories = ["tests", "ci", "refactor", "generated-status", "docs-no-contract-change", "deps", "release-prep", "changelog-tooling"]

[[projects]]
key = "product"
label = "Product"
changelog = "CHANGELOG.md"

[[projects]]
key = "vscode"
label = "VS Code extension"
changelog = "vscode-extension/CHANGELOG.md"
"#
            ),
        )
        .map_err(|e| e.to_string())?;
        write_valid_changie(dir)?;
        let list = write_changed_files(dir, &["crates/perl-parser/src/lib.rs"])?;
        let outcome =
            check(Some("HEAD".to_string()), Some(list), None, false, Some(dir.to_path_buf()))
                .map_err(|e| e.to_string())?;
        assert_eq!(
            outcome,
            CheckOutcome::AdvisoryFinding,
            "missing disposition during the armed advisory soak must be an AdvisoryFinding, exit 0"
        );
        Ok(())
    }

    #[test]
    fn check_missing_disposition_after_blocking_boundary_is_blocking_violation_exit1()
    -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        run_git(dir, &["init", "-q"])?;
        run_git(dir, &["config", "user.email", "test@example.com"])?;
        run_git(dir, &["config", "user.name", "test"])?;
        std::fs::write(dir.join("seed.txt"), "seed").map_err(|e| e.to_string())?;
        run_git(dir, &["add", "seed.txt"])?;
        run_git(dir, &["commit", "-q", "-m", "seed"])?;
        let sha = run_git_output(dir, &["rev-parse", "HEAD"])?;

        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join(POLICY_FILE),
            format!(
                r#"
schema_version = 1
enforcement = "blocking"
retrospective_covered_through = "da86c123a"
advisory_expected_from = "{sha}"
blocking_enforced_from = "{sha}"
exemption_categories = ["tests", "ci", "refactor", "generated-status", "docs-no-contract-change", "deps", "release-prep", "changelog-tooling"]

[[projects]]
key = "product"
label = "Product"
changelog = "CHANGELOG.md"

[[projects]]
key = "vscode"
label = "VS Code extension"
changelog = "vscode-extension/CHANGELOG.md"
"#
            ),
        )
        .map_err(|e| e.to_string())?;
        write_valid_changie(dir)?;
        let list = write_changed_files(dir, &["crates/perl-parser/src/lib.rs"])?;
        let outcome =
            check(Some("HEAD".to_string()), Some(list), None, false, Some(dir.to_path_buf()))
                .map_err(|e| e.to_string())?;
        assert_eq!(
            outcome,
            CheckOutcome::BlockingViolation,
            "missing disposition past a reached blocking boundary must be a BlockingViolation, exit 1"
        );
        Ok(())
    }

    #[test]
    fn check_fragment_disposition_is_policy_satisfied_exit0() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir)?;
        write_valid_changie(dir)?;
        let unreleased = dir.join(".changes/unreleased");
        std::fs::create_dir_all(&unreleased).map_err(|e| e.to_string())?;
        std::fs::write(
            unreleased.join("product-1-Added-101010.yaml"),
            "project: product\ncomponent: Developer experience\nkind: Added\nbody: A long enough body.\ncustom:\n  PR: \"1\"\n",
        )
        .map_err(|e| e.to_string())?;
        let list = write_changed_files(dir, &[".changes/unreleased/product-1-Added-101010.yaml"])?;
        let outcome = check(None, Some(list), None, false, Some(dir.to_path_buf()))
            .map_err(|e| e.to_string())?;
        assert_eq!(outcome, CheckOutcome::PolicySatisfied);
        Ok(())
    }

    fn run_git(dir: &Path, args: &[&str]) -> std::result::Result<(), String> {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .map_err(|e| format!("failed to spawn git {args:?}: {e}"))?;
        if !status.success() {
            return Err(format!("git {args:?} failed with status {status:?}"));
        }
        Ok(())
    }

    fn run_git_output(dir: &Path, args: &[&str]) -> std::result::Result<String, String> {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|e| format!("failed to spawn git {args:?}: {e}"))?;
        if !out.status.success() {
            return Err(format!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}
