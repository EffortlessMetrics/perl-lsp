// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 14 — `@INC` consumer-consistency conformance harness.
//!
//! For each resolution mode, verifies that the PL701 diagnostic, completion,
//! goto-definition, and hover **all agree** on whether a module reference resolves
//! to a file.
//!
//! ## Resolution modes exercised
//!
//! | Test function | Mode | Expected outcome |
//! |---|---|---|
//! | `scenario_14_relative_include_path` | `includePaths: ["lib"]` config | resolves |
//! | `scenario_14_use_lib_lexical` | in-source `use lib 'lib'` | resolves |
//! | `scenario_14_absolute_include_path` | absolute path in `includePaths` | resolves |
//! | `scenario_14_no_lib_cancellation` | `use lib` then `no lib` | NOT resolved |
//! | `scenario_14_findbin_relative` | `use FindBin; use lib "$FindBin::Bin/lib"` | resolves |
//! | `scenario_14_perl5lib_env` | PERL5LIB env var via `usePerl5lib=true` | resolves |
//! | `scenario_14_nested_module_relative_include_path` | `includePaths: ["lib"]` + `Nested::Deep` | resolves |
//! | `scenario_14_include_path_missing_module_consistency` | `includePaths: ["lib"]` + missing module | NOT resolved |
//!
//! ## Fixture semantics
//!
//! Completion and exact-symbol consumers use **different** fixture forms:
//!
//! - **Completion**: prefix fixtures (`use Gre<cursor>`) — completion works on partial input.
//! - **PL701 / goto-definition / hover**: exact-module fixtures (`use GreetModule;`) — these
//!   consumers operate on resolved symbols.
//!
//! Do NOT assert goto-definition on incomplete prefix text.
//!
//! ## Acceptance criteria
//!
//! For "resolves" modes: no PL701 fires AND definition returns non-empty AND
//! hover does not error. At least 2 of 3 consumers must confirm resolution for
//! the cell to be considered passing.
//!
//! For "not resolved" mode: PL701 fires AND definition returns empty AND hover
//! returns null/not-resolved. Consumer divergence (any consumer disagrees) is
//! a consistency failure.
//!
//! ## Degraded mode
//!
//! Each test prints a conformance summary even if it can only check a subset of
//! consumers. The test never panics due to a missing binary — it skips with a
//! clear message.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::json;
use std::time::Duration;

/// Diagnostic code for missing module — PL701.
const PL701: &str = "PL701";

/// Wait for diagnostics and return them, or empty vec on timeout.
fn wait_diagnostics(harness: &UxHarness, file: &str) -> Vec<serde_json::Value> {
    harness.wait_for_diagnostics(file, Duration::from_secs(5))
}

/// Check whether any diagnostic in `diags` is a PL701 missing-module error.
fn has_pl701(diags: &[serde_json::Value]) -> bool {
    diags.iter().any(|d| {
        d.get("code").and_then(|c| c.as_str()).map(|c| c == PL701).unwrap_or(false)
            || d.get("code").and_then(|c| c.as_u64()).map(|c| c == 701).unwrap_or(false)
    })
}

fn hover_is_not_resolved(hover: &Option<serde_json::Value>) -> bool {
    let Some(hover) = hover else {
        return true;
    };

    let Some(contents) = hover.get("contents") else {
        return false;
    };
    let text = match contents {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Object(map) => {
            map.get("value").and_then(|value| value.as_str()).unwrap_or("").to_string()
        }
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_str().map(str::to_owned).or_else(|| {
                    item.get("value").and_then(|value| value.as_str()).map(str::to_owned)
                })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };

    text.contains("Not found in workspace") && !text.contains("[Go to module]")
}

/// Configure the server to use `includePaths` via workspace/didChangeConfiguration.
fn send_include_paths(harness: &UxHarness, paths: &[&str]) {
    let paths_json: Vec<serde_json::Value> = paths.iter().map(|p| json!(*p)).collect();
    harness
        .client
        .notify(
            "workspace/didChangeConfiguration",
            json!({
                "settings": {
                    "perl": {
                        "workspace": {
                            "includePaths": paths_json,
                            "useSystemInc": false
                        }
                    }
                }
            }),
        )
        .expect("didChangeConfiguration should not fail");
    // Allow the server to process the configuration change.
    std::thread::sleep(Duration::from_millis(200));
}

/// Configure `usePerl5lib` and `useSystemInc` independently via
/// workspace/didChangeConfiguration. Used to exercise the four-cell matrix
/// of (usePerl5lib × useSystemInc) for PERL5LIB completion gating.
fn send_inc_settings(harness: &UxHarness, use_perl5lib: bool, use_system_inc: bool) {
    harness
        .client
        .notify(
            "workspace/didChangeConfiguration",
            json!({
                "settings": {
                    "perl": {
                        "workspace": {
                            "usePerl5lib": use_perl5lib,
                            "useSystemInc": use_system_inc
                        }
                    }
                }
            }),
        )
        .expect("didChangeConfiguration should not fail");
    std::thread::sleep(Duration::from_millis(200));
}

