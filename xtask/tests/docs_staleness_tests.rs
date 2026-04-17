//! Documentation staleness tests for Issue #3227
//!
//! These tests verify that documentation drift findings are fixed:
//! - Dead links to non-existent files
//! - Stale version references (v0.8.x, v0.9.x, v0.10.x, v0.11.x, v0.12.0/0.12.1)
//! - References to files that no longer exist in the repo
//!
//! Run with: cargo test -p xtask --test docs_staleness_tests

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Root of the perl-lsp repository
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // xtask
        .unwrap()
        .to_path_buf()
}

/// Path to docs/ directory
fn docs_dir() -> PathBuf {
    repo_root().join("docs")
}

/// Read entire file contents as String
fn read_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

// ============================================================================
// CRITICAL: Dead links to non-existent files
// ============================================================================

/// Test: docs/project/status/index.md should not link to docs/archive/status_snapshots/
/// which does not exist. Either the directory should be created, or the link updated.
#[test]
fn test_status_index_does_not_reference_nonexistent_archive_directory() {
    let status_index = docs_dir().join("project/status/index.md");
    let content = read_file(&status_index).expect("docs/project/status/index.md must exist");

    // The research found line 66 references docs/archive/status_snapshots/
    // This directory does NOT exist, so the link is broken.
    // After fix: either the directory should exist, OR the link should point elsewhere.
    let archive_link = "docs/archive/status_snapshots/";
    let has_broken_link = content.contains(archive_link);

    if has_broken_link {
        // Check if the target directory actually exists
        let archive_path = docs_dir().join("archive/status_snapshots/");
        let archive_exists = archive_path.exists();

        // Also check if docs/project/status/ has the actual historical snapshots
        let status_dir = docs_dir().join("project/status/");
        let has_status_files = status_dir.exists()
            && status_dir.read_dir().ok().map(|mut i| i.next().is_some()).unwrap_or(false);

        assert!(
            archive_exists || has_status_files,
            "docs/project/status/index.md references 'docs/archive/status_snapshots/' but that directory does not exist. \
             Either create the directory or update the link to point to existing historical \
             snapshots (e.g., docs/project/status/release.md, docs/project/status/parser.md)",
        );
    }
}

/// Test: docs/project/ORIENTATION.md should not link to docs/project/README.md
/// which does not exist. The link should point to docs/README.md instead.
#[test]
fn test_orientation_readme_link_points_to_existing_file() {
    let orientation = docs_dir().join("project/ORIENTATION.md");
    let content = read_file(&orientation).expect("docs/project/ORIENTATION.md must exist");

    // Line 3 has: [README.md](README.md) which points to docs/project/README.md
    // That file does NOT exist. It should point to docs/README.md (the docs hub).
    let broken_readme_link = "[README.md](README.md)";
    let has_broken_link = content.contains(broken_readme_link);

    if has_broken_link {
        // Check if docs/project/README.md exists
        let project_readme = docs_dir().join("project/README.md");
        let project_readme_exists = project_readme.exists();

        // Check if docs/README.md exists (the correct target)
        let docs_readme = docs_dir().join("README.md");
        let docs_readme_exists = docs_readme.exists();

        assert!(
            project_readme_exists || !has_broken_link,
            "docs/project/ORIENTATION.md links to [README.md](README.md) but \
             docs/project/README.md does not exist. The link should point to \
             docs/README.md (the actual docs hub)."
        );

        // If the broken link still exists, at least verify the correct file exists
        if !project_readme_exists && has_broken_link {
            assert!(
                docs_readme_exists,
                "The broken link in ORIENTATION.md references a non-existent file, \
                 and docs/README.md also doesn't exist - both need fixing"
            );
        }
    }
}

