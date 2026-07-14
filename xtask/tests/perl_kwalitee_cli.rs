//! CLI integration tests for `cargo xtask perl-kwalitee`.
//!
//! Unit tests for the pure evaluation logic live in the `perl-kwalitee` crate.
//! These tests exercise the actual built `xtask` binary end-to-end: the
//! `explain` / `report` / `check` subcommands, the emitted schema-v1 JSON, exit
//! codes, and profile scoping. `report`/`check` are driven against a hermetic
//! fixture tree via `--repo-root` so they are fast and deterministic (no live
//! workspace gates, no `update-status`).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use assert_cmd::Command;
use color_eyre::eyre::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Build a minimal, *clean* distribution tree the native indicators all pass on.
fn clean_fixture() -> Result<TempDir> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/perl-kwalitee\"]\n[workspace.metadata.publish]\nallow = []\n",
    )?;
    write(
        root,
        "crates/perl-kwalitee/Cargo.toml",
        "[package]\nname = \"perl-kwalitee\"\nlicense.workspace = true\npublish = false\n",
    )?;
    // A clean first-mile surface (no external-tool product framing).
    write(root, "vscode-extension/package.json", "\"description\": \"native Perl debugger\"\n")?;
    // perl-dap CLI source with no `--bridge` flag → dap.cli_native_only passes.
    write(root, "crates/perl-dap/src/main.rs", "fn main() {}\n")?;
    Ok(dir)
}

fn write(root: &Path, rel: &str, contents: &str) -> Result<()> {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, contents)?;
    Ok(())
}

/// Run `perl-kwalitee report` against `root` and return the parsed JSON receipt.
///
/// Receipts are written to a throwaway directory (not into `root`) so the
/// evaluated tree is never polluted — this lets `root` point at a real
/// subdirectory of the live repo without dirtying tracked files.
fn report_json(root: &Path, profile: &str) -> Result<Value> {
    let out_dir = tempfile::tempdir()?;
    let out = out_dir.path().join("kwalitee.json");
    let md = out_dir.path().join("kwalitee.md");
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "report", "--profile", profile, "--repo-root"])
        .arg(root)
        .args(["--json"])
        .arg(&out)
        .args(["--markdown"])
        .arg(&md)
        .output()?;
    assert!(
        output.status.success(),
        "report should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(md.exists(), "markdown receipt should be written");
    Ok(serde_json::from_slice(&std::fs::read(&out)?)?)
}

fn status_of<'a>(receipt: &'a Value, id: &str) -> &'a str {
    receipt["indicators"]
        .as_array()
        .expect("indicators array")
        .iter()
        .find(|i| i["id"] == id)
        .unwrap_or_else(|| panic!("indicator {id} not found"))["status"]
        .as_str()
        .expect("status string")
}

#[test]
fn explain_known_indicator_succeeds() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "explain", "release.no_external_tooling"])
        .output()?;
    assert!(output.status.success(), "explain should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("release.no_external_tooling"), "names the id: {stdout}");
    assert!(stdout.contains("[release]"), "shows the area: {stdout}");
    assert!(stdout.contains("Why:") && stdout.contains("Fix:"), "shows rationale + fix: {stdout}");
    Ok(())
}

#[test]
fn explain_nightly_indicator_succeeds() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "explain", "formatter.corpus_idempotent"])
        .output()?;
    assert!(output.status.success(), "nightly indicator should be explainable");
    let stdout = String::from_utf8(output.stdout)?;
    // Verify the explanation targets the right indicator and is non-empty, not
    // just that the command exits 0.
    assert!(stdout.contains("formatter.corpus_idempotent"), "names the id: {stdout}");
    assert!(stdout.contains("[formatter]"), "shows the area: {stdout}");
    assert!(stdout.contains("Why:") && stdout.contains("Fix:"), "shows rationale + fix: {stdout}");
    Ok(())
}

#[test]
fn explain_unknown_indicator_fails() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "explain", "nope.not_real"])
        .output()?;
    assert!(!output.status.success(), "unknown id should exit non-zero");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("Known indicators:"), "lists known ids on failure: {stderr}");
    Ok(())
}

#[test]
fn report_pr_emits_valid_schema_v1() -> Result<()> {
    let dir = clean_fixture()?;
    let receipt = report_json(dir.path(), "pr")?;

    assert_eq!(receipt["kind"], "perl_kwalitee");
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["profile"], "pr");
    // Every documented top-level field is present.
    for field in [
        "generated_at",
        "commit",
        "score",
        "verdict",
        "mandatory_passed",
        "mandatory_failed_count",
        "mandatory_unverified_count",
        "warning_count",
        "unverified_count",
        "indicators",
    ] {
        assert!(receipt.get(field).is_some(), "missing top-level field `{field}`");
    }
    assert!(!receipt["indicators"].as_array().expect("array").is_empty());
    Ok(())
}

