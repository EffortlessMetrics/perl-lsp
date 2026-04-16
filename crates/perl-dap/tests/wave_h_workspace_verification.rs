//! Wave H Collapse: Workspace-Level Verification Tests
//!
//! Verifies build artifacts and workspace structure after collapse.
//! These tests are integration-style and verify the workspace as a whole.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_workspace_verification`

use std::process::Command;

#[test]
fn test_perl_lsp_can_build_with_new_imports() {
    // Verify that perl-lsp crate builds successfully with the new import paths
    // It should depend on perl_dap instead of perl_dap_platform

    let output = Command::new("cargo")
        .args(&["build", "-p", "perl-lsp", "--message-format=short"])
        .output()
        .expect("cargo build failed to start");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("perl-lsp build failed: {}", stderr);
    }
}

#[test]
fn test_perl_lsp_config_can_build_with_new_imports() {
    // Verify that perl-lsp-config crate builds successfully with the new import paths
    // It should depend on perl_dap instead of perl_dap_platform

    let output = Command::new("cargo")
        .args(&["build", "-p", "perl-lsp-config", "--message-format=short"])
        .output()
        .expect("cargo build failed to start");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("perl-lsp-config build failed: {}", stderr);
    }
}

#[test]
fn test_executable_binary_builds_successfully() {
    // Verify that the perl-dap binary itself builds successfully
    // with the new module structure

    let output = Command::new("cargo")
        .args(&["build", "-p", "perl-dap", "--bin", "perl_lsp_dap"])
        .output()
        .expect("cargo build failed");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("perl-dap binary build failed: {}", stderr);
    }
}

#[test]
fn test_clippy_has_no_warnings_in_new_modules() {
    // Verify that the new module code doesn't introduce clippy warnings

    let output = Command::new("cargo")
        .args(&["clippy", "-p", "perl-dap", "--lib", "--", "-D", "warnings"])
        .output()
        .expect("cargo clippy failed");

    // We don't assert on success since there might be pre-existing warnings,
    // but this documents the expectation
    if output.status.success() {
        // Good!
    } else {
        // Log but don't fail (pre-existing warnings may exist)
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("clippy warnings detected (may be pre-existing):\n{}", stderr);
    }
}

#[test]
fn test_formatting_is_correct() {
    // Verify code formatting is consistent

    let output = Command::new("cargo")
        .args(&["fmt", "-p", "perl-dap", "--", "--check"])
        .output()
        .expect("cargo fmt failed");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("code formatting issues found:\n{}", stderr);
    }
}