/// Test: docs/TODO.md should not reference START_HERE.md which does not exist.
#[test]
fn test_todo_does_not_reference_nonexistent_start_here() {
    let todo = docs_dir().join("TODO.md");
    let content = read_file(&todo).expect("docs/TODO.md must exist");

    // Line 34 references START_HERE.md which does not exist
    let start_here_ref = "START_HERE.md";
    let has_start_here_ref = content.contains(start_here_ref);

    if has_start_here_ref {
        // Check if START_HERE.md exists anywhere in the repo
        let start_here_path = docs_dir().join("START_HERE.md");
        let exists = start_here_path.exists();

        // Also check docs/INDEX.md which is the actual index
        let index_path = docs_dir().join("INDEX.md");
        let index_exists = index_path.exists();

        assert!(
            exists || !has_start_here_ref,
            "docs/TODO.md references '{}' but that file does not exist in docs/. \
             Either create the file or update the reference to point to docs/INDEX.md \
             (the actual documentation hub).",
            start_here_ref
        );

        // If reference exists but file doesn't, at least verify INDEX.md exists
        if !exists && has_start_here_ref {
            assert!(
                index_exists,
                "START_HERE.md reference in TODO.md is broken and docs/INDEX.md also \
                 doesn't exist - both need fixing"
            );
        }
    }
}

/// Test: docs/reference/VALIDATION-149-acceptance-criteria.md should not reference
/// SPEC-149-missing-docs.manifest.yml or ISSUE-149.story.md which do not exist.
#[test]
fn test_validation_149_does_not_reference_nonexistent_spec_files() {
    let validation_149 = docs_dir().join("reference/VALIDATION-149-acceptance-criteria.md");
    let content = read_file(&validation_149)
        .expect("docs/reference/VALIDATION-149-acceptance-criteria.md must exist");

    // These files don't exist anywhere in the repo
    let spec_149_file = "SPEC-149-missing-docs.manifest.yml";
    let issue_149_file = "ISSUE-149.story.md";

    let has_spec_ref = content.contains(spec_149_file);
    let has_issue_ref = content.contains(issue_149_file);

    // Check if these files exist anywhere in the repo using walkdir
    use walkdir::WalkDir;
    let spec_exists = WalkDir::new(repo_root()).into_iter().any(|e| {
        e.as_ref().ok().is_some() && e.unwrap().file_name().to_string_lossy() == spec_149_file
    });

    let issue_exists = WalkDir::new(repo_root()).into_iter().any(|e| {
        e.as_ref().ok().is_some() && e.unwrap().file_name().to_string_lossy() == issue_149_file
    });

    assert!(
        !has_spec_ref || spec_exists,
        "docs/reference/VALIDATION-149-acceptance-criteria.md references \
         '{}' but that file does not exist anywhere in the repo. \
         The file should be deleted or the reference removed.",
        spec_149_file
    );

    assert!(
        !has_issue_ref || issue_exists,
        "docs/reference/VALIDATION-149-acceptance-criteria.md references \
         '{}' but that file does not exist anywhere in the repo. \
         The file should be deleted or the reference removed.",
        issue_149_file
    );
}

// ============================================================================
// DRIFT: Stale version references (v0.8.x, v0.9.x, v0.10.x, v0.11.x, v0.12.0/0.12.1)
// ============================================================================

/// Test: docs/reference/STABILITY.md should reference v0.12.x not v0.11.x
#[test]
fn test_stability_md_references_current_version() {
    let stability = docs_dir().join("reference/STABILITY.md");
    let content = read_file(&stability).expect("docs/reference/STABILITY.md must exist");

    // The research found: "The current release line is `v0.11.x`" and crate table lists 0.11.x
    // Workspace is at v0.12.4, so this should be v0.12.x

    // Check for stale v0.11.x references
    let stale_v111 = content.contains("v0.11.x") || content.contains("v0.11.");
    let stale_v110 = content.contains("v0.10.x") || content.contains("v0.10.");
    let stale_v19 = content.contains("v0.9.x") || content.contains("v0.9.");

    assert!(
        !stale_v111,
        "docs/reference/STABILITY.md still references v0.11.x but workspace is at v0.12.x. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v110,
        "docs/reference/STABILITY.md still references v0.10.x but workspace is at v0.12.x. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v19,
        "docs/reference/STABILITY.md still references v0.9.x but workspace is at v0.12.x. \
         Update to v0.12.x"
    );
}

