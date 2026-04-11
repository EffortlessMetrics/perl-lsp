//! Parser benchmark statistics subcommand.
//!
//! Reads the most-recently-modified benchmark JSON from `benchmarks/results/`
//! (or an explicit `--input` path), emits a human-readable table, and
//! optionally writes `.ci/metrics/parser.json`.

use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Benchmark JSON schema
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BenchmarkFile {
    pub benchmarks: BTreeMap<String, BenchmarkEntry>,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkEntry {
    pub mean: Option<TimingStat>,
    pub median: Option<TimingStat>,
    pub std_dev: Option<TimingStat>,
    pub source_lines: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TimingStat {
    pub nanoseconds: f64,
    pub microseconds: f64,
}

// ---------------------------------------------------------------------------
// Output schema for .ci/metrics/parser.json
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ParserMetricsOutput {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    metrics: ParserMetrics,
}

#[derive(Debug, Serialize)]
struct ParserMetrics {
    benchmark_count: usize,
    slowest: Vec<SlowEntry>,
}

#[derive(Debug, Serialize)]
struct SlowEntry {
    name: String,
    mean_us: f64,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run `cargo xtask metrics parser-stats`.
pub fn run(input: Option<PathBuf>, json: bool) -> Result<()> {
    let root = project_root()?;

    // Resolve input path: explicit or most-recent in benchmarks/results/
    let input_path = match input {
        Some(p) => p,
        None => find_latest_benchmark_json(&root)?,
    };

    let raw = fs::read_to_string(&input_path)
        .with_context(|| format!("reading benchmark file: {}", input_path.display()))?;

    let file: BenchmarkFile =
        serde_json::from_str(&raw).with_context(|| "parsing benchmark JSON")?;

    print_table(&file);

    if json {
        write_json_output(&root, &file)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return the most-recently modified `*.json` in `benchmarks/results/`.
fn find_latest_benchmark_json(root: &Path) -> Result<PathBuf> {
    let results_dir = root.join("benchmarks").join("results");
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&results_dir)
        .with_context(|| format!("reading {}", results_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, e.path()))
        })
        .collect();

    candidates.sort_by(|a, b| b.0.cmp(&a.0)); // newest first

    candidates.into_iter().map(|(_, p)| p).next().ok_or_else(|| {
        color_eyre::eyre::eyre!("no *.json files found in {}", results_dir.display())
    })
}

/// Print a human-readable table sorted by mean descending.
/// Entries without timing data (status: incomplete / not_run) are listed at the bottom.
fn print_table(file: &BenchmarkFile) {
    let mut timed: Vec<(&str, &BenchmarkEntry)> = file
        .benchmarks
        .iter()
        .filter(|(_, v)| v.mean.is_some())
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    timed.sort_by(|a, b| {
        let an = a.1.mean.as_ref().map(|s| s.nanoseconds).unwrap_or(0.0);
        let bn = b.1.mean.as_ref().map(|s| s.nanoseconds).unwrap_or(0.0);
        bn.partial_cmp(&an).unwrap_or(std::cmp::Ordering::Equal)
    });

    let no_timing: Vec<(&str, &BenchmarkEntry)> = file
        .benchmarks
        .iter()
        .filter(|(_, v)| v.mean.is_none())
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    println!(
        "{:<40} {:>12} {:>12} {:>12} {:>8}",
        "Benchmark", "Mean (µs)", "Median (µs)", "Std Dev (µs)", "LOC"
    );
    println!("{}", "-".repeat(88));
    for (name, entry) in &timed {
        let mean_us = entry.mean.as_ref().map(|s| s.microseconds).unwrap_or(0.0);
        let median_us = entry.median.as_ref().map(|s| s.microseconds).unwrap_or(0.0);
        let std_dev_us = entry.std_dev.as_ref().map(|s| s.microseconds).unwrap_or(0.0);
        let loc = entry.source_lines.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
        println!(
            "{:<40} {:>12.2} {:>12.2} {:>12.2} {:>8}",
            name, mean_us, median_us, std_dev_us, loc
        );
    }
    for (name, entry) in &no_timing {
        let loc = entry.source_lines.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
        println!(
            "{:<40} {:>12} {:>12} {:>12} {:>8}",
            name, "(no data)", "(no data)", "(no data)", loc
        );
    }
    println!();
    println!("{} benchmark(s) loaded ({} with timing data).", file.benchmarks.len(), timed.len());
}

/// Write `.ci/metrics/parser.json`.
fn write_json_output(root: &Path, file: &BenchmarkFile) -> Result<()> {
    let metrics_dir = root.join(".ci").join("metrics");
    fs::create_dir_all(&metrics_dir)
        .with_context(|| format!("creating {}", metrics_dir.display()))?;

    let mut entries: Vec<(&str, &BenchmarkEntry)> = file
        .benchmarks
        .iter()
        .filter(|(_, v)| v.mean.is_some())
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    entries.sort_by(|a, b| {
        let an = a.1.mean.as_ref().map(|s| s.nanoseconds).unwrap_or(0.0);
        let bn = b.1.mean.as_ref().map(|s| s.nanoseconds).unwrap_or(0.0);
        bn.partial_cmp(&an).unwrap_or(std::cmp::Ordering::Equal)
    });

    let slowest: Vec<SlowEntry> = entries
        .iter()
        .take(5)
        .map(|(name, e)| SlowEntry {
            name: (*name).to_string(),
            mean_us: e.mean.as_ref().map(|s| s.microseconds).unwrap_or(0.0),
        })
        .collect();

    let output = ParserMetricsOutput {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "parser",
        metrics: ParserMetrics { benchmark_count: file.benchmarks.len(), slowest },
    };

    let out_path = metrics_dir.join("parser.json");
    let json = serde_json::to_string_pretty(&output).with_context(|| "serializing metrics")?;
    fs::write(&out_path, &json).with_context(|| format!("writing {}", out_path.display()))?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_benchmark_entry() -> Result<()> {
        let json = r#"{"benchmarks":{"parse_simple":{"mean":{"nanoseconds":21255.87,"microseconds":21.26},"median":{"nanoseconds":19773.55,"microseconds":19.77},"std_dev":{"nanoseconds":5731.70,"microseconds":5.73},"source_lines":15}}}"#;
        let file: BenchmarkFile = serde_json::from_str(json)?;
        assert_eq!(file.benchmarks.len(), 1);
        let mean_us =
            file.benchmarks["parse_simple"].mean.as_ref().map(|s| s.microseconds).unwrap_or(0.0);
        assert!((mean_us - 21.26).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_empty_benchmarks_does_not_panic() -> Result<()> {
        let json = r#"{"benchmarks":{}}"#;
        let file: BenchmarkFile = serde_json::from_str(json)?;
        assert_eq!(file.benchmarks.len(), 0);
        Ok(())
    }

    #[test]
    fn test_print_table_does_not_panic_with_multiple_entries() -> Result<()> {
        let json = r#"{"benchmarks":{"fast":{"mean":{"nanoseconds":1000.0,"microseconds":1.0},"median":{"nanoseconds":900.0,"microseconds":0.9},"std_dev":{"nanoseconds":50.0,"microseconds":0.05},"source_lines":10},"slow":{"mean":{"nanoseconds":50000.0,"microseconds":50.0},"median":{"nanoseconds":48000.0,"microseconds":48.0},"std_dev":{"nanoseconds":2000.0,"microseconds":2.0},"source_lines":null}}}"#;
        let file: BenchmarkFile = serde_json::from_str(json)?;
        // print_table must not panic even with missing source_lines
        print_table(&file);
        Ok(())
    }

    /// Verify the JSON output schema: correct keys, schema_version=1, subsystem="parser",
    /// benchmark_count reflects total entries, and slowest is sorted descending by mean_us.
    #[test]
    fn test_write_json_output_schema() -> Result<()> {
        let json = r#"{"benchmarks":{
            "fast":{"mean":{"nanoseconds":1000.0,"microseconds":1.0},"median":{"nanoseconds":900.0,"microseconds":0.9},"std_dev":{"nanoseconds":50.0,"microseconds":0.05},"source_lines":10},
            "slow":{"mean":{"nanoseconds":50000.0,"microseconds":50.0},"median":{"nanoseconds":48000.0,"microseconds":48.0},"std_dev":{"nanoseconds":2000.0,"microseconds":2.0},"source_lines":null},
            "incomplete":{"source_lines":42}
        }}"#;
        let file: BenchmarkFile = serde_json::from_str(json)?;
        let tmp = TempDir::new()?;
        write_json_output(tmp.path(), &file)?;

        let out_path = tmp.path().join(".ci").join("metrics").join("parser.json");
        let raw = fs::read_to_string(&out_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;

        // Schema contract checks
        assert_eq!(parsed["schema_version"], 1, "schema_version must be 1");
        assert_eq!(parsed["subsystem"], "parser", "subsystem must be 'parser'");
        assert!(parsed["measured_at"].is_string(), "measured_at must be a string");
        assert_eq!(
            parsed["metrics"]["benchmark_count"], 3,
            "benchmark_count must include incomplete entries"
        );

        // slowest must contain only timed entries, sorted descending by mean_us
        let slowest = parsed["metrics"]["slowest"].as_array().expect("slowest must be an array");
        assert_eq!(slowest.len(), 2, "slowest must only contain timed entries");
        assert_eq!(slowest[0]["name"], "slow", "first entry must be the slowest benchmark");
        assert!(
            (slowest[0]["mean_us"].as_f64().unwrap() - 50.0).abs() < 0.01,
            "mean_us must match"
        );
        assert_eq!(slowest[1]["name"], "fast", "second entry must be the faster benchmark");

        Ok(())
    }

    /// When all benchmark entries are incomplete (real-world scenario: 3 of 4 current benchmarks),
    /// write_json_output must succeed and emit an empty slowest list, not panic or emit wrong data.
    #[test]
    fn test_write_json_output_all_incomplete() -> Result<()> {
        let json = r#"{"benchmarks":{
            "a":{"source_lines":10},
            "b":{"source_lines":null},
            "c":{}
        }}"#;
        let file: BenchmarkFile = serde_json::from_str(json)?;
        let tmp = TempDir::new()?;
        write_json_output(tmp.path(), &file)?;

        let out_path = tmp.path().join(".ci").join("metrics").join("parser.json");
        let raw = fs::read_to_string(&out_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;

        assert_eq!(parsed["metrics"]["benchmark_count"], 3);
        let slowest = parsed["metrics"]["slowest"].as_array().expect("slowest must be an array");
        assert!(slowest.is_empty(), "slowest must be empty when no entries have timing data");

        Ok(())
    }

    /// When benchmarks/results/ exists but contains no JSON files, the error message must
    /// mention the directory path so the user knows where to look.
    #[test]
    fn test_find_latest_benchmark_json_empty_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let results_dir = tmp.path().join("benchmarks").join("results");
        fs::create_dir_all(&results_dir).expect("create results dir");
        // No JSON files in results_dir
        let err = find_latest_benchmark_json(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no *.json files found"),
            "error must mention 'no *.json files found', got: {msg}"
        );
    }
}
