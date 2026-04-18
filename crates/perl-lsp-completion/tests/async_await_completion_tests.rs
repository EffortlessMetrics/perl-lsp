//! Tests for async/await keyword completion and documentation in perl-lsp-completion.
//!
//! These tests verify that Perl 5.36+ async/await keywords have proper
//! completion support and documentation per ADR-3538.

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};

// ----------------------------------------------------------------------------
// Helper utilities (matching patterns from extended_unit_tests.rs)
// ----------------------------------------------------------------------------

fn parse_and_provider(code: &str) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, None)
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    let provider = parse_and_provider(code);
    provider.get_completions(code, code.len())
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|i| i.label == label)
}

// ============================================================================
// AC1: Keyword completion for async and await
// ============================================================================

#[test]
fn async_keyword_appears_in_completions() {
    // async should appear when typing 'as' or 'asy' or 'async'
    let code = "asy";
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|i| i.label == "async"),
        "async keyword should appear in completions when typing 'asy', got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn await_keyword_appears_in_completions() {
    // await should appear when typing 'awai' or 'await'
    let code = "awai";
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|i| i.label == "await"),
        "await keyword should appear in completions when typing 'awai', got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

// ============================================================================
// AC2: Hover documentation for async and await
// ============================================================================

#[test]
fn async_keyword_has_documentation() {
    // async keyword should have documentation explaining it's an experimental
    // Perl 5.36+ feature
    let code = "async";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "async"));
    assert!(
        item.documentation.is_some(),
        "async keyword should have documentation, got: {:?}",
        item.documentation
    );

    let doc = must_some(item.documentation.as_ref());
    // Documentation should mention Perl 5.36+ and experimental
    assert!(
        doc.to_lowercase().contains("5.36") || doc.to_lowercase().contains("experimental"),
        "async documentation should mention Perl 5.36+ or experimental, got: {doc}"
    );
}

#[test]
fn await_keyword_has_documentation() {
    // await keyword should have documentation explaining it suspends
    // execution until a Future completes (Perl 5.36+ experimental)
    let code = "await";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "await"));
    assert!(
        item.documentation.is_some(),
        "await keyword should have documentation, got: {:?}",
        item.documentation
    );

    let doc = must_some(item.documentation.as_ref());
    // Documentation should mention Future or suspend or experimental
    assert!(
        doc.to_lowercase().contains("future")
            || doc.to_lowercase().contains("suspend")
            || doc.to_lowercase().contains("experimental"),
        "await documentation should mention Future, suspend, or experimental, got: {doc}"
    );
}

// ============================================================================
// async and await have correct completion item kinds
// ============================================================================

#[test]
fn async_keyword_completion_kind_is_keyword() {
    // async is not a snippet (no tab stop placeholders), so it should be Keyword kind
    let code = "async";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "async"));
    // kind should be Keyword (not Snippet) since async doesn't have placeholders
    assert!(
        matches!(item.kind, CompletionItemKind::Keyword),
        "async should be CompletionItemKind::Keyword, got: {:?}",
        item.kind
    );
}

#[test]
fn await_keyword_completion_kind_is_keyword() {
    // await is not a snippet (no tab stop placeholders), so it should be Keyword kind
    let code = "await";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "await"));
    // kind should be Keyword (not Snippet) since await doesn't have placeholders
    assert!(
        matches!(item.kind, CompletionItemKind::Keyword),
        "await should be CompletionItemKind::Keyword, got: {:?}",
        item.kind
    );
}

// ============================================================================
// Detail field should indicate "keyword"
// ============================================================================

#[test]
fn async_keyword_detail_is_keyword() {
    let code = "async";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "async"));
    assert!(
        item.detail.as_ref().is_some_and(|d| d == "keyword"),
        "async detail should be Some(\"keyword\"), got: {:?}",
        item.detail
    );
}

#[test]
fn await_keyword_detail_is_keyword() {
    let code = "await";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "await"));
    assert!(
        item.detail.as_ref().is_some_and(|d| d == "keyword"),
        "await detail should be Some(\"keyword\"), got: {:?}",
        item.detail
    );
}
