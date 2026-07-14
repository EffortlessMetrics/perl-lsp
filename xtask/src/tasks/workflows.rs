//! Workflow Contracts checks — actionlint + zizmor + native contract checks.
//!
//! FOUNDATION / ADVISORY-UNARMED (tracking issue #3788; parent #3785). This
//! task validates `.github/workflows/*.yml` against an external-tool +
//! native contract: actionlint (syntax/expression/shellcheck), zizmor
//! (security audit), local action/reusable-workflow ref integrity, explicit
//! `permissions:`, and an action-pinning policy. It mirrors the Changie
//! advisory-first rollout shape exactly (`xtask/src/tasks/changelog.rs`,
//! `policy/changelog.toml`) — same three-clock model, same
//! instrument-vs-verdict exit-code split.
//!
//! ## Exit codes (`cargo xtask workflows check`)
//!
//! - **0** — policy satisfied, OR an advisory finding was reported (a
//!   contract violation during the soak window between
//!   `advisory_expected_from` and `blocking_enforced_from`). Both are
//!   non-fatal.
//! - **1** — a *blocking* policy violation. Only reachable once
//!   `policy/workflow-contracts.toml`'s `blocking_enforced_from` is set AND
//!   the PR's base is at/after that commit. With `advisory_expected_from`
//!   AND `blocking_enforced_from` both empty (the state shipped by this PR),
//!   this path is unreachable — this scaffold is advisory-*unarmed*, one
//!   step earlier than the Changie gate was at its own PR1 (which armed
//!   `advisory_expected_from` from day one). Arming is deferred to a
//!   follow-up PR that first enumerates and either fixes or allowlists the
//!   ~160-finding pre-existing pinning baseline (see
//!   `policy/workflow-contracts.toml`'s comment block) — arming today would
//!   report that entire pre-existing baseline as "new".
//! - **2** — an instrument/config failure: `policy/workflow-contracts.toml`
//!   fails to parse, `actionlint`/`zizmor` are missing from `PATH` or crash
//!   (non-JSON output) during a real (non-`--self-test`) run, or the
//!   `.github/workflows` directory cannot be read. These are tooling
//!   problems, not policy findings, and are never silently downgraded to a
//!   passing exit. A workflow YAML file that fails to parse is, by
//!   contrast, a POLICY finding scoped to that one file (mirrors
//!   `changelog.rs::check_fragment_file`'s treatment of a malformed
//!   fragment) — it does not abort the run.
//!
//! ## Boundary (from #3785 / #3788)
//!
//! This checker validates workflow-FILE contracts. It does NOT prove
//! repo-specific merge semantics (final-head / non-draft / required-path /
//! executed-not-skipped) — that stays with `xtask/src/tasks/merge_ready.rs`.
//! It does not replace `workflow_policy_lint.rs` / `workflow_trigger_lint.rs`
//! / `ci_audit_workflows.rs` (repo-specific policy-shape lints); it is the
//! external-tool contract layer plus a stricter pinning/permissions/local-ref
//! contract. The SHA-pin primitive (`is_sha_pinned`) is shared with
//! `workflow_policy_lint.rs` rather than re-implemented, so the two checkers
//! agree on what "pinned" means.
//!
//! ## Single policy source
//!
//! `policy/workflow-contracts.toml` is deserialized into
//! [`WorkflowContractsPolicy`] and is the *only* place enforcement mode, the
//! three-clock boundaries, the first-party pinning allowlist, and the
//! runner-label allowlists are declared — nothing here hand-duplicates it.

use crate::tasks::workflow_policy_lint::is_sha_pinned;
use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value as YamlValue;
use std::path::{Path, PathBuf};
use std::process::Command;

const POLICY_FILE: &str = "policy/workflow-contracts.toml";
const WORKFLOWS_DIR: &str = ".github/workflows";

/// The sole authority for workflow-contracts policy:
/// `policy/workflow-contracts.toml`, deserialized. Mirrors
/// [`crate::tasks::changelog`]'s `ChangelogPolicy` shape and the three-clock
/// model it documents.
#[derive(Debug, Deserialize)]
struct WorkflowContractsPolicy {
    #[allow(dead_code)] // reserved for future schema-migration checks
    schema_version: u32,
    enforcement: Enforcement,
    #[serde(default)]
    advisory_expected_from: Option<String>,
    #[serde(default)]
    blocking_enforced_from: Option<String>,
    #[serde(default)]
    first_party_pin_allowlist: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)] // reserved for the actionlint runner-label config, PR2
    self_hosted_runner_labels: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)] // reserved for the actionlint matrix.os handling, PR2
    matrix_os_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Enforcement {
    Advisory,
    Blocking,
}

