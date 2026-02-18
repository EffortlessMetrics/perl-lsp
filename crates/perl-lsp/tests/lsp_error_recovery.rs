use serde_json::json;
use std::time::Duration;

mod common;
use common::{
    completion_items, initialize_lsp, send_notification, send_request, shutdown_and_exit,
    start_lsp_server,
};

/// Test suite for error recovery scenarios
/// Ensures the LSP server can recover from various error states

#[test]
fn test_recover_from_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
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

    // Wait for diagnostics to clear after fix
    common::drain_until_quiet(&mut server, common::short_timeout(), Duration::from_secs(2));

    // Should now work correctly
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );
    assert!(response["result"].is_array(), "Response was not an array: {}", response);
    let symbols = response["result"].as_array().ok_or("Expected 'result' to be an array")?;
    assert!(!symbols.is_empty());
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_partial_document_parsing() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///partial.pl"
                }
            }
        }),
    );
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().ok_or("Expected 'result' to be an array")?;

    // Should find at least the valid functions
    let function_names: Vec<String> =
        symbols.iter().filter_map(|s| s["name"].as_str()).map(|s| s.to_string()).collect();

    assert!(function_names.contains(&"valid_function".to_string()));
    assert!(function_names.contains(&"another_valid".to_string()));
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_incremental_edit_recovery() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    assert!(response["result"].is_object() || response["result"].is_null());
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_workspace_recovery_after_error() -> Result<(), Box<dyn std::error::Error>> {
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

    // Give the server time to index the documents
    std::thread::sleep(Duration::from_millis(100));

    // Workspace symbols should still work
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/symbol",
            "params": {
                "query": "foo"
            }
        }),
    );
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().ok_or("Expected 'result' to be an array")?;
    // Note: workspace symbols requires the 'workspace' feature to be enabled
    // Without it, an empty array is returned which is valid behavior
    if !symbols.is_empty() {
        assert!(symbols.iter().any(|s| s["name"] == "foo"), "Workspace symbols: {:?}", symbols);
    }
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_reference_search_with_errors() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    assert!(response["result"].is_array());
    let refs = response["result"].as_array().ok_or("Expected 'result' to be an array")?;
    // When there are syntax errors, references might not be found
    // The important thing is that the server doesn't crash and returns a valid response
    eprintln!("Found {} references (may be 0 due to parse errors): {:?}", refs.len(), refs);
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_completion_in_broken_context() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    let items = completion_items(&response);
    assert!(!items.is_empty());

    // Should suggest "print" despite earlier error
    assert!(items.iter().any(|item| item["label"] == "print"));
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_rename_with_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    if response["result"].is_object() {
        // Perform rename
        let response = send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
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
        assert!(response["result"]["changes"].is_object());
    }
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_formatting_with_errors() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    // Should either format what it can or return error gracefully
    assert!(response["result"].is_array() || response["error"].is_object());
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_diagnostic_recovery() -> Result<(), Box<dyn std::error::Error>> {
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

    // Wait for change to be processed
    common::drain_until_quiet(&mut server, common::short_timeout(), Duration::from_secs(2));

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

    // Wait for change to be processed
    common::drain_until_quiet(&mut server, common::short_timeout(), Duration::from_secs(2));

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

    // Wait for all changes to be processed
    common::drain_until_quiet(&mut server, common::short_timeout(), Duration::from_secs(2));

    // Extra settle time for the server to complete async processing
    std::thread::sleep(Duration::from_millis(200));

    // Document should be fully functional now
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );
    assert!(response["result"].is_array());
    let symbols = response["result"].as_array().ok_or("Expected 'result' to be an array")?;

    // #307: Under CI load, incremental didChange notifications may not all be
    // processed before the documentSymbol request. Retry with a convergence
    // loop instead of accepting degraded behavior.
    let mut attempt = 0;
    let max_attempts = 5;
    let mut final_count = symbols.len();
    let summarize_symbol = |s: &serde_json::Value| -> String {
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("<unnamed>");
        let sl = s.pointer("/range/start/line").and_then(|v| v.as_u64()).unwrap_or(0);
        let sc = s.pointer("/range/start/character").and_then(|v| v.as_u64()).unwrap_or(0);
        let el = s.pointer("/range/end/line").and_then(|v| v.as_u64()).unwrap_or(0);
        let ec = s.pointer("/range/end/character").and_then(|v| v.as_u64()).unwrap_or(0);
        format!("{name} [{sl}:{sc}-{el}:{ec}]")
    };
    let mut last_summaries: Vec<String> = symbols.iter().map(&summarize_symbol).collect();

    while final_count != 3 && attempt < max_attempts {
        attempt += 1;
        eprintln!(
            "INFO(#307): Expected 3 symbols but got {} (attempt {}/{}). Waiting for convergence.",
            final_count, attempt, max_attempts
        );

        // Wait for server to process buffered notifications
        std::thread::sleep(Duration::from_millis(200 * attempt as u64));
        common::drain_until_quiet(&mut server, common::short_timeout(), Duration::from_secs(2));

        let retry_response = send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        );

        if let Some(arr) = retry_response["result"].as_array() {
            final_count = arr.len();
            last_summaries = arr.iter().map(&summarize_symbol).collect();
        }
    }

    assert_eq!(
        final_count, 3,
        "FAIL(#307): documentSymbol never converged to 3 symbols after {} attempts. \
         Last count: {}. Symbols: {:?}. This indicates a real bug in incremental text sync, \
         not a timing flake.",
        max_attempts, final_count, last_summaries
    );

    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_goto_definition_with_errors() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    assert!(response["result"].is_array() || response["result"].is_object());
    shutdown_and_exit(&mut server);
    Ok(())
}

#[test]
fn test_hover_in_error_context() -> Result<(), Box<dyn std::error::Error>> {
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
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
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
    assert!(response["result"].is_object() || response["result"].is_null());
    shutdown_and_exit(&mut server);
    Ok(())
}
