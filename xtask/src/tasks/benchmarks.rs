//! Benchmark task wrappers and helpers.

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};
use walkdir::WalkDir;

use crate::utils::project_root;

pub fn run_benchmarks(
    output: Option<PathBuf>,
    quick: bool,
    category: Option<String>,
) -> Result<()> {
    let root = project_root()?;
    let script = root.join("benchmarks").join("scripts").join("run-benchmarks.sh");

    let mut args: Vec<String> = Vec::new();
    if let Some(output_file) = output {
        args.push("--output".to_string());
        args.push(output_file.to_string_lossy().into_owned());
    }
    if quick {
        args.push("--quick".to_string());
    }
    if let Some(category) = category {
        args.push("--category".to_string());
        args.push(category);
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_script(&script, &arg_refs, "benchmarks runner")
}

pub fn compare_benchmarks(fail_on_regression: bool) -> Result<()> {
    let root = project_root()?;
    let script = root.join("benchmarks").join("scripts").join("compare.sh");

    let args = if fail_on_regression { vec!["--fail-on-regression"] } else { Vec::<&str>::new() };
    run_script(&script, &args, "benchmark comparison")
}

pub fn format_benchmarks(receipt: bool, markdown: bool) -> Result<()> {
    let root = project_root()?;
    let script = root.join("benchmarks").join("scripts").join("format-results.py");
    let source = Path::new("benchmarks/results/latest.json");

    let mut args: Vec<String> = vec![source.display().to_string()];
    if receipt {
        args.push("--receipt".to_string());
    }
    if markdown {
        args.push("--markdown".to_string());
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_python_script(&script, &arg_refs, "benchmark results formatter")
}

pub fn alert_benchmarks(format: Option<String>, check: bool) -> Result<()> {
    let root = project_root()?;
    let script = root.join("benchmarks").join("scripts").join("alert.py");

    let mut args = Vec::<String>::new();
    if let Some(format) = format {
        args.push("--format".to_string());
        args.push(format);
    }
    if check {
        args.push("--check".to_string());
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_python_script(&script, &arg_refs, "benchmark alerting")
}

pub fn extract_criterion(base_path: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let base_path = base_path.unwrap_or_else(|| root.clone());
    let results_root = base_path.join("target").join("criterion");

    let by_category = parse_criterion_results(&results_root)?;
    let output_path = output.unwrap_or_else(|| root.join("benchmarks/results/latest.json"));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let mut categories = Map::new();
    let mut total = 0usize;
    for (category, mut benchmarks) in by_category {
        total += benchmarks.len();
        benchmarks.insert("_category".to_string(), Value::String(category.clone()));
        categories.insert(category, Value::Object(benchmarks));
    }

    let (git_sha, git_dirty) = git_status(&root)?;
    let rust_version = rust_version()?;
    let payload = serde_json::json!({
        "version": "0.9.0",
        "timestamp": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "git_sha": git_sha,
        "git_dirty": git_dirty,
        "environment": {
            "os": std::env::consts::OS,
            "rust_version": rust_version,
            "extracted_from": "criterion",
        },
        "results": categories,
    });

    fs::write(
        &output_path,
        serde_json::to_string_pretty(&payload).context("Failed to encode benchmark payload")?,
    )
    .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("Results extracted to {}", output_path.display());
    println!("Total benchmarks: {total}");
    for category in payload
        .get("results")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|results| results.iter())
    {
        let count = category
            .1
            .as_object()
            .map(|benchmarks| benchmarks.keys().filter(|name| !name.starts_with("_")).count())
            .unwrap_or(0);
        println!("  {}: {}", category.0, count);
    }

    Ok(())
}

pub fn test_alert_system() -> Result<()> {
    let root = project_root()?;
    let alert_script = root.join("benchmarks/scripts/alert.py");
    let baseline_path = root.join("benchmarks/baselines/v0.9.0.json");
    let config_path = root.join(".ci/benchmark-thresholds.yaml");
    let workdir = root.join("target").join("xtask").join("bench-alert-test");
    fs::create_dir_all(&workdir)
        .with_context(|| format!("Failed to create {}", workdir.display()))?;

    if !alert_script.exists() {
        bail!("Missing benchmark alert script at {}", alert_script.display());
    }
    if !baseline_path.exists() {
        bail!("Missing benchmark baseline at {}", baseline_path.display());
    }
    if !config_path.exists() {
        bail!("Missing benchmark threshold config at {}", config_path.display());
    }

    let baseline = load_json(&baseline_path)?;

    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_no_regression.json"),
            multiplier: 1.0,
            format: None,
            expected: &["No performance alerts detected"],
            expect_success: true,
        },
    )?;
    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_warning.json"),
            multiplier: 1.11,
            format: None,
            expected: &["WARNING", "1"],
            expect_success: true,
        },
    )?;
    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_regression.json"),
            multiplier: 1.25,
            format: None,
            expected: &["REGRESSION"],
            expect_success: true,
        },
    )?;
    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_critical.json"),
            multiplier: 1.60,
            format: None,
            expected: &["CRITICAL"],
            expect_success: true,
        },
    )?;
    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_markdown.json"),
            multiplier: 1.25,
            format: Some("markdown"),
            expected: &["## Performance Benchmark Results", "⚠️ Performance Regressions"],
            expect_success: true,
        },
    )?;
    run_alert_check_case(
        &root,
        &alert_script,
        &baseline_path,
        &baseline,
        workdir.join("alert_test_check.json"),
    )?;
    run_alert_case(
        &root,
        &alert_script,
        &config_path,
        &baseline_path,
        &baseline,
        AlertCase {
            current_path: workdir.join("alert_test_improvement.json"),
            multiplier: 0.80,
            format: None,
            expected: &["IMPROVED"],
            expect_success: true,
        },
    )?;

    println!("All benchmark alert-system checks passed");
    Ok(())
}

