//! BDD-style workflow coverage for core LSP behaviors.
//!
//! These tests are structured as Given/When/Then scenarios to validate
//! end-to-end user workflows using the real JSON-RPC harness.

mod support;

use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use support::lsp_harness::{LspHarness, TempWorkspace};

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {}", name);
        Self { name }
    }

    fn given(&self, msg: &str) {
        eprintln!("[{}] Given {}", self.name, msg);
    }

    fn when(&self, msg: &str) {
        eprintln!("[{}] When {}", self.name, msg);
    }

    fn then(&self, msg: &str) {
        eprintln!("[{}] Then {}", self.name, msg);
    }
}

fn find_position(text: &str, needle: &str) -> (u32, u32) {
    perl_tdd_support::must_some(
        text.split('\n').enumerate().find_map(|(line_idx, line)| {
            line.find(needle).map(|col| (line_idx as u32, col as u32))
        }),
    )
}

fn ref_uris(response: &Value) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    if let Some(arr) = response.as_array() {
        for item in arr {
            if let Some(uri) = item.get("uri").and_then(|v| v.as_str()) {
                uris.insert(uri.to_string());
            } else if let Some(uri) = item.pointer("/location/uri").and_then(|v| v.as_str()) {
                uris.insert(uri.to_string());
            }
        }
    }
    uris
}

fn workspace_edit_uris(edit: &Value) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();

    if let Some(changes) = edit.get("changes").and_then(|v| v.as_object()) {
        for (uri, _) in changes {
            uris.insert(uri.clone());
        }
    }

    if let Some(doc_changes) = edit.get("documentChanges").and_then(|v| v.as_array()) {
        for change in doc_changes {
            if let Some(uri) = change.pointer("/textDocument/uri").and_then(|v| v.as_str()) {
                uris.insert(uri.to_string());
            }
        }
    }

    uris
}

fn workspace_edit_new_texts_for_uri(edit: &Value, target_uri: &str) -> Vec<String> {
    let mut new_texts = Vec::new();

    if let Some(changes) = edit.get("changes").and_then(Value::as_object)
        && let Some(edits) = changes.get(target_uri).and_then(Value::as_array)
    {
        new_texts.extend(
            edits
                .iter()
                .filter_map(|entry| entry.get("newText").and_then(Value::as_str))
                .map(ToOwned::to_owned),
        );
    }

    if let Some(doc_changes) = edit.get("documentChanges").and_then(Value::as_array) {
        for change in doc_changes {
            let uri_matches =
                change.pointer("/textDocument/uri").and_then(Value::as_str) == Some(target_uri);
            if !uri_matches {
                continue;
            }

            if let Some(edits) = change.get("edits").and_then(Value::as_array) {
                new_texts.extend(
                    edits
                        .iter()
                        .filter_map(|entry| entry.get("newText").and_then(Value::as_str))
                        .map(ToOwned::to_owned),
                );
            }
        }
    }

    new_texts
}

fn uri_matches(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }

    if cfg!(windows) {
        return expected.eq_ignore_ascii_case(actual);
    }

    false
}

fn uri_set_contains(uris: &BTreeSet<String>, target_uri: &str) -> bool {
    uris.iter().any(|uri| uri_matches(target_uri, uri))
}

fn first_location_uri(response: &Value) -> Option<String> {
    if let Some(arr) = response.as_array() {
        arr.first().and_then(|v| v.get("uri").and_then(Value::as_str)).map(ToOwned::to_owned)
    } else {
        response.get("uri").and_then(Value::as_str).map(ToOwned::to_owned)
    }
}

fn wait_for_definition_uri(
    harness: &mut LspHarness,
    request_uri: &str,
    line: u32,
    character: u32,
    want_uri: &str,
    budget: Duration,
) -> Result<Value, String> {
    let start = Instant::now();
    let mut last_response = None;

    while start.elapsed() < budget {
        let response = harness.request_with_timeout(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": request_uri },
                "position": { "line": line, "character": character }
            }),
            Duration::from_millis(500),
        )?;

        if first_location_uri(&response).as_deref() == Some(want_uri) {
            return Ok(response);
        }

        last_response = Some(response);
        harness.barrier();
        std::thread::sleep(Duration::from_millis(50));
    }

    Err(format!(
        "definition did not resolve to {want_uri} within {budget:?}; last response: {last_response:?}"
    ))
}

fn wait_for_references_uris(
    harness: &mut LspHarness,
    request_uri: &str,
    line: u32,
    character: u32,
    want_uris: &[&str],
    budget: Duration,
) -> Result<Value, String> {
    let start = Instant::now();
    let mut last_response = None;
    let mut last_error = None;

    while start.elapsed() < budget {
        match harness.request_with_timeout(
            "textDocument/references",
            json!({
                "textDocument": { "uri": request_uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
            Duration::from_secs(2),
        ) {
            Ok(response) => {
                let uris = ref_uris(&response);
                if want_uris.iter().all(|want_uri| uri_set_contains(&uris, want_uri)) {
                    return Ok(response);
                }
                last_response = Some(response);
            }
            Err(error) => last_error = Some(error),
        }

        harness.barrier();
        std::thread::sleep(Duration::from_millis(50));
    }

    Err(format!(
        "references did not include {:?} within {:?}; last response: {:?}; last error: {:?}",
        want_uris, budget, last_response, last_error
    ))
}

fn wait_for_rename_edit_uris(
    harness: &mut LspHarness,
    request_uri: &str,
    line: u32,
    character: u32,
    new_name: &str,
    want_uris: &[&str],
    budget: Duration,
) -> Result<Value, String> {
    let start = Instant::now();
    let mut last_response = None;
    let mut last_error = None;

    while start.elapsed() < budget {
        match harness.request_with_timeout(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": request_uri },
                "position": { "line": line, "character": character },
                "newName": new_name
            }),
            Duration::from_secs(2),
        ) {
            Ok(response) => {
                let uris = workspace_edit_uris(&response);
                if want_uris.iter().all(|want_uri| uri_set_contains(&uris, want_uri)) {
                    return Ok(response);
                }
                last_response = Some(response);
            }
            Err(error) => last_error = Some(error),
        }

        harness.barrier();
        std::thread::sleep(Duration::from_millis(50));
    }

    Err(format!(
        "rename did not touch {:?} within {:?}; last response: {:?}; last error: {:?}",
        want_uris, budget, last_response, last_error
    ))
}

fn location_start_line(response: &Value) -> Option<u64> {
    if let Some(arr) = response.as_array() {
        arr.first().and_then(|v| v.pointer("/range/start/line").and_then(Value::as_u64))
    } else {
        response.pointer("/range/start/line").and_then(Value::as_u64)
    }
}

fn completion_labels(response: &Value) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    let items = response.get("items").and_then(Value::as_array).or_else(|| response.as_array());

    if let Some(items) = items {
        for item in items {
            if let Some(label) = item.get("label").and_then(Value::as_str) {
                labels.insert(label.to_string());
            }
        }
    }

    labels
}

