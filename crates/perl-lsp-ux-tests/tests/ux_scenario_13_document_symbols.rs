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

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_13_document_symbols.rs";

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

fn create_symbols_harness() -> Result<UxHarness> {
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("Greeter.pm", SYMBOLS_SOURCE),
    )?;
    harness.open_file("Greeter.pm", SYMBOLS_SOURCE)?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(harness)
}

fn create_empty_harness() -> Result<UxHarness> {
    let source = "# empty file\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("empty.pl", source))?;
    harness.open_file("empty.pl", source)?;
    Ok(harness)
}

fn symbol_has_valid_shape(symbol: &Value) -> bool {
    let has_name = symbol.get("name").and_then(Value::as_str).is_some();
    let has_valid_kind =
        symbol.get("kind").is_none_or(|kind| kind.as_u64().is_some_and(|k| (1..=26).contains(&k)));
    let has_range_or_location = symbol.get("range").is_some() || symbol.get("location").is_some();
    has_name && has_valid_kind && has_range_or_location
}

fn symbols_include_known_sub_name(symbols: &[Value]) -> bool {
    symbols
        .iter()
        .filter_map(|symbol| symbol.get("name").and_then(Value::as_str))
        .any(|name| ["new", "greet", "farewell"].contains(&name))
}

#[test]
fn scenario_13_document_symbol_does_not_error() {
    run_ux_scenario(
        "document_symbols_core",
        SCENARIO_FILE,
        "scenario_13_document_symbol_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_symbols_harness()?;
            recorder.mark_request_start("document_symbols_rich_file");
            let result = harness.document_symbols("Greeter.pm");
            if result.is_ok() {
                recorder.mark_first_useful_result("document_symbols_rich_file");
            }
            recorder.check(
                "textDocument/documentSymbol does not return a JSON-RPC error",
                result.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_13_returned_symbols_have_valid_shape() {
    run_ux_scenario(
        "document_symbols_core",
        SCENARIO_FILE,
        "scenario_13_returned_symbols_have_valid_shape",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_symbols_harness()?;
            recorder.mark_request_start("document_symbols_shape");
            let symbols = harness.document_symbols("Greeter.pm")?;
            if !symbols.is_empty() {
                recorder.mark_first_useful_result("document_symbols_shape");
            }
            recorder.check(
                "returned document symbols have valid shape when present",
                symbols.iter().all(symbol_has_valid_shape),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_13_rich_file_returns_known_sub_names() {
    run_ux_scenario(
        "document_symbols_core",
        SCENARIO_FILE,
        "scenario_13_rich_file_returns_known_sub_names",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_symbols_harness()?;
            recorder.mark_request_start("document_symbols_known_subs");
            let symbols = harness.document_symbols("Greeter.pm")?;
            let found_known_name = symbols_include_known_sub_name(&symbols);
            if found_known_name {
                recorder.mark_first_useful_result("document_symbols_known_subs");
            }
            recorder.check(
                "rich file returns empty degraded result or at least one known sub name",
                symbols.is_empty() || found_known_name,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_13_empty_file_returns_empty_or_null() {
    run_ux_scenario(
        "document_symbols_core",
        SCENARIO_FILE,
        "scenario_13_empty_file_returns_empty_or_null",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_empty_harness()?;
            recorder.mark_request_start("document_symbols_empty_file");
            let result = harness.document_symbols("empty.pl");
            if result.is_ok() {
                recorder.mark_first_useful_result("document_symbols_empty_file");
            }
            recorder.check(
                "documentSymbol on empty file does not return a JSON-RPC error",
                result.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
