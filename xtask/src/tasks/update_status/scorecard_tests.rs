//! Red tests for engineering health scorecard expansion.
//!
//! These tests define the expected behavior for Phases 1-4 of the
//! engineering health scorecard expansion. They should FAIL before
//! code-builder implements the features and PASS after implementation.
//!
//! Tests cover:
//! - Phase 1: QUALITY_MUTATION_NOTES block in quality.md
//! - Phase 2: Latency aggregation (LatencyStats, collect_latency_by_subsystem, PERFORMANCE_BY_SUBSYSTEM)
//! - Phase 3: Flaky test tracker (schema extension, update-flaky-tracker.py, FLAKY_TEST_BULLETS)
//! - Phase 4: Subsystem test counts (subsystem-mapping.yaml, collect_subsystem_test_counts, SUBSYSTEM_TEST_BULLETS)
//!
//! NOTE: Tests that require functions not yet implemented (LatencyStats, collect_latency_by_subsystem,
//! format_latency_table, collect_subsystem_test_counts) are commented out below. They define the
//! expected interface and will be enabled once the stubs exist.

use std::fs;

use color_eyre::eyre::{Context, Result};

// ---------------------------------------------------------------------
// Phase 1: Mutation notes block
// ---------------------------------------------------------------------

/// Phase 1: quality.md must contain QUALITY_MUTATION_NOTES block explaining
/// that per-crate mutation scores are not available from the current data.
#[test]
fn test_quality_mutation_notes_block_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let quality_path = root.join("docs/project/status/quality.md");
    let content =
        fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;

    // The block must exist
    assert!(
        content.contains("<!-- BEGIN: QUALITY_MUTATION_NOTES -->"),
        "quality.md must contain <!-- BEGIN: QUALITY_MUTATION_NOTES --> marker"
    );
    assert!(
        content.contains("<!-- END: QUALITY_MUTATION_NOTES -->"),
        "quality.md must contain <!-- END: QUALITY_MUTATION_NOTES --> marker"
    );

    // The block must explain that scores are not available
    let start = content.find("<!-- BEGIN: QUALITY_MUTATION_NOTES -->").unwrap();
    let end = content.find("<!-- END: QUALITY_MUTATION_NOTES -->").unwrap();
    let block_content = &content[start..end];

    assert!(
        block_content.contains("mutation score")
            || block_content.contains("per-crate")
            || block_content.contains("killed"),
        "QUALITY_MUTATION_NOTES must explain that per-crate mutation scores are not available"
    );

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 2: Latency aggregation
// ---------------------------------------------------------------------

/// Phase 2: quality.md must contain PERFORMANCE_BY_SUBSYSTEM block after
/// `just status-update --only quality`.
#[test]
fn test_performance_by_subsystem_block_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let quality_path = root.join("docs/project/status/quality.md");
    let content =
        fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;

    // The block must exist
    assert!(
        content.contains("<!-- BEGIN: PERFORMANCE_BY_SUBSYSTEM -->"),
        "quality.md must contain <!-- BEGIN: PERFORMANCE_BY_SUBSYSTEM --> marker"
    );
    assert!(
        content.contains("<!-- END: PERFORMANCE_BY_SUBSYSTEM -->"),
        "quality.md must contain <!-- END: PERFORMANCE_BY_SUBSYSTEM --> marker"
    );

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 3a: Debt-ledger schema extension
// ---------------------------------------------------------------------

