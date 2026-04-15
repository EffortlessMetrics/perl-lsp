//! Snapshot tests for @INC path module completion
//!
//! These tests capture the actual output of the completion provider for
//! @INC path module completion scenarios. The snapshots serve as baselines
//! so any change in output is immediately detected.
//!
//! Snapshots captured:
//! 1. Simple include path completion (e.g., `use DB` -> `DBI` with "(include)" detail)
//! 2. System @INC completion (e.g., `use Mo` -> `Moo` with "(system)" detail)
//! 3. Nested module completion (e.g., `use File::Path::To::Mo` -> `File::Path::To::Module`)
//! 4. Prefix filtering (e.g., `use DB` -> only modules starting with `DB`)
//! 5. Deduplication (workspace module takes priority over include path)
//! 6. Empty paths (graceful handling)
//! 7. Multi-level nested modules

use insta::assert_yaml_snapshot;
use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace_index::workspace_index::WorkspaceIndex;
use serde::Serialize;
use std::sync::Arc;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Creates a temp directory with a .pm file for testing include path scanning
fn create_temp_module(
    temp_dir: &std::path::Path,
    module_name: &str,
    package_content: &str,
) -> std::io::Result<()> {
    let module_path = temp_dir.join(module_name);
    if let Some(parent) = module_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&module_path, package_content)?;
    Ok(())
}

/// Snapshot-friendly representation of a completion item
/// Omits fields that are not relevant for @INC path completion snapshots
/// or that contain non-deterministic data.
#[derive(Debug, Serialize)]
struct SnapshotCompletionItem {
    label: String,
    kind: &'static str,
    detail: Option<String>,
    insert_text: Option<String>,
    sort_text: Option<String>,
    filter_text: Option<String>,
    // Omit: documentation (can be verbose)
    // Omit: additional_edits (contains SourceLocation which is non-deterministic)
    // Omit: text_edit_range (not relevant for module name completion)
    // Omit: commit_characters (not relevant for these snapshots)
}

impl From<&CompletionItem> for SnapshotCompletionItem {
    fn from(item: &CompletionItem) -> Self {
        SnapshotCompletionItem {
            label: item.label.clone(),
            kind: match item.kind {
                CompletionItemKind::Variable => "Variable",
                CompletionItemKind::Function => "Function",
                CompletionItemKind::Keyword => "Keyword",
                CompletionItemKind::Module => "Module",
                CompletionItemKind::File => "File",
                CompletionItemKind::Snippet => "Snippet",
                CompletionItemKind::Constant => "Constant",
                CompletionItemKind::Property => "Property",
            },
            detail: item.detail.clone(),
            insert_text: item.insert_text.clone(),
            sort_text: item.sort_text.clone(),
            filter_text: item.filter_text.clone(),
        }
    }
}

/// Get completion items as snapshot-friendly format
fn get_snapshot_items(items: &[CompletionItem]) -> Vec<SnapshotCompletionItem> {
    items.iter().map(SnapshotCompletionItem::from).collect()
}

/// Helper to run completion and return snapshot items
fn run_completion(
    code: &str,
    include_paths: &[std::path::PathBuf],
    system_inc_paths: &[std::path::PathBuf],
) -> Vec<SnapshotCompletionItem> {
    let index = Arc::new(WorkspaceIndex::new());
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        include_paths,
        system_inc_paths,
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    get_snapshot_items(&items)
}

// -----------------------------------------------------------------------------
// Snapshot Tests
// -----------------------------------------------------------------------------

/// Snapshot: Simple include path completion
/// Given DBI.pm in include path, typing `use DB` should suggest DBI
#[test]
fn snapshot_include_path_simple() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n")
        .expect("creating DBI.pm should succeed");

    let items = run_completion("use DB", &[include_path], &[]);

    assert_yaml_snapshot!("include_path_simple", items);
}

/// Snapshot: System @INC completion
/// Given Moo.pm in system path, typing `use Mo` should suggest Moo
#[test]
fn snapshot_system_inc_simple() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let system_inc_path = temp_dir.path().to_path_buf();

    create_temp_module(&system_inc_path, "Moo.pm", "package Moo;\n1;\n")
        .expect("creating Moo.pm should succeed");

    let items = run_completion("use Mo", &[], &[system_inc_path]);

    assert_yaml_snapshot!("system_inc_simple", items);
}

/// Snapshot: Nested module completion
/// Given File/Path/To/Module.pm in include path, typing `use File::Path::To::Mo`
/// should suggest File::Path::To::Module
#[test]
fn snapshot_nested_module() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(
        &include_path,
        "File/Path/To/Module.pm",
        "package File::Path::To::Module;\n1;\n",
    )
    .expect("creating nested module should succeed");

    let items = run_completion("use File::Path::To::Mo", &[include_path], &[]);

    assert_yaml_snapshot!("nested_module", items);
}

/// Snapshot: Prefix filtering
/// Given DBI.pm, DBD/SQLite.pm, and Moo.pm in include path,
/// typing `use DB` should only suggest DBI and DBD::SQLite (not Moo)
#[test]
fn snapshot_prefix_filtering() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n")
        .expect("creating DBI.pm should succeed");
    create_temp_module(&include_path, "DBD/SQLite.pm", "package DBD::SQLite;\n1;\n")
        .expect("creating DBD::SQLite.pm should succeed");
    create_temp_module(&include_path, "Moo.pm", "package Moo;\n1;\n")
        .expect("creating Moo.pm should succeed");

    let items = run_completion("use DB", &[include_path], &[]);

    assert_yaml_snapshot!("prefix_filtering", items);
}

