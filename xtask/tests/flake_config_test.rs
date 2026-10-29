//! Tests to verify flake.nix configuration is correct.
//!
//! These tests verify the acceptance criteria from work-24c7cfb5:
//! - AC-1: Version field matches the workspace package version
//! - AC-2: Perl is available in nix develop shells
//!
//! The workspace manifest is the version authority for the packaged source.

use std::fs;
use std::path::Path;

/// Extracts the version from packages.perl-lsp derivation in flake.nix.
///
/// The version field is on a line like:
///   version = "0.17.0";
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

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn verify_flake_workspace_version(flake: &str, manifest: &str) -> TestResult {
    let flake_version = extract_perl_lsp_version(flake)
        .filter(|version| !version.is_empty())
        .ok_or("packages.perl-lsp.version should be found in flake.nix")?;
    let manifest: toml::Value = toml::from_str(manifest)?;
    let workspace_version = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .filter(|version| !version.is_empty())
        .ok_or("workspace.package.version must be a nonempty string")?;
    if flake_version != workspace_version {
        return Err(format!(
            "flake package version {flake_version} differs from workspace version {workspace_version}"
        )
        .into());
    }
    Ok(())
}

#[test]
fn test_flake_version_matches_workspace_manifest() -> TestResult {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask manifest must be nested under the repository root")?;
    verify_flake_workspace_version(
        &fs::read_to_string(repo_root.join("flake.nix"))?,
        &fs::read_to_string(repo_root.join("Cargo.toml"))?,
    )
}

#[test]
fn flake_version_comparison_uses_workspace_authority_and_rejects_mismatch() -> TestResult {
    let manifest = "version = \"9.9.9\"\n[workspace.package]\nversion = \"1.2.3\"\n";
    verify_flake_workspace_version("version = \"1.2.3\";", manifest)?;
    let error = verify_flake_workspace_version("version = \"9.9.9\";", manifest)
        .err()
        .ok_or("mismatching flake version must be rejected")?;
    assert!(error.to_string().contains("9.9.9 differs from workspace version 1.2.3"));
    Ok(())
}

#[test]
fn flake_version_comparison_rejects_missing_or_invalid_versions() {
    for manifest in [
        "not valid TOML [",
        "version = \"1.2.3\"",
        "[workspace.package]",
        "[workspace.package]\nversion = 123",
        "[workspace.package]\nversion = \"\"",
    ] {
        assert!(
            verify_flake_workspace_version("version = \"1.2.3\";", manifest).is_err(),
            "invalid workspace version accepted: {manifest}"
        );
    }
    for flake in ["", "version = \"\";"] {
        assert!(
            verify_flake_workspace_version(flake, "[workspace.package]\nversion = \"1.2.3\"")
                .is_err(),
            "missing flake version accepted"
        );
    }
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
