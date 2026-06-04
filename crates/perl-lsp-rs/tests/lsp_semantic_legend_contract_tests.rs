/// Legend contract tests: the token type indices and modifier bitmasks emitted in semantic
/// token responses must round-trip correctly through the advertised legend.
///
/// Three bugs were fixed together in PR #2772 (issue #2103):
///   1. Legend ordering mismatch — types list reordered to match capability advertisement.
///   2. `sql_string` not advertised — was emitted at index 15 (old legend) but decoded as
///      "comment" by clients using the advertised legend (which had no sql_string entry).
///   3. `defaultLibrary` modifier bitmask — was `8` (bit 3) but advertised at bit 9 (512),
///      causing `$_`, `%ENV`, `@_` to render as `static` instead of library-defined.
///
/// Each test here:
///   1. Sends `initialize` and captures both `tokenTypes` and `tokenModifiers` legend arrays.
///   2. Opens a document with a specific construct.
///   3. Requests `textDocument/semanticTokens/full`.
///   4. Verifies emitted indices and bitmasks decode correctly through the ADVERTISED legend.
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

fn recv_msg(
    reader: &mut BufReader<std::process::ChildStdout>,
) -> Result<serde_json::Value, BoxError> {
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

fn decoded_semantic_tokens(
    response: &serde_json::Value,
) -> Result<Vec<(usize, usize, usize, usize, u32)>, BoxError> {
    let data = response["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;

    let mut line = 0usize;
    let mut col = 0usize;
    let mut decoded = Vec::new();
    let chunks = data.chunks_exact(5);
    if !chunks.remainder().is_empty() {
        return Err("semanticTokens data length must be divisible by 5".into());
    }

    for chunk in chunks {
        let [delta_line, delta_start, length, token_type, token_modifiers] = chunk else {
            continue;
        };
        let dl = delta_line.as_u64().ok_or("delta_line not u64")? as usize;
        let ds = delta_start.as_u64().ok_or("delta_start not u64")? as usize;
        let len = length.as_u64().ok_or("length not u64")? as usize;
        let type_idx = token_type.as_u64().ok_or("token_type not u64")? as usize;
        let mods = token_modifiers.as_u64().ok_or("token_modifiers not u64")? as u32;

        line += dl;
        col = if dl == 0 { col + ds } else { ds };
        decoded.push((line, col, len, type_idx, mods));
    }
    Ok(decoded)
}

#[test]
fn semantic_token_label_type_decodes_for_perl_labels() -> Result<(), BoxError> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

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

    let advertised_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .ok_or("semanticTokensProvider.legend.tokenTypes missing from initialize response")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

    let label_idx = advertised_legend
        .iter()
        .position(|token_type| token_type == "label")
        .ok_or("SemanticTokenTypes.label must be advertised once provider support exists")?;

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}).to_string(),
    )?;

    let source = "OUTER: while ($x) { last OUTER; }\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///semantic_label_contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///semantic_label_contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let labels = decoded_semantic_tokens(&sem_resp)?
        .into_iter()
        .filter_map(|(line, col, len, type_idx, mods)| {
            (type_idx == label_idx).then_some((line, col, len, mods))
        })
        .collect::<Vec<_>>();

    assert!(
        labels.contains(&(0, 0, 5, 1)),
        "labeled statement declaration should decode as label+declaration; labels={labels:?}, response={sem_resp:?}"
    );
    assert!(
        labels.contains(&(0, 25, 5, 0)),
        "loop-control label reference should decode as label without declaration modifier; labels={labels:?}, response={sem_resp:?}"
    );

    let range_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/semanticTokens/range",
        "params": {
            "textDocument": { "uri": "file:///semantic_label_contract_test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 33 }
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &range_req)?;
    let range_resp = recv_until_id(&mut reader, 3)?;
    let range_labels = decoded_semantic_tokens(&range_resp)?
        .into_iter()
        .filter_map(|(line, col, len, type_idx, mods)| {
            (type_idx == label_idx).then_some((line, col, len, mods))
        })
        .collect::<Vec<_>>();

    assert!(
        range_labels.contains(&(0, 0, 5, 1)),
        "range semantic tokens should decode labeled statement declarations through the advertised label index; labels={range_labels:?}, response={range_resp:?}"
    );
    assert!(
        range_labels.contains(&(0, 25, 5, 0)),
        "range semantic tokens should decode loop-control label references through the advertised label index; labels={range_labels:?}, response={range_resp:?}"
    );

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}).to_string(),
    )?;
    let _ = recv_until_id(&mut reader, 4)?;
    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}).to_string(),
    )?;
    let _ = child.wait();
    Ok(())
}

