// Test infrastructure — allow test-friendly patterns used throughout this module.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 20 — Real-workspace provider expectations (completion / goto-definition /
//! hover / diagnostics).
//!
//! This is the **receipt-first baseline** for the four-file CPAN-style fixture at
//! `crates/perl-workspace/tests/fixtures/semantic_real_workspace/cpan_style/`.
//!
//! ## Fixture layout
//!
//! ```text
//! lib/
//!   RealBaseline/
//!     App.pm   — package RealBaseline::App; use parent 'RealBaseline::Base';
//!                use RealBaseline::Util qw(helper alias);
//!                subs: new, run, name
//!     Base.pm  — package RealBaseline::Base;  subs: shared, reset
//!     Util.pm  — package RealBaseline::Util;  subs: helper, bounce;  *alias = \&helper
//! script/
//!   real-baseline.pl — use RealBaseline::App; $app = App->new; $app->run
//! ```
//!
//! ## Classification key
//!
//! | Category | Assertion form |
//! |---|---|
//! | `works` | `assert!` / `assert_eq!` — receipt asserts correct behavior; regression-alerts if it changes |
//! | `known gap` | `eprintln!("status: …: unsupported — <reason>")` + soft/skip — records limitation as passing |
//! | `dynamic boundary` | documents the typeglob / coderef limitation explicitly |
//! | `unexpected` | `panic!("regression: …")` — genuine regression of *existing* behavior |
//!
//! ## Claim boundary
//!
//! RECEIPT-FIRST. This PR does **NOT** fix semantic regressions surfaced by these
//! receipts; gaps and unexpected results get follow-up issues filed separately.
//!
//! ## Acceptance commands
//!
//! ```bash
//! RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests \
//!     --test ux_scenario_20_real_workspace_providers -- --nocapture --test-threads=1
//! ```

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::path::PathBuf;
use std::time::Duration;

// ── Fixture sources (inlined from the cpan_style fixture) ────────────────────

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

// ── Fixture path helper (for cross-referencing absolute paths in receipts) ───

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has parent")
        .join("perl-workspace")
        .join("tests")
        .join("fixtures")
        .join("semantic_real_workspace")
        .join("cpan_style")
}

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

// ── Helpers ──────────────────────────────────────────────────────────────────

fn has_pl701(diags: &[serde_json::Value]) -> bool {
    diags.iter().any(|d| {
        d.get("code").and_then(|c| c.as_str()).map(|c| c == "PL701").unwrap_or(false)
            || d.get("code").and_then(|c| c.as_u64()).map(|c| c == 701).unwrap_or(false)
    })
}

fn is_lsp_location_shape(entry: &serde_json::Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri(entry: &serde_json::Value) -> Option<&str> {
    entry.get("uri").or_else(|| entry.get("targetUri")).and_then(serde_json::Value::as_str)
}

// ═══════════════════════════════════════════════════════════════════════════
//  COMPLETION RECEIPTS
// ═══════════════════════════════════════════════════════════════════════════

/// works — completion in App.pm (line 11, after `bless`) produces items that
/// include the `$class` variable from the same sub. Validates that in-package
/// completion does not error.
#[test]
fn scenario_20_completion_in_app_pm_does_not_error() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    // Cursor at line 12 (0-indexed), inside the `run` sub body — after `helper($`.
    // This exercises in-package completion without requiring a specific label.
    let items = harness
        .completion("lib/RealBaseline/App.pm", 12, 5)
        .map_err(|e| anyhow::anyhow!("completion returned JSON-RPC error: {e}"))?;

    // works: the server MUST NOT return an error — even empty is acceptable.
    // The receipt here is "no error". An error would be a regression.
    eprintln!("status: completion/App.pm-line12: {} items returned", items.len());
    harness.assert_no_crash();
    Ok(())
}

/// works — completion for prefix `RealBaseline::` in the script should surface
/// `RealBaseline::App` (since App.pm is in the workspace).
#[test]
fn scenario_20_completion_module_prefix_surfaces_real_baseline_app() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;
    // Line 3: `use RealBaseline::App;`  — cursor at end of `RealBaseline::` prefix (col 17).
    // We ask for completion at col 17 which is after `use RealBaseline::`.
    let labels = harness.completion_labels("script/real-baseline.pl", 3, 17)?;

    if labels.iter().any(|l| l.contains("RealBaseline") || l.contains("App")) {
        eprintln!("status: completion/module-prefix: works — RealBaseline module label found");
    } else {
        eprintln!(
            "status: completion/module-prefix: known gap — no RealBaseline label; \
             got: {labels:?}"
        );
    }

    // works: the request itself must not produce a JSON-RPC error.
    harness.assert_no_crash();
    Ok(())
}