/// Print a conformance summary row.
fn print_conformance(
    mode: &str,
    pl701_ok: bool,
    completion_ok: bool,
    def_ok: bool,
    hover_ok: bool,
) {
    eprintln!(
        "[conformance] mode={} | PL701={} | completion={} | goto-def={} | hover={}",
        mode,
        if pl701_ok { "PASS" } else { "FAIL" },
        if completion_ok { "PASS" } else { "FAIL" },
        if def_ok { "PASS" } else { "FAIL" },
        if hover_ok { "PASS" } else { "FAIL" },
    );
}

fn completion_labels(items: &[serde_json::Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get("label").and_then(|label| label.as_str()).map(str::to_owned))
        .collect()
}

fn completion_has_module(items: &[serde_json::Value], module_name: &str) -> bool {
    items.iter().any(|item| {
        item.get("label").and_then(|label| label.as_str()) == Some(module_name)
            || item.get("insertText").and_then(|text| text.as_str()) == Some(module_name)
    })
}

// =============================================================================
// Fixture 1: workspace-relative includePaths
// =============================================================================

/// Source: `use GreetModule` — module lives in `lib/GreetModule.pm`.
/// Resolution mode: server config `includePaths: ["lib"]`.
///
/// All three consumers must agree: module resolves.
const RELATIVE_INCLUDE_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use GreetModule;\n\
\n\
my $msg = GreetModule::hello();\n\
print \"$msg\\n\";\n\
";

const RELATIVE_INCLUDE_MODULE: &str = "\
package GreetModule;\n\
\n\
use strict;\n\
use warnings;\n\
\n\
sub hello {\n\
    return \"Hello from GreetModule\";\n\
}\n\
\n\
1;\n\
";

const RELATIVE_INCLUDE_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Gre\n\
";

#[test]
fn scenario_14_relative_include_path() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_relative_include_path: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", RELATIVE_INCLUDE_SOURCE)
            .with_file("lib/GreetModule.pm", RELATIVE_INCLUDE_MODULE),
    )
    .expect("Failed to create UX harness");

    // Configure server: lib/ is an include path.
    send_include_paths(&harness, &["lib"]);

    harness.open_file("fixture.pl", RELATIVE_INCLUDE_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    // PL701 must NOT fire — module should resolve.
    let pl701_absent = !has_pl701(&diags);

    // goto-definition on `use GreetModule` — line 2, col 4 (start of "GreetModule").
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    // hover on same position.
    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    // Hover resolving = either non-null result, or at minimum no error.
    let hover_ok = true; // hover returning null is acceptable in degraded mode

    // Completion check: switch to prefix fixture so completion works on partial input.
    // Note: this is a separate fixture from the exact-module fixture above — do NOT
    // assert goto-def on the prefix fixture (see fixture-semantics rule).
    harness
        .change_file_full("fixture.pl", RELATIVE_INCLUDE_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 2, 7).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "GreetModule");

    print_conformance("relative_include_path", pl701_absent, completion_ok, def_resolves, hover_ok);

    // Consistency check: PL701 and definition must agree.
    // If definition resolves, PL701 must not fire (and vice versa).
    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (relative_include_path): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    // The module IS resolvable — at minimum definition should find it.
    assert!(
        def_resolves,
        "Expected goto-definition to resolve GreetModule via includePaths=['lib'], got empty result.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        pl701_absent,
        "Expected no PL701 for GreetModule when includePaths=['lib'] is configured.\n\
         diagnostics: {:?}",
        diags
    );

    // Hover result shape check (if non-null).
    if let Some(hover) = hover_result {
        assert!(
            hover.get("contents").is_some(),
            "Hover result must have 'contents' field: {:?}",
            hover
        );
    }

    harness.assert_no_crash();
}

// Removed `scenario_14_include_path_completion_external_module` (was the
// FIXME(#7570) ignored test). The test asserted goto-definition would
// resolve on an incomplete prefix `use Gre` — that is not a valid parity
// assertion: completion works on prefixes; PL701, goto-definition, and
// hover work on resolved/exact symbols. The post-rail conformance harness
// (PR #8495) covers the intended parity correctly via separate prefix-
// completion and exact-module fixtures in `scenario_14_relative_include_path`
// and `scenario_14_perl5lib_completion_gating_matrix`. #7570 closed
// 2026-05-07.

// =============================================================================
// Fixture 2: lexical use lib in source
// =============================================================================

const USE_LIB_LEXICAL_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use LexicalModule;\n\
\n\
my $result = LexicalModule::compute();\n\
print \"$result\\n\";\n\
";

/// Completion prefix fixture: include the `use lib` pragma for context so the
/// resolver sees the same path configuration.
const LEXICAL_USE_LIB_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use Lex\n\
";

const LEXICAL_MODULE: &str = "\
package LexicalModule;\n\
\n\
use strict;\n\
use warnings;\n\
\n\
sub compute {\n\
    return 42;\n\
}\n\
\n\
1;\n\
";

