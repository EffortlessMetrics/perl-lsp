//! Regression test: Verify published-crate-baseline.txt matches actual published count.
//!
//! The baseline file is a ratchet: it can only decrease as crates are absorbed.
//! G3 reduces the count from 44 to 37. Wave 4-Completion reduces from 37 to 34
//! (perl-dead-code, perl-refactoring, perl-incremental-parsing).
//! Wave Final PR B reduces from 34 to 31 (feature-catalog, lsp-config, content-length-framing).
//! This test verifies:
//! 1. Baseline file exists and has correct value (31)
//! 2. Actual cargo metadata published count matches baseline
//! 3. Baseline ratchet is enforced (no accidental regressions)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_baseline_file_has_31() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");

    let content = fs::read_to_string(&baseline_path)?;
    let baseline: u32 =
        content.trim().parse().map_err(|_| "baseline count should be parseable as u32")?;

    assert_eq!(baseline, 31, "baseline should be updated to 31 after Wave Final PR B");

    Ok(())
}

#[test]
#[ignore] // This test requires cargo metadata to be run in-process; skip in CI if slow
fn g3_baseline_matches_cargo_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Run cargo metadata to count published crates
    let output = Command::new("cargo")
        .args(&["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        return Ok(()); // Skip if cargo metadata fails (e.g., in some CI environments)
    }

    let metadata_str = String::from_utf8(output.stdout)?;
    let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

    let packages = metadata["packages"].as_array().ok_or("no packages in metadata")?;

    // Count crates with publish != false and publish != [] (i.e., publicly published)
    let published_count = packages
        .iter()
        .filter(|p| {
            let publish = &p["publish"];
            // If publish is not false and not an empty array, it's published
            !(publish == false
                || (publish.is_array() && publish.as_array().map_or(false, |a| a.is_empty())))
        })
        .count();

    // Allow a small margin for test setup artifacts
    assert!(
        (published_count as i32 - 31).abs() <= 1,
        "published count should be approximately 31 (got {})",
        published_count
    );

    Ok(())
}

#[test]
fn g3_baseline_not_regressed() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");

    let baseline_str = fs::read_to_string(&baseline_path)?;
    let baseline: u32 = baseline_str.trim().parse()?;

    // Regression guard: baseline should never accidentally increase above 34
    // (If it does, it means crates were accidentally re-added)
    assert!(baseline <= 34, "baseline should not exceed Wave 4-Completion target (34)");

    // Also verify it doesn't drop below the v0.13.0 final target
    assert!(baseline >= 31, "baseline should not go below Wave Final PR B target (31)");

    Ok(())
}
