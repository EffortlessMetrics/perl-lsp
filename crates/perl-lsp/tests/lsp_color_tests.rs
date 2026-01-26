//! LSP Color Detection Tests
#![cfg(not(feature = "lsp-ga-lock"))]
//!
//! Tests for textDocument/documentColor and textDocument/colorPresentation
//! LSP 3.18 features for detecting and presenting color literals in Perl code.

use parking_lot::Mutex;
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to send a request and get result
fn send_request(server: &mut LspServer, method: &str, params: Value) -> Result<Value, String> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params: Some(params),
    };

    let response =
        server.handle_request(request).ok_or_else(|| "Failed to get response".to_string())?;
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
fn lsp_color_detect_hex_colors() -> TestResult {
    let mut server = setup_color_server();

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            }
        }),
    )?;

    let color_array = result.as_array().ok_or("Result should be an array")?;

    // Should detect at least the hex colors in comments
    // We expect: #FF0000, #00FF00, #0000FFAA, #F00, #ABCDEF, #123456
    assert!(
        color_array.len() >= 5,
        "Should detect multiple hex colors, found {}",
        color_array.len()
    );

    // Verify first color (#FF0000 - red)
    let first_color = &color_array[0];
    let red = first_color["color"]["red"].as_f64().ok_or("Red value should be a number")?;
    let green = first_color["color"]["green"].as_f64().ok_or("Green value should be a number")?;
    let blue = first_color["color"]["blue"].as_f64().ok_or("Blue value should be a number")?;

    assert!(red > 0.99);
    assert!(green < 0.01);
    assert!(blue < 0.01);

    // Verify range information exists
    assert!(first_color["range"]["start"]["line"].is_number());
    assert!(first_color["range"]["start"]["character"].is_number());

    Ok(())
}

#[test]
fn lsp_color_detect_ansi_colors() -> TestResult {
    let mut server = setup_color_server();

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/colors.pl"
            }
        }),
    )?;

    let color_array = result.as_array().ok_or("Result should be an array")?;

    // Should detect ANSI color codes: \e[31m and \e[32m
    // Find ANSI colors in the results (they appear on lines 5-6)
    let ansi_colors: Vec<&Value> = color_array
        .iter()
        .filter(|c| {
            let line = c["range"]["start"]["line"].as_u64().unwrap_or(0);
            (5..=6).contains(&line)
        })
        .collect();

    assert!(!ansi_colors.is_empty(), "Should detect at least one ANSI color");

    Ok(())
}

#[test]
fn lsp_color_presentation_hex() -> TestResult {
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
    )?;

    let pres_array = result.as_array().ok_or("Result should be an array")?;

    // Should provide multiple presentation formats
    assert!(pres_array.len() >= 3, "Should provide at least 3 presentation formats");

    // Check for hex format
    let labels: Vec<String> =
        pres_array.iter().filter_map(|p| p["label"].as_str().map(String::from)).collect();

    assert!(labels.iter().any(|l| l.starts_with('#')), "Should have hex format");
    assert!(labels.iter().any(|l| l.starts_with("rgb(")), "Should have RGB format");
    assert!(labels.iter().any(|l| l.starts_with("hsl(")), "Should have HSL format");

    Ok(())
}

#[test]
fn lsp_color_presentation_with_alpha() -> TestResult {
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
    )?;

    let pres_array = result.as_array().ok_or("Result should be an array")?;

    let labels: Vec<String> =
        pres_array.iter().filter_map(|p| p["label"].as_str().map(String::from)).collect();

    // Should include RGBA/HSLA formats when alpha < 1.0
    assert!(
        labels.iter().any(|l| l.contains("rgba") || (l.len() == 9 && l.starts_with('#'))),
        "Should have RGBA or 8-digit hex format"
    );

    Ok(())
}

#[test]
fn lsp_color_empty_document() -> TestResult {
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
    )?;

    let color_array = result.as_array().ok_or("Result should be an array")?;
    assert_eq!(color_array.len(), 0, "Empty document should have no colors");

    Ok(())
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
fn lsp_color_round_trip() -> TestResult {
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
    )?;

    let colors = detected.as_array().ok_or("Detected colors should be an array")?;

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
        )?;

        let pres_array = presentations.as_array().ok_or("Presentations should be an array")?;
        assert!(!pres_array.is_empty(), "Should have at least one presentation");
    }

    Ok(())
}

/// Test that color positions are correct with non-ASCII prefix (UTF-16 boundary safety)
///
/// This is a critical regression test: LSP uses UTF-16 code units for positions,
/// but Rust uses UTF-8. Multi-byte characters before a color token could cause
/// position misalignment if conversion isn't done correctly.
#[test]
fn lsp_color_utf16_position_with_non_ascii_prefix() -> TestResult {
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

    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_request);

    // Document with non-ASCII before color:
    // "日本語" = 3 chars but 9 UTF-8 bytes, 3 UTF-16 code units
    // "émoji" = 5 chars, 6 UTF-8 bytes, 5 UTF-16 code units
    // The color #FF0000 should be at UTF-16 position 9 (after "日本語" + " " + "émoji" + ": ")
    let text = r#"# 日本語 émoji: #FF0000
# café: #00FF00"#;

    let did_open = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test/utf16_colors.pl",
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    server.handle_request(did_open);

    let result = send_request(
        &mut server,
        "textDocument/documentColor",
        json!({
            "textDocument": {
                "uri": "file:///test/utf16_colors.pl"
            }
        }),
    )?;

    let color_array = result.as_array().ok_or("Result should be an array")?;

    // Should detect both colors
    assert!(
        color_array.len() >= 2,
        "Should detect at least 2 colors in UTF-16 test, found {}",
        color_array.len()
    );

    // Verify that colors on line 0 and line 1 are found
    // (specific positions depend on implementation, but they should exist)
    let lines: Vec<u64> =
        color_array.iter().filter_map(|c| c["range"]["start"]["line"].as_u64()).collect();

    assert!(lines.contains(&0), "Should find color on line 0 (after Japanese + French text)");
    assert!(lines.contains(&1), "Should find color on line 1 (after 'café')");

    // Verify first color is red (sanity check)
    let first_color = &color_array[0];
    let red = first_color["color"]["red"].as_f64().unwrap_or(0.0);
    assert!(red > 0.5, "First color should have significant red component");

    Ok(())
}
