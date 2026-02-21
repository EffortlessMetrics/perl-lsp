//! BDD-style workflow coverage for core LSP behaviors.
//!
//! These tests are structured as Given/When/Then scenarios to validate
//! end-to-end user workflows using the real JSON-RPC harness.

mod support;

use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeSet;
use std::time::Duration;
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
    for (line_idx, line) in text.split('\n').enumerate() {
        if let Some(col) = line.find(needle) {
            return (line_idx as u32, col as u32);
        }
    }
    panic!("needle '{}' not found in text", needle);
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

fn first_location_uri(response: &Value) -> Option<String> {
    if let Some(arr) = response.as_array() {
        arr.first().and_then(|v| v.get("uri").and_then(Value::as_str)).map(ToOwned::to_owned)
    } else {
        response.get("uri").and_then(Value::as_str).map(ToOwned::to_owned)
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

fn diagnostic_error_count(report: &Value) -> usize {
    diagnostic_items(report)
        .iter()
        .filter(|diag| diag.get("severity").and_then(Value::as_u64) == Some(1))
        .count()
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

    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2)).ok();

    scenario.when("requesting definition on the qualified call in the script");
    let (line, character) = find_position(main, "process_data()");
    let definition = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    scenario.then("the definition resolves to the module file");
    let def_uri = first_location_uri(&definition).unwrap_or_default();
    assert_eq!(def_uri, module_uri);

    scenario.when("requesting references on the module definition");
    let (def_line, def_char) = find_position(module, "process_data");
    let references = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": module_uri },
            "position": { "line": def_line, "character": def_char },
            "context": { "includeDeclaration": true }
        }),
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

    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2)).ok();

    scenario.when("renaming the function at its declaration");
    let (def_line, def_char) = find_position(module, "process_data");
    let edit = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": module_uri },
            "position": { "line": def_line, "character": def_char },
            "newName": "process_records"
        }),
    )?;

    scenario.then("the workspace edit touches both files");
    let uris = workspace_edit_uris(&edit);
    assert!(uris.contains(&module_uri), "rename should edit module file");
    assert!(uris.contains(&main_uri), "rename should edit main script file");

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

    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2)).ok();

    scenario.when("updating both files with didChange to a new function name");
    harness.change_full(&module_uri, 2, module_v2)?;
    harness.change_full(&main_uri, 2, main_v2)?;
    harness.barrier();

    harness.wait_for_symbol("process_records", Some(&module_uri), Duration::from_secs(2)).ok();

    scenario.then("go-to-definition resolves the updated symbol across files");
    let (line, character) = find_position(main_v2, "process_records()");
    let definition = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": character }
        }),
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
    harness.wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2)).ok();

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
    let edit = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": character },
            "newName": "process_records"
        }),
    )?;

    scenario.then("rename returns edits affecting both declaration and usage files");
    let uris = workspace_edit_uris(&edit);
    assert!(uris.contains(&module_uri), "rename should edit declaration file");
    assert!(uris.contains(&main_uri), "rename should edit usage file");

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

    scenario.when("requesting document formatting");
    let formatting = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }),
    )?;

    scenario.then("the response is null or a valid list of text edits");
    assert!(
        formatting.is_null() || formatting.is_array(),
        "formatting should return null or text edit array"
    );

    if let Some(edits) = formatting.as_array()
        && let Some(first_edit) = edits.first()
    {
        assert!(has_lsp_range(first_edit), "text edits should include an LSP range structure");
        assert!(
            first_edit.get("newText").and_then(Value::as_str).is_some(),
            "text edits should include newText"
        );
    }

    Ok(())
}
