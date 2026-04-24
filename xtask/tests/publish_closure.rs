//! Integration tests for the `cargo xtask publish-closure` subcommand.
//!
//! These tests verify the CLI contract:
//! - Default invocation exits 0 with success message
//! - `--crate-name` filter works
//! - Invalid crate names exit 1 with clear error
//!
//! Tests use assert_cmd to verify the real binary behavior.

use assert_cmd::Command;
use color_eyre::eyre::Result;

/// Test that `cargo xtask publish-closure` (no args) exits 0 on master.
///
/// Verifies the default behavior: check all allowlisted crates for transitive
/// normal-dep violations. Should succeed on master (no violations).
#[test]
fn publish_closure_passes_on_master() -> Result<()> {
    Command::cargo_bin("xtask")?.args(["publish-closure"]).assert().success();
    Ok(())
}

/// Test that `cargo xtask publish-closure --crate-name perl-token` exits 0.
///
/// Verifies single-crate filtering works. perl-token is a known published crate
/// with no publish-closure violations.
#[test]
fn publish_closure_single_crate_filter() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-token"])
        .assert()
        .success();
    Ok(())
}

/// Test that `cargo xtask publish-closure --crate-name nonexistent-crate-xyz` exits 1.
///
/// Verifies error handling for invalid crate names. Should exit with status 1
/// and produce a clear error message naming the unrecognized crate.
#[test]
fn publish_closure_unknown_crate_exits_nonzero() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "nonexistent-crate-xyz"])
        .assert()
        .failure();
    Ok(())
}
