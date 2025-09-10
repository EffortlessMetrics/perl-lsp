use serde_json::json;
use std::thread;

mod common;
use common::{
    adaptive_sleep_ms, initialize_lsp, max_concurrent_threads,
    send_notification, send_request, start_lsp_server,
};

/// Test suite for concurrent operations and race conditions
/// Ensures the LSP server handles concurrent requests correctly

#[test]
fn test_concurrent_document_modifications() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///concurrent.pl";

    // Open initial document
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
                    "text": "my $x = 1;"
                }
            }
        }),
    );

    // Send rapid concurrent modifications (adaptive to thread constraints)
    let max_threads = max_concurrent_threads();
    let thread_count = (max_threads * 2).min(18).max(2); // Scale between 2-18 based on available threads
    
    let handles: Vec<_> = (2..thread_count + 2)
        .map(|version| {
            thread::spawn(move || {
                // Simulate concurrent edits from different threads
                // Use adaptive sleep to reduce contention under thread constraints
                thread::sleep(adaptive_sleep_ms(version as u64 % 10));
                (version, format!("my $x = {};", version))
            })
        })
        .collect();

    // Apply all modifications
    for handle in handles {
        let (version, text) = handle.join().unwrap();
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": version
                    },
                    "contentChanges": [{
                        "text": text
                    }]
                }
            }),
        );
    }

    // Wait for system to stabilize before final request
    thread::sleep(adaptive_sleep_ms(50));
    
    // Final request should see consistent state
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
}

#[test]
fn test_concurrent_requests() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open test document
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub foo { }\nsub bar { }\nmy $x = 42;\nprint $x;"
                }
            }
        }),
    );

    // Send multiple requests simultaneously
    let request_ids = vec![
        (1, "textDocument/documentSymbol"),
        (2, "textDocument/hover"),
        (3, "textDocument/completion"),
        (4, "textDocument/definition"),
        (5, "textDocument/references"),
    ];

    for (id, method) in request_ids {
        let params = match method {
            "textDocument/documentSymbol" => json!({
                "textDocument": { "uri": "file:///test.pl" }
            }),
            _ => json!({
                "textDocument": { "uri": "file:///test.pl" },
                "position": { "line": 2, "character": 3 }
            }),
        };

        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        );
        
        // Brief pause to prevent overwhelming the server under thread constraints
        if max_concurrent_threads() <= 4 {
            thread::sleep(adaptive_sleep_ms(10));
        }
    }
}

#[test]
fn test_race_condition_open_close() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Rapidly open and close documents (adaptive count)
    let document_count = (max_concurrent_threads()).min(10).max(2);
    for i in 0..document_count {
        let uri = format!("file:///race{}.pl", i);

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
                        "text": format!("my $var{} = {};", i, i)
                    }
                }
            }),
        );

        // Immediately try to use it
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i * 2 + 1,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": &uri },
                    "position": { "line": 0, "character": 3 }
                }
            }),
        );

        // Close document
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": { "uri": &uri }
                }
            }),
        );

        // Brief pause between rapid operations to prevent overwhelming under thread constraints
        if max_concurrent_threads() <= 4 {
            thread::sleep(adaptive_sleep_ms(5));
        }
        
        // Try to use after close (should fail gracefully)
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i * 2 + 2,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": &uri },
                    "position": { "line": 0, "character": 3 }
                }
            }),
        );
    }
}

#[test]
fn test_workspace_symbol_during_changes() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open multiple documents (adaptive count)
    let document_count = (max_concurrent_threads() / 2).min(5).max(2);
    for i in 0..document_count {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///workspace{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": format!("sub func{} {{ return {}; }}", i, i)
                    }
                }
            }),
        );
    }

    // Start workspace symbol search
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "workspace/symbol",
            "params": {
                "query": "func"
            }
        }),
    );

    // Modify documents during search (adaptive count)
    for i in 0..document_count {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///workspace{}.pl", i),
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": format!("sub modified_func{} {{ return {}; }}", i, i * 2)
                    }]
                }
            }),
        );
    }

    // Another workspace search
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "workspace/symbol",
            "params": {
                "query": "modified"
            }
        }),
    );
}

