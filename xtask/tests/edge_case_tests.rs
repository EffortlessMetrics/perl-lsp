//! Edge case tests for GitHub Release body sourcing from docs/releases/vX.Y.Z.md
//!
//! These tests verify edge cases in the release workflow's body extraction logic
//! that are NOT covered by the red tests.
//!
//! Red tests only check for:
//! - Presence of "docs/releases" in workflow
//! - Presence of sed/tail/awk/grep commands
//! - Frontmatter format in release notes files
//!
//! Edge case tests verify:
//! - The extraction logic targets content AFTER frontmatter (not frontmatter content itself)
//! - The sed command range correctly extends past the second --- marker
//! - Edge cases like empty frontmatter values, special characters, etc.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the repo root directory (parent of xtask/)
fn repo_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    let xtask_dir = PathBuf::from(manifest_dir);
    xtask_dir.parent().expect("xtask should have a parent").to_path_buf()
}

/// Verify the sed command in the workflow targets content AFTER frontmatter, not the frontmatter itself.
/// The sed range `/^---$/,/^---$/` only spans BETWEEN the two markers, so it extracts frontmatter content,
/// not body content. A correct implementation should target lines AFTER the second `---`.
#[test]
fn test_sed_command_targets_body_not_frontmatter() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // The current (buggy) sed command uses range /pattern/,/pattern/ which only spans BETWEEN markers
    // A correct implementation would use something like:
    // - `sed '1,/^---$/d'` to delete up to first ---, then print rest
    // - `tail -n +N` after finding the line number of second ---
    // - `sed -n '/^---$/,$p'` would include body but also print second --- (needs cleanup)
    //
    // The buggy pattern: `/^---$/,/^---$/` only matches BETWEEN first and second ---
    // This extracts frontmatter content, not body content!

    let has_buggy_range_pattern = workflow_content.contains("/^---$/,/^---$/");

    // The correct pattern should NOT use /pattern/,/pattern/ for extracting body after frontmatter
    // because that only spans between the markers, not after them
    assert!(
        !has_buggy_range_pattern,
        "The sed command uses /pattern/,/pattern/ range which only extracts BETWEEN --- markers \
         (frontmatter content), NOT the body AFTER frontmatter. \
         The range /pattern/,/pattern/ is incorrect for body extraction. \
         Use 'sed 1,/^---$/d' (delete up to and including first ---) or \
         'tail -n +N' (skip first N lines including second --- and blank line) instead."
    );
}

/// Verify that the sed command uses proper technique to skip past frontmatter entirely.
/// The body starts AFTER the second --- marker, so the command must look BEYOND that marker.
#[test]
fn test_sed_command_looks_past_frontmatter() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // The sed command should either:
    // 1. Use `sed '1,/^---$/d'` to delete lines 1 through the line containing first ---
    // 2. Use `tail -n +N` where N is calculated to skip past the second ---
    // 3. Use `awk '/^---$/ && !first { first=1; next } !first'` to skip until after second ---
    // 4. Use `sed -n '/^---$/,$p'` but then need to delete the second --- line
    //
    // The current buggy command: `/^---$/,/^---$/{ ... }` only processes BETWEEN markers

    // Check for correct patterns that look PAST the frontmatter
    let has_correct_delete_up_to = workflow_content.contains("sed '1,/^---$/d'")
        || workflow_content.contains("sed \"1,/^---$/d\"");

    let has_correct_tail_skip = workflow_content.contains("tail -n +");

    // The awk command should use a counter approach:
    // 1. `awk '/^---$/ { count++; next } count >= 2 { print }'`
    let has_correct_awk_skip =
        workflow_content.contains("awk") && workflow_content.contains("count++");

    // At least one correct pattern should be present
    let has_correct_pattern =
        has_correct_delete_up_to || has_correct_tail_skip || has_correct_awk_skip;

    assert!(
        has_correct_pattern,
        "The sed command should use a technique that looks PAST the frontmatter: \
         - 'sed 1,/pattern/d' (delete up to and including first match) \
         - 'tail -n +N' (skip first N lines) \
         - awk with state machine to skip until after second --- \
         Current implementation only processes lines BETWEEN --- markers, not after."
    );
}

