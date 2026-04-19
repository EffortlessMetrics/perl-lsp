//! Import map tests for `use` statement export awareness and completion.
//!
//! Tests cover:
//! - `extract_import_map` parsing qw() lists
//! - `extract_import_map` unioning multiple use statements
//! - `extract_import_map` skipping pragmas and lowercase modules
//! - Workspace completion promotes imported symbols (sort_text "2_")
//! - Workspace completion downranks explicitly-not-imported symbols (sort_text "4_")

use perl_lsp_completion::CompletionProvider;
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

fn parse_provider_with_index(code: &str, index: Arc<WorkspaceIndex>) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, Some(index))
}

fn make_list_util_index() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/List/Util.pm"));
    let code = r#"package List::Util;
our @EXPORT_OK = qw(sum min max first);
sub sum { }
sub min { }
sub max { }
sub first { }
1;
"#;
    must(index.index_file(uri, code.to_string()));
    index
}

fn make_file_find_index() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/File/Find.pm"));
    let code = r#"package File::Find;
our @EXPORT_OK = qw(find finddepth);
sub find { }
sub finddepth { }
1;
"#;
    must(index.index_file(uri, code.to_string()));
    index
}

fn make_loader_index() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/My/Loader.pm"));
    let code = r#"package My::Loader;
our @EXPORT = qw(load_data);
our @EXPORT_OK = qw(load_data process);
sub load_data { }
sub process { }
1;
"#;
    must(index.index_file(uri, code.to_string()));
    index
}

// ---------------------------------------------------------------------------
// Test 1: extract_import_map parses qw correctly
// ---------------------------------------------------------------------------

#[test]
fn extract_import_map_parses_qw_list() {
    // Source imports sum, min, max from List::Util; cursor is after "su" prefix
    let source = "use List::Util qw(sum min max);\nsu";
    let pos = source.len();
    let index = make_list_util_index();
    let provider = parse_provider_with_index(source, index);

    let items = provider.get_completions(source, pos);

    let sum_item = must_some(
        items
            .iter()
            .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum")),
    );
    assert!(
        sum_item
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "sum should be promoted (sort_text starts with '2_') because it is imported; got: {:?}",
        sum_item.sort_text
    );
}

// ---------------------------------------------------------------------------
// Test 2: extract_import_map unions multiple use statements
// ---------------------------------------------------------------------------

#[test]
fn extract_import_map_unions_multiple_use_stmts() {
    // Two separate use statements for the same module — both symbols imported.
    // Verify sum (from first use) is promoted with prefix "su".
    let source_su = "use List::Util qw(sum);\nuse List::Util qw(min);\nsu";
    let index = make_list_util_index();
    let provider_su = parse_provider_with_index(source_su, Arc::clone(&index));
    let items_su = provider_su.get_completions(source_su, source_su.len());

    let sum_item = must_some(
        items_su
            .iter()
            .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum")),
    );
    assert!(
        sum_item
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "sum should be promoted after union; got: {:?}",
        sum_item.sort_text
    );

    // Verify min (from second use) is also promoted with prefix "mi".
    let source_mi = "use List::Util qw(sum);\nuse List::Util qw(min);\nmi";
    let provider_mi = parse_provider_with_index(source_mi, Arc::clone(&index));
    let items_mi = provider_mi.get_completions(source_mi, source_mi.len());

    let min_item = must_some(
        items_mi
            .iter()
            .find(|i| i.label == "min" || i.insert_text.as_deref() == Some("min")),
    );
    assert!(
        min_item
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "min should be promoted after union; got: {:?}",
        min_item.sort_text
    );
}

// ---------------------------------------------------------------------------
// Test 3: extract_import_map skips pragmas (lowercase module names)
// ---------------------------------------------------------------------------

