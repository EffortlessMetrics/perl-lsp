//! Performance benchmarks for LSP server.
//! Measures response times and throughput for various operations.
//! These tests are slow and should only run with `cargo test --features stress-tests`.
#![cfg(feature = "stress-tests")]

use serde_json::json;
use std::time::{Duration, Instant};

mod common;
use common::test_utils::generators;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Performance requirements
const MAX_INIT_TIME: Duration = Duration::from_millis(1000);
const MAX_PARSE_TIME_PER_KB: Duration = Duration::from_micros(500);
const MAX_DIAGNOSTIC_TIME: Duration = Duration::from_millis(100);
const MAX_SYMBOL_TIME: Duration = Duration::from_millis(50);
const MAX_DEFINITION_TIME: Duration = Duration::from_millis(30);
const MAX_REFERENCES_TIME: Duration = Duration::from_millis(100);
const MAX_HOVER_TIME: Duration = Duration::from_millis(20);

#[test]
fn benchmark_initialization() {
    let start = Instant::now();
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    let duration = start.elapsed();

    println!("Initialization time: {:?}", duration);
    assert!(
        duration < MAX_INIT_TIME,
        "Initialization took {:?}, expected < {:?}",
        duration,
        MAX_INIT_TIME
    );
}

#[test]
fn benchmark_parsing_simple_file() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let simple_code = r#"
#!/usr/bin/perl
use strict;
use warnings;

my $x = 42;
print "Hello, World!\n";
"#;

    let uri = "file:///simple.pl";
    let start = Instant::now();

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
                    "text": simple_code
                }
            }
        }),
    );

    // Request diagnostics to ensure parsing is complete
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    let size_kb = simple_code.len() as f64 / 1024.0;
    let time_per_kb = duration.as_micros() as f64 / size_kb;

    println!("Parse time: {:?} ({:.0} µs/KB)", duration, time_per_kb);
    assert!(duration < MAX_PARSE_TIME_PER_KB * (size_kb.ceil() as u32), "Parsing took too long");
}

#[test]
fn benchmark_parsing_large_file() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Generate a 100KB file
    let large_code = generators::generate_large_file(2500);
    let uri = "file:///large.pl";

    let start = Instant::now();

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
                    "text": large_code
                }
            }
        }),
    );

    // Request diagnostics
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    let size_kb = large_code.len() as f64 / 1024.0;
    let time_per_kb = duration.as_micros() as f64 / size_kb;

    println!(
        "Large file parse time: {:?} ({:.0} µs/KB for {:.1} KB)",
        duration, time_per_kb, size_kb
    );

    // Allow more time for large files but ensure linear scaling
    assert!(time_per_kb < 1000.0, "Parsing doesn't scale linearly");
}

#[test]
fn benchmark_incremental_parsing() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let initial_code = "my $x = 1;\n".repeat(100);
    let uri = "file:///incremental.pl";

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
                    "text": initial_code
                }
            }
        }),
    );

    // Measure incremental update time
    let mut total_duration = Duration::ZERO;
    let iterations = 100;

    for i in 0..iterations {
        let modified_code = format!("{}my $y = {};\n", initial_code, i);
        let start = Instant::now();

        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": i + 2
                    },
                    "contentChanges": [{
                        "text": modified_code
                    }]
                }
            }),
        );

        total_duration += start.elapsed();
    }

    let avg_duration = total_duration / iterations;
    println!("Average incremental parse time: {:?}", avg_duration);
    assert!(avg_duration < Duration::from_millis(10), "Incremental parsing too slow");
}

#[test]
fn benchmark_diagnostics() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code_with_errors = r#"
my $x = ;  # Missing value
print $undefined_var;
sub { }  # Missing name
for my $i (@) { }  # Missing array
"#;

    let uri = "file:///errors.pl";

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
                    "text": code_with_errors
                }
            }
        }),
    );

    let start = Instant::now();

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Diagnostic time: {:?}", duration);
    assert!(
        duration < MAX_DIAGNOSTIC_TIME,
        "Diagnostics took {:?}, expected < {:?}",
        duration,
        MAX_DIAGNOSTIC_TIME
    );
}

