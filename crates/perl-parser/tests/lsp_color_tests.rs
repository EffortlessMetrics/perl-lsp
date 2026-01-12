//! LSP Color Detection Tests
//!
//! Tests for textDocument/documentColor and textDocument/colorPresentation
//! LSP 3.18 features for detecting and presenting color literals in Perl code.

use parking_lot::Mutex;
use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::sync::Arc;

/// Helper to send a request and get result
fn send_request(server: &mut LspServer, method: &str, params: Value) -> Result<Value, String> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params: Some(params),
    };

    let response = server.handle_request(request).expect("Should get response");
    if let Some(error) = response.error {
        return Err(error.message);
    }
    response.result.ok_or_else(|| "No result".to_string())
}

/// Create a test server with a document containing color codes
fn setup_color_server() -> LspServer {
    let output = Arc::new(Mutex::new(Box::new(Vec::new()) as Box<dyn std::io::Write + Send>));
    let mut server = LspServer::with_output(output);

    // Initialize the server
    send_request(
        &mut server,
        "initialize",
        json!({
            "capabilities": {},
            "processId": 12345,
            "rootUri": "file:///test"
        }),
    )
    .ok();

    // Send initialized notification
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_request);

    // Open a document with various color formats
    let text = r#"#!/usr/bin/perl
# Red color: #FF0000
# Green: #00FF00
# Blue with alpha: #0000FFAA
# Short form: #F00
# ANSI red: print "\e[31mRed text\e[0m";
# ANSI green: print "\e[32mGreen text\e[0m";
# More colors in comments: #ABCDEF #123456
"#;

    let did_open = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None, // Notification
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test/colors.pl",
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    server.handle_request(did_open);
    server
}

#[test]
fn lsp_color_detect_hex_colors() {
    let mut server = setup_color_server();

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            }
        }),
    )
    .unwrap();

    let color_array = result.as_array().expect("Should be an array");

    // Should detect at least the hex colors in comments
    // We expect: #FF0000, #00FF00, #0000FFAA, #F00, #ABCDEF, #123456
    assert!(
        color_array.len() >= 5,
        "Should detect multiple hex colors, found {}",
        color_array.len()
    );

    // Verify first color (#FF0000 - red)
    let first_color = &color_array[0];
    assert!(first_color["color"]["red"].as_f64().unwrap() > 0.99);
    assert!(first_color["color"]["green"].as_f64().unwrap() < 0.01);
    assert!(first_color["color"]["blue"].as_f64().unwrap() < 0.01);

    // Verify range information exists
    assert!(first_color["range"]["start"]["line"].is_number());
    assert!(first_color["range"]["start"]["character"].is_number());
}

#[test]
fn lsp_color_detect_ansi_colors() {
    let mut server = setup_color_server();

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            }
        }),
    )
    .unwrap();

    let color_array = result.as_array().expect("Should be an array");

    // Should detect ANSI color codes: \e[31m and \e[32m
    // Find ANSI colors in the results (they appear on lines 5-6)
    let ansi_colors: Vec<&Value> = color_array
        .iter()
        .filter(|c| {
            let line = c["range"]["start"]["line"].as_u64().unwrap_or(0);
            line >= 5 && line <= 6
        })
        .collect();

    assert!(ansi_colors.len() >= 1, "Should detect at least one ANSI color");
}

#[test]
fn lsp_color_presentation_hex() {
    let mut server = setup_color_server();

    // Test color presentation for pure red
    let result = send_request(
        &mut server,
        "textDocument/colorPresentation",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            },
            "color": {
                "red": 1.0,
                "green": 0.0,
                "blue": 0.0,
                "alpha": 1.0
            },
            "range": {
                "start": {"line": 1, "character": 16},
                "end": {"line": 1, "character": 23}
            }
        }),
    )
    .unwrap();

    let pres_array = result.as_array().expect("Should be an array");

    // Should provide multiple presentation formats
    assert!(pres_array.len() >= 3, "Should provide at least 3 presentation formats");

    // Check for hex format
    let labels: Vec<String> =
        pres_array.iter().filter_map(|p| p["label"].as_str().map(String::from)).collect();

    assert!(labels.iter().any(|l| l.starts_with('#')), "Should have hex format");
    assert!(labels.iter().any(|l| l.starts_with("rgb(")), "Should have RGB format");
    assert!(labels.iter().any(|l| l.starts_with("hsl(")), "Should have HSL format");
}

#[test]
fn lsp_color_presentation_with_alpha() {
    let mut server = setup_color_server();

    // Test color presentation with alpha channel
    let result = send_request(
        &mut server,
        "textDocument/colorPresentation",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            },
            "color": {
                "red": 0.5,
                "green": 0.5,
                "blue": 0.5,
                "alpha": 0.5
            },
            "range": {
                "start": {"line": 3, "character": 20},
                "end": {"line": 3, "character": 29}
            }
        }),
    )
    .unwrap();

    let pres_array = result.as_array().expect("Should be an array");

    let labels: Vec<String> =
        pres_array.iter().filter_map(|p| p["label"].as_str().map(String::from)).collect();

    // Should include RGBA/HSLA formats when alpha < 1.0
    assert!(
        labels.iter().any(|l| l.contains("rgba") || (l.len() == 9 && l.starts_with('#'))),
        "Should have RGBA or 8-digit hex format"
    );
}

#[test]
fn lsp_color_empty_document() {
    let output = Arc::new(Mutex::new(Box::new(Vec::new()) as Box<dyn std::io::Write + Send>));
    let mut server = LspServer::with_output(output);

    // Initialize
    send_request(
        &mut server,
        "initialize",
        json!({
            "capabilities": {},
            "processId": 12345,
            "rootUri": "file:///test"
        }),
    )
    .ok();

    // Send initialized notification
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_request);

    // Open empty document
    let did_open = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test/empty.pl",
                "languageId": "perl",
                "version": 1,
                "text": ""
            }
        })),
    };
    server.handle_request(did_open);

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/empty.pl"
            }
        }),
    )
    .unwrap();

    let color_array = result.as_array().expect("Should be an array");
    assert_eq!(color_array.len(), 0, "Empty document should have no colors");
}

#[test]
fn lsp_color_invalid_params() {
    let mut server = setup_color_server();

    // Missing textDocument field
    let result = send_request(&mut server, "textDocument/documentColor", json!({}));
    assert!(result.is_err(), "Should return error for missing textDocument");

    // Missing color field in presentation
    let result = send_request(
        &mut server,
        "textDocument/colorPresentation",
        json!({
            "textDocument": {"uri": "file:///test/colors.pl"}
        }),
    );
    assert!(result.is_err(), "Should return error for missing color");
}

#[test]
fn lsp_color_round_trip() {
    let mut server = setup_color_server();

    // Get colors from document
    let detected = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            }
        }),
    )
    .unwrap();

    let colors = detected.as_array().expect("Should be an array");

    if let Some(first_color) = colors.first() {
        // Use detected color to get presentations
        let presentations = send_request(
            &mut server,
            "textDocument/colorPresentation",
            json!({
                "textDocument": {
                    "uri": "file:///test/colors.pl"
                },
                "color": first_color["color"],
                "range": first_color["range"]
            }),
        )
        .unwrap();

        let pres_array = presentations.as_array().unwrap();
        assert!(pres_array.len() > 0, "Should have at least one presentation");
    }
}
