//! Tests for async/await keyword presence in perl-lexer keyword lists.
//!
//! These tests verify that Perl 5.36+ async/await keywords are properly
//! registered in all required keyword lists per ADR-3538.
//!
//! NOTE: These tests are expected to FAIL until the implementation adds
//! async/await to the keyword lists.

use perl_lexer::{
    KEYWORDS, is_keyword, is_lexer_keyword, is_lsp_completion_keyword, is_parser_lsp_keyword,
};

// ============================================================================
// AC1: async and await must be in KEYWORDS (canonical keyword list)
// ============================================================================

#[test]
fn async_keyword_present_in_keywords() {
    // async must be in the canonical KEYWORDS list
    assert!(is_keyword("async"), "async must be recognized as a keyword via is_keyword()");
}

#[test]
fn await_keyword_present_in_keywords() {
    // await must be in the canonical KEYWORDS list
    assert!(is_keyword("await"), "await must be recognized as a keyword via is_keyword()");
}

// ============================================================================
// AC1: async and await must be in LSP_COMPLETION_KEYWORDS
// ============================================================================

#[test]
fn async_keyword_present_in_lsp_completion_keywords() {
    // async must be in LSP_COMPLETION_KEYWORDS for completion support
    assert!(
        is_lsp_completion_keyword("async"),
        "async must be in LSP_COMPLETION_KEYWORDS for keyword completion"
    );
}

#[test]
fn await_keyword_present_in_lsp_completion_keywords() {
    // await must be in LSP_COMPLETION_KEYWORDS for completion support
    assert!(
        is_lsp_completion_keyword("await"),
        "await must be in LSP_COMPLETION_KEYWORDS for keyword completion"
    );
}

// ============================================================================
// AC1: async and await must be in PARSER_LSP_KEYWORDS
// ============================================================================

#[test]
fn async_keyword_present_in_parser_lsp_keywords() {
    // async must be in PARSER_LSP_KEYWORDS
    assert!(is_parser_lsp_keyword("async"), "async must be in PARSER_LSP_KEYWORDS");
}

#[test]
fn await_keyword_present_in_parser_lsp_keywords() {
    // await must be in PARSER_LSP_KEYWORDS
    assert!(is_parser_lsp_keyword("await"), "await must be in PARSER_LSP_KEYWORDS");
}

// ============================================================================
// AC3: await must be in LEXER_KEYWORDS (async must NOT be)
// Per ADR-3538: async is NOT added to LEXER_KEYWORDS because the parser
// treats `async { }` as a function call, not a keyword token.
// ============================================================================

#[test]
fn await_keyword_present_in_lexer_keywords() {
    // await must be in LEXER_KEYWORDS for lexer-level tokenization
    assert!(
        is_lexer_keyword("await"),
        "await must be in LEXER_KEYWORDS for lexer-level keyword recognition"
    );
}

#[test]
fn async_keyword_must_not_be_in_lexer_keywords() {
    // async must NOT be in LEXER_KEYWORDS because the parser treats
    // `async { }` as a function call (block as first argument).
    // Adding async to LEXER_KEYWORDS would cause incorrect semantic token emission.
    assert!(
        !is_lexer_keyword("async"),
        "async must NOT be in LEXER_KEYWORDS (parser treats async as function call)"
    );
}

// ============================================================================
// Verify keyword lists are still sorted (binary search invariant)
// ============================================================================

#[test]
fn keywords_list_contains_async_await_in_order() {
    // Verify async and await appear in KEYWORDS in sorted order
    let async_pos = KEYWORDS.iter().position(|&k| k == "async");
    let await_pos = KEYWORDS.iter().position(|&k| k == "await");

    assert!(async_pos.is_some(), "async must be present in KEYWORDS");
    assert!(await_pos.is_some(), "await must be present in KEYWORDS");

    // They should be in sorted order (async < await alphabetically)
    let async_idx = async_pos.unwrap();
    let await_idx = await_pos.unwrap();
    assert!(
        async_idx < await_idx,
        "KEYWORDS must be sorted: async (index {}) should come before await (index {})",
        async_idx,
        await_idx
    );
}