fn hover_text(hover: &Value) -> String {
    if let Some(text) = hover.pointer("/contents/value").and_then(Value::as_str) {
        return text.to_string();
    }

    if let Some(text) = hover.get("contents").and_then(Value::as_str) {
        return text.to_string();
    }

    if let Some(arr) = hover.get("contents").and_then(Value::as_array) {
        let combined = arr
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .map(ToOwned::to_owned)
                    .or_else(|| item.get("value").and_then(Value::as_str).map(ToOwned::to_owned))
            })
            .collect::<Vec<_>>()
            .join("\n");
        return combined;
    }

    String::new()
}

fn diagnostic_items(report: &Value) -> &[Value] {
    report.get("items").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn highlight_items(response: &Value) -> &[Value] {
    response.as_array().map_or(&[], Vec::as_slice)
}

fn diagnostic_error_count(report: &Value) -> usize {
    diagnostic_items(report)
        .iter()
        .filter(|diag| diag.get("severity").and_then(Value::as_u64) == Some(1))
        .count()
}

fn selection_range_depth(selection_range: &Value) -> usize {
    let mut depth = 0;
    let mut current = selection_range;

    loop {
        depth += 1;
        let Some(parent) = current.get("parent") else {
            break;
        };
        if parent.is_null() {
            break;
        }
        current = parent;
    }

    depth
}

fn collect_symbol_names(symbol: &Value, names: &mut Vec<String>) {
    if let Some(name) = symbol.get("name").and_then(Value::as_str) {
        names.push(name.to_string());
    }

    if let Some(children) = symbol.get("children").and_then(Value::as_array) {
        for child in children {
            collect_symbol_names(child, names);
        }
    }
}

fn symbol_names(response: &Value) -> Vec<String> {
    let mut names = Vec::new();

    if let Some(arr) = response.as_array() {
        for symbol in arr {
            collect_symbol_names(symbol, &mut names);
        }
    }

    names
}

fn code_action_titles(actions: &Value) -> Vec<String> {
    actions
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|action| {
                    action.get("title").and_then(Value::as_str).map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn has_lsp_range(value: &Value) -> bool {
    let range = if value.get("start").is_some() && value.get("end").is_some() {
        value
    } else {
        value.get("range").unwrap_or(&Value::Null)
    };

    range.get("start").is_some() && range.get("end").is_some()
}

fn highlight_kinds(response: &Value) -> Vec<u64> {
    response
        .as_array()
        .map(|items| {
            items.iter().filter_map(|item| item.get("kind").and_then(Value::as_u64)).collect()
        })
        .unwrap_or_default()
}

fn inlay_labels(response: &Value) -> Vec<String> {
    response
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("label").and_then(Value::as_str).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn line_span(range: &Value) -> Option<(u64, u64)> {
    let start = range.pointer("/range/start/line").and_then(Value::as_u64)?;
    let end = range.pointer("/range/end/line").and_then(Value::as_u64)?;
    Some((start, end))
}

fn setup_workspace(files: &[(&str, &str)]) -> Result<(LspHarness, TempWorkspace), String> {
    let (mut harness, workspace) = LspHarness::with_workspace(files)?;

    // Give the server a moment to settle after initialize.
    harness.barrier();

    Ok((harness, workspace))
}

#[test]
#[serial]
fn bdd_definition_and_references_across_files() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Cross-file definition and references");

    let module = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return 1;
}

sub call_internal {
    return process_data();
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_data();
my $also = process_data();
"#;

    scenario.given("a workspace with a module and a script that call the same function");
    let (mut harness, workspace) = setup_workspace(&[("lib/Foo.pm", module), ("main.pl", main)])?;

    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module)?;
    harness.open(&main_uri, main)?;
    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("requesting definition on the qualified call in the script");
    let (line, character) = find_position(main, "process_data()");
    let definition = wait_for_definition_uri(
        &mut harness,
        &main_uri,
        line,
        character,
        &module_uri,
        Duration::from_secs(10),
    )?;

    scenario.then("the definition resolves to the module file");
    let def_uri = first_location_uri(&definition).unwrap_or_default();
    assert_eq!(def_uri, module_uri);

    scenario.when("requesting references on the module definition");
    let (def_line, def_char) = find_position(module, "process_data");
    let references = wait_for_references_uris(
        &mut harness,
        &module_uri,
        def_line,
        def_char,
        &[&module_uri, &main_uri],
        Duration::from_secs(10),
    )?;

    scenario.then("references include both module and script locations");
    let uris = ref_uris(&references);
    assert!(uris.contains(&module_uri), "references should include module file");
    assert!(uris.contains(&main_uri), "references should include main script file");

    Ok(())
}

#[test]
#[serial]
fn bdd_rename_updates_workspace_edits() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Rename propagates across workspace");

    let module = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return 1;
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_data();
my $also = process_data();
"#;

    scenario.given("a workspace with qualified and bare calls to the same function");
    let (mut harness, workspace) = setup_workspace(&[("lib/Foo.pm", module), ("main.pl", main)])?;

    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module)?;
    harness.open(&main_uri, main)?;

    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("renaming the function at its declaration");
    let (def_line, def_char) = find_position(module, "process_data");
    let edit = wait_for_rename_edit_uris(
        &mut harness,
        &module_uri,
        def_line,
        def_char,
        "process_records",
        &[&module_uri, &main_uri],
        Duration::from_secs(10),
    )?;

    scenario.then("the workspace edit touches both files");
    let uris = workspace_edit_uris(&edit);
    assert!(uris.contains(&module_uri), "rename should edit module file");
    assert!(uris.contains(&main_uri), "rename should edit main script file");

    scenario.then("rename edits include the new symbol text in both files");
    let module_texts = workspace_edit_new_texts_for_uri(&edit, &module_uri);
    let main_texts = workspace_edit_new_texts_for_uri(&edit, &main_uri);
    assert!(
        module_texts.iter().any(|text| text.contains("process_records")),
        "module edits should contain new function name; got {module_texts:?}"
    );
    assert!(
        main_texts.iter().any(|text| text.contains("process_records")),
        "main edits should contain new function name; got {main_texts:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_workspace_symbols_expose_module_api() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Workspace symbol search surfaces module APIs");

    let module = r#"package Toolkit;
use strict;
use warnings;

sub transform {
    return "ok";
}

1;
"#;

    scenario.given("a workspace with a module defining a public function");
    let (mut harness, workspace) = setup_workspace(&[("lib/Toolkit.pm", module)])?;

    let module_uri = workspace.uri("lib/Toolkit.pm");
    harness.open(&module_uri, module)?;

    harness.wait_for_symbol("transform", Some(&module_uri), Duration::from_secs(2)).ok();

    scenario.when("searching workspace symbols for the function name");
    let result = harness.request(
        "workspace/symbol",
        json!({
            "query": "transform"
        }),
    )?;

    scenario.then("the symbol list contains the module function");
    let names: Vec<String> = match result.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect(),
        None => Vec::new(),
    };

    assert!(
        names.iter().any(|n| n == "transform" || n.ends_with("transform")),
        "workspace symbols should include 'transform'"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_editor_intelligence_for_test_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Editor intelligence for test workflow");

    let test_file = r#"use strict;
use warnings;
use Test::More tests => 1;

sub calculate_total {
    my ($left, $right) = @_;
    return $left + $right;
}

my $value = calc
is(calculate_total(1, 2), 3, 'adds values');
"#;

    scenario.given("a test file with a local helper function and an in-progress call site");
    let (mut harness, workspace) = setup_workspace(&[("t/calculator.t", test_file)])?;
    let uri = workspace.uri("t/calculator.t");
    harness.open(&uri, test_file)?;

    harness.wait_for_symbol("calculate_total", Some(&uri), Duration::from_secs(2)).ok();

    scenario.when("requesting completion at a partially typed function name");
    let (completion_line, completion_col) = find_position(test_file, "my $value = calc");
    let completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": {
                "line": completion_line,
                "character": completion_col + "my $value = calc".len() as u32
            }
        }),
    )?;

    scenario.then("completion includes the local helper function");
    let labels = completion_labels(&completion);
    assert!(
        labels.iter().any(|label| label == "calculate_total" || label.ends_with("calculate_total")),
        "completion should include calculate_total; got {labels:?}"
    );

    scenario.when("requesting hover on the helper call in an assertion");
    let (hover_line, hover_col) = find_position(test_file, "calculate_total(1, 2)");
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": hover_line, "character": hover_col }
        }),
    )?;

    scenario.then("hover returns non-empty content");
    assert!(!hover_text(&hover).is_empty(), "hover content should be non-empty");

    scenario.when("requesting signature help while editing function arguments");
    let signature_help = harness.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": uri },
            "position": {
                "line": hover_line,
                "character": hover_col + "calculate_total(1, ".len() as u32
            }
        }),
    )?;

    scenario.then("signature help includes at least one signature");
    let signatures =
        signature_help.get("signatures").and_then(Value::as_array).cloned().unwrap_or_default();
    assert!(!signatures.is_empty(), "signature help should include signatures");

    Ok(())
}