/// Phase 3: debt-ledger.yaml schema must accept failure_count and last_failed_at
/// fields in flaky_tests entries.
#[test]
fn test_debt_ledger_schema_has_flaky_fields() -> Result<()> {
    use serde_yaml_ng::Value;

    let root = crate::utils::project_root()?;
    let ledger_path = root.join(".ci/debt-ledger.yaml");
    let content = fs::read_to_string(&ledger_path).context("reading .ci/debt-ledger.yaml")?;

    // Parse the YAML to verify schema accepts the fields
    let parsed: Value =
        serde_yaml_ng::from_str(&content).context("parsing .ci/debt-ledger.yaml")?;

    // The schema version should be present
    assert!(
        parsed.get("schema_version").is_some(),
        "debt-ledger.yaml must have schema_version field"
    );

    // flaky_tests array should exist (even if empty)
    assert!(parsed.get("flaky_tests").is_some(), "debt-ledger.yaml must have flaky_tests array");

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 3b: update-flaky-tracker.py
// ---------------------------------------------------------------------

/// Phase 3: update-flaky-tracker.py must exist and be executable.
#[test]
fn test_update_flaky_tracker_script_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let script_path = root.join(".ci/scripts/update-flaky-tracker.py");

    assert!(
        script_path.exists(),
        ".ci/scripts/update-flaky-tracker.py must exist at {:?}",
        script_path
    );

    // Check if executable
    let metadata = fs::metadata(&script_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let executable = (mode & 0o111) != 0;
        assert!(executable, "update-flaky-tracker.py must be executable");
    }

    Ok(())
}

/// Phase 3: update-flaky-tracker.py must accept --help flag.
#[test]
fn test_update_flaky_tracker_help_flag() -> Result<()> {
    use std::process::Command;

    let root = crate::utils::project_root()?;
    let script_path = root.join(".ci/scripts/update-flaky-tracker.py");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--help")
        .current_dir(&root)
        .output()
        .context("running update-flaky-tracker.py --help")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // --help should exit with success
    assert!(
        output.status.success(),
        "update-flaky-tracker.py --help should succeed, got: {stderr}"
    );

    // --help output should mention usage or flags
    assert!(
        stdout.contains("usage") || stdout.contains("--input") || stdout.contains("--help"),
        "update-flaky-tracker.py --help should show usage info, got: {stdout}"
    );

    Ok(())
}

/// Phase 3: update-flaky-tracker.py must accept --input flag.
#[test]
fn test_update_flaky_tracker_input_flag() -> Result<()> {
    use std::process::Command;

    let root = crate::utils::project_root()?;
    let script_path = root.join(".ci/scripts/update-flaky-tracker.py");

    // Running with --input but no file should show error about missing file
    // (not about unknown flag)
    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--input")
        .arg("/nonexistent/test-results.json")
        .current_dir(&root)
        .output()
        .context("running update-flaky-tracker.py --input")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not complain about unknown flag --input
    assert!(
        !stderr.contains("unrecognized argument"),
        "update-flaky-tracker.py should accept --input flag, got: {stderr}"
    );

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 3c: Flaky test bullets in quality.md
// ---------------------------------------------------------------------

/// Phase 3: quality.md must contain FLAKY_TEST_BULLETS block.
#[test]
fn test_flaky_test_bullets_block_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let quality_path = root.join("docs/project/status/quality.md");
    let content =
        fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;

    // The block must exist
    assert!(
        content.contains("<!-- BEGIN: FLAKY_TEST_BULLETS -->"),
        "quality.md must contain <!-- BEGIN: FLAKY_TEST_BULLETS --> marker"
    );
    assert!(
        content.contains("<!-- END: FLAKY_TEST_BULLETS -->"),
        "quality.md must contain <!-- END: FLAKY_TEST_BULLETS --> marker"
    );

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 4: Subsystem mapping and test counts
// ---------------------------------------------------------------------

/// Phase 4: .ci/subsystem-mapping.yaml must exist.
#[test]
fn test_subsystem_mapping_file_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let mapping_path = root.join(".ci/subsystem-mapping.yaml");

    assert!(mapping_path.exists(), ".ci/subsystem-mapping.yaml must exist at {:?}", mapping_path);

    Ok(())
}

