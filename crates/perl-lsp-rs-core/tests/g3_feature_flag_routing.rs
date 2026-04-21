//! Edge case test: Verify feature flag routing after G3 absorption.
//!
//! Decision D5 specified optional gating of lsp-types via lsp-compat feature.
//! Orchestrator decision (Option A): Keep lsp-types as required, lsp-compat as signal feature.
//! Rationale: rs-core uses lsp_types unconditionally in 5+ modules (capability_map, protocol,
//! providers, tooling, uri), making conditional compilation invasive. Real optional-gating
//! (WASM-style builds) is deferred as a follow-up issue.
//!
//! These tests verify the Option A approach: lsp-types required, lsp-compat empty but present.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_lsp_compat_feature_signal_not_gating() {
    // Orchestrator decision (Option A) from PR #4539:
    // Keep lsp-types as REQUIRED, not optional. Keep lsp-compat as an empty SIGNAL feature.
    //
    // Rationale: capability_map, protocol, providers, tooling, and uri modules all use
    // lsp_types unconditionally. Making it optional requires invasive per-module cfg gating.
    // The lsp-compat feature is a consumer signal for dependent crates like perl-lsp-rs
    // that need compatibility tracking. Real optional-gating for WASM-style builds is
    // deferred as a follow-up issue.
    //
    // This test is a REGRESSION GUARD: verifies that lsp-compat feature exists as a signal,
    // and lsp-types is required (not optional).

    let root = workspace_root();
    let core_toml = root.join("crates/perl-lsp-rs-core/Cargo.toml");

    let content = fs::read_to_string(&core_toml).expect("should read core Cargo.toml");

    // Check that lsp-compat feature exists (as empty signal, not gating)
    let has_lsp_compat = content.contains("lsp-compat = []");
    assert!(
        has_lsp_compat,
        "lsp-compat feature should exist as a signal feature: 'lsp-compat = []'"
    );

    // Check that lsp-types is required (not optional)
    let has_lsp_types_required = content.contains("lsp-types.workspace = true")
        || (content.contains("lsp-types = { workspace = true }")
            && !content.contains("lsp-types = { workspace = true, optional = true }"));
    assert!(
        has_lsp_types_required,
        "lsp-types should be a required dependency (not optional). Follow-up: implement optional gating for WASM-style builds."
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
        // Check for `pub mod <module>` (direct sub-module declaration).
        // Note: `pub use` re-export paths use plain string matching on the module name;
        // the `.*` form below is a literal string match, not a regex, so we check for
        // the concrete patterns that actually appear in lib.rs.
        let declared_as_mod = content.contains(&format!("pub mod {}", module));
        let reexported = content.contains(&format!("pub use {}::", module))
            || content.contains(&format!("pub use crate::{}::", module));
        assert!(
            declared_as_mod || reexported,
            "Module {} should be publicly declared (pub mod) or re-exported (pub use) from perl_lsp_rs_core lib.rs",
            module
        );
    }

    Ok(())
}
