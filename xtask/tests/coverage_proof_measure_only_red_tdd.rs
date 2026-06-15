//! RED TDD tests for #1470 — coverage-proof measurement decoupling.
//! Tests assert DESIRED behavior (which FAILS now).
//! Builder will make them pass by:
//! 1. Wrapping ALL commands non-fatally in generate-coverage-pack-commands.py
//! 2. Adding pack-cap enforcement in ci_route.rs
//! 3. Exposing exact failure classes in quality-gate receipt artifacts
//!
//! RIPR#1428: Suppress RIPR gap for new test file — tests are red and guard
//! against regressions in coverage measurement logic (not covered yet by
//! scoped proof packs).

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::tempdir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn test_coverage_pack_non_fatal_lib_test_failure() -> TestResult {
    // DESIRED: quality-gate receipt has test_failure_class field documenting
    // when test commands exited non-zero but coverage is still measured.
    // Gate verdict is based on coverage number, not test exit code.
    //
    // FAILS NOW: quality-gate receipt has no test_failure_class field.
    // Builder must add this field when wrapping commands non-fatally.

    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    // Simulate good patch coverage (97%)
    write_coverage_receipt(&coverage, &current_head(&root)?, Some(97.0), json!([]))?;

    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    // DESIRED: receipt has test_failure_class field (can be None or a failure class name).
    // FAILS NOW: field doesn't exist yet.
    assert!(
        payload.get("test_failure_class").is_some(),
        "receipt must have test_failure_class field to document test failures separately from coverage verdict"
    );

    Ok(())
}

#[test]
fn test_coverage_pack_non_fatal_integration_test_failure() -> TestResult {
    // DESIRED: generate-coverage-pack-commands.py wraps ALL test commands non-fatally,
    // not just --tests commands (integration-only per #1282/#1269).
    // This is verified by checking receipt has test_failure_class field.
    //
    // FAILS NOW: field doesn't exist; script only partially wraps commands.
    // Builder must extend wrap logic to all commands and add receipt field.

    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    // Simulate excellent patch coverage (98%)
    write_coverage_receipt(&coverage, &current_head(&root)?, Some(98.0), json!([]))?;

    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    // DESIRED: receipt has test_failure_class field, indicating that
    // non-fatal wrapping is in place for all test commands.
    // FAILS NOW: field doesn't exist yet.
    assert!(
        payload.get("test_failure_class").is_some(),
        "receipt must have test_failure_class field (proof that non-fatal wrapping applies to all commands, not just --tests)"
    );

    Ok(())
}

