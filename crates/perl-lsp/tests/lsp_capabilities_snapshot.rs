//! Capability snapshot tests to prevent drift
//!
//! This test ensures that changes to advertised capabilities are intentional
//! and tracked in changelog

use perl_lsp::protocol::capabilities::{BuildFlags, capabilities_json};
use serde_json::Value;
use perl_tdd_support::must;

/// Snapshot of production capabilities (v0.8.5)
const PRODUCTION_CAPABILITIES_SNAPSHOT: &str =
    include_str!("snapshots/production_capabilities.json");

/// Snapshot of GA-lock capabilities
const GA_LOCK_CAPABILITIES_SNAPSHOT: &str = include_str!("snapshots/ga_lock_capabilities.json");

#[test]
fn test_production_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let actual = capabilities_json(BuildFlags::production());
    let expected: Value = serde_json::from_str(PRODUCTION_CAPABILITIES_SNAPSHOT)?;

    if actual != expected {
        // Pretty print the diff for debugging
        let actual_pretty = serde_json::to_string_pretty(&actual)?;
        let expected_pretty = serde_json::to_string_pretty(&expected)?;

        must(Err::<(), _>(format!(
            "Production capabilities have changed!\n\
            If this is intentional:\n\
            1. Update the changelog\n\
            2. Regenerate snapshot with: cargo test --test lsp_capabilities_snapshot -- --ignored\n\
            3. Commit the new snapshot\n\n\
            Expected:\n{}\n\n\
            Actual:\n{}",
            expected_pretty, actual_pretty
        )));
    }

    Ok(())
}

#[test]
fn test_ga_lock_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let actual = capabilities_json(BuildFlags::ga_lock());
    let expected: Value = serde_json::from_str(GA_LOCK_CAPABILITIES_SNAPSHOT)?;

    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual)?;
        let expected_pretty = serde_json::to_string_pretty(&expected)?;

        must(Err::<(), _>(format!(
            "GA-lock capabilities have changed!\n\
            This should NEVER change without a major version bump.\n\n\
            Expected:\n{}\n\n\
            Actual:\n{}",
            expected_pretty, actual_pretty
        )));
    }

    Ok(())
}

/// Helper test to regenerate snapshots (run with --ignored)
#[test]
#[ignore = "MANUAL: Regenerate with: cargo test -p perl-lsp --test lsp_capabilities_snapshot regenerate -- --ignored"]
fn regenerate_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    let snapshots_dir = Path::new("tests/snapshots");
    fs::create_dir_all(snapshots_dir)?;

    // Generate production snapshot
    let production_caps = capabilities_json(BuildFlags::production());
    let production_json = serde_json::to_string_pretty(&production_caps)?;
    fs::write(snapshots_dir.join("production_capabilities.json"), production_json)?;

    // Generate GA lock snapshot
    let ga_lock_caps = capabilities_json(BuildFlags::ga_lock());
    let ga_lock_json = serde_json::to_string_pretty(&ga_lock_caps)?;
    fs::write(snapshots_dir.join("ga_lock_capabilities.json"), ga_lock_json)?;

    println!("Snapshots regenerated successfully!");
    println!("Please review the changes and commit if intentional.");

    Ok(())
}
