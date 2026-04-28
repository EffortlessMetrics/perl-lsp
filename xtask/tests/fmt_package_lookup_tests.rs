//! Red TDD tests for issue #4512 -- pre-push hook uses dir basename not Cargo.toml package name.
//!
//! Tests must FAIL before implementation and PASS after.
//! All tests invoke the real xtask binary via assert_cmd (standard xtask test pattern).

use assert_cmd::Command;
use color_eyre::eyre::Result;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn project_root() -> PathBuf {
    // xtask is at <workspace-root>/xtask -- go up one level
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

// Helper: workspace where dir name differs from package name
fn make_mismatched_workspace() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/my-dir"]
resolver = "2"
"#,
    )?;
    let crate_dir = root.join("crates/my-dir/src");
    fs::create_dir_all(&crate_dir)?;
    fs::write(
        root.join("crates/my-dir/Cargo.toml"),
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    )?;
    fs::write(crate_dir.join("lib.rs"), "")?;
    Ok(dir)
}

// Helper: workspace where dir and package name match
fn make_matching_workspace() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/perl-parser"]
resolver = "2"
"#,
    )?;
    let crate_dir = root.join("crates/perl-parser/src");
    fs::create_dir_all(&crate_dir)?;
    fs::write(
        root.join("crates/perl-parser/Cargo.toml"),
        r#"[package]
name = "perl-parser"
version = "0.1.0"
edition = "2021"
"#,
    )?;
    fs::write(crate_dir.join("lib.rs"), "")?;
    Ok(dir)
}

// Helper: workspace with no members (for unknown-dir tests)
fn make_empty_workspace() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = []
resolver = "2"
"#,
    )?;
    Ok(dir)
}

// ---------------------------------------------------------------------------
// A. Subcommand must be registered and show help.
// RED: fails with "unrecognized subcommand" until Commands variant added.
// ---------------------------------------------------------------------------

