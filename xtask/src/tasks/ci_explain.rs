//! CI failure explainer — `cargo xtask ci explain`
//!
//! Reads gate receipts from `target/receipts/receipt.json` (or a path supplied
//! via `--receipt`) and prints a compact, actionable summary:
//!
//! ```text
//! blocking_check:  fmt
//! failure_class:   code_regression
//! source_file_line: xtask/src/tasks/ci_explain.rs:12
//! exists_on_base:  unknown
//! reproduce:       cargo xtask fmt --check
//! ```
//!
//! When no receipts exist it degrades gracefully:
//! ```text
//! inconclusive: no receipts; run `cargo xtask gates`
//! ```
//!
//! **Out of scope in this version**: `--run-id` (GH artifact download, tracked in #2652)
//! and `--base` (base-branch comparison, tracked in #2653).

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

// ── Receipt types (subset of gates::Receipt / gates::GateResult) ─────────────

/// Supported schema version string produced by `cargo xtask gates`.
const SUPPORTED_SCHEMA_VERSION: &str = "gates.v1";

#[derive(Debug, Default, Deserialize)]
struct Receipt {
    /// Schema version emitted by `gates.rs`; used to detect incompatible receipt shapes.
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    gates: Vec<GateResult>,
}

// ── Receipt load error ────────────────────────────────────────────────────────

/// Typed load error so `run()` can emit distinct inconclusive messages.
#[derive(Debug)]
enum ReceiptLoadError {
    /// The receipt file does not exist.
    Absent,
    /// The file exists but is not valid JSON or does not match the Receipt schema.
    Malformed(String),
    /// The file parsed, but its `schema_version` is not supported.
    UnsupportedSchema(String),
}

#[derive(Debug, Deserialize)]
struct GateResult {
    gate_name: String,
    status: String,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    first_failure: Option<FirstFailure>,
    #[serde(default)]
    output_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FirstFailure {
    #[serde(default)]
    site: Option<String>,
    // `test` and `message` are part of the gates receipt schema; keep them for
    // forward-compat deserialization even though ci-explain does not yet surface them.
    #[allow(dead_code)]
    #[serde(default)]
    test: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    message: Option<String>,
}

// ── Output receipt ────────────────────────────────────────────────────────────

/// The structured explanation emitted for machine consumers.
#[derive(Debug, Serialize)]
pub struct ExplainReceipt {
    /// Name of the first required gate that failed.
    pub blocking_check_name: Option<String>,
    /// Classification of the failure.
    pub failure_class: String,
    /// `file:line` extracted from first_failure.site (or output_summary heuristic).
    pub source_file_line: Option<String>,
    /// Base-branch comparison not yet implemented; always `"unknown"` (see #2653).
    pub exists_on_base: String,
    /// Exact command to reproduce locally.
    pub local_reproduction_command: Option<String>,
}

// ── FailureClass ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
enum FailureClass {
    CodeRegression,
    StaleBase,
    Unknown,
}

impl FailureClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CodeRegression => "code_regression",
            Self::StaleBase => "stale_base",
            Self::Unknown => "unknown",
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(receipt_path: Option<PathBuf>) -> color_eyre::eyre::Result<()> {
    let path = resolve_receipt_path(receipt_path.as_deref());
    let out = match load_receipt(&path) {
        Ok(r) => format_explanation(&explain(&r)),
        Err(e) => format_load_error(&e),
    };
    print!("{out}");
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Compute the path to the gate receipt JSON (pure — no I/O).
///
/// Resolution order:
/// 1. If `explicit` is `Some`, return that path directly.
/// 2. Otherwise return `target/receipts/receipt.json`.
fn resolve_receipt_path(explicit: Option<&Path>) -> PathBuf {
    explicit.map_or_else(|| PathBuf::from("target/receipts/receipt.json"), Path::to_path_buf)
}

fn load_receipt(path: &Path) -> std::result::Result<Receipt, ReceiptLoadError> {
    let raw = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ReceiptLoadError::Absent
        } else {
            ReceiptLoadError::Malformed(format!("reading {}: {e}", path.display()))
        }
    })?;
    let receipt: Receipt = serde_json::from_str(&raw)
        .map_err(|e| ReceiptLoadError::Malformed(format!("parsing {}: {e}", path.display())))?;
    // Validate schema version when present; an absent field is treated as compatible
    // (older receipts without the field are still readable).
    if let Some(ref ver) = receipt.schema_version {
        if ver != SUPPORTED_SCHEMA_VERSION {
            return Err(ReceiptLoadError::UnsupportedSchema(ver.clone()));
        }
    }
    Ok(receipt)
}

