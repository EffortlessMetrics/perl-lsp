use crate::utils::project_root;
use color_eyre::eyre::{Context, Result};
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
    family: String,
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

pub fn run(manifest: Option<PathBuf>, output: Option<PathBuf>, status_md: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let manifest_path = root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let manifest = load_manifest(&manifest_path)?;
    let artifact = build_artifact(manifest);

    write_json(&output_path, &artifact)?;
    fs::write(&status_path, render_status_markdown(&artifact))
        .with_context(|| format!("writing {}", status_path.display()))?;

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
    let fixture_ids = manifest.fixtures.iter().map(|fixture| fixture.id.clone()).collect::<Vec<_>>();
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

fn write_json(path: &Path, artifact: &Artifact) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(artifact)?;
    fs::write(path, format!("{payload}\n")).with_context(|| format!("writing {}", path.display()))
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

    #[test]
    fn scorecard_is_deterministic() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase { id: "b".to_string(), family: "x".to_string(), path: "b.pl".to_string() },
                FixtureCase { id: "a".to_string(), family: "x".to_string(), path: "a.pl".to_string() },
            ],
        };
        let artifact = build_artifact(manifest);
        assert_eq!(artifact.measured_at, "deterministic-fixture-baseline");
        assert_eq!(artifact.fixture_ids, vec!["b".to_string(), "a".to_string()]);
        assert!(artifact.rows.values().all(|row| row.status == "baseline_pending"));
        Ok(())
    }

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
}