#[test]
fn benchmark_document_symbols() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = generators::generate_symbols(50);
    let uri = "file:///symbols.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();

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

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Symbol extraction time: {:?}", duration);
    assert!(
        duration < MAX_SYMBOL_TIME,
        "Symbol extraction took {:?}, expected < {:?}",
        duration,
        MAX_SYMBOL_TIME
    );
}

#[test]
fn benchmark_goto_definition() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = r#"
my $var = 42;
sub function {
    my $local = 10;
    return $local;
}
print $var;
function();
"#;

    let uri = "file:///definition.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 6, "character": 7 }  // $var reference
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Go to definition time: {:?}", duration);
    assert!(
        duration < MAX_DEFINITION_TIME,
        "Go to definition took {:?}, expected < {:?}",
        duration,
        MAX_DEFINITION_TIME
    );
}

#[test]
fn benchmark_find_references() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = r#"
my $shared = 42;
sub func1 { return $shared; }
sub func2 { $shared = 100; }
sub func3 { print $shared; }
for (1..10) { $shared++; }
"#;

    let uri = "file:///references.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 },  // $shared declaration
                "context": { "includeDeclaration": true }
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Find references time: {:?}", duration);
    assert!(
        duration < MAX_REFERENCES_TIME,
        "Find references took {:?}, expected < {:?}",
        duration,
        MAX_REFERENCES_TIME
    );
}

#[test]
fn benchmark_hover() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = r#"
use strict;
my $var = 42;
print "Hello";
open(my $fh, '<', 'file.txt');
"#;

    let uri = "file:///hover.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 3, "character": 1 }  // print
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Hover time: {:?}", duration);
    assert!(
        duration < MAX_HOVER_TIME,
        "Hover took {:?}, expected < {:?}",
        duration,
        MAX_HOVER_TIME
    );
}

#[test]
fn benchmark_concurrent_requests() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open multiple documents
    for i in 0..10 {
        let code = format!("my $var_{} = {};\n", i, i);
        let uri = format!("file:///concurrent_{}.pl", i);

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
                        "text": code
                    }
                }
            }),
        );
    }

    let start = Instant::now();

    // Send multiple requests rapidly
    for i in 0..10 {
        let uri = format!("file:///concurrent_{}.pl", i);

        // Interleave different request types
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i * 3,
                "method": "textDocument/diagnostic",
                "params": {
                    "textDocument": { "uri": &uri }
                }
            }),
        );

        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i * 3 + 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": &uri }
                }
            }),
        );

        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i * 3 + 2,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": &uri },
                    "position": { "line": 0, "character": 3 }
                }
            }),
        );
    }

    // Read all responses
    for _ in 0..30 {
        let _ = read_response(&mut server);
    }

    let duration = start.elapsed();
    let avg_per_request = duration / 30;

    println!("Total time for 30 concurrent requests: {:?}", duration);
    println!("Average time per request: {:?}", avg_per_request);

    assert!(avg_per_request < Duration::from_millis(50), "Concurrent request handling too slow");
}

#[test]
fn benchmark_memory_usage() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open many large documents to test memory usage
    for i in 0..50 {
        let code = generators::generate_large_file(100); // ~4KB each
        let uri = format!("file:///memory_{}.pl", i);

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
                        "text": code
                    }
                }
            }),
        );
    }

    // Perform operations on all documents
    let start = Instant::now();

    for i in 0..50 {
        let uri = format!("file:///memory_{}.pl", i);

        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        );

        let _ = read_response(&mut server);
    }

    let duration = start.elapsed();

    println!("Time to process 50 documents: {:?}", duration);
    assert!(duration < Duration::from_secs(5), "Processing many documents took too long");
}

