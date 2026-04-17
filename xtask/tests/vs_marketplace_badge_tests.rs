//! Tests for VS Marketplace badge URLs in documentation.
//!
//! These tests verify that the deprecated `visual-studio-marketplace` shields.io badge
//! route has been replaced with static equivalents.
//!
//! The deprecated route was:
//!   - https://img.shields.io/visual-studio-marketplace/i/EffortlessMetrics.perl-lsp-rs (installs)
//!   - https://img.shields.io/visual-studio-marketplace/v/EffortlessMetrics.perl-lsp-rs (version)
//!
//! The correct static format is:
//!   - https://img.shields.io/badge/VS%20Marketplace-<count>-0078D4

use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Files that are allowed to contain the deprecated `visual-studio-marketplace` URL
/// (only for documentation/warning purposes).
const ALLOWED_DEPRECATED_URL_FILES: &[&str] = &[
    "VSCODE_MARKETPLACE_PUNCH_LIST.md", // Documents the deprecation warning
];

/// README files that should contain VS Marketplace badges.
const README_FILES_WITH_BADGES: &[&str] = &["README.md", "vscode-extension/README.md"];

/// The correct item name for VS Marketplace.
const CORRECT_ITEM_NAME: &str = "EffortlessMetrics.perl-lsp-rs";

/// The deprecated VS Marketplace badge route pattern.
const DEPRECATED_BADGE_PATTERN: &str = "visual-studio-marketplace";

/// The correct static badge URL pattern that should be used.
const CORRECT_BADGE_PATTERN: &str = "img.shields.io/badge/VS%20Marketplace";

/// Finds all README.md files in the repository.
fn find_readme_files(root: &Path) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name().to_string_lossy().eq_ignore_ascii_case("README.md")
                || e.file_name().to_string_lossy().eq_ignore_ascii_case("README")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Check if a file is in the allowed list for deprecated URLs.
fn is_allowed_for_deprecated_url(path: &Path) -> bool {
    path.file_name()
        .map(|n| {
            ALLOWED_DEPRECATED_URL_FILES
                .iter()
                .any(|&allowed| n.to_string_lossy().eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false)
}

/// Tests that no deprecated `visual-studio-marketplace` badges exist in README files.
/// The only exception is documentation that explains the deprecation itself.
#[test]
fn test_no_deprecated_vs_marketplace_badges_in_readmes() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    let readme_files = find_readme_files(&root);

    // Filter to only READMEs that should have VS Marketplace badges
    let readmes_with_vs_marketplace = readme_files
        .into_iter()
        .filter(|p| {
            p.parent()
                .map(|parent| {
                    parent
                        .file_name()
                        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("vscode-extension"))
                        .unwrap_or(false)
                        || p.file_name()
                            .map(|n| n.to_string_lossy().eq_ignore_ascii_case("README.md"))
                            .unwrap_or(false)
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    for readme_path in readmes_with_vs_marketplace {
        let content = fs::read_to_string(&readme_path)?;

        // If the file contains the deprecated pattern, it must be in the allowed list
        if content.contains(DEPRECATED_BADGE_PATTERN) {
            assert!(
                is_allowed_for_deprecated_url(&readme_path),
                "File '{}' contains deprecated '{}' but is not in the allowed list. \
                 The deprecated VS Marketplace badge route should have been replaced with static badges.",
                readme_path.display(),
                DEPRECATED_BADGE_PATTERN
            );
        }
    }

    Ok(())
}

/// Tests that VS Marketplace badges use the correct static `img.shields.io/badge/...` format.
#[test]
fn test_vs_marketplace_badges_use_static_format() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    for readme_name in README_FILES_WITH_BADGES {
        let readme_path = root.join(readme_name);
        if !readme_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&readme_path)?;

        // Check for VS Marketplace links
        if content.contains("marketplace.visualstudio.com") {
            // If there are VS Marketplace links, there should be static badges (not deprecated)
            assert!(
                content.contains(CORRECT_BADGE_PATTERN),
                "File '{}' contains VS Marketplace links but does not use the correct \
                 static badge format '{}'. Expected: img.shields.io/badge/VS%20Marketplace-...",
                readme_path.display(),
                CORRECT_BADGE_PATTERN
            );
        }
    }

    Ok(())
}

/// Tests that VS Marketplace badge URLs point to the correct item name.
#[test]
fn test_vs_marketplace_badge_urls_use_correct_item_name() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    for readme_name in README_FILES_WITH_BADGES {
        let readme_path = root.join(readme_name);
        if !readme_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&readme_path)?;

        // Check for VS Marketplace links
        if content.contains("marketplace.visualstudio.com") {
            // All VS Marketplace links should use the correct item name
            assert!(
                content.contains(&format!("itemName={}", CORRECT_ITEM_NAME)),
                "File '{}' contains VS Marketplace links but does not use the correct item name \
                 '{}'. Found links should have 'itemName={}'.",
                readme_path.display(),
                CORRECT_ITEM_NAME,
                CORRECT_ITEM_NAME
            );
        }
    }

    Ok(())
}

/// Tests that the VS Code extension PUBLISHING.md contains a reminder to update
/// static badge counts after releases.
#[test]
fn test_publishing_guide_contains_badge_refresh_reminder() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    let publishing_guide = root.join("vscode-extension/PUBLISHING.md");
    if !publishing_guide.exists() {
        return Ok(()); // Skip if file doesn't exist
    }

    let content = fs::read_to_string(&publishing_guide)?;

    // The publishing guide should contain a reminder about manually refreshing badge counts
    // since shields.io deprecated the live VS Marketplace badge route.
    let has_badge_reminder = content.to_lowercase().contains("badge")
        && (content.to_lowercase().contains("refresh")
            || content.to_lowercase().contains("update")
            || content.to_lowercase().contains("manually"));

    assert!(
        has_badge_reminder,
        "File '{}' should contain a reminder to manually refresh/update badge counts \
         after releases, since the deprecated VS Marketplace badge route requires static badges.",
        publishing_guide.display()
    );

    Ok(())
}

/// Tests that the VSCODE_MARKETPLACE_PUNCH_LIST.md documents the deprecation.
#[test]
fn test_punch_list_documents_deprecation() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    let punch_list = root.join("docs/project/VSCODE_MARKETPLACE_PUNCH_LIST.md");
    if !punch_list.exists() {
        return Ok(()); // Skip if file doesn't exist
    }

    let content = fs::read_to_string(&punch_list)?;

    // The punch list should document the deprecation of the visual-studio-marketplace badge route
    assert!(
        content.contains("visual-studio-marketplace")
            && content.to_lowercase().contains("deprecat"),
        "File '{}' should document the deprecation of the 'visual-studio-marketplace' badge route.",
        punch_list.display()
    );

    Ok(())
}