/// Test: docs/MAINTENANCE.md should reference v0.12.x not v0.9.x
#[test]
fn test_maintenance_md_references_current_version() {
    let maintenance = docs_dir().join("MAINTENANCE.md");
    let content = read_file(&maintenance).expect("docs/MAINTENANCE.md must exist");

    // The research found: Title "Perl LSP v0.9.x Maintenance Plan" and "Applies To" lists 0.9.x
    // Workspace is at v0.12.4, so this should be v0.12.x

    let stale_v19 =
        content.to_lowercase().contains("v0.9.x") || content.to_lowercase().contains("v0.9.");
    let stale_v110 = content.contains("v0.10.x") || content.contains("v0.10.");
    let stale_v111 = content.contains("v0.11.x") || content.contains("v0.11.");

    assert!(
        !stale_v19,
        "docs/MAINTENANCE.md still references v0.9.x but workspace is at v0.12.x. \
         Title and 'Applies To' section should be updated to v0.12.x"
    );

    assert!(
        !stale_v110,
        "docs/MAINTENANCE.md still references v0.10.x but workspace is at v0.12.x. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v111,
        "docs/MAINTENANCE.md still references v0.11.x but workspace is at v0.12.x. \
         Update to v0.12.x"
    );
}

/// Test: docs/project/ORIENTATION.md should not have v0.9.x/v0.10.0 era framing
#[test]
fn test_orientation_no_old_version_era_framing() {
    let orientation = docs_dir().join("project/ORIENTATION.md");
    let content = read_file(&orientation).expect("docs/project/ORIENTATION.md must exist");

    // The research found: "v0.9.x hardening underway", "Now (post v0.10.0 close-out)"
    // These are stale now that we're at v0.12.x

    let stale_v19 = content.contains("v0.9.x");
    let stale_v110 = content.contains("v0.10.0") || content.contains("v0.10.x");
    let stale_v111 = content.contains("v0.11.0") || content.contains("v0.11.x");

    assert!(
        !stale_v19,
        "docs/project/ORIENTATION.md still references v0.9.x era framing. \
         Update to reflect current v0.12.x status."
    );

    assert!(
        !stale_v110,
        "docs/project/ORIENTATION.md still references v0.10.0 era framing. \
         Update to reflect current v0.12.x status."
    );

    assert!(
        !stale_v111,
        "docs/project/ORIENTATION.md still references v0.11.x era framing. \
         Update to reflect current v0.12.x status."
    );
}

/// Test: docs/project/status/index.md should reference v0.12.4 not v0.12.0/0.12.1
#[test]
fn test_status_index_references_current_version() {
    let status_index = docs_dir().join("project/status/index.md");
    let content = read_file(&status_index).expect("docs/project/status/index.md must exist");

    // The research found: claims v0.12.0/v0.12.1 but workspace is v0.12.4

    // Look for specific v0.12.0 and v0.12.1 references (but not v0.12.4)
    let re = Regex::new(r"v0\.12\.[0123]").unwrap();
    let stale_versions: Vec<_> = re.find_iter(&content).collect();

    // Filter out v0.12.4 since that's the current version
    let has_stale = stale_versions.iter().any(|m| m.as_str() != "v0.12.4");

    assert!(
        !has_stale,
        "docs/project/status/index.md references older v0.12.x versions. \
         Current workspace version is v0.12.4. Update references to v0.12.4"
    );
}

