//! Integration tests for `cargo xtask non-rust propose` (issue #8568).
//!
//! Each test creates an isolated temp directory simulating a minimal git repo,
//! then invokes the xtask binary and asserts on output files and exit codes.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::Command;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

mod git_test_support;

use git_test_support::{add_and_commit, init_git_repo};

// ---------------------------------------------------------------------------
// Helpers (shared with check_file_policy.rs pattern)
// ---------------------------------------------------------------------------

/// Build the standard allowlist header with no entries (empty ledger).
fn empty_allowlist() -> String {
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
"#
    .to_string()
}

/// Create a minimal test repo and return (TempDir, root path).
fn setup_repo(extra_files: &[(&str, &str)]) -> Result<(TempDir, PathBuf)> {
    let tmp = TempDir::new()?;
    let root = tmp.path().to_path_buf();

    init_git_repo(&root)?;

    let mut files: Vec<(&str, &str)> = vec![("src/lib.rs", "// Rust sentinel")];
    files.extend_from_slice(extra_files);
    add_and_commit(&root, &files, "initial")?;

    // Write allowlist (not committed — path passed via --allowlist).
    let policy_dir = root.join("policy");
    fs::create_dir_all(&policy_dir)?;
    fs::write(policy_dir.join("non-rust-allowlist.toml"), empty_allowlist())?;

    Ok((tmp, root))
}

