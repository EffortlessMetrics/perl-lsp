use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const FIXTURE_PATH: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_scorecard_fixture.json";
const METRICS_OUTPUT_PATH: &str = ".ci/metrics/editor_ux_scorecard.json";
const RATCHET_RECEIPT_PATH: &str = "target/receipts/metrics/editor_ux.json";
const STATUS_DOC_PATH: &str = "docs/project/status/editor_ux.md";

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Table,
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    scenarios: Vec<FixtureScenario>,
}

#[derive(Debug, Deserialize)]
struct FixtureScenario {
    id: String,
    hover_correct: Option<bool>,
    completion_top1_correct: Option<bool>,
    completion_top5_correct: Option<bool>,
    definition_exact_hit: Option<bool>,
    symbol_correct: Option<bool>,
    cross_file_success: Option<bool>,
    latency_ms_by_request: BTreeMap<String, Vec<u64>>,
}

#[derive(Debug, Serialize)]
struct ScorecardArtifact {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    fixture_source: String,
    scenario_count: usize,
    scenario_ids: Vec<String>,
    metrics: ScorecardMetrics,
}

#[derive(Debug, Serialize)]
struct ScorecardMetrics {
    hover_correctness_pct: Option<f64>,
    completion_top1_pct: Option<f64>,
    completion_top5_pct: Option<f64>,
    definition_exact_hit_pct: Option<f64>,
    symbol_correctness_pct: Option<f64>,
    cross_file_success_pct: Option<f64>,
    latency_ms_by_request: BTreeMap<String, LatencyPercentiles>,
}

#[derive(Debug, Serialize)]
struct LatencyPercentiles {
    p50_ms: u64,
    p95_ms: u64,
    samples: usize,
}

#[derive(Debug, Serialize)]
struct RatchetReceipt {
    subsystem: String,
    generated_at: String,
    commit: String,
    floor_metrics: BTreeMap<String, Option<f64>>,
    improvement_metrics: BTreeMap<String, Option<f64>>,
}

pub fn run(format: OutputFormat) -> Result<()> {
    let root = project_root()?;
    let fixture_path = root.join(FIXTURE_PATH);
    let fixture = load_fixture(&fixture_path)?;
    let artifact = build_artifact(&fixture, fixture_path.to_string_lossy().to_string());

    let metrics_output = root.join(METRICS_OUTPUT_PATH);
    write_json(&metrics_output, &artifact)?;

    let ratchet_output = root.join(RATCHET_RECEIPT_PATH);
    let ratchet_receipt = build_ratchet_receipt(&artifact);
    write_json(&ratchet_output, &ratchet_receipt)?;

    let status_output = root.join(STATUS_DOC_PATH);
    write_status_doc(&status_output, &artifact)?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&artifact)?),
        OutputFormat::Table => print_table(&artifact),
    }

    println!("Wrote {}", metrics_output.display());
    println!("Wrote {}", ratchet_output.display());
    println!("Wrote {}", status_output.display());
    Ok(())
}

fn load_fixture(path: &Path) -> Result<FixtureFile> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn build_artifact(fixture: &FixtureFile, fixture_source: String) -> ScorecardArtifact {
    let mut latency_samples: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for scenario in &fixture.scenarios {
        for (request_class, samples) in &scenario.latency_ms_by_request {
            latency_samples
                .entry(request_class.clone())
                .or_default()
                .extend(samples.iter().copied());
        }
    }

    let latency_ms_by_request = latency_samples
        .into_iter()
        .filter_map(|(request_class, mut samples)| {
            if samples.is_empty() {
                return None;
            }
            samples.sort_unstable();
            let p50 = percentile_nearest_rank(&samples, 50.0);
            let p95 = percentile_nearest_rank(&samples, 95.0);
            Some((
                request_class,
                LatencyPercentiles { p50_ms: p50, p95_ms: p95, samples: samples.len() },
            ))
        })
        .collect::<BTreeMap<_, _>>();

    ScorecardArtifact {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "editor_ux",
        fixture_source,
        scenario_count: fixture.scenarios.len(),
        scenario_ids: fixture.scenarios.iter().map(|scenario| scenario.id.clone()).collect(),
        metrics: ScorecardMetrics {
            hover_correctness_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.hover_correct),
            ),
            completion_top1_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.completion_top1_correct),
            ),
            completion_top5_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.completion_top5_correct),
            ),
            definition_exact_hit_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.definition_exact_hit),
            ),
            symbol_correctness_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.symbol_correct),
            ),
            cross_file_success_pct: percent_true(
                fixture.scenarios.iter().filter_map(|s| s.cross_file_success),
            ),
            latency_ms_by_request,
        },
    }
}

