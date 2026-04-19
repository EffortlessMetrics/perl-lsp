//! Red TDD tests for issue #4500: Wave G1a Collapse (15 Providers → perl-lsp-rs-core)
//!
//! These tests validate the structural outcome of the microcrate collapse:
//! - All 15 provider crates are removed from crates/ directory
//! - Published crate count decreases from 74 → 59
//! - The providers module is registered in perl-lsp-rs-core
//! - Consumer crates have updated imports
//!
//! These tests must FAIL before implementation and PASS after.

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    // Walk up from the manifest directory to the workspace root.
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // xtask is at <root>/xtask -- go up one level
    dir.pop();
    dir
}

/// All 15 old provider crate directories must be deleted after collapse.
/// This test asserts that none of the following exist:
/// - perl-lsp-completion-item
/// - perl-lsp-file-completion
/// - perl-lsp-code-lens
/// - perl-lsp-document-highlight
/// - perl-lsp-folding
/// - perl-lsp-selection-range
/// - perl-lsp-inlay-hints
/// - perl-lsp-type-hierarchy
/// - perl-lsp-formatting-types
/// - perl-lsp-on-type-formatting
/// - perl-lsp-color-provider
/// - perl-lsp-symbol-query
/// - perl-lsp-import-management
/// - perl-lsp-document-links
/// - perl-lsp-workspace-symbols
#[test]
fn test_all_15_old_provider_crates_directories_removed() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let crates_dir = root.join("crates");

    let old_crates = vec![
        "perl-lsp-completion-item",
        "perl-lsp-file-completion",
        "perl-lsp-code-lens",
        "perl-lsp-document-highlight",
        "perl-lsp-folding",
        "perl-lsp-selection-range",
        "perl-lsp-inlay-hints",
        "perl-lsp-type-hierarchy",
        "perl-lsp-formatting-types",
        "perl-lsp-on-type-formatting",
        "perl-lsp-color-provider",
        "perl-lsp-symbol-query",
        "perl-lsp-import-management",
        "perl-lsp-document-links",
        "perl-lsp-workspace-symbols",
    ];

    let mut still_present = Vec::new();
    for crate_name in old_crates {
        let crate_path = crates_dir.join(crate_name);
        if crate_path.exists() {
            still_present.push(crate_name);
        }
    }

    assert!(
        still_present.is_empty(),
        "G1a collapse: the following 15 provider crates must be deleted but still exist:\n{}\n\n\
         Expected: all 15 crate directories removed from crates/\n\
         See .spec/4500-wave-g1a-providers/acceptance.md line 13",
        still_present.iter().map(|s| format!("  - crates/{}/", s)).collect::<Vec<_>>().join("\n")
    );
    Ok(())
}

/// The published crate count must decrease from 74 → 59 after collapse.
/// This test reads xtask/published-crate-baseline.txt and asserts it contains
/// exactly the number 59 (which corresponds to 74 − 15 = 59).
#[test]
fn test_published_crate_count_is_59_after_collapse() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");

    let content = fs::read_to_string(&baseline_path)?;
    let count: u32 = content.trim().parse()?;

    assert_eq!(
        count, 59,
        "G1a collapse: published crate count must be exactly 59.\n\
         Before G1a: 74 crates (includes 15 providers)\n\
         After G1a:  59 crates (15 providers collapsed into perl-lsp-rs-core)\n\
         Equation: 74 − 15 = 59\n\
         Current count in xtask/published-crate-baseline.txt: {}\n\
         See .spec/4500-wave-g1a-providers/acceptance.md line 67",
        count
    );
    Ok(())
}