#[test]
fn test_reference_search_during_edits() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open document with variable
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
                    "text": "my $shared = 1;\nprint $shared;\n$shared++;"
                }
            }
        }),
    );

    // Start reference search
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": "file:///refs.pl" },
                "position": { "line": 0, "character": 3 },
                "context": { "includeDeclaration": true }
            }
        }),
    );

    // Modify document while search might be in progress
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///refs.pl",
                    "version": 2
                },
                "contentChanges": [{
                    "text": "my $shared = 1;\nprint $shared;\n$shared++;\n$shared--;"
                }]
            }
        }),
    );

    // Another reference search
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": "file:///refs.pl" },
                "position": { "line": 0, "character": 3 },
                "context": { "includeDeclaration": false }
            }
        }),
    );
}

#[test]
fn test_completion_cache_invalidation() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///completion.pl";

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
                    "text": "my $var1 = 1;\nmy $var2 = 2;\n$v"
                }
            }
        }),
    );

    // Request completion
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 2 }
            }
        }),
    );

    // Change document (should invalidate completion cache)
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
                    "text": "my $variable1 = 1;\nmy $variable2 = 2;\n$vari"
                }]
            }
        }),
    );

    // Request completion again (should reflect new state)
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 5 }
            }
        }),
    );
}

#[test]
fn test_diagnostic_publishing_race() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///diagnostic.pl";

    // Rapidly change document to trigger diagnostic updates (adaptive count)
    let version_count = (max_concurrent_threads() * 2).min(20).max(5);
    for version in 1..version_count {
        let has_error = version % 3 == 0;
        let text = if has_error {
            format!("my $x = ;  # Error in version {}", version)
        } else {
            format!("my $x = {};  # Valid in version {}", version, version)
        };

        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": if version == 1 { "textDocument/didOpen" } else { "textDocument/didChange" },
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "perl",
                        "version": version
                    },
                    "contentChanges": if version == 1 {
                        json!([])
                    } else {
                        json!([{ "text": text.clone() }])
                    },
                    "text": if version == 1 { Some(text.clone()) } else { None }
                }
            }),
        );

        // Small delay to allow diagnostics to process (adaptive)
        thread::sleep(adaptive_sleep_ms(5));
    }

    // Allow system to stabilize before final verification
    thread::sleep(adaptive_sleep_ms(100));
    
    // Final state should be consistent
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
}

#[test]
fn test_multi_file_rename_race() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create multiple files with shared variable (adaptive count)
    let file_count = (max_concurrent_threads() / 2).min(3).max(1);
    for i in 0..file_count {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///rename{}.pl", i),
                        "languageId": "perl",
                        "version": 1,
                        "text": "use Common;\nmy $shared = get_shared();\nprint $shared;"
                    }
                }
            }),
        );
    }

    // Start rename operation
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/rename",
            "params": {
                "textDocument": { "uri": "file:///rename0.pl" },
                "position": { "line": 1, "character": 4 },
                "newName": "$renamed"
            }
        }),
    );

    // Modify files during rename (adaptive count)
    for i in 0..file_count {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///rename{}.pl", i),
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": "use Common;\nmy $shared = get_shared();\nprint $shared;\n# Modified"
                    }]
                }
            }),
        );
    }
}

#[test]
fn test_call_hierarchy_during_refactoring() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create call hierarchy
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///hierarchy.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
sub main {
    foo();
    bar();
}

sub foo {
    baz();
}

sub bar {
    baz();
}

sub baz {
    print "called";
}
"#
                }
            }
        }),
    );

    // Prepare call hierarchy
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/prepareCallHierarchy",
            "params": {
                "textDocument": { "uri": "file:///hierarchy.pl" },
                "position": { "line": 14, "character": 4 }  // On 'baz'
            }
        }),
    );

    // Modify document during hierarchy traversal
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///hierarchy.pl",
                    "version": 2
                },
                "contentChanges": [{
                    "text": r#"
sub main {
    foo();
    bar();
    qux();  # Added
}

sub foo {
    baz();
}

sub bar {
    baz();
}

sub baz {
    print "called";
}

sub qux {
    baz();  # New caller
}
"#
                }]
            }
        }),
    );
}

#[test]
fn test_semantic_tokens_consistency() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///semantic.pl";

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
                    "text": "package Test;\nuse strict;\nmy $var = 42;\nsub func { return $var; }"
                }
            }
        }),
    );

    // Request semantic tokens multiple times rapidly (adaptive count)
    let request_count = max_concurrent_threads().min(5).max(2);
    for id in 1..request_count {
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/semanticTokens/full",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        );

        // Interleave with edits
        if id % 2 == 0 {
            send_notification(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "version": id + 1
                        },
                        "contentChanges": [{
                            "text": format!("package Test;\nuse strict;\nmy $var = {};\nsub func {{ return $var; }}", id * 10)
                        }]
                    }
                }),
            );
        }
    }
}
