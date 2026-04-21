//! Integration test: Verify published crate count is 37 after Wave G3 absorption.
//!
//! Wave G3 absorbs 7 crates: governance, protocol, uri, transport, performance,
//! critic-parser, tooling. Reduces published count from 44 → 37.
//! Config and content-length-framing remain published per D3/D4.

use std::fs;
use std::path::Path;

#[test]
fn g3_published_count_is_37() -> Result<(), Box<dyn std::error::Error>> {
    // Read xtask/published-crate-baseline.txt
    let baseline_path = "xtask/published-crate-baseline.txt";
    assert!(
        Path::new(baseline_path).exists(),
        "baseline file should exist at xtask/published-crate-baseline.txt"
    );

    let content = fs::read_to_string(baseline_path)?;
    let count: u32 = content.trim().parse().map_err(|_| "failed to parse baseline count as u32")?;

    assert_eq!(count, 37, "published crate count should be 37 after Wave G3");

    Ok(())
}

#[test]
fn g3_absorbed_crates_are_in_workspace_but_unpublished() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that absorbed crates still exist but have publish = false
    let absorbed = vec![
        "crates/perl-lsp-feature-governance/Cargo.toml",
        "crates/perl-lsp-protocol/Cargo.toml",
        "crates/perl-lsp-uri/Cargo.toml",
        "crates/perl-lsp-transport/Cargo.toml",
        "crates/perl-lsp-performance/Cargo.toml",
        "crates/perl-lsp-critic-parser/Cargo.toml",
        "crates/perl-lsp-tooling/Cargo.toml",
    ];

    for crate_toml in absorbed {
        assert!(
            Path::new(crate_toml).exists(),
            "absorbed crate should still exist (kept as workspace member)"
        );

        let content = fs::read_to_string(crate_toml)?;
        assert!(
            content.contains("publish = false"),
            "absorbed crate should have 'publish = false' set"
        );
    }

    Ok(())
}
