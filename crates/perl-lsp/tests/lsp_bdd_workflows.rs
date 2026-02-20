//! BDD-style workflow coverage for core LSP behaviors.
//!
//! These tests are structured as Given/When/Then scenarios to validate
//! end-to-end user workflows using the real JSON-RPC harness.

mod support;

use serde_json::{Value, json};
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
            } else if let Some(uri) = item
                .pointer("/location/uri")
                .and_then(|v| v.as_str())
            {
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
            if let Some(uri) = change
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
            {
                uris.insert(uri.to_string());
            }
        }
    }

    uris
}

fn setup_workspace(files: &[(&str, &str)]) -> Result<(LspHarness, TempWorkspace), String> {
    let (mut harness, workspace) = LspHarness::with_workspace(files)?;

    // Give the server a moment to settle after initialize.
    harness.barrier();

    Ok((harness, workspace))
}

#[test]
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

    harness
        .wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2))
        .ok();

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
    let def_uri = if let Some(arr) = definition.as_array() {
        arr.first()
            .and_then(|v| v.get("uri").and_then(|u| u.as_str()))
            .unwrap_or_default()
            .to_string()
    } else {
        definition
            .get("uri")
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string()
    };
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

    harness
        .wait_for_symbol("process_data", Some(&module_uri), Duration::from_secs(2))
        .ok();

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

    harness
        .wait_for_symbol("transform", Some(&module_uri), Duration::from_secs(2))
        .ok();

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
