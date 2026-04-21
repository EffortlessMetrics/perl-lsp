//! Edge case test: Verify feature flag routing after G3 absorption.
//!
//! Decision D5 extends rs-core feature `lsp-compat = ["dep:lsp-types"]` and adds
//! `lsp-types = { workspace = true, optional = true }` to dependencies.
//!
//! This test verifies that the feature flag is correctly configured and accessible.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_lsp_compat_feature_extended_with_lsp_types() {
    // Per D5: lsp-compat feature MUST declare lsp-types dependency routing.
    // Spec requirement D5: lsp-compat = ["dep:lsp-types"]
    //
    // Note: lsp-types itself remains unconditional in [dependencies] because
    // multiple modules (capability_map, protocol, providers, tooling, uri) use it internally.
    // The lsp-compat feature is a CONSUMER SIGNAL for dependent crates like perl-lsp-rs.
    //
    // This is a REGRESSION GUARD: verifies D5 feature routing remains implemented.

    let root = workspace_root();
    let core_toml = root.join("crates/perl-lsp-rs-core/Cargo.toml");

    let content = fs::read_to_string(&core_toml).expect("should read core Cargo.toml");

    // Check if lsp-compat = ["dep:lsp-types"] is present (D5 requirement)
    let has_lsp_types_routing = content.contains(r#"lsp-compat = ["dep:lsp-types"]"#);

    assert!(
        has_lsp_types_routing,
        "SPEC VIOLATION D5: lsp-compat feature should declare dep:lsp-types routing. \
                Per context.md D5, must change from 'lsp-compat = []' to 'lsp-compat = [\"dep:lsp-types\"]'"
    );
}

#[test]
fn g3_perl_lsp_binary_removed_dead_feature_refs() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let binary_toml = root.join("crates/perl-lsp/Cargo.toml");

    let content = fs::read_to_string(&binary_toml)?;

    // Verify that dead refs to absorbed crate features are removed
    // (only perl-lsp-rs-core/lsp-ga-lock should remain)
    // Per D5: "remove dead refs to perl-lsp-protocol/lsp-ga-lock and perl-lsp-feature-governance/lsp-ga-lock"

    // Filter out comments when checking for feature refs
    let lines_without_comments: Vec<&str> = content
        .lines()
        .map(|line| if let Some(hash) = line.find('#') { &line[..hash] } else { line })
        .collect();
    let filtered_content = lines_without_comments.join("\n");

    // Verify that dead feature refs are removed (comments don't count)
    let protocol_dead_ref = filtered_content.contains("perl-lsp-protocol/lsp-ga-lock");
    let governance_dead_ref = filtered_content.contains("perl-lsp-feature-governance/lsp-ga-lock");

    assert!(
        !protocol_dead_ref && !governance_dead_ref,
        "perl-lsp/Cargo.toml [features] should not reference protocol or governance (must use rs-core only)"
    );

    // Should still have rs-core reference
    assert!(
        filtered_content.contains("perl-lsp-rs-core") && filtered_content.contains("lsp-ga-lock"),
        "perl-lsp/Cargo.toml should retain perl-lsp-rs-core/lsp-ga-lock reference"
    );

    Ok(())
}

#[test]
fn g3_absorbed_modules_in_public_api() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let lib_rs = root.join("crates/perl-lsp-rs-core/src/lib.rs");

    let content = fs::read_to_string(&lib_rs)?;

    // Verify that all 7 absorbed modules are re-exported from lib.rs
    let modules = vec![
        "governance",
        "protocol",
        "uri",
        "transport",
        "performance",
        "critic_parser",
        "tooling",
    ];

    for module in modules {
        assert!(
            content.contains(&format!("pub mod {}", module))
                || content.contains(&format!("pub use .*{}::", module)),
            "Module {} should be publicly exported from perl_lsp_rs_core lib.rs",
            module
        );
    }

    Ok(())
}
