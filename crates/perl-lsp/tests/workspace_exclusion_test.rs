//! Test to validate workspace exclusion strategy
//!
//! This test ensures that:
//! 1. Excluded crates are properly isolated from the workspace
//! 2. Workspace members don't accidentally depend on excluded crates
//! 3. Excluded crates can still be built independently when needed

use perl_tdd_support::{must, must_some};
use std::path::Path;

#[test]
fn test_workspace_excludes_documented_crates() {
    // Navigate to workspace root (two levels up from perl-lsp crate)
    let workspace_root =
        must_some(Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()));

    // Expected exclusions as documented in Cargo.toml
    // Note: tree-sitter-perl-c was removed from the repo; only check dirs that exist
    let expected_exclusions = vec!["tree-sitter-perl", "fuzz", "archive"];

    for excluded in expected_exclusions {
        let excluded_path = workspace_root.join(excluded);
        assert!(
            excluded_path.exists(),
            "Excluded directory '{}' should exist at: {}",
            excluded,
            excluded_path.display()
        );
    }
}

#[test]
fn test_excluded_crates_have_cargo_toml() {
    let workspace_root =
        must_some(Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()));

    // Crates that should have their own Cargo.toml for independent building
    let crates_with_manifest = vec!["fuzz/Cargo.toml"];

    for manifest_path in crates_with_manifest {
        let full_path = workspace_root.join(manifest_path);
        assert!(
            full_path.exists(),
            "Excluded crate manifest should exist: {}",
            full_path.display()
        );
    }
}

#[test]
fn test_workspace_toml_excludes_section() {
    let workspace_root =
        must_some(Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()));

    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let cargo_toml_content = must(std::fs::read_to_string(&cargo_toml_path));

    // Verify the exclusions are present in Cargo.toml
    assert!(
        cargo_toml_content.contains("exclude = ["),
        "Cargo.toml should have an exclude section"
    );

    assert!(
        cargo_toml_content.contains("tree-sitter-perl"),
        "tree-sitter-perl should be in exclusions"
    );

    assert!(cargo_toml_content.contains("\"fuzz\""), "fuzz should be in exclusions");

    assert!(cargo_toml_content.contains("\"archive\""), "archive should be in exclusions");
}

#[test]
fn test_workspace_dependencies_dont_reference_excluded() {
    let workspace_root =
        must_some(Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()));

    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let cargo_toml_content = must(std::fs::read_to_string(&cargo_toml_path));

    // Parse workspace.dependencies section
    let lines: Vec<&str> = cargo_toml_content.lines().collect();
    let mut in_workspace_deps = false;
    let mut found_excluded_ref = false;

    for line in lines {
        if line.starts_with("[workspace.dependencies]") {
            in_workspace_deps = true;
            continue;
        }

        if in_workspace_deps {
            // Stop at next section
            if line.starts_with('[') && !line.starts_with("# ") {
                break;
            }

            // Check for references to excluded crate directories in path declarations.
            // The crate `tree-sitter-perl` at `crates/tree-sitter-perl-rs` is a workspace
            // member, not an excluded crate, so we only flag paths pointing into excluded dirs.
            if !line.trim().starts_with('#')
                && (line.contains("path = \"tree-sitter-perl-c")
                    || line.contains("path = \"tree-sitter-perl/")
                    || line.contains("path = \"tree-sitter-perl\""))
            {
                found_excluded_ref = true;
                break;
            }
        }
    }

    assert!(
        !found_excluded_ref,
        "workspace.dependencies should not reference excluded crates (found reference in non-comment line)"
    );
}

#[test]
fn test_exclusion_strategy_is_documented() {
    let workspace_root =
        must_some(Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()));

    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let cargo_toml_content = must(std::fs::read_to_string(&cargo_toml_path));

    // Verify the exclusion section has descriptive comments
    assert!(
        cargo_toml_content.contains("Legacy C parser") || cargo_toml_content.contains("cargo-fuzz"),
        "Exclusion entries should have descriptive comments"
    );
}
