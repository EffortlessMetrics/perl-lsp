//! Edge case and regression tests for perl-workspace collapse (Wave A #4426).
//!
//! These tests verify boundary conditions, error paths, and regression scenarios
//! that supplement the red-TDD integration tests. They exercise:
//! - Backward-compat path resolution under all API styles
//! - Module re-export completeness and naming stability
//! - Dual-path resolution for workspace vs workspace::* paths
//! - Consumer crate compatibility after rename
//! - Publish-closure gate verification

use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

// =============================================================================
// Test 1: Backward-compat path resolution — workspace::monitoring
// =============================================================================

/// Verify that callers using `perl_workspace::workspace::monitoring::*` paths
/// (old-style) still resolve correctly. This is a regression guard for
/// refactoring that might drop the backward-compat module.
#[test]
fn test_backward_compat_workspace_monitoring_path_complete() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    // Check that workspace/monitoring.rs re-exports from the main monitoring module
    let workspace_monitoring = crate_dir.join("src/workspace/monitoring.rs");
    assert!(
        workspace_monitoring.exists(),
        "workspace/monitoring.rs must exist for backward-compat path perl_workspace::workspace::monitoring::"
    );

    let content = std::fs::read_to_string(&workspace_monitoring)
        .expect("Failed to read workspace/monitoring.rs");

    // Should have pub use statements re-exporting from crate::monitoring
    assert!(
        content.contains("pub use crate::monitoring"),
        "workspace/monitoring.rs must re-export from crate::monitoring module"
    );

    // Should re-export at least some key types
    let expected_re_exports = vec!["IndexPhase", "IndexMetrics", "IndexResourceLimits"];

    for export in expected_re_exports {
        assert!(
            content.contains(export),
            "workspace/monitoring.rs should re-export type '{}' from crate::monitoring",
            export
        );
    }
}

// =============================================================================
// Test 2: Backward-compat modules in workspace/ exist (optional per spec)
// =============================================================================

/// Verify backward-compat modules in workspace/ directory exist if they're present.
/// The spec allows but doesn't require these for backward-compat as long as
/// lib.rs declares pub mod workspace and workspace/mod.rs is accessible.
#[test]
fn test_backward_compat_modules_if_present_have_reexports() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let backward_compat_modules = vec![
        ("workspace/monitoring.rs", "monitoring"),
        ("workspace/slo.rs", "slo"),
        ("workspace/state_machine.rs", "state_machine"),
        ("workspace/discovery.rs", "discovery"),
        ("workspace/folder.rs", "folder"),
        ("workspace/ignore.rs", "ignore"),
    ];

    // If any backward-compat module exists, verify it re-exports from main module
    for (module_path, module_name) in backward_compat_modules {
        let full_path = crate_dir.join("src").join(module_path);
        if full_path.exists() {
            let content = std::fs::read_to_string(&full_path)
                .expect(&format!("Failed to read {}", module_path));

            assert!(
                content.contains(&format!("pub use crate::{}", module_name))
                    || content.contains(&format!("pub use crate::{}::", module_name)),
                "If {} exists, it should re-export from crate::{}",
                module_path,
                module_name
            );
        }
    }
}

// =============================================================================
// Test 3: lib.rs declares all 6 new modules as pub mod
// =============================================================================

/// Verify that lib.rs declares all 6 new modules (discovery, folder, ignore,
/// monitoring, slo, state_machine) without duplicates.
#[test]
fn test_lib_rs_module_declarations_unique() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let lib_file = crate_dir.join("src/lib.rs");
    let content = std::fs::read_to_string(&lib_file).expect("Failed to read lib.rs");

    let modules = vec!["discovery", "folder", "ignore", "monitoring", "slo", "state_machine"];

    for module_name in &modules {
        let pub_mod_pattern = format!("pub mod {};", module_name);
        let count = content.matches(&pub_mod_pattern).count();

        assert_eq!(
            count, 1,
            "lib.rs should declare '{}' exactly once with 'pub mod', found {}",
            module_name, count
        );
    }
}

// =============================================================================
// Test 4: Workspace/mod.rs declares backward-compat modules
// =============================================================================

/// Verify that workspace/mod.rs declares the backward-compat modules
/// (monitoring, slo, state_machine, etc.) so they're accessible via pub mod.
#[test]
fn test_workspace_mod_rs_declares_backward_compat_modules() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let workspace_mod = crate_dir.join("src/workspace/mod.rs");
    let content = std::fs::read_to_string(&workspace_mod).expect("Failed to read workspace/mod.rs");

    // Should declare at least monitoring, slo, state_machine as pub mod
    // These are the backward-compat modules in workspace/
    let expected_pub_mods = vec!["monitoring", "slo", "state_machine"];

    for module_name in expected_pub_mods {
        let pub_mod_pattern = format!("pub mod {}", module_name);
        assert!(
            content.contains(&pub_mod_pattern),
            "workspace/mod.rs should declare 'pub mod {}' for backward compatibility",
            module_name
        );
    }
}

