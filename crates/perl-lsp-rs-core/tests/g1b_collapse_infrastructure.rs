//! Infrastructure and migration tests for Wave G1b provider collapse.
//!
//! These tests verify:
//! 1. The 10 G1b crate directories are deleted
//! 2. perl-lsp/Cargo.toml no longer depends on them
//! 3. perl-lsp/src has been migrated to use perl_lsp_rs_core::providers::*
//! 4. published-crate-baseline.txt is updated from 59 to 49
//! 5. Snapshot files have been migrated to the correct location

#![allow(clippy::expect_used)]

use std::fs;

// Helper to get the workspace root
fn get_workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let p = std::path::PathBuf::from(manifest_dir);
    // CARGO_MANIFEST_DIR = .../crates/perl-lsp-rs-core
    // parent = .../crates
    // parent.parent = workspace root
    let parent = p.parent().unwrap_or(&p);
    parent.parent().unwrap_or(parent).to_path_buf()
}

// ============================================================================
// CRATE DELETION TESTS
// ============================================================================

/// Test that perl-lsp-rename crate directory is deleted.
#[test]
fn test_crate_perl_lsp_rename_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-rename");
    assert!(!path.exists(), "perl-lsp-rename should be deleted, but {} exists", path.display());
}

/// Test that perl-lsp-diagnostics crate directory is deleted.
#[test]
fn test_crate_perl_lsp_diagnostics_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-diagnostics");
    assert!(
        !path.exists(),
        "perl-lsp-diagnostics should be deleted, but {} exists",
        path.display()
    );
}

/// Test that perl-lsp-inline-completion crate directory is deleted.
#[test]
fn test_crate_perl_lsp_inline_completion_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-inline-completion");
    assert!(
        !path.exists(),
        "perl-lsp-inline-completion should be deleted, but {} exists",
        path.display()
    );
}

/// Test that perl-lsp-semantic-tokens crate directory is deleted.
#[test]
fn test_crate_perl_lsp_semantic_tokens_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-semantic-tokens");
    assert!(
        !path.exists(),
        "perl-lsp-semantic-tokens should be deleted, but {} exists",
        path.display()
    );
}

/// Test that perl-lsp-formatting crate directory is deleted.
#[test]
fn test_crate_perl_lsp_formatting_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-formatting");
    assert!(!path.exists(), "perl-lsp-formatting should be deleted, but {} exists", path.display());
}

/// Test that perl-lsp-ai-provider crate directory is deleted.
#[test]
fn test_crate_perl_lsp_ai_provider_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-ai-provider");
    assert!(
        !path.exists(),
        "perl-lsp-ai-provider should be deleted, but {} exists",
        path.display()
    );
}

/// Test that perl-lsp-completion crate directory is deleted.
#[test]
fn test_crate_perl_lsp_completion_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-completion");
    assert!(!path.exists(), "perl-lsp-completion should be deleted, but {} exists", path.display());
}

/// Test that perl-lsp-navigation crate directory is deleted.
#[test]
fn test_crate_perl_lsp_navigation_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-navigation");
    assert!(!path.exists(), "perl-lsp-navigation should be deleted, but {} exists", path.display());
}

/// Test that perl-lsp-code-actions crate directory is deleted.
#[test]
fn test_crate_perl_lsp_code_actions_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-code-actions");
    assert!(
        !path.exists(),
        "perl-lsp-code-actions should be deleted, but {} exists",
        path.display()
    );
}

/// Test that perl-lsp-providers crate directory is deleted.
#[test]
fn test_crate_perl_lsp_providers_deleted() {
    let root = get_workspace_root();
    let path = root.join("crates/perl-lsp-providers");
    assert!(!path.exists(), "perl-lsp-providers should be deleted, but {} exists", path.display());
}

// ============================================================================
// PUBLISHED CRATE BASELINE TEST
// ============================================================================

