//! Capability snapshot tests to prevent drift
//!
//! This test ensures that changes to advertised capabilities are intentional
//! and tracked in changelog

use perl_lsp::protocol::capabilities::{BuildFlags, capabilities_json};
use perl_tdd_support::must;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Snapshot of production capabilities (v0.8.5)
const PRODUCTION_CAPABILITIES_SNAPSHOT: &str =
    include_str!("snapshots/production_capabilities.json");

/// Snapshot of GA-lock capabilities
const GA_LOCK_CAPABILITIES_SNAPSHOT: &str = include_str!("snapshots/ga_lock_capabilities.json");

/// Snapshot of all in-tree capabilities used by CI and tooling.
const ALL_CAPABILITIES_SNAPSHOT: &str = include_str!("snapshots/all_capabilities.json");

struct SnapshotCase<'a> {
    name: &'a str,
    flags: BuildFlags,
    expected: &'a str,
    output_file: &'a str,
    drift_message: &'a str,
}

fn snapshot_cases() -> Vec<SnapshotCase<'static>> {
    vec![
        SnapshotCase {
            name: "production",
            flags: BuildFlags::production(),
            expected: PRODUCTION_CAPABILITIES_SNAPSHOT,
            output_file: "production_capabilities.json",
            drift_message: "Production capabilities have changed!\n\
                If this is intentional:\n\
                1. Update the changelog\n\
                2. Validate regeneration with: cargo test -p perl-lsp --test lsp_capabilities_snapshot regenerate_snapshots\n\
                3. Commit the new snapshot",
        },
        SnapshotCase {
            name: "ga-lock",
            flags: BuildFlags::ga_lock(),
            expected: GA_LOCK_CAPABILITIES_SNAPSHOT,
            output_file: "ga_lock_capabilities.json",
            drift_message: "GA-lock capabilities have changed!\n\
                This should NEVER change without a major version bump.",
        },
        SnapshotCase {
            name: "all",
            flags: BuildFlags::all(),
            expected: ALL_CAPABILITIES_SNAPSHOT,
            output_file: "all_capabilities.json",
            drift_message: "All-capabilities snapshot has changed!\n\
                Review CI/tooling expectations and commit the refreshed snapshot if intentional.",
        },
    ]
}

fn compare_snapshot(case: &SnapshotCase<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let actual = capabilities_json(case.flags.clone());
    let expected: Value = serde_json::from_str(case.expected)?;

    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual)?;
        let expected_pretty = serde_json::to_string_pretty(&expected)?;

        must(Err::<(), _>(format!(
            "{}\n\nExpected:\n{}\n\nActual:\n{}",
            case.drift_message, expected_pretty, actual_pretty
        )));
    }

    Ok(())
}

fn expected_snapshot_paths(root: &Path) -> Vec<PathBuf> {
    snapshot_cases().into_iter().map(|case| root.join(case.output_file)).collect()
}

#[test]
fn test_production_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    compare_snapshot(&snapshot_cases()[0])
}

#[test]
fn test_ga_lock_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    compare_snapshot(&snapshot_cases()[1])
}

#[test]
fn test_all_capabilities_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    compare_snapshot(&snapshot_cases()[2])
}

fn write_snapshots(snapshots_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(snapshots_dir)?;

    for case in snapshot_cases() {
        let rendered = serde_json::to_string_pretty(&capabilities_json(case.flags))?;
        fs::write(snapshots_dir.join(case.output_file), rendered)?;
    }

    Ok(())
}

/// Validates snapshot regeneration logic without mutating repository files.
#[test]
fn regenerate_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    write_snapshots(temp_dir.path())?;

    for case in snapshot_cases() {
        let generated = fs::read_to_string(temp_dir.path().join(case.output_file))?;
        let expected =
            serde_json::to_string_pretty(&serde_json::from_str::<Value>(case.expected)?)?;

        assert_eq!(
            generated, expected,
            "regenerated {} snapshot should match checked-in snapshot",
            case.name
        );
    }

    let mut generated_files = expected_snapshot_paths(temp_dir.path());
    generated_files.sort();

    let mut expected_files = vec![
        temp_dir.path().join("all_capabilities.json"),
        temp_dir.path().join("ga_lock_capabilities.json"),
        temp_dir.path().join("production_capabilities.json"),
    ];
    expected_files.sort();

    assert_eq!(
        generated_files, expected_files,
        "regeneration should cover every tracked capability snapshot"
    );

    Ok(())
}
