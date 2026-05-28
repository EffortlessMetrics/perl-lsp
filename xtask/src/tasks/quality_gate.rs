//! Quality gates for the proof lane.

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
const NEW_RIPR_GAP_SUGGESTED_TEST: &str = "Add or update the focused test named by RIPR review guidance for the changed file, line, and seam.";

#[derive(Clone, Debug, ValueEnum)]
pub enum QualityGateMode {
    EnforcePatchCoverage,
    EnforceNewRipr,
}

impl QualityGateMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::EnforcePatchCoverage => "enforce-patch-coverage",
            Self::EnforceNewRipr => "enforce-new-ripr",
        }
    }
}

#[derive(Debug)]
pub struct QualityGateArgs {
    pub mode: QualityGateMode,
    pub ripr_receipt: PathBuf,
    pub ripr_pr_receipt: PathBuf,
    pub review_receipt: PathBuf,
    pub coverage_receipt: PathBuf,
    pub codecov: PathBuf,
    pub patch_coverage: Option<f64>,
    pub ripr_base: String,
    pub ripr_head: String,
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
    match args.mode {
        QualityGateMode::EnforcePatchCoverage => evaluate_patch_coverage(&head, args),
        QualityGateMode::EnforceNewRipr => evaluate_new_ripr(&head, args),
    }
}

