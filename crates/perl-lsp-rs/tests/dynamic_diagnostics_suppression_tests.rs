//! Runtime tests for dynamic-diagnostics suppression (issue #7878).
//!
//! Validates the 5 cases from the issue. NOTE: `PL109 UnquotedBareword` fires
//! for bare identifiers (e.g. `print bar;`) under `use strict 'subs'` — it
//! does NOT fire for function calls like `bar()` which the parser emits as
//! `FunctionCall` nodes. Tests use bare-identifier form to exercise suppression.
//!
//! 1. `Foo->import(@names); print bar;` — no PL109 for `bar` (dynamic import before call)
//! 2. `print bar; Foo->import(@names);` — PL109 still fires (import comes after)
//! 3. `eval "sub generated_from_string { 1 }"; print generated_from_string;` — suppressed
//! 4. `eval "sub generated_from_string { 1 }"; print truly_undefined;` — only `generated` suppressed
//! 5. No workspace semantics available — legacy PL109 still fires
//!
//! Cases 1 and 2 are tested via `DiagnosticsProvider::get_diagnostics_with_path_and_semantics`
//! with a manually constructed `WorkspaceSemanticQueries`. The `ImportExportIndex` is not
//! yet populated by `WorkspaceIndex::index_file` for `Foo->import(@names)` patterns
//! (tracked by #7875) so end-to-end indexing is not used for these cases.
//!
//! Cases 3 and 4 use `WorkspaceIndex::index_file` end-to-end because the
//! eval-sub extractor IS wired into `build_canonical_fact_shard_for_ast`.
//!
//! Case 5 is a regression guard: without workspace semantics the `PL109`
//! diagnostic still fires as before.

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

// ── Cases 1 & 2: dynamic import order-awareness ──
//
// `ImportExportIndex` is not yet populated by `WorkspaceIndex::index_file` for
// `Foo->import(@names)` patterns (tracked by #7875). We test at the provider
// level using manually constructed `WorkspaceSemanticQueries` to validate
// the complete wiring from provider → scope_issues_to_diagnostics_with_semantics
// → dynamic_callable_may_be_visible_at → suppression decision.

/// Case 1: Dynamic import at byte 0 suppresses a bareword at a later offset.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case1_dynamic_import_before_bareword_suppresses_pl109(
) -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::DiagnosticsProvider;
    use perl_parser::Parser;
    use perl_semantic_facts::{
        AnchorId, Confidence, FileId, ImportKind, ImportSpec, ImportSymbols, Provenance,
    };
    use perl_workspace::semantic::imports::ImportExportIndex;
    use perl_workspace::semantic::queries::WorkspaceSemanticQueries;
    use perl_workspace::semantic::references::ReferenceIndex;
    use perl_workspace::workspace_index::FileFactShard;
    use std::collections::HashMap;

    // Source: dynamic import statement, then a bare identifier `bar`.
    // The import's span_start_byte (0) is before `bar`'s byte offset.
    let source = "use strict 'subs';\nFoo->import(@names);\nprint bar;\n";
    let file_id = FileId(1001);
    let shard_key = "file:///test_import_before.pl";

    let mut parser = Parser::new(source);
    let ast_node = parser.parse()?;
    let ast = std::sync::Arc::new(ast_node);
    let parse_errors = parser.errors().to_vec();

    let ref_index = ReferenceIndex::new();
    let mut ie_index = ImportExportIndex::new();

    // Dynamic import at byte 0 — before `bar` at byte ~40.
    ie_index.add_file_imports(
        shard_key,
        file_id,
        vec![ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Dynamic,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(1)),
            scope_id: None,
            span_start_byte: Some(0),
        }],
    );

    let mut shards = HashMap::new();
    shards.insert(shard_key.to_string(), FileFactShard {
        source_uri: shard_key.to_string(),
        file_id,
        content_hash: 0,
        anchors_hash: None,
        entities_hash: None,
        occurrences_hash: None,
        edges_hash: None,
        anchors: vec![],
        entities: vec![],
        occurrences: vec![],
        edges: vec![],
    });

    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());

    let diagnostics = provider.get_diagnostics_with_path_and_semantics(
        &ast,
        &parse_errors,
        source,
        None,
        &[],
        None,
        file_id,
        &queries,
    );

    let pl109_for_bar =
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL109") && d.message.contains("bar"));

    if pl109_for_bar {
        return Err(format!(
            "Case 1: PL109 must NOT fire for `bar` when a Dynamic import \
             precedes the bareword offset.\nDiagnostics: {diagnostics:#?}"
        )
        .into());
    }

    Ok(())
}

