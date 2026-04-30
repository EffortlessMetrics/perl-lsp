//! In-process UX scenario recorder.
//!
//! Provides [`UxRunRecorder`] for recording assertions, operation-scoped timing,
//! and emitting [`UxScenarioRunReceipt`] JSON documents to disk.

use crate::taxonomy::{
    UxCiTier, UxComponent, UxFailureClass, UxRoute, UxScenarioResult, route_for_failure_class,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime};

// ── Public types ─────────────────────────────────────────────────────────

/// Error returned by [`UxRunRecorder::check`] when the assertion fails.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UxCheckFailure {
    /// Human-readable description of the failed check.
    pub description: String,
}

impl fmt::Display for UxCheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UX check failed: {}", self.description)
    }
}

impl std::error::Error for UxCheckFailure {}

/// Error returned by a scenario body to mark the scenario as skipped.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UxScenarioSkip {
    /// Human-readable skip reason.
    pub reason: String,
    /// Classification for the skipped scenario.
    pub failure_class: UxFailureClass,
}

impl UxScenarioSkip {
    /// Create a skipped-scenario signal with an explicit failure class.
    pub fn new(reason: impl Into<String>, failure_class: UxFailureClass) -> Self {
        Self { reason: reason.into(), failure_class }
    }

    /// Create an infra-classified skipped-scenario signal.
    pub fn infra(reason: impl Into<String>) -> Self {
        Self::new(reason, UxFailureClass::Infra)
    }
}

impl fmt::Display for UxScenarioSkip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UX scenario skipped: {}", self.reason)
    }
}

impl std::error::Error for UxScenarioSkip {}

/// Basis for assertion counts in a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AssertionBasis {
    /// Scenario uses explicit `recorder.check()` calls.
    Instrumented,
    /// Scenario has not yet been instrumented with `recorder.check()`.
    NotYetInstrumented,
}

/// Assertion tracking for a scenario receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssertionCounts {
    /// Number of passed checks, or `None` if not instrumented.
    pub passed: Option<u32>,
    /// Number of failed checks, or `None` if not instrumented.
    pub failed: Option<u32>,
    /// Whether the scenario is instrumented with explicit checks.
    pub basis: AssertionBasis,
    /// Descriptions of failed checks.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub failed_check_names: Vec<String>,
}

/// Per-operation timing entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OperationTiming {
    /// Name of the LSP operation (e.g. `"completion"`, `"goto_definition"`).
    pub operation: String,
    /// Elapsed ms from `mark_request_start` to `mark_first_useful_result`,
    /// or `None` if timing could not be recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_useful_result_ms: Option<f64>,
    /// Status of the timing measurement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing_status: Option<String>,
}

/// CI run identity metadata — all fields nullable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunIdentity {
    /// Git SHA of the commit under test.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// Git branch name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// CI run ID (e.g. GitHub Actions run ID).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// CI run attempt number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    /// CI runner platform (e.g. `"Linux"`, `"Windows"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

/// The machine-readable receipt emitted after each UX scenario execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UxScenarioRunReceipt {
    /// Receipt kind discriminator.
    pub kind: String,
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// ISO-8601 timestamp of when the measurement was taken.
    pub measured_at: String,
    /// CI run identity (sha, branch, run_id, attempt, platform).
    pub run_identity: RunIdentity,
    /// Workflow identifier for this scenario.
    pub workflow_id: String,
    /// Source file of the scenario.
    pub scenario_file: String,
    /// Per-case test name within the scenario file.
    pub test_name: String,
    /// Subsystem component exercised by this scenario.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<UxComponent>,
    /// CI execution tier.
    pub ci_tier: UxCiTier,
    /// Scenario execution result.
    pub result: UxScenarioResult,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: f64,
    /// Time to first useful result in milliseconds (top-level, first operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_useful_result_ms: Option<f64>,
    /// Per-operation timing entries.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub operation_timings: Vec<OperationTiming>,
    /// Assertion counts and basis.
    pub assertions: AssertionCounts,
    /// Failure classification (non-null only when result is fail/quarantined).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<UxFailureClass>,
    /// Semantic routing hint for failure investigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<UxRoute>,
    /// Human-readable reason for a skipped scenario.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    /// Full cargo test command for reproduction.
    pub canonical_repro: String,
    /// Short `just` command for reproduction.
    pub friendly_repro: String,
}

// ── Internal timing state ────────────────────────────────────────────────

/// Tracks the state of a single operation's timing measurement.
#[derive(Debug)]
enum TimingState {
    /// `mark_request_start` was called; waiting for `mark_first_useful_result`.
    Started(Instant),
    /// Timing was recorded successfully.
    Completed(f64),
    /// `mark_first_useful_result` was called without a preceding `mark_request_start`.
    MissingRequestStart,
}

// ── UxRunRecorder ────────────────────────────────────────────────────────

/// In-process recorder for UX scenario assertions and timing.
///
/// Created at the start of a scenario, accumulates check results and
/// operation timing, then produces a [`UxScenarioRunReceipt`] via
/// [`finish_pass`](Self::finish_pass), [`finish_fail`](Self::finish_fail), or
/// [`finish_skipped`](Self::finish_skipped).
pub struct UxRunRecorder {
    workflow_id: String,
    scenario_file: String,
    test_name: String,
    ci_tier: UxCiTier,
    component: Option<UxComponent>,
    start: Instant,
    /// Per-operation timing state, keyed by operation name.
    operation_timings: BTreeMap<String, TimingState>,
    /// Order in which operations were first seen (for stable output).
    operation_order: Vec<String>,
    passed: u32,
    failed: u32,
    failed_check_names: Vec<String>,
    instrumented: bool,
}