#[test]
#[serial]
fn bdd_pull_diagnostics_recovers_after_syntax_fix() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Pull diagnostics recover after syntax fix");

    let broken = r#"use strict;
use warnings;

sub compute_value {
    my ($x) = @_;
    if ($x > 10 {
        return $x;
    }
    return 0;
}
"#;

    let fixed = r#"use strict;
use warnings;

sub compute_value {
    my ($x) = @_;
    if ($x > 10) {
        return $x;
    }
    return 0;
}
"#;

    scenario.given("a Perl file with a real syntax error");
    let (mut harness, workspace) = setup_workspace(&[("broken.pl", broken)])?;
    let uri = workspace.uri("broken.pl");
    harness.open(&uri, broken)?;

    scenario.when("requesting pull diagnostics");
    let broken_report = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("diagnostics include parse issues");
    let broken_item_count = diagnostic_items(&broken_report).len();
    assert!(broken_item_count > 0, "broken file should produce diagnostics");

    scenario.when("fixing the syntax error with an incremental didChange");
    harness.change_full(&uri, 2, fixed)?;
    harness.barrier();

    let fixed_report = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("error-level diagnostics are cleared");
    let fixed_item_count = diagnostic_items(&fixed_report).len();
    let fixed_errors = diagnostic_error_count(&fixed_report);
    assert!(
        fixed_item_count < broken_item_count,
        "fixed code should reduce diagnostics (broken={broken_item_count}, fixed={fixed_item_count})"
    );
    assert_eq!(fixed_errors, 0, "fixed code should have no error diagnostics");

    Ok(())
}

#[test]
#[serial]
fn bdd_local_variable_navigation_and_highlights() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Local variable navigation and highlights stay aligned");

    let script = r#"use strict;
use warnings;

my $value = 41;
my $result = $value + 1;
print $value;
$value = $result;
"#;

    scenario.given("a Perl script with a local variable used for reads and writes");
    let (mut harness, workspace) = setup_workspace(&[("variable_flow.pl", script)])?;
    let uri = workspace.uri("variable_flow.pl");
    harness.open(&uri, script)?;

    scenario.when("requesting declaration from a variable usage");
    let (usage_line, usage_character) = find_position(script, "$value + 1");
    let declaration = harness.request(
        "textDocument/declaration",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": usage_line, "character": usage_character }
        }),
    )?;

    scenario.then("the declaration resolves to the original lexical binding");
    let declaration_uri = first_location_uri(&declaration).unwrap_or_default();
    assert_eq!(declaration_uri, uri, "declaration should stay within the same file");

    let declaration_line = declaration
        .as_array()
        .and_then(|arr| arr.first())
        .or_else(|| declaration.as_object().map(|_| &declaration))
        .and_then(|location| location.pointer("/range/start/line"))
        .and_then(Value::as_u64)
        .ok_or("declaration should include a start line")?;
    assert_eq!(declaration_line, 3, "declaration should point to `my $value = 41;`");

    scenario.when("requesting document highlights for the same variable");
    let highlights = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": usage_line, "character": usage_character }
        }),
    )?;

    scenario.then("all reads and writes for the lexical variable are highlighted");
    let highlight_entries = highlight_items(&highlights);
    assert_eq!(
        highlight_entries.len(),
        4,
        "expected declaration, arithmetic use, print use, and assignment target highlights"
    );
    assert!(
        highlight_entries
            .iter()
            .all(|entry| entry.get("range").is_some() && entry.get("kind").is_some()),
        "document highlights should include range and kind for each match"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_refactoring_workflow_surfaces_symbols_and_actions() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Refactoring workflow surfaces symbols and actions");

    let legacy = r#"sub legacy_process {
    my ($items) = @_;
    my $total = 0;
    foreach my $item (@$items) {
        $total = $total + $item;
    }
    return $total;
}

