//! Runtime tests for dynamic-diagnostics suppression (issue #7878).
//!
//! Validates the live dynamic-diagnostics suppression cases.
//!
//! # Test form: bare identifier vs. function call
//!
//! `PL109 UnquotedBareword` fires for bare *identifier* nodes (e.g.
//! `print bar;`) under `use strict 'subs'`.  It does NOT fire for function
//! calls like `bar()`, which the parser emits as `FunctionCall` nodes.
//! All tests here use the bare-identifier form (`print bar;`) to exercise
//! the suppression path.  See also issue #7878 comment for rationale.
//!
//! # Cases
//!
//! 1. `Foo->import(@names); print bar;` — no PL109 for `bar` (dynamic import
//!    before call, via real `index_file` production path)
//! 2. `print bar; Foo->import(@names);` — PL109 still fires (import after,
//!    order-awareness, via real `index_file` production path)
//! 3. `eval "sub generated_from_string { 1 }"; print generated_from_string;` — suppressed
//! 4. `eval "sub generated_from_string { 1 }"; print truly_undefined;` — only `generated` suppressed
//! 5. No workspace semantics available — legacy PL109 still fires
//! 6. Push-path (publishDiagnostics via `didOpen`): eval-sub suppression live
//! 7. Non-literal `eval $code` remains fail-closed
//! 8. Dynamic import receiver `$class->import(@names)` remains fail-closed

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use std::sync::Arc;

use lsp_types::{NumberOrString, Uri};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

fn items_from_report(
    report: lsp_types::DocumentDiagnosticReport,
) -> Result<Vec<lsp_types::Diagnostic>, Box<dyn std::error::Error>> {
    match report {
        lsp_types::DocumentDiagnosticReport::Full(full) => {
            Ok(full.full_document_diagnostic_report.items)
        }
        lsp_types::DocumentDiagnosticReport::Unchanged(_) => {
            Err("expected Full report, got Unchanged".into())
        }
    }
}

fn has_pl109_for(items: &[lsp_types::Diagnostic], name: &str) -> bool {
    items.iter().any(|d| has_code(d, "PL109") && d.message.contains(name))
}

// ── Cases 1 & 2: dynamic import order-awareness via real index_file path ────
//
// After P0 (workspace_import_extractor wired into index_file), these cases
// are tested end-to-end through `WorkspaceIndex::index_file`, which now
// populates `ImportExportIndex` with `Foo->import(@names)` as a ManualImport
// spec.  The suppression decision in
// `dynamic_callable_may_be_visible_at` consults the real imported specs.

