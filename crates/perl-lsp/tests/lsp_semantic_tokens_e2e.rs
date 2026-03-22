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

    let response = client
        .request("textDocument/semanticTokens/full", json!({"textDocument": {"uri": uri}}))?;
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

    // Legend indices (must match capabilities_for() advertisement order in perl-lsp-protocol).
    // If indices diverge, clients decode emitted tokenType values to wrong colours (issue #2103).
    // See lsp_semantic_legend_contract_tests.rs for structural validation.
    //   0=namespace  1=type       2=class      3=interface  4=enum     5=enumMember
    //   6=typeParameter  7=function  8=method  9=property  10=macro  11=variable
    //  12=parameter  13=keyword  14=modifier  15=comment  16=string  17=number
    //  18=regexp  19=operator  20=sql_string

    // Expected tokens after overlap removal (LSP specification compliant).
    // We now emit separate non-overlapping tokens for the `sub` keyword, function name,
    // and referenced variable inside the sub body rather than one synthetic combined span.
    let expected_non_overlapping = [
        (0, 0, 2, 13),  // my - keyword (index 13)
        (0, 3, 2, 11),  // $x - variable (index 11)
        (0, 6, 1, 19),  // = - operator (index 19)
        (0, 8, 1, 17),  // 1 - number (index 17)
        (1, 0, 3, 13),  // sub - keyword (index 13)
        (1, 4, 3, 7),   // foo - function (index 7)
        (1, 10, 2, 11), // $x - variable reference (index 11)
        (2, 0, 5, 7),   // foo(); - function (index 7)
    ];

    assert_eq!(tokens.len(), expected_non_overlapping.len(), "semantic token count mismatch");

    for (i, &expected_token) in expected_non_overlapping.iter().enumerate() {
        assert_eq!(tokens[i], expected_token, "token {} mismatch", i);
    }

    client.shutdown()?;
    Ok(())
}