#[test]
fn benchmark_deep_nesting() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let deep_code = generators::generate_nested_code(50);
    let uri = "file:///deep.pl";

    let start = Instant::now();

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
                    "text": deep_code
                }
            }
        }),
    );

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let _ = read_response(&mut server);
    let duration = start.elapsed();

    println!("Deep nesting parse time: {:?}", duration);
    assert!(duration < Duration::from_millis(500), "Deep nesting took too long to parse");
}

/// Summary benchmark that runs all tests and reports overall performance
#[test]
fn benchmark_summary() {
    println!("\n=== LSP Performance Benchmark Summary ===\n");

    let benchmarks: Vec<(&str, fn() -> Duration)> = vec![
        ("Initialization", benchmark_initialization_time as fn() -> Duration),
        ("Simple file parsing", benchmark_simple_file_time as fn() -> Duration),
        ("Large file parsing", benchmark_large_file_time as fn() -> Duration),
        ("Incremental updates", benchmark_incremental_time as fn() -> Duration),
        ("Diagnostics", benchmark_diagnostics_time as fn() -> Duration),
        ("Symbol extraction", benchmark_symbols_time as fn() -> Duration),
        ("Go to definition", benchmark_definition_time as fn() -> Duration),
        ("Find references", benchmark_references_time as fn() -> Duration),
        ("Hover", benchmark_hover_time as fn() -> Duration),
        ("Concurrent requests", benchmark_concurrent_time as fn() -> Duration),
    ];

    for (name, benchmark_fn) in benchmarks {
        let duration = benchmark_fn();
        println!("{:.<30} {:>10.2?}", name, duration);
    }

    println!("\n=========================================\n");
}

// Helper functions for summary benchmark
fn benchmark_initialization_time() -> Duration {
    let start = Instant::now();
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);
    start.elapsed()
}

fn benchmark_simple_file_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "my $x = 42;\n";
    let uri = "file:///bench.pl";

    let start = Instant::now();
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
                    "text": code
                }
            }
        }),
    );
    start.elapsed()
}

fn benchmark_large_file_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = generators::generate_large_file(1000);
    let uri = "file:///large.pl";

    let start = Instant::now();
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
                    "text": code
                }
            }
        }),
    );
    start.elapsed()
}

fn benchmark_incremental_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "my $x = 1;\n";
    let uri = "file:///inc.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
    for i in 0..10 {
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": i + 2
                    },
                    "contentChanges": [{
                        "text": format!("{}my $y = {};\n", code, i)
                    }]
                }
            }),
        );
    }
    start.elapsed() / 10
}

fn benchmark_diagnostics_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "my $x = ;\n";
    let uri = "file:///diag.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let _ = read_response(&mut server);
    start.elapsed()
}

fn benchmark_symbols_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = generators::generate_symbols(20);
    let uri = "file:///sym.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
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
    let _ = read_response(&mut server);
    start.elapsed()
}

fn benchmark_definition_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "my $x = 42;\nprint $x;\n";
    let uri = "file:///def.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 7 }
            }
        }),
    );
    let _ = read_response(&mut server);
    start.elapsed()
}

fn benchmark_references_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "my $x = 42;\n$x++;\nprint $x;\n";
    let uri = "file:///ref.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let _ = read_response(&mut server);
    start.elapsed()
}

fn benchmark_hover_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let code = "print 'test';\n";
    let uri = "file:///hover.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );
    let _ = read_response(&mut server);
    start.elapsed()
}

fn benchmark_concurrent_time() -> Duration {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open a document
    let code = "my $x = 42;\n";
    let uri = "file:///concurrent.pl";

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
                    "text": code
                }
            }
        }),
    );

    let start = Instant::now();

    // Send 10 requests
    for i in 0..10 {
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 3 }
                }
            }),
        );
    }

    // Read all responses
    for _ in 0..10 {
        let _ = read_response(&mut server);
    }

    start.elapsed() / 10
}