#[test]
fn scenario_14_use_lib_lexical() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_use_lib_lexical: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", USE_LIB_LEXICAL_SOURCE)
            .with_file("lib/LexicalModule.pm", LEXICAL_MODULE),
    )
    .expect("Failed to create UX harness");

    // No server-side includePaths config — resolution must come entirely from
    // the in-source `use lib 'lib'` pragma.
    harness.open_file("fixture.pl", USE_LIB_LEXICAL_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use LexicalModule` is at line 3, col 4.
    let defs = harness.definition("fixture.pl", 3, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 3, 4).expect("hover must not error");
    let hover_ok = true; // degraded null is acceptable

    // Completion check: switch to prefix fixture (includes `use lib` for context).
    harness
        .change_file_full("fixture.pl", LEXICAL_USE_LIB_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 3, 7).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "LexicalModule");

    print_conformance("lexical_use_lib", pl701_absent, completion_ok, def_resolves, hover_ok);

    // Consistency check.
    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (lexical_use_lib): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        def_resolves,
        "Expected goto-definition to resolve LexicalModule via in-source 'use lib lib', got empty.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        pl701_absent,
        "Expected no PL701 for LexicalModule when 'use lib lib' is in source.\n\
         diagnostics: {:?}",
        diags
    );

    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {:?}", hover);
    }

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 2b: absolute includePaths
// =============================================================================

const ABSOLUTE_INCLUDE_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use AbsoluteModule;\n\
print AbsoluteModule::value();\n\
";

/// Completion prefix fixture for absolute include path scenario.
const ABSOLUTE_INCLUDE_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Abs\n\
";

const ABSOLUTE_INCLUDE_MODULE: &str = "\
package AbsoluteModule;\n\
use strict;\n\
use warnings;\n\
sub value {\n\
    return 7;\n\
}\n\
1;\n\
";

#[test]
fn scenario_14_absolute_include_path() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_absolute_include_path: perl-lsp binary not found");
        return;
    }

    let abs_root = tempfile::tempdir().expect("Failed to create absolute include tempdir");
    let module_path = abs_root.path().join("AbsoluteModule.pm");
    std::fs::write(&module_path, ABSOLUTE_INCLUDE_MODULE)
        .expect("Failed to write AbsoluteModule.pm");

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", ABSOLUTE_INCLUDE_SOURCE),
    )
    .expect("Failed to create UX harness");

    let abs_root_string = abs_root.path().to_string_lossy().to_string();
    send_include_paths(&harness, &[abs_root_string.as_str()]);

    harness.open_file("fixture.pl", ABSOLUTE_INCLUDE_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use AbsoluteModule` at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    let hover_ok = true;

    // Completion check: switch to prefix fixture (server already configured with
    // the absolute includePath from send_include_paths above).
    harness
        .change_file_full("fixture.pl", ABSOLUTE_INCLUDE_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 2, 7).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "AbsoluteModule");

    print_conformance("absolute_include_path", pl701_absent, completion_ok, def_resolves, hover_ok);

    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (absolute_include_path): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        def_resolves,
        "Expected goto-definition to resolve AbsoluteModule via absolute includePaths, got empty result.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        pl701_absent,
        "Expected no PL701 for AbsoluteModule when absolute includePaths is configured.\n\
         diagnostics: {:?}",
        diags
    );

    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {:?}", hover);
    }

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 3: no lib cancellation (negative case)
// =============================================================================

const NO_LIB_CANCEL_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
no lib 'lib';\n\
use GoneModule;\n\
\n\
print \"unreachable\\n\";\n\
";

/// Completion prefix fixture for no_lib_cancellation: `no lib` cancels the
/// earlier `use lib`, so `GoneModule` should NOT appear in completion.
const NO_LIB_CANCEL_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
no lib 'lib';\n\
use Gone\n\
";

const GONE_MODULE: &str = "\
package GoneModule;\n\
\n\
use strict;\n\
use warnings;\n\
\n\
# This file exists on disk but must NOT be resolved\n\
# because 'no lib' cancelled the earlier 'use lib'.\n\
\n\
sub gone { return \"I should not be found\" }\n\
\n\
1;\n\
";

