//! Workspace/indexing scorecard aggregator.
//!
//! `cargo xtask metrics workspace-stats` reads runtime receipts from
//! `.ci/metrics/receipts/*.json` and prints a compact summary that combines
//! latency, reliability, cache-efficiency, and resource-usage signals.
//! This broadens the scorecard beyond a single lens and surfaces how diverse
//! the currently observed telemetry is.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct WorkspaceAggregate {
    sessions: usize,
    sessions_all_slos_met: usize,
    observed_operation_types: HashSet<String>,
    operations_total: u64,
    operations_success: u64,
    operations_error: u64,
    p95_values_ms: Vec<u64>,
    slo_compliance_rates: Vec<f64>,
    cache_hits: u64,
    cache_misses: u64,
    memory_samples_bytes: Vec<u64>,
}

#[derive(Debug)]
struct MetricDiversity {
    covered_families: usize,
    total_families: usize,
    missing_families: Vec<&'static str>,
}

/// Run `cargo xtask metrics workspace-stats`.
pub fn run() -> Result<()> {
    let root = project_root()?;
    let receipt_dir = root.join(".ci").join("metrics").join("receipts");

    let receipt_paths = receipt_files(&receipt_dir)?;
    if receipt_paths.is_empty() {
        bail!(
            "no workspace receipts found at {}\n  expected files like .ci/metrics/receipts/<session>.json",
            receipt_dir.display()
        );
    }

    let aggregate = aggregate_receipts(&receipt_paths)?;
    print_report(&receipt_dir, &aggregate);

    Ok(())
}

fn receipt_files(receipt_dir: &Path) -> Result<Vec<PathBuf>> {
    if !receipt_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(receipt_dir)
        .with_context(|| format!("reading receipt dir {}", receipt_dir.display()))?
    {
        let entry = entry.with_context(|| "reading receipt dir entry")?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn aggregate_receipts(receipt_paths: &[PathBuf]) -> Result<WorkspaceAggregate> {
    let mut aggregate = WorkspaceAggregate::default();

    for path in receipt_paths {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading workspace receipt {}", path.display()))?;
        let receipt: Value = serde_json::from_str(&raw)
            .with_context(|| format!("parsing workspace receipt {}", path.display()))?;

        aggregate.sessions += 1;

        if find_bool(&receipt, &["all_slos_met"]).unwrap_or(false) {
            aggregate.sessions_all_slos_met += 1;
        }

        if let Some(bytes) = find_u64(&receipt, &["total_memory_usage"]) {
            aggregate.memory_samples_bytes.push(bytes);
        }

        aggregate_slo_stats(&receipt, &mut aggregate);
        aggregate_cache_stats(&receipt, &mut aggregate);
    }

    Ok(aggregate)
}

fn aggregate_slo_stats(receipt: &Value, aggregate: &mut WorkspaceAggregate) {
    if let Some(map) = find_object(receipt, &["slo_stats"]) {
        for (operation, stats) in map {
            aggregate.observed_operation_types.insert(operation.to_string());

            let total = find_u64(stats, &["total_operations", "total_count", "count"]).unwrap_or(0);
            let success = find_u64(stats, &["success_count", "successful_operations", "successes"])
                .unwrap_or(0);
            let error =
                find_u64(stats, &["error_count", "failed_operations", "errors"]).unwrap_or(0);

            aggregate.operations_total += total;
            aggregate.operations_success += success;
            aggregate.operations_error += error;

            if let Some(p95) = find_u64(stats, &["p95_ms", "p95_latency_ms", "p95_latency"]) {
                aggregate.p95_values_ms.push(p95);
            }

            if let Some(rate) = find_f64(stats, &["slo_compliance_rate", "slo_met_rate"]) {
                aggregate.slo_compliance_rates.push(rate);
            }
        }
    }
}

fn aggregate_cache_stats(receipt: &Value, aggregate: &mut WorkspaceAggregate) {
    if let Some(map) = find_object(receipt, &["cache_stats"]) {
        for (_cache_name, stats) in map {
            aggregate.cache_hits += find_u64(stats, &["hits", "cache_hits"]).unwrap_or(0);
            aggregate.cache_misses += find_u64(stats, &["misses", "cache_misses"]).unwrap_or(0);
        }
    }
}

fn print_report(receipt_dir: &Path, aggregate: &WorkspaceAggregate) {
    let success_rate = ratio(aggregate.operations_success, aggregate.operations_total);
    let error_rate = ratio(aggregate.operations_error, aggregate.operations_total);
    let cache_hit_rate = ratio(aggregate.cache_hits, aggregate.cache_hits + aggregate.cache_misses);

    let avg_p95_ms = mean_u64(&aggregate.p95_values_ms);
    let max_p95_ms = aggregate.p95_values_ms.iter().copied().max();
    let avg_slo_compliance_rate = mean_f64(&aggregate.slo_compliance_rates);
    let avg_memory_mib = mean_u64(&aggregate.memory_samples_bytes).map(|v| v / (1024 * 1024));
    let sessions_all_slos_met_rate =
        ratio(aggregate.sessions_all_slos_met as u64, aggregate.sessions as u64);

    let diversity = metric_diversity(aggregate);

    println!("Workspace metrics summary");
    println!("=========================");
    println!("Receipt directory: {}", receipt_dir.display());
    println!("Receipts analyzed: {}", aggregate.sessions);
    println!();

    println!("Reliability");
    println!("-----------");
    println!("Operations observed: {}", aggregate.operations_total);
    println!("Success rate:        {}", fmt_pct(success_rate));
    println!("Error rate:          {}", fmt_pct(error_rate));
    println!("SLO-met sessions:    {}", fmt_pct(sessions_all_slos_met_rate));
    if let Some(rate) = avg_slo_compliance_rate {
        println!("Avg SLO compliance:  {}", fmt_pct(Some(rate)));
    } else {
        println!("Avg SLO compliance:  n/a");
    }
    println!();

    println!("Latency");
    println!("-------");
    println!("Operation types seen: {}", aggregate.observed_operation_types.len());
    println!("Average p95 latency:  {}", fmt_optional_ms(avg_p95_ms));
    println!("Max p95 latency:      {}", fmt_optional_ms(max_p95_ms));
    println!();

    println!("Efficiency + Resource");
    println!("---------------------");
    println!("Cache hit rate:       {}", fmt_pct(cache_hit_rate));
    println!("Avg memory footprint: {}", fmt_optional_mib(avg_memory_mib));
    println!();

    println!("Metric diversity");
    println!("----------------");
    println!(
        "Coverage: {}/{} families ({})",
        diversity.covered_families,
        diversity.total_families,
        fmt_pct(Some(diversity.covered_families as f64 / diversity.total_families as f64))
    );
    if diversity.missing_families.is_empty() {
        println!("Missing families: none");
    } else {
        println!("Missing families: {}", diversity.missing_families.join(", "));
    }
}

fn metric_diversity(aggregate: &WorkspaceAggregate) -> MetricDiversity {
    let mut covered = 0_usize;
    let mut missing = Vec::new();

    // 1) Operation volume.
    if aggregate.operations_total > 0 {
        covered += 1;
    } else {
        missing.push("operation_volume");
    }

    // 2) Latency.
    if !aggregate.p95_values_ms.is_empty() {
        covered += 1;
    } else {
        missing.push("latency");
    }

    // 3) Reliability (success/error outcomes).
    if aggregate.operations_success > 0 || aggregate.operations_error > 0 {
        covered += 1;
    } else {
        missing.push("reliability_outcomes");
    }

    // 4) SLO compliance.
    if aggregate.sessions_all_slos_met > 0 || !aggregate.slo_compliance_rates.is_empty() {
        covered += 1;
    } else {
        missing.push("slo_compliance");
    }

    // 5) Cache behavior.
    if aggregate.cache_hits > 0 || aggregate.cache_misses > 0 {
        covered += 1;
    } else {
        missing.push("cache_efficiency");
    }

    // 6) Memory/resource footprint.
    if !aggregate.memory_samples_bytes.is_empty() {
        covered += 1;
    } else {
        missing.push("resource_usage");
    }

    MetricDiversity { covered_families: covered, total_families: 6, missing_families: missing }
}

fn find_object<'a>(root: &'a Value, keys: &[&str]) -> Option<&'a serde_json::Map<String, Value>> {
    keys.iter().find_map(|key| root.get(key)?.as_object())
}

