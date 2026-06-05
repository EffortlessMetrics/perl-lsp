//! Contract tests for actionable quality-gate PR summaries.

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::tempdir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn quality_gate_summary_names_gates_receipts_and_repair_packets() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 94.9, 94.2)?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 3)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 2)?;
    write_review_guidance_receipt(&paths.review, &head)?;
    write_active_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "fixture should fail so the summary includes repairs");

    let receipt: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let patch_action = receipt
        .pointer("/next_actions/0")
        .ok_or("quality gate receipt must include patch coverage repair action first")?;
    assert_eq!(
        patch_action.get("kind").and_then(Value::as_str),
        Some("patch_coverage_below_target")
    );
    assert_eq!(patch_action.get("file_scope").and_then(Value::as_str), Some("changed_files"));
    assert_eq!(
        patch_action.pointer("/top_files/0/path").and_then(Value::as_str),
        Some("xtask/src/tasks/ripr_evidence.rs")
    );
    assert_eq!(
        patch_action.pointer("/top_files/0/sample_uncovered_lines/0").and_then(Value::as_u64),
        Some(212)
    );
    let project_action = receipt
        .pointer("/next_actions/1")
        .ok_or("quality gate receipt missing project coverage action")?;
    assert_eq!(
        project_action.get("kind").and_then(Value::as_str),
        Some("project_coverage_below_target")
    );
    assert_eq!(
        project_action.pointer("/recommended_project_clusters/0/name").and_then(Value::as_str),
        Some("proof-infrastructure")
    );
    let ripr_action = receipt
        .pointer("/next_actions/2")
        .ok_or("quality gate receipt missing RIPR total action")?;
    assert_eq!(ripr_action.get("kind").and_then(Value::as_str), Some("ripr_total_unresolved"));
    assert_eq!(
        ripr_action.pointer("/recommended_first_clusters/0/name").and_then(Value::as_str),
        Some("ci-report-formatting")
    );

    let summary = fs::read_to_string(&paths.summary)?;
    for required in [
        "## Quality Gates",
        "- new RIPR gaps: `2`",
        "- total RIPR+ gaps: `3`",
        "- patch coverage: `94.90%` / `95.00%`",
        "- project coverage: `94.20%` / `95.00%`",
        "- coverage receipt: `present`",
        "- diff RIPR receipt: `present`",
        "- repo RIPR+ receipt: `present`",
        "- review guidance receipt: `present`",
        "- receipt freshness: `coverage=present, repo_ripr=present, diff_ripr=present, review_guidance=present, exceptions=present`",
        "- active temporary exceptions: `1`",
        "## Proof Commands",
        "- verify: `rtk cargo xtask quality-gate --mode enforce",
        "- receipt: `rtk cargo xtask quality-gate --mode enforce",
        "ripr_total_unresolved",
        "project_coverage_below_target",
        "new_ripr_gap",
        "quality_exception_active_final_blocker",
        "coverage cluster: `proof-infrastructure` (file_count: 2, uncovered_line_count: 37) reason `Coverage proof, quality-gate, workflow, and policy surfaces are owned by this lane.`",
        "coverage cluster example file: `xtask/src/tasks/quality_baseline.rs`",
        "ripr cluster: `ci-report-formatting` (score: 5, active_file_count: 3, gap_kind_count: 2) reason `Receipt and report formatting gaps should become agent repair packets.`",
        "ripr cluster example gap kind: `receipt_missing`",
        "changed coverage file: `xtask/src/tasks/ripr_evidence.rs` sample uncovered lines: 212, 213, 214",
        "coverage file: `xtask/src/tasks/quality_gate.rs` sample uncovered lines: 41, 42, 43",
        "ripr gap: `RIPR-SPEC-ONE` `xtask/src/tasks/quality_gate.rs:77` seam `summary_renderer` reason `summary does not expose repair packet` suggested test `assert repair packet markdown`",
        "ripr gap: `RIPR-SPEC-TWO` `xtask/src/tasks/quality_gate.rs:88` seam `receipt_freshness` reason `summary does not expose stale receipt` suggested test `assert receipt freshness markdown`",
    ] {
        assert!(summary.contains(required), "quality summary missing `{required}`:\n{summary}");
    }

    Ok(())
}

#[test]
fn pull_request_template_has_quality_gate_repair_packet_fields() -> TestResult {
    let root = repo_root()?;
    let template = fs::read_to_string(root.join(".github/PULL_REQUEST_TEMPLATE.md"))?;

    for required in [
        "## Objective",
        "## Quality Gates",
        "target/receipts/quality/quality-gate.md",
        "## Claim Boundary",
        "## Non-goals",
        "## Local Proof Commands",
        "new RIPR gaps:",
        "total RIPR+ gaps:",
        "patch coverage:",
        "project coverage:",
        "receipt freshness:",
        "exception status:",
        "local verify command:",
        "receipt command:",
        "## RIPR / Coverage Effect",
        "## Cleanup Performed",
        "## Remaining Work",
    ] {
        assert!(template.contains(required), "PR template missing `{required}`");
    }

    Ok(())
}

struct FixturePaths {
    coverage: PathBuf,
    codecov: PathBuf,
    ripr: PathBuf,
    ripr_pr: PathBuf,
    review: PathBuf,
    exceptions: PathBuf,
    receipt: PathBuf,
    summary: PathBuf,
}

