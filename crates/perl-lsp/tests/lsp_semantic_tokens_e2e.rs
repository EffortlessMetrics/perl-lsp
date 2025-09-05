use serde_json::json;

mod support;
use support::lsp_client::LspClient;

/// Ensure semantic tokens provide expected ranges and types.
#[test]
fn semantic_tokens_expected_ranges() {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin);

    let uri = "file:///semantic.pl";
    let source = "my $x = 1;\nsub foo { $x }\nfoo();\n";
    client.did_open(uri, "perl", source);

    let response =
        client.request("textDocument/semanticTokens/full", json!({"textDocument": {"uri": uri}}));
    let data = response["result"]["data"]
        .as_array()
        .expect("semanticTokens response should contain data array");

    // Decode LSP semantic tokens relative encoding
    let mut line = 0usize;
    let mut col = 0usize;
    let mut tokens = Vec::new();
    for chunk in data.chunks(5) {
        let dl = chunk[0].as_u64().unwrap() as usize;
        let ds = chunk[1].as_u64().unwrap() as usize;
        let len = chunk[2].as_u64().unwrap() as usize;
        let token_type = chunk[3].as_u64().unwrap() as usize;
        line += dl;
        if dl == 0 {
            col += ds;
        } else {
            col = ds;
        }
        tokens.push((line, col, len, token_type));
    }

    // Legend used by the server (see semantic_tokens.rs)
    let legend = [
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

    // Expected tokens for the sample document
    let expected = [
        (0, 0, 2, "keyword"),   // my
        (0, 3, 2, "variable"),  // $x
        (0, 6, 1, "operator"),  // =
        (0, 8, 1, "number"),    // 1
        (1, 0, 3, "keyword"),   // sub
        (1, 0, 14, "function"), // sub foo { $x }
        (2, 0, 5, "function"),  // foo();
    ];

    assert_eq!(tokens.len(), expected.len(), "semantic token count mismatch");

    for (i, &(l, c, len, kind)) in expected.iter().enumerate() {
        let idx = legend.iter().position(|&k| k == kind).unwrap();
        assert_eq!(tokens[i], (l, c, len, idx), "token {} mismatch", i);
    }

    client.shutdown();
}
