//! Integration tests for `cargo xtask check-file-policy` and
//! `cargo xtask non-rust check` (issue #8566, PR 4 of the file-policy rollout).
//!
//! Each test creates an isolated temp directory simulating a minimal git repo
//! with a synthetic allowlist and one or more tracked files, then invokes the
//! xtask binary and asserts on exit codes and JSON output schema.

// These integration tests assert CLI process behavior; localized expect/unwrap
// calls keep fixture setup and JSON assertions readable.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

mod git_test_support;

use git_test_support::{add_and_commit, init_git_repo};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal but valid allowlist TOML string.
fn minimal_allowlist(entries: &str) -> String {
    format!(
        r#"schema_version = 1
policy = "non-rust-allowlist"
owner = "test"
status = "advisory"
updated = "2026-01-01"

[defaults]
rust_is_default = true
xtask_is_default_for_repo_automation = true
new_non_rust_requires_review = true
broad_globs_require_reason = true
coverage_required_for_production_surfaces = true

{entries}
"#
    )
}

/// Create a test workspace with:
///   - a minimal git repo
///   - `policy/non-rust-allowlist.toml` populated with `allowlist_content`
///   - one Rust file (`src/lib.rs`) and the provided extra files
///
/// Returns the `TempDir` (keep alive) and the repo root path.
fn setup_test_repo(
    allowlist_content: &str,
    extra_files: &[(&str, &str)],
) -> Result<(TempDir, PathBuf)> {
    let tmp = TempDir::new()?;
    let root = tmp.path().to_path_buf();

    init_git_repo(&root)?;

    // Commit Rust sentinel + extra files.
    let mut files: Vec<(&str, &str)> = vec![("src/lib.rs", "// Rust sentinel")];
    files.extend_from_slice(extra_files);
    add_and_commit(&root, &files, "initial")?;

    // Write allowlist (NOT committed — loaded via --allowlist flag so it
    // doesn't need to be tracked).
    let policy_dir = root.join("policy");
    fs::create_dir_all(&policy_dir)?;
    fs::write(policy_dir.join("non-rust-allowlist.toml"), allowlist_content)?;

    Ok((tmp, root))
}

/// Run `cargo xtask check-file-policy` scoped to a temp repo.
///
/// Passes both `--allowlist <root>/policy/non-rust-allowlist.toml` and the
/// hidden `--root <root>` seam so that `git ls-files` runs in the temp repo
/// rather than in the real workspace.
fn run_check(root: &Path, extra_args: &[&str]) -> Result<std::process::Output> {
    let allowlist_path = root.join("policy/non-rust-allowlist.toml");
    let root_str = root.to_str().expect("root is UTF-8");
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(root);
    cmd.args(["check-file-policy", "--allowlist"]);
    cmd.arg(&allowlist_path);
    cmd.args(["--root", root_str]);
    cmd.args(extra_args);
    let output = cmd.output()?;
    Ok(output)
}

