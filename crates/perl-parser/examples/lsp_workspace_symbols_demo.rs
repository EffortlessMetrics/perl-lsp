//! Demo showing how to add workspace symbols to the LSP server

use perl_parser::{LspServer, JsonRpcRequest, JsonRpcResponse, WorkspaceSymbolsProvider};
use serde_json::json;

fn main() {
    println!("=== LSP Workspace Symbols Demo ===\n");
    
    // This demonstrates how to extend the LSP server with workspace symbols
    
    // 1. Create the workspace symbols provider
    let mut workspace_symbols = WorkspaceSymbolsProvider::new();
    
    // 2. Index some sample files
    let files = vec![
        ("file:///project/lib/Utils.pm", r#"
package Utils;

sub trim {
    my $str = shift;
    $str =~ s/^\s+|\s+$//g;
    return $str;
}

sub encode_html {
    my $text = shift;
    # ... encoding logic
}

1;
"#),
        ("file:///project/lib/Database.pm", r#"
package Database;

sub connect {
    my ($host, $port) = @_;
    # ... connection logic
}

sub query {
    my ($sql, @params) = @_;
    # ... query logic
}

1;
"#),
        ("file:///project/scripts/main.pl", r#"
#!/usr/bin/perl
use strict;
use warnings;

use lib '../lib';
use Utils;
use Database;

sub main {
    my $db = Database::connect('localhost', 5432);
    # ... main logic
}

sub process_data {
    my ($data) = @_;
    # ... processing logic
}

main();
"#),
    ];
    
    // Index each file
    for (uri, content) in &files {
        println!("Indexing: {}", uri);
        let mut parser = perl_parser::Parser::new(content);
        if let Ok(ast) = parser.parse() {
            workspace_symbols.index_document(uri, &ast);
        }
    }
    
    println!("\n--- Testing workspace symbol searches ---\n");
    
    // Test various searches
    let searches = vec![
        ("", "All symbols"),
        ("con", "Functions starting with 'con'"),
        ("query", "Exact match"),
        ("db", "Fuzzy match"),
        ("Utils", "Package name"),
    ];
    
    for (query, description) in searches {
        println!("Search '{}' ({})", query, description);
        let results = workspace_symbols.search(query);
        for symbol in &results {
            println!("  - {} ({})", symbol.name, symbol.location.uri);
        }
        println!();
    }
    
    // 3. Show how to handle LSP request
    println!("--- Handling LSP workspace/symbol request ---\n");
    
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "workspace/symbol".to_string(),
        params: Some(json!({
            "query": "connect"
        })),
    };
    
    // In a real implementation, add this to LspServer::handle_request:
    // "workspace/symbol" => self.handle_workspace_symbols(request.params),
    
    let response = handle_workspace_symbols(&workspace_symbols, request.params);
    println!("Request: workspace/symbol with query 'connect'");
    println!("Response: {}", serde_json::to_string_pretty(&response).unwrap());
}

fn handle_workspace_symbols(
    provider: &WorkspaceSymbolsProvider,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let query = params
        .and_then(|p| p.get("query"))
        .and_then(|q| q.as_str())
        .unwrap_or("");
    
    let symbols = provider.search(query);
    
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        result: Some(json!(symbols)),
        error: None,
    }
}