/// works — completion inside Base.pm at sub declaration site does not crash and
/// returns items that have valid label/insertText shape when non-empty.
#[test]
fn scenario_20_completion_items_valid_shape_in_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 5 (0-indexed), `sub shared {` — cursor after `sub ` (col 4) to get
    // contextual completions in package context.
    let items = harness.completion("lib/RealBaseline/Base.pm", 4, 4)?;

    // works: every returned item must have a user-visible completion field.
    for item in &items {
        let has_label = item.get("label").and_then(serde_json::Value::as_str).is_some();
        let has_insert = item.get("insertText").and_then(serde_json::Value::as_str).is_some();
        let has_filter = item.get("filterText").and_then(serde_json::Value::as_str).is_some();
        assert!(
            has_label || has_insert || has_filter,
            "completion item must have a label/insertText/filterText field: {item:?}"
        );
    }

    eprintln!("status: completion/Base.pm-shape: {} items, all shape-valid", items.len());
    harness.assert_no_crash();
    Ok(())
}

/// known gap — completion of imported symbol `helper` inside App.pm.
/// After `use RealBaseline::Util qw(helper alias)`, completion at a call site
/// for `helper` should suggest the imported name. This may not yet be surfaced.
#[test]
fn scenario_20_completion_imported_symbol_helper_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 13 (0-indexed): `    helper($self->name);` — cursor at col 4 after `help`.
    let labels = harness.completion_labels("lib/RealBaseline/App.pm", 13, 7)?;

    if labels.iter().any(|l| l.contains("helper")) {
        eprintln!("status: completion/imported-helper: works — helper label found");
    } else {
        eprintln!(
            "status: completion/imported-helper: known gap — `helper` not in completion; \
             cross-file imported symbol completion not yet implemented. \
             got labels: {labels:?}"
        );
    }

    // This is a known gap — the request must not error regardless.
    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  GOTO-DEFINITION RECEIPTS
// ═══════════════════════════════════════════════════════════════════════════

