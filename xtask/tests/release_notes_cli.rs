//! End-to-end CLI tests for `cargo xtask release-notes`.
//!
//! These exercise the same extraction code as the in-module unit tests, but
//! through the compiled binary — verifying argument parsing, exit codes, and
//! stdout/file output wiring. The hidden `--root` flag points the command at
//! a throwaway tempdir so tests never depend on the repo's shipped notes.

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::TempDir;

fn write_note(root: &std::path::Path, tag: &str, contents: &str) -> Result<()> {
    let dir = root.join("docs").join("releases");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{tag}.md")), contents)?;
    Ok(())
}

fn root_arg(root: &std::path::Path) -> &str {
    use perl_tdd_support::must_some;
    must_some(root.to_str())
}

const SAMPLE_NOTE: &str = "---\n\
version: \"9.9.9\"\n\
tag: \"v9.9.9\"\n\
notes_status: canonical\n\
---\n\
\n\
# v9.9.9\n\
\n\
## Summary\n\
\n\
Sample body used by release-notes CLI tests.\n";

const SAMPLE_BODY: &str =
    "# v9.9.9\n\n## Summary\n\nSample body used by release-notes CLI tests.\n";

#[test]
fn release_notes_cli_emits_body_to_stdout() -> Result<()> {
    let temp = TempDir::new()?;
    write_note(temp.path(), "v9.9.9", SAMPLE_NOTE)?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", root_arg(temp.path()), "--tag", "v9.9.9"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert_eq!(stdout, SAMPLE_BODY);
    Ok(())
}

#[test]
fn release_notes_cli_writes_output_file() -> Result<()> {
    let temp = TempDir::new()?;
    write_note(temp.path(), "v9.9.9", SAMPLE_NOTE)?;
    let out = temp.path().join("notes.out.md");
    let out_str = root_arg(&out);

    cargo_bin_cmd!("xtask")
        .args([
            "release-notes",
            "--root",
            root_arg(temp.path()),
            "--tag",
            "v9.9.9",
            "--output",
            out_str,
        ])
        .assert()
        .success();

    let body = fs::read_to_string(&out)?;
    assert_eq!(body, SAMPLE_BODY);
    Ok(())
}

#[test]
fn release_notes_cli_accepts_bare_version() -> Result<()> {
    let temp = TempDir::new()?;
    // Note lives at v9.9.9 but caller passes bare `9.9.9`.
    write_note(temp.path(), "v9.9.9", SAMPLE_NOTE)?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", root_arg(temp.path()), "--tag", "9.9.9"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert_eq!(stdout, SAMPLE_BODY);
    Ok(())
}

#[test]
fn release_notes_cli_fails_when_file_missing() -> Result<()> {
    let temp = TempDir::new()?;
    // Intentionally write no release notes file.
    fs::create_dir_all(temp.path().join("docs").join("releases"))?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", root_arg(temp.path()), "--tag", "v0.0.0-missing"])
        .assert()
        .failure();

    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert!(
        stderr.contains("release notes file missing"),
        "expected clear 'release notes file missing' error, got: {stderr}"
    );
    Ok(())
}

#[test]
fn release_notes_cli_fails_on_unterminated_frontmatter() -> Result<()> {
    let temp = TempDir::new()?;
    write_note(
        temp.path(),
        "v9.9.9",
        "---\nversion: \"9.9.9\"\nno closing fence in sight\n# v9.9.9\nbody\n",
    )?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", root_arg(temp.path()), "--tag", "v9.9.9"])
        .assert()
        .failure();

    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert!(
        stderr.contains("never closed"),
        "expected 'never closed' frontmatter error, got: {stderr}"
    );
    Ok(())
}

#[test]
fn release_notes_cli_without_frontmatter_returns_full_file() -> Result<()> {
    let temp = TempDir::new()?;
    let raw = "# v9.9.9\n\nPlain body, no frontmatter.\n";
    write_note(temp.path(), "v9.9.9", raw)?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["release-notes", "--root", root_arg(temp.path()), "--tag", "v9.9.9"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert_eq!(stdout, raw);
    Ok(())
}
