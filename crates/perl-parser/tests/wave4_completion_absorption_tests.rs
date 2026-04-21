//! Tests for Wave 4 absorption: perl-dead-code, perl-refactoring, perl-incremental-parsing
//!
//! These tests verify that the three satellite crates have been properly absorbed
//! into perl-parser as internal modules, with correct visibility and configuration.

use std::fs;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Get workspace root from CARGO_MANIFEST_DIR (crates/perl-parser → workspace root).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Join a workspace-relative path to the workspace root.
fn ws(path: &str) -> PathBuf {
    workspace_root().join(path)
}

// =============================================================================
// Section 1: Module Accessibility Tests
// =============================================================================

/// Test that DeadCodeDetector and related types are accessible via perl_parser::dead_code
#[test]
fn test_dead_code_module_accessible() -> TestResult {
    // After absorption, perl_parser::dead_code should expose the main types
    // This test compiles if the module exists and has the right items
    let _type_ref: Option<perl_parser::dead_code::DeadCodeType> = None;
    let _detector_type: Option<perl_parser::dead_code::DeadCodeDetector> = None;
    let _analysis_type: Option<perl_parser::dead_code::DeadCodeAnalysis> = None;
    Ok(())
}

/// Test that the dead_code_detector compatibility alias still works
#[test]
fn test_dead_code_detector_compat_alias() -> TestResult {
    // Backwards compatibility: perl_parser::dead_code_detector should still be usable
    let _compat_alias: Option<perl_parser::dead_code_detector::DeadCodeDetector> = None;
    Ok(())
}

/// Test that refactor submodules are accessible via perl_parser::refactor
#[test]
fn test_refactor_module_accessible() -> TestResult {
    // After absorption, perl_parser::refactor should have submodules like import_optimizer
    let _import_opt: Option<perl_parser::refactor::import_optimizer::ImportOptimizer> = None;
    Ok(())
}

/// Test that refactoring engine is accessible
#[test]
fn test_refactoring_engine_accessible() -> TestResult {
    // perl_parser::refactor::refactoring should contain the unified engine
    // This verifies the module path is correct post-absorption
    let _engine: Option<perl_parser::refactor::refactoring::RefactoringEngine> = None;
    Ok(())
}

#[cfg(feature = "incremental")]
/// Test that incremental parsing module is accessible via perl_parser::incremental
#[test]
fn test_incremental_module_accessible() -> TestResult {
    // After absorption, perl_parser::incremental should expose IncrementalState and friends
    let _state_type: Option<perl_parser::incremental::IncrementalState> = None;
    let _edit_type: Option<perl_parser::incremental::Edit> = None;
    let _checkpoint_type: Option<perl_parser::incremental::LexCheckpoint> = None;
    Ok(())
}

#[cfg(feature = "incremental")]
/// Test that incremental submodules are accessible
#[test]
fn test_incremental_submodules_accessible() -> TestResult {
    // Verify that submodules like incremental_document are accessible
    let _doc_type: Option<perl_parser::incremental::incremental_document::IncrementalDocument> =
        None;
    Ok(())
}

// =============================================================================
// Section 2: Cargo.toml Publish Flag Tests
// =============================================================================

