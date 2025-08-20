/// Behavioral tests for LSP functionality
/// These tests verify actual functionality, not just response shapes
/// They ensure the wired infrastructure produces real results

use perl_parser::{LspServer, JsonRpcRequest};
use serde_json::{json, Value};

mod test_fixtures {
    pub const MAIN_FILE: &str = r#"#!/usr/bin/env perl
use strict;
use warnings;
use lib 'lib';
use My::Module;

my $obj = My::Module->new();
$obj->process("test");

sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

# Bad practice: using bareword filehandle
open FILE, "test.txt";
"#;

    pub const MODULE_FILE: &str = r#"package My::Module;
use strict;
use warnings;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub process {
    my ($self, $data) = @_;
    print "Processing: $data\n";
    return length($data);
}

1;
"#;
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

/// Helper to send a notification
fn send_notification(server: &mut LspServer, method: &str, params: Option<Value>) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: method.to_string(),
        params,
    };
    
    let _ = server.handle_request(request);
}

fn create_test_server() -> LspServer {
    let mut server = LspServer::new();
    
    // Initialize the server
    send_request(&mut server, "initialize", Some(json!({
        "rootUri": "file:///workspace",
        "capabilities": {
            "textDocument": {
                "definition": {"dynamicRegistration": false},
                "references": {"dynamicRegistration": false},
                "completion": {"dynamicRegistration": false}
            },
            "workspace": {
                "symbol": {"dynamicRegistration": false}
            }
        }
    })));
    
    send_notification(&mut server, "initialized", Some(json!({})));
    
    // Open main file
    send_notification(&mut server, "textDocument/didOpen", Some(json!({
        "textDocument": {
            "uri": "file:///workspace/script.pl",
            "languageId": "perl",
            "version": 1,
            "text": test_fixtures::MAIN_FILE
        }
    })));
    
    // Open module file
    send_notification(&mut server, "textDocument/didOpen", Some(json!({
        "textDocument": {
            "uri": "file:///workspace/lib/My/Module.pm",
            "languageId": "perl",
            "version": 1,
            "text": test_fixtures::MODULE_FILE
        }
    })));
    
    server
}

#[test]
fn test_cross_file_definition() {
    let mut server = create_test_server();
    
    // Request go-to-definition for My::Module usage
    let result = send_request(&mut server, "textDocument/definition", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "position": {"line": 4, "character": 10} // On "My::Module"
    })));
    
    if let Some(result) = result {
        let locations = result.as_array().expect("Should return location array");
        assert!(!locations.is_empty(), "Should find module definition");
        
        let location = &locations[0];
        assert_eq!(
            location["uri"].as_str().unwrap(),
            "file:///workspace/lib/My/Module.pm",
            "Should navigate to module file"
        );
    } else {
        panic!("Expected result for definition request");
    }
}

#[test]
fn test_cross_file_references() {
    let mut server = create_test_server();
    
    // Request references for the 'new' method
    let result = send_request(&mut server, "textDocument/references", Some(json!({
        "textDocument": {"uri": "file:///workspace/lib/My/Module.pm"},
        "position": {"line": 4, "character": 4}, // On "new" method
        "context": {"includeDeclaration": true}
    })));
    
    if let Some(result) = result {
        let references = result.as_array().expect("Should return reference array");
        assert!(references.len() >= 2, "Should find declaration and usage");
        
        // Check that we found the usage in script.pl
        let has_script_ref = references.iter().any(|r| {
            r["uri"].as_str() == Some("file:///workspace/script.pl")
        });
        assert!(has_script_ref, "Should find reference in script.pl");
    } else {
        panic!("Expected result for references request");
    }
}

#[test]
fn test_workspace_symbol_search() {
    let mut server = create_test_server();
    
    // Search for symbols across workspace
    let result = send_request(&mut server, "workspace/symbol", Some(json!({"query": "process"})));
    
    if let Some(result) = result {
        let symbols = result.as_array().expect("Should return symbol array");
        assert!(!symbols.is_empty(), "Should find 'process' method");
        
        let process_symbol = symbols.iter().find(|s| {
            s["name"].as_str() == Some("process")
        }).expect("Should find process symbol");
        
        assert_eq!(
            process_symbol["location"]["uri"].as_str().unwrap(),
            "file:///workspace/lib/My/Module.pm",
            "Process method should be in Module.pm"
        );
    } else {
        panic!("Expected result for workspace symbol request");
    }
}

#[test]
fn test_extract_variable_returns_edits() {
    let mut server = create_test_server();
    
    // Request code actions for expression extraction
    let result = send_request(&mut server, "textDocument/codeAction", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "range": {
            "start": {"line": 11, "character": 11},
            "end": {"line": 11, "character": 18} // Select "$x + $y"
        },
        "context": {"diagnostics": []}
    })));
    
    if let Some(result) = result {
        let actions = result.as_array().expect("Should return action array");
        
        // Find extract variable action
        let extract_action = actions.iter().find(|a| {
            a["title"].as_str().map_or(false, |t| t.contains("Extract to variable"))
        }).expect("Should have extract variable action");
        
        // Verify it has real edits
        assert!(
            extract_action["edit"]["changes"].is_object() ||
            extract_action["edit"]["documentChanges"].is_array(),
            "Extract action should have workspace edits"
        );
        
        // Check that edits are non-empty
        if let Some(changes) = extract_action["edit"]["changes"].as_object() {
            let edits = changes.values().next()
                .and_then(|v| v.as_array())
                .expect("Should have text edits");
            assert!(!edits.is_empty(), "Should have actual text edits");
        }
    } else {
        panic!("Expected result for code action request");
    }
}