/// Find the first required gate that failed.
fn find_blocking_gate(receipt: &Receipt) -> Option<&GateResult> {
    receipt
        .gates
        .iter()
        .find(|gate| gate.required.unwrap_or(true) && is_failing_status(&gate.status))
}

fn is_failing_status(status: &str) -> bool {
    matches!(status, "fail" | "timeout" | "error")
}

/// Derive a failure class from available evidence.
///
/// Resolution order (mirrors the failure_classifier heuristics, but without
/// live CI data — we reason from the local receipt only):
///
/// 1. If `output_summary` mentions "stale" or "behind master" → `stale_base`.
/// 2. If the blocking gate has a `first_failure.site` → `code_regression`.
/// 3. If the gate is a lint/fmt/clippy gate with an `output_summary` → `code_regression`.
/// 4. Else → `unknown`.
///
/// Note: `master_red` is not emitted here — base-branch comparison needs a
/// base receipt and is tracked in #2653.
fn classify_failure(blocking: &GateResult) -> FailureClass {
    // If output_summary mentions "master" or "stale" → stale_base hint.
    if let Some(ref summary) = blocking.output_summary {
        let low = summary.to_lowercase();
        if low.contains("stale") || low.contains("behind master") {
            return FailureClass::StaleBase;
        }
    }

    // first_failure.site exists → most likely a code regression.
    if blocking.first_failure.as_ref().and_then(|f| f.site.as_ref()).is_some() {
        return FailureClass::CodeRegression;
    }

    // fmt / lint gates — output_summary present → regression.
    let gate = blocking.gate_name.to_lowercase();
    if (gate.contains("fmt") || gate.contains("lint") || gate.contains("clippy"))
        && blocking.output_summary.is_some()
    {
        return FailureClass::CodeRegression;
    }

    FailureClass::Unknown
}

/// Extract a `file:line` string from the first failure.
fn extract_source_file_line(gate: &GateResult) -> Option<String> {
    // Prefer first_failure.site if available.
    if let Some(site) = gate.first_failure.as_ref().and_then(|f| f.site.as_ref()) {
        return Some(normalize_site(site));
    }

    // Fall back: scan output_summary for a `src/...rs:N` pattern.
    if let Some(ref summary) = gate.output_summary {
        if let Some(site) = extract_site_from_text(summary) {
            return Some(site);
        }
    }

    None
}

/// Normalize a panic site: strip trailing `:col` component, convert `\` to `/`.
fn normalize_site(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    // Pattern: path:line:col → path:line
    let parts: Vec<&str> = normalized.splitn(3, ':').collect();
    // Handle Windows paths like C:/foo/bar:12:5
    // After replace, a Windows abs path starts with a drive letter + '/'
    if parts.len() == 3 {
        // Guard: if parts[1] is entirely digits, we have path:line:col
        if parts[2].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
        {
            return format!("{}:{}", parts[0], parts[1]);
        }
    }
    normalized
}

