//! NEGATIVE TEST: Verify that `perl-content-length-framing` is NOT absorbed.
//!
//! Per Decision D4: `perl-content-length-framing` is used by multiple consumers
//! (perl-dap, perl-lsp-transport, perl-lsp binary) and must remain published.
//! It is added as a direct dependency of perl-lsp-rs-core, but NOT absorbed.

use std::fs;
use std::path::Path;

#[test]
fn g3_content_length_framing_stays_published() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that perl-content-length-framing/src/lib.rs still exists as a standalone crate
    let framing_lib = "crates/perl-content-length-framing/src/lib.rs";
    assert!(
        Path::new(framing_lib).exists(),
        "perl-content-length-framing/src/lib.rs should still exist (not absorbed into rs-core)"
    );

    // Verify that framing is NOT present in rs-core as a module
    let rs_core_framing_module = "crates/perl-lsp-rs-core/src/content_length_framing.rs";
    let rs_core_framing_mod_dir = "crates/perl-lsp-rs-core/src/content_length_framing/";
    assert!(
        !Path::new(rs_core_framing_module).exists() && !Path::new(rs_core_framing_mod_dir).exists(),
        "content_length_framing should not be absorbed into rs-core as a module"
    );

    // Verify that framing Cargo.toml still has publish = true (or no publish field)
    let framing_toml = "crates/perl-content-length-framing/Cargo.toml";
    let content = fs::read_to_string(framing_toml)?;
    assert!(
        !content.contains("publish = false"),
        "perl-content-length-framing/Cargo.toml should not have 'publish = false'"
    );

    Ok(())
}

#[test]
fn g3_content_length_framing_added_to_rs_core_deps() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that perl-content-length-framing is added as a direct dependency of rs-core
    let rs_core_toml = "crates/perl-lsp-rs-core/Cargo.toml";
    let content = fs::read_to_string(rs_core_toml)?;

    assert!(
        content.contains("perl-content-length-framing"),
        "perl-lsp-rs-core/Cargo.toml should include perl-content-length-framing as a dependency (per D4)"
    );

    Ok(())
}

#[test]
fn g3_shared_consumers_can_use_framing() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that the three known consumers (perl-dap, transport, perl-lsp binary)
    // can still use perl-content-length-framing
    // This is a smoke test: verify dap and lsp Cargo.tomls reference it or rs-core
    let dap_toml = "crates/perl-dap/Cargo.toml";
    let lsp_toml = "crates/perl-lsp/Cargo.toml";

    // At least one should reference framing directly or indirectly via rs-core
    let dap_content = fs::read_to_string(dap_toml)?;
    let lsp_content = fs::read_to_string(lsp_toml)?;

    assert!(
        dap_content.contains("perl-content-length-framing")
            || dap_content.contains("perl-lsp-rs-core"),
        "perl-dap should use perl-content-length-framing or rs-core"
    );

    assert!(
        lsp_content.contains("perl-content-length-framing")
            || lsp_content.contains("perl-lsp-rs-core"),
        "perl-lsp should use perl-content-length-framing or rs-core"
    );

    Ok(())
}
