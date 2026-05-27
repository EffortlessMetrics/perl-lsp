//! Patch coverage quality gate for the proof lane.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

const PATCH_TARGET: f64 = 95.0;

#[derive(Clone, Debug, ValueEnum)]
pub enum QualityGateMode {
    EnforcePatchCoverage,
}

impl QualityGateMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::EnforcePatchCoverage => "enforce-patch-coverage",
        }
    }
}

#[derive(Debug)]
pub struct QualityGateArgs {
    pub mode: QualityGateMode,
    pub coverage_receipt: PathBuf,
    pub codecov: PathBuf,
    pub patch_coverage: Option<f64>,
    pub receipt: PathBuf,
    pub summary: PathBuf,
    pub check: bool,
}

#[derive(Debug)]
struct GateEvaluation {
    receipt: Value,
    markdown: String,
    failed: bool,
}

pub fn run(args: QualityGateArgs) -> Result<()> {
    let root = std::env::current_dir().context("resolving current directory")?;
    let evaluation = evaluate(&root, &args)?;
    let receipt_text = render_json(&evaluation.receipt)?;

    if args.check {
        assert_current(&args.receipt, &receipt_text, "quality gate JSON receipt")?;
        assert_current(&args.summary, &evaluation.markdown, "quality gate Markdown summary")?;
    } else {
        write_text(&args.receipt, &receipt_text)?;
        write_text(&args.summary, &evaluation.markdown)?;
    }

    if evaluation.failed {
        bail!(
            "quality gate failed; see receipt {} and summary {}",
            args.receipt.display(),
            args.summary.display()
        );
    }

    println!(
        "quality gate passed; receipt {} summary {}",
        args.receipt.display(),
        args.summary.display()
    );
    Ok(())
}

fn evaluate(root: &Path, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let head = current_head(root)?;
    let codecov_status = read_codecov_patch_status(&args.codecov)?;
    let coverage = read_coverage_receipt(&args.coverage_receipt, &head);
    let mut next_actions = Vec::new();

    if !matches!(coverage.status.as_str(), "present") {
        next_actions.push(coverage_receipt_action(&coverage, &head, args));
    }

    let patch = args.patch_coverage.or(coverage.patch);
    let patch_source = if args.patch_coverage.is_some() {
        Some("cli")
    } else if coverage.patch.is_some() {
        Some("coverage_receipt")
    } else {
        None
    };

    if coverage.status == "present" && patch.is_none() {
        next_actions.push(patch_coverage_unknown_action(args));
    }

    if let Some(patch) = patch {
        if patch < PATCH_TARGET {
            next_actions.push(patch_coverage_below_target_action(
                patch,
                patch_source.unwrap_or("unknown"),
                &coverage,
                args,
            ));
        }
    }

    if codecov_status != "present" {
        next_actions.push(codecov_policy_action(&codecov_status, args));
    }

    let failed = next_actions
        .iter()
        .any(|action| action.get("blocking").and_then(Value::as_bool) == Some(true));
    let decision = if failed { "fail" } else { "pass" };

    let receipt = json!({
        "schema_version": 1,
        "kind": "quality_gate",
        "mode": args.mode.as_str(),
        "decision": decision,
        "head": head,
        "coverage": {
            "status": coverage.status,
            "receipt": display_path(&args.coverage_receipt),
            "receipt_head": coverage.receipt_head,
            "patch": patch.map(round2),
            "patch_source": patch_source,
            "target": PATCH_TARGET,
            "lcov": coverage.lcov,
            "codecov_config": display_path(&args.codecov),
            "codecov_config_status": codecov_status,
        },
        "next_actions": next_actions,
    });
    let markdown = render_markdown(&receipt, args)?;

    Ok(GateEvaluation { receipt, markdown, failed })
}

#[derive(Debug)]
struct CoverageReceipt {
    status: String,
    receipt_head: Option<String>,
    lcov: Option<String>,
    patch: Option<f64>,
    top_files: Vec<Value>,
}

