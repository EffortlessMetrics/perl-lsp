//! Render memory plateau trend tables from structured receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Options for `cargo xtask memory-trends render`.
pub struct MemoryTrendsConfig {
    pub input_dir: PathBuf,
    pub history_dirs: Vec<PathBuf>,
    pub baseline: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
struct TrendRow {
    date: String,
    commit: String,
    scenario: String,
    files: Option<u64>,
    tail_growth_kb: Option<i64>,
    median_tail_slope_kb_per_file: Option<f64>,
    passed: Option<bool>,
    artifact: String,
}

#[derive(Debug, Deserialize)]
struct WorkloadPayload {
    n_files: Option<u64>,
    #[allow(dead_code)]
    n_changes: Option<u64>,
}

/// Render memory plateau trends.
pub fn render(config: MemoryTrendsConfig) -> Result<()> {
    let markdown = render_markdown(&collect_rows(&config)?)?;
    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&config.output, markdown)
        .wrap_err_with(|| format!("failed to write {}", config.output.display()))?;
    println!("Rendered {}", config.output.display());
    Ok(())
}

fn collect_rows(config: &MemoryTrendsConfig) -> Result<Vec<TrendRow>> {
    let mut rows = Vec::new();

    if config.baseline.exists() {
        rows.extend(rows_from_json_file(&config.baseline)?);
    }

    for dir in std::iter::once(&config.input_dir).chain(config.history_dirs.iter()) {
        if dir.exists() {
            rows.extend(rows_from_dir(dir)?);
        }
    }

    rows = dedup_rows(rows);
    rows.sort_by(|a, b| {
        (a.date.as_str(), a.scenario.as_str(), a.artifact.as_str()).cmp(&(
            b.date.as_str(),
            b.scenario.as_str(),
            b.artifact.as_str(),
        ))
    });

    Ok(rows)
}

fn dedup_rows(rows: Vec<TrendRow>) -> Vec<TrendRow> {
    let mut deduped: Vec<TrendRow> = Vec::new();
    for row in rows {
        if let Some(existing) = deduped.iter_mut().find(|existing| same_measurement(existing, &row))
        {
            if prefer_replacement(&row, existing) {
                *existing = row;
            }
        } else {
            deduped.push(row);
        }
    }
    deduped
}

fn same_measurement(a: &TrendRow, b: &TrendRow) -> bool {
    a.date == b.date
        && a.commit == b.commit
        && a.scenario == b.scenario
        && a.files == b.files
        && a.tail_growth_kb == b.tail_growth_kb
        && a.median_tail_slope_kb_per_file == b.median_tail_slope_kb_per_file
        && a.passed == b.passed
}

fn prefer_replacement(candidate: &TrendRow, existing: &TrendRow) -> bool {
    is_baseline_artifact(&existing.artifact) && !is_baseline_artifact(&candidate.artifact)
}

fn is_baseline_artifact(artifact: &str) -> bool {
    artifact.contains(".ci/metrics/baselines/")
}

fn rows_from_dir(dir: &Path) -> Result<Vec<TrendRow>> {
    let mut rows = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("json")
        {
            continue;
        }
        rows.extend(rows_from_json_file(entry.path())?);
    }
    Ok(rows)
}

fn rows_from_json_file(path: &Path) -> Result<Vec<TrendRow>> {
    let text =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .wrap_err_with(|| format!("invalid JSON in {}", path.display()))?;

    if value.get("kind").and_then(Value::as_str) == Some("memory_plateau") {
        return Ok(vec![row_from_receipt(path, &value)]);
    }

    if value.get("kind").and_then(Value::as_str) == Some("memory_plateau_baseline") {
        return Ok(rows_from_baseline(path, &value));
    }

    if looks_like_plateau_summary(&value) {
        return Ok(vec![row_from_plateau_summary(path, &value)]);
    }

    Ok(Vec::new())
}