fn evaluate_patch_coverage(head: &str, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let codecov_status = read_codecov_patch_status(&args.codecov)?;
    let coverage = read_coverage_receipt(&args.coverage_receipt, head);
    let mut next_actions = Vec::new();

    if !matches!(coverage.status.as_str(), "present") {
        next_actions.push(coverage_receipt_action(&coverage, head, args));
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

fn evaluate_new_ripr(head: &str, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let ripr = read_ripr_plus_receipt(&args.ripr_receipt, head);
    let ripr_pr = read_ripr_pr_receipt(&args.ripr_pr_receipt, head);
    let review = read_review_guidance_receipt(&args.review_receipt, head);
    let mut next_actions = Vec::new();

    if ripr.status != "present" {
        next_actions.push(ripr_receipt_action(&ripr, head, args));
    }
    if ripr_pr.status != "present" {
        next_actions.push(ripr_pr_receipt_action(&ripr_pr, head, args));
    }
    if review.status != "present" {
        next_actions.push(ripr_review_receipt_action(&review, head, args));
    }

    if ripr_pr.status == "present" {
        match ripr_pr.new_unresolved {
            Some(count) if count > 0 => {
                next_actions.push(new_ripr_gap_action(count, &ripr_pr, &review, args));
                if review.status == "present" && review.top_gaps.is_empty() {
                    next_actions.push(ripr_review_guidance_gap_action(&review, head, args));
                }
            }
            None => next_actions.push(new_ripr_gap_unknown_action(&ripr_pr, args)),
            _ => {}
        }
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
        "ripr_plus": {
            "status": ripr.status,
            "receipt": display_path(&args.ripr_receipt),
            "receipt_head": ripr.receipt_head,
            "expected_head": head,
            "unresolved": ripr.unresolved,
        },
        "ripr_pr": {
            "status": ripr_pr.status,
            "receipt": display_path(&args.ripr_pr_receipt),
            "receipt_head_sha": ripr_pr.receipt_head_sha,
            "expected_head_sha": head,
            "base": ripr_pr.base,
            "base_sha": ripr_pr.base_sha,
            "new_unresolved": ripr_pr.new_unresolved,
        },
        "review_guidance": {
            "status": review.status,
            "receipt": display_path(&args.review_receipt),
            "receipt_head_sha": review.receipt_head_sha,
            "expected_head_sha": head,
            "base": review.base,
            "base_sha": review.base_sha,
            "top_gaps": review.top_gaps,
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

#[derive(Debug)]
struct RiprPlusReceipt {
    status: String,
    receipt_head: Option<String>,
    unresolved: Option<u64>,
}

#[derive(Debug)]
struct RiprPrReceipt {
    status: String,
    receipt_head_sha: Option<String>,
    base: Option<String>,
    base_sha: Option<String>,
    new_unresolved: Option<u64>,
}

#[derive(Debug)]
struct ReviewGuidanceReceipt {
    status: String,
    receipt_head_sha: Option<String>,
    base: Option<String>,
    base_sha: Option<String>,
    top_gaps: Vec<Value>,
}

enum JsonReceipt {
    Missing,
    Invalid,
    Present(Value),
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

fn read_json_receipt(path: &Path) -> JsonReceipt {
    let Ok(raw) = fs::read_to_string(path) else {
        return JsonReceipt::Missing;
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(value) => JsonReceipt::Present(value),
        Err(_) => JsonReceipt::Invalid,
    }
}

fn read_ripr_plus_receipt(path: &Path, expected_head: &str) -> RiprPlusReceipt {
    match read_json_receipt(path) {
        JsonReceipt::Missing => {
            RiprPlusReceipt { status: "missing".to_string(), receipt_head: None, unresolved: None }
        }
        JsonReceipt::Invalid => {
            RiprPlusReceipt { status: "invalid".to_string(), receipt_head: None, unresolved: None }
        }
        JsonReceipt::Present(payload) => {
            let receipt_head = payload.get("head").and_then(Value::as_str).map(ToOwned::to_owned);
            let status =
                if receipt_head.as_deref() == Some(expected_head) { "present" } else { "stale" };
            RiprPlusReceipt {
                status: status.to_string(),
                receipt_head,
                unresolved: payload.get("unresolved").and_then(Value::as_u64),
            }
        }
    }
}

fn read_ripr_pr_receipt(path: &Path, expected_head: &str) -> RiprPrReceipt {
    match read_json_receipt(path) {
        JsonReceipt::Missing => RiprPrReceipt {
            status: "missing".to_string(),
            receipt_head_sha: None,
            base: None,
            base_sha: None,
            new_unresolved: None,
        },
        JsonReceipt::Invalid => RiprPrReceipt {
            status: "invalid".to_string(),
            receipt_head_sha: None,
            base: None,
            base_sha: None,
            new_unresolved: None,
        },
        JsonReceipt::Present(payload) => {
            let receipt_head_sha =
                payload.get("head_sha").and_then(Value::as_str).map(ToOwned::to_owned);
            let status = if receipt_head_sha.as_deref() == Some(expected_head) {
                "present"
            } else {
                "stale"
            };
            RiprPrReceipt {
                status: status.to_string(),
                receipt_head_sha,
                base: payload.get("base").and_then(Value::as_str).map(ToOwned::to_owned),
                base_sha: payload.get("base_sha").and_then(Value::as_str).map(ToOwned::to_owned),
                new_unresolved: payload.pointer("/summary/severe_gaps").and_then(Value::as_u64),
            }
        }
    }
}

fn read_review_guidance_receipt(path: &Path, expected_head: &str) -> ReviewGuidanceReceipt {
    match read_json_receipt(path) {
        JsonReceipt::Missing => ReviewGuidanceReceipt {
            status: "missing".to_string(),
            receipt_head_sha: None,
            base: None,
            base_sha: None,
            top_gaps: Vec::new(),
        },
        JsonReceipt::Invalid => ReviewGuidanceReceipt {
            status: "invalid".to_string(),
            receipt_head_sha: None,
            base: None,
            base_sha: None,
            top_gaps: Vec::new(),
        },
        JsonReceipt::Present(payload) => {
            let receipt_head_sha =
                payload.get("head_sha").and_then(Value::as_str).map(ToOwned::to_owned);
            let mut status = if receipt_head_sha.as_deref() == Some(expected_head) {
                "present"
            } else {
                "stale"
            }
            .to_string();
            let top_gaps =
                if status == "present" { review_guidance_items(&payload, 3) } else { Vec::new() };
            if status == "present"
                && top_gaps.is_empty()
                && review_guidance_declares_items(&payload)
            {
                status = "incomplete".to_string();
            }

            ReviewGuidanceReceipt {
                status,
                receipt_head_sha,
                base: payload.get("base").and_then(Value::as_str).map(ToOwned::to_owned),
                base_sha: payload.get("base_sha").and_then(Value::as_str).map(ToOwned::to_owned),
                top_gaps,
            }
        }
    }
}

fn review_guidance_items(value: &Value, limit: usize) -> Vec<Value> {
    let mut gaps = Vec::new();
    for source in ["comments", "summary_only"] {
        let Some(items) = value.get(source).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            if gaps.len() >= limit {
                return gaps;
            }
            let gap = review_guidance_item(source, item);
            if review_guidance_item_is_actionable(&gap) {
                gaps.push(gap);
            }
        }
    }
    gaps
}

fn review_guidance_declares_items(value: &Value) -> bool {
    ["comments", "summary_only"].iter().any(|field| {
        value.get(*field).and_then(Value::as_array).is_some_and(|items| !items.is_empty())
    }) || ["/summary/comments", "/summary/summary_only"].iter().any(|pointer| {
        value.pointer(pointer).and_then(Value::as_u64).is_some_and(|count| count > 0)
    })
}

fn review_guidance_item_is_actionable(item: &Value) -> bool {
    string_field_is_filled(item, "gap_id")
        && string_field_is_filled(item, "path")
        && item.get("line").and_then(Value::as_u64).is_some_and(|line| line > 0)
        && string_field_is_filled(item, "seam")
        && string_field_is_filled(item, "reason")
        && string_field_is_filled(item, "suggested_test")
}

fn string_field_is_filled(item: &Value, field: &str) -> bool {
    item.get(field).and_then(Value::as_str).is_some_and(|value| !value.trim().is_empty())
}

fn review_guidance_item(source: &str, item: &Value) -> Value {
    json!({
        "source": source,
        "gap_id": first_string(item, &["/canonical_gap_id", "/gap_id", "/identity/canonical_gap_id"]),
        "path": first_string(item, &["/placement/path", "/path", "/file"]),
        "line": first_u64(item, &["/placement/line", "/line"]),
        "seam": first_string(item, &["/seam", "/placement/mode", "/owner", "/evidence_record/seam"]),
        "reason": first_string(item, &["/reason", "/why", "/message", "/kind"]),
        "suggested_test": first_string(item, &["/suggested_test/intent", "/suggested_test", "/repair", "/test"]),
    })
}

fn first_string(item: &Value, pointers: &[&str]) -> Option<String> {
    pointers.iter().find_map(|pointer| {
        item.pointer(pointer).and_then(Value::as_str).and_then(|value| {
            let value = value.trim();
            if value.is_empty() { None } else { Some(value.to_string()) }
        })
    })
}

fn first_u64(item: &Value, pointers: &[&str]) -> Option<u64> {
    pointers.iter().find_map(|pointer| item.pointer(pointer).and_then(Value::as_u64))
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

fn ripr_receipt_action(
    ripr: &RiprPlusReceipt,
    expected_head: &str,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "ripr_receipt_not_current",
        "blocking": true,
        "path": display_path(&args.ripr_receipt),
        "reason": ripr.status,
        "receipt_head": ripr.receipt_head,
        "expected_head": expected_head,
        "repair": "Regenerate and check the repo-wide RIPR+ receipt. This transition gate does not require total RIPR+ zero yet, but it does require current total-debt proof.",
        "verify": ripr_plus_command(args, true),
        "receipt": ripr_plus_command(args, false),
    })
}

fn ripr_pr_receipt_action(
    ripr_pr: &RiprPrReceipt,
    expected_head: &str,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "ripr_pr_receipt_not_current",
        "blocking": true,
        "path": display_path(&args.ripr_pr_receipt),
        "reason": ripr_pr.status,
        "receipt_head_sha": ripr_pr.receipt_head_sha,
        "expected_head_sha": expected_head,
        "repair": "Regenerate and check the diff-scoped RIPR PR receipt so new severe gaps are measured against this HEAD.",
        "verify": ripr_pr_command(args, true),
        "receipt": ripr_pr_command(args, false),
    })
}

fn ripr_review_receipt_action(
    review: &ReviewGuidanceReceipt,
    expected_head: &str,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "ripr_review_receipt_not_current",
        "blocking": true,
        "path": display_path(&args.review_receipt),
        "reason": review.status,
        "receipt_head_sha": review.receipt_head_sha,
        "expected_head_sha": expected_head,
        "repair": "Regenerate and check the RIPR review-guidance receipt for this HEAD so failing gates can name the exact file, line, seam, and suggested proof.",
        "verify": ripr_review_command(args, true),
        "receipt": ripr_review_command(args, false),
    })
}

