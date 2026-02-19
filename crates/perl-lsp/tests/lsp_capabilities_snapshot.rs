//! Capability snapshot tests to prevent drift
//!
//! This test ensures that changes to advertised capabilities are intentional
//! and tracked in changelog

use perl_lsp::protocol::capabilities::{BuildFlags, capabilities_json};
use perl_tdd_support::must;
use serde_json::Value;
use std::path::Path;

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
            2. Validate regeneration with: cargo test -p perl-lsp --test lsp_capabilities_snapshot regenerate_snapshots\n\
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

fn write_snapshots(snapshots_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    fs::create_dir_all(snapshots_dir)?;

    // Generate production snapshot
    let production_caps = capabilities_json(BuildFlags::production());
    let production_json = serde_json::to_string_pretty(&production_caps)?;
    fs::write(snapshots_dir.join("production_capabilities.json"), production_json)?;

    // Generate GA lock snapshot
    let ga_lock_caps = capabilities_json(BuildFlags::ga_lock());
    let ga_lock_json = serde_json::to_string_pretty(&ga_lock_caps)?;
    fs::write(snapshots_dir.join("ga_lock_capabilities.json"), ga_lock_json)?;

    Ok(())
}

/// Validates snapshot regeneration logic without mutating repository files.
#[test]
fn regenerate_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let temp_dir = tempfile::tempdir()?;
    write_snapshots(temp_dir.path())?;

    let generated_production =
        fs::read_to_string(temp_dir.path().join("production_capabilities.json"))?;
    let generated_ga_lock = fs::read_to_string(temp_dir.path().join("ga_lock_capabilities.json"))?;

    let expected_production = serde_json::to_string_pretty(&serde_json::from_str::<Value>(
        PRODUCTION_CAPABILITIES_SNAPSHOT,
    )?)?;
    let expected_ga_lock = serde_json::to_string_pretty(&serde_json::from_str::<Value>(
        GA_LOCK_CAPABILITIES_SNAPSHOT,
    )?)?;

    assert_eq!(
        generated_production, expected_production,
        "regenerated production snapshot should match checked-in snapshot"
    );
    assert_eq!(
        generated_ga_lock, expected_ga_lock,
        "regenerated ga-lock snapshot should match checked-in snapshot"
    );

    Ok(())
}
