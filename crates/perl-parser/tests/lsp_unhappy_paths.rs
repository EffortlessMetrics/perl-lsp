use serde_json::json;
use std::io::Write;
use std::time::Duration;

mod common;
use common::{start_lsp_server, send_request, send_notification, initialize_lsp, read_response};

/// Test suite for unhappy paths and error scenarios
/// Ensures the LSP server handles errors gracefully

#[test]
fn test_malformed_json_request() {
    let mut server = start_lsp_server();
    
    // Send malformed JSON
    writeln!(server.stdin.as_mut().unwrap(), 
        "Content-Length: 20\r\n\r\n{{invalid json here}}")
        .expect("Failed to write");
    
    // Server should respond with parse error
    let response = read_response(&mut server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32700); // Parse error
}

#[test]
fn test_invalid_method() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Call non-existent method
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/nonExistentMethod",
        "params": {}
    }));
    
    let response = read_response(&mut server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // Method not found
}

#[test]
fn test_missing_required_params() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Send completion request without required params
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {} // Missing textDocument and position
    }));
    
    let response = read_response(&mut server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602); // Invalid params
}

#[test]
fn test_invalid_uri_format() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Send document with invalid URI
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "not-a-valid-uri", // Invalid URI format
                "languageId": "perl",
                "version": 1,
                "text": "print 'test';"
            }
        }
    }));
    
    // Try to get diagnostics - should handle gracefully
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/diagnostic",
        "params": {
            "textDocument": {
                "uri": "not-a-valid-uri"
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["error"].is_object() || response["result"]["items"].as_array().unwrap().is_empty());
}

#[test]
fn test_document_not_found() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Request operations on non-existent document
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///non/existent/file.pl"
            },
            "position": {
                "line": 0,
                "character": 0
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["error"].is_object() || response["result"]["items"].as_array().unwrap().is_empty());
}

#[test]
fn test_out_of_bounds_position() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Open a small document
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "print 'hello';"
            }
        }
    }));
    
    // Request completion at out-of-bounds position
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "position": {
                "line": 9999,
                "character": 9999
            }
        }
    }));
    
    let response = read_response(&mut server);
    // Should handle gracefully, return empty or error
    assert!(response["error"].is_object() || response["result"]["items"].as_array().unwrap().is_empty());
}

#[test]
fn test_concurrent_document_edits() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    let uri = "file:///concurrent.pl";
    
    // Open document
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\nmy $y = 2;\nmy $z = 3;"
            }
        }
    }));
    
    // Send multiple rapid edits
    for i in 2..10 {
        send_notification(&mut server, json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": i
                },
                "contentChanges": [{
                    "text": format!("my $x = {};\nmy $y = {};\nmy $z = {};", i, i+1, i+2)
                }]
            }
        }));
    }
    
    // Request symbols - should use latest version
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": uri
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
fn test_version_mismatch() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    let uri = "file:///version.pl";
    
    // Open with version 1
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;"
            }
        }
    }));
    
    // Send change with wrong version (skip version 2)
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": uri,
                "version": 3  // Wrong version
            },
            "contentChanges": [{
                "text": "my $x = 2;"
            }]
        }
    }));
    
    // Server should handle version mismatch gracefully
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": uri
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
fn test_invalid_regex_pattern() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Open document with invalid regex
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///regex.pl",
                "languageId": "perl",
                "version": 1,
                "text": "if ($x =~ /[/) { print 'test'; }"  // Unclosed bracket
            }
        }
    }));
    
    // Should get syntax error diagnostic
    std::thread::sleep(Duration::from_millis(100));
    
    // Request hover on invalid regex
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {
                "uri": "file:///regex.pl"
            },
            "position": {
                "line": 0,
                "character": 10
            }
        }
    }));
    
    let response = read_response(&mut server);
    // Should handle parse error gracefully
    assert!(response.is_object());
}