fn read_coverage_receipt(path: &Path, expected_head: &str) -> CoverageReceipt {
    let Ok(raw) = fs::read_to_string(path) else {
        return CoverageReceipt {
            status: "missing".to_string(),
            receipt_head: None,
            lcov: None,
            patch: None,
            top_files: Vec::new(),
        };
    };
    let Ok(payload) = serde_json::from_str::<Value>(&raw) else {
        return CoverageReceipt {
            status: "invalid".to_string(),
            receipt_head: None,
            lcov: None,
            patch: None,
            top_files: Vec::new(),
        };
    };

    let receipt_head = payload.get("head").and_then(Value::as_str).map(ToOwned::to_owned);
    let status = if receipt_head.as_deref() == Some(expected_head) { "present" } else { "stale" };
    let patch = payload.pointer("/coverage/patch").and_then(Value::as_f64);
    let lcov = payload.get("lcov").and_then(Value::as_str).map(ToOwned::to_owned);
    let top_files = payload
        .get("files_below_target")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(actionable_file_gap).take(3).collect::<Vec<_>>())
        .unwrap_or_default();

    CoverageReceipt { status: status.to_string(), receipt_head, lcov, patch, top_files }
}

fn actionable_file_gap(file: &Value) -> Option<Value> {
    let path = file.get("path").and_then(Value::as_str)?.trim();
    if path.is_empty() {
        return None;
    }
    let samples = file
        .get("sample_uncovered_lines")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_u64)
        .filter(|line| *line > 0)
        .take(10)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return None;
    }

    Some(json!({
        "path": path,
        "line_coverage": file.get("line_coverage").and_then(Value::as_f64),
        "sample_uncovered_lines": samples,
    }))
}

fn read_codecov_patch_status(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading Codecov config {}", path.display()))?;
    let parsed: YamlValue = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing Codecov config {}", path.display()))?;
    let target = yaml_path(&parsed, &["coverage", "status", "patch", "default", "target"])
        .and_then(yaml_scalar);
    let threshold = yaml_path(&parsed, &["coverage", "status", "patch", "default", "threshold"])
        .and_then(yaml_scalar);
    let informational =
        yaml_path(&parsed, &["coverage", "status", "patch", "default", "informational"])
            .and_then(yaml_scalar);

    if target.as_deref() == Some("95%")
        && threshold.as_deref() == Some("0%")
        && informational.as_deref() != Some("true")
    {
        Ok("present".to_string())
    } else {
        Ok("patch_policy_not_blocking".to_string())
    }
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

fn yaml_scalar(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn coverage_receipt_action(
    coverage: &CoverageReceipt,
    expected_head: &str,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "coverage_receipt_not_current",
        "blocking": true,
        "path": display_path(&args.coverage_receipt),
        "reason": coverage.status,
        "receipt_head": coverage.receipt_head,
        "expected_head": expected_head,
        "repair": "Refresh the LCOV coverage receipt before running the aggregate quality gate.",
        "verify": coverage_baseline_command(args, true),
        "receipt": coverage_baseline_command(args, false),
    })
}

fn patch_coverage_unknown_action(args: &QualityGateArgs) -> Value {
    json!({
        "kind": "patch_coverage_unknown",
        "blocking": true,
        "path": display_path(&args.coverage_receipt),
        "reason": "coverage receipt did not include coverage.patch and no --patch-coverage value was provided",
        "repair": "Record the PR patch coverage percentage from Codecov or regenerate the coverage receipt with patch coverage evidence.",
        "verify": coverage_baseline_command(args, true),
        "receipt": coverage_baseline_command(args, false),
    })
}

