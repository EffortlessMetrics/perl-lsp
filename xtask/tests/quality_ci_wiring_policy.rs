//! Contract tests for first blocking proof-lane CI wiring.

use std::{fs, path::PathBuf};

use assert_cmd::Command;
use perl_tdd_support::{must, must_some};

#[test]
fn ripr_workflow_blocks_new_gaps_and_requires_receipts() {
    let root = repo_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/ripr.yml")));

    assert!(workflow.contains("name: ripr+ New Gap Gate"), "RIPR check name must be explicit");
    let gate_step = must_some(workflow_step(&workflow, "Enforce new RIPR gap quality gate"));
    assert!(
        !gate_step.contains("continue-on-error: true"),
        "RIPR new-gap gate must be blocking after PR8"
    );
    for required in [
        "- name: Enforce new RIPR gap quality gate",
        "cargo xtask quality-gate",
        "--mode enforce-new-ripr",
        "--ripr-receipt target/receipts/quality/ripr-plus.json",
        "--ripr-pr-receipt target/ripr/pr/repo-exposure.json",
        "--review-receipt target/ripr/review/comments.json",
        "--receipt target/receipts/quality/quality-gate-ripr.json",
        "--summary target/receipts/quality/quality-gate-ripr.md",
        "--check",
        "target/ripr/pr/**",
        "target/ripr/review/**",
        "target/receipts/quality/ripr-plus.json",
        "target/receipts/quality/quality-gate-ripr.json",
        "target/receipts/quality/quality-gate-ripr.md",
        "if-no-files-found: error",
    ] {
        assert!(workflow.contains(required), "RIPR workflow missing `{required}`");
    }
    assert!(
        workflow.contains("target/receipts/quality/quality-gate-ripr.md")
            && workflow.contains("GITHUB_STEP_SUMMARY"),
        "RIPR workflow must append quality-gate Markdown to the GitHub summary"
    );
}