#[test]
fn test_circular_module_dependency() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Create circular dependency
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///ModuleA.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package ModuleA;\nuse ModuleB;\n1;"
            }
        }
    }));
    
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///ModuleB.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package ModuleB;\nuse ModuleA;\n1;"
            }
        }
    }));
    
    // Request references - should handle circular deps
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": "file:///ModuleA.pm"
            },
            "position": {
                "line": 0,
                "character": 8
            },
            "context": {
                "includeDeclaration": true
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
fn test_extremely_long_line() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Create document with extremely long line
    let long_string = "x".repeat(100000);
    let content = format!("my $x = '{}';\nprint $x;", long_string);
    
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///long.pl",
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        }
    }));
    
    // Request completion in long line
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///long.pl"
            },
            "position": {
                "line": 0,
                "character": 50000
            }
        }
    }));
    
    let response = read_response(&mut server);
    // Should handle without crashing
    assert!(response.is_object());
}

#[test]
fn test_deeply_nested_structure() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Create deeply nested structure
    let mut nested = String::from("sub test {\n");
    for _ in 0..100 {
        nested.push_str("    if (1) {\n");
    }
    nested.push_str("        print 'deep';\n");
    for _ in 0..100 {
        nested.push_str("    }\n");
    }
    nested.push_str("}\n");
    
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///nested.pl",
                "languageId": "perl",
                "version": 1,
                "text": nested
            }
        }
    }));
    
    // Request symbols - should handle deep nesting
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": "file:///nested.pl"
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
fn test_binary_content() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Try to open binary content
    let binary_content = "#!/usr/bin/perl\n\0\x7F\x7E\x00binary data here\n";
    
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///binary.pl",
                "languageId": "perl",
                "version": 1,
                "text": binary_content
            }
        }
    }));
    
    // Should handle binary data gracefully
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": "file:///binary.pl"
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_cancel_request() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Send a request
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "position": {
                "line": 0,
                "character": 0
            }
        }
    }));
    
    // Immediately cancel it
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "$/cancelRequest",
        "params": {
            "id": 42
        }
    }));
    
    // Read response - should be cancelled
    let response = read_response(&mut server);
    if response["error"].is_object() {
        assert_eq!(response["error"]["code"], -32800); // Request cancelled
    }
}

#[test]
fn test_shutdown_without_exit() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Send shutdown request
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "shutdown",
        "params": null
    }));
    
    let response = read_response(&mut server);
    assert_eq!(response["result"], json!(null));
    
    // Try to send another request after shutdown
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "position": {
                "line": 0,
                "character": 0
            }
        }
    }));
    
    // Should get error - server is shut down
    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_invalid_capability_request() {
    let mut server = start_lsp_server();
    
    // Initialize without certain capabilities
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {
                // Explicitly disable some capabilities
                "textDocument": {
                    "completion": null,
                    "hover": null
                }
            }
        }
    }));
    
    let _response = read_response(&mut server);
    
    // Try to use disabled capability
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "position": {
                "line": 0,
                "character": 0
            }
        }
    }));
    
    let response = read_response(&mut server);
    // Should handle gracefully
    assert!(response.is_object());
}

#[test]
fn test_unicode_unhappy_paths() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Document with various Unicode edge cases
    let content = r#"
        my $零 = 0;  # Chinese zero
        my $😀 = "emoji";  # Emoji variable
        my $א = "Hebrew";  # RTL script
        my $\u{200B} = "zero-width space";  # Invisible character
        my $café = "coffee";  # Combining character
    "#;
    
    send_notification(&mut server, json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///unicode.pl",
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        }
    }));
    
    // Request symbols - should handle Unicode
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": "file:///unicode.pl"
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
fn test_memory_stress() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    
    // Open many documents
    for i in 0..100 {
        let content = format!("my $var{} = {};\n", i, i).repeat(100);
        send_notification(&mut server, json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": format!("file:///stress{}.pl", i),
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }));
    }
    
    // Server should still respond
    send_request(&mut server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": "file:///stress50.pl"
            }
        }
    }));
    
    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}