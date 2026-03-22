//! Tests for --check-format flag on the perl-lsp binary.
//!
//! Covers JSON output mode, the recovered-errors bug fix,
//! and regression coverage for text mode.
//!
//! Run with:
//!   RUST_TEST_THREADS=2 cargo test -p perl-lsp --test check_format_tests -- --test-threads=2

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a temporary Perl file with the given content and return its path as
/// an owned String.  The `TempDir` is returned to keep it alive.
fn write_temp_perl(
    content: &str,
) -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("test.pl");
    std::fs::write(&file, content)?;
    let path = file.to_str().ok_or("non-UTF-8 temp path")?.to_string();
    Ok((dir, path))
}

// ---------------------------------------------------------------------------
// JSON format — basic structure
// ---------------------------------------------------------------------------

/// JSON output for a valid file must be parseable and contain "ok" status.
#[test]
fn test_check_json_clean_file() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\nuse warnings;\nprint \"hello\\n\";\n")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    assert!(output.status.success(), "expected exit 0 for clean file");
    let stdout = String::from_utf8(output.stdout)?;
    // Must be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    // Top-level must have "files" array and "summary"
    assert!(parsed.get("files").is_some(), "JSON must have 'files' key");
    assert!(parsed.get("summary").is_some(), "JSON must have 'summary' key");

    // The single file entry must have status "ok"
    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    assert_eq!(files.len(), 1, "expected exactly one file entry");
    assert_eq!(files[0]["status"], "ok", "expected status 'ok' for clean file");

    // Summary counts
    let summary = &parsed["summary"];
    assert_eq!(summary["total"], 1);
    assert_eq!(summary["ok"], 1);
    assert_eq!(summary["fail"], 0);

    Ok(())
}

/// JSON output for a file with parse errors must include errors with line/col.
#[test]
fn test_check_json_error_file() -> Result<(), Box<dyn std::error::Error>> {
    // Deliberately malformed Perl that the parser should reject.
    let (_dir, path) = write_temp_perl("sub foo { \n    my $x = ;\n}\n")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    // Exit code must be non-zero for a file with errors
    assert!(!output.status.success(), "expected non-zero exit for parse error");
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    assert_eq!(files.len(), 1);
    let entry = &files[0];

    // Status must be "fail" for a file with errors
    assert_eq!(entry["status"], "fail", "expected status 'fail' for errored file");

    // Errors array must be present and non-empty
    let errors = entry["errors"].as_array().ok_or("'errors' is not an array")?;
    assert!(!errors.is_empty(), "errors array must be non-empty for a failing file");

    // Each error must have a "message" field
    for err in errors {
        assert!(err.get("message").is_some(), "each error must have a 'message' field");
    }

    // Summary must reflect the failure
    assert_eq!(parsed["summary"]["fail"], 1);
    assert_eq!(parsed["summary"]["ok"], 0);

    Ok(())
}

/// IO error (nonexistent file) should produce status "error" in JSON output.
#[test]
fn test_check_json_io_error() -> Result<(), Box<dyn std::error::Error>> {
    // Use a path inside a fresh tempdir that is dropped immediately, guaranteeing
    // it does not exist on disk (more robust than a hardcoded /tmp sentinel).
    let dir = tempfile::tempdir()?;
    let nonexistent = dir.path().join("does_not_exist.pl");
    drop(dir); // directory is now deleted; the path is guaranteed absent
    let nonexistent = nonexistent.to_str().ok_or("non-UTF-8 temp path")?.to_string();

    let output = cargo_bin_cmd!("perl-lsp")
        .args(["--check", "--check-format", "json", &nonexistent])
        .output()?;

    // Non-zero exit code
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    assert_eq!(files.len(), 1);
    // IO errors get status "error" (distinct from parse "fail")
    assert_eq!(files[0]["status"], "error", "IO failure should have status 'error'");

    let summary = &parsed["summary"];
    assert_eq!(summary["error"], 1);
    assert_eq!(summary["ok"], 0);
    assert_eq!(summary["fail"], 0);

    Ok(())
}

/// --check-format json must emit valid JSON even for an empty-looking parse result.
#[test]
fn test_check_json_valid_structure_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir1, path1) = write_temp_perl("use strict;\nprint 1;\n")?;
    let (_dir2, path2) = write_temp_perl("use warnings;\nprint 2;\n")?;

    let output = cargo_bin_cmd!("perl-lsp")
        .args(["--check", "--check-format", "json", &path1, &path2])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    // Must parse as JSON without error
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    assert_eq!(files.len(), 2, "expected 2 file entries");
    assert_eq!(parsed["summary"]["total"], 2);
    assert_eq!(parsed["summary"]["ok"], 2);

    Ok(())
}

/// JSON output must include the schema version field.
#[test]
fn test_check_json_has_version() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\n")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    assert!(parsed.get("version").is_some(), "JSON output must have 'version' field");
    assert_eq!(parsed["version"], 1, "version must be 1");

    Ok(())
}

// ---------------------------------------------------------------------------
// JSON format — path field
// ---------------------------------------------------------------------------

/// The "path" field in each file entry must match the input path.
#[test]
fn test_check_json_file_path_matches() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\n")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    assert_eq!(files[0]["path"], path.as_str(), "path field must match input path");

    Ok(())
}

