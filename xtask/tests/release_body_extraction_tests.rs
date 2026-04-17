//! Integration tests for GitHub Release body sourcing from docs/releases/vX.Y.Z.md
//!
//! These tests verify that the GitHub Actions release workflow has been updated
//! to source the release body from the markdown file instead of using hardcoded content.
//!
//! The release.yml workflow should:
//! 1. Read the release notes file: docs/releases/v{version}.md
//! 2. Extract the body content (after the YAML frontmatter --- markers)
//! 3. Use the extracted body as the GitHub Release body
//!
//! These tests will FAIL until the feature is implemented in the release.yml workflow.

use std::env;
use std::fs;
use std::path::PathBuf;

/// Get the repo root directory (parent of xtask/)
fn repo_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    let xtask_dir = PathBuf::from(manifest_dir);
    xtask_dir.parent().expect("xtask should have a parent").to_path_buf()
}

/// Test that the release workflow reads release notes from docs/releases/
/// This test verifies the workflow file has been updated to source release body
/// from the markdown file.
#[test]
fn test_release_workflow_sources_release_notes_from_file() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");

    // Read the workflow file
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // The workflow should read from docs/releases/v{version}.md
    // Look for a step that reads the release notes file
    let reads_release_notes = workflow_content.contains("docs/releases/v${VERSION}.md")
        || workflow_content
            .contains("docs/releases/v${{ needs.release-metadata.outputs.version }}.md")
        || workflow_content.contains("docs/releases/${VERSION}.md")
        || workflow_content.contains("'docs/releases/' + version + '.md'")
        || workflow_content.contains("$(cat docs/releases/)")
        || workflow_content.contains("source docs/releases/");

    assert!(
        reads_release_notes,
        "release.yml workflow should read release notes from docs/releases/v${{version}}.md. \
         Currently it uses hardcoded release notes in the Generate release notes step. \
         Expected to find a command that reads docs/releases/... but found none."
    );
}

/// Test that the release workflow does NOT use hardcoded release notes
#[test]
fn test_release_workflow_does_not_use_hardcoded_notes() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");

    // Read the workflow file
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // The hardcoded release notes contain "cargo install perllsp"
    let has_hardcoded_notes = workflow_content.contains("## Perl LSP")
        && workflow_content.contains("cargo install perllsp");

    assert!(
        !has_hardcoded_notes,
        "release.yml workflow should NOT have hardcoded release notes starting with \
         '## Perl LSP' and containing 'cargo install perllsp'. \
         These notes should be sourced from docs/releases/vX.Y.Z.md instead."
    );
}

/// Test that the release workflow extracts body after frontmatter separator
#[test]
fn test_release_workflow_extracts_body_after_frontmatter() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");

    // Read the workflow file
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // The workflow should skip the YAML frontmatter (between --- markers)
    // This is typically done with `sed`, `tail`, or similar commands
    let extracts_frontmatter = workflow_content.contains("---")
        && (workflow_content.contains("tail -n")
            || workflow_content.contains("sed -n")
            || workflow_content.contains("awk")
            || workflow_content.contains("grep -A"));

    assert!(
        extracts_frontmatter,
        "release.yml workflow should extract body after the --- frontmatter separator. \
         Expected to find commands like tail -n, sed -n, or similar to skip frontmatter."
    );
}

/// Test that the actual release notes file exists and has correct format
#[test]
fn test_release_notes_file_exists_with_frontmatter_format() {
    let releases_dir = repo_root().join("docs/releases");

    if !releases_dir.exists() {
        panic!("docs/releases directory should exist");
    }

    // Read the most recent release file (v0.12.4.md or similar)
    let release_files: Vec<_> = fs::read_dir(&releases_dir)
        .expect("should be able to read docs/releases")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    if release_files.is_empty() {
        panic!("docs/releases should have at least one release notes file");
    }

    // Check the most recent file (highest version)
    let mut release_files: Vec<_> = release_files;
    release_files.sort_by(|a, b| {
        let name_a_binding = a.file_name();
        let name_b_binding = b.file_name();
        let name_a = name_a_binding.to_string_lossy();
        let name_b = name_b_binding.to_string_lossy();
        name_b.cmp(&name_a) // Descending order
    });

    let latest_release = &release_files[0];
    let content =
        fs::read_to_string(latest_release.path()).expect("should be able to read release file");

    // Verify frontmatter format
    let has_frontmatter_start = content.starts_with("---\n");
    let frontmatter_count = content.matches("---\n").count();

    assert!(
        has_frontmatter_start,
        "Release file {} should start with --- frontmatter",
        latest_release.file_name().to_string_lossy()
    );

    assert!(
        frontmatter_count >= 2,
        "Release file {} should have at least 2 --- markers (start and end of frontmatter), found {}",
        latest_release.file_name().to_string_lossy(),
        frontmatter_count
    );

    // Verify version in frontmatter
    let has_version = content.contains("version:");
    assert!(
        has_version,
        "Release file {} should contain version: in frontmatter",
        latest_release.file_name().to_string_lossy()
    );

    // Verify body starts after second ---
    // Note: after the second --- there's a blank line before the actual content
    let lines: Vec<&str> = content.lines().collect();
    let second_dash_idx = lines[1..].iter().position(|l| *l == "---");

    if let Some(idx) = second_dash_idx {
        // idx is relative to lines[1..], so actual index is idx + 1
        let actual_idx = idx + 1;
        // Skip the --- line and the blank line after it
        // Body starts at actual_idx + 2
        if actual_idx + 2 < lines.len() {
            let body_start = lines[actual_idx + 2];
            // Body should start with # version heading, not with frontmatter content
            assert!(
                body_start.starts_with("# "),
                "Release body should start with # heading, but found: {}",
                body_start
            );
        } else {
            panic!("Release file should have content after frontmatter");
        }
    } else {
        panic!("Release file should have closing --- marker");
    }
}