#[test]
fn semantic_token_result_indexes_stay_within_advertised_legend_bounds() -> Result<(), BoxError> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

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

    let advertised_type_legend =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .ok_or("semanticTokensProvider.legend.tokenTypes missing from initialize response")?;
    let advertised_modifier_legend = init_resp["result"]["capabilities"]["semanticTokensProvider"]
        ["legend"]["tokenModifiers"]
        .as_array()
        .ok_or("semanticTokensProvider.legend.tokenModifiers missing from initialize response")?;
    let type_count = advertised_type_legend.len() as u64;
    let allowed_modifier_mask = if advertised_modifier_legend.len() >= u64::BITS as usize {
        u64::MAX
    } else {
        (1u64 << advertised_modifier_legend.len()) - 1
    };

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}).to_string(),
    )?;

    let source = "my $dbh = undef;\n$dbh->prepare(\"SELECT 1\");\nsub f { return $_; }\nOUTER: while ($x) { last OUTER; }\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///semantic_bounds_contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///semantic_bounds_contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let data = sem_resp["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;
    assert_eq!(
        data.len() % 5,
        0,
        "semanticTokens data must be encoded as 5-uinteger tuples: {data:?}"
    );

    for (token_idx, chunk) in data.chunks(5).enumerate() {
        let type_idx = chunk[3].as_u64().ok_or("token_type not u64")?;
        assert!(
            type_idx < type_count,
            "semantic token {token_idx} type index {type_idx} is outside advertised legend length {type_count}"
        );

        let modifier_bits = chunk[4].as_u64().ok_or("token_modifiers not u64")?;
        assert_eq!(
            modifier_bits & !allowed_modifier_mask,
            0,
            "semantic token {token_idx} modifier bits {modifier_bits:#b} exceed advertised modifier legend mask {allowed_modifier_mask:#b}"
        );
    }

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}).to_string(),
    )?;
    let _ = recv_until_id(&mut reader, 3)?;
    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}).to_string(),
    )?;
    let _ = child.wait();
    Ok(())
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
    let advertised_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
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