/// Run `cargo xtask non-rust check` scoped to a temp repo (same seams as run_check).
fn run_non_rust_check(root: &Path, extra_args: &[&str]) -> Result<std::process::Output> {
    let allowlist_path = root.join("policy/non-rust-allowlist.toml");
    let root_str = root.to_str().expect("root is UTF-8");
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(root);
    cmd.args(["non-rust", "check", "--allowlist"]);
    cmd.arg(&allowlist_path);
    cmd.args(["--root", root_str]);
    cmd.args(extra_args);
    let output = cmd.output()?;
    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Advisory mode must never exit with code 1, even if there are unallowlisted files.
#[test]
fn advisory_mode_never_fails_on_unallowlisted() -> Result<()> {
    // An allowlist with no entries — everything will be "unclassified".
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "advisory"])?;
    assert!(
        output.status.success(),
        "advisory mode must exit 0 even with unallowlisted files; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// `--mode advisory` is the default — no explicit flag needed.
#[test]
fn advisory_mode_is_default() -> Result<()> {
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    // No --mode flag.
    let output = run_check(&root, &[])?;
    assert!(
        output.status.success(),
        "default mode must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// blocking-allowlist must fail when a non-Rust file has no allowlist entry.
#[test]
fn blocking_allowlist_fails_on_unallowlisted() -> Result<()> {
    // Allowlist has no entries for README.md.
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-allowlist"])?;
    assert!(
        !output.status.success(),
        "blocking-allowlist must exit 1 when unallowlisted files exist; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-allowlist must exit 0 when all non-Rust files are allowlisted.
#[test]
fn blocking_allowlist_ok_when_all_allowlisted() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "readme"
path = "README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Top-level readme"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-allowlist"])?;
    assert!(
        output.status.success(),
        "blocking-allowlist must exit 0 when all non-Rust files are allowlisted; \
         stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// blocking-allowlist must fail when an entry is missing `owner`.
#[test]
fn blocking_allowlist_fails_on_missing_owner() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "readme"
path = "README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = ""
reason = "Top-level readme"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-allowlist"])?;
    assert!(
        !output.status.success(),
        "blocking-allowlist must exit 1 when an entry has an empty owner; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-allowlist must fail when an entry has malformed matcher fields.
#[test]
fn blocking_allowlist_fails_on_invalid_glob() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "bad-glob"
glob = "["
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Invalid glob entry"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-allowlist"])?;
    assert!(
        !output.status.success(),
        "blocking-allowlist must exit 1 when an entry has an invalid glob; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-strict must fail when an entry has a past `review_after` date.
#[test]
fn blocking_strict_fails_on_expired_review_after() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "readme"
path = "README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Top-level readme"
covered_by = ["xtask"]
created = "2020-01-01"
review_after = "2020-06-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-strict"])?;
    assert!(
        !output.status.success(),
        "blocking-strict must exit 1 when review_after is in the past; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-strict must fail when two entries share the same id.
#[test]
fn blocking_strict_fails_on_duplicate_ids() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "readme"
path = "README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "First readme entry"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"

[[allow]]
id = "readme"
path = "CONTRIBUTING.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Duplicate id entry"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(
        &allowlist,
        &[("README.md", "# hi"), ("CONTRIBUTING.md", "# contributing")],
    )?;

    let output = run_check(&root, &["--mode", "blocking-strict"])?;
    assert!(
        !output.status.success(),
        "blocking-strict must exit 1 when duplicate entry ids exist; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-strict must fail when an entry has an absolute path.
#[test]
fn blocking_strict_fails_on_absolute_path() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "abs-readme"
path = "/absolute/path/README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Absolute path entry"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-strict"])?;
    assert!(
        !output.status.success(),
        "blocking-strict must exit 1 when an entry uses an absolute path; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-strict must fail when an entry has a broad glob without
/// `broad_glob_reason`.
#[test]
fn blocking_strict_fails_on_broad_glob_no_reason() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "all-files"
glob = "**/*"
kind = "mixed"
language = "mixed"
surface = "repo"
classification = "mixed"
owner = "repo"
reason = "Everything"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-strict"])?;
    assert!(
        !output.status.success(),
        "blocking-strict must exit 1 when a broad glob has no broad_glob_reason; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// blocking-strict must fail when a non-retired entry matches no tracked file.
#[test]
fn blocking_strict_fails_on_unused_entry() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "missing-file"
path = "MISSING.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Entry for a file that is not tracked"
covered_by = ["xtask"]
created = "2026-01-01"
review_after = "2027-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-strict"])?;
    assert!(
        !output.status.success(),
        "blocking-strict must exit 1 when an entry matches no tracked file; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// `--json` flag: the receipt file must be created and must match the
/// expected schema (schema_version 1, required top-level keys).
#[test]
fn json_output_schema_v1_valid() -> Result<()> {
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let receipt_path = root.join("target/policy/file-policy-report.json");
    let receipt_arg = receipt_path.to_str().expect("path is UTF-8");

    let output = run_check(&root, &["--mode", "advisory", "--json", receipt_arg])?;
    assert!(
        output.status.success(),
        "advisory mode must exit 0 even with --json; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(receipt_path.exists(), "receipt JSON must be written to {receipt_arg}");

    let content = fs::read_to_string(&receipt_path)?;
    let v: Value = serde_json::from_str(&content)?;

    // Required top-level fields.
    assert_eq!(v["schema_version"], 1, "schema_version must be 1");
    assert!(v["mode"].is_string(), "mode must be a string");
    assert_eq!(v["mode"], "advisory", "mode must be advisory");
    assert!(v["total_tracked"].is_number(), "total_tracked must be a number");
    assert!(v["non_rust"].is_number(), "non_rust must be a number");
    assert!(v["unclassified"].is_number(), "unclassified must be a number");
    assert!(v["expired"].is_number(), "expired must be a number");
    assert!(v["stale_review_after"].is_number(), "stale_review_after must be a number");
    assert!(v["duplicate_ids"].is_number(), "duplicate_ids must be a number");
    assert!(v["unused_entries"].is_number(), "unused_entries must be a number");
    assert!(v["violations"].is_array(), "violations must be an array");

    // Advisory with one unclassified file: violations must be empty (advisory never populates
    // them for unallowlisted files), and unclassified must be > 0.
    assert!(
        v["unclassified"].as_u64().unwrap_or(0) > 0,
        "unclassified should be >= 1 (README.md has no allowlist entry)"
    );
    assert_eq!(
        v["violations"].as_array().map(|a| a.len()).unwrap_or(0),
        0,
        "advisory mode must not add unallowlisted-file violations"
    );

    Ok(())
}

/// Without `--json`, the checker writes the default JSON and Markdown receipts.
#[test]
fn default_receipts_are_written() -> Result<()> {
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "advisory"])?;
    assert!(
        output.status.success(),
        "advisory mode must exit 0 and write default receipts; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        root.join("target/policy/file-policy-report.json").exists(),
        "default JSON receipt must be written"
    );
    assert!(
        root.join("target/policy/file-policy-report.md").exists(),
        "default Markdown report must be written"
    );
    Ok(())
}

/// `cargo xtask non-rust check` subcommand is also wired and works.
#[test]
fn non_rust_check_subcommand_exits_zero_advisory() -> Result<()> {
    let allowlist = minimal_allowlist("");
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_non_rust_check(&root, &["--mode", "advisory"])?;
    assert!(
        output.status.success(),
        "`non-rust check --mode advisory` must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// Help text is accessible and exits 0.
#[test]
fn check_file_policy_help_exits_zero() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["check-file-policy", "--help"]).output()?;
    assert!(output.status.success(), "check-file-policy --help should exit 0");
    Ok(())
}

/// blocking-allowlist must fail on an expired `expires` date.
#[test]
fn blocking_allowlist_fails_on_expired_expires() -> Result<()> {
    let allowlist = minimal_allowlist(
        r#"
[[allow]]
id = "readme"
path = "README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Old entry"
covered_by = ["xtask"]
created = "2020-01-01"
review_after = "2027-01-01"
expires = "2021-01-01"
"#,
    );
    let (_tmp, root) = setup_test_repo(&allowlist, &[("README.md", "# hi")])?;

    let output = run_check(&root, &["--mode", "blocking-allowlist"])?;
    assert!(
        !output.status.success(),
        "blocking-allowlist must exit 1 when an entry has expired; \
         stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