my $answer = legacy_process([1, 2, 3]);
"#;

    scenario.given("a legacy script that needs modernization and refactoring support");
    let (mut harness, workspace) = setup_workspace(&[("legacy.pl", legacy)])?;
    let uri = workspace.uri("legacy.pl");
    harness.open(&uri, legacy)?;

    scenario.when("requesting document symbols for navigation");
    let symbols = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("symbols include the legacy function");
    let names = symbol_names(&symbols);
    assert!(
        names.iter().any(|name| name == "legacy_process"),
        "document symbols should include legacy_process; got {names:?}"
    );

    scenario.when("requesting code actions for the file");
    let line_count = legacy.lines().count() as u32;
    let actions = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": line_count, "character": 0 }
            },
            "context": { "diagnostics": [] }
        }),
    )?;

    scenario.then("action list includes practical refactoring or modernization fixes");
    let titles = code_action_titles(&actions);
    assert!(!titles.is_empty(), "expected at least one code action");
    assert!(
        titles.iter().any(|title| {
            let title = title.to_ascii_lowercase();
            title.contains("strict")
                || title.contains("warning")
                || title.contains("extract")
                || title.contains("import")
        }),
        "code actions should include modernization/refactor suggestions; got {titles:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_incremental_changes_refresh_cross_file_navigation() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Incremental changes refresh cross-file navigation");

    let module_v1 = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return 1;
}

1;
"#;

    let main_v1 = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_data();
"#;

    let module_v2 = r#"package Foo;
use strict;
use warnings;

sub process_records {
    return 1;
}

1;
"#;

    let main_v2 = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_records();
"#;

    scenario.given("a workspace with cross-file calls indexed by the server");
    let (mut harness, workspace) =
        setup_workspace(&[("lib/Foo.pm", module_v1), ("main.pl", main_v1)])?;
    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module_v1)?;
    harness.open(&main_uri, main_v1)?;
    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("updating both files with didChange to a new function name");
    harness.change_full(&module_uri, 2, module_v2)?;
    harness.change_full(&main_uri, 2, main_v2)?;
    harness.barrier();
    harness.wait_for_symbol("process_records", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.then("go-to-definition resolves the updated symbol across files");
    let (line, character) = find_position(main_v2, "process_records()");
    let definition = wait_for_definition_uri(
        &mut harness,
        &main_uri,
        line,
        character,
        &module_uri,
        Duration::from_secs(10),
    )?;
    let def_uri = first_location_uri(&definition).unwrap_or_default();
    assert_eq!(def_uri, module_uri, "definition should resolve to updated module symbol");

    scenario.when("searching workspace symbols for the updated function");
    let symbols = harness.request(
        "workspace/symbol",
        json!({
            "query": "process_records"
        }),
    )?;

    scenario.then("workspace symbols include the updated function name");
    let names = symbol_names(&symbols);
    assert!(
        names.iter().any(|name| name == "process_records" || name.ends_with("process_records")),
        "workspace symbols should include process_records; got {names:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_prepare_rename_then_rename_from_call_site() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Prepare rename then rename from call site");

    let module = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return 1;
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_data();
"#;

    scenario.given("a workspace where a function is called from another file");
    let (mut harness, workspace) = setup_workspace(&[("lib/Foo.pm", module), ("main.pl", main)])?;

    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module)?;
    harness.open(&main_uri, main)?;
    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    let (line, character) = find_position(main, "process_data()");

    scenario.when("checking prepareRename at the call site");
    let prepare = harness.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("prepareRename returns a valid range");
    assert!(has_lsp_range(&prepare), "prepareRename should return a range-compatible payload");

    scenario.when("renaming the symbol from the same call site");
    let edit = wait_for_rename_edit_uris(
        &mut harness,
        &main_uri,
        line,
        character,
        "process_records",
        &[&module_uri, &main_uri],
        Duration::from_secs(10),
    )?;

    scenario.then("rename returns edits affecting both declaration and usage files");
    let uris = workspace_edit_uris(&edit);
    assert!(uris.contains(&module_uri), "rename should edit declaration file");
    assert!(uris.contains(&main_uri), "rename should edit usage file");

    Ok(())
}

#[test]
#[serial]
fn bdd_document_highlights_distinguish_reads_from_writes() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Document highlights distinguish reads from writes");

    let code = r#"use strict;
use warnings;

my $count = 1;
$count += 2;
print $count;
"#;

    scenario.given("a Perl file with the same variable declared, mutated, and read");
    let (mut harness, workspace) = setup_workspace(&[("highlights.pl", code)])?;
    let uri = workspace.uri("highlights.pl");
    harness.open(&uri, code)?;

    scenario.when("requesting document highlights on the variable usage");
    let (line, character) = find_position(code, "$count += 2");
    let highlights = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 1 }
        }),
    )?;

    scenario
        .then("the server returns highlights covering declaration, write, and read occurrences");
    let kinds = highlight_kinds(&highlights);
    assert!(kinds.len() >= 3, "expected at least 3 highlights; got {highlights:?}");
    assert!(
        kinds.contains(&2),
        "highlights should include a read occurrence (kind=2); got {kinds:?}"
    );
    assert!(
        kinds.contains(&3),
        "highlights should include a write occurrence (kind=3); got {kinds:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_pull_diagnostics_supports_unchanged_report_cycle() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Pull diagnostics supports unchanged report cycle");

    let code = r#"use strict;
use warnings;

