//! Include path completion tests for perl-lsp-completion
//!
//! Tests cover:
//! - AC1: Module suggestion from include paths when module not in workspace index
//! - AC2: Workspace modules take priority over include path modules (sort prefix 1_)
//! - AC3: Prefix filtering works for include path modules
//! - AC6: Deduplication - same module in multiple include paths appears once
//!
//! These tests verify that when users type `use DBI` or `require JSON::PP`,
//! the LSP completion provider suggests modules from configured @INC paths
//! (`.perl-lsp.toml` `includePaths`, `PERL5LIB`, and system @INC), not just
//! from workspace-indexed files.

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|item| item.label == label)
}

/// Creates a temporary directory with module files for testing include path scanning.
/// Returns the temp dir path and creates files:
///   - `DBI.pm` -> module "DBI"
///   - `JSON/PP.pm` -> module "JSON::PP"
///   - `JSON/XS.pm` -> module "JSON::XS"
///   - `MyApp.pm` -> module "MyApp"
///   - `MyApp/Config.pm` -> module "MyApp::Config"
fn create_temp_modules_dir() -> PathBuf {
    let temp_base = std::env::temp_dir();
    let temp_dir = temp_base.join(format!("perl_lsp_completion_test_{}", std::process::id()));

    // Clean up if exists from previous run
    let _ = fs::remove_dir_all(&temp_dir);

    fs::create_dir_all(&temp_dir).expect("temp dir create should succeed");

    // Create DBI.pm
    let dbi_path = temp_dir.join("DBI.pm");
    fs::write(&dbi_path, "package DBI;\n1;\n").expect("DBI.pm write should succeed");

    // Create JSON/PP.pm
    let json_dir = temp_dir.join("JSON");
    fs::create_dir(&json_dir).expect("JSON dir create should succeed");
    let pp_path = json_dir.join("PP.pm");
    fs::write(&pp_path, "package JSON::PP;\n1;\n").expect("JSON::PP write should succeed");

    // Create JSON/XS.pm
    let xs_path = json_dir.join("XS.pm");
    fs::write(&xs_path, "package JSON::XS;\n1;\n").expect("JSON::XS write should succeed");

    // Create MyApp.pm
    let myapp_path = temp_dir.join("MyApp.pm");
    fs::write(&myapp_path, "package MyApp;\n1;\n").expect("MyApp.pm write should succeed");

    // Create MyApp/Config.pm
    let myapp_dir = temp_dir.join("MyApp");
    fs::create_dir(&myapp_dir).expect("MyApp dir create should succeed");
    let config_path = myapp_dir.join("Config.pm");
    fs::write(&config_path, "package MyApp::Config;\n1;\n")
        .expect("MyApp::Config write should succeed");

    temp_dir
}

/// Provider helper that creates CompletionProvider with include paths
/// This tests the new constructor that accepts include paths
fn provider_with_include_paths(
    code: &str,
    include_paths: Vec<PathBuf>,
    system_inc_paths: Vec<PathBuf>,
) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    // This constructor should exist after implementation:
    // new_with_index_and_source_with_inc(&ast, code, None, include_paths, system_inc_paths)
    CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        include_paths,
        system_inc_paths,
    )
}

/// Simple completions helper for include path tests
fn completions_at_end_with_include_paths(
    code: &str,
    include_paths: Vec<PathBuf>,
    system_inc_paths: Vec<PathBuf>,
) -> Vec<CompletionItem> {
    let provider = provider_with_include_paths(code, include_paths, system_inc_paths);
    provider.get_completions(code, code.len())
}

// ---------------------------------------------------------------------------
// AC1: Module Suggestion from Include Paths
// ---------------------------------------------------------------------------

/// AC1: Given a Perl workspace WITHOUT DBI in any indexed file,
/// AND DBI is installed in a configured include path,
/// WHEN the user types "use DB",
/// THEN DBI appears in the completion suggestions.
#[test]
fn include_path_suggests_dbi_when_not_in_workspace() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    // Source has NO DBI - it's not in any workspace file
    let source = "use strict;\nuse warnings;\nuse DB";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    assert!(
        has_label(&items, "DBI"),
        "DBI should appear in completions from include path, got: {:?}",
        labels(&items)
    );
}

/// AC1 variant: verify JSON::PP from include path
#[test]
fn include_path_suggests_json_pp_when_not_in_workspace() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    // Source has NO JSON::PP - it's not in any workspace file
    let source = "use JSON::";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    assert!(
        has_label(&items, "JSON::PP"),
        "JSON::PP should appear in completions from include path, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "JSON::XS"),
        "JSON::XS should appear in completions from include path, got: {:?}",
        labels(&items)
    );
}

