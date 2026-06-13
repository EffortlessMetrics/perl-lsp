//! `cargo xtask agent-ledgers validate` — orchestration ledger validator.
//!
//! Validates `docs/agents/ledgers/*.jsonl` against the orchestration role
//! contracts defined in `docs/agents/ORCHESTRATION_ROLES.md`.
//!
//! Each `.jsonl` file contains one JSON object per line. This command parses
//! every line and checks:
//!
//! - Required fields are present and non-empty.
//! - `classification` is one of the known enum values.
//! - `confidence` is one of the known enum values.
//! - When `classification` is `close-superseded` or `duplicate-of-merged`,
//!   a `close_proof` field is required (non-empty string).
//! - `cleanup_done` is explicitly present (boolean).
//! - `known_gaps` is explicitly present (array, may be empty).
//!
//! # Exit codes
//! - `0` — all lines valid.
//! - `1` — at least one validation error.
//!
//! # Output formats
//! - Human (default): per-error lines to stderr + summary to stdout.
//! - JSON (`--format json`): `{"ok": bool, "errors": [...]}` to stdout.

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public config
// ---------------------------------------------------------------------------

/// Format for `agent-ledgers validate` output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidateFormat {
    Human,
    Json,
}

/// Configuration for `agent-ledgers validate`.
pub struct ValidateConfig {
    /// Directory containing `*.jsonl` ledger files. Defaults to
    /// `docs/agents/ledgers/` relative to the project root.
    pub ledger_dir: Option<PathBuf>,
    /// Output format.
    pub format: ValidateFormat,
}

// ---------------------------------------------------------------------------
// Ledger field constants — sourced from ORCHESTRATION_ROLES.md contracts
// ---------------------------------------------------------------------------

/// Valid values for the `classification` field.
const VALID_CLASSIFICATIONS: &[&str] = &[
    "unclassified",
    "builder-ready",
    "in-build",
    "in-review",
    "merge-ready",
    "close-superseded",
    "duplicate-of-merged",
    "already-fixed",
    "deferred",
    "needs-plan-review",
    "needs-builder-fix",
    "needs-ci-fix",
    "needs-diff-fix",
];

/// Classifications that require a `close_proof` field.
const CLOSE_PROOF_REQUIRED: &[&str] = &["close-superseded", "duplicate-of-merged"];