#[test]
fn extract_import_map_skips_pragmas() {
    // Pragmas are lowercase — none should appear in import map.
    // "say" from use feature qw(say) is NOT a function being imported.
    let source = "use strict;\nuse warnings;\nuse feature qw(say);\n";
    let index = make_list_util_index();
    let provider = parse_provider_with_index(source, index);

    // List::Util symbols are not mentioned — workspace symbols may appear but
    // must NOT be promoted (no import_map entry for List::Util).
    let items = provider.get_completions(source, source.len());

    if let Some(sum_item) = items
        .iter()
        .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum"))
    {
        assert!(
            !sum_item
                .sort_text
                .as_deref()
                .is_some_and(|s| s.starts_with("2_")),
            "sum should NOT be promoted when module is not used; got: {:?}",
            sum_item.sort_text
        );
    }
    // No assertion required if sum is absent — the important thing is no false promotion
}

// ---------------------------------------------------------------------------
// Test 4: workspace completion promotes imported symbols
// ---------------------------------------------------------------------------

#[test]
fn workspace_completion_promotes_imported_symbols() {
    // use List::Util qw(sum min) — sum should be boosted to "2_"
    let source = "use List::Util qw(sum min);\nsu";
    let pos = source.len();
    let index = make_list_util_index();
    let provider = parse_provider_with_index(source, index);

    let items = provider.get_completions(source, pos);

    let sum_item = must_some(
        items
            .iter()
            .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum")),
    );
    assert!(
        sum_item
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "sum (imported) should have sort_text starting '2_'; got: {:?}",
        sum_item.sort_text
    );
    assert!(
        sum_item
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("List::Util")),
        "sum detail should reference 'List::Util'; got: {:?}",
        sum_item.detail
    );
}

// ---------------------------------------------------------------------------
// Test 5: explicit empty import list downranks symbols
// ---------------------------------------------------------------------------

#[test]
fn workspace_completion_downranks_explicit_empty_import() {
    // use List::Util qw() — explicit empty qw list: nothing in namespace.
    // Note: `use Module ()` collapses to empty args in the AST (indistinguishable
    // from `use Module`), but `use Module qw()` is detectable as an explicit
    // empty import list.
    let source = "use List::Util qw();\nsu";
    let pos = source.len();
    let index = make_list_util_index();
    let provider = parse_provider_with_index(source, index);

    let items = provider.get_completions(source, pos);

    // sum may appear but should be downranked to "5_" with detail "not imported".
    // Tier 5 is the lowest priority tier: symbols explicitly not imported via
    // `use Module qw()` should appear after everything else (keywords are also 5_).
    if let Some(sum_item) = items
        .iter()
        .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum"))
    {
        assert!(
            sum_item
                .sort_text
                .as_deref()
                .is_some_and(|s| s.starts_with("5_")),
            "sum with explicit empty import should be downranked (sort_text '5_'); got: {:?}",
            sum_item.sort_text
        );
        assert!(
            sum_item
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("not imported")),
            "sum detail should say 'not imported'; got: {:?}",
            sum_item.detail
        );
    }
    // If sum doesn't appear at all with empty import, that is also acceptable
}

// ---------------------------------------------------------------------------
// Test 6: non-imported symbols from used module stay at normal priority
// ---------------------------------------------------------------------------

#[test]
fn workspace_completion_non_imported_stays_normal_priority() {
    // use List::Util qw(sum) — max is available in the module but was not
    // listed in the import. This is the "Some(_)" arm in the import_map match,
    // which ranks non-imported symbols at tier 4_ (after core builtins at 3_,
    // before explicitly-not-imported empty-qw symbols at 5_).
    let source = "use List::Util qw(sum);\nma";
    let pos = source.len();
    let index = make_list_util_index();
    let provider = parse_provider_with_index(source, index);

    let items = provider.get_completions(source, pos);

    if let Some(max_item) = items
        .iter()
        .find(|i| i.label == "max" || i.insert_text.as_deref() == Some("max"))
    {
        // Should NOT be boosted to tier 2_ (only explicitly imported symbols get 2_)
        assert!(
            !max_item
                .sort_text
                .as_deref()
                .is_some_and(|s| s.starts_with("2_")),
            "max (not imported) should NOT be promoted to 2_; got: {:?}",
            max_item.sort_text
        );
        // Should be at tier 4_ (workspace, not imported with explicit list)
        assert!(
            max_item
                .sort_text
                .as_deref()
                .is_some_and(|s| s.starts_with("4_")),
            "max (not in explicit import list) should be at tier 4_; got: {:?}",
            max_item.sort_text
        );
    }
}

