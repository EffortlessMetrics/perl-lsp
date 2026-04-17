//! Integration tests for auto-import completion edits (issue #2322).
//!
//! These tests verify that when a user selects a completion item from a module
//! method, an `additionalTextEdits` entry is produced to auto-insert the
//! corresponding `use Module;` statement.

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_provider(code: &str) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, None)
}

fn parse_provider_with_index(code: &str, index: Arc<WorkspaceIndex>) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, Some(index))
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    let provider = parse_provider(code);
    provider.get_completions(code, code.len())
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|i| i.label == label)
}

// ---------------------------------------------------------------------------
// Static module call auto-import (e.g., DBI->connect)
// ---------------------------------------------------------------------------

/// Completing `DBI->` on a file with no existing `use DBI` should generate
/// an auto-import edit that inserts `use DBI;\n` at the correct position.
#[test]
fn dbi_static_method_generates_auto_import_when_not_imported() {
    let code = "use strict;\nuse warnings;\n\nmy $dbh = DBI->";
    let items = completions_at_end(code);

    // We expect some DBI method to have an additional_edit
    let item_with_edit = items.iter().find(|i| !i.additional_edits.is_empty());
    assert!(
        item_with_edit.is_some(),
        "Expected at least one DBI method completion with auto-import edit. \
         Items: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );

    let item = item_with_edit.unwrap();
    assert_eq!(item.additional_edits.len(), 1, "Should have exactly one auto-import edit");
    let (loc, text) = &item.additional_edits[0];
    assert_eq!(text, "use DBI;\n", "Auto-import text should be `use DBI;\\n`");
    // Insertion is zero-width (start == end)
    assert_eq!(loc.start, loc.end, "Auto-import is a zero-width insertion");
}

/// When `use DBI;` is already present, no auto-import edit is generated.
#[test]
fn dbi_static_method_no_auto_import_when_already_imported() {
    let code = "use strict;\nuse DBI;\n\nmy $dbh = DBI->";
    let items = completions_at_end(code);

    let has_any_import_edit = items.iter().any(|i| !i.additional_edits.is_empty());
    assert!(!has_any_import_edit, "Should produce no auto-import edits when DBI already imported");
}

/// The auto-import insertion point is after the last `use` block.
#[test]
fn dbi_auto_import_inserts_after_use_block() {
    // "use strict;\n" = 12 bytes, "use warnings;\n" = 14 bytes => 26 bytes total for the use block
    let code = "use strict;\nuse warnings;\n\nmy $dbh = DBI->";
    let items = completions_at_end(code);

    let item_with_edit = items.iter().find(|i| !i.additional_edits.is_empty());
    assert!(item_with_edit.is_some(), "Expected a DBI method completion with import edit");
    let (loc, _) = &item_with_edit.unwrap().additional_edits[0];
    // "use strict;\n" = 12 + "use warnings;\n" = 14 => offset 26
    assert_eq!(loc.start, 26, "Should insert after the last use statement line");
}

/// When there is no use block at all, insertion point is at offset 0.
#[test]
fn dbi_auto_import_inserts_at_top_when_no_use_block() {
    let code = "my $dbh = DBI->";
    let items = completions_at_end(code);

    let item_with_edit = items.iter().find(|i| !i.additional_edits.is_empty());
    assert!(item_with_edit.is_some(), "Expected a DBI method completion with import edit");
    let (loc, _) = &item_with_edit.unwrap().additional_edits[0];
    assert_eq!(loc.start, 0, "Should insert at offset 0 when no use block exists");
}

// ---------------------------------------------------------------------------
// Static Module->new() auto-import for non-DBI modules
// ---------------------------------------------------------------------------

/// Completing `LWP::UserAgent->` should auto-import `LWP::UserAgent`.
#[test]
fn lwp_static_new_generates_auto_import() {
    let code = "use strict;\n\nmy $ua = LWP::UserAgent->";
    let items = completions_at_end(code);

    // "new" should appear (generic Object methods) with an auto-import edit
    let new_item = find_item(&items, "new");
    assert!(new_item.is_some(), "Expected 'new' in completions for LWP::UserAgent->");
    let new_item = new_item.unwrap();
    assert!(
        !new_item.additional_edits.is_empty(),
        "Expected auto-import edit on 'new' for LWP::UserAgent"
    );
    let (_, text) = &new_item.additional_edits[0];
    assert_eq!(text, "use LWP::UserAgent;\n");
}

/// Completing `LWP::UserAgent->` when already imported produces no edit.
#[test]
fn lwp_no_auto_import_when_already_imported() {
    let code = "use strict;\nuse LWP::UserAgent;\n\nmy $ua = LWP::UserAgent->";
    let items = completions_at_end(code);

    let new_item = find_item(&items, "new");
    if let Some(item) = new_item {
        assert!(
            item.additional_edits.is_empty(),
            "Should have no auto-import edit when LWP::UserAgent already imported"
        );
    }
    // If 'new' is not returned that's also acceptable — no crash is the requirement
}

// ---------------------------------------------------------------------------
// Workspace-indexed method auto-import
// ---------------------------------------------------------------------------

/// A workspace-indexed method from a known package should carry an auto-import
/// edit when the package is not yet imported.
#[test]
fn workspace_method_auto_import_when_not_imported() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/MyApp/Client.pm"));
    must(index.index_file(uri, "package MyApp::Client;\nsub fetch { }\n1;\n".to_string()));

    let code = "use strict;\n\nmy $c = MyApp::Client->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let fetch_item = find_item(&items, "fetch");
    assert!(fetch_item.is_some(), "Expected 'fetch' from workspace index");
    let fetch_item = fetch_item.unwrap();
    assert!(
        !fetch_item.additional_edits.is_empty(),
        "Expected auto-import edit on workspace-indexed method 'fetch'"
    );
    let (_, text) = &fetch_item.additional_edits[0];
    assert_eq!(text, "use MyApp::Client;\n");
}

/// When the package is already imported, no auto-import edit for workspace methods.
#[test]
fn workspace_method_no_auto_import_when_already_imported() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/MyApp/Client.pm"));
    must(index.index_file(uri, "package MyApp::Client;\nsub fetch { }\n1;\n".to_string()));

    let code = "use strict;\nuse MyApp::Client;\n\nmy $c = MyApp::Client->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let fetch_item = find_item(&items, "fetch");
    if let Some(item) = fetch_item {
        assert!(
            item.additional_edits.is_empty(),
            "Should produce no auto-import edit when package already imported"
        );
    }
}

// ---------------------------------------------------------------------------
// CompletionItemKind unused import suppression
// ---------------------------------------------------------------------------

// Ensure the CompletionItemKind import is used in an assertion
#[test]
fn method_completions_have_function_kind() {
    let code = "DBI->";
    let items = completions_at_end(code);
    let method_items: Vec<_> =
        items.iter().filter(|i| i.kind == CompletionItemKind::Function).collect();
    assert!(!method_items.is_empty(), "Expected Function-kind items for DBI methods");
}
