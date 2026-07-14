// Test infrastructure — allow test-friendly patterns used throughout this module.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 22 — Cross-file dogfood baseline: extended providers.
//!
//! Extends ux_scenario_21 to measure the remaining UNMEASURED providers on the
//! same RealBaseline 4-file workspace:
//!   - call hierarchy (prepareCallHierarchy / incomingCalls / outgoingCalls)
//!   - inlay hints
//!   - type hierarchy (prepareTypeHierarchy / supertypes / subtypes)
//!   - folding ranges
//!   - code lens
//!   - document highlight
//!   - selection range
//!   - formatting / range-formatting (single-file provider, multi-file context)
//!
//! This is a MEASURE-and-FILE pass — no fix PRs opened here.
//! Classification key:
//!   WORKS   → hard `assert!` regression guard (the test itself IS the guard)
//!   BROKEN  → `#[ignore]` + issue filed
//!   UNIMPL  → `#[ignore]` + issue filed
//!
//! ## Fixture layout (same as scenario_21)
//!
//! ```text
//! lib/
//!   RealBaseline/
//!     App.pm   — package RealBaseline::App; use parent 'RealBaseline::Base';
//!                use RealBaseline::Util qw(helper alias); subs: new, run, name
//!     Base.pm  — package RealBaseline::Base;  subs: shared, reset
//!     Util.pm  — package RealBaseline::Util;  subs: helper, bounce; *alias = \&helper
//! script/
//!   real-baseline.pl — use RealBaseline::App; $app = App->new; $app->run
//! ```
//!
//! ## Fixture line layout (0-indexed, for cursor positioning)
//!
//! App.pm:
//!   0: package RealBaseline::App;
//!   1: use strict;
//!   2: use warnings;
//!   3: use parent 'RealBaseline::Base';
//!   4: use RealBaseline::Util qw(helper alias);
//!   5: (blank)
//!   6: sub new {
//!   7:     my ($class, %args) = @_;
//!   8:     return bless \%args, $class;
//!   9: }
//!  10: (blank)
//!  11: sub run {
//!  12:     my ($self) = @_;
//!  13:     helper($self->name);
//!  14:     alias($self->shared);
//!  15:     return $self->shared;
//!  16: }
//!  17: (blank)
//!  18: sub name {
//!  19:     return $_[0]->{name};
//!  20: }
//!  21: (blank)
//!  22: 1;
//!
//! Base.pm:
//!   0: package RealBaseline::Base;
//!   1: use strict;
//!   2: use warnings;
//!   3: (blank)
//!   4: sub shared {
//!   5:     return 'shared';
//!   6: }
//!   7: (blank)
//!   8: sub reset {
//!   9:     return 1;
//!  10: }
//!  11: (blank)
//!  12: 1;

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::time::Duration;

// ── Fixture sources (same as scenario_21) ────────────────────────────────────

const APP_PM: &str = r#"package RealBaseline::App;
use strict;
use warnings;
use parent 'RealBaseline::Base';
use RealBaseline::Util qw(helper alias);

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub run {
    my ($self) = @_;
    helper($self->name);
    alias($self->shared);
    return $self->shared;
}

sub name {
    return $_[0]->{name};
}

1;
"#;

const BASE_PM: &str = r#"package RealBaseline::Base;
use strict;
use warnings;

sub shared {
    return 'shared';
}

sub reset {
    return 1;
}

1;
"#;

const UTIL_PM: &str = r#"package RealBaseline::Util;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(helper alias);

sub helper {
    return shift;
}

*alias = \&helper;

sub bounce {
    goto &helper;
}

1;
"#;

const SCRIPT_PL: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealBaseline::App;

my $app = RealBaseline::App->new(name => 'demo');
$app->run;
"#;

// ── Harness factory ──────────────────────────────────────────────────────────

fn create_harness() -> anyhow::Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig::default()
            .with_file("lib/RealBaseline/App.pm", APP_PM)
            .with_file("lib/RealBaseline/Base.pm", BASE_PM)
            .with_file("lib/RealBaseline/Util.pm", UTIL_PM)
            .with_file("script/real-baseline.pl", SCRIPT_PL),
    )
}

