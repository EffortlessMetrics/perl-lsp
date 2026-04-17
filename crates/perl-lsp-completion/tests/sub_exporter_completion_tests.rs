//! Sub::Exporter completion tests for perl-lsp-completion.
//!
//! Tests cover:
//! - Simple exports array: `use MyModule { exports => [qw(foo bar)] }`
//! - -setup syntax: `use MyModule -setup => { exports => [qw(foo)] }`
//! - Groups/tags: `use MyModule { exports => [...], groups => { default => [...] } }`
//! - Renaming with -as: `func1 => { -as => 'my_func1' }`
//! - MethodCall with HashLiteral: `Module->import({ exports => [qw(foo)] })`

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
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

fn completions_at_end_with_index(code: &str, index: Arc<WorkspaceIndex>) -> Vec<CompletionItem> {
    let provider = parse_provider_with_index(code, index);
    provider.get_completions(code, code.len())
}

fn has_promoted_label(items: &[CompletionItem], label: &str) -> bool {
    items
        .iter()
        .any(|i| i.label == label && i.sort_text.as_deref().is_some_and(|s| s.starts_with("2_")))
}

fn make_sub_exporter_module_index() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/My/SubExporterModule.pm"));
    let code = r#"package My::SubExporterModule;
use Sub::Exporter -setup => {
    exports => [qw(foo bar baz)],
    groups => {
        default => [qw(foo bar)],
        all => [qw(foo bar baz)],
    },
};
sub foo { }
sub bar { }
sub baz { }
1;
"#;
    must(index.index_file(uri, code.to_string()));
    index
}

fn make_sub_exporter_with_renaming_index() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/Module/WithSubExporter.pm"));
    let code = r#"package Module::WithSubExporter;
use Sub::Exporter -setup => {
    exports => [qw(func1 func2)],
};
sub func1 { }
sub func2 { }
1;
"#;
    must(index.index_file(uri, code.to_string()));
    index
}

// ---------------------------------------------------------------------------
// Test 1: Simple exports array - symbols are promoted
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_simple_exports_array_symbols_are_promoted() {
    // `use MyModule { exports => [qw(foo bar)] };`
    // foo and bar should be promoted (sort_text "2_")
    let source = "use My::SubExporterModule { exports => [qw(foo bar)] };\nfo";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    // foo should be promoted because it's in the exports list
    assert!(
        has_promoted_label(&items, "foo"),
        "foo should be promoted (sort_text '2_') because it's in the Sub::Exporter exports; got: {:?}",
        items.iter().find(|i| i.label == "foo").map(|i| &i.sort_text)
    );
}

// ---------------------------------------------------------------------------
// Test 2: Simple exports array - all listed symbols are extracted
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_simple_exports_array_extracts_all_symbols() {
    // `use MyModule { exports => [qw(foo bar)] };`
    // Both foo and bar should appear as completions
    let source = "use My::SubExporterModule { exports => [qw(foo bar)] };\nba";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "bar"),
        "bar should be promoted because it's in the Sub::Exporter exports; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 3: Empty exports yields no symbols
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_empty_exports_yields_no_symbols() {
    // `use MyModule { exports => [] };`
    // No symbols should be promoted
    let source = "use My::SubExporterModule { exports => [] };\nfo";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    // foo should NOT be promoted since exports is empty
    assert!(
        !has_promoted_label(&items, "foo"),
        "foo should NOT be promoted because exports array is empty; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 4: -setup syntax with exports
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_setup_syntax_exports_symbols() {
    // `use MyModule -setup => { exports => [qw(foo)] };`
    // foo should be promoted
    let source = "use My::SubExporterModule -setup => { exports => [qw(foo)] };\nfo";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "foo"),
        "foo should be promoted with -setup syntax; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 5: Groups with default tag
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_groups_default_tag_symbols() {
    // Groups define default => [qw(foo bar)], so foo and bar should be available
    // (Groups are for tag-based imports, so individual symbols still come from exports)
    let source = "use My::SubExporterModule { exports => [qw(foo bar baz)], groups => { default => [qw(foo bar)] } };\nba";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "bar"),
        "bar should be promoted because it's in the exports list; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 6: Renaming with -as
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_renaming_with_as() {
    // `use Module func1 => { -as => 'my_func1' };`
    // my_func1 should appear as a completion
    let source = "use Module::WithSubExporter func1 => { -as => 'my_func1' };\nmy_";
    let index = make_sub_exporter_with_renaming_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "my_func1"),
        "my_func1 should be promoted because it's the renamed export; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 7: MethodCall with HashLiteral
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_method_call_hash_literal() {
    // `Module->import({ exports => [qw(foo)] });`
    // foo should be promoted
    let source =
        "use My::SubExporterModule;\nMy::SubExporterModule->import({ exports => [qw(foo)] });\nfo";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "foo"),
        "foo should be promoted from MethodCall HashLiteral; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 8: No regression - standard qw() imports still work
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_no_regression_standard_qw_imports() {
    // Standard `use Foo qw(bar baz)` still works
    let source = "use List::Util qw(sum min max);\nsu";
    let index = {
        let idx = Arc::new(WorkspaceIndex::new());
        let uri = must(Url::parse("file:///workspace/List/Util.pm"));
        let code = r#"package List::Util;
our @EXPORT_OK = qw(sum min max first);
sub sum { }
sub min { }
sub max { }
sub first { }
1;
"#;
        must(idx.index_file(uri, code.to_string()));
        idx
    };
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "sum"),
        "sum should still be promoted with standard qw() imports; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 9: Multiple exports from different patterns
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_multiple_export_patterns() {
    // Multiple Sub::Exporter imports should all be extracted
    let source = "use My::SubExporterModule { exports => [qw(foo)] };\nuse My::SubExporterModule { exports => [qw(bar)] };\nba";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "bar"),
        "bar should be promoted from second Sub::Exporter use; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 10: baz symbol from exports (not in default group)
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_baz_symbol_from_exports_not_default() {
    // baz is in exports but not in default group
    // It should still be available (exports are always available)
    let source = "use My::SubExporterModule { exports => [qw(foo bar baz)] };\nba";
    let index = make_sub_exporter_module_index();
    let items = completions_at_end_with_index(source, index);

    assert!(
        has_promoted_label(&items, "baz"),
        "baz should be promoted because it's in the exports list; got items: {:?}",
        items.iter().map(|i| format!("{}/{:?}", i.label, i.sort_text)).collect::<Vec<_>>()
    );
}
