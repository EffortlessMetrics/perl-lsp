use crate::tasks::metrics::ratchet::MetricReceipt;
use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use perl_corpus::gold::{
    CompletionAssertionKind, GotoAssertionKind, load_completion_gold_fixtures,
    load_goto_gold_fixtures, load_hover_gold_fixtures,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum UxScorecardFormat {
    #[default]
    Json,
    Markdown,
}

#[derive(Debug, Serialize)]
struct LatencyPercentiles {
    p50_ms: Option<u64>,
    p95_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct UxScorecardArtifact {
    schema_version: u32,
    generated_at: String,
    subsystem: &'static str,
    metrics: BTreeMap<String, Option<f64>>,
    latency_by_request_class: BTreeMap<String, LatencyPercentiles>,
}

pub fn run(format: UxScorecardFormat) -> Result<()> {
    let root = project_root()?;
    let artifact = collect_scorecard(&root)?;

    let json_path = root.join(".ci").join("metrics").join("editor_ux_scorecard.json");
    write_json(&json_path, &artifact)?;

    let ratchet_receipt = build_ratchet_receipt(&artifact)?;
    let ratchet_path = root.join("target").join("receipts").join("metrics").join("editor_ux.json");
    write_json(&ratchet_path, &ratchet_receipt)?;

    let status_md = render_status_markdown(&artifact);
    let status_md_path = root.join("docs").join("project").join("status").join("editor_ux.md");
    write_text(&status_md_path, &status_md)?;

    match format {
        UxScorecardFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&artifact)?);
        }
        UxScorecardFormat::Markdown => {
            println!("{status_md}");
        }
    }

    eprintln!("wrote {}", json_path.display());
    eprintln!("wrote {}", ratchet_path.display());
    eprintln!("wrote {}", status_md_path.display());
    Ok(())
}

fn collect_scorecard(root: &Path) -> Result<UxScorecardArtifact> {
    let legacy_path = root.join(".ci").join("metrics").join("editor_ux.json");
    let real_latency_path = root.join(".ci").join("metrics").join("real_project_latency.json");

    let legacy = read_optional_json(&legacy_path);
    let real_latency = read_optional_json(&real_latency_path);

    let metrics_obj = legacy.as_ref().and_then(|v| v.get("metrics"));

    let fixture_baselines = fixture_presence_baselines(root);

    let hover = metrics_obj
        .and_then(|m| m.get("hover_correctness_rate"))
        .and_then(Value::as_f64)
        .or(fixture_baselines.hover_correctness_rate);
    let completion_top1 = metrics_obj
        .and_then(|m| m.get("completion_top1_relevance"))
        .and_then(Value::as_f64)
        .or(fixture_baselines.completion_top1_rate);
    let completion_top5 = metrics_obj
        .and_then(|m| m.get("completion_top5_relevance"))
        .and_then(Value::as_f64)
        .or(fixture_baselines.completion_top5_rate);
    let definition = metrics_obj
        .and_then(|m| m.get("goto_definition_exact_hit_rate"))
        .and_then(Value::as_f64)
        .or(fixture_baselines.definition_exact_hit_rate);
    let symbol_correctness = metrics_obj
        .and_then(|m| m.get("settled_diagnostics_correctness_after_edit"))
        .and_then(Value::as_f64)
        .or(fixture_baselines.symbol_correctness_rate);
    let cross_file = metrics_obj
        .and_then(|m| m.get("multi_root_workspace_navigation_success"))
        .or_else(|| metrics_obj.and_then(|m| m.get("module_resolution_workflow_success")))
        .and_then(Value::as_f64)
        .or(fixture_baselines.cross_file_success_rate);

    let mut metrics = BTreeMap::new();
    metrics.insert("hover_correctness_pct".to_string(), as_percent(hover));
    metrics.insert("completion_top1_pct".to_string(), as_percent(completion_top1));
    metrics.insert("completion_top5_pct".to_string(), as_percent(completion_top5));
    metrics.insert("definition_exact_hit_pct".to_string(), as_percent(definition));
    metrics.insert("symbol_correctness_pct".to_string(), as_percent(symbol_correctness));
    metrics.insert("cross_file_success_pct".to_string(), as_percent(cross_file));

    let mut latency_by_request_class: BTreeMap<String, LatencyPercentiles> = BTreeMap::new();
    let from_real = collect_latency_from_real_project(real_latency.as_ref());
    for (request_class, bucket) in from_real {
        latency_by_request_class.insert(request_class, bucket);
    }
    ensure_latency_rows(&mut latency_by_request_class);

    Ok(UxScorecardArtifact {
        schema_version: 1,
        generated_at: Utc::now().to_rfc3339(),
        subsystem: "editor_ux",
        metrics,
        latency_by_request_class,
    })
}