/// Case 2: Dynamic import at a byte offset AFTER `bar` — PL109 must still fire.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case2_dynamic_import_after_bareword_pl109_still_fires(
) -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::diagnostics::DiagnosticsProvider;
    use perl_parser::Parser;
    use perl_semantic_facts::{
        AnchorId, Confidence, FileId, ImportKind, ImportSpec, ImportSymbols, Provenance,
    };
    use perl_workspace::semantic::imports::ImportExportIndex;
    use perl_workspace::semantic::queries::WorkspaceSemanticQueries;
    use perl_workspace::semantic::references::ReferenceIndex;
    use perl_workspace::workspace_index::FileFactShard;
    use std::collections::HashMap;

    // Source: bare identifier `bar` at byte 0, then the import comes later (byte 200).
    let source = "use strict 'subs';\nprint bar;\nFoo->import(@names);\n";
    let file_id = FileId(1002);
    let shard_key = "file:///test_import_after.pl";

    let mut parser = Parser::new(source);
    let ast_node = parser.parse()?;
    let ast = std::sync::Arc::new(ast_node);
    let parse_errors = parser.errors().to_vec();

    let ref_index = ReferenceIndex::new();
    let mut ie_index = ImportExportIndex::new();

    // Dynamic import at byte 200 — AFTER `bar` at byte ~25.
    ie_index.add_file_imports(
        shard_key,
        file_id,
        vec![ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Dynamic,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(1)),
            scope_id: None,
            span_start_byte: Some(200),
        }],
    );

    let mut shards = HashMap::new();
    shards.insert(shard_key.to_string(), FileFactShard {
        source_uri: shard_key.to_string(),
        file_id,
        content_hash: 0,
        anchors_hash: None,
        entities_hash: None,
        occurrences_hash: None,
        edges_hash: None,
        anchors: vec![],
        entities: vec![],
        occurrences: vec![],
        edges: vec![],
    });

    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());

    let diagnostics = provider.get_diagnostics_with_path_and_semantics(
        &ast,
        &parse_errors,
        source,
        None,
        &[],
        None,
        file_id,
        &queries,
    );

    let pl109_for_bar =
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL109") && d.message.contains("bar"));

    if !pl109_for_bar {
        return Err(format!(
            "Case 2: PL109 MUST fire for `bar` when the Dynamic import \
             comes AFTER the bareword offset.\nDiagnostics: {diagnostics:#?}"
        )
        .into());
    }

    Ok(())
}

// ── Cases 3 & 4: eval-sub suppression end-to-end ──

/// Case 3: `eval "sub generated_from_string { 1 }"` + bare use of `generated_from_string`.
/// WorkspaceIndex::index_file populates eval-sub evidence so PL109 must NOT fire.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn case3_eval_named_sub_suppresses_pl109_for_that_name(
) -> Result<(), Box<dyn std::error::Error>> {
    use perl_workspace::workspace_index::WorkspaceIndex;
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;

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
fn case4_eval_named_sub_does_not_suppress_unrelated_pl109(
) -> Result<(), Box<dyn std::error::Error>> {
    use perl_workspace::workspace_index::WorkspaceIndex;
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;

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
    let pl109_generated = items
        .iter()
        .any(|d| has_code(d, "PL109") && d.message.contains("generated_from_string"));
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

// ── Case 5: no workspace semantics — legacy diagnostics still emit ──

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

/// Pull diagnostics case: the pull provider's textDocument/diagnostic path
/// also threads semantic queries for eval-sub suppression (case 3 via pull path).
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
#[test]
fn pull_diagnostics_eval_sub_suppression_via_workspace_context(
) -> Result<(), Box<dyn std::error::Error>> {
    use perl_workspace::workspace_index::WorkspaceIndex;
    use perl_lsp::features::diagnostics::PullDiagnosticsContext;

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