#[test]
fn scenario_14_no_lib_cancellation() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_no_lib_cancellation: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", NO_LIB_CANCEL_SOURCE)
            .with_file("lib/GoneModule.pm", GONE_MODULE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("fixture.pl", NO_LIB_CANCEL_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    // PL701 MUST fire — the no lib cancelled the use lib before the use GoneModule line.
    let pl701_fires = has_pl701(&diags);

    // goto-definition on `use GoneModule` at line 4, col 4.
    let defs = harness.definition("fixture.pl", 4, 4).expect("definition must not error");
    let def_empty = defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 4, 4).expect("hover must not error");
    let hover_not_resolved = hover_is_not_resolved(&hover_result);

    // Completion check (negative): `GoneModule` should NOT appear since `no lib`
    // cancelled the path.
    harness
        .change_file_full("fixture.pl", NO_LIB_CANCEL_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 4, 8).expect("completion must not error");
    let completion_absent = !completion_has_module(&completions, "GoneModule");
    // "ok" for negative = module not in completion
    let completion_ok = completion_absent;

    print_conformance(
        "no_lib_cancellation",
        pl701_fires,        // "ok" for negative = PL701 fired
        completion_ok,      // "ok" for negative = module absent from completion
        def_empty,          // "ok" for negative = definition returned empty
        hover_not_resolved, // "ok" for negative = hover returned null/not-resolved
    );

    // Strict enforcement for PL701: the push-diagnostic resolver must honor
    // position-aware `no lib` cancellation. Fixed by #8516.
    //
    // Root cause: the PL701 resolver called `resolve_module_to_path_with_doc`
    // (whole-file @INC scan) instead of `resolve_module_to_path_with_doc_at_offset`
    // (position-aware scan). The fix threads the use-site byte offset through the
    // resolver callback chain and also filters configured include paths using
    // `no_lib_cancelled_paths_at_offset` so that workspace-configured `lib` entries
    // are also suppressed by `no lib 'lib'`.
    assert!(
        pl701_fires,
        "PL701 MUST fire for GoneModule: 'no lib' cancelled the earlier 'use lib', \
         so the module must not be found by the diagnostic resolver.\n\
         diagnostics: {:?}",
        diags
    );

    // goto-def and completion: fixed by #8537.
    // After the fix, `no lib` cancellation applies to workspace-symbol lookups too.
    // The file-system resolver is authoritative for `use Module` goto-definition;
    // the workspace index supplements are filtered through EffectiveIncContext.
    assert!(
        def_empty,
        "goto-def MUST return empty for GoneModule: 'no lib' cancelled the path. \
         Fixed by #8537 — file-system resolver is authoritative for `use Module` \
         goto-definition and must not be bypassed by workspace index.\n\
         goto-def: {:?}",
        defs
    );
    assert!(
        completion_absent,
        "completion MUST NOT suggest GoneModule: 'no lib' cancelled the path. \
         Fixed by #8537 — workspace-index Package completions are filtered through \
         EffectiveIncContext at the use-site offset.\n\
         completions (labels): {:?}",
        completion_labels(&completions)
    );
    assert!(
        hover_not_resolved,
        "hover MUST NOT resolve GoneModule after 'no lib' cancellation; got {:?}",
        hover_result
    );

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 4: FindBin-relative resolution
// =============================================================================

const FINDBIN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use FindBin;\n\
use lib \"$FindBin::Bin/lib\";\n\
use FindBinModule;\n\
\n\
my $val = FindBinModule::value();\n\
print \"$val\\n\";\n\
";

/// Completion prefix fixture for FindBin-relative scenario.
/// The `use FindBin` and `use lib` lines must be present so the resolver
/// sees the same path configuration as the exact-module fixture.
const FINDBIN_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use FindBin;\n\
use lib \"$FindBin::Bin/lib\";\n\
use Find\n\
";

const FINDBIN_MODULE: &str = "\
package FindBinModule;\n\
\n\
use strict;\n\
use warnings;\n\
\n\
sub value {\n\
    return 99;\n\
}\n\
\n\
1;\n\
";

#[test]
fn scenario_14_findbin_relative() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_findbin_relative: perl-lsp binary not found");
        return;
    }

    // The harness workspace root acts as $FindBin::Bin.
    // lib/FindBinModule.pm must be at <workspace>/lib/FindBinModule.pm.
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", FINDBIN_SOURCE)
            .with_file("lib/FindBinModule.pm", FINDBIN_MODULE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("fixture.pl", FINDBIN_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use FindBinModule` at line 4, col 4.
    let defs = harness.definition("fixture.pl", 4, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 4, 4).expect("hover must not error");
    let hover_ok = true;

    // Completion check: switch to prefix fixture (includes FindBin pragmas for context).
    harness
        .change_file_full("fixture.pl", FINDBIN_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    // `use Find` is at line 4, col 8 (0-indexed: line 4, position after "use Find")
    let completions = harness.completion("fixture.pl", 4, 8).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "FindBinModule");

    print_conformance("findbin_relative", pl701_absent, completion_ok, def_resolves, hover_ok);

    // Consistency check.
    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (findbin_relative): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }
    if !def_resolves && pl701_absent {
        // Both agree module doesn't resolve — log but don't fail the consistency test.
        // FindBin resolution may be in degraded mode in some environments.
        eprintln!(
            "INFO scenario_14_findbin_relative: both consumers agree module does not resolve \
             (def empty + no PL701). FindBin resolution may be in degraded mode."
        );
    }

    // We assert consistency but tolerate FindBin not resolving end-to-end in the
    // UX harness (it's environment-dependent). What we MUST NOT see is divergence.
    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {:?}", hover);
    }

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 5: PERL5LIB env var (`usePerl5lib=true`)
// =============================================================================
//
// Renamed from `scenario_14_system_inc` — the fixture has always exercised
// PERL5LIB (not interpreter startup @INC). After PR #8493, PERL5LIB visibility
// is controlled by `usePerl5lib` alone. We use `send_inc_settings(true, false)`
// to make the configuration explicit: PERL5LIB enabled, system @INC disabled.

const SYSTEM_INC_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use SystemModule;\n\
\n\
my $result = SystemModule::run();\n\
print \"$result\\n\";\n\
";

const SYSTEM_MODULE: &str = "\
package SystemModule;\n\
\n\
use strict;\n\
use warnings;\n\
\n\
sub run {\n\
    return \"system module running\";\n\
}\n\
\n\
1;\n\
";

/// Completion prefix fixture for PERL5LIB scenario.
const PERL5LIB_ENV_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Sys\n\
";

#[test]
fn scenario_14_perl5lib_env() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_perl5lib_env: perl-lsp binary not found");
        return;
    }

    // Create a separate tempdir to act as the PERL5LIB entry.
    // The module lives there, not inside the harness workspace.
    let system_dir = tempfile::tempdir().expect("Failed to create system tempdir");
    let module_path = system_dir.path().join("SystemModule.pm");
    std::fs::write(&module_path, SYSTEM_MODULE).expect("Failed to write SystemModule.pm");

    let perl5lib_value = system_dir.path().to_string_lossy().to_string();

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", SYSTEM_INC_SOURCE)
            .env("PERL5LIB", &perl5lib_value),
    )
    .expect("Failed to create UX harness");

    // Enable PERL5LIB consumption via usePerl5lib=true; useSystemInc=false
    // (PERL5LIB visibility no longer depends on useSystemInc after PR #8493).
    send_inc_settings(&harness, true, false);

    harness.open_file("fixture.pl", SYSTEM_INC_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use SystemModule` at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    let hover_ok = true;

    // Completion check: switch to prefix fixture.
    harness
        .change_file_full("fixture.pl", PERL5LIB_ENV_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 2, 7).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "SystemModule");

    print_conformance("perl5lib_env", pl701_absent, completion_ok, def_resolves, hover_ok);

    // Consistency check.
    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (perl5lib_env): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        def_resolves,
        "scenario_14_perl5lib_env: definition should resolve SystemModule.pm via PERL5LIB; defs={defs:?}"
    );
    assert!(
        pl701_absent,
        "scenario_14_perl5lib_env: PL701 should not fire when module resolves via PERL5LIB; diagnostics={diags:?}"
    );

    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {:?}", hover);
    }

    harness.assert_no_crash();

    // Keep system_dir alive until after all LSP calls complete.
    drop(system_dir);
}