/// Test that xtask/published-crate-baseline.txt is updated from 59 to 49.
#[test]
fn test_published_crate_baseline_updated() {
    let root = get_workspace_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");
    assert!(baseline_path.exists(), "xtask/published-crate-baseline.txt should exist");

    let content = fs::read_to_string(baseline_path)
        .expect("Failed to read xtask/published-crate-baseline.txt");
    let trimmed = content.trim();

    // After G2: 49 - 5 absorbed crates = 44 (transport deferred to G3)
    // History: G1b → 49 (59 - 10); G2 → 44 (49 - 5)
    assert_eq!(
        trimmed, "44",
        "published-crate-baseline.txt should be '44' (post-G2), but found '{}'",
        trimmed
    );
}

// ============================================================================
// PERL-LSP CARGO.TOML DEPENDENCY TESTS
// ============================================================================

/// Test that perl-lsp/Cargo.toml no longer depends on any G1b crates.
#[test]
fn test_perl_lsp_cargo_toml_no_g1b_deps() {
    let root = get_workspace_root();
    let cargo_path = root.join("crates/perl-lsp/Cargo.toml");
    assert!(cargo_path.exists(), "crates/perl-lsp/Cargo.toml should exist");

    let content =
        fs::read_to_string(cargo_path).expect("Failed to read crates/perl-lsp/Cargo.toml");

    // These 10 dependencies should be removed after G1b collapse
    let forbidden_deps = [
        "perl-lsp-providers",
        "perl-lsp-formatting",
        "perl-lsp-code-actions",
        "perl-lsp-inline-completion",
        "perl-lsp-ai-provider",
        "perl-lsp-completion",
        "perl-lsp-diagnostics",
        "perl-lsp-navigation",
        "perl-lsp-rename",
        "perl-lsp-semantic-tokens",
    ];

    for dep in forbidden_deps.iter() {
        // Check for the pattern "perl-lsp-X = {"
        let pattern = format!("{} = {{", dep);
        assert!(
            !content.contains(&pattern),
            "crates/perl-lsp/Cargo.toml should not contain '{}' after G1b collapse, but found it",
            pattern
        );
    }
}

/// Test that perl-lsp/Cargo.toml still depends on perl-lsp-rs-core.
#[test]
fn test_perl_lsp_cargo_toml_has_core_dep() {
    let root = get_workspace_root();
    let cargo_path = root.join("crates/perl-lsp/Cargo.toml");
    assert!(cargo_path.exists(), "crates/perl-lsp/Cargo.toml should exist");

    let content =
        fs::read_to_string(cargo_path).expect("Failed to read crates/perl-lsp/Cargo.toml");

    // perl-lsp-rs-core must remain
    assert!(
        content.contains("perl-lsp-rs-core"),
        "crates/perl-lsp/Cargo.toml must contain perl-lsp-rs-core dependency after G1b collapse"
    );
}

// ============================================================================
// PERL-LSP SOURCE MIGRATION TESTS
// ============================================================================

/// Test that perl-lsp/src/features/rename.rs doesn't import from old perl_lsp_rename crate.
#[test]
fn test_perl_lsp_src_features_rename_migrated() {
    let root = get_workspace_root();
    let file = root.join("crates/perl-lsp/src/features/rename.rs");
    if file.exists() {
        let content = fs::read_to_string(file).expect("Failed to read features/rename.rs");

        // Should use new path
        assert!(
            content.contains("perl_lsp_rs_core::providers::rename"),
            "features/rename.rs should use perl_lsp_rs_core::providers::rename after migration"
        );

        // Should not use old path
        assert!(
            !content.contains("use perl_lsp_rename::"),
            "features/rename.rs should not use perl_lsp_rename:: after migration"
        );
    }
}

/// Test that perl-lsp/src/features/diagnostics doesn't import from old perl_lsp_diagnostics crate.
#[test]
fn test_perl_lsp_src_features_diagnostics_migrated() {
    let root = get_workspace_root();
    let file = root.join("crates/perl-lsp/src/features/diagnostics/mod.rs");
    if file.exists() {
        let content = fs::read_to_string(file).expect("Failed to read features/diagnostics/mod.rs");

        // Should use new path
        assert!(
            content.contains("perl_lsp_rs_core::providers::diagnostics"),
            "features/diagnostics/mod.rs should use perl_lsp_rs_core::providers::diagnostics after migration"
        );

        // Should not use old path
        assert!(
            !content.contains("use perl_lsp_diagnostics::"),
            "features/diagnostics/mod.rs should not use perl_lsp_diagnostics:: after migration"
        );
    }
}

