use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FIXTURE_MANIFEST: &str =
    "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/manifest.json";
const DEFAULT_OUTPUT: &str = "target/receipts/metrics/semantic_scorecard.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/semantic_scorecard.md";

const METRICS: &[&str] = &[
    "definition_hit_at_1",
    "definition_hit_at_5",
    "reference_precision",
    "reference_recall",
    "completion_top1",
    "completion_top5",
    "undefined_symbol_false_positive_rate",
    "rename_unsafe_edit_count",
    "safe_delete_external_ref_detection",
    "query_latency_p50",
    "query_latency_p95",
];

#[derive(Debug, Deserialize)]
struct SemanticManifest {
    fixture_family_version: u32,
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    /// Fixture family label (reserved for future grouping/filtering).
    #[allow(dead_code)]
    family: String,
    /// Relative path to the fixture file (reserved for future harness use).
    #[allow(dead_code)]
    path: String,
}

#[derive(Debug, Serialize)]
struct MetricRow {
    status: &'static str,
    value: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    fixture_family_version: u32,
    fixture_count: usize,
    fixture_ids: Vec<String>,
    rows: BTreeMap<String, MetricRow>,
    notes: &'static str,
}

pub fn run(
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    status_md: Option<PathBuf>,
    check: bool,
) -> Result<()> {
    let root = project_root()?;
    let manifest_path =
        root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let manifest = load_manifest(&manifest_path)?;
    let artifact = build_artifact(manifest);

    let json_payload = render_json(&artifact)?;
    let markdown_payload = render_status_markdown(&artifact);

    if check {
        check_output_matches(&output_path, &json_payload)?;
        check_output_matches(&status_path, &markdown_payload)?;
        println!(
            "semantic scorecard is up-to-date: {} and {}",
            output_path.display(),
            status_path.display()
        );
        return Ok(());
    }

    write_payload(&output_path, &json_payload)?;
    write_payload(&status_path, &markdown_payload)?;

    println!("semantic scorecard updated: {}", output_path.display());
    println!("status page updated: {}", status_path.display());
    Ok(())
}

fn load_manifest(path: &Path) -> Result<SemanticManifest> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut parsed: SemanticManifest =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    parsed.fixtures.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(parsed)
}

fn build_artifact(manifest: SemanticManifest) -> Artifact {
    let fixture_ids =
        manifest.fixtures.iter().map(|fixture| fixture.id.clone()).collect::<Vec<_>>();
    let mut rows = BTreeMap::new();
    for &metric in METRICS {
        rows.insert(metric.to_string(), MetricRow { status: "baseline_pending", value: None });
    }

    Artifact {
        schema_version: 1,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        rows,
        notes: "Initial harness: metrics intentionally baseline_pending until semantic facts land.",
    }
}

fn render_json(artifact: &Artifact) -> Result<String> {
    let payload = serde_json::to_string_pretty(artifact)?;
    Ok(format!("{payload}\n"))
}

fn write_payload(path: &Path, payload: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, payload).with_context(|| format!("writing {}", path.display()))
}

fn check_output_matches(path: &Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path)
        .with_context(|| format!("reading {} (run without --check to regenerate)", path.display()))?;
    if actual == expected {
        return Ok(());
    }
    Err(eyre!(
        "stale semantic scorecard output: {} (run `cargo xtask semantic-scorecard` to regenerate)",
        path.display()
    ))
}