fn patch_coverage_below_target_action(
    patch: f64,
    source: &str,
    coverage: &CoverageReceipt,
    args: &QualityGateArgs,
) -> Value {
    let path = coverage
        .top_files
        .first()
        .and_then(|file| file.get("path"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| args.coverage_receipt.to_str().unwrap_or("coverage"));

    json!({
        "kind": "patch_coverage_below_target",
        "blocking": true,
        "path": path,
        "current": round2(patch),
        "target": PATCH_TARGET,
        "source": source,
        "top_files": coverage.top_files,
        "suggested_test": "Prefer focused tests for error paths, boundary conditions, config parsing, serialization, cancellation, and output contracts.",
        "repair": "Add behavior-oriented tests for the uncovered changed-code surfaces, then refresh coverage evidence.",
        "verify": quality_gate_command(args, true, Some(patch)),
        "receipt": quality_gate_command(args, false, Some(patch)),
    })
}

fn codecov_policy_action(status: &str, args: &QualityGateArgs) -> Value {
    json!({
        "kind": "codecov_patch_policy_not_blocking",
        "blocking": true,
        "path": display_path(&args.codecov),
        "reason": status,
        "repair": "Set Codecov patch status to target 95%, threshold 0%, and keep it blocking.",
        "verify": quality_gate_command(args, true, args.patch_coverage),
        "receipt": quality_gate_command(args, false, args.patch_coverage),
    })
}

fn render_markdown(receipt: &Value, args: &QualityGateArgs) -> Result<String> {
    let decision = receipt.get("decision").and_then(Value::as_str).unwrap_or("unknown");
    let coverage = receipt.get("coverage").unwrap_or(&Value::Null);
    let patch = coverage
        .get("patch")
        .and_then(Value::as_f64)
        .map(|value| format!("{value:.2}%"))
        .unwrap_or_else(|| "unknown".to_string());
    let source = coverage.get("patch_source").and_then(Value::as_str).unwrap_or("unknown");
    let status = coverage.get("status").and_then(Value::as_str).unwrap_or("unknown");

    let mut markdown = String::new();
    markdown.push_str("# Quality Gate\n\n");
    markdown.push_str("## Quality Gates\n\n");
    markdown.push_str(&format!("- decision: `{decision}`\n"));
    markdown.push_str(&format!("- mode: `{}`\n", args.mode.as_str()));
    markdown.push_str(&format!("- coverage receipt: `{status}`\n"));
    markdown.push_str(&format!("- patch coverage: `{patch}` / `95.00%`\n"));
    markdown.push_str(&format!("- patch source: `{source}`\n"));
    markdown.push_str(&format!(
        "- verify: `{}`\n",
        quality_gate_command(args, true, args.patch_coverage)
    ));
    markdown.push('\n');

    let actions =
        receipt.get("next_actions").and_then(Value::as_array).cloned().unwrap_or_default();
    if actions.is_empty() {
        markdown.push_str("## Next Actions\n\n- none\n");
        return Ok(markdown);
    }

    markdown.push_str("## Next Actions\n\n");
    for action in actions {
        let kind = action.get("kind").and_then(Value::as_str).unwrap_or("unknown");
        markdown.push_str(&format!("### {kind}\n\n"));
        for field in ["path", "reason", "repair", "verify", "receipt", "suggested_test"] {
            if let Some(value) = action.get(field).and_then(Value::as_str) {
                markdown.push_str(&format!("- {field}: `{value}`\n"));
            }
        }
        if let Some(files) = action.get("top_files").and_then(Value::as_array) {
            for file in files {
                let path = file.get("path").and_then(Value::as_str).unwrap_or("unknown");
                let samples = file
                    .get("sample_uncovered_lines")
                    .and_then(Value::as_array)
                    .map(|lines| {
                        lines
                            .iter()
                            .filter_map(Value::as_u64)
                            .map(|line| line.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                markdown.push_str(&format!("- file: `{path}` sample uncovered lines: {samples}\n"));
            }
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn coverage_baseline_command(args: &QualityGateArgs, check: bool) -> String {
    let mut command = format!(
        "rtk cargo xtask coverage-baseline --lcov target/lcov.info --receipt {} --codecov {}",
        args.coverage_receipt.display(),
        args.codecov.display()
    );
    if let Some(patch) = args.patch_coverage {
        command.push_str(&format!(" --patch-coverage {patch:.2}"));
    }
    if check {
        command.push_str(" --check");
    }
    command
}

fn quality_gate_command(args: &QualityGateArgs, check: bool, patch: Option<f64>) -> String {
    let mut command = format!(
        "rtk cargo xtask quality-gate --mode {} --coverage-receipt {} --codecov {} --receipt {} --summary {}",
        args.mode.as_str(),
        args.coverage_receipt.display(),
        args.codecov.display(),
        args.receipt.display(),
        args.summary.display()
    );
    if let Some(patch) = patch {
        command.push_str(&format!(" --patch-coverage {patch:.2}"));
    }
    if check {
        command.push_str(" --check");
    }
    command
}

fn assert_current(path: &Path, expected: &str, label: &str) -> Result<()> {
    let existing =
        fs::read_to_string(path).with_context(|| format!("reading {label} {}", path.display()))?;
    if normalize(&existing) != normalize(expected) {
        bail!("{label} is stale: {}", path.display());
    }
    Ok(())
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

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn render_json(value: &Value) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
