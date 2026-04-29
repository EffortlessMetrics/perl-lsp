use std::{collections::BTreeMap, fs, path::PathBuf};

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_MANIFEST: &str = "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/fixtures.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticFixture {
    pub id: String,
    pub family: String,
    pub description: String,
    pub entry_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticScorecardManifest {
    pub schema_version: u32,
    pub fixture_set: String,
    pub fixtures: Vec<SemanticFixture>,
    pub metrics: BTreeMap<String, MetricDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricDefinition {
    pub status: String,
    pub unit: String,
    pub value: Option<f64>,
    pub notes: String,
}

pub fn run(manifest: Option<PathBuf>, json: bool) -> Result<()> {
    let root = crate::utils::project_root()?;
    let manifest_path = manifest.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let scorecard = load_manifest(&manifest_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&scorecard)?);
    } else {
        println!("semantic fixture set: {}", scorecard.fixture_set);
        println!("schema version: {}", scorecard.schema_version);
        println!("fixtures: {}", scorecard.fixtures.len());
        for fixture in &scorecard.fixtures {
            println!("- {} [{}] {}", fixture.id, fixture.family, fixture.description);
        }
        println!("\nmetrics:");
        for (name, metric) in &scorecard.metrics {
            let rendered = metric
                .value
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "baseline pending".to_string());
            println!("- {name}: {} ({rendered} {})", metric.status, metric.unit);
        }
    }

    Ok(())
}

fn load_manifest(path: &PathBuf) -> Result<SemanticScorecardManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading semantic scorecard manifest at {}", path.display()))?;
    let manifest: SemanticScorecardManifest =
        serde_json::from_str(&raw).context("parsing semantic scorecard manifest")?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_required_metric_keys() -> Result<()> {
        let root = crate::utils::project_root()?;
        let manifest = load_manifest(&root.join(DEFAULT_MANIFEST))?;
        let required = [
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

        for key in required {
            assert!(manifest.metrics.contains_key(key), "missing metric: {key}");
        }

        Ok(())
    }

    #[test]
    fn fixture_ids_are_deterministically_sorted() -> Result<()> {
        let root = crate::utils::project_root()?;
        let manifest = load_manifest(&root.join(DEFAULT_MANIFEST))?;
        let ids: Vec<&str> = manifest.fixtures.iter().map(|fixture| fixture.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        Ok(())
    }
}
