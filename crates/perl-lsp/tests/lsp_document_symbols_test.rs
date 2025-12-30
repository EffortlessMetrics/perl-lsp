//! Tests for textDocument/documentSymbol LSP feature
//!
//! These tests validate the document symbol provider functionality including:
//! - Basic symbol extraction (packages, subroutines, variables)
//! - Nested symbol structures (closures, multiple packages)
//! - Empty document handling
//! - Constants and labels
//! - All variable types (scalar, array, hash, our, local, state)
//! - Hierarchical symbol structures

use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

fn setup_server() -> LspServer {
    let mut server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    server.handle_request(init_request);

    // Send initialized notification per LSP 3.17 protocol requirements
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);

    server
}

fn open_document(server: &mut LspServer, uri: &str, content: &str) {
    let notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
        id: None,
    };

    server.handle_request(notification);
}

#[test]
fn test_document_symbols_basic() {
    let mut server = setup_server();

    let content = r#"
package MyModule;

use strict;
use warnings;

my $global_var = 42;
our @shared_array = (1, 2, 3);

sub hello {
    my $local = "world";
    print "Hello, $local\n";
}

sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

1;
"#;

    open_document(&mut server, "file:///test.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    // Check that we have symbols
    assert!(result.is_array());
    let symbols = result.as_array().unwrap();
    assert!(!symbols.is_empty());

    // Check for package symbol
    let package_symbol = symbols.iter().find(|s| s["name"].as_str() == Some("MyModule"));
    assert!(package_symbol.is_some());
    assert_eq!(package_symbol.unwrap()["kind"], 4); // Module

    // Check for subroutine symbols
    let hello_sub = symbols.iter().find(|s| s["name"].as_str() == Some("hello"));
    assert!(hello_sub.is_some());
    assert_eq!(hello_sub.unwrap()["kind"], 12); // Function

    let calc_sub = symbols.iter().find(|s| s["name"].as_str() == Some("calculate"));
    assert!(calc_sub.is_some());
    assert_eq!(calc_sub.unwrap()["kind"], 12); // Function

    // Check for variable symbols
    let global_var = symbols.iter().find(|s| s["name"].as_str() == Some("$global_var"));
    assert!(global_var.is_some());
    assert_eq!(global_var.unwrap()["kind"], 13); // Variable

    let shared_array = symbols.iter().find(|s| s["name"].as_str() == Some("@shared_array"));
    assert!(shared_array.is_some());
    assert_eq!(shared_array.unwrap()["kind"], 18); // Array
}

#[test]
fn test_document_symbols_nested() {
    let mut server = setup_server();

    let content = r#"
package Outer;

sub parent_sub {
    my $parent_var = 10;
    
    my $closure = sub {
        my $inner_var = 20;
        return $parent_var + $inner_var;
    };
    
    return $closure;
}

package Inner;

sub another_sub {
    my %hash = (key => 'value');
    return \%hash;
}

1;
"#;

    open_document(&mut server, "file:///nested.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///nested.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    let symbols = result.as_array().unwrap();

    // Check for both packages
    let outer_package = symbols.iter().find(|s| s["name"].as_str() == Some("Outer"));
    assert!(outer_package.is_some());

    let inner_package = symbols.iter().find(|s| s["name"].as_str() == Some("Inner"));
    assert!(inner_package.is_some());

    // Check for subroutines
    let parent_sub = symbols.iter().find(|s| s["name"].as_str() == Some("parent_sub"));
    assert!(parent_sub.is_some());

    let another_sub = symbols.iter().find(|s| s["name"].as_str() == Some("another_sub"));
    assert!(another_sub.is_some());
}

#[test]
fn test_document_symbols_empty_document() {
    let mut server = setup_server();

    open_document(&mut server, "file:///empty.pl", "");

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///empty.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    // Should return empty array for empty document
    assert!(result.is_array());
    let symbols = result.as_array().unwrap();
    assert!(symbols.is_empty());
}

#[test]
fn test_document_symbols_with_constants() {
    let mut server = setup_server();

    let content = r#"
use constant PI => 3.14159;
use constant {
    TRUE => 1,
    FALSE => 0,
};

sub area {
    my $radius = shift;
    return PI * $radius * $radius;
}
"#;

    open_document(&mut server, "file:///constants.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///constants.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    let symbols = result.as_array().unwrap();

    // Check for function
    let area_sub = symbols.iter().find(|s| s["name"].as_str() == Some("area"));
    assert!(area_sub.is_some());
    assert_eq!(area_sub.unwrap()["kind"], 12); // Function
}

#[test]
fn test_document_symbols_with_labels() {
    let mut server = setup_server();

    let content = r#"
OUTER: for my $i (1..10) {
    INNER: for my $j (1..10) {
        next OUTER if $i + $j > 15;
        last INNER if $j > 5;
    }
}

sub process {
    goto DONE if !@_;
    # process...
    DONE: return;
}
"#;

    open_document(&mut server, "file:///labels.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///labels.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    let symbols = result.as_array().unwrap();

    // Check for subroutine
    let process_sub = symbols.iter().find(|s| s["name"].as_str() == Some("process"));
    assert!(process_sub.is_some());
}

#[test]
fn test_document_symbols_all_variable_types() {
    let mut server = setup_server();

    let content = r#"
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');

our $shared_scalar = "shared";
our @shared_array = ();
our %shared_hash = ();

local $/ = "\n";
state $persistent = 0;
"#;

    open_document(&mut server, "file:///variables.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///variables.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    let symbols = result.as_array().unwrap();

    // Check for scalar variables
    let scalar = symbols.iter().find(|s| s["name"].as_str() == Some("$scalar"));
    assert!(scalar.is_some());
    assert_eq!(scalar.unwrap()["kind"], 13); // Variable

    // Check for array variables
    let array = symbols.iter().find(|s| s["name"].as_str() == Some("@array"));
    assert!(array.is_some());
    assert_eq!(array.unwrap()["kind"], 18); // Array

    // Check for hash variables
    let hash = symbols.iter().find(|s| s["name"].as_str() == Some("%hash"));
    assert!(hash.is_some());
    assert_eq!(hash.unwrap()["kind"], 19); // Object (closest to hash)

    // Check for shared variables
    let shared_scalar = symbols.iter().find(|s| s["name"].as_str() == Some("$shared_scalar"));
    assert!(shared_scalar.is_some());

    let shared_array = symbols.iter().find(|s| s["name"].as_str() == Some("@shared_array"));
    assert!(shared_array.is_some());

    let shared_hash = symbols.iter().find(|s| s["name"].as_str() == Some("%shared_hash"));
    assert!(shared_hash.is_some());
}

#[test]
fn test_document_symbols_hierarchical_structure() {
    let mut server = setup_server();

    let content = r#"
package Parent;

my $package_var = 1;

sub parent_method {
    my $method_var = 2;
    
    if (1) {
        my $block_var = 3;
    }
}

package Child;

sub child_method {
    my $child_var = 4;
}
"#;

    open_document(&mut server, "file:///hierarchy.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///hierarchy.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();

    let symbols = result.as_array().unwrap();

    // Check that we have the expected top-level symbols
    assert!(symbols.iter().any(|s| s["name"].as_str() == Some("Parent")));
    assert!(symbols.iter().any(|s| s["name"].as_str() == Some("Child")));
    assert!(symbols.iter().any(|s| s["name"].as_str() == Some("parent_method")));
    assert!(symbols.iter().any(|s| s["name"].as_str() == Some("child_method")));
    assert!(symbols.iter().any(|s| s["name"].as_str() == Some("$package_var")));
}