// =============================================================================
// Test 5: No cyclic imports in backward-compat modules (if present)
// =============================================================================

/// Verify that backward-compat modules don't create cyclic imports by importing
/// from crate::workspace::* when they should import directly from crate::.
#[test]
fn test_no_cyclic_reexports_in_backward_compat_modules() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let backward_compat_files = vec![
        "src/workspace/monitoring.rs",
        "src/workspace/slo.rs",
        "src/workspace/state_machine.rs",
        "src/workspace/discovery.rs",
        "src/workspace/folder.rs",
        "src/workspace/ignore.rs",
    ];

    for file_path in backward_compat_files {
        let full_path = crate_dir.join(file_path);
        if full_path.exists() {
            let content = std::fs::read_to_string(&full_path)
                .expect(&format!("Failed to read {}", file_path));

            // If the file exists, it should NOT create cycles
            // by importing from crate::workspace::* when it's in workspace/
            assert!(
                !content.contains("use crate::workspace::"),
                "File {} should not import from crate::workspace::* (would create cycle)",
                file_path
            );
        }
    }
}

// =============================================================================
// Test 6: api.rs lists expected public surface explicitly
// =============================================================================

/// Verify that api.rs (if it exists) contains explicit pub use statements
/// and does NOT use wildcards for observability modules (monitoring, slo, state_machine).
#[test]
fn test_api_rs_explicit_reexports_no_wildcards() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let api_file = crate_dir.join("src/api.rs");

    if api_file.exists() {
        let content = std::fs::read_to_string(&api_file).expect("Failed to read api.rs");

        // api.rs should NOT have wildcards for these modules
        let forbidden_wildcards =
            vec!["pub use monitoring::*", "pub use slo::*", "pub use state_machine::*"];

        for wildcard in forbidden_wildcards {
            assert!(
                !content.contains(wildcard),
                "api.rs should not have wildcard '{}' — be explicit about public types",
                wildcard
            );
        }

        // If the file has any pub use statements, they should be reasonably specific
        if content.contains("pub use") {
            // Rough check: if there are pub use statements, they should have semicolons
            // and not all be on one mega-line
            let line_count = content.lines().count();
            assert!(
                line_count > 3,
                "api.rs should have multiple lines if it has pub use statements"
            );
        }
    }
}

// =============================================================================
// Test 7: Cargo.toml package name is perl-workspace
// =============================================================================

/// Verify the package name in the crate's Cargo.toml is 'perl-workspace',
/// not 'perl-workspace-index' (the directory name) or old satellite names.
#[test]
fn test_cargo_toml_package_name_is_perl_workspace() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

    // The package name line should contain perl-workspace
    // and NOT contain perl-workspace-index
    let has_correct_name = content.contains("name = \"perl-workspace\"");
    let has_old_name = content.contains("name = \"perl-workspace-index\"");

    assert!(has_correct_name, "Cargo.toml [package] name should be 'perl-workspace'");

    assert!(!has_old_name, "Cargo.toml [package] name should NOT be 'perl-workspace-index'");
}

// =============================================================================
// Test 8: No dependencies on deleted satellite crates
// =============================================================================

/// Verify that Cargo.toml does NOT list any of the 6 deleted satellite crates
/// as dependencies.
#[test]
fn test_cargo_toml_no_deleted_satellite_deps() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

    let deleted_crates = vec![
        "perl-workspace-discovery",
        "perl-workspace-folder",
        "perl-workspace-ignore",
        "perl-workspace-index-monitoring",
        "perl-workspace-index-slo",
        "perl-workspace-index-state-machine",
    ];

    for crate_name in deleted_crates {
        assert!(
            !content.contains(&format!("\"{}\"", crate_name)),
            "Cargo.toml should not depend on deleted satellite crate {}",
            crate_name
        );
    }
}

// =============================================================================
// Test 9: Workspace members align with on-disk crate manifests
// =============================================================================