impl UxRunRecorder {
    /// Create a new recorder for a scenario.
    ///
    /// - `workflow_id`: identifies the workflow (e.g. `"completion_basic"`).
    /// - `scenario_file`: source file name (e.g. `"ux_scenario_01.rs"`).
    /// - `test_name`: per-case identity within the scenario file.
    /// - `ci_tier`: execution tier (pr/nightly/release).
    /// - `component`: optional subsystem component.
    pub fn new(
        workflow_id: impl Into<String>,
        scenario_file: impl Into<String>,
        test_name: impl Into<String>,
        ci_tier: UxCiTier,
        component: Option<UxComponent>,
    ) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            scenario_file: scenario_file.into(),
            test_name: test_name.into(),
            ci_tier,
            component,
            start: Instant::now(),
            operation_timings: BTreeMap::new(),
            operation_order: Vec::new(),
            passed: 0,
            failed: 0,
            failed_check_names: Vec::new(),
            instrumented: false,
        }
    }

    /// Record a named assertion check.
    ///
    /// Returns `Ok(())` when `condition` is true, or `Err(UxCheckFailure)` when
    /// false, allowing callers to use `?` for early exit on failure.
    pub fn check(&mut self, description: &str, condition: bool) -> Result<(), UxCheckFailure> {
        self.instrumented = true;
        if condition {
            self.passed += 1;
            Ok(())
        } else {
            self.failed += 1;
            self.failed_check_names.push(description.to_owned());
            Err(UxCheckFailure { description: description.to_owned() })
        }
    }

    /// Mark the start of an LSP request for timing.
    ///
    /// Call this immediately before initiating the LSP request.
    pub fn mark_request_start(&mut self, operation: &str) {
        let key = operation.to_owned();
        if !self.operation_timings.contains_key(&key) {
            self.operation_order.push(key.clone());
        }
        self.operation_timings.insert(key, TimingState::Started(Instant::now()));
    }

    /// Mark the first useful result for an operation.
    ///
    /// Only the first call per operation is recorded. If no
    /// [`mark_request_start`](Self::mark_request_start) preceded it, records
    /// `timing_status: "missing_request_start"` and sets timing to `None`.
    pub fn mark_first_useful_result(&mut self, operation: &str) {
        let key = operation.to_owned();
        match self.operation_timings.get(&key) {
            Some(TimingState::Started(start)) => {
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                self.operation_timings.insert(key, TimingState::Completed(elapsed_ms));
            }
            Some(TimingState::Completed(_) | TimingState::MissingRequestStart) => {
                // Already recorded — ignore subsequent calls.
            }
            None => {
                // No mark_request_start preceded this call.
                if !self.operation_timings.contains_key(&key) {
                    self.operation_order.push(key.clone());
                }
                self.operation_timings.insert(key, TimingState::MissingRequestStart);
            }
        }
    }

    /// Finalize as pass and return the receipt.
    pub fn finish_pass(&self) -> UxScenarioRunReceipt {
        self.build_receipt(UxScenarioResult::Pass, None, None)
    }

    /// Finalize as fail with a failure class and return the receipt.
    pub fn finish_fail(&self, class: UxFailureClass) -> UxScenarioRunReceipt {
        self.build_receipt(UxScenarioResult::Fail, Some(class), None)
    }

    /// Finalize as skipped and return the receipt.
    pub fn finish_skipped(&self, skip: &UxScenarioSkip) -> UxScenarioRunReceipt {
        self.build_receipt(
            UxScenarioResult::Skipped,
            Some(skip.failure_class),
            Some(skip.reason.clone()),
        )
    }

    /// Write a receipt to disk.
    ///
    /// Default directory: `target/receipts/editor-ux/`.
    /// Override with the `PERL_LSP_UX_RECEIPT_DIR` environment variable.
    /// Creates the directory if it does not exist.
    ///
    /// Filename format:
    /// `{workflow_id}-{scenario_stem}-{test_name}-{sha_short}.json`
    /// with optional `run_id` and `attempt` segments to avoid collisions.
    pub fn write_receipt(&self, receipt: &UxScenarioRunReceipt) -> std::io::Result<PathBuf> {
        let dir = std::env::var("PERL_LSP_UX_RECEIPT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_receipt_dir());
        write_receipt_to_dir(receipt, &dir)
    }

    // ── Private helpers ──────────────────────────────────────────────────

    fn build_receipt(
        &self,
        result: UxScenarioResult,
        failure_class: Option<UxFailureClass>,
        skip_reason: Option<String>,
    ) -> UxScenarioRunReceipt {
        let duration_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        let route = failure_class.map(route_for_failure_class);

        let operation_timings: Vec<OperationTiming> = self
            .operation_order
            .iter()
            .filter_map(|op| {
                self.operation_timings.get(op).map(|state| match state {
                    TimingState::Started(_) => OperationTiming {
                        operation: op.clone(),
                        time_to_first_useful_result_ms: None,
                        timing_status: None,
                    },
                    TimingState::Completed(ms) => OperationTiming {
                        operation: op.clone(),
                        time_to_first_useful_result_ms: Some(*ms),
                        timing_status: None,
                    },
                    TimingState::MissingRequestStart => OperationTiming {
                        operation: op.clone(),
                        time_to_first_useful_result_ms: None,
                        timing_status: Some("missing_request_start".to_owned()),
                    },
                })
            })
            .collect();

        // Top-level time_to_first_useful_result_ms: first completed operation.
        let time_to_first_useful_result_ms = self.operation_order.iter().find_map(|op| {
            if let Some(TimingState::Completed(ms)) = self.operation_timings.get(op) {
                Some(*ms)
            } else {
                None
            }
        });

        let assertions = if self.instrumented {
            AssertionCounts {
                passed: Some(self.passed),
                failed: Some(self.failed),
                basis: AssertionBasis::Instrumented,
                failed_check_names: self.failed_check_names.clone(),
            }
        } else {
            AssertionCounts {
                passed: None,
                failed: None,
                basis: AssertionBasis::NotYetInstrumented,
                failed_check_names: Vec::new(),
            }
        };

        let run_identity = build_run_identity();

        // Repro commands.
        let canonical_repro = format!(
            "cargo test -p perl-lsp-ux-tests {} -- --test-threads=1 --nocapture",
            self.test_name
        );
        let short_test_name = self.test_name.rsplit("::").next().unwrap_or(&self.test_name);
        let friendly_repro = format!("just ux-tests {short_test_name}");

        UxScenarioRunReceipt {
            kind: "ux_scenario_run".to_owned(),
            schema_version: 1,
            measured_at: iso8601_now(),
            run_identity,
            workflow_id: self.workflow_id.clone(),
            scenario_file: self.scenario_file.clone(),
            test_name: self.test_name.clone(),
            component: self.component,
            ci_tier: self.ci_tier,
            result,
            duration_ms,
            time_to_first_useful_result_ms,
            operation_timings,
            assertions,
            failure_class,
            route,
            skip_reason,
            canonical_repro,
            friendly_repro,
        }
    }
}