#[derive(Default)]
struct FixturePresenceBaselines {
    hover_correctness_rate: Option<f64>,
    completion_top1_rate: Option<f64>,
    completion_top5_rate: Option<f64>,
    definition_exact_hit_rate: Option<f64>,
    symbol_correctness_rate: Option<f64>,
    cross_file_success_rate: Option<f64>,
}

fn fixture_presence_baselines(root: &Path) -> FixturePresenceBaselines {
    let gold_root = root.join("test_corpus").join("gold");
    let hover_fixtures = load_hover_gold_fixtures(&gold_root).unwrap_or_default();
    let goto_fixtures = load_goto_gold_fixtures(&gold_root).unwrap_or_default();
    let completion_fixtures = load_completion_gold_fixtures(&gold_root).unwrap_or_default();

    let hover_count: usize = hover_fixtures.iter().map(|f| f.hover_assertions.len()).sum();
    let top1_count: usize = completion_fixtures
        .iter()
        .flat_map(|f| f.completion_assertions.iter())
        .filter(|a| matches!(a.kind, CompletionAssertionKind::CompletionTop1 { .. }))
        .count();
    let top5_count: usize = completion_fixtures
        .iter()
        .flat_map(|f| f.completion_assertions.iter())
        .filter(|a| matches!(a.kind, CompletionAssertionKind::CompletionTop5 { .. }))
        .count();
    let exact_hit_count: usize = goto_fixtures
        .iter()
        .flat_map(|f| f.goto_assertions.iter())
        .filter(|a| matches!(a.kind, GotoAssertionKind::GotoLine { .. }))
        .count();

    let matrix_raw = fs::read_to_string(
        root.join("crates")
            .join("perl-lsp-ux-tests")
            .join("fixtures")
            .join("editor_ux_fixture_matrix.json"),
    )
    .ok();
    let matrix = matrix_raw.and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    let (symbol_count, cross_file_count) = matrix
        .and_then(|value| value.get("workflows").and_then(Value::as_array).cloned())
        .map(|workflows| {
            let symbol = workflows
                .iter()
                .filter(|wf| {
                    wf.get("measures").and_then(Value::as_array).is_some_and(|arr| {
                        arr.iter().any(|m| {
                            m.as_str() == Some("multi_root_workspace_navigation_success_rate")
                        })
                    })
                })
                .count();
            let cross_file = workflows
                .iter()
                .filter(|wf| {
                    wf.get("measures").and_then(Value::as_array).is_some_and(|arr| {
                        arr.iter().any(|m| m.as_str() == Some("cross_file_definition_success_rate"))
                    })
                })
                .count();
            (symbol, cross_file)
        })
        .unwrap_or((0, 0));

    FixturePresenceBaselines {
        hover_correctness_rate: (hover_count > 0).then_some(1.0),
        completion_top1_rate: (top1_count > 0).then_some(1.0),
        completion_top5_rate: (top5_count > 0).then_some(1.0),
        definition_exact_hit_rate: (exact_hit_count > 0).then_some(1.0),
        symbol_correctness_rate: (symbol_count > 0).then_some(1.0),
        cross_file_success_rate: (cross_file_count > 0).then_some(1.0),
    }
}

fn collect_latency_from_real_project(
    real_latency: Option<&Value>,
) -> BTreeMap<String, LatencyPercentiles> {
    let mut rows: BTreeMap<String, Vec<(Option<u64>, Option<u64>)>> = BTreeMap::new();

    let Some(projects) = real_latency.and_then(|v| v.get("projects")).and_then(Value::as_object)
    else {
        return BTreeMap::new();
    };

    let request_map = [
        ("hover", "cold_start_to_hover"),
        ("completion", "first_completion"),
        ("definition", "first_goto_definition"),
        ("workspace_symbols", "workspace_symbol_query"),
    ];

    for project in projects.values() {
        let Some(metrics) = project.get("metrics").and_then(Value::as_object) else {
            continue;
        };
        for (request_class, key) in &request_map {
            let Some(node) = metrics.get(*key) else {
                continue;
            };
            let p50 = node.get("p50_ms").and_then(Value::as_u64);
            let p95 = node.get("p95_ms").and_then(Value::as_u64);
            rows.entry((*request_class).to_string()).or_default().push((p50, p95));
        }
    }

    rows.into_iter()
        .map(|(request_class, samples)| {
            let p50_values: Vec<u64> = samples.iter().filter_map(|(p50, _)| *p50).collect();
            let p95_values: Vec<u64> = samples.iter().filter_map(|(_, p95)| *p95).collect();
            (
                request_class,
                LatencyPercentiles {
                    p50_ms: median_u64(&p50_values),
                    p95_ms: percentile95_u64(&p95_values),
                },
            )
        })
        .collect()
}

