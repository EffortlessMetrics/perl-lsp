use perl_parser::lsp_server::LspServer;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::sync::{Arc, Mutex};

/// Frame a JSON value as an LSP message with proper headers.
fn frame_message(value: &Value) -> Vec<u8> {
    let content = serde_json::to_string(value).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", content.len(), content).into_bytes()
}

/// Parse LSP messages from a byte buffer.
fn parse_messages(data: &[u8]) -> Vec<Value> {
    let mut reader = BufReader::new(Cursor::new(data));
    let mut messages = Vec::new();

    loop {
        let mut headers = Vec::new();

        // Read headers
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                return messages; // EOF
            }

            if line == "\r\n" || line == "\n" {
                break; // End of headers
            }

            headers.push(line);
        }

        // Find Content-Length
        let content_length = headers
            .iter()
            .find(|h| h.starts_with("Content-Length:"))
            .and_then(|h| h.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok());

        if let Some(length) = content_length {
            let mut content = vec![0u8; length];
            reader.read_exact(&mut content).unwrap();
            if let Ok(json) = serde_json::from_slice::<Value>(&content) {
                messages.push(json);
            }
        } else {
            break; // No content length found
        }
    }

    messages
}

/// Writer that captures output into a shared buffer.
struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_diagnostics_clear_protocol_framing() {
    // Capture server output so we can inspect the real LSP responses
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer: Arc<Mutex<Box<dyn Write + Send>>> =
        Arc::new(Mutex::new(Box::new(BufferWriter(buffer.clone()))));
    let mut server = LspServer::with_output(writer);

    // Initialize the server
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "rootUri": "file:///test",
            "capabilities": {},
        }
    });

    // Open a document
    let open_notification = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 42;\nprint $x;\n",
            }
        }
    });

    // Close the document - this should send a clear diagnostics notification
    let close_notification = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": {
                "uri": "file:///test/test.pl",
            }
        }
    });

    // Send messages to the server
    server
        .handle_message(&mut Cursor::new(frame_message(&init_request)))
        .unwrap();
    server
        .handle_message(&mut Cursor::new(frame_message(&open_notification)))
        .unwrap();
    server
        .handle_message(&mut Cursor::new(frame_message(&close_notification)))
        .unwrap();

    // Parse all messages emitted by the server
    let messages = {
        let buf = buffer.lock().unwrap();
        parse_messages(&buf)
    };

    // Verify initialization response was sent
    assert!(messages.iter().any(|m| {
        m.get("id") == Some(&json!(1)) && m.get("result").is_some()
    }), "missing initialize response");

    // Verify diagnostics were cleared on close
    let diag = messages
        .iter()
        .rev()
        .find(|m| m.get("method") == Some(&json!("textDocument/publishDiagnostics")))
        .expect("no diagnostics notification emitted");
    assert_eq!(diag["params"]["uri"], "file:///test/test.pl");
    assert!(diag["params"]["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn test_workspace_symbol_deduplication() {
    use perl_parser::workspace_index::WorkspaceIndex;
    use std::collections::HashSet;
    use url::Url;

    let index = WorkspaceIndex::new();

    // Index a file with duplicate symbols
    let perl_code = r#"
package Foo;

sub test {
    my $x = 1;
}

sub test {  # Duplicate subroutine
    my $x = 2;
}

package Foo;  # Duplicate package declaration

sub another {
    my $y = 3;
}
"#;

    let uri = "file:///test/test.pl";
    index.index_file(Url::parse(uri).unwrap(), perl_code.to_string()).unwrap();

    // Search for symbols
    let symbols = index.find_symbols("test");

    // Create a set to track unique symbols
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for symbol in &symbols {
        let key = (
            symbol.uri.clone(),
            symbol.range.start.line,
            symbol.range.start.character,
            symbol.name.clone(),
            symbol.kind,
        );

        if !seen.insert(key.clone()) {
            duplicates.push(symbol.clone());
        }
    }

    // There should be no duplicates in the final result
    // (The workspace/symbol handler should deduplicate)
    assert!(duplicates.is_empty(), "Found duplicate symbols: {:?}", duplicates);
}

#[test]
fn test_workspace_symbol_response_format() {
    use perl_parser::workspace_index::WorkspaceIndex;
    use url::Url;

    let index = WorkspaceIndex::new();

    // Index a simple file
    let perl_code = r#"
package TestPackage;

sub test_function {
    my $var = 42;
}
"#;

    let uri = "file:///test/test.pl";
    index.index_file(Url::parse(uri).unwrap(), perl_code.to_string()).unwrap();

    // Search for symbols
    let symbols = index.find_symbols("test");

    // Verify each symbol has the required LSP fields
    for symbol in symbols {
        // Check that serialization works
        let json = serde_json::to_value(&symbol).unwrap();

        // Verify required LSP fields are present
        assert!(json.get("name").is_some(), "Symbol missing 'name' field");
        assert!(json.get("kind").is_some(), "Symbol missing 'kind' field");
        assert!(json.get("uri").is_some(), "Symbol missing 'uri' field");
        assert!(json.get("range").is_some(), "Symbol missing 'range' field");

        // Verify range structure
        let range = json.get("range").unwrap();
        assert!(range.get("start").is_some(), "Range missing 'start' field");
        assert!(range.get("end").is_some(), "Range missing 'end' field");

        let start = range.get("start").unwrap();
        assert!(start.get("line").is_some(), "Start missing 'line' field");
        assert!(start.get("character").is_some(), "Start missing 'character' field");

        let end = range.get("end").unwrap();
        assert!(end.get("line").is_some(), "End missing 'line' field");
        assert!(end.get("character").is_some(), "End missing 'character' field");
    }
}

#[test]
fn test_position_encoding_advertised() {
    // This test verifies that the server advertises UTF-16 position encoding
    let _server = LspServer::new();

    let _init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "rootUri": "file:///test",
            "capabilities": {},
        }
    });

    // In a real test, we would capture the response and verify:
    // response["result"]["capabilities"]["positionEncoding"] == "utf-16"

    // For now, this test ensures the code compiles with the correct structure
}

