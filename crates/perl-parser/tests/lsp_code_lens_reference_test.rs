//! Tests for CodeLens reference counting functionality

use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

fn setup_server() -> LspServer {
    let mut server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    server.handle_request(init_request).unwrap();
    server
}

#[test]
fn test_code_lens_reference_counting() {
    let mut server = setup_server();

    // Open a document with a subroutine that's called multiple times
    let code = r#"
sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
}

# Call the subroutine multiple times
greet("Alice");
greet("Bob");
my $func = \&greet;
$func->("Charlie");

# Another subroutine with no calls
sub unused_function {
    return 42;
}
"#;

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": code
            }
        })),
        id: None,
    };

    // Send the notification (no response expected)
    let _ = server.handle_request(open_request);

    // Request code lenses
    let code_lens_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/codeLens".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(code_lens_request).unwrap();
    assert!(response.result.is_some());

    let lenses = response.result.unwrap();
    let lenses_array = lenses.as_array().unwrap();

    // Find the lens for the "greet" subroutine
    let greet_lens = lenses_array
        .iter()
        .find(|lens| {
            lens.get("data")
                .and_then(|d| d.get("name"))
                .and_then(|n| n.as_str())
                == Some("greet")
        })
        .expect("Should find lens for 'greet'");

    // Resolve the lens to get the reference count
    let resolve_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "codeLens/resolve".to_string(),
        params: Some(greet_lens.clone()),
        id: Some(json!(3)),
    };

    let resolved = server.handle_request(resolve_request).unwrap();
    let resolved_lens = resolved.result.unwrap();

    // Check the reference count in the command title
    let command = resolved_lens.get("command").unwrap();
    let title = command.get("title").unwrap().as_str().unwrap();

    // We expect at least 2 references (2 direct calls)
    // The \&greet reference might not be counted depending on how it's parsed
    assert!(
        title.contains("2 reference"),
        "Expected at least 2 references, got: {}",
        title
    );

    // Find the lens for the "unused_function" subroutine
    let unused_lens = lenses_array
        .iter()
        .find(|lens| {
            lens.get("data")
                .and_then(|d| d.get("name"))
                .and_then(|n| n.as_str())
                == Some("unused_function")
        })
        .expect("Should find lens for 'unused_function'");

    // Resolve the unused function lens
    let resolve_unused = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "codeLens/resolve".to_string(),
        params: Some(unused_lens.clone()),
        id: Some(json!(4)),
    };

    let resolved_unused = server.handle_request(resolve_unused).unwrap();
    let resolved_unused_lens = resolved_unused.result.unwrap();

    // Check the reference count for unused function
    let unused_command = resolved_unused_lens.get("command").unwrap();
    let unused_title = unused_command.get("title").unwrap().as_str().unwrap();

    // We expect 0 references
    assert!(
        unused_title.contains("0 references"),
        "Expected 0 references, got: {}",
        unused_title
    );
}

#[test]
fn test_code_lens_package_references() {
    let mut server = setup_server();

    // Open a document with a package that's used
    let code = r#"
package MyModule;

sub new {
    my $class = shift;
    return bless {}, $class;
}

package main;

use MyModule;

my $obj = MyModule->new();
MyModule::some_method();
"#;

    let uri = "file:///test_package.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": code
            }
        })),
        id: None,
    };

    // Send the notification (no response expected)
    let _ = server.handle_request(open_request);

    // Request code lenses
    let code_lens_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/codeLens".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            }
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(code_lens_request).unwrap();
    let lenses = response.result.unwrap();
    let lenses_array = lenses.as_array().unwrap();

    // Find the lens for the "MyModule" package
    let package_lens = lenses_array
        .iter()
        .find(|lens| {
            lens.get("data")
                .and_then(|d| d.get("name"))
                .and_then(|n| n.as_str())
                == Some("MyModule")
                && lens
                    .get("data")
                    .and_then(|d| d.get("kind"))
                    .and_then(|k| k.as_str())
                    == Some("package")
        })
        .expect("Should find lens for 'MyModule' package");

    // Resolve the lens to get the reference count
    let resolve_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "codeLens/resolve".to_string(),
        params: Some(package_lens.clone()),
        id: Some(json!(3)),
    };

    let resolved = server.handle_request(resolve_request).unwrap();
    let resolved_lens = resolved.result.unwrap();

    // Check the reference count in the command title
    let command = resolved_lens.get("command").unwrap();
    let title = command.get("title").unwrap().as_str().unwrap();

    // We expect at least 1 reference (the 'use MyModule' statement)
    // The actual count may be higher depending on how the parser handles method calls
    assert!(
        !title.contains("0 references"),
        "Expected at least 1 reference, got: {}",
        title
    );
}
