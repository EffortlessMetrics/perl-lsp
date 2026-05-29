//! Quality gates for the proof lane.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{NaiveDate, Utc};
use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

const PATCH_TARGET: f64 = 95.0;
const PROJECT_TARGET: f64 = 95.0;
const NEW_RIPR_GAP_SUGGESTED_TEST: &str = "Add or update the focused test named by RIPR review guidance for the changed file, line, and seam.";

#[derive(Clone, Debug, ValueEnum)]
pub enum QualityGateMode {
    Enforce,
    EnforcePatchCoverage,
    EnforceNewRipr,
}

impl QualityGateMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Enforce => "enforce",
            Self::EnforcePatchCoverage => "enforce-patch-coverage",
            Self::EnforceNewRipr => "enforce-new-ripr",
        }
    }
}

#[derive(Debug)]
pub struct QualityGateArgs {
    pub mode: QualityGateMode,
    pub exception_policy: PathBuf,
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
        QualityGateMode::Enforce => evaluate_final(&head, args),
        QualityGateMode::EnforcePatchCoverage => evaluate_patch_coverage(&head, args),
        QualityGateMode::EnforceNewRipr => evaluate_new_ripr(&head, args),
    }
}

fn evaluate_final(head: &str, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let codecov_patch_status = read_codecov_patch_status(&args.codecov)?;
    let codecov_project_status = read_codecov_project_status(&args.codecov)?;
    let coverage = read_coverage_receipt(&args.coverage_receipt, head);
    let ripr = read_ripr_plus_receipt(&args.ripr_receipt, head);
    let ripr_pr = read_ripr_pr_receipt(&args.ripr_pr_receipt, head);
    let review = read_review_guidance_receipt(&args.review_receipt, head);
    let exceptions = read_exception_policy(&args.exception_policy, today());
    let mut next_actions = Vec::new();
    next_actions.extend(exceptions.actions.clone());

    if !matches!(coverage.status.as_str(), "present") {
        next_actions.push(coverage_receipt_action(&coverage, head, args));
    }
    if coverage.status == "present" {
        let patch = args.patch_coverage.or(coverage.patch);
        let patch_source = if args.patch_coverage.is_some() {
            Some("cli")
        } else if coverage.patch.is_some() {
            Some("coverage_receipt")
        } else {
            None
        };
        if patch.is_none() {
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
        match coverage.project {
            Some(project) if project < PROJECT_TARGET => {
                next_actions.push(project_coverage_below_target_action(project, &coverage, args));
            }
            None => next_actions.push(project_coverage_unknown_action(args)),
            _ => {}
        }
        if coverage.scope.as_deref() != Some("workspace") {
            next_actions.push(coverage_scope_not_workspace_action(&coverage, args));
        }
    }

    if codecov_patch_status != "present" {
        next_actions.push(codecov_policy_action(&codecov_patch_status, args));
    }
    if codecov_project_status != "present" {
        next_actions.push(codecov_project_policy_action(&codecov_project_status, args));
    }

    if ripr.status != "present" {
        next_actions.push(ripr_receipt_action(&ripr, head, args));
    }
    if ripr.status == "present" {
        match ripr.unresolved {
            Some(count) if count > 0 => {
                next_actions.push(ripr_total_unresolved_action(count, args))
            }
            None => next_actions.push(ripr_total_unknown_action(args)),
            _ => {}
        }
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
    if exceptions.receipt.get("active_count").and_then(Value::as_u64).is_some_and(|count| count > 0)
    {
        next_actions.push(quality_exception_active_final_blocker_action(&exceptions, args));
    }

    let failed = next_actions
        .iter()
        .any(|action| action.get("blocking").and_then(Value::as_bool) == Some(true));
    let decision = if failed { "fail" } else { "pass" };
    let patch = args.patch_coverage.or(coverage.patch);

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
            "project": coverage.project.map(round2),
            "target": PROJECT_TARGET,
            "scope": coverage.scope,
            "lcov": coverage.lcov,
            "codecov_config": display_path(&args.codecov),
            "codecov_patch_status": codecov_patch_status,
            "codecov_project_status": codecov_project_status,
        },
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
        "temporary_exceptions": exceptions.receipt,
        "next_actions": next_actions,
    });
    let markdown = render_markdown(&receipt, args)?;

    Ok(GateEvaluation { receipt, markdown, failed })
}