fn row_from_receipt(path: &Path, value: &Value) -> TrendRow {
    TrendRow {
        date: date_from_value(value.get("generated_at")).unwrap_or_else(|| "-".to_string()),
        commit: value
            .get("commit")
            .and_then(Value::as_str)
            .map(short_commit)
            .unwrap_or_else(|| "-".to_string()),
        scenario: value.get("scenario").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        files: value.get("files").and_then(Value::as_u64),
        tail_growth_kb: value.get("tail_growth_kb").and_then(Value::as_i64),
        median_tail_slope_kb_per_file: value
            .get("median_tail_slope_kb_per_file")
            .and_then(Value::as_f64),
        passed: value.get("passed").and_then(Value::as_bool),
        artifact: path.display().to_string(),
    }
}

fn rows_from_baseline(path: &Path, value: &Value) -> Vec<TrendRow> {
    let source = value.get("source");
    let date = source
        .and_then(|s| date_from_value(s.get("captured_at")))
        .unwrap_or_else(|| "-".to_string());
    let commit = source
        .and_then(|s| s.get("commit"))
        .and_then(Value::as_str)
        .map(short_commit)
        .unwrap_or_else(|| "-".to_string());

    value
        .get("scenarios")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|scenario| {
            let scenario_name = scenario.get("scenario").and_then(Value::as_str)?;
            Some(TrendRow {
                date: date.clone(),
                commit: commit.clone(),
                scenario: scenario_name.to_string(),
                files: scenario.get("files").and_then(Value::as_u64),
                tail_growth_kb: scenario.get("tail_growth_kb").and_then(Value::as_i64),
                median_tail_slope_kb_per_file: scenario
                    .get("median_tail_slope_kb_per_file")
                    .and_then(Value::as_f64),
                passed: scenario.get("passed").and_then(Value::as_bool),
                artifact: path.display().to_string(),
            })
        })
        .collect()
}

fn row_from_plateau_summary(path: &Path, value: &Value) -> TrendRow {
    let workload_path = adjacent_workload_path(path);
    let workload = workload_path.as_ref().and_then(|p| read_workload(p).ok());

    TrendRow {
        date: "-".to_string(),
        commit: "-".to_string(),
        scenario: workload_path
            .as_deref()
            .and_then(|p| super::metrics::memory::infer_scenario(p).ok())
            .unwrap_or_else(|| scenario_from_path(path)),
        files: workload.and_then(|payload| payload.n_files),
        tail_growth_kb: value.get("tail_growth_kb").and_then(Value::as_i64),
        median_tail_slope_kb_per_file: value
            .get("median_tail_slope_kb_per_file")
            .and_then(Value::as_f64),
        passed: value.get("passed").and_then(Value::as_bool),
        artifact: path.display().to_string(),
    }
}

fn render_markdown(rows: &[TrendRow]) -> Result<String> {
    let mut out = String::new();
    out.push_str("# Memory Plateau Trends\n\n");
    out.push_str("> Generated by `cargo xtask memory-trends render` from memory plateau receipts, summaries, and committed baselines.\n\n");
    out.push_str("| Date | Commit | Scenario | Files | Tail growth KB | Median tail slope KB/file | Result | Artifact |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | ---: | --- | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | `{}` | {} | {} | {} | {} | `{}` |\n",
            row.date,
            row.commit,
            row.scenario,
            format_option_u64(row.files),
            format_option_i64(row.tail_growth_kb),
            format_option_f64(row.median_tail_slope_kb_per_file),
            format_result(row.passed),
            row.artifact
        ));
    }
    Ok(out)
}

fn looks_like_plateau_summary(value: &Value) -> bool {
    value.get("tail_growth_kb").is_some()
        && value.get("median_tail_slope_kb_per_file").is_some()
        && value.get("passed").is_some()
}

fn adjacent_workload_path(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let workload_name = name.strip_suffix(".plateau.json").map(|stem| format!("{stem}.json"))?;
    Some(path.with_file_name(workload_name))
}

fn read_workload(path: &Path) -> Result<WorkloadPayload> {
    let text =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).wrap_err_with(|| format!("invalid JSON in {}", path.display()))
}

