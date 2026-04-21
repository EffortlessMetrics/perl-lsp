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
fn g3_absorbed_crates_directories_deleted() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Wave G3 implementation choice: absorbed crate directories are DELETED, not kept with publish=false.
    // This diverges from G2 but matches builder's implementation of full absorption cleanup.
    // Regression guard: verify directories are absent (not left behind as stubs).
    let absorbed = vec![
        "crates/perl-lsp-feature-governance",
        "crates/perl-lsp-protocol",
        "crates/perl-lsp-uri",
        "crates/perl-lsp-transport",
        "crates/perl-lsp-performance",
        "crates/perl-lsp-critic-parser",
        "crates/perl-lsp-tooling",
    ];

    for crate_dir in absorbed {
        let dir_path = root.join(crate_dir);
        assert!(!dir_path.exists(), "absorbed crate directory should be deleted: {crate_dir}");
    }

    Ok(())
}
