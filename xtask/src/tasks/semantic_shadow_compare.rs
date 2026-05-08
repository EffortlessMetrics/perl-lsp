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
            ShadowQueryName::Hover,
            "Foo::bar",
            None,
            Some(vec!["hover:Foo::bar"]),
            "unavailable fact-backed hover fixture",
            vec![trace(
                ProviderSurface::Hover,
                ProviderFactSourceKind::Fallback,
                Provenance::SearchFallback,
                Confidence::Low,
                ProviderFactFreshness::NotApplicable,
                ProviderFallbackState::Unavailable,
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
        assert_eq!(artifact.verdict_counts.get("same"), Some(&1));
        assert_eq!(artifact.verdict_counts.get("improved"), Some(&1));
        assert_eq!(artifact.verdict_counts.get("regression"), Some(&1));
        assert_eq!(artifact.verdict_counts.get("ambiguous"), Some(&1));
        assert_eq!(artifact.verdict_counts.get("unavailable"), Some(&1));
        assert_eq!(artifact.release_readiness_verdict_counts.get("same"), Some(&1));
        assert_eq!(artifact.release_readiness_verdict_counts.get("improved"), Some(&1));
        assert_eq!(artifact.release_readiness_verdict_counts.get("regression"), Some(&0));
        assert_eq!(artifact.release_readiness_verdict_counts.get("unavailable"), Some(&0));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("regression"), Some(&1));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("ambiguous"), Some(&1));
        assert_eq!(artifact.schema_fixture_verdict_counts.get("unavailable"), Some(&1));
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
        assert!(markdown.contains("## Fact Source Traces"));
        assert!(markdown.contains("| release-readiness | FindDefinition | Definition | CompilerFact | SemanticAnalyzer | High | Fresh | Shadow |"));
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