/// Test that perl-dead-code has publish = false set
#[test]
fn test_perl_dead_code_publish_false() -> TestResult {
    let cargo_toml_path = ws("crates/perl-dead-code/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    // After absorption, this file should be marked as not publishable
    if content.contains("publish = false") {
        Ok(())
    } else {
        Err("perl-dead-code/Cargo.toml must have publish = false".into())
    }
}

/// Test that perl-refactoring has publish = false set
#[test]
fn test_perl_refactoring_publish_false() -> TestResult {
    let cargo_toml_path = ws("crates/perl-refactoring/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    // After absorption, this file should be marked as not publishable
    if content.contains("publish = false") {
        Ok(())
    } else {
        Err("perl-refactoring/Cargo.toml must have publish = false".into())
    }
}

/// Test that perl-incremental-parsing has publish = false set
#[test]
fn test_perl_incremental_parsing_publish_false() -> TestResult {
    let cargo_toml_path = ws("crates/perl-incremental-parsing/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    // After absorption, this file should be marked as not publishable
    if content.contains("publish = false") {
        Ok(())
    } else {
        Err("perl-incremental-parsing/Cargo.toml must have publish = false".into())
    }
}

// =============================================================================
// Section 3: Allowlist Verification Tests
// =============================================================================

/// Test that perl-dead-code is NOT in the workspace publish allowlist
#[test]
fn test_perl_dead_code_not_in_allowlist() -> TestResult {
    let root_cargo_toml = fs::read_to_string(ws("Cargo.toml"))?;

    // Find the [workspace.metadata.publish.allow] section
    if let Some(allow_section) = root_cargo_toml.split("[workspace.metadata.publish.allow]").nth(1)
    {
        // Get just the content until the next section
        let allow_entries = allow_section.split('[').next().unwrap_or("");

        if allow_entries.contains("perl-dead-code") {
            return Err("perl-dead-code must be removed from allowlist after absorption".into());
        }
    }

    Ok(())
}

/// Test that perl-refactoring is NOT in the workspace publish allowlist
#[test]
fn test_perl_refactoring_not_in_allowlist() -> TestResult {
    let root_cargo_toml = fs::read_to_string(ws("Cargo.toml"))?;

    if let Some(allow_section) = root_cargo_toml.split("[workspace.metadata.publish.allow]").nth(1)
    {
        let allow_entries = allow_section.split('[').next().unwrap_or("");

        if allow_entries.contains("perl-refactoring") {
            return Err("perl-refactoring must be removed from allowlist after absorption".into());
        }
    }

    Ok(())
}

/// Test that perl-incremental-parsing is NOT in the workspace publish allowlist
#[test]
fn test_perl_incremental_parsing_not_in_allowlist() -> TestResult {
    let root_cargo_toml = fs::read_to_string(ws("Cargo.toml"))?;

    if let Some(allow_section) = root_cargo_toml.split("[workspace.metadata.publish.allow]").nth(1)
    {
        let allow_entries = allow_section.split('[').next().unwrap_or("");

        if allow_entries.contains("perl-incremental-parsing") {
            return Err(
                "perl-incremental-parsing must be removed from allowlist after absorption".into()
            );
        }
    }

    Ok(())
}

// =============================================================================
// Section 4: Published Count Baseline Tests
// =============================================================================

/// Test that published-crate-baseline.txt is updated to 34 (down from 37)
#[test]
fn test_published_count_baseline_is_34() -> TestResult {
    let baseline_path = ws("xtask/published-crate-baseline.txt");
    let content = fs::read_to_string(&baseline_path)?;
    let baseline_count = content.trim().parse::<u32>()?;

    if baseline_count == 34 {
        Ok(())
    } else {
        Err(format!("published-crate-baseline.txt must be 34, got {}", baseline_count).into())
    }
}

// =============================================================================
// Section 5: Import Rewiring Tests
// =============================================================================

/// Test that text_sync.rs has NO perl_incremental_parsing:: references
/// (all should be rewritten to perl_parser::incremental::)
#[test]
fn test_text_sync_imports_rewired() -> TestResult {
    let text_sync_path = ws("crates/perl-lsp/src/runtime/text_sync.rs");
    let content = fs::read_to_string(&text_sync_path)?;

    // Count occurrences of the old import
    let old_import_count = content.matches("perl_incremental_parsing::").count();

    if old_import_count > 0 {
        return Err(format!(
            "text_sync.rs still contains {} perl_incremental_parsing:: references. \
             All must be rewritten to perl_parser::incremental::",
            old_import_count
        )
        .into());
    }

    Ok(())
}

// =============================================================================
// Section 6: Dependency Cleanup Tests
// =============================================================================

/// Test that perl-parser/Cargo.toml no longer depends on perl-dead-code
#[test]
fn test_perl_parser_no_dead_code_dep() -> TestResult {
    let cargo_toml_path = ws("crates/perl-parser/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    // Look for the dependency line (should be removed)
    if content.contains("perl-dead-code = { workspace = true }") {
        return Err("perl-parser Cargo.toml still depends on perl-dead-code".into());
    }

    Ok(())
}

/// Test that perl-parser/Cargo.toml no longer depends on perl-refactoring
#[test]
fn test_perl_parser_no_refactoring_dep() -> TestResult {
    let cargo_toml_path = ws("crates/perl-parser/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    if content.contains("perl-refactoring = { workspace = true }") {
        return Err("perl-parser Cargo.toml still depends on perl-refactoring".into());
    }

    Ok(())
}

/// Test that perl-parser/Cargo.toml no longer optionally depends on perl-incremental-parsing
#[test]
fn test_perl_parser_no_incremental_dep() -> TestResult {
    let cargo_toml_path = ws("crates/perl-parser/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    if content.contains("perl-incremental-parsing = { workspace = true, optional = true }") {
        return Err("perl-parser Cargo.toml still depends on perl-incremental-parsing".into());
    }

    Ok(())
}

/// Test that perl-lsp/Cargo.toml no longer depends on perl-incremental-parsing
#[test]
fn test_perl_lsp_no_incremental_dep() -> TestResult {
    let cargo_toml_path = ws("crates/perl-lsp/Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    if content.contains("perl-incremental-parsing = {") {
        return Err("perl-lsp Cargo.toml still depends on perl-incremental-parsing".into());
    }

    Ok(())
}

// =============================================================================
// Section 7: Module Structure Tests
// =============================================================================

/// Test that perl-parser/src/dead_code/mod.rs exists (not just a re-export shim)
#[test]
fn test_dead_code_module_structure() -> TestResult {
    // After absorption, the module should be a real directory with content
    let dead_code_mod_path = ws("crates/perl-parser/src/dead_code");

    if !dead_code_mod_path.exists() {
        return Err("perl-parser/src/dead_code/ directory must exist after absorption".into());
    }

    if !dead_code_mod_path.is_dir() {
        return Err("perl-parser/src/dead_code/ must be a directory, not a file".into());
    }

    Ok(())
}

/// Test that perl-parser/src/refactor/ is a real directory (not just a re-export shim file)
#[test]
fn test_refactor_module_structure() -> TestResult {
    let refactor_path = ws("crates/perl-parser/src/refactor");

    if !refactor_path.exists() {
        return Err("perl-parser/src/refactor/ directory must exist after absorption".into());
    }

    if !refactor_path.is_dir() {
        return Err("perl-parser/src/refactor/ must be a directory, not a file".into());
    }

    Ok(())
}

#[cfg(feature = "incremental")]
/// Test that perl-parser/src/incremental/ is a real directory (not just a re-export shim file)
#[test]
fn test_incremental_module_structure() -> TestResult {
    let incremental_path = ws("crates/perl-parser/src/incremental");

    if !incremental_path.exists() {
        return Err("perl-parser/src/incremental/ directory must exist after absorption".into());
    }

    if !incremental_path.is_dir() {
        return Err("perl-parser/src/incremental/ must be a directory, not a file".into());
    }

    Ok(())
}

// =============================================================================
// Section 8: Feature Flag Tests
// =============================================================================

#[cfg(feature = "incremental")]
/// Test that incremental feature properly includes the module
#[test]
fn test_incremental_feature_gated() -> TestResult {
    // This test only compiles when feature "incremental" is enabled
    // If we reach here, the feature compilation succeeded
    let _: perl_parser::incremental::IncrementalState;
    Ok(())
}

// =============================================================================
// Section 9: Default Feature Preservation Tests (Green TDD edge cases)
// =============================================================================

/// Test that workspace_refactor is in default features (memory feedback)
/// Ensures feature-gating didn't accidentally disable the refactor module by default
#[test]
fn test_workspace_refactor_in_default_features() -> TestResult {
    let cargo_toml_path = ws("crates/perl-parser/Cargo.toml");
    let content = fs::read_to_string(cargo_toml_path)?;

    // The `default = [...]` line lives inside the [features] section — split on
    // "[features]" and search the text that follows the header.
    let features_section = content
        .split("[features]")
        .nth(1)
        .ok_or("No [features] section found in perl-parser/Cargo.toml")?;

    let default_line = features_section
        .lines()
        .find(|line| line.starts_with("default = "))
        .ok_or("No `default = ` line found in [features] section")?;

    if default_line.contains("workspace_refactor") {
        Ok(())
    } else {
        Err(format!(
            "workspace_refactor must be in default features after absorption; got: {}",
            default_line
        )
        .into())
    }
}

// =============================================================================
// Section 10: Compatibility Alias Symmetry Tests
// =============================================================================

/// Test that dead_code_detector alias refers to the same type as dead_code
/// Prevents silent divergence that would break backward compatibility
#[test]
fn test_dead_code_alias_symmetry() -> TestResult {
    use std::any::type_name;

    // Both paths should resolve to the same underlying type
    let direct_type = type_name::<perl_parser::dead_code::DeadCodeDetector>();
    let compat_type = type_name::<perl_parser::dead_code_detector::DeadCodeDetector>();

    if direct_type == compat_type {
        Ok(())
    } else {
        Err(format!(
            "dead_code_detector alias diverged: direct={}, compat={}",
            direct_type, compat_type
        )
        .into())
    }
}

// =============================================================================
// Section 11: Old Crate Directory Markers (G2 pattern assertion)
// =============================================================================

/// Test that absorbed crate directories are marked unpublished
/// Stronger than just checking publish flag: verifies integration
#[test]
fn test_absorbed_crates_unpublished_marker() -> TestResult {
    for crate_name in &["perl-dead-code", "perl-refactoring", "perl-incremental-parsing"] {
        let cargo_toml = ws(&format!("crates/{}/Cargo.toml", crate_name));
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if !content.contains("publish = false") {
                return Err(format!(
                    "Absorbed crate {} must have publish = false marker",
                    crate_name
                )
                .into());
            }
        }
    }
    Ok(())
}

// =============================================================================
// Section 12: Module Re-export Verification
// =============================================================================

/// Test that refactor module is properly exported in lib.rs
/// Prevents silent breakage if lib.rs export was missed
#[test]
fn test_refactor_module_in_lib_exports() -> TestResult {
    let lib_rs = fs::read_to_string(ws("crates/perl-parser/src/lib.rs"))?;

    // Should have pub mod refactor or re-export it
    if lib_rs.contains("pub mod refactor") {
        Ok(())
    } else {
        Err("refactor module not properly declared in perl-parser/src/lib.rs".into())
    }
}

/// Test that dead_code module is properly exported in lib.rs
#[test]
fn test_dead_code_module_in_lib_exports() -> TestResult {
    let lib_rs = fs::read_to_string(ws("crates/perl-parser/src/lib.rs"))?;

    if lib_rs.contains("pub mod dead_code") {
        Ok(())
    } else {
        Err("dead_code module not properly declared in perl-parser/src/lib.rs".into())
    }
}

#[cfg(feature = "incremental")]
/// Test that incremental module is properly exported in lib.rs
#[test]
fn test_incremental_module_in_lib_exports() -> TestResult {
    let lib_rs = fs::read_to_string(ws("crates/perl-parser/src/lib.rs"))?;

    if lib_rs.contains("pub mod incremental") {
        Ok(())
    } else {
        Err("incremental module not properly declared in perl-parser/src/lib.rs".into())
    }
}

// =============================================================================
// Section 13: Cross-Crate Dependency Isolation
// =============================================================================

/// Test that no crate except perl-parser depends on perl-dead-code
/// Prevents dueling module paths and import confusion
#[test]
fn test_no_extraneous_dead_code_deps() -> TestResult {
    let crates_dir = ws("crates");

    for entry in
        fs::read_dir(&crates_dir).map_err(|e| format!("Failed to read crates dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let crate_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip the source crate itself
        if crate_name == "perl-dead-code" {
            continue;
        }

        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;

            if content.contains("perl-dead-code = {") {
                return Err(format!(
                    "Crate {} still depends on perl-dead-code (should use perl-parser instead)",
                    crate_name
                )
                .into());
            }
        }
    }

    Ok(())
}

/// Test that no crate except perl-parser depends on perl-refactoring
#[test]
fn test_no_extraneous_refactoring_deps() -> TestResult {
    let crates_dir = ws("crates");

    for entry in
        fs::read_dir(&crates_dir).map_err(|e| format!("Failed to read crates dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let crate_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip source crates themselves
        if crate_name == "perl-refactoring" || crate_name == "perl-parser" {
            continue;
        }

        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;

            if content.contains("perl-refactoring = {") {
                return Err(format!(
                    "Crate {} still depends on perl-refactoring (should use perl-parser instead)",
                    crate_name
                )
                .into());
            }
        }
    }

    Ok(())
}

/// Test that no crate except perl-parser depends on perl-incremental-parsing
#[test]
fn test_no_extraneous_incremental_deps() -> TestResult {
    let crates_dir = ws("crates");

    for entry in
        fs::read_dir(&crates_dir).map_err(|e| format!("Failed to read crates dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let crate_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip source crates themselves
        if crate_name == "perl-incremental-parsing" || crate_name == "perl-parser" {
            continue;
        }

        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;

            if content.contains("perl-incremental-parsing = {") {
                return Err(format!(
                    "Crate {} still depends on perl-incremental-parsing (should use perl-parser::incremental instead)",
                    crate_name
                )
                .into());
            }
        }
    }

    Ok(())
}
