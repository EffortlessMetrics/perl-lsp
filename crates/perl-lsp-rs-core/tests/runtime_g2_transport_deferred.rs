//! Wave G2/G3 transport state verification.
//!
//! At G2: transport was deferred (cycle block: protocol → rs-core, transport → protocol)
//! At G3: transport IS absorbed into perl-lsp-rs-core::transport (cycle resolved by protocol absorption)
//!
//! This file documents the G2 → G3 transition and updates tests accordingly:
//! - G2 tests verified transport remained standalone
//! - G3 tests verify transport is now ABSORBED and DELETED from workspace
//!
//! NOTE: These tests were written for G2 (post-collapse, pre-G3 absorption).
//! They have been updated to reflect G3 absorption per acceptance criteria.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // Tests run from the workspace root
    // CARGO_MANIFEST_DIR is crates/perl-lsp-rs-core, so go up 2 levels
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Test that the transport crate directory is DELETED (absorbed in G3).
/// Updated for G3: transport is now fully absorbed into perl-lsp-rs-core::transport
/// and the standalone crate directory has been removed.
#[test]
fn test_transport_crate_directory_exists() -> Result<(), Box<dyn std::error::Error>> {
    let transport_path = repo_root().join("crates/perl-lsp-transport");
    assert!(
        !transport_path.exists(),
        "perl-lsp-transport directory should be DELETED (absorbed into rs-core in G3)"
    );
    Ok(())
}

/// Test that transport Cargo.toml is DELETED (absorbed in G3).
/// Updated for G3: transport no longer exists as standalone published crate.
#[test]
fn test_transport_cargo_toml_exists() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = repo_root().join("crates/perl-lsp-transport/Cargo.toml");
    assert!(
        !cargo_toml.exists(),
        "perl-lsp-transport/Cargo.toml should be DELETED (crate absorbed in G3)"
    );
    Ok(())
}

/// Test that transport src/lib.rs is DELETED (absorbed in G3).
/// Updated for G3: transport source is now part of perl-lsp-rs-core.
#[test]
fn test_transport_lib_rs_exists() -> Result<(), Box<dyn std::error::Error>> {
    let lib_rs = repo_root().join("crates/perl-lsp-transport/src/lib.rs");
    assert!(
        !lib_rs.exists(),
        "perl-lsp-transport/src/lib.rs should be DELETED (source absorbed into rs-core in G3)"
    );
    Ok(())
}

/// Test that transport is NOW ABSORBED into perl-lsp-rs-core::transport (G3).
/// Updated for G3: transport module is accessible from rs-core, not runtime.
#[test]
fn test_runtime_transport_not_absorbed() -> Result<(), Box<dyn std::error::Error>> {
    // Regression guard: transport should be accessible from rs-core::transport.
    // Touch a concrete transport item so this check stays meaningful and warning-free.
    let _type_name =
        std::any::type_name::<perl_lsp_rs_core::transport::ContentLengthMessageReader>();
    Ok(())
}

/// Test that transport is REMOVED from workspace members (absorbed).
/// Updated for G3: standalone transport crate no longer exists.
#[test]
fn test_transport_in_workspace_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_toml = std::fs::read_to_string(repo_root().join("Cargo.toml"))?;

    // Filter out comments to check for actual workspace members
    let lines_without_comments: Vec<&str> = workspace_toml
        .lines()
        .map(|line| if let Some(hash) = line.find('#') { &line[..hash] } else { line })
        .collect();
    let filtered = lines_without_comments.join("\n");

    assert!(
        !filtered.contains("perl-lsp-transport"),
        "perl-lsp-transport should NOT be listed in workspace members (absorbed in G3)"
    );
    Ok(())
}

/// Test that transport tests/ directory is DELETED (absorbed in G3).
/// Updated for G3: transport tests are now part of rs-core test suite.
#[test]
fn test_transport_tests_directory_exists() -> Result<(), Box<dyn std::error::Error>> {
    let tests_path = repo_root().join("crates/perl-lsp-transport/tests");
    assert!(
        !tests_path.exists(),
        "perl-lsp-transport/tests directory should be DELETED (tests absorbed in G3)"
    );
    Ok(())
}

/// Test that transport README is DELETED (absorbed in G3).
/// Updated for G3: documentation is now part of rs-core.
#[test]
fn test_transport_readme_exists() -> Result<(), Box<dyn std::error::Error>> {
    let readme = repo_root().join("crates/perl-lsp-transport/README.md");
    assert!(!readme.exists(), "perl-lsp-transport/README.md should be DELETED (absorbed in G3)");
    Ok(())
}

/// Test that transport src/framing.rs is DELETED (absorbed in G3).
/// Updated for G3: framing module is now part of perl-lsp-rs-core::transport.
#[test]
fn test_transport_framing_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    let framing = repo_root().join("crates/perl-lsp-transport/src/framing.rs");
    assert!(
        !framing.exists(),
        "perl-lsp-transport/src/framing.rs should be DELETED (absorbed into rs-core in G3)"
    );
    Ok(())
}

/// Test that transport crate NO LONGER EXISTS (absorbed in G3).
/// Updated for G3: transport has been fully absorbed into perl-lsp-rs-core.
#[test]
fn test_transport_is_published() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml_path = repo_root().join("crates/perl-lsp-transport/Cargo.toml");
    assert!(
        !cargo_toml_path.exists(),
        "perl-lsp-transport/Cargo.toml should NOT exist (crate absorbed in G3)"
    );
    Ok(())
}

/// Test that transport no longer has external protocol dependency (absorbed in G3).
/// Updated for G3: transport is now part of rs-core, so the external dependency is gone.
#[test]
fn test_transport_depends_on_protocol() -> Result<(), Box<dyn std::error::Error>> {
    // Regression guard: after G3 absorption, transport crate doesn't exist anymore
    let cargo_toml_path = repo_root().join("crates/perl-lsp-transport/Cargo.toml");
    assert!(
        !cargo_toml_path.exists(),
        "perl-lsp-transport should NOT exist as standalone crate (absorbed in G3)"
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
