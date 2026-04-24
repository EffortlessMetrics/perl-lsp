//! Lexer benchmark scorecard summary subcommand.
//!
//! Reads `target/criterion/lexer_scorecard.json` (or `--input`) emitted by
//! `cargo bench -p perl-lexer --bench lexer_benchmarks` and optionally writes
//! `.ci/metrics/lexer.json` so status docs can reference one stable artifact.

use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct LexerMetricsOutput {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    metrics: LexerMetrics,
}

#[derive(Debug, Serialize)]
struct LexerMetrics {
    family_count: usize,
    families: Vec<LexerFamily>,
}

#[derive(Debug, Serialize, Clone)]
struct LexerFamily {
    name: String,
    sample_count: u64,
    total_time_ns: f64,
    mean_time_ns: f64,
    tokens_per_second: f64,
}

pub fn run(input: Option<PathBuf>, json: bool) -> Result<()> {
    let root = project_root()?;
    let input_path = input.unwrap_or_else(|| default_input_path(&root));

    let raw = fs::read_to_string(&input_path)
        .with_context(|| format!("reading benchmark file: {}", input_path.display()))?;
    let parsed: Value =
        serde_json::from_str(&raw).with_context(|| "parsing lexer scorecard JSON")?;

    let families = parse_families(&parsed)?;
    print_table(&families);

    if json {
        write_json_output(&root, &families)?;
    }

    Ok(())
}

fn default_input_path(root: &Path) -> PathBuf {
    let workspace_target = root.join("target").join("criterion").join("lexer_scorecard.json");
    if workspace_target.exists() {
        return workspace_target;
    }

    root.join("crates")
        .join("perl-lexer")
        .join("target")
        .join("criterion")
        .join("lexer_scorecard.json")
}

fn parse_families(parsed: &Value) -> Result<Vec<LexerFamily>> {
    let Some(families) = parsed.get("families").and_then(Value::as_object) else {
        bail!("missing 'families' object in lexer scorecard");
    };

    let mut rows = Vec::with_capacity(families.len());
    for (name, row) in families {
        let sample_count = row.get("sample_count").and_then(Value::as_u64).unwrap_or(0);
        let total_time_ns = row.get("total_time_ns").and_then(Value::as_f64).unwrap_or(0.0);
        let mean_time_ns = row.get("mean_time_ns").and_then(Value::as_f64).unwrap_or(0.0);
        let tokens_per_second = row.get("tokens_per_second").and_then(Value::as_f64).unwrap_or(0.0);

        rows.push(LexerFamily {
            name: name.to_string(),
            sample_count,
            total_time_ns,
            mean_time_ns,
            tokens_per_second,
        });
    }

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

fn print_table(families: &[LexerFamily]) {
    println!(
        "{:<24} {:>10} {:>16} {:>16} {:>16}",
        "Family", "Samples", "Total (ms)", "Mean (µs)", "Tokens/sec"
    );
    println!("{}", "-".repeat(88));

    for family in families {
        println!(
            "{:<24} {:>10} {:>16.3} {:>16.3} {:>16.1}",
            family.name,
            family.sample_count,
            family.total_time_ns / 1_000_000.0,
            family.mean_time_ns / 1_000.0,
            family.tokens_per_second,
        );
    }

    println!();
    println!("{} benchmark family row(s) loaded.", families.len());
}

fn write_json_output(root: &Path, families: &[LexerFamily]) -> Result<()> {
    let metrics_dir = root.join(".ci").join("metrics");
    fs::create_dir_all(&metrics_dir)
        .with_context(|| format!("creating {}", metrics_dir.display()))?;

    let output = LexerMetricsOutput {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "lexer",
        metrics: LexerMetrics { family_count: families.len(), families: families.to_vec() },
    };

    let out_path = metrics_dir.join("lexer.json");
    let encoded =
        serde_json::to_string_pretty(&output).with_context(|| "serializing lexer metrics")?;
    fs::write(&out_path, encoded).with_context(|| format!("writing {}", out_path.display()))?;

    println!("Wrote {}", out_path.display());
    Ok(())
}
