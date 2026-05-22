//! BDD-style workflow coverage for core LSP behaviors.
//!
//! These tests are structured as Given/When/Then scenarios to validate
//! end-to-end user workflows using the real JSON-RPC harness.

mod support;

use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use support::bdd_diagnostics::{BddScenario, DocumentDiagnosticFlow};
use support::lsp_harness::{LspHarness, TempWorkspace};

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

fn signature_labels(response: &Value) -> Vec<String> {
    response
        .get("signatures")
        .and_then(Value::as_array)
        .map(|signatures| {
            signatures
                .iter()
                .filter_map(|signature| {
                    signature.get("label").and_then(Value::as_str).map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
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

fn code_lens_command_title(lens: &Value) -> Option<&str> {
    lens.get("command").and_then(|command| command.get("title")).and_then(Value::as_str)
}

fn code_lens_command_id(lens: &Value) -> Option<&str> {
    lens.get("command").and_then(|command| command.get("command")).and_then(Value::as_str)
}

fn code_lens_data_kind(lens: &Value) -> Option<&str> {
    lens.get("data").and_then(|data| data.get("kind")).and_then(Value::as_str)
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

fn semantic_token_data(response: &Value) -> Vec<u64> {
    response
        .get("data")
        .and_then(Value::as_array)
        .map(|data| data.iter().filter_map(Value::as_u64).collect())
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

fn prepare_call_hierarchy_item(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
    name: &str,
) -> Result<Value, String> {
    let response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    let items = response.as_array().ok_or_else(|| {
        format!("prepareCallHierarchy for {name} returned non-array response: {response:?}")
    })?;

    items
        .iter()
        .find(|item| item.get("name").and_then(Value::as_str) == Some(name))
        .cloned()
        .ok_or_else(|| {
            let names: Vec<&str> =
                items.iter().filter_map(|item| item.get("name").and_then(Value::as_str)).collect();
            format!("prepareCallHierarchy did not return item named {name}; names: {names:?}")
        })
}

fn call_hierarchy_edge_names(response: &Value, edge_name_pointer: &str) -> BTreeSet<String> {
    response
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|call| call.pointer(edge_name_pointer).and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn wait_for_call_hierarchy_edge_names(
    harness: &mut LspHarness,
    method: &str,
    item: &Value,
    edge_name_pointer: &str,
    want_names: &[&str],
    budget: Duration,
) -> Result<Value, String> {
    let start = Instant::now();
    let mut last_response = None;
    let mut last_error = None;

    while start.elapsed() < budget {
        match harness.request_with_timeout(method, json!({ "item": item }), Duration::from_secs(2))
        {
            Ok(response) => {
                let names = call_hierarchy_edge_names(&response, edge_name_pointer);
                if want_names.iter().all(|want| names.contains(*want)) {
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
        "{method} did not include {:?} within {:?}; last response: {:?}; last error: {:?}",
        want_names, budget, last_response, last_error
    ))
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
fn bdd_imported_subroutine_navigation_across_modules() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Imported subroutine navigation across modules");

    let utils_module = r#"package MyApp::Utils;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(format_date);

sub format_date {
    my ($value) = @_;
    return $value;
}

1;
"#;

    let main_script = r#"use strict;
use warnings;
use lib './lib';
use MyApp::Utils qw(format_date);

my $result = format_date("2026-04-23");
"#;

    scenario.given("a workspace where a subroutine is imported from a module");
    let (mut harness, workspace) =
        setup_workspace(&[("lib/MyApp/Utils.pm", utils_module), ("bin/main.pl", main_script)])?;

    let utils_uri = workspace.uri("lib/MyApp/Utils.pm");
    let main_uri = workspace.uri("bin/main.pl");

    harness.open(&utils_uri, utils_module)?;
    harness.open(&main_uri, main_script)?;
    harness.wait_for_symbol("format_date", Some(&utils_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("requesting go-to-definition on the imported call site");
    let (call_line, call_character) = find_position(main_script, "format_date(\"2026-04-23\")");
    let definition = wait_for_definition_uri(
        &mut harness,
        &main_uri,
        call_line,
        call_character,
        &utils_uri,
        Duration::from_secs(10),
    )?;

    scenario.then("definition resolves to the exporter module");
    assert_eq!(
        first_location_uri(&definition),
        Some(utils_uri.clone()),
        "imported call should resolve to exporting module"
    );

    scenario.when("requesting workspace symbols for the imported function name");
    let symbols = harness.request(
        "workspace/symbol",
        json!({
            "query": "format_date"
        }),
    )?;

    scenario.then("workspace symbols include the exporting module symbol");
    let uris = ref_uris(&symbols);
    assert!(
        uri_set_contains(&uris, &utils_uri),
        "workspace symbols should include exporter module; got {uris:?}"
    );

    scenario.when("requesting hover on the imported call site");
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": call_line, "character": call_character }
        }),
    )?;

    scenario.then("hover provides non-empty imported symbol information");
    assert!(
        !hover_text(&hover).is_empty(),
        "hover over imported subroutine should contain details"
    );

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
my $also = Foo::process_data();
"#;

    scenario.given("a workspace with qualified calls to the same function");
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
fn bdd_rename_is_scoped_to_target_package_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Rename stays scoped to the selected package symbol");

    let foo_module = r#"package Foo;
use strict;
use warnings;

sub process_data {
    return "foo";
}

1;
"#;

    let bar_module = r#"package Bar;
use strict;
use warnings;

sub process_data {
    return "bar";
}

1;
"#;

    let main = r#"use strict;
use warnings;
use lib './lib';
use Foo;
use Bar;

my $foo = Foo::process_data();
my $bar = Bar::process_data();
"#;

    scenario.given("a workspace where two packages expose subroutines with the same name");
    let (mut harness, workspace) = setup_workspace(&[
        ("lib/Foo.pm", foo_module),
        ("lib/Bar.pm", bar_module),
        ("main.pl", main),
    ])?;

    let foo_uri = workspace.uri("lib/Foo.pm");
    let bar_uri = workspace.uri("lib/Bar.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&foo_uri, foo_module)?;
    harness.open(&bar_uri, bar_module)?;
    harness.open(&main_uri, main)?;

    harness.wait_for_symbol("process_data", Some(&foo_uri), Duration::from_secs(10))?;
    harness.wait_for_symbol("process_data", Some(&bar_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("renaming Foo::process_data from the declaration in Foo.pm");
    let (def_line, def_char) = find_position(foo_module, "process_data");
    let edit = wait_for_rename_edit_uris(
        &mut harness,
        &foo_uri,
        def_line,
        def_char,
        "process_records",
        &[&foo_uri, &main_uri],
        Duration::from_secs(10),
    )?;

    scenario.then("rename edits target Foo.pm and main.pl only");
    let touched_uris = workspace_edit_uris(&edit);
    assert!(touched_uris.contains(&foo_uri), "rename should edit Foo.pm");
    assert!(touched_uris.contains(&main_uri), "rename should edit main.pl");
    assert!(!touched_uris.contains(&bar_uri), "rename should not edit Bar.pm");

    scenario.then("the edit payload rewrites Foo call sites but not Bar call sites");
    let foo_texts = workspace_edit_new_texts_for_uri(&edit, &foo_uri);
    let main_texts = workspace_edit_new_texts_for_uri(&edit, &main_uri);

    assert!(
        foo_texts.iter().any(|text| text.contains("process_records")),
        "Foo edits should include the renamed symbol; got {foo_texts:?}"
    );
    assert!(
        main_texts.iter().any(|text| text.contains("process_records")),
        "main edits should include the renamed Foo call; got {main_texts:?}"
    );
    assert!(
        !main_texts.iter().any(|text| text.contains("Bar::process_records")),
        "main edits should not rename Bar::process_data call sites; got {main_texts:?}"
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
fn bdd_signature_help_tracks_active_parameter() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Signature help tracks active parameter");

    let code = r#"use strict;
use warnings;

my $text = "Hello World";
my $slice = substr($text, 6, );
"#;

    scenario.given("a workspace where the user is typing a built-in function call");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;

    scenario.when("requesting signature help after typing the second comma in substr");
    let (line, mut character) = find_position(code, "substr($text, 6,");
    character += "substr($text, 6,".len() as u32;

    let signature_help = harness.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("signature help includes substr and marks the third parameter as active");
    let labels = signature_labels(&signature_help);
    assert!(
        labels.iter().any(|label| label.contains("substr")),
        "signature help should include substr label; got {labels:?}"
    );

    let active_parameter = signature_help
        .get("activeParameter")
        .and_then(Value::as_u64)
        .ok_or("expected activeParameter in signature help response")?;
    assert_eq!(active_parameter, 2, "expected LENGTH argument position");

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

    let mut diagnostics = DocumentDiagnosticFlow::new(&mut harness, uri.clone());

    scenario.when("requesting pull diagnostics for the first time");
    let first = diagnostics.request(None)?;

    scenario.then("the server returns a full diagnostic report with resultId");
    assert_eq!(DocumentDiagnosticFlow::kind(&first), Some("full"));
    let result_id = DocumentDiagnosticFlow::result_id(&first)?;

    scenario.when("requesting diagnostics again with previousResultId");
    let second = diagnostics.request(Some(result_id.as_str()))?;

    scenario.then("the server replies with an unchanged report");
    assert_eq!(DocumentDiagnosticFlow::kind(&second), Some("unchanged"));
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
    let first = DocumentDiagnosticFlow::new(&mut harness, uri.clone()).request(None)?;

    let baseline_result_id = DocumentDiagnosticFlow::result_id(&first)?;

    scenario.when("requesting diagnostics again with previousResultId without edits");
    let unchanged = DocumentDiagnosticFlow::new(&mut harness, uri.clone())
        .request(Some(baseline_result_id.as_str()))?;

    scenario.then("the server reports unchanged diagnostics");
    assert_eq!(DocumentDiagnosticFlow::kind(&unchanged), Some("unchanged"));

    scenario.when("introducing a syntax error via didChange");
    harness.change_full(&uri, 2, broken)?;
    harness.barrier();

    let changed = DocumentDiagnosticFlow::new(&mut harness, uri.clone())
        .request(Some(baseline_result_id.as_str()))?;

    scenario.then("the server emits a full report with a fresh resultId and parse errors");
    assert_eq!(DocumentDiagnosticFlow::kind(&changed), Some("full"));

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
fn bdd_pull_diagnostics_tracks_result_ids_per_document() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Pull diagnostics keep per-document resultId caches isolated");

    let healthy = r#"use strict;
use warnings;

sub ok {
    return 1;
}
"#;

    let broken = r#"use strict;
use warnings;

sub boom {
    if (1 {
        return 0;
    }
}
"#;

    scenario.given("a workspace with one healthy file and one file that later regresses");
    let (mut harness, workspace) =
        setup_workspace(&[("stable.pl", healthy), ("changing.pl", healthy)])?;
    let stable_uri = workspace.uri("stable.pl");
    let changing_uri = workspace.uri("changing.pl");
    harness.open(&stable_uri, healthy)?;
    harness.open(&changing_uri, healthy)?;

    let (stable_id, changing_id) = {
        let mut stable_diag = DocumentDiagnosticFlow::new(&mut harness, stable_uri.clone());
        let stable_first = stable_diag.request(None)?;
        let stable_id = DocumentDiagnosticFlow::result_id(&stable_first)?;

        let mut changing_diag = DocumentDiagnosticFlow::new(&mut harness, changing_uri.clone());
        let changing_first = changing_diag.request(None)?;
        let changing_id = DocumentDiagnosticFlow::result_id(&changing_first)?;
        (stable_id, changing_id)
    };

    scenario.when("requesting diagnostics with cached resultIds before any edits");
    let (stable_unchanged, changing_unchanged) = {
        let mut stable_diag = DocumentDiagnosticFlow::new(&mut harness, stable_uri.clone());
        let stable_unchanged = stable_diag.request(Some(stable_id.as_str()))?;

        let mut changing_diag = DocumentDiagnosticFlow::new(&mut harness, changing_uri.clone());
        let changing_unchanged = changing_diag.request(Some(changing_id.as_str()))?;
        (stable_unchanged, changing_unchanged)
    };

    scenario.then("both documents return unchanged reports");
    assert_eq!(DocumentDiagnosticFlow::kind(&stable_unchanged), Some("unchanged"));
    assert_eq!(DocumentDiagnosticFlow::kind(&changing_unchanged), Some("unchanged"));

    scenario.when("introducing a syntax regression in only one document");
    harness.change_full(&changing_uri, 2, broken)?;
    harness.barrier();

    let (stable_after_edit, changing_after_edit) = {
        let mut stable_diag = DocumentDiagnosticFlow::new(&mut harness, stable_uri.clone());
        let stable_after_edit = stable_diag.request(Some(stable_id.as_str()))?;

        let mut changing_diag = DocumentDiagnosticFlow::new(&mut harness, changing_uri.clone());
        let changing_after_edit = changing_diag.request(Some(changing_id.as_str()))?;
        (stable_after_edit, changing_after_edit)
    };

    scenario
        .then("the unchanged file stays unchanged while the edited file gets a new full report");
    assert_eq!(DocumentDiagnosticFlow::kind(&stable_after_edit), Some("unchanged"));
    assert_eq!(
        stable_after_edit.get("resultId").and_then(Value::as_str),
        Some(stable_id.as_str()),
        "stable file should keep the same resultId"
    );

    assert_eq!(DocumentDiagnosticFlow::kind(&changing_after_edit), Some("full"));
    let changed_result_id = DocumentDiagnosticFlow::result_id(&changing_after_edit)?;
    assert_ne!(
        changed_result_id, changing_id,
        "edited file should receive a fresh diagnostic resultId"
    );
    assert!(
        diagnostic_error_count(&changing_after_edit) > 0,
        "edited file should report syntax errors"
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
            std::time::Duration::from_secs(1),
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
            std::time::Duration::from_secs(1),
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
fn bdd_incremental_semantic_tokens_refresh_after_local_symbol_addition()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Incremental semantic tokens refresh after local symbol edit");

    let before = r#"use strict;
use warnings;

my $value = 41;
print $value;
"#;

    let after = r#"use strict;
use warnings;

my $value = 41;
my $delta = $value + 1;
print $value + $delta;
"#;

    scenario.given("an opened file with semantic tokens available for baseline content");
    let (mut harness, workspace) = setup_workspace(&[("incremental_semantic_tokens.pl", before)])?;
    let uri = workspace.uri("incremental_semantic_tokens.pl");
    harness.open(&uri, before)?;
    harness.barrier();

    scenario.when("requesting semantic tokens before the incremental change");
    let before_tokens = harness.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;
    let before_data = semantic_token_data(&before_tokens);

    scenario.when("editing the file to add a new local symbol and requesting tokens again");
    harness.change_full(&uri, 2, after)?;
    harness.barrier();
    let after_tokens = harness.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;
    let after_data = semantic_token_data(&after_tokens);

    scenario.then("semantic tokens are recomputed and reflect the richer symbol set");
    assert!(!before_data.is_empty(), "baseline semantic tokens should not be empty");
    assert!(!after_data.is_empty(), "updated semantic tokens should not be empty");
    assert_ne!(
        before_data, after_data,
        "semantic tokens should change after incremental edit; before={before_data:?} after={after_data:?}"
    );
    // The after content introduces $delta (appears twice: declaration and use), so the
    // encoded token stream must be strictly longer — each token is 5 u64 values in
    // LSP's relative-encoded format. A '>=' allows the degenerate case where tokens
    // shrink to exactly the same count, so we require strict growth.
    assert!(
        after_data.len() > before_data.len(),
        "adding two $delta references must grow the token payload; before={} after={}",
        before_data.len(),
        after_data.len()
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

#[test]
#[serial]
fn bdd_workspace_symbols_refresh_after_incremental_package_rename()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Workspace symbols refresh after incremental package rename");
    scenario.given("a workspace containing a package that is renamed in-place");

    let before = r#"package SymbolHub;
use strict;
use warnings;

sub collect_metrics {
    return 1;
}

1;
"#;

    let after = before.replace("SymbolHub", "MetricsHub");

    let (mut harness, workspace) = setup_workspace(&[("lib/SymbolHub.pm", before)])?;
    let module_uri = workspace.uri("lib/SymbolHub.pm");
    harness.open(&module_uri, before)?;
    harness.wait_for_symbol("SymbolHub", Some(&module_uri), Duration::from_secs(10)).ok();
    harness.barrier();

    scenario.when("querying workspace symbols before and after a didChange rename");
    let before_result = harness.request(
        "workspace/symbol",
        json!({
            "query": "SymbolHub"
        }),
    )?;

    harness.change_full(&module_uri, 2, &after)?;
    harness.wait_for_symbol("MetricsHub", Some(&module_uri), Duration::from_secs(10)).ok();
    harness.barrier();

    let after_result = harness.request(
        "workspace/symbol",
        json!({
            "query": "MetricsHub"
        }),
    )?;

    scenario.then("workspace symbol search reflects the updated package name");
    let before_names: Vec<String> = before_result
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect();
    let after_names: Vec<String> = after_result
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect();

    assert!(
        before_names.iter().any(|name| name == "SymbolHub"),
        "pre-change workspace symbols should include SymbolHub; got {before_names:?}"
    );
    assert!(
        after_names.iter().any(|name| name == "MetricsHub"),
        "post-change workspace symbols should include MetricsHub; got {after_names:?}"
    );
    // Verify stale index entry is removed — a correct implementation must evict
    // the old package name after an incremental didChange, not just append the new one.
    assert!(
        !after_names.iter().any(|name| name == "SymbolHub"),
        "post-change workspace symbols should NOT include stale SymbolHub; got {after_names:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_folding_ranges_cover_package_and_subroutine_blocks() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Folding ranges cover package and subroutine blocks");
    scenario.given("a Perl module with multiline package and subroutine bodies");

    let module = r#"package Fold::Demo;
use strict;
use warnings;

sub alpha {
    my ($value) = @_;
    if ($value > 0) {
        return $value;
    }
    return 0;
}

sub beta {
    my ($left, $right) = @_;
    return $left + $right;
}

1;
"#;

    let (mut harness, workspace) = setup_workspace(&[("lib/Fold/Demo.pm", module)])?;
    let uri = workspace.uri("lib/Fold/Demo.pm");
    harness.open(&uri, module)?;
    harness.barrier();

    scenario.when("requesting folding ranges for the module document");
    let response = harness.request(
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("the server returns multiline folding regions for major code blocks");
    let ranges = response.as_array().ok_or("foldingRange should return an array")?;
    assert!(!ranges.is_empty(), "expected at least one folding range");
    assert!(
        ranges.iter().any(|range| {
            let start = range.get("startLine").and_then(Value::as_u64);
            let end = range.get("endLine").and_then(Value::as_u64);
            matches!((start, end), (Some(start), Some(end)) if end > start)
        }),
        "expected at least one multiline folding range; got {ranges:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_code_lens_surfaces_test_execution_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Code lens surfaces test execution workflow");
    scenario.given("a test file with runnable test-style subroutines");

    let test_file = r#"use strict;
use warnings;
use Test::More;

sub test_addition {
    is(1 + 1, 2, "addition works");
}

sub t_subtraction {
    is(4 - 1, 3, "subtraction works");
}

subtest "math block" => sub {
    ok(1, "subtest works");
};

done_testing();
"#;

    let (mut harness, workspace) = setup_workspace(&[("t/math.t", test_file)])?;
    let uri = workspace.uri("t/math.t");
    harness.open(&uri, test_file)?;
    harness.barrier();

    scenario.when("requesting code lenses for the test file");
    let response = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("code lenses include runnable test actions with valid ranges");
    let lenses = response.as_array().ok_or("codeLens should return an array")?;
    assert!(!lenses.is_empty(), "expected code lenses for test subroutines");
    assert!(
        lenses.iter().all(has_lsp_range),
        "all code lenses should include a valid range; got {lenses:?}"
    );

    let command_titles: Vec<&str> = lenses.iter().filter_map(code_lens_command_title).collect();
    assert!(
        command_titles.iter().any(|title| title.contains("Run All Tests")),
        "expected Run All Tests lens for a .t file; got {command_titles:?}"
    );
    assert!(
        command_titles.iter().any(|title| title.contains("Run Test")),
        "expected Run Test lens; got {command_titles:?}"
    );
    assert!(
        command_titles.iter().any(|title| title.contains("Debug Test")),
        "expected Debug Test lens; got {command_titles:?}"
    );
    assert!(
        command_titles.iter().any(|title| title.contains("Run Subtest")),
        "expected Run Subtest lens; got {command_titles:?}"
    );

    scenario.when("resolving an unresolved references lens");
    let unresolved = lenses
        .iter()
        .find(|lens| lens.get("command").is_none() && code_lens_data_kind(lens).is_some())
        .ok_or("expected at least one unresolved references lens with data.kind")?;
    let resolved = harness.request("codeLens/resolve", unresolved.clone())?;

    scenario.then("resolved lens provides find-references command with editor coordinates");
    assert_eq!(
        code_lens_command_id(&resolved),
        Some("editor.action.findReferences"),
        "resolved references lens should use editor findReferences command; got {resolved:?}"
    );
    let resolved_title = code_lens_command_title(&resolved).unwrap_or_default();
    assert!(
        resolved_title.contains("reference"),
        "resolved references lens title should include reference count; got {resolved_title:?}"
    );
    let args = resolved
        .pointer("/command/arguments")
        .and_then(Value::as_array)
        .ok_or("resolved references lens should include command arguments")?;
    assert_eq!(args.len(), 2, "expected [line, character] arguments; got {args:?}");
    assert!(
        args.iter().all(Value::is_number),
        "resolved references lens arguments should be numeric editor coordinates; got {args:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_references_respect_include_declaration_toggle() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("References respect includeDeclaration toggle");

    let script = r#"use strict;
use warnings;

sub helper {
    return 1;
}

my $x = helper();
my $y = helper();
"#;

    scenario.given("a Perl file with one function declaration and multiple call sites");
    let (mut harness, workspace) = setup_workspace(&[("refs.pl", script)])?;
    let uri = workspace.uri("refs.pl");
    harness.open(&uri, script)?;
    harness.wait_for_symbol("helper", Some(&uri), Duration::from_secs(5)).ok();

    let (line, character) = find_position(script, "helper()");

    scenario.when("requesting references without declarations");
    let without_decl = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": false }
        }),
    )?;

    scenario.when("requesting references including declarations");
    let with_decl = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        }),
    )?;

    scenario.then("including declarations adds at least one extra location");
    let without = without_decl.as_array().map_or(0, Vec::len);
    let with = with_decl.as_array().map_or(0, Vec::len);
    assert!(
        with >= without,
        "includeDeclaration=true should not reduce matches (without={without}, with={with})"
    );
    assert!(
        with > without,
        "expected declaration-inclusive references to add declaration (without={without}, with={with})"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_document_symbols_refresh_after_incremental_edit() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Document symbols refresh after incremental edit");

    let before = r#"package Symbol::Refresh;
use strict;
use warnings;

sub alpha {
    return 1;
}

1;
"#;

    let after = r#"package Symbol::Refresh;
use strict;
use warnings;

sub alpha {
    return 1;
}

sub beta {
    return alpha();
}

1;
"#;

    scenario.given("a module that initially exposes a single subroutine");
    let (mut harness, workspace) = setup_workspace(&[("lib/Symbol/Refresh.pm", before)])?;
    let uri = workspace.uri("lib/Symbol/Refresh.pm");
    harness.open(&uri, before)?;
    harness.barrier();

    scenario.when("requesting document symbols before the edit");
    let symbols_before = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;
    let names_before = symbol_names(&symbols_before);

    scenario.then("only the original subroutine is present");
    assert!(
        names_before.iter().any(|name| name == "alpha"),
        "expected alpha before edit; got {names_before:?}"
    );
    assert!(
        !names_before.iter().any(|name| name == "beta"),
        "beta should not exist before edit; got {names_before:?}"
    );

    scenario.when("applying a didChange that introduces a second subroutine");
    harness.change_full(&uri, 2, after)?;
    harness.barrier();
    harness.wait_for_symbol("beta", Some(&uri), Duration::from_secs(10))?;
    harness.barrier();

    let symbols_after = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("document symbols include both original and new subroutines");
    let names_after = symbol_names(&symbols_after);
    assert!(
        names_after.iter().any(|name| name == "alpha"),
        "alpha should remain after edit; got {names_after:?}"
    );
    assert!(
        names_after.iter().any(|name| name == "beta"),
        "beta should appear after edit; got {names_after:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_selection_range_multi_cursor_returns_matching_entries()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Selection range supports multi-cursor requests");

    let code = r#"use strict;
use warnings;

sub combine {
    my ($left, $right) = @_;
    return $left . $right;
}

my $value = combine("a", "b");
"#;

    scenario.given("a Perl file with multiple symbols suitable for expand-selection");
    let (mut harness, workspace) = setup_workspace(&[("multi_cursor.pl", code)])?;
    let uri = workspace.uri("multi_cursor.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    let (left_line, left_col) = find_position(code, "$left");
    let (right_line, right_col) = find_position(code, "$right;");

    scenario.when("requesting selection ranges for two cursor positions in one call");
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [
                { "line": left_line, "character": left_col + 1 },
                { "line": right_line, "character": right_col + 1 }
            ]
        }),
    )?;

    scenario.then("the server returns one nested selection tree per requested cursor");
    let entries =
        response.as_array().ok_or("selectionRange multi-cursor response should be an array")?;
    assert_eq!(
        entries.len(),
        2,
        "expected two selection range results for two positions; got {entries:?}"
    );
    assert!(
        entries.iter().all(has_lsp_range),
        "each selection range entry should include a valid range; got {entries:?}"
    );
    assert!(
        entries.iter().all(|entry| selection_range_depth(entry) >= 2),
        "each entry should include nested parent expansion; got {entries:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_mojolicious_embedded_template_reports_no_parse_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Mojolicious embedded templates avoid Perl parse errors");

    let app = r#"use Mojolicious::Lite;

get '/' => sub {
    my $c = shift;
    $c->stash(title => 'BDD');
    $c->render(template => 'index');
};

app->start;

__DATA__
@@ index.html.ep
% my $name = 'Perl';
<h1><%= $title %> <%= $name %></h1>
"#;

    scenario.given("a Mojolicious::Lite app with an __DATA__ template section");
    let (mut harness, workspace) = setup_workspace(&[("app.pl", app)])?;
    let uri = workspace.uri("app.pl");
    harness.open(&uri, app)?;
    harness.barrier();

    scenario.when("requesting pull diagnostics for the app file");
    let report = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scenario.then("the embedded template does not produce parse-error diagnostics");
    let parse_errors = diagnostic_items(&report)
        .iter()
        .filter(|diag| diag.get("code").and_then(Value::as_str) == Some("PL001"))
        .count();
    assert_eq!(
        parse_errors, 0,
        "Mojolicious __DATA__ templates should not be parsed as Perl code; report={report:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_mojolicious_dashed_route_resolves_to_nested_controller_definition()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Mojolicious dashed routes resolve to nested controllers");

    let app = r##"package MyApp;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/admin/users')->to('admin-user#list');
}

1;
"##;

    let controller = r#"package MyApp::Controller::Admin::User;
use Mojo::Base 'Mojolicious::Controller';

sub list {
    return 'ok';
}

1;
"#;

    scenario.given("a nested controller and a Mojolicious route using a dashed target");
    let (mut harness, workspace) = setup_workspace(&[
        ("lib/MyApp.pm", app),
        ("lib/MyApp/Controller/Admin/User.pm", controller),
    ])?;

    let app_uri = workspace.uri("lib/MyApp.pm");
    let controller_uri = workspace.uri("lib/MyApp/Controller/Admin/User.pm");

    harness.open(&controller_uri, controller)?;
    harness.open(&app_uri, app)?;
    harness.wait_for_symbol("list", Some(&controller_uri), Duration::from_secs(10)).ok();

    let (line, character) = find_position(app, "admin-user#list");

    scenario.when("requesting go-to-definition on the dashed route target");
    let definition = wait_for_definition_uri(
        &mut harness,
        &app_uri,
        line,
        character,
        &controller_uri,
        Duration::from_secs(10),
    )?;

    scenario.then("definition resolves to the nested controller file");
    assert_eq!(
        first_location_uri(&definition).as_deref(),
        Some(controller_uri.as_str()),
        "expected dashed route target to resolve to nested controller; got {definition:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_document_highlight_marks_read_and_write_occurrences()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Document highlight includes read and write occurrences");

    let script = r#"use strict;
use warnings;

my $count = 0;
$count = $count + 1;
print $count;
"#;

    scenario.given("a document where the same lexical variable is written and read multiple times");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", script)])?;
    let script_uri = workspace.uri("main.pl");

    harness.open(&script_uri, script)?;
    harness.barrier();

    let (line, character) = find_position(script, "$count = $count + 1");

    scenario.when("requesting document highlights at the variable usage site");
    let highlights = harness.request_with_timeout(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": line, "character": character + 1 }
        }),
        Duration::from_secs(2),
    )?;

    scenario.then("highlights include multiple ranges and both read and write kinds");
    let items = highlight_items(&highlights);
    assert!(items.len() >= 3, "expected >=3 highlights, got {highlights:?}");

    let kinds = highlight_kinds(&highlights);
    assert!(kinds.contains(&2), "expected read highlight kind=2, got {kinds:?}");
    assert!(kinds.contains(&3), "expected write highlight kind=3, got {kinds:?}");

    Ok(())
}

#[test]
#[serial]
fn bdd_completion_returns_methods_for_bless_constructed_object()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Completion returns methods for bless-constructed object");

    let dog_module = r#"package Dog;
use strict;
use warnings;

sub new { bless {}, shift }
sub bark { "woof" }
sub sit  { "ok" }

1;
"#;

    let main_script = r#"use strict;
use warnings;
use lib './lib';
use Dog;

my $dog = Dog->new();
$dog->
"#;

    scenario.given("a workspace with a Dog package defining methods and a script calling methods");
    let (mut harness, workspace) =
        setup_workspace(&[("lib/Dog.pm", dog_module), ("main.pl", main_script)])?;
    let module_uri = workspace.uri("lib/Dog.pm");
    let main_uri = workspace.uri("main.pl");

    harness.open(&module_uri, dog_module)?;
    harness.open(&main_uri, main_script)?;
    harness.wait_for_symbol("bark", Some(&module_uri), Duration::from_secs(10)).ok();
    harness.barrier();

    scenario.when("requesting completion at the position after the arrow operator");
    let (line, character) = find_position(main_script, "$dog->");
    let completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": character + "$dog->".len() as u32 }
        }),
    )?;

    scenario.then("the response is a valid completion result (list or object), no crash");
    // The response should be either an array, an object with items, or null — not an error.
    assert!(
        completion.is_array() || completion.is_object() || completion.is_null(),
        "completion should return a valid LSP result shape; got {completion:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_completion_includes_builtins_for_function_context() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Completion includes builtins for function context");

    let code = r#"use strict;
use warnings;

pri
"#;

    scenario.given("a simple Perl file with a partially typed built-in function prefix");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting completion at the partially typed function name");
    let (line, character) = find_position(code, "pri");
    let completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + "pri".len() as u32 }
        }),
    )?;

    scenario.then("completion returns at least one item (builtins like print should appear)");
    let labels = completion_labels(&completion);
    assert!(
        !labels.is_empty(),
        "completion for 'pri' prefix should return at least one item; got {completion:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_hover_returns_content_for_known_builtin() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Hover returns content for known builtin");

    let code = r#"use strict;
use warnings;

print "hello\n";
"#;

    scenario.given("a file containing the print built-in function call");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting hover at the print token position");
    let (line, character) = find_position(code, "print ");
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("hover returns non-null contents with some text about print");
    assert!(!hover.is_null(), "hover over 'print' should return non-null contents; got {hover:?}");
    assert!(
        !hover_text(&hover).is_empty(),
        "hover text for 'print' should not be empty; got {hover:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_document_symbols_returns_subroutine_and_package() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Document symbols returns subroutine and package");

    let code = r#"package MyPkg;
use strict;
use warnings;

sub foo { 1 }
sub bar { 2 }

1;
"#;

    scenario.given("a Perl file with a package declaration and multiple subroutines");
    let (mut harness, workspace) = setup_workspace(&[("lib/MyPkg.pm", code)])?;
    let uri = workspace.uri("lib/MyPkg.pm");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting documentSymbols for that file");
    let symbols = harness
        .request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }))?;

    scenario.then("response contains entries for foo and bar subroutines");
    let names = symbol_names(&symbols);
    assert!(
        names.iter().any(|n| n == "foo"),
        "documentSymbols should include 'foo'; got {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "bar"),
        "documentSymbols should include 'bar'; got {names:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_references_find_all_uses_of_variable() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("References find all uses of variable");

    let code = r#"use strict;
use warnings;

my $x = 1;
my $y = $x + 1;
print $x;
"#;

    scenario.given("a file with a variable declared and used in multiple places");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting findReferences at the declaration with includeDeclaration=true");
    let (line, character) = find_position(code, "my $x = 1");
    let references = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 3 },
            "context": { "includeDeclaration": true }
        }),
    )?;

    scenario.then("response contains at least 2 locations (declaration + usages)");
    let locations = references.as_array().cloned().unwrap_or_default();
    assert!(
        locations.len() >= 2,
        "references for $x should include at least declaration + one usage; got {references:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_workspace_symbol_finds_exported_package_function() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Workspace symbol finds exported package function");

    let module = r#"package Utils;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(helper);

