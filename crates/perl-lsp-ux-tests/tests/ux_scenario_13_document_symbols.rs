// Binary skip messages are visible only in integration-test output.
#![allow(clippy::print_stderr)]

//! Scenario 13 — Document symbols feature grid coverage.
//!
//! Verifies that `textDocument/documentSymbol` is wired up end-to-end for the
//! LSP feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/documentSymbol` MUST NOT return a JSON-RPC error.
//! - When symbols are returned they MUST have at least a `name` field.
//! - A file with named subs and packages MUST return those source-backed symbols.
//! - No crash signatures after the request.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

/// Source with three named subs and a package declaration — rich symbol table.
const SYMBOLS_SOURCE: &str = "\
package Greeter;\n\
use strict;\n\
use warnings;\n\
\n\
sub new {\n\
    my ($class, %opts) = @_;\n\
    return bless { name => $opts{name} // 'World' }, $class;\n\
}\n\
\n\
sub greet {\n\
    my ($self) = @_;\n\
    return \"Hello, \" . $self->{name} . \"!\";\n\
}\n\
\n\
sub farewell {\n\
    my ($self) = @_;\n\
    return \"Goodbye, \" . $self->{name} . \"!\";\n\
}\n\
\n\
1;\n\
";

#[test]
fn scenario_13_document_symbol_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )?;

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    let result = harness.document_symbols("Greeter.pm");
    assert!(
        result.is_ok(),
        "textDocument/documentSymbol must not return a JSON-RPC error \
         — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_13_returned_symbols_have_valid_shape() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )?;

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("Greeter.pm")?;

    for sym in &symbols {
        assert!(sym.get("name").is_some(), "Each symbol must have a 'name' field, got: {:?}", sym);
        // kind is required by the LSP spec (1-26 SymbolKind enum).
        if let Some(kind) = sym.get("kind") {
            assert!(
                kind.as_u64().is_some_and(|k| (1..=26).contains(&k)),
                "Symbol 'kind' must be 1-26, got: {kind}"
            );
        }
        // DocumentSymbol has 'range'; SymbolInformation has 'location'.
        // Either is acceptable.
        let has_range = sym.get("range").is_some();
        let has_location = sym.get("location").is_some();
        assert!(
            has_range || has_location,
            "Symbol must have either 'range' (DocumentSymbol) or 'location' \
             (SymbolInformation), got: {:?}",
            sym
        );
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_13_rich_file_returns_known_package_and_sub_names() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )?;

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("Greeter.pm")?;
    let names = symbol_names(&symbols);

    for expected in ["Greeter", "new", "greet", "farewell"] {
        assert!(
            names.iter().any(|actual| actual == expected),
            "Expected source-backed document symbol `{expected}` in Greeter.pm, got: {names:?}"
        );
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_13_empty_file_returns_empty_or_null() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return Ok(());
    }

    let source = "# empty file\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("empty.pl", source))?;

    harness.open_file("empty.pl", source)?;

    let symbols = harness.document_symbols("empty.pl")?;

    // Empty list is the correct response for a file with no symbols.
    // We just verify no crash.
    let _ = symbols;

    harness.assert_no_crash();
    Ok(())
}

fn symbol_names(symbols: &[Value]) -> Vec<String> {
    let mut names = Vec::new();
    for symbol in symbols {
        collect_symbol_names(symbol, &mut names);
    }
    names.sort();
    names.dedup();
    names
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