/// The overall policy verdict for a `check()` run. Distinct from instrument
/// failure (an `Err` from `check()`): every variant here means the
/// instrument worked and produced a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    /// No findings, OR findings were computed but the advisory boundary
    /// isn't armed yet for this run.
    PolicySatisfied,
    /// One or more contract findings were reported (advisory boundary
    /// armed). Still exits 0.
    AdvisoryFinding,
    /// One or more contract findings remain unresolved past the blocking
    /// boundary. The only outcome that should map to a non-zero (1) exit.
    BlockingViolation,
}

/// Where a run sits relative to the three-clock policy boundaries. Mirrors
/// `changelog.rs::Boundary` exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Boundary {
    NotArmed,
    Advisory,
    Blocking,
}

fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.is_empty())
}

/// Is `sha` an ancestor of `base`? `None` means inconclusive (e.g. `sha`
/// unresolvable in a shallow clone) — degrades to "not confirmed", never
/// escalates a boundary. Mirrors `changelog.rs::is_ancestor`.
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

fn boundary_state(root: &Path, policy: &WorkflowContractsPolicy, base: &str) -> Boundary {
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

fn load_policy(root: &Path) -> Result<WorkflowContractsPolicy> {
    let path = root.join(POLICY_FILE);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| eyre!("failed to parse {}: {e}", path.display()))
}

/// Accumulates findings for a single run. Mirrors `changelog.rs::Report`.
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

/// Every `uses:` reference across every job/step in a workflow document.
///
/// Two distinct forms: a JOB-level `jobs.<id>.uses:` invokes a reusable
/// workflow (no `steps:` on that job at all); a STEP-level
/// `jobs.<id>.steps[].uses:` invokes an action. Both must be collected — a
/// job that only calls a reusable workflow has no `steps` key, so gating the
/// job-level check behind a successful `steps` lookup (the original bug)
/// silently drops every reusable-workflow reference from both the
/// local-ref-integrity and pinning-policy checks below.
fn all_uses_refs(workflow: &YamlValue) -> Vec<String> {
    let mut out = Vec::new();
    let Some(jobs) = workflow.get("jobs").and_then(YamlValue::as_mapping) else {
        return out;
    };
    for job in jobs.values() {
        let Some(job_map) = job.as_mapping() else { continue };

        // Job-level reusable-workflow reference.
        if let Some(uses) =
            job_map.get(YamlValue::String("uses".to_string())).and_then(YamlValue::as_str)
        {
            out.push(uses.to_string());
        }

        // Step-level action references.
        if let Some(steps) =
            job_map.get(YamlValue::String("steps".to_string())).and_then(YamlValue::as_sequence)
        {
            for step in steps {
                if let Some(uses) = step
                    .as_mapping()
                    .and_then(|m| m.get(YamlValue::String("uses".to_string())))
                    .and_then(YamlValue::as_str)
                {
                    out.push(uses.to_string());
                }
            }
        }
    }
    out
}

/// Local action/reusable-workflow refs (`uses: ./...`) that don't resolve to
/// a file in-repo. Not covered by actionlint, which only validates the
/// *shape* of `uses:`, not whether a local path actually exists.
fn find_local_ref_findings(root: &Path, workflow: &YamlValue) -> Vec<String> {
    let mut findings = Vec::new();
    for uses in all_uses_refs(workflow) {
        let Some(rel) = uses.strip_prefix("./") else { continue };
        // A local reusable-workflow ref carries an action-version suffix
        // (`@main`/`@<sha>`) that isn't part of the filesystem path.
        let rel_path = rel.split('@').next().unwrap_or(rel);
        if !root.join(rel_path).exists() {
            findings.push(format!("local ref does not exist: `{uses}` (resolved: {rel_path})"));
        }
    }
    findings
}

/// `uses:` refs that are neither SHA-pinned nor covered by the policy's
/// first-party allowlist (prefix match, e.g. `"actions/"` covers
/// `actions/checkout`).
fn find_pinning_findings(workflow: &YamlValue, allowlist: &[String]) -> Vec<String> {
    let mut findings = Vec::new();
    for uses in all_uses_refs(workflow) {
        if uses.starts_with("./") || uses.starts_with("docker://") {
            continue;
        }
        if allowlist.iter().any(|prefix| uses.starts_with(prefix.as_str())) {
            continue;
        }
        if !is_sha_pinned(&uses) {
            findings.push(format!("not SHA-pinned (and not first-party-allowlisted): `{uses}`"));
        }
    }
    findings
}

