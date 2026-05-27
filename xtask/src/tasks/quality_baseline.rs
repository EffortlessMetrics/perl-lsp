//! Coverage baseline receipt generation for the proof lane.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::eyre::{Context, Result, bail};
use serde_json::{Value as JsonValue, json};
use serde_yaml_ng::Value as YamlValue;

#[derive(Debug)]
pub struct CoverageBaselineArgs {
    pub lcov: PathBuf,
    pub receipt: PathBuf,
    pub codecov: PathBuf,
    pub patch_coverage: Option<f64>,
    pub check: bool,
}

#[derive(Debug, Default)]
struct LcovSummary {
    line_hit: u64,
    line_found: u64,
    files: Vec<FileCoverage>,
}

#[derive(Debug, Default)]
struct FileCoverage {
    path: String,
    line_hit: u64,
    line_found: u64,
    uncovered_lines: Vec<u64>,
}

pub fn run(args: CoverageBaselineArgs) -> Result<()> {
    let root = std::env::current_dir().context("resolving current directory")?;
    let receipt = build_receipt(&root, &args)?;
    let rendered = render_json(&receipt)?;

    if args.check {
        let existing = fs::read_to_string(&args.receipt)
            .with_context(|| format!("reading coverage receipt {}", args.receipt.display()))?;
        if normalize(&existing) != normalize(&rendered) {
            bail!(
                "coverage baseline receipt is stale: regenerate with `rtk cargo xtask coverage-baseline --lcov {} --receipt {} --codecov {}`",
                args.lcov.display(),
                args.receipt.display(),
                args.codecov.display()
            );
        }
        println!("coverage baseline receipt is current: {}", args.receipt.display());
        return Ok(());
    }

    write_text(&args.receipt, &rendered)?;
    println!("wrote coverage baseline receipt: {}", args.receipt.display());
    Ok(())
}

fn build_receipt(root: &Path, args: &CoverageBaselineArgs) -> Result<JsonValue> {
    let lcov = parse_lcov(&args.lcov)?;
    let codecov = read_codecov_status(&args.codecov)?;
    let line_coverage = percent(lcov.line_hit, lcov.line_found);
    let files_below_target = lcov
        .files
        .iter()
        .filter(|file| !file.path.trim().is_empty())
        .filter(|file| file.line_found > 0 && percent(file.line_hit, file.line_found) < 95.0)
        .filter_map(file_gap_json)
        .collect::<Vec<_>>();

    let mut coverage = serde_json::Map::new();
    if let Some(patch) = args.patch_coverage {
        coverage.insert("patch".to_string(), json!(round2(patch)));
    }

    Ok(json!({
        "schema_version": 1,
        "kind": "coverage_baseline",
        "head": current_head(root)?,
        "lcov": display_path(&args.lcov),
        "coverage": coverage,
        "codecov_status": codecov,
        "coverage_scope": {
            "kind": "lcov",
            "source_files": lcov.files.len(),
        },
        "measured": {
            "line_hit": lcov.line_hit,
            "line_found": lcov.line_found,
            "line_coverage": line_coverage,
        },
        "files_below_target": files_below_target,
    }))
}

fn parse_lcov(path: &Path) -> Result<LcovSummary> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading LCOV {}", path.display()))?;
    let mut summary = LcovSummary::default();
    let mut current = FileCoverage::default();

    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("SF:") {
            finish_file(&mut summary, &mut current);
            current.path = path.trim().replace('\\', "/");
            continue;
        }

        if let Some(entry) = line.strip_prefix("DA:") {
            let Some((line_number, hit_count)) = entry.split_once(',') else {
                continue;
            };
            let Ok(line_number) = line_number.trim().parse::<u64>() else {
                continue;
            };
            if line_number == 0 {
                continue;
            }
            let Ok(hit_count) = hit_count.trim().parse::<u64>() else {
                continue;
            };
            current.line_found += 1;
            if hit_count > 0 {
                current.line_hit += 1;
            } else {
                current.uncovered_lines.push(line_number);
            }
            continue;
        }

        if line.trim() == "end_of_record" {
            finish_file(&mut summary, &mut current);
        }
    }
    finish_file(&mut summary, &mut current);

    Ok(summary)
}

fn finish_file(summary: &mut LcovSummary, current: &mut FileCoverage) {
    if current.path.trim().is_empty() && current.line_found == 0 {
        return;
    }
    summary.line_hit += current.line_hit;
    summary.line_found += current.line_found;
    summary.files.push(std::mem::take(current));
}

fn read_codecov_status(path: &Path) -> Result<JsonValue> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading Codecov config {}", path.display()))?;
    let parsed: YamlValue = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing Codecov config {}", path.display()))?;
    let status = yaml_path(&parsed, &["coverage", "status"]).unwrap_or(&YamlValue::Null);
    yaml_to_json(status)
}

fn yaml_path<'a>(value: &'a YamlValue, path: &[&str]) -> Option<&'a YamlValue> {
    let mut current = value;
    for key in path {
        current = match current {
            YamlValue::Mapping(mapping) => mapping.get(YamlValue::String((*key).to_string()))?,
            _ => return None,
        };
    }
    Some(current)
}

fn yaml_to_json(value: &YamlValue) -> Result<JsonValue> {
    serde_json::to_value(value).context("converting YAML value to JSON")
}

fn file_gap_json(file: &FileCoverage) -> Option<JsonValue> {
    let samples =
        file.uncovered_lines.iter().copied().filter(|line| *line > 0).take(10).collect::<Vec<_>>();
    if samples.is_empty() {
        return None;
    }

    Some(json!({
        "path": file.path,
        "line_hit": file.line_hit,
        "line_found": file.line_found,
        "line_coverage": percent(file.line_hit, file.line_found),
        "sample_uncovered_lines": samples,
    }))
}

fn current_head(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .context("running git rev-parse HEAD")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed with status {}", output.status);
    }
    Ok(String::from_utf8(output.stdout)
        .context("git rev-parse HEAD returned non-UTF8 output")?
        .trim()
        .to_string())
}

fn percent(hit: u64, found: u64) -> f64 {
    if found == 0 { 0.0 } else { round2(hit as f64 * 100.0 / found as f64) }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn render_json(value: &JsonValue) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