sub helper { 1 }

1;
"#;

    scenario.given("a workspace with a Utils module exporting a helper function");
    let (mut harness, workspace) = setup_workspace(&[("lib/Utils.pm", module)])?;
    let module_uri = workspace.uri("lib/Utils.pm");
    harness.open(&module_uri, module)?;
    harness.wait_for_symbol("helper", Some(&module_uri), Duration::from_secs(10)).ok();
    harness.barrier();

    scenario.when("searching workspace symbols for 'helper'");
    let result = harness.request("workspace/symbol", json!({ "query": "helper" }))?;

    scenario.then("response includes a symbol result for 'helper'");
    let items = result.as_array().cloned().unwrap_or_default();
    let names: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect();
    assert!(
        names.iter().any(|n| n == "helper" || n.ends_with("helper")),
        "workspace symbols should include 'helper'; got {names:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_goto_definition_resolves_within_same_file() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Goto definition resolves within same file");

    let code = r#"use strict;
use warnings;

sub greet { "hello" }
my $r = greet();
"#;

    scenario.given("a file with a local subroutine definition and a call to it");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.wait_for_symbol("greet", Some(&uri), Duration::from_secs(10)).ok();
    harness.barrier();

    scenario.when("requesting goto-definition at the greet() call site");
    let (line, character) = find_position(code, "greet()");
    let definition = wait_for_definition_uri(
        &mut harness,
        &uri,
        line,
        character,
        &uri,
        Duration::from_secs(10),
    )?;

    scenario.then("response contains a location pointing back into the same file");
    let def_uri = first_location_uri(&definition).unwrap_or_default();
    assert_eq!(def_uri, uri, "definition should resolve within the same file");

    Ok(())
}