sub healthy_sub {
    return 1;
}
"#;

    scenario.given("a Perl file that already has stable diagnostics");
    let (mut harness, workspace) = setup_workspace(&[("stable.pl", code)])?;
    let uri = workspace.uri("stable.pl");
    harness.open(&uri, code)?;

    scenario.when("requesting pull diagnostics for the first time");
    let first = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("the server returns a full diagnostic report with resultId");
    assert_eq!(first.get("kind").and_then(Value::as_str), Some("full"));
    let result_id = first
        .get("resultId")
        .and_then(Value::as_str)
        .ok_or("first diagnostic report missing resultId")?
        .to_string();

    scenario.when("requesting diagnostics again with previousResultId");
    let second = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "previousResultId": result_id
        }),
    )?;

    scenario.then("the server replies with an unchanged report");
    assert_eq!(second.get("kind").and_then(Value::as_str), Some("unchanged"));
    assert_eq!(
        second.get("resultId").and_then(Value::as_str),
        Some(result_id.as_str()),
        "unchanged report should keep the same resultId"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_pull_diagnostics_emits_new_result_after_file_change()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Pull diagnostics emit new result after file change");

    let healthy = r#"use strict;
use warnings;

sub score {
    return 1;
}
"#;

    let broken = r#"use strict;
use warnings;

sub score {
    if (1 {
        return 1;
    }
}
"#;

    scenario.given("a Perl file with a stable diagnostic resultId");
    let (mut harness, workspace) = setup_workspace(&[("cycle.pl", healthy)])?;
    let uri = workspace.uri("cycle.pl");
    harness.open(&uri, healthy)?;

    scenario.when("requesting pull diagnostics to establish a baseline resultId");
    let first = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    let baseline_result_id = first
        .get("resultId")
        .and_then(Value::as_str)
        .ok_or("first diagnostic report missing resultId")?
        .to_string();

    scenario.when("requesting diagnostics again with previousResultId without edits");
    let unchanged = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "previousResultId": baseline_result_id.clone()
        }),
    )?;

    scenario.then("the server reports unchanged diagnostics");
    assert_eq!(unchanged.get("kind").and_then(Value::as_str), Some("unchanged"));

    scenario.when("introducing a syntax error via didChange");
    harness.change_full(&uri, 2, broken)?;
    harness.barrier();

    let changed = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "previousResultId": baseline_result_id.clone()
        }),
    )?;

    scenario.then("the server emits a full report with a fresh resultId and parse errors");
    assert_eq!(changed.get("kind").and_then(Value::as_str), Some("full"));

    let changed_result_id = changed
        .get("resultId")
        .and_then(Value::as_str)
        .ok_or("changed diagnostic report missing resultId")?;

    assert_ne!(
        changed_result_id, baseline_result_id,
        "changed diagnostics should provide a new resultId"
    );
    assert!(
        diagnostic_error_count(&changed) > 0,
        "syntax regression should produce error diagnostics"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_variable_navigation_and_highlights_stay_in_sync() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Variable navigation and highlights stay in sync");

    let code = r#"use strict;
use warnings;

my $name = "Perl";
my $message = "Hello, $name";
$name =~ s/Perl/BDD/;
print $name;
"#;

    scenario.given("a Perl document with one lexical variable used in reads and writes");
    let (mut harness, workspace) = setup_workspace(&[("highlights.pl", code)])?;
    let uri = workspace.uri("highlights.pl");
    harness.open(&uri, code)?;

    scenario.when("requesting declaration from a later variable use");
    let (decl_line, decl_character) = find_position(code, "$name;");
    let declaration = harness.request(
        "textDocument/declaration",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": decl_line, "character": decl_character + 1 }
        }),
    )?;

    scenario.then("declaration points back to the original lexical binding");
    assert_eq!(first_location_uri(&declaration), Some(uri.clone()));
    assert_eq!(
        location_start_line(&declaration),
        Some(3),
        "declaration should resolve to 'my $name' on line 3"
    );

    scenario.when("requesting document highlights on that same variable");
    let highlights = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": decl_line, "character": decl_character + 1 }
        }),
    )?;

    scenario.then("highlights include declaration, mutation, and read occurrences");
    let highlight_items = highlights.as_array().cloned().unwrap_or_default();
    assert!(
        highlight_items.len() >= 4,
        "expected highlights for declaration and multiple uses; got {highlight_items:?}"
    );
    assert!(
        highlight_items.iter().all(has_lsp_range),
        "every highlight should include an LSP range; got {highlight_items:?}"
    );
    assert!(
        highlight_items.iter().any(|item| item.get("kind").and_then(Value::as_u64) == Some(3)),
        "expected at least one write highlight; got {highlight_items:?}"
    );
    assert!(
        highlight_items.iter().any(|item| item.get("kind").and_then(Value::as_u64) == Some(2)),
        "expected at least one read highlight; got {highlight_items:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_formatting_workflow_returns_structured_edits() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Formatting workflow returns structured edits");

    let unformatted = r#"sub messy_code{
my$x=10;
if($x>5){print"big"}
return$x*2}
"#;

    scenario.given("an unformatted Perl file in the workspace");
    let (mut harness, workspace) = setup_workspace(&[("format.pl", unformatted)])?;
    let uri = workspace.uri("format.pl");
    harness.open(&uri, unformatted)?;
    let formatting_timeout = if cfg!(windows)
        || std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
    {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(5)
    };

    scenario.when("requesting document formatting");
    let formatting_response = harness.request_raw_with_timeout(
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/formatting",
            "params": {
                "textDocument": { "uri": uri },
                "options": { "tabSize": 4, "insertSpaces": true }
            }
        }),
        formatting_timeout,
    );

    scenario.then("the response is structured edits or a graceful tooling error");
    if let Some(result) = formatting_response.get("result") {
        assert!(
            result.is_null() || result.is_array(),
            "formatting should return null or text edit array"
        );

        if let Some(edits) = result.as_array()
            && let Some(first_edit) = edits.first()
        {
            assert!(has_lsp_range(first_edit), "text edits should include an LSP range structure");
            assert!(
                first_edit.get("newText").and_then(Value::as_str).is_some(),
                "text edits should include newText"
            );
        }
    } else if perl_lsp::execute_command::command_exists("perltidy") {
        // perltidy IS installed but still returned an error — this is a real failure.
        // The server must surface a structured error with data.error_kind so that LSP
        // clients can present targeted remediation (e.g. "check Perl syntax").
        let error = formatting_response
            .get("error")
            .ok_or("formatting response should include either result or error")?;
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .ok_or("formatting error should include a message")?;
        assert!(!message.is_empty(), "formatting error message should not be empty");

        let error_kind =
            error.get("data").and_then(|d| d.get("error_kind")).and_then(Value::as_str).ok_or(
                "formatting error should carry a structured data.error_kind field \
                 (expected one of: perltidy_not_found, perltidy_error, io_error)",
            )?;
        assert!(
            matches!(error_kind, "perltidy_not_found" | "perltidy_error" | "io_error"),
            "data.error_kind should be a known tooling-error kind, got: {error_kind:?}"
        );
    } else {
        // perltidy is NOT installed on this machine.  The integration-test harness
        // cannot reliably exercise the perltidy-not-found error path: workspace-scan
        // latency frequently causes the formatting response to arrive after the test
        // timeout, yielding a synthetic harness error that lacks data.error_kind.
        //
        // The structured-error contract (data.error_kind = "perltidy_not_found") is
        // covered at the unit level in perl-lsp-formatting / perl-lsp
        // (see formatting_error_to_rpc and its tests).
        eprintln!(
            "[skip] perltidy not installed — structured-error shape is verified by unit tests"
        );
    }

    Ok(())
}

#[test]
#[serial]
fn bdd_navigation_workflow_expands_selection_and_highlights_symbol_usage()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario =
        BddScenario::new("Navigation workflow expands selection and highlights symbol usage");

    let code = r#"use strict;
