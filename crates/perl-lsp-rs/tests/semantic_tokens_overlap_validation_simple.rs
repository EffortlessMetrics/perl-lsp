//! Semantic-token invariant validation through the LIVE canonical renderer.
//!
//! These tests exercise `textDocument/semanticTokens/full` end-to-end (the
//! `collect_semantic_tokens` path) rather than any standalone provider, so the
//! invariants they pin — non-empty generation, positive lengths, same-line
//! non-overlap, idempotence, UTF-8 sanity — hold for the tokens editors
//! actually receive. (#3388: single canonical renderer; the legacy AST-only
//! `SemanticTokensProvider` is retired.)
//!
//! Labels: tests:semantic-tokens, tests:mutation-hardening

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Spin up a fresh LspServer, open `uri` with `source`, request full semantic
/// tokens, and return decoded absolute (line, col, len, type) tuples.
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

/// Assert the core wire invariants for a decoded token stream: positive
/// lengths and no same-line overlap (each token ends at or before the next
/// token on that line begins — the invariant this file is named for).
fn assert_stream_invariants(tokens: &[(u32, u32, u32, u32)], label: &str) {
    for (i, tok) in tokens.iter().enumerate() {
        assert!(tok.2 > 0, "{label}: token {i} must have positive length, got {tok:?}");
    }
    for pair in tokens.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.0 == b.0 {
            assert!(a.1 + a.2 <= b.1, "{label}: same-line tokens overlap: {a:?} then {b:?}");
        }
    }
}

#[test]
fn live_tokens_basic_generation() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = get_tokens("file:///overlap_basic.pl", "my $x = 42;\n")?;
    assert!(!tokens.is_empty(), "variable declaration should produce semantic tokens");
    assert_stream_invariants(&tokens, "basic");
    Ok(())
}

#[test]
fn live_tokens_complex_code_no_overlap() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"package My::Module;

use strict;
use warnings;

sub process_data {
    my ($self, %args) = @_;
    my @items = @{ $args{items} // [] };
    foreach my $item (@items) {
        next unless defined $item;
        $self->{count}++;
    }
    return $self->{count};
}

1;
"#;
    let tokens = get_tokens("file:///overlap_complex.pl", code)?;
    assert!(!tokens.is_empty(), "complex code should produce semantic tokens");
    assert_stream_invariants(&tokens, "complex");
    Ok(())
}

#[test]
fn live_tokens_utf8_lengths_sane() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $greeting = \"héllo wörld — ünïcode\";\nmy $emoji = \"🐪 camel\";\n";
    let tokens = get_tokens("file:///overlap_utf8.pl", code)?;
    assert!(!tokens.is_empty(), "UTF-8 code should produce semantic tokens");
    assert_stream_invariants(&tokens, "utf8");
    for tok in &tokens {
        assert!(tok.2 < 100, "UTF-16 token length should stay reasonable, got {tok:?}");
    }
    Ok(())
}

#[test]
fn live_tokens_idempotent_across_requests() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub greet { my ($name) = @_; return \"hi $name\"; }\n";
    let first = get_tokens("file:///overlap_idem.pl", code)?;
    let second = get_tokens("file:///overlap_idem.pl", code)?;
    assert_eq!(first, second, "identical source must decode to identical token streams");
    Ok(())
}

#[test]
fn live_tokens_nested_structures() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my %config = (
    db => {
        host => "localhost",
        opts => [1, 2, 3],
    },
);
for my $key (sort keys %config) {
    if (ref $config{$key} eq 'HASH') {
        while (my ($k, $v) = each %{ $config{$key} }) {
            print "$k\n";
        }
    }
}
"#;
    let tokens = get_tokens("file:///overlap_nested.pl", code)?;
    assert!(!tokens.is_empty(), "nested structures should produce semantic tokens");
    assert_stream_invariants(&tokens, "nested");
    Ok(())
}