fn build_ratchet_receipt(artifact: &ScorecardArtifact) -> RatchetReceipt {
    let mut floor_metrics = BTreeMap::new();
    floor_metrics
        .insert("hover_correctness_pct".to_string(), artifact.metrics.hover_correctness_pct);
    floor_metrics.insert("completion_top1_pct".to_string(), artifact.metrics.completion_top1_pct);
    floor_metrics.insert("completion_top5_pct".to_string(), artifact.metrics.completion_top5_pct);
    floor_metrics
        .insert("definition_exact_hit_pct".to_string(), artifact.metrics.definition_exact_hit_pct);
    floor_metrics
        .insert("symbol_correctness_pct".to_string(), artifact.metrics.symbol_correctness_pct);
    floor_metrics
        .insert("cross_file_success_pct".to_string(), artifact.metrics.cross_file_success_pct);

    for (request_class, latency) in &artifact.metrics.latency_ms_by_request {
        floor_metrics
            .insert(format!("latency_{request_class}_p50_ms"), Some(latency.p50_ms as f64));
        floor_metrics
            .insert(format!("latency_{request_class}_p95_ms"), Some(latency.p95_ms as f64));
    }

    RatchetReceipt {
        subsystem: artifact.subsystem.to_string(),
        generated_at: artifact.measured_at.clone(),
        commit: current_commit(),
        floor_metrics,
        improvement_metrics: BTreeMap::new(),
    }
}

