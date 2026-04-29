use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils;

const DEFAULT_MANIFEST: &str = "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/manifest.toml";

#[derive(Debug, Clone, Deserialize)]
struct SemanticManifest {
    fixtures: Vec<SemanticFixture>,
    metrics: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SemanticFixture {
    id: String,
    family: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum MetricStatus {
    BaselinePending,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FixtureMetric {
    metric: String,
    status: MetricStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FixtureScore {
    fixture_id: String,
    family: String,
    description: String,
    metrics: Vec<FixtureMetric>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SemanticScorecard {
    manifest_path: String,
    fixture_count: usize,
    metric_count: usize,
    scores: Vec<FixtureScore>,
}

pub fn run(manifest: Option<PathBuf>) -> Result<()> {
    let root = utils::project_root()?;
    let manifest_path = manifest.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let scorecard = load_scorecard(&manifest_path)?;
    println!("{}", serde_json::to_string_pretty(&scorecard)?);
    Ok(())
}

pub fn load_scorecard(manifest_path: &Path) -> Result<SemanticScorecard> {
    let raw = fs::read_to_string(manifest_path)
        .wrap_err_with(|| format!("reading semantic scorecard manifest: {}", manifest_path.display()))?;
    let mut manifest: SemanticManifest = toml::from_str(&raw).wrap_err("parsing semantic scorecard manifest TOML")?;

    manifest.fixtures.sort_by(|a, b| a.id.cmp(&b.id));
    manifest.metrics.sort();

    let scores = manifest
        .fixtures
        .into_iter()
        .map(|fixture| FixtureScore {
            fixture_id: fixture.id,
            family: fixture.family,
            description: fixture.description,
            metrics: manifest
                .metrics
                .iter()
                .cloned()
                .map(|metric| FixtureMetric { metric, status: MetricStatus::BaselinePending })
                .collect(),
        })
        .collect::<Vec<_>>();

    Ok(SemanticScorecard {
        manifest_path: manifest_path.strip_prefix(utils::project_root()?).unwrap_or(manifest_path).display().to_string(),
        fixture_count: scores.len(),
        metric_count: manifest.metrics.len(),
        scores,
    })
}