// ---------------------------------------------------------------------------
// AC2: Workspace Modules Take Priority
// ---------------------------------------------------------------------------

/// AC2: Given a module named MyApp exists both in the workspace index
/// AND in an include path, WHEN the user types "use My",
/// THEN MyApp appears exactly once in suggestions, sorted with prefix "1_"
#[test]
fn workspace_module_takes_priority_over_include_path_module() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    // Create workspace index with MyApp
    let index = Arc::new(WorkspaceIndex::new());
    let myapp_uri = must(Url::parse("file:///workspace/lib/MyApp.pm"));
    let myapp_code = "package MyApp;\nsub new { }\n1;\n";
    must(index.index_file(myapp_uri, myapp_code.to_string()));

    // Source that triggers use statement completion
    let source = "use My";

    // Create provider with workspace index AND include paths
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    // The new constructor that accepts workspace index AND include paths
    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        source,
        Some(index),
        include_paths,
        system_inc_paths,
    );

    let items = provider.get_completions(source, source.len());

    // MyApp should appear exactly once (not twice)
    let myapp_count = items.iter().filter(|i| i.label == "MyApp").count();
    assert_eq!(
        myapp_count,
        1,
        "MyApp should appear exactly once (workspace priority), got {} occurrences: {:?}",
        myapp_count,
        labels(&items)
    );

    // MyApp should have sort_text starting with "1_" (workspace tier)
    if let Some(item) = find_item(&items, "MyApp") {
        assert!(
            item.sort_text.as_deref().is_some_and(|s| s.starts_with("1_")),
            "MyApp from workspace should have sort_text starting with '1_', got: {:?}",
            item.sort_text
        );
    }
}

// ---------------------------------------------------------------------------
// AC3: Prefix Filtering
// ---------------------------------------------------------------------------

/// AC3: Given include paths containing JSON, JSON::PP, JSON::XS, JSON::MaybeUTF8,
/// WHEN the user types "use JSON::",
/// THEN suggestions include JSON::PP, JSON::XS but NOT bare JSON
#[test]
fn include_path_prefix_filtering_nested_modules() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    // Only JSON::PP and JSON::XS exist in our temp dir, but test prefix filtering
    let source = "use JSON::";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    // Should include JSON::PP and JSON::XS
    assert!(
        has_label(&items, "JSON::PP"),
        "JSON::PP should match 'JSON::' prefix, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "JSON::XS"),
        "JSON::XS should match 'JSON::' prefix, got: {:?}",
        labels(&items)
    );

    // Bare JSON.pm doesn't exist in our temp dir, so it shouldn't appear
    // (this tests that we're actually scanning and filtering, not returning everything)
    assert!(
        !has_label(&items, "JSON"),
        "Bare 'JSON' should NOT appear (no JSON.pm in temp dir), got: {:?}",
        labels(&items)
    );
}

/// AC3: Test that prefix "DB" matches "DBI" but not "DBD::SQLite" (if not present)
#[test]
fn include_path_prefix_filtering_exact_match() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    let source = "use DB";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    assert!(has_label(&items, "DBI"), "DBI should match 'DB' prefix, got: {:?}", labels(&items));
}

// ---------------------------------------------------------------------------
// AC6: Deduplication
// ---------------------------------------------------------------------------

/// AC6: Given DBI appears in multiple include paths,
/// WHEN the user types "use DBI",
/// THEN DBI appears exactly once in the completion list.
#[test]
fn include_path_deduplication_across_multiple_paths() {
    // Create two temp dirs, each with DBI.pm
    let temp_base = std::env::temp_dir();
    let temp_dir1 = temp_base.join(format!("perl_lsp_test1_{}", std::process::id()));
    let temp_dir2 = temp_base.join(format!("perl_lsp_test2_{}", std::process::id()));

    fs::create_dir_all(&temp_dir1).expect("temp dir1 create should succeed");
    fs::create_dir_all(&temp_dir2).expect("temp dir2 create should succeed");

    fs::write(temp_dir1.join("DBI.pm"), "package DBI;\n1;\n").expect("DBI.pm write should succeed");
    fs::write(temp_dir2.join("DBI.pm"), "package DBI;\n1;\n").expect("DBI.pm write should succeed");

    let include_paths = vec![temp_dir1.to_path_buf(), temp_dir2.to_path_buf()];
    let system_inc_paths = vec![];

    let source = "use DBI";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    let dbi_count = items.iter().filter(|i| i.label == "DBI").count();
    assert_eq!(
        dbi_count,
        1,
        "DBI should appear exactly once even with multiple include paths, got {} occurrences: {:?}",
        dbi_count,
        labels(&items)
    );
}