/// Scan free-form text for a `something.rs:NNN` pattern.
fn extract_site_from_text(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        // Look for word containing .rs:NNN
        if let Some(pos) = word.find(".rs:") {
            // candidate includes the ".rs" suffix (pos + 3 chars: '.', 'r', 's')
            let candidate = &word[..pos + 3];
            // rest is everything after ".rs:" — take leading digits as line number
            let rest = &word[pos + 4..];
            let line_part: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !line_part.is_empty() {
                // Strip any leading punctuation that attached to the token (e.g. from `(path.rs:N)`)
                let path = candidate.trim_start_matches(['\'', '"', '(', '[']);
                // candidate already ends in ".rs" — just append ":line"
                return Some(format!("{path}:{line_part}"));
            }
        }
    }
    None
}

fn build_repro_command(gate: &GateResult) -> Option<String> {
    gate.command.as_ref().map(|cmd| {
        // Prefer the repro hint from output_summary if present
        // (gates sometimes embed a "repro:" line in the summary).
        if let Some(ref summary) = gate.output_summary {
            for line in summary.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("repro:") {
                    return rest.trim().to_string();
                }
                if let Some(rest) = trimmed.strip_prefix("reproduce:") {
                    return rest.trim().to_string();
                }
            }
        }
        cmd.clone()
    })
}

fn explain(receipt: &Receipt) -> ExplainReceipt {
    let blocking = find_blocking_gate(receipt);

    let Some(gate) = blocking else {
        return ExplainReceipt {
            blocking_check_name: None,
            failure_class: "none".to_string(),
            source_file_line: None,
            exists_on_base: "unknown".to_string(), // see #2653
            local_reproduction_command: None,
        };
    };

    let class = classify_failure(gate);
    let source_file_line = extract_source_file_line(gate);
    let repro = build_repro_command(gate);

    ExplainReceipt {
        blocking_check_name: Some(gate.gate_name.clone()),
        failure_class: class.as_str().to_string(),
        source_file_line,
        exists_on_base: "unknown".to_string(), // see #2653
        local_reproduction_command: repro,
    }
}

/// Format a receipt load error as an inconclusive message (pure — no I/O).
fn format_load_error(err: &ReceiptLoadError) -> String {
    match err {
        ReceiptLoadError::Absent => {
            "inconclusive: no receipts; run `cargo xtask gates`\n".to_string()
        }
        ReceiptLoadError::Malformed(msg) => {
            format!("inconclusive: receipt is malformed — {msg}\n")
        }
        ReceiptLoadError::UnsupportedSchema(ver) => format!(
            "inconclusive: unsupported receipt schema \"{ver}\" (expected \"{SUPPORTED_SCHEMA_VERSION}\"); upgrade xtask\n"
        ),
    }
}

