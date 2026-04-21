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

    // Verify --simplified flag is present in the justfile recipe (critical for baseline stability).
    // The CI job delegates to `just public-api-check`, so the flag lives in the justfile, not the
    // workflow YAML itself. Check there instead to avoid asserting on a comment.
    let justfile = fs::read_to_string(root.join("justfile"))?;
    assert!(
        justfile.contains("--simplified"),
        "justfile public-api recipes must use '--simplified' flag for cargo public-api"
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

/// Test G (regression guard): public-api-check script body has correct hard-fail behaviour
///
/// Specifically verifies:
/// 1. `set -euo pipefail` is present so the script aborts on errors.
/// 2. The grep pipeline uses `|| true` so that an empty match (e.g., from a compile error
///    silenced by `2>/dev/null`) does NOT abort the script early via set -e, allowing the
///    FAILED counter and the final exit-1 to report the real problem instead.
/// 3. The `diff -u` comparison runs and FAILED is set on non-zero diff exit.
#[test]
fn public_api_check_script_has_correct_fail_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    // Extract the public-api-check recipe body (up to the next recipe header)
    let check_body = justfile
        .split("public-api-check:")
        .nth(1)
        .ok_or("Could not find public-api-check recipe in justfile")?
        .split("\npublic-api-update:")
        .next()
        .ok_or("Could not delimit public-api-check recipe body")?;

    assert!(
        check_body.contains("set -euo pipefail"),
        "public-api-check must use 'set -euo pipefail'"
    );

    // The grep invocation must have '|| true' to avoid aborting the loop when
    // cargo-public-api produces empty output (e.g., due to a compile error silenced
    // by `2>/dev/null`).  Without it, grep exits 1 on zero matches and set -e kills
    // the script before the FAILED counter is evaluated.
    assert!(
        check_body.contains("grep \"^pub \"") || check_body.contains("grep '^pub '"),
        "public-api-check must grep for '^pub ' items"
    );
    assert!(
        check_body.contains("grep \"^pub \" > \"/tmp/${crate}-current.txt\" || true")
            || check_body.contains("grep \"^pub \" > \"/tmp/${crate}-current.txt\"  || true")
            || (check_body.contains("grep \"^pub \"") && check_body.contains("|| true")),
        "public-api-check grep pipeline must end with '|| true' to prevent set -e abort on empty output"
    );

    assert!(
        check_body.contains("diff -u"),
        "public-api-check must use 'diff -u' to compare baseline vs current"
    );

    assert!(check_body.contains("FAILED=1"), "public-api-check must set FAILED=1 on diff mismatch");

    assert!(check_body.contains("exit 1"), "public-api-check must exit 1 when FAILED > 0");

    Ok(())
}

/// Test H (edge case): Facade crate list is in sync across justfile, CI, and baseline directory
///
/// This test verifies that the set of 5 facade crates is consistent in:
/// - justfile `public-api-check` and `public-api-update` recipes
/// - CI workflow `.github/workflows/ci-nightly.yml` public-api-check job
/// - baseline directory `.ci/public-api-baselines/`
///
/// If the crate list drifts (e.g., someone adds a 6th facade but forgets to update justfile),
/// the check might miss that crate and CI would silently allow API breakage.
#[test]
fn facade_crate_list_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let expected_crates = vec!["perl-lsp-rs", "perl-parser", "perl-uri", "perl-dap", "perllsp"];

    // Read justfile and verify all 5 crates appear in public-api recipes
    let justfile = fs::read_to_string(root.join("justfile"))?;
    let public_api_section = justfile
        .split("public-api-check:")
        .nth(1)
        .ok_or("Could not find public-api-check recipe in justfile")?
        .split("public-api-update:") // End at next recipe
        .next()
        .ok_or("Could not parse public-api-check recipe")?;

    for crate_name in &expected_crates {
        assert!(
            public_api_section.contains(crate_name),
            "Justfile public-api-check recipe must reference facade crate: {}",
            crate_name
        );
    }

    // Read CI workflow and verify all 5 crates appear in the job description
    let workflow = fs::read_to_string(root.join(".github/workflows/ci-nightly.yml"))?;
    let ci_check_section = workflow
        .split("public-api-check:")
        .nth(1)
        .ok_or("Could not find public-api-check job in CI workflow")?
        .split('\n')
        .take_while(|line| !line.starts_with("  ") || line.starts_with("    "))
        .collect::<Vec<_>>()
        .join("\n");

    for crate_name in &expected_crates {
        assert!(
            ci_check_section.contains(crate_name),
            "CI workflow public-api-check job must reference facade crate: {}",
            crate_name
        );
    }

    // Verify baseline directory has exactly 5 baseline files
    let baselines_dir = root.join(".ci/public-api-baselines");
    let baseline_files: Vec<_> = fs::read_dir(&baselines_dir)?
        .filter_map(|entry| {
            entry
                .ok()
                .and_then(|e| e.file_name().into_string().ok())
                .filter(|name| name.ends_with(".txt"))
        })
        .collect();

    assert_eq!(
        baseline_files.len(),
        5,
        "Expected 5 baseline files (.txt) in .ci/public-api-baselines/, found {}",
        baseline_files.len()
    );

    for crate_name in &expected_crates {
        let expected_file = format!("{}.txt", crate_name);
        assert!(
            baseline_files.contains(&expected_file),
            "Baseline file missing for facade crate: {}/{}",
            baselines_dir.display(),
            expected_file
        );
    }

    Ok(())
}

