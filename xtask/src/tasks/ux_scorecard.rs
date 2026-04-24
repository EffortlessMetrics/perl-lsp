use crate::utils::project_root;
use chrono::Utc;
use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SOURCE_FIXTURE: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_scorecard_fixture.json";
const METRICS_OUTPUT: &str = ".ci/metrics/editor_ux_scorecard.json";
const STATUS_OUTPUT: &str = "docs/project/status/editor_ux.json";

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct ScenarioFixture {
    scenario_id: String,
    hover_correct: Option<bool>,
    completion_top1_correct: Option<bool>,
    completion_top5_correct: Option<bool>,
    definition_exact_hit: Option<bool>,
    symbol_correct: Option<bool>,
    cross_file_success: Option<bool>,
    #[serde(default)]
    latency_samples_ms: BTreeMap<String, Vec<f64>>,
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    schema_version: u32,
    scenarios: Vec<ScenarioFixture>,
}

#[derive(Debug, Serialize, Clone)]
struct LatencyPercentiles {
    p50_ms: f64,
    p95_ms: f64,
}

#[derive(Debug, Serialize, Clone)]
struct UxScorecardArtifact {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    scenario_count: usize,
    metrics_pct: BTreeMap<String, Option<f64>>,
    latency_ms_by_request_class: BTreeMap<String, LatencyPercentiles>,
    generated_at: String,
    commit: String,
    floor_metrics: BTreeMap<String, Option<f64>>,
    improvement_metrics: BTreeMap<String, Option<f64>>,
    provenance: BTreeMap<String, String>,
    ratchet_policy: BTreeMap<String, String>,
}

pub fn run(format: OutputFormat) -> Result<()> {
    let root = project_root()?;
    let artifact = build_artifact(&root)?;
    write_json(&root.join(METRICS_OUTPUT), &artifact)?;

    let status = build_status_json(&artifact);
    fs::write(root.join(STATUS_OUTPUT), serde_json::to_string_pretty(&status)?)
        .with_context(|| format!("writing {STATUS_OUTPUT}"))?;

    match format {
        OutputFormat::Human => print_human(&artifact),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&artifact)?),
    }

    Ok(())
}

pub fn generated_status_json(root: &Path) -> Result<String> {
    let artifact = build_artifact(root)?;
    let status = build_status_json(&artifact);
    serde_json::to_string_pretty(&status).context("serializing editor UX status JSON")
}

pub fn load_compact_summary(root: &Path) -> Option<serde_json::Value> {
    let path = root.join(METRICS_OUTPUT);
    let raw = fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(json!({
        "hover_correctness_pct": parsed.get("metrics_pct")?.get("hover_correctness_pct")?.clone(),
        "completion_top1_pct": parsed.get("metrics_pct")?.get("completion_top1_pct")?.clone(),
        "completion_top5_pct": parsed.get("metrics_pct")?.get("completion_top5_pct")?.clone(),
        "definition_exact_hit_pct": parsed.get("metrics_pct")?.get("definition_exact_hit_pct")?.clone(),
        "symbol_correctness_pct": parsed.get("metrics_pct")?.get("symbol_correctness_pct")?.clone(),
        "cross_file_success_pct": parsed.get("metrics_pct")?.get("cross_file_success_pct")?.clone(),
    }))
}