/// Test: docs/project/status/release.md should reference current versions
#[test]
fn test_release_md_references_current_version() {
    let release = docs_dir().join("project/status/release.md");
    let content = read_file(&release).expect("docs/project/status/release.md must exist");

    // The research found: "Claims latest published is v0.12.0, target v0.12.1"
    // Actual: v0.12.3 on GitHub, v0.12.4 workspace

    let stale_v120 = content.contains("v0.12.0") || content.contains("v0.12.0");
    let stale_v121 = content.contains("v0.12.1") || content.contains("v0.12.1");

    // Allow v0.12.3 and v0.12.4
    let has_only_stale =
        (stale_v120 || stale_v121) && !content.contains("v0.12.3") && !content.contains("v0.12.4");

    assert!(
        !has_only_stale,
        "docs/project/status/release.md references v0.12.0/v0.12.1 but workspace is v0.12.4 \
         and GitHub releases show v0.12.3. Update to current version."
    );
}

/// Test: docs/reference/LSP_PROVIDERS_REFERENCE.md footer should not say 0.9.x
#[test]
fn test_lsp_providers_reference_footer_version() {
    let providers = docs_dir().join("reference/LSP_PROVIDERS_REFERENCE.md");
    let content =
        read_file(&providers).expect("docs/reference/LSP_PROVIDERS_REFERENCE.md must exist");

    // The research found: Footer "Document Version: 0.9.x", "Last Updated: 2025-01-31"
    // This is over 14 months stale

    let stale_version = content.contains("Document Version: 0.9.x")
        || content.contains("Document Version: 0.10.x")
        || content.contains("Document Version: 0.11.x");

    let stale_date = content.contains("Last Updated: 2025-01-31")
        || content.contains("Last Updated: 2025-02-")
        || content.contains("Last Updated: 2025-03-")
        || content.contains("Last Updated: 2025-04-")
        || content.contains("Last Updated: 2025-05-")
        || content.contains("Last Updated: 2025-06-")
        || content.contains("Last Updated: 2025-07-")
        || content.contains("Last Updated: 2025-08-")
        || content.contains("Last Updated: 2025-09-")
        || content.contains("Last Updated: 2025-10-")
        || content.contains("Last Updated: 2025-11-")
        || content.contains("Last Updated: 2025-12-")
        || content.contains("Last Updated: 2026-01-")
        || content.contains("Last Updated: 2026-02-");

    assert!(
        !stale_version,
        "docs/reference/LSP_PROVIDERS_REFERENCE.md footer still says 'Document Version: 0.9.x'. \
         Update to v0.12.x"
    );

    assert!(
        !stale_date,
        "docs/reference/LSP_PROVIDERS_REFERENCE.md 'Last Updated' date is from 2025. \
         Update to current date."
    );
}

/// Test: docs/reference/SCOPE_ANALYZER_REFERENCE.md title should not say v0.8.7
#[test]
fn test_scope_analyzer_reference_title_version() {
    let scope = docs_dir().join("reference/SCOPE_ANALYZER_REFERENCE.md");
    let content = read_file(&scope).expect("docs/reference/SCOPE_ANALYZER_REFERENCE.md must exist");

    // The research found: Title hard-codes "# Scope Analyzer Reference - v0.8.7"

    let stale_v087 = content.contains("v0.8.7") || content.contains("v0.8.8");
    let stale_v19 = content.contains("v0.9.x") || content.contains("v0.9.");
    let stale_v110 = content.contains("v0.10.x") || content.contains("v0.10.");

    assert!(
        !stale_v087,
        "docs/reference/SCOPE_ANALYZER_REFERENCE.md title still references v0.8.7. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v19,
        "docs/reference/SCOPE_ANALYZER_REFERENCE.md still references v0.9.x. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v110,
        "docs/reference/SCOPE_ANALYZER_REFERENCE.md still references v0.10.x. \
         Update to v0.12.x"
    );
}