/// Test I (edge case): Non-facade crates do not have baselines
///
/// This test verifies that the public API ratchet only applies to the 5 primary
/// facades and does not create baseline files for internal support crates like
/// `perl-tdd-support`, `perl-corpus`, or other internal satellites.
///
/// If baselines leak to internal crates, it creates unnecessary maintenance burden
/// and suggests the scope has drifted from "facade-only".
#[test]
fn non_facade_crates_have_no_baselines() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let baselines_dir = root.join(".ci/public-api-baselines");

    // List of internal crates that should NOT have baselines
    let internal_crates = vec![
        "perl-tdd-support",
        "perl-corpus",
        "perl-lexer",
        "perl-lexer-core",
        "perl-parser-core",
    ];

    for crate_name in &internal_crates {
        let baseline_path = baselines_dir.join(format!("{}.txt", crate_name));
        assert!(
            !baseline_path.exists(),
            "Internal crate {} should NOT have a baseline file (only 5 facades should)",
            crate_name
        );
    }

    Ok(())
}

/// Test J (edge case & regression): perllsp baseline has exactly 2 lines (mod + re-export)
///
/// `perllsp` is a thin binary wrapper that re-exports from `perl-lsp-rs`.
/// Its public API surface should be minimal: just the module declaration and
/// the re-export statement. This test verifies the format is preserved as expected.
///
/// If perllsp's baseline grows significantly, it may indicate:
/// - Additional public API was accidentally added to the lib target
/// - The re-export pattern changed
/// - The baseline was regenerated incorrectly
#[test]
fn perllsp_baseline_has_expected_reexport_format() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let perllsp_baseline = root.join(".ci/public-api-baselines/perllsp.txt");

    let content = fs::read_to_string(&perllsp_baseline)
        .map_err(|e| format!("Failed to read perllsp baseline: {}", e))?;

    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(
        lines.len(),
        2,
        "perllsp baseline should have exactly 2 lines (mod + re-export), got {}. Content:\n{}",
        lines.len(),
        content
    );

    // Verify first line is the module declaration
    assert!(
        lines[0].starts_with("pub mod perllsp"),
        "perllsp baseline first line should be module declaration, got: {}",
        lines[0]
    );

    // Verify second line is a re-export (uses pub use and contains <<...>>)
    assert!(
        lines[1].contains("pub use") && lines[1].contains("<<"),
        "perllsp baseline second line should be a re-export pattern, got: {}",
        lines[1]
    );

    Ok(())
}

/// Test K (regression guard): Tool version is pinned consistently
///
/// This test verifies that `cargo-public-api` version is specified identically in:
/// - justfile `_public-api-install` recipe
/// - CI workflow `.github/workflows/ci-nightly.yml` install step
///
/// Version drift between local and CI could cause baselines to diverge if the tool
/// changes its output format between versions. This test catches silent mismatches.
#[test]
fn tool_version_pinned_consistently() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();

    // Extract version from justfile (looks like: --version 0.50.1;)
    let justfile = fs::read_to_string(root.join("justfile"))?;
    let justfile_version_line = justfile
        .lines()
        .find(|line| line.contains("cargo-public-api") && line.contains("--version"))
        .ok_or("Could not find cargo-public-api version in justfile")?;

    // Parse version: extract digits.digits.digits pattern
    let justfile_version_num = justfile_version_line
        .split("--version")
        .nth(1)
        .and_then(|s| {
            // Extract version like "0.50.1" from strings like " 0.50.1; \" or " 0.50.1"
            s.trim()
                .split(|c: char| !c.is_numeric() && c != '.')
                .find(|s| !s.is_empty() && s.chars().next().is_some_and(|c| c.is_numeric()))
        })
        .ok_or("Could not parse version from justfile")?;

    // Extract version from CI workflow (looks like: --version 0.50.1)
    let workflow = fs::read_to_string(root.join(".github/workflows/ci-nightly.yml"))?;
    let ci_version_line = workflow
        .lines()
        .find(|line| line.contains("cargo-public-api") && line.contains("--version"))
        .ok_or("Could not find cargo-public-api version in CI workflow")?;

    let ci_version_num = ci_version_line
        .split("--version")
        .nth(1)
        .and_then(|s| {
            s.trim()
                .split(|c: char| !c.is_numeric() && c != '.')
                .find(|s| !s.is_empty() && s.chars().next().is_some_and(|c| c.is_numeric()))
        })
        .ok_or("Could not parse version from CI workflow")?;

    assert_eq!(
        justfile_version_num, ci_version_num,
        "cargo-public-api version mismatch: justfile={}, CI={}. Both must be identical.",
        justfile_version_num, ci_version_num
    );

    Ok(())
}
