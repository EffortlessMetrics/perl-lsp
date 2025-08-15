use serde_json::json;
use std::time::{Duration, Instant};

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Memory pressure and resource exhaustion tests
/// Tests server behavior under extreme memory conditions

#[test]
fn test_extremely_large_document() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create a 10MB document
    let mut large_content = String::with_capacity(10 * 1024 * 1024);
    for i in 0..100000 {
        large_content
            .push_str(&format!("my $var_{} = 'value_{}'; # Long comment to pad the line\n", i, i));
    }

    let uri = "file:///large_document.pl";

    // Open extremely large document
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
                    "text": large_content
                }
            }
        }),
    );

    // Try to get symbols for large document
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    let elapsed = start.elapsed();

    // Should complete within reasonable time
    assert!(elapsed < Duration::from_secs(10));
    assert!(response.is_object());
}

#[test]
fn test_many_small_documents() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open 1000 small documents
    for i in 0..1000 {
        let uri = format!("file:///doc_{}.pl", i);
        let content = format!("#!/usr/bin/perl\nmy $var{} = {};\nprint $var{};\n", i, i, i);

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
                        "text": content
                    }
                }
            }),
        );

        // Give server time to process without overwhelming
        if i % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // Server should still be responsive
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workspace/symbol",
            "params": {
                "query": "var500"
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_deep_ast_nesting() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create deeply nested expression (may cause stack overflow)
    let mut nested = String::from("1");
    for _ in 0..5000 {
        nested = format!("({} + 1)", nested);
    }

    let content = format!("my $x = {};", nested);
    let uri = "file:///deep_nesting.pl";

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
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_wide_ast_tree() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create document with many top-level symbols
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!("sub func_{} {{ print '{}'; }}\n", i, i));
    }

    let uri = "file:///wide_tree.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Request symbols (should handle many symbols)
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
fn test_memory_leak_detection() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///leak_test.pl";

    // Repeatedly open and close same document
    for i in 0..100 {
        // Open document
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "perl",
                        "version": i,
                        "text": format!("print 'version {}';", i)
                    }
                }
            }),
        );

        // Perform operations
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {"uri": uri}
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
                    "textDocument": {"uri": uri}
                }
            }),
        );
    }

    // Server should not have leaked memory
    // (In real test, would check process memory usage)
}

#[test]
fn test_infinite_loop_in_content() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document that might trigger infinite parsing
    let content = r#"
# Pathological regular expressions
$text =~ /(a*)*b/;
$text =~ /((a+)+)+b/;

# Circular references
my $x = \$x;
my @a = (\@a);
my %h = (self => \%h);

# Infinite heredoc?
print <<'EOF'
This heredoc never ends...
"#;

    let uri = "file:///infinite.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Should parse without hanging
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(5));
    assert!(response.is_object());
}

#[test]
fn test_exponential_backtracking() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Pattern that could cause exponential backtracking
    let mut content = String::from("my $x = '");
    content.push_str(&"a".repeat(1000));
    content.push_str("' =~ /(a+)+b/;");

    let uri = "file:///backtrack.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Should handle without exponential time
    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(2));
    assert!(response.is_object());
}

#[test]
fn test_recursive_macro_expansion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Document with recursive-like patterns
    let content = r#"
use constant A => 'B';
use constant B => 'C';
use constant C => 'A';

my $x = A . B . C;

BEGIN {
    eval "BEGIN { eval 'BEGIN { print }' }";
}
"#;

    let uri = "file:///recursive.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Should handle recursive patterns
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 5, "character": 10} // On 'A'
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_cache_exhaustion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create many unique documents to exhaust any caches
    for i in 0..1000 {
        let uri = format!("file:///cache_test_{}.pl", i);

        // Each document has unique content
        let content =
            format!("package Package{};\nsub unique_func_{} {{ return {}; }}\n1;", i, i, i);

        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri.clone(),
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );

        // Request hover to trigger caching
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": 1, "character": 5}
                }
            }),
        );

        let _ = read_response(&mut server);
    }

    // Cache should handle eviction properly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 9999,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///cache_test_0.pl"},
                "position": {"line": 1, "character": 5}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_string_explosion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create document with many string concatenations
    let mut content = String::from("my $str = ");
    for i in 0..1000 {
        content.push_str(&format!("'part{}' . ", i));
    }
    content.push_str("'end';");

    let uri = "file:///string_explosion.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Should handle many string operations
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_symbol_table_explosion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create document with many symbols
    let mut content = String::new();

    // Many variables
    for i in 0..1000 {
        content.push_str(&format!("my $var_{} = {};\n", i, i));
    }

    // Many subroutines
    for i in 0..1000 {
        content.push_str(&format!("sub sub_{} {{ return {}; }}\n", i, i));
    }

    // Many packages
    for i in 0..100 {
        content.push_str(&format!("package Pkg{}; our $pkg_var = {};\n", i, i));
    }

    let uri = "file:///symbol_explosion.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Request all symbols
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Search in large symbol table
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workspace/symbol",
            "params": {
                "query": "var_500"
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_diagnostic_explosion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create document with many syntax errors
    let mut content = String::new();
    for i in 0..1000 {
        // Each line has an error
        content.push_str(&format!("my $var_{} = \n", i)); // Missing semicolon
        content.push_str("print 'test' \n"); // Missing semicolon
        content.push_str("sub { }\n"); // Missing name
        content.push_str("if () { }\n"); // Empty condition
    }

    let uri = "file:///many_errors.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Should handle many diagnostics
    std::thread::sleep(Duration::from_millis(500));

    // Request code actions for error
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {"uri": uri},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 10, "character": 0}
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_reference_chain() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create long chain of references
    let mut content = String::new();
    content.push_str("my $var0 = 1;\n");
    for i in 1..1000 {
        content.push_str(&format!("my $var{} = $var{};\n", i, i - 1));
    }

    let uri = "file:///ref_chain.pl";

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
                    "text": content
                }
            }
        }),
    );

    // Find references to first variable (should find many)
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 4}, // On $var0
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
fn test_incremental_parsing_stress() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///incremental.pl";
    let initial = "print 'hello';\n".repeat(100);

    // Open document
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
                    "text": initial
                }
            }
        }),
    );

    // Send many rapid incremental changes
    for i in 2..1002 {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": i
                    },
                    "contentChanges": [{
                        "range": {
                            "start": {"line": 0, "character": 7},
                            "end": {"line": 0, "character": 12}
                        },
                        "text": format!("change{}", i)
                    }]
                }
            }),
        );
    }

    // Server should handle rapid changes
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}