struct AlertCase<'a> {
    current_path: PathBuf,
    multiplier: f64,
    format: Option<&'a str>,
    expected: &'a [&'a str],
    expect_success: bool,
}

fn run_alert_case(
    root: &Path,
    alert_script: &Path,
    config_path: &Path,
    baseline_path: &Path,
    baseline: &Value,
    case: AlertCase<'_>,
) -> Result<()> {
    let mut current = baseline.clone();
    mutate_parse_simple_script(&mut current, case.multiplier)?;
    fs::write(&case.current_path, serde_json::to_string_pretty(&current)?)
        .with_context(|| format!("Failed to write {}", case.current_path.display()))?;

    let mut args = vec![
        alert_script.to_string_lossy().to_string(),
        baseline_path.to_string_lossy().to_string(),
        case.current_path.to_string_lossy().to_string(),
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
    ];
    if let Some(format) = case.format {
        args.push("--format".to_string());
        args.push(format.to_string());
    }

    let output = Command::new("python3")
        .current_dir(root)
        .args(&args)
        .output()
        .context("Failed to execute alert.py")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if case.expect_success != output.status.success() {
        bail!("alert.py command status mismatch (expected success: {})", case.expect_success);
    }

    for fragment in case.expected {
        if !stdout.contains(fragment) {
            bail!("Expected output to contain '{fragment}'");
        }
    }

    Ok(())
}

fn run_alert_check_case(
    root: &Path,
    alert_script: &Path,
    baseline_path: &Path,
    baseline: &Value,
    current_path: PathBuf,
) -> Result<()> {
    let mut current = baseline.clone();
    mutate_parse_simple_script(&mut current, 1.60)?;

    fs::write(&current_path, serde_json::to_string_pretty(&current)?)
        .with_context(|| format!("Failed to write {}", current_path.display()))?;

    let config_path =
        root.join("target").join("xtask").join("bench-alert-test").join("critical_check.yaml");
    let mut config_payload = String::new();
    config_payload.push_str("defaults:\n");
    config_payload.push_str("  warn_threshold_pct: 10\n");
    config_payload.push_str("  regression_threshold_pct: 20\n");
    config_payload.push_str("  critical_threshold_pct: 50\n");
    config_payload.push_str("  improvement_threshold_pct: 10\n");
    config_payload.push_str("alerting:\n");
    config_payload.push_str("  fail_on_critical: true\n");
    fs::write(&config_path, config_payload).context("Failed to write critical-check config")?;

    let output = Command::new("python3")
        .current_dir(root)
        .args([
            alert_script.to_str().ok_or_else(|| io::Error::other("Invalid alert script path"))?,
            baseline_path.to_str().ok_or_else(|| io::Error::other("Invalid baseline path"))?,
            current_path.to_str().ok_or_else(|| io::Error::other("Invalid current output path"))?,
            "--config",
            config_path.to_str().ok_or_else(|| io::Error::other("Invalid check config path"))?,
            "--check",
        ])
        .output()
        .context("Failed to execute alert.py with check flag")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!("Expected critical check failure. output={stdout}");
    }

    Ok(())
}

