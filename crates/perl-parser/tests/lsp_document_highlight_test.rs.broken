//! Real tests for Document Highlight feature
//! Tests that the LSP server correctly highlights all occurrences of a symbol

use perl_parser::lsp_server::LspServer;
use serde_json::{json, Value};
use std::io::Cursor;

mod support;
use support::*;

/// Helper to set up LSP server with document
fn setup_server_with_document(content: &str) -> (LspServer, String) {
    let output = Cursor::new(Vec::new());
    let mut server = LspServer::new(Box::new(output));
    
    // Initialize server
    let init_result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    })).unwrap();
    assert!(init_result.is_some());
    
    // Open document
    let uri = "file:///test.pl";
    server.handle_request(json!({
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
    })).unwrap();
    
    (server, uri.to_string())
}

#[test]
fn test_document_highlight_variable() {
    let content = r#"my $foo = 42;
print $foo;
$foo = $foo + 1;
my $bar = $foo * 2;"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position of first $foo
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 } // Position of $foo in "my $foo"
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should find 5 occurrences of $foo
    assert_eq!(highlights_arr.len(), 5, "Should find 5 occurrences of $foo");
    
    // Verify each highlight has correct structure
    for highlight in highlights_arr {
        assert!(highlight.is_object());
        let obj = highlight.as_object().unwrap();
        assert!(obj.contains_key("range"));
        assert!(obj.contains_key("kind"));
        
        let kind = obj["kind"].as_u64().unwrap();
        assert!(kind >= 1 && kind <= 3, "Kind should be Text(1), Read(2), or Write(3)");
    }
}

#[test]
fn test_document_highlight_subroutine() {
    let content = r#"sub calculate {
    return 42;
}

my $result = calculate();
calculate();
print "Result: ", calculate();"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position of first 'calculate'
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 5 } // Position of 'calculate' in "sub calculate"
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should find 4 occurrences of 'calculate'
    assert_eq!(highlights_arr.len(), 4, "Should find 4 occurrences of 'calculate'");
}

#[test]
fn test_document_highlight_method_call() {
    let content = r#"my $obj = MyClass->new();
$obj->process();
$obj->process(42);
my $other = OtherClass->new();
$other->process();"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position of 'process' method
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 7 } // Position of 'process' in "$obj->process()"
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should find all 'process' method calls
    assert!(highlights_arr.len() >= 2, "Should find at least 2 occurrences of 'process' method");
}

#[test]
fn test_document_highlight_package() {
    let content = r#"package MyPackage;

sub new {
    my $class = shift;
    return bless {}, $class;
}

package main;
use MyPackage;
my $obj = MyPackage->new();"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position of 'MyPackage'
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 9 } // Position of 'MyPackage' in "package MyPackage"
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should find 3 occurrences of 'MyPackage'
    assert!(highlights_arr.len() >= 2, "Should find at least 2 occurrences of 'MyPackage'");
}

#[test]
fn test_document_highlight_no_symbol() {
    let content = r#"# This is a comment
my $foo = 42;"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position within comment
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 5 } // Position within comment
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should return empty array for non-symbol positions
    assert_eq!(highlights_arr.len(), 0, "Should return empty array for non-symbol positions");
}

#[test]
fn test_document_highlight_write_vs_read() {
    let content = r#"my $counter = 0;
$counter = 10;
print $counter;
$counter++;"#;
    
    let (mut server, uri) = setup_server_with_document(content);
    
    // Request highlight at position of $counter
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 } // Position of $counter
        }
    })).unwrap();
    
    let highlights = response.unwrap();
    assert!(highlights.is_array());
    let highlights_arr = highlights.as_array().unwrap();
    
    // Should find 4 occurrences
    assert_eq!(highlights_arr.len(), 4, "Should find 4 occurrences of $counter");
    
    // Check that we have both read and write kinds
    let mut has_write = false;
    let mut has_read = false;
    
    for highlight in highlights_arr {
        let kind = highlight["kind"].as_u64().unwrap();
        if kind == 3 { has_write = true; }
        if kind == 2 || kind == 1 { has_read = true; }
    }
    
    assert!(has_write, "Should have at least one write highlight");
    assert!(has_read, "Should have at least one read highlight");
}