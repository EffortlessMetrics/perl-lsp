//! Edge case tests for LSP functionality
//!
//! Tests scenarios that might break or behave unexpectedly

use perl_parser::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};

fn setup_server() -> LspServer {
    let mut server = LspServer::new();
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    server.handle_request(request);
    server
}

fn open_doc(server: &mut LspServer, uri: &str, text: &str) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    server.handle_request(request);
}

#[allow(dead_code)]
fn get_diagnostics(server: &mut LspServer, uri: &str) -> Option<Value> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/diagnostic".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri}
        })),
    };

    server.handle_request(request).and_then(|response| response.result)
}

#[test]
fn test_empty_file_handling() {
    let mut server = setup_server();

    // Test completely empty file
    open_doc(&mut server, "file:///empty.pl", "");

    // Should not crash and should handle gracefully
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///empty.pl"}
        })),
    };

    let response = server.handle_request(request);
    assert!(response.is_some());
}

#[test]
fn test_malformed_perl_recovery() {
    let mut server = setup_server();

    // Test file with severe syntax errors
    let malformed = r#"
sub broken {
    my ($x = @_;  # Missing closing paren
    if ($x > 10   # Missing closing paren and brace
        print "hello;  # Missing quote
    # Missing closing brace for sub
    
my $var = ;  # Missing value
for (;;  # Incomplete for loop
"#;

    open_doc(&mut server, "file:///malformed.pl", malformed);

    // Server should not crash and should provide some diagnostics
    // Even if parsing fails, it should handle gracefully
}

#[test]
fn test_unicode_edge_cases() {
    let mut server = setup_server();

    // Test various Unicode scenarios
    let unicode_code = r#"
# Unicode in comments: 你好 мир 🌍
my $变量 = "Hello";
my $café = "coffee";
my $π = 3.14159;
sub Σ { return sum(@_); }
my $emoji = "🦀";
"#;

    open_doc(&mut server, "file:///unicode.pl", unicode_code);

    // Test that Unicode symbols work in navigation
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///unicode.pl"}
        })),
    };

    let response = server.handle_request(request);
    assert!(response.is_some());
}

#[test]
fn test_large_line_handling() {
    let mut server = setup_server();

    // Create a file with an extremely long line
    let long_line = "my $var = \"".to_string() + &"x".repeat(10000) + "\";";

    open_doc(&mut server, "file:///longline.pl", &long_line);

    // Should handle without stack overflow or excessive memory
}

#[test]
fn test_rapid_edits() {
    let mut server = setup_server();

    // Simulate rapid typing
    let uri = "file:///rapid.pl";
    open_doc(&mut server, uri, "");

    // Send many rapid updates
    for i in 0..100 {
        let text = format!("my $var{} = {};\n", i, i);
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: "textDocument/didChange".to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri,
                    "version": i + 2
                },
                "contentChanges": [{
                    "text": text
                }]
            })),
        };
        server.handle_request(request);
    }

    // Server should handle rapid updates without issues
}

#[test]
fn test_circular_references() {
    let mut server = setup_server();

    // Test code with potential circular references
    let circular = r#"
package A;
use B;
sub foo { B::bar(); }

package B;
use A;
sub bar { A::foo(); }
"#;

    open_doc(&mut server, "file:///circular.pl", circular);

    // Find references should not infinite loop
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/references".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///circular.pl"},
            "position": {"line": 2, "character": 10},
            "context": {"includeDeclaration": true}
        })),
    };

    let response = server.handle_request(request);
    assert!(response.is_some());
}

#[test]
fn test_special_variable_handling() {
    let mut server = setup_server();

    // Test Perl's special variables
    let special_vars = r#"
$_ = "default";
@_ = (1, 2, 3);
$@ = "error";
$! = "system error";
$$ = 12345;
$/ = "\n";
$\ = "";
$, = "";
$" = " ";
$; = "\034";
$# = "";
$% = 0;
$= = 60;
$- = 0;
$~ = "STDOUT";
$^ = "STDOUT_TOP";
$| = 0;
$. = 0;
"#;

    open_doc(&mut server, "file:///special.pl", special_vars);

    // These should be recognized but not cause issues
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///special.pl"}
        })),
    };

    let response = server.handle_request(request);
    assert!(response.is_some());
}

#[test]
fn test_heredoc_edge_cases() {
    let mut server = setup_server();

    // Test complex heredoc scenarios
    let heredocs = r#"
my $text = <<'END';
This is a heredoc with 'single quotes'
END

my $interpolated = <<"EOF";
Variable: $var
Array: @array
EOF

my $indented = <<~"INDENTED";
    This text
    is indented
    INDENTED

# Multiple heredocs
print <<'FIRST', <<SECOND;
First heredoc
FIRST
Second heredoc
SECOND
"#;

    open_doc(&mut server, "file:///heredoc.pl", heredocs);

    // Should parse heredocs correctly
}

#[test]
fn test_regex_with_special_delimiters() {
    let mut server = setup_server();

    // Test regex with various delimiters
    let regex_code = r#"
$text =~ m!pattern!;
$text =~ m{pattern}g;
$text =~ m[pattern]i;
$text =~ m<pattern>x;
$text =~ s|old|new|g;
$text =~ s#old#new#;
$text =~ tr/a-z/A-Z/;
$text =~ y/a-z/A-Z/;
"#;

    open_doc(&mut server, "file:///regex.pl", regex_code);

    // Should handle all regex delimiters
}

#[test]
fn test_incomplete_statements() {
    let mut server = setup_server();

    // Test code with incomplete statements (common while typing)
    let incomplete = r#"
my $x = 
sub foo {
    my $y = $x + 
    if ($y > 
        print
    for my $i (
"#;

    open_doc(&mut server, "file:///incomplete.pl", incomplete);

    // Should not crash on incomplete code
}

#[test]
fn test_mixed_encodings() {
    let mut server = setup_server();

    // Test file with mixed content
    let mixed = r#"
# -*- coding: utf-8 -*-
use utf8;
my $text = "Hello 世界";
my $latin1 = "café";
my $emoji = "🎉";
"#;

    open_doc(&mut server, "file:///mixed.pl", mixed);

    // Should handle different encodings
}

#[test]
fn test_boundary_positions() {
    let mut server = setup_server();

    let code = "my $x = 1;";
    open_doc(&mut server, "file:///boundary.pl", code);

    // Test position at start of file
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///boundary.pl"},
            "position": {"line": 0, "character": 0}
        })),
    };
    server.handle_request(request);

    // Test position at end of file
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///boundary.pl"},
            "position": {"line": 0, "character": 10}
        })),
    };
    server.handle_request(request);

    // Test position beyond end of line (should handle gracefully)
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///boundary.pl"},
            "position": {"line": 0, "character": 100}
        })),
    };
    server.handle_request(request);

    // Test position on non-existent line
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///boundary.pl"},
            "position": {"line": 10, "character": 0}
        })),
    };
    server.handle_request(request);
}

#[test]
fn test_concurrent_file_operations() {
    let mut server = setup_server();

    // Open multiple files
    for i in 0..10 {
        let uri = format!("file:///file{}.pl", i);
        let code = format!("my $var{} = {};", i, i);
        open_doc(&mut server, &uri, &code);
    }

    // Perform operations on different files
    for i in 0..10 {
        let uri = format!("file:///file{}.pl", i);
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(i)),
            method: "textDocument/documentSymbol".to_string(),
            params: Some(json!({
                "textDocument": {"uri": uri}
            })),
        };
        server.handle_request(request);
    }

    // Should handle multiple files without confusion
}
