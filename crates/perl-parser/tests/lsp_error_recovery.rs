use serde_json::json;
use std::time::Duration;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Test suite for error recovery scenarios
/// Ensures the LSP server can recover from various error states

#[test]
fn test_recover_from_parse_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///recovery.pl";

    // Start with invalid syntax
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
                    "text": "my $x = ;  # Missing value\nprint $x"
                }
            }
        }),
    );

    // Should get diagnostic error
    std::thread::sleep(Duration::from_millis(100));

    // Fix the syntax error
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [{
                    "text": "my $x = 42;  # Fixed\nprint $x;"
                }]
            }
        }),
    );

    // Should now work correctly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().unwrap();
    assert!(symbols.len() > 0);
}

#[test]
fn test_partial_document_parsing() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document with mixed valid and invalid sections
    let content = r#"
# Valid section
sub valid_function {
    my $x = 42;
    return $x;
}

# Invalid section
sub invalid_function {
    my $y = ;  # Parse error
    return 
}

# Another valid section
sub another_valid {
    print "Hello";
}
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///partial.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should still find valid symbols
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///partial.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().unwrap();

    // Should find at least the valid functions
    let function_names: Vec<String> = symbols
        .iter()
        .filter_map(|s| s["name"].as_str())
        .map(|s| s.to_string())
        .collect();

    assert!(function_names.contains(&"valid_function".to_string()));
    assert!(function_names.contains(&"another_valid".to_string()));
}

#[test]
fn test_incremental_edit_recovery() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///incremental.pl";

    // Start with valid document
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
                    "text": "my $x = 1;\nmy $y = 2;\nprint $x + $y;"
                }
            }
        }),
    );

    // Make edit that breaks syntax temporarily
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 1, "character": 8 },
                        "end": { "line": 1, "character": 9 }
                    },
                    "text": ""  // Delete '2', leaving "my $y = ;"
                }]
            }
        }),
    );

    // Complete the edit to fix syntax
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 3
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 1, "character": 8 },
                        "end": { "line": 1, "character": 8 }
                    },
                    "text": "3"  // Add '3', making "my $y = 3;"
                }]
            }
        }),
    );

    // Should work correctly after recovery
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "position": {
                    "line": 1,
                    "character": 3  // On $y
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_object() || response["result"].is_null());
}

#[test]
fn test_workspace_recovery_after_error() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open multiple files, one with error
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///good1.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub foo { return 42; }"
                }
            }
        }),
    );

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///bad.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub bar { return ;; }"  // Syntax error
                }
            }
        }),
    );

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///good2.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub baz { return 'hello'; }"
                }
            }
        }),
    );

    // Workspace symbols should still work
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workspace/symbol",
            "params": {
                "query": "foo"
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().unwrap();
    assert!(symbols.iter().any(|s| s["name"] == "foo"));
}

#[test]
fn test_reference_search_with_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document with partial errors
    let content = r#"
my $var = 42;

sub use_var {
    print $var;  # Valid reference
}

sub broken {
    print $var;;  # Syntax error but reference should still be found
}

print $var;  # Another valid reference
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///refs.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Find references should work despite errors
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": {
                    "uri": "file:///refs.pl"
                },
                "position": {
                    "line": 1,
                    "character": 3  // On $var declaration
                },
                "context": {
                    "includeDeclaration": false
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
    let refs = response["result"].as_array().unwrap();
    assert!(refs.len() >= 2); // Should find at least the valid references
}

#[test]
fn test_completion_in_broken_context() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document with syntax error before completion point
    let content = r#"
sub broken {
    my $x = ;  # Syntax error
    
    # Try completion here
    pri
}
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///broken.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Completion should still work
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///broken.pl"
                },
                "position": {
                    "line": 5,
                    "character": 7  // After "pri"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"]["items"].is_array());
    let items = response["result"]["items"].as_array().unwrap();

    // Should suggest "print" despite earlier error
    assert!(items.iter().any(|item| item["label"] == "print"));
}

#[test]
fn test_rename_with_parse_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let content = r#"
my $old_name = 42;

sub valid {
    print $old_name;
}

sub invalid {
    print $old_name;;  # Extra semicolon
}

$old_name++;
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///rename.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Prepare rename should work
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/prepareRename",
            "params": {
                "textDocument": {
                    "uri": "file:///rename.pl"
                },
                "position": {
                    "line": 1,
                    "character": 3  // On $old_name
                }
            }
        }),
    );

    let response = read_response(&mut server);
    if response["result"].is_object() {
        // Perform rename
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/rename",
                "params": {
                    "textDocument": {
                        "uri": "file:///rename.pl"
                    },
                    "position": {
                        "line": 1,
                        "character": 3
                    },
                    "newName": "$new_name"
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response["result"]["changes"].is_object());
    }
}

#[test]
fn test_formatting_with_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document with formatting issues and syntax error
    let content = r#"my$x=1;my$y=2;
sub foo{print$x;}
sub bar{print$y;;}  # Syntax error
my   $z   =   3;"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///format.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Format document request
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/formatting",
            "params": {
                "textDocument": {
                    "uri": "file:///format.pl"
                },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }
        }),
    );

    let response = read_response(&mut server);
    // Should either format what it can or return error gracefully
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
fn test_diagnostic_recovery() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///diagnostic.pl";

    // Start with multiple errors
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
                    "text": "my $x = ;\nmy $y = ;\nmy $z = ;"
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(100));

    // Fix one error at a time
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 8 },
                        "end": { "line": 0, "character": 8 }
                    },
                    "text": "1"
                }]
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(100));

    // Fix second error
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 3
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 1, "character": 8 },
                        "end": { "line": 1, "character": 8 }
                    },
                    "text": "2"
                }]
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(100));

    // Fix last error
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 4
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 2, "character": 8 },
                        "end": { "line": 2, "character": 8 }
                    },
                    "text": "3"
                }]
            }
        }),
    );

    // All diagnostics should be cleared
    std::thread::sleep(Duration::from_millis(100));

    // Document should be fully functional now
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().unwrap();
    assert_eq!(symbols.len(), 3); // Should have all three variables
}

#[test]
fn test_goto_definition_with_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let content = r#"
sub my_func {
    return 42;;  # Syntax error
}

my $result = my_func();  # Should still find definition
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///goto.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Go to definition should work despite error in target
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": "file:///goto.pl"
                },
                "position": {
                    "line": 5,
                    "character": 14  // On my_func call
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["result"].is_object());
}

#[test]
fn test_hover_in_error_context() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let content = r#"
use strict;
use warnings;

my $valid_var = 42;

sub broken {
    my $x = ;  # Error
    print $valid_var;  # Hover should work here
}
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///hover.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Hover should work on valid variable despite nearby error
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": "file:///hover.pl"
                },
                "position": {
                    "line": 8,
                    "character": 11  // On $valid_var
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_object() || response["result"].is_null());
}
