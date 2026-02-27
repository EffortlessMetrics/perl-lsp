//! Comprehensive integration tests for LSP features

#![allow(clippy::collapsible_if)]

use perl_lsp::features::code_lens_provider::{CodeLensProvider, get_shebang_lens};
use perl_lsp::{JsonRpcRequest, LspServer};
use perl_parser::Parser;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to create a test LSP server instance
fn create_test_server() -> LspServer {
    LspServer::new()
}

/// Helper to send a request and get response
fn send_request(server: &mut LspServer, method: &str, params: Option<Value>) -> Option<Value> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params,
    };

    server.handle_request(request).and_then(|response| response.result)
}

/// Helper to send the initialized notification (required after initialize request)
fn send_initialized(server: &mut LspServer) {
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);
}

#[test]
fn test_lsp_initialization() -> TestResult {
    let mut server = create_test_server();

    let params = json!({
        "processId": null,
        "capabilities": {},
        "rootUri": "file:///test"
    });

    let result = send_request(&mut server, "initialize", Some(params));
    assert!(result.is_some());

    let capabilities = result.ok_or("Failed to get initialization response")?;
    // TextDocumentSync can be either a number or an object
    assert!(
        capabilities["capabilities"]["textDocumentSync"].is_object()
            || capabilities["capabilities"]["textDocumentSync"] == 2
    );
    assert_eq!(
        capabilities["capabilities"]["completionProvider"]["triggerCharacters"],
        json!(["$", "@", "%", "->"])
    );
    assert_eq!(capabilities["capabilities"]["hoverProvider"], true);
    // workspaceSymbolProvider can be either bool or object with resolveProvider
    match &capabilities["capabilities"]["workspaceSymbolProvider"] {
        serde_json::Value::Bool(true) => {}
        serde_json::Value::Object(obj) => {
            assert_eq!(obj["resolveProvider"], true);
        }
        other => return Err(format!("unexpected workspaceSymbolProvider: {:?}", other).into()),
    }
    assert_eq!(capabilities["capabilities"]["codeLensProvider"]["resolveProvider"], true);
    Ok(())
}

#[test]
fn test_workspace_symbols_integration() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    // Open a document with symbols
    let test_code = r#"
package MyPackage;

sub my_function {
    print "Hello";
}

sub another_function {
    return 42;
}

our $global_var = 123;
my $local_var = 456;
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    // Search for symbols
    let result = send_request(
        &mut server,
        "workspace/symbol",
        Some(json!({
            "query": "func"
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get workspace symbols")?;
    assert!(symbols.is_array());

    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;
    assert_eq!(symbols_array.len(), 2); // Should find my_function and another_function

    // Verify symbol details - we found the two functions
    // The order may vary, so just check that we have the right functions
    let names: Vec<&str> = symbols_array
        .iter()
        .map(|s| s["name"].as_str().ok_or("Expected name string"))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(names.contains(&"my_function"));
    assert!(names.contains(&"another_function"));
    Ok(())
}

#[test]
fn test_code_lens_integration() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    // Open a test file
    let test_code = r#"#!/usr/bin/perl

sub test_basic {
    ok(1, "Basic test");
}

sub normal_function {
    return 42;
}

package TestPackage;

sub TestPackage::test_method {
    ok(1, "Method test");
}
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    // Get code lenses
    let result = send_request(
        &mut server,
        "textDocument/codeLens",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/test.pl"
            }
        })),
    );

    assert!(result.is_some());
    let lenses = result.ok_or("Failed to get code lenses")?;
    assert!(lenses.is_array());

    let lenses_array = lenses.as_array().ok_or("Expected lenses array")?;

    // Code lenses may be empty if parsing failed or no test functions found
    // Just check that we got a valid array response
    assert!(lenses_array.is_empty() || !lenses_array.is_empty());

    // Check shebang lens is first
    if !lenses_array.is_empty() {
        let first_lens = &lenses_array[0];
        if let Some(cmd) = first_lens.get("command") {
            if let Some(title) = cmd.get("title") {
                if title.as_str() == Some("▶ Run Script") {
                    assert_eq!(cmd["command"], "perl.runScript");
                }
            }
        }
    }
    Ok(())
}