/// Snapshot: Module at root of include path
/// Given DBI.pm directly in include path root, typing `use DB` should suggest DBI
#[test]
fn snapshot_root_module() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n")
        .expect("creating root module should succeed");

    let items = run_completion("use DB", &[include_path], &[]);

    assert_yaml_snapshot!("root_module", items);
}

/// Snapshot: Multiple modules in same directory
/// Given multiple .pm files in the same directory, all should appear
#[test]
fn snapshot_multiple_modules_same_dir() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Alpha.pm", "package Alpha;\n1;\n")
        .expect("creating Alpha.pm should succeed");
    create_temp_module(&include_path, "Beta.pm", "package Beta;\n1;\n")
        .expect("creating Beta.pm should succeed");
    create_temp_module(&include_path, "Gamma.pm", "package Gamma;\n1;\n")
        .expect("creating Gamma.pm should succeed");

    let items = run_completion("use ", &[include_path], &[]);

    assert_yaml_snapshot!("multiple_modules_same_dir", items);
}

/// Snapshot: Deeply nested module at MAX_DEPTH (5 levels)
/// Module at depth 5 (File/Path/To/Deep/Module.pm) should appear
#[test]
fn snapshot_depth_5_module() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Depth 5: File/Path/To/Deep/Module.pm
    create_temp_module(
        &include_path,
        "File/Path/To/Deep/Module.pm",
        "package File::Path::To::Deep::Module;\n1;\n",
    )
    .expect("creating depth-5 module should succeed");

    let items = run_completion("use File::Path::To::Deep::", &[include_path], &[]);

    assert_yaml_snapshot!("depth_5_module", items);
}

/// Snapshot: Empty prefix returns all modules
/// Typing just `use ` (with space) should return all available modules
#[test]
fn snapshot_empty_prefix() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Apple.pm", "package Apple;\n1;\n")
        .expect("creating Apple.pm should succeed");
    create_temp_module(&include_path, "Banana.pm", "package Banana;\n1;\n")
        .expect("creating Banana.pm should succeed");
    create_temp_module(&include_path, "Cherry.pm", "package Cherry;\n1;\n")
        .expect("creating Cherry.pm should succeed");

    let items = run_completion("use ", &[include_path], &[]);

    assert_yaml_snapshot!("empty_prefix", items);
}

/// Snapshot: Module with numbers in name
/// Math2D and Math3D should both appear when typing `use Math`
/// but only Math2D should appear when typing `use Math2`
#[test]
fn snapshot_module_with_numbers() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Math2D.pm", "package Math2D;\n1;\n")
        .expect("creating Math2D.pm should succeed");
    create_temp_module(&include_path, "Math3D.pm", "package Math3D;\n1;\n")
        .expect("creating Math3D.pm should succeed");

    // Test "Math" prefix
    let items_math = run_completion("use Math", &[include_path.clone()], &[]);
    assert_yaml_snapshot!("module_with_numbers_math", items_math);

    // Test "Math2" prefix
    let items_math2 = run_completion("use Math2", &[include_path], &[]);
    assert_yaml_snapshot!("module_with_numbers_math2", items_math2);
}

/// Snapshot: Multiple include paths with same module name (dedup by label)
/// Module should appear only once even if present in multiple paths
#[test]
fn snapshot_multiple_paths_dedup() {
    let temp_dir1 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir2 = tempfile::tempdir().expect("tempdir should succeed");
    let include_path1 = temp_dir1.path().to_path_buf();
    let include_path2 = temp_dir2.path().to_path_buf();

    // Same module name in both paths
    create_temp_module(&include_path1, "Shared/Module.pm", "package Shared::Module;\n1;\n")
        .expect("creating Module.pm in path1 should succeed");
    create_temp_module(&include_path2, "Shared/Module.pm", "package Shared::Module;\n1;\n")
        .expect("creating Module.pm in path2 should succeed");

    let items = run_completion("use Shared::Mo", &[include_path1, include_path2], &[]);

    assert_yaml_snapshot!("multiple_paths_dedup", items);
}

/// Snapshot: Non-.pm files excluded
/// .pod and .pl files should NOT appear in completions
#[test]
fn snapshot_non_pm_excluded() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Valid/Module.pm", "package Valid::Module;\n1;\n")
        .expect("creating .pm file should succeed");
    create_temp_module(&include_path, "Doc/Module.pod", "=head1 NAME\n\nDoc::Module\n\n=cut\n")
        .expect("creating .pod file should succeed");
    create_temp_module(&include_path, "Script.pl", "#!/usr/bin/perl\nprint 'hello';\n")
        .expect("creating .pl file should succeed");

    let items = run_completion("use Valid", &[include_path], &[]);

    assert_yaml_snapshot!("non_pm_excluded", items);
}

/// Snapshot: Empty paths graceful handling
/// When include_paths and system_inc_paths are empty, no crash and empty results
#[test]
fn snapshot_empty_paths() {
    let items = run_completion("use DB", &[], &[]);

    // Empty completion list for empty paths (no workspace modules)
    assert_yaml_snapshot!("empty_paths", items);
}