fn mutate_parse_simple_script(data: &mut Value, multiplier: f64) -> Result<()> {
    let parse_simple = data
        .get_mut("benchmarks")
        .and_then(|benchmarks| benchmarks.get_mut("parser"))
        .and_then(|parser| parser.get_mut("parse_simple_script"))
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing benchmarks.parser.parse_simple_script"))?;

    if let Some(mean) = parse_simple.get_mut("mean").and_then(Value::as_object_mut) {
        if let Some(ns) = mean.get_mut("nanoseconds") {
            multiply_json_number(ns, multiplier)?;
            return Ok(());
        }
        if let Some(point_estimate) = mean.get_mut("point_estimate") {
            multiply_json_number(point_estimate, multiplier)?;
            return Ok(());
        }
    }

    if let Some(mean_ns) = parse_simple.get_mut("mean_ns") {
        multiply_json_number(mean_ns, multiplier)?;
        return Ok(());
    }

    bail!("Could not find a supported parse_simple_script mean value")
}

fn multiply_json_number(value: &mut Value, multiplier: f64) -> Result<()> {
    let parsed = value
        .as_f64()
        .or_else(|| value.as_u64().map(|value| value as f64))
        .ok_or_else(|| color_eyre::eyre::eyre!("Expected numeric benchmark value"))?;
    let next = (parsed * multiplier).round();
    let next = serde_json::Number::from_f64(next.max(0.0))
        .ok_or_else(|| color_eyre::eyre::eyre!("Failed to encode scaled benchmark value"))?;
    *value = Value::Number(next);
    Ok(())
}

fn parse_criterion_results(criterion_dir: &Path) -> Result<BTreeMap<String, Map<String, Value>>> {
    if !criterion_dir.exists() {
        return Ok(BTreeMap::new());
    }

    let mut by_category: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for entry in WalkDir::new(criterion_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() != "estimates.json" {
            continue;
        }

        let path = entry.path();
        let relative = path
            .strip_prefix(criterion_dir)
            .context("Failed to relativize criterion result path")?;
        let parts: Vec<_> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().to_string())
            .collect();
        let Some((group, bench_name)) = parse_criterion_identity(&parts) else {
            eprintln!("Warning: unexpected criterion path {}", path.display());
            continue;
        };
        let category = categorize_benchmark(&group, &bench_name);

        let file_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let estimate: Value = serde_json::from_str(&file_content)
            .with_context(|| format!("Invalid JSON in {}", path.display()))?;

        let mean = match estimate.get("mean").and_then(Value::as_object) {
            Some(mean) => mean,
            None => {
                eprintln!("Warning: missing mean entry in {}", path.display());
                continue;
            }
        };
        let mean_ns = extract_u64(mean.get("point_estimate")).or_else(|_| {
            extract_u64(mean.get("estimate")).or_else(|_| extract_u64(mean.get("value")))
        })?;
        let confidence = mean.get("confidence_interval").and_then(Value::as_object);
        let low_ns = confidence
            .and_then(|object| object.get("lower_bound"))
            .and_then(|value| extract_u64(Some(value)).ok())
            .unwrap_or(mean_ns);
        let high_ns = confidence
            .and_then(|object| object.get("upper_bound"))
            .and_then(|value| extract_u64(Some(value)).ok())
            .unwrap_or(mean_ns);

        let (unit, display) = display_duration(mean_ns);

        let mut entry = Map::new();
        entry.insert("mean_ns".to_string(), Value::from(mean_ns));
        entry.insert("low_ns".to_string(), Value::from(low_ns));
        entry.insert("high_ns".to_string(), Value::from(high_ns));
        entry.insert("unit".to_string(), Value::String(unit));
        entry.insert("display".to_string(), Value::String(display));

        by_category.entry(category).or_default().insert(bench_name, Value::Object(entry));
    }

    Ok(by_category)
}

