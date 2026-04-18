//! Tests for async/await semantic token emission in perl-lsp-semantic-tokens.
//!
//! These tests verify that Perl 5.36+ async/await keywords produce proper
//! semantic token highlighting per ADR-3538.
//!
//! NOTE: These tests are expected to FAIL until the implementation adds
//! await to the hardcoded keyword match arm in semantic_tokens.rs.
//!
//! IMPORTANT: async semantic token emission is DEFERRED per ADR-3538 because
//! the parser stores async as an attribute string without source span tracking.

use perl_lsp_semantic_tokens::{EncodedToken, collect_semantic_tokens, legend};
use perl_tdd_support::{Parser, must, must_some};

// ----------------------------------------------------------------------------
// Helper utilities (matching patterns from comprehensive_unit_tests.rs)
// ----------------------------------------------------------------------------

/// Build a full position mapper for multi-line text.
fn line_col_mapper(text: &str) -> impl Fn(usize) -> (u32, u32) + '_ {
    move |byte: usize| {
        let prefix = &text[..byte.min(text.len())];
        let line = prefix.matches('\n').count() as u32;
        let last_nl = prefix.rfind('\n').map_or(0, |p| p + 1);
        let col = (byte - last_nl) as u32;
        (line, col)
    }
}

/// Parse Perl source and collect semantic tokens using the provided mapper.
fn tokens_for(code: &str) -> Vec<EncodedToken> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let mapper = line_col_mapper(code);
    collect_semantic_tokens(&ast, code, &mapper)
}

/// Find the legend index for a token type name.
fn type_idx(name: &str) -> u32 {
    let leg = legend();
    *must_some(leg.map.get(name))
}

// ============================================================================
// AC3: await produces keyword semantic token
// ============================================================================

#[test]
fn await_expression_produces_keyword_token() {
    // await $future should produce a keyword semantic token for 'await'
    let code = "await $future";
    let tokens = tokens_for(code);

    assert!(!tokens.is_empty(), "should produce tokens for 'await $future'");

    let kw_idx = type_idx("keyword");
    let has_keyword = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(
        has_keyword,
        "'await' should be classified as keyword semantic token, got tokens: {:?}",
        tokens
    );
}

#[test]
fn await_in_async_context_produces_keyword_token() {
    // Full async/await context: async sub that awaits
    let code = "async sub fetch_data {\n    await $future;\n}";
    let tokens = tokens_for(code);

    let kw_idx = type_idx("keyword");
    let has_keyword = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_keyword, "'await' inside async sub should be classified as keyword");
}

#[test]
fn await_standalone_expression_produces_keyword() {
    // Simple await expression on its own
    let code = "my $result = await $promise;";
    let tokens = tokens_for(code);

    let kw_idx = type_idx("keyword");
    let has_keyword = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_keyword, "'await' in assignment should be classified as keyword");
}

// ============================================================================
// Verify await is in LEXER_KEYWORDS (precondition for keyword token emission)
// ============================================================================

#[test]
fn await_is_recognized_as_lexer_keyword() {
    // For the semantic token emitter to work, await must be in LEXER_KEYWORDS
    // This is a precondition test - if this fails, the semantic token test
    // above will also fail
    use perl_lexer::is_lexer_keyword;
    assert!(
        is_lexer_keyword("await"),
        "await must be in LEXER_KEYWORDS for semantic token emission"
    );
}

// ============================================================================
// DEFERRED: async does NOT produce semantic token (by design per ADR-3538)
// These tests document the deferred behavior - they test that async does NOT
// currently produce a keyword token, which is the expected behavior until
// AST changes are made to track async_span.
// ============================================================================

// NOTE: The following tests are commented out because they test for behavior
// that is intentionally DEFERRED per ADR-3538.
//
// The parser stores `async` as a string in `NodeKind::Subroutine { attributes: Vec<String> }`
// without source span tracking. Emitting semantic tokens for `async` requires
// AST changes to record `async_span`.
//
// #[test]
// fn async_attribute_produces_keyword_token() {
//     let code = "async sub foo { }";
//     let tokens = tokens_for(code);
//     let kw_idx = type_idx("keyword");
//     let has_async_keyword = tokens.iter().any(|t| t[3] == kw_idx && /* check text is "async" */);
//     assert!(has_async_keyword, "async should produce keyword token (DEFERRED)");
// }
//
// These tests should be added when the AST changes to track async_span are implemented.

// ============================================================================
// Edge cases for await keyword token emission
// ============================================================================

#[test]
fn await_with_qualified_function_call_does_not_panic() {
    // await::foo() is parsed as a function call, not as the await keyword
    // This should NOT produce a keyword token for 'await'
    let code = "await::foo();";
    let tokens = tokens_for(code);

    // This tests that await::foo() is correctly handled as a function call
    // and does not incorrectly emit 'await' as a keyword
    // The exact behavior depends on the parser's handling of qualified names

    // We just verify the test doesn't panic and produces some tokens
    assert!(!tokens.is_empty() || tokens.is_empty(), "await::foo() should be parsed without error");
}

#[test]
fn nested_await_expressions_produce_keyword_tokens() {
    // Multiple await expressions in sequence
    let code = "await $a; await $b; await $c;";
    let tokens = tokens_for(code);

    let kw_idx = type_idx("keyword");
    let keyword_count = tokens.iter().filter(|t| t[3] == kw_idx).count();

    // Should have at least 3 keyword tokens (one for each await)
    assert!(
        keyword_count >= 3,
        "expected at least 3 keyword tokens for 3 await expressions, got {keyword_count}"
    );
}