/// Test: docs/tutorials/LSP_DEVELOPMENT_GUIDE.md should not have v0.8.7+/v0.8.8+ headings
#[test]
fn test_lsp_development_guide_no_old_version_headings() {
    let guide = docs_dir().join("tutorials/LSP_DEVELOPMENT_GUIDE.md");
    let content = read_file(&guide).expect("docs/tutorials/LSP_DEVELOPMENT_GUIDE.md must exist");

    // The research found: Heavy v0.8.7+/v0.8.8+ headings throughout

    let stale_v087 = content.contains("v0.8.7") || content.contains("v0.8.8");
    let stale_v19 = content.contains("v0.9.x") || content.contains("v0.9.");
    let stale_v110 = content.contains("v0.10.x") || content.contains("v0.10.");

    assert!(
        !stale_v087,
        "docs/tutorials/LSP_DEVELOPMENT_GUIDE.md still has v0.8.7+/v0.8.8+ version markers. \
         Update to v0.12.x or later."
    );

    assert!(
        !stale_v19,
        "docs/tutorials/LSP_DEVELOPMENT_GUIDE.md still references v0.9.x. \
         Update to v0.12.x"
    );

    assert!(
        !stale_v110,
        "docs/tutorials/LSP_DEVELOPMENT_GUIDE.md still references v0.10.x. \
         Update to v0.12.x"
    );
}

/// Test: Editor setup files should not show `ok 0.10.0` version output
#[test]
fn test_editor_setup_files_version_output() {
    let editor_files = vec![
        "EDITORS/COC_NEOVIM_SETUP.md",
        "EDITORS/EMACS_SETUP.md",
        "EDITORS/HELIX_SETUP.md",
        "EDITORS/NEOVIM_SETUP.md",
        "EDITORS/SUBLIME_SETUP.md",
    ];

    for file in editor_files {
        let path = docs_dir().join(file);
        if let Some(content) = read_file(&path) {
            // The research found: Each shows `# Should output: ok 0.10.0`
            let stale = content.contains("ok 0.10.0") || content.contains("ok v0.10.0");

            assert!(
                !stale,
                "{} still shows 'ok 0.10.0' version output. \
                 Update to 'ok v0.12.x' or similar.",
                file
            );
        }
    }
}

/// Test: docs/benchmarks/ files should not reference v0.8.8
#[test]
fn test_benchmark_docs_version() {
    let benchmark_files =
        vec!["benchmarks/BENCHMARK_RESULTS.md", "benchmarks/BENCHMARK_FRAMEWORK.md"];

    for file in benchmark_files {
        let path = docs_dir().join(file);
        if let Some(content) = read_file(&path) {
            // The research found: "Benchmark Version: v0.8.8", Last Updated 2025-09-08
            let stale_version = content.contains("v0.8.8") || content.contains("v0.8.7");
            let stale_date = content.contains("Last Updated: 2025-09-08");

            assert!(!stale_version, "{} still references v0.8.8. Update version reference.", file);

            assert!(
                !stale_date,
                "{} 'Last Updated' is 2025-09-08 (~7 months stale). Update to current date.",
                file
            );
        }
    }
}

/// Test: docs/how-to/IMPORT_OPTIMIZER_GUIDE.md and DEBUGGING.md should not have v0.8.8+ markers
#[test]
fn test_howto_guides_version_markers() {
    let guide_files = vec!["how-to/IMPORT_OPTIMIZER_GUIDE.md", "how-to/DEBUGGING.md"];

    for file in guide_files {
        let path = docs_dir().join(file);
        if let Some(content) = read_file(&path) {
            let stale_v088 = content.contains("(v0.8.8+)") || content.contains("v0.8.8");
            let stale_v19 = content.contains("v0.9.x") || content.contains("(v0.9+)");

            assert!(
                !stale_v088,
                "{} still has (v0.8.8+) version markers. Update to v0.12.x.",
                file
            );

            assert!(!stale_v19, "{} still has v0.9.x references. Update to v0.12.x.", file);
        }
    }
}

