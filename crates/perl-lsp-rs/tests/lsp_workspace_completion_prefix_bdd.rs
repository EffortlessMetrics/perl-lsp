//! BDD tests for workspace completion prefix filtering (issue #6861).
//!
//! The workspace index uses substring (`contains`) matching internally for
//! `workspace/symbol` queries, which is correct per LSP 3.17 §5.3.2.  But when
//! the same index results are used for `textDocument/completion`, only candidates
//! whose name *starts with* the typed prefix should be offered.  Returning
//! symbols that merely contain the prefix anywhere is noise that degrades UX
//! on large codebases.
//!
//! These tests verify that the completion provider applies a `starts_with` guard
//! before surfacing workspace symbols, while the `workspace/symbol` endpoint
//! continues to return all substring matches.

mod support;

use serial_test::serial;
use support::lsp_harness::LspHarness;
use support::ux_bdd::{UxScenario, completion_contains_label, completion_labels};

/// Extract symbol names from a `workspace/symbol` response array.
fn workspace_symbol_names(response: &serde_json::Value) -> Vec<String> {
    response
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Scenario 1 — cross-file subroutine prefix filtering (core bug regression)
// ---------------------------------------------------------------------------
//
// Two modules define subroutines that share a common fragment "bar":
//
//   helpers.pm: sub foobar { }     ← CONTAINS "bar" but does NOT start with it
//   reporter.pm: sub bar_report { } ← STARTS WITH "bar"
//
// Typing "bar" in main.pl should surface only `bar_report`.
// Before the fix, `foobar` would also appear because find_symbols uses contains().

#[test]
#[serial]
fn bdd_completion_workspace_prefix_excludes_contains_only_match()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Workspace completion uses starts_with, not contains");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    scenario.given("helpers.pm defines 'foobar' — contains 'bar' but does not start with it");
    harness
        .open("file:///workspace/helpers.pm", "package Helpers;\nsub foobar { return 1; }\n1;\n")?;

    scenario.given("reporter.pm defines 'bar_report' — starts with 'bar'");
    harness.open(
        "file:///workspace/reporter.pm",
        "package Reporter;\nsub bar_report { return 1; }\n1;\n",
    )?;

    scenario.given("main.pl is the completion target with cursor after 'bar'");
    harness.open(
        "file:///workspace/main.pl",
        // Line 0: use Helpers;
        // Line 1: use Reporter;
        // Line 2: bar   ← cursor position (line 2, col 3)
        "use Helpers;\nuse Reporter;\nbar",
    )?;
    harness.barrier();

    scenario.when("requesting completion at the 'bar' prefix in main.pl");
    let completions = harness.completion_at("file:///workspace/main.pl", 2, 3)?;
    let labels = completion_labels(&completions);

    scenario.then("'bar_report' is offered — name starts with the typed prefix");
    assert!(
        completion_contains_label(&completions, "bar_report"),
        "bar_report should appear (name starts with 'bar'), got: {labels:?}"
    );

    scenario.then("'foobar' is NOT offered — name merely contains 'bar', not starts with it");
    assert!(
        !completion_contains_label(&completions, "foobar"),
        "foobar must be absent (name does not start with 'bar'), got: {labels:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2 — longer prefix: mid-name fragment excluded
// ---------------------------------------------------------------------------
//
// Typing a prefix that appears mid-word in a cross-file subroutine name
// should not surface that subroutine.
//
//   utils.pm: sub not_fix_it { }   ← contains "fix" in the middle
//   fixer.pm: sub fix_data { }     ← starts with "fix"
//
// Typing "fix" must return `fix_data` only.

#[test]
#[serial]
fn bdd_completion_workspace_mid_name_fragment_excluded() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Mid-name fragment not offered as completion candidate");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    scenario.given("utils.pm defines 'not_fix_it' — 'fix' appears mid-name");
    harness
        .open("file:///workspace/utils.pm", "package Utils;\nsub not_fix_it { return 0; }\n1;\n")?;

    scenario.given("fixer.pm defines 'fix_data' — name starts with 'fix'");
    harness
        .open("file:///workspace/fixer.pm", "package Fixer;\nsub fix_data { return 1; }\n1;\n")?;

    harness.open("file:///workspace/main2.pl", "use Utils;\nuse Fixer;\nfix")?;
    harness.barrier();

    scenario.when("requesting completion at the 'fix' prefix");
    let completions = harness.completion_at("file:///workspace/main2.pl", 2, 3)?;
    let labels = completion_labels(&completions);

    scenario.then("'fix_data' appears — starts with 'fix'");
    assert!(
        completion_contains_label(&completions, "fix_data"),
        "fix_data should appear (starts with 'fix'), got: {labels:?}"
    );

    scenario.then("'not_fix_it' does NOT appear — 'fix' is a mid-name fragment");
    assert!(
        !completion_contains_label(&completions, "not_fix_it"),
        "not_fix_it must be absent ('fix' only appears mid-name), got: {labels:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3 — workspace/symbol endpoint is unaffected (uses substring match)
// ---------------------------------------------------------------------------
//
// `workspace/symbol` is a distinct LSP feature whose spec says substring
// matching is valid.  The fix to completion must NOT change symbol search.

#[test]
#[serial]
fn bdd_workspace_symbol_still_uses_substring_match() -> Result<(), Box<dyn std::error::Error>> {
    let scenario =
        UxScenario::new("workspace/symbol continues to use substring matching after the fix");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    scenario.given("helpers.pm defines 'foobar' and reporter.pm defines 'bar_report'");
    harness
        .open("file:///workspace/helpers.pm", "package Helpers;\nsub foobar { return 1; }\n1;\n")?;
    harness.open(
        "file:///workspace/reporter.pm",
        "package Reporter;\nsub bar_report { return 1; }\n1;\n",
    )?;
    harness.barrier();

    scenario.when("querying workspace/symbol with 'bar'");
    let sym_response =
        harness.request("workspace/symbol", serde_json::json!({ "query": "bar" }))?;
    let names = workspace_symbol_names(&sym_response);

    scenario.then("'bar_report' is in the symbol list (starts with 'bar')");
    assert!(
        names.contains(&"bar_report".to_string()),
        "bar_report should be in workspace symbols, got: {names:?}"
    );

    scenario.then(
        "'foobar' is also in the symbol list — substring match is correct for workspace/symbol",
    );
    assert!(
        names.contains(&"foobar".to_string()),
        "foobar should appear in workspace/symbol (substring match), got: {names:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 4 — prefix match returns workspace symbol while excluding others
// ---------------------------------------------------------------------------
//
// Typing `comp` should offer `compute` from the workspace (starts with `comp`)
// while NOT offering `decompose` (contains `comp` but starts with `de`).

#[test]
#[serial]
fn bdd_completion_prefix_match_returns_start_with_only() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Prefix 'comp' offers 'compute' but excludes 'decompose'");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    scenario.given("util.pm defines 'compute' (starts with 'comp') and 'decompose' (contains 'comp' but not at start)");
    harness.open(
        "file:///workspace/util.pm",
        "package Util;\nsub compute { return 42; }\nsub decompose { return 0; }\n1;\n",
    )?;
    // Cursor after "comp" on line 1
    harness.open("file:///workspace/consumer.pl", "use Util;\ncomp")?;
    harness.barrier();

    scenario.when("completion is requested at 'comp' prefix");
    // Line 1, col 4 = after "comp"
    let completions = harness.completion_at("file:///workspace/consumer.pl", 1, 4)?;
    let labels = completion_labels(&completions);

    scenario.then("'compute' appears — its name starts with 'comp'");
    assert!(
        completion_contains_label(&completions, "compute"),
        "compute should appear (starts with 'comp'), got: {labels:?}"
    );

    scenario.then("'decompose' does NOT appear — 'comp' is a mid-name fragment");
    assert!(
        !completion_contains_label(&completions, "decompose"),
        "decompose must be absent ('comp' appears mid-name in decompose), got: {labels:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 5 — qualified name prefix (Package::sub) correctly matched
// ---------------------------------------------------------------------------
//
// When the user types a qualified prefix like `Reporter::bar`, only subroutines
// in the Reporter namespace that start with `bar` should appear.  This verifies
// that the qualified_name check in the filter is also correct.

#[test]
#[serial]
fn bdd_completion_qualified_prefix_filters_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Qualified prefix 'Reporter::bar' filters to bar_report only");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///workspace/reporter.pm",
        "package Reporter;\nsub bar_report { return 1; }\nsub baz_other { return 2; }\n1;\n",
    )?;
    // Cursor after "Reporter::bar"  (line 1, col 13)
    harness.open("file:///workspace/qualified.pl", "use Reporter;\nReporter::bar")?;
    harness.barrier();

    scenario.when("completing at 'Reporter::bar' (col 13 = after the full prefix)");
    let completions = harness.completion_at("file:///workspace/qualified.pl", 1, 13)?;
    let labels = completion_labels(&completions);

    scenario.then("'Reporter::bar_report' or 'bar_report' is offered");
    let has_match = completion_contains_label(&completions, "bar_report")
        || completion_contains_label(&completions, "Reporter::bar_report");
    assert!(
        has_match,
        "bar_report (or Reporter::bar_report) should appear for qualified prefix, got: {labels:?}"
    );

    scenario
        .then("'baz_other' or 'Reporter::baz_other' is NOT offered — doesn't match 'bar' prefix");
    let has_baz = completion_contains_label(&completions, "baz_other")
        || completion_contains_label(&completions, "Reporter::baz_other");
    assert!(
        !has_baz,
        "baz_other must not appear — 'baz_other' doesn't start with 'bar', got: {labels:?}"
    );

    Ok(())
}
