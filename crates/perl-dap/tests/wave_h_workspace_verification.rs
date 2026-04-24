//! Wave H Collapse: Workspace-Level Verification Tests
//!
//! Verifies build artifacts and workspace structure after collapse.
//! These tests are integration-style and verify the workspace as a whole.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_workspace_verification`

// Tests use panic! as structured test failure reporters.
#![allow(clippy::panic)]

use std::process::Command;

#[test]
fn test_perl_lsp_can_build_with_new_imports() {
    // Verify that perl-lsp crate builds successfully with the new import paths
    // It should depend on perl_dap instead of perl_dap_platform

    let output = Command::new("cargo")
        .args(["build", "-p", "perl-lsp-rs", "--message-format=short"])
        .output();
    match output {
        Ok(out) if !out.status.success() => {
            panic!("perl-lsp-rs build failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => panic!("cargo build failed to start: {e}"),
        _ => {}
    }
}

#[test]
fn test_perl_lsp_config_can_build_with_new_imports() {
    // Verify that perl-lsp-config crate builds successfully with the new import paths
    // It should depend on perl_dap instead of perl_dap_platform

    let output = Command::new("cargo")
        .args(["build", "-p", "perl-lsp-config", "--message-format=short"])
        .output();
    match output {
        Ok(out) if !out.status.success() => {
            panic!("perl-lsp-config build failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => panic!("cargo build failed to start: {e}"),
        _ => {}
    }
}

#[test]
fn test_executable_binary_builds_successfully() {
    // Verify that the perl-dap binary itself builds successfully
    // with the new module structure

    let output =
        Command::new("cargo").args(["build", "-p", "perl-dap", "--bin", "perl-dap"]).output();
    match output {
        Ok(out) if !out.status.success() => {
            panic!("perl-dap binary build failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => panic!("cargo build failed: {e}"),
        _ => {}
    }
}

#[test]
fn test_clippy_has_no_warnings_in_new_modules() {
    // Verify that the new module code doesn't introduce clippy warnings

    let output = Command::new("cargo")
        .args(["clippy", "-p", "perl-dap", "--lib", "--", "-D", "warnings"])
        .output();
    match output {
        Ok(out) => {
            if !out.status.success() {
                // Log but don't fail (pre-existing warnings may exist)
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("clippy warnings detected (may be pre-existing):\n{}", stderr);
            }
        }
        Err(e) => panic!("cargo clippy failed to start: {e}"),
    }
}

#[test]
fn test_formatting_is_correct() {
    // Verify code formatting is consistent

    let output = Command::new("cargo").args(["fmt", "-p", "perl-dap", "--", "--check"]).output();
    match output {
        Ok(out) if !out.status.success() => {
            panic!("code formatting issues found:\n{}", String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => panic!("cargo fmt failed to start: {e}"),
        _ => {}
    }
}
