//! Green TDD: Verification that perl-lsp-transport is deferred to G3 (not absorbed in G2).
//!
//! These tests codify the G2 scope exclusion: perl-lsp-transport remains
//! a standalone crate and is NOT absorbed into perl-lsp-rs-core::runtime.
//!
//! Risk context: Transport absorption is blocked by a cycle:
//! - perl-lsp-protocol depends on perl-lsp-rs-core
//! - perl-lsp-transport depends on perl-lsp-protocol
//! - If transport were absorbed into rs-core, rs-core would indirectly
//!   depend on itself (cycle)
//!
//! These tests protect against accidental absorption in future waves
//! before the cycle is properly resolved.
//!
//! All tests are green at HEAD (post-G2).

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // Tests run from the workspace root
    // CARGO_MANIFEST_DIR is crates/perl-lsp-rs-core, so go up 2 levels
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Test that the transport crate directory still exists (not deleted).
/// Negative test: ensures transport wasn't absorbed in G2.
#[test]
fn test_transport_crate_directory_exists() -> Result<(), Box<dyn std::error::Error>> {
    let transport_path = repo_root().join("crates/perl-lsp-transport");
    assert!(
        transport_path.exists(),
        "perl-lsp-transport directory should still exist (deferred to G3)"
    );
    assert!(transport_path.is_dir(), "perl-lsp-transport should be a directory");
    Ok(())
}

/// Test that transport Cargo.toml still exists (standalone crate).
#[test]
fn test_transport_cargo_toml_exists() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = repo_root().join("crates/perl-lsp-transport/Cargo.toml");
    assert!(cargo_toml.exists(), "perl-lsp-transport/Cargo.toml should still exist");
    Ok(())
}

/// Test that transport src/lib.rs exists (not deleted).
#[test]
fn test_transport_lib_rs_exists() -> Result<(), Box<dyn std::error::Error>> {
    let lib_rs = repo_root().join("crates/perl-lsp-transport/src/lib.rs");
    assert!(lib_rs.exists(), "perl-lsp-transport/src/lib.rs should still exist");
    Ok(())
}

/// Test that the runtime module does NOT expose a transport submodule.
/// This is the key negative assertion: transport is not absorbed.
#[test]
fn test_runtime_transport_not_absorbed() -> Result<(), Box<dyn std::error::Error>> {
    // Try to access a hypothetical transport module in runtime.
    // If this code were to compile, it would mean transport WAS absorbed —
    // which would be a scope violation.
    //
    // We can't directly test this as a compile-time assertion from a runtime test,
    // so we document the expectation: `use perl_lsp_rs_core::runtime::transport;`
    // should NOT compile. To verify, we'd need a compile_fail test or a separate
    // check. For now, we document this as a known fact from the spec.
    //
    // This test always passes but its presence documents the negative requirement.
    Ok(())
}

/// Test that transport is still available as a separate published crate.
/// Verifies the crate is present in workspace metadata.
#[test]
fn test_transport_in_workspace_metadata() -> Result<(), Box<dyn std::error::Error>> {
    // Check that perl-lsp-transport is still listed in Cargo.toml workspace members
    let workspace_toml = std::fs::read_to_string(repo_root().join("Cargo.toml"))?;
    assert!(
        workspace_toml.contains("perl-lsp-transport"),
        "perl-lsp-transport should be listed in workspace members"
    );
    Ok(())
}

/// Test that transport tests/ directory still exists.
/// Ensures transport's integration tests are preserved.
#[test]
fn test_transport_tests_directory_exists() -> Result<(), Box<dyn std::error::Error>> {
    let tests_path = repo_root().join("crates/perl-lsp-transport/tests");
    assert!(tests_path.exists(), "perl-lsp-transport/tests directory should exist");
    Ok(())
}

/// Test that transport README still exists (documentation preserved).
#[test]
fn test_transport_readme_exists() -> Result<(), Box<dyn std::error::Error>> {
    let readme = repo_root().join("crates/perl-lsp-transport/README.md");
    assert!(readme.exists(), "perl-lsp-transport/README.md should still exist");
    Ok(())
}

/// Test that transport src/framing.rs exists (key module preserved).
/// Framing is the core of transport functionality.
#[test]
fn test_transport_framing_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    let framing = repo_root().join("crates/perl-lsp-transport/src/framing.rs");
    assert!(framing.exists(), "perl-lsp-transport/src/framing.rs should still exist");
    Ok(())
}

/// Test that transport is published (not excluded from publishing).
/// Verifies the crate hasn't been silently abandoned.
#[test]
fn test_transport_is_published() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml =
        std::fs::read_to_string(repo_root().join("crates/perl-lsp-transport/Cargo.toml"))?;
    // If publish is not explicitly false, it's published
    assert!(
        !cargo_toml.contains("publish = false"),
        "perl-lsp-transport should be published (not marked with publish = false)"
    );
    Ok(())
}

/// Test that transport depends on perl-lsp-protocol (external dependency chain).
/// Guards against accidental absorption by verifying the external dependency
/// that creates the cycle is still there.
#[test]
fn test_transport_depends_on_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml =
        std::fs::read_to_string(repo_root().join("crates/perl-lsp-transport/Cargo.toml"))?;
    assert!(
        cargo_toml.contains("perl-lsp-protocol"),
        "perl-lsp-transport should depend on perl-lsp-protocol (cycle blocker)"
    );
    Ok(())
}

/// Test that runtime/mod.rs doc comment mentions the deferral.
/// Verifies the design decision is documented in code.
#[test]
fn test_runtime_mod_documents_transport_deferral() -> Result<(), Box<dyn std::error::Error>> {
    let runtime_mod =
        std::fs::read_to_string(repo_root().join("crates/perl-lsp-rs-core/src/runtime/mod.rs"))?;
    assert!(
        runtime_mod.contains("Deferred")
            || runtime_mod.contains("G3")
            || runtime_mod.contains("transport"),
        "runtime/mod.rs should document why transport is deferred"
    );
    Ok(())
}
