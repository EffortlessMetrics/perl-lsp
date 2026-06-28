// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 13 — Document symbols feature grid coverage.
//!
//! Verifies that `textDocument/documentSymbol` is wired up end-to-end for the
//! LSP feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/documentSymbol` MUST NOT return a JSON-RPC error.
//! - When symbols are returned they MUST have at least a `name` field.
//! - A file with named subs and packages SHOULD return at least one symbol.
//! - An empty result is acceptable for degraded-mode servers.
//! - No crash signatures after the request.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

/// Source with two named subs and a package declaration — rich symbol table.
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
fn scenario_13_document_symbol_does_not_error() {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(300));

    let result = harness.document_symbols("Greeter.pm");
    assert!(
        result.is_ok(),
        "textDocument/documentSymbol must not return a JSON-RPC error \
         — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_13_returned_symbols_have_valid_shape() {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("Greeter.pm").expect("documentSymbol must not error");

    for sym in &symbols {
        assert!(sym.get("name").is_some(), "Each symbol must have a 'name' field, got: {:?}", sym);
        // kind is required by the LSP spec (1-26 SymbolKind enum).
        if let Some(kind) = sym.get("kind") {
            let k = kind.as_u64().unwrap_or(0);
            assert!((1..=26).contains(&k), "Symbol 'kind' must be 1-26, got: {}", k);
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
}

#[test]
fn scenario_13_rich_file_returns_known_sub_names() {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("Greeter.pm", SYMBOLS_SOURCE).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("Greeter.pm").expect("documentSymbol must not error");

    if symbols.is_empty() {
        eprintln!(
            "INFO scenario_13: documentSymbol returned empty list \
             (degraded mode acceptable — sub-symbol extraction may not be implemented yet)"
        );
        harness.assert_no_crash();
        return;
    }

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // At least one of our three subs should appear.
    let found_any = names.iter().any(|n| ["new", "greet", "farewell"].contains(n));
    assert!(
        found_any,
        "Expected at least one of [new, greet, farewell] in document symbols, \
         got: {:?}",
        names
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_13_empty_file_returns_empty_or_null() {
    if !binary_available() {
        eprintln!("SKIP scenario_13: perl-lsp binary not found");
        return;
    }

    let source = "# empty file\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("empty.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("empty.pl", source).expect("didOpen should succeed");

    let symbols =
        harness.document_symbols("empty.pl").expect("documentSymbol on empty file must not error");

    // Empty list is the correct response for a file with no symbols.
    // We just verify no crash.
    let _ = symbols;

    harness.assert_no_crash();
}