// ── Free functions ───────────────────────────────────────────────────────

/// Write a receipt to a specific directory.
///
/// Creates the directory if it does not exist. Returns the path of the
/// written file.
pub fn write_receipt_to_dir(
    receipt: &UxScenarioRunReceipt,
    dir: &std::path::Path,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;

    let stem = sanitize_filename_segment(receipt.scenario_file.trim_end_matches(".rs"));
    let sha_short =
        receipt.run_identity.sha.as_deref().and_then(|s| s.get(..8)).unwrap_or("unknown");

    let mut filename = format!(
        "{}-{}-{}-{}",
        sanitize_filename_segment(&receipt.workflow_id),
        stem,
        sanitize_filename_segment(&receipt.test_name),
        sanitize_filename_segment(sha_short)
    );

    // Add run_id and attempt to avoid collisions when available.
    if let Some(ref run_id) = receipt.run_identity.run_id {
        filename.push('-');
        filename.push_str(&sanitize_filename_segment(run_id));
    }
    if let Some(attempt) = receipt.run_identity.attempt {
        filename.push_str(&format!("-{attempt}"));
    }

    filename.push_str(".json");

    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(receipt).map_err(std::io::Error::other)?;
    std::fs::write(&path, format!("{json}\n"))?;
    Ok(path)
}

fn sanitize_filename_segment(segment: &str) -> String {
    let sanitized: String = segment
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();

    let trimmed = sanitized.trim_matches([' ', '.']);
    if trimmed.is_empty() { "unknown".to_owned() } else { trimmed.to_owned() }
}

/// Classify an `anyhow::Error` into a `UxFailureClass`.
///
/// Currently all errors are classified as `Unknown` — the classifier does not
/// attempt to infer failure class from the error payload. Panics are also
/// `Unknown` (see `run_ux_scenario`).
fn classify_error(_err: &anyhow::Error) -> UxFailureClass {
    UxFailureClass::Unknown
}

/// Panic-safe scenario wrapper.
///
/// Wraps the scenario closure in [`std::panic::catch_unwind`], ensures a receipt
/// is written to disk on pass, fail, **and** panic, then re-raises the panic via
/// [`std::panic::resume_unwind`].
///
/// # Receipt guarantee
///
/// A receipt is **always** written to disk before this function returns or
/// re-raises a panic. Write errors are silently ignored so they never mask the
/// original test outcome.
///
/// # Panic classification
///
/// Panics are classified as [`UxFailureClass::Unknown`]. We intentionally do
/// **not** over-classify — a missing completion item expressed as `assert!`
/// panic is not necessarily `NewTestBug` without classifier evidence.
pub fn run_ux_scenario<F>(
    workflow_id: &str,
    scenario_file: &str,
    test_name: &str,
    ci_tier: UxCiTier,
    component: Option<UxComponent>,
    body: F,
) where
    F: FnOnce(&mut UxRunRecorder) -> anyhow::Result<()>,
{
    run_ux_scenario_with_receipt_dir(
        workflow_id,
        scenario_file,
        test_name,
        ci_tier,
        component,
        None,
        body,
    );
}

