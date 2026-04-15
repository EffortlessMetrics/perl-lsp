//! Layering invariant test: verify parser crate has no LSP provider dependencies.
//!
//! This test ensures that `perl-parser` remains a pure leaf crate without
//! LSP-shaped dependencies. The parser should only depend on core language
//! processing crates, not on application-layer LSP providers.
//!
//! The test is designed to FAIL before the refactor (#4414) and PASS after
//! removal of the 8 LSP provider re-exports and dependencies.

use std::process::Command;

/// Test: parser crate has no LSP provider dependencies in dependency tree.
///
/// Verifies that `cargo tree -p perl-parser --edges normal` output does NOT
/// contain any of the 8 LSP provider crate names:
/// - perl-lsp-code-actions
/// - perl-lsp-completion
/// - perl-lsp-diagnostics
/// - perl-lsp-inlay-hints
/// - perl-lsp-navigation
/// - perl-lsp-rename
/// - perl-lsp-semantic-tokens
/// - perl-lsp-tooling
///
/// **Before refactor**: This test FAILS (LSP crates are in Cargo.toml as dependencies)
/// **After refactor**: This test PASSES (LSP crates are removed from dependencies)
#[test]
fn when_parser_layering_is_correct_then_no_lsp_provider_deps_in_tree() {
    let output = Command::new("cargo")
        .args(&["tree", "-p", "perl-parser", "--edges", "normal"])
        .output()
        .expect("Failed to run cargo tree");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Collect all LSP provider crate names that should NOT appear in tree
    let forbidden_crates = vec![
        "perl-lsp-code-actions",
        "perl-lsp-completion",
        "perl-lsp-diagnostics",
        "perl-lsp-inlay-hints",
        "perl-lsp-navigation",
        "perl-lsp-rename",
        "perl-lsp-semantic-tokens",
        "perl-lsp-tooling",
    ];

    // Check each line of output for any forbidden crate names
    let mut found_lsp_deps = Vec::new();
    for line in stdout.lines() {
        for crate_name in &forbidden_crates {
            if line.contains(crate_name) {
                found_lsp_deps.push(format!("  {}", line.trim()));
            }
        }
    }

    if !found_lsp_deps.is_empty() {
        panic!(
            "ERROR: perl-parser still depends on LSP provider crates (should be removed per #4414):\n{}\n\nFull cargo tree output:\n{}",
            found_lsp_deps.join("\n"),
            stdout
        );
    }

    // Verify the command succeeded
    if !output.status.success() {
        panic!("cargo tree command failed:\nstdout:\n{}\nstderr:\n{}", stdout, stderr);
    }
}

/// Test: perl-parser imports can be resolved directly from perl-lsp-semantic-tokens.
///
/// Validates that the `semantic_tokens` module, which will be re-imported from
/// `perl_lsp_semantic_tokens` after the refactor, is accessible and its
/// public API is unchanged.
///
/// **Before refactor**: semantic_tokens comes from perl-lsp-semantic-tokens (indirect via re-export)
/// **After refactor**: semantic_tokens must be imported directly from perl-lsp-semantic-tokens
#[test]
fn when_semantic_tokens_refactored_then_legend_function_works() {
    // After refactor (#4414), semantic_tokens is no longer re-exported from perl_parser.
    // Import directly from perl_lsp_semantic_tokens.
    use perl_lsp_semantic_tokens as semantic_tokens;

    let legend = semantic_tokens::legend();
    assert!(!legend.token_types.is_empty(), "semantic token types should not be empty");
    assert!(!legend.modifiers.is_empty(), "semantic token modifiers should not be empty");

    // Verify common token types exist
    assert!(
        legend.token_types.contains(&"keyword".to_string()),
        "should contain 'keyword' token type"
    );
    assert!(
        legend.token_types.contains(&"variable".to_string()),
        "should contain 'variable' token type"
    );
}

/// Test: semantic_tokens import alias works after refactoring.
///
/// Validates that the import alias pattern used in ast_snapshot_tests.rs:13
/// compiles correctly after the refactor. The refactor changes the import from:
///   use perl_parser::{Parser, semantic_tokens};
/// to:
///   use perl_parser::Parser;
///   use perl_lsp_semantic_tokens as semantic_tokens;
///
/// This test imports semantic_tokens module using the new pattern to ensure
/// it compiles and the legend() function is accessible.
#[test]
fn when_semantic_tokens_import_refactored_then_legend_accessible() {
    // This pattern matches the refactored import in ast_snapshot_tests.rs after #4414
    use perl_lsp_semantic_tokens as semantic_tokens_module;

    let legend = semantic_tokens_module::legend();
    assert!(
        !legend.token_types.is_empty(),
        "semantic token types should not be empty after refactor"
    );
    assert!(
        !legend.modifiers.is_empty(),
        "semantic token modifiers should not be empty after refactor"
    );
}