fn render_status_markdown(artifact: &Artifact) -> String {
    let mut text = String::new();
    text.push_str("# Semantic Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&format!("Fixture family version: `{}`  \n", artifact.fixture_family_version));
    text.push_str(&format!("Fixtures loaded: `{}`\n\n", artifact.fixture_count));
    text.push_str("## Fixture IDs\n\n");
    for id in &artifact.fixture_ids {
        text.push_str(&format!("- `{id}`\n"));
    }

    text.push_str("\n## Metrics\n\n| Metric | Status | Value |\n|---|---|---:|\n");
    for (metric, row) in &artifact.rows {
        let value = row.value.map(|n| n.to_string()).unwrap_or_else(|| "n/a".to_string());
        text.push_str(&format!("| {metric} | {} | {value} |\n", row.status));
    }

    text.push_str("\n");
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify build_artifact preserves insertion order of fixture IDs and emits
    /// all expected metric rows as baseline_pending.
    #[test]
    fn build_artifact_preserves_fixture_order_and_emits_all_metrics() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase {
                    id: "b".to_string(),
                    family: "x".to_string(),
                    path: "b.pl".to_string(),
                },
                FixtureCase {
                    id: "a".to_string(),
                    family: "x".to_string(),
                    path: "a.pl".to_string(),
                },
            ],
        };
        let artifact = build_artifact(manifest);

        // Fixture IDs are preserved in input order (sorting is load_manifest's job).
        assert_eq!(artifact.measured_at, "deterministic-fixture-baseline");
        assert_eq!(artifact.fixture_ids, vec!["b".to_string(), "a".to_string()]);
        assert_eq!(artifact.fixture_count, 2);

        // All 11 canonical metric rows must be present and set to baseline_pending.
        assert_eq!(artifact.rows.len(), METRICS.len(), "row count must match METRICS constant");
        for &metric_name in METRICS {
            let row = artifact.rows.get(metric_name).unwrap_or_else(|| {
                panic!("expected metric '{metric_name}' to be present in artifact rows")
            });
            assert_eq!(
                row.status, "baseline_pending",
                "metric '{metric_name}' should be baseline_pending"
            );
            assert!(row.value.is_none(), "metric '{metric_name}' value should be None at baseline");
        }

        // Rows are in alphabetical order (BTreeMap) — spot-check boundary keys.
        let keys: Vec<&str> = artifact.rows.keys().map(String::as_str).collect();
        assert_eq!(keys.first().copied(), Some("completion_top1"));
        assert_eq!(keys.last().copied(), Some("undefined_symbol_false_positive_rate"));

        Ok(())
    }

    /// Verify load_manifest sorts fixtures by id, making the pipeline deterministic
    /// regardless of the order fixtures appear in the JSON file.
    #[test]
    fn manifest_load_sorts_fixtures() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"z","family":"f","path":"z.pl"},{"id":"a","family":"f","path":"a.pl"}]}"#,
        )?;
        let parsed = load_manifest(tmp.path())?;
        assert_eq!(parsed.fixtures[0].id, "a");
        assert_eq!(parsed.fixtures[1].id, "z");
        Ok(())
    }

    /// Verify that the full pipeline (load_manifest -> build_artifact) is stable
    /// across two calls with the same manifest data in a different order: the
    /// fixture IDs in the artifact must be identical both times.
    #[test]
    fn full_pipeline_is_stable_across_orderings() -> Result<()> {
        let tmp_fwd = tempfile::NamedTempFile::new()?;
        let tmp_rev = tempfile::NamedTempFile::new()?;
        // Same two fixtures, different JSON order.
        fs::write(
            tmp_fwd.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"alpha","family":"f","path":"a.pl"},{"id":"beta","family":"f","path":"b.pl"}]}"#,
        )?;
        fs::write(
            tmp_rev.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"beta","family":"f","path":"b.pl"},{"id":"alpha","family":"f","path":"a.pl"}]}"#,
        )?;

        let artifact_fwd = build_artifact(load_manifest(tmp_fwd.path())?);
        let artifact_rev = build_artifact(load_manifest(tmp_rev.path())?);

        // Both should produce the same sorted fixture list.
        assert_eq!(
            artifact_fwd.fixture_ids, artifact_rev.fixture_ids,
            "fixture IDs must be identical regardless of input order"
        );
        assert_eq!(artifact_fwd.fixture_ids, vec!["alpha".to_string(), "beta".to_string()]);
        Ok(())
    }
}