use warnings;

sub calculate_total {
    my ($left, $right) = @_;
    my $total = $left + $right;
    $total += 1;
    return $total;
}

my $value = calculate_total(1, 2);
print $value;
"#;

    scenario.given("a Perl file with a local variable used in assignment, mutation, and return");
    let (mut harness, workspace) = setup_workspace(&[("navigation.pl", code)])?;
    let uri = workspace.uri("navigation.pl");
    harness.open(&uri, code)?;
    harness.wait_for_symbol("calculate_total", Some(&uri), Duration::from_secs(2)).ok();

    scenario.when("requesting document highlights on the local variable inside the subroutine");
    let (highlight_line, highlight_col) = find_position(code, "$total =");
    let highlights = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": highlight_line, "character": highlight_col }
        }),
    )?;

    scenario.then("the server highlights the declaration, mutation, and return usage sites");
    let highlight_items =
        highlights.as_array().ok_or("documentHighlight should return an array")?;
    assert_eq!(highlight_items.len(), 3, "expected three highlights for $total");
    assert!(
        highlight_items.iter().all(has_lsp_range),
        "all highlights should include valid ranges"
    );

    scenario.when("requesting selection ranges from the function call arguments");
    let (selection_line, selection_col) = find_position(code, "1, 2");
    let selection_ranges = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [{
                "line": selection_line,
                "character": selection_col + 1
            }]
        }),
    )?;

    scenario.then("the server returns a nested selection hierarchy for editor expand-selection");
    let ranges = selection_ranges.as_array().ok_or("selectionRange should return an array")?;
    assert_eq!(ranges.len(), 1, "expected one selection range result");
    let depth = selection_range_depth(&ranges[0]);
    assert!(depth >= 2, "selection range should provide nested expansion, got depth {depth}");
    assert!(has_lsp_range(&ranges[0]), "selection range should include a valid range");

    Ok(())
}
#[test]
fn bdd_goto_definition_with_multiple_declarations() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Go-to-definition with multiple declarations in scope");
    scenario.given("a script with multiple variables of the same name in different scopes");

    let script = r#"use strict;
use warnings;

my $x = 10;
{
    my $x = 20;
    print $x;
}
print $x;
"#;

    let (mut harness, workspace) = setup_workspace(&[("script.pl", script)])?;
    let uri = workspace.uri("script.pl");
    harness.open(&uri, script)?;
    harness.wait_for_symbol("x", Some(&uri), std::time::Duration::from_secs(5)).ok();
    harness.barrier();

    scenario.when("requesting definition on the inner variable usage");
    let (line_inner, col_inner) = find_position(script, "print $x;");

    // wait for definition to actually resolve
    let response_inner = wait_for_definition_uri(
        &mut harness,
        &uri,
        line_inner,
        col_inner + 6, // +6 for "print "
        &uri,
        std::time::Duration::from_secs(5),
    )?;

    scenario.then("the definition should resolve to the inner declaration");
    let empty_vec = vec![];
    let locations = response_inner.as_array().unwrap_or(&empty_vec);
    assert_eq!(locations.len(), 1);
    let inner_def_line = location_start_line(&locations[0]).unwrap();
    assert_eq!(inner_def_line, 5, "Expected inner $x declaration at line 5 (0-indexed)");

    scenario.when("requesting definition on the outer variable usage");
    // find the *last* instance of "print $x;"
    let last_print_idx = script.rfind("print $x;").unwrap();
    let prefix = &script[..last_print_idx];
    let line_outer = prefix.chars().filter(|&c| c == '\n').count() as u32;
    let col_outer = prefix.chars().rev().take_while(|&c| c != '\n').count() as u32 + 6; // offset for "print "

    let response_outer = wait_for_definition_uri(
        &mut harness,
        &uri,
        line_outer,
        col_outer,
        &uri,
        std::time::Duration::from_secs(5),
    )?;

    scenario.then("the definition should resolve to the outer declaration");
    let empty_vec_outer = vec![];
    let locations_outer = response_outer.as_array().unwrap_or(&empty_vec_outer);
    assert_eq!(locations_outer.len(), 1);
    let outer_def_line = location_start_line(&locations_outer[0]).unwrap();
    assert_eq!(outer_def_line, 3, "Expected outer $x declaration at line 3 (0-indexed)");

    Ok(())
}

#[test]
fn bdd_hover_displays_module_documentation() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Hover displays module links");
    scenario.given("a workspace with a module and a script that uses it");

    let module = r#"package Foo;
use strict;
use warnings;

=head1 NAME

Foo - A module for fooing

=cut

sub do_foo {
    return 1;
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;

Foo::do_foo();
"#;

    let (mut harness, workspace) = setup_workspace(&[("lib/Foo.pm", module), ("main.pl", main)])?;
    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module)?;
    harness.open(&main_uri, main)?;
    harness.wait_for_symbol("do_foo", Some(&module_uri), std::time::Duration::from_secs(5)).ok();
    harness.barrier();

    scenario.when("requesting hover on the module name");
    let (line, col) = find_position(main, "use Foo;");

    let response = harness
        .request_with_timeout(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": line, "character": col + 4 } // offset for "use "
            }),
            std::time::Duration::from_millis(1000),
        )
        .unwrap_or(serde_json::Value::Null);

    scenario.then("the hover response should contain the module links and MetaCPAN reference");
    assert!(!response.is_null(), "Hover response should not be null");

    let hover_text = hover_text(&response);
    assert!(
        hover_text.contains("**Foo**"),
        "Hover should contain module name, got: {}",
        hover_text
    );
    assert!(
        hover_text.contains("View on MetaCPAN"),
        "Hover should contain MetaCPAN link, got: {}",
        hover_text
    );

    Ok(())
}

#[test]
fn bdd_document_symbols_handles_nested_packages() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Document symbols handles nested packages");
    scenario.given("a script with multiple nested package declarations");

    let script = r#"package Outer;

sub outer_func {}

package Outer::Inner;

sub inner_func {}

package main;

