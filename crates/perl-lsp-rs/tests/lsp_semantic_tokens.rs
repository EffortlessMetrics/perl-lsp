use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn semantic_tokens_emit_data() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let uri = "file:///tokens.pl";
    let text = r#"package Foo; my $x = 1; sub bar { return $x } $x = 2; bar();"#;
    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    srv.handle_request(open);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let res = srv.handle_request(req).ok_or("handle_request returned None")?;
    let result = res.result.ok_or("response result is None")?;
    let arr = result["data"].as_array().ok_or("data field is not an array")?;
    assert!(!arr.is_empty(), "semantic tokens should return data");

    // Verify encoding is valid (5-tuples)
    assert_eq!(arr.len() % 5, 0, "semantic tokens must be 5-tuples");

    Ok(())
}

/// Verifies that semantic tokens apply the "modification" modifier (bit 7 = 128) to
/// variables that appear as the direct LHS of a simple assignment (`$x = ...`), while
/// leaving declaration and read tokens unchanged. Closes #5716.
#[test]
fn semantic_tokens_modification_modifier_on_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();

    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
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

    let uri = "file:///test_modification.pl";
    // Line 0: my $x = 1;  -> $x declared (bit 0 = declaration, not bit 7)
    // Line 1: $x = 2;     -> $x LHS write (bit 7 = modification, not bit 0)
    // Line 2: $y = $x;    -> $x RHS read (neither bit 0 nor bit 7)
    let code = "my $x = 1;\n$x = 2;\n$y = $x;";

    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": code
            }
        })),
    };
    srv.handle_request(open);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let res = srv.handle_request(req).ok_or("handle_request returned None")?;
    let result = res.result.ok_or("response result is None")?;
    let data = result["data"].as_array().ok_or("data field is not an array")?;

    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples");

    let data_u32: Vec<u32> = data.iter().filter_map(|v| v.as_u64().map(|u| u as u32)).collect();

    // Decode delta-encoded 5-tuples into (line, col, length, type, modifiers).
    let mut tokens: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
    let mut abs_line = 0u32;
    let mut abs_col = 0u32;
    for chunk in data_u32.chunks(5) {
        if chunk.len() < 5 {
            break;
        }
        let (dl, dc, len, kind, mods) = (chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]);
        if dl > 0 {
            abs_line += dl;
            abs_col = dc;
        } else {
            abs_col += dc;
        }
        tokens.push((abs_line, abs_col, len, kind, mods));
    }

    let has_bit = |mods: u32, bit: u32| (mods & (1 << bit)) != 0;
    const DECLARATION_BIT: u32 = 0;
    const MODIFICATION_BIT: u32 = 7;
    const VARIABLE_TOKEN_TYPE: u32 = 11;

    // kind 11 = "variable" per the legend; len 2 = "$x" (sigil + one-char name).
    // Look up by line so the test is robust against other tokens on the same line.
    let find_var = |target_line: u32| {
        tokens
            .iter()
            .find(|(line, _, len, kind, _)| {
                *line == target_line && *len == 2 && *kind == VARIABLE_TOKEN_TYPE
            })
            .copied()
    };
    let find_var_at = |target_line: u32, target_col: u32| {
        tokens
            .iter()
            .find(|(line, col, len, kind, _)| {
                *line == target_line
                    && *col == target_col
                    && *len == 2
                    && *kind == VARIABLE_TOKEN_TYPE
            })
            .copied()
    };

    // Line 0: declaration - bit 0 set, bit 7 not set
    let (_, _, _, _, mods0) =
        find_var(0).ok_or("no variable token found on line 0 (expected $x declaration)")?;
    assert!(
        has_bit(mods0, DECLARATION_BIT),
        "declaration $x must have 'declaration' modifier (bit 0); modifiers = {mods0}"
    );
    assert!(
        !has_bit(mods0, MODIFICATION_BIT),
        "declaration $x must NOT have 'modification' modifier (bit 7); modifiers = {mods0}"
    );

    // Line 1: assignment LHS - bit 7 set, bit 0 not set
    let (_, _, _, _, mods1) =
        find_var(1).ok_or("no variable token found on line 1 (expected $x assignment LHS)")?;
    assert!(
        !has_bit(mods1, DECLARATION_BIT),
        "assignment-LHS $x must NOT have 'declaration' modifier (bit 0); modifiers = {mods1}"
    );
    assert!(
        has_bit(mods1, MODIFICATION_BIT),
        "assignment-LHS $x must have 'modification' modifier (bit 7); modifiers = {mods1}"
    );

    // Line 2: RHS read - neither bit 0 nor bit 7 set
    let (_, _, _, _, mods2) = find_var_at(2, 5)
        .ok_or("no variable token found at line 2, column 5 (expected $x read)")?;
    assert!(
        !has_bit(mods2, DECLARATION_BIT),
        "read $x must NOT have 'declaration' modifier (bit 0); modifiers = {mods2}"
    );
    assert!(
        !has_bit(mods2, MODIFICATION_BIT),
        "read $x must NOT have 'modification' modifier (bit 7); modifiers = {mods2}"
    );

    Ok(())
}
