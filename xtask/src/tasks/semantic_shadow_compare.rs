use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use perl_semantic_facts::{
    AnchorId, Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowCompareVerdict, ShadowQueryInput, ShadowQueryName,
    ShadowResultSummary, summarize_identities,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT: &str = "docs/project/status/semantic_shadow_compare.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/semantic_shadow_compare.md";

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    receipts: Vec<SemanticShadowCompareReceipt>,
    verdict_counts: BTreeMap<String, usize>,
    release_readiness_verdict_counts: BTreeMap<String, usize>,
    schema_fixture_verdict_counts: BTreeMap<String, usize>,
    notes: &'static str,
}

pub fn run(output: Option<PathBuf>, status_md: Option<PathBuf>, check: bool) -> Result<()> {
    let root = project_root()?;
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));
    let artifact = build_artifact();
    let payload = serialize_json(&artifact)?;
    let status_markdown = render_status_markdown(&artifact);

    if check {
        verify_file_matches(&output_path, &payload)?;
        verify_file_matches(&status_path, &status_markdown)?;
        println!("semantic shadow compare check passed: outputs are current");
        return Ok(());
    }

    write_file(&output_path, &payload)?;
    write_file(&status_path, &status_markdown)?;
    println!("semantic shadow compare updated: {}", output_path.display());
    println!("status page updated: {}", status_path.display());
    Ok(())
}