/// The providers module must be declared in perl-lsp-rs-core/src/lib.rs.
/// This test checks that `pub mod providers;` appears in the lib.rs file.
#[test]
fn test_providers_module_declared_in_perl_lsp_rs_core_lib() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let lib_path = root.join("crates/perl-lsp-rs-core/src/lib.rs");

    let content = fs::read_to_string(&lib_path)?;
    assert!(
        content.contains("pub mod providers;"),
        "G1a collapse: `pub mod providers;` must be declared in crates/perl-lsp-rs-core/src/lib.rs.\n\
         See .spec/4500-wave-g1a-providers/checklist.md Step 1.2"
    );
    Ok(())
}

/// The providers/mod.rs file must exist and declare all 15 submodules.
/// This test verifies the file exists and contains declarations for all submodules.
#[test]
fn test_providers_mod_rs_file_exists_with_all_submodules() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let mod_path = root.join("crates/perl-lsp-rs-core/src/providers/mod.rs");

    let content = fs::read_to_string(&mod_path)?;

    let required_modules = vec![
        "completion_item",
        "file_completion",
        "code_lens",
        "document_highlight",
        "folding",
        "selection_range",
        "inlay_hints",
        "type_hierarchy",
        "formatting_types",
        "on_type_formatting",
        "color",
        "symbol_query",
        "import_management",
        "document_links",
        "workspace_symbols",
    ];

    let mut missing = Vec::new();
    for module_name in required_modules {
        if !content.contains(&format!("pub mod {};", module_name))
            && !content.contains(&format!("pub mod {}", module_name))
        {
            missing.push(module_name);
        }
    }

    assert!(
        missing.is_empty(),
        "G1a collapse: providers/mod.rs must declare all 15 submodules but is missing:\n{}\n\n\
         See .spec/4500-wave-g1a-providers/acceptance.md line 10",
        missing.iter().map(|s| format!("  - pub mod {};", s)).collect::<Vec<_>>().join("\n")
    );
    Ok(())
}

/// Verify that the wired_crates_integration_test.rs has been updated
/// to use the new perl_lsp_rs_core::providers::* imports.
/// This test checks for absence of old crate names and presence of new imports.
#[test]
fn test_wired_crates_integration_uses_new_provider_imports()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let test_path = root.join("crates/perl-lsp/tests/wired_crates_integration_test.rs");

    let content = fs::read_to_string(&test_path)?;

    // These old crate names should NOT appear in the file after collapse
    let old_imports = vec![
        "perl_lsp_completion_item::",
        "perl_lsp_file_completion::",
        "perl_lsp_symbol_query::",
        "perl_lsp_workspace_symbols::",
        "perl_lsp_formatting_types::",
        "perl_lsp_import_management::",
        "perl_lsp_document_links::",
    ];

    let mut still_present = Vec::new();
    for old_import in old_imports {
        if content.contains(old_import) {
            still_present.push(old_import);
        }
    }

    assert!(
        still_present.is_empty(),
        "G1a collapse: wired_crates_integration_test.rs must be updated to use new provider imports.\n\
         The following old import names must be replaced with perl_lsp_rs_core::providers::*:\n{}\n\n\
         See .spec/4500-wave-g1a-providers/acceptance.md line 53-59",
        still_present.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n")
    );
    Ok(())
}

/// Verify that new provider imports DO appear in wired_crates_integration_test.rs.
/// This test checks for at least 6 lines using perl_lsp_rs_core::providers::.
#[test]
fn test_wired_crates_integration_has_new_provider_imports() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let test_path = root.join("crates/perl-lsp/tests/wired_crates_integration_test.rs");

    let content = fs::read_to_string(&test_path)?;

    let new_import_count = content.matches("perl_lsp_rs_core::providers::").count();

    assert!(
        new_import_count >= 6,
        "G1a collapse: wired_crates_integration_test.rs must have at least 6 new provider imports.\n\
         Expected: perl_lsp_rs_core::providers::PROVIDER:: appearing ≥ 6 times\n\
         Found: {} occurrences\n\n\
         See .spec/4500-wave-g1a-providers/acceptance.md line 53-59",
        new_import_count
    );
    Ok(())
}