#[test]
fn test_routing_skip_vs_routing_bug_distinction() -> TestResult {
    // DESIRED: receipt distinguishes routing_skip (no coverable code changed → valid)
    // from routing_bug (production code changed but zero packs → fail loud).
    //
    // FAILS NOW: no routing_classification field in receipt.
    // Builder must add this field to CiRouteReceipt to support the distinction.

    let root = repo_root()?;
    let dir = tempdir()?;
    let receipt = dir.path().join("ci-route.json");
    let summary = dir.path().join("ci-route.md");

    // Simulate a PR that touches only docs (no coverable code changed).
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(&root)
        .args([
            "ci",
            "route",
            "--base",
            "origin/main",
            "--head",
            "HEAD",
            "--receipt",
            receipt.to_str().ok_or("invalid path")?,
            "--summary",
            summary.to_str().ok_or("invalid path")?,
            "--changed-file",
            "README.md",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    // DESIRED: receipt has routing_classification field naming the routing decision.
    // FAILS NOW: field doesn't exist yet.
    let routing_class = route
        .get("routing_classification")
        .and_then(Value::as_str)
        .ok_or("receipt must have routing_classification field")?;

    // For docs-only change, should be routing_skip.
    assert_eq!(
        routing_class, "routing_skip",
        "docs-only change should classify as routing_skip (valid policy skip)"
    );

    Ok(())
}

#[test]
fn test_exact_failure_class_taxonomy_in_artifact() -> TestResult {
    // DESIRED: quality-gate receipt exposes exact failure class:
    // coverage_shortfall | test_failure | setup_failure | routing_skip | routing_bug
    //
    // FAILS NOW: no failure_class field; failures are undifferentiated.
    // Builder must add receipt.failure_class field documenting the exact gate logic path.

    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    // Case 1: coverage_shortfall (genuine coverage failure)
    write_coverage_receipt(
        &coverage,
        &current_head(&root)?,
        Some(94.9), // Below 95% threshold
        json!([{
            "path": "crates/test-crate/src/lib.rs",
            "line_hit": 40,
            "line_found": 100,
            "line_coverage": 40.0,
            "sample_uncovered_lines": [12, 13, 17]
        }]),
    )?;

    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(!output.status.success(), "coverage below 95% must fail the gate");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    // Builder will add explicit failure_class field. Red test asserts it exists.
    let failure_class = payload.get("failure_class").and_then(Value::as_str).or_else(|| {
        // If builder hasn't added it yet, red test will fail here.
        None
    });

    if let Some(class) = failure_class {
        assert_eq!(
            class, "coverage_shortfall",
            "coverage below threshold must be classified as coverage_shortfall"
        );
    } else {
        // Red TDD: this test documents that the builder must add failure_class field.
        panic!("receipt must have failure_class field; builder will add it");
    }

    Ok(())
}

#[test]
fn test_coverage_proof_scoping_no_full_suite_expansion() -> TestResult {
    // DESIRED: receipt has coverage_pack_cap, coverage_packs_skipped,
    // and coverage_pack_skip_reason fields documenting pack cap enforcement.
    // Multi-file PRs should NOT trigger full-suite expansion.
    //
    // FAILS NOW: no such fields in route receipt.
    // Builder must add these fields to CiRouteReceipt and enforce cap logic.

    let root = repo_root()?;
    let dir = tempdir()?;
    let receipt = dir.path().join("ci-route.json");
    let summary = dir.path().join("ci-route.md");

    // Simulate a PR touching multiple files.
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(&root)
        .args([
            "ci",
            "route",
            "--base",
            "origin/main",
            "--head",
            "HEAD",
            "--receipt",
            receipt.to_str().ok_or("invalid path")?,
            "--summary",
            summary.to_str().ok_or("invalid path")?,
            "--changed-file",
            "xtask/src/tasks/ci_route.rs",
        ])
        .args(["--changed-file", "crates/perl-ast/src/lib.rs"])
        .args(["--changed-file", "crates/perl-lexer/src/lib.rs"])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    // DESIRED: receipt has coverage_pack_cap field (should be 10).
    // FAILS NOW: field doesn't exist.
    let cap = route
        .get("coverage_pack_cap")
        .and_then(Value::as_u64)
        .ok_or("receipt must have coverage_pack_cap field")?;
    assert_eq!(cap, 10, "pack cap should be set to 10");

    // DESIRED: receipt has coverage_packs_skipped and coverage_pack_skip_reason.
    // FAILS NOW: fields don't exist.
    let packs_skipped = route
        .get("coverage_packs_skipped")
        .and_then(Value::as_u64)
        .ok_or("receipt must have coverage_packs_skipped field")?;

    if packs_skipped > 0 {
        let skip_reason = route
            .get("coverage_pack_skip_reason")
            .and_then(Value::as_str)
            .ok_or("receipt must document why packs were skipped")?;
        assert!(
            !skip_reason.trim().is_empty(),
            "skip_reason must be non-empty when packs are skipped"
        );
    }

    Ok(())
}

#[test]
fn test_genuine_coverage_shortfall_still_fails_gate() -> TestResult {
    // REGRESSION GUARD: a genuine coverage shortfall (patch < 95% with good coverage
    // measurement) must STILL fail the gate. This test ensures the builder's refactoring
    // doesn't accidentally make all coverage failures silent.
    //
    // This test PASSES (does not fail) — it guards against regression, not a new feature.

    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    // Simulate genuine coverage shortfall: patch = 94.5% (below 95%)
    write_coverage_receipt(
        &coverage,
        &current_head(&root)?,
        Some(94.5),
        json!([{
            "path": "crates/perl-parser/src/lib.rs",
            "line_hit": 18,
            "line_found": 30,
            "line_coverage": 60.0,
            "sample_uncovered_lines": [50, 51, 52]
        }]),
    )?;

    // Gate should FAIL because coverage is below threshold.
    // This behavior is CORRECT and must not change after the builder's fixes.
    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(!output.status.success(), "gate must fail when patch coverage is below 95%");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(
        payload.get("decision").and_then(Value::as_str),
        Some("fail"),
        "decision must be 'fail' when coverage is below target"
    );

    Ok(())
}

// ============================================================================
// Helper functions (mirror of quality_gate_patch_coverage_cli_policy.rs tests)
// ============================================================================

fn patch_quality_gate_command(
    root: &Path,
    coverage: &Path,
    receipt: &Path,
    summary: &Path,
    patch: Option<f64>,
) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce-patch-coverage"]);
    command.arg("--coverage-receipt").arg(coverage);
    command.args(["--codecov", "codecov.yml"]);
    command.arg("--receipt").arg(receipt);
    command.arg("--summary").arg(summary);
    if let Some(patch) = patch {
        command.arg("--patch-coverage").arg(format!("{patch:.2}"));
    }
    Ok(command)
}

fn write_coverage_receipt(
    path: &Path,
    head: &str,
    patch: Option<f64>,
    files_below_target: Value,
) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut coverage = serde_json::Map::new();
    if let Some(patch) = patch {
        coverage.insert("patch".to_string(), json!(patch));
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "lcov": "target/lcov.info",
            "coverage": coverage,
            "codecov_status": {
                "patch": {
                    "default": {
                        "target": "95%",
                        "threshold": "0%",
                        "if_ci_failed": "error"
                    }
                }
            },
            "measured": {
                "line_hit": 96,
                "line_found": 100,
                "line_coverage": 96.0
            },
            "patch_files_below_target": json!([]),
            "files_below_target": files_below_target
        }))?,
    )?;
    Ok(())
}

fn repo_root() -> TestResult<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest must be nested under repo root".into())
}

fn current_head(root: &Path) -> TestResult<String> {
    let output = StdCommand::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git rev-parse HEAD failed with status {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
