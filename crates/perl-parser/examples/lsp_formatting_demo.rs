//! Demo of LSP formatting capabilities

use perl_parser::LspServer;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

fn main() {
    println!("=== LSP Formatting Demo ===\n");
    
    // Create LSP server
    let mut server = LspServer::new();
    
    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "capabilities": {},
        "workspaceFolders": null
    });
    
    server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": init_params
    })).unwrap());
    
    // Mark as initialized
    server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    })).unwrap());
    
    // Open a document with unformatted code
    let unformatted_code = r#"sub calculate_total{my($items,$tax_rate)=@_;my$subtotal=0;foreach my$item(@$items){$subtotal+=$item->{price}*$item->{quantity};}my$tax=$subtotal*$tax_rate;return$subtotal+$tax;}"#;
    
    println!("Original code:");
    println!("{}\n", unformatted_code);
    
    server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": unformatted_code
            }
        }
    })).unwrap());
    
    // Request formatting
    println!("Requesting document formatting...\n");
    
    if let Some(response) = server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }
    })).unwrap()) {
        if let serde_json::Value::Object(resp) = serde_json::to_value(response).unwrap() {
            if let Some(result) = resp.get("result") {
                if let Some(edits) = result.as_array() {
                    if !edits.is_empty() {
                        println!("Formatting successful! {} edits returned", edits.len());
                        for (i, edit) in edits.iter().enumerate() {
                            println!("\nEdit {}:", i + 1);
                            if let Some(new_text) = edit["newText"].as_str() {
                                println!("{}", new_text);
                            }
                        }
                    } else {
                        println!("No formatting changes needed.");
                    }
                } else {
                    println!("Unexpected result format");
                }
            } else if let Some(error) = resp.get("error") {
                println!("Formatting error: {:?}", error);
            }
        }
    }
    
    // Test range formatting
    println!("\n\n=== Range Formatting Demo ===\n");
    
    let multi_line_code = r#"my $x = 1;
sub messy_function{my$a=shift;my$b=shift;return$a+$b;}
my $y = 2;"#;
    
    println!("Code with messy middle line:");
    println!("{}\n", multi_line_code);
    
    // Update document
    server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 2
            },
            "contentChanges": [{
                "text": multi_line_code
            }]
        }
    })).unwrap());
    
    // Request range formatting (just the middle line)
    println!("Requesting range formatting for line 2...\n");
    
    if let Some(response) = server.handle_request(serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/rangeFormatting",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            },
            "range": {
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 60 }
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }
    })).unwrap()) {
        if let serde_json::Value::Object(resp) = serde_json::to_value(response).unwrap() {
            if let Some(result) = resp.get("result") {
                if let Some(edits) = result.as_array() {
                    if !edits.is_empty() {
                        println!("Range formatting successful!");
                        for edit in edits {
                            if let Some(new_text) = edit["newText"].as_str() {
                                println!("Formatted line:");
                                println!("{}", new_text);
                            }
                        }
                    } else {
                        println!("No formatting changes needed for the range.");
                    }
                }
            }
        }
    }
    
    println!("\n=== Demo Complete ===");
    println!("\nNote: If perltidy is not installed, formatting will fail.");
    println!("Install perltidy with: cpan Perl::Tidy");
}