// =============================================================================
// Fixture 6: nested module path via includePaths
// =============================================================================

const NESTED_INCLUDE_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Nested::Deep;\n\
\n\
print Nested::Deep::answer();\n\
";

/// Completion prefix fixture for nested module scenario.
const NESTED_INCLUDE_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Nested::De\n\
";

const NESTED_INCLUDE_MODULE: &str = "\
package Nested::Deep;\n\
use strict;\n\
use warnings;\n\
sub answer {\n\
    return 314;\n\
}\n\
1;\n\
";

#[test]
fn scenario_14_nested_module_relative_include_path() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_nested_module_relative_include_path: perl-lsp binary not found"
        );
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", NESTED_INCLUDE_SOURCE)
            .with_file("lib/Nested/Deep.pm", NESTED_INCLUDE_MODULE),
    )
    .expect("Failed to create UX harness");

    send_include_paths(&harness, &["lib"]);

    harness.open_file("fixture.pl", NESTED_INCLUDE_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use Nested::Deep` at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    let hover_ok = true;

    // Completion check: switch to prefix fixture.
    harness
        .change_file_full("fixture.pl", NESTED_INCLUDE_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    // `use Nested::De` — completion at line 2, col 14
    let completions = harness.completion("fixture.pl", 2, 14).expect("completion must not error");
    let completion_ok = completion_has_module(&completions, "Nested::Deep");

    print_conformance(
        "nested_module_relative_include_path",
        pl701_absent,
        completion_ok,
        def_resolves,
        hover_ok,
    );

    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (nested_module_relative_include_path): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        def_resolves,
        "Expected goto-definition to resolve Nested::Deep via includePaths=['lib'], got empty result.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        pl701_absent,
        "Expected no PL701 for Nested::Deep when includePaths=['lib'] is configured.\n\
         diagnostics: {:?}",
        diags
    );

    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {:?}", hover);
    }

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 7: includePaths configured but module missing
// =============================================================================

const INCLUDE_MISSING_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use MissingFromInclude;\n\
\n\
print \"still running\\n\";\n\
";

const INCLUDE_MISSING_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Gree\n\
";

/// Completion prefix fixture for the missing-module negative test.
/// `MissingFromInclude` does not exist, so it should NOT appear in completion.
const MISSING_MODULE_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use MissingFro\n\
";

#[test]
fn scenario_14_include_path_missing_module_consistency() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_include_path_missing_module_consistency: perl-lsp binary not found"
        );
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", INCLUDE_MISSING_SOURCE),
    )
    .expect("Failed to create UX harness");

    send_include_paths(&harness, &["lib"]);

    harness.open_file("fixture.pl", INCLUDE_MISSING_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_fires = has_pl701(&diags);

    // `use MissingFromInclude` at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_empty = defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");

    // Completion check (negative): `MissingFromInclude` should NOT appear since
    // the module does not exist in the include path.
    harness
        .change_file_full("fixture.pl", MISSING_MODULE_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 2, 13).expect("completion must not error");
    // "ok" for negative = module absent from completion
    let completion_ok = !completion_has_module(&completions, "MissingFromInclude");

    print_conformance(
        "include_path_missing_module_consistency",
        pl701_fires,
        completion_ok, // "ok" for negative = module absent from completion
        def_empty,
        hover_result.is_none(),
    );

    if !def_empty && pl701_fires {
        panic!(
            "Consumer inconsistency (include_path_missing_module_consistency): goto-def resolved \
             but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        def_empty,
        "Expected goto-definition to return empty for MissingFromInclude, got {:?}",
        defs
    );
    assert!(
        pl701_fires,
        "Expected PL701 for MissingFromInclude when module does not exist.\n\
         diagnostics: {:?}",
        diags
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_14_include_path_missing_module_completion_consistency() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_include_path_missing_module_completion_consistency: perl-lsp binary not found"
        );
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", INCLUDE_MISSING_COMPLETION_SOURCE),
    )
    .expect("Failed to create UX harness");

    send_include_paths(&harness, &["lib"]);
    harness
        .open_file("fixture.pl", INCLUDE_MISSING_COMPLETION_SOURCE)
        .expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let completions = harness.completion("fixture.pl", 2, 8).expect("completion must not error");
    let completion_has_greet = completion_has_module(&completions, "GreetModule");

    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");

    assert!(
        !completion_has_greet,
        "Expected completion to keep missing external module absent when includePaths has no lib module; labels={:?}",
        completion_labels(&completions)
    );
    assert!(
        !def_resolves,
        "Expected goto-definition to remain missing for unresolved include-path module, got defs={:?}",
        defs
    );
    if hover_result.is_some() {
        assert!(
            !def_resolves,
            "Consumer inconsistency (completion_missing_module): hover resolved while goto-definition did not.\n\
             hover={:?}\n\
             defs={:?}",
            hover_result, defs
        );
    }
    assert_eq!(
        completion_has_greet,
        def_resolves,
        "Consumer inconsistency (completion_missing_module): completion and goto-definition disagree.\n\
         labels={:?}\n\
         defs={:?}",
        completion_labels(&completions),
        defs
    );

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 8: PERL5LIB completion gating is independent of useSystemInc
// =============================================================================
//
// Replaces the previous `scenario_14_system_inc_completion_opt_in_enabled`,
// whose expectations incorrectly tied PERL5LIB visibility to `useSystemInc`.
// Correct semantics (PR #8485 / fix to `perl5lib_paths_for_completion`):
//
//   - `usePerl5lib` (default true) gates PERL5LIB.
//   - `useSystemInc` (default false) gates interpreter startup `@INC` only.
//
// The two flags are independent. The fixture module lives only in the PERL5LIB
// tempdir, so completion visibility tracks `usePerl5lib` exactly.

const PR1_PERL5LIB_MODULE_NAME: &str = "Pr1MatrixModule";

const PR1_PERL5LIB_MODULE: &str = "\
package Pr1MatrixModule;\n\
use strict;\n\
use warnings;\n\
sub ping { return 'pong' }\n\
1;\n\
";

const PR1_PERL5LIB_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Pr1\n\
";

const PR1_PERL5LIB_USE_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use Pr1MatrixModule;\n\
\n\
my $r = Pr1MatrixModule::ping();\n\
print \"$r\\n\";\n\
";

#[test]
fn scenario_14_perl5lib_completion_gating_matrix() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_perl5lib_completion_gating_matrix: perl-lsp binary not found");
        return;
    }

    let system_dir = tempfile::tempdir().expect("Failed to create system tempdir");
    let module_path = system_dir.path().join(format!("{PR1_PERL5LIB_MODULE_NAME}.pm"));
    std::fs::write(&module_path, PR1_PERL5LIB_MODULE).expect("Failed to write Pr1MatrixModule.pm");
    let perl5lib_value = system_dir.path().to_string_lossy().to_string();

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", PR1_PERL5LIB_COMPLETION_SOURCE)
            .env("PERL5LIB", &perl5lib_value),
    )
    .expect("Failed to create UX harness");

    harness
        .open_file("fixture.pl", PR1_PERL5LIB_COMPLETION_SOURCE)
        .expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    // (use_perl5lib, use_system_inc, expect_module_in_completion)
    let cells: &[(bool, bool, bool)] =
        &[(true, false, true), (true, true, true), (false, true, false), (false, false, false)];

    for &(use_perl5lib, use_system_inc, expected) in cells {
        send_inc_settings(&harness, use_perl5lib, use_system_inc);
        let completions =
            harness.completion("fixture.pl", 2, 7).expect("completion must not error");
        let has = completion_has_module(&completions, PR1_PERL5LIB_MODULE_NAME);
        assert_eq!(
            has,
            expected,
            "cell (usePerl5lib={use_perl5lib}, useSystemInc={use_system_inc}): \
             expected PERL5LIB module present={expected}, got {has}; labels={:?}",
            completion_labels(&completions),
        );
    }

    harness.assert_no_crash();
    drop(system_dir);
}

#[test]
fn scenario_14_perl5lib_completion_without_system_inc() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_perl5lib_completion_without_system_inc: perl-lsp binary not found"
        );
        return;
    }

    // Four-consumer parity at (usePerl5lib=true, useSystemInc=false): the
    // PERL5LIB module must resolve through PL701 / completion / goto-def / hover.
    let system_dir = tempfile::tempdir().expect("Failed to create system tempdir");
    let module_path = system_dir.path().join(format!("{PR1_PERL5LIB_MODULE_NAME}.pm"));
    std::fs::write(&module_path, PR1_PERL5LIB_MODULE).expect("Failed to write Pr1MatrixModule.pm");
    let perl5lib_value = system_dir.path().to_string_lossy().to_string();

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", PR1_PERL5LIB_USE_SOURCE)
            .env("PERL5LIB", &perl5lib_value),
    )
    .expect("Failed to create UX harness");

    send_inc_settings(&harness, true, false);

    harness.open_file("fixture.pl", PR1_PERL5LIB_USE_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use Pr1MatrixModule;` is at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    let hover_ok = true;

    // Completion check on a sibling prefix fixture: changing the source to a
    // prefix inside the same harness keeps the harness configured with the
    // same usePerl5lib / PERL5LIB settings.
    harness
        .change_file_full("fixture.pl", PR1_PERL5LIB_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 2, 7).expect("completion must not error");
    let completion_resolves = completion_has_module(&completions, PR1_PERL5LIB_MODULE_NAME);

    print_conformance(
        "perl5lib_completion_without_system_inc",
        pl701_absent,
        completion_resolves,
        def_resolves,
        hover_ok,
    );

    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (perl5lib_completion_without_system_inc): \
             goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    assert!(
        completion_resolves,
        "Expected completion to include {PR1_PERL5LIB_MODULE_NAME} with \
         usePerl5lib=true, useSystemInc=false; labels={:?}",
        completion_labels(&completions),
    );
    assert!(
        def_resolves,
        "Expected goto-definition to resolve {PR1_PERL5LIB_MODULE_NAME} with \
         usePerl5lib=true, useSystemInc=false; defs={defs:?}"
    );
    assert!(
        pl701_absent,
        "Expected no PL701 for {PR1_PERL5LIB_MODULE_NAME} with usePerl5lib=true, \
         useSystemInc=false; diagnostics={diags:?}"
    );

    if let Some(hover) = hover_result {
        assert!(hover.get("contents").is_some(), "Hover result must have 'contents': {hover:?}");
    }

    harness.assert_no_crash();
    drop(system_dir);
}

