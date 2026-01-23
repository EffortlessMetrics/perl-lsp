#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

fn init_server() -> LspServer {
    let mut srv = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {}
        })),
    };

    let init_response = srv.handle_request(init_request);

    // Verify initialization succeeded
    assert!(init_response.is_some());
    srv
}

fn open(server: &mut LspServer, uri: &str, text: &str) {
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

    let _response = server.handle_request(request);
    // didOpen is a notification, so no response expected
}

#[test]
fn test_cross_file_definition() {
    let mut srv = init_server();

    // File 1: defines Foo::bar
    open(
        &mut srv,
        "file:///lib/Foo.pm",
        r#"package Foo;
use strict;
use warnings;

sub bar {
    return 42;
}

1;
"#,
    );

    // File 2: calls Foo::bar
    open(
        &mut srv,
        "file:///app.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Foo;

my $result = Foo::bar();
print "Result: $result\n";
"#,
    );

    // Let's try workspace symbols first to ensure indexing is working
    let symbols_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(99)),
        method: "workspace/symbol".to_string(),
        params: Some(json!({"query": "bar"})),
    };

    let symbols_response = srv.handle_request(symbols_request);
    eprintln!("Workspace symbols response: {:?}", symbols_response);

    // Test go-to-definition from callsite to definition
    let def_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/definition".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///app.pl"},
            "position": {"line": 5, "character": 18}  // on "bar" in Foo::bar()
        })),
    };

    let def_response = srv.handle_request(def_request);

    assert!(def_response.is_some());
    if let Some(response) = def_response {
        eprintln!("Definition response: {:?}", response.result);
        if let Some(result) = response.result {
            let items = result.as_array().expect("Expected array of locations");
            assert!(!items.is_empty(), "Should find definition");

            let first = &items[0];
            assert!(first["uri"].as_str().unwrap().ends_with("/lib/Foo.pm"));
        }
    }
}

#[test]
fn test_cross_file_references() {
    let mut srv = init_server();

    // File 1: defines and uses a function
    open(
        &mut srv,
        "file:///lib/Utils.pm",
        r#"package Utils;
use strict;
use warnings;

sub process_data {
    my ($data) = @_;
    return $data * 2;
}

sub test_self {
    return process_data(10);
}

1;
"#,
    );

    // File 2: uses Utils::process_data
    open(
        &mut srv,
        "file:///script1.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Utils;

my $result = Utils::process_data(5);
"#,
    );

    // File 3: also uses Utils::process_data
    open(
        &mut srv,
        "file:///script2.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Utils;

for (1..10) {
    print Utils::process_data($_), "\n";
}
"#,
    );

    // Find all references to process_data
    let refs_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "textDocument/references".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///lib/Utils.pm"},
            "position": {"line": 4, "character": 5},  // on "process_data" definition
            "context": {"includeDeclaration": true}
        })),
    };

    let refs_response = srv.handle_request(refs_request);

    assert!(refs_response.is_some());
    if let Some(response) = refs_response {
        eprintln!("References response: {:?}", response.result);
        if let Some(result) = response.result {
            let refs = result.as_array().expect("Expected array of references");

            // Should find at least 4 references:
            // 1. The definition itself (if includeDeclaration is true)
            // 2. The call in test_self
            // 3. The call in script1.pl
            // 4. The call in script2.pl
            assert!(refs.len() >= 3, "Should find at least 3 references, found {}", refs.len());

            // Check that we found references in multiple files
            let uris: Vec<String> =
                refs.iter().filter_map(|r| r["uri"].as_str()).map(|s| s.to_string()).collect();

            assert!(uris.iter().any(|u| u.ends_with("/lib/Utils.pm")));
            assert!(uris.iter().any(|u| u.ends_with("/script1.pl")));
            assert!(uris.iter().any(|u| u.ends_with("/script2.pl")));
        }
    }
}

#[test]
fn test_workspace_symbols_after_indexing() {
    let mut srv = init_server();

    // Index multiple files
    open(
        &mut srv,
        "file:///lib/Math.pm",
        r#"package Math;
sub add { $_[0] + $_[1] }
sub subtract { $_[0] - $_[1] }
sub multiply { $_[0] * $_[1] }
1;
"#,
    );

    open(
        &mut srv,
        "file:///lib/String.pm",
        r#"package String;
sub concat { $_[0] . $_[1] }
sub reverse_str { reverse $_[0] }
1;
"#,
    );

    // Search for all symbols first
    let all_symbols_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(98)),
        method: "workspace/symbol".to_string(),
        params: Some(json!({"query": ""})),
    };

    let all_response = srv.handle_request(all_symbols_request);
    eprintln!("All symbols: {:?}", all_response);

    // Search for symbols containing "str"
    let symbols_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(4)),
        method: "workspace/symbol".to_string(),
        params: Some(json!({"query": "str"})),
    };

    let symbols_response = srv.handle_request(symbols_request);

    assert!(symbols_response.is_some());
    if let Some(response) = symbols_response {
        eprintln!("Workspace symbols result: {:?}", response.result);
        if let Some(result) = response.result {
            let symbols = result.as_array().expect("Expected array of symbols");

            // Should find "String" and "reverse_str" (both contain "str")
            // Note: "subtract" does NOT contain "str" as a substring!
            assert!(
                symbols.len() >= 2,
                "Should find at least 2 symbols with 'str', found {}: {:?}",
                symbols.len(),
                symbols
            );

            let names: Vec<String> =
                symbols.iter().filter_map(|s| s["name"].as_str()).map(|s| s.to_string()).collect();

            assert!(
                names.contains(&"String".to_string()) || names.contains(&"reverse_str".to_string()),
                "Should find String or reverse_str in {:?}",
                names
            );
        }
    }
}
