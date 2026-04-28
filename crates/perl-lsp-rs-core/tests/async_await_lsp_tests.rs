//! Tests for async/await LSP support (Perl 5.36+ experimental keywords).
//!
//! Covers three subsystems (now in perl-lsp-rs-core post-collapse):
//!   1. perl-lexer keyword presence (keyword list registration)
//!   2. Completion provider (keyword_doc + completion items)
//!   3. Semantic token emitter (await → "keyword" token)
//!
//! Per ADR-3538:
//! - async/await in KEYWORDS, LSP_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS
//! - await in LEXER_KEYWORDS; async NOT (parser treats `async {}` as fn call)
//! - await semantic token emitted; async deferred (no async_span in AST)

use perl_lsp_rs_core::providers::{
    completion::{CompletionItem, CompletionItemKind, CompletionProvider},
    semantic_tokens::{collect_semantic_tokens, legend},
};
use perl_parser::Parser;

// ============================================================================
// Helper utilities
// ============================================================================

fn parse_and_provider(code: &str) -> CompletionProvider {
    let ast = Parser::new(code).parse_with_recovery().ast;
    CompletionProvider::new_with_index_and_source(&ast, code, None)
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    let provider = parse_and_provider(code);
    provider.get_completions(code, code.len())
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|i| i.label == label)
}

fn line_col_mapper(text: &str) -> impl Fn(usize) -> (u32, u32) + '_ {
    move |byte: usize| {
        let prefix = &text[..byte.min(text.len())];
        let line = prefix.matches('\n').count() as u32;
        let last_nl = prefix.rfind('\n').map_or(0, |p| p + 1);
        let col = (byte - last_nl) as u32;
        (line, col)
    }
}

fn tokens_for(code: &str) -> Vec<perl_lsp_rs_core::providers::semantic_tokens::EncodedToken> {
    let ast = Parser::new(code).parse_with_recovery().ast;
    let mapper = line_col_mapper(code);
    collect_semantic_tokens(&ast, code, &mapper)
}

fn type_idx(name: &str) -> u32 {
    let leg = legend();
    leg.map.get(name).copied().unwrap_or(u32::MAX)
}

// ============================================================================
// Section 1: Keyword list presence (lexer crate)
// ============================================================================

#[test]
fn async_present_in_keywords() {
    use perl_lexer::is_keyword;
    assert!(is_keyword("async"), "async must be in KEYWORDS");
}

#[test]
fn await_present_in_keywords() {
    use perl_lexer::is_keyword;
    assert!(is_keyword("await"), "await must be in KEYWORDS");
}

#[test]
fn async_present_in_lsp_completion_keywords() {
    use perl_lexer::is_lsp_completion_keyword;
    assert!(is_lsp_completion_keyword("async"), "async must be in LSP_COMPLETION_KEYWORDS");
}

#[test]
fn await_present_in_lsp_completion_keywords() {
    use perl_lexer::is_lsp_completion_keyword;
    assert!(is_lsp_completion_keyword("await"), "await must be in LSP_COMPLETION_KEYWORDS");
}

#[test]
fn async_present_in_parser_lsp_keywords() {
    use perl_lexer::is_parser_lsp_keyword;
    assert!(is_parser_lsp_keyword("async"), "async must be in PARSER_LSP_KEYWORDS");
}

#[test]
fn await_present_in_parser_lsp_keywords() {
    use perl_lexer::is_parser_lsp_keyword;
    assert!(is_parser_lsp_keyword("await"), "await must be in PARSER_LSP_KEYWORDS");
}

#[test]
fn await_present_in_lexer_keywords() {
    use perl_lexer::is_lexer_keyword;
    assert!(is_lexer_keyword("await"), "await must be in LEXER_KEYWORDS for lexer tokenization");
}

#[test]
fn async_not_in_lexer_keywords() {
    use perl_lexer::is_lexer_keyword;
    assert!(
        !is_lexer_keyword("async"),
        "async must NOT be in LEXER_KEYWORDS (parser treats async{{}} as function call)"
    );
}

// ============================================================================
// Section 2: Completion items (keyword completion + documentation)
// ============================================================================

#[test]
fn async_appears_in_completions() {
    let items = completions_at_end("asy");
    assert!(
        items.iter().any(|i| i.label == "async"),
        "async should appear when typing 'asy', got labels: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn await_appears_in_completions() {
    let items = completions_at_end("awai");
    assert!(
        items.iter().any(|i| i.label == "await"),
        "await should appear when typing 'awai', got labels: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn async_has_documentation() {
    let items = completions_at_end("async");
    let item = find_item(&items, "async").expect("async must appear in completions");
    let doc = item.documentation.as_deref().expect("async must have documentation");
    assert!(
        doc.contains("5.36") || doc.to_lowercase().contains("experimental"),
        "async documentation should mention Perl 5.36+ or experimental, got: {doc}"
    );
}

#[test]
fn await_has_documentation() {
    let items = completions_at_end("await");
    let item = find_item(&items, "await").expect("await must appear in completions");
    let doc = item.documentation.as_deref().expect("await must have documentation");
    assert!(
        doc.to_lowercase().contains("future")
            || doc.to_lowercase().contains("suspend")
            || doc.to_lowercase().contains("experimental"),
        "await documentation should mention Future, suspend, or experimental, got: {doc}"
    );
}

#[test]
fn async_completion_kind_is_keyword() {
    let items = completions_at_end("async");
    let item = find_item(&items, "async").expect("async must appear in completions");
    assert!(
        matches!(item.kind, CompletionItemKind::Keyword),
        "async should have CompletionItemKind::Keyword, got: {:?}",
        item.kind
    );
}

#[test]
fn await_completion_kind_is_keyword() {
    let items = completions_at_end("await");
    let item = find_item(&items, "await").expect("await must appear in completions");
    assert!(
        matches!(item.kind, CompletionItemKind::Keyword),
        "await should have CompletionItemKind::Keyword, got: {:?}",
        item.kind
    );
}

// ============================================================================
// Section 3: Semantic tokens (await → keyword token)
// ============================================================================

#[test]
fn await_expression_produces_keyword_token() {
    let code = "await $future";
    let tokens = tokens_for(code);
    let kw_idx = type_idx("keyword");
    let has_keyword = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(
        has_keyword,
        "'await' should produce a keyword semantic token, got tokens: {:?}",
        tokens
    );
}

#[test]
fn await_in_async_context_produces_keyword_token() {
    let code = "async sub fetch_data {\n    await $future;\n}";
    let tokens = tokens_for(code);
    let kw_idx = type_idx("keyword");
    assert!(
        tokens.iter().any(|t| t[3] == kw_idx),
        "'await' inside async sub should produce keyword semantic token"
    );
}

#[test]
fn await_in_assignment_produces_keyword_token() {
    let code = "my $result = await $promise;";
    let tokens = tokens_for(code);
    let kw_idx = type_idx("keyword");
    assert!(
        tokens.iter().any(|t| t[3] == kw_idx),
        "'await' in assignment should produce keyword semantic token"
    );
}

#[test]
fn await_qualified_function_call_does_not_panic() {
    // await::foo() is a qualified function call, not the await keyword
    let code = "await::foo();";
    let _tokens = tokens_for(code);
    // Just verify no panic — parsing a qualified name should not crash
}