/// Does this workflow declare `permissions:` at the top level, or on every
/// job? A workflow with no jobs is vacuously "not covered" (nothing to
/// scope), reported as a finding rather than silently passing.
fn has_explicit_permissions(workflow: &YamlValue) -> bool {
    if workflow.get("permissions").is_some() {
        return true;
    }
    let Some(jobs) = workflow.get("jobs").and_then(YamlValue::as_mapping) else {
        return false;
    };
    if jobs.is_empty() {
        return false;
    }
    jobs.values().all(|job| {
        job.as_mapping()
            .is_some_and(|m| m.get(YamlValue::String("permissions".to_string())).is_some())
    })
}

fn collect_workflow_files(dir: &Path) -> std::result::Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read workflows dir {}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .collect();
    files.sort();
    Ok(files)
}

/// Run every native (non-external-tool) check across all workflow files.
/// Returns `true` if any finding was reported. A per-file read/parse failure
/// is a finding scoped to that file, not an instrument failure — mirrors
/// `changelog.rs::check_fragment_file`'s treatment of a malformed fragment.
fn check_native(root: &Path, files: &[PathBuf], report: &mut Report) -> bool {
    let mut has_finding = false;
    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.warn(format!("{rel}: could not read: {e}"));
                has_finding = true;
                continue;
            }
        };
        let workflow: YamlValue = match serde_yaml_ng::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                report.warn(format!("{rel}: does not parse as YAML: {e}"));
                has_finding = true;
                continue;
            }
        };

        if !has_explicit_permissions(&workflow) {
            report.warn(format!(
                "{rel}: no explicit `permissions:` block at top level or on every job"
            ));
            has_finding = true;
        }

        for f in find_local_ref_findings(root, &workflow) {
            report.warn(format!("{rel}: {f}"));
            has_finding = true;
        }
    }
    has_finding
}

/// Is `bin` on `PATH` and does `--version`/`-version` succeed?
fn tool_available(bin: &str, version_flag: &str) -> bool {
    Command::new(bin).arg(version_flag).output().map(|o| o.status.success()).unwrap_or(false)
}

/// actionlint's documented exit codes (verified against its own usage docs,
/// "Exit Status Codes" section): 0 = no findings, 1 = findings present
/// (a NORMAL outcome, not a crash), 2 = invalid command-line option, 3 =
/// fatal error. Only 2/3 (and any other undocumented code) are instrument
/// failures.
fn actionlint_exit_is_clean(code: Option<i32>) -> bool {
    matches!(code, Some(0) | Some(1))
}

/// zizmor's documented exit codes (verified against its own usage docs,
/// "Exit codes" section): 0 = no findings, 1 = error during audit, 2 =
/// argument-parsing failure, 3 = no inputs collected (1/2/3 are instrument
/// failures), 11-14 = findings present at increasing severity (a NORMAL
/// outcome, not a crash — only reachable without `--no-exit-codes`/SARIF).
fn zizmor_exit_is_clean(code: Option<i32>) -> bool {
    matches!(code, Some(0) | Some(11) | Some(12) | Some(13) | Some(14))
}

