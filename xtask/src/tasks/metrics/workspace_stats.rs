//! Workspace index memory and timing statistics subcommand.
//!
//! Reads runtime metric receipts from `.ci/metrics/receipts/*.json` and prints
//! an aggregate summary suitable for local scorecard review.

use crate::utils::project_root;
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
struct ReceiptSloStats {
    #[serde(default)]
    total_count: u64,
    #[serde(default)]
    success_count: u64,
    #[serde(default)]
    error_count: u64,
    #[serde(default)]
    p95_duration_ms: Option<f64>,
    #[serde(default)]
    p95_ms: Option<f64>,
}

impl ReceiptSloStats {
    fn p95_ms(&self) -> Option<f64> {
        self.p95_duration_ms.or(self.p95_ms)
    }
}

#[derive(Debug, Clone, Default)]
struct WorkspaceReceipt {
    total_memory_usage: Option<u64>,
    all_slos_met: Option<bool>,
    slo_stats: BTreeMap<String, ReceiptSloStats>,
}

#[derive(Debug, Clone, Default)]
struct AggregatedSlo {
    total_count: u64,
    success_count: u64,
    error_count: u64,
    p95_samples: Vec<f64>,
}

/// Run `cargo xtask metrics workspace-stats`.
pub fn run() -> Result<()> {
    let root = project_root()?;
    let receipts_dir = root.join(".ci").join("metrics").join("receipts");
    let receipts = load_receipts(&receipts_dir)?;

    if receipts.is_empty() {
        println!("No workspace receipts found at {}", receipts_dir.display());
        println!("Expected files: .ci/metrics/receipts/*.json");
        return Ok(());
    }

    print_summary(&receipts);
    Ok(())
}

fn load_receipts(receipts_dir: &Path) -> Result<Vec<WorkspaceReceipt>> {
    if !receipts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in fs::read_dir(receipts_dir)
        .wrap_err_with(|| format!("reading {}", receipts_dir.display()))?
    {
        let entry = entry.wrap_err("reading receipt directory entry")?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .wrap_err_with(|| format!("reading receipt file {}", path.display()))?;
        let value: Value = serde_json::from_str(&content)
            .wrap_err_with(|| format!("parsing receipt JSON {}", path.display()))?;

        if let Some(receipt) = parse_receipt(&value) {
            out.push(receipt);
        }
    }

    Ok(out)
}