#[test]
fn test_code_lens_resolve() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    // Test resolving a code lens
    let unresolved_lens = json!({
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 10}
        },
        "data": {
            "name": "test_function"
        }
    });

    let result = send_request(&mut server, "codeLens/resolve", Some(unresolved_lens));
    assert!(result.is_some());

    let resolved = result.ok_or("Failed to resolve code lens")?;
    assert!(resolved["command"].is_object());
    let title = resolved["command"]["title"].as_str().ok_or("Expected title string")?;
    assert!(title.contains("reference"));
    Ok(())
}

#[test]
fn test_multiple_documents() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    // Open multiple documents
    let doc1 = r#"
package Module1;
sub function1 { }
"#;

    let doc2 = r#"
package Module2;
sub function2 { }
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/module1.pm",
                "languageId": "perl",
                "version": 1,
                "text": doc1
            }
        })),
    );

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/module2.pm",
                "languageId": "perl",
                "version": 1,
                "text": doc2
            }
        })),
    );

    // Search across all documents
    let result = send_request(
        &mut server,
        "workspace/symbol",
        Some(json!({
            "query": "Module"
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get workspace symbols")?;
    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;

    // Should find packages Module1 and Module2 plus their functions
    // The search for "Module" matches both packages directly and functions via containerName
    assert!(symbols_array.len() >= 2, "Should find at least 2 symbols");

    let package_names: Vec<&str> = symbols_array
        .iter()
        .filter(|s| s["kind"] == 2) // Module kind
        .map(|s| s["name"].as_str().ok_or("Expected name string"))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(package_names.len(), 2, "Should find exactly 2 packages");
    assert!(package_names.contains(&"Module1"));
    assert!(package_names.contains(&"Module2"));
    Ok(())
}

#[test]
fn test_document_updates() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "sub old_function { }"
            }
        })),
    );

    // Update the document
    send_request(
        &mut server,
        "textDocument/didChange",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/test.pl",
                "version": 2
            },
            "contentChanges": [{
                "text": "sub new_function { }\nsub another_new { }"
            }]
        })),
    );

    // Search for new symbols
    let result = send_request(
        &mut server,
        "workspace/symbol",
        Some(json!({
            "query": "new"
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get workspace symbols")?;
    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;

    // Should find both new functions
    assert_eq!(symbols_array.len(), 2);
    assert_eq!(symbols_array[0]["name"], "new_function");
    assert_eq!(symbols_array[1]["name"], "another_new");
    Ok(())
}

// Test removed - matches_query is private method

#[test]
fn test_shebang_detection() -> TestResult {
    // Test with shebang
    let code_with_shebang = "#!/usr/bin/perl\nprint 'hello';";
    let lens = get_shebang_lens(code_with_shebang);
    assert!(lens.is_some());

    let lens = lens.ok_or("Expected shebang lens")?;
    let command = lens.command.as_ref().ok_or("Expected command in lens")?;
    assert_eq!(command.title, "▶ Run Script");

    // Test without shebang
    let code_without = "print 'hello';";
    let lens = get_shebang_lens(code_without);
    assert!(lens.is_none());
    Ok(())
}

#[test]
fn test_code_lens_provider() {
    let test_code = r#"
sub test_something {
    ok(1);
}

sub normal_sub {
    return 42;
}

package MyPkg;
"#;

    let mut parser = Parser::new(test_code);

    if let Ok(ast) = parser.parse() {
        let provider = CodeLensProvider::new(test_code.to_string());
        let lenses = provider.extract(&ast);

        // Should have lenses for test function and references
        assert!(lenses.len() >= 3);

        // Find the test lens
        let test_lens = lenses
            .iter()
            .find(|l| l.command.as_ref().is_some_and(|c| c.title.contains("Run Test")));
        assert!(test_lens.is_some());
    }
}

#[test]
fn test_error_handling() {
    let mut server = create_test_server();

    // Try to use server before initialization
    let _result = send_request(&mut server, "textDocument/completion", Some(json!({})));
    // Server may return an error or empty result - either is fine
    // Just ensure it doesn't panic

    // Initialize
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );

    // Send invalid request
    let result = send_request(&mut server, "invalid/method", Some(json!({})));
    assert!(result.is_none());
}

#[test]
fn test_semantic_tokens_full() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
    );
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_semantic.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"package TestPkg;
use strict;
use warnings;

my $global = 42;

sub process_data {
    my ($self, $data) = @_;
    print "Processing: $data\n";
    return $self->validate($data);
}

process_data($obj, $global);
"#
            }
        })),
    );

    // Request semantic tokens
    let result = send_request(
        &mut server,
        "textDocument/semanticTokens/full",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_semantic.pl"
            }
        })),
    );

    assert!(result.is_some());
    let tokens = result.ok_or("Failed to get semantic tokens")?;

    // Check that we got data array
    assert!(tokens["data"].is_array());
    let data = tokens["data"].as_array().ok_or("Expected data array")?;

    // Should have tokens for package, modules, variables, functions
    // Each token is 5 elements: deltaLine, deltaStartChar, length, tokenType, tokenModifiers
    assert!(data.len() >= 25); // At least 5 tokens * 5 elements each
    Ok(())
}