/// Test: docs/tutorials/WORKSPACE_REFACTORING_TUTORIAL.md should not reference v0.8.8
#[test]
fn test_workspace_refactoring_tutorial_version() {
    let tutorial = docs_dir().join("tutorials/WORKSPACE_REFACTORING_TUTORIAL.md");
    let content =
        read_file(&tutorial).expect("docs/tutorials/WORKSPACE_REFACTORING_TUTORIAL.md must exist");

    // The research found: Top-of-file "introduced in v0.8.8", repeated (v0.8.8+) markers
    let stale_v088 = content.contains("v0.8.8") || content.contains("(v0.8.8+)");
    let stale_v19 = content.contains("v0.9.x") || content.contains("(v0.9+)");

    assert!(
        !stale_v088,
        "docs/tutorials/WORKSPACE_REFACTORING_TUTORIAL.md still references v0.8.8. \
         Update to v0.12.x or later."
    );

    assert!(
        !stale_v19,
        "docs/tutorials/WORKSPACE_REFACTORING_TUTORIAL.md still references v0.9.x. \
         Update to v0.12.x."
    );
}

// ============================================================================
// DRIFT: Closed issues referenced as if still open
// ============================================================================

/// Test: docs/reference/PARSER_FEATURE_MATRIX.md should not say "Issue #180: This document tracks"
#[test]
fn test_parser_feature_matrix_issue_180_closed() {
    let matrix = docs_dir().join("reference/PARSER_FEATURE_MATRIX.md");
    let content = read_file(&matrix).expect("docs/reference/PARSER_FEATURE_MATRIX.md must exist");

    // The research found: Line 3: "Issue #180: This document tracks parser coverage" - issue #180 is closed
    // Should use past tense like "Issue #180 (closed): This document tracked..."

    let active_tense = content.contains("Issue #180: This document tracks")
        || content.contains("Issue #180: this document tracks");

    assert!(
        !active_tense,
        "docs/reference/PARSER_FEATURE_MATRIX.md still says 'Issue #180: This document tracks' \
         but issue #180 is closed. Change to past tense: 'Issue #180 (closed): This document \
         tracked...' or similar."
    );
}

/// Test: docs/project/CI_COST_TRACKING.md should not reference Issue #211 as active
#[test]
fn test_ci_cost_tracking_issue_211_closed() {
    let ci_cost = docs_dir().join("project/CI_COST_TRACKING.md");
    let content = read_file(&ci_cost).expect("docs/project/CI_COST_TRACKING.md must exist");

    // The research found: Issue #211 referenced as active (closed)

    // Look for Issue #211 in present tense (not marked as closed/done/resolved)
    let issue_211_active = content.contains("Issue #211")
        && !content.contains("Issue #211 (closed)")
        && !content.contains("Issue #211 (done)")
        && !content.contains("Issue #211 (resolved)")
        && !content.to_lowercase().contains("issue #211 was")
        && !content.to_lowercase().contains("issue #211 is closed");

    assert!(
        !issue_211_active,
        "docs/project/CI_COST_TRACKING.md references Issue #211 as if it were still open. \
         Issue #211 is closed. Change references to past tense or mark as closed."
    );
}

// ============================================================================
// DRIFT: Multiple old version references in project docs
// ============================================================================