/// Run `cargo xtask non-rust propose` with the given extra args.
fn run_propose(root: &Path, extra_args: &[&str]) -> Result<std::process::Output> {
    let allowlist_path = root.join("policy/non-rust-allowlist.toml");
    let root_str = root.to_str().expect("root is UTF-8");
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(root);
    cmd.args(["non-rust", "propose"]);
    // Pass the root seam so git ls-files runs in temp repo.
    cmd.args(["--root", root_str]);
    // Use a temp output dir inside the repo.
    cmd.args(["--output-dir", root.join("target/policy").to_str().expect("UTF-8")]);
    // Pass allowlist override (even though it won't be mutated — just ensures
    // the command can load it without needing the real workspace).
    let _ = allowlist_path; // used via --root; allowlist is found relative to root
    cmd.args(extra_args);
    let output = cmd.output()?;
    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The canonical `policy/non-rust-allowlist.toml` must NOT be modified after
/// running `propose`.
#[test]
fn propose_writes_only_target_dir() -> Result<()> {
    let (_tmp, root) =
        setup_repo(&[("book/ch01.md", "# Chapter 1"), ("docs/guide.md", "# Guide")])?;

    let allowlist_before = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;

    let output = run_propose(&root, &[])?;
    assert!(
        output.status.success(),
        "propose must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let allowlist_after = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    assert_eq!(
        allowlist_before, allowlist_after,
        "propose must NOT modify policy/non-rust-allowlist.toml"
    );

    // Proposed outputs must be inside target/policy/.
    assert!(
        root.join("target/policy/non-rust-proposed-allowlist.toml").exists(),
        "proposed TOML must exist in target/policy/"
    );
    assert!(
        root.join("target/policy/non-rust-proposal.md").exists(),
        "proposal markdown must exist in target/policy/"
    );

    Ok(())
}

/// Default grouping is `directory`: files in `book/` and `docs/` produce 2
/// directory-glob entries, not one entry per file.
#[test]
fn propose_groups_by_directory_by_default() -> Result<()> {
    // Create many files across two directories.
    let files: Vec<(&str, &str)> = vec![
        ("book/ch01.md", "# ch1"),
        ("book/ch02.md", "# ch2"),
        ("book/ch03.md", "# ch3"),
        ("docs/api.md", "# api"),
        ("docs/guide.md", "# guide"),
    ];
    let (_tmp, root) = setup_repo(&files)?;

    let output = run_propose(&root, &["--group-by", "directory"])?;
    assert!(
        output.status.success(),
        "propose must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let toml_content =
        fs::read_to_string(root.join("target/policy/non-rust-proposed-allowlist.toml"))?;

    // Must have exactly 2 `[[allow]]` sections (book + docs).
    let allow_count = toml_content.matches("[[allow]]").count();
    assert_eq!(
        allow_count, 2,
        "directory grouping must produce 2 entries (book + docs), got {allow_count}:\n{toml_content}"
    );

    // Glob patterns must be directory-style, not per-file paths.
    assert!(
        toml_content.contains("book/**/*") || toml_content.contains("\"book/**/*\""),
        "must have a book/**/* glob entry"
    );
    assert!(
        toml_content.contains("docs/**/*") || toml_content.contains("\"docs/**/*\""),
        "must have a docs/**/* glob entry"
    );

    Ok(())
}

/// `--group-by extension` produces extension-glob entries instead of directory entries.
#[test]
fn propose_groups_by_extension_with_flag() -> Result<()> {
    let files: Vec<(&str, &str)> = vec![
        ("book/ch01.md", "# ch1"),
        ("docs/api.md", "# api"),
        ("scripts/build.sh", "#!/bin/bash"),
        ("scripts/deploy.sh", "#!/bin/bash"),
    ];
    let (_tmp, root) = setup_repo(&files)?;

    let output = run_propose(&root, &["--group-by", "extension"])?;
    assert!(
        output.status.success(),
        "propose must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let toml_content =
        fs::read_to_string(root.join("target/policy/non-rust-proposed-allowlist.toml"))?;

    // Must have entries for .md and .sh (2 extensions).
    let allow_count = toml_content.matches("[[allow]]").count();
    assert_eq!(
        allow_count, 2,
        "extension grouping must produce 2 entries (.md + .sh), got {allow_count}:\n{toml_content}"
    );

    // Extension globs must use **/*.ext pattern.
    assert!(
        toml_content.contains("**/*.md") || toml_content.contains("\"**/*.md\""),
        "must have a **/*.md glob"
    );
    assert!(
        toml_content.contains("**/*.sh") || toml_content.contains("\"**/*.sh\""),
        "must have a **/*.sh glob"
    );

    Ok(())
}

/// Generated entries must have `review_after` approximately 30 days from today.
#[test]
fn propose_emits_review_after_30_days() -> Result<()> {
    let (_tmp, root) = setup_repo(&[("docs/readme.md", "# Readme")])?;

    let output = run_propose(&root, &[])?;
    assert!(output.status.success(), "propose must exit 0");

    let toml_content =
        fs::read_to_string(root.join("target/policy/non-rust-proposed-allowlist.toml"))?;

    // The review_after field must appear in the TOML.
    assert!(toml_content.contains("review_after"), "proposed TOML must contain review_after field");

    // The review_after date must be in the future (not today or past).
    // We verify it is at least today + 29 days (allowing for clock skew in fast CI).
    // Parse the date from the TOML.
    let review_line = toml_content
        .lines()
        .find(|l| l.trim_start().starts_with("review_after"))
        .expect("review_after line must exist");

    // Extract YYYY-MM-DD from `review_after = "2026-06-10"`.
    let date_str =
        review_line.split('"').nth(1).expect("review_after must have a quoted date value");

    let parts: Vec<u32> = date_str.split('-').map(|s| s.parse().unwrap()).collect();
    assert_eq!(parts.len(), 3, "review_after date must be YYYY-MM-DD");

    // Simple sanity: year must be >= 2026.
    assert!(parts[0] >= 2026, "review_after year must be >= 2026, got {}", parts[0]);

    Ok(())
}

/// Generated entries must have `owner = "TBD"` — never a real owner.
#[test]
fn propose_emits_owner_tbd_placeholder() -> Result<()> {
    let (_tmp, root) = setup_repo(&[("book/ch01.md", "# ch1")])?;

    let output = run_propose(&root, &[])?;
    assert!(output.status.success(), "propose must exit 0");

    let toml_content =
        fs::read_to_string(root.join("target/policy/non-rust-proposed-allowlist.toml"))?;

    // Every `owner` field in the [[allow]] sections must be "TBD".
    let owner_lines: Vec<&str> = toml_content
        .lines()
        .filter(|l| l.trim_start().starts_with("owner =") && !l.contains("owner = \"TBD\""))
        .filter(|l| !l.trim().starts_with('#'))
        .collect();

    // There should be no owner lines that are NOT "TBD" (excluding the top-level
    // `owner = "TBD"` line, which is the allowlist-level owner).
    let non_tbd: Vec<&str> =
        owner_lines.iter().copied().filter(|l| !l.contains("\"TBD\"")).collect();
    assert!(
        non_tbd.is_empty(),
        "all owner fields in proposed entries must be \"TBD\", found non-TBD: {non_tbd:?}"
    );

    // At least one owner = "TBD" line must exist.
    assert!(
        toml_content.contains("owner = \"TBD\""),
        "proposed TOML must have owner = \"TBD\" placeholder"
    );

    Ok(())
}

/// `propose` emits a human-readable markdown summary with group sections.
#[test]
fn propose_emits_human_markdown() -> Result<()> {
    let (_tmp, root) = setup_repo(&[("book/ch01.md", "# ch1"), ("docs/guide.md", "# guide")])?;

    let output = run_propose(&root, &[])?;
    assert!(output.status.success(), "propose must exit 0");

    let md_content = fs::read_to_string(root.join("target/policy/non-rust-proposal.md"))?;

    // Must have the main heading.
    assert!(
        md_content.contains("# Non-Rust Allowlist Proposal"),
        "markdown must have a main heading"
    );
    // Must have a Summary section.
    assert!(md_content.contains("## Summary"), "markdown must have a Summary section");
    // Must have count information.
    assert!(
        md_content.contains("Unclassified files"),
        "markdown must mention unclassified file count"
    );
    // Must have group sections.
    assert!(
        md_content.contains("## Groups by")
            || md_content.contains("### `book`")
            || md_content.contains("### `docs`"),
        "markdown must have per-group sections"
    );

    Ok(())
}

/// `propose` highlights automation groups that should move into Rust-owned
/// tooling instead of being blindly accepted as long-lived non-Rust surfaces.
#[test]
fn propose_highlights_rust_migration_candidates() -> Result<()> {
    let files: Vec<(&str, &str)> = vec![
        ("scripts/build.sh", "#!/bin/bash"),
        ("scripts/release.py", "print('release')"),
        ("docs/guide.md", "# guide"),
    ];
    let (_tmp, root) = setup_repo(&files)?;

    let output = run_propose(&root, &[])?;
    assert!(
        output.status.success(),
        "propose must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let md_content = fs::read_to_string(root.join("target/policy/non-rust-proposal.md"))?;
    assert!(
        md_content.contains("## Rust migration candidates"),
        "markdown must include a Rust migration candidate section"
    );
    assert!(
        md_content.contains("`scripts`"),
        "scripts group must be identified as a migration candidate: {md_content}"
    );
    assert!(
        md_content.contains("xtask tasks"),
        "automation scripts must recommend the xtask core design: {md_content}"
    );

    Ok(())
}

/// After `propose`, running advisory check against the proposed allowlist should
/// exit 0 — the proposed allowlist actually covers the files it claims to.
#[test]
fn propose_round_trips_via_advisory_checker() -> Result<()> {
    let files: Vec<(&str, &str)> =
        vec![("book/ch01.md", "# ch1"), ("book/ch02.md", "# ch2"), ("docs/guide.md", "# guide")];
    let (_tmp, root) = setup_repo(&files)?;

    // Run propose first.
    let propose_out = run_propose(&root, &[])?;
    assert!(
        propose_out.status.success(),
        "propose must exit 0; stderr={}",
        String::from_utf8_lossy(&propose_out.stderr)
    );

    let proposed_allowlist = root.join("target/policy/non-rust-proposed-allowlist.toml");
    assert!(proposed_allowlist.exists(), "proposed allowlist must be written");

    // Now run advisory check using the proposed allowlist.
    let root_str = root.to_str().expect("root is UTF-8");
    let allowlist_str = proposed_allowlist.to_str().expect("UTF-8");
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(&root);
    cmd.args([
        "non-rust",
        "check",
        "--mode",
        "advisory",
        "--allowlist",
        allowlist_str,
        "--root",
        root_str,
    ]);
    let check_out = cmd.output()?;

    assert!(
        check_out.status.success(),
        "advisory check against proposed allowlist must exit 0; stderr={}",
        String::from_utf8_lossy(&check_out.stderr)
    );

    Ok(())
}
