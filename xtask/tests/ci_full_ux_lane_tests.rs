//! Release-confidence lane wiring tests for automated UX coverage.
//!
//! Guards the contract that `just ci-full` includes the `perl-lsp-ux-tests`
//! harness so local release-confidence runs do not silently miss the same UX
//! surface that CI exercises.

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

#[test]
fn test_ci_full_runs_ux_tests() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    let ci_full_start =
        justfile.find("\nci-full:\n").ok_or("ci-full recipe must exist in justfile")?;
    let ci_full_body = &justfile[ci_full_start..];
    let next_recipe = ci_full_body
        .find("\n# Local CI parity")
        .ok_or("ci-full recipe terminator marker must exist in justfile")?;
    let ci_full_body = &ci_full_body[..next_recipe];

    assert!(
        ci_full_body.contains("@just ux-tests"),
        "ci-full must run `just ux-tests` so the local release-confidence lane \
         includes automated first-5-minutes UX workflows.\n\
         Current ci-full recipe:\n{}",
        ci_full_body
    );

    Ok(())
}

#[test]
fn test_ux_tests_recipe_builds_and_exports_binary() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    let ux_tests_start =
        justfile.find("\nux-tests:\n").ok_or("ux-tests recipe must exist in justfile")?;
    let ux_tests_body = &justfile[ux_tests_start..];
    let next_recipe = ux_tests_body
        .find("\n# @INC consumer-consistency conformance harness.")
        .ok_or("ux-tests recipe terminator marker must exist in justfile")?;
    let ux_tests_body = &ux_tests_body[..next_recipe];

    assert!(
        ux_tests_body.contains("cargo build -p perl-lsp-rs --bin perl-lsp"),
        "ux-tests must build the perl-lsp binary explicitly so local runs do not depend on a \
         prebuilt artifact.\nCurrent ux-tests recipe:\n{}",
        ux_tests_body
    );
    assert!(
        ux_tests_body.contains("PERL_LSP_BIN={{justfile_directory()}}/target/debug/perl-lsp"),
        "ux-tests must export an absolute PERL_LSP_BIN rooted at the justfile so the harness \
         does not depend on the crate working directory.\n\
         Current ux-tests recipe:\n{}",
        ux_tests_body
    );

    Ok(())
}
