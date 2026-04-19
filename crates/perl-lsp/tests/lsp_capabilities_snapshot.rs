//! Capability snapshot tests to prevent drift
//!
//! This test ensures that changes to advertised capabilities are intentional
//! and tracked in changelog

use perl_lsp::protocol::capabilities::{BuildFlags, capabilities_json};
use perl_tdd_support::must;
use serde_json::{Value, json};
use std::path::Path;

/// Snapshot of production capabilities (v0.8.5)
const PRODUCTION_CAPABILITIES_SNAPSHOT: &str =
    include_str!("snapshots/production_capabilities.json");

/// Snapshot of GA-lock capabilities
const GA_LOCK_CAPABILITIES_SNAPSHOT: &str = include_str!("snapshots/ga_lock_capabilities.json");

/// Snapshot of all feature-enabled capabilities used for contract coverage.
const ALL_CAPABILITIES_SNAPSHOT: &str = include_str!("snapshots/all_capabilities.json");

/// Snapshot of feature ids emitted by each capability profile.
const FEATURE_IDS_SNAPSHOT: &str = include_str!("snapshots/capability_profile_feature_ids.json");

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
            2. Validate regeneration with: cargo test -p perl-lsp-rs --test lsp_capabilities_snapshot regenerate_snapshots\n\
            3. Commit the new snapshot\n\n\
            Expected:\n{}\n\n\
            Actual:\n{}",
            expected_pretty, actual_pretty
        )));
    }

    Ok(())
}

#[test]
fn test_all_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let actual = capabilities_json(BuildFlags::all());
    let expected: Value = serde_json::from_str(ALL_CAPABILITIES_SNAPSHOT)?;

    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual)?;
        let expected_pretty = serde_json::to_string_pretty(&expected)?;

        must(Err::<(), _>(format!(
            "All-capabilities snapshot has changed!\n\
            Expected:\n{}\n\n\
            Actual:\n{}",
            expected_pretty, actual_pretty
        )));
    }

    Ok(())
}

#[test]
fn test_capability_profile_feature_ids_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let actual = capability_profile_feature_ids();
    let expected: Value = serde_json::from_str(FEATURE_IDS_SNAPSHOT)?;

    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual)?;
        let expected_pretty = serde_json::to_string_pretty(&expected)?;

        must(Err::<(), _>(format!(
            "Capability profile feature IDs changed!\n\
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

fn capability_profile_feature_ids() -> Value {
    json!({
        "production": BuildFlags::production().to_feature_ids(),
        "ga_lock": BuildFlags::ga_lock().to_feature_ids(),
        "all": BuildFlags::all().to_feature_ids(),
    })
}

fn write_snapshots(snapshots_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    fs::create_dir_all(snapshots_dir)?;

    // Generate production snapshot
    let production_caps = capabilities_json(BuildFlags::production());
    let production_json = serde_json::to_string_pretty(&production_caps)?;
    fs::write(
        snapshots_dir.join("production_capabilities.json"),
        production_json,
    )?;

    // Generate GA lock snapshot
    let ga_lock_caps = capabilities_json(BuildFlags::ga_lock());
    let ga_lock_json = serde_json::to_string_pretty(&ga_lock_caps)?;
    fs::write(
        snapshots_dir.join("ga_lock_capabilities.json"),
        ga_lock_json,
    )?;

    // Generate all-features snapshot
    let all_caps = capabilities_json(BuildFlags::all());
    let all_json = serde_json::to_string_pretty(&all_caps)?;
    fs::write(snapshots_dir.join("all_capabilities.json"), all_json)?;

    // Generate feature-id coverage snapshot
    let feature_ids_json = serde_json::to_string_pretty(&capability_profile_feature_ids())?;
    fs::write(
        snapshots_dir.join("capability_profile_feature_ids.json"),
        feature_ids_json,
    )?;

    Ok(())
}

/// Validates snapshot regeneration logic without mutating repository files.
#[test]
fn regenerate_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // If UPDATE_SNAPSHOTS=1, write to the real snapshots directory
    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let snapshots_dir = manifest_dir.join("tests").join("snapshots");
        write_snapshots(&snapshots_dir)?;
        eprintln!("Snapshots updated in {:?}", snapshots_dir);
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    write_snapshots(temp_dir.path())?;

    let generated_production =
        fs::read_to_string(temp_dir.path().join("production_capabilities.json"))?;
    let generated_ga_lock = fs::read_to_string(temp_dir.path().join("ga_lock_capabilities.json"))?;
    let generated_all = fs::read_to_string(temp_dir.path().join("all_capabilities.json"))?;
    let generated_feature_ids =
        fs::read_to_string(temp_dir.path().join("capability_profile_feature_ids.json"))?;

    let expected_production = serde_json::to_string_pretty(&serde_json::from_str::<Value>(
        PRODUCTION_CAPABILITIES_SNAPSHOT,
    )?)?;
    let expected_ga_lock = serde_json::to_string_pretty(&serde_json::from_str::<Value>(
        GA_LOCK_CAPABILITIES_SNAPSHOT,
    )?)?;
    let expected_all =
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(ALL_CAPABILITIES_SNAPSHOT)?)?;
    let expected_feature_ids =
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(FEATURE_IDS_SNAPSHOT)?)?;

    assert_eq!(
        generated_production, expected_production,
        "regenerated production snapshot should match checked-in snapshot"
    );
    assert_eq!(
        generated_ga_lock, expected_ga_lock,
        "regenerated ga-lock snapshot should match checked-in snapshot"
    );
    assert_eq!(
        generated_all, expected_all,
        "regenerated all-capabilities snapshot should match checked-in snapshot"
    );
    assert_eq!(
        generated_feature_ids, expected_feature_ids,
        "regenerated feature-id snapshot should match checked-in snapshot"
    );

    Ok(())
}