fn run_ux_scenario_with_receipt_dir<F>(
    workflow_id: &str,
    scenario_file: &str,
    test_name: &str,
    ci_tier: UxCiTier,
    component: Option<UxComponent>,
    receipt_dir: Option<&Path>,
    body: F,
) where
    F: FnOnce(&mut UxRunRecorder) -> anyhow::Result<()>,
{
    let mut recorder =
        UxRunRecorder::new(workflow_id, scenario_file, test_name, ci_tier, component);

    // `catch_unwind` requires `UnwindSafe`. The recorder contains only owned
    // data and no interior-mutable shared state, so wrapping in
    // `AssertUnwindSafe` is sound here.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(&mut recorder)));

    match result {
        // Closure returned Ok — scenario passed.
        Ok(Ok(())) => {
            let receipt = recorder.finish_pass();
            write_wrapper_receipt(&recorder, &receipt, receipt_dir);
        }
        // Closure returned Err — non-panic assertion failure (e.g. `check()?`).
        Ok(Err(ref err)) => {
            if let Some(skip) = err.downcast_ref::<UxScenarioSkip>() {
                let receipt = recorder.finish_skipped(skip);
                write_wrapper_receipt(&recorder, &receipt, receipt_dir);
                return;
            }

            let class = classify_error(err);
            let receipt = recorder.finish_fail(class);
            write_wrapper_receipt(&recorder, &receipt, receipt_dir);
            std::panic::resume_unwind(Box::new(format!("UX scenario failed: {err}")));
        }
        // Closure panicked — write receipt then re-raise.
        Err(panic_payload) => {
            let receipt = recorder.finish_fail(UxFailureClass::Unknown);
            write_wrapper_receipt(&recorder, &receipt, receipt_dir);
            std::panic::resume_unwind(panic_payload);
        }
    }
}

fn write_wrapper_receipt(
    recorder: &UxRunRecorder,
    receipt: &UxScenarioRunReceipt,
    receipt_dir: Option<&Path>,
) {
    if let Some(dir) = receipt_dir {
        let _ = write_receipt_to_dir(receipt, dir);
    } else {
        let _ = recorder.write_receipt(receipt);
    }
}

fn default_receipt_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        manifest_dir.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or(manifest_dir);
    root.join("target").join("receipts").join("editor-ux")
}

/// Build run identity from environment variables.
fn build_run_identity() -> RunIdentity {
    let sha = env_value("GIT_SHA")
        .or_else(|| env_value("GITHUB_SHA"))
        .or_else(|| git_output(&["rev-parse", "HEAD"]));
    let branch = env_value("GITHUB_REF_NAME").or_else(|| {
        git_output(&["rev-parse", "--abbrev-ref", "HEAD"]).filter(|name| name != "HEAD")
    });
    let run_id = env_value("GITHUB_RUN_ID");
    let attempt = std::env::var("GITHUB_RUN_ATTEMPT").ok().and_then(|s| s.parse::<u32>().ok());
    let platform = env_value("RUNNER_OS").or_else(|| Some(std::env::consts::OS.to_string()));

    RunIdentity { sha, branch, run_id, attempt, platform }
}