/// Run `actionlint -format '{{json .}}'` over the workflows directory.
///
/// `Err` means the instrument itself failed: an unspawnable binary, an exit
/// code outside actionlint's documented "ran successfully" set
/// ([`actionlint_exit_is_clean`] — this catches a crash whose diagnostics
/// went only to stderr with empty/garbled stdout, which would otherwise be
/// silently accepted as "0 findings"), or output that isn't valid JSON.
/// actionlint's exit 1 (findings present) is expected and is not an error
/// condition here.
///
/// The exact JSON field names actionlint emits per finding are not pinned
/// down here (verified from actionlint's own docs, which show the template
/// accessors — `Message`/`Filepath`/`Line`/`Column`/`EndColumn`/`Kind` — but
/// not a complete raw JSON sample). Findings are therefore kept as raw
/// [`serde_json::Value`]s and summarized defensively
/// ([`summarize_json_finding`]) rather than deserialized into a strict
/// struct, so a field-name mismatch degrades to a less-pretty summary line
/// instead of an instrument failure. Tighten this once the first real CI run
/// confirms the exact schema.
fn run_actionlint(root: &Path) -> std::result::Result<Vec<serde_json::Value>, String> {
    let out = Command::new("actionlint")
        .current_dir(root)
        .args(["-format", "{{json .}}"])
        .output()
        .map_err(|e| format!("failed to spawn actionlint: {e}"))?;
    if !actionlint_exit_is_clean(out.status.code()) {
        return Err(format!(
            "actionlint exited {:?} (outside its documented 0=clean/1=findings set) — \
             stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_json_array_output(&out.stdout, "actionlint")
}

/// Run `zizmor --format json .github/workflows` over the workflows
/// directory. Same instrument-vs-finding split as [`run_actionlint`]
/// (via [`zizmor_exit_is_clean`]); same defensive raw-`Value` parsing for
/// the same reason (schema not pinned down from docs alone).
fn run_zizmor(root: &Path) -> std::result::Result<Vec<serde_json::Value>, String> {
    let out = Command::new("zizmor")
        .current_dir(root)
        .args(["--format", "json", WORKFLOWS_DIR])
        .output()
        .map_err(|e| format!("failed to spawn zizmor: {e}"))?;
    if !zizmor_exit_is_clean(out.status.code()) {
        return Err(format!(
            "zizmor exited {:?} (outside its documented 0=clean/11-14=findings set) — \
             stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_json_array_output(&out.stdout, "zizmor")
}

fn parse_json_array_output(
    stdout: &[u8],
    tool: &str,
) -> std::result::Result<Vec<serde_json::Value>, String> {
    let text = String::from_utf8_lossy(stdout);
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<serde_json::Value>>(&text)
        .map_err(|e| format!("{tool} produced output that isn't a JSON array: {e}"))
}

/// Best-effort human summary of one raw finding, trying a few plausible key
/// names (see [`run_actionlint`] for why this isn't a strict struct).
fn summarize_json_finding(finding: &serde_json::Value) -> String {
    let msg = finding
        .get("message")
        .or_else(|| finding.get("desc"))
        .and_then(|v| v.as_str())
        .unwrap_or("(no message field found)");
    let file = finding
        .get("filepath")
        .and_then(|v| v.as_str())
        .or_else(|| finding.get("locations").and_then(|l| l.get(0)).and_then(|v| v.as_str()));
    match file {
        Some(f) => format!("{f}: {msg}"),
        None => msg.to_string(),
    }
}

fn report_policy(policy: &WorkflowContractsPolicy) {
    println!(
        "  policy: enforcement={}",
        match policy.enforcement {
            Enforcement::Advisory => "advisory",
            Enforcement::Blocking => "blocking",
        }
    );
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

#[derive(Debug, Serialize)]
struct ReceiptFinding {
    level: &'static str,
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
struct WorkflowContractsReceipt {
    schema_version: &'static str,
    receipt_kind: &'static str,
    passed: bool,
    finding_count: usize,
    findings: Vec<ReceiptFinding>,
}

fn write_receipt(
    path: &Path,
    findings: &[String],
    passed: bool,
) -> std::result::Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }
    let receipt = WorkflowContractsReceipt {
        schema_version: "1",
        receipt_kind: "workflow_contracts",
        passed,
        finding_count: findings.len(),
        findings: findings
            .iter()
            .map(|m| ReceiptFinding {
                level: "warning",
                code: "workflow-contract",
                message: m.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
    std::fs::write(path, format!("{json}\n"))
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

/// Entry point for `cargo xtask workflows check`.
///
/// Returns `Err` only for instrument/config failures (exit 2 at the CLI
/// layer); every reachable policy verdict — satisfied, advisory finding, or
/// blocking violation — is an `Ok(CheckOutcome)`. See the module docs for
/// the full exit-code contract.
pub fn check(
    base: Option<String>,
    self_test: bool,
    receipt: Option<PathBuf>,
    root: Option<PathBuf>,
) -> Result<CheckOutcome> {
    let root = match root {
        Some(r) => r,
        None => crate::utils::project_root()?,
    };

    println!("workflow-contracts check (tracking issue #3788; parent #3785)");

    let policy = load_policy(&root)?;
    report_policy(&policy);

    let mut report = Report::default();
    let mut all_findings: Vec<String> = Vec::new();

    let workflows_dir = root.join(WORKFLOWS_DIR);
    let files = collect_workflow_files(&workflows_dir).map_err(|e| eyre!(e))?;

    // Native checks: permissions + local-ref integrity (schema/parse errors
    // scoped per-file, not an instrument failure).
    let native_finding = check_native(&root, &files, &mut report);
    if native_finding {
        all_findings.push("native contract check(s) reported a finding above".to_string());
    }

    // Pinning check needs the policy's allowlist, so it runs here rather
    // than inside `check_native` (which has no policy in scope).
    for path in &files {
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        let Ok(workflow) = serde_yaml_ng::from_str::<YamlValue>(&content) else { continue };
        for f in find_pinning_findings(&workflow, &policy.first_party_pin_allowlist) {
            report.warn(format!("{rel}: {f}"));
            all_findings.push(format!("{rel}: {f}"));
        }
    }

    report.ok(format!("checked {} workflow file(s) natively", files.len()));

    if self_test {
        // Self-test validates policy parsing + the native checks against the
        // real tree; external-tool findings are best-effort (skip
        // gracefully if the binaries aren't installed locally — mirrors
        // `changelog.rs::changie_available()`'s local-dev degrade). CI's
        // workflow installs both tools before invoking this command, so a
        // real (non-self-test) run always has them.
        run_external_tool_best_effort("actionlint", run_actionlint, &root, &mut report);
        run_external_tool_best_effort("zizmor", run_zizmor, &root, &mut report);
        report.emit();
        return Ok(CheckOutcome::PolicySatisfied);
    }

    if !tool_available("actionlint", "-version") {
        return Err(eyre!("actionlint instrument failure: not found on PATH"));
    }
    match run_actionlint(&root) {
        Ok(findings) => {
            report.info(format!("actionlint: {} finding(s)", findings.len()));
            for f in &findings {
                let summary = summarize_json_finding(f);
                report.warn(format!("actionlint: {summary}"));
                all_findings.push(format!("actionlint: {summary}"));
            }
        }
        Err(e) => return Err(eyre!("actionlint instrument failure: {e}")),
    }

    if !tool_available("zizmor", "--version") {
        return Err(eyre!("zizmor instrument failure: not found on PATH"));
    }
    match run_zizmor(&root) {
        Ok(findings) => {
            report.info(format!("zizmor: {} finding(s)", findings.len()));
            for f in &findings {
                let summary = summarize_json_finding(f);
                report.warn(format!("zizmor: {summary}"));
                all_findings.push(format!("zizmor: {summary}"));
            }
        }
        Err(e) => return Err(eyre!("zizmor instrument failure: {e}")),
    }

    let has_finding = !all_findings.is_empty();
    let outcome = if !has_finding {
        CheckOutcome::PolicySatisfied
    } else {
        let base = base.unwrap_or_else(|| "origin/main".to_string());
        match boundary_state(&root, &policy, &base) {
            Boundary::NotArmed => {
                report.info(
                    "finding(s) reported above, but the advisory boundary \
                     (`advisory_expected_from`) is not yet armed for this run's base — not \
                     counted as a policy outcome.",
                );
                CheckOutcome::PolicySatisfied
            }
            Boundary::Advisory => CheckOutcome::AdvisoryFinding,
            Boundary::Blocking => CheckOutcome::BlockingViolation,
        }
    };

    if let Some(path) = receipt {
        if let Err(e) = write_receipt(&path, &all_findings, !has_finding) {
            report.warn(format!("could not write receipt {}: {e}", path.display()));
        } else {
            report.info(format!("receipt written: {}", path.display()));
        }
    }

    report.emit();
    Ok(outcome)
}

/// Run an external tool and fold its findings into `report`, degrading to an
/// INFO skip (not a failure) when the binary isn't on `PATH` — the
/// `--self-test` local-dev ergonomics path. A real run never calls this;
/// see `check()`'s non-self-test branch for the hard instrument-failure
/// behavior.
fn run_external_tool_best_effort(
    name: &str,
    runner: fn(&Path) -> std::result::Result<Vec<serde_json::Value>, String>,
    root: &Path,
    report: &mut Report,
) {
    if !tool_available(name, if name == "actionlint" { "-version" } else { "--version" }) {
        report.info(format!("{name} not on PATH; skipping (advisory, self-test)"));
        return;
    }
    match runner(root) {
        Ok(findings) => {
            report.ok(format!("{name}: ran successfully, {} finding(s)", findings.len()))
        }
        Err(e) => report.warn(format!("{name}: ran but produced unexpected output: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy(
        advisory_expected_from: &str,
        blocking_enforced_from: &str,
    ) -> WorkflowContractsPolicy {
        WorkflowContractsPolicy {
            schema_version: 1,
            enforcement: Enforcement::Advisory,
            advisory_expected_from: non_empty_owned(advisory_expected_from),
            blocking_enforced_from: non_empty_owned(blocking_enforced_from),
            first_party_pin_allowlist: vec!["actions/".to_string(), "github/".to_string()],
            self_hosted_runner_labels: vec![],
            matrix_os_allowlist: vec![],
        }
    }

    fn non_empty_owned(s: &str) -> Option<String> {
        if s.is_empty() { None } else { Some(s.to_string()) }
    }

    fn parse_workflow(yaml: &str) -> YamlValue {
        serde_yaml_ng::from_str(yaml).expect("test fixture must parse")
    }

    // --- has_explicit_permissions ---

    #[test]
    fn top_level_permissions_satisfies() {
        let wf = parse_workflow("permissions:\n  contents: read\njobs:\n  a:\n    steps: []\n");
        assert!(has_explicit_permissions(&wf));
    }

    #[test]
    fn per_job_permissions_satisfies() {
        let wf = parse_workflow(
            "jobs:\n  a:\n    permissions:\n      contents: read\n    steps: []\n  b:\n    permissions:\n      contents: read\n    steps: []\n",
        );
        assert!(has_explicit_permissions(&wf));
    }

    #[test]
    fn missing_permissions_is_flagged() {
        let wf = parse_workflow("jobs:\n  a:\n    steps: []\n");
        assert!(!has_explicit_permissions(&wf));
    }

    #[test]
    fn partial_per_job_permissions_is_flagged() {
        let wf = parse_workflow(
            "jobs:\n  a:\n    permissions:\n      contents: read\n    steps: []\n  b:\n    steps: []\n",
        );
        assert!(!has_explicit_permissions(&wf));
    }

    #[test]
    fn no_jobs_is_flagged() {
        let wf = parse_workflow("on: push\n");
        assert!(!has_explicit_permissions(&wf));
    }

    // --- find_pinning_findings ---

    #[test]
    fn sha_pinned_third_party_is_clean() {
        let wf = parse_workflow(
            "jobs:\n  a:\n    steps:\n      - uses: rhysd/actionlint@aa1e0e28c3e42d99f7d78e39c7cb52ed03d1f890\n",
        );
        let findings = find_pinning_findings(&wf, &["actions/".to_string()]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn tag_pinned_third_party_is_flagged() {
        let wf = parse_workflow("jobs:\n  a:\n    steps:\n      - uses: some-org/some-action@v1\n");
        let findings = find_pinning_findings(&wf, &["actions/".to_string()]);
        assert!(findings.iter().any(|f| f.contains("some-org/some-action")), "{findings:?}");
    }

    #[test]
    fn tag_pinned_first_party_allowlisted_is_clean() {
        let wf = parse_workflow("jobs:\n  a:\n    steps:\n      - uses: actions/checkout@v7\n");
        let findings = find_pinning_findings(&wf, &["actions/".to_string()]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn local_ref_is_never_a_pinning_finding() {
        let wf = parse_workflow("jobs:\n  a:\n    steps:\n      - uses: ./.github/actions/local\n");
        let findings = find_pinning_findings(&wf, &[]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    // --- find_local_ref_findings ---

    #[test]
    fn existing_local_ref_is_clean() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(tmp.path().join(".github/actions/local"))
            .map_err(|e| e.to_string())?;
        std::fs::write(tmp.path().join(".github/actions/local/action.yml"), "name: x")
            .map_err(|e| e.to_string())?;
        let wf = parse_workflow("jobs:\n  a:\n    steps:\n      - uses: ./.github/actions/local\n");
        let findings = find_local_ref_findings(tmp.path(), &wf);
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn missing_local_ref_is_flagged() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let wf = parse_workflow(
            "jobs:\n  a:\n    steps:\n      - uses: ./.github/actions/does-not-exist\n",
        );
        let findings = find_local_ref_findings(tmp.path(), &wf);
        assert!(findings.iter().any(|f| f.contains("does-not-exist")), "{findings:?}");
        Ok(())
    }

    #[test]
    fn local_reusable_workflow_ref_strips_version_suffix() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(tmp.path().join(".github/workflows")).map_err(|e| e.to_string())?;
        std::fs::write(tmp.path().join(".github/workflows/reusable.yml"), "on: workflow_call")
            .map_err(|e| e.to_string())?;
        let wf = parse_workflow("jobs:\n  a:\n    uses: ./.github/workflows/reusable.yml@main\n");
        let findings = find_local_ref_findings(tmp.path(), &wf);
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    /// Discriminates the positive case above: this fixture deliberately does
    /// NOT create `reusable.yml`, so a job-level (no `steps`) local reusable-
    /// workflow ref must still be checked for existence and flagged. Without
    /// `all_uses_refs` walking `jobs.<id>.uses`, this assertion would be
    /// vacuously satisfied by an empty findings list -- pinning that bug is
    /// the whole point of this test (see #3885 factory-droid P1).
    #[test]
    fn missing_job_level_local_reusable_workflow_ref_is_flagged() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let wf = parse_workflow("jobs:\n  a:\n    uses: ./.github/workflows/reusable.yml@main\n");
        let findings = find_local_ref_findings(tmp.path(), &wf);
        assert!(
            findings.iter().any(|f| f.contains("reusable.yml")),
            "job-level local reusable-workflow ref must be checked for existence, got {findings:?}"
        );
        Ok(())
    }

    /// Job-level (no `steps`) remote reusable-workflow refs must also be seen
    /// by the pinning check, not just step-level action refs (#3885 P2).
    #[test]
    fn job_level_remote_reusable_workflow_ref_is_checked_for_pinning() {
        let wf = parse_workflow(
            "jobs:\n  a:\n    uses: some-org/some-repo/.github/workflows/x.yml@v1\n",
        );
        let findings = find_pinning_findings(&wf, &["actions/".to_string()]);
        assert!(
            findings.iter().any(|f| f.contains("some-org/some-repo")),
            "job-level remote reusable-workflow ref must be pinning-checked, got {findings:?}"
        );
    }

    // --- boundary_state: three-clock cutoff coverage (mirrors changelog.rs) ---

    #[test]
    fn boundary_not_armed_when_advisory_expected_from_empty() {
        let policy = test_policy("", "");
        let boundary = boundary_state(Path::new("."), &policy, "HEAD");
        assert_eq!(boundary, Boundary::NotArmed);
    }

    #[test]
    fn boundary_not_armed_when_ancestry_unresolvable() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let policy = test_policy("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef", "");
        let boundary = boundary_state(tmp.path(), &policy, "HEAD");
        assert_eq!(boundary, Boundary::NotArmed);
        Ok(())
    }

    #[test]
    fn boundary_never_blocking_when_blocking_enforced_from_empty() {
        let mut policy = test_policy("", "");
        policy.enforcement = Enforcement::Blocking;
        policy.blocking_enforced_from = None;
        let boundary = boundary_state(Path::new("."), &policy, "HEAD");
        assert_ne!(boundary, Boundary::Blocking);
    }

    // --- exit-status gates: instrument-failure unit coverage (#3885 P2, no process spawn) ---
    // Codes verified against each tool's own "Exit Status"/"Exit codes" docs.

    #[test]
    fn actionlint_exit_0_is_clean() {
        assert!(actionlint_exit_is_clean(Some(0)));
    }

    #[test]
    fn actionlint_exit_1_findings_is_clean() {
        // Findings present is a NORMAL outcome, not a crash.
        assert!(actionlint_exit_is_clean(Some(1)));
    }

    #[test]
    fn actionlint_exit_2_invalid_option_is_not_clean() {
        assert!(!actionlint_exit_is_clean(Some(2)));
    }

    #[test]
    fn actionlint_exit_3_fatal_is_not_clean() {
        assert!(!actionlint_exit_is_clean(Some(3)));
    }

    #[test]
    fn actionlint_missing_exit_code_is_not_clean() {
        // e.g. killed by a signal on Unix.
        assert!(!actionlint_exit_is_clean(None));
    }

    #[test]
    fn zizmor_exit_0_is_clean() {
        assert!(zizmor_exit_is_clean(Some(0)));
    }

    #[test]
    fn zizmor_exit_11_through_14_findings_are_clean() {
        for code in [11, 12, 13, 14] {
            assert!(zizmor_exit_is_clean(Some(code)), "exit {code} (findings) must be clean");
        }
    }

    #[test]
    fn zizmor_exit_1_audit_error_is_not_clean() {
        assert!(!zizmor_exit_is_clean(Some(1)));
    }

    #[test]
    fn zizmor_exit_2_argument_error_is_not_clean() {
        assert!(!zizmor_exit_is_clean(Some(2)));
    }

    #[test]
    fn zizmor_exit_3_no_inputs_is_not_clean() {
        assert!(!zizmor_exit_is_clean(Some(3)));
    }

    // --- parse_json_array_output: instrument-failure unit coverage (no process spawn) ---

    #[test]
    fn empty_output_is_no_findings() {
        let result = parse_json_array_output(b"", "actionlint");
        assert_eq!(result, Ok(Vec::new()));
    }

    #[test]
    fn valid_json_array_parses() {
        let result = parse_json_array_output(br#"[{"message":"x"}]"#, "actionlint");
        let findings = result.expect("valid JSON array must parse");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn garbled_output_is_instrument_failure() {
        let result = parse_json_array_output(b"not json at all {{{", "zizmor");
        let msg = result.expect_err("garbled output must be Err, not silently empty");
        assert!(msg.contains("zizmor"), "{msg}");
    }

    // --- summarize_json_finding: defensive field extraction ---

    #[test]
    fn summarize_prefers_message_field() {
        let v: serde_json::Value = serde_json::from_str(r#"{"message":"m","filepath":"f.yml"}"#)
            .expect("test JSON must parse");
        assert_eq!(summarize_json_finding(&v), "f.yml: m");
    }

    #[test]
    fn summarize_falls_back_to_desc_field() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"desc":"d"}"#).expect("test JSON must parse");
        assert_eq!(summarize_json_finding(&v), "d");
    }

    #[test]
    fn summarize_never_panics_on_unknown_shape() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"totally":"unexpected"}"#).expect("test JSON must parse");
        // Must not panic; exact wording isn't load-bearing.
        let _ = summarize_json_finding(&v);
    }

    // --- load_policy: malformed config is an instrument failure ---

    #[test]
    fn load_policy_missing_file_is_err() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(load_policy(tmp.path()).is_err());
        Ok(())
    }

    #[test]
    fn load_policy_malformed_toml_is_err() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(tmp.path().join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(tmp.path().join(POLICY_FILE), "this = [is not : valid toml")
            .map_err(|e| e.to_string())?;
        assert!(load_policy(tmp.path()).is_err());
        Ok(())
    }

    #[test]
    fn real_policy_toml_deserializes() -> std::result::Result<(), String> {
        let root = crate::utils::project_root().map_err(|e| e.to_string())?;
        let policy = load_policy(&root).map_err(|e| e.to_string())?;
        assert_eq!(policy.enforcement, Enforcement::Advisory);
        assert!(
            policy.first_party_pin_allowlist.iter().any(|p| p == "actions/"),
            "expected actions/ in the first-party pin allowlist"
        );
        Ok(())
    }

    // --- check(): end-to-end exit-path coverage (mutation-check style) ---

    fn write_policy(dir: &Path, advisory_expected_from: &str) -> std::result::Result<(), String> {
        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join(POLICY_FILE),
            format!(
                r#"
schema_version = 1
enforcement = "advisory"
advisory_expected_from = "{advisory_expected_from}"
blocking_enforced_from = ""
first_party_pin_allowlist = ["actions/", "github/"]
self_hosted_runner_labels = []
matrix_os_allowlist = []
"#
            ),
        )
        .map_err(|e| e.to_string())
    }

    fn write_clean_workflow(dir: &Path) -> std::result::Result<(), String> {
        let workflows = dir.join(".github/workflows");
        std::fs::create_dir_all(&workflows).map_err(|e| e.to_string())?;
        std::fs::write(
            workflows.join("ci.yml"),
            "permissions:\n  contents: read\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v7\n",
        )
        .map_err(|e| e.to_string())
    }

    #[test]
    fn check_malformed_policy_toml_is_instrument_failure_exit2() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("policy")).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(POLICY_FILE), "this = [is not : valid toml")
            .map_err(|e| e.to_string())?;
        let result = check(None, true, None, Some(dir.to_path_buf()));
        assert!(result.is_err(), "malformed policy/workflow-contracts.toml must be exit 2");
        Ok(())
    }

    #[test]
    fn check_self_test_with_clean_tree_is_policy_satisfied() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir, "")?;
        write_clean_workflow(dir)?;
        let outcome =
            check(None, true, None, Some(dir.to_path_buf())).map_err(|e| e.to_string())?;
        assert_eq!(outcome, CheckOutcome::PolicySatisfied);
        Ok(())
    }

    #[test]
    fn write_receipt_round_trips_findings() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = tmp.path().join("receipt.json");
        write_receipt(&path, &["ci.yml: not SHA-pinned: foo/bar@v1".to_string()], false)?;
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let parsed: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        assert_eq!(parsed["receipt_kind"], "workflow_contracts");
        assert_eq!(parsed["passed"], false);
        assert_eq!(parsed["finding_count"], 1);
        assert_eq!(parsed["findings"][0]["message"], "ci.yml: not SHA-pinned: foo/bar@v1");
        Ok(())
    }

    #[test]
    fn write_receipt_empty_findings_passes() -> std::result::Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = tmp.path().join("nested").join("receipt.json");
        write_receipt(&path, &[], true)?;
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let parsed: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        assert_eq!(parsed["passed"], true);
        assert_eq!(parsed["finding_count"], 0);
        Ok(())
    }

    #[test]
    fn check_missing_workflows_dir_is_instrument_failure_exit2() -> std::result::Result<(), String>
    {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dir = tmp.path();
        write_policy(dir, "")?;
        // No .github/workflows dir at all.
        let result = check(None, true, None, Some(dir.to_path_buf()));
        assert!(result.is_err(), "an unreadable workflows dir must be an instrument failure");
        Ok(())
    }
}