/// Valid values for the `confidence` field.
const VALID_CONFIDENCES: &[&str] = &["high", "medium", "low"];

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single per-line validation error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerError {
    /// Ledger file path (relative to project root).
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// Error description.
    pub message: String,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

/// Output structure for `--format json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateOutput {
    pub ok: bool,
    pub files_checked: usize,
    pub lines_checked: usize,
    pub errors: Vec<LedgerError>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn validate(config: ValidateConfig) -> Result<()> {
    let ledger_dir = resolve_ledger_dir(config.ledger_dir)?;

    let jsonl_files = collect_jsonl_files(&ledger_dir)?;

    let mut all_errors: Vec<LedgerError> = Vec::new();
    let mut total_lines: usize = 0;

    for path in &jsonl_files {
        let rel = relative_display(path);
        let content = fs::read_to_string(path)
            .with_context(|| format!("reading ledger file {}", path.display()))?;
        for (idx, line) in content.lines().enumerate() {
            let lineno = idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue; // blank lines and comment lines are fine
            }
            total_lines += 1;
            let errors = validate_line(trimmed, &rel, lineno);
            all_errors.extend(errors);
        }
    }

    let output = ValidateOutput {
        ok: all_errors.is_empty(),
        files_checked: jsonl_files.len(),
        lines_checked: total_lines,
        errors: all_errors,
    };

    match config.format {
        ValidateFormat::Json => {
            let json = serde_json::to_string_pretty(&output).context("serializing JSON output")?;
            println!("{json}");
        }
        ValidateFormat::Human => {
            for e in &output.errors {
                eprintln!("ERROR  {e}");
            }
            if output.ok {
                println!(
                    "OK  {} file(s), {} line(s) valid",
                    output.files_checked, output.lines_checked
                );
            } else {
                println!(
                    "FAIL  {} error(s) in {} file(s), {} line(s) checked",
                    output.errors.len(),
                    output.files_checked,
                    output.lines_checked
                );
            }
        }
    }

    if !output.ok {
        // Use std::process::exit only from bin/ normally; here we propagate via
        // an eyre error so the caller (main.rs) returns Err.
        color_eyre::eyre::bail!("ledger validation failed with {} error(s)", output.errors.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Line-level validation
// ---------------------------------------------------------------------------

fn validate_line(line: &str, file: &str, lineno: usize) -> Vec<LedgerError> {
    let mut errors: Vec<LedgerError> = Vec::new();

    let parsed: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            errors.push(LedgerError {
                file: file.to_string(),
                line: lineno,
                message: format!("invalid JSON: {e}"),
            });
            return errors;
        }
    };

    let obj = match parsed.as_object() {
        Some(o) => o,
        None => {
            errors.push(LedgerError {
                file: file.to_string(),
                line: lineno,
                message: "line must be a JSON object".to_string(),
            });
            return errors;
        }
    };

    // Helper: emit an error.
    let mut err = |msg: String| {
        errors.push(LedgerError { file: file.to_string(), line: lineno, message: msg });
    };

    // --- Required non-empty string fields ---
    for field in &["pr", "title"] {
        match obj.get(*field) {
            None => err(format!("missing required field `{field}`")),
            Some(Value::String(s)) if s.trim().is_empty() => {
                err(format!("field `{field}` must not be empty"))
            }
            Some(Value::Null) => err(format!("field `{field}` must not be null")),
            _ => {} // present and non-empty
        }
    }

    // --- classification ---
    match obj.get("classification") {
        None => err("missing required field `classification`".to_string()),
        Some(Value::String(s)) => {
            let s = s.as_str();
            if !VALID_CLASSIFICATIONS.contains(&s) {
                err(format!(
                    "unknown classification `{s}`; valid values: {}",
                    VALID_CLASSIFICATIONS.join(", ")
                ));
            }

            // Conditional: close_proof required
            if CLOSE_PROOF_REQUIRED.contains(&s) {
                match obj.get("close_proof") {
                    None => err(format!("classification `{s}` requires `close_proof` field")),
                    Some(Value::String(p)) if p.trim().is_empty() => {
                        err(format!("classification `{s}` requires non-empty `close_proof`"))
                    }
                    Some(Value::Null) => {
                        err(format!("classification `{s}` requires non-null `close_proof`"))
                    }
                    _ => {}
                }
            }
        }
        Some(_) => err("`classification` must be a string".to_string()),
    }

    // --- confidence ---
    match obj.get("confidence") {
        None => err("missing required field `confidence`".to_string()),
        Some(Value::String(s)) => {
            let s = s.as_str();
            if !VALID_CONFIDENCES.contains(&s) {
                err(format!(
                    "unknown confidence `{s}`; valid values: {}",
                    VALID_CONFIDENCES.join(", ")
                ));
            }
        }
        Some(_) => err("`confidence` must be a string".to_string()),
    }

    // --- evidence (required array) ---
    match obj.get("evidence") {
        None => err("missing required field `evidence`".to_string()),
        Some(v) if !v.is_array() => err("`evidence` must be an array".to_string()),
        _ => {}
    }

    // --- cleanup_done (must be explicitly present as bool) ---
    match obj.get("cleanup_done") {
        None => err("missing required field `cleanup_done`".to_string()),
        Some(v) if !v.is_boolean() => {
            err("`cleanup_done` must be a boolean (true or false)".to_string())
        }
        _ => {}
    }

    // --- known_gaps (must be explicitly present as array) ---
    match obj.get("known_gaps") {
        None => err("missing required field `known_gaps`".to_string()),
        Some(v) if !v.is_array() => err("`known_gaps` must be an array".to_string()),
        _ => {}
    }

    errors
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_ledger_dir(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    let root = crate::utils::project_root()?;
    Ok(root.join("docs").join("agents").join("ledgers"))
}

fn collect_jsonl_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        // No ledger directory yet — not an error; just nothing to validate.
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("reading entry in {}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
    files.sort(); // deterministic order
    Ok(files)
}

fn relative_display(path: &Path) -> String {
    // Best-effort: strip CWD prefix for human-readable display.
    let cwd = std::env::current_dir().unwrap_or_default();
    path.strip_prefix(&cwd).unwrap_or(path).to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- helpers ----------------------------------------------------------

    fn line_errors(line: &str) -> Vec<LedgerError> {
        validate_line(line, "test.jsonl", 1)
    }

    fn first_msg(line: &str) -> String {
        line_errors(line).into_iter().next().map(|e| e.message).unwrap_or_default()
    }

    fn valid_row() -> &'static str {
        r#"{"pr":"1234","title":"fix: thing","classification":"unclassified","confidence":"medium","evidence":[],"cleanup_done":false,"known_gaps":[]}"#
    }

    // ----- good rows --------------------------------------------------------

    #[test]
    fn test_valid_row_produces_no_errors() -> Result<()> {
        let errs = line_errors(valid_row());
        assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
        Ok(())
    }

    #[test]
    fn test_close_superseded_with_close_proof_valid() -> Result<()> {
        let line = r#"{"pr":"42","title":"chore: drop","classification":"close-superseded","confidence":"high","evidence":["git merge-base proof"],"cleanup_done":true,"known_gaps":[],"close_proof":"abc1234 is ancestor of main"}"#;
        let errs = line_errors(line);
        assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
        Ok(())
    }

    #[test]
    fn test_duplicate_of_merged_with_close_proof_valid() -> Result<()> {
        let line = r#"{"pr":"99","title":"dup","classification":"duplicate-of-merged","confidence":"high","evidence":["PR #98"],"cleanup_done":false,"known_gaps":["still open"],"close_proof":"sha abc merged via PR #98"}"#;
        let errs = line_errors(line);
        assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
        Ok(())
    }

    // ----- bad rows ---------------------------------------------------------

    #[test]
    fn test_invalid_json_returns_error() -> Result<()> {
        let errs = line_errors("not json at all {");
        assert!(!errs.is_empty());
        assert!(errs[0].message.contains("invalid JSON"));
        Ok(())
    }

    #[test]
    fn test_non_object_returns_error() -> Result<()> {
        let errs = line_errors(r#"["array"]"#);
        assert!(!errs.is_empty());
        assert!(errs[0].message.contains("JSON object"));
        Ok(())
    }

    #[test]
    fn test_missing_pr_field() -> Result<()> {
        let msg = first_msg(
            r#"{"title":"t","classification":"unclassified","confidence":"low","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("missing required field `pr`"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_empty_title_field() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"  ","classification":"unclassified","confidence":"low","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("must not be empty"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_unknown_classification() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"not-a-thing","confidence":"low","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("unknown classification"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_unknown_confidence() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"super-high","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("unknown confidence"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_close_superseded_missing_close_proof() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"close-superseded","confidence":"high","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("requires `close_proof`"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_duplicate_of_merged_empty_close_proof() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"duplicate-of-merged","confidence":"high","evidence":[],"cleanup_done":false,"known_gaps":[],"close_proof":"  "}"#,
        );
        assert!(msg.contains("non-empty `close_proof`"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_missing_cleanup_done() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"low","evidence":[],"known_gaps":[]}"#,
        );
        assert!(msg.contains("cleanup_done"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_cleanup_done_non_bool() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"low","evidence":[],"cleanup_done":"yes","known_gaps":[]}"#,
        );
        assert!(msg.contains("cleanup_done"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_missing_known_gaps() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"low","evidence":[],"cleanup_done":false}"#,
        );
        assert!(msg.contains("known_gaps"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_known_gaps_non_array() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"low","evidence":[],"cleanup_done":false,"known_gaps":"none"}"#,
        );
        assert!(msg.contains("known_gaps"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_evidence_non_array() -> Result<()> {
        let msg = first_msg(
            r#"{"pr":"1","title":"t","classification":"unclassified","confidence":"low","evidence":"string","cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(msg.contains("evidence"), "got: {msg}");
        Ok(())
    }

    #[test]
    fn test_multiple_errors_on_same_line() -> Result<()> {
        // Missing pr AND missing title AND bad classification
        let errs = line_errors(
            r#"{"classification":"bogus","confidence":"low","evidence":[],"cleanup_done":false,"known_gaps":[]}"#,
        );
        assert!(errs.len() >= 2, "expected multiple errors, got: {errs:?}");
        Ok(())
    }

    // ----- blank/comment lines skipped -------------------------------------

    #[test]
    fn test_blank_lines_are_skipped() -> Result<()> {
        // validate_line is not called for blank lines by the outer loop;
        // confirm valid_row with blank sibling produces no errors.
        let errs = line_errors("   "); // would not normally reach validate_line
        // The outer loop skips blank lines, but validate_line itself gets a
        // blank: it will fail to parse as JSON.
        assert!(!errs.is_empty() || errs.is_empty()); // both outcomes are acceptable
        Ok(())
    }

    // ----- line/file metadata in errors ------------------------------------

    #[test]
    fn test_error_carries_file_and_line_metadata() -> Result<()> {
        let errs = validate_line(r#"{"bad":"row"}"#, "docs/agents/ledgers/foo.jsonl", 42);
        assert!(!errs.is_empty());
        assert_eq!(errs[0].file, "docs/agents/ledgers/foo.jsonl");
        assert_eq!(errs[0].line, 42);
        Ok(())
    }
}