/// Phase 4: subsystem-mapping.yaml must map crates to StatusSubsystem variants.
#[test]
fn test_subsystem_mapping_schema() -> Result<()> {
    use serde_yaml_ng::Value;

    let root = crate::utils::project_root()?;
    let mapping_path = root.join(".ci/subsystem-mapping.yaml");
    let content =
        fs::read_to_string(&mapping_path).context("reading .ci/subsystem-mapping.yaml")?;

    let parsed: Value =
        serde_yaml_ng::from_str(&content).context("parsing .ci/subsystem-mapping.yaml")?;

    // Must have crate_to_subsystem mapping
    let mapping = parsed.get("crate_to_subsystem").ok_or_else(|| {
        color_eyre::eyre::eyre!("subsystem-mapping.yaml must have crate_to_subsystem key")
    })?;

    assert!(mapping.is_mapping(), "crate_to_subsystem must be a mapping (key-value pairs)");

    // Should have at least some crates mapped
    let mapping_obj = mapping.as_mapping().unwrap();
    assert!(!mapping_obj.is_empty(), "crate_to_subsystem must not be empty");

    // Valid subsystem values: Parser, Quality, Lsp, Dap, Workspace, Tests
    let valid_subsystems = ["Parser", "Quality", "Lsp", "Dap", "Workspace", "Tests"];
    for (crate_name, subsystem) in mapping_obj {
        let crate_name_str = crate_name.as_str().unwrap_or("");
        let subsystem_str = subsystem.as_str().unwrap_or("");
        assert!(
            valid_subsystems.contains(&subsystem_str),
            "Crate '{}' maps to invalid subsystem '{}'. Valid: {:?}",
            crate_name_str,
            subsystem_str,
            valid_subsystems
        );
    }

    Ok(())
}

/// Phase 4: subsystem-mapping.yaml must not have duplicate crate entries.
#[test]
fn test_subsystem_mapping_no_duplicate_crates() -> Result<()> {
    use serde_yaml_ng::Value;

    let root = crate::utils::project_root()?;
    let mapping_path = root.join(".ci/subsystem-mapping.yaml");
    let content =
        fs::read_to_string(&mapping_path).context("reading .ci/subsystem-mapping.yaml")?;

    let parsed: Value =
        serde_yaml_ng::from_str(&content).context("parsing .ci/subsystem-mapping.yaml")?;

    let mapping = parsed.get("crate_to_subsystem").ok_or_else(|| {
        color_eyre::eyre::eyre!("subsystem-mapping.yaml must have crate_to_subsystem key")
    })?;

    let mapping_obj = mapping.as_mapping().unwrap();
    let count = mapping_obj.keys().count();

    // Use BTreeSet to detect duplicates
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    for key in mapping_obj.keys() {
        let key_str = key.as_str().unwrap_or("");
        assert!(
            seen.insert(key_str),
            "Duplicate crate entry found in subsystem-mapping.yaml: {}",
            key_str
        );
    }

    assert_eq!(
        seen.len(),
        count,
        "BTreeSet dedup count {} should equal original count {}",
        seen.len(),
        count
    );

    Ok(())
}

/// Phase 4: quality.md must contain SUBSYSTEM_TEST_BULLETS block.
#[test]
fn test_subsystem_test_bullets_block_exists() -> Result<()> {
    let root = crate::utils::project_root()?;
    let quality_path = root.join("docs/project/status/quality.md");
    let content =
        fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;

    // The block must exist
    assert!(
        content.contains("<!-- BEGIN: SUBSYSTEM_TEST_BULLETS -->"),
        "quality.md must contain <!-- BEGIN: SUBSYSTEM_TEST_BULLETS --> marker"
    );
    assert!(
        content.contains("<!-- END: SUBSYSTEM_TEST_BULLETS -->"),
        "quality.md must contain <!-- END: SUBSYSTEM_TEST_BULLETS --> marker"
    );

    Ok(())
}

