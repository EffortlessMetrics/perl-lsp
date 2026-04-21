//! NEGATIVE TEST: Verify that `perl-lsp-config` is NOT absorbed (Wave H follow-up).
//!
//! Per Decision D3: `perl-lsp-config` has a hard cycle via `perl-dap`.
//! It must remain published and standalone. This test asserts it is NOT
//! absorbed into perl-lsp-rs-core.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // Tests run from the crate directory; navigate up to workspace root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_config_stays_published_not_absorbed() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Verify that perl-lsp-config/src/lib.rs still exists as a standalone crate
    let config_lib = root.join("crates/perl-lsp-config/src/lib.rs");
    assert!(
        config_lib.exists(),
        "perl-lsp-config/src/lib.rs should still exist (not absorbed into rs-core)"
    );

    // Verify that config is NOT present in rs-core as a module
    let rs_core_config_module = root.join("crates/perl-lsp-rs-core/src/config.rs");
    let rs_core_config_mod_dir = root.join("crates/perl-lsp-rs-core/src/config/");
    assert!(
        !rs_core_config_module.exists() && !rs_core_config_mod_dir.exists(),
        "config should not be absorbed into rs-core as a module"
    );

    // Verify that config Cargo.toml still has publish = true (or no publish field)
    let config_toml = root.join("crates/perl-lsp-config/Cargo.toml");
    let content = fs::read_to_string(&config_toml)?;
    assert!(
        !content.contains("publish = false"),
        "perl-lsp-config/Cargo.toml should not have 'publish = false'"
    );

    Ok(())
}

#[test]
fn g3_config_follows_wave_h_deferred() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Verify that a note or comment exists about config being deferred to Wave H
    // (This is more of a documentation check than a hard assertion)
    // Look for it in the ADR or spec files
    let adr_path = root.join("docs/adr/0041-microcrate-collapse.md");
    assert!(adr_path.exists(), "ADR 0041 should exist and document config deferral");

    let adr_content = fs::read_to_string(&adr_path)?;
    assert!(
        adr_content.contains("perl-lsp-config") && adr_content.contains("Wave H"),
        "ADR should document perl-lsp-config deferral to Wave H"
    );

    Ok(())
}
