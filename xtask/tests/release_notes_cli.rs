//! CLI-level BDD coverage for `cargo xtask release-notes` (issue #4340).
//!
//! The unit tests in `xtask/src/tasks/release_notes.rs` cover the extraction
//! core. These tests exercise the full CLI binary so we catch argument
//! wiring, stdout behaviour, and exit-code semantics the GitHub Release
//! workflow depends on.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::TempDir;

fn write_notes(root: &std::path::Path, version: &str, content: &str) {
    let dir = root.join("docs").join("releases");
    fs::create_dir_all(&dir).expect("create docs/releases");
    fs::write(dir.join(format!("v{version}.md")), content).expect("write notes");
}

/// Given a repo layout with a curated notes file,
/// When `cargo xtask release-notes <version>` runs,
/// Then stdout is the body with front-matter stripped, and exit code is 0.
#[test]
fn release_notes_prints_body_without_front_matter() {
    let tmp = TempDir::new().unwrap();
    write_notes(tmp.path(), "1.2.3", "---\nversion: \"1.2.3\"\n---\n\n# v1.2.3\n\nCurated body.\n");

    let assert = cargo_bin_cmd!("xtask")
        .current_dir(tmp.path())
        .args(["release-notes", "1.2.3"])
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().next(), Some("# v1.2.3"));
    assert!(stdout.contains("Curated body."));
    assert!(!stdout.contains("version: \"1.2.3\""), "front-matter leaked");
}

/// Given a missing notes file,
/// When the CLI runs,
/// Then it fails (non-zero exit) with a diagnostic naming the missing path.
#[test]
fn release_notes_fails_when_file_missing() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("docs/releases")).unwrap();

    let assert = cargo_bin_cmd!("xtask")
        .current_dir(tmp.path())
        .args(["release-notes", "9.9.9"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("release notes file not found"), "stderr: {stderr}");
    assert!(stderr.contains("v9.9.9.md"), "stderr: {stderr}");
}

/// Given --file pointing at a custom location,
/// When the CLI runs,
/// Then the file is read directly without requiring the docs/releases layout.
#[test]
fn release_notes_file_flag_reads_arbitrary_path() {
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom.md");
    fs::write(&custom, "---\nversion: \"5.0.0\"\n---\n\n# v5.0.0\n\nFrom explicit --file.\n")
        .unwrap();

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--file", custom.to_str().unwrap()])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("From explicit --file."));
    assert!(!stdout.contains("version: \"5.0.0\""));
}

/// Given `--root` pointing at a sibling checkout,
/// When the CLI runs,
/// Then the extractor resolves notes against the provided root.
#[test]
fn release_notes_root_flag_resolves_against_custom_root() {
    let tmp = TempDir::new().unwrap();
    let fake_repo = tmp.path().join("fake-repo");
    write_notes(&fake_repo, "7.7.7", "---\nv: \"7.7.7\"\n---\n\n# v7.7.7\n\nBody.\n");

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", fake_repo.to_str().unwrap(), "7.7.7"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.lines().next(), Some("# v7.7.7"));
}

/// Given an invalid version string,
/// When the CLI runs,
/// Then it fails with a version-validation diagnostic.
#[test]
fn release_notes_rejects_invalid_version() {
    let tmp = TempDir::new().unwrap();

    let assert = cargo_bin_cmd!("xtask")
        .current_dir(tmp.path())
        .args(["release-notes", "not-a-version"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("invalid version"), "stderr: {stderr}");
}