#[test]
fn coverage_workflow_blocks_patch_coverage_and_requires_receipts() {
    let root = repo_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/ci-nightly.yml")));
    let justfile = must(fs::read_to_string(root.join("justfile")));
    let coverage_start = must_some(workflow.find("  test-coverage:"));
    let coverage_tail = &workflow[coverage_start..];
    let coverage_end = must_some(coverage_tail.find("\n  tautology-check:"));
    let coverage_job = &coverage_tail[..coverage_end];

    assert!(
        workflow.contains("types: [opened, synchronize, reopened, ready_for_review, labeled]"),
        "coverage workflow must rerun when a draft PR becomes ready"
    );
    assert!(
        coverage_job.contains("name: Codecov / Patch 95"),
        "coverage job must have a branch-protection-ready check name"
    );
    assert!(
        coverage_job.contains("uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd")
            && coverage_job.contains("persist-credentials: false"),
        "coverage proof checkout must be pinned and must not persist write credentials"
    );
    assert!(
        coverage_job.contains("github.event.pull_request.draft != true")
            && coverage_job.contains("github.event.action != 'labeled'")
            && !coverage_job.contains("ci:coverage"),
        "patch coverage must be a front-door PR gate, skip label-only churn, and not be label-gated"
    );
    let codecov_upload_start = must_some(coverage_job.find("- name: Upload coverage to Codecov"));
    let after_codecov_upload = &coverage_job[codecov_upload_start..];
    let codecov_upload_end =
        must_some(after_codecov_upload.find("\n      - name: Upload coverage proof artifacts"));
    let codecov_upload_step = &after_codecov_upload[..codecov_upload_end];
    assert_eq!(
        codecov_upload_step.matches("fail_ci_if_error: true").count(),
        1,
        "Codecov upload must fail the job when coverage upload/status integration errors"
    );
    assert!(
        codecov_upload_step.contains("uses: codecov/codecov-action@")
            && codecov_upload_step.contains("files: target/lcov.info"),
        "Codecov upload step must upload the workspace library plus xtask proof-lane LCOV receipt"
    );
    for required in [
        "BASE_REF: ${{ github.base_ref || github.event.repository.default_branch }}",
        "base_ref=\"$BASE_REF\"",
        "just coverage-proof \"origin/$base_ref\"",
        "cache-targets: false",
        "RUSTFLAGS: \"-Cdebuginfo=0\"",
        "CARGO_BUILD_JOBS: 1",
        "- name: Emit changed-file coverage route summary",
        "cargo xtask ci route",
        "--receipt target/receipts/quality/ci-route.json",
        "--summary target/receipts/quality/ci-route.md",
        "target/receipts/quality/ci-route.md",
        "target/receipts/quality/quality-gate-coverage.md",
        "GITHUB_STEP_SUMMARY",
        "name: coverage-proof-${{ github.sha }}",
        "target/lcov.info",
        "target/receipts/quality/ci-route.json",
        "target/receipts/quality/ci-route.md",
        "target/receipts/quality/coverage-baseline.json",
        "target/receipts/quality/quality-gate-coverage.json",
        "target/receipts/quality/quality-gate-coverage.md",
        "if-no-files-found: error",
    ] {
        assert!(coverage_job.contains(required), "coverage job missing `{required}`");
    }
    assert!(
        coverage_job
            .contains("uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"),
        "coverage proof artifact upload must pin the action SHA"
    );
    assert!(
        !coverage_job.contains("continue-on-error: true"),
        "coverage patch proof must not make Codecov upload advisory"
    );
    for required in [
        "coverage-proof base='origin/main':",
        "coverage_target=\"${CARGO_TARGET_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/perl-lsp-swarm-coverage-target}\"",
        "export CARGO_TARGET_DIR=\"$coverage_target\"",
        "cargo llvm-cov clean --workspace",
        "coverage_env=\"$coverage_target/llvm-cov-env.sh\"",
        "cargo llvm-cov show-env --sh",
        "source \"$coverage_env\"",
        "cargo test --workspace --lib --locked",
        "cargo test -p xtask --bin xtask quality_baseline --locked",
        "cargo test -p xtask --bin xtask merge_ready --locked",
        "cargo test -p xtask --bin xtask queue_reconciler --locked",
        "cargo test -p xtask --bin xtask ci_route --locked",
        "cargo test -p xtask --bin xtask ripr --locked",
        "cargo test -p xtask --bin xtask inline_completion_quality --locked",
        "cargo test -p xtask --bin xtask semantic_inline_receipts --locked",
        "cargo test -p xtask --bin xtask semantic_inline_next_edit --locked",
        "cargo test -p xtask --locked",
        "--test ci_route_cli",
        "--test quality_ci_wiring_policy",
        "--test quality_gate_patch_coverage_cli_policy",
        "--test semantic_inline_receipts_cli",
        "--test semantic_inline_next_edit_cli",
        "cargo llvm-cov report --lcov --output-path target/lcov.info",
        "--lcov --output-path target/lcov.info",
        "cargo xtask coverage-baseline",
        "--patch-base \"{{base}}\"",
        "--scope workspace-lib-xtask-quality",
        "cargo xtask quality-gate",
        "--mode enforce-patch-coverage",
        "--receipt target/receipts/quality/quality-gate-coverage.json",
        "--summary target/receipts/quality/quality-gate-coverage.md",
        "--check",
    ] {
        assert!(justfile.contains(required), "coverage-proof missing `{required}`");
    }
}

#[test]
fn coverage_proof_exercises_lsp_318_claim_guard() {
    let root = repo_root();

    must(Command::cargo_bin("xtask"))
        .current_dir(root)
        .arg("check-lsp-318-claims")
        .assert()
        .success();
}