fn find_u64(root: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| root.get(key)?.as_u64())
}

fn find_bool(root: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| root.get(key)?.as_bool())
}

fn find_f64(root: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| root.get(key)?.as_f64())
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 { None } else { Some(numerator as f64 / denominator as f64) }
}

fn mean_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }

    let sum: u128 = values.iter().map(|v| u128::from(*v)).sum();
    Some((sum / values.len() as u128) as u64)
}

fn mean_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let sum: f64 = values.iter().sum();
    Some(sum / values.len() as f64)
}

fn fmt_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}%", v * 100.0),
        None => "n/a".to_string(),
    }
}

fn fmt_optional_ms(value: Option<u64>) -> String {
    match value {
        Some(v) => format!("{v} ms"),
        None => "n/a".to_string(),
    }
}

fn fmt_optional_mib(value: Option<u64>) -> String {
    match value {
        Some(v) => format!("{v} MiB"),
        None => "n/a".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_receipts_collects_diverse_metric_families() -> Result<()> {
        let temp_dir = tempfile::tempdir().with_context(|| "creating tempdir")?;
        let receipt_path = temp_dir.path().join("session-a.json");
        fs::write(
            &receipt_path,
            r#"{
              "all_slos_met": true,
              "total_memory_usage": 3145728,
              "slo_stats": {
                "definition_lookup": {
                  "total_operations": 100,
                  "success_count": 97,
                  "error_count": 3,
                  "p95_ms": 41,
                  "slo_compliance_rate": 0.96
                },
                "completion": {
                  "total_operations": 80,
                  "success_count": 78,
                  "error_count": 2,
                  "p95_ms": 65,
                  "slo_compliance_rate": 0.92
                }
              },
              "cache_stats": {
                "ast": {"hits": 120, "misses": 30},
                "symbol": {"hits": 75, "misses": 25}
              }
            }"#,
        )
        .with_context(|| "writing receipt fixture")?;

        let aggregate = aggregate_receipts(&[receipt_path])?;

        assert_eq!(aggregate.sessions, 1);
        assert_eq!(aggregate.operations_total, 180);
        assert_eq!(aggregate.operations_success, 175);
        assert_eq!(aggregate.operations_error, 5);
        assert_eq!(aggregate.cache_hits, 195);
        assert_eq!(aggregate.cache_misses, 55);
        assert_eq!(aggregate.observed_operation_types.len(), 2);

        let diversity = metric_diversity(&aggregate);
        assert_eq!(diversity.covered_families, 6);
        assert!(diversity.missing_families.is_empty());

        Ok(())
    }

    #[test]
    fn metric_diversity_reports_missing_families_for_sparse_receipts() {
        let aggregate = WorkspaceAggregate { sessions: 1, ..WorkspaceAggregate::default() };

        let diversity = metric_diversity(&aggregate);

        assert!(diversity.covered_families < diversity.total_families);
        assert!(diversity.missing_families.contains(&"latency"));
        assert!(diversity.missing_families.contains(&"cache_efficiency"));
        assert!(diversity.missing_families.contains(&"resource_usage"));
    }
}