fn build_artifact() -> Artifact {
    let receipts = vec![
        receipt_from_identities(
            ShadowQueryName::FindDefinition,
            "Foo::bar",
            Some(vec!["lib/Foo.pm:10:5"]),
            Some(vec!["lib/Foo.pm:10:5"]),
            "definition exact match fixture",
            vec![trace(
                ProviderSurface::Definition,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindReferences,
            "Foo::bar",
            Some(vec!["lib/Foo.pm:10:5"]),
            Some(vec!["lib/Foo.pm:10:5", "t/foo.t:4:1"]),
            "reference improved count fixture",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::SemanticFact,
                Provenance::SemanticAnalyzer,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindDefinition,
            "imported_func",
            Some(vec!["lib/Foo.pm:12:5"]),
            Some(vec!["lib/Foo.pm:12:5"]),
            "definition shadow proof: compiler-ranked imported symbol candidate is traced through ImportSpec/ExportSet; no live navigation behavior change",
            vec![trace(
                ProviderSurface::Definition,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindDefinition,
            "generated_accessor",
            Some(vec!["generated:Foo::generated_accessor:virtual"]),
            Some(vec!["generated:Foo::generated_accessor:virtual"]),
            "definition shadow proof: framework-generated candidate is labeled as generated/virtual, not treated as an exact source location",
            vec![trace(
                ProviderSurface::Definition,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindDefinition,
            "dynamic_symbol",
            Some(vec![]),
            Some(vec![]),
            "definition shadow proof: dynamic-boundary candidate blocks false precision instead of inventing a definition",
            vec![trace(
                ProviderSurface::Definition,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindDefinition,
            "low_confidence_candidate",
            Some(vec!["lib/Foo.pm:10:5"]),
            Some(vec!["lib/Foo.pm:10:5"]),
            "definition shadow proof: low-confidence candidate remains fallback and does not outrank exact syntax",
            vec![trace(
                ProviderSurface::Definition,
                ProviderFactSourceKind::Fallback,
                Provenance::NameHeuristic,
                Confidence::Low,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Fallback,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindReferences,
            "imported_func",
            Some(vec!["t/foo.t:6:9"]),
            Some(vec!["t/foo.t:6:9"]),
            "references shadow proof: compiler-ranked imported symbol occurrence is traced through ImportSpec/ExportSet; no live navigation behavior change",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindReferences,
            "generated_accessor",
            Some(vec!["t/foo.t:7:3"]),
            Some(vec!["t/foo.t:7:3"]),
            "references shadow proof: framework-generated member occurrence is labeled with framework provenance",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindReferences,
            "dynamic_symbol",
            Some(vec![]),
            Some(vec![]),
            "references shadow proof: dynamic-boundary occurrence blocks false precision instead of adding an ordinary reference",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::FindReferences,
            "low_confidence_candidate",
            Some(vec!["t/foo.t:8:3"]),
            Some(vec!["t/foo.t:8:3"]),
            "references shadow proof: low-confidence occurrence remains fallback and does not outrank exact references",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::Fallback,
                Provenance::NameHeuristic,
                Confidence::Low,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Fallback,
            )],
        ),
        receipt_from_counts(
            ShadowQueryName::CountUsages,
            "Foo::bar",
            ShadowResultSummary { available: true, match_count: 4, identities: Vec::new() },
            ShadowResultSummary { available: true, match_count: 3, identities: Vec::new() },
            "count regression sentinel fixture",
            vec![trace(
                ProviderSurface::References,
                ProviderFactSourceKind::SemanticFact,
                Provenance::SemanticAnalyzer,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::VisibleSymbols,
            "Foo::bar",
            Some(vec!["alpha", "beta"]),
            Some(vec!["alpha", "gamma"]),
            "visible symbol ambiguity fixture",
            vec![trace(
                ProviderSurface::Completion,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::CompletionVisibility,
            "completion_import_candidates",
            Some(vec!["legacy_helper"]),
            Some(vec!["legacy_helper", "imported_func"]),
            "completion shadow proof: compiler visible-symbol candidate from ImportSpec/ExportSet; legacy_candidates=1; compiler_fact_candidates=2; rank_delta=+1; no_expected_legacy_candidates_removed",
            vec![trace(
                ProviderSurface::Completion,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::CompletionVisibility,
            "completion_generated_candidates",
            Some(vec![]),
            Some(vec!["generated_accessor"]),
            "completion shadow proof: framework-generated member is labeled as generated, not live-ranked; legacy_candidates=0; compiler_fact_candidates=1; rank_delta=+1; generated_labels=generated_accessor",
            vec![trace(
                ProviderSurface::Completion,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::CompletionVisibility,
            "completion_dynamic_boundary",
            Some(vec![]),
            Some(vec![]),
            "completion shadow proof: dynamic-boundary hint is traced and blocked, not ranked as an ordinary completion; dynamic_boundary_blockers=symbolic_ref_candidate",
            vec![trace(
                ProviderSurface::Completion,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::Hover,
            "hover_imported_symbol",
            Some(vec!["hover:imported_symbol"]),
            Some(vec!["hover:imported_symbol"]),
            "hover provenance proof: imported symbol hover labels source/confidence/freshness without changing fallback behavior",
            vec![trace(
                ProviderSurface::Hover,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Primary,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::Hover,
            "hover_generated_member",
            Some(vec!["hover:generated_member"]),
            Some(vec!["hover:generated_member"]),
            "hover provenance proof: framework-generated member hover labels framework provenance and confidence",
            vec![trace(
                ProviderSurface::Hover,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Primary,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::Hover,
            "hover_dynamic_boundary",
            Some(vec![]),
            Some(vec![]),
            "hover provenance proof: dynamic-boundary hover explains uncertainty and stays blocked instead of inventing a definition",
            vec![trace(
                ProviderSurface::Hover,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::Hover,
            "hover_fallback",
            Some(vec!["hover:legacy"]),
            Some(vec!["hover:legacy"]),
            "hover provenance proof: missing compiler facts preserve legacy fallback with explicit fallback trace",
            vec![trace(
                ProviderSurface::Hover,
                ProviderFactSourceKind::Fallback,
                Provenance::SearchFallback,
                Confidence::Low,
                ProviderFactFreshness::NotApplicable,
                ProviderFallbackState::Fallback,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DiagnosticsCheck,
            "imported_func",
            Some(vec![]),
            Some(vec!["false_positive_removed:imported_func"]),
            "diagnostics live cutover fixture: legacy=warn compiler_fact=suppress via ImportSpec/ExportSet; false_positive_delta=-1; false_negative_delta=0",
            vec![trace(
                ProviderSurface::Diagnostics,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Primary,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DiagnosticsCheck,
            "generated_accessor",
            Some(vec![]),
            Some(vec!["false_positive_removed:generated_accessor"]),
            "diagnostics live cutover fixture: legacy=warn compiler_fact=suppress via framework-generated visibility; false_positive_delta=-1; false_negative_delta=0",
            vec![trace(
                ProviderSurface::Diagnostics,
                ProviderFactSourceKind::CompilerFact,
                Provenance::FrameworkSynthesis,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Primary,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DiagnosticsCheck,
            "genuinely_missing",
            Some(vec!["warn:genuinely_missing"]),
            Some(vec!["warn:genuinely_missing"]),
            "diagnostics live cutover fixture: compiler facts preserve exact undefined-symbol warning; false_positive_delta=0; false_negative_delta=0",
            vec![trace(
                ProviderSurface::Diagnostics,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Primary,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DiagnosticsCheck,
            "ambiguous_import",
            Some(vec!["warn:ambiguous_import"]),
            Some(vec!["weak_warn:ambiguous_import"]),
            "diagnostics live cutover fixture: ambiguous/low-confidence compiler fact falls back instead of suppressing; false_positive_delta=0; false_negative_delta=0",
            vec![trace(
                ProviderSurface::Diagnostics,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::Low,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Fallback,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DiagnosticsCheck,
            "symbolic_ref_boundary",
            Some(vec![]),
            Some(vec!["dynamic_boundary_blocked:symbolic_ref_boundary"]),
            "diagnostics shadow fixture: legacy=warn compiler_fact=dynamic-boundary-blocked for symbolic ref; false_positive_delta=-1; false_negative_delta=0",
            vec![trace(
                ProviderSurface::Diagnostics,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::WorkspaceSymbols,
            "workspace_symbol_imported",
            Some(vec!["workspace:Foo::imported_func"]),
            Some(vec!["workspace:Foo::imported_func"]),
            "workspace-symbol shadow proof: fresh compiler fact source/freshness trace matches legacy identity without changing live provider behavior",
            vec![trace(
                ProviderSurface::WorkspaceSymbols,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::WorkspaceSymbols,
            "workspace_symbol_generated",
            Some(vec![]),
            Some(vec!["generated:Foo::generated_accessor:virtual"]),
            "workspace-symbol shadow proof: framework-generated candidate is labeled as generated/virtual, not treated as an exact source-backed symbol",
            vec![trace(
                ProviderSurface::WorkspaceSymbols,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::WorkspaceSymbols,
            "workspace_symbol_dynamic_boundary",
            Some(vec![]),
            Some(vec![]),
            "workspace-symbol shadow proof: dynamic-boundary facts block false workspace-symbol precision",
            vec![trace(
                ProviderSurface::WorkspaceSymbols,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::WorkspaceSymbols,
            "workspace_symbol_stale_fact",
            Some(vec![]),
            Some(vec![]),
            "workspace-symbol shadow proof: stale compiler facts cannot authorize workspace-symbol answers",
            vec![trace(
                ProviderSurface::WorkspaceSymbols,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DocumentSymbols,
            "document_symbol_explicit",
            Some(vec!["document:Foo:package:0:0"]),
            Some(vec!["document:Foo:package:0:0"]),
            "document-symbol shadow proof: explicit syntax source/freshness trace matches legacy identity without changing live provider behavior",
            vec![trace(
                ProviderSurface::DocumentSymbols,
                ProviderFactSourceKind::ParserSyntax,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DocumentSymbols,
            "document_symbol_generated",
            Some(vec![]),
            Some(vec!["generated:Foo::generated_accessor:virtual"]),
            "document-symbol shadow proof: framework-generated candidate is labeled as generated/virtual, not treated as an exact source-backed symbol",
            vec![trace(
                ProviderSurface::DocumentSymbols,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DocumentSymbols,
            "document_symbol_dynamic_boundary",
            Some(vec![]),
            Some(vec![]),
            "document-symbol shadow proof: dynamic-boundary facts block false document-symbol precision",
            vec![trace(
                ProviderSurface::DocumentSymbols,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::DocumentSymbols,
            "document_symbol_stale_fact",
            Some(vec![]),
            Some(vec![]),
            "document-symbol shadow proof: stale compiler facts cannot authorize document-symbol answers",
            vec![trace(
                ProviderSurface::DocumentSymbols,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::RenamePlan,
            "rename_exact_static",
            Some(vec!["edit:definition:anchor:1"]),
            Some(vec!["edit:definition:anchor:1"]),
            "rename boundary proof: exact static edit remains traceable as a fresh semantic fact",
            vec![trace(
                ProviderSurface::Rename,
                ProviderFactSourceKind::SemanticFact,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::RenamePlan,
            "rename_dynamic_boundary",
            Some(vec!["blocker:DynamicBoundary"]),
            Some(vec!["blocker:DynamicBoundary"]),
            "rename boundary proof: dynamic-boundary facts block unsafe edits instead of authorizing rename",
            vec![trace(
                ProviderSurface::Rename,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::RenamePlan,
            "rename_stale_compiler_fact",
            Some(vec!["blocker:StaleFact"]),
            Some(vec!["blocker:StaleFact"]),
            "rename boundary proof: stale compiler facts cannot authorize edits",
            vec![trace(
                ProviderSurface::Rename,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::RenamePlan,
            "rename_low_confidence",
            Some(vec!["blocker:AmbiguousReference"]),
            Some(vec!["blocker:AmbiguousReference"]),
            "rename boundary proof: ambiguous low-confidence facts remain blockers",
            vec![trace(
                ProviderSurface::Rename,
                ProviderFactSourceKind::SemanticFact,
                Provenance::NameHeuristic,
                Confidence::Low,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::SafeDeletePlan,
            "safe_delete_exact_static",
            Some(vec!["safe_delete:allowed"]),
            Some(vec!["safe_delete:allowed"]),
            "safe-delete boundary proof: exact static no-reference plan remains traceable",
            vec![trace(
                ProviderSurface::SafeDelete,
                ProviderFactSourceKind::SemanticFact,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::SafeDeletePlan,
            "safe_delete_dynamic_boundary",
            Some(vec!["blocker:DynamicBoundary"]),
            Some(vec!["blocker:DynamicBoundary"]),
            "safe-delete boundary proof: dynamic-boundary facts block unsafe deletion",
            vec![trace(
                ProviderSurface::SafeDelete,
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::SafeDeletePlan,
            "safe_delete_generated_member",
            Some(vec!["blocker:GeneratedMember"]),
            Some(vec!["blocker:GeneratedMember"]),
            "safe-delete boundary proof: framework-generated members block deletion unless a generator-aware plan exists",
            vec![trace(
                ProviderSurface::SafeDelete,
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
        ),
        receipt_from_identities(
            ShadowQueryName::SafeDeletePlan,
            "safe_delete_stale_compiler_fact",
            Some(vec!["blocker:StaleFact"]),
            Some(vec!["blocker:StaleFact"]),
            "safe-delete boundary proof: stale compiler facts cannot authorize deletion",
            vec![trace(
                ProviderSurface::SafeDelete,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
        ),
    ];

    let verdict_counts = count_verdicts(&receipts);
    let release_readiness_verdict_counts =
        count_verdicts(receipts.iter().filter(|receipt| is_release_readiness_receipt(receipt)));
    let schema_fixture_verdict_counts =
        count_verdicts(receipts.iter().filter(|receipt| !is_release_readiness_receipt(receipt)));

    Artifact {
        schema_version: 3,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic_shadow_compare",
        receipts,
        verdict_counts,
        release_readiness_verdict_counts,
        schema_fixture_verdict_counts,
        notes: "0.13.2 semantic shadow proof: release-readiness counts include provider-gating receipts only; schema fixture receipts exercise non-gating verdict shapes.",
    }
}

fn count_verdicts<'a>(
    receipts: impl IntoIterator<Item = &'a SemanticShadowCompareReceipt>,
) -> BTreeMap<String, usize> {
    let mut verdict_counts = empty_verdict_counts();
    for receipt in receipts {
        let key = verdict_key(receipt.verdict).to_string();
        *verdict_counts.entry(key).or_default() += 1;
    }
    verdict_counts
}

fn empty_verdict_counts() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("same".to_string(), 0),
        ("improved".to_string(), 0),
        ("regression".to_string(), 0),
        ("ambiguous".to_string(), 0),
        ("unavailable".to_string(), 0),
    ])
}

fn is_release_readiness_receipt(receipt: &SemanticShadowCompareReceipt) -> bool {
    matches!(receipt.query, ShadowQueryName::FindDefinition | ShadowQueryName::FindReferences)
}

fn receipt_scope(receipt: &SemanticShadowCompareReceipt) -> &'static str {
    if is_release_readiness_receipt(receipt) { "release-readiness" } else { "schema-fixture" }
}

fn receipt_from_identities(
    query: ShadowQueryName,
    symbol: &str,
    old: Option<Vec<&str>>,
    new: Option<Vec<&str>>,
    note: &str,
    fact_source_traces: Vec<ProviderFactTrace>,
) -> SemanticShadowCompareReceipt {
    SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        query,
        ShadowQueryInput { symbol: symbol.to_string() },
        summarize_identities(old.map(strs_to_strings)),
        summarize_identities(new.map(strs_to_strings)),
        vec![note.to_string()],
        fact_source_traces,
    )
}

fn receipt_from_counts(
    query: ShadowQueryName,
    symbol: &str,
    old_result: ShadowResultSummary,
    new_result: ShadowResultSummary,
    note: &str,
    fact_source_traces: Vec<ProviderFactTrace>,
) -> SemanticShadowCompareReceipt {
    SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        query,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_result,
        new_result,
        vec![note.to_string()],
        fact_source_traces,
    )
}

fn trace(
    surface: ProviderSurface,
    source: ProviderFactSourceKind,
    provenance: Provenance,
    confidence: Confidence,
    freshness: ProviderFactFreshness,
    fallback_state: ProviderFallbackState,
) -> ProviderFactTrace {
    ProviderFactTrace::new(
        surface,
        source,
        provenance,
        confidence,
        freshness,
        fallback_state,
        Some("deterministic-fixture".to_string()),
        Some(AnchorId(1)),
        Some(1),
    )
}

fn strs_to_strings(values: Vec<&str>) -> Vec<String> {
    values.into_iter().map(str::to_string).collect()
}

fn verdict_key(verdict: ShadowCompareVerdict) -> &'static str {
    match verdict {
        ShadowCompareVerdict::Same => "same",
        ShadowCompareVerdict::Improved => "improved",
        ShadowCompareVerdict::Regression => "regression",
        ShadowCompareVerdict::Ambiguous => "ambiguous",
        ShadowCompareVerdict::Unavailable => "unavailable",
    }
}

fn serialize_json(artifact: &Artifact) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(artifact)?))
}

fn write_file(path: &Path, payload: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, payload).with_context(|| format!("writing {}", path.display()))
}

fn verify_file_matches(path: &Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if actual != expected {
        bail!(
            "{} is stale; run `cargo xtask semantic-shadow-compare` to refresh generated outputs",
            path.display()
        );
    }
    Ok(())
}

fn render_status_markdown(artifact: &Artifact) -> String {
    let mut text = String::new();
    text.push_str("# Semantic Shadow Compare\n\n");
    text.push_str(&format!("Measured: `{}`\n\n", artifact.measured_at));
    text.push_str(&format!("Receipts: `{}`\n\n", artifact.receipts.len()));

    text.push_str("## Verdict Counts\n\n| Verdict | Count |\n|---|---:|\n");
    for (verdict, count) in &artifact.verdict_counts {
        text.push_str(&format!("| {verdict} | {count} |\n"));
    }

    text.push_str("\n## Release-Readiness Verdict Counts\n\n");
    text.push_str("| Verdict | Count |\n|---|---:|\n");
    for (verdict, count) in &artifact.release_readiness_verdict_counts {
        text.push_str(&format!("| {verdict} | {count} |\n"));
    }

    text.push_str("\n## Schema Fixture Verdict Counts\n\n");
    text.push_str("| Verdict | Count |\n|---|---:|\n");
    for (verdict, count) in &artifact.schema_fixture_verdict_counts {
        text.push_str(&format!("| {verdict} | {count} |\n"));
    }

    text.push_str("\n## Receipts\n\n");
    text.push_str("| Scope | Query | Symbol | Verdict | Old count | New count |\n");
    text.push_str("|---|---|---|---|---:|---:|\n");
    for receipt in &artifact.receipts {
        text.push_str(&format!(
            "| {} | {:?} | `{}` | {} | {} | {} |\n",
            receipt_scope(receipt),
            receipt.query,
            receipt.input.symbol,
            verdict_key(receipt.verdict),
            receipt.old_result.match_count,
            receipt.new_result.match_count
        ));
    }

    text.push_str("\n## Fact Source Traces\n\n");
    text.push_str(
        "| Scope | Query | Surface | Source | Provenance | Confidence | Freshness | State |\n",
    );
    text.push_str("|---|---|---|---|---|---|---|---|\n");
    for receipt in &artifact.receipts {
        for trace in &receipt.fact_source_traces {
            text.push_str(&format!(
                "| {} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} |\n",
                receipt_scope(receipt),
                receipt.query,
                trace.surface,
                trace.source,
                trace.provenance,
                trace.confidence,
                trace.freshness,
                trace.fallback_state
            ));
        }
    }

    text.push('\n');
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_includes_required_verdict_rows() {
        let artifact = build_artifact();
        assert_eq!(artifact.schema_version, 3);
        assert_eq!(artifact.verdict_counts.get("same"), Some(&29));
        assert_eq!(artifact.verdict_counts.get("improved"), Some(&8));
        assert_eq!(artifact.verdict_counts.get("regression"), Some(&1));
        assert_eq!(artifact.verdict_counts.get("ambiguous"), Some(&2));
        assert_eq!(artifact.verdict_counts.get("unavailable"), Some(&0));
        assert_eq!(artifact.release_readiness_verdict_counts.get("same"), Some(&9));
        assert_eq!(artifact.release_readiness_verdict_counts.get("improved"), Some(&1));
        assert_eq!(artifact.release_readiness_verdict_counts.get("regression"), Some(&0));
        assert_eq!(artifact.release_readiness_verdict_counts.get("unavailable"), Some(&0));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("same"), Some(&20));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("improved"), Some(&7));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("regression"), Some(&1));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("ambiguous"), Some(&2));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("unavailable"), Some(&0));
        assert!(
            artifact.receipts.iter().all(|receipt| !receipt.fact_source_traces.is_empty()),
            "every deterministic shadow-compare receipt should carry fact-source trace proof"
        );
    }

    #[test]
    fn artifact_json_is_deterministic() -> Result<()> {
        let first = serialize_json(&build_artifact())?;
        let second = serialize_json(&build_artifact())?;
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn status_markdown_separates_release_readiness_from_schema_fixtures() {
        let markdown = render_status_markdown(&build_artifact());
        assert!(markdown.contains("## Release-Readiness Verdict Counts"));
        assert!(markdown.contains("## Schema Fixture Verdict Counts"));
        assert!(markdown.contains("| release-readiness | FindDefinition"));
        assert!(markdown.contains("| schema-fixture | CountUsages"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | `completion_import_candidates` | improved | 1 | 2 |"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | `completion_generated_candidates` | improved | 0 | 1 |"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | `completion_dynamic_boundary` | same | 0 | 0 |"));
        assert!(markdown.contains("## Fact Source Traces"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | CompilerFact | SemanticAnalyzer | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | Completion | CompilerFact | ImportExportInference | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | Completion | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | CompletionVisibility | Completion | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | Hover | Hover | CompilerFact | ImportExportInference | High | Fresh | Primary |"));
        assert!(markdown.contains("| schema-fixture | Hover | Hover | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Primary |"));
        assert!(markdown.contains("| schema-fixture | Hover | Hover | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | Hover | Hover | Fallback | SearchFallback | Low | NotApplicable | Fallback |"));
        assert!(markdown.contains("| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | ImportExportInference | High | Fresh | Primary |"));
        assert!(markdown.contains("| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | FrameworkSynthesis | High | Fresh | Primary |"));
        assert!(markdown.contains("| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | ImportExportInference | Low | Fresh | Fallback |"));
        assert!(markdown.contains("| schema-fixture | DiagnosticsCheck | Diagnostics | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains(
            "| schema-fixture | WorkspaceSymbols | `workspace_symbol_imported` | same | 1 | 1 |"
        ));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | `workspace_symbol_generated` | improved | 0 | 1 |"));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | `workspace_symbol_dynamic_boundary` | same | 0 | 0 |"));
        assert!(markdown.contains(
            "| schema-fixture | WorkspaceSymbols | `workspace_symbol_stale_fact` | same | 0 | 0 |"
        ));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | CompilerFact | ImportExportInference | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |"));
        assert!(markdown.contains(
            "| schema-fixture | DocumentSymbols | `document_symbol_explicit` | same | 1 | 1 |"
        ));
        assert!(markdown.contains(
            "| schema-fixture | DocumentSymbols | `document_symbol_generated` | improved | 0 | 1 |"
        ));
        assert!(markdown.contains(
            "| schema-fixture | DocumentSymbols | `document_symbol_dynamic_boundary` | same | 0 | 0 |"
        ));
        assert!(markdown.contains(
            "| schema-fixture | DocumentSymbols | `document_symbol_stale_fact` | same | 0 | 0 |"
        ));
        assert!(markdown.contains("| schema-fixture | DocumentSymbols | DocumentSymbols | ParserSyntax | ExactAst | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | DocumentSymbols | DocumentSymbols | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | DocumentSymbols | DocumentSymbols | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | DocumentSymbols | DocumentSymbols | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |"));
        assert!(markdown.contains("| schema-fixture | RenamePlan | Rename | SemanticFact | ExactAst | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | RenamePlan | Rename | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | RenamePlan | Rename | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |"));
        assert!(markdown.contains("| schema-fixture | RenamePlan | Rename | SemanticFact | NameHeuristic | Low | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | SafeDeletePlan | SafeDelete | SemanticFact | ExactAst | High | Fresh | Shadow |"));
        assert!(markdown.contains("| schema-fixture | SafeDeletePlan | SafeDelete | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | SafeDeletePlan | SafeDelete | FrameworkAdapter | FrameworkSynthesis | High | Fresh | Blocked |"));
        assert!(markdown.contains("| schema-fixture | SafeDeletePlan | SafeDelete | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | CompilerFact | ImportExportInference | High | Fresh | Shadow |"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | Fallback | NameHeuristic | Low | Fresh | Fallback |"));
        assert!(markdown.contains("| release-readiness | FindReferences | References | CompilerFact | ImportExportInference | High | Fresh | Shadow |"));
        assert!(markdown.contains("| release-readiness | FindReferences | References | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |"));
        assert!(markdown.contains("| release-readiness | FindReferences | References | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |"));
        assert!(markdown.contains("| release-readiness | FindReferences | References | Fallback | NameHeuristic | Low | Fresh | Fallback |"));
    }

    #[test]
    fn verify_file_matches_detects_drift() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(tmp.path(), "actual\n")?;
        let err = verify_file_matches(tmp.path(), "expected\n").expect_err("must fail on drift");
        assert!(err.to_string().contains("is stale"));
        Ok(())
    }
}