sub main_func {}
"#;

    let (mut harness, workspace) = setup_workspace(&[("script.pl", script)])?;
    let uri = workspace.uri("script.pl");
    harness.open(&uri, script)?;
    harness.barrier();

    scenario.when("requesting document symbols");
    let response = harness
        .request_with_timeout(
            "textDocument/documentSymbol",
            json!({
                "textDocument": { "uri": uri }
            }),
            std::time::Duration::from_millis(1000),
        )
        .unwrap_or(serde_json::Value::Null);

    scenario.then("the document symbols should include all packages and their subroutines");
    let empty_vec = vec![];
    let symbols = response.as_array().unwrap_or(&empty_vec);

    // Helper to search recursively
    fn find_symbol(nodes: &[serde_json::Value], name: &str) -> bool {
        for node in nodes {
            if node["name"].as_str().unwrap_or_default() == name {
                return true;
            }
            if let Some(children) = node["children"].as_array() {
                if find_symbol(children, name) {
                    return true;
                }
            }
        }
        false
    }

    assert!(find_symbol(symbols, "Outer"), "Expected Outer package");
    assert!(find_symbol(symbols, "outer_func"), "Expected outer_func");
    assert!(find_symbol(symbols, "Outer::Inner"), "Expected Outer::Inner package");
    assert!(find_symbol(symbols, "inner_func"), "Expected inner_func");
    assert!(find_symbol(symbols, "main"), "Expected main package");
    assert!(find_symbol(symbols, "main_func"), "Expected main_func");

    Ok(())
}
#[test]
#[serial]
fn bdd_references_respects_include_declaration_flag() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("References honor includeDeclaration behavior");

    let module = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return 1;
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;

my $result = Foo::process_data();
my $again = Foo::process_data();
"#;

    scenario.given("a workspace where a symbol has one declaration and multiple call sites");
    let (mut harness, workspace) = setup_workspace(&[("lib/Foo.pm", module), ("main.pl", main)])?;
    let module_uri = workspace.uri("lib/Foo.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, module)?;
    harness.open(&main_uri, main)?;
    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(10))?;
    harness.barrier();

    let (line, character) = find_position(module, "process_data");

    scenario.when("requesting references with includeDeclaration=false");
    let without_decl = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": module_uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": false }
        }),
    )?;

    scenario.then("usage locations are still returned when declarations are excluded");
    let without_decl_locations = without_decl
        .as_array()
        .ok_or("references response should be an array for includeDeclaration=false")?;
    let without_decl_uris = ref_uris(&without_decl);
    assert!(
        uri_set_contains(&without_decl_uris, &main_uri),
        "usage file should still be included when includeDeclaration=false; got {without_decl_uris:?}"
    );
    assert!(
        without_decl_locations.len() >= 2,
        "expected at least the two call-site references in main.pl; got {without_decl_locations:?}"
    );

    scenario.when("requesting references with includeDeclaration=true");
    let with_decl = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": module_uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        }),
    )?;

    scenario.then("includeDeclaration=true returns at least as many locations as usage-only mode");
    let with_decl_locations = with_decl
        .as_array()
        .ok_or("references response should be an array for includeDeclaration=true")?;
    let with_decl_uris = ref_uris(&with_decl);
    assert!(
        uri_set_contains(&with_decl_uris, &main_uri),
        "usage file should remain present when includeDeclaration=true; got {with_decl_uris:?}"
    );
    assert!(
        with_decl_uris.len() >= without_decl_uris.len(),
        "includeDeclaration=true should not return fewer URI buckets than includeDeclaration=false; without={without_decl_uris:?} with={with_decl_uris:?}"
    );
    assert!(
        with_decl_locations.len() >= without_decl_locations.len(),
        "includeDeclaration=true should not return fewer locations than includeDeclaration=false; without={} with={}",
        without_decl_locations.len(),
        with_decl_locations.len()
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_incremental_completion_reflects_new_local_symbol() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Incremental completion reflects new local symbols");

    let before = r#"use strict;
use warnings;

my $value = cal
"#;

    let after = r#"use strict;
use warnings;

sub calculate_total {
    my ($left, $right) = @_;
    return $left + $right;
}

my $value = cal
"#;

    scenario.given("a file where completion initially has no local function declaration");
    let (mut harness, workspace) = setup_workspace(&[("incremental_completion.pl", before)])?;
    let uri = workspace.uri("incremental_completion.pl");
    harness.open(&uri, before)?;
    harness.barrier();

    let (line, col) = find_position(before, "my $value = cal");
    let completion_character = col + "my $value = cal".len() as u32;

    scenario.when("requesting completion before introducing the helper function");
    let before_completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": completion_character }
        }),
    )?;

    let before_labels = completion_labels(&before_completion);

    scenario.when("adding a helper function via didChange and requesting completion again");
    harness.change_full(&uri, 2, after)?;
    harness.barrier();
    harness.wait_for_symbol("calculate_total", Some(&uri), Duration::from_secs(10))?;
    harness.barrier();

    let (line_after, col_after) = find_position(after, "my $value = cal");
    let after_completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": {
                "line": line_after,
                "character": col_after + "my $value = cal".len() as u32
            }
        }),
    )?;

    scenario.then("the refreshed completion list now includes the newly declared function");
    let after_labels = completion_labels(&after_completion);
    assert!(
        !before_labels.contains("calculate_total"),
        "baseline completion should not already include calculate_total; got {before_labels:?}"
    );
    assert!(
        after_labels
            .iter()
            .any(|label| label == "calculate_total" || label.ends_with("calculate_total")),
        "completion after didChange should include calculate_total; got {after_labels:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_prepare_rename_rejects_non_symbol_positions() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Prepare rename rejects non-symbol positions");

    let code = r#"use strict;
use warnings;

my $value = 41;
print $value + 1;
"#;

    scenario.given("a file where rename is attempted over punctuation instead of an identifier");
    let (mut harness, workspace) = setup_workspace(&[("rename_invalid.pl", code)])?;
    let uri = workspace.uri("rename_invalid.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    let (line, plus_col) = find_position(code, "+ 1");

    scenario.when("requesting prepareRename on the '+' operator");
    let prepare = harness.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": plus_col }
        }),
    )?;

    scenario.then("the server declines rename at that position");
    assert!(
        prepare.is_null() || !has_lsp_range(&prepare),
        "prepareRename at operator positions should not return a symbol range; got {prepare:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_prepare_rename_returns_range_for_keyword_token() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Prepare rename returns range for keyword token");

    let code = r#"use strict;
use warnings;