fn parse_criterion_identity(parts: &[String]) -> Option<(String, String)> {
    if parts.len() < 3 || parts.last().is_none_or(|part| part != "estimates.json") {
        return None;
    }

    // Ignore Criterion comparison-diff directories. We only want direct run output.
    if parts.iter().any(|part| part == "change") {
        return None;
    }

    let marker = parts.get(parts.len().saturating_sub(2)).map(String::as_str);
    if matches!(marker, Some("base")) {
        return None;
    }

    if matches!(marker, Some("new")) && parts.len() >= 4 {
        let bench_name = parts[parts.len() - 3].clone();
        let group = parts[..parts.len() - 3].join("/");
        let group = if group.is_empty() { "unknown".to_string() } else { group };
        return Some((group, bench_name));
    }

    let bench_name = parts[parts.len() - 2].clone();
    let group = parts[..parts.len() - 2].join("/");
    let group = if group.is_empty() { "unknown".to_string() } else { group };
    Some((group, bench_name))
}

fn categorize_benchmark(group: &str, bench_name: &str) -> String {
    let group = group.to_ascii_lowercase();
    let bench = bench_name.to_ascii_lowercase();

    if group.contains("parser") || bench.contains("parse") {
        "parser".to_string()
    } else if group.contains("lexer") || bench.contains("token") {
        "lexer".to_string()
    } else if group.contains("rope") || group.contains("position") || group.contains("lsp") {
        "lsp".to_string()
    } else if group.contains("index") || group.contains("workspace") || bench.contains("symbol") {
        "index".to_string()
    } else {
        "other".to_string()
    }
}

fn extract_u64(value: Option<&Value>) -> Result<u64> {
    let value = value.ok_or_else(|| color_eyre::eyre::eyre!("Missing numeric value"))?;
    value
        .as_u64()
        .or_else(|| value.as_f64().map(|value| value.max(0.0) as u64))
        .or_else(|| value.as_i64().map(|value| value.max(0) as u64))
        .ok_or_else(|| color_eyre::eyre::eyre!("Unsupported numeric value"))
}

fn display_duration(ns: u64) -> (String, String) {
    if ns < 1_000 {
        ("ns".to_string(), format!("{ns} ns"))
    } else if ns < 1_000_000 {
        ("us".to_string(), format!("{:.1} us", ns as f64 / 1_000.0))
    } else if ns < 1_000_000_000 {
        ("ms".to_string(), format!("{:.1} ms", ns as f64 / 1_000_000.0))
    } else {
        ("s".to_string(), format!("{:.2} s", ns as f64 / 1_000_000_000.0))
    }
}