fn scenario_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .trim_end_matches(".plateau")
        .replace('-', "_")
}

fn date_from_value(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).date_naive().to_string())
        .or_else(|| raw.get(0..10).map(ToOwned::to_owned))
}

fn short_commit(commit: &str) -> String {
    commit.chars().take(12).collect()
}

fn format_option_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |n| n.to_string())
}

fn format_option_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |n| n.to_string())
}

fn format_option_f64(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |n| format!("{n:.3}"))
}

fn format_result(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "passed",
        Some(false) => "failed",
        None => "n/a",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_trends_includes_baseline_and_receipt_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let input = temp.path().join("target-memory");
        fs::create_dir_all(&input)?;
        let baseline = temp.path().join(".ci/metrics/baselines/memory_plateau.json");
        fs::create_dir_all(baseline.parent().unwrap())?;
        let output = temp.path().join("trends.md");

        fs::write(
            &baseline,
            r#"{
  "kind": "memory_plateau_baseline",
  "source": {
    "commit": "abcdef0123456789",
    "captured_at": "2026-05-06T15:24:03Z"
  },
  "scenarios": [
    {
      "scenario": "lsp_doc_churn_delete",
      "files": 500,
      "tail_growth_kb": 152,
      "median_tail_slope_kb_per_file": 0.69,
      "passed": true
    }
  ]
}"#,
        )?;
        fs::write(
            input.join("nightly-doc-churn.receipt.json"),
            r#"{
  "kind": "memory_plateau",
  "scenario": "lsp_doc_churn_delete",
  "files": 500,
  "changes_per_file": 10,
  "tail_growth_kb": 152,
  "median_tail_slope_kb_per_file": 0.69,
  "passed": true,
  "commit": "abcdef0123456789",
  "generated_at": "2026-05-06T15:24:03Z"
}"#,
        )?;
        fs::write(
            input.join("doc.receipt.json"),
            r#"{
  "kind": "memory_plateau",
  "scenario": "lsp_doc_churn_delete_smoke",
  "files": 75,
  "changes_per_file": 5,
  "tail_growth_kb": 0,
  "median_tail_slope_kb_per_file": 0.0,
  "passed": true,
  "commit": "1234567890abcdef",
  "generated_at": "2026-05-07T05:15:03Z"
}"#,
        )?;

        render(MemoryTrendsConfig {
            input_dir: input,
            history_dirs: Vec::new(),
            baseline,
            output: output.clone(),
        })?;

        let markdown = fs::read_to_string(output)?;
        assert!(markdown.contains("lsp_doc_churn_delete"));
        assert!(markdown.contains("lsp_doc_churn_delete_smoke"));
        assert!(markdown.contains("abcdef012345"));
        assert!(markdown.contains("1234567890ab"));
        assert_eq!(markdown.matches("`lsp_doc_churn_delete`").count(), 1);
        assert!(markdown.contains("nightly-doc-churn.receipt.json"));
        Ok(())
    }

    #[test]
    fn plateau_summary_uses_adjacent_workload_for_files_and_scenario() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let input = temp.path().join("target-memory");
        fs::create_dir_all(&input)?;
        let output = temp.path().join("trends.md");
        let missing_baseline = temp.path().join("missing-baseline.json");

        fs::write(input.join("doc_churn_500_delete.json"), r#"{"n_files":500,"n_changes":10}"#)?;
        fs::write(
            input.join("doc_churn_500_delete.plateau.json"),
            r#"{"tail_growth_kb":152,"median_tail_slope_kb_per_file":0.69,"passed":true}"#,
        )?;

        render(MemoryTrendsConfig {
            input_dir: input,
            history_dirs: Vec::new(),
            baseline: missing_baseline,
            output: output.clone(),
        })?;

        let markdown = fs::read_to_string(output)?;
        assert!(markdown.contains("lsp_doc_churn_delete"));
        assert!(markdown.contains("| 500 | 152 | 0.690 | passed |"));
        Ok(())
    }
}