/// works — goto-definition on `RealBaseline::Base` inside App.pm should resolve
/// to Base.pm (cross-file definition).
#[test]
fn scenario_20_goto_definition_parent_class_resolves_to_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 3 (0-indexed): `use parent 'RealBaseline::Base';`
    // cursor at col 12, inside `RealBaseline`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        3,
        12,
        5,
        Duration::from_millis(200),
    )?;

    // Validate shape on any returned results.
    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    if defs.is_empty() {
        eprintln!(
            "status: goto-def/parent-class: known gap — cross-file parent class \
             definition returned empty (indexing may not have settled)"
        );
    } else {
        let points_to_base =
            defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Base.pm")).unwrap_or(false));
        if points_to_base {
            eprintln!("status: goto-def/parent-class: works — resolved to Base.pm");
        } else {
            eprintln!(
                "status: goto-def/parent-class: known gap — results do not point to Base.pm; \
                 got: {defs:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — goto-definition on `sub shared` call in App.pm should resolve to
/// Base.pm (inherited method definition).
#[test]
fn scenario_20_goto_definition_inherited_method_shared_resolves_to_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 15 (0-indexed): `    alias($self->shared);`
    // cursor at col 18, inside `shared`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        15,
        18,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    if defs.is_empty() {
        eprintln!(
            "status: goto-def/inherited-shared: known gap — inherited method call \
             resolution returned empty"
        );
    } else {
        let points_to_base =
            defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Base.pm")).unwrap_or(false));
        if points_to_base {
            eprintln!("status: goto-def/inherited-shared: works — resolved to Base.pm");
        } else {
            eprintln!(
                "status: goto-def/inherited-shared: known gap — results present but don't \
                 point to Base.pm; got: {defs:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — goto-definition on `$self->shared` in App.pm MUST resolve to
/// Base.pm (inherited method in `use parent` parent class).
///
/// This hard-assertion test was added after the dogfood audit (#3059) found that
/// the soft-failure version was masking real product behavior: the test passed as
/// a "known gap" even when the behavior was actually working. A failure here means
/// cross-file inherited-method goto-definition has regressed.
///
/// ## Test case
/// In `App.pm`: `alias($self->shared);` — `$self` is `RealBaseline::App`,
/// `shared` is defined in `RealBaseline::Base` (via `use parent 'RealBaseline::Base'`).
/// goto-def on `shared` must navigate to `sub shared` in `Base.pm`.
#[test]
fn scenario_20_goto_definition_inherited_method_shared_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 15 (0-indexed): `    alias($self->shared);`
    // cursor at col 18, inside `shared`.
    // Receiver: $self is RealBaseline::App; `shared` is inherited from RealBaseline::Base.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        15,
        18,
        5,
        Duration::from_millis(200),
    )?;

    // Every returned entry must be a valid LSP Location or LocationLink.
    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    // Hard assert — must return at least one result.
    assert!(
        !defs.is_empty(),
        "regression: goto-def on inherited `$self->shared` in App.pm returned empty. \
         Expected navigation to Base.pm::shared (dogfood audit #3059)."
    );

    // Hard assert — the result must point to Base.pm.
    let points_to_base =
        defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Base.pm")).unwrap_or(false));
    assert!(
        points_to_base,
        "regression: goto-def on inherited `$self->shared` returned results but none point \
         to Base.pm. Got: {defs:?}. Expected Base.pm::shared (dogfood audit #3059)."
    );

    eprintln!("status: goto-def/inherited-shared-hard-assert: locked — resolved to Base.pm");
    harness.assert_no_crash();
    Ok(())
}

/// works — goto-definition on `helper` import in App.pm should resolve to
/// Util.pm (imported sub definition).
#[test]
fn scenario_20_goto_definition_imported_helper_resolves_to_util_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 13 (0-indexed): `    helper($self->name);`
    // cursor at col 4, inside `helper`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        13,
        4,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    if defs.is_empty() {
        eprintln!(
            "status: goto-def/imported-helper: known gap — imported sub call \
             returned empty definition"
        );
    } else {
        let points_to_util =
            defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Util.pm")).unwrap_or(false));
        if points_to_util {
            eprintln!("status: goto-def/imported-helper: works — resolved to Util.pm");
        } else {
            eprintln!(
                "status: goto-def/imported-helper: known gap — results do not point to \
                 Util.pm; got: {defs:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — goto-definition on `sub run` call in the script resolves to App.pm.
/// Static method call: `RealBaseline::App->new`.
#[test]
fn scenario_20_goto_definition_static_method_call_new_resolves_to_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    // Line 5 (0-indexed): `my $app = RealBaseline::App->new(name => 'demo');`
    // cursor at col 36, inside `new`.
    let defs = harness.definition_with_retry(
        "script/real-baseline.pl",
        5,
        36,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    if defs.is_empty() {
        eprintln!(
            "status: goto-def/static-new: known gap — static method call `new` \
             returned empty definition"
        );
    } else {
        let points_to_app =
            defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("App.pm")).unwrap_or(false));
        if points_to_app {
            eprintln!("status: goto-def/static-new: works — resolved to App.pm");
        } else {
            eprintln!(
                "status: goto-def/static-new: known gap — results do not point to App.pm; \
                 got: {defs:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// dynamic boundary — goto-definition on `alias` (typeglob alias) in Util.pm.
/// `*alias = \&helper;` creates a typeglob reference; the definition target is
/// ambiguous at the static analysis level.
#[test]
fn scenario_20_goto_definition_typeglob_alias_dynamic_boundary() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 11 (0-indexed): `*alias = \&helper;`
    // cursor at col 1, on the `alias` glob assignment.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/Util.pm",
        11,
        1,
        3,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry for typeglob alias must be a Location or LocationLink: {entry:?}"
        );
    }

    // dynamic boundary: any non-crashing result (including empty) is acceptable.
    // Static tools cannot reliably resolve typeglob assignments.
    eprintln!(
        "status: goto-def/typeglob-alias: dynamic boundary — \
         `*alias = \\&helper` is a runtime coderef assignment; static resolution \
         is a known limitation. Result: {} locations",
        defs.len()
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  HOVER RECEIPTS
// ═══════════════════════════════════════════════════════════════════════════

/// works — hover on `sub shared` in Base.pm must not return a JSON-RPC error.
#[test]
fn scenario_20_hover_sub_shared_in_base_pm_does_not_error() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 4 (0-indexed): `sub shared {`  cursor at col 4 (inside `shared`).
    let result = harness.hover("lib/RealBaseline/Base.pm", 4, 4);
    assert!(
        result.is_ok(),
        "hover on `sub shared` in Base.pm must not return JSON-RPC error: {result:?}"
    );

    match result {
        Ok(Some(ref hov)) => {
            // works: has `contents` field.
            assert!(
                hov.get("contents").is_some(),
                "hover result must have a `contents` field: {hov:?}"
            );
            eprintln!("status: hover/sub-shared: works — contents present");
        }
        Ok(None) => {
            eprintln!(
                "status: hover/sub-shared: known gap — hover returned null for \
                 sub declaration (degraded mode)"
            );
        }
        Err(_) => unreachable!("error case handled above"),
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — hover on `RealBaseline::Util` import in App.pm must not crash.
#[test]
fn scenario_20_hover_module_import_in_app_pm_does_not_crash() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 4 (0-indexed): `use RealBaseline::Util qw(helper alias);`
    // cursor at col 4, inside `RealBaseline`.
    let result = harness.hover("lib/RealBaseline/App.pm", 4, 4);
    assert!(result.is_ok(), "hover on module import must not return JSON-RPC error: {result:?}");

    match result {
        Ok(Some(_)) => {
            eprintln!("status: hover/module-import: works — hover result returned");
        }
        Ok(None) => {
            eprintln!(
                "status: hover/module-import: known gap — hover returned null for \
                 cross-file module import (degraded mode)"
            );
        }
        Err(_) => unreachable!(),
    }

    harness.assert_no_crash();
    Ok(())
}

/// known gap — hover on inherited method call `$self->shared` in App.pm.
/// The method is defined in Base.pm (parent class). Full cross-file hover
/// for inherited methods requires type-inference chain.
#[test]
fn scenario_20_hover_inherited_method_call_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 15 (0-indexed): `    return $self->shared;`
    // cursor at col 18, inside `shared`.
    let result = harness.hover("lib/RealBaseline/App.pm", 15, 18);
    assert!(
        result.is_ok(),
        "hover on inherited method call must not return JSON-RPC error: {result:?}"
    );

    match result {
        Ok(Some(ref hov)) => {
            // If we got a result, check its shape.
            assert!(
                hov.get("contents").is_some(),
                "hover result must have `contents` field: {hov:?}"
            );
            eprintln!(
                "status: hover/inherited-shared-call: works — hover returned contents \
                 for inherited method call"
            );
        }
        Ok(None) => {
            eprintln!(
                "status: hover/inherited-shared-call: known gap — hover returned null \
                 for inherited method; receiver type inference not yet implemented"
            );
        }
        Err(_) => unreachable!(),
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — hover result shape is valid (MarkupContent or MarkedString) when non-null.
#[test]
fn scenario_20_hover_result_has_valid_contents_shape() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 7 (0-indexed): `sub helper {`  cursor at col 4.
    let result = harness.hover("lib/RealBaseline/Util.pm", 7, 4);
    assert!(result.is_ok(), "hover must not return JSON-RPC error: {result:?}");

    if let Ok(Some(ref hov)) = result {
        let contents = hov.get("contents");
        assert!(contents.is_some(), "hover result must have `contents` field: {hov:?}");
        let contents = contents.expect("contents checked above");
        let valid = contents.get("value").is_some()
            || contents.get("kind").is_some()
            || contents.is_string()
            || contents.is_array();
        assert!(
            valid,
            "hover `contents` must be MarkupContent, MarkedString, or array: {contents:?}"
        );
        eprintln!("status: hover/util-helper-shape: works — contents shape valid");
    } else {
        eprintln!(
            "status: hover/util-helper-shape: known gap — hover returned null for \
             sub helper declaration"
        );
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  DIAGNOSTICS RECEIPTS
// ═══════════════════════════════════════════════════════════════════════════

/// works — clean App.pm (all imports present in workspace) should not fire PL701
/// for the known imports.
#[test]
fn scenario_20_diagnostics_known_modules_do_not_fire_pl701() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Wait for diagnostics to settle.
    let diags = harness.wait_for_diagnostics("lib/RealBaseline/App.pm", Duration::from_secs(5));

    // works: PL701 (missing module) must NOT fire for modules that exist in the
    // workspace — that would be a false positive regression.
    let pl701_fires = has_pl701(&diags);
    if pl701_fires {
        // This is a regression — PL701 fired for a module that is in the workspace.
        // Do NOT panic — record it as a known gap for follow-up.
        eprintln!(
            "status: diagnostics/no-false-pl701: known gap — PL701 fired for workspace-present \
             modules in App.pm; this is a false positive. Diags: {diags:?}"
        );
    } else {
        eprintln!("status: diagnostics/no-false-pl701: works — no PL701 for known modules");
    }

    // Validate shape of any diagnostics that did arrive.
    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range` field: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message` field: {diag:?}");
        if let Some(severity) = diag.get("severity") {
            let s = severity.as_u64().unwrap_or(0);
            assert!((1..=4).contains(&s), "diagnostic severity must be 1-4, got: {s}");
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — a file with a genuinely missing module should fire PL701.
#[test]
fn scenario_20_diagnostics_missing_module_fires_pl701() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    // Create harness with only the App.pm file; Base.pm and Util.pm are absent.
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("lib/RealBaseline/App.pm", APP_PM))?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let diags = harness.wait_for_diagnostics("lib/RealBaseline/App.pm", Duration::from_secs(5));

    if has_pl701(&diags) {
        eprintln!(
            "status: diagnostics/missing-module-pl701: works — PL701 fires for \
             genuinely missing RealBaseline::Base / RealBaseline::Util"
        );
    } else {
        eprintln!(
            "status: diagnostics/missing-module-pl701: known gap — PL701 did not fire \
             for missing module; missing-module detection may not yet be active. \
             Diags: {diags:?}"
        );
    }

    // Shape validation for any diagnostics received.
    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range`: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message`: {diag:?}");
    }

    harness.assert_no_crash();
    Ok(())
}

/// dynamic boundary — `*alias = \&helper;` is a typeglob coderef assignment.
/// The server MUST NOT fire a false diagnostic claiming `alias` is an unknown symbol,
/// because it is explicitly created via the typeglob mechanism.
#[test]
fn scenario_20_diagnostics_typeglob_alias_no_false_positive() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let diags = harness.wait_for_diagnostics("lib/RealBaseline/Util.pm", Duration::from_secs(5));

    // dynamic boundary: we specifically check that `alias` does not cause a
    // spurious "symbol not found" or "bare word" diagnostic.
    let alias_false_pos = diags.iter().any(|d| {
        d.get("message").and_then(|m| m.as_str()).map(|m| m.contains("alias")).unwrap_or(false)
    });

    if alias_false_pos {
        eprintln!(
            "status: diagnostics/typeglob-alias-boundary: known gap — server fires a diagnostic \
             mentioning `alias`; typeglob assignment boundary not yet recognized. \
             Diags: {diags:?}"
        );
    } else {
        eprintln!(
            "status: diagnostics/typeglob-alias-boundary: dynamic boundary — no false \
             positive for typeglob `*alias` assignment"
        );
    }

    // Shape validation.
    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range`: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message`: {diag:?}");
    }

    harness.assert_no_crash();
    Ok(())
}

/// works — after opening all four files, the server must send at least one
/// `textDocument/publishDiagnostics` notification (possibly empty) for each
/// opened file, within the deadline.
#[test]
fn scenario_20_diagnostics_notification_received_for_all_files() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    // Allow notifications to arrive.
    std::thread::sleep(Duration::from_secs(2));

    let events = harness.peek_notifications();
    let mut seen_files: Vec<String> = Vec::new();

    for ev in &events {
        if let LspEvent::Diagnostics { uri, .. } = ev {
            seen_files.push(uri.clone());
        }
    }

    if seen_files.is_empty() {
        eprintln!(
            "status: diagnostics/notification-received: known gap — server did not \
             publish diagnostics notifications within 2s (may require external linter)"
        );
    } else {
        eprintln!(
            "status: diagnostics/notification-received: works — received diagnostics \
             notifications for: {seen_files:?}"
        );
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  FIXTURE INTEGRITY CHECK
// ═══════════════════════════════════════════════════════════════════════════

/// Structural smoke test — verify the fixture directory exists on disk (not a
/// UX binary test; this never skips even when the binary is absent).
#[test]
fn scenario_20_fixture_exists_on_disk() {
    let root = fixture_root();
    assert!(root.exists(), "real-workspace fixture directory must exist at: {}", root.display());
    for relative in [
        "lib/RealBaseline/App.pm",
        "lib/RealBaseline/Base.pm",
        "lib/RealBaseline/Util.pm",
        "script/real-baseline.pl",
    ] {
        let path = root.join(relative);
        assert!(path.exists(), "fixture file must exist: {}", path.display());
    }
    eprintln!("status: fixture-integrity: works — all 4 fixture files present on disk");
}

// ═══════════════════════════════════════════════════════════════════════════
//  HARD-ASSERTION REGRESSION GUARDS (scoreboard P0 harden pass)
//
//  Each test below corresponds to a soft-failure above that was converted to a
//  hard assertion after running the suite and observing the actual outcome.
//
//  PASS → hard `assert!` locked in as a regression guard.
//  FAIL → hard `assert!` written but `#[ignore]` until the underlying gap is fixed.
//
//  See: https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3059
// ═══════════════════════════════════════════════════════════════════════════

// ── COMPLETION ───────────────────────────────────────────────────────────────

/// Regression lock — completion for `RealBaseline::` prefix MUST surface
/// a label containing "RealBaseline" or "App".
///
/// Observed PASS on current main: RealBaseline module label found.
#[test]
fn scenario_20_completion_module_prefix_surfaces_real_baseline_app_hard_assert()
-> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;
    // Line 3: `use RealBaseline::App;` — cursor after `use RealBaseline::` (col 17).
    let labels = harness.completion_labels("script/real-baseline.pl", 3, 17)?;

    assert!(
        labels.iter().any(|l| l.contains("RealBaseline") || l.contains("App")),
        "regression: completion for `RealBaseline::` prefix must include a RealBaseline \
         or App label. Got: {labels:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Known gap — completion for imported symbol `helper` does not surface in App.pm.
///
/// Observed FAIL on current main: cross-file imported symbol completion not implemented.
/// Hard assertion written; test is ignored until the gap is fixed.
/// Tracking: https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3069
#[test]
#[ignore = "real gap — cross-file imported symbol completion not implemented (tracking #3069)"]
fn scenario_20_completion_imported_symbol_helper_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 13 (0-indexed): `    helper($self->name);` — cursor at col 7 after `help`.
    let labels = harness.completion_labels("lib/RealBaseline/App.pm", 13, 7)?;

    assert!(
        labels.iter().any(|l| l.contains("helper")),
        "cross-file imported symbol `helper` must appear in completion labels. \
         Got: {labels:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ── GOTO-DEFINITION ──────────────────────────────────────────────────────────

/// Regression lock — goto-def on `RealBaseline::Base` in App.pm MUST resolve to Base.pm.
///
/// Observed PASS on current main: resolved to Base.pm.
#[test]
fn scenario_20_goto_definition_parent_class_resolves_to_base_pm_hard_assert() -> anyhow::Result<()>
{
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 3 (0-indexed): `use parent 'RealBaseline::Base';` — col 12 inside `RealBaseline`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        3,
        12,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    assert!(
        !defs.is_empty(),
        "regression: goto-def on parent class `RealBaseline::Base` in App.pm returned empty. \
         Expected navigation to Base.pm."
    );

    let points_to_base =
        defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Base.pm")).unwrap_or(false));
    assert!(
        points_to_base,
        "regression: goto-def on `RealBaseline::Base` must point to Base.pm. Got: {defs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — goto-def on inherited method `$self->shared` in App.pm MUST resolve to Base.pm.
///
/// Observed PASS on current main: resolved to Base.pm.
/// This companion hard-assert is in addition to the one added in PR #3068.
#[test]
fn scenario_20_goto_definition_inherited_shared_to_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;

    // Line 15 (0-indexed): `    alias($self->shared);` — col 18 inside `shared`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        15,
        18,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    assert!(
        !defs.is_empty(),
        "regression: goto-def on inherited `$self->shared` in App.pm returned empty. \
         Expected navigation to Base.pm::shared."
    );

    let points_to_base =
        defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Base.pm")).unwrap_or(false));
    assert!(
        points_to_base,
        "regression: goto-def on `$self->shared` must resolve to Base.pm. Got: {defs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — goto-def on imported `helper` call in App.pm MUST resolve to Util.pm.
///
/// Observed PASS on current main: resolved to Util.pm.
#[test]
fn scenario_20_goto_definition_imported_helper_to_util_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 13 (0-indexed): `    helper($self->name);` — col 4 inside `helper`.
    let defs = harness.definition_with_retry(
        "lib/RealBaseline/App.pm",
        13,
        4,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    assert!(
        !defs.is_empty(),
        "regression: goto-def on imported `helper` call in App.pm returned empty. \
         Expected navigation to Util.pm."
    );

    let points_to_util =
        defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("Util.pm")).unwrap_or(false));
    assert!(
        points_to_util,
        "regression: goto-def on imported `helper` must resolve to Util.pm. Got: {defs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — goto-def on static `new` call in script MUST resolve to App.pm.
///
/// Observed PASS on current main: resolved to App.pm.
#[test]
fn scenario_20_goto_definition_static_new_to_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    // Line 5 (0-indexed): `my $app = RealBaseline::App->new(name => 'demo');`
    // cursor at col 36, inside `new`.
    let defs = harness.definition_with_retry(
        "script/real-baseline.pl",
        5,
        36,
        5,
        Duration::from_millis(200),
    )?;

    for entry in &defs {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    assert!(
        !defs.is_empty(),
        "regression: goto-def on static method call `RealBaseline::App->new` returned empty. \
         Expected navigation to App.pm::new."
    );

    let points_to_app =
        defs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("App.pm")).unwrap_or(false));
    assert!(
        points_to_app,
        "regression: goto-def on `RealBaseline::App->new` must resolve to App.pm. Got: {defs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ── HOVER ────────────────────────────────────────────────────────────────────

/// Regression lock — hover on `sub shared` in Base.pm MUST return a non-null result
/// with a `contents` field.
///
/// Observed PASS on current main: hover returned contents.
#[test]
fn scenario_20_hover_sub_shared_in_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 4 (0-indexed): `sub shared {` — col 4 inside `shared`.
    let result = harness.hover("lib/RealBaseline/Base.pm", 4, 4);

    assert!(
        result.is_ok(),
        "regression: hover on `sub shared` in Base.pm must not return JSON-RPC error: {result:?}"
    );

    let hov = result?
        .expect("regression: hover on `sub shared` in Base.pm must return a non-null result");
    assert!(
        hov.get("contents").is_some(),
        "regression: hover result must have a `contents` field: {hov:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — hover on module import in App.pm MUST return a non-null result.
///
/// Observed PASS on current main: hover result returned.
#[test]
fn scenario_20_hover_module_import_in_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 4 (0-indexed): `use RealBaseline::Util qw(helper alias);` — col 4.
    let result = harness.hover("lib/RealBaseline/App.pm", 4, 4);

    assert!(
        result.is_ok(),
        "regression: hover on module import must not return JSON-RPC error: {result:?}"
    );
    assert!(
        result?.is_some(),
        "regression: hover on module import in App.pm must return a non-null result"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — hover on inherited method call `$self->shared` in App.pm MUST return
/// a non-null result and the hover content MUST mention the method name or its source package.
///
/// Fixed by #3070: `build_inherited_method_hover` now falls back to the open document store
/// when `find_definition` returns None (index not yet settled), so hover is deterministic.
#[test]
fn scenario_20_hover_inherited_method_call_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 15 (0-indexed): `    return $self->shared;` — col 18 inside `shared`.
    let result = harness.hover("lib/RealBaseline/App.pm", 15, 18);

    assert!(
        result.is_ok(),
        "hover on inherited method call must not return JSON-RPC error: {result:?}"
    );

    let hov = result?
        .expect("hover on `$self->shared` must return a non-null result with hover contents");

    // Provenance check: the hover text must mention the method name or its source package.
    // This guards against a vacuous non-null (e.g. a generic "unknown" response).
    let contents_value =
        hov.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        contents_value.contains("shared") || contents_value.contains("Base"),
        "hover contents must mention `shared` or its source `Base` package; got: {hov:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — hover on `sub helper` in Util.pm MUST return a non-null result
/// with a valid contents shape.
///
/// Observed PASS on current main: contents shape valid.
#[test]
fn scenario_20_hover_sub_helper_valid_shape_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    std::thread::sleep(Duration::from_millis(300));

    // Line 7 (0-indexed): `sub helper {` — col 4.
    let result = harness.hover("lib/RealBaseline/Util.pm", 7, 4);

    assert!(result.is_ok(), "hover must not return JSON-RPC error: {result:?}");

    let hov = result?
        .expect("regression: hover on `sub helper` in Util.pm must return a non-null result");
    let contents = hov.get("contents").expect("hover result must have `contents` field");

    let valid = contents.get("value").is_some()
        || contents.get("kind").is_some()
        || contents.is_string()
        || contents.is_array();
    assert!(
        valid,
        "regression: hover `contents` must be MarkupContent, MarkedString, or array: {contents:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ── DIAGNOSTICS ──────────────────────────────────────────────────────────────

/// Regression lock — clean workspace (all modules present) MUST NOT fire PL701.
///
/// Observed PASS on current main: no PL701 for known modules.
#[test]
fn scenario_20_diagnostics_no_false_pl701_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let diags = harness.wait_for_diagnostics("lib/RealBaseline/App.pm", Duration::from_secs(5));

    // Validate shape of all diagnostics.
    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range` field: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message` field: {diag:?}");
        if let Some(severity) = diag.get("severity") {
            let s = severity.as_u64().unwrap_or(0);
            assert!((1..=4).contains(&s), "diagnostic severity must be 1-4, got: {s}");
        }
    }

    assert!(
        !has_pl701(&diags),
        "regression: PL701 (missing module) must NOT fire for workspace-present modules. \
         This is a false positive. Diags: {diags:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — a file with a genuinely missing module MUST fire PL701.
///
/// Observed PASS on current main: PL701 fires for missing modules.
#[test]
fn scenario_20_diagnostics_missing_module_fires_pl701_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    // Only App.pm is present — Base.pm and Util.pm are absent.
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("lib/RealBaseline/App.pm", APP_PM))?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let diags = harness.wait_for_diagnostics("lib/RealBaseline/App.pm", Duration::from_secs(5));

    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range`: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message`: {diag:?}");
    }

    assert!(
        has_pl701(&diags),
        "regression: PL701 must fire when modules RealBaseline::Base and RealBaseline::Util \
         are genuinely missing. Diags: {diags:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Known gap — server fires a PL304 diagnostic mentioning 'alias' for the typeglob
/// `*alias = \&helper`, which is a false positive for this dynamic symbol.
///
/// Observed FAIL on current main: PL304 fires for 'alias' because it appears in
/// `@EXPORT_OK` but the server does not recognize typeglob-created subs as documented.
/// Hard assertion written; test is ignored until the gap is fixed.
/// Tracking: https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3071
#[test]
fn scenario_20_diagnostics_typeglob_alias_no_false_positive_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let diags = harness.wait_for_diagnostics("lib/RealBaseline/Util.pm", Duration::from_secs(5));

    for diag in &diags {
        assert!(diag.get("range").is_some(), "diagnostic must have `range`: {diag:?}");
        assert!(diag.get("message").is_some(), "diagnostic must have `message`: {diag:?}");
    }

    let alias_false_pos = diags.iter().any(|d| {
        d.get("message").and_then(|m| m.as_str()).map(|m| m.contains("alias")).unwrap_or(false)
    });

    assert!(
        !alias_false_pos,
        "false positive: server must not fire a diagnostic mentioning 'alias' for typeglob \
         `*alias = \\&helper`. The typeglob creates a valid sub. Diags: {diags:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — server MUST send at least one `textDocument/publishDiagnostics`
/// notification for each opened file within 2 seconds.
///
/// Observed PASS on current main: notifications received for all 4 files.
#[test]
fn scenario_20_diagnostics_notification_received_for_all_files_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_20: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;
    harness.open_file("script/real-baseline.pl", SCRIPT_PL)?;

    std::thread::sleep(Duration::from_secs(2));

    let events = harness.peek_notifications();
    let seen_files: Vec<String> = events
        .iter()
        .filter_map(|ev| {
            if let LspEvent::Diagnostics { uri, .. } = ev { Some(uri.clone()) } else { None }
        })
        .collect();

    assert!(
        !seen_files.is_empty(),
        "regression: server must publish at least one textDocument/publishDiagnostics \
         notification within 2s of opening files. None received."
    );

    harness.assert_no_crash();
    Ok(())
}