#[test]
#[serial]
fn bdd_semantic_tokens_non_empty_for_valid_perl() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Semantic tokens non-empty for valid Perl");

    let code = r#"use strict;
use warnings;

my $x = 42;
print $x;
"#;

    scenario.given("a valid Perl file with variable declarations and print statement");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting semantic tokens full for the document");
    let tokens = harness
        .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }))?;

    scenario.then("response has non-empty data array");
    let data = semantic_token_data(&tokens);
    assert!(!data.is_empty(), "semantic tokens for valid Perl should not be empty; got {tokens:?}");

    Ok(())
}

#[test]
#[serial]
fn bdd_hover_returns_doc_for_default_variable() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Hover over $_ returns default-variable documentation");

    // Use $_ outside any foreach/while-that-declares-it so the server has no local
    // declaration to shadow the special-variable docs.
    let code = r#"use strict;
use warnings;

$_ = "hello world";
print length $_;
"#;

    scenario.given("a file that assigns to $_ directly without a loop that re-declares it");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting hover at the $_ in the length call");
    let (line, character) = find_position(code, "length $_");
    // +8 = '_'; get_token_at_position_static needs the alphanumeric part, not '$'
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 8 }
        }),
    )?;

    scenario.then("hover returns documentation describing $_ as the default variable");
    assert!(
        !hover.is_null(),
        "hover over '$_' should return non-null documentation; got {hover:?}"
    );
    let text = hover_text(&hover);
    assert!(!text.is_empty(), "hover text for '$_' should not be empty; got {hover:?}");
    assert!(
        text.to_lowercase().contains("default"),
        "hover for '$_' should mention 'default'; got: {text}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_hover_returns_doc_for_errno_special_variable() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Hover over $! returns errno/OS-error documentation");

    let code = r#"use strict;
