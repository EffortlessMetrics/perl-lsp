//! Red TDD: Wave Final absorption tests for #4541.
//! Tests that perl-feature-catalog, perl-lsp-config, perl-content-length-framing,
//! and platform module are properly absorbed into perl-lsp-rs-core.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

/// Test 1: feature_catalog_module_accessible via perl_lsp_rs_core::feature_catalog::*
#[test]
fn test_feature_catalog_module_accessible() {
    // The feature_catalog module should be accessible from perl-lsp-rs-core
    // and expose the same API as the original crate.
    use perl_lsp_rs_core::feature_catalog;

    // Should have the Maturity enum
    let _ = feature_catalog::Maturity::Ga;
    let _ = feature_catalog::Maturity::Production;
    let _ = feature_catalog::Maturity::Experimental;
    let _ = feature_catalog::Maturity::Preview;
    let _ = feature_catalog::Maturity::Planned;

    // Should have key functions
    assert!(feature_catalog::DEFAULT_DAP_FEATURES.len() > 0);
}

/// Test 2: config_module_accessible via perl_lsp_rs_core::config::*
#[test]
fn test_config_module_accessible() {
    use perl_lsp_rs_core::config;

    // Should have ServerConfig struct
    let config = config::ServerConfig::default();
    assert!(config.inlay_hints_enabled);
    assert!(config.test_runner_enabled);
    assert_eq!(config.test_runner_command, "perl");

    // Should have WorkspaceConfig struct
    let ws_config = config::WorkspaceConfig::default();
    assert!(ws_config.include_paths.contains(&"lib".to_string()));
    assert!(!ws_config.use_system_inc);
}

/// Test 3: framing_module_accessible via perl_lsp_rs_core::transport::framing::*
#[test]
fn test_framing_module_accessible() {
    use perl_lsp_rs_core::transport::framing;

    // Should have ContentLengthFramer
    let framer = framing::ContentLengthFramer::new();
    assert_eq!(framer, framing::ContentLengthFramer::default());

    // Should have FramingError enum
    let _ = framing::FramingError::InvalidHeader;
    let _ = framing::FramingError::MissingContentLength;

    // Should have frame() function
    let body = b"test";
    let framed = framing::frame(body);
    assert!(framed.len() > body.len());
    assert!(String::from_utf8_lossy(&framed).contains("Content-Length:"));
}

/// Test 4: platform_module_accessible with resolve_perl_path_with_toolchain
#[test]
fn test_platform_module_with_resolve_perl_path() {
    use perl_lsp_rs_core::platform;

    // The three key platform functions should be present
    // (they're copied from perl-dap::platform to break the cycle)
    // resolve_perl_path_with_toolchain should be accessible
    let result = platform::resolve_perl_path_with_toolchain();
    // Result can be Ok or Err depending on the test environment
    let _ = result;
}

/// Test 5: perl-lsp-config has no cyclic dependency on perl-dap
#[test]
fn test_config_cargo_toml_has_no_dap_cycle() {
    let root = workspace_root();
    let config_toml_path = root.join("crates/perl-lsp-config/Cargo.toml");

    // After absorption, crates/perl-lsp-config no longer exists
    // So we just verify the rs-core config module doesn't import perl-dap
    let rs_core_config = root.join("crates/perl-lsp-rs-core/src/config.rs");
    if rs_core_config.exists() {
        let content =
            fs::read_to_string(&rs_core_config).expect("rs-core config.rs should be readable");
        assert!(
            !content.contains("perl_dap::"),
            "rs-core config.rs must not import from perl_dap (cycle break)"
        );
        assert!(
            content.contains("crate::platform"),
            "rs-core config.rs should use crate::platform for perl path resolution"
        );
    } else {
        // If config.rs doesn't exist, check that crates/perl-lsp-config is gone too
        assert!(
            !config_toml_path.exists(),
            "perl-lsp-config Cargo.toml should not exist if rs-core config.rs is missing"
        );
    }
}

/// Test 6: perl-feature-catalog not in publish allowlist of root Cargo.toml
#[test]
fn test_perl_feature_catalog_not_published() {
    let root = workspace_root();
    let root_toml =
        fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml should exist");

    // After absorption, perl-feature-catalog should not be in the publish allow list
    // Find the allow section and check
    let allow_section_start =
        root_toml.find("[workspace.metadata.publish]").unwrap_or(root_toml.len());
    let after_allow = &root_toml[allow_section_start..];

    assert!(
        !after_allow.contains("\"perl-feature-catalog\""),
        "perl-feature-catalog should be removed from publish allowlist after absorption"
    );
}

/// Test 7: perl-lsp-config not in publish allowlist of root Cargo.toml
#[test]
fn test_perl_lsp_config_not_published() {
    let root = workspace_root();
    let root_toml =
        fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml should exist");

    // After absorption, perl-lsp-config should not be in the publish allow list
    let allow_section_start =
        root_toml.find("[workspace.metadata.publish]").unwrap_or(root_toml.len());
    let after_allow = &root_toml[allow_section_start..];

    assert!(
        !after_allow.contains("\"perl-lsp-config\""),
        "perl-lsp-config should be removed from publish allowlist after absorption"
    );
}

/// Test 8: perl-content-length-framing not in publish allowlist of root Cargo.toml
#[test]
fn test_perl_content_length_framing_not_published() {
    let root = workspace_root();
    let root_toml =
        fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml should exist");

    // After absorption, perl-content-length-framing should not be in the publish allow list
    let allow_section_start =
        root_toml.find("[workspace.metadata.publish]").unwrap_or(root_toml.len());
    let after_allow = &root_toml[allow_section_start..];

    assert!(
        !after_allow.contains("\"perl-content-length-framing\""),
        "perl-content-length-framing should be removed from publish allowlist after absorption"
    );
}

