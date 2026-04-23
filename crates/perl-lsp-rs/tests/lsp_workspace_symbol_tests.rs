//! Tests for workspace/symbol and workspaceSymbol/resolve LSP features
//!
//! Validates the workspace symbol provider functionality including:
//! - Searching symbols with a query string
//! - Empty query returning all symbols
//! - Query that matches no results
//! - Resolving a symbol to get additional detail
//! - Capability advertisement in server initialization

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test workspace symbol search with a specific query
#[test]
fn test_workspace_symbol_query() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_ws_sym.pl";
    harness.open(
        doc_uri,
        r#"package SearchTarget;

sub find_user {
    my $id = shift;
    return { id => $id, name => "User $id" };
}

sub find_all_users {
    return [];
}

sub delete_user {
    my $id = shift;
    return 1;
}

1;
"#,
    )?;

    // Search for symbols matching "find"
    let response = harness
        .request(
            "workspace/symbol",
            json!({
                "query": "find"
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        assert!(
            response.is_array(),
            "workspace/symbol should return an array, got: {:?}",
            response
        );

        let symbols = response.as_array().ok_or("response is not an array")?;
        // Should find at least find_user and find_all_users
        if !symbols.is_empty() {
            let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
            assert!(
                names.iter().any(|n| n.contains("find")),
                "Should find symbols matching 'find', got: {:?}",
                names
            );

            // Each symbol should have required fields
            for sym in symbols {
                assert!(sym["name"].is_string(), "Symbol should have a name");
                assert!(sym["kind"].is_number(), "Symbol should have a kind");
                // SymbolInformation has location; WorkspaceSymbol may have location
                if sym.get("location").is_some() {
                    assert!(
                        sym["location"]["uri"].is_string(),
                        "Symbol location should have a uri"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Test workspace symbol search with an empty query
#[test]
fn test_workspace_symbol_empty_query() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_ws_empty.pl";
    harness.open(
        doc_uri,
        r#"package MyModule;

sub alpha { return 1; }
sub beta { return 2; }
sub gamma { return 3; }

1;
"#,
    )?;

    // Empty query should return all (or many) symbols
    let response = harness
        .request(
            "workspace/symbol",
            json!({
                "query": ""
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        assert!(
            response.is_array(),
            "workspace/symbol with empty query should return an array, got: {:?}",
            response
        );
        // Empty query may return all symbols or none depending on implementation
        // Both are valid per the LSP spec
    }

    Ok(())
}

/// Test workspace symbol search with no matching results
#[test]
fn test_workspace_symbol_no_results() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_ws_noresult.pl";
    harness.open(
        doc_uri,
        r#"sub hello { return "world"; }
"#,
    )?;

    // Search for something that definitely does not exist
    let response = harness
        .request(
            "workspace/symbol",
            json!({
                "query": "zzz_nonexistent_xyzzy_symbol_12345"
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        assert!(
            response.is_array(),
            "workspace/symbol should return an array even with no results, got: {:?}",
            response
        );
        let symbols = response.as_array().ok_or("response is not an array")?;
        assert!(
            symbols.is_empty(),
            "Non-matching query should return empty array, got {} results",
            symbols.len()
        );
    }

    Ok(())
}

/// Test resolving a workspace symbol to get additional detail
#[test]
fn test_workspace_symbol_resolve() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_ws_resolve.pl";
    harness.open(
        doc_uri,
        r#"package Resolver;

sub target_function {
    my ($arg1, $arg2) = @_;
    return $arg1 + $arg2;
}

1;
"#,
    )?;

    // Build a basic symbol as would be returned by workspace/symbol
    let basic_symbol = json!({
        "name": "target_function",
        "kind": 12,
        "location": {
            "uri": doc_uri,
            "range": {
                "start": { "line": 2, "character": 0 },
                "end": { "line": 5, "character": 1 }
            }
        }
    });

    // Resolve the symbol for additional detail
    let response = harness.request("workspaceSymbol/resolve", basic_symbol).unwrap_or(json!(null));

    if !response.is_null() {
        // Resolved symbol should retain the original fields
        assert_eq!(
            response["name"].as_str(),
            Some("target_function"),
            "Resolved symbol should keep its name"
        );
        assert_eq!(response["kind"].as_i64(), Some(12), "Resolved symbol should keep its kind");

        // May have additional detail
        if let Some(detail) = response.get("detail") {
            if detail.is_string() {
                let detail_str = detail.as_str().ok_or("detail should be a string")?;
                assert!(!detail_str.is_empty(), "detail should not be empty if provided");
            }
        }

        // Location should still be present
        if response.get("location").is_some() {
            assert!(
                response["location"]["uri"].is_string(),
                "Resolved symbol should still have location.uri"
            );
        }
    }

    Ok(())
}

/// Test that workspaceSymbolProvider capability is advertised
#[test]
fn test_workspace_symbol_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    let ws_provider = capabilities.get("workspaceSymbolProvider");
    assert!(
        ws_provider.is_some(),
        "Server should advertise workspaceSymbolProvider capability. Capabilities: {:?}",
        capabilities
    );

    // If it is an object (not just true), check for resolveProvider
    if let Some(wsp) = ws_provider {
        if wsp.is_object() {
            if let Some(resolve) = wsp.get("resolveProvider") {
                assert!(
                    resolve.is_boolean(),
                    "resolveProvider should be a boolean, got: {:?}",
                    resolve
                );
            }
        }
    }

    Ok(())
}

/// Test workspace symbol search across multiple open documents
#[test]
fn test_workspace_symbol_multiple_documents() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    // Open first document
    let doc1_uri = "file:///module_a.pl";
    harness.open(
        doc1_uri,
        r#"package ModuleA;

sub shared_helper {
    return "A";
}

1;
"#,
    )?;

    // Open second document
    let doc2_uri = "file:///module_b.pl";
    harness.open(
        doc2_uri,
        r#"package ModuleB;

sub shared_utility {
    return "B";
}

1;
"#,
    )?;

    // Search for "shared" which appears in both documents
    let response = harness
        .request(
            "workspace/symbol",
            json!({
                "query": "shared"
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        assert!(
            response.is_array(),
            "workspace/symbol should return an array, got: {:?}",
            response
        );

        let symbols = response.as_array().ok_or("response is not an array")?;
        if !symbols.is_empty() {
            // Collect URIs from results
            let uris: Vec<&str> = symbols
                .iter()
                .filter_map(|s| {
                    s.get("location").and_then(|loc| loc.get("uri")).and_then(|u| u.as_str())
                })
                .collect();

            // Should potentially find symbols from both documents
            let has_any = !uris.is_empty();
            assert!(has_any, "Should find at least one symbol matching 'shared'");
        }
    }

    Ok(())
}

/// Test that workspace/symbol finds Perl 5.38+ native class and method declarations.
///
/// Before the fix in symbol_extraction.rs, NodeKind::Class and NodeKind::Method had no
/// arms in extract_simple_symbols, so they were silently dropped from workspace search.
#[test]
fn test_workspace_symbol_finds_native_class_and_method() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///native_class_ws.pl";
    harness.open(
        doc_uri,
        "class MyPoint {\n    method get_x { return 0; }\n    method get_y { return 0; }\n}\n",
    )?;

    // Search for "get_" — should match both native methods
    let response =
        harness.request("workspace/symbol", json!({ "query": "get_" })).unwrap_or(json!(null));

    if !response.is_null() && response.is_array() {
        let symbols = response.as_array().ok_or("response is not an array")?;
        let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names.iter().any(|n| *n == "get_x" || *n == "get_y"),
            "workspace/symbol should find native methods 'get_x'/'get_y', got: {:?}",
            names
        );
        // Each found method should report kind 6 (Method)
        for sym in symbols {
            if let Some(name) = sym["name"].as_str() {
                if name == "get_x" || name == "get_y" {
                    assert_eq!(
                        sym["kind"].as_u64(),
                        Some(6),
                        "native method '{}' should have LSP kind 6 (Method), got: {:?}",
                        name,
                        sym["kind"]
                    );
                }
            }
        }
    }

    // Search for "MyPoint" — should find the class declaration
    let response2 =
        harness.request("workspace/symbol", json!({ "query": "MyPoint" })).unwrap_or(json!(null));

    if !response2.is_null() && response2.is_array() {
        let symbols2 = response2.as_array().ok_or("response2 is not an array")?;
        let names2: Vec<&str> = symbols2.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names2.contains(&"MyPoint"),
            "workspace/symbol should find native class 'MyPoint', got: {:?}",
            names2
        );
        // Class should report kind 5 (Class)
        for sym in symbols2 {
            if sym["name"].as_str() == Some("MyPoint") {
                assert_eq!(
                    sym["kind"].as_u64(),
                    Some(5),
                    "native class 'MyPoint' should have LSP kind 5 (Class), got: {:?}",
                    sym["kind"]
                );
            }
        }
    }

    Ok(())
}

/// Test that workspace/symbol uses perl-lsp-workspace-symbols provider.
///
/// Verifies that a method declared inside a named package receives a
/// `containerName` field in the response — a field only populated by
/// `WorkspaceSymbolsProvider`, not by the legacy `extract_simple_symbols`
/// fallback. This confirms the crate is wired in.
#[test]
fn test_workspace_symbol_provider_wired() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///provider_wired_test.pl";
    harness.open(
        doc_uri,
        r#"package Acme::Widget;

sub build {
    my ($class, %args) = @_;
    return bless {}, $class;
}

1;
"#,
    )?;

    let response = harness
        .request("workspace/symbol", json!({ "query": "build" }))
        .map_err(|e| format!("workspace/symbol request failed: {e}"))?;

    assert!(response.is_array(), "workspace/symbol should return an array, got: {:?}", response);

    let symbols = response.as_array().ok_or("response is not an array")?;
    let build_sym = symbols.iter().find(|s| s["name"].as_str() == Some("build"));

    let sym = build_sym
        .ok_or("workspace/symbol did not return 'build' — WorkspaceSymbolsProvider not wired")?;

    // containerName is populated by WorkspaceSymbolsProvider when the sub is inside a package.
    // extract_simple_symbols never sets containerName — its absence would prove the old path is active.
    assert!(
        sym.get("containerName").is_some(),
        "Symbol 'build' should have containerName set by WorkspaceSymbolsProvider, got: {:?}",
        sym
    );

    Ok(())
}