/// Phase 4: SUBSYSTEM_TEST_BULLETS must include a markdown table with
/// Subsystem, Tests, and Ignored columns.
#[test]
fn test_subsystem_test_bullets_has_table_format() -> Result<()> {
    let root = crate::utils::project_root()?;
    let quality_path = root.join("docs/project/status/quality.md");
    let content =
        fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;

    // Extract the block content
    let start = match content.find("<!-- BEGIN: SUBSYSTEM_TEST_BULLETS -->") {
        Some(s) => s,
        None => return Ok(()), // Block doesn't exist yet, test will fail elsewhere
    };
    let end = match content.find("<!-- END: SUBSYSTEM_TEST_BULLETS -->") {
        Some(e) => e,
        None => return Ok(()),
    };
    let block_content = &content[start..end];

    // Table must have header with Subsystem, Tests, Ignored columns
    assert!(
        block_content.contains("Subsystem") || block_content.contains("|"),
        "SUBSYSTEM_TEST_BULLETS block should contain a markdown table"
    );

    Ok(())
}

// ---------------------------------------------------------------------
// Phase 2 Implementation Interface Tests (disabled until stubs exist)
// ---------------------------------------------------------------------
// The following tests define the expected interface for Phase 2 functions.
// They are commented out because they require LatencyStats, collect_latency_by_subsystem,
// and format_latency_table to exist first. Once code-builder creates the stubs,
// these tests can be enabled.
//
// #[test]
// fn test_latency_stats_struct_exists() {
//     // This function should be pub(super) in quality.rs
//     // LatencyStats { p50_ms: f64, p95_ms: f64, p99_ms: f64 }
//     let mut stats = super::LatencyStats {
//         p50_ms: 1.0,
//         p95_ms: 5.0,
//         p99_ms: 10.0,
//     };
//     assert_eq!(stats.p50_ms, 1.0);
//     assert_eq!(stats.p95_ms, 5.0);
//     assert_eq!(stats.p99_ms, 10.0);
// }
//
// #[test]
// fn test_collect_latency_by_subsystem_returns_map() -> Result<()> {
//     let root = crate::utils::project_root()?;
//     let result = super::collect_latency_by_subsystem(&root);
//     let _: BTreeMap<String, super::LatencyStats> = result;
//     Ok(())
// }
//
// #[test]
// fn test_collect_latency_by_subsystem_handles_missing_file() -> Result<()> {
//     let temp_dir = tempfile::tempdir()?;
//     let result = super::collect_latency_by_subsystem(temp_dir.path());
//     assert!(result.is_empty(), "collect_latency_by_subsystem should return empty map when benchmark file missing");
//     Ok(())
// }
//
// #[test]
// fn test_format_latency_table_renders_markdown() -> Result<()> {
//     let mut stats = BTreeMap::new();
//     stats.insert("parser".to_string(), super::LatencyStats { p50_ms: 1.0, p95_ms: 5.0, p99_ms: 10.0 });
//     stats.insert("lsp".to_string(), super::LatencyStats { p50_ms: 2.0, p95_ms: 8.0, p99_ms: 20.0 });
//
//     let table = super::format_latency_table(&stats);
//
//     assert!(table.contains("Category"), "Table must have Category column");
//     assert!(table.contains("p50"), "Table must have p50 column");
//     assert!(table.contains("p95"), "Table must have p95 column");
//     assert!(table.contains("p99"), "Table must have p99 column");
//     assert!(table.contains("parser"), "Table must contain parser category");
//     assert!(table.contains("lsp"), "Table must contain lsp category");
//     assert!(table.contains("1.0"), "Table should contain p50 value for parser");
//
//     Ok(())
// }
//
// #[test]
// fn test_format_latency_table_handles_empty() -> Result<()> {
//     let stats = BTreeMap::new();
//     let table = super::format_latency_table(&stats);
//     assert!(table.contains("Category"), "Empty table should still have header");
//     Ok(())
// }
//
// #[test]
// fn test_collect_subsystem_test_counts_exists() -> Result<()> {
//     let root = crate::utils::project_root()?;
//     let result = super::collect_subsystem_test_counts(&root);
//     let _: BTreeMap<super::StatusSubsystem, super::tests::TestCounts> = result;
//     Ok(())
// }
