//! Integration test: Verify published crate count is 37 after Wave G3 absorption.
//!
//! Wave G3 absorbs 7 crates: governance, protocol, uri, transport, performance,
//! critic-parser, tooling. Reduces published count from 44 → 37.
//! Config and content-length-framing remain published per D3/D4.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_published_count_is_37() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Read xtask/published-crate-baseline.txt
    let baseline_path = root.join("xtask/published-crate-baseline.txt");
    assert!(
        baseline_path.exists(),
        "baseline file should exist at xtask/published-crate-baseline.txt"
    );

    let content = fs::read_to_string(&baseline_path)?;
    let count: u32 = content.trim().parse().map_err(|_| "failed to parse baseline count as u32")?;

    assert_eq!(count, 37, "published crate count should be 37 after Wave G3");

    Ok(())
}

#[test]
fn g3_absorbed_crates_are_in_workspace_but_unpublished() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

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
        let toml_path = root.join(crate_toml);
        assert!(
            toml_path.exists(),
            "absorbed crate should still exist (kept as workspace member): {crate_toml}"
        );

        let content = fs::read_to_string(&toml_path)?;
        assert!(
            content.contains("publish = false"),
            "absorbed crate should have 'publish = false' set: {crate_toml}"
        );
    }

    Ok(())
}