fn git_status(root: &Path) -> Result<(String, bool)> {
    let sha_output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .context("Failed to run git rev-parse")?;
    let sha = if sha_output.status.success() {
        String::from_utf8_lossy(&sha_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    let dirty = Command::new("git")
        .current_dir(root)
        .args(["diff", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(true);

    Ok((sha, dirty))
}

fn rust_version() -> Result<String> {
    let output =
        Command::new("rustc").arg("--version").output().context("Failed to run rustc --version")?;
    if !output.status.success() {
        return Ok("unknown".to_string());
    }

    let line = String::from_utf8_lossy(&output.stdout);
    let mut parts = line.split_whitespace();
    parts.next();
    Ok(parts.next().unwrap_or("unknown").to_string())
}

fn load_json(path: &Path) -> Result<Value> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("Invalid JSON in {}", path.display()))
}

fn run_script(script: &Path, args: &[&str], label: &str) -> Result<()> {
    let status = Command::new("bash")
        .arg(script)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute {}", label))?;

    if status.success() {
        Ok(())
    } else {
        bail!("benchmark script failed: {}", label);
    }
}

fn run_python_script(script: &Path, args: &[&str], label: &str) -> Result<()> {
    let status = Command::new("python3")
        .arg(script)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute {}", label))?;

    if status.success() {
        Ok(())
    } else {
        bail!("python benchmark task failed: {}", label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_criterion_identity_handles_new_layout() {
        let parts = vec![
            "parser".to_string(),
            "large_file".to_string(),
            "new".to_string(),
            "estimates.json".to_string(),
        ];
        let result = parse_criterion_identity(&parts);
        assert_eq!(result, Some(("parser".to_string(), "large_file".to_string())));
    }

    #[test]
    fn parse_criterion_identity_supports_nested_group_names() {
        let parts = vec![
            "lsp".to_string(),
            "rope".to_string(),
            "position_conversion".to_string(),
            "new".to_string(),
            "estimates.json".to_string(),
        ];
        let result = parse_criterion_identity(&parts);
        assert_eq!(result, Some(("lsp/rope".to_string(), "position_conversion".to_string())));
    }

    #[test]
    fn parse_criterion_identity_rejects_too_few_parts() {
        assert_eq!(parse_criterion_identity(&[]), None);
        assert_eq!(parse_criterion_identity(&["estimates.json".to_string()]), None);
        assert_eq!(
            parse_criterion_identity(&["group".to_string(), "estimates.json".to_string()]),
            None
        );
    }

    #[test]
    fn parse_criterion_identity_rejects_base_runs() {
        let parts = vec![
            "parser".to_string(),
            "large_file".to_string(),
            "base".to_string(),
            "estimates.json".to_string(),
        ];
        assert_eq!(parse_criterion_identity(&parts), None);
    }

    #[test]
    fn parse_criterion_identity_rejects_change_dirs() {
        let parts = vec![
            "parser".to_string(),
            "large_file".to_string(),
            "change".to_string(),
            "estimates.json".to_string(),
        ];
        assert_eq!(parse_criterion_identity(&parts), None);
    }

    #[test]
    fn parse_criterion_identity_handles_plain_layout_without_new_subdir() {
        // Older Criterion versions write estimates.json directly under bench_name/
        let parts =
            vec!["parser".to_string(), "large_file".to_string(), "estimates.json".to_string()];
        let result = parse_criterion_identity(&parts);
        assert_eq!(result, Some(("parser".to_string(), "large_file".to_string())));
    }

    #[test]
    fn parse_criterion_results_excludes_base_and_change_results() -> Result<()> {
        let dir = TempDir::new()?;
        let root = dir.path();

        let include_path = root.join("parser").join("parse_simple").join("new");
        fs::create_dir_all(&include_path)?;
        fs::write(
            include_path.join("estimates.json"),
            r#"{"mean":{"point_estimate":1000.0,"confidence_interval":{"lower_bound":900.0,"upper_bound":1100.0}}}"#,
        )?;

        let base_path = root.join("parser").join("parse_simple").join("base");
        fs::create_dir_all(&base_path)?;
        fs::write(base_path.join("estimates.json"), r#"{"mean":{"point_estimate":5000.0}}"#)?;

        let change_path = root.join("parser").join("parse_simple").join("change");
        fs::create_dir_all(&change_path)?;
        fs::write(change_path.join("estimates.json"), r#"{"mean":{"point_estimate":8000.0}}"#)?;

        let results = parse_criterion_results(root)?;
        let parser = results
            .get("parser")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing parser category"))?;
        let parse_simple = parser
            .get("parse_simple")
            .and_then(Value::as_object)
            .ok_or_else(|| color_eyre::eyre::eyre!("missing parse_simple benchmark"))?;
        assert_eq!(parse_simple.get("mean_ns"), Some(&Value::from(1000_u64)));
        Ok(())
    }
}
