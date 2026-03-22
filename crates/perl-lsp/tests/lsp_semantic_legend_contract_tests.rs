/// Legend contract test: the token type indices emitted in semantic token responses
/// must resolve to the correct type names when looked up in the advertised legend.
///
/// This test catches legend desynchronization: if the server's internal legend
/// (`legend()` in `semantic_tokens.rs`) has a different ordering than what the server
/// advertises to clients during `initialize`, then every token will be rendered with
/// the wrong colour in every LSP client.
///
/// The test:
///   1. Sends `initialize` and captures the `semanticTokensProvider.legend.tokenTypes` array.
///   2. Opens a document with well-known constructs (keyword, variable, operator, number, function).
///   3. Requests `textDocument/semanticTokens/full`.
///   4. For each token, indexes into the ADVERTISED legend with the emitted `tokenType` field.
///   5. Asserts the resolved type name equals the expected type for that construct.
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

type BoxError = Box<dyn std::error::Error>;

fn send_msg(stdin: &mut std::process::ChildStdin, body: &str) -> Result<(), BoxError> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(body.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

fn recv_msg(reader: &mut BufReader<std::process::ChildStdout>) -> Result<serde_json::Value, BoxError> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let len = content_length.ok_or("Content-Length header missing")?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn recv_until_id(
    reader: &mut BufReader<std::process::ChildStdout>,
    id: u64,
) -> Result<serde_json::Value, BoxError> {
    loop {
        let msg = recv_msg(reader)?;
        if msg.get("id") == Some(&serde_json::json!(id)) {
            return Ok(msg);
        }
        // Discard notifications / other messages
    }
}

#[test]
fn semantic_token_indices_match_advertised_legend() -> Result<(), BoxError> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

    // --- initialize ---
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {
                "general": { "positionEncodings": ["utf-16"] }
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &init_req)?;
    let init_resp = recv_until_id(&mut reader, 1)?;

    // Extract the advertised token type legend from the initialize response.
    let advertised_legend: Vec<String> = init_resp["result"]["capabilities"]
        ["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .ok_or("semanticTokensProvider.legend.tokenTypes missing from initialize response")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();

    // --- initialized notification ---
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    })
    .to_string();
    send_msg(&mut stdin, &initialized)?;

    // --- didOpen ---
    let source = "my $x = 1;\nsub foo { $x }\nfoo();\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    // --- semanticTokens/full ---
    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let data = sem_resp["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;

    // Decode delta-encoded tokens into (line, col, len, type_name) tuples.
    let mut line = 0usize;
    let mut col = 0usize;
    let mut decoded: Vec<(usize, usize, usize, String)> = Vec::new();
    for chunk in data.chunks(5) {
        let dl = chunk[0].as_u64().ok_or("delta_line not u64")? as usize;
        let ds = chunk[1].as_u64().ok_or("delta_start not u64")? as usize;
        let len = chunk[2].as_u64().ok_or("length not u64")? as usize;
        let type_idx = chunk[3].as_u64().ok_or("token_type not u64")? as usize;

        line += dl;
        col = if dl == 0 { col + ds } else { ds };

        let type_name = advertised_legend
            .get(type_idx)
            .cloned()
            .unwrap_or_else(|| format!("OUT_OF_RANGE({})", type_idx));
        decoded.push((line, col, len, type_name));
    }

    // Expected: for each known construct, the advertised legend must resolve to the
    // correct semantic type name. These are the positions in "my $x = 1;\nsub foo { $x }\nfoo();\n"
    let expected: &[((usize, usize, usize), &str)] = &[
        ((0, 0, 2), "keyword"),   // my
        ((0, 3, 2), "variable"),  // $x
        ((0, 6, 1), "operator"),  // =
        ((0, 8, 1), "number"),    // 1
        ((1, 0, 3), "keyword"),   // sub
        ((1, 4, 3), "function"),  // foo
        ((1, 10, 2), "variable"), // $x reference
        ((2, 0, 5), "function"),  // foo()
    ];

    assert_eq!(
        decoded.len(),
        expected.len(),
        "token count mismatch — decoded tokens: {:?}",
        decoded
    );

    for (i, &((exp_line, exp_col, exp_len), exp_type)) in expected.iter().enumerate() {
        let (act_line, act_col, act_len, ref act_type) = decoded[i];
        assert_eq!(
            (act_line, act_col, act_len),
            (exp_line, exp_col, exp_len),
            "token {} position mismatch",
            i
        );
        assert_eq!(
            act_type, exp_type,
            "token {} at ({},{}) len={}: advertised legend resolved to '{}' but expected '{}'",
            i, exp_line, exp_col, exp_len, act_type, exp_type
        );
    }

    // --- shutdown ---
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    })
    .to_string();
    send_msg(&mut stdin, &shutdown_req)?;
    let _ = recv_until_id(&mut reader, 3)?;

    let exit_notif =
        serde_json::json!({"jsonrpc": "2.0", "method": "exit", "params": null}).to_string();
    send_msg(&mut stdin, &exit_notif)?;

    let _ = child.wait();
    Ok(())
}
