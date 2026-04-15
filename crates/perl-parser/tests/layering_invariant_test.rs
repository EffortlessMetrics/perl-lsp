//! Layering invariant test: verify parser crate has no LSP provider dependencies.
//!
//! This test ensures that `perl-parser` remains a pure leaf crate without
//! LSP-shaped dependencies. The parser should only depend on core language
//! processing crates, not on application-layer LSP providers.
//!
//! The test is designed to FAIL before the refactor (#4414) and PASS after
//! removal of the 8 LSP provider re-exports and dependencies.

use std::path::Path;
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
    #[allow(clippy::expect_used)]
    let output = Command::new("cargo")
        .args(["tree", "-p", "perl-parser", "--edges", "normal"])
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
    let found_lsp_deps: Vec<_> = stdout
        .lines()
        .filter_map(|line| {
            for crate_name in &forbidden_crates {
                if line.contains(crate_name) {
                    return Some(format!("  {}", line.trim()));
                }
            }
            None
        })
        .collect();

    assert!(
        found_lsp_deps.is_empty(),
        "ERROR: perl-parser still depends on LSP provider crates (should be removed per #4414):\n{}\n\nFull cargo tree output:\n{}",
        found_lsp_deps.join("\n"),
        stdout
    );

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "cargo tree command failed:\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
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

/// Test: refactor module re-exports are still accessible.
///
/// Regression guard: verify that the `refactor` module, which provides
/// import optimization and code modernization, is still accessible after
/// the refactor. The refactor module should remain as a legitimate re-export.
///
/// This test validates that lines 436-442 of lib.rs (refactor re-exports)
/// remain accessible.
#[test]
fn when_refactor_module_refactored_then_import_optimizer_accessible() {
    // These re-exports should survive the refactor per acceptance criterion line 14:
    // "Lines 498-513 in crates/perl-parser/src/lib.rs are preserved (legitimate refactor/tokens re-exports)"
    use perl_parser::refactor::import_optimizer;

    // Verify the module is accessible and contains expected public types
    // (the import_optimizer module should exist and be re-exported)
    // The mere fact that we can import it proves it's accessible
    let _ = std::any::type_name::<import_optimizer::ImportOptimizer>();
}

/// Test: tokens module re-exports are still accessible.
///
/// Regression guard: verify that the `tokens` module, which provides
/// token stream handling and token wrappers, is still accessible after
/// the refactor. The tokens module should remain as a legitimate re-export.
///
/// This test validates that lines 443-450 of lib.rs (tokens re-exports)
/// remain accessible.
#[test]
fn when_tokens_module_refactored_then_token_stream_accessible() {
    // These re-exports should survive the refactor per acceptance criterion line 14:
    // "Lines 498-513 in crates/perl-parser/src/lib.rs are preserved (legitimate refactor/tokens re-exports)"
    use perl_parser::tokens::token_stream;

    // Verify the module is accessible and has expected public content
    // The token_stream module should be re-exported from perl-parser-core
    let _ = std::any::type_name::<token_stream::TokenStream>();
}

/// Test: parser crate does NOT depend on DAP (Debug Adapter Protocol) crates.
///
/// Boundary condition: verify that the layering invariant is symmetric.
/// Just as LSP provider crates should not be dependencies, DAP provider
/// crates (like perl-dap, perl-dap-*) should also not be in the dependency
/// tree of perl-parser. The parser is a language processor, not tied to
/// any particular IDE or debug protocol.
///
/// This test extends the LSP layering check to also catch any accidental
/// DAP dependencies that might creep in.
#[test]
fn when_parser_layering_is_correct_then_no_dap_provider_deps_in_tree() {
    #[allow(clippy::expect_used)]
    let output = Command::new("cargo")
        .args(["tree", "-p", "perl-parser", "--edges", "normal"])
        .output()
        .expect("Failed to run cargo tree");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Collect all DAP-related crate names that should NOT appear in tree
    let forbidden_crates =
        vec!["perl-dap", "perl-dap-types", "perl-dap-protocol", "perl-dap-server"];

    // Check each line of output for any forbidden crate names
    let found_dap_deps: Vec<_> = stdout
        .lines()
        .filter_map(|line| {
            for crate_name in &forbidden_crates {
                if line.contains(crate_name) {
                    return Some(format!("  {}", line.trim()));
                }
            }
            None
        })
        .collect();

    assert!(
        found_dap_deps.is_empty(),
        "ERROR: perl-parser unexpectedly depends on DAP crates (should be kept separate per #4410):\n{}\n\nFull cargo tree output:\n{}",
        found_dap_deps.join("\n"),
        stdout
    );

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "cargo tree command failed:\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
}

/// Test: tooling.rs file was removed cleanly.
///
/// Boundary condition: verify that the `crates/perl-parser/src/tooling.rs`
/// file that contained perltidy/perlcritic re-exports no longer exists.
/// This is a regression guard to ensure the refactor cleanup was complete.
///
/// The tooling module was removed in this refactor because it re-exported
/// LSP-provider-level tooling, which violates the parser layer's abstraction.
#[test]
fn when_tooling_module_refactored_then_tooling_rs_removed() {
    // Get the absolute path to the workspace root
    let workspace_root = env!("CARGO_MANIFEST_DIR");

    let tooling_rs_path = Path::new(workspace_root).join("src").join("tooling.rs");

    assert!(
        !tooling_rs_path.exists(),
        "tooling.rs should be removed in refactor #4414, but found at: {}",
        tooling_rs_path.display()
    );
}

/// Test: refactor/tokens re-export paths match spec expectations.
///
/// Integration test: verify that the re-export structure matches the
/// acceptance criterion (lines 436-450 of lib.rs are preserved).
/// This validates that direct imports of refactoring and token utilities
/// work as expected and haven't been broken by the refactor.
///
/// This test uses actual import patterns to verify the re-exports are
/// truly available, not just syntactically present.
#[test]
fn when_refactor_tokens_preserved_then_imports_are_valid() {
    // Verify refactor module functions are still accessible via re-export and work at runtime.
    use perl_parser::import_optimizer::ImportOptimizer;
    use perl_parser::token_stream::TokenStream;

    // ImportOptimizer::new() proves the constructor is accessible via the re-exported path.
    // If the re-export chain (perl-parser → perl-refactoring) were broken, this would
    // fail to compile — not just vacuously pass.
    let optimizer = ImportOptimizer::new();
    let result = optimizer.analyze_content("use strict;\nuse warnings;\n");
    assert!(
        result.is_ok(),
        "ImportOptimizer::analyze_content should succeed on valid Perl: {:?}",
        result.err()
    );

    // TokenStream::new() proves the constructor is accessible and produces a usable stream.
    // The stream over a real Perl snippet should peek successfully (not fail immediately).
    let mut stream = TokenStream::new("my $x = 42;");
    let first = stream.peek();
    assert!(
        first.is_ok(),
        "TokenStream over 'my $x = 42;' should peek successfully"
    );
}