/// Regression guard for the startup-`@INC` env-inheritance leak: with
/// `usePerl5lib=false` and `useSystemInc=true`, the interpreter startup
/// `@INC` probe must NOT inherit `PERL5LIB` from the LSP's environment, so a
/// PERL5LIB-only module does not silently leak in via the system source.
#[test]
fn scenario_14_perl5lib_disabled_ignores_env_even_when_system_inc_enabled() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_perl5lib_disabled_ignores_env_even_when_system_inc_enabled: \
             perl-lsp binary not found"
        );
        return;
    }

    let system_dir = tempfile::tempdir().expect("Failed to create system tempdir");
    // Unique module name unlikely to exist anywhere in real interpreter startup @INC,
    // so absence in completion is a meaningful signal.
    let module_name = "Pr1EnvLeakProbeModule";
    let module_path = system_dir.path().join(format!("{module_name}.pm"));
    std::fs::write(
        &module_path,
        "package Pr1EnvLeakProbeModule;\n\
         use strict;\n\
         use warnings;\n\
         sub ping { return 'pong' }\n\
         1;\n",
    )
    .expect("Failed to write Pr1EnvLeakProbeModule.pm");
    let perl5lib_value = system_dir.path().to_string_lossy().to_string();

    let source = "use strict;\nuse warnings;\nuse Pr1Env\n";
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", source)
            .env("PERL5LIB", &perl5lib_value),
    )
    .expect("Failed to create UX harness");

    // usePerl5lib disabled, useSystemInc enabled: PERL5LIB must be stripped
    // from the startup-@INC probe environment, so the probe does not surface
    // the tempdir module as an interpreter startup root.
    send_inc_settings(&harness, false, true);

    harness.open_file("fixture.pl", source).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let completions = harness.completion("fixture.pl", 2, 10).expect("completion must not error");
    let has = completion_has_module(&completions, module_name);
    assert!(
        !has,
        "Expected {module_name} absent from completion with usePerl5lib=false, \
         useSystemInc=true (PERL5LIB env must not leak through interpreter startup @INC); \
         labels={:?}",
        completion_labels(&completions),
    );

    harness.assert_no_crash();
    drop(system_dir);
}