fn evaluate_patch_coverage(head: &str, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let codecov_status = read_codecov_patch_status(&args.codecov)?;
    let coverage = read_coverage_receipt(&args.coverage_receipt, head);
    let exceptions = read_exception_policy(&args.exception_policy, today());
    let mut next_actions = Vec::new();
    next_actions.extend(exceptions.actions.clone());

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
            "project": coverage.project.map(round2),
            "target": PATCH_TARGET,
            "scope": coverage.scope,
            "lcov": coverage.lcov,
            "codecov_config": display_path(&args.codecov),
            "codecov_config_status": codecov_status,
        },
        "temporary_exceptions": exceptions.receipt,
        "next_actions": next_actions,
    });
    let markdown = render_markdown(&receipt, args)?;

    Ok(GateEvaluation { receipt, markdown, failed })
}

fn evaluate_new_ripr(head: &str, args: &QualityGateArgs) -> Result<GateEvaluation> {
    let ripr = read_ripr_plus_receipt(&args.ripr_receipt, head);
    let ripr_pr = read_ripr_pr_receipt(&args.ripr_pr_receipt, head);
    let review = read_review_guidance_receipt(&args.review_receipt, head);
    let exceptions = read_exception_policy(&args.exception_policy, today());
    let mut next_actions = Vec::new();
    next_actions.extend(exceptions.actions.clone());

    if ripr.status != "present" {
        next_actions.push(ripr_receipt_action(&ripr, head, args));
    }
    if ripr_pr.status != "present" {
        next_actions.push(ripr_pr_receipt_action(&ripr_pr, head, args));
    }
    let review_receipt_blocks_without_new_gaps =
        matches!(review.status.as_str(), "missing" | "invalid" | "stale");
    if review_receipt_blocks_without_new_gaps {
        next_actions.push(ripr_review_receipt_action(&review, head, args));
    }

    if ripr_pr.status == "present" {
        match ripr_pr.new_unresolved {
            Some(count) if count > 0 => {
                next_actions.push(new_ripr_gap_action(count, &ripr_pr, &review, args));
                if review.status != "present" {
                    if !review_receipt_blocks_without_new_gaps {
                        next_actions.push(ripr_review_receipt_action(&review, head, args));
                    }
                } else if review.top_gaps.is_empty() {
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
        "temporary_exceptions": exceptions.receipt,
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
    project: Option<f64>,
    scope: Option<String>,
    patch_files: Vec<Value>,
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

#[derive(Debug)]
struct ExceptionPolicyEvaluation {
    receipt: Value,
    actions: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct QualityExceptionPolicy {
    schema_version: u64,
    policy: String,
    owner: String,
    status: String,
    updated: String,
    #[serde(default)]
    due_review: Option<String>,
    #[serde(default)]
    requirements: ExceptionRequirements,
    #[serde(default, rename = "exception")]
    exceptions: Vec<QualityException>,
}

#[derive(Debug, Default, Deserialize)]
struct ExceptionRequirements {
    #[serde(default)]
    required_active: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QualityException {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    scope: String,
    owner: String,
    #[serde(default)]
    issue: Option<String>,
    reason: String,
    final_target: String,
    evidence: String,
    removal_criteria: String,
    created: String,
    review_after: String,
    expires: String,
}

enum JsonReceipt {
    Missing,
    Invalid,
    Present(Value),
}

fn read_exception_policy(path: &Path, today: NaiveDate) -> ExceptionPolicyEvaluation {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => {
            return ExceptionPolicyEvaluation {
                receipt: json!({
                    "status": "missing",
                    "policy": display_path(path),
                    "active_count": 0,
                    "final_enforcement_blocked": false,
                    "active": [],
                }),
                actions: vec![quality_exception_policy_action(
                    path,
                    "missing",
                    "quality exception policy ledger is missing",
                )],
            };
        }
    };

    let policy = match toml::from_str::<QualityExceptionPolicy>(&raw) {
        Ok(policy) => policy,
        Err(_) => {
            return ExceptionPolicyEvaluation {
                receipt: json!({
                    "status": "invalid",
                    "policy": display_path(path),
                    "active_count": 0,
                    "final_enforcement_blocked": false,
                    "active": [],
                }),
                actions: vec![quality_exception_policy_action(
                    path,
                    "invalid_toml",
                    "quality exception policy ledger is not valid TOML",
                )],
            };
        }
    };

    let due_review = policy.due_review.as_deref().unwrap_or("fail");
    let mut active = Vec::new();
    let mut active_ids = BTreeSet::new();
    let mut actions = Vec::new();

    if policy.schema_version != 1 || policy.policy != "quality-gate-exceptions" {
        actions.push(quality_exception_policy_action(
            path,
            "invalid_header",
            "quality exception policy must use schema_version = 1 and policy = \"quality-gate-exceptions\"",
        ));
    }
    if policy.owner.trim().is_empty()
        || policy.status != "active"
        || policy.updated.trim().is_empty()
    {
        actions.push(quality_exception_policy_action(
            path,
            "invalid_metadata",
            "quality exception policy must have owner, status = \"active\", and updated",
        ));
    }

    for exception in &policy.exceptions {
        let validation_errors = exception_validation_errors(exception);
        if !validation_errors.is_empty() {
            actions.push(quality_exception_invalid_action(path, exception, validation_errors));
            continue;
        }

        let review_after = parse_policy_date(&exception.review_after);
        let expires = parse_policy_date(&exception.expires);
        let created = parse_policy_date(&exception.created);
        if review_after.is_none() || expires.is_none() || created.is_none() {
            actions.push(quality_exception_invalid_action(
                path,
                exception,
                vec!["created, review_after, and expires must use YYYY-MM-DD".to_string()],
            ));
            continue;
        }

        let Some(expires) = expires else {
            continue;
        };
        if expires < today {
            actions.push(quality_exception_expired_action(path, exception, expires, today));
            continue;
        }

        active_ids.insert(exception.id.clone());
        active.push(quality_exception_receipt_entry(exception, review_after, expires));

        let Some(review_after) = review_after else {
            continue;
        };
        if review_after <= today {
            actions.push(quality_exception_review_due_action(
                path,
                exception,
                review_after,
                today,
                due_review,
            ));
        }
    }

    let missing_required = policy
        .requirements
        .required_active
        .iter()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_required.is_empty() {
        actions.push(quality_exception_required_missing_action(
            path,
            &missing_required,
            &policy.requirements.required_active,
        ));
    }

    let blocking =
        actions.iter().any(|action| action.get("blocking").and_then(Value::as_bool) == Some(true));
    let status = if blocking { "invalid" } else { "present" };

    ExceptionPolicyEvaluation {
        receipt: json!({
            "status": status,
            "policy": display_path(path),
            "due_review": due_review,
            "required_active": policy.requirements.required_active,
            "active_count": active.len(),
            "final_enforcement_blocked": !active.is_empty(),
            "active": active,
            "missing_required": missing_required,
        }),
        actions,
    }
}

fn exception_validation_errors(exception: &QualityException) -> Vec<String> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("id", exception.id.as_str()),
        ("kind", exception.kind.as_str()),
        ("scope", exception.scope.as_str()),
        ("owner", exception.owner.as_str()),
        ("reason", exception.reason.as_str()),
        ("final_target", exception.final_target.as_str()),
        ("evidence", exception.evidence.as_str()),
        ("removal_criteria", exception.removal_criteria.as_str()),
        ("created", exception.created.as_str()),
        ("review_after", exception.review_after.as_str()),
        ("expires", exception.expires.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{field} is required"));
        }
    }
    if !exception.kind.trim().is_empty() && exception.kind != "temporary_burndown" {
        errors.push("kind must be temporary_burndown".to_string());
    }
    errors
}

fn parse_policy_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn quality_exception_receipt_entry(
    exception: &QualityException,
    review_after: Option<NaiveDate>,
    expires: NaiveDate,
) -> Value {
    json!({
        "id": exception.id,
        "kind": exception.kind,
        "scope": exception.scope,
        "owner": exception.owner,
        "issue": exception.issue,
        "reason": exception.reason,
        "final_target": exception.final_target,
        "evidence": exception.evidence,
        "removal_criteria": exception.removal_criteria,
        "review_after": review_after.map(|date| date.to_string()).unwrap_or_else(|| exception.review_after.clone()),
        "expires": expires.to_string(),
        "blocks_final_enforcement": true,
    })
}

fn read_coverage_receipt(path: &Path, expected_head: &str) -> CoverageReceipt {
    let Ok(raw) = fs::read_to_string(path) else {
        return CoverageReceipt {
            status: "missing".to_string(),
            receipt_head: None,
            lcov: None,
            patch: None,
            project: None,
            scope: None,
            patch_files: Vec::new(),
            top_files: Vec::new(),
        };
    };
    let Ok(payload) = serde_json::from_str::<Value>(&raw) else {
        return CoverageReceipt {
            status: "invalid".to_string(),
            receipt_head: None,
            lcov: None,
            patch: None,
            project: None,
            scope: None,
            patch_files: Vec::new(),
            top_files: Vec::new(),
        };
    };

    let receipt_head = payload.get("head").and_then(Value::as_str).map(ToOwned::to_owned);
    let status = if receipt_head.as_deref() == Some(expected_head) { "present" } else { "stale" };
    let patch = payload.pointer("/coverage/patch").and_then(Value::as_f64);
    let project = payload.pointer("/coverage/project").and_then(Value::as_f64);
    let scope = payload.get("scope").and_then(Value::as_str).map(ToOwned::to_owned);
    let lcov = payload.get("lcov").and_then(Value::as_str).map(ToOwned::to_owned);
    let top_files = payload
        .get("files_below_target")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(actionable_file_gap).take(3).collect::<Vec<_>>())
        .unwrap_or_default();
    let patch_files = payload
        .get("patch_files_below_target")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(actionable_file_gap).take(3).collect::<Vec<_>>())
        .unwrap_or_default();

    CoverageReceipt {
        status: status.to_string(),
        receipt_head,
        lcov,
        patch,
        project,
        scope,
        patch_files,
        top_files,
    }
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
            let producer_status = payload.get("status").and_then(Value::as_str);
            let mut status = if receipt_head_sha.as_deref() != Some(expected_head) {
                "stale"
            } else if matches!(producer_status, Some("error" | "incomplete")) {
                producer_status.unwrap_or("incomplete")
            } else {
                "present"
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

fn read_codecov_project_status(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading Codecov config {}", path.display()))?;
    let parsed: YamlValue = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing Codecov config {}", path.display()))?;
    let target = yaml_path(&parsed, &["coverage", "status", "project", "default", "target"])
        .and_then(yaml_scalar);
    let threshold = yaml_path(&parsed, &["coverage", "status", "project", "default", "threshold"])
        .and_then(yaml_scalar);
    let informational =
        yaml_path(&parsed, &["coverage", "status", "project", "default", "informational"])
            .and_then(yaml_scalar);
    let threshold = threshold.as_deref().and_then(parse_percent);

    if target.as_deref() == Some("95%")
        && threshold.is_some_and(|value| value <= 0.25)
        && informational.as_deref() != Some("true")
    {
        Ok("present".to_string())
    } else {
        Ok("project_policy_not_blocking".to_string())
    }
}

fn parse_percent(value: &str) -> Option<f64> {
    value.trim().trim_end_matches('%').parse::<f64>().ok()
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
    let uses_changed_files = !coverage.patch_files.is_empty();
    let top_files = if uses_changed_files { &coverage.patch_files } else { &coverage.top_files };
    let path = coverage
        .patch_files
        .first()
        .or_else(|| coverage.top_files.first())
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
        "file_scope": if uses_changed_files { "changed_files" } else { "project_fallback" },
        "top_files": top_files,
        "suggested_test": "Prefer focused tests for error paths, boundary conditions, config parsing, serialization, cancellation, and output contracts.",
        "repair": "Add behavior-oriented tests for the uncovered changed-code surfaces, then refresh coverage evidence.",
        "verify": quality_gate_command(args, true, Some(patch)),
        "receipt": quality_gate_command(args, false, Some(patch)),
    })
}

fn project_coverage_unknown_action(args: &QualityGateArgs) -> Value {
    json!({
        "kind": "project_coverage_unknown",
        "blocking": true,
        "path": display_path(&args.coverage_receipt),
        "reason": "coverage receipt did not include coverage.project",
        "repair": "Regenerate the workspace coverage receipt with project coverage evidence before final enforcement.",
        "verify": coverage_baseline_command(args, true),
        "receipt": coverage_baseline_command(args, false),
    })
}

fn project_coverage_below_target_action(
    project: f64,
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
        "kind": "project_coverage_below_target",
        "blocking": true,
        "path": path,
        "current": round2(project),
        "target": PROJECT_TARGET,
        "top_files": coverage.top_files,
        "suggested_test": "Prioritize public API boundaries, error handling, config parsing, serialization, cancellation, provider decisions, and report generators.",
        "repair": "Burn down meaningful uncovered behavior until workspace project coverage reaches the final target, then refresh coverage evidence.",
        "verify": quality_gate_command(args, true, args.patch_coverage),
        "receipt": quality_gate_command(args, false, args.patch_coverage),
    })
}

