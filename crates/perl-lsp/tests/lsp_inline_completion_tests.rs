//! Tests for LSP inline completion support

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

fn setup_server() -> Result<LspServer, Box<dyn std::error::Error>> {
    let server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    server.handle_request(init_request);

    // Send initialized notification per LSP 3.17 protocol requirements
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);

    Ok(server)
}

fn open_doc(server: &LspServer, uri: &str, text: &str) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text,
            }
        })),
    };
    server.handle_request(request);
}

fn inline_completion(
    server: &LspServer,
    uri: &str,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "textDocument/inlineCompletion".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })),
    };
    let response = server
        .handle_request(request)
        .ok_or("inline completion response")?;
    response.result.ok_or("result field present".into())
}

#[test]
fn test_inline_completion_after_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "my $obj = Package->");
    let result = inline_completion(&server, uri, 0, 19)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert_eq!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?,
        "new()"
    );
    Ok(())
}

#[test]
fn test_inline_completion_after_use() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "use ");
    let result = inline_completion(&server, uri, 0, 4)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    let mut suggestions = Vec::new();
    for item in items.iter() {
        let text = item["insertText"]
            .as_str()
            .ok_or("insertText not a string")?;
        suggestions.push(text.to_string());
    }
    assert!(suggestions.contains(&"strict;".to_string()));
    assert!(suggestions.contains(&"warnings;".to_string()));
    Ok(())
}

#[test]
fn test_inline_completion_after_use_preserves_priority_order()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "use ");
    let result = inline_completion(&server, uri, 0, 4)?;
    let items = result["items"].as_array().ok_or("items array")?;

    let inserts: Vec<&str> = items
        .iter()
        .map(|item| item["insertText"].as_str().ok_or("insertText not a string"))
        .collect::<Result<_, _>>()?;

    assert_eq!(inserts.first().copied(), Some("strict;"));
    assert_eq!(inserts.get(1).copied(), Some("warnings;"));
    assert_eq!(inserts.get(2).copied(), Some("feature ':5.36';"));
    Ok(())
}

#[test]
fn test_inline_completion_shebang() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "#!");
    let result = inline_completion(&server, uri, 0, 2)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert_eq!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?,
        "/usr/bin/env perl"
    );
    Ok(())
}

#[test]
fn test_inline_completion_sub_body() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "sub test ");
    let result = inline_completion(&server, uri, 0, 9)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?
            .contains("{")
    );
    Ok(())
}

#[test]
fn test_inline_completion_no_suggestions() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&server, uri, "my $x = 42;");
    let result = inline_completion(&server, uri, 0, 10)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(items.is_empty());
    Ok(())
}

#[test]
fn test_inline_completion_after_arrow_with_multibyte_prefix()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    let text = "my $emoji = \"😀\"; my $obj = Package->";
    open_doc(&server, uri, text);

    // A real LSP client sends UTF-16 code unit count, not byte count.
    // This emoji string is 39 UTF-8 bytes but 37 UTF-16 code units.
    let character = text.encode_utf16().count() as u32;
    let result = inline_completion(&server, uri, 0, character)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(!items.is_empty());
    assert_eq!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?,
        "new()"
    );
    Ok(())
}

/// Regression test: verify UTF-16 position handling uses microcrate's
/// utf16_line_col_to_offset, not naive byte indexing.
///
/// The string "my $emoji = \"😀\"; my $obj = Package->" has:
///   UTF-8 byte length: 39
///   UTF-16 code units: 37 (emoji 😀 is 2 UTF-16 units, 4 UTF-8 bytes)
///
/// LSP sends UTF-16 position 37 (end of "->"). The naive byte implementation
/// would slice [..37] which lands BEFORE "->", producing no completion.
/// The microcrate uses utf16_line_col_to_offset which correctly maps
/// UTF-16 position 37 to byte offset 39 (end of string), triggering new().
#[test]
fn test_inline_completion_utf16_position_correct() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    // "😀" is 4 UTF-8 bytes but 2 UTF-16 code units.
    // UTF-16 end-of-string position = 37 (not 39, the byte length).
    let text = "my $emoji = \"😀\"; my $obj = Package->";
    open_doc(&server, uri, text);

    // Use UTF-16 encoded length as the LSP character position (this is what
    // a real editor sends — UTF-16 code unit count, not byte count).
    let utf16_character = text.encode_utf16().count() as u32;
    // Confirm our understanding: UTF-16 len < UTF-8 len due to 4-byte emoji
    assert_eq!(utf16_character, 37, "expected 37 UTF-16 code units");

    let result = inline_completion(&server, uri, 0, utf16_character)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(
        !items.is_empty(),
        "expected inline completion after -> when using correct UTF-16 position"
    );
    assert_eq!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?,
        "new()",
        "expected 'new()' suggestion after ->"
    );
    Ok(())
}

/// Edge case: completion requested on line 1 of a multiline document.
/// Verifies that line_context_at_position correctly skips line 0 and
/// returns the right prefix for line 1.
#[test]
fn test_inline_completion_multiline_doc_line1() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///test.pl";
    // Two-line document. Completion is requested on line 1 at end of "->".
    let text = "use strict;\nmy $obj = Package->";
    open_doc(&server, uri, text);

    let second_line = "my $obj = Package->";
    let character = second_line.encode_utf16().count() as u32;
    let result = inline_completion(&server, uri, 1, character)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(
        !items.is_empty(),
        "expected completion on line 1 of multiline doc"
    );
    assert_eq!(
        items[0]["insertText"]
            .as_str()
            .ok_or("insertText not a string")?,
        "new()",
        "expected 'new()' suggestion after -> on line 1"
    );
    Ok(())
}

#[test]
fn test_inline_completion_empty_file_returns_scaffold() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///empty.pl";
    open_doc(&server, uri, "");

    let result = inline_completion(&server, uri, 0, 0)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(
        !items.is_empty(),
        "expected scaffold suggestions for an empty file"
    );
    assert!(items.iter().any(|item| {
        item["insertText"]
            .as_str()
            .map(|text| text.contains("use strict;"))
            .unwrap_or(false)
    }));
    Ok(())
}

#[test]
fn test_inline_completion_mid_code_uses_nearby_variable_context()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///context.pl";
    let text = "use strict;\n\nsub helper {\n    my $result = compute();\n    \n}\n";
    open_doc(&server, uri, text);

    let result = inline_completion(&server, uri, 4, 4)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(
        !items.is_empty(),
        "expected contextual inline completions inside a sub"
    );
    assert!(items.iter().any(|item| {
        item["insertText"]
            .as_str()
            .map(|text| text == "return $result;")
            .unwrap_or(false)
    }));
    Ok(())
}

#[test]
fn test_inline_completion_after_comment_keeps_contextual_suggestions()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server()?;
    let uri = "file:///comment-context.pl";
    let text =
        "use Test::More;\n\nsub helper {\n    my $result = 1;\n    # explain next step\n    \n}\n";
    open_doc(&server, uri, text);

    let result = inline_completion(&server, uri, 5, 4)?;
    let items = result["items"].as_array().ok_or("items array")?;

    assert!(
        !items.is_empty(),
        "expected inline completions on a blank line after a comment"
    );
    assert!(items.iter().any(|item| {
        item["insertText"]
            .as_str()
            .map(|text| text == "return $result;")
            .unwrap_or(false)
    }));
    Ok(())
}