my $value = 1;
print $value;
"#;

    scenario.given("a Perl document and a cursor positioned on a keyword token");
    let (mut harness, workspace) = setup_workspace(&[("rename_guard.pl", code)])?;
    let uri = workspace.uri("rename_guard.pl");
    harness.open(&uri, code)?;

    let (line, character) = find_position(code, "print $value;");

    scenario.when("requesting prepareRename on the `print` keyword token");
    let response = harness.request_raw_with_timeout(
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/prepareRename",
            "params": {
                "textDocument": { "uri": uri },
                "position": {
                    "line": line,
                    "character": character
                }
            }
        }),
        Duration::from_secs(2),
    );

    scenario.then("the server returns a valid range payload rather than crashing");
    assert!(
        response.get("error").is_none(),
        "prepareRename should not hard-fail; got {response:?}"
    );
    assert!(
        response.get("result").is_some_and(has_lsp_range),
        "prepareRename should return a range-compatible result; got {response:?}"
    );
    assert_eq!(
        response.pointer("/result/placeholder").and_then(Value::as_str),
        Some("print"),
        "prepareRename should surface the touched token as placeholder"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_references_toggle_include_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("References remain stable across includeDeclaration toggle");

    let code = r#"use strict;
use warnings;

my $total = 1;
print $total;
$total += 2;
"#;

    scenario.given("a file with one lexical declaration and two usages");
    let (mut harness, workspace) = setup_workspace(&[("references.pl", code)])?;
    let uri = workspace.uri("references.pl");
    harness.open(&uri, code)?;

    let (line, character) = find_position(code, "$total += 2");

    scenario.when("requesting references with includeDeclaration=true");
    let with_declaration = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 1 },
            "context": { "includeDeclaration": true }
        }),
    )?;

    scenario.then("the response includes declaration and usages");
    let with_decl_items = with_declaration.as_array().cloned().unwrap_or_default();
    assert!(
        with_decl_items.len() >= 3,
        "expected declaration + 2 usages when includeDeclaration=true; got {with_decl_items:?}"
    );

    scenario.when("requesting references with includeDeclaration=false");
    let without_declaration = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 1 },
            "context": { "includeDeclaration": false }
        }),
    )?;

    scenario.then("the response stays structurally valid and returns reference locations");
    let without_decl_items = without_declaration.as_array().cloned().unwrap_or_default();
    assert!(
        !without_decl_items.is_empty(),
        "reference lookup with includeDeclaration=false should still return locations"
    );
    assert!(
        without_decl_items.len() >= with_decl_items.len().saturating_sub(1),
        "includeDeclaration=false should not catastrophically reduce references (with={}, without={})",
        with_decl_items.len(),
        without_decl_items.len()
    );
    assert!(
        without_decl_items.iter().all(|item| item.get("uri").is_some() && has_lsp_range(item)),
        "reference entries should preserve uri + range fields; got {without_decl_items:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_structural_navigation_supports_folding_and_inlay_hints()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Structural navigation supports folding and inlay hints");

    let code = r#"use strict;
use warnings;

sub render {
    my ($name) = @_;
    if ($name) {
        return substr($name, 0, 3);
    }
    return "n/a";
}
"#;

    scenario.given("a Perl document with nested blocks and a builtin call that takes arguments");
    let (mut harness, workspace) = setup_workspace(&[("structure.pl", code)])?;
    let uri = workspace.uri("structure.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting folding ranges for the document");
    let folding = harness.request(
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("the server returns foldable structural ranges");
    let folding_ranges = folding.as_array().ok_or("foldingRange should return an array payload")?;
    assert!(!folding_ranges.is_empty(), "expected at least one folding range");
    assert!(
        folding_ranges
            .iter()
            .all(|range| range.get("startLine").is_some() && range.get("endLine").is_some()),
        "all folding ranges should expose startLine and endLine"
    );

    scenario.when("requesting inlay hints for the same range");
    let inlay = harness.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": code.lines().count() as u32, "character": 0 }
            }
        }),
    )?;

    scenario.then("inlay hints return a valid payload shape for the requested range");
    let hints = inlay.as_array().ok_or("inlayHint should return an array payload")?;
    assert!(
        hints.iter().all(|hint| hint.get("position").is_some() && hint.get("label").is_some()),
        "every inlay hint should include position and label when present"
    );

    let labels = inlay_labels(&inlay);
    if !labels.is_empty() {
        assert!(
            labels.iter().any(|label| matches!(label.as_str(), "expr:" | "offset:" | "length:")),
            "expected substr-style parameter hints in {labels:?}"
        );
    }

    Ok(())
}

#[test]
#[serial]
fn bdd_selection_ranges_expand_progressively() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Selection ranges expand progressively");

    let code = r#"use strict;
use warnings;

sub compute_total {
    my ($a, $b) = @_;
    my $sum = $a + $b;
    return $sum;
}
"#;

    scenario.given("a Perl file with a function body and nested expressions");
    let (mut harness, workspace) = setup_workspace(&[("selection.pl", code)])?;
    let uri = workspace.uri("selection.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    let (line, character) = find_position(code, "$sum");

    scenario.when("requesting selection ranges on a symbol inside the function body");
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [{ "line": line, "character": character + 1 }]
        }),
    )?;

    scenario.then("the server returns nested parent ranges to allow expansion");
    let ranges = response.as_array().ok_or("selectionRange response should be an array")?;
    let first = ranges.first().ok_or("selectionRange response should contain one item")?;
    let depth = selection_range_depth(first);
    assert!(
        depth >= 2,
        "selection range should provide at least one parent expansion; got depth {depth}"
    );
    let child_span = line_span(first).ok_or("selection range should include child line span")?;
    let parent = first.get("parent").ok_or("selection range should include parent")?;
    let parent_span = line_span(parent).ok_or("selection range parent should include line span")?;
    assert!(
        parent_span.0 <= child_span.0 && parent_span.1 >= child_span.1,
        "parent range should enclose child range (child={child_span:?}, parent={parent_span:?})"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_workspace_symbol_query_matches_package_and_subroutine()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Workspace symbol query matches package and subroutine");
    scenario.given("a workspace with package and subroutine symbols");

    let module = r#"package SymbolHub;
use strict;
use warnings;

sub collect_metrics {
    return 1;
}

1;
"#;

    let (mut harness, workspace) = setup_workspace(&[("lib/SymbolHub.pm", module)])?;
    let module_uri = workspace.uri("lib/SymbolHub.pm");
    harness.open(&module_uri, module)?;
    harness.wait_for_symbol("collect_metrics", Some(&module_uri), Duration::from_secs(10)).ok();
    harness.barrier();

    scenario.when("searching workspace symbols using a package-oriented query");
    let result = harness.request(
        "workspace/symbol",
        json!({
            "query": "SymbolHub"
        }),
    )?;

    scenario.then("the symbol list includes both package and subroutine entries");
    let items = result.as_array().cloned().unwrap_or_default();
    assert!(!items.is_empty(), "workspace/symbol should return entries for SymbolHub query");

    let names: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect();

    assert!(
        names.iter().any(|name| name == "SymbolHub"),
        "workspace symbols should include package name SymbolHub; got {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "collect_metrics" || name.ends_with("collect_metrics")),
        "workspace symbols should include collect_metrics; got {names:?}"
    );

    Ok(())
}