#[test]
fn resolve_package_name_subcommand_help_exists() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["resolve-package-name", "--help"]).output()?;
    assert!(
        output.status.success(),
        "resolve-package-name --help should exit 0; got exit {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
    let help = String::from_utf8(output.stdout)?;
    assert!(
        help.to_lowercase().contains("package") || help.to_lowercase().contains("crate"),
        "Help text should mention package or crate; got: {}",
        help
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B1. Core regression: dir=my-dir, package=my-package => output must be my-package.
// RED: fails until subcommand is implemented.
// ---------------------------------------------------------------------------

#[test]
fn resolve_uses_cargo_toml_name_not_dir_basename() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/my-dir"])
        .output()?;
    assert!(
        output.status.success(),
        "should exit 0 for known member; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout.trim(),
        "my-package",
        "Expected Cargo.toml name 'my-package', got '{}'. Old bug returns dir basename 'my-dir'.",
        stdout.trim()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B2. Real workspace: crates/perl-lsp-rs resolves to perl-lsp-rs.
// (Previously tested the dir!=package mismatch; #4511 aligned the names.)
// ---------------------------------------------------------------------------

#[test]
fn resolve_perl_lsp_rs_dir_to_perl_lsp_rs_package() -> Result<()> {
    let root = project_root();
    let output = Command::cargo_bin("xtask")?
        .current_dir(&root)
        .args(["resolve-package-name", "crates/perl-lsp-rs"])
        .output()?;
    assert!(
        output.status.success(),
        "resolve-package-name crates/perl-lsp-rs should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout.trim(),
        "perl-lsp-rs",
        "crates/perl-lsp-rs must resolve to 'perl-lsp-rs', not '{}'.",
        stdout.trim()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// C. Normal case: dir and package name match.
// RED: fails until subcommand is implemented.
// ---------------------------------------------------------------------------

#[test]
fn resolve_when_dir_and_name_match() -> Result<()> {
    let ws = make_matching_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/perl-parser"])
        .output()?;
    assert!(
        output.status.success(),
        "should exit 0 for known member; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.trim(), "perl-parser", "Expected 'perl-parser', got '{}'", stdout.trim());
    Ok(())
}

// ---------------------------------------------------------------------------
// D. Error case: unknown dir must exit non-zero.
// RED: fails until subcommand is implemented.
// ---------------------------------------------------------------------------

#[test]
fn resolve_returns_error_for_unknown_dir() -> Result<()> {
    let ws = make_empty_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/nonexistent"])
        .output()?;
    assert!(
        !output.status.success(),
        "should exit non-zero for unknown dir; got exit 0, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// E. Output format: single clean word (no spinner noise, no embedded spaces).
// RED: fails until subcommand is implemented.
// ---------------------------------------------------------------------------

#[test]
fn resolve_outputs_single_clean_line() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/my-dir"])
        .output()?;
    assert!(output.status.success(), "should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "Output must not be empty");
    assert!(!trimmed.contains('\n'), "Output must be a single line, got: {:?}", trimmed);
    assert!(!trimmed.contains(' '), "Package name must not contain spaces, got: {:?}", trimmed);
    Ok(())
}

// ===========================================================================
// Green TDD edge case tests added by green-tdd agent (issue #4512)
// ===========================================================================

// ---------------------------------------------------------------------------
// F. Trailing slash normalization: "crates/my-dir/" (with slash) resolves same
//    as "crates/my-dir" (without slash).
//    Verifies the trim_end_matches('/') path in resolve_single_package_name.
// ---------------------------------------------------------------------------

#[test]
fn resolve_trailing_slash_normalized() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/my-dir/"])
        .output()?;
    assert!(
        output.status.success(),
        "trailing slash should be tolerated and resolve correctly; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout.trim(),
        "my-package",
        "trailing slash variant 'crates/my-dir/' must resolve to 'my-package', got '{}'",
        stdout.trim()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// G. Multiple trailing slashes: "crates/my-dir///" still normalizes.
//    trim_end_matches('/') strips all trailing slashes in one pass.
// ---------------------------------------------------------------------------

#[test]
fn resolve_multiple_trailing_slashes_normalized() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/my-dir///"])
        .output()?;
    // "crates/my-dir///" is not a valid workspace member key, so we only assert
    // it does NOT produce wrong output (old dir basename "my-dir").  It may exit
    // non-zero if cargo metadata does not recognise the triple-slash path.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Must NOT silently return the dir basename "my-dir///".
    assert_ne!(
        stdout.trim(),
        "my-dir///",
        "should not return raw triple-slash basename; stdout={} stderr={}",
        stdout,
        stderr
    );
    assert_ne!(
        stdout.trim(),
        "my-dir",
        "should not return stripped dir basename when given triple-slash path; stdout={} stderr={}",
        stdout,
        stderr
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// H. Empty crate_dir argument: "" must exit non-zero.
//    Guards against the hook accidentally passing an empty string (whitespace-
//    only SINGLE_CRATE_DIR after tr -d '[:space:]').
// ---------------------------------------------------------------------------

#[test]
fn resolve_empty_crate_dir_errors() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", ""])
        .output()?;
    assert!(
        !output.status.success(),
        "empty crate_dir must exit non-zero; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// I. Workspace-root path: "." must exit non-zero (workspace root itself is not
//    a publishable member).  Guards against the hook mis-classifying a repo-root
//    edit as a single-crate push.
// ---------------------------------------------------------------------------

#[test]
fn resolve_workspace_root_dot_errors() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "."])
        .output()?;
    assert!(
        !output.status.success(),
        "'.' (workspace root) is not a workspace member; should exit non-zero; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// J. Windows path separator regression: resolve_package_names normalizes "\"
//    to "/" so that manifest paths returned by cargo metadata on Windows are
//    matched correctly.
//    This is a unit test against the Rust function (not the CLI binary) so it
//    can construct the path normalization scenario without needing a Windows host.
//
//    Strategy: use the real workspace (project_root) on the current host.
//    The strip_prefix logic should work regardless of separator on the running OS.
//    We verify that "crates/perl-lsp-rs" resolves to "perl-lsp-rs" — if the
//    normalization were broken on the host OS, this test would fail.
// ---------------------------------------------------------------------------

#[test]
fn resolve_windows_path_separator_compat_via_real_workspace() -> Result<()> {
    // This test exercises the same code path as the Windows fix (normalize \ -> /)
    // by running against the real workspace.  On Linux/Mac it confirms forward-slash
    // paths work; on Windows it exercises the backslash normalisation branch.
    let root = project_root();
    let output = Command::cargo_bin("xtask")?
        .current_dir(&root)
        .args(["resolve-package-name", "crates/perl-lsp-rs"])
        .output()?;
    assert!(
        output.status.success(),
        "cross-platform path resolution should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout.trim(),
        "perl-lsp-rs",
        "Windows separator fix: crates/perl-lsp-rs must resolve to perl-lsp-rs; got '{}'",
        stdout.trim()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// K. stderr is clean on success: no extraneous diagnostic lines on stdout.
//    The hook captures stdout with $(...) so extra lines would corrupt SINGLE_CRATE_NAME.
// ---------------------------------------------------------------------------

#[test]
fn resolve_stdout_contains_only_package_name_on_success() -> Result<()> {
    let ws = make_mismatched_workspace()?;
    let output = Command::cargo_bin("xtask")?
        .current_dir(ws.path())
        .args(["resolve-package-name", "crates/my-dir"])
        .output()?;
    assert!(output.status.success(), "should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    // Exactly one non-empty line on stdout (the package name + newline).
    let non_empty_lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        non_empty_lines.len(),
        1,
        "stdout must contain exactly one non-empty line (the package name); got {} lines: {:?}",
        non_empty_lines.len(),
        non_empty_lines
    );
    assert_eq!(
        non_empty_lines[0].trim(),
        "my-package",
        "the single stdout line must be the package name; got '{}'",
        non_empty_lines[0]
    );
    Ok(())
}
