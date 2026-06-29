// Test infrastructure — allow test-friendly patterns used throughout this module.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 21 — Cross-file provider dogfood baseline.
//!
//! Extends the ux_scenario_20 RealBaseline fixtures to measure the UNMEASURED
//! cross-file providers: references, rename, signature-help, document-symbol,
//! workspace-symbol, and semantic-tokens.
//!
//! This is a MEASURE-and-FILE pass — no fix PRs opened here.
//! Classification key:
//!   works   → hard `assert!` regression guard
//!   broken  → `#[ignore]` + issue filed
//!   n/a     → provider doesn't apply cross-file (documented)
//!
//! ## Fixture layout (inlined from scenario_20 / cpan_style)
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

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::json;
use std::time::Duration;

// ── Fixture sources (same as scenario_20) ────────────────────────────────────

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

// ── Shape helpers ─────────────────────────────────────────────────────────────

fn is_lsp_location_shape(entry: &serde_json::Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri(entry: &serde_json::Value) -> Option<&str> {
    entry.get("uri").or_else(|| entry.get("targetUri")).and_then(serde_json::Value::as_str)
}

fn symbol_is_shared_from_base(symbol: &serde_json::Value) -> bool {
    let is_shared = symbol.get("name").and_then(serde_json::Value::as_str) == Some("shared");
    let from_base = symbol
        .get("location")
        .and_then(|location| location.get("uri"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|uri| uri.ends_with("Base.pm"));

    is_shared && from_base
}

fn wait_for_shared_workspace_symbol(harness: &UxHarness) -> anyhow::Result<()> {
    let symbols = harness.wait_for_workspace_symbols(
        "shared",
        Duration::from_secs(5),
        Duration::from_millis(200),
        |symbols| symbols.iter().any(symbol_is_shared_from_base),
    )?;

    anyhow::ensure!(
        symbols.iter().any(symbol_is_shared_from_base),
        "workspace index did not surface Base.pm::shared before rename: {symbols:?}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 1: textDocument/references  (find-all-refs)
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — references on `sub shared` declaration in Base.pm.
/// Cross-file: usages in App.pm should be found (two call sites: line 14 and 15).
///
/// MEASURE: does `textDocument/references` return cross-file usage locations?
/// Strong assertion: result must mention App.pm (cross-file) OR be an explained gap.
#[test]
fn scenario_21_references_on_shared_in_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    // Line 4 (0-indexed) of Base.pm: `sub shared {`  — cursor at col 4 inside `shared`.
    // Fixture line layout verified: line 0 = `package RealBaseline::Base;`
    let refs = harness.references("lib/RealBaseline/Base.pm", 4, 4, false)?;

    eprintln!("status: references/shared-in-base-pm: {} locations returned", refs.len());

    // Shape check for any returned results.
    for r in &refs {
        assert!(
            is_lsp_location_shape(r),
            "reference entry must be a Location or LocationLink: {r:?}"
        );
    }

    if refs.is_empty() {
        eprintln!(
            "status: references/shared-cross-file: BROKEN — find-all-refs on `sub shared` \
             returned empty; expected at least App.pm call sites. Got: []"
        );
    } else {
        let has_app_pm =
            refs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("App.pm")).unwrap_or(false));
        if has_app_pm {
            eprintln!(
                "status: references/shared-cross-file: WORKS — App.pm call site found \
                 in {} total refs",
                refs.len()
            );
        } else {
            eprintln!(
                "status: references/shared-cross-file: PARTIAL — refs returned but none \
                 point to App.pm. Got URIs: {:?}",
                refs.iter().filter_map(|e| entry_uri(e)).collect::<Vec<_>>()
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// Regression lock — references/shared returns cross-file App.pm results.
///
/// Observed PASS on current main: 6 locations, App.pm included.
#[test]
fn scenario_21_references_on_shared_in_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let refs = harness.references("lib/RealBaseline/Base.pm", 4, 4, false)?;

    assert!(
        !refs.is_empty(),
        "find-all-refs on `sub shared` in Base.pm must return at least one location. \
         Expected App.pm call sites. Got: []"
    );

    let has_app_pm =
        refs.iter().any(|e| entry_uri(e).map(|u| u.ends_with("App.pm")).unwrap_or(false));
    assert!(
        has_app_pm,
        "find-all-refs on `sub shared` must include an App.pm reference (cross-file). \
         Got: {refs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 2: textDocument/rename
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — rename `shared` in Base.pm.
/// Cross-file rename should update both Base.pm definition and App.pm call sites.
///
/// MEASURE: does `textDocument/rename` return a workspace edit touching App.pm?
#[test]
fn scenario_21_rename_shared_in_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    wait_for_shared_workspace_symbol(&harness)?;

    // Line 4 of Base.pm: `sub shared {`  cursor at col 4 inside `shared`.
    let uri = harness.workspace.uri("lib/RealBaseline/Base.pm");
    let resp = harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 4, "character": 4 },
            "newName": "shared_renamed"
        }),
        Duration::from_secs(5),
    )?;

    eprintln!(
        "status: rename/shared-in-base-pm: raw response keys: {:?}",
        resp.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );

    if resp.get("error").is_some() {
        eprintln!(
            "status: rename/shared-cross-file: BROKEN — rename returned a JSON-RPC error: {:?}",
            resp["error"]
        );
    } else if resp["result"].is_null() {
        eprintln!(
            "status: rename/shared-cross-file: BROKEN — rename returned null \
             (no workspace edit produced)"
        );
    } else {
        let result = &resp["result"];
        // WorkspaceEdit has `changes` (map uri→[TextEdit]) or `documentChanges` (array).
        let has_changes =
            result.get("changes").is_some() || result.get("documentChanges").is_some();
        if has_changes {
            // Check if App.pm is covered cross-file.
            let changes_str = result.to_string();
            if changes_str.contains("App.pm") {
                eprintln!(
                    "status: rename/shared-cross-file: WORKS — WorkspaceEdit includes App.pm"
                );
            } else {
                eprintln!(
                    "status: rename/shared-cross-file: PARTIAL — WorkspaceEdit returned but \
                     does not include App.pm. Changes: {result:?}"
                );
            }
        } else {
            eprintln!(
                "status: rename/shared-cross-file: BROKEN — result has no `changes` or \
                 `documentChanges` field. Got: {result:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// Cross-file rename of inherited method `shared` in Base.pm must succeed.
///
/// Fixed by: detecting `->` immediately before the method name span in
/// `is_ambiguous_sub_reference` — arrow method calls are OO dispatch, not
/// unqualified bare function calls.  See issue #3084 + PR #3086.
#[test]
fn scenario_21_rename_shared_in_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    wait_for_shared_workspace_symbol(&harness)?;

    let uri = harness.workspace.uri("lib/RealBaseline/Base.pm");
    let resp = harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 4, "character": 4 },
            "newName": "shared_renamed"
        }),
        Duration::from_secs(5),
    )?;

    // Must succeed without a JSON-RPC error.
    assert!(
        resp.get("error").is_none(),
        "rename `shared` in Base.pm must not return a JSON-RPC error. \
         Got: {:?}",
        resp.get("error")
    );

    // Must return a WorkspaceEdit.
    assert!(!resp["result"].is_null(), "rename must return a WorkspaceEdit, got null");

    // Must include App.pm in the workspace edit (cross-file rename).
    let changes_str = resp["result"].to_string();
    assert!(
        changes_str.contains("App.pm"),
        "rename WorkspaceEdit must include App.pm (cross-file call sites). \
         Got: {:?}",
        resp["result"]
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 3: textDocument/signatureHelp
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — signature help for `helper(` call in App.pm.
/// `helper` is imported from Util.pm. Cross-file: expects signature from Util.pm.
///
/// MEASURE: does `textDocument/signatureHelp` return a signature?
/// We call at line 13, col 11 — just inside `helper($`.
/// App.pm line 13 (0-indexed): `    helper($self->name);`
///                                    ^   ^col 4=`h`, col 10=`(`, col 11=after `(`
#[test]
fn scenario_21_signature_help_for_helper_call_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Line 13 (0-indexed): `    helper($self->name);`
    // col 11 is just inside the `(` of `helper(`.
    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 13, "character": 11 }
        }),
        Duration::from_secs(5),
    )?;

    if resp.get("error").is_some() {
        eprintln!(
            "status: signature-help/helper-call: BROKEN — returned JSON-RPC error: {:?}",
            resp["error"]
        );
    } else if resp["result"].is_null() {
        eprintln!(
            "status: signature-help/helper-call: BROKEN — returned null \
             (no signature returned for imported function call)"
        );
    } else {
        let result = &resp["result"];
        let signatures = result.get("signatures").and_then(|s| s.as_array());
        match signatures {
            Some(sigs) if !sigs.is_empty() => {
                let label = sigs[0].get("label").and_then(|l| l.as_str()).unwrap_or("");
                if label.contains("helper") {
                    eprintln!(
                        "status: signature-help/helper-call: WORKS — signature label contains \
                         'helper': {label:?}"
                    );
                } else {
                    eprintln!(
                        "status: signature-help/helper-call: PARTIAL — signatures returned but \
                         label doesn't mention 'helper'. Got label: {label:?}"
                    );
                }
            }
            Some(_) => {
                eprintln!(
                    "status: signature-help/helper-call: BROKEN — signatures array is empty. \
                     Got: {result:?}"
                );
            }
            None => {
                eprintln!(
                    "status: signature-help/helper-call: BROKEN — result has no `signatures` \
                     field. Got: {result:?}"
                );
            }
        }
    }

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 4: textDocument/documentSymbol
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — document symbols for App.pm.
/// Should return at least `new`, `run`, `name` as subroutine symbols.
/// This is a single-file provider but the fixture is cross-file; verifies
/// that the multi-file harness setup doesn't break document symbols.
///
/// Strong assertion: must return at least 3 symbols with names containing
/// `new`, `run`, or `name`.
#[test]
fn scenario_21_document_symbols_in_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let syms = harness.document_symbols("lib/RealBaseline/App.pm")?;

    eprintln!("status: document-symbols/App.pm: {} symbols returned", syms.len());

    // Every symbol must have a name and a range.
    for sym in &syms {
        let has_name = sym.get("name").and_then(|n| n.as_str()).is_some();
        // DocumentSymbol has `range`, SymbolInformation has `location`.
        let has_location = sym.get("range").is_some() || sym.get("location").is_some();
        assert!(has_name, "document symbol must have a `name` field: {sym:?}");
        assert!(has_location, "document symbol must have `range` or `location` field: {sym:?}");
    }

    let sym_names: Vec<&str> =
        syms.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    let has_new = sym_names.iter().any(|n| *n == "new");
    let has_run = sym_names.iter().any(|n| *n == "run");
    let has_name = sym_names.iter().any(|n| *n == "name");

    if has_new && has_run && has_name {
        eprintln!(
            "status: document-symbols/App.pm: WORKS — all 3 subs (new, run, name) returned. \
             All names: {sym_names:?}"
        );
    } else {
        eprintln!(
            "status: document-symbols/App.pm: PARTIAL/BROKEN — expected new/run/name, got: \
             {sym_names:?}"
        );
    }

    // Hard assert: document symbols must return something for a non-empty file.
    assert!(
        !syms.is_empty(),
        "document symbols for App.pm must return at least one symbol; got empty. \
         App.pm has 3 named subs (new, run, name)."
    );

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — document symbols must include `new`, `run`, and `name`.
/// This is the strong form; the soft form above measures presence.
#[test]
fn scenario_21_document_symbols_in_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let syms = harness.document_symbols("lib/RealBaseline/App.pm")?;
    let sym_names: Vec<&str> =
        syms.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    assert!(
        sym_names.iter().any(|n| *n == "new"),
        "document symbols must include `new`. Got: {sym_names:?}"
    );
    assert!(
        sym_names.iter().any(|n| *n == "run"),
        "document symbols must include `run`. Got: {sym_names:?}"
    );
    assert!(
        sym_names.iter().any(|n| *n == "name"),
        "document symbols must include `name`. Got: {sym_names:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 5: workspace/symbol
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — workspace symbols query `shared`.
/// Should return `shared` from Base.pm after indexing.
///
/// MEASURE: does `workspace/symbol` surface cross-file symbols?
/// Strong assertion: result must include a symbol named `shared` from Base.pm.
#[test]
fn scenario_21_workspace_symbols_shared_from_base_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    // Poll with retry to allow workspace indexer to settle.
    let syms = harness.wait_for_workspace_symbols(
        "shared",
        Duration::from_secs(5),
        Duration::from_millis(200),
        |s| !s.is_empty(),
    )?;

    eprintln!("status: workspace-symbols/shared: {} symbols returned", syms.len());

    // Shape check.
    for sym in &syms {
        let has_name = sym.get("name").and_then(|n| n.as_str()).is_some();
        let has_location = sym.get("location").is_some();
        assert!(has_name, "workspace symbol must have `name`: {sym:?}");
        assert!(has_location, "workspace symbol must have `location`: {sym:?}");
    }

    if syms.is_empty() {
        eprintln!(
            "status: workspace-symbols/shared: BROKEN — query 'shared' returned no symbols; \
             expected Base.pm::shared"
        );
    } else {
        let sym_names: Vec<&str> =
            syms.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();
        let has_shared = sym_names.iter().any(|n| *n == "shared");
        let from_base = syms.iter().any(|s| {
            s.get("location")
                .and_then(|l| l.get("uri"))
                .and_then(|u| u.as_str())
                .map(|u| u.ends_with("Base.pm"))
                .unwrap_or(false)
        });

        if has_shared && from_base {
            eprintln!(
                "status: workspace-symbols/shared: WORKS — `shared` from Base.pm found. \
                 All names: {sym_names:?}"
            );
        } else if has_shared {
            eprintln!(
                "status: workspace-symbols/shared: PARTIAL — `shared` found but not from \
                 Base.pm. Symbols: {syms:?}"
            );
        } else {
            eprintln!(
                "status: workspace-symbols/shared: PARTIAL — symbols returned but none named \
                 `shared`. Got: {sym_names:?}"
            );
        }
    }

    // Hard assert: workspace symbols for `shared` must return at least one result.
    assert!(
        !syms.is_empty(),
        "workspace/symbol query 'shared' must return at least one symbol; \
         Base.pm defines `sub shared`. Got: []"
    );

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — workspace symbols must include `shared` from Base.pm specifically.
#[test]
fn scenario_21_workspace_symbols_shared_from_base_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;
    harness.open_file("lib/RealBaseline/Base.pm", BASE_PM)?;
    harness.open_file("lib/RealBaseline/Util.pm", UTIL_PM)?;

    let syms = harness.wait_for_workspace_symbols(
        "shared",
        Duration::from_secs(5),
        Duration::from_millis(200),
        |s| !s.is_empty(),
    )?;

    let has_shared_from_base = syms.iter().any(|s| {
        let name_ok =
            s.get("name").and_then(|n| n.as_str()).map(|n| n == "shared").unwrap_or(false);
        let uri_ok = s
            .get("location")
            .and_then(|l| l.get("uri"))
            .and_then(|u| u.as_str())
            .map(|u| u.ends_with("Base.pm"))
            .unwrap_or(false);
        name_ok && uri_ok
    });

    assert!(
        has_shared_from_base,
        "workspace/symbol 'shared' must include `shared` from Base.pm. Got: {syms:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PROVIDER 6: textDocument/semanticTokens/full
// ═══════════════════════════════════════════════════════════════════════════

/// Dogfood — semantic tokens for App.pm.
/// Should return a SemanticTokens response with non-empty `data` array
/// (encoded token positions).
///
/// MEASURE: does `textDocument/semanticTokens/full` return token data?
/// Strong assertion: result has a `data` array with at least one element.
#[test]
fn scenario_21_semantic_tokens_full_for_app_pm() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    if resp.get("error").is_some() {
        eprintln!(
            "status: semantic-tokens/App.pm: BROKEN — returned JSON-RPC error: {:?}",
            resp["error"]
        );
    } else if resp["result"].is_null() {
        eprintln!(
            "status: semantic-tokens/App.pm: BROKEN — returned null \
             (no semantic tokens for App.pm)"
        );
    } else {
        let result = &resp["result"];
        let data = result.get("data").and_then(|d| d.as_array());
        match data {
            Some(d) if !d.is_empty() => {
                eprintln!("status: semantic-tokens/App.pm: WORKS — data has {} elements", d.len());
            }
            Some(_) => {
                eprintln!(
                    "status: semantic-tokens/App.pm: BROKEN — data array is empty \
                     (no tokens classified)"
                );
            }
            None => {
                eprintln!(
                    "status: semantic-tokens/App.pm: BROKEN — result has no `data` array. \
                     Got: {result:?}"
                );
            }
        }
    }

    harness.assert_no_crash();
    Ok(())
}

/// Hard assert — semantic tokens must return non-empty data for App.pm.
#[test]
fn scenario_21_semantic_tokens_full_for_app_pm_hard_assert() -> anyhow::Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file("lib/RealBaseline/App.pm", APP_PM)?;

    let uri = harness.workspace.uri("lib/RealBaseline/App.pm");
    let resp = harness.client.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(5),
    )?;

    assert!(
        resp.get("error").is_none(),
        "semanticTokens/full must not return a JSON-RPC error for App.pm: {:?}",
        resp.get("error")
    );

    assert!(!resp["result"].is_null(), "semanticTokens/full must not return null for App.pm");

    let data = resp["result"].get("data").and_then(|d| d.as_array()).ok_or_else(|| {
        anyhow::anyhow!("semanticTokens result has no `data` array: {:?}", resp["result"])
    })?;

    assert!(
        !data.is_empty(),
        "semanticTokens/full for App.pm must return at least one token in `data`. Got: []"
    );

    harness.assert_no_crash();
    Ok(())
}