fn build_artifact(root: &Path) -> Result<UxScorecardArtifact> {
    let fixture_path = root.join(SOURCE_FIXTURE);
    let fixture_raw = fs::read_to_string(&fixture_path)
        .with_context(|| format!("reading {}", fixture_path.display()))?;
    let fixture: FixtureFile = serde_json::from_str(&fixture_raw)
        .with_context(|| format!("parsing {}", fixture_path.display()))?;

    if fixture.schema_version != 1 {
        bail!("unsupported schema_version {} in {}", fixture.schema_version, SOURCE_FIXTURE);
    }

    let mut latency_samples: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for scenario in &fixture.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            bail!("scenario_id must not be empty in {}", SOURCE_FIXTURE);
        }
        for (class, samples) in &scenario.latency_samples_ms {
            let bucket = latency_samples.entry(class.clone()).or_default();
            for sample in samples {
                bucket.push(*sample);
            }
        }
    }

    let latency_ms_by_request_class = latency_samples
        .into_iter()
        .filter_map(|(class, samples)| {
            let p50 = percentile(&samples, 0.5)?;
            let p95 = percentile(&samples, 0.95)?;
            Some((class, LatencyPercentiles { p50_ms: p50, p95_ms: p95 }))
        })
        .collect::<BTreeMap<_, _>>();

    let mut metrics_pct = BTreeMap::new();
    metrics_pct.insert(
        "hover_correctness_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.hover_correct)),
    );
    metrics_pct.insert(
        "completion_top1_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.completion_top1_correct)),
    );
    metrics_pct.insert(
        "completion_top5_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.completion_top5_correct)),
    );
    metrics_pct.insert(
        "definition_exact_hit_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.definition_exact_hit)),
    );
    metrics_pct.insert(
        "symbol_correctness_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.symbol_correct)),
    );
    metrics_pct.insert(
        "cross_file_success_pct".to_string(),
        percent_true(fixture.scenarios.iter().filter_map(|s| s.cross_file_success)),
    );

    let mut floor_metrics = metrics_pct.clone();
    for (class, latency) in &latency_ms_by_request_class {
        floor_metrics.insert(format!("latency_{class}_p50_ms"), Some(latency.p50_ms));
        floor_metrics.insert(format!("latency_{class}_p95_ms"), Some(latency.p95_ms));
    }

    let mut provenance = BTreeMap::new();
    provenance.insert("fixture".to_string(), SOURCE_FIXTURE.to_string());
    provenance
        .insert("generator".to_string(), "cargo xtask ux-scorecard --format json".to_string());

    let mut ratchet_policy = BTreeMap::new();
    ratchet_policy.insert("mode".to_string(), "regression_only".to_string());
    ratchet_policy.insert(
        "baseline".to_string(),
        ".ci/metrics/baselines/editor_ux.json (checked via cargo xtask metrics ratchet-check editor_ux --current .ci/metrics/editor_ux_scorecard.json)".to_string(),
    );

    let measured_at = Utc::now().to_rfc3339();

    Ok(UxScorecardArtifact {
        schema_version: 1,
        measured_at: measured_at.clone(),
        subsystem: "editor_ux",
        scenario_count: fixture.scenarios.len(),
        metrics_pct,
        latency_ms_by_request_class,
        generated_at: measured_at,
        commit: "HEAD".to_string(),
        floor_metrics,
        improvement_metrics: BTreeMap::new(),
        provenance,
        ratchet_policy,
    })
}

fn build_status_json(artifact: &UxScorecardArtifact) -> serde_json::Value {
    json!({
        "schema_version": 2,
        "receipt_kind": "measured_scorecard",
        "scorecard": "editor_ux",
        "measured_at": artifact.measured_at,
        "scenario_count": artifact.scenario_count,
        "metrics_pct": artifact.metrics_pct,
        "latency_ms_by_request_class": artifact.latency_ms_by_request_class,
        "provenance": artifact.provenance,
        "ratchet_policy": artifact.ratchet_policy,
    })
}

fn write_json(path: &Path, artifact: &UxScorecardArtifact) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(artifact)?;
    fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}

fn percentile(samples: &[f64], quantile: f64) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let index = ((sorted.len() - 1) as f64 * quantile).round() as usize;
    sorted.get(index).copied()
}

fn percent_true<I>(iter: I) -> Option<f64>
where
    I: Iterator<Item = bool>,
{
    let mut total = 0usize;
    let mut positives = 0usize;
    for value in iter {
        total += 1;
        if value {
            positives += 1;
        }
    }
    if total == 0 {
        return None;
    }
    Some((positives as f64 / total as f64) * 100.0)
}

fn print_human(artifact: &UxScorecardArtifact) {
    println!("Editor UX scorecard ({} scenarios)", artifact.scenario_count);
    for (name, value) in &artifact.metrics_pct {
        match value {
            Some(v) => println!("  {name}: {v:.1}%"),
            None => println!("  {name}: n/a"),
        }
    }
    for (class, lat) in &artifact.latency_ms_by_request_class {
        println!("  {class}: p50={:.1}ms p95={:.1}ms", lat.p50_ms, lat.p95_ms);
    }
    println!("wrote {} and {}", METRICS_OUTPUT, STATUS_OUTPUT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_true_handles_missing_measurements() {
        assert_eq!(percent_true(Vec::<bool>::new().into_iter()), None);
        assert_eq!(percent_true(vec![true, false, true].into_iter()), Some(66.66666666666666));
    }

    #[test]
    fn percentile_works_for_small_samples() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(percentile(&samples, 0.5), Some(30.0));
        assert_eq!(percentile(&samples, 0.95), Some(50.0));
    }

    #[test]
    fn load_compact_summary_returns_none_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(load_compact_summary(tmp.path()).is_none());
    }
}