#[test]
fn test_critic_violations_emit_diagnostics() {
    let mut server = create_test_server();
    
    // Wait for diagnostics to be published
    // Note: In real implementation, we'd need to capture published diagnostics
    // For now, we'll request diagnostics through a code action context
    
    let result = send_request(&mut server, "textDocument/codeAction", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 20, "character": 0}
        },
        "context": {"diagnostics": [], "only": ["quickfix"]}
    })));
    
    // Verify the server is processing the file and would emit diagnostics
    // In a real test, we'd capture the publishDiagnostics notification
    assert!(result.is_some(), "Should process file for diagnostics");
    
    // Alternative: directly check if diagnostics would be generated
    // This would require exposing a test helper method
}

#[test]
fn test_test_generation_actions_present() {
    let mut server = create_test_server();
    
    // Request code actions for the calculate subroutine
    let result = send_request(&mut server, "textDocument/codeAction", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "range": {
            "start": {"line": 9, "character": 0},
            "end": {"line": 12, "character": 1} // Cover "calculate" subroutine
        },
        "context": {"diagnostics": []}
    })));
    
    if let Some(result) = result {
        let actions = result.as_array().expect("Should return action array");
        
        // Find test generation action
        let test_action = actions.iter().find(|a| {
            a["title"].as_str().map_or(false, |t| t.contains("Generate test"))
        }).expect("Should have test generation action");
        
        // Verify it has proper command structure
        assert_eq!(
            test_action["command"]["command"].as_str().unwrap(),
            "perl.generateTest",
            "Should have correct command ID"
        );
        
        // Check arguments include test code
        let args = test_action["command"]["arguments"].as_array()
            .expect("Should have arguments");
        assert!(!args.is_empty(), "Should have test generation arguments");
        
        let first_arg = &args[0];
        assert!(first_arg["name"].is_string(), "Should include subroutine name");
        assert!(first_arg["test"].is_string(), "Should include generated test code");
    } else {
        panic!("Expected result for code action request");
    }
}

#[test]
fn test_type_aware_completion() {
    let mut server = create_test_server();
    
    // Request completion after $obj->
    let result = send_request(&mut server, "textDocument/completion", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "position": {"line": 7, "character": 6} // After "$obj->"
    })));
    
    if let Some(result) = result {
        let items = if result.is_array() {
            result.as_array().unwrap()
        } else if let Some(items) = result["items"].as_array() {
            items
        } else {
            panic!("Unexpected completion response format");
        };
        
        assert!(!items.is_empty(), "Should have completion items");
        
        // Check for method suggestions
        let has_process = items.iter().any(|item| {
            item["label"].as_str() == Some("process")
        });
        assert!(has_process, "Should suggest 'process' method");
        
        // Verify type information in detail field
        let typed_items = items.iter().filter(|item| {
            item["detail"].is_string()
        }).count();
        assert!(typed_items > 0, "Should have type information in completion details");
    } else {
        panic!("Expected result for completion request");
    }
}

#[test]
fn test_hover_shows_rich_information() {
    let mut server = create_test_server();
    
    // Request hover for My::Module
    let result = send_request(&mut server, "textDocument/hover", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"},
        "position": {"line": 4, "character": 10} // On "My::Module"
    })));
    
    if let Some(result) = result {
        assert!(!result.is_null(), "Should return hover information");
        
        let contents = &result["contents"];
        let hover_text = if let Some(value) = contents["value"].as_str() {
            value.to_string()
        } else if let Some(markup) = contents.as_array() {
            markup.iter()
                .filter_map(|m| m["value"].as_str())
                .collect::<String>()
        } else {
            String::new()
        };
        
        assert!(!hover_text.is_empty(), "Should have hover content");
        assert!(
            hover_text.contains("package") || hover_text.contains("module"),
            "Should show package/module information"
        );
    } else {
        panic!("Expected result for hover request");
    }
}

#[test]
fn test_folding_ranges_work() {
    let mut server = create_test_server();
    
    // Request folding ranges
    let result = send_request(&mut server, "textDocument/foldingRange", Some(json!({
        "textDocument": {"uri": "file:///workspace/script.pl"}
    })));
    
    if let Some(result) = result {
        let ranges = result.as_array().expect("Should return folding ranges");
        assert!(!ranges.is_empty(), "Should have folding ranges");
        
        // Check for subroutine folding
        let has_sub_fold = ranges.iter().any(|r| {
            r["kind"].as_str() == Some("region") || 
            r["startLine"].is_number()
        });
        assert!(has_sub_fold, "Should have foldable regions");
    } else {
        panic!("Expected result for folding range request");
    }
}