#[test]
fn docs_describe_transitional_blocking_contract() {
    let root = repo_root();
    let ripr_doc = must(fs::read_to_string(root.join("docs/ci/ripr.md")));
    let coverage_doc = must(fs::read_to_string(root.join("docs/how-to/COVERAGE.md")));
    let status_doc =
        must(fs::read_to_string(root.join("docs/project/status/coverage_and_ripr_enforcement.md")));

    assert!(
        ripr_doc.contains("blocks PRs that introduce")
            && ripr_doc.contains("not require repo-wide RIPR+ total zero"),
        "RIPR docs must distinguish new-gap blocking from final total-zero enforcement"
    );
    assert!(
        coverage_doc.contains("coverage proof workflow now runs the patch coverage quality gate")
            && coverage_doc.contains("just coverage-proof <base>")
            && coverage_doc.contains("Project coverage remains informational during burn-down"),
        "coverage docs must describe PR8 patch enforcement and transitional project coverage"
    );
    assert!(
        status_doc.contains("quality-gate")
            && status_doc.contains("Markdown summaries")
            && status_doc.contains("Current Blocking Proof Floor")
            && status_doc.contains("Codecov upload or")
            && status_doc.contains("processing failures through `fail_ci_if_error: true`")
            && status_doc.contains(
                "generated quality-gate receipts are freshness-checked for patch, new-RIPR,"
            )
            && status_doc.contains("project coverage is visible in coverage receipts")
            && status_doc.contains("total active RIPR+ unresolved gaps")
            && status_doc.contains("Active RIPR+ Inventory")
            && status_doc.contains("active_unresolved")
            && status_doc.contains("suppressed_unresolved")
            && status_doc.contains("top_active_gap_kinds")
            && status_doc.contains("recommended_first_clusters")
            && status_doc.contains("Project Coverage Burn-Down Inventory")
            && status_doc.contains("project_burndown.remaining_percentage_points")
            && status_doc.contains("project_files_below_target")
            && status_doc.contains("top_project_files")
            && status_doc.contains("recommended_project_clusters")
            && status_doc.contains("An active temporary exception is not a final-enforcement pass"),
        "status doc must keep final targets separate from transitional enforcement"
    );
}

#[test]
fn conventional_required_checks_record_live_proof_floor() {
    let root = repo_root();
    let policy = must(fs::read_to_string(root.join(".ci/policies/required-checks.toml")));
    let merge_ready_doc = must(fs::read_to_string(root.join("docs/ci/merge-ready-protocol.md")));
    let status_doc =
        must(fs::read_to_string(root.join("docs/project/status/coverage_and_ripr_enforcement.md")));
    let parsed: toml::Value = must(toml::from_str(&policy));

    for required in
        ["Perl LSP Rust Small Result", "ripr+ New Gap Gate", "Codecov / Patch 95", "codecov/patch"]
    {
        assert!(
            policy_required_check(&parsed, required),
            "required-check policy must mark `{required}` as required under GitHub enforcement"
        );
        assert!(
            merge_ready_doc.contains(required) && status_doc.contains(required),
            "docs must name live required proof context `{required}`"
        );
    }
}

fn policy_required_check(policy: &toml::Value, name: &str) -> bool {
    policy.get("checks").and_then(toml::Value::as_array).into_iter().flatten().any(|item| {
        item.get("name").and_then(toml::Value::as_str) == Some(name)
            && item.get("required").and_then(toml::Value::as_bool) == Some(true)
            && item.get("enforcement").and_then(toml::Value::as_str)
                == Some("github-branch-protection")
    })
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    must_some(manifest.parent()).to_path_buf()
}

fn workflow_step<'a>(content: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("- name: {name}");
    let start = content.find(&needle)?;
    let rest = &content[start..];
    let next = rest
        .lines()
        .skip(1)
        .scan(needle.len() + 1, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find_map(
            |(offset, line)| {
                if line.trim_start().starts_with("- name:") { Some(offset) } else { None }
            },
        )
        .unwrap_or(rest.len());
    Some(&rest[..next])
}
