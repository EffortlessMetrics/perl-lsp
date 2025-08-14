/// Tests for new LSP features in v0.8.2
use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

/// Test document links for MetaCPAN
#[test]
fn test_document_links_metacpan() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use Data::Dumper;
use File::Path qw(make_path);
require Module::Load;
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let links = response["result"].as_array().unwrap();
    assert!(
        links.len() >= 3,
        "Should have links for Data::Dumper, File::Path, and Module::Load"
    );

    // Check Data::Dumper link
    let dumper_link = links
        .iter()
        .find(|l| l["target"].as_str().unwrap_or("").contains("Data::Dumper"))
        .expect("Should have Data::Dumper link");
    assert!(
        dumper_link["target"]
            .as_str()
            .unwrap()
            .contains("metacpan.org")
    );
}

/// Test document links for local files
#[test]
fn test_document_links_local_files() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///workspace/src/main.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
require "lib/Utils.pm";
do "config/settings.pl";
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let links = response["result"].as_array().unwrap();
    assert!(
        links.len() >= 2,
        "Should have links for lib/Utils.pm and config/settings.pl"
    );

    // Check that links are file:// URIs
    for link in links {
        let target = link["target"].as_str().unwrap_or("");
        assert!(
            target.starts_with("file://"),
            "Local file links should use file:// protocol"
        );
    }
}

/// Test selection ranges
#[test]
fn test_selection_ranges() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
sub process_data {
    my ($data) = @_;
    if ($data > 10) {
        return $data * 2;
    }
    return $data;
}
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/selectionRange",
            "params": {
                "textDocument": { "uri": uri },
                "positions": [
                    { "line": 4, "character": 15 }  // Inside "$data * 2"
                ]
            }
        }),
    );

    let ranges = response["result"].as_array().unwrap();
    assert!(!ranges.is_empty(), "Should have selection ranges");

    let first_range = &ranges[0];
    assert!(first_range.get("range").is_some(), "Should have a range");

    // Check that we have a parent chain (identifier -> expression -> statement -> block -> function)
    let mut current = first_range;
    let mut depth = 0;
    while let Some(parent) = current.get("parent") {
        current = parent;
        depth += 1;
    }
    assert!(
        depth >= 2,
        "Should have at least 2 levels of selection hierarchy"
    );
}

/// Test on-type formatting for braces
#[test]
fn test_on_type_formatting_brace() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "if ($x) {"
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/onTypeFormatting",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 },  // After '{'
                "ch": "{",
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }
        }),
    );

    // Check if we got edits (might be null if no formatting needed)
    if let Some(edits) = response["result"].as_array() {
        // If edits are returned, they should be valid TextEdit objects
        for edit in edits {
            assert!(edit.get("range").is_some(), "Edit should have a range");
            assert!(edit.get("newText").is_some(), "Edit should have newText");
        }
    }
}

/// Test on-type formatting for newline
#[test]
fn test_on_type_formatting_newline() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub test {\n    my $x = 1;\n"
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/onTypeFormatting",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 0 },  // Start of new line
                "ch": "\n",
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }
        }),
    );

    // Check if we got edits
    if let Some(edits) = response["result"].as_array() {
        // Newline formatting might add indentation
        for edit in edits {
            assert!(edit.get("range").is_some(), "Edit should have a range");
            let new_text = edit["newText"].as_str().unwrap_or("");
            // Check if indentation was added (should match the previous line)
            if !new_text.is_empty() {
                assert!(
                    new_text.chars().all(|c| c == ' ' || c == '\t'),
                    "Newline formatting should only add whitespace"
                );
            }
        }
    }
}

/// Test workspace/didChangeWatchedFiles registration
#[test]
fn test_file_watcher_registration() {
    let mut server = start_lsp_server();

    // Initialize with dynamic registration support
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": "file:///test",
                "capabilities": {
                    "workspace": {
                        "didChangeWatchedFiles": {
                            "dynamicRegistration": true
                        }
                    }
                }
            }
        }),
    );

    assert!(
        response.get("result").is_some(),
        "Initialize should succeed"
    );

    // Send initialized notification - this should trigger file watcher registration
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    // The server should have sent a client/registerCapability request
    // In a real test harness, we'd capture outgoing requests
    // For now, just verify the server doesn't crash
}

/// Test that selection ranges handle edge cases
#[test]
fn test_selection_ranges_edge_cases() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "$x"  // Very simple expression
                }
            }
        }),
    );

    // Test at the start of the identifier
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/selectionRange",
            "params": {
                "textDocument": { "uri": uri },
                "positions": [
                    { "line": 0, "character": 0 },  // At '$'
                    { "line": 0, "character": 1 }   // At 'x'
                ]
            }
        }),
    );

    let ranges = response["result"].as_array().unwrap();
    assert_eq!(ranges.len(), 2, "Should return a range for each position");
}

/// Test on-type formatting with tabs
#[test]
fn test_on_type_formatting_tabs() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "if ($x) {"
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/onTypeFormatting",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 },
                "ch": "{",
                "options": {
                    "tabSize": 8,
                    "insertSpaces": false  // Use tabs
                }
            }
        }),
    );

    // Check formatting with tabs
    if let Some(edits) = response["result"].as_array() {
        for edit in edits {
            let new_text = edit["newText"].as_str().unwrap_or("");
            // If indentation is added, it should use tabs when insertSpaces is false
            if new_text.contains('\t') || new_text.contains(' ') {
                eprintln!("Formatting applied with tabs setting");
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn test_document_links_windows_path_with_space() {
    use serde_json::json;
    mod common;
    use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Simulate a file living in a folder with a space; we don't need the file to exist.
    let uri = "file:///C:/Temp/Perl%20LSP%20Demo/main.pl";
    let text = r#"require "lib\\Thing.pm";"#;

    // Open doc
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    );

    // Request document links
    let resp = send_request(
        &mut server,
        json!({
            "jsonrpc":"2.0",
            "id": 99001,
            "method":"textDocument/documentLink",
            "params":{ "textDocument": { "uri": uri } }
        }),
    );

    let links = resp["result"].as_array().expect("links array");
    assert!(!links.is_empty(), "should return at least one link");
    let target = links[0]["target"].as_str().expect("target uri");

    // Percent-encoded space must be preserved; path join must be forward-slash normalized
    assert!(
        target.ends_with("C:/Temp/Perl%20LSP%20Demo/lib/Thing.pm"),
        "unexpected target: {target}"
    );
}