fn ripr_review_guidance_gap_action(
    review: &ReviewGuidanceReceipt,
    expected_head: &str,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "ripr_review_guidance_not_actionable",
        "blocking": true,
        "path": display_path(&args.review_receipt),
        "reason": if review.status == "present" { "no_actionable_top_gaps" } else { review.status.as_str() },
        "receipt_head_sha": review.receipt_head_sha,
        "expected_head_sha": expected_head,
        "repair": "Regenerate RIPR review guidance so new-gap failures include actionable gap id, file, positive line, seam, reason, and suggested test.",
        "verify": ripr_review_command(args, true),
        "receipt": ripr_review_command(args, false),
    })
}

fn new_ripr_gap_unknown_action(ripr_pr: &RiprPrReceipt, args: &QualityGateArgs) -> Value {
    json!({
        "kind": "new_ripr_gap_unknown",
        "blocking": true,
        "path": display_path(&args.ripr_pr_receipt),
        "reason": "diff-scoped summary.severe_gaps is not measured",
        "receipt_head_sha": ripr_pr.receipt_head_sha,
        "repair": "Regenerate the diff-scoped RIPR PR receipt so new-gap count comes from summary.severe_gaps.",
        "verify": ripr_pr_command(args, true),
        "receipt": ripr_pr_command(args, false),
    })
}

