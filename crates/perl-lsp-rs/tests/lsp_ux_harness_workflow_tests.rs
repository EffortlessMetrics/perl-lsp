//! UX-focused workflow coverage using the canonical `LspHarness`.
//!
//! These tests validate common editor interactions end-to-end so that
//! harness-level regressions are caught where developers feel them.

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn has_non_empty_array(value: &serde_json::Value) -> bool {
    value.as_array().is_some_and(|items| !items.is_empty())
}

#[test]
fn ux_workflow_completion_hover_and_definition() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;

    let uri = "file:///ux_flow.pl";
    let doc = r#"use strict;
use warnings;

sub helper {
    return shift;
}

my $value = helper(41);
$val
"#;
    harness.open(uri, doc)?;
    harness.barrier();

    let completion = harness.completion(uri, 8, 4)?;
    let labels = completion
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("label").and_then(|label| label.as_str()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        labels.contains(&"$value"),
        "completion should suggest lexical variable '$value', got labels: {labels:?}"
    );

    let hover = harness.hover(uri, 7, 12)?;
    assert!(
        hover.get("contents").is_some(),
        "hover on helper call should include contents: {hover:#}"
    );

    let definition = harness.definition(uri, 7, 12)?;
    assert!(
        has_non_empty_array(&definition),
        "goto-definition should return at least one location: {definition:#}"
    );

    harness.shutdown_gracefully();
    Ok(())
}

#[test]
fn ux_workflow_diagnostics_update_after_edit() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;

    let uri = "file:///ux_diagnostics.pl";
    harness.open(uri, "my $x = ;\n")?;
    harness.barrier();

    let initial = harness.document_diagnostic(uri)?;
    let initial_items =
        initial.get("items").and_then(|items| items.as_array()).cloned().unwrap_or_default();
    assert!(!initial_items.is_empty(), "invalid Perl should report diagnostics: {initial:#}");

    harness.change_full(uri, 2, "my $x = 1;\nprint $x;\n")?;
    harness.did_save(uri)?;
    harness.barrier();

    let after_fix = harness.document_diagnostic(uri)?;
    let fixed_items =
        after_fix.get("items").and_then(|items| items.as_array()).cloned().unwrap_or_default();
    assert!(
        fixed_items.len() < initial_items.len(),
        "fixing syntax should reduce diagnostics. before={}, after={}, full_after={after_fix:#}",
        initial_items.len(),
        fixed_items.len()
    );

    harness.shutdown_gracefully();
    Ok(())
}

#[test]
fn ux_workflow_workspace_symbol_round_trip_with_temp_workspace() -> TestResult {
    let files = [
        ("lib/Foo.pm", "package Foo;\nsub run { return 1; }\n1;\n"),
        ("bin/main.pl", "use lib 'lib';\nuse Foo;\nmy $x = Foo::run();\n"),
    ];

    let (mut harness, workspace) = LspHarness::with_workspace(&files)?;

    harness.wait_for_symbol("Foo", None, Duration::from_secs(3))?;

    let symbols = harness.request("workspace/symbol", json!({ "query": "Foo" }))?;
    let symbol_array = symbols.as_array().cloned().unwrap_or_default();
    assert!(!symbol_array.is_empty(), "workspace/symbol should include Foo results: {symbols:#}");

    let expected_fragment =
        format!("{}/lib/Foo.pm", workspace.root_uri.trim_start_matches("file://"));
    let found_location = symbol_array.iter().any(|symbol| {
        symbol
            .pointer("/location/uri")
            .and_then(|uri| uri.as_str())
            .is_some_and(|uri| uri.contains("Foo.pm") || uri.contains(&expected_fragment))
    });
    assert!(found_location, "expected Foo.pm symbol location in response: {symbols:#}");

    harness.shutdown_gracefully();
    Ok(())
}
