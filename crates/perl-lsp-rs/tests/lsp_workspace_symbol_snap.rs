mod support;

use insta::assert_yaml_snapshot;
use serde::Serialize;
use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct SymbolSnapshot {
    name: String,
    kind: u64,
    container_name: Option<String>,
    uri: Option<String>,
    start_line: Option<u64>,
}

fn normalize_symbols(response: &Value) -> Result<Vec<SymbolSnapshot>, Box<dyn std::error::Error>> {
    let symbols = response
        .as_array()
        .ok_or_else(|| format!("workspace/symbol should return an array, got: {response:?}"))?;

    let mut normalized = symbols
        .iter()
        .map(|symbol| SymbolSnapshot {
            name: symbol["name"].as_str().unwrap_or_default().to_string(),
            kind: symbol["kind"].as_u64().unwrap_or_default(),
            container_name: symbol
                .get("containerName")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            uri: symbol
                .get("location")
                .and_then(|location| location.get("uri"))
                .and_then(|uri| uri.as_str())
                .map(ToOwned::to_owned),
            start_line: symbol
                .get("location")
                .and_then(|location| location.get("range"))
                .and_then(|range| range.get("start"))
                .and_then(|start| start.get("line"))
                .and_then(|line| line.as_u64()),
        })
        .collect::<Vec<_>>();

    normalized.sort();
    Ok(normalized)
}

#[test]
fn workspace_symbol_query_snapshot() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.open(
        "file:///ws_snap_a.pl",
        r#"package Acme::Customer;

sub ws_snap_find_user {
    return shift;
}

sub ws_snap_find_all_users {
    return [];
}

1;
"#,
    )?;

    harness.open(
        "file:///ws_snap_b.pl",
        r#"package Acme::Helpers;

sub ws_snap_find_order {
    return shift;
}

1;
"#,
    )?;

    let response = harness.request("workspace/symbol", json!({ "query": "ws_snap_find" }))?;
    let normalized = normalize_symbols(&response)?;

    assert_yaml_snapshot!("workspace_symbol_query_find", normalized);
    Ok(())
}

#[test]
fn workspace_symbol_native_class_snapshot() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.open(
        "file:///ws_native_class.pl",
        "class MyPoint {
    method ws_snap_get_x { return 0; }
    method ws_snap_get_y { return 1; }
}
",
    )?;

    let response = harness.request("workspace/symbol", json!({ "query": "ws_snap_get_" }))?;
    let normalized = normalize_symbols(&response)?;

    assert_yaml_snapshot!("workspace_symbol_native_methods", normalized);
    Ok(())
}

#[test]
fn workspace_symbol_capability_shape_snapshot() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;
    let capability = init_response
        .get("capabilities")
        .and_then(|capabilities| capabilities.get("workspaceSymbolProvider"))
        .ok_or("workspaceSymbolProvider capability missing")?;

    assert_yaml_snapshot!("workspace_symbol_provider_capability", capability);
    Ok(())
}
