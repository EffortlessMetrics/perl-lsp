//! Tests for include-path scanning in module completion
//!
//! These tests verify that module completion works with external @INC paths.
//!
//! ## What These Tests Cover
//!
//! - AC1: Modules from configured includePaths appear in completion
//! - AC2: Modules from PERL5LIB appear in completion
//! - AC3: Modules from system @INC appear when useSystemInc is true
//! - AC4: External modules sort after workspace but before generic symbols
//! - AC7: Prefix filtering works for external modules

use perl_lsp_completion::{CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

/// RAII guard for a temporary directory that auto-cleans on drop
struct TempModuleDir {
    path: PathBuf,
}

impl TempModuleDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("perl-lsp-test-inc-{}", std::process::id()));
        Self { path }
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

impl Drop for TempModuleDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Helper to create a .pm file at a given path
fn create_module_file(parent: &Path, relative_path: &str, package_name: &str) {
    let full_path = parent.join(relative_path);
    if let Some(parent_dir) = full_path.parent() {
        fs::create_dir_all(parent_dir).ok();
    }
    fs::write(&full_path, format!("package {};\n1;\n", package_name)).ok();
}

// ══════════════════════════════════════════════════════════════════════════════
// AC1: Modules from configured includePaths appear in completion
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_include_paths_modules_appear_in_completion() {
    // Create a temp directory with a module
    let temp_dir = TempModuleDir::new();
    let lib_path = temp_dir.path();

    // Create DBI.pm in the include path (lib/DBI.pm -> DBI)
    create_module_file(&lib_path, "lib/DBI.pm", "DBI");
    create_module_file(&lib_path, "lib/Moo.pm", "Moo");

    // Create the completion provider with include_paths
    let code = "use DB";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![lib_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    // DBI should appear as a module completion from include path
    assert!(
        completions.iter().any(|c| c.label == "DBI" && c.kind == CompletionItemKind::Module),
        "DBI from includePaths should appear in completion; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AC2: Modules from PERL5LIB appear in completion (via include_paths parameter)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_perl5lib_modules_appear_in_completion() {
    // Create a temp directory simulating PERL5LIB
    let temp_dir = TempModuleDir::new();
    let ext_lib_path = temp_dir.path();

    // Create Moo.pm in the "PERL5LIB" path
    create_module_file(&ext_lib_path, "Moo.pm", "Moo");

    // The include_paths simulates what would come from PERL5LIB
    let code = "use Mo";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![ext_lib_path.clone()],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    // Moo should appear from the include path
    assert!(
        completions.iter().any(|c| c.label == "Moo" && c.kind == CompletionItemKind::Module),
        "Moo from PERL5LIB-like path should appear; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AC3: Modules from system @INC appear when useSystemInc is true
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_system_inc_modules_appear_when_use_system_inc_enabled() {
    // Create a temp directory simulating system @INC
    let temp_dir = TempModuleDir::new();
    let system_inc_path = temp_dir.path();

    // Create strict.pm in the system @INC path
    create_module_file(&system_inc_path, "strict.pm", "strict");

    let code = "use st";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // When useSystemInc is true, system_inc_paths should be scanned
    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![],
        vec![system_inc_path.clone()],
    );

    let completions = provider.get_completions(code, code.len());

    // strict should appear from system @INC
    assert!(
        completions.iter().any(|c| c.label == "strict" && c.kind == CompletionItemKind::Module),
        "strict from system @INC should appear when useSystemInc is true; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AC4: External modules sort after workspace but before generic symbols
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_external_modules_sort_after_workspace_tier() {
    // Create a workspace index with a module
    let index = Arc::new(WorkspaceIndex::new());
    let workspace_uri = Url::parse("file:///workspace/My/Module.pm").unwrap();
    index.index_file(workspace_uri, "package My::Module;\n1;\n".to_string()).unwrap();

    // Create an external include path with a different module
    let temp_dir = TempModuleDir::new();
    let include_path = temp_dir.path();
    create_module_file(&include_path, "lib/External/Module.pm", "External::Module");

    let code = "use My";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        Some(index),
        vec![include_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    // Get both workspace and external module completions
    let my_module = completions.iter().find(|c| c.label == "My::Module");
    let external_module = completions.iter().find(|c| c.label == "External::Module");

    let (Some(my_module), Some(external_module)) = (my_module, external_module) else {
        panic!(
            "Expected both My::Module (workspace) and External::Module (external); got: {:?}",
            completions
                .iter()
                .filter(|c| c.kind == CompletionItemKind::Module)
                .map(|c| &c.label)
                .collect::<Vec<_>>()
        );
    };

    // Workspace modules should have sort_text starting with "1_" (tier 1)
    // External modules should have sort_text starting with "2_" (tier 2)
    let my_sort = my_module.sort_text.as_deref().unwrap_or("");
    let ext_sort = external_module.sort_text.as_deref().unwrap_or("");

    assert!(
        my_sort.starts_with("1_"),
        "Workspace module should have tier 1 sort prefix, got: {}",
        my_sort
    );
    assert!(
        ext_sort.starts_with("2_"),
        "External module should have tier 2 sort prefix, got: {}",
        ext_sort
    );

    // Tier 1 should sort before tier 2
    assert!(
        my_sort < ext_sort,
        "Workspace module (tier 1) should sort before external module (tier 2): {} vs {}",
        my_sort,
        ext_sort
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AC7: Prefix filtering works for external modules
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_prefix_filtering_works_for_external_modules() {
    // Create a temp directory with multiple modules
    let temp_dir = TempModuleDir::new();
    let lib_path = temp_dir.path();

    // Create DBD/MySQL.pm and DBD/Oracle.pm
    create_module_file(&lib_path, "lib/DBD/MySQL.pm", "DBD::MySQL");
    create_module_file(&lib_path, "lib/DBD/Oracle.pm", "DBD::Oracle");
    create_module_file(&lib_path, "lib/DBI.pm", "DBI");

    let code = "use DBD::My";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![lib_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    // DBD::MySQL should appear (matches prefix)
    assert!(
        completions.iter().any(|c| c.label == "DBD::MySQL"),
        "DBD::MySQL should appear with prefix DBD::My; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );

    // DBD::Oracle should NOT appear (doesn't match prefix)
    assert!(
        !completions.iter().any(|c| c.label == "DBD::Oracle"),
        "DBD::Oracle should NOT appear when prefix is DBD::My"
    );

    // DBI should NOT appear (doesn't match prefix)
    assert!(
        !completions.iter().any(|c| c.label == "DBI"),
        "DBI should NOT appear when prefix is DBD::My"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Nested module path conversion tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_module_paths_converted_correctly() {
    let temp_dir = TempModuleDir::new();
    let lib_path = temp_dir.path();

    // Create lib/Foo/Bar/Baz.pm -> Foo::Bar::Baz
    create_module_file(&lib_path, "lib/Foo/Bar/Baz.pm", "Foo::Bar::Baz");

    let code = "use Foo::Bar::Ba";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![lib_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "Foo::Bar::Baz"),
        "Foo::Bar::Baz should appear from lib/Foo/Bar/Baz.pm; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_vendor_perl_path_stripped() {
    let temp_dir = TempModuleDir::new();
    let lib_path = temp_dir.path();

    // Create lib/perl5/vendor_perl/My/Vendor/Module.pm -> My::Vendor::Module
    create_module_file(
        &lib_path,
        "lib/perl5/vendor_perl/My/Vendor/Module.pm",
        "My::Vendor::Module",
    );

    let code = "use My::Vendor::Mo";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![lib_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "My::Vendor::Module"),
        "My::Vendor::Module should appear from vendor_perl path; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Both include_paths and system_inc_paths are scanned
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_both_include_paths_and_system_inc_scanned() {
    let temp_dir = TempModuleDir::new();
    let include_path = temp_dir.path().join("include");
    let system_inc_path = temp_dir.path().join("system");

    // Create Foo.pm in include_paths
    create_module_file(&include_path, "Foo.pm", "Foo");
    // Create Bar.pm in system_inc_paths
    create_module_file(&system_inc_path, "Bar.pm", "Bar");

    let code = "use ";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![include_path],
        vec![system_inc_path],
    );

    let completions = provider.get_completions(code, code.len());

    // Both Foo (from include_paths) and Bar (from system_inc_paths) should appear
    assert!(
        completions.iter().any(|c| c.label == "Foo"),
        "Foo from include_paths should appear; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "Bar"),
        "Bar from system_inc_paths should appear; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Deduplication tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_deduplication_across_sources() {
    // Create a workspace index with My::Module
    let index = Arc::new(WorkspaceIndex::new());
    let workspace_uri = Url::parse("file:///workspace/My/Module.pm").unwrap();
    index.index_file(workspace_uri, "package My::Module;\n1;\n".to_string()).unwrap();

    // Create an external include path with the SAME module
    let temp_dir = TempModuleDir::new();
    let include_path = temp_dir.path();
    create_module_file(&include_path, "lib/My/Module.pm", "My::Module");

    let code = "use My::Module";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        Some(index),
        vec![include_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    // My::Module should appear exactly once (deduplicated)
    let my_module_count = completions.iter().filter(|c| c.label == "My::Module").count();
    assert_eq!(
        my_module_count, 1,
        "My::Module should appear exactly once (deduplicated); got {} occurrences",
        my_module_count
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Empty paths and edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_include_paths_works() {
    let code = "use ";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Provider with empty include_paths and system_inc_paths
    let provider =
        CompletionProvider::new_with_index_and_source_with_inc(&ast, code, None, vec![], vec![]);

    let completions = provider.get_completions(code, code.len());

    // Should still work (just won't have external module completions)
    // The common modules like strict, warnings should still appear
    assert!(
        completions.iter().any(|c| c.label == "strict"),
        "strict (common module) should still appear"
    );
}

#[test]
fn test_require_statement_triggers_include_path_completion() {
    let temp_dir = TempModuleDir::new();
    let lib_path = temp_dir.path();

    create_module_file(&lib_path, "lib/DBI.pm", "DBI");

    let code = "require DB";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_with_inc(
        &ast,
        code,
        None,
        vec![lib_path.join("lib")],
        vec![],
    );

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "DBI" && c.kind == CompletionItemKind::Module),
        "DBI from includePaths should appear for require; got: {:?}",
        completions
            .iter()
            .filter(|c| c.kind == CompletionItemKind::Module)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}