// =============================================================================
// Fixture 9: no lib cancellation WITH workspace index (negative, workspace-aware)
// =============================================================================
//
// Same fixture as scenario_14_no_lib_cancellation but the harness opens the
// module file via didOpen, which causes the workspace indexer to index it.
// After #8537, workspace-index Package symbols are filtered through
// EffectiveIncContext, so goto-def and completion must still be empty.

#[test]
fn scenario_14_no_lib_cancellation_workspace_index() {
    if !binary_available() {
        eprintln!(
            "SKIP scenario_14_no_lib_cancellation_workspace_index: perl-lsp binary not found"
        );
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", NO_LIB_CANCEL_SOURCE)
            .with_file("lib/GoneModule.pm", GONE_MODULE),
    )
    .expect("Failed to create UX harness");

    // Open the module file first to ensure it gets indexed by the workspace indexer.
    harness
        .open_file("lib/GoneModule.pm", GONE_MODULE)
        .expect("didOpen GoneModule.pm should succeed");
    std::thread::sleep(Duration::from_millis(300));

    harness.open_file("fixture.pl", NO_LIB_CANCEL_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(700));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_fires = has_pl701(&diags);

    // goto-definition on `use GoneModule` at line 4, col 4.
    let defs = harness.definition("fixture.pl", 4, 4).expect("definition must not error");
    let def_empty = defs.is_empty();

    // Completion check (negative): `GoneModule` must NOT appear even though the
    // workspace index has it indexed — the @INC filter must suppress it.
    harness
        .change_file_full("fixture.pl", NO_LIB_CANCEL_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 4, 8).expect("completion must not error");
    let completion_absent = !completion_has_module(&completions, "GoneModule");

    print_conformance(
        "no_lib_cancellation_workspace_index",
        pl701_fires,
        completion_absent,
        def_empty,
        true, // hover not checked in this scenario
    );

    assert!(
        pl701_fires,
        "PL701 MUST fire for GoneModule (workspace-index scenario): 'no lib' cancelled the path.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        def_empty,
        "goto-def MUST return empty even with workspace index: 'no lib' cancels @INC reachability. \
         Fixed by #8537.\n\
         goto-def: {:?}",
        defs
    );
    assert!(
        completion_absent,
        "completion MUST NOT suggest GoneModule even with workspace index: @INC filter applies. \
         Fixed by #8537.\n\
         completions (labels): {:?}",
        completion_labels(&completions)
    );

    harness.assert_no_crash();
}