/// JSON output for an ok file must include "errors": [] for schema consistency.
///
/// CI tools iterating `.files[].errors` expect a uniform shape regardless of
/// status.  Absence of the key (null in jq) breaks `[.files[].errors | length]
/// | add` style queries.
#[test]
fn test_check_json_ok_entry_has_empty_errors_array() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\nprint 1;\n")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let files = parsed["files"].as_array().ok_or("'files' is not an array")?;
    let entry = &files[0];
    assert_eq!(entry["status"], "ok");

    // "errors" must be present as an empty array — not absent (null) — so that
    // consumers can uniformly iterate `.errors` without a null guard.
    let errors = entry["errors"].as_array().ok_or("'errors' must be an array on ok entries")?;
    assert!(errors.is_empty(), "ok entry must have empty errors array, got {errors:?}");

    Ok(())
}

// ---------------------------------------------------------------------------
// Text format regression — must be unchanged
// ---------------------------------------------------------------------------

/// Text mode (default) must still output "ok" for a clean file (regression guard).
#[test]
fn test_check_text_format_ok() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\nprint \"hello\\n\";\n")?;
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--check", "--check-format", "text", &path])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"));
    Ok(())
}

/// Text mode without explicit --check-format must still produce "ok" (default is text).
#[test]
fn test_check_default_format_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("use strict;\nprint \"hello\\n\";\n")?;
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--check", &path]).assert().success().stdout(predicate::str::contains("ok"));
    Ok(())
}

/// Text mode must still report FAIL for a file with errors (regression guard).
#[test]
fn test_check_text_format_fail() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("sub foo { \n    my $x = ;\n}\n")?;
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--check", "--check-format", "text", &path])
        .assert()
        .failure()
        .stdout(predicate::str::contains("FAIL"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Invalid --check-format value
// ---------------------------------------------------------------------------

/// Passing an unrecognized format value must fail with a helpful error message.
#[test]
fn test_check_invalid_format_value() {
    cargo_bin_cmd!("perl-lsp")
        .args(["--check", "--check-format", "xml", "somefile.pl"])
        .assert()
        .failure();
}

/// --check-format without --check must fail (requires constraint).
#[test]
fn test_check_format_without_check_fails() {
    cargo_bin_cmd!("perl-lsp").args(["--check-format", "json"]).assert().failure();
}

// ---------------------------------------------------------------------------
// Mixed ok/fail/error across multiple files
// ---------------------------------------------------------------------------

/// Summary counts must correctly reflect a mix of ok and fail files.
#[test]
fn test_check_json_mixed_results_summary() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir_ok, path_ok) = write_temp_perl("use strict;\nprint 1;\n")?;
    let (_dir_fail, path_fail) = write_temp_perl("sub foo { \n    my $x = ;\n}\n")?;
    // Guarantee a nonexistent path without relying on /tmp sentinel names.
    let gone_dir = tempfile::tempdir()?;
    let nonexistent = gone_dir.path().join("gone.pl");
    drop(gone_dir);
    let nonexistent = nonexistent.to_str().ok_or("non-UTF-8 temp path")?.to_string();

    let output = cargo_bin_cmd!("perl-lsp")
        .args(["--check", "--check-format", "json", &path_ok, &path_fail, &nonexistent])
        .output()?;

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    let summary = &parsed["summary"];
    assert_eq!(summary["total"], 3, "total must be 3");
    assert_eq!(summary["ok"], 1, "ok count must be 1");
    assert_eq!(summary["fail"], 1, "fail count must be 1");
    assert_eq!(summary["error"], 1, "error count must be 1");

    Ok(())
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

/// `--check --check-format json` with no file arguments must still emit valid
/// JSON (not empty stdout) so that `| jq` consumers don't crash on empty stdin.
#[test]
fn test_check_json_no_files_emits_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    // Pass --check with no trailing files.  clap's trailing_var_arg allows this.
    let output = cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json"]).output()?;

    // Exit must be non-zero (no files = usage error) …
    assert!(!output.status.success(), "expected non-zero exit when no files given");

    // … but stdout must still be parseable JSON with an empty files array.
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not valid JSON: {e}\nraw: {stdout}"))?;

    let files = parsed["files"].as_array().ok_or("'files' must be an array")?;
    assert!(files.is_empty(), "files array must be empty when no files were given");

    let summary = &parsed["summary"];
    assert_eq!(summary["total"], 0);
    assert_eq!(summary["ok"], 0);
    assert_eq!(summary["fail"], 0);
    assert_eq!(summary["error"], 0);

    Ok(())
}

/// `--check --check-format json` on a zero-byte (empty) file must produce
/// status "ok" — an empty Perl file is valid syntax.
#[test]
fn test_check_json_empty_file_is_ok() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, path) = write_temp_perl("")?;
    let output =
        cargo_bin_cmd!("perl-lsp").args(["--check", "--check-format", "json", &path]).output()?;

    assert!(output.status.success(), "expected exit 0 for empty (valid) file");
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let files = parsed["files"].as_array().ok_or("'files' must be an array")?;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["status"], "ok", "empty file must report 'ok'");
    assert_eq!(parsed["summary"]["ok"], 1);

    Ok(())
}