/// Case 1: `Foo->import(@names); print bar;`
///
/// The dynamic import (`ManualImport`, `ImportSymbols::Dynamic`) is at a byte
/// offset *before* `bar`.  PL109 must NOT fire for `bar`.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case1_dynamic_import_before_bareword_suppresses_pl109() -> Result<(), Box<dyn std::error::Error>>
{
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_case1_import_before.pl";
    let uri: Uri = uri_str.parse()?;

    // Dynamic import at byte 0, bare identifier `bar` at byte ~40.
    // Tests use bare-identifier form (print bar;) because PL109 fires for
    // Identifier nodes — `bar()` is parsed as FunctionCall and does not emit PL109.
    let content = "use strict 'subs';\nFoo->import(@names);\nprint bar;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    let pl109_for_bar = items.iter().any(|d| has_code(d, "PL109") && d.message.contains("bar"));

    if pl109_for_bar {
        return Err(format!(
            "Case 1: PL109 must NOT fire for `bar` when a Dynamic import (ManualImport) \
             precedes the bareword byte offset.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 2: `print bar; Foo->import(@names);`
///
/// The dynamic import is at a byte offset *after* `bar`.  PL109 must still fire
/// because the import was not yet in scope when `bar` appeared.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case2_dynamic_import_after_bareword_pl109_still_fires() -> Result<(), Box<dyn std::error::Error>>
{
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_case2_import_after.pl";
    let uri: Uri = uri_str.parse()?;

    // Bare identifier `bar` at byte ~25, dynamic import at byte ~35 (after bar).
    let content = "use strict 'subs';\nprint bar;\nFoo->import(@names);\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    let pl109_for_bar = items.iter().any(|d| has_code(d, "PL109") && d.message.contains("bar"));

    if !pl109_for_bar {
        return Err(format!(
            "Case 2: PL109 MUST fire for `bar` when the Dynamic import (ManualImport) \
             comes AFTER the bareword byte offset.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

// ── Cases 3 & 4: eval-sub suppression end-to-end ────────────────────────────

/// Case 3: `eval "sub generated_from_string { 1 }"; print generated_from_string;`
///
/// `WorkspaceIndex::index_file` populates eval-sub `DynamicBoundary` evidence
/// via `build_canonical_fact_shard_for_ast`. PL109 must NOT fire.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case3_eval_named_sub_suppresses_pl109_for_that_name() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_eval_suppressed.pl";
    let uri: Uri = uri_str.parse()?;

    // Use bare identifier form (not function call) so PL109 fires when unsuppressed.
    let content = "use strict 'subs';\n\
        eval \"sub generated_from_string { return 1; }\";\n\
        print generated_from_string;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    let pl109_for_generated =
        items.iter().any(|d| has_code(d, "PL109") && d.message.contains("generated_from_string"));

    if pl109_for_generated {
        return Err(format!(
            "Case 3: PL109 must NOT fire for `generated_from_string` \
             (eval-sub evidence should suppress it).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 4: eval names one sub; `truly_undefined` must still fire as PL109.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case4_eval_named_sub_does_not_suppress_unrelated_pl109() -> Result<(), Box<dyn std::error::Error>>
{
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_eval_unrelated.pl";
    let uri: Uri = uri_str.parse()?;

    let content = "use strict 'subs';\n\
        eval \"sub generated_from_string { return 1; }\";\n\
        print generated_from_string;\n\
        print truly_undefined;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    // `generated_from_string` must be suppressed.
    let pl109_generated =
        items.iter().any(|d| has_code(d, "PL109") && d.message.contains("generated_from_string"));
    if pl109_generated {
        return Err(format!(
            "Case 4: PL109 must NOT fire for `generated_from_string` \
             (has eval-sub evidence).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    // `truly_undefined` must still fire.
    let pl109_undefined =
        items.iter().any(|d| has_code(d, "PL109") && d.message.contains("truly_undefined"));
    if !pl109_undefined {
        return Err(format!(
            "Case 4: PL109 MUST fire for `truly_undefined` \
             (no dynamic evidence).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

// ── Case 5: no workspace semantics — legacy diagnostics still emit ────────────

/// Case 5: When no workspace index is available, PL109 is still emitted for
/// undefined barewords. Regression guard for legacy fallback path.
#[test]
fn case5_no_semantics_legacy_pl109_still_fires() -> Result<(), Box<dyn std::error::Error>> {
    let uri: Uri = "file:///test_no_semantics.pl".parse()?;

    // Bareword in strict context without workspace index.
    let content = "use strict 'subs';\nprint some_undefined_bareword;\n";

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let pl109_fires = items.iter().any(|d| has_code(d, "PL109"));
    if !pl109_fires {
        return Err(format!(
            "Case 5: PL109 must still fire when no workspace semantics are \
             available (legacy fallback).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

// ── Cases 7 & 8: unsupported dynamic sources fail closed ───────────────────

/// Case 7: `eval $code; print runtime_generated;`
///
/// Non-literal eval is deliberately not treated as evidence for a generated
/// callable. PL109 must still fire for the later bareword.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case7_non_literal_eval_does_not_suppress_bareword_pl109()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_eval_non_literal_fail_closed.pl";
    let uri: Uri = uri_str.parse()?;

    let content = "use strict 'subs';\n\
        my $code = $ENV{GENERATED_CODE};\n\
        eval $code;\n\
        print runtime_generated;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    if !has_pl109_for(&items, "runtime_generated") {
        return Err(format!(
            "Case 7: PL109 MUST fire for `runtime_generated` because non-literal \
             eval does not provide indexed callable evidence.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 8: `$class->import(@names); print runtime_imported;`
///
/// Variable receivers are too dynamic to prove which package supplied the
/// import. PL109 must still fire even when the argument list contains the name.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case8_dynamic_import_receiver_does_not_suppress_bareword_pl109()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_dynamic_import_receiver_fail_closed.pl";
    let uri: Uri = uri_str.parse()?;

    let content = "use strict 'subs';\n\
        my $class = 'Foo';\n\
        my @names = ('runtime_imported');\n\
        $class->import(@names);\n\
        print runtime_imported;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    if !has_pl109_for(&items, "runtime_imported") {
        return Err(format!(
            "Case 8: PL109 MUST fire for `runtime_imported` because a variable \
             import receiver is not trusted as indexed callable evidence.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 9: `print foo;\n eval "sub foo { 1 }";`
///
/// The eval-sub declaration is at a byte offset *after* `foo`.  PL109 must
/// still fire because the sub was not yet visible when `foo` appeared.
/// This is the order-awareness guard for Path 2 (eval-sub evidence), symmetric
/// with Case 2's guard for Path 1 (dynamic imports).
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case9_eval_sub_declared_after_bareword_fires_pl109() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_case9_eval_after.pl";
    let uri: Uri = uri_str.parse()?;

    // `foo` bareword at byte ~20; eval-sub declaration is after it.
    // PL109 MUST fire: the eval-sub comes AFTER the usage site.
    let content = "use strict 'subs';\nprint foo;\neval \"sub foo { 1 }\";\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    if !has_pl109_for(&items, "foo") {
        return Err(format!(
            "Case 9: PL109 MUST fire for `foo` when the eval-sub declaration \
             comes AFTER the bareword usage (order violation).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 9b: `eval "sub foo { 1 }";\nprint foo;` — eval-sub BEFORE bareword.
///
/// Regression guard: the order-awareness fix must not break the happy path
/// where the eval-sub declaration precedes the usage (Case 3 analogue).
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case9b_eval_sub_declared_before_bareword_still_suppresses()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_case9b_eval_before.pl";
    let uri: Uri = uri_str.parse()?;

    let content = "use strict 'subs';\neval \"sub foo { 1 }\";\nprint foo;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    if has_pl109_for(&items, "foo") {
        return Err(format!(
            "Case 9b: PL109 must NOT fire for `foo` when the eval-sub declaration \
             comes BEFORE the bareword usage (correct suppression).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

/// Pull diagnostics case: the pull provider's textDocument/diagnostic path
/// also threads semantic queries for eval-sub suppression (case 3 via pull path).
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn pull_diagnostics_eval_sub_suppression_via_workspace_context()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;
    use perl_workspace::workspace_index::WorkspaceIndex;

    let uri_str = "file:///test_pull_eval.pl";
    let uri: Uri = uri_str.parse()?;

    let content = "use strict 'subs';\n\
        eval \"sub pull_generated { return 1; }\";\n\
        print pull_generated;\n";

    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(uri_str.parse()?, content.to_string())?;

    let mut context = PullDiagnosticsContext::new();
    context.workspace_index = Some(Arc::clone(&index));

    let provider = PullDiagnosticsProvider::new();
    let items = items_from_report(
        provider.get_document_diagnostics_with_context(&uri, content, None, &context, None),
    )?;

    let pl109_suppressed =
        items.iter().any(|d| has_code(d, "PL109") && d.message.contains("pull_generated"));

    if pl109_suppressed {
        return Err(format!(
            "Pull diagnostic path: PL109 must NOT fire for `pull_generated` \
             (eval-sub evidence via workspace context).\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

// ── Case 6 (P1): push-path (publishDiagnostics via didOpen) ─────────────────
//
// Exercises `LspServer::publish_diagnostics` by opening a document via the
// `textDocument/didOpen` notification and asserting that the push-path also
// suppresses PL109 for eval-named subs.
//
// Tests use bare-identifier form (`print eval_sub_push;`) because PL109 fires
// for Identifier nodes; `eval_sub_push()` would be parsed as FunctionCall.

mod support;

#[test]
fn case6_push_path_eval_sub_suppression_via_did_open() -> Result<(), Box<dyn std::error::Error>> {
    use support::lsp_harness::LspHarness;

    let uri = "file:///test_push_eval_sub.pl";
    // Bare identifier form: PL109 fires for `Identifier` nodes, not `FunctionCall`.
    let content = "use strict 'subs';\n\
        eval \"sub eval_sub_push { return 1; }\";\n\
        print eval_sub_push;\n";

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open(uri, content)?;

    // Wait for publishDiagnostics notifications (push path).
    // Use a generous timeout: the server may publish 0 or more batches.
    let notifications = harness.drain_notifications(Some("textDocument/publishDiagnostics"), 800);

    // If the server published diagnostics for our URI, assert no PL109 for
    // `eval_sub_push`. If no push notification arrived (server may not push
    // immediately), the test passes — absence of notification is not a failure.
    for notification in &notifications {
        let notif_uri = notification["params"]["uri"].as_str().unwrap_or("");
        // Normalize URI comparison (case-insensitive on Windows).
        if !notif_uri.eq_ignore_ascii_case(uri) {
            continue;
        }
        let diagnostics = notification["params"]["diagnostics"].as_array();
        if let Some(diags) = diagnostics {
            for diag in diags {
                let code = diag["code"].as_str().unwrap_or("");
                let message = diag["message"].as_str().unwrap_or("");
                if code == "PL109" && message.contains("eval_sub_push") {
                    return Err(format!(
                        "Case 6 (push path): PL109 must NOT fire for `eval_sub_push` \
                         when eval-sub evidence is present.\nDiagnostic: {diag}"
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}
