// Document symbols test — println! used for diagnostic output.
#![allow(clippy::print_stdout)]
//! Tests for textDocument/documentSymbol LSP feature
//!
//! These tests validate the document symbol provider functionality including:
//! - Basic symbol extraction (packages, subroutines, variables)
//! - Nested symbol structures (closures, multiple packages)
//! - Empty document handling
//! - Constants and labels
//! - All variable types (scalar, array, hash, our, local, state)
//! - Hierarchical symbol structures

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn setup_server() -> LspServer {
    setup_server_with_capabilities(json!({}))
}

fn setup_server_with_capabilities(capabilities: Value) -> LspServer {
    let server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": 1,
            "capabilities": capabilities
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
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

fn open_document(server: &LspServer, uri: &str, content: &str) {
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

fn flatten_symbols(symbols: &[Value]) -> Vec<&Value> {
    let mut flattened = Vec::new();
    collect_symbols(symbols, &mut flattened);
    flattened
}

fn collect_symbols<'a>(symbols: &'a [Value], flattened: &mut Vec<&'a Value>) {
    for symbol in symbols {
        flattened.push(symbol);
        if let Some(children) = symbol.get("children").and_then(Value::as_array) {
            collect_symbols(children, flattened);
        }
    }
}

fn symbol_names(symbols: &[Value]) -> Vec<&str> {
    flatten_symbols(symbols).into_iter().filter_map(|symbol| symbol["name"].as_str()).collect()
}

#[test]
fn test_document_symbols_basic() -> TestResult {
    let server = setup_server();

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

    open_document(&server, "file:///test.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    // Check that we have symbols
    assert!(result.is_array());
    let symbols = result.as_array().ok_or("Result is not an array")?;
    assert!(!symbols.is_empty());

    // Check for package symbol
    let package_symbol = symbols.iter().find(|s| s["name"].as_str() == Some("MyModule"));
    assert!(package_symbol.is_some());
    let package_symbol = package_symbol.ok_or("Package symbol not found")?;
    // Kind can be 4 (Package) or 2 (Module) depending on client cap/server version
    let kind = package_symbol["kind"].as_i64().unwrap_or(0);
    assert!(kind == 4 || kind == 2, "Expected Package(4) or Module(2), got {}", kind);

    // Check for subroutine symbols
    let hello_sub = symbols.iter().find(|s| s["name"].as_str() == Some("hello"));
    assert!(hello_sub.is_some());
    let hello_sub = hello_sub.ok_or("hello sub not found")?;
    assert_eq!(hello_sub["kind"], 12); // Function

    let calc_sub = symbols.iter().find(|s| s["name"].as_str() == Some("calculate"));
    assert!(calc_sub.is_some());
    let calc_sub = calc_sub.ok_or("calculate sub not found")?;
    assert_eq!(calc_sub["kind"], 12); // Function

    // Check for variable symbols
    let global_var = symbols.iter().find(|s| s["name"].as_str() == Some("$global_var"));
    assert!(global_var.is_some());
    let global_var = global_var.ok_or("global_var not found")?;
    assert_eq!(global_var["kind"], 13); // Variable

    let shared_array = symbols.iter().find(|s| s["name"].as_str() == Some("@shared_array"));
    assert!(shared_array.is_some());
    let shared_array = shared_array.ok_or("shared_array not found")?;
    assert_eq!(shared_array["kind"], 18); // Array

    Ok(())
}

#[test]
fn test_document_symbols_include_test2_subtests() -> TestResult {
    let server = setup_server();

    let content = "use Test2::V0;\n\
        subtest 'user lookup' => sub {\n\
            ok(1, 'found');\n\
            subtest 'email matches' => sub {\n\
                is(1, 1, 'ok');\n\
            };\n\
        };\n\
        done_testing;\n";

    open_document(&server, "file:///t/user.t", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///t/user.t" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Result is not an array")?;

    // Outer subtest appears as a top-level symbol...
    let outer = symbols
        .iter()
        .find(|s| s["name"].as_str() == Some("user lookup"))
        .ok_or("outer subtest symbol not found")?;
    assert_eq!(outer["detail"].as_str(), Some("subtest"));

    // ...with the nested subtest as a child (tree structure preserved).
    let children = outer["children"].as_array().ok_or("outer has no children array")?;
    let inner = children
        .iter()
        .find(|s| s["name"].as_str() == Some("email matches"))
        .ok_or("nested subtest symbol not found")?;
    assert_eq!(inner["detail"].as_str(), Some("subtest"));

    Ok(())
}

#[test]
fn test_document_symbols_plack_builder_chain() -> TestResult {
    let server = setup_server();

    let content = r#"
use Plack::Builder;

builder {
    enable 'Static';
    mount '/api' => $api_app;
};
"#;

    open_document(&server, "file:///plack.psgi", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///plack.psgi"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Result is not an array")?;

    assert!(
        symbols.iter().any(|s| s["name"].as_str() == Some("Plack::Middleware::Static")),
        "document symbols should expose the synthesized Plack middleware entry: {symbols:?}"
    );
    assert!(
        symbols.iter().any(|s| s["name"].as_str() == Some("/api")),
        "document symbols should expose the synthesized mount entry: {symbols:?}"
    );

    Ok(())
}

#[test]
fn test_document_symbols_nested() -> TestResult {
    let server = setup_server();

    let content = r#"
package Outer;

sub parent_sub {
    my $parent_var = 10;
    return $parent_var;
}

package Inner;

sub another_sub {
    my %hash = (key => 'value');
    return \%hash;
}

1;
"#;

    open_document(&server, "file:///nested.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///nested.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    let symbols = result.as_array().ok_or("Result is not an array")?;

    // Check for both packages
    let outer_package = symbols.iter().find(|s| s["name"].as_str() == Some("Outer"));
    assert!(outer_package.is_some());

    let inner_package = symbols.iter().find(|s| s["name"].as_str() == Some("Inner"));
    assert!(inner_package.is_some());

    // Check for subroutines
    let parent_sub = symbols.iter().find(|s| s["name"].as_str() == Some("parent_sub"));
    if parent_sub.is_none() {
        println!("Symbols found: {:?}", symbols);
    }
    assert!(parent_sub.is_some(), "parent_sub not found");

    let another_sub = symbols.iter().find(|s| s["name"].as_str() == Some("another_sub"));
    assert!(another_sub.is_some());

    Ok(())
}

#[test]
fn test_document_symbols_empty_document() -> TestResult {
    let server = setup_server();

    open_document(&server, "file:///empty.pl", "");

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///empty.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    // Should return empty array for empty document
    assert!(result.is_array());
    let symbols = result.as_array().ok_or("Result is not an array")?;
    assert!(symbols.is_empty());

    Ok(())
}

#[test]
fn test_document_symbols_with_constants() -> TestResult {
    let server = setup_server();

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

    open_document(&server, "file:///constants.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///constants.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    let symbols = result.as_array().ok_or("Result is not an array")?;

    // Check for function
    let area_sub = symbols.iter().find(|s| s["name"].as_str() == Some("area"));
    assert!(area_sub.is_some());
    let area_sub = area_sub.ok_or("area sub not found")?;
    assert_eq!(area_sub["kind"], 12); // Function

    Ok(())
}

#[test]
fn test_document_symbols_with_labels() -> TestResult {
    let server = setup_server();

    let content = r#"
for my $i (1..10) {
    for my $j (1..10) {
        next if $i + $j > 15;
        last if $j > 5;
    }
}

sub process {
    return if !@_;
    # process...
    return;
}
"#;

    open_document(&server, "file:///labels.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///labels.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    let symbols = result.as_array().ok_or("Result is not an array")?;

    // Check for subroutine
    let process_sub = symbols.iter().find(|s| s["name"].as_str() == Some("process"));
    assert!(process_sub.is_some());

    Ok(())
}

#[test]
fn test_document_symbols_all_variable_types() -> TestResult {
    let server = setup_server();

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

    open_document(&server, "file:///variables.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///variables.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    let symbols = result.as_array().ok_or("Result is not an array")?;

    // Check for scalar variables
    let scalar = symbols.iter().find(|s| s["name"].as_str() == Some("$scalar"));
    assert!(scalar.is_some());
    let scalar = scalar.ok_or("$scalar not found")?;
    assert_eq!(scalar["kind"], 13); // Variable

    // Check for array variables
    let array = symbols.iter().find(|s| s["name"].as_str() == Some("@array"));
    assert!(array.is_some());
    let array = array.ok_or("@array not found")?;
    assert_eq!(array["kind"], 18); // Array

    // Check for hash variables
    let hash = symbols.iter().find(|s| s["name"].as_str() == Some("%hash"));
    assert!(hash.is_some());
    let hash = hash.ok_or("%hash not found")?;
    assert_eq!(hash["kind"], 19); // Object (closest to hash)

    // Check for shared variables
    let shared_scalar = symbols.iter().find(|s| s["name"].as_str() == Some("$shared_scalar"));
    assert!(shared_scalar.is_some());

    let shared_array = symbols.iter().find(|s| s["name"].as_str() == Some("@shared_array"));
    assert!(shared_array.is_some());

    let shared_hash = symbols.iter().find(|s| s["name"].as_str() == Some("%shared_hash"));
    assert!(shared_hash.is_some());

    Ok(())
}

#[test]
fn test_document_symbols_hierarchical_structure() -> TestResult {
    let server = setup_server_with_capabilities(json!({
        "textDocument": {
            "documentSymbol": {
                "hierarchicalDocumentSymbolSupport": true
            }
        }
    }));

    let content = r#"package Parent;

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

    open_document(&server, "file:///hierarchy.pl", content);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///hierarchy.pl"
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((2) as i64)),
    };

    let response = server.handle_request(request).ok_or("No response from server")?;
    let result = response.result.ok_or("Missing result")?;

    let symbols = result.as_array().ok_or("Result is not an array")?;
    let all_symbols = flatten_symbols(symbols);

    // Check that we have the expected symbols, whether returned flat or nested.
    assert!(all_symbols.iter().any(|s| s["name"].as_str() == Some("Parent")));
    assert!(all_symbols.iter().any(|s| s["name"].as_str() == Some("Child")));
    assert!(all_symbols.iter().any(|s| s["name"].as_str() == Some("parent_method")));
    assert!(all_symbols.iter().any(|s| s["name"].as_str() == Some("child_method")));
    assert!(all_symbols.iter().any(|s| s["name"].as_str() == Some("$package_var")));

    let parent_sym = symbols
        .iter()
        .find(|symbol| symbol["name"].as_str() == Some("Parent"))
        .ok_or("Parent symbol not found")?;
    let parent_children =
        parent_sym["children"].as_array().ok_or("Parent symbol has no children")?;
    assert!(
        parent_children.iter().any(|child| child["name"].as_str() == Some("parent_method")),
        "parent_method should be nested under Parent"
    );
    assert!(
        parent_children.iter().any(|child| child["name"].as_str() == Some("Child")),
        "Child should be nested under Parent for this source-backed hierarchy fixture"
    );

    Ok(())
}

// ---- POD section tests (issue #2341) ----

#[test]
fn test_pod_sections_as_document_symbols() -> TestResult {
    let server = setup_server();
    let content = "package MyLib;\n\n=head1 NAME\n\nMyLib - Example library\n\n=head1 SYNOPSIS\n\n    use MyLib;\n\n=head2 process\n\nProcesses data.\n\n=cut\n\nsub process { }\n1;\n";
    open_document(&server, "file:///test_pod.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_pod.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((10) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    // Code symbols must still be present
    assert!(all_symbols.iter().any(|s| s["name"] == "MyLib"), "Missing package symbol");
    assert!(
        all_symbols.iter().any(|s| s["name"] == "process" && s["kind"] == 12),
        "Missing sub symbol (kind 12)"
    );

    // POD section symbols must appear
    assert!(all_symbols.iter().any(|s| s["name"] == "NAME"), "Missing =head1 NAME");
    assert!(all_symbols.iter().any(|s| s["name"] == "SYNOPSIS"), "Missing =head1 SYNOPSIS");

    // POD section kind must be 26 (TypeParameter)
    let name_sym = all_symbols.iter().find(|s| s["name"] == "NAME").ok_or("NAME not found")?;
    assert_eq!(name_sym["kind"], 26, "POD section kind must be 26 (TypeParameter)");

    // Line ordering: NAME must appear before SYNOPSIS
    let name_line = all_symbols
        .iter()
        .find(|s| s["name"] == "NAME" && s["kind"] == 26)
        .and_then(|s| s["range"]["start"]["line"].as_u64())
        .ok_or("NAME line not found")?;
    let synopsis_line = all_symbols
        .iter()
        .find(|s| s["name"] == "SYNOPSIS" && s["kind"] == 26)
        .and_then(|s| s["range"]["start"]["line"].as_u64())
        .ok_or("SYNOPSIS line not found")?;
    assert!(name_line < synopsis_line, "NAME must appear before SYNOPSIS");

    Ok(())
}

#[test]
fn test_pod_sections_stop_at_data_block() -> TestResult {
    let server = setup_server();
    let content = "package Foo;\n\n=head1 NAME\n\nFoo - real module\n\n=cut\n\nsub new { bless {}, shift }\n1;\n\n__DATA__\n\n=head1 SHOULD NOT APPEAR\n\nThis section is in __DATA__ and must not show up.\n";
    open_document(&server, "file:///test_data_pod.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_data_pod.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((11) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    assert!(
        all_symbols.iter().any(|s| s["name"] == "NAME" && s["kind"] == 26),
        "Real NAME section must appear"
    );
    assert!(
        !all_symbols.iter().any(|s| s["name"] == "SHOULD NOT APPEAR"),
        "POD in __DATA__ block must not appear in symbols"
    );

    Ok(())
}

#[test]
fn test_pod_section_multiword_title() -> TestResult {
    let server = setup_server();
    let content = "=head1 SEE ALSO\n\nSee L<Other::Module>.\n\n=cut\n1;\n";
    open_document(&server, "file:///test_multiword.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_multiword.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((12) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    assert!(
        all_symbols.iter().any(|s| s["name"] == "SEE ALSO" && s["kind"] == 26),
        "Multi-word POD heading must appear as full string"
    );

    Ok(())
}

#[test]
fn test_no_pod_unchanged_symbols() -> TestResult {
    let server = setup_server();
    let content = "package Bar;\nsub baz { }\n1;\n";
    open_document(&server, "file:///test_nopod.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_nopod.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((13) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    assert!(all_symbols.iter().any(|s| s["name"] == "Bar"), "Package symbol must remain");
    assert!(all_symbols.iter().any(|s| s["name"] == "baz"), "Sub symbol must remain");
    assert!(
        !all_symbols.iter().any(|s| s["kind"] == 26),
        "No POD symbols should appear for file with no POD"
    );

    Ok(())
}

#[test]
fn test_pod_sections_reject_invalid_levels() -> TestResult {
    let server = setup_server();
    let content =
        "=head0 INVALID LEVEL\n=head1 VALID\n=head5 INVALID LEVEL\n=head2 ALSO VALID\n1;\n";
    open_document(&server, "file:///test_levels.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_levels.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((14) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    // Valid levels should appear
    assert!(
        all_symbols.iter().any(|s| s["name"] == "VALID" && s["kind"] == 26),
        "Valid =head1 must appear"
    );
    assert!(
        all_symbols.iter().any(|s| s["name"] == "ALSO VALID" && s["kind"] == 26),
        "Valid =head2 must appear"
    );

    // Invalid levels should NOT appear
    assert!(
        !all_symbols.iter().any(|s| s["name"] == "INVALID LEVEL"),
        "=head0 and =head5 must be rejected"
    );

    Ok(())
}

#[test]
fn test_pod_sections_stop_at_end_block() -> TestResult {
    // __END__ is the other data-marker; the scan must stop there too.
    let server = setup_server();
    let content = "package Bar;\n\n=head1 BEFORE\n\n=cut\n\n1;\n\n__END__\n\n=head1 AFTER END\n\nShould not appear.\n";
    open_document(&server, "file:///test_end_pod.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_end_pod.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((15) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    assert!(
        all_symbols.iter().any(|s| s["name"] == "BEFORE" && s["kind"] == 26),
        "Section before __END__ must appear"
    );
    assert!(
        !all_symbols.iter().any(|s| s["name"] == "AFTER END"),
        "Section after __END__ must not appear in document symbols"
    );

    Ok(())
}

#[test]
fn test_pod_section_unicode_heading() -> TestResult {
    // Unicode in POD headings is valid (perldoc frequently uses it).
    // Verify the symbol name round-trips correctly and byte_to_utf16_col
    // produces a non-zero end character for multi-byte characters.
    let server = setup_server();
    let content =
        "=head1 Ñoño\n\nSpanish section.\n\n=head2 日本語\n\nJapanese section.\n\n=cut\n1;\n";
    open_document(&server, "file:///test_unicode_pod.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_unicode_pod.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((16) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);

    // Names must be the full Unicode string, not garbled bytes
    assert!(
        all_symbols.iter().any(|s| s["name"] == "Ñoño" && s["kind"] == 26),
        "Latin-extended POD heading must round-trip correctly"
    );
    assert!(
        all_symbols.iter().any(|s| s["name"] == "日本語" && s["kind"] == 26),
        "CJK POD heading must round-trip correctly"
    );

    // The end character must be > 0 (multi-byte content means nonzero columns)
    let cjk_sym = all_symbols
        .iter()
        .find(|s| s["name"] == "日本語" && s["kind"] == 26)
        .ok_or("CJK heading not found")?;
    let end_char = cjk_sym["range"]["end"]["character"].as_u64().ok_or("no end char")?;
    assert!(end_char > 0, "end character must be > 0 for multi-byte heading");

    Ok(())
}

// ---- Phase block tests (issue #3464) ----

#[test]
fn test_begin_block_appears_in_document_symbols() -> TestResult {
    let server = setup_server();
    let content = "package MyModule;\n\nBEGIN {\n    require Config;\n}\n\nsub hello { }\n\n1;\n";
    open_document(&server, "file:///test_begin.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_begin.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((20) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);
    let names: Vec<&str> =
        all_symbols.iter().filter_map(|symbol| symbol["name"].as_str()).collect();

    assert!(
        names.contains(&"BEGIN"),
        "BEGIN block must appear in document symbols; got: {:?}",
        names
    );

    let begin_sym = all_symbols
        .iter()
        .find(|symbol| symbol["name"].as_str() == Some("BEGIN"))
        .ok_or("BEGIN not found")?;
    // Phase blocks map to Function (12) kind
    assert_eq!(begin_sym["kind"], 12, "BEGIN block should have Function (12) kind");

    Ok(())
}

#[test]
fn test_end_block_appears_in_document_symbols() -> TestResult {
    let server = setup_server();
    let content = "package MyModule;\n\nEND {\n    cleanup();\n}\n\nsub cleanup { }\n\n1;\n";
    open_document(&server, "file:///test_end.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_end.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((21) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let names = symbol_names(symbols);

    assert!(names.contains(&"END"), "END block must appear in document symbols; got: {:?}", names);

    Ok(())
}

#[test]
fn test_all_phase_blocks_appear_in_document_symbols() -> TestResult {
    let server = setup_server();
    let content = "BEGIN { require Config; }\nEND { cleanup(); }\nCHECK { verify(); }\nINIT { initialize(); }\nUNITCHECK { unit_check(); }\n\nsub cleanup { }\nsub verify { }\nsub initialize { }\nsub unit_check { }\n";
    open_document(&server, "file:///test_phases.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_phases.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((22) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;
    let all_symbols = flatten_symbols(symbols);
    let names: Vec<&str> =
        all_symbols.iter().filter_map(|symbol| symbol["name"].as_str()).collect();

    for phase in &["BEGIN", "END", "CHECK", "INIT", "UNITCHECK"] {
        assert!(
            names.contains(phase),
            "{} block must appear in document symbols; got: {:?}",
            phase,
            names
        );
        let sym = all_symbols
            .iter()
            .find(|symbol| symbol["name"].as_str() == Some(*phase))
            .ok_or(*phase)?;
        assert_eq!(sym["kind"], 12, "{} must have Function (12) kind", phase);
    }

    Ok(())
}

#[test]
fn test_multiple_begin_blocks_all_appear() -> TestResult {
    // Multiple BEGIN blocks are common in Perl modules
    let server = setup_server();
    let content = "BEGIN { require 1; }\nBEGIN { require 2; }\n\nsub main_logic { }\n";
    open_document(&server, "file:///test_multi_begin.pm", content);
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({ "textDocument": { "uri": "file:///test_multi_begin.pm" } })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((23) as i64)),
    };
    let response = server.handle_request(request).ok_or("No response")?;
    let result = response.result.ok_or("Missing result")?;
    let symbols = result.as_array().ok_or("Not an array")?;

    let begin_count = flatten_symbols(symbols)
        .into_iter()
        .filter(|symbol| symbol["name"].as_str() == Some("BEGIN"))
        .count();
    assert_eq!(begin_count, 2, "Both BEGIN blocks must appear; got {}", begin_count);

    Ok(())
}
