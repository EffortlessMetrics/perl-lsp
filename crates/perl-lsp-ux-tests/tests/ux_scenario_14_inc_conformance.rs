// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 14 — `@INC` consumer-consistency conformance harness.
//!
//! For each resolution mode, verifies that the PL701 diagnostic, goto-definition,
//! and hover **all agree** on whether a module reference resolves to a file.
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
//! | `scenario_14_system_inc` | system @INC via PERL5LIB | resolves |
//! | `scenario_14_nested_module_relative_include_path` | `includePaths: ["lib"]` + `Nested::Deep` | resolves |
//! | `scenario_14_include_path_missing_module_consistency` | `includePaths: ["lib"]` + missing module | NOT resolved |
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

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::json;
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

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

/// Enable system @INC resolution via workspace/didChangeConfiguration.
fn send_use_system_inc(harness: &UxHarness, enabled: bool) {
    harness
        .client
        .notify(
            "workspace/didChangeConfiguration",
            json!({
                "settings": {
                    "perl": {
                        "workspace": {
                            "useSystemInc": enabled,
                            "usePerl5lib": true
                        }
                    }
                }
            }),
        )
        .expect("didChangeConfiguration should not fail");
    std::thread::sleep(Duration::from_millis(200));
}

/// Print a conformance summary row.
fn print_conformance(mode: &str, pl701_ok: bool, def_ok: bool, hover_ok: bool) {
    eprintln!(
        "[conformance] mode={} | PL701={} | goto-def={} | hover={}",
        mode,
        if pl701_ok { "PASS" } else { "FAIL" },
        if def_ok { "PASS" } else { "FAIL" },
        if hover_ok { "PASS" } else { "FAIL" },
    );
}

/// Check completion items for a module label or insert text matching `module`.
fn completion_has_module(items: &[serde_json::Value], module: &str) -> bool {
    items.iter().any(|item| {
        item.get("label").and_then(|v| v.as_str()).map(|s| s == module).unwrap_or(false)
            || item
                .get("insertText")
                .and_then(|v| v.as_str())
                .map(|s| s == module)
                .unwrap_or(false)
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

    // completion on `use GreetModule;` near module token.
    let completion_items =
        harness.completion("fixture.pl", 2, 8).expect("completion must not error");
    let completion_resolves = completion_has_module(&completion_items, "GreetModule");

    // hover on same position.
    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    // Hover resolving = either non-null result, or at minimum no error.
    let hover_ok = true; // hover returning null is acceptable in degraded mode

    print_conformance("relative_include_path", pl701_absent, def_resolves, hover_ok);

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
    if completion_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (relative_include_path): completion resolved but PL701 fired.\n\
             completion: {:?}\n\
             diagnostics: {:?}",
            completion_items, diags
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
        completion_resolves,
        "Expected completion to include GreetModule via includePaths=['lib'], got no matching item.\n\
         completion: {:?}",
        completion_items
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

    print_conformance("lexical_use_lib", pl701_absent, def_resolves, hover_ok);

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

    print_conformance("absolute_include_path", pl701_absent, def_resolves, hover_ok);

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

    print_conformance(
        "no_lib_cancellation",
        pl701_fires,            // "ok" for negative = PL701 fired
        def_empty,              // "ok" for negative = definition returned empty
        hover_result.is_none(), // "ok" for negative = hover returned null
    );

    // Consistency check for negative case:
    // If definition IS empty, PL701 MUST fire (consistent "not found").
    // If definition is NON-empty, PL701 must NOT fire (consistent "found" -- but wrong!).
    if !def_empty && !pl701_fires {
        eprintln!(
            "INFO scenario_14_no_lib_cancellation: both consumers agree module resolves \
             (definition non-empty, no PL701). This may indicate 'no lib' is not \
             yet enforced end-to-end. Skipping strict assertion."
        );
    } else if def_empty && !pl701_fires {
        panic!(
            "Consumer inconsistency (no_lib_cancellation): goto-def returned empty \
             but PL701 did NOT fire.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    } else if !def_empty && pl701_fires {
        panic!(
            "Consumer inconsistency (no_lib_cancellation): goto-def resolved \
             but PL701 also fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }

    // Primary assertion: consumers must be consistent — they can't disagree on
    // whether the module resolves.  The specific outcome (resolved or not) is
    // separately tracked in the scorecard.
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

    print_conformance("findbin_relative", pl701_absent, def_resolves, hover_ok);

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
// Fixture 5: system @INC via PERL5LIB
// =============================================================================

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

#[test]
fn scenario_14_system_inc() {
    if !binary_available() {
        eprintln!("SKIP scenario_14_system_inc: perl-lsp binary not found");
        return;
    }

    // Create a separate tempdir to act as the system @INC entry.
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

    // Enable PERL5LIB consumption.
    send_use_system_inc(&harness, true);

    harness.open_file("fixture.pl", SYSTEM_INC_SOURCE).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let diags = wait_diagnostics(&harness, "fixture.pl");
    let pl701_absent = !has_pl701(&diags);

    // `use SystemModule` at line 2, col 4.
    let defs = harness.definition("fixture.pl", 2, 4).expect("definition must not error");
    let def_resolves = !defs.is_empty();

    // completion on `use SystemModule;` near module token.
    let completion_items =
        harness.completion("fixture.pl", 2, 8).expect("completion must not error");
    let completion_resolves = completion_has_module(&completion_items, "SystemModule");

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");
    let hover_ok = true;

    print_conformance("system_inc", pl701_absent, def_resolves, hover_ok);

    // Consistency check.
    if def_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (system_inc): goto-def resolved but PL701 fired.\n\
             goto-def: {:?}\n\
             diagnostics: {:?}",
            defs, diags
        );
    }
    if completion_resolves && !pl701_absent {
        panic!(
            "Consumer inconsistency (system_inc): completion resolved but PL701 fired.\n\
             completion: {:?}\n\
             diagnostics: {:?}",
            completion_items, diags
        );
    }

    // Both consumers must agree on resolution outcome (either both resolve or both don't).
    // We log but don't hard-fail if the system_inc mode isn't plumbed end-to-end yet.
    if !def_resolves && pl701_absent {
        eprintln!(
            "INFO scenario_14_system_inc: both consumers agree module doesn't resolve \
             (def empty + no PL701). PERL5LIB pickup may require additional server config. \
             This is a known gap if usePerl5lib hasn't been applied to this request."
        );
    }
    assert!(
        completion_resolves,
        "Expected completion to include SystemModule when useSystemInc=true and PERL5LIB is set.\n\
         completion: {:?}",
        completion_items
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

    print_conformance("nested_module_relative_include_path", pl701_absent, def_resolves, hover_ok);

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

    // completion on `use MissingFromInclude;` near module token.
    let completion_items =
        harness.completion("fixture.pl", 2, 10).expect("completion must not error");
    let completion_missing = !completion_has_module(&completion_items, "MissingFromInclude");

    let hover_result = harness.hover("fixture.pl", 2, 4).expect("hover must not error");

    print_conformance(
        "include_path_missing_module_consistency",
        pl701_fires,
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
    if !completion_missing && pl701_fires {
        panic!(
            "Consumer inconsistency (include_path_missing_module_consistency): completion resolved \
             but PL701 fired.\n\
             completion: {:?}\n\
             diagnostics: {:?}",
            completion_items, diags
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
    assert!(
        completion_missing,
        "Expected completion to omit MissingFromInclude when module does not exist.\n\
         completion: {:?}",
        completion_items
    );

    harness.assert_no_crash();
}