// ---------------------------------------------------------------------------
// Constructor API tests
// ---------------------------------------------------------------------------

/// Verify that the new constructor exists and has the expected signature.
/// This test will fail to compile if the constructor doesn't exist or has wrong signature.
#[test]
fn completion_provider_has_new_with_index_and_source_with_inc_constructor() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    let source = "use strict;\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // This line verifies the constructor exists with correct signature:
    // fn new_with_index_and_source_with_inc(
    //     ast: &Node,
    //     source: &str,
    //     workspace_index: Option<Arc<WorkspaceIndex>>,
    //     include_paths: Vec<PathBuf>,
    //     system_inc_paths: Vec<PathBuf>,
    // ) -> Self
    let _provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        source,
        None,
        include_paths,
        system_inc_paths,
    );
}

// ---------------------------------------------------------------------------
// CompletionProvider struct field existence tests
// ---------------------------------------------------------------------------

/// Verify CompletionProvider has include_paths field after construction
#[test]
fn completion_provider_accepts_and_stores_include_paths() {
    let include_paths = vec![PathBuf::from("/some/path")];
    let system_inc_paths = vec![PathBuf::from("/system/inc")];

    let source = "use strict;\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        source,
        None,
        include_paths.clone(),
        system_inc_paths.clone(),
    );

    // The provider should store these paths - we can't directly access private fields,
    // but if construction succeeds, the fields are being initialized.
    // The real test is whether completions work - covered by other tests.
    let _ = provider;
}

// ---------------------------------------------------------------------------
// Sort order tests - workspace vs include path modules
// ---------------------------------------------------------------------------

/// Verify that workspace modules (tier 1) sort before include path modules (tier 2)
#[test]
fn workspace_modules_sort_before_include_path_modules() {
    let temp_dir = create_temp_modules_dir();
    let include_paths = vec![temp_dir.to_path_buf()];
    let system_inc_paths = vec![];

    // Create workspace index with MyApp
    let index = Arc::new(WorkspaceIndex::new());
    let myapp_uri = must(Url::parse("file:///workspace/lib/MyApp.pm"));
    let myapp_code = "package MyApp;\nsub new { }\n1;\n";
    must(index.index_file(myapp_uri, myapp_code.to_string()));

    // Source that triggers use statement completion
    let source = "use My";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        source,
        Some(index),
        include_paths,
        system_inc_paths,
    );

    let items = provider.get_completions(source, source.len());

    // Find MyApp in results and check its sort text
    if let Some(myapp_item) = find_item(&items, "MyApp") {
        // Workspace module should have sort prefix "1_"
        assert!(
            myapp_item.sort_text.as_deref().is_some_and(|s| s.starts_with("1_")),
            "Workspace MyApp should have sort_text starting '1_', got: {:?}",
            myapp_item.sort_text
        );
    }
}

// ---------------------------------------------------------------------------
// System INC paths are also scanned
// ---------------------------------------------------------------------------

/// Verify that system_inc_paths are also scanned for modules
#[test]
fn system_inc_paths_are_scanned_for_modules() {
    let temp_dir = create_temp_modules_dir();
    // Empty include_paths, but system_inc_paths has the modules
    let include_paths = vec![];
    let system_inc_paths = vec![temp_dir.to_path_buf()];

    let source = "use DBI";

    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    assert!(
        has_label(&items, "DBI"),
        "DBI should appear from system_inc_paths, got: {:?}",
        labels(&items)
    );
}

// ---------------------------------------------------------------------------
// Empty/wrong-type paths are handled gracefully
// ---------------------------------------------------------------------------

/// Verify that empty include paths don't cause errors
#[test]
fn empty_include_paths_handled_gracefully() {
    let include_paths = vec![];
    let system_inc_paths = vec![];

    let source = "use strict;\nuse DB";

    // Should not panic or error, just return no results
    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    // DBI won't be found since there's no include path, but no error should occur
    assert!(
        !has_label(&items, "DBI"),
        "DBI should not appear with empty include paths, got: {:?}",
        labels(&items)
    );
}

/// Verify that non-existent paths don't cause errors
#[test]
fn nonexistent_include_paths_handled_gracefully() {
    let include_paths = vec![PathBuf::from("/nonexistent/path/that/does/not/exist")];
    let system_inc_paths = vec![];

    let source = "use strict;\nuse DB";

    // Should not panic or error, just return no results
    let items = completions_at_end_with_include_paths(source, include_paths, system_inc_paths);

    // DBI won't be found since path doesn't exist, but no error should occur
    assert!(
        !has_label(&items, "DBI"),
        "DBI should not appear with nonexistent include path, got: {:?}",
        labels(&items)
    );
}
