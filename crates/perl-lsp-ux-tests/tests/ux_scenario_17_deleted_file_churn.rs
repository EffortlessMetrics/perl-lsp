// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 17 — deleting a watched file evicts stale symbols and definition targets.
//!
//! Verifies that a real `workspace/didChangeWatchedFiles` Deleted event removes
//! stale search results and cross-file definitions from the UX surface.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use serde_json::json;
use std::time::{Duration, Instant};

const MODULE_SOURCE: &str = "\
package ModuleGone;\n\
\n\
sub gone_value_4068 {\n\
    return 42;\n\
}\n\
\n\
1;\n\
";

const SCRIPT_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use ModuleGone;\n\
\n\
my $value = ModuleGone::gone_value_4068();\n\
print \"$value\\n\";\n\
";

fn symbol_names(symbols: &[Value]) -> Vec<&str> {
    symbols.iter().filter_map(|symbol| symbol["name"].as_str()).collect()
}

#[test]
fn scenario_17_deleted_module_evicted_from_symbols_and_definition() {
    if !binary_available() {
        eprintln!("SKIP scenario_17: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file("main.pl", SCRIPT_SOURCE)
            .with_file("lib/ModuleGone.pm", MODULE_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("main.pl", SCRIPT_SOURCE).expect("didOpen should succeed");

    let cursor = harness.position_cursor("main.pl", 5, 25);
    let before_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_before = Vec::new();
    let mut defs_before = Vec::new();
    while Instant::now() < before_deadline {
        symbols_before = harness
            .workspace_symbols("gone_value_4068")
            .expect("workspace/symbol must not error before delete");
        defs_before =
            harness.definition_at(&cursor).expect("definition must not error before delete");

        if !symbols_before.is_empty() && !defs_before.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        !symbols_before.is_empty(),
        "Expected gone_value_4068 to be searchable before delete, got names {:?}",
        symbol_names(&symbols_before)
    );
    assert!(
        !defs_before.is_empty(),
        "Expected definition to resolve before delete, got {:?}",
        defs_before
    );
    harness.assert_normalized_eq(
        &defs_before[0],
        &json!({
            "uri": "file://$WORKSPACE/lib/ModuleGone.pm",
            "range": defs_before[0]["range"].clone(),
        }),
    );

    harness.workspace.delete("lib/ModuleGone.pm").expect("module delete should succeed");
    harness
        .notify_watched_files(&[("lib/ModuleGone.pm", 3)])
        .expect("didChangeWatchedFiles Deleted notification must not fail");

    let after_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_after = Vec::new();
    let mut defs_after = Vec::new();
    while Instant::now() < after_deadline {
        symbols_after = harness
            .workspace_symbols("gone_value_4068")
            .expect("workspace/symbol must not error after delete");
        defs_after =
            harness.definition_at(&cursor).expect("definition must not error after delete");

        if symbols_after.is_empty() && defs_after.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        symbols_after.is_empty(),
        "Expected deleted symbol gone_value_4068 to disappear from workspace/symbol, got names {:?}",
        symbol_names(&symbols_after)
    );
    assert!(
        defs_after.is_empty(),
        "Expected deleted module definition target to disappear after delete, got {:?}",
        defs_after
    );

    harness.assert_no_crash();
}