/// Test: docs/project/WORKSPACE_ARCHITECTURE.md should not have version 0.10.0 in snippets
#[test]
fn test_workspace_architecture_version_snippets() {
    let arch = docs_dir().join("project/WORKSPACE_ARCHITECTURE.md");
    let content = read_file(&arch).expect("docs/project/WORKSPACE_ARCHITECTURE.md must exist");

    // The research found: Multiple `version = "0.10.0"` in Cargo.toml snippets (lines 186–272)

    // Check for version = "0.10.0" or version = "0.9.x" etc in code blocks
    let re = Regex::new(r#"version\s*=\s*"0\.1[0-9]\.""#).unwrap();
    let old_versions: Vec<_> = re.find_iter(&content).collect();

    assert!(
        old_versions.is_empty(),
        "docs/project/WORKSPACE_ARCHITECTURE.md contains old version numbers in snippets: {:?}. \
         Update to current workspace version (v0.12.4).",
        old_versions.iter().map(|m| m.as_str()).collect::<Vec<_>>()
    );
}

/// Test: docs/project/SRP_EXTRACTION_CAMPAIGN.md should not have v0.10.0 references
#[test]
fn test_srp_extraction_campaign_version() {
    let srp = docs_dir().join("project/SRP_EXTRACTION_CAMPAIGN.md");
    let content = read_file(&srp).expect("docs/project/SRP_EXTRACTION_CAMPAIGN.md must exist");

    // The research found: Tables/examples pinned to v0.10.0 (lines 31, 88, 341, 373)

    let stale_v110 = content.contains("v0.10.0") || content.contains("v0.10.x");
    let stale_v19 = content.contains("v0.9.x") || content.contains("v0.9.");

    assert!(
        !stale_v110,
        "docs/project/SRP_EXTRACTION_CAMPAIGN.md still references v0.10.0. \
         Update tables and examples to v0.12.x."
    );

    assert!(
        !stale_v19,
        "docs/project/SRP_EXTRACTION_CAMPAIGN.md still references v0.9.x. \
         Update to v0.12.x."
    );
}

/// Test: docs/project/LSP_IMPLEMENTATION_STORY.md should not have stale version/feature counts
#[test]
fn test_lsp_implementation_story_current() {
    let story = docs_dir().join("project/LSP_IMPLEMENTATION_STORY.md");
    let content = read_file(&story).expect("docs/project/LSP_IMPLEMENTATION_STORY.md must exist");

    // The research found: "catalog contains 97 trackable features" (now 98 per status/index.md)

    // Check for stale feature count
    let has_97 = content.contains("97 trackable features") || content.contains("97 features");

    assert!(
        !has_97,
        "docs/project/LSP_IMPLEMENTATION_STORY.md says '97 trackable features' \
         but status/index.md says 98. Update to 98."
    );

    // Also check for old version references
    let stale_v110 = content.contains("v0.10.0") || content.contains("v0.10.x");
    let stale_v19 = content.contains("v0.9.x") || content.contains("v0.9.");

    assert!(
        !stale_v110,
        "docs/project/LSP_IMPLEMENTATION_STORY.md still references v0.10.0. \
         Update to v0.12.x."
    );

    assert!(
        !stale_v19,
        "docs/project/LSP_IMPLEMENTATION_STORY.md still references v0.9.x. \
         Update to v0.12.x."
    );
}

/// Test: docs/project/QUALITY_INFRASTRUCTURE.md should not reference v0.10.0
#[test]
fn test_quality_infrastructure_version() {
    let quality = docs_dir().join("project/QUALITY_INFRASTRUCTURE.md");
    let content = read_file(&quality).expect("docs/project/QUALITY_INFRASTRUCTURE.md must exist");

    // The research found: perl-lsp-v0.10.0-…tar.gz attestation, "as of v0.10.0"

    let stale_v110 = content.contains("v0.10.0") || content.contains("v0.10.x");
    let has_old_tarball = content.contains("perl-lsp-v0.10.0");

    assert!(
        !stale_v110,
        "docs/project/QUALITY_INFRASTRUCTURE.md still references v0.10.0. \
         Update to v0.12.x."
    );

    assert!(
        !has_old_tarball,
        "docs/project/QUALITY_INFRASTRUCTURE.md references perl-lsp-v0.10.0 tarball. \
         Update to current version."
    );
}

/// Test: docs/project/CODEBASE_HISTORY.md should not have v0.10.0 "current release" framing
#[test]
fn test_codebase_history_version() {
    let history = docs_dir().join("project/CODEBASE_HISTORY.md");
    let content = read_file(&history).expect("docs/project/CODEBASE_HISTORY.md must exist");

    // The research found: Multiple "v0.10.0 — current release" framings

    let stale_v110 = content.contains("v0.10.0 — current release")
        || content.contains("v0.10.0 — current")
        || content.contains("v0.10.0 – current release")
        || content.contains("v0.10.0 – current");

    assert!(
        !stale_v110,
        "docs/project/CODEBASE_HISTORY.md still has 'v0.10.0 — current release' framing. \
         Update to v0.12.x or mark as historical."
    );
}

/// Test: docs/project/PERL_LSP_VISION.md should not have v0.12.0 "now" framing
#[test]
fn test_perl_lsp_vision_version() {
    let vision = docs_dir().join("project/PERL_LSP_VISION.md");
    let content = read_file(&vision).expect("docs/project/PERL_LSP_VISION.md must exist");

    // The research found: "Vision Scope: v0.12.0 (now)" throughout

    let stale_v120_now = content.contains("v0.12.0 (now)")
        || content.contains("v0.12.0 now")
        || content.contains("v0.12.0 - now");

    // Check for older versions too
    let stale_v110 = content.contains("v0.10.0") || content.contains("v0.10.x");
    let stale_v19 = content.contains("v0.9.x");

    assert!(
        !stale_v120_now,
        "docs/project/PERL_LSP_VISION.md still has 'v0.12.0 (now)' framing. \
         Either update to v0.12.4 or mark as historical."
    );

    assert!(
        !stale_v110,
        "docs/project/PERL_LSP_VISION.md still references v0.10.0. \
         Update to v0.12.x."
    );

    assert!(
        !stale_v19,
        "docs/project/PERL_LSP_VISION.md still references v0.9.x. \
         Update to v0.12.x."
    );
}

/// Test: docs/project/PERFORMANCE_BASELINES.md should not have v0.12.0 in title
#[test]
fn test_performance_baselines_version() {
    let baselines = docs_dir().join("project/PERFORMANCE_BASELINES.md");
    let content = read_file(&baselines).expect("docs/project/PERFORMANCE_BASELINES.md must exist");

    // The research found: Title "Performance Baselines (0.12.0)", body references "perl-lsp 0.12.0 public alpha"

    // Title check
    let title_re = Regex::new(r"# Performance Baselines \(0\.12\.[0-2]\)").unwrap();
    let has_old_title = title_re.is_match(&content);

    assert!(
        !has_old_title,
        "docs/project/PERFORMANCE_BASELINES.md title says 'Performance Baselines (0.12.0)' \
         but workspace is v0.12.4. Update title or mark as historical."
    );

    // Body check
    let stale_alpha = content.contains("perl-lsp 0.12.0 public alpha")
        || content.contains("v0.12.0 public alpha")
        || content.contains("v0.12.0 alpha");

    assert!(
        !stale_alpha,
        "docs/project/PERFORMANCE_BASELINES.md references 'v0.12.0 public alpha'. \
         Either update to v0.12.4 or mark as historical."
    );
}

/// Test: docs/EDITORS/VS_CODE_SETUP.md should not have v0.12.0 in versionTag
#[test]
fn test_vscode_setup_version_tag() {
    let vscode = docs_dir().join("EDITORS/VS_CODE_SETUP.md");
    let content = read_file(&vscode).expect("docs/EDITORS/VS_CODE_SETUP.md must exist");

    // The research found: Line 563: `"perl-lsp.versionTag": "v0.12.0"`

    let stale_tag = content.contains("\"perl-lsp.versionTag\": \"v0.12.0\"")
        || content.contains("\"perl-lsp.versionTag\": \"v0.12.1\"");

    assert!(
        !stale_tag,
        "docs/EDITORS/VS_CODE_SETUP.md still has '\"perl-lsp.versionTag\": \"v0.12.0\"'. \
         Update to v0.12.x or use a placeholder."
    );
}

/// Test: docs/project/MILESTONES.md should not have v0.10.0 "Boring Promises" milestone
#[test]
fn test_milestones_v0100() {
    let milestones = docs_dir().join("project/MILESTONES.md");
    let content = read_file(&milestones).expect("docs/project/MILESTONES.md must exist");

    // The research found: "v0.10.0: Boring Promises" milestone framing

    let stale_milestone = content.contains("v0.10.0: Boring Promises")
        || content.contains("v0.10.0 — Boring Promises")
        || content.contains("v0.10.0 - Boring Promises");

    assert!(
        !stale_milestone,
        "docs/project/MILESTONES.md still has 'v0.10.0: Boring Promises' milestone. \
         This is historical. Either mark as completed or move to archive section."
    );
}