/// Format an explanation as a human-readable string (pure — no I/O).
fn format_explanation(explanation: &ExplainReceipt) -> String {
    if explanation.blocking_check_name.is_none() {
        return "All gates passing\n".to_string();
    }

    let mut out = String::new();
    writeln!(
        out,
        "blocking_check:   {}",
        explanation.blocking_check_name.as_deref().unwrap_or("-")
    )
    .ok();
    writeln!(out, "failure_class:    {}", explanation.failure_class).ok();
    writeln!(out, "source_file_line: {}", explanation.source_file_line.as_deref().unwrap_or("-"))
        .ok();
    writeln!(out, "exists_on_base:   {}", explanation.exists_on_base).ok();
    writeln!(
        out,
        "reproduce:        {}",
        explanation.local_reproduction_command.as_deref().unwrap_or("-")
    )
    .ok();
    out
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gate(name: &str, status: &str, required: bool) -> GateResult {
        GateResult {
            gate_name: name.to_string(),
            status: status.to_string(),
            required: Some(required),
            command: Some(format!("cargo xtask {name}")),
            first_failure: None,
            output_summary: None,
        }
    }

    fn make_gate_with_failure(name: &str, site: Option<&str>, message: Option<&str>) -> GateResult {
        GateResult {
            gate_name: name.to_string(),
            status: "fail".to_string(),
            required: Some(true),
            command: Some(format!("cargo xtask {name}")),
            first_failure: Some(FirstFailure {
                site: site.map(ToString::to_string),
                test: None,
                message: message.map(ToString::to_string),
            }),
            output_summary: None,
        }
    }

    fn make_receipt(gates: Vec<GateResult>) -> Receipt {
        Receipt { gates, ..Receipt::default() }
    }

    // ── resolve_receipt_path ─────────────────────────────────────────────────

    #[test]
    fn resolve_receipt_path_explicit_wins() {
        let explicit = PathBuf::from("my/custom/receipt.json");
        let result = resolve_receipt_path(Some(&explicit));
        assert_eq!(result, explicit);
    }

    #[test]
    fn resolve_receipt_path_default_when_none() {
        let result = resolve_receipt_path(None);
        assert_eq!(result, PathBuf::from("target/receipts/receipt.json"));
    }

    // ── load_receipt error variants ──────────────────────────────────────────

    #[test]
    fn load_receipt_absent_file_returns_absent_error() {
        let path = PathBuf::from("target/receipts/nonexistent-receipt-ci-explain-test.json");
        let result = load_receipt(&path);
        assert!(matches!(result, Err(ReceiptLoadError::Absent)));
    }

    #[test]
    fn load_receipt_malformed_json_returns_malformed_error() {
        use std::fs;
        use tempfile::TempDir;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("bad.json");
        fs::write(&path, b"not valid json").expect("write");
        let result = load_receipt(&path);
        assert!(matches!(result, Err(ReceiptLoadError::Malformed(_))));
    }

    #[test]
    fn load_receipt_unsupported_schema_returns_error() {
        use std::fs;
        use tempfile::TempDir;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("receipt.json");
        fs::write(&path, br#"{"schema_version":"gates.v99","gates":[]}"#).expect("write");
        let result = load_receipt(&path);
        assert!(matches!(result, Err(ReceiptLoadError::UnsupportedSchema(_))));
    }

    #[test]
    fn load_receipt_absent_schema_version_is_accepted() {
        use std::fs;
        use tempfile::TempDir;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("receipt.json");
        // Older receipts without schema_version must still parse successfully.
        fs::write(&path, br#"{"gates":[]}"#).expect("write");
        let result = load_receipt(&path);
        assert!(result.is_ok());
    }

    // ── find_blocking_gate ───────────────────────────────────────────────────

    #[test]
    fn find_blocking_gate_picks_first_required_fail() {
        let receipt = make_receipt(vec![
            make_gate("lint", "pass", true),
            make_gate("test", "fail", true),
            make_gate("fmt", "fail", false), // not required
        ]);
        let blocking = find_blocking_gate(&receipt);
        assert!(blocking.is_some());
        assert_eq!(blocking.map(|g| &g.gate_name), Some(&"test".to_string()));
    }

    #[test]
    fn find_blocking_gate_returns_none_when_all_pass() {
        let receipt =
            make_receipt(vec![make_gate("lint", "pass", true), make_gate("test", "pass", true)]);
        assert!(find_blocking_gate(&receipt).is_none());
    }

    #[test]
    fn find_blocking_gate_skips_non_required_failures() {
        let receipt = make_receipt(vec![
            make_gate("optional", "fail", false),
            make_gate("required", "pass", true),
        ]);
        assert!(find_blocking_gate(&receipt).is_none());
    }

    #[test]
    fn find_blocking_gate_treats_timeout_as_blocking() {
        let receipt = make_receipt(vec![make_gate("test", "timeout", true)]);
        assert!(find_blocking_gate(&receipt).is_some());
    }

    #[test]
    fn find_blocking_gate_treats_error_as_blocking() {
        let receipt = make_receipt(vec![make_gate("test", "error", true)]);
        assert!(find_blocking_gate(&receipt).is_some());
    }

    // ── classify_failure ─────────────────────────────────────────────────────

    #[test]
    fn classify_code_regression_when_first_failure_site_present() {
        let gate = make_gate_with_failure("test", Some("crates/foo/src/lib.rs:42"), None);
        let class = classify_failure(&gate);
        assert_eq!(class, FailureClass::CodeRegression);
    }

    #[test]
    fn classify_code_regression_for_fmt_gate_with_output() {
        let mut gate = make_gate("fmt", "fail", true);
        gate.output_summary = Some("diff detected in src/main.rs".to_string());
        let class = classify_failure(&gate);
        assert_eq!(class, FailureClass::CodeRegression);
    }

    #[test]
    fn classify_code_regression_for_clippy_gate_with_output() {
        let mut gate = make_gate("clippy", "fail", true);
        gate.output_summary = Some("error: unused variable".to_string());
        let class = classify_failure(&gate);
        assert_eq!(class, FailureClass::CodeRegression);
    }

    #[test]
    fn classify_unknown_without_evidence() {
        let gate = make_gate("test", "fail", true);
        let class = classify_failure(&gate);
        assert_eq!(class, FailureClass::Unknown);
    }

    #[test]
    fn classify_stale_base_from_output_summary() {
        let mut gate = make_gate("test", "fail", true);
        gate.output_summary = Some("PR is stale and behind master".to_string());
        let class = classify_failure(&gate);
        assert_eq!(class, FailureClass::StaleBase);
    }

    // ── extract_source_file_line ─────────────────────────────────────────────

    #[test]
    fn extract_from_first_failure_site() {
        let gate = make_gate_with_failure("test", Some("crates/foo/src/lib.rs:42"), None);
        let result = extract_source_file_line(&gate);
        assert_eq!(result, Some("crates/foo/src/lib.rs:42".to_string()));
    }

    #[test]
    fn extract_strips_column_from_site() {
        let gate = make_gate_with_failure("test", Some("crates/foo/src/lib.rs:42:7"), None);
        let result = extract_source_file_line(&gate);
        // "crates/foo/src/lib.rs:42:7" → parts = ["crates/foo/src/lib.rs", "42", "7"]
        // Both parts are all-digit → format "path:line" = "crates/foo/src/lib.rs:42"
        assert_eq!(result, Some("crates/foo/src/lib.rs:42".to_string()));
    }

    #[test]
    fn extract_from_output_summary_fallback() {
        let mut gate = make_gate("fmt", "fail", true);
        gate.output_summary =
            Some("error in xtask/src/tasks/ci_explain.rs:12 formatting".to_string());
        let result = extract_source_file_line(&gate);
        assert_eq!(result, Some("xtask/src/tasks/ci_explain.rs:12".to_string()));
    }

    #[test]
    fn extract_returns_none_when_no_evidence() {
        let gate = make_gate("test", "fail", true);
        let result = extract_source_file_line(&gate);
        assert!(result.is_none());
    }

    // ── normalize_site ───────────────────────────────────────────────────────

    #[test]
    fn normalize_site_backslashes() {
        let result = normalize_site("crates\\foo\\src\\lib.rs:42:7");
        assert_eq!(result, "crates/foo/src/lib.rs:42");
    }

    #[test]
    fn normalize_site_no_column() {
        let result = normalize_site("crates/foo/src/lib.rs:42");
        assert_eq!(result, "crates/foo/src/lib.rs:42");
    }

    // ── build_repro_command ──────────────────────────────────────────────────

    #[test]
    fn repro_command_returns_gate_command() {
        let gate = make_gate("fmt", "fail", true);
        let repro = build_repro_command(&gate);
        assert_eq!(repro, Some("cargo xtask fmt".to_string()));
    }

    #[test]
    fn repro_command_prefers_repro_line_in_summary() {
        let mut gate = make_gate("test", "fail", true);
        gate.output_summary =
            Some("gate failed\nrepro: cargo test -p perl-parser -- my_test\n".to_string());
        let repro = build_repro_command(&gate);
        assert_eq!(repro, Some("cargo test -p perl-parser -- my_test".to_string()));
    }

    #[test]
    fn repro_command_none_when_no_command() {
        let gate = GateResult {
            gate_name: "test".to_string(),
            status: "fail".to_string(),
            required: Some(true),
            command: None,
            first_failure: None,
            output_summary: None,
        };
        assert!(build_repro_command(&gate).is_none());
    }

    // ── explain (integration) ────────────────────────────────────────────────

    #[test]
    fn explain_all_passing_returns_none_blocking() {
        let receipt = make_receipt(vec![make_gate("lint", "pass", true)]);
        let result = explain(&receipt);
        assert!(result.blocking_check_name.is_none());
        assert_eq!(result.failure_class, "none");
        assert_eq!(result.exists_on_base, "unknown");
    }

    #[test]
    fn explain_failing_gate_produces_blocking_check_name() {
        let receipt = make_receipt(vec![make_gate_with_failure(
            "test",
            Some("crates/foo/src/lib.rs:10"),
            None,
        )]);
        let result = explain(&receipt);
        assert_eq!(result.blocking_check_name, Some("test".to_string()));
        assert_eq!(result.failure_class, "code_regression");
        assert_eq!(result.source_file_line, Some("crates/foo/src/lib.rs:10".to_string()));
        assert_eq!(result.exists_on_base, "unknown");
    }

    // ── format_explanation ───────────────────────────────────────────────────

    #[test]
    fn format_explanation_all_passing() {
        let receipt = make_receipt(vec![make_gate("lint", "pass", true)]);
        let explanation = explain(&receipt);
        let output = format_explanation(&explanation);
        assert_eq!(output, "All gates passing\n");
    }

    #[test]
    fn format_explanation_blocking_with_site() {
        let receipt = make_receipt(vec![make_gate_with_failure(
            "fmt",
            Some("xtask/src/tasks/ci_explain.rs:42"),
            None,
        )]);
        let explanation = explain(&receipt);
        let output = format_explanation(&explanation);
        assert!(output.contains("blocking_check:   fmt\n"));
        assert!(output.contains("failure_class:    code_regression\n"));
        assert!(output.contains("source_file_line: xtask/src/tasks/ci_explain.rs:42\n"));
        assert!(output.contains("exists_on_base:   unknown\n"));
        assert!(output.contains("reproduce:        cargo xtask fmt\n"));
    }

    #[test]
    fn format_explanation_blocking_without_site() {
        let receipt = make_receipt(vec![make_gate("test", "fail", true)]);
        let explanation = explain(&receipt);
        let output = format_explanation(&explanation);
        assert!(output.contains("blocking_check:   test\n"));
        assert!(output.contains("failure_class:    unknown\n"));
        assert!(output.contains("source_file_line: -\n"));
        assert!(output.contains("exists_on_base:   unknown\n"));
        assert!(output.contains("reproduce:        cargo xtask test\n"));
    }

    #[test]
    fn format_explanation_stale_base_class() {
        let mut gate = make_gate("test", "fail", true);
        gate.output_summary = Some("PR is stale and behind master".to_string());
        let receipt = make_receipt(vec![gate]);
        let explanation = explain(&receipt);
        let output = format_explanation(&explanation);
        assert!(output.contains("failure_class:    stale_base\n"));
    }

    // ── format_load_error ────────────────────────────────────────────────────

    #[test]
    fn format_load_error_absent() {
        let output = format_load_error(&ReceiptLoadError::Absent);
        assert_eq!(output, "inconclusive: no receipts; run `cargo xtask gates`\n");
    }

    #[test]
    fn format_load_error_malformed() {
        let output =
            format_load_error(&ReceiptLoadError::Malformed("bad json at line 3".to_string()));
        assert_eq!(output, "inconclusive: receipt is malformed — bad json at line 3\n");
    }

    #[test]
    fn format_load_error_unsupported_schema() {
        let output =
            format_load_error(&ReceiptLoadError::UnsupportedSchema("gates.v99".to_string()));
        assert_eq!(
            output,
            "inconclusive: unsupported receipt schema \"gates.v99\" (expected \"gates.v1\"); upgrade xtask\n"
        );
    }
}