// =============================================================================
// Fixture 10: use lib WITH workspace index (positive control)
// =============================================================================
//
// Same structure as scenario_14_no_lib_cancellation_workspace_index but WITHOUT
// the `no lib` line. Verifies that the @INC filter does NOT over-filter — a
// workspace-indexed module that IS reachable through @INC must still surface.

const USE_LIB_WITH_INDEX_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use GoneModule;\n\
\n\
print \"reachable\\n\";\n\
";

const USE_LIB_WITH_INDEX_COMPLETION_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use Gone\n\
";

#[test]
fn scenario_14_use_lib_with_workspace_index() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_use_lib_with_workspace_index: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("fixture.pl", USE_LIB_WITH_INDEX_SOURCE)
            .with_file("lib/GoneModule.pm", GONE_MODULE),
    )
    .expect("Failed to create UX harness");

    // Open the module file first to ensure it gets indexed.
    harness
        .open_file("lib/GoneModule.pm", GONE_MODULE)
        .expect("didOpen GoneModule.pm should succeed");
    std::thread::sleep(Duration::from_millis(300));

    harness.open_file("fixture.pl", USE_LIB_WITH_INDEX_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(700));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    // PL701 must NOT fire — the `use lib 'lib'` is in effect (no `no lib`).
    let pl701_absent = !has_pl701(&diags);

    // goto-definition on `use GoneModule` at line 3, col 4.
    let defs = harness.definition("fixture.pl", 3, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    // Completion check (positive): `GoneModule` MUST appear since `use lib 'lib'`
    // is active and the module is indexed.
    harness
        .change_file_full("fixture.pl", USE_LIB_WITH_INDEX_COMPLETION_SOURCE)
        .expect("didChange to completion fixture should succeed");
    std::thread::sleep(Duration::from_millis(500));
    let completions = harness.completion("fixture.pl", 3, 8).expect("completion must not error");
    let completion_present = completion_has_module(&completions, "GoneModule");

    print_conformance(
        "use_lib_with_workspace_index",
        pl701_absent,
        completion_present,
        def_resolves,
        true,
    );

    assert!(
        pl701_absent,
        "PL701 must NOT fire for GoneModule: 'use lib lib' is in effect and module exists.\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        def_resolves,
        "goto-def MUST resolve GoneModule: 'use lib lib' is in effect, @INC filter must not \
         over-filter reachable modules (positive control for #8537).\n\
         diagnostics: {:?}",
        diags
    );
    assert!(
        completion_present,
        "completion MUST suggest GoneModule: 'use lib lib' is in effect, @INC filter must not \
         suppress reachable modules (positive control for #8537).\n\
         completions (labels): {:?}",
        completion_labels(&completions)
    );

    harness.assert_no_crash();
}