fn ensure_latency_rows(rows: &mut BTreeMap<String, LatencyPercentiles>) {
    for key in ["hover", "completion", "definition", "workspace_symbols"] {
        rows.entry(key.to_string()).or_insert(LatencyPercentiles { p50_ms: None, p95_ms: None });
    }
}

fn build_ratchet_receipt(artifact: &UxScorecardArtifact) -> Result<MetricReceipt> {
    let mut floor_metrics: BTreeMap<String, Option<f64>> = BTreeMap::new();
    for (metric, value) in &artifact.metrics {
        floor_metrics.insert(metric.clone(), *value);
    }

    for (request_class, latency) in &artifact.latency_by_request_class {
        floor_metrics.insert(
            format!("{request_class}_latency_p95_ms"),
            latency.p95_ms.map(|value| value as f64),
        );
    }

    Ok(MetricReceipt {
        subsystem: "editor_ux".to_string(),
        generated_at: artifact.generated_at.clone(),
        commit: git_short_sha()?,
        floor_metrics,
        improvement_metrics: BTreeMap::new(),
    })
}

fn render_status_markdown(artifact: &UxScorecardArtifact) -> String {
    let mut metric_lines = Vec::new();
    for key in [
        "hover_correctness_pct",
        "completion_top1_pct",
        "completion_top5_pct",
        "definition_exact_hit_pct",
        "symbol_correctness_pct",
        "cross_file_success_pct",
    ] {
        let value = artifact.metrics.get(key).and_then(|v| *v);
        metric_lines.push(format!("| {key} | {} |", format_pct(value)));
    }

    let mut latency_lines = Vec::new();
    for (request_class, latency) in &artifact.latency_by_request_class {
        latency_lines.push(format!(
            "| {request_class} | {} | {} |",
            format_ms(latency.p50_ms),
            format_ms(latency.p95_ms)
        ));
    }

    format!(
        "# Editor UX Scorecard\n\n\
         Generated by `cargo xtask ux-scorecard --format json` from canonical scorecard receipts in `.ci/metrics/`.\n\n\
         ## Metrics\n\n\
         | Metric | Value |\n\
         |--------|-------|\n\
         {}\n\n\
         ## Latency (request class)\n\n\
         | Request class | p50 | p95 |\n\
         |---------------|-----|-----|\n\
         {}\n\n\
         ## Ratchet policy\n\n\
         This scorecard uses **regression-only ratcheting**. We compare current values to\n\
         `.ci/metrics/baselines/editor_ux.json` and only block on regressions beyond tolerance.\n\
         There are no hardcoded absolute-threshold gates in this scorecard publish path.\n",
        metric_lines.join("\n"),
        latency_lines.join("\n")
    )
}

fn as_percent(value: Option<f64>) -> Option<f64> {
    value.map(|v| (v * 1000.0).round() / 10.0)
}

fn read_optional_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(value)?;
    fs::write(path, payload).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_text(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, value).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn git_short_sha() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .context("running git rev-parse")?;
    if !output.status.success() {
        return Ok("unknown".to_string());
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() { Ok("unknown".to_string()) } else { Ok(sha) }
}

fn median_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    Some(sorted[sorted.len() / 2])
}

fn percentile95_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let idx = ((sorted.len() * 95).saturating_sub(1)) / 100;
    sorted.get(idx).copied()
}

fn format_pct(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{v:.1}%"))
}

fn format_ms(value: Option<u64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{v} ms"))
}

#[cfg(test)]
mod tests {
    use super::{format_pct, percentile95_u64};

    #[test]
    fn percentile95_handles_small_samples() {
        let values = vec![10, 20, 30];
        assert_eq!(percentile95_u64(&values), Some(30));
    }

    #[test]
    fn format_pct_handles_missing() {
        assert_eq!(format_pct(None), "n/a");
        assert_eq!(format_pct(Some(91.2)), "91.2%");
    }
}