// ---------------------------------------------------------------------------
// Test 7: alternate qw delimiters (qw[], qw{}, qw//, qw<>, qw||, qw!!)
// ---------------------------------------------------------------------------

#[test]
fn extract_import_map_parses_alternate_qw_delimiters() {
    // Test all valid Perl qw delimiters
    let test_cases = vec![
        ("use List::Util qw[sum min];\nsu", "square brackets"),
        ("use List::Util qw{sum min};\nsu", "curly braces"),
        ("use List::Util qw/sum min/;\nsu", "slashes"),
        ("use List::Util qw<sum min>;\nsu", "angle brackets"),
        ("use List::Util qw|sum min|;\nsu", "pipes"),
        ("use List::Util qw!sum min!;\nsu", "exclamation marks"),
    ];

    for (source, description) in test_cases {
        let index = make_list_util_index();
        let provider = parse_provider_with_index(source, Arc::clone(&index));
        let items = provider.get_completions(source, source.len());

        let sum_item = must_some(
            items
                .iter()
                .find(|i| i.label == "sum" || i.insert_text.as_deref() == Some("sum")),
        );
        assert!(
            sum_item
                .sort_text
                .as_deref()
                .is_some_and(|s| s.starts_with("2_")),
            "sum with {} delimiters should be promoted; got: {:?}",
            description,
            sum_item.sort_text
        );
    }
}

#[test]
fn require_import_promotes_symbol() {
    let source = "require My::Loader;\nMy::Loader->import('load_data');\nlo";
    let index = make_loader_index();
    let provider = parse_provider_with_index(source, index);
    let items = provider.get_completions(source, source.len());

    let item = must_some(
        items
            .iter()
            .find(|i| i.label == "load_data" || i.insert_text.as_deref() == Some("load_data")),
    );
    assert!(
        item.sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "load_data should be promoted by require+import; got: {:?}",
        item.sort_text
    );
}

#[test]
fn module_runtime_alias_import_promotes_symbol() {
    let source = "my $mod = use_module('My::Loader');\n$mod->import('load_data');\nlo";
    let index = make_loader_index();
    let provider = parse_provider_with_index(source, index);
    let items = provider.get_completions(source, source.len());

    let item = must_some(
        items
            .iter()
            .find(|i| i.label == "load_data" || i.insert_text.as_deref() == Some("load_data")),
    );
    assert!(
        item.sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "load_data should be promoted by $mod->import after static use_module; got: {:?}",
        item.sort_text
    );
}

#[test]
fn extract_import_map_expands_known_posix_tag() {
    let source = "use File::Find qw(:find);\nfi";
    let index = make_file_find_index();
    let provider = parse_provider_with_index(source, index);
    let items = provider.get_completions(source, source.len());

    let wifexited = must_some(
        items
            .iter()
            .find(|i| i.label == "finddepth" || i.insert_text.as_deref() == Some("finddepth")),
    );
    assert!(
        wifexited
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "finddepth should be promoted as imported via :find; got: {:?}",
        wifexited.sort_text
    );
}

#[test]
fn extract_import_map_supports_mixed_tag_and_symbol_imports() {
    let source = "use File::Find qw(:find find);\nfi";
    let index = make_file_find_index();
    let provider = parse_provider_with_index(source, index);
    let items = provider.get_completions(source, source.len());

    let seek_set = must_some(
        items
            .iter()
            .find(|i| i.label == "find" || i.insert_text.as_deref() == Some("find")),
    );
    assert!(
        seek_set
            .sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("2_")),
        "find should be promoted when mixed with tags; got: {:?}",
        seek_set.sort_text
    );
}