fn parse_receipt(value: &Value) -> Option<WorkspaceReceipt> {
    let statistics = value.get("statistics").unwrap_or(value);
    let total_memory_usage = statistics
        .get("total_memory_usage")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| value.get("total_memory_usage").and_then(serde_json::Value::as_u64));

    let all_slos_met = statistics
        .get("all_slos_met")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| value.get("all_slos_met").and_then(serde_json::Value::as_bool));

    let slo_root = statistics.get("slo_stats").or_else(|| value.get("slo_stats"));
    let slo_stats = slo_root
        .and_then(serde_json::Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| {
                    serde_json::from_value::<ReceiptSloStats>(v.clone())
                        .ok()
                        .map(|stats| (k.clone(), stats))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    if total_memory_usage.is_none() && all_slos_met.is_none() && slo_stats.is_empty() {
        return None;
    }

    Some(WorkspaceReceipt { total_memory_usage, all_slos_met, slo_stats })
}

fn print_summary(receipts: &[WorkspaceReceipt]) {
    let sessions = receipts.len() as u64;
    let memory_samples: Vec<u64> = receipts.iter().filter_map(|r| r.total_memory_usage).collect();
    let slos_met_sessions = receipts.iter().filter(|r| r.all_slos_met == Some(true)).count() as u64;

    let mut by_operation: BTreeMap<String, AggregatedSlo> = BTreeMap::new();
    for receipt in receipts {
        for (operation, stats) in &receipt.slo_stats {
            let agg = by_operation.entry(operation.clone()).or_default();
            agg.total_count = agg.total_count.saturating_add(stats.total_count);
            agg.success_count = agg.success_count.saturating_add(stats.success_count);
            agg.error_count = agg.error_count.saturating_add(stats.error_count);
            if let Some(p95) = stats.p95_ms() {
                agg.p95_samples.push(p95);
            }
        }
    }

    println!("Workspace metrics summary ({} session receipts)", sessions);
    println!("{}", "-".repeat(72));

    if memory_samples.is_empty() {
        println!("Memory usage: n/a");
    } else {
        let total_memory: u128 = memory_samples.iter().map(|v| *v as u128).sum();
        let avg_bytes = total_memory / memory_samples.len() as u128;
        println!(
            "Memory usage: avg {:.2} MiB across {} sample(s)",
            (avg_bytes as f64) / (1024.0 * 1024.0),
            memory_samples.len()
        );
    }

    if slos_met_sessions > 0 {
        println!("SLO sessions fully met: {slos_met_sessions}/{sessions}");
    } else {
        println!("SLO sessions fully met: 0/{sessions} (or unavailable)");
    }

    if by_operation.is_empty() {
        println!("No per-operation SLO stats found in receipts.");
        return;
    }

    println!();
    println!(
        "{:<24} {:>10} {:>10} {:>10} {:>12}",
        "Operation", "Total", "Success%", "Errors", "Avg p95 ms"
    );
    println!("{}", "-".repeat(72));

    for (operation, agg) in by_operation {
        let success_rate = if agg.total_count == 0 {
            0.0
        } else {
            (agg.success_count as f64 / agg.total_count as f64) * 100.0
        };

        let avg_p95 = if agg.p95_samples.is_empty() {
            String::from("n/a")
        } else {
            let sum: f64 = agg.p95_samples.iter().sum();
            let avg = sum / agg.p95_samples.len() as f64;
            format!("{avg:.2}")
        };

        println!(
            "{:<24} {:>10} {:>9.1}% {:>10} {:>12}",
            operation, agg.total_count, success_rate, agg.error_count, avg_p95
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_nested_statistics_shape() -> Result<()> {
        let value = serde_json::json!({
            "statistics": {
                "total_memory_usage": 1048576,
                "all_slos_met": true,
                "slo_stats": {
                    "definition_lookup": {
                        "total_count": 10,
                        "success_count": 9,
                        "error_count": 1,
                        "p95_duration_ms": 12.5
                    }
                }
            }
        });

        let receipt =
            parse_receipt(&value).ok_or_else(|| color_eyre::eyre::eyre!("receipt should parse"))?;
        assert_eq!(receipt.total_memory_usage, Some(1_048_576));
        assert_eq!(receipt.all_slos_met, Some(true));
        assert!(receipt.slo_stats.contains_key("definition_lookup"));
        Ok(())
    }

    #[test]
    fn parses_flat_statistics_shape() -> Result<()> {
        let value = serde_json::json!({
            "total_memory_usage": 2048,
            "all_slos_met": false,
            "slo_stats": {
                "hover": {
                    "total_count": 2,
                    "success_count": 2,
                    "error_count": 0,
                    "p95_ms": 3.5
                }
            }
        });

        let receipt =
            parse_receipt(&value).ok_or_else(|| color_eyre::eyre::eyre!("receipt should parse"))?;
        assert_eq!(receipt.total_memory_usage, Some(2048));
        assert_eq!(receipt.all_slos_met, Some(false));
        assert_eq!(receipt.slo_stats.get("hover").map(|s| s.total_count), Some(2));
        Ok(())
    }

    #[test]
    fn loads_only_json_receipts() -> Result<()> {
        let temp = TempDir::new()?;
        let dir = temp.path();

        fs::write(dir.join("session-1.json"), r#"{"statistics":{"total_memory_usage": 1000}}"#)?;
        fs::write(dir.join("README.txt"), "ignore me")?;

        let receipts = load_receipts(dir)?;
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].total_memory_usage, Some(1000));
        Ok(())
    }
}