fn new_ripr_gap_action(
    count: u64,
    ripr_pr: &RiprPrReceipt,
    review: &ReviewGuidanceReceipt,
    args: &QualityGateArgs,
) -> Value {
    let path = review
        .top_gaps
        .first()
        .and_then(|gap| gap.get("path"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            args.ripr_pr_receipt.to_str().unwrap_or("target/ripr/pr/repo-exposure.json")
        });

    json!({
        "kind": "new_ripr_gap",
        "blocking": true,
        "path": path,
        "new_unresolved": count,
        "receipt_head_sha": ripr_pr.receipt_head_sha,
        "top_gaps": review.top_gaps,
        "suggested_test": NEW_RIPR_GAP_SUGGESTED_TEST,
        "repair": "Add focused tests that expose the new RIPR seam before merging, then refresh RIPR receipts.",
        "verify": quality_gate_command(args, true, None),
        "receipt": quality_gate_command(args, false, None),
    })
}

fn render_markdown(receipt: &Value, args: &QualityGateArgs) -> Result<String> {
    let decision = receipt.get("decision").and_then(Value::as_str).unwrap_or("unknown");

    let mut markdown = String::new();
    markdown.push_str("# Quality Gate\n\n");
    markdown.push_str("## Quality Gates\n\n");
    markdown.push_str(&format!("- decision: `{decision}`\n"));
    markdown.push_str(&format!("- mode: `{}`\n", args.mode.as_str()));
    if let Some(coverage) = receipt.get("coverage") {
        let patch = coverage
            .get("patch")
            .and_then(Value::as_f64)
            .map(|value| format!("{value:.2}%"))
            .unwrap_or_else(|| "unknown".to_string());
        let source = coverage.get("patch_source").and_then(Value::as_str).unwrap_or("unknown");
        let status = coverage.get("status").and_then(Value::as_str).unwrap_or("unknown");
        markdown.push_str(&format!("- coverage receipt: `{status}`\n"));
        markdown.push_str(&format!("- patch coverage: `{patch}` / `95.00%`\n"));
        markdown.push_str(&format!("- patch source: `{source}`\n"));
    }
    if let Some(ripr_pr) = receipt.get("ripr_pr") {
        let status = ripr_pr.get("status").and_then(Value::as_str).unwrap_or("unknown");
        let new_unresolved = ripr_pr
            .get("new_unresolved")
            .and_then(Value::as_u64)
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        markdown.push_str(&format!("- diff RIPR receipt: `{status}`\n"));
        markdown.push_str(&format!("- new RIPR gaps: `{new_unresolved}`\n"));
    }
    if let Some(ripr) = receipt.get("ripr_plus") {
        let status = ripr.get("status").and_then(Value::as_str).unwrap_or("unknown");
        let unresolved = ripr
            .get("unresolved")
            .and_then(Value::as_u64)
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        markdown.push_str(&format!("- repo RIPR+ receipt: `{status}`\n"));
        markdown.push_str(&format!("- total RIPR+ gaps: `{unresolved}`\n"));
    }
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
        if let Some(gaps) = action.get("top_gaps").and_then(Value::as_array) {
            for gap in gaps {
                let gap_id = gap.get("gap_id").and_then(Value::as_str).unwrap_or("unknown");
                let path = gap.get("path").and_then(Value::as_str).unwrap_or("unknown");
                let line = gap.get("line").and_then(Value::as_u64).unwrap_or(0);
                let seam = gap.get("seam").and_then(Value::as_str).unwrap_or("unknown");
                let reason = gap.get("reason").and_then(Value::as_str).unwrap_or("unspecified");
                let suggested_test = gap
                    .get("suggested_test")
                    .and_then(Value::as_str)
                    .unwrap_or("add focused proof");
                markdown.push_str(&format!(
                    "- gap: `{gap_id}` `{path}:{line}` seam `{seam}` reason `{reason}` suggested test `{suggested_test}`\n"
                ));
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
    let mut command = format!("rtk cargo xtask quality-gate --mode {}", args.mode.as_str());
    match args.mode {
        QualityGateMode::EnforcePatchCoverage => {
            command.push_str(&format!(
                " --coverage-receipt {} --codecov {}",
                args.coverage_receipt.display(),
                args.codecov.display()
            ));
        }
        QualityGateMode::EnforceNewRipr => {
            command.push_str(&format!(
                " --ripr-receipt {} --ripr-pr-receipt {} --review-receipt {} --ripr-base {} --ripr-head {}",
                args.ripr_receipt.display(),
                args.ripr_pr_receipt.display(),
                args.review_receipt.display(),
                args.ripr_base,
                args.ripr_head
            ));
        }
    }
    command.push_str(&format!(
        " --receipt {} --summary {}",
        args.receipt.display(),
        args.summary.display()
    ));
    if let Some(patch) = patch {
        command.push_str(&format!(" --patch-coverage {patch:.2}"));
    }
    if check {
        command.push_str(" --check");
    }
    command
}

fn ripr_plus_command(args: &QualityGateArgs, check: bool) -> String {
    let mut command =
        format!("rtk cargo xtask ripr-plus --receipt {}", args.ripr_receipt.display());
    if check {
        command.push_str(" --check");
    }
    command
}

fn ripr_pr_command(args: &QualityGateArgs, check: bool) -> String {
    let mut command =
        format!("rtk cargo xtask ripr-pr --base {} --head {}", args.ripr_base, args.ripr_head);
    if check {
        command.push_str(" --check");
    }
    command
}

fn ripr_review_command(args: &QualityGateArgs, check: bool) -> String {
    let mut command = format!(
        "rtk cargo xtask ripr-review-comments --base {} --head {}",
        args.ripr_base, args.ripr_head
    );
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
