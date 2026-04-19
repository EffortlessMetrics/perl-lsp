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
fn make_mismatched_workspace() -> TempDir {
    let dir = TempDir::new().expect("create tempdir");
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/my-dir"]
resolver = "2"
"#,
    )
    .expect("write workspace Cargo.toml");
    let crate_dir = root.join("crates/my-dir/src");
    fs::create_dir_all(&crate_dir).expect("create crate src");
    fs::write(
        root.join("crates/my-dir/Cargo.toml"),
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write crate Cargo.toml");
    fs::write(crate_dir.join("lib.rs"), "").expect("write lib.rs");
    dir
}

// Helper: workspace where dir and package name match
fn make_matching_workspace() -> TempDir {
    let dir = TempDir::new().expect("create tempdir");
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/perl-parser"]
resolver = "2"
"#,
    )
    .expect("write workspace Cargo.toml");
    let crate_dir = root.join("crates/perl-parser/src");
    fs::create_dir_all(&crate_dir).expect("create crate src");
    fs::write(
        root.join("crates/perl-parser/Cargo.toml"),
        r#"[package]
name = "perl-parser"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write crate Cargo.toml");
    fs::write(crate_dir.join("lib.rs"), "").expect("write lib.rs");
    dir
}

// Helper: workspace with no members (for unknown-dir tests)
fn make_empty_workspace() -> TempDir {
    let dir = TempDir::new().expect("create tempdir");
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = []
resolver = "2"
"#,
    )
    .expect("write workspace Cargo.toml");
    dir
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
    let ws = make_mismatched_workspace();
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
// B2. Actual fix: crates/perl-lsp resolves to perl-lsp-rs (real workspace).
// RED: fails until subcommand is implemented.
// ---------------------------------------------------------------------------

#[test]
fn resolve_perl_lsp_dir_to_perl_lsp_rs_package() -> Result<()> {
    let root = project_root();
    let output = Command::cargo_bin("xtask")?
        .current_dir(&root)
        .args(["resolve-package-name", "crates/perl-lsp"])
        .output()?;
    assert!(
        output.status.success(),
        "resolve-package-name crates/perl-lsp should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout.trim(),
        "perl-lsp-rs",
        "crates/perl-lsp must resolve to 'perl-lsp-rs', not '{}'. This is bug #4512.",
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
    let ws = make_matching_workspace();
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
    let ws = make_empty_workspace();
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
    let ws = make_mismatched_workspace();
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