fn write_status_doc(path: &Path, artifact: &ScorecardArtifact) -> Result<()> {
    let mut text = String::new();
    text.push_str("# Editor UX Scorecard\n\n");
    text.push_str("Generated by `cargo xtask ux-scorecard --format json` from canonical harness fixtures.\n\n");
    text.push_str("## Top-line metrics\n\n");
    text.push_str("| Metric | Value |\n|---|---:|\n");
    text.push_str(&format!(
        "| hover correctness | {} |\n",
        fmt_pct(artifact.metrics.hover_correctness_pct)
    ));
    text.push_str(&format!(
        "| completion top-1 | {} |\n",
        fmt_pct(artifact.metrics.completion_top1_pct)
    ));
    text.push_str(&format!(
        "| completion top-5 | {} |\n",
        fmt_pct(artifact.metrics.completion_top5_pct)
    ));
    text.push_str(&format!(
        "| definition exact-hit | {} |\n",
        fmt_pct(artifact.metrics.definition_exact_hit_pct)
    ));
    text.push_str(&format!(
        "| symbol correctness | {} |\n",
        fmt_pct(artifact.metrics.symbol_correctness_pct)
    ));
    text.push_str(&format!(
        "| cross-file success | {} |\n",
        fmt_pct(artifact.metrics.cross_file_success_pct)
    ));

    text.push_str("\n## Latency by request class\n\n");
    text.push_str("| Request class | p50 (ms) | p95 (ms) | Samples |\n|---|---:|---:|---:|\n");
    for (request_class, latency) in &artifact.metrics.latency_ms_by_request {
        text.push_str(&format!(
            "| {request_class} | {} | {} | {} |\n",
            latency.p50_ms, latency.p95_ms, latency.samples
        ));
    }

    text.push_str("\n## Regression-only ratchet policy\n\n");
    text.push_str(
        "Editor UX uses floor-metric ratcheting only: current values must not regress below `.ci/metrics/baselines/editor_ux.json`.\n",
    );
    text.push_str(
        "No absolute threshold gate is enforced here; floors rise only through explicit baseline promotion after stable wins.\n\n",
    );
    text.push_str("- Check gate: `cargo xtask metrics ratchet-check editor_ux`\n");
    text.push_str(
        "- Promote floor after stable wins: `cargo xtask metrics promote-baseline editor_ux`\n",
    );

    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn print_table(artifact: &ScorecardArtifact) {
    println!("Editor UX scorecard ({})", artifact.measured_at);
    println!("scenarios: {}", artifact.scenario_count);
    println!(
        "hover={} completion_top1={} completion_top5={} definition={} symbol={} cross_file={}",
        fmt_pct(artifact.metrics.hover_correctness_pct),
        fmt_pct(artifact.metrics.completion_top1_pct),
        fmt_pct(artifact.metrics.completion_top5_pct),
        fmt_pct(artifact.metrics.definition_exact_hit_pct),
        fmt_pct(artifact.metrics.symbol_correctness_pct),
        fmt_pct(artifact.metrics.cross_file_success_pct)
    );
    println!("latency:");
    for (request_class, latency) in &artifact.metrics.latency_ms_by_request {
        println!(
            "  {}: p50={}ms p95={}ms (n={})",
            request_class, latency.p50_ms, latency.p95_ms, latency.samples
        );
    }
}

fn write_json<T: Serialize>(path: &Path, payload: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(payload)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn current_commit() -> String {
    let output = std::process::Command::new("git").args(["rev-parse", "--short", "HEAD"]).output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

fn percentile_nearest_rank(sorted_samples: &[u64], percentile: f64) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }

    let rank = ((percentile / 100.0) * sorted_samples.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[idx]
}

fn percent_true<I>(iter: I) -> Option<f64>
where
    I: Iterator<Item = bool>,
{
    let mut total = 0usize;
    let mut trues = 0usize;

    for value in iter {
        total += 1;
        if value {
            trues += 1;
        }
    }

    if total == 0 {
        return None;
    }

    Some((trues as f64 / total as f64) * 100.0)
}

fn fmt_pct(value: Option<f64>) -> String {
    value.map(|pct| format!("{pct:.1}%")).unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;

    #[test]
    fn percentile_uses_nearest_rank() {
        let values = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(percentile_nearest_rank(&values, 50.0), 50);
        assert_eq!(percentile_nearest_rank(&values, 95.0), 100);
    }

    #[test]
    fn build_artifact_computes_percentages() -> Result<()> {
        let fixture = FixtureFile {
            scenarios: vec![
                FixtureScenario {
                    id: "s1".to_string(),
                    hover_correct: Some(true),
                    completion_top1_correct: Some(true),
                    completion_top5_correct: Some(true),
                    definition_exact_hit: Some(false),
                    symbol_correct: Some(true),
                    cross_file_success: Some(true),
                    latency_ms_by_request: BTreeMap::from([(
                        "hover".to_string(),
                        vec![10, 20, 30],
                    )]),
                },
                FixtureScenario {
                    id: "s2".to_string(),
                    hover_correct: Some(false),
                    completion_top1_correct: Some(false),
                    completion_top5_correct: Some(true),
                    definition_exact_hit: Some(true),
                    symbol_correct: Some(false),
                    cross_file_success: Some(true),
                    latency_ms_by_request: BTreeMap::from([(
                        "hover".to_string(),
                        vec![40, 50, 60],
                    )]),
                },
            ],
        };

        let artifact = build_artifact(&fixture, "fixture.json".to_string());
        assert_eq!(artifact.metrics.hover_correctness_pct, Some(50.0));
        assert_eq!(artifact.metrics.completion_top1_pct, Some(50.0));
        assert_eq!(artifact.metrics.completion_top5_pct, Some(100.0));
        assert_eq!(artifact.metrics.definition_exact_hit_pct, Some(50.0));
        assert_eq!(artifact.metrics.symbol_correctness_pct, Some(50.0));
        assert_eq!(artifact.metrics.cross_file_success_pct, Some(100.0));
        assert_eq!(artifact.metrics.latency_ms_by_request["hover"].p50_ms, 30);
        assert_eq!(artifact.metrics.latency_ms_by_request["hover"].p95_ms, 60);
        Ok(())
    }

    #[test]
    fn run_errors_when_fixture_missing() -> Result<()> {
        let err = load_fixture(Path::new("/tmp/this/does/not/exist.json")).expect_err("must fail");
        if err.to_string().contains("reading") {
            Ok(())
        } else {
            Err(eyre!("unexpected error: {err}"))
        }
    }
}