/// Test 9: Old crate directories are deleted (perl-feature-catalog)
#[test]
fn test_perl_feature_catalog_dir_deleted() {
    let root = workspace_root();
    let path = root.join("crates/perl-feature-catalog");
    assert!(!path.exists(), "crates/perl-feature-catalog must be deleted after absorption");
}

/// Test 10: Old crate directories are deleted (perl-lsp-config)
#[test]
fn test_perl_lsp_config_dir_deleted() {
    let root = workspace_root();
    let path = root.join("crates/perl-lsp-config");
    assert!(!path.exists(), "crates/perl-lsp-config must be deleted after absorption");
}

/// Test 11: Old crate directories are deleted (perl-content-length-framing)
#[test]
fn test_perl_content_length_framing_dir_deleted() {
    let root = workspace_root();
    let path = root.join("crates/perl-content-length-framing");
    assert!(!path.exists(), "crates/perl-content-length-framing must be deleted after absorption");
}

/// Test 12: perl-lsp runtime uses rewritten config imports (zero perl_lsp_config:: refs)
#[test]
fn test_perl_lsp_runtime_rewired_config_imports() {
    let root = workspace_root();
    // Check that perl-lsp/src/runtime files have been rewired to use perl_lsp_rs_core::config
    // instead of perl_lsp_config::

    fn scan_dir(path: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let _ = scan_dir(&path); // recursive
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                let content = fs::read_to_string(&path).expect("should be able to read Rust file");

                assert!(
                    !content.contains("perl_lsp_config::"),
                    "File {} still contains perl_lsp_config:: imports after absorption",
                    path.display()
                );
            }
        }
        Ok(())
    }

    let runtime_dir = root.join("crates/perl-lsp/src/runtime");
    if runtime_dir.exists() {
        let _ = scan_dir(&runtime_dir);
    }
}

/// Test 13: perl-dap uses rewritten framing imports (zero perl_content_length_framing:: refs)
#[test]
fn test_perl_dap_rewired_framing_imports() {
    let root = workspace_root();
    // Check that perl-dap files have been rewired to use perl_lsp_rs_core::transport::framing
    // instead of perl_content_length_framing::

    fn scan_dir(path: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let _ = scan_dir(&path); // recursive
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                let content = fs::read_to_string(&path).expect("should be able to read Rust file");

                assert!(
                    !content.contains("perl_content_length_framing::"),
                    "File {} still contains perl_content_length_framing:: imports after absorption",
                    path.display()
                );
            }
        }
        Ok(())
    }

    let dap_src = root.join("crates/perl-dap/src");
    if dap_src.exists() {
        let _ = scan_dir(&dap_src);
    }
}

/// Test 14: G3 negative test g3_config_stays_standalone.rs is deleted
#[test]
fn test_g3_config_stays_standalone_deleted() {
    let root = workspace_root();
    let path = root.join("crates/perl-lsp-rs-core/tests/g3_config_stays_standalone.rs");
    assert!(
        !path.exists(),
        "g3_config_stays_standalone.rs must be deleted (superseded by absorption)"
    );
}

/// Test 15: G3 negative test g3_content_length_framing_stays.rs is deleted
#[test]
fn test_g3_content_length_framing_stays_deleted() {
    let root = workspace_root();
    let path = root.join("crates/perl-lsp-rs-core/tests/g3_content_length_framing_stays.rs");
    assert!(
        !path.exists(),
        "g3_content_length_framing_stays.rs must be deleted (superseded by absorption)"
    );
}

/// Test 16: Baseline updated to 31 published crates in xtask/published-crate-baseline.txt
#[test]
fn test_baseline_count_is_31() {
    let root = workspace_root();
    let baseline = fs::read_to_string(root.join("xtask/published-crate-baseline.txt"))
        .expect("xtask/published-crate-baseline.txt should exist");

    let count: usize = baseline.trim().parse().expect("baseline should contain a single number");

    assert_eq!(count, 31, "Published crate count should be 31 (was 34, minus 3 absorbed crates)");
}

/// Test 17: Amendment 9 marker present in ADR 0041
#[test]
fn test_adr_0041_has_amendment_9() {
    let root = workspace_root();
    let adr = fs::read_to_string(root.join("docs/adr/0041-microcrate-collapse.md"))
        .expect("ADR 0041 should exist");

    assert!(
        adr.contains("Amendment 9"),
        "ADR 0041 should contain 'Amendment 9' marker documenting Wave Final"
    );
}

/// Test 18: publish allowlist contains no more than 31 quoted crate entries
#[test]
fn test_publish_allowlist_has_31_entries() {
    let root = workspace_root();
    let root_toml =
        fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml should exist");

    // Extract [workspace.metadata.publish] section
    let Some(allow_section) = root_toml.split("[workspace.metadata.publish]").nth(1) else {
        panic!("[workspace.metadata.publish] section not found in root Cargo.toml");
    };

    // Count quoted entries that look like crate names (quoted strings in allow = [...])
    // Find the `allow = [` array
    let allow_start = allow_section.find("allow = [").unwrap_or(0);
    let allow_content = &allow_section[allow_start..];
    let allow_end = allow_content.find(']').unwrap_or(allow_content.len());
    let allow_array = &allow_content[..allow_end];

    // Count quoted entries (crate names are double-quoted strings)
    let count = allow_array.matches('"').count() / 2;

    assert_eq!(
        count, 31,
        "Published allowlist should contain exactly 31 crate entries (got {})",
        count
    );
}
