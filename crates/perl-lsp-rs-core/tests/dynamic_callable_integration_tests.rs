//! Real end-to-end integration tests for `dynamic_callable_may_be_visible_at`
//! suppression of `UnquotedBareword` diagnostics.
//!
//! These tests use real `WorkspaceSemanticQueries` (not stubs) and exercise
//! the full path:
//!
//!   source string
//!   → real parser
//!   → ImportExtractor (real import data)
//!   → eval_sub_extractor (real eval-sub evidence)
//!   → WorkspaceSemanticQueries
//!   → scope_issues_to_diagnostics_with_semantics
//!   → assert UnquotedBareword suppressed (or fired) per case
//!
//! # Test cases
//!
//! 1. `Foo->import(@names); bar();` → suppress (import precedes call)
//! 2. `bar(); Foo->import(@names);` → still diagnose (import AFTER call, order-aware)
//! 3. `eval "sub generated_from_string { 1 }"; generated_from_string();` → suppress
//! 4. `eval "sub generated_from_string { 1 }"; truly_undefined_sub();` → diagnose
//! 5. `truly_undefined_sub();` → diagnose (no dynamic evidence at all)

use std::collections::HashMap;

use perl_lsp_rs_core::providers::diagnostics::scope::scope_issues_to_diagnostics_with_semantics;
use perl_semantic_analyzer::analysis::import_extractor::ImportExtractor;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};
use perl_semantic_facts::FileId;
use perl_workspace::semantic::eval_sub_extractor::extract_eval_sub_boundaries;
use perl_workspace::semantic::imports::ImportExportIndex;
use perl_workspace::semantic::queries::WorkspaceSemanticQueries;
use perl_workspace::semantic::references::ReferenceIndex;
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ── Helper ──

/// Index `source` under `uri`, extract dynamic import and eval-sub evidence,
/// and return everything needed to construct a `WorkspaceSemanticQueries`.
fn build_real_queries(
    source: &str,
    uri: &str,
) -> Result<(FileId, HashMap<String, FileFactShard>, ReferenceIndex, ImportExportIndex)> {
    // Use WorkspaceIndex to index the file and populate fact shards.
    let index = WorkspaceIndex::new();
    let url = url::Url::parse(uri)?;
    index
        .index_file(url, source.to_string())
        .map_err(|e| -> Box<dyn std::error::Error> { format!("index_file failed: {e}").into() })?;

    let shard =
        index.file_fact_shard(uri).ok_or_else(|| format!("fact shard missing for {uri}"))?;
    let file_id = shard.file_id;

    // Parse and extract import specs.
    let ast = {
        let mut parser = perl_parser::Parser::new(source);
        parser.parse().map_err(|e| format!("parse error: {e:?}"))?
    };
    let import_specs = ImportExtractor::extract(&ast, file_id);
    let mut ie_index = ImportExportIndex::new();
    ie_index.add_file_imports(uri, file_id, import_specs);

    // Extract eval-sub boundaries and merge into shard.
    let eval_triples = extract_eval_sub_boundaries(&ast, file_id);
    let mut full_shard = shard;
    for (entity, anchor, occurrence) in eval_triples {
        full_shard.entities.push(entity);
        full_shard.anchors.push(anchor);
        full_shard.occurrences.push(occurrence);
    }

    let mut shards = HashMap::new();
    shards.insert(full_shard.source_uri.clone(), full_shard);

    Ok((file_id, shards, ReferenceIndex::new(), ie_index))
}

fn bareword_issue(name: &str, range: (usize, usize)) -> ScopeIssue {
    ScopeIssue {
        kind: IssueKind::UnquotedBareword,
        variable_name: name.to_string(),
        line: 1,
        range,
        description: format!("Bareword '{name}' not allowed under 'use strict'"),
    }
}

// ── Case 1: dynamic import before call → suppress ──

#[test]
fn dynamic_import_before_call_suppresses_bareword() -> Result<()> {
    let source = "Foo->import(@names);\nbar();\n";
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c1_before.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    let bar_offset = source.find("bar").unwrap_or(21);
    let issues = vec![bareword_issue("bar", (bar_offset, bar_offset + 3))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert!(
        diagnostics.is_empty(),
        "case 1: dynamic import before call should suppress UnquotedBareword, got: {diagnostics:?}"
    );
    Ok(())
}

// ── Case 2: dynamic import AFTER call → still diagnose (order-aware) ──

#[test]
fn dynamic_import_after_call_still_diagnoses_bareword() -> Result<()> {
    let source = "bar();\nFoo->import(@names);\n";
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c2_after.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    // "bar" is at byte 0 — before the import at byte 7.
    let issues = vec![bareword_issue("bar", (0, 3))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert_eq!(
        diagnostics.len(),
        1,
        "case 2: dynamic import AFTER call must not suppress the bareword (order-aware), got: {diagnostics:?}"
    );
    Ok(())
}

// ── Case 3: eval-sub defines NAME → suppress that NAME ──

#[test]
fn eval_sub_suppresses_named_sub_at_call_site() -> Result<()> {
    let source = r#"eval "sub generated_from_string { 1 }"; generated_from_string();"#;
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c3_eval.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    let call_offset = source.find("generated_from_string();").unwrap_or(40);
    let issues = vec![bareword_issue("generated_from_string", (call_offset, call_offset + 21))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert!(
        diagnostics.is_empty(),
        "case 3: eval-sub named sub should suppress UnquotedBareword for that name, got: {diagnostics:?}"
    );
    Ok(())
}

// ── Case 4: eval defines NAME, but OTHER bareword is still diagnosed ──

#[test]
fn eval_sub_does_not_suppress_unrelated_bareword() -> Result<()> {
    let source = r#"eval "sub generated_from_string { 1 }"; truly_undefined_sub();"#;
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c4_unrelated.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    let call_offset = source.find("truly_undefined_sub();").unwrap_or(40);
    let issues = vec![bareword_issue("truly_undefined_sub", (call_offset, call_offset + 19))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert_eq!(
        diagnostics.len(),
        1,
        "case 4: truly_undefined_sub has no evidence and must still fire, got: {diagnostics:?}"
    );
    Ok(())
}

// ── Case 5: no dynamic evidence → always diagnose ──

#[test]
fn no_dynamic_evidence_always_diagnoses() -> Result<()> {
    let source = "truly_undefined_sub();\n";
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c5_static.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    let issues = vec![bareword_issue("truly_undefined_sub", (0, 19))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert_eq!(
        diagnostics.len(),
        1,
        "case 5: no dynamic evidence must produce diagnostic, got: {diagnostics:?}"
    );
    Ok(())
}

// ── Case 6: eval-sub declared AFTER usage → still diagnose (order-awareness) ──

#[test]
fn eval_sub_declared_after_bareword_still_diagnoses() -> Result<()> {
    // bareword at byte 0; eval-sub at byte ~24 (after the usage site).
    let source = r#"generated_from_string(); eval "sub generated_from_string { 1 }";"#;
    let (file_id, shards, ref_index, ie_index) =
        build_real_queries(source, "file:///test/c6_eval_after.pl")?;
    let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

    let call_offset = 0_usize;
    let issues = vec![bareword_issue("generated_from_string", (call_offset, call_offset + 21))];
    let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, file_id, &queries);

    assert_eq!(
        diagnostics.len(),
        1,
        "case 6: eval-sub declared after usage must NOT suppress UnquotedBareword — \
         order-awareness guard must fire. Got: {diagnostics:?}"
    );
    Ok(())
}