/// Verify workspace members listed in root Cargo.toml align with crates on disk.
///
/// This catches accidental drift (e.g. removing a member entry but leaving the crate,
/// or adding a crate directory without wiring it into the workspace), while avoiding
/// brittle hard-coded member count thresholds as the workspace evolves.
#[test]
fn test_workspace_members_match_crates_directory() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(&cargo_manifest_dir);
    let workspace_root = manifest_path.parent().unwrap().parent().unwrap();

    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).expect("Failed to read workspace Cargo.toml");

    // Count unique "crates/" entries (each workspace member)
    let members: HashSet<&str> = content
        .lines()
        .filter(|line| line.contains("\"crates/") && !line.contains("."))
        .filter_map(|line| {
            let start = line.find("crates/")?;
            let end = line[start..].find('"')? + start;
            Some(&line[start..end])
        })
        .collect();

    // Parse current workspace members from the manifest.
    assert!(
        !members.is_empty(),
        "Workspace members list must not be empty"
    );

    // Discover crate directories that contain a Cargo.toml.
    let crates_dir = workspace_root.join("crates");
    let crate_dirs: HashSet<String> = std::fs::read_dir(&crates_dir)
        .expect("Failed to read crates directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("Cargo.toml").exists())
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| format!("crates/{name}"))
        })
        .collect();

    assert_eq!(
        members.len(),
        crate_dirs.len(),
        "Workspace member count should match on-disk crate count"
    );

    for crate_dir in &crate_dirs {
        assert!(
            members.contains(crate_dir.as_str()),
            "Workspace members should include {crate_dir}"
        );
    }
}

// =============================================================================
// Test 10: Publish allowlist excludes old satellite crates
// =============================================================================

/// Verify publish allowlist does NOT include old satellite crates.
/// The baseline (before collapse) was 120, so target is 114 after removing 6.
#[test]
fn test_publish_allowlist_excludes_old_satellites() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(&cargo_manifest_dir);
    let workspace_root = manifest_path.parent().unwrap().parent().unwrap();

    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).expect("Failed to read workspace Cargo.toml");

    // Extract the [workspace.metadata.publish] allowlist
    let allowlist_section = content.split("[workspace.metadata.publish]").nth(1).unwrap_or("");

    // Verify old satellite crates are NOT in allowlist
    // Note: perl-workspace-discovery is kept as a separate Tier 6 crate (Wave E refactor)
    let old_satellites = vec![
        "perl-workspace-folder",
        "perl-workspace-ignore",
        "perl-workspace-index-monitoring",
        "perl-workspace-index-slo",
        "perl-workspace-index-state-machine",
    ];

    for satellite in old_satellites {
        assert!(
            !allowlist_section.contains(&format!("\"{}\"", satellite)),
            "Publish allowlist should not contain old satellite '{}'",
            satellite
        );
    }

    // Verify new name IS in allowlist
    assert!(
        allowlist_section.contains("\"perl-workspace\""),
        "Publish allowlist should contain new crate name 'perl-workspace'"
    );
}

// =============================================================================
// Test 11: Hardcoded string updates in perl-ci-hygiene
// =============================================================================

/// Verify that hardcoded crate name references in perl-ci-hygiene have been
/// updated to use 'perl-workspace' instead of 'perl-workspace-index' or satellite names.
#[test]
fn test_perl_ci_hygiene_crate_names_updated() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(&cargo_manifest_dir);
    let workspace_root = manifest_path.parent().unwrap().parent().unwrap();

    let hygiene_main = workspace_root.join("crates/perl-ci-hygiene/src/main.rs");
    if hygiene_main.exists() {
        let content =
            std::fs::read_to_string(&hygiene_main).expect("Failed to read perl-ci-hygiene main.rs");

        // Should not reference old crate names (except perl-workspace-index as directory name)
        // Note: perl-workspace-discovery is a valid Tier 6 crate (Wave E), so it's allowed
        let old_names = vec![
            "perl-workspace-folder",
            "perl-workspace-ignore",
            "perl-workspace-index-monitoring",
            "perl-workspace-index-slo",
            "perl-workspace-index-state-machine",
        ];

        for old_name in old_names {
            assert!(
                !content.contains(&format!("\"{}\"", old_name)),
                "perl-ci-hygiene should not reference old satellite crate name '{}'",
                old_name
            );
        }

        // Should reference the new name
        assert!(
            content.contains("\"perl-workspace\""),
            "perl-ci-hygiene should reference new crate name 'perl-workspace'"
        );
    }
}

// =============================================================================
// Test 12: Hardcoded string updates in perl-parser tests
// =============================================================================

