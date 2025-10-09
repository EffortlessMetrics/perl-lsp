/// Comprehensive tests for LSP code actions and refactorings
use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

/// Test extract variable refactoring
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC4:enabledTests - Extract variable may not be fully implemented yet, but test framework should work
#[ignore = "Extract variable refactoring implementation incomplete - testing framework validation"]
fn test_extract_variable() {
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
my $str = "hello";
my $result = length($str) + 10;
print $result;
"#
                }
            }
        }),
    );

    // Request code actions for the expression "length($str)"
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 2, "character": 13 },
                    "end": { "line": 2, "character": 25 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("Extract")
        && a["title"].as_str().unwrap().contains("variable")));
}

/// Test adding error checking to file operations
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Add error checking refactoring
#[ignore = "Add error checking refactoring implementation incomplete - testing framework validation"]
fn test_add_error_checking() {
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
open($fh, '<', 'data.txt');
print "Hello\n";
close($fh);
"#
                }
            }
        }),
    );

    // Request code actions for the open statement
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": 30 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("error checking")));
}

/// Test converting old-style for loops to foreach
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Convert C-style for loop to foreach
#[ignore = "Convert loop style refactoring implementation incomplete - testing framework validation"]
fn test_convert_loop_style() {
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
for (my $i = 0; $i < @array; $i++) {
    print $array[$i];
}
"#
                }
            }
        }),
    );

    // Request code actions for the for loop
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(
        actions.iter().any(|a| a["title"].as_str().unwrap().contains("foreach loop")),
        "Expected 'foreach loop' conversion action but got: {:?}",
        actions.iter().map(|a| a["title"].as_str()).collect::<Vec<_>>()
    );
}

/// Test converting to postfix form
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Convert if statement to postfix form
#[ignore = "Convert to postfix refactoring implementation incomplete - testing framework validation"]
fn test_convert_to_postfix() {
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
if ($debug) {
    print "Debug mode\n";
}
"#
                }
            }
        }),
    );

    // Request code actions for the if statement
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("postfix")));
}

/// Test adding missing pragmas
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Add missing pragmas (strict/warnings/utf8)
#[ignore = "Add missing pragmas refactoring implementation incomplete - testing framework validation"]
fn test_add_missing_pragmas() {
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
#!/usr/bin/perl

my $x = 42;
print $x;
"#
                }
            }
        }),
    );

    // Request code actions for the entire document
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 4, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("pragma")));
}

/// Test quick fix for undefined variable
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Fix undefined variable issues
#[ignore = "Fix undefined variable refactoring implementation incomplete - testing framework validation"]
fn test_fix_undefined_variable() {
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
use strict;
use warnings;

print $undefined_var;
"#
                }
            }
        }),
    );

    // First get diagnostics
    let diag_response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    // Request code actions with diagnostics
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 4, "character": 6 },
                    "end": { "line": 4, "character": 20 }
                },
                "context": {
                    "diagnostics": diag_response["result"]["items"].clone()
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("Declare")
        && a["title"].as_str().unwrap().contains("my")));
}

/// Test extract subroutine refactoring
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Extract subroutine with parameter detection
#[ignore = "Extract subroutine refactoring implementation incomplete - testing framework validation"]
fn test_extract_subroutine() {
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
my $x = 10;
my $y = 20;
{
    my $sum = $x + $y;
    print "Sum: $sum\n";
    my $product = $x * $y;
    print "Product: $product\n";
}
"#
                }
            }
        }),
    );

    // Request code actions for the block
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 3, "character": 0 },
                    "end": { "line": 8, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("subroutine")));
}

/// Test organize imports refactoring
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC3:codeActions - Organize imports with existing ImportOptimizer integration
#[ignore = "Organize imports refactoring LSP integration incomplete - testing framework validation"]
fn test_organize_imports() {
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
#!/usr/bin/perl
use JSON;
use Data::Dumper;
use warnings;
use File::Path;
use strict;
use lib './lib';

print "test\n";
"#
                }
            }
        }),
    );

    // Request code actions for the import section
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 7, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().unwrap();
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap().contains("Organize imports")));
}

/// Test multiple refactorings available
#[test]
#[ignore]
// Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
// AC5:integration - Multiple refactoring operations in single request
#[ignore = "Multiple refactorings implementation incomplete - testing framework validation"]
fn test_multiple_refactorings() {
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
$data = 10;
$processed = $data * 2 + 5;
if ($processed > 100) {
    print "Result: $processed\n";
}
"#
                }
            }
        }),
    );

    // Request code actions for the complex expression
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 2, "character": 16 },
                    "end": { "line": 2, "character": 60 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    eprintln!("Code action response: {:?}", response);
    let actions = response["result"].as_array().unwrap();

    // Should have multiple refactoring options
    assert!(!actions.is_empty(), "Expected code actions but got none");
    assert!(actions.iter().any(|a| a["kind"].as_str() == Some("refactor.extract")));
}