/// Contract test for the `sql_string` token type (bug 2 of PR #2772).
///
/// Before the fix: `sql_string` was not in the advertised `tokenTypes` array. The server
/// emitted index 15 (its position in the old internal legend). Clients looked up index 15
/// in the advertised legend and decoded it as "comment" (index 15 = "comment" in the
/// 20-item advertised list). SQL strings appeared as comments in every LSP client.
///
/// After the fix: `sql_string` is at index 20 in both the internal legend and the
/// advertised legend, so it decodes correctly.
#[test]
fn sql_string_index_decodes_correctly_via_advertised_legend() -> Result<(), BoxError> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

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

    let advertised_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .ok_or("tokenTypes missing from initialize response")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

    // Verify sql_string is present in the advertised legend at all — if absent, every
    // sql_string token would decode to a wrong type or OUT_OF_RANGE.
    let sql_string_advertised_idx = advertised_legend
        .iter()
        .position(|s| s == "sql_string")
        .ok_or("sql_string is not in the advertised tokenTypes legend")?;

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}).to_string(),
    )?;

    // DBI pattern: $dbh->prepare("SELECT ...") — the string arg should be sql_string.
    let source = "my $dbh = undef;\n$dbh->prepare(\"SELECT 1\");\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///sql_contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///sql_contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let data = sem_resp["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;

    // Decode all tokens and collect their type names via the ADVERTISED legend.
    // We scan every token — position-independent — to find any that resolve to "sql_string".
    // This is robust against parser changes that shift token positions.
    let mut decoded_types: Vec<String> = Vec::new();
    for chunk in data.chunks(5) {
        let type_idx = chunk[3].as_u64().ok_or("token_type not u64")? as usize;
        let type_name = advertised_legend
            .get(type_idx)
            .cloned()
            .unwrap_or_else(|| format!("OUT_OF_RANGE({})", type_idx));
        decoded_types.push(type_name);
    }

    // At least one token must decode to "sql_string" via the ADVERTISED legend.
    // Before the fix: sql_string was not in the advertised legend, so the emitted index
    // (15 in the old internal legend) decoded as "comment" (index 15 in the advertised list).
    let found_sql_string = decoded_types.iter().any(|t| t == "sql_string");
    assert!(
        found_sql_string,
        "No token decoded to 'sql_string' via the advertised legend for DBI prepare call \
         '$dbh->prepare(\"SELECT 1\")'. Decoded types: {:?}. \
         sql_string is at advertised index {}. \
         This is the legendary bug: if sql_string is not in the advertised legend, \
         the emitted index decodes as a wrong type (e.g. 'comment').",
        decoded_types, sql_string_advertised_idx
    );

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}).to_string(),
    )?;
    let _ = recv_until_id(&mut reader, 3)?;
    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}).to_string(),
    )?;
    let _ = child.wait();
    Ok(())
}

/// Contract test for the `defaultLibrary` modifier bitmask (bug 3 of PR #2772).
///
/// Before the fix: the server emitted bitmask `8` for special variables like `$_`.
/// The advertised modifier legend had `defaultLibrary` at bit 9 (bitmask 512). So
/// clients decoded bit 3 (bitmask 8) as `static` — special variables appeared as
/// static variables rather than library-defined builtins.
///
/// After the fix: the server emits bitmask `512`, which decodes as `defaultLibrary`
/// from the advertised modifier legend.
#[test]
fn default_library_modifier_bitmask_decodes_correctly_via_advertised_legend() -> Result<(), BoxError>
{
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

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

    let advertised_type_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .ok_or("tokenTypes missing from initialize response")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

    let advertised_mod_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenModifiers"]
            .as_array()
            .ok_or("tokenModifiers missing from initialize response")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

    // Verify defaultLibrary is in the advertised modifier legend and note its bit position.
    let default_library_bit = advertised_mod_legend
        .iter()
        .position(|s| s == "defaultLibrary")
        .ok_or("defaultLibrary is not in the advertised tokenModifiers legend")?;
    let expected_bitmask = 1u32 << default_library_bit;

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}).to_string(),
    )?;

    // `$_` is the canonical special variable — always gets the defaultLibrary modifier.
    // Wrap in a sub so the parser produces a Variable AST node that the semantic token
    // walker visits. Bare `print $_;` at top level may not produce a Variable node.
    let source = "sub f { return $_; }\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///modifier_contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///modifier_contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let data = sem_resp["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;

    let var_idx = advertised_type_legend
        .iter()
        .position(|s| s == "variable")
        .ok_or("variable not in advertised tokenTypes legend")? as u64;

    // Scan all tokens for a variable token with the defaultLibrary bit set.
    // Position-independent: we don't assume where $_ lands — we just look for any
    // variable that has the defaultLibrary modifier set.
    let mut all_variable_modifiers: Vec<u32> = Vec::new();
    let mut found_default_library_variable = false;
    for chunk in data.chunks(5) {
        let type_idx = chunk[3].as_u64().ok_or("token_type not u64")?;
        let modifier_bits = chunk[4].as_u64().ok_or("token_modifiers not u64")? as u32;

        if type_idx == var_idx {
            all_variable_modifiers.push(modifier_bits);
            if modifier_bits & expected_bitmask != 0 {
                found_default_library_variable = true;
            }
        }
    }

    assert!(
        found_default_library_variable,
        "No variable token with the defaultLibrary modifier bit set was found for 'sub f {{ return $_; }}\n'. \
         defaultLibrary is at bit {} in the advertised legend (expected bitmask {:#b}). \
         All variable token modifier values seen: {:?}. \
         This is the legendary bug: the server was emitting bitmask 8 (bit 3 = 'static') \
         instead of bitmask {} (bit {} = 'defaultLibrary'), so clients decoded $_ as 'static'.",
        default_library_bit,
        expected_bitmask,
        all_variable_modifiers,
        expected_bitmask,
        default_library_bit
    );

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}).to_string(),
    )?;
    let _ = recv_until_id(&mut reader, 3)?;
    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}).to_string(),
    )?;
    let _ = child.wait();
    Ok(())
}

