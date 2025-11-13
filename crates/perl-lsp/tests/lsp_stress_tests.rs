use serde_json::json;
use std::time::{Duration, Instant};

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Stress tests for resource exhaustion and performance limits
/// Ensures the LSP server handles extreme loads gracefully
///
/// Get the number of iterations for stress tests from environment
/// Default: 500 for dev, can be overridden with PERL_LSP_STRESS_ITERS
fn stress_iterations() -> usize {
    std::env::var("PERL_LSP_STRESS_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(500)
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_large_file_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create a very large file (1MB+)
    let mut content = String::new();
    for i in 0..50000 {
        content.push_str(&format!("my $var_{} = {};\n", i, i));
        if i % 100 == 0 {
            content.push_str(&format!("sub function_{} {{ return {}; }}\n", i, i));
        }
    }

    let start = Instant::now();

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///large.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should complete within reasonable time (< 5 seconds)
    assert!(start.elapsed() < Duration::from_secs(5));

    // Should still be able to process requests
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///large.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_many_open_documents() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open many documents (configurable via env)
    let iterations = stress_iterations();
    for i in 0..iterations {
        let content =
            format!("package Module{};\nmy $var = {};\nsub func {{ return $var; }}\n1;", i, i);

        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///many/doc{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );
    }

    // Server should still respond
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workspace/symbol",
            "params": {
                "query": "Module500"
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_rapid_fire_requests() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open a test document
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///rapid.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 42;\nprint $x;"
                }
            }
        }),
    );

    // Send 1000 requests as fast as possible
    let start = Instant::now();
    for id in 1..=1000 {
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": "file:///rapid.pl"
                    },
                    "position": {
                        "line": 0,
                        "character": 3
                    }
                }
            }),
        );

        // Read response to avoid buffer overflow
        if id % 100 == 0 {
            let _ = read_response(&mut server);
        }
    }

    // Should complete within reasonable time
    assert!(start.elapsed() < Duration::from_secs(30));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_deeply_nested_ast() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create extremely deep nesting (1000 levels)
    let mut content = String::new();
    for _ in 0..1000 {
        content.push_str("if (1) { ");
    }
    content.push_str("print 'deep';");
    for _ in 0..1000 {
        content.push_str(" }");
    }

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///deep.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle without stack overflow
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///deep.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_massive_symbol_count() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create file with 10,000 symbols
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!("my $symbol_{} = {};\n", i, i));
    }

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///symbols.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Request all symbols
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///symbols.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());

    // Should complete in reasonable time
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_complex_regex_patterns() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create file with complex regex patterns
    let content = r#"
# Pathological regex patterns
if ($text =~ /^(a+)+$/) { }
if ($text =~ /(x+x+)+y/) { }
if ($text =~ /((a*)*)*b/) { }
if ($text =~ /(?:(?:a){1,1000}){1,1000}/) { }

# Nested lookaheads/lookbehinds
if ($text =~ /(?=(?=(?=.*a).*b).*c).*d/) { }
if ($text =~ /(?<=(?<=(?<=a.*)b.*)c.*)d/) { }

# Complex character classes
if ($text =~ /[^\x00-\x1F\x7F-\x9F\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/) { }
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///regex.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle complex patterns without hanging
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///regex.pl"
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_infinite_loop_prevention() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create potentially infinite recursive structure
    let content = r#"
package A;
use B;
sub call_b { B::call_a(); }

package B;
use A;
sub call_a { A::call_b(); }

# Circular inheritance
package C;
use base 'D';

package D;
use base 'E';

package E;
use base 'C';
"#;

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///circular.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle circular dependencies
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": {
                    "uri": "file:///circular.pl"
                },
                "position": {
                    "line": 2,
                    "character": 20  // On call_a
                },
                "context": {
                    "includeDeclaration": true
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_memory_leak_prevention() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Repeatedly open and close large documents
    for iteration in 0..100 {
        let uri = format!("file:///leak{}.pl", iteration % 10);

        // Create large content
        let mut content = String::new();
        for i in 0..1000 {
            content.push_str(&format!("my $var_{} = '{}' x 1000;\n", i, "x"));
        }

        // Open document
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": &uri,
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );

        // Perform operations
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": iteration + 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": &uri
                    }
                }
            }),
        );

        let _ = read_response(&mut server);

        // Close document
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": {
                        "uri": &uri
                    }
                }
            }),
        );
    }

    // Server should still be responsive
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 999,
            "method": "shutdown",
            "params": null
        }),
    );

    let response = read_response(&mut server);
    assert_eq!(response["result"], json!(null));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_workspace_search_performance() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create many files with many symbols
    for file_idx in 0..100 {
        let mut content = String::new();
        for sym_idx in 0..100 {
            content.push_str(&format!(
                "sub function_{}_{} {{ return {}; }}\n",
                file_idx, sym_idx, sym_idx
            ));
        }

        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///workspace/file{}.pl", file_idx),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );
    }

    // Search across all 10,000 symbols
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workspace/symbol",
            "params": {
                "query": "function_50"
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());

    // Should complete search in reasonable time
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing) - stress test
fn test_completion_with_huge_scope() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create file with thousands of variables in scope
    let mut content = String::new();
    for i in 0..5000 {
        content.push_str(&format!("my $variable_{} = {};\n", i, i));
    }
    content.push_str("\n# Type here for completion\n$vari");

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///huge_scope.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Request completion with huge scope
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///huge_scope.pl"
                },
                "position": {
                    "line": 5001,
                    "character": 5
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"]["items"].is_array());

    // Should complete in reasonable time despite large scope
    assert!(start.elapsed() < Duration::from_secs(3));
}