#[test]
fn report_repo_root_stamps_commit_of_evaluated_tree() -> Result<()> {
    // The fixture tree is not a git repo, so the receipt's commit must be
    // "unknown" — it must NOT leak the live workspace's HEAD SHA (provenance:
    // the receipt describes the --repo-root tree, so its commit tracks that tree).
    let dir = clean_fixture()?;
    let receipt = report_json(dir.path(), "pr")?;
    assert_eq!(receipt["commit"], "unknown", "non-git fixture must not stamp the live HEAD");
    Ok(())
}

#[test]
fn report_repo_root_subdir_of_git_repo_stamps_unknown() -> Result<()> {
    // A --repo-root pointing at a SUBDIRECTORY of a git repo (here: the xtask
    // crate dir inside the live workspace) must still stamp "unknown", not the
    // parent repo's HEAD — `git rev-parse HEAD` walks up, so the commit is only
    // valid when root is the repo top level.
    let subdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let receipt = report_json(&subdir, "pr")?;
    assert_eq!(receipt["commit"], "unknown", "a repo subdir must not stamp the parent HEAD");
    Ok(())
}

#[test]
fn missing_repo_root_errors() -> Result<()> {
    // A typo/missing --repo-root must error, not silently evaluate an empty tree
    // (which would let a non-strict `check` pass without evaluating anything).
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("does-not-exist");
    for sub in ["check", "report"] {
        let output = Command::cargo_bin("xtask")?
            .args(["perl-kwalitee", sub, "--repo-root"])
            .arg(&missing)
            .output()?;
        assert!(!output.status.success(), "{sub} with a missing --repo-root must error");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not an existing directory"),
            "{sub} should explain the bad --repo-root: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn report_pr_native_indicators_pass_release_not_applicable() -> Result<()> {
    let dir = clean_fixture()?;
    let receipt = report_json(dir.path(), "pr")?;

    for id in [
        "manifest.workspace_member_declared",
        "manifest.publish_policy_clean",
        "license.declared",
        "product_surface.native_only",
        "dap.cli_native_only",
    ] {
        assert_eq!(status_of(&receipt, id), "pass", "native indicator {id} should pass");
    }
    for id in ["release.native_binaries_present", "release.no_external_tooling"] {
        assert_eq!(status_of(&receipt, id), "not_applicable", "{id} n/a under pr");
    }
    Ok(())
}

#[test]
fn nightly_profile_scopes_nightly_indicators() -> Result<()> {
    let dir = clean_fixture()?;
    // Under pr, a nightly-only indicator is not applicable...
    assert_eq!(
        status_of(&report_json(dir.path(), "pr")?, "formatter.corpus_idempotent"),
        "not_applicable",
    );
    // ...and under nightly it is evaluated (unverified — no receipt in the fixture).
    assert_eq!(
        status_of(&report_json(dir.path(), "nightly")?, "formatter.corpus_idempotent"),
        "unverified",
    );
    Ok(())
}

#[test]
fn report_reads_readiness_receipt() -> Result<()> {
    let dir = clean_fixture()?;
    // A native-tooling readiness receipt at the default path under the fixture
    // root should flip formatter.native_default / critic.native_default to pass.
    write(
        dir.path(),
        "target/receipts/native-tooling/readiness.json",
        "{\"kind\":\"native_tooling_readiness\",\"commit\":\"\",\"criteria\":[\
         {\"area\":\"formatter\",\"name\":\"native-default engine\",\"status\":\"ready\"},\
         {\"area\":\"critic\",\"name\":\"native default\",\"status\":\"ready\"}]}",
    )?;
    let receipt = report_json(dir.path(), "pr")?;
    assert_eq!(status_of(&receipt, "formatter.native_default"), "pass");
    assert_eq!(status_of(&receipt, "critic.native_default"), "pass");
    Ok(())
}

#[test]
fn check_clean_fixture_exits_zero() -> Result<()> {
    // Native indicators pass; receipt-backed + docs are unverified (advisory
    // Warn under non-strict), so the gate does not fail.
    let dir = clean_fixture()?;
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "check", "--profile", "pr", "--repo-root"])
        .arg(dir.path())
        .output()?;
    assert!(
        output.status.success(),
        "clean fixture should not fail the gate: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Perl Kwalitee:"), "prints a summary line: {stdout}");
    Ok(())
}

#[test]
fn check_fails_on_broken_manifest() -> Result<()> {
    // Root manifest does not declare the crate → a mandatory native indicator
    // fails → verdict fail → non-zero exit.
    let dir = clean_fixture()?;
    write(dir.path(), "Cargo.toml", "[workspace]\nmembers = [\"crates/other\"]\n")?;
    let output = Command::cargo_bin("xtask")?
        .args(["perl-kwalitee", "check", "--profile", "pr", "--repo-root"])
        .arg(dir.path())
        .output()?;
    assert!(!output.status.success(), "a mandatory native failure must fail the gate");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("FAIL") || combined.contains("check failed"),
        "reports failure: {combined}"
    );
    Ok(())
}