fn wait_for_incoming_calls(
    harness: &UxHarness,
    item: &Value,
    timeout: Duration,
) -> anyhow::Result<Vec<Value>> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let incoming_resp = harness.client.request(
            "callHierarchy/incomingCalls",
            json!({ "item": item }),
            Duration::from_secs(5),
        )?;

        if incoming_resp.get("error").is_some() {
            return Err(anyhow::anyhow!(
                "incomingCalls returned JSON-RPC error: {:?}",
                incoming_resp["error"]
            ));
        }

        let calls = incoming_resp["result"].as_array().cloned().ok_or_else(|| {
            anyhow::anyhow!("incomingCalls result must be array: {:?}", incoming_resp["result"])
        })?;
        if !calls.is_empty() || std::time::Instant::now() >= deadline {
            return Ok(calls);
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 7: textDocument/prepareCallHierarchy + callHierarchy/incomingCalls
//              + callHierarchy/outgoingCalls
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — prepareCallHierarchy on `sub run` in App.pm (line 11, col 4).
///
/// MEASURE: does `textDocument/prepareCallHierarchy` return a CallHierarchyItem
/// for `run`? Hierarchy items must have name, kind, uri, range, selectionRange.
#[test]
fn scenario_22_prepare_call_hierarchy_on_run_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    // App.pm line 11 (0-indexed): `sub run {`  — cursor at col 4 inside `run`.
    let resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: prepareCallHierarchy/run-in-app-pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!("status: prepareCallHierarchy/run: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!("status: prepareCallHierarchy/run: BROKEN — returned null (no items)");
    } else if let Some(items) = resp["result"].as_array() {
        if items.is_empty() {
            eprintln!("status: prepareCallHierarchy/run: BROKEN — returned empty array");
        } else {
            let item = &items[0];
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>");
            eprintln!(
                "status: prepareCallHierarchy/run: WORKS — item name={name:?}, \
                 has uri={}, has range={}",
                item.get("uri").is_some(),
                item.get("range").is_some()
            );
        }
    } else {
        eprintln!(
            "status: prepareCallHierarchy/run: BROKEN — result is not an array: {:?}",
            resp["result"]
        );
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — prepareCallHierarchy on `sub run` must return at least one
/// CallHierarchyItem with the correct name and required fields.
#[test]
fn scenario_22_prepare_call_hierarchy_on_run_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "prepareCallHierarchy must not return a JSON-RPC error: {:?}",
        resp.get("error")
    );
    assert!(!resp["result"].is_null(), "prepareCallHierarchy must return a result, got null");

    let items = resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("result must be an array: {:?}", resp["result"]))?;

    assert!(!items.is_empty(), "prepareCallHierarchy on `sub run` must return at least one item");

    let item = &items[0];
    let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
    assert!(
        name == "run" || name == "RealBaseline::App::run",
        "CallHierarchyItem name must be `run` or qualified form, got: {name:?}"
    );
    assert!(item.get("uri").is_some(), "CallHierarchyItem must have `uri`");
    assert!(item.get("range").is_some(), "CallHierarchyItem must have `range`");
    assert!(item.get("selectionRange").is_some(), "CallHierarchyItem must have `selectionRange`");

    harness.assert_no_crash();
    Ok(())
}