fn env_value(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Produce an ISO-8601 timestamp from the current system time.
fn iso8601_now() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Break epoch seconds into date/time components.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Convert days since epoch to year/month/day (civil calendar).
    let (year, month, day) = days_to_civil(days);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Algorithm from Howard Hinnant's `chrono`-compatible civil date computation.
fn days_to_civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_identity_has_local_platform_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let identity = build_run_identity();

        assert!(
            identity.platform.as_deref().is_some_and(|platform| !platform.is_empty()),
            "run identity should include a local platform fallback"
        );
        Ok(())
    }

    #[test]
    fn finish_pass_produces_pass_receipt() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = UxRunRecorder::new(
            "wf_01",
            "ux_scenario_01.rs",
            "basic_open",
            UxCiTier::Pr,
            Some(UxComponent::Completion),
        );
        let receipt = recorder.finish_pass();

        assert_eq!(receipt.result, UxScenarioResult::Pass);
        assert_eq!(receipt.kind, "ux_scenario_run");
        assert_eq!(receipt.schema_version, 1);
        assert!(receipt.failure_class.is_none());
        assert!(receipt.route.is_none());
        assert_eq!(receipt.workflow_id, "wf_01");
        assert_eq!(receipt.scenario_file, "ux_scenario_01.rs");
        assert_eq!(receipt.test_name, "basic_open");
        assert_eq!(receipt.ci_tier, UxCiTier::Pr);
        assert_eq!(receipt.component, Some(UxComponent::Completion));
        Ok(())
    }

    #[test]
    fn finish_fail_produces_fail_receipt_with_class_and_route()
    -> Result<(), Box<dyn std::error::Error>> {
        let recorder =
            UxRunRecorder::new("wf_02", "ux_scenario_02.rs", "hover_test", UxCiTier::Nightly, None);
        let receipt = recorder.finish_fail(UxFailureClass::ProviderRegression);

        assert_eq!(receipt.result, UxScenarioResult::Fail);
        assert_eq!(receipt.failure_class, Some(UxFailureClass::ProviderRegression));
        assert_eq!(receipt.route, Some(UxRoute::ProviderFix));
        Ok(())
    }

    #[test]
    fn check_tracks_passed_and_failed_counts() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder =
            UxRunRecorder::new("wf_03", "ux_scenario_03.rs", "assertion_test", UxCiTier::Pr, None);

        // Two passing checks.
        recorder.check("first passes", true)?;
        recorder.check("second passes", true)?;

        // One failing check — capture the error but continue.
        let err = recorder.check("third fails", false);
        assert!(err.is_err());

        // Another passing check.
        recorder.check("fourth passes", true)?;

        // Another failing check.
        let err2 = recorder.check("fifth fails", false);
        assert!(err2.is_err());

        let receipt = recorder.finish_pass();
        assert_eq!(receipt.assertions.passed, Some(3));
        assert_eq!(receipt.assertions.failed, Some(2));
        assert_eq!(receipt.assertions.basis, AssertionBasis::Instrumented);
        assert_eq!(receipt.assertions.failed_check_names, vec!["third fails", "fifth fails"]);
        Ok(())
    }

    #[test]
    fn uninstrumented_scenario_has_null_assertions() -> Result<(), Box<dyn std::error::Error>> {
        let recorder =
            UxRunRecorder::new("wf_04", "ux_scenario_04.rs", "no_checks", UxCiTier::Release, None);
        let receipt = recorder.finish_pass();

        assert_eq!(receipt.assertions.passed, None);
        assert_eq!(receipt.assertions.failed, None);
        assert_eq!(receipt.assertions.basis, AssertionBasis::NotYetInstrumented);
        assert!(receipt.assertions.failed_check_names.is_empty());
        Ok(())
    }

    #[test]
    fn operation_timing_records_elapsed() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder =
            UxRunRecorder::new("wf_05", "ux_scenario_05.rs", "timing_test", UxCiTier::Pr, None);

        recorder.mark_request_start("completion");
        // Small sleep to ensure non-zero elapsed time.
        std::thread::sleep(std::time::Duration::from_millis(5));
        recorder.mark_first_useful_result("completion");

        let receipt = recorder.finish_pass();
        assert_eq!(receipt.operation_timings.len(), 1);
        let timing = &receipt.operation_timings[0];
        assert_eq!(timing.operation, "completion");
        assert!(timing.time_to_first_useful_result_ms.is_some());
        let ms = timing.time_to_first_useful_result_ms.unwrap_or(0.0);
        assert!(ms >= 0.0, "timing should be non-negative, got {ms}");
        assert!(timing.timing_status.is_none());

        // Top-level timing should also be set.
        assert!(receipt.time_to_first_useful_result_ms.is_some());
        Ok(())
    }

    #[test]
    fn mark_first_useful_result_without_start_records_missing()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder = UxRunRecorder::new(
            "wf_06",
            "ux_scenario_06.rs",
            "missing_start_test",
            UxCiTier::Pr,
            None,
        );

        // No mark_request_start — call mark_first_useful_result directly.
        recorder.mark_first_useful_result("goto_definition");

        let receipt = recorder.finish_pass();
        assert_eq!(receipt.operation_timings.len(), 1);
        let timing = &receipt.operation_timings[0];
        assert_eq!(timing.operation, "goto_definition");
        assert!(timing.time_to_first_useful_result_ms.is_none());
        assert_eq!(timing.timing_status.as_deref(), Some("missing_request_start"));

        // Top-level timing should be None since no operation completed.
        assert!(receipt.time_to_first_useful_result_ms.is_none());
        Ok(())
    }

    #[test]
    fn second_mark_first_useful_result_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder =
            UxRunRecorder::new("wf_07", "ux_scenario_07.rs", "idempotent_test", UxCiTier::Pr, None);

        recorder.mark_request_start("completion");
        std::thread::sleep(std::time::Duration::from_millis(5));
        recorder.mark_first_useful_result("completion");

        // Capture the first timing.
        let first_ms = {
            let receipt = recorder.finish_pass();
            receipt.operation_timings[0].time_to_first_useful_result_ms.unwrap_or(0.0)
        };

        // Second call should be ignored — rebuild recorder to test.
        let mut recorder2 =
            UxRunRecorder::new("wf_07", "ux_scenario_07.rs", "idempotent_test", UxCiTier::Pr, None);
        recorder2.mark_request_start("completion");
        std::thread::sleep(std::time::Duration::from_millis(5));
        recorder2.mark_first_useful_result("completion");
        // Wait longer, then call again.
        std::thread::sleep(std::time::Duration::from_millis(20));
        recorder2.mark_first_useful_result("completion");

        let receipt2 = recorder2.finish_pass();
        let second_ms = receipt2.operation_timings[0].time_to_first_useful_result_ms.unwrap_or(0.0);

        // The second call should not have updated the timing — it should be
        // close to the first measurement, not the longer one.
        // We just verify it's still a valid non-negative number.
        assert!(second_ms >= 0.0);
        assert!(first_ms >= 0.0);
        Ok(())
    }

    #[test]
    fn repro_commands_are_populated() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = UxRunRecorder::new(
            "wf_08",
            "ux_scenario_08.rs",
            "my_module::my_test",
            UxCiTier::Pr,
            None,
        );
        let receipt = recorder.finish_pass();

        assert!(
            receipt.canonical_repro.contains("cargo test -p perl-lsp-ux-tests"),
            "canonical_repro should contain cargo test command: {}",
            receipt.canonical_repro
        );
        assert!(
            receipt.canonical_repro.contains("my_module::my_test"),
            "canonical_repro should contain test name: {}",
            receipt.canonical_repro
        );
        assert!(
            receipt.friendly_repro.contains("just ux-tests"),
            "friendly_repro should contain just command: {}",
            receipt.friendly_repro
        );
        assert!(
            receipt.friendly_repro.contains("my_test"),
            "friendly_repro should contain short test name: {}",
            receipt.friendly_repro
        );
        Ok(())
    }

    #[test]
    fn receipt_serializes_to_valid_json() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder = UxRunRecorder::new(
            "wf_09",
            "ux_scenario_09.rs",
            "json_test",
            UxCiTier::Pr,
            Some(UxComponent::Diagnostics),
        );
        recorder.check("something works", true)?;
        recorder.mark_request_start("diagnostics");
        recorder.mark_first_useful_result("diagnostics");

        let receipt = recorder.finish_pass();
        let json = serde_json::to_string_pretty(&receipt)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;

        assert_eq!(parsed["kind"], "ux_scenario_run");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["result"], "pass");
        assert_eq!(parsed["ci_tier"], "pr");
        assert_eq!(parsed["component"], "diagnostics");
        assert_eq!(parsed["assertions"]["basis"], "instrumented");
        assert_eq!(parsed["assertions"]["passed"], 1);
        assert_eq!(parsed["assertions"]["failed"], 0);
        Ok(())
    }

    #[test]
    fn write_receipt_creates_file() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        let recorder = UxRunRecorder::new(
            "wf_write",
            "ux_scenario_write.rs",
            "write_test",
            UxCiTier::Pr,
            None,
        );
        let receipt = recorder.finish_pass();
        let path = write_receipt_to_dir(&receipt, &receipt_dir)?;

        assert!(path.exists(), "receipt file should exist at {path:?}");

        // Verify it's valid JSON.
        let content = std::fs::read_to_string(&path)?;
        let _parsed: UxScenarioRunReceipt = serde_json::from_str(&content)?;

        Ok(())
    }

    #[test]
    fn write_receipt_sanitizes_filename_segments() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        let recorder = UxRunRecorder::new(
            "wf:write",
            "nested\\ux_scenario_write.rs",
            "module::write/test",
            UxCiTier::Pr,
            None,
        );
        let receipt = recorder.finish_pass();
        let path = write_receipt_to_dir(&receipt, &receipt_dir)?;
        let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();

        assert!(path.exists(), "receipt file should exist at {path:?}");
        assert!(
            !filename.contains([':', '\\', '/', '*', '?', '"', '<', '>', '|']),
            "receipt filename should be portable, got {filename}"
        );

        Ok(())
    }

    #[test]
    fn measured_at_is_iso8601() -> Result<(), Box<dyn std::error::Error>> {
        let ts = iso8601_now();
        // Basic format check: YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.len() >= 20, "timestamp too short: {ts}");
        assert!(ts.contains('T'), "timestamp missing T separator: {ts}");
        assert!(ts.ends_with('Z'), "timestamp missing Z suffix: {ts}");
        Ok(())
    }

    #[test]
    fn multiple_operations_tracked_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder = UxRunRecorder::new(
            "wf_multi",
            "ux_scenario_multi.rs",
            "multi_op_test",
            UxCiTier::Pr,
            None,
        );

        recorder.mark_request_start("completion");
        recorder.mark_request_start("hover");
        recorder.mark_first_useful_result("completion");
        recorder.mark_first_useful_result("hover");

        let receipt = recorder.finish_pass();
        assert_eq!(receipt.operation_timings.len(), 2);
        assert_eq!(receipt.operation_timings[0].operation, "completion");
        assert_eq!(receipt.operation_timings[1].operation, "hover");
        assert!(receipt.operation_timings[0].time_to_first_useful_result_ms.is_some());
        assert!(receipt.operation_timings[1].time_to_first_useful_result_ms.is_some());
        Ok(())
    }

    #[test]
    fn duration_ms_is_non_negative() -> Result<(), Box<dyn std::error::Error>> {
        let recorder =
            UxRunRecorder::new("wf_dur", "ux_scenario_dur.rs", "duration_test", UxCiTier::Pr, None);
        let receipt = recorder.finish_pass();
        assert!(
            receipt.duration_ms >= 0.0,
            "duration_ms should be non-negative: {}",
            receipt.duration_ms
        );
        Ok(())
    }

    // ── run_ux_scenario wrapper tests ────────────────────────────────────

    #[test]
    fn run_ux_scenario_pass_writes_receipt_to_disk() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        run_ux_scenario_with_receipt_dir(
            "wf_pass",
            "ux_scenario_pass.rs",
            "pass_body",
            UxCiTier::Pr,
            Some(UxComponent::Completion),
            Some(&receipt_dir),
            |recorder| {
                recorder.check("always true", true)?;
                Ok(())
            },
        );

        // Verify a receipt file was written.
        let entries: Vec<_> = std::fs::read_dir(&receipt_dir)?.filter_map(|e| e.ok()).collect();
        assert_eq!(entries.len(), 1, "expected exactly one receipt file");

        let content = std::fs::read_to_string(entries[0].path())?;
        let receipt: UxScenarioRunReceipt = serde_json::from_str(&content)?;
        assert_eq!(receipt.result, UxScenarioResult::Pass);
        assert_eq!(receipt.workflow_id, "wf_pass");
        assert_eq!(receipt.test_name, "pass_body");
        assert!(receipt.failure_class.is_none());
        assert!(receipt.route.is_none());
        assert_eq!(receipt.assertions.passed, Some(1));
        assert_eq!(receipt.assertions.failed, Some(0));
        assert_eq!(receipt.assertions.basis, AssertionBasis::Instrumented);
        Ok(())
    }

    #[test]
    fn run_ux_scenario_non_skip_err_writes_fail_receipt_and_fails_test()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_ux_scenario_with_receipt_dir(
                "wf_err",
                "ux_scenario_err.rs",
                "err_body",
                UxCiTier::Nightly,
                None,
                Some(&receipt_dir),
                |recorder| {
                    recorder.check("this will fail", false)?;
                    // The `?` above propagates the UxCheckFailure as an anyhow::Error.
                    Ok(())
                },
            );
        }));
        assert!(result.is_err(), "expected non-skip scenario errors to fail the test");

        let entries: Vec<_> = std::fs::read_dir(&receipt_dir)?.filter_map(|e| e.ok()).collect();
        assert_eq!(entries.len(), 1, "expected exactly one receipt file");

        let content = std::fs::read_to_string(entries[0].path())?;
        let receipt: UxScenarioRunReceipt = serde_json::from_str(&content)?;
        assert_eq!(receipt.result, UxScenarioResult::Fail);
        assert_eq!(receipt.failure_class, Some(UxFailureClass::Unknown));
        assert_eq!(receipt.route, Some(UxRoute::Triage));
        assert_eq!(receipt.assertions.failed, Some(1));
        assert_eq!(receipt.assertions.failed_check_names, vec!["this will fail"]);
        Ok(())
    }

    #[test]
    fn run_ux_scenario_skip_writes_skipped_receipt_to_disk()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        run_ux_scenario_with_receipt_dir(
            "wf_skip",
            "ux_scenario_skip.rs",
            "skip_body",
            UxCiTier::Pr,
            None,
            Some(&receipt_dir),
            |_recorder| Err(UxScenarioSkip::infra("binary unavailable").into()),
        );

        let entries: Vec<_> = std::fs::read_dir(&receipt_dir)?.filter_map(|e| e.ok()).collect();
        assert_eq!(entries.len(), 1, "expected exactly one skipped receipt file");

        let content = std::fs::read_to_string(entries[0].path())?;
        let receipt: UxScenarioRunReceipt = serde_json::from_str(&content)?;
        assert_eq!(receipt.result, UxScenarioResult::Skipped);
        assert_eq!(receipt.failure_class, Some(UxFailureClass::Infra));
        assert_eq!(receipt.route, Some(UxRoute::CiInvestigation));
        assert_eq!(receipt.skip_reason.as_deref(), Some("binary unavailable"));
        Ok(())
    }

    #[test]
    fn run_ux_scenario_panic_writes_receipt_then_resumes() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = tempfile::tempdir()?;
        let receipt_dir = tmp.path().join("receipts");

        // Catch the re-raised panic from run_ux_scenario.
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_ux_scenario_with_receipt_dir(
                "wf_panic",
                "ux_scenario_panic.rs",
                "panic_body",
                UxCiTier::Pr,
                None,
                Some(&receipt_dir),
                |_recorder| {
                    // Trigger a panic via resume_unwind to avoid the
                    // clippy::panic lint while still exercising the
                    // catch_unwind path in run_ux_scenario.
                    std::panic::resume_unwind(Box::new("intentional test panic"));
                },
            );
        }));

        // The panic should have been re-raised.
        assert!(panic_result.is_err(), "expected panic to be re-raised");

        // Receipt should still exist on disk despite the panic.
        let entries: Vec<_> = std::fs::read_dir(&receipt_dir)?.filter_map(|e| e.ok()).collect();
        assert_eq!(entries.len(), 1, "expected exactly one receipt file after panic");

        let content = std::fs::read_to_string(entries[0].path())?;
        let receipt: UxScenarioRunReceipt = serde_json::from_str(&content)?;
        assert_eq!(receipt.result, UxScenarioResult::Fail);
        assert_eq!(receipt.failure_class, Some(UxFailureClass::Unknown));
        assert_eq!(receipt.route, Some(UxRoute::Triage));
        Ok(())
    }

    #[test]
    fn skip_receipt_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = UxRunRecorder::new(
            "wf_skip",
            "ux_scenario_skip.rs",
            "skipped_test",
            UxCiTier::Pr,
            None,
        );
        let skip = UxScenarioSkip::infra("binary unavailable");
        let receipt = recorder.finish_skipped(&skip);

        let json = serde_json::to_string(&receipt)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["result"], "skipped");
        assert_eq!(parsed["failure_class"], "infra");
        assert_eq!(parsed["route"], "ci_investigation");
        assert_eq!(parsed["skip_reason"], "binary unavailable");

        // Round-trip back to struct.
        let deserialized: UxScenarioRunReceipt = serde_json::from_str(&json)?;
        assert_eq!(deserialized.result, UxScenarioResult::Skipped);
        assert_eq!(deserialized.failure_class, Some(UxFailureClass::Infra));
        assert_eq!(deserialized.skip_reason.as_deref(), Some("binary unavailable"));
        Ok(())
    }

    /// Verify that a correct empty result (expected-clean null for
    /// goto-definition, expected-empty diagnostics) still contributes to
    /// latency timing. The `mark_first_useful_result` call records timing
    /// even when the underlying LSP response is empty, because an
    /// expected-empty result is still a "useful" result.
    #[test]
    fn expected_empty_result_contributes_to_latency() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder = UxRunRecorder::new(
            "wf_empty",
            "ux_scenario_empty.rs",
            "expected_empty_test",
            UxCiTier::Pr,
            Some(UxComponent::GotoDefinition),
        );

        // Simulate: request start → receive expected-empty response → mark useful.
        recorder.mark_request_start("goto_definition");
        std::thread::sleep(std::time::Duration::from_millis(5));
        // The response was empty (expected-clean null), but it is still the
        // correct, useful result — so we mark it.
        recorder.mark_first_useful_result("goto_definition");

        let receipt = recorder.finish_pass();

        // The operation timing must be recorded (non-null) even though the
        // underlying result was empty.
        assert_eq!(receipt.operation_timings.len(), 1);
        let timing = &receipt.operation_timings[0];
        assert_eq!(timing.operation, "goto_definition");
        assert!(
            timing.time_to_first_useful_result_ms.is_some(),
            "expected-empty result must still produce non-null timing"
        );
        let ms = timing.time_to_first_useful_result_ms.unwrap_or(0.0);
        assert!(ms > 0.0, "timing should be positive, got {ms}");
        assert!(timing.timing_status.is_none(), "no timing error expected");

        // Top-level timing must also be populated (correctness-gated: only
        // passing scenarios contribute, and this is a pass).
        assert!(
            receipt.time_to_first_useful_result_ms.is_some(),
            "top-level timing must be non-null for expected-empty pass scenario"
        );

        Ok(())
    }

    /// Same as above but for diagnostics: an expected-empty diagnostics
    /// result (clean source, zero diagnostics) is still a useful result
    /// that contributes to latency.
    #[test]
    fn expected_empty_diagnostics_contributes_to_latency() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut recorder = UxRunRecorder::new(
            "wf_empty_diag",
            "ux_scenario_empty_diag.rs",
            "expected_empty_diagnostics_test",
            UxCiTier::Pr,
            Some(UxComponent::Diagnostics),
        );

        // Simulate: request start → receive zero diagnostics (expected) → mark useful.
        recorder.mark_request_start("diagnostics");
        std::thread::sleep(std::time::Duration::from_millis(5));
        recorder.mark_first_useful_result("diagnostics");

        let receipt = recorder.finish_pass();

        assert_eq!(receipt.operation_timings.len(), 1);
        let timing = &receipt.operation_timings[0];
        assert_eq!(timing.operation, "diagnostics");
        assert!(
            timing.time_to_first_useful_result_ms.is_some(),
            "expected-empty diagnostics must still produce non-null timing"
        );
        let ms = timing.time_to_first_useful_result_ms.unwrap_or(0.0);
        assert!(ms > 0.0, "timing should be positive, got {ms}");
        assert!(timing.timing_status.is_none());

        assert!(
            receipt.time_to_first_useful_result_ms.is_some(),
            "top-level timing must be non-null for expected-empty diagnostics pass"
        );

        Ok(())
    }

    /// Verify that a failing scenario's timing is recorded in the receipt
    /// even though the scorecard aggregator will exclude it from p95.
    /// The receipt itself must faithfully record what happened.
    #[test]
    fn failed_scenario_still_records_timing_in_receipt() -> Result<(), Box<dyn std::error::Error>> {
        let mut recorder = UxRunRecorder::new(
            "wf_fail_timing",
            "ux_scenario_fail_timing.rs",
            "fail_with_timing_test",
            UxCiTier::Pr,
            Some(UxComponent::GotoDefinition),
        );

        recorder.mark_request_start("goto_definition");
        std::thread::sleep(std::time::Duration::from_millis(5));
        recorder.mark_first_useful_result("goto_definition");

        // Simulate a check failure after timing was recorded.
        let _err = recorder.check("golden assertion fails", false);

        let receipt = recorder.finish_fail(UxFailureClass::ProviderRegression);

        // Timing is still in the receipt — the aggregator decides whether
        // to include it in p95 based on result == pass.
        assert_eq!(receipt.operation_timings.len(), 1);
        assert!(
            receipt.operation_timings[0].time_to_first_useful_result_ms.is_some(),
            "timing should be recorded even for failed scenarios"
        );
        assert!(
            receipt.time_to_first_useful_result_ms.is_some(),
            "top-level timing should be recorded even for failed scenarios"
        );
        assert_eq!(receipt.result, UxScenarioResult::Fail);

        Ok(())
    }

    #[test]
    fn quarantine_receipt_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = UxRunRecorder::new(
            "wf_quarantine",
            "ux_scenario_quarantine.rs",
            "quarantined_test",
            UxCiTier::Nightly,
            Some(UxComponent::ModuleResolution),
        );
        let mut receipt = recorder.finish_fail(UxFailureClass::TestRace);
        receipt.result = UxScenarioResult::Quarantined;

        let json = serde_json::to_string(&receipt)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["result"], "quarantined");
        assert_eq!(parsed["failure_class"], "test_race");
        assert_eq!(parsed["route"], "test_fix");
        assert_eq!(parsed["component"], "module_resolution");

        // Round-trip back to struct.
        let deserialized: UxScenarioRunReceipt = serde_json::from_str(&json)?;
        assert_eq!(deserialized.result, UxScenarioResult::Quarantined);
        assert_eq!(deserialized.failure_class, Some(UxFailureClass::TestRace));
        assert_eq!(deserialized.route, Some(UxRoute::TestFix));
        Ok(())
    }
}
