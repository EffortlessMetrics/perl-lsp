//! Tests that `isa` infix operator (Perl 5.32+) is classified as a keyword
//! in semantic token responses. Closes #778.
//!
//! `isa` is a LEXER_KEYWORD in `perl-lexer`, so it arrives in
//! `collect_semantic_tokens` as `TokenType::Keyword("isa")`.  Before the fix,
//! the `Keyword` arm's explicit match list does not include `"isa"`, so the
//! token is silently dropped (`continue`).  After the fix, `"isa"` is included
//! and emits token-type `"keyword"` (legend index 13).
//!
//! `cmp` has the same shape — it is also a LEXER_KEYWORD that was missing from
//! the match arm — so it is used as a regression anchor here.

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Helper: spin up a fresh LspServer, open `uri` with `source`, request
/// full semantic tokens, and return the decoded (line, col, len, type) tuples.
fn get_tokens(
    uri: &str,
    source: &str,
) -> Result<Vec<(u32, u32, u32, u32)>, Box<dyn std::error::Error>> {
    let srv = LspServer::new();

    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities": {}})),
    };
    srv.handle_request(init);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        })),
    };
    srv.handle_request(open);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };

    let res = srv.handle_request(req).ok_or("handle_request returned None")?;
    let result = res.result.ok_or("response result is None")?;
    let data = result["data"].as_array().ok_or("data field is not an array")?;

    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples");

    let data_u32: Vec<u32> = data.iter().filter_map(|v| v.as_u64().map(|u| u as u32)).collect();

    // Decode delta-encoded 5-tuples into absolute (line, col, len, type)
    let mut tokens: Vec<(u32, u32, u32, u32)> = Vec::new();
    let mut abs_line = 0u32;
    let mut abs_col = 0u32;
    for chunk in data_u32.chunks(5) {
        if chunk.len() < 5 {
            break;
        }
        let (dl, dc, len, kind, _mods) = (chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]);
        if dl > 0 {
            abs_line += dl;
            abs_col = dc;
        } else {
            abs_col += dc;
        }
        tokens.push((abs_line, abs_col, len, kind));
    }

    Ok(tokens)
}

/// `isa` in `if ($x isa Foo) { }` must be classified as token type `keyword` (index 13).
///
/// Before fix: `isa` falls through the `Keyword` arm as an unrecognised keyword
/// and is skipped (`continue`), so no keyword token is emitted for the `isa` span.
/// After fix: `"isa"` is included in the match arm and emits type 13.
///
/// # Token positions in `if ($x isa Foo) { }\n`
/// ```text
/// col:  0123456789012345678
///       if ($x isa Foo) { }
/// ```
/// - `if`  → col 0, len 2
/// - `$x`  → col 4, len 2
/// - `isa` → col 7, len 3
/// - `Foo` → col 11, len 3
#[test]
fn test_isa_infix_operator_classified_as_keyword() -> Result<(), Box<dyn std::error::Error>> {
    // Legend indices (must match capabilities_for() in perl-lsp-protocol):
    // 13 = keyword
    const KEYWORD_TYPE: u32 = 13;

    let source = "if ($x isa Foo) { }\n";
    let tokens = get_tokens("file:///isa_test.pl", source)?;

    // `isa` occupies columns 7..10 on line 0 (3 characters)
    let isa_token =
        tokens.iter().find(|&&(line, col, len, _kind)| line == 0 && col == 7 && len == 3);

    let isa_token = isa_token.ok_or(
        "no token found at line 0, col 7, len 3 (expected 'isa'); \
         'isa' must be added to the keyword match arm in semantic_tokens.rs",
    )?;

    assert_eq!(
        isa_token.3, KEYWORD_TYPE,
        "'isa' at line 0 col 7 must have token type {KEYWORD_TYPE} (keyword), \
         got {} — add 'isa' to the keyword match arm in semantic_tokens.rs",
        isa_token.3
    );

    Ok(())
}

/// Regression: `cmp` (a LEXER_KEYWORD like `isa`) must also be classified as keyword.
///
/// `cmp` is in `LEXER_KEYWORDS` so it arrives as `TokenType::Keyword("cmp")`.
/// It should be emitted with token type `keyword` (13).
///
/// # Token positions in `$a cmp $b\n`
/// ```text
/// col: 0123456789
///      $a cmp $b
/// ```
/// - `$a`  → col 0, len 2
/// - `cmp` → col 3, len 3
/// - `$b`  → col 7, len 2
#[test]
fn test_cmp_keyword_classified_as_keyword() -> Result<(), Box<dyn std::error::Error>> {
    const KEYWORD_TYPE: u32 = 13;

    let source = "$a cmp $b\n";
    let tokens = get_tokens("file:///cmp_test.pl", source)?;

    // cmp is at col 3, len 3 on line 0
    let cmp_tok = tokens.iter().find(|&&(line, col, len, _)| line == 0 && col == 3 && len == 3);

    let cmp_tok = cmp_tok.ok_or(
        "no token at line 0 col 3 len 3 (expected 'cmp'); \
         'cmp' must also be in the keyword match arm",
    )?;

    assert_eq!(
        cmp_tok.3, KEYWORD_TYPE,
        "'cmp' must be token type {KEYWORD_TYPE} (keyword), got {}",
        cmp_tok.3
    );

    Ok(())
}

/// `$x` in `if ($x isa Foo) { }` must NOT be classified as keyword.
/// It should remain a variable (type 11).
#[test]
fn test_isa_does_not_contaminate_adjacent_variable() -> Result<(), Box<dyn std::error::Error>> {
    const KEYWORD_TYPE: u32 = 13;
    const VARIABLE_TYPE: u32 = 11;

    let source = "if ($x isa Foo) { }\n";
    let tokens = get_tokens("file:///isa_no_contaminate.pl", source)?;

    // $x is at col 4, len 2 (variable)
    let x_tok = tokens.iter().find(|&&(line, col, len, _)| line == 0 && col == 4 && len == 2);
    let x_tok = x_tok.ok_or("no token at line 0 col 4 len 2 (expected '$x')")?;
    assert_eq!(
        x_tok.3, VARIABLE_TYPE,
        "'$x' must be variable (type {VARIABLE_TYPE}), got {}",
        x_tok.3
    );

    // `Foo` should NOT be classified as keyword (it's a bareword class name)
    let foo_tok = tokens.iter().find(|&&(line, col, len, _)| line == 0 && col == 11 && len == 3);
    if let Some(foo) = foo_tok {
        assert_ne!(
            foo.3, KEYWORD_TYPE,
            "'Foo' must NOT be classified as keyword (type {KEYWORD_TYPE}), got {}",
            foo.3
        );
    }

    Ok(())
}