/// Verify that test files in perl-parser that reference workspace crate names
/// have been updated (lines 607-608 mentioned in builder notes).
#[test]
fn test_perl_parser_test_references_updated() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(&cargo_manifest_dir);
    let workspace_root = manifest_path.parent().unwrap().parent().unwrap();

    let missing_docs_test =
        workspace_root.join("crates/perl-parser/tests/missing_docs_ac_tests.rs");
    if missing_docs_test.exists() {
        let content = std::fs::read_to_string(&missing_docs_test)
            .expect("Failed to read missing_docs_ac_tests.rs");

        // Should not have old satellite crate references in crate list
        let old_names = vec![
            "perl-workspace-discovery",
            "perl-workspace-folder",
            "perl-workspace-ignore",
            "perl-workspace-index-monitoring",
            "perl-workspace-index-slo",
            "perl-workspace-index-state-machine",
        ];

        for old_name in old_names {
            assert!(
                !content.contains(&format!("\"{}\"", old_name)),
                "perl-parser tests should not reference old crate name '{}'",
                old_name
            );
        }
    }
}

// =============================================================================
// Test 13: All consumer crates updated to use perl_workspace
// =============================================================================

/// Verify that all 8 consumer crates import from perl_workspace, not old names.
#[test]
fn test_consumer_crates_import_from_perl_workspace() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(&cargo_manifest_dir);
    let workspace_root = manifest_path.parent().unwrap().parent().unwrap();

    let consumers = vec![
        "perl-dead-code",
        "perl-lsp",
        "perl-lsp-completion",
        "perl-lsp-diagnostics",
        "perl-module",
        "perl-parser",
        "perl-refactoring",
        "perl-semantic-analyzer",
    ];

    for consumer in consumers {
        let lib_rs = workspace_root.join(format!("crates/{}/src/lib.rs", consumer));
        if lib_rs.exists() {
            let content = std::fs::read_to_string(&lib_rs)
                .expect(&format!("Failed to read {}/src/lib.rs", consumer));

            // Check if the file imports from workspace at all
            if content.contains("workspace") {
                // If it does, verify it uses the new path
                // Should be: use perl_workspace:: (with new package name)
                // NOT: use perl_workspace_index:: or use perl_workspace_*::

                let forbidden_patterns = vec![
                    "use perl_workspace_index::",
                    "use perl_workspace_discovery::",
                    "use perl_workspace_folder::",
                    "use perl_workspace_ignore::",
                    "use perl_workspace_index_monitoring::",
                    "use perl_workspace_index_slo::",
                    "use perl_workspace_index_state_machine::",
                ];

                for forbidden in forbidden_patterns {
                    assert!(
                        !content.contains(forbidden),
                        "Consumer crate {} should not use old import pattern '{}'",
                        consumer,
                        forbidden
                    );
                }
            }
        }
    }
}

// =============================================================================
// Test 14: New modules directory structure is complete
// =============================================================================

/// Verify that all 6 new fold-modules have mod.rs and are non-empty.
#[test]
fn test_new_modules_have_complete_structure() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let modules = vec![
        ("discovery", "Directory workspace discovery functionality"),
        ("folder", "Folder enumeration and path handling"),
        ("ignore", "Ignore pattern matching for workspace files"),
        ("monitoring", "Performance and resource monitoring"),
        ("slo", "Service level objective tracking"),
        ("state_machine", "Index lifecycle state management"),
    ];

    for (module_name, _description) in modules {
        let mod_file = crate_dir.join(format!("src/{}/mod.rs", module_name));
        assert!(mod_file.exists(), "Module {} should have src/{}/mod.rs", module_name, module_name);

        let content = std::fs::read_to_string(&mod_file)
            .expect(&format!("Failed to read src/{}/mod.rs", module_name));

        // File should not be empty
        assert!(
            !content.trim().is_empty(),
            "Module {} src/{}/mod.rs should not be empty",
            module_name,
            module_name
        );

        // Should have at least a module docstring or pub items
        let has_docs = content.contains("//!");
        let has_pub_items = content.contains("pub ");
        assert!(
            has_docs || has_pub_items,
            "Module {} should have documentation or public items",
            module_name
        );
    }
}

// =============================================================================
// Test 15: No duplicate module declarations
// =============================================================================

/// Verify lib.rs doesn't declare the same module twice (common refactoring error).
#[test]
fn test_no_duplicate_module_declarations_in_lib_rs() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir =
        PathBuf::from(&cargo_manifest_dir).parent().unwrap().join("perl-workspace-index");

    let lib_file = crate_dir.join("src/lib.rs");
    let content = std::fs::read_to_string(&lib_file).expect("Failed to read lib.rs");

    let modules =
        vec!["discovery", "folder", "ignore", "monitoring", "slo", "state_machine", "workspace"];

    for module_name in modules {
        let pub_mod_decl = format!("pub mod {}", module_name);
        let count = content.matches(&pub_mod_decl).count();

        assert!(
            count <= 1,
            "lib.rs should declare module '{}' at most once, found {} declarations",
            module_name,
            count
        );
    }
}