impl FixturePaths {
    fn new(root: &Path) -> Self {
        Self {
            coverage: root.join("coverage-baseline.json"),
            codecov: root.join("codecov.yml"),
            ripr: root.join("ripr-plus.json"),
            ripr_pr: root.join("repo-exposure.json"),
            review: root.join("comments.json"),
            exceptions: root.join("quality-gate-exceptions.toml"),
            receipt: root.join("quality-gate.json"),
            summary: root.join("quality-gate.md"),
        }
    }
}

fn final_quality_gate_command(root: &Path, paths: &FixturePaths) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce"]);
    command.arg("--coverage-receipt").arg(&paths.coverage);
    command.arg("--codecov").arg(&paths.codecov);
    command.arg("--ripr-receipt").arg(&paths.ripr);
    command.arg("--ripr-pr-receipt").arg(&paths.ripr_pr);
    command.arg("--review-receipt").arg(&paths.review);
    command.arg("--exception-policy").arg(&paths.exceptions);
    command.arg("--receipt").arg(&paths.receipt);
    command.arg("--summary").arg(&paths.summary);
    Ok(command)
}

fn write_coverage_receipt(path: &Path, head: &str, patch: f64, project: f64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "scope": "workspace",
            "lcov": "target/lcov.info",
            "coverage": {
                "patch": patch,
                "project": project
            },
            "files_below_target": [
                {
                    "path": "xtask/src/tasks/quality_gate.rs",
                    "line_coverage": 72.0,
                    "sample_uncovered_lines": [41, 42, 43]
                }
            ],
            "patch_files_below_target": [
                {
                    "path": "xtask/src/tasks/ripr_evidence.rs",
                    "line_coverage": 67.9,
                    "sample_uncovered_lines": [212, 213, 214]
                }
            ],
            "recommended_project_clusters": [
                {
                    "name": "proof-infrastructure",
                    "file_count": 2,
                    "uncovered_line_count": 37,
                    "reason": "Coverage proof, quality-gate, workflow, and policy surfaces are owned by this lane.",
                    "example_files": [
                        "xtask/src/tasks/quality_baseline.rs",
                        "xtask/src/tasks/quality_gate.rs"
                    ]
                }
            ]
        }),
    )
}

fn write_final_codecov(path: &Path) -> TestResult {
    write_text(
        path,
        r#"coverage:
  status:
    project:
      default:
        target: 95%
        threshold: 0.25%
    patch:
      default:
        target: 95%
        threshold: 0%
"#,
    )
}

fn write_ripr_plus_receipt(path: &Path, head: &str, unresolved: u64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "ripr_plus_baseline",
            "head": head,
            "unresolved": unresolved,
            "recommended_first_clusters": [
                {
                    "name": "ci-report-formatting",
                    "score": 5,
                    "active_file_count": 3,
                    "gap_kind_count": 2,
                    "reason": "Receipt and report formatting gaps should become agent repair packets.",
                    "example_files": ["xtask/src/tasks/quality_gate.rs"],
                    "example_gap_kinds": ["receipt_missing"]
                }
            ]
        }),
    )
}

fn write_ripr_pr_receipt(path: &Path, head: &str, severe_gaps: u64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "kind": "pr_evidence",
            "scope": "diff",
            "base": "quality-summary-test-base",
            "base_sha": "quality-summary-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "severe_gaps": severe_gaps
            }
        }),
    )
}

fn write_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-summary-test-base",
            "base_sha": "quality-summary-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 2,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [
                {
                    "canonical_gap_id": "RIPR-SPEC-ONE",
                    "reason": "summary does not expose repair packet",
                    "placement": {
                        "path": "xtask/src/tasks/quality_gate.rs",
                        "line": 77,
                        "mode": "summary_renderer"
                    },
                    "suggested_test": {
                        "intent": "assert repair packet markdown"
                    }
                },
                {
                    "canonical_gap_id": "RIPR-SPEC-TWO",
                    "reason": "summary does not expose stale receipt",
                    "placement": {
                        "path": "xtask/src/tasks/quality_gate.rs",
                        "line": 88,
                        "mode": "receipt_freshness"
                    },
                    "suggested_test": {
                        "intent": "assert receipt freshness markdown"
                    }
                }
            ],
            "summary_only": [],
            "suppressed": []
        }),
    )
}

fn write_active_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = ["ripr-total-burndown"]

[[exception]]
id = "ripr-total-burndown"
kind = "temporary_burndown"
scope = "ripr_plus_total"
owner = "proof-lane"
issue = "#8197"
reason = "transition burn-down remains active"
final_target = "repo-wide ripr+ unresolved total = 0"
evidence = "target/receipts/quality/ripr-plus.json"
removal_criteria = "remove when RIPR+ total is zero"
created = "2026-05-28"
review_after = "2099-01-01"
expires = "2099-12-31"
"##,
    )
}

fn repo_root() -> TestResult<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest must be nested under repo root".into())
}

fn current_head(root: &Path) -> TestResult<String> {
    let output = StdCommand::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git rev-parse HEAD failed with status {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn write_text(path: &Path, value: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value)?;
    Ok(())
}

fn write_json(path: &Path, value: Value) -> TestResult {
    write_text(path, &serde_json::to_string_pretty(&value)?)
}