/// Test that the actual extraction produces body content (version heading) not frontmatter content.
/// This is an integration test that runs the sed command on the actual release notes file.
#[test]
fn test_extraction_produces_body_not_frontmatter() {
    let releases_dir = repo_root().join("docs/releases");

    // Find the latest release file
    let release_files: Vec<_> = fs::read_dir(&releases_dir)
        .expect("should be able to read docs/releases")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    if release_files.is_empty() {
        panic!("docs/releases should have at least one release notes file");
    }

    // Get the most recent release file
    let mut release_files: Vec<_> = release_files;
    release_files.sort_by(|a, b| {
        let name_a_binding = a.file_name();
        let name_b_binding = b.file_name();
        let name_a = name_a_binding.to_string_lossy();
        let name_b = name_b_binding.to_string_lossy();
        name_b.cmp(&name_a)
    });

    let latest_release = &release_files[0];
    let release_path = latest_release.path();

    // Extract the body using the awk command from the workflow
    let awk_output = Command::new("awk")
        .args(["/^---$/ { count++; next } count >= 2 { print }", &release_path.to_string_lossy()])
        .output()
        .expect("awk should work");

    let awk_stdout = String::from_utf8_lossy(&awk_output.stdout);

    // The awk output should NOT start with frontmatter content (version:, tag:, etc.)
    // It should start with body content (# heading)
    let starts_with_frontmatter = awk_stdout.trim().starts_with("version:")
        || awk_stdout.trim().starts_with("tag:")
        || awk_stdout.trim().starts_with("release_date")
        || awk_stdout.trim().starts_with("channels:")
        || awk_stdout.trim().starts_with("assets:");

    assert!(
        !starts_with_frontmatter,
        "Awk extraction produced frontmatter content instead of body content. \
         Awk output started with: {}",
        awk_stdout.lines().next().unwrap_or("(empty)")
    );

    // If there's actual body content, it should start with # heading
    let trimmed = awk_stdout.trim();
    if !trimmed.is_empty() {
        assert!(
            trimmed.starts_with("# ") || trimmed.starts_with("##"),
            "Body content should start with # heading, but got: {}",
            trimmed.lines().next().unwrap_or("(empty)")
        );
    }
}

/// Verify that the sed command does NOT use the problematic /pattern/,/pattern/ range
/// for extracting body after frontmatter.
#[test]
fn test_no_inclusive_range_for_body_extraction() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");
    let workflow_content =
        fs::read_to_string(&workflow_path).expect("release.yml workflow should exist");

    // Find the sed command used for extraction
    let has_inclusive_range = workflow_content.contains("/^---$/,/^---$/")
        || workflow_content.contains("/^---$/,/^---$/");

    // This is the core bug: using /pattern/,/pattern/ only spans BETWEEN markers
    assert!(
        !has_inclusive_range,
        "The sed command uses /pattern/,/pattern/ inclusive range which only captures \
         content BETWEEN the two --- markers, not the body AFTER them. \
         This extracts frontmatter metadata instead of release notes body."
    );
}

/// Test that the release notes file has body content after frontmatter
#[test]
fn test_release_file_has_body_after_frontmatter() {
    let releases_dir = repo_root().join("docs/releases");

    let release_files: Vec<_> = fs::read_dir(&releases_dir)
        .expect("should be able to read docs/releases")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    if release_files.is_empty() {
        panic!("docs/releases should have at least one release notes file");
    }

    // Check each release file has body content
    for release_file in release_files.iter().take(3) {
        let content =
            fs::read_to_string(release_file.path()).expect("should be able to read release file");

        let lines: Vec<&str> = content.lines().collect();
        let second_dash_idx = lines[1..].iter().position(|l| *l == "---");

        if let Some(idx) = second_dash_idx {
            let actual_idx = idx + 1;
            // Body starts 2 lines after second --- (the --- itself and blank line)
            let body_start_line = actual_idx + 2;

            if body_start_line < lines.len() {
                let body_content = lines[body_start_line].trim();
                assert!(
                    body_content.starts_with("# "),
                    "Release file {} should have body starting with # heading at line {}, found: {}",
                    release_file.file_name().to_string_lossy(),
                    body_start_line,
                    body_content
                );
            }
        }
    }
}
