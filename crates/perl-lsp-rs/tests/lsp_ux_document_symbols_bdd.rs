//! BDD UX scenarios for document symbols.
//!
//! Focus: user-visible symbol outline behavior during live editing workflows.

mod support;

use serde_json::Value;
use serial_test::serial;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use support::lsp_harness::{LspHarness, TempWorkspace};

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, msg: &str) {
        eprintln!("[{}] Given {msg}", self.name);
    }

    fn when(&self, msg: &str) {
        eprintln!("[{}] When {msg}", self.name);
    }

    fn then(&self, msg: &str) {
        eprintln!("[{}] Then {msg}", self.name);
    }
}

fn symbol_names(symbols: &Value) -> BTreeSet<String> {
    fn walk_document_symbol(symbol: &Value, names: &mut BTreeSet<String>) {
        if let Some(name) = symbol.get("name").and_then(Value::as_str) {
            names.insert(name.to_owned());
        }

        if let Some(children) = symbol.get("children").and_then(Value::as_array) {
            for child in children {
                walk_document_symbol(child, names);
            }
        }
    }

    let mut names = BTreeSet::new();
    if let Some(arr) = symbols.as_array() {
        for symbol in arr {
            if symbol.get("location").is_some() {
                if let Some(name) = symbol.get("name").and_then(Value::as_str) {
                    names.insert(name.to_owned());
                }
            } else {
                walk_document_symbol(symbol, &mut names);
            }
        }
    }

    names
}

fn wait_for_symbol_names(
    harness: &mut LspHarness,
    uri: &str,
    required: &[&str],
    forbidden: &[&str],
    budget: Duration,
) -> Result<BTreeSet<String>, String> {
    let start = Instant::now();
    let mut last = BTreeSet::new();

    while start.elapsed() < budget {
        let response = harness.document_symbols(uri)?;
        let names = symbol_names(&response);

        let has_required = required.iter().all(|name| names.contains(*name));
        let has_no_forbidden = forbidden.iter().all(|name| !names.contains(*name));

        if has_required && has_no_forbidden {
            return Ok(names);
        }

        last = names;
        harness.barrier();
        std::thread::sleep(Duration::from_millis(40));
    }

    Err(format!("document symbols did not converge within {budget:?}. Last symbols: {:?}", last))
}

fn setup_workspace(files: &[(&str, &str)]) -> Result<(LspHarness, TempWorkspace), String> {
    let workspace = TempWorkspace::new()?;
    for (path, content) in files {
        workspace.write(path, content)?;
    }

    let mut harness = LspHarness::new();
    harness.initialize_ready(&workspace.root_uri, None)?;

    Ok((harness, workspace))
}

#[test]
#[serial]
fn bdd_document_symbols_reflect_incremental_rename() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = Scenario::new("Document symbols update after didChange rename");

    let before = "package Demo;\nsub alpha { return 1; }\nsub beta { return alpha(); }\n1;\n";
    let after = "package Demo;\nsub alpha_renamed { return 1; }\nsub beta { return alpha_renamed(); }\n1;\n";

    scenario.given("a document with two subroutines in the editor");
    let (mut harness, workspace) = setup_workspace(&[("lib/Demo.pm", before)])?;
    let uri = workspace.uri("lib/Demo.pm");
    harness.open_document(&uri, before)?;

    scenario.when("the user renames one subroutine via didChange");
    harness.change_full(&uri, 2, after)?;

    scenario.then("the symbol outline contains the new symbol and removes the old one");
    let names = wait_for_symbol_names(
        &mut harness,
        &uri,
        &["Demo", "alpha_renamed", "beta"],
        &["alpha"],
        Duration::from_secs(3),
    )?;
    assert!(names.contains("alpha_renamed"));
    assert!(!names.contains("alpha"));

    Ok(())
}

#[test]
#[serial]
fn bdd_document_symbols_track_new_declaration_without_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = Scenario::new("Document symbols reflect newly added declaration in open buffer");

    let before = "package Demo;\nsub alpha { return 1; }\n1;\n";
    let after = "package Demo;\nsub alpha { return 1; }\nsub gamma { return 2; }\n1;\n";

    scenario.given("an already open Perl module");
    let (mut harness, workspace) = setup_workspace(&[("lib/Demo.pm", before)])?;
    let uri = workspace.uri("lib/Demo.pm");
    harness.open_document(&uri, before)?;

    scenario.when("the user adds another subroutine in memory");
    harness.change_full(&uri, 2, after)?;

    scenario.then("documentSymbol includes both declarations from the latest buffer state");
    let names = wait_for_symbol_names(
        &mut harness,
        &uri,
        &["alpha", "gamma"],
        &[],
        Duration::from_secs(3),
    )?;
    assert!(names.contains("alpha"));
    assert!(names.contains("gamma"));

    Ok(())
}