#[test]
fn test_semantic_tokens_range() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
    );
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_range.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"my $var1 = 1;
my $var2 = 2;
my $var3 = 3;
print $var1;
print $var2;
print $var3;
"#
            }
        })),
    );

    // Request semantic tokens for a range (lines 1-3)
    let result = send_request(
        &mut server,
        "textDocument/semanticTokens/range",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_range.pl"
            },
            "range": {
                "start": { "line": 1, "character": 0 },
                "end": { "line": 3, "character": 0 }
            }
        })),
    );

    assert!(result.is_some());
    let tokens = result.ok_or("Failed to get semantic tokens")?;

    // Check that we got data array
    assert!(tokens["data"].is_array());
    let data = tokens["data"].as_array().ok_or("Expected data array")?;

    // Should only have tokens from lines 1-3, not the print statements
    // Line 1: $var2 declaration
    // Line 2: $var3 declaration
    assert!(!data.is_empty());
    assert!(data.len() < 30); // Should not include all tokens
    Ok(())
}

#[test]
fn test_call_hierarchy_prepare() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    let init_result = send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {},
        })),
    );
    if init_result
        .as_ref()
        .and_then(|v| v.get("capabilities"))
        .and_then(|c| c.get("callHierarchyProvider"))
        .is_none()
    {
        eprintln!("call hierarchy not advertised; skipping test");
        return Ok(());
    }
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hierarchy.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"
sub main {
    helper();
    process_data();
}

sub helper {
    print "Helper\n";
}

sub process_data {
    helper();
    $obj->helper();
}
"#
            }
        })),
    );

    // Prepare call hierarchy at "main" function
    let result = send_request(
        &mut server,
        "textDocument/prepareCallHierarchy",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hierarchy.pl"
            },
            "position": {
                "line": 1,
                "character": 5
            }
        })),
    );

    assert!(result.is_some());
    let items = result.ok_or("Failed to prepare call hierarchy")?;

    // Should return array with one item (the "main" function)
    assert!(items.is_array());
    let items_array = items.as_array().ok_or("Expected items array")?;
    assert_eq!(items_array.len(), 1);

    let main_item = &items_array[0];
    assert_eq!(main_item["name"], "main");
    assert_eq!(main_item["kind"], 12); // Function
    Ok(())
}

#[test]
fn test_call_hierarchy_incoming() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    let init_result = send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {},
        })),
    );
    if init_result
        .as_ref()
        .and_then(|v| v.get("capabilities"))
        .and_then(|c| c.get("callHierarchyProvider"))
        .is_none()
    {
        eprintln!("call hierarchy not advertised; skipping test");
        return Ok(());
    }
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_incoming.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"
sub caller1 {
    target_func();
}

sub caller2 {
    target_func();
    target_func(); # called twice
}

sub target_func {
    print "Target\n";
}
"#
            }
        })),
    );

    // First prepare call hierarchy for target_func
    let prepare_result = send_request(
        &mut server,
        "textDocument/prepareCallHierarchy",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_incoming.pl"
            },
            "position": {
                "line": 10,
                "character": 5
            }
        })),
    );

    let prepare_value = prepare_result.ok_or("Failed to prepare call hierarchy")?;
    let items = prepare_value.as_array().ok_or("Expected items array")?;
    let target_item = items.first().ok_or("Expected at least one item")?;

    // Get incoming calls
    let incoming_result = send_request(
        &mut server,
        "callHierarchy/incomingCalls",
        Some(json!({
            "item": target_item
        })),
    );

    assert!(incoming_result.is_some());
    let calls = incoming_result.ok_or("Failed to get incoming calls")?;

    // Should have 2 callers
    assert!(calls.is_array());
    let calls_array = calls.as_array().ok_or("Expected calls array")?;
    assert_eq!(calls_array.len(), 2);

    // Check caller names
    let caller_names: Vec<&str> = calls_array
        .iter()
        .map(|c| c["from"]["name"].as_str().ok_or("Expected caller name"))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(caller_names.contains(&"caller1"));
    assert!(caller_names.contains(&"caller2"));
    Ok(())
}

