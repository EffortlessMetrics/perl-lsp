use serde_json::json;

mod support;
use support::lsp_client::LspClient;

/// Ensure semantic tokens provide expected ranges and types.
#[test]

fn semantic_tokens_expected_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;

    let uri = "file:///semantic.pl";
    let source = "my $x = 1;\nsub foo { $x }\nfoo();\n";
    client.did_open(uri, "perl", source)?;

    let response =
        client.request("textDocument/semanticTokens/full", json!({"textDocument": {"uri": uri}}));
    let data = response["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response should contain data array")?;

    // Decode LSP semantic tokens relative encoding
    let mut line = 0usize;
    let mut col = 0usize;
    let mut tokens = Vec::new();
    for chunk in data.chunks(5) {
        let dl = chunk[0].as_u64().ok_or("delta line should be u64")? as usize;
        let ds = chunk[1].as_u64().ok_or("delta start should be u64")? as usize;
        let len = chunk[2].as_u64().ok_or("length should be u64")? as usize;
        let token_type = chunk[3].as_u64().ok_or("token type should be u64")? as usize;
        line += dl;
        if dl == 0 {
            col += ds;
        } else {
            col = ds;
        }
        tokens.push((line, col, len, token_type));
    }

    // Legend used by the server (see semantic_tokens.rs) - kept for reference
    let _legend = [
        "namespace",
        "class",
        "function",
        "method",
        "variable",
        "parameter",
        "property",
        "keyword",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "type",
        "macro",
    ];

    // Expected tokens after overlap removal (LSP specification compliant)
    // The longer "sub foo { $x }" function token takes precedence over "sub" keyword
    let expected_non_overlapping = [
        (0, 0, 2, 7),  // my - keyword (index 7)
        (0, 3, 2, 4),  // $x - variable (index 4)
        (0, 6, 1, 12), // = - operator (index 12)
        (0, 8, 1, 10), // 1 - number (index 10)
        (1, 0, 14, 2), // sub foo { $x } - function (index 2) - longer token preferred
        (2, 0, 5, 2),  // foo(); - function (index 2)
    ];

    assert_eq!(tokens.len(), expected_non_overlapping.len(), "semantic token count mismatch");

    for (i, &expected_token) in expected_non_overlapping.iter().enumerate() {
        assert_eq!(tokens[i], expected_token, "token {} mismatch", i);
    }

    client.shutdown()?;
    Ok(())
}