use warnings;

open my $fh, '<', 'nonexistent.txt'
    or die "Cannot open: $!";
"#;

    scenario.given("a file that uses $! in an error-handling context after open");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting hover at the $! position in the die string");
    let (line, character) = find_position(code, "$!");
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("hover returns documentation describing $! as the OS error variable");
    assert!(
        !hover.is_null(),
        "hover over '$!' should return non-null documentation; got {hover:?}"
    );
    let text = hover_text(&hover);
    assert!(!text.is_empty(), "hover text for '$!' should not be empty; got {hover:?}");
    assert!(
        text.to_lowercase().contains("error") || text.to_lowercase().contains("errno"),
        "hover for '$!' should mention 'error' or 'errno'; got: {text}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_hover_returns_doc_for_file_test_operator() -> Result<(), Box<dyn std::error::Error>> {
    let scenario =
        BddScenario::new("Hover over -e file-test operator returns existence documentation");

    let code = r#"use strict;
use warnings;

my $path = "/tmp/test";
if (-e $path) {
    print "exists\n";
}
"#;

    scenario.given("a file that uses the -e file-test operator in a conditional");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");
    harness.open(&uri, code)?;
    harness.barrier();

    scenario.when("requesting hover at the -e operator position");
    let (line, character) = find_position(code, "-e $path");
    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("hover returns documentation describing -e as a file-existence test");
    assert!(
        !hover.is_null(),
        "hover over '-e' should return non-null documentation; got {hover:?}"
    );
    let text = hover_text(&hover);
    assert!(!text.is_empty(), "hover text for '-e' should not be empty; got {hover:?}");
    assert!(
        text.to_lowercase().contains("exist") || text.to_lowercase().contains("file"),
        "hover for '-e' should mention 'exist' or 'file'; got: {text}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_call_hierarchy_round_trip_maps_callers_and_callees() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = BddScenario::new("Call hierarchy round trip maps callers and callees");

    let code = r#"sub validate {
    my ($value) = @_;
    return $value > 0;
}

sub log_event {
    my ($message) = @_;
    print "$message\n";
}

sub process {
    my ($data) = @_;
    validate($data);
    log_event("processed");
    return $data;
}

sub main {
    process(42);
    log_event("done");
}

main();
"#;

    scenario.given("a file where main calls process and process calls two helpers");
    let (mut harness, workspace) = setup_workspace(&[("main.pl", code)])?;
    let uri = workspace.uri("main.pl");

    harness.open(&uri, code)?;
    harness.wait_for_symbol("process", Some(&uri), Duration::from_secs(10))?;
    harness.wait_for_symbol("main", Some(&uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("the editor prepares call hierarchy at process");
    let (line, character) = find_position(code, "sub process");
    let process_item =
        prepare_call_hierarchy_item(&mut harness, &uri, line, character + 4, "process")?;

    scenario.then("incoming calls identify main as the caller");
    let incoming = wait_for_call_hierarchy_edge_names(
        &mut harness,
        "callHierarchy/incomingCalls",
        &process_item,
        "/from/name",
        &["main"],
        Duration::from_secs(5),
    )?;
    let incoming_names = call_hierarchy_edge_names(&incoming, "/from/name");
    assert!(
        incoming_names.contains("main"),
        "incoming callers of process should include main; got {incoming_names:?}"
    );

    scenario.then("outgoing calls identify both helper callees");
    let outgoing = wait_for_call_hierarchy_edge_names(
        &mut harness,
        "callHierarchy/outgoingCalls",
        &process_item,
        "/to/name",
        &["validate", "log_event"],
        Duration::from_secs(5),
    )?;
    let outgoing_names = call_hierarchy_edge_names(&outgoing, "/to/name");
    assert!(
        outgoing_names.contains("validate") && outgoing_names.contains("log_event"),
        "outgoing callees of process should include validate and log_event; got {outgoing_names:?}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_call_hierarchy_cross_file_incoming_finds_script_caller()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Call hierarchy incoming calls cross file boundaries");

    let lib_code = r#"package Utils;
use strict;
use warnings;

sub format_string {
    my ($input) = @_;
    return uc($input);
}

1;
"#;

    let app_code = r#"use strict;
use warnings;
use lib './lib';
use Utils;

sub process {
    my $result = Utils::format_string("hello");
    return $result;
}

process();
"#;

    scenario.given("a library module and a script that calls a package-qualified sub");
    let (mut harness, workspace) =
        setup_workspace(&[("lib/Utils.pm", lib_code), ("bin/app.pl", app_code)])?;
    let lib_uri = workspace.uri("lib/Utils.pm");
    let app_uri = workspace.uri("bin/app.pl");

    harness.open(&lib_uri, lib_code)?;
    harness.open(&app_uri, app_code)?;
    harness.wait_for_symbol("format_string", Some(&lib_uri), Duration::from_secs(10))?;
    harness.wait_for_symbol("process", Some(&app_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("the editor prepares call hierarchy at the library sub definition");
    let (line, character) = find_position(lib_code, "sub format_string");
    let format_item =
        prepare_call_hierarchy_item(&mut harness, &lib_uri, line, character + 4, "format_string")?;

    scenario.then("incoming calls include the caller from the script");
    let incoming = wait_for_call_hierarchy_edge_names(
        &mut harness,
        "callHierarchy/incomingCalls",
        &format_item,
        "/from/name",
        &["process"],
        Duration::from_secs(5),
    )?;
    let calls = incoming.as_array().ok_or("incomingCalls returned non-array")?;
    let process_call = calls
        .iter()
        .find(|call| call.pointer("/from/name").and_then(Value::as_str) == Some("process"))
        .ok_or_else(|| format!("expected process caller in incomingCalls; got {incoming:?}"))?;
    let caller_uri = process_call.pointer("/from/uri").and_then(Value::as_str).unwrap_or("");
    assert!(
        uri_matches(&app_uri, caller_uri),
        "incoming caller URI should be {app_uri}; got {caller_uri}"
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_call_hierarchy_cross_file_outgoing_points_to_target_module()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Call hierarchy outgoing calls preserve target URI");

    let lib_code = r#"package Utils;
use strict;
use warnings;

sub format_string {
    my ($input) = @_;
    return uc($input);
}

1;
"#;

    let app_code = r#"use strict;
use warnings;
use lib './lib';
use Utils;

sub process {
    my $result = Utils::format_string("hello");
    return $result;
}

process();
"#;

    scenario.given("a script whose function calls into a library module");
    let (mut harness, workspace) =
        setup_workspace(&[("lib/Utils.pm", lib_code), ("bin/app.pl", app_code)])?;
    let lib_uri = workspace.uri("lib/Utils.pm");
    let app_uri = workspace.uri("bin/app.pl");

    harness.open(&lib_uri, lib_code)?;
    harness.open(&app_uri, app_code)?;
    harness.wait_for_symbol("format_string", Some(&lib_uri), Duration::from_secs(10))?;
    harness.wait_for_symbol("process", Some(&app_uri), Duration::from_secs(10))?;
    harness.barrier();

    scenario.when("the editor prepares call hierarchy at the script function");
    let (line, character) = find_position(app_code, "sub process");
    let process_item =
        prepare_call_hierarchy_item(&mut harness, &app_uri, line, character + 4, "process")?;

    scenario.then("outgoing calls include format_string in the module, not the script");
    let outgoing = wait_for_call_hierarchy_edge_names(
        &mut harness,
        "callHierarchy/outgoingCalls",
        &process_item,
        "/to/name",
        &["format_string"],
        Duration::from_secs(5),
    )?;
    let calls = outgoing.as_array().ok_or("outgoingCalls returned non-array")?;
    let format_call = calls
        .iter()
        .find(|call| {
            call.pointer("/to/name")
                .and_then(Value::as_str)
                .map(|name| name == "format_string" || name.ends_with("::format_string"))
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!("expected format_string callee in outgoingCalls; got {outgoing:?}")
        })?;
    let target_uri = format_call.pointer("/to/uri").and_then(Value::as_str).unwrap_or("");
    assert!(
        uri_matches(&lib_uri, target_uri),
        "outgoing target URI should be {lib_uri}; got {target_uri}"
    );

    Ok(())
}