fn coverage_scope_not_workspace_action(
    coverage: &CoverageReceipt,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "coverage_scope_not_workspace",
        "blocking": true,
        "path": display_path(&args.coverage_receipt),
        "reason": format!("coverage scope is {}", coverage.scope.as_deref().unwrap_or("unknown")),
        "repair": "Regenerate the coverage receipt from workspace-wide coverage, not a crate or partial path subset.",
        "verify": coverage_baseline_command(args, true),
        "receipt": coverage_baseline_command(args, false),
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

fn codecov_project_policy_action(status: &str, args: &QualityGateArgs) -> Value {
    json!({
        "kind": "codecov_project_policy_not_blocking",
        "blocking": true,
        "path": display_path(&args.codecov),
        "reason": status,
        "repair": "Promote Codecov project status to blocking at target 95% with threshold 0.25% or tighter before final enforcement.",
        "verify": quality_gate_command(args, true, args.patch_coverage),
        "receipt": quality_gate_command(args, false, args.patch_coverage),
    })
}

fn quality_exception_policy_action(path: &Path, reason: &str, repair: &str) -> Value {
    json!({
        "kind": "quality_exception_policy_not_current",
        "blocking": true,
        "path": display_path(path),
        "reason": reason,
        "repair": repair,
        "verify": quality_exception_policy_command(path, true),
        "receipt": quality_exception_policy_command(path, false),
    })
}

fn quality_exception_invalid_action(
    path: &Path,
    exception: &QualityException,
    errors: Vec<String>,
) -> Value {
    json!({
        "kind": "quality_exception_invalid",
        "blocking": true,
        "path": display_path(path),
        "id": exception.id,
        "reason": errors.join("; "),
        "repair": "Fill kind = \"temporary_burndown\", scope, owner, reason, final_target, evidence, removal_criteria, review_after, and expires for the temporary quality exception.",
        "verify": quality_exception_policy_command(path, true),
        "receipt": quality_exception_policy_command(path, false),
    })
}

fn quality_exception_expired_action(
    path: &Path,
    exception: &QualityException,
    expires: NaiveDate,
    today: NaiveDate,
) -> Value {
    json!({
        "kind": "quality_exception_expired",
        "blocking": true,
        "path": display_path(path),
        "id": exception.id,
        "reason": format!("expires {expires} is before {today}"),
        "repair": "Remove the temporary quality exception by completing its removal criteria, or replace it with a fresh policy PR that names new evidence and expiry.",
        "verify": quality_exception_policy_command(path, true),
        "receipt": quality_exception_policy_command(path, false),
    })
}

fn quality_exception_review_due_action(
    path: &Path,
    exception: &QualityException,
    review_after: NaiveDate,
    today: NaiveDate,
    due_review: &str,
) -> Value {
    let blocking = due_review != "warn";
    json!({
        "kind": "quality_exception_review_due",
        "blocking": blocking,
        "path": display_path(path),
        "id": exception.id,
        "reason": format!("review_after {review_after} is on or before {today}"),
        "repair": "Re-review the temporary quality exception, update current evidence, and either remove it or move review_after/expires in a policy PR.",
        "verify": quality_exception_policy_command(path, true),
        "receipt": quality_exception_policy_command(path, false),
    })
}

fn quality_exception_required_missing_action(
    path: &Path,
    missing: &[String],
    required: &[String],
) -> Value {
    json!({
        "kind": "quality_exception_required_missing",
        "blocking": true,
        "path": display_path(path),
        "reason": format!("missing required active temporary quality exception(s): {}", missing.join(", ")),
        "missing": missing,
        "required_active": required,
        "repair": "Document every transitional burn-down exception in policy/quality-gate-exceptions.toml, or remove it from required_active after the target has been met and enforcement is final.",
        "verify": quality_exception_policy_command(path, true),
        "receipt": quality_exception_policy_command(path, false),
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

fn ripr_total_unresolved_action(count: u64, args: &QualityGateArgs) -> Value {
    json!({
        "kind": "ripr_total_unresolved",
        "blocking": true,
        "path": display_path(&args.ripr_receipt),
        "unresolved": count,
        "reason": "repo-wide RIPR+ unresolved total is above zero",
        "repair": "Burn down the remaining repo-wide RIPR+ gap cluster with focused tests, then refresh the RIPR+ receipt.",
        "verify": ripr_plus_command(args, true),
        "receipt": ripr_plus_command(args, false),
    })
}

fn ripr_total_unknown_action(args: &QualityGateArgs) -> Value {
    json!({
        "kind": "ripr_total_unknown",
        "blocking": true,
        "path": display_path(&args.ripr_receipt),
        "reason": "repo-wide RIPR+ receipt did not include unresolved",
        "repair": "Regenerate the RIPR+ receipt so final enforcement can prove unresolved total = 0.",
        "verify": ripr_plus_command(args, true),
        "receipt": ripr_plus_command(args, false),
    })
}

fn quality_exception_active_final_blocker_action(
    exceptions: &ExceptionPolicyEvaluation,
    args: &QualityGateArgs,
) -> Value {
    json!({
        "kind": "quality_exception_active_final_blocker",
        "blocking": true,
        "path": display_path(&args.exception_policy),
        "reason": "active temporary quality exceptions remain",
        "active": exceptions.receipt.get("active").cloned().unwrap_or_else(|| json!([])),
        "repair": "Complete the burn-down removal criteria and remove active temporary quality exceptions before final enforcement can pass.",
        "verify": quality_gate_command(args, true, args.patch_coverage),
        "receipt": quality_gate_command(args, false, args.patch_coverage),
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
        let project = coverage
            .get("project")
            .and_then(Value::as_f64)
            .map(|value| format!("{value:.2}%"))
            .unwrap_or_else(|| "unknown".to_string());
        let source = coverage.get("patch_source").and_then(Value::as_str).unwrap_or("unknown");
        let status = coverage.get("status").and_then(Value::as_str).unwrap_or("unknown");
        let scope = coverage.get("scope").and_then(Value::as_str).unwrap_or("unknown");
        let codecov_patch = coverage
            .get("codecov_patch_status")
            .or_else(|| coverage.get("codecov_config_status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let codecov_project =
            coverage.get("codecov_project_status").and_then(Value::as_str).unwrap_or("unknown");
        markdown.push_str(&format!("- coverage receipt: `{status}`\n"));
        markdown.push_str(&format!("- patch coverage: `{patch}` / `95.00%`\n"));
        markdown.push_str(&format!("- project coverage: `{project}` / `95.00%`\n"));
        markdown.push_str(&format!("- coverage scope: `{scope}`\n"));
        markdown.push_str(&format!("- Codecov patch policy: `{codecov_patch}`\n"));
        if coverage.get("codecov_project_status").is_some() {
            markdown.push_str(&format!("- Codecov project policy: `{codecov_project}`\n"));
        }
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
    if let Some(review) = receipt.get("review_guidance") {
        let status = review.get("status").and_then(Value::as_str).unwrap_or("unknown");
        markdown.push_str(&format!("- review guidance receipt: `{status}`\n"));
    }
    if let Some(exceptions) = receipt.get("temporary_exceptions") {
        let status = exceptions.get("status").and_then(Value::as_str).unwrap_or("unknown");
        let active_count =
            exceptions.get("active_count").and_then(Value::as_u64).unwrap_or_default();
        let final_blocked =
            exceptions.get("final_enforcement_blocked").and_then(Value::as_bool).unwrap_or(false);
        markdown.push_str(&format!("- temporary exceptions: `{status}`\n"));
        markdown.push_str(&format!("- active temporary exceptions: `{active_count}`\n"));
        markdown.push_str(&format!("- final enforcement blocked: `{final_blocked}`\n"));
        if let Some(active) = exceptions.get("active").and_then(Value::as_array) {
            for exception in active {
                let id = exception.get("id").and_then(Value::as_str).unwrap_or("unknown");
                let target =
                    exception.get("final_target").and_then(Value::as_str).unwrap_or("unknown");
                markdown.push_str(&format!("- exception: `{id}` final target `{target}`\n"));
            }
        }
    }
    let freshness = receipt_freshness_summary(receipt);
    if !freshness.is_empty() {
        markdown.push_str(&format!("- receipt freshness: `{freshness}`\n"));
    }
    markdown.push('\n');

    markdown.push_str("## Proof Commands\n\n");
    markdown.push_str(&format!(
        "- verify: `{}`\n",
        quality_gate_command(args, true, args.patch_coverage)
    ));
    markdown.push_str(&format!(
        "- receipt: `{}`\n",
        quality_gate_command(args, false, args.patch_coverage)
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
            let coverage_file_label = match (kind, action.get("file_scope").and_then(Value::as_str))
            {
                ("patch_coverage_below_target", Some("changed_files")) => "changed coverage file",
                ("patch_coverage_below_target", Some("project_fallback")) => {
                    "project fallback coverage file"
                }
                _ => "coverage file",
            };
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
                markdown.push_str(&format!(
                    "- {coverage_file_label}: `{path}` sample uncovered lines: {samples}\n"
                ));
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
                    "- ripr gap: `{gap_id}` `{path}:{line}` seam `{seam}` reason `{reason}` suggested test `{suggested_test}`\n"
                ));
            }
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn receipt_freshness_summary(receipt: &Value) -> String {
    [
        ("coverage", "/coverage/status"),
        ("repo_ripr", "/ripr_plus/status"),
        ("diff_ripr", "/ripr_pr/status"),
        ("review_guidance", "/review_guidance/status"),
        ("exceptions", "/temporary_exceptions/status"),
    ]
    .iter()
    .filter_map(|(label, pointer)| {
        receipt.pointer(pointer).and_then(Value::as_str).map(|status| format!("{label}={status}"))
    })
    .collect::<Vec<_>>()
    .join(", ")
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
    command.push_str(&format!(" --exception-policy {}", args.exception_policy.display()));
    match args.mode {
        QualityGateMode::Enforce => {
            command.push_str(&format!(
                " --coverage-receipt {} --codecov {} --ripr-receipt {} --ripr-pr-receipt {} --review-receipt {} --ripr-base {} --ripr-head {}",
                args.coverage_receipt.display(),
                args.codecov.display(),
                args.ripr_receipt.display(),
                args.ripr_pr_receipt.display(),
                args.review_receipt.display(),
                args.ripr_base,
                args.ripr_head
            ));
        }
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

fn quality_exception_policy_command(path: &Path, check: bool) -> String {
    let mut command = format!(
        "rtk cargo xtask quality-gate --mode enforce-patch-coverage --exception-policy {} --coverage-receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml --receipt target/receipts/quality/quality-gate.json --summary target/receipts/quality/quality-gate.md",
        path.display()
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