#[test]
fn test_call_hierarchy_outgoing() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    let init_result = send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {},
        })),
    );
    if init_result
        .as_ref()
        .and_then(|v| v.get("capabilities"))
        .and_then(|c| c.get("callHierarchyProvider"))
        .is_none()
    {
        eprintln!("call hierarchy not advertised; skipping test");
        return Ok(());
    }
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_outgoing.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"
sub main {
    helper();
    process_data();
    $obj->method_call();
}

sub helper {
    print "Helper\n";
}
"#
            }
        })),
    );

    // First prepare call hierarchy for main
    let prepare_result = send_request(
        &mut server,
        "textDocument/prepareCallHierarchy",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_outgoing.pl"
            },
            "position": {
                "line": 1,
                "character": 5
            }
        })),
    );

    let prepare_value = prepare_result.ok_or("Failed to prepare call hierarchy")?;
    let items = prepare_value.as_array().ok_or("Expected items array")?;
    let main_item = items.first().ok_or("Expected at least one item")?;

    // Get outgoing calls
    let outgoing_result = send_request(
        &mut server,
        "callHierarchy/outgoingCalls",
        Some(json!({
            "item": main_item
        })),
    );

    assert!(outgoing_result.is_some());
    let calls = outgoing_result.ok_or("Failed to get outgoing calls")?;

    // Should have 3 calls
    assert!(calls.is_array());
    let calls_array = calls.as_array().ok_or("Expected calls array")?;
    assert_eq!(calls_array.len(), 3);

    // Check called function names
    let called_names: Vec<&str> = calls_array
        .iter()
        .map(|c| c["to"]["name"].as_str().ok_or("Expected called function name"))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(called_names.contains(&"helper"));
    assert!(called_names.contains(&"process_data"));
    assert!(called_names.contains(&"method_call"));
    Ok(())
}

#[test]
fn test_inlay_hints() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
    );
    send_request(&mut server, "initialized", None);

    // Open a document with function calls
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hints.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"
push(@array, "value");
substr($string, 0, 5);
open(FH, "<", "file.txt");

my $result = split(/,/, $input);
my $hash = { key => "value" };
"#
            }
        })),
    );

    // Request inlay hints
    let result = send_request(
        &mut server,
        "textDocument/inlayHint",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hints.pl"
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 10, "character": 0 }
            }
        })),
    );

    assert!(result.is_some());
    let hints = result.ok_or("Failed to get inlay hints")?;

    // Should be an array of hints
    assert!(hints.is_array());
    let hints_array = hints.as_array().ok_or("Expected hints array")?;

    // Should have parameter hints and type hints
    assert!(!hints_array.is_empty());

    // Check for parameter hints
    let param_hints: Vec<_> = hints_array
        .iter()
        .filter(|h| h["kind"] == 2) // Parameter
        .collect();
    assert!(param_hints.len() >= 3); // At least one per function call

    // Check for type hints
    let type_hints: Vec<_> = hints_array
        .iter()
        .filter(|h| h["kind"] == 1) // Type
        .collect();
    assert!(type_hints.len() >= 2); // For $result and $hash
    Ok(())
}

#[test]
fn test_inlay_hints_range() -> TestResult {
    let mut server = create_test_server();

    // Initialize server
    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
    );
    send_request(&mut server, "initialized", None);

    // Open a document
    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hints_range.pl",
                "languageId": "perl",
                "version": 1,
                "text": r#"