// ============================================================================
// DIAGNOSTIC SNAPSHOT MIGRATION TESTS
// ============================================================================

/// Test that 4 diagnostics snapshots are migrated to perl-lsp-rs-core/tests/snapshots/.
#[test]
fn test_diagnostics_snapshots_migrated_missing_pragmas() {
    let root = get_workspace_root();
    let snap_path = root.join("crates/perl-lsp-rs-core/tests/snapshots/diag_snap__missing_pragmas_and_unused_variable.snap");
    assert!(
        snap_path.exists(),
        "Diagnostics snapshot {} should exist after migration",
        snap_path.display()
    );
}

/// Test that diagnostics snapshot for package_module is migrated.
#[test]
fn test_diagnostics_snapshots_migrated_package_module() {
    let root = get_workspace_root();
    let snap_path = root
        .join("crates/perl-lsp-rs-core/tests/snapshots/diag_snap__package_module_happy_path.snap");
    assert!(
        snap_path.exists(),
        "Diagnostics snapshot {} should exist after migration",
        snap_path.display()
    );
}

/// Test that diagnostics snapshot for script is migrated.
#[test]
fn test_diagnostics_snapshots_migrated_script() {
    let root = get_workspace_root();
    let snap_path =
        root.join("crates/perl-lsp-rs-core/tests/snapshots/diag_snap__script_happy_path.snap");
    assert!(
        snap_path.exists(),
        "Diagnostics snapshot {} should exist after migration",
        snap_path.display()
    );
}

/// Test that diagnostics snapshot for security is migrated.
#[test]
fn test_diagnostics_snapshots_migrated_security() {
    let root = get_workspace_root();
    let snap_path =
        root.join("crates/perl-lsp-rs-core/tests/snapshots/diag_snap__security_string_eval.snap");
    assert!(
        snap_path.exists(),
        "Diagnostics snapshot {} should exist after migration",
        snap_path.display()
    );
}

/// Test that old diagnostics snapshots in perl-lsp-diagnostics are deleted.
#[test]
fn test_old_diagnostics_snapshots_deleted() {
    let root = get_workspace_root();
    let old_snap_path = root.join("crates/perl-lsp-diagnostics");
    assert!(
        !old_snap_path.exists(),
        "crates/perl-lsp-diagnostics should be deleted, so its snapshots should be gone"
    );
}

// ============================================================================
// DIAGNOSTIC TEST FILE MIGRATION TEST
// ============================================================================

/// Test that diag_snap.rs test file is migrated to perl-lsp-rs-core/tests/.
#[test]
fn test_diag_snap_test_file_migrated() {
    let root = get_workspace_root();
    let test_file = root.join("crates/perl-lsp-rs-core/tests/diag_snap.rs");
    assert!(
        test_file.exists(),
        "Diagnostics test file {} should exist after migration",
        test_file.display()
    );
}

/// Test that the migrated diag_snap.rs uses the new provider path.
#[test]
fn test_diag_snap_uses_new_provider_path() {
    let root = get_workspace_root();
    let test_file = root.join("crates/perl-lsp-rs-core/tests/diag_snap.rs");
    if test_file.exists() {
        let content = fs::read_to_string(test_file).expect("Failed to read diag_snap.rs");

        // Should have the new path
        assert!(
            content.contains("perl_lsp_rs_core::providers::diagnostics::"),
            "diag_snap.rs should use perl_lsp_rs_core::providers::diagnostics:: after migration"
        );

        // Should not have the old path
        assert!(
            !content.contains("perl_lsp_diagnostics::"),
            "diag_snap.rs should not use perl_lsp_diagnostics:: after migration"
        );
    }
}