/// Contract test for the `sql_heredoc_keyword` token type (Issue #2059).
///
/// Verifies that `sql_heredoc_keyword` is present in the advertised legend AND that
/// opening a document with a `<<SQL` heredoc containing `SELECT` produces at least one
/// token that decodes to `sql_heredoc_keyword` via the advertised legend.
///
/// Guards against the legend index ordering class of bug fixed in PR #2772: if
/// `sql_heredoc_keyword` is missing from the advertised legend, the emitted index would
/// decode as a wrong type for every LSP client.
#[test]
fn sql_heredoc_keyword_index_decodes_correctly_via_advertised_legend() -> Result<(), BoxError> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout);

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

    let advertised_legend: Vec<String> =
        init_resp["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .ok_or("tokenTypes missing from initialize response")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

    // Verify sql_heredoc_keyword is present in the advertised legend.
    let sql_heredoc_advertised_idx = advertised_legend
        .iter()
        .position(|s| s == "sql_heredoc_keyword")
        .ok_or("sql_heredoc_keyword is not in the advertised tokenTypes legend")?;

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}).to_string(),
    )?;

    // Heredoc with SQL keyword — should produce sql_heredoc_keyword tokens.
    let source = "my $sql = <<SQL;\nSELECT * FROM users WHERE id = ?\nSQL\n";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///sql_heredoc_contract_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    send_msg(&mut stdin, &did_open)?;

    let sem_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": { "uri": "file:///sql_heredoc_contract_test.pl" }
        }
    })
    .to_string();
    send_msg(&mut stdin, &sem_req)?;
    let sem_resp = recv_until_id(&mut reader, 2)?;

    let data = sem_resp["result"]["data"]
        .as_array()
        .ok_or("semanticTokens response missing data array")?;

    let mut decoded_types: Vec<String> = Vec::new();
    for chunk in data.chunks(5) {
        let type_idx = chunk[3].as_u64().ok_or("token_type not u64")? as usize;
        let type_name = advertised_legend
            .get(type_idx)
            .cloned()
            .unwrap_or_else(|| format!("OUT_OF_RANGE({})", type_idx));
        decoded_types.push(type_name);
    }

    let found = decoded_types.iter().any(|t| t == "sql_heredoc_keyword");
    assert!(
        found,
        "No token decoded to 'sql_heredoc_keyword' via the advertised legend for <<SQL heredoc \
         containing SELECT. Decoded types: {:?}. \
         sql_heredoc_keyword is at advertised index {}.",
        decoded_types, sql_heredoc_advertised_idx
    );

    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}).to_string(),
    )?;
    let _ = recv_until_id(&mut reader, 3)?;
    send_msg(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}).to_string(),
    )?;
    let _ = child.wait();
    Ok(())
}