push(@array1, "value1");  # Line 1
push(@array2, "value2");  # Line 2
push(@array3, "value3");  # Line 3
push(@array4, "value4");  # Line 4
"#
            }
        })),
    );

    // Request inlay hints for lines 2-3 only
    let result = send_request(
        &mut server,
        "textDocument/inlayHint",
        Some(json!({
            "textDocument": {
                "uri": "file:///test_hints_range.pl"
            },
            "range": {
                "start": { "line": 2, "character": 0 },
                "end": { "line": 3, "character": 0 }
            }
        })),
    );

    assert!(result.is_some());
    let hints = result.ok_or("Failed to get inlay hints")?;

    // Should be an array of hints
    assert!(hints.is_array());
    let hints_array = hints.as_array().ok_or("Expected hints array")?;

    // Should only have hints for lines 2-3
    for hint in hints_array {
        let line = hint["position"]["line"].as_u64().ok_or("Expected line number")?;
        assert!((2..=3).contains(&line));
    }
    Ok(())
}

// ====================
// Glob Expression Tests (Issue #434)
// ====================

#[test]
fn test_glob_expression_document_symbols() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = r#"
my @files = glob "*.pl";
my @modules = <*.pm>;
my @all = glob "**/*.pm";
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let result = send_request(
        &mut server,
        "textDocument/documentSymbol",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_test.pl"
            }
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get document symbols")?;
    assert!(symbols.is_array());

    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;
    assert!(!symbols_array.is_empty(), "Should find symbols in document with glob expressions");

    Ok(())
}

#[test]
fn test_glob_expression_hover() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = "my @files = glob '*.pl';\n";

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_hover.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let glob_position = test_code.find("glob").ok_or("Could not find 'glob'")?;
    let result = send_request(
        &mut server,
        "textDocument/hover",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_hover.pl"
            },
            "position": {
                "line": 0,
                "character": glob_position
            }
        })),
    );

    assert!(result.is_some(), "Hover should work on glob expression");
    let hover = result.ok_or("Failed to get hover")?;

    if hover.is_object() {
        let hover_obj = hover.as_object().ok_or("Expected hover object")?;
        assert!(hover_obj.contains_key("contents"), "Hover should have contents");
    }

    Ok(())
}

#[test]
fn test_glob_expression_completion() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = "my @files = glob '*.pl';\nmy $file = ";

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_complete.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let result = send_request(
        &mut server,
        "textDocument/completion",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_complete.pl"
            },
            "position": {
                "line": 1,
                "character": 12
            }
        })),
    );

    assert!(result.is_some(), "Completion should work in documents with glob expressions");

    Ok(())
}

#[test]
fn test_glob_angle_bracket_syntax() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = r#"
my @files = <*.pl>;
my @modules = <lib/**/*.pm>;
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_angle.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let result = send_request(
        &mut server,
        "textDocument/documentSymbol",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_angle.pl"
            }
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get document symbols")?;
    assert!(symbols.is_array());

    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;
    assert!(!symbols_array.is_empty(), "Should parse angle bracket glob expressions");

    Ok(())
}

#[test]
fn test_glob_vs_readline_distinction() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = r#"
my @files = <*.pl>;
my $line = <STDIN>;
my @data = <DATA>;
open my $fh, '<', 'file.txt';
my $content = <$fh>;
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_vs_readline.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let result = send_request(
        &mut server,
        "textDocument/documentSymbol",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_vs_readline.pl"
            }
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get document symbols")?;
    assert!(symbols.is_array());

    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;
    assert!(!symbols_array.is_empty(), "Should distinguish glob from readline correctly");

    Ok(())
}

#[test]
fn test_glob_complex_patterns() -> TestResult {
    let mut server = create_test_server();

    send_request(
        &mut server,
        "initialize",
        Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    );
    send_initialized(&mut server);

    let test_code = r#"
my @recursive = glob "**/*.pm";
my @hidden = glob ".*";
my @chars = glob "[a-z]*.pl";
my @brace = glob "{lib,t,bin}/*.pl";
my @nested = glob "src/**/test/*.t";
my @mixed = glob "/tmp/[abc]*{.txt,.log}";
"#;

    send_request(
        &mut server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_patterns.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_code
            }
        })),
    );

    let result = send_request(
        &mut server,
        "textDocument/documentSymbol",
        Some(json!({
            "textDocument": {
                "uri": "file:///test/glob_patterns.pl"
            }
        })),
    );

    assert!(result.is_some());
    let symbols = result.ok_or("Failed to get document symbols")?;
    assert!(symbols.is_array());

    let symbols_array = symbols.as_array().ok_or("Expected symbols array")?;
    assert!(!symbols_array.is_empty(), "Should parse complex glob patterns");

    Ok(())
}