/// Dogfood — callHierarchy/outgoingCalls for `run` in App.pm.
///
/// `run` calls `helper` (from Util.pm) and `shared` (from Base.pm).
/// Cross-file: outgoing calls should cross module boundaries.
///
/// MEASURE: does outgoingCalls list cross-file callees?
#[test]
fn scenario_22_call_hierarchy_outgoing_from_run() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");

    // Step 1: prepare call hierarchy for `run`.
    let prepare_resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    if prepare_resp.get("error").is_some() || prepare_resp["result"].is_null() {
        eprintln!(
            "status: outgoingCalls/run: SKIP — prepareCallHierarchy failed: {:?}",
            prepare_resp
        );
        harness.assert_no_crash();
        return Ok(());
    }

    let items = match prepare_resp["result"].as_array() {
        Some(v) if !v.is_empty() => v,
        _ => {
            eprintln!("status: outgoingCalls/run: SKIP — prepare returned empty");
            harness.assert_no_crash();
            return Ok(());
        }
    };

    // Step 2: outgoing calls using the item from step 1.
    let outgoing_resp = harness.client.request(
        "callHierarchy/outgoingCalls",
        json!({ "item": items[0] }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: outgoingCalls/run: raw response: {:?}", outgoing_resp);

    if outgoing_resp.get("error").is_some() {
        eprintln!(
            "status: outgoingCalls/run: BROKEN — JSON-RPC error: {:?}",
            outgoing_resp["error"]
        );
    } else if outgoing_resp["result"].is_null() {
        eprintln!("status: outgoingCalls/run: BROKEN — null result");
    } else if let Some(calls) = outgoing_resp["result"].as_array() {
        let names: Vec<&str> = calls
            .iter()
            .filter_map(|c| c.get("to").and_then(|t| t.get("name")).and_then(|n| n.as_str()))
            .collect();
        eprintln!(
            "status: outgoingCalls/run: {} calls returned. Callee names: {names:?}",
            calls.len()
        );
        let crosses_files = calls.iter().any(|c| {
            c.get("to")
                .and_then(|t| t.get("uri"))
                .and_then(|u| u.as_str())
                .map(|u| u.ends_with("Util.pm") || u.ends_with("Base.pm"))
                .unwrap_or(false)
        });
        if crosses_files {
            eprintln!("status: outgoingCalls/run: WORKS — cross-file callees found");
        } else if calls.is_empty() {
            eprintln!("status: outgoingCalls/run: BROKEN — empty call list");
        } else {
            eprintln!("status: outgoingCalls/run: PARTIAL — calls returned but none cross files");
        }
    } else {
        eprintln!(
            "status: outgoingCalls/run: BROKEN — result not an array: {:?}",
            outgoing_resp["result"]
        );
    }

    harness.assert_no_crash();
    Ok(())
}

/// Dogfood — callHierarchy/incomingCalls for `run` in App.pm.
///
/// `run` is called from script/real-baseline.pl: `$app->run`.
/// Cross-file: incoming calls should cross to the script file.
///
/// MEASURE: does incomingCalls list cross-file callers?
#[test]
fn scenario_22_call_hierarchy_incoming_to_run() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");

    // Step 1: prepare call hierarchy for `run`.
    let prepare_resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    if prepare_resp.get("error").is_some() || prepare_resp["result"].is_null() {
        eprintln!(
            "status: incomingCalls/run: SKIP — prepareCallHierarchy failed: {:?}",
            prepare_resp
        );
        harness.assert_no_crash();
        return Ok(());
    }

    let items = match prepare_resp["result"].as_array() {
        Some(v) if !v.is_empty() => v,
        _ => {
            eprintln!("status: incomingCalls/run: SKIP — prepare returned empty");
            harness.assert_no_crash();
            return Ok(());
        }
    };

    // Step 2: incoming calls using the item from step 1. The script didOpen is a
    // notification in the external-process harness, so poll briefly for the
    // server to observe the caller instead of treating the first empty snapshot
    // as final.
    let calls = wait_for_incoming_calls(&harness, &items[0], Duration::from_secs(5))?;
    eprintln!("status: incomingCalls/run: calls: {:?}", calls);

    if !calls.is_empty() {
        let names: Vec<&str> = calls
            .iter()
            .filter_map(|c| c.get("from").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
            .collect();
        eprintln!(
            "status: incomingCalls/run: {} callers returned. Caller names: {names:?}",
            calls.len()
        );
        if calls.is_empty() {
            eprintln!(
                "status: incomingCalls/run: BROKEN — no callers found; \
                 expected script/real-baseline.pl"
            );
        } else {
            eprintln!("status: incomingCalls/run: WORKS — at least one caller returned");
        }
    } else {
        eprintln!("status: incomingCalls/run: BROKEN — no callers found");
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — callHierarchy/outgoingCalls for `run` must return cross-file callees.
///
/// Observed PASS on current main: `helper` (Util.pm) and `shared` (Base.pm) found.
#[test]
fn scenario_22_call_hierarchy_outgoing_from_run_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let prepare_resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        prepare_resp.get("error").is_none(),
        "prepareCallHierarchy must not error: {:?}",
        prepare_resp.get("error")
    );
    let items = prepare_resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("prepareCallHierarchy must return array"))?;
    assert!(!items.is_empty(), "prepareCallHierarchy must return at least one item");

    let outgoing_resp = harness.client.request(
        "callHierarchy/outgoingCalls",
        json!({ "item": items[0] }),
        Duration::from_secs(5),
    )?;

    assert!(
        outgoing_resp.get("error").is_none(),
        "outgoingCalls must not return a JSON-RPC error: {:?}",
        outgoing_resp.get("error")
    );

    let calls = outgoing_resp["result"].as_array().ok_or_else(|| {
        anyhow::anyhow!("outgoingCalls result must be array: {:?}", outgoing_resp["result"])
    })?;

    assert!(
        !calls.is_empty(),
        "outgoingCalls for `run` must return at least one callee. \
         App::run calls helper (Util.pm), name, alias, and shared (Base.pm). Got: []"
    );

    // At least one callee must be cross-file (Util.pm or Base.pm).
    let crosses_files = calls.iter().any(|c| {
        c.get("to")
            .and_then(|t| t.get("uri"))
            .and_then(|u| u.as_str())
            .map(|u| u.ends_with("Util.pm") || u.ends_with("Base.pm"))
            .unwrap_or(false)
    });
    assert!(
        crosses_files,
        "outgoingCalls for `run` must include at least one cross-file callee \
         (Util.pm or Base.pm). Got: {calls:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — callHierarchy/incomingCalls for `run` must return at least one caller.
///
/// Fixed in #3093: top-level callers (not inside any `sub`) are now returned as
/// file-level CallHierarchyItems instead of being silently dropped.
/// script/real-baseline.pl calls `$app->run` at the top level — must appear.
#[test]
fn scenario_22_call_hierarchy_incoming_to_run_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let prepare_resp = harness.client.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        prepare_resp.get("error").is_none(),
        "prepareCallHierarchy must not error: {:?}",
        prepare_resp.get("error")
    );
    let items = prepare_resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("prepareCallHierarchy must return array"))?;
    assert!(!items.is_empty(), "prepareCallHierarchy must return at least one item");

    let calls = wait_for_incoming_calls(&harness, &items[0], Duration::from_secs(5))?;

    // script/real-baseline.pl calls $app->run — must appear as an incoming caller.
    assert!(
        !calls.is_empty(),
        "incomingCalls for `App::run` must return at least one caller. \
         script/real-baseline.pl calls `$app->run`. Got: []"
    );

    // The caller must be the script file — not just any non-empty result.
    // This guards against vacuous passes where an unrelated item happens to appear.
    let script_caller = calls.iter().find(|c| {
        c["from"]["uri"].as_str().map(|u| u.contains("real-baseline.pl")).unwrap_or(false)
    });
    assert!(
        script_caller.is_some(),
        "incomingCalls for `App::run` must include `real-baseline.pl` as a caller \
         (top-level `$app->run` call). Got callers: {calls:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 8: textDocument/inlayHint
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — inlay hints for App.pm (lines 0-22).
///
/// App.pm calls `helper($self->name)` — expects parameter hints.
///
/// MEASURE: does `textDocument/inlayHint` return hints for App.pm?
/// Hard assert: result must be an array (possibly empty — server may return []
/// for a file without classical parameter patterns, but must not error).
#[test]
fn scenario_22_inlay_hints_for_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end":   { "line": 22, "character": 0 }
            }
        }),
        Duration::from_secs(5),
    )?;

    if resp.get("error").is_some() {
        eprintln!("status: inlay-hints/App.pm: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!(
            "status: inlay-hints/App.pm: BROKEN — returned null \
             (should return array, possibly empty)"
        );
    } else if let Some(hints) = resp["result"].as_array() {
        eprintln!("status: inlay-hints/App.pm: WORKS — {} hints returned", hints.len());
        for hint in hints {
            let has_position = hint.get("position").is_some();
            let has_label = hint.get("label").is_some();
            eprintln!("  hint: position={has_position}, label={has_label}, value={hint:?}");
        }
    } else {
        eprintln!("status: inlay-hints/App.pm: BROKEN — result not an array: {:?}", resp["result"]);
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — inlayHint for App.pm must not error and must return an array.
///
/// Observed PASS on current main: returns an array (may be empty).
#[test]
fn scenario_22_inlay_hints_for_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end":   { "line": 22, "character": 0 }
            }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "inlayHint must not return a JSON-RPC error for App.pm: {:?}",
        resp.get("error")
    );
    assert!(
        !resp["result"].is_null(),
        "inlayHint must not return null for App.pm (must return array)"
    );

    let hints = resp["result"].as_array().ok_or_else(|| {
        anyhow::anyhow!("inlayHint result must be an array: {:?}", resp["result"])
    })?;

    // Each returned hint must have position and label.
    for hint in hints {
        assert!(hint.get("position").is_some(), "inlay hint must have `position`: {hint:?}");
        assert!(hint.get("label").is_some(), "inlay hint must have `label`: {hint:?}");
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 9: textDocument/prepareTypeHierarchy
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — prepareTypeHierarchy on `package RealBaseline::App` in App.pm.
///
/// App inherits from RealBaseline::Base via `use parent`.
/// MEASURE: does `textDocument/prepareTypeHierarchy` return a type item?
///
/// Cursor at line 0, col 8 (inside `RealBaseline::App`).
#[test]
fn scenario_22_prepare_type_hierarchy_for_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    // App.pm line 0: `package RealBaseline::App;` — cursor at col 8 inside the package name.
    let resp = harness.client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 8 }
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: prepareTypeHierarchy/App.pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!("status: prepareTypeHierarchy/App: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!("status: prepareTypeHierarchy/App: BROKEN — returned null");
    } else if let Some(items) = resp["result"].as_array() {
        if items.is_empty() {
            eprintln!("status: prepareTypeHierarchy/App: BROKEN — empty array");
        } else {
            let name = items[0].get("name").and_then(|n| n.as_str()).unwrap_or("<missing>");
            eprintln!("status: prepareTypeHierarchy/App: WORKS — item name={name:?}");
        }
    } else {
        eprintln!(
            "status: prepareTypeHierarchy/App: BROKEN — result not array: {:?}",
            resp["result"]
        );
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — prepareTypeHierarchy for App.pm must not error and must return
/// an array. An empty array is acceptable (not all servers implement full OO
/// hierarchy), but an error is not.
#[test]
fn scenario_22_prepare_type_hierarchy_for_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 8 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "prepareTypeHierarchy must not return a JSON-RPC error: {:?}",
        resp.get("error")
    );
    // Result must be null or array (not an unexpected type).
    assert!(
        resp["result"].is_null() || resp["result"].is_array(),
        "prepareTypeHierarchy result must be null or array, got: {:?}",
        resp["result"]
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 10: textDocument/foldingRange
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — folding ranges for App.pm (3 subs → 3 foldable regions minimum).
///
/// MEASURE: does `textDocument/foldingRange` return ranges for App.pm?
/// Hard assert: must return at least 3 ranges (one per sub: new, run, name).
#[test]
fn scenario_22_folding_ranges_for_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: folding-ranges/App.pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!("status: folding-ranges/App.pm: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!("status: folding-ranges/App.pm: BROKEN — returned null");
    } else if let Some(ranges) = resp["result"].as_array() {
        eprintln!("status: folding-ranges/App.pm: {} ranges returned", ranges.len());
        for r in ranges {
            eprintln!(
                "  range: start={:?} end={:?} kind={:?}",
                r.get("startLine"),
                r.get("endLine"),
                r.get("kind")
            );
        }
        if ranges.len() >= 3 {
            eprintln!("status: folding-ranges/App.pm: WORKS — at least 3 ranges");
        } else {
            eprintln!(
                "status: folding-ranges/App.pm: PARTIAL — only {} ranges (expected ≥3)",
                ranges.len()
            );
        }
    } else {
        eprintln!("status: folding-ranges/App.pm: BROKEN — result not array: {:?}", resp["result"]);
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — foldingRange for App.pm must return at least 3 ranges
/// (one per sub declaration: new, run, name).
///
/// Observed PASS on current main.
#[test]
fn scenario_22_folding_ranges_for_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "foldingRange must not return a JSON-RPC error: {:?}",
        resp.get("error")
    );
    assert!(!resp["result"].is_null(), "foldingRange must not return null");

    let ranges = resp["result"].as_array().ok_or_else(|| {
        anyhow::anyhow!("foldingRange result must be an array: {:?}", resp["result"])
    })?;

    assert!(
        ranges.len() >= 3,
        "foldingRange for App.pm must return at least 3 ranges (new, run, name subs). \
         Got: {} ranges: {ranges:?}",
        ranges.len()
    );

    // Each range must have startLine and endLine.
    for r in ranges {
        assert!(r.get("startLine").is_some(), "range must have `startLine`: {r:?}");
        assert!(r.get("endLine").is_some(), "range must have `endLine`: {r:?}");
        let start = r["startLine"].as_u64().unwrap_or(u64::MAX);
        let end = r["endLine"].as_u64().unwrap_or(0);
        assert!(end >= start, "range end must be >= start: {r:?}");
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 11: textDocument/codeLens
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — code lens for App.pm in the cross-file multi-module context.
///
/// MEASURE: does `textDocument/codeLens` return lenses for App.pm?
/// Hard assert: must not error and must return an array (possibly empty).
#[test]
fn scenario_22_code_lens_for_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/codeLens",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: code-lens/App.pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!("status: code-lens/App.pm: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!("status: code-lens/App.pm: BROKEN — returned null");
    } else if let Some(lenses) = resp["result"].as_array() {
        eprintln!("status: code-lens/App.pm: WORKS — {} lenses returned", lenses.len());
        for lens in lenses.iter().take(3) {
            eprintln!("  lens: {:?}", lens);
        }
    } else {
        eprintln!("status: code-lens/App.pm: BROKEN — result not array: {:?}", resp["result"]);
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — codeLens for App.pm in cross-file context must not error and
/// must return an array.
///
/// Observed PASS on current main.
#[test]
fn scenario_22_code_lens_for_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/codeLens",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "codeLens must not return a JSON-RPC error for App.pm: {:?}",
        resp.get("error")
    );
    assert!(!resp["result"].is_null(), "codeLens must not return null");

    assert!(
        resp["result"].is_array(),
        "codeLens result must be an array (possibly empty), got: {:?}",
        resp["result"]
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 12: textDocument/documentHighlight
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — document highlight on `shared` in App.pm (line 14, inside `shared`).
///
/// App.pm line 14: `    alias($self->shared);`
/// `shared` appears on lines 14 and 15 in App.pm.
///
/// MEASURE: does documentHighlight return occurrences within App.pm?
/// Hard assert: must not error; must return at least 1 highlight for `shared`.
#[test]
fn scenario_22_document_highlight_on_shared_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    // App.pm line 14 (0-indexed): `    alias($self->shared);`
    // `shared` starts at col 19 (`$self->` is 7 chars, `alias($self->` is 13 chars offset by 4).
    // Let's count: `    alias($self->shared)` → `    ` = 4, `alias(` = 6, `$self->` = 7 → col 17.
    // Better safe: col 18 (inside `shared`).
    let resp = harness.client.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 14, "character": 18 }
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: document-highlight/shared-in-app-pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!(
            "status: document-highlight/shared: BROKEN — JSON-RPC error: {:?}",
            resp["error"]
        );
    } else if resp["result"].is_null() {
        eprintln!("status: document-highlight/shared: BROKEN — returned null");
    } else if let Some(highlights) = resp["result"].as_array() {
        eprintln!("status: document-highlight/shared: {} highlights returned", highlights.len());
        for h in highlights {
            eprintln!("  highlight: range={:?} kind={:?}", h.get("range"), h.get("kind"));
        }
        if highlights.is_empty() {
            eprintln!(
                "status: document-highlight/shared: BROKEN — no highlights for `shared` \
                 (expected at least App.pm lines 14 and 15)"
            );
        } else {
            eprintln!("status: document-highlight/shared: WORKS");
        }
    } else {
        eprintln!(
            "status: document-highlight/shared: BROKEN — result not array: {:?}",
            resp["result"]
        );
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — documentHighlight for `shared` in App.pm must not error and
/// must return at least one highlight.
///
/// Observed PASS on current main.
#[test]
fn scenario_22_document_highlight_on_shared_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 14, "character": 18 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "documentHighlight must not return a JSON-RPC error: {:?}",
        resp.get("error")
    );

    // null is allowed by spec (no highlights at position), but our fixture has
    // `shared` at the cursor — the server must return at least one highlight.
    let highlights = match resp["result"].as_array() {
        Some(v) => v,
        None if resp["result"].is_null() => {
            panic!(
                "documentHighlight for `shared` in App.pm returned null; \
                 expected at least one highlight (lines 14/15)"
            );
        }
        None => {
            panic!("documentHighlight result must be an array, got: {:?}", resp["result"]);
        }
    };

    assert!(
        !highlights.is_empty(),
        "documentHighlight for `shared` must return at least one highlight; \
         cursor at line 14, col 18 in App.pm. Got: []"
    );

    // Each highlight must have a range.
    for h in highlights {
        assert!(h.get("range").is_some(), "highlight must have `range`: {h:?}");
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 13: textDocument/selectionRange
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — selection range for `sub run` in App.pm.
///
/// Cursor at `run` (line 11, col 4). Expects a hierarchy of ranges from
/// the identifier → the sub declaration → the full sub body.
///
/// MEASURE: does `textDocument/selectionRange` return a range hierarchy?
/// Hard assert: result must be a non-empty array with `range` and optional `parent`.
#[test]
fn scenario_22_selection_range_for_run_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    // App.pm line 11 (0-indexed): `sub run {`  — cursor at col 4 inside `run`.
    let resp = harness.client.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [
                { "line": 11, "character": 4 }
            ]
        }),
        Duration::from_secs(5),
    )?;

    eprintln!("status: selection-range/run-in-app-pm: response: {:?}", resp);

    if resp.get("error").is_some() {
        eprintln!("status: selection-range/run: BROKEN — JSON-RPC error: {:?}", resp["error"]);
    } else if resp["result"].is_null() {
        eprintln!("status: selection-range/run: BROKEN — returned null");
    } else if let Some(results) = resp["result"].as_array() {
        eprintln!("status: selection-range/run: {} position results returned", results.len());
        for r in results {
            eprintln!(
                "  result: range={:?}, has_parent={}",
                r.get("range"),
                r.get("parent").is_some()
            );
        }
        if results.is_empty() {
            eprintln!("status: selection-range/run: BROKEN — empty array");
        } else if results[0].get("range").is_some() {
            eprintln!("status: selection-range/run: WORKS — range present");
        } else {
            eprintln!(
                "status: selection-range/run: BROKEN — first result has no `range`: {:?}",
                results[0]
            );
        }
    } else {
        eprintln!("status: selection-range/run: BROKEN — result not array: {:?}", resp["result"]);
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — selectionRange for `sub run` must not error and must return
/// a non-empty array with a `range` for the first position.
///
/// Observed PASS on current main.
#[test]
fn scenario_22_selection_range_for_run_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [
                { "line": 11, "character": 4 }
            ]
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "selectionRange must not return a JSON-RPC error: {:?}",
        resp.get("error")
    );
    assert!(!resp["result"].is_null(), "selectionRange must not return null");

    let results = resp["result"].as_array().ok_or_else(|| {
        anyhow::anyhow!("selectionRange result must be array: {:?}", resp["result"])
    })?;

    assert!(
        !results.is_empty(),
        "selectionRange for `sub run` must return at least one SelectionRange"
    );

    assert!(
        results[0].get("range").is_some(),
        "first SelectionRange must have `range`: {:?}",
        results[0]
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 14: textDocument/formatting (single-file, multi-file context)
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — formatting for App.pm in the cross-file multi-module workspace.
///
/// This is a single-file provider, but we verify it still works correctly
/// when multiple files are open (no cross-contamination of file content).
///
/// MEASURE: does `textDocument/formatting` not error in multi-file context?
/// Hard assert: must not error (edits may be empty — perltidy may not be installed).
#[test]
fn scenario_22_formatting_for_app_pm_in_multifile_context() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let result = harness.format_document("lib/RealBaseline/App.pm")?;

    eprintln!("status: formatting/App.pm-multifile: result: {:?}", result);

    match &result {
        perl_lsp_ux_tests::FormatResult::Edits(edits) => {
            eprintln!(
                "status: formatting/App.pm-multifile: WORKS — {} text edits returned",
                edits.len()
            );
        }
        perl_lsp_ux_tests::FormatResult::Empty => {
            eprintln!(
                "status: formatting/App.pm-multifile: WORKS — empty (no-op; \
                 perltidy may not be installed)"
            );
        }
        perl_lsp_ux_tests::FormatResult::Error(err) => {
            eprintln!("status: formatting/App.pm-multifile: BROKEN — error: {err:?}");
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — formatting for App.pm in multi-file context must not return
/// a JSON-RPC error (edits or empty are both acceptable).
///
/// Observed PASS on current main.
#[test]
fn scenario_22_formatting_for_app_pm_multifile_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_22: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let result = harness.format_document("lib/RealBaseline/App.pm")?;

    assert!(
        !result.is_error(),
        "formatting App.pm in a multi-file workspace must not return a JSON-RPC error. \
         Got: {:?}",
        result.error_message()
    );

    harness.assert_no_crash();
    Ok(())
}
