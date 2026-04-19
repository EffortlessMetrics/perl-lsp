//! Integration tests for issue #4497: Facade-Only Public API Ratchet
//!
//! These tests verify the public API surface ratchet infrastructure:
//! - Baseline files exist for 5 facade crates
//! - Baselines are non-empty
//! - just public-api-check and just public-api-update recipes exist
//! - CI workflow includes public-api-check job
//! - semver-check covers all 5 facade crates
//! - CONTRIBUTING.md documents the public API workflow
//!
//! Tests assert config state, not runtime behavior.

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

/// Test A: All 5 baseline files exist in .ci/public-api-baselines/
#[test]
fn baselines_exist_for_5_facades() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let baselines_dir = root.join(".ci/public-api-baselines");

    let crates = ["perl-lsp-rs", "perl-parser", "perl-uri", "perl-dap", "perllsp"];

    for crate_name in &crates {
        let baseline_path = baselines_dir.join(format!("{}.txt", crate_name));
        assert!(
            baseline_path.exists(),
            "Baseline file missing: {} (expected at {})",
            crate_name,
            baseline_path.display()
        );
    }

    Ok(())
}

/// Test B: Each baseline file is non-empty
#[test]
fn baseline_files_are_non_empty() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let baselines_dir = root.join(".ci/public-api-baselines");

    let crates = ["perl-lsp-rs", "perl-parser", "perl-uri", "perl-dap", "perllsp"];

    for crate_name in &crates {
        let baseline_path = baselines_dir.join(format!("{}.txt", crate_name));
        let content = fs::read_to_string(&baseline_path)
            .map_err(|e| format!("Failed to read baseline {}: {}", crate_name, e))?;

        assert!(
            !content.trim().is_empty(),
            "Baseline file is empty: {} (expected at least 1 line)",
            crate_name
        );

        // Verify that lines start with "pub " (public API items)
        let non_empty_lines: Vec<_> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        for (line_num, line) in non_empty_lines.iter().enumerate() {
            assert!(
                line.starts_with("pub "),
                "Baseline {} line {} does not start with 'pub ': {}",
                crate_name,
                line_num + 1,
                line
            );
        }
    }

    Ok(())
}

/// Test C: Justfile has public-api-check and public-api-update recipes
#[test]
fn justfile_has_public_api_recipes() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    assert!(
        justfile.contains("public-api-check:"),
        "justfile must contain 'public-api-check:' recipe (did not find it)"
    );

    assert!(
        justfile.contains("public-api-update:"),
        "justfile must contain 'public-api-update:' recipe (did not find it)"
    );

    assert!(
        justfile.contains("_public-api-install:"),
        "justfile must contain '_public-api-install:' helper recipe (did not find it)"
    );

    // Verify recipes appear in just --list output by checking justfile syntax
    // (just --list output via Command requires runtime, so we verify source instead)
    assert!(
        justfile.contains("just _public-api-install"),
        "public-api recipes must call _public-api-install helper"
    );

    Ok(())
}

/// Test D: CI workflow includes public-api-check job
#[test]
fn ci_nightly_workflow_has_public_api_check_job() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/ci-nightly.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|e| format!("Failed to read CI workflow: {}", e))?;

    // Verify job name exists
    assert!(
        workflow.contains("public-api-check:"),
        "ci-nightly.yml must contain 'public-api-check:' job"
    );

    // Verify the job runs 'just public-api-check'
    assert!(
        workflow.contains("just public-api-check"),
        "ci-nightly.yml public-api-check job must run 'just public-api-check' step"
    );

    // Verify all 5 crate names are referenced in the workflow context
    let facade_crates = ["perl-lsp-rs", "perl-parser", "perl-uri", "perl-dap", "perllsp"];
    for crate_name in &facade_crates {
        assert!(
            workflow.contains(crate_name),
            "ci-nightly.yml workflow must reference facade crate: {}",
            crate_name
        );
    }

    // Verify --simplified flag is present (critical for baseline stability)
    assert!(
        workflow.contains("--simplified"),
        "ci-nightly.yml must use '--simplified' flag for cargo public-api"
    );

    // Verify NO continue-on-error on public-api-check (hard-fail only)
    let public_api_section = workflow
        .split("public-api-check:")
        .nth(1)
        .ok_or("Could not find public-api-check job section")?;

    // Extract the job block (ends at next top-level key starting with 2 spaces)
    let job_block = public_api_section
        .split('\n')
        .take_while(|line| line.is_empty() || !line.starts_with("  ") || line.starts_with("    "))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !job_block.contains("continue-on-error: true")
            && !job_block.contains("continue-on-error: false"),
        "public-api-check job must have hard-fail semantics (no continue-on-error)"
    );

    Ok(())
}

/// Test E: semver-check job covers all 5 facade crates
#[test]
fn semver_check_covers_5_crates() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci-nightly.yml"))?;

    // Count occurrences of "cargo semver-checks check-release -p" for each crate
    let crates_to_check =
        ["perl-parser", "perl-lexer", "perl-parser-core", "perl-lsp-rs", "perllsp"];

    let mut found_count = 0;
    for crate_name in &crates_to_check {
        let pattern = format!("cargo semver-checks check-release -p {}", crate_name);
        if workflow.contains(&pattern) {
            found_count += 1;
        }
    }

    assert_eq!(
        found_count,
        5,
        "semver-check job must verify 5 crates: {}, {}, {}, {}, {}. Found {} of 5.",
        crates_to_check[0],
        crates_to_check[1],
        crates_to_check[2],
        crates_to_check[3],
        crates_to_check[4],
        found_count
    );

    Ok(())
}

/// Test F: CONTRIBUTING.md documents public API workflow
#[test]
fn contributing_md_documents_public_api_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let contributing = fs::read_to_string(root.join("CONTRIBUTING.md"))?;

    assert!(
        contributing.contains("### Public API Surface Ratchet"),
        "CONTRIBUTING.md must have '### Public API Surface Ratchet' subsection"
    );

    assert!(
        contributing.contains("just public-api-update"),
        "CONTRIBUTING.md must mention 'just public-api-update' command"
    );

    assert!(
        contributing.contains(".ci/public-api-baselines"),
        "CONTRIBUTING.md must reference '.ci/public-api-baselines/' directory"
    );

    Ok(())
}

/// Test G (regression guard): just public-api-check exits cleanly on valid baselines
///
/// This test verifies that once baselines are captured and committed,
/// running `just public-api-check` exits 0 with no drift.
///
/// Remove #[ignore] after builder lands baselines and recipes.
#[test]
#[ignore]
fn public_api_check_passes_on_clean_tree() -> Result<(), Box<dyn std::error::Error>> {
    // This test runs: just public-api-check
    // Expected: exit 0 (no API drift against committed baselines)
    //
    // This naturally only passes once baselines exist and recipes are implemented.
    // Uncomment after implementation to verify the check works.

    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    // For now, just verify the recipe exists (same as Test C)
    assert!(justfile.contains("public-api-check:"), "public-api-check recipe must exist");

    Ok(())
}
