//! Memory plateau receipt and summary helpers.

use chrono::Utc;
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for `cargo xtask metrics memory`.
pub struct MemoryMetricsConfig {
    pub scenario: String,
    pub workload_json: PathBuf,
    pub plateau_json: PathBuf,
    pub receipt: Option<PathBuf>,
    pub commit: Option<String>,
    pub event: String,
    pub markdown: bool,
}

#[derive(Debug, Deserialize)]
struct WorkloadPayload {
    n_files: u64,
    n_changes: u64,
    #[serde(default)]
    workspace_symbol: bool,
    #[serde(default)]
    delete_after_close: bool,
    #[serde(default)]
    settle_seconds: f64,
}

#[derive(Debug, Deserialize)]
struct PlateauSummary {
    samples: u64,
    tail_growth_kb: i64,
    tail_growth_pct: f64,
    median_tail_slope_kb_per_file: f64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct MemoryPlateauReceipt {
    check: &'static str,
    kind: &'static str,
    schema_version: &'static str,
    event: String,
    verdict: &'static str,
    scenario: String,
    files: u64,
    changes_per_file: u64,
    workspace_symbol: bool,
    delete_after_close: bool,
    settle_seconds: f64,
    tail_growth_kb: i64,
    tail_growth_pct: f64,
    median_tail_slope_kb_per_file: f64,
    samples: u64,
    passed: bool,
    commit: Option<String>,
    generated_at: String,
    metrics: serde_json::Value,
    artifacts: Vec<serde_json::Value>,
}

/// Run `cargo xtask metrics memory`.
pub fn run(config: MemoryMetricsConfig) -> Result<()> {
    let workload = read_json::<WorkloadPayload>(&config.workload_json)?;
    let plateau = read_json::<PlateauSummary>(&config.plateau_json)?;

    let receipt = MemoryPlateauReceipt {
        check: "memory-plateau",
        kind: "memory_plateau",
        schema_version: "1",
        event: config.event,
        verdict: if plateau.passed { "pass" } else { "fail" },
        scenario: config.scenario,
        files: workload.n_files,
        changes_per_file: workload.n_changes,
        workspace_symbol: workload.workspace_symbol,
        delete_after_close: workload.delete_after_close,
        settle_seconds: workload.settle_seconds,
        tail_growth_kb: plateau.tail_growth_kb,
        tail_growth_pct: plateau.tail_growth_pct,
        median_tail_slope_kb_per_file: plateau.median_tail_slope_kb_per_file,
        samples: plateau.samples,
        passed: plateau.passed,
        commit: config.commit,
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        metrics: json!({
            "tail_growth_kb": plateau.tail_growth_kb,
            "tail_growth_pct": plateau.tail_growth_pct,
            "median_tail_slope_kb_per_file": plateau.median_tail_slope_kb_per_file,
            "samples": plateau.samples,
        }),
        artifacts: vec![
            json!({
                "kind": "workload_json",
                "path": config.workload_json,
            }),
            json!({
                "kind": "plateau_summary",
                "path": config.plateau_json,
            }),
        ],
    };

    if let Some(path) = &config.receipt {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(path, serde_json::to_string_pretty(&receipt)? + "\n")
            .wrap_err_with(|| format!("failed to write {}", path.display()))?;
    }

    if config.markdown {
        print_markdown(&receipt);
    } else {
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    }

    Ok(())
}

fn read_json<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).wrap_err_with(|| format!("invalid JSON in {}", path.display()))
}

fn print_markdown(receipt: &MemoryPlateauReceipt) {
    println!("## Memory plateau receipt");
    println!();
    println!(
        "| Scenario | Files | Changes/file | Tail growth KB | Median tail slope KB/file | Result |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | --- |");
    println!(
        "| {} | {} | {} | {} | {:.3} | {} |",
        receipt.scenario,
        receipt.files,
        receipt.changes_per_file,
        receipt.tail_growth_kb,
        receipt.median_tail_slope_kb_per_file,
        if receipt.passed { "passed" } else { "failed" }
    );
}

pub fn infer_scenario(workload_json: &Path) -> Result<String> {
    let name = workload_json
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| eyre!("cannot infer scenario from {}", workload_json.display()))?;

    match name {
        "nightly-doc-churn" | "doc_churn_500_delete" => Ok("lsp_doc_churn_delete".to_string()),
        "nightly-workspace-symbol" | "workspace_symbol_300_delete" => {
            Ok("lsp_workspace_symbol_churn_delete".to_string())
        }
        "pr-smoke-doc-churn" => Ok("lsp_doc_churn_delete_smoke".to_string()),
        other => Ok(other.replace('-', "_")),
    }
}