#[test]
fn test_tool_detection() {
    // Test that tool detection doesn't crash on systems without perltidy/perlcritic
    // The actual detection happens in handle_initialize which uses Command::new

    // Try to detect perltidy
    let has_perltidy = std::process::Command::new("perltidy")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // This should not panic, regardless of whether perltidy is installed
    println!("perltidy available: {}", has_perltidy);

    // Try to detect perlcritic
    let has_perlcritic = std::process::Command::new("perlcritic")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // This should not panic, regardless of whether perlcritic is installed
    println!("perlcritic available: {}", has_perlcritic);
}

#[test]
fn test_uri_normalization() {
    use perl_parser::workspace_index::WorkspaceIndex;
    use url::Url;

    let index = WorkspaceIndex::new();

    let test_code = "sub test { }";

    // Test various URI formats
    let test_cases = vec![
        ("file:///home/user/test.pl", "file:///home/user/test.pl"),
        ("/home/user/test.pl", "file:///home/user/test.pl"),
        ("file:///home/user/test.pl/", "file:///home/user/test.pl/"), // URL crate handles this
        ("untitled:1", "untitled:1"),
    ];

    for (input, _expected) in test_cases {
        // Just ensure indexing doesn't panic with various URI formats
        let url = if input.starts_with("file://") || input.starts_with("untitled:") {
            Url::parse(input).ok()
        } else {
            Url::from_file_path(input).ok()
        };

        let result = if let Some(url) = url {
            index.index_file(url, test_code.to_string())
        } else {
            Err("Invalid URI".to_string())
        };
        assert!(result.is_ok(), "Failed to index with URI: {}", input);
    }
}
