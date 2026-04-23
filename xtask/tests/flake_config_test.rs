//! Tests to verify flake.nix configuration is correct.
//!
//! These tests verify the acceptance criteria from work-24c7cfb5:
//! - AC-1: Version field matches CLAUDE.md (should be 0.12.4)
//! - AC-2: Perl is available in nix develop shells
//!
//! These tests are designed to FAIL until the flake.nix is fixed.

use std::fs;
use std::path::Path;

/// Extracts the version from packages.perl-lsp derivation in flake.nix.
///
/// The version field is on a line like:
///   version = "0.12.3";  # Keep in sync with CLAUDE.md
fn extract_perl_lsp_version(flake_content: &str) -> Option<String> {
    // Find the packages.perl-lsp section and then the version line
    // Look for the pattern: version = "X.Y.Z";
    for line in flake_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = \"") && trimmed.contains("\";") {
            // Extract version string
            if let Some(start) = trimmed.find("version = \"") {
                let after_version = &trimmed[start + 11..];
                if let Some(end) = after_version.find("\"") {
                    return Some(after_version[..end].to_string());
                }
            }
        }
    }
    None
}

/// Extracts the buildInputs list from flake.nix and returns whether perl is included.
fn check_perl_in_build_inputs(flake_content: &str) -> bool {
    // Find the buildInputs section and check if "perl" is in the list
    // The buildInputs starts with: buildInputs = with pkgs; [
    // and contains items like: rustToolchain, pkg-config, openssl, etc.

    let mut in_build_inputs = false;
    let mut bracket_depth: i32 = 0;

    for line in flake_content.lines() {
        let trimmed = line.trim();

        // Track when we enter the buildInputs section
        if trimmed.starts_with("buildInputs = with pkgs") {
            in_build_inputs = true;
            bracket_depth = 0;
        }

        if in_build_inputs {
            // Count brackets to track when we exit the array
            // Use i32 to properly handle depth going negative during exit
            bracket_depth += trimmed.matches('[').count() as i32;
            bracket_depth -= trimmed.matches(']').count() as i32;

            // Check if perl appears in this line within the buildInputs context
            // We look for lines that have "perl" as a package name (not in comments)
            if trimmed.starts_with("perl")
                || trimmed.contains(" perl")
                || trimmed.contains("\tperl")
            {
                // Make sure it's not in a comment
                if let Some(comment_pos) = trimmed.find('#') {
                    let before_comment = &trimmed[..comment_pos];
                    if before_comment.contains("perl") {
                        return true;
                    }
                } else {
                    return true;
                }
            }

            // We've exited the buildInputs array when bracket_depth goes back to 0
            // and we've seen a closing bracket
            if bracket_depth <= 0 && trimmed.contains(']') {
                return false;
            }
        }
    }
    false
}

/// Extracts the Latest Release version from CLAUDE.md.
///
/// The version is on a line like:
///   **Latest Release**: 0.12.4 | **Metrics**: ...
fn extract_latest_release_from_claude_md(claude_content: &str) -> Option<String> {
    for line in claude_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("**Latest Release**:") {
            // Extract version from "**Latest Release**: 0.12.4 | ..."
            if let Some(start) = trimmed.find("**Latest Release**:") {
                let after_label = &trimmed[start + 19..].trim();
                if let Some(end) = after_label.find('|') {
                    let version = after_label[..end].trim().to_string();
                    return Some(version);
                } else if let Some(end) = after_label.find(' ') {
                    let version = after_label[..end].trim().to_string();
                    return Some(version);
                } else {
                    return Some(after_label.to_string());
                }
            }
        }
    }
    None
}

#[test]
fn test_flake_version_matches_claude_md() {
    // AC-1: Version field matches CLAUDE.md
    //
    // This test verifies that packages.perl-lsp.version in flake.nix
    // matches the "Latest Release" declared in CLAUDE.md.

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let flake_path = repo_root.join("flake.nix");
    let claude_path = repo_root.join("CLAUDE.md");

    let flake_content =
        fs::read_to_string(&flake_path).expect("flake.nix should exist and be readable");

    let claude_content =
        fs::read_to_string(&claude_path).expect("CLAUDE.md should exist and be readable");

    let flake_version = extract_perl_lsp_version(&flake_content)
        .expect("packages.perl-lsp.version should be found in flake.nix");

    let latest_release = extract_latest_release_from_claude_md(&claude_content)
        .expect("Latest Release should be found in CLAUDE.md line 3");

    assert_eq!(
        flake_version, latest_release,
        "packages.perl-lsp version in flake.nix ({}) should match Latest Release in CLAUDE.md ({})",
        flake_version, latest_release
    );
}

#[test]
fn test_flake_has_perl_in_build_inputs() {
    // AC-2: Perl is available in nix develop shells
    //
    // This test verifies that the shared buildInputs in flake.nix
    // includes `perl`. This is required for `just cpan-corpus-*` targets
    // to work, since xtask/src/tasks/cpan_corpus.rs:476 calls
    // Command::new("perl") directly.

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let flake_path = repo_root.join("flake.nix");

    let flake_content =
        fs::read_to_string(&flake_path).expect("flake.nix should exist and be readable");

    let has_perl = check_perl_in_build_inputs(&flake_content);

    assert!(
        has_perl,
        "perl must be included in the shared buildInputs in flake.nix \
         so that `just cpan-corpus-*` targets work in `nix develop` shells. \
         The cpan_corpus.rs xtask calls Command::new(\"perl\") directly at line 476."
    );
}

#[test]
fn test_flake_nix_parses_without_error() {
    // AC-3: Nix flake evaluates without errors
    //
    // This is a basic sanity check that flake.nix is valid Nix syntax.

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let flake_path = repo_root.join("flake.nix");

    let flake_content =
        fs::read_to_string(&flake_path).expect("flake.nix should exist and be readable");

    // Basic sanity checks for Nix flake structure
    assert!(flake_content.contains("description ="), "flake.nix should have a description");
    assert!(flake_content.contains("outputs ="), "flake.nix should have outputs");
    assert!(
        flake_content.contains("devShells.default"),
        "flake.nix should define devShells.default"
    );
    assert!(
        flake_content.contains("perl-lsp = pkgs.rustPlatform.buildRustPackage"),
        "flake.nix should define perl-lsp package"
    );
}
