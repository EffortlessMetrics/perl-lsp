//! Tests for textDocument/foldingRange LSP feature

use perl_lsp::{JsonRpcRequest, LspServer};
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

    // Send initialized notification (required after successful initialize)
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
fn test_folding_ranges_subroutines() {
    let mut server = setup_server();

    let content = r#"
sub hello {
    print "Hello, World!\n";
    my $x = 42;
    return $x;
}

sub nested {
    my $a = 1;
    if ($a > 0) {
        print "Positive\n";
        while ($a < 10) {
            print "$a\n";
            $a++;
        }
    }
    return $a;
}
"#;

    open_document(&mut server, "file:///test.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();

    // Check that we have folding ranges
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Should have ranges for both subroutines, the if block, and the while block
    assert!(ranges.len() >= 4, "Expected at least 4 folding ranges, got {}", ranges.len());

    // Check that subroutine ranges exist
    let has_sub_range = ranges
        .iter()
        .any(|r| r["startLine"].as_u64() == Some(1) && r["endLine"].as_u64().is_some());
    assert!(has_sub_range, "Should have folding range for first subroutine");
}

#[cfg(feature = "lsp-extras")]
#[test]
fn test_folding_ranges_blocks() {
    let mut server = setup_server();

    let content = r#"
if ($condition) {
    print "True\n";
    my $x = 1;
    my $y = 2;
};

while ($count < 10) {
    print "$count\n";
    $count++;
};

for (my $i = 0; $i < 5; $i++) {
    print "$i\n";
};

foreach my $item (@items) {
    process($item);
};
"#;

    open_document(&mut server, "file:///blocks.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///blocks.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Should have ranges for if, while, for, and foreach blocks
    assert!(ranges.len() >= 4, "Expected at least 4 folding ranges for control structures");
}

#[test]
fn test_folding_ranges_packages() {
    let mut server = setup_server();

    let content = r#"
package MyModule {
    sub new {
        my $class = shift;
        return bless {}, $class;
    }
    
    sub method {
        my $self = shift;
        print "Method called\n";
    }
}

package AnotherModule {
    sub function {
        print "Function\n";
    }
}
"#;

    open_document(&mut server, "file:///packages.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///packages.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Should have ranges for both packages and their subroutines
    assert!(ranges.len() >= 2, "Expected at least 2 folding ranges for packages");
}

#[test]
fn test_folding_ranges_try_catch() {
    let mut server = setup_server();

    let content = r#"
try {
    dangerous_operation();
    another_operation();
} catch ($e) {
    log_error($e);
    recover();
} finally {
    cleanup();
}
"#;

    open_document(&mut server, "file:///try.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///try.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Should have ranges for try, catch, and finally blocks
    assert!(!ranges.is_empty(), "Expected folding ranges for try-catch-finally");
}

#[test]
fn test_folding_ranges_data_structures() {
    let mut server = setup_server();

    let content = r#"
my @array = (
    1,
    2,
    3,
    4,
    5
);

my %hash = (
    name => 'John',
    age => 30,
    city => 'New York',
    country => 'USA'
);
"#;

    open_document(&mut server, "file:///data.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///data.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Should have ranges for multi-line array and hash literals
    assert!(ranges.len() >= 2, "Expected folding ranges for array and hash literals");
}

#[test]
fn test_folding_ranges_imports() {
    let mut server = setup_server();

    let content = r#"
use strict;
use warnings;
use feature 'say';
use Data::Dumper;
use File::Path qw(make_path);
use List::Util qw(sum max min);

sub main {
    say "Hello";
}
"#;

    open_document(&mut server, "file:///imports.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///imports.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Check for import group folding
    let has_import_range =
        ranges.iter().any(|r| r.get("kind").and_then(|k| k.as_str()) == Some("imports"));
    assert!(has_import_range, "Should have folding range for import group");
}

#[test]
fn test_folding_ranges_empty_document() {
    let mut server = setup_server();

    open_document(&mut server, "file:///empty.pl", "");

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/foldingRange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///empty.pl"
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(request).unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.is_array());
    let ranges = result.as_array().unwrap();

    // Empty document should have no folding ranges
    assert_eq!(ranges.len(), 0, "Empty document should have no folding ranges");
}
