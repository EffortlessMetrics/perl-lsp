use color_eyre::eyre::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CiRouteArgs {
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub summary: PathBuf,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct CiRouteReceipt {
    schema_version: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    base: String,
    head: String,
    changed_files: Vec<String>,
    changed_surfaces: Vec<String>,
    required_proof_packs: Vec<ProofPackReceipt>,
    skipped_by_policy: BTreeMap<String, String>,
    coverage_pack_selector: Vec<String>,
    coverage_proof_packs: Vec<CoverageProofPackReceipt>,
    estimated_lem: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProofPackReceipt {
    id: String,
    commands: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProofPack {
    id: &'static str,
    commands: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct CoverageProofPackReceipt {
    id: String,
    files: Vec<String>,
    commands: Vec<String>,
    coverage_filters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoveragePackManifest {
    pack: Vec<CoveragePack>,
}

#[derive(Debug, Clone, Deserialize)]
struct CoveragePack {
    id: String,
    files: Vec<String>,
    commands: Vec<String>,
    coverage_filters: Vec<String>,
    #[serde(default = "default_lcov")]
    lcov: bool,
}

const COVERAGE_PACKS_TOML: &str = include_str!("../../../.ci/coverage-packs.toml");
const NON_LCOV_COVERAGE_SKIP_REASON: &str =
    "non-LCOV CI policy/routing surface; covered by focused CI gates";
const NON_SOURCE_LCOV_COVERAGE_SKIP_REASON: &str =
    "LCOV coverage pack matched only non-source files; covered by focused CI gates";

const PREFLIGHT_PACK: ProofPack = ProofPack {
    id: "preflight",
    commands: &[
        "cargo xtask pr title-check --no-gh",
        "cargo fmt -p xtask -- --check",
        "git diff --check",
    ],
};

const DOCS_PACK: ProofPack = ProofPack {
    id: "docs-focused",
    commands: &["cargo xtask check-devex-docs", "cargo xtask doc-claims"],
};

const XTASK_SEMANTIC_INLINE_PACK: ProofPack = ProofPack {
    id: "xtask-semantic-inline-receipts",
    commands: &[
        "cargo test -p xtask --bin xtask --profile agent --locked semantic_inline_receipts -- --nocapture",
        "cargo test -p xtask --bin xtask --profile agent --locked semantic_inline_next_edit -- --nocapture",
        "cargo test -p xtask --test semantic_inline_receipts_cli --profile agent --locked -- --nocapture",
        "cargo test -p xtask --test semantic_inline_next_edit_cli --profile agent --locked -- --nocapture",
    ],
};

const XTASK_SUPPORTED_EDITOR_INLINE_PACK: ProofPack = ProofPack {
    id: "xtask-supported-editor-inline-smoke",
    commands: &[
        "cargo test -p xtask --bin xtask --profile agent --locked supported_editor_inline_smoke -- --nocapture",
        "cargo test -p xtask --test supported_editor_inline_smoke_cli --profile agent --locked -- --nocapture",
        "cargo test -p xtask --bin xtask --profile agent --locked semantic_inline_receipts -- --nocapture",
    ],
};

const INLINE_CORE_PACK: ProofPack = ProofPack {
    id: "inline-core",
    commands: &[
        "cargo test -p perl-lsp-rs-core --lib --profile agent --locked inline_completion -- --nocapture",
    ],
};

const INLINE_UX_FIXTURES_PACK: ProofPack = ProofPack {
    id: "inline-ux-fixtures",
    commands: &[
        "cargo test -p perl-lsp-rs-core --test inline_completion_ux_fixtures --profile agent --locked -- --nocapture",
    ],
};

const XTASK_INLINE_COMPLETION_QUALITY_PACK: ProofPack = ProofPack {
    id: "xtask-inline-completion-quality",
    commands: &[
        "cargo run -p xtask --profile agent --locked -- inline-completion-quality --receipt target/receipts/inline-completion-quality.json",
    ],
};

const COMPLETION_CORE_PACK: ProofPack = ProofPack {
    id: "completion-core",
    commands: &[
        "cargo test -p perl-lsp-rs-core --lib --profile agent --locked completion::completion -- --nocapture",
    ],
};

const UX_SCENARIO_PACK: ProofPack = ProofPack {
    id: "ux-scenario-focused",
    commands: &[
        "cargo test -p perl-lsp-ux-tests --profile agent --locked -- --nocapture",
        "python -m json.tool crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json",
    ],
};

const CI_POLICY_PACK: ProofPack = ProofPack {
    id: "ci-policy-focused",
    commands: &[
        "python -m unittest scripts/ci/test_ci_classify.py",
        "cargo xtask workflow-trigger-lint --policy .ci/policies/required-checks.toml --receipt target/receipts/workflow-trigger-lint.json",
        "cargo test -p xtask --test quality_ci_wiring_policy --profile agent --locked -- --nocapture",
    ],
};

const CI_ROUTE_PACK: ProofPack = ProofPack {
    id: "ci-route-receipt",
    commands: &[
        "python -m unittest scripts/ci/test_route_codecov_packs.py",
        "cargo test -p xtask --bin xtask --profile agent --locked ci_route -- --nocapture",
        "cargo test -p xtask --test ci_route_cli --profile agent --locked -- --nocapture",
        "cargo run -p xtask --profile agent --locked -- ci route --base origin/main --head HEAD --receipt target/receipts/ci-route.json",
    ],
};

const CI_ACTUALS_PACK: ProofPack = ProofPack {
    id: "ci-actuals-focused",
    commands: &["python -m unittest scripts/ci/test_emit_ci_actuals.py"],
};

const RIPR_SUMMARY_PACK: ProofPack = ProofPack {
    id: "ripr-summary-focused",
    commands: &["python -m unittest scripts/ci/test_ripr_summary.py"],
};

const LEARNED_ESTIMATE_PACK: ProofPack = ProofPack {
    id: "learned-estimate-focused",
    commands: &["python -m unittest scripts/ci/test_learned_estimate.py"],
};

const RISK_PACKS_VALIDATOR_PACK: ProofPack = ProofPack {
    id: "risk-packs-validator-focused",
    commands: &["python -m unittest scripts/ci/test_validate_risk_packs.py"],
};

const GATE_LANE_MAPPING_PACK: ProofPack = ProofPack {
    id: "gate-lane-mapping-focused",
    commands: &["python -m unittest scripts/ci/test_validate_gate_lane_mapping.py"],
};

const TRUST_LANES_VALIDATOR_PACK: ProofPack = ProofPack {
    id: "trust-lanes-validator-focused",
    commands: &["python -m unittest scripts/ci/test_validate_trust_lanes.py"],
};

const RECEIPTS_JUNIT_PACK: ProofPack = ProofPack {
    id: "receipts-junit-focused",
    commands: &["python -m unittest scripts/ci/test_receipts_to_junit.py"],
};

const CORE_PACKAGE_VALIDATOR_PACK: ProofPack = ProofPack {
    id: "core-package-validator-focused",
    commands: &["python -m unittest scripts/ci/test_check_perl_lsp_rs_core_package.py"],
};

const AGGREGATE_LANE_HISTORY_PACK: ProofPack = ProofPack {
    id: "aggregate-lane-history-focused",
    commands: &["python -m unittest scripts/ci/test_aggregate_lane_history.py"],
};

const PR_PLAN_PACK: ProofPack = ProofPack {
    id: "pr-plan-focused",
    commands: &["python -m unittest scripts/ci/test_pr_plan.py"],
};

const PR_OVERLAP_PACK: ProofPack =
    ProofPack { id: "pr-overlap-focused", commands: &["python scripts/tests/test_pr_overlap.py"] };

const CONTROL_PLANE_LOCK_PACK: ProofPack = ProofPack {
    id: "control-plane-lock-focused",
    commands: &["bash scripts/test-control-plane-lock.sh"],
};

const AGENT_PREFLIGHT_PACK: ProofPack = ProofPack {
    id: "agent-preflight-focused",
    commands: &["bash scripts/test-agent-preflight.sh"],
};

const CI_AUDIT_WORKFLOWS_SHIM_PACK: ProofPack = ProofPack {
    id: "ci-audit-workflows-shim-focused",
    commands: &["python scripts/tests/test-ci-audit-workflows-shim.py"],
};

const DOC_CLAIMS_SHIM_PACK: ProofPack = ProofPack {
    id: "doc-claims-shim-focused",
    commands: &["python scripts/tests/test-check-doc-claims-shim.py"],
};

const FEATURES_INVARIANTS_SHIM_PACK: ProofPack = ProofPack {
    id: "features-invariants-shim-focused",
    commands: &["python scripts/tests/test-check-features-invariants-shim.py"],
};

const DEBT_REPORT_SHIM_PACK: ProofPack = ProofPack {
    id: "debt-report-shim-focused",
    commands: &["python scripts/tests/test-debt-report-shim.py"],
};

const PREFLIGHT_WRAPPER_PACK: ProofPack = ProofPack {
    id: "preflight-wrapper-focused",
    commands: &["bash scripts/tests/test-preflight-wrapper.sh"],
};

const INSTALL_GITHOOKS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "install-githooks-wrapper-focused",
    commands: &["bash scripts/tests/test-install-githooks-wrapper.sh"],
};

const E2E_GATE_WRAPPER_PACK: ProofPack = ProofPack {
    id: "e2e-gate-wrapper-focused",
    commands: &["bash scripts/tests/test-e2e-gate-wrapper.sh"],
};

const EXECUTE_GATE_WRAPPER_PACK: ProofPack = ProofPack {
    id: "execute-gate-wrapper-focused",
    commands: &["bash scripts/tests/test-execute-gate-wrapper.sh"],
};

const RUN_GATES_WRAPPER_PACK: ProofPack = ProofPack {
    id: "run-gates-wrapper-focused",
    commands: &["bash scripts/tests/test-run-gates-wrapper.sh"],
};

const GATE_LOCAL_WRAPPER_PACK: ProofPack = ProofPack {
    id: "gate-local-wrapper-focused",
    commands: &["bash scripts/tests/test-gate-local-wrapper.sh"],
};

const LIST_GATES_WRAPPER_PACK: ProofPack = ProofPack {
    id: "list-gates-wrapper-focused",
    commands: &["python scripts/tests/test-list-gates-wrapper.py"],
};

const FORBID_FATAL_CONSTRUCTS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "forbid-fatal-constructs-wrapper-focused",
    commands: &["bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh"],
};

const DEAD_CODE_WRAPPER_PACK: ProofPack = ProofPack {
    id: "dead-code-wrapper-focused",
    commands: &["bash scripts/tests/test-dead-code-wrapper.sh"],
};

const CHECK_TOOLCHAIN_WRAPPER_PACK: ProofPack = ProofPack {
    id: "check-toolchain-wrapper-focused",
    commands: &["bash scripts/tests/test-check-rust-toolchain-wrapper.sh"],
};

const DEVEX_DOCTOR_WRAPPER_PACK: ProofPack = ProofPack {
    id: "devex-doctor-wrapper-focused",
    commands: &["bash scripts/tests/test-devex-doctor-wrapper.sh"],
};

const DEVEX_TARGETED_CHECKS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "devex-targeted-checks-wrapper-focused",
    commands: &["bash scripts/tests/test-devex-targeted-checks-wrapper.sh"],
};

const LSP_CANCELLATION_WRAPPER_PACK: ProofPack = ProofPack {
    id: "lsp-cancellation-wrapper-focused",
    commands: &["bash scripts/tests/test-lsp-cancellation-wrapper.sh"],
};

const TEST_CAPPED_WRAPPER_PACK: ProofPack = ProofPack {
    id: "test-capped-wrapper-focused",
    commands: &["bash scripts/tests/test-test-capped-wrapper.sh"],
};

const TEST_E2E_CAPPED_WRAPPER_PACK: ProofPack = ProofPack {
    id: "test-e2e-capped-wrapper-focused",
    commands: &["bash scripts/tests/test-test-e2e-capped-wrapper.sh"],
};

const CI_COST_MONITOR_WRAPPER_PACK: ProofPack = ProofPack {
    id: "ci-cost-monitor-wrapper-focused",
    commands: &["bash scripts/tests/test-ci-cost-monitor-wrapper.sh"],
};

const BUILD_TIMING_RECEIPT_WRAPPER_PACK: ProofPack = ProofPack {
    id: "build-timing-receipt-wrapper-focused",
    commands: &["bash scripts/tests/test-build-timing-receipt-wrapper.sh"],
};

const COMPARE_BUILD_TIMING_WRAPPER_PACK: ProofPack = ProofPack {
    id: "compare-build-timing-wrapper-focused",
    commands: &["bash scripts/tests/test-compare-build-timing-wrapper.sh"],
};

const COVERAGE_BASELINE_SCRIPT_PACK: ProofPack = ProofPack {
    id: "coverage-baseline-script-focused",
    commands: &["bash scripts/tests/test-check-coverage-baseline.sh"],
};

const UPDATE_COVERAGE_BASELINE_SCRIPT_PACK: ProofPack = ProofPack {
    id: "update-coverage-baseline-script-focused",
    commands: &["bash scripts/tests/test-update-coverage-baseline.sh"],
};

const GENERATE_RECEIPT_SCRIPT_PACK: ProofPack = ProofPack {
    id: "generate-receipt-script-focused",
    commands: &["bash scripts/tests/test-generate-receipt.sh"],
};

const QUICK_RECEIPTS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "quick-receipts-wrapper-focused",
    commands: &["bash scripts/tests/test-quick-receipts-wrapper.sh"],
};

const PUBLISH_RECEIPTS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "publish-receipts-wrapper-focused",
    commands: &["bash scripts/tests/test-publish-receipts-wrapper.sh"],
};

const GENERATE_BADGES_WRAPPER_PACK: ProofPack = ProofPack {
    id: "generate-badges-wrapper-focused",
    commands: &["bash scripts/tests/test-generate-badges-wrapper.sh"],
};

const IGNORED_TEST_COUNT_WRAPPER_PACK: ProofPack = ProofPack {
    id: "ignored-test-count-wrapper-focused",
    commands: &["bash scripts/tests/test-ignored-test-count-wrapper.sh"],
};

const SWARM_SUMMARY_WRAPPER_PACK: ProofPack = ProofPack {
    id: "swarm-summary-wrapper-focused",
    commands: &["bash scripts/tests/test-swarm-summary-wrapper.sh"],
};

const WORKSPACE_EXCLUSIONS_WRAPPER_PACK: ProofPack = ProofPack {
    id: "workspace-exclusions-wrapper-focused",
    commands: &["bash scripts/tests/test-validate-workspace-exclusions-wrapper.sh"],
};

const WORKTREE_CLEANUP_WRAPPER_PACK: ProofPack = ProofPack {
    id: "worktree-cleanup-wrapper-focused",
    commands: &["bash scripts/tests/test-cleanup-worktrees-wrapper.sh"],
};

const CLEANUP_COMPLETED_WORKTREES_PACK: ProofPack = ProofPack {
    id: "cleanup-completed-worktrees-focused",
    commands: &["bash scripts/tests/test-cleanup-completed-worktrees.sh"],
};

const CLEAN_TMP_TARGETS_PACK: ProofPack = ProofPack {
    id: "clean-tmp-targets-focused",
    commands: &["bash scripts/tests/test-clean-tmp-targets.sh"],
};

const SWARM_CLEANUP_PACK: ProofPack = ProofPack {
    id: "swarm-cleanup-focused",
    commands: &[
        "bash scripts/tests/test_swarm_clean.sh",
        "bash scripts/tests/test_swarm_doctor.sh",
    ],
};

const PRE_MERGE_CHECK_PACK: ProofPack = ProofPack {
    id: "pre-merge-check-focused",
    commands: &["bash scripts/tests/test-pre-merge-check.sh"],
};

const GENERAL_RUST_PACK: ProofPack = ProofPack {
    id: "rust-focused",
    commands: &["cargo check --workspace --all-targets --profile agent --locked"],
};

pub fn run(args: CiRouteArgs) -> Result<()> {
    let changed_files = if args.changed_files.is_empty() {
        git_changed_files(&args.base, &args.head)?
    } else {
        normalize_changed_files(args.changed_files)
    };
    let receipt = route_receipt(&args.base, &args.head, changed_files)?;
    write_receipt(&args.receipt, &receipt)?;
    let markdown = render_summary(&args.receipt, &args.summary, &receipt);
    write_text(&args.summary, &markdown)?;
    println!(
        "ci route receipt OK: {} changed files, {} proof packs, receipt {} summary {}",
        receipt.changed_files.len(),
        receipt.required_proof_packs.len(),
        args.receipt.display(),
        args.summary.display()
    );
    Ok(())
}

fn git_changed_files(base: &str, head: &str) -> Result<Vec<String>> {
    let output =
        Command::new("git").args(["diff", "--name-only", &format!("{base}...{head}")]).output()?;
    if !output.status.success() {
        bail!(
            "git diff --name-only {base}...{head} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let raw = String::from_utf8(output.stdout)?;
    Ok(normalize_changed_files(raw.lines().map(ToString::to_string).collect()))
}

fn normalize_changed_files(files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .map(|file| file.replace('\\', "/"))
        .filter(|file| !file.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn route_receipt(base: &str, head: &str, changed_files: Vec<String>) -> Result<CiRouteReceipt> {
    let mut route = RouteBuilder::default();
    route.add_pack(PREFLIGHT_PACK);

    if changed_files.is_empty() {
        route.add_surface("no_changes");
    }

    let docs_only =
        !changed_files.is_empty() && changed_files.iter().all(|file| is_docs_file(file));
    if docs_only {
        route.add_surface("docs");
        route.add_pack(DOCS_PACK);
        route.skip("full-ux-regression", "docs-only change");
        route.skip("release-smoke", "no release surface changed");
        route.skip("docker", "no docker or release workflow changed");
        route.skip("codecov-patch-95", "docs-only change");
    } else {
        for file in &changed_files {
            route_file(file, &mut route);
        }
        route.skip("release-smoke", "no release surface changed");
        route.skip("docker", "no docker or release workflow changed");
    }

    let requested_coverage_pack_selector: Vec<String> =
        route.coverage_pack_selector.iter().cloned().collect();
    let (coverage_pack_selector, skipped_coverage_packs, coverage_proof_packs) =
        coverage_proof_pack_selection(&requested_coverage_pack_selector, &changed_files)?;
    for (pack, reason) in skipped_coverage_packs {
        route.skip(pack, reason);
    }
    let estimated_lem = route.estimated_lem(coverage_pack_selector.len());

    Ok(CiRouteReceipt {
        schema_version: "ci-route.v1",
        provider_action: "changed_file_proof_pack_route",
        claim_boundary: "CI-enforced changed-file proof routing; selected coverage pack commands feed Codecov / Patch 95 on pull requests",
        base: base.to_string(),
        head: head.to_string(),
        changed_files,
        changed_surfaces: route.surfaces.into_iter().collect(),
        required_proof_packs: route
            .proof_packs
            .into_values()
            .map(|pack| ProofPackReceipt {
                id: pack.id.to_string(),
                commands: pack.commands.iter().map(|command| (*command).to_string()).collect(),
            })
            .collect(),
        skipped_by_policy: route.skipped_by_policy,
        coverage_pack_selector,
        coverage_proof_packs,
        estimated_lem,
    })
}

fn route_file(file: &str, route: &mut RouteBuilder) {
    if file == "xtask/src/tasks/supported_editor_inline_smoke.rs"
        || file == "xtask/tests/supported_editor_inline_smoke_cli.rs"
    {
        route.add_surface("xtask-supported-editor-inline-smoke");
        route.add_pack(XTASK_SUPPORTED_EDITOR_INLINE_PACK);
        route.add_coverage_pack("patch-coverage-xtask-supported-editor-inline-smoke");
        route.skip("full-ux-regression", "supported-editor smoke receipt change");
        return;
    }

    if file == "xtask/src/tasks/semantic_inline_receipts.rs"
        || file == "xtask/src/tasks/semantic_inline_next_edit.rs"
        || file == "xtask/tests/semantic_inline_receipts_cli.rs"
        || file == "xtask/tests/semantic_inline_next_edit_cli.rs"
    {
        route.add_surface("xtask-semantic-inline-receipts");
        route.add_pack(XTASK_SEMANTIC_INLINE_PACK);
        route.add_coverage_pack("patch-coverage-xtask-semantic-inline");
        route.skip("full-ux-regression", "semantic receipt/dashboard change");
        return;
    }

    if file.starts_with("crates/perl-lsp-rs-core/src/providers/inline_completion/") {
        route.add_surface("inline-core");
        route.add_pack(INLINE_CORE_PACK);
        route.add_coverage_pack("patch-coverage-inline-provider-core");
        return;
    }

    if file == "crates/perl-lsp-rs-core/tests/inline_completion_ux_fixtures.rs" {
        route.add_surface("inline-ux-fixtures");
        route.add_pack(INLINE_UX_FIXTURES_PACK);
        route.add_coverage_pack("patch-coverage-inline-ux-fixtures");
        return;
    }

    if file == "xtask/src/tasks/inline_completion_quality.rs" {
        route.add_surface("xtask-inline-completion-quality");
        route.add_pack(XTASK_INLINE_COMPLETION_QUALITY_PACK);
        route.add_coverage_pack("patch-coverage-xtask-inline-quality");
        return;
    }

    if file.starts_with("crates/perl-lsp-rs-core/src/providers/completion/") {
        route.add_surface("completion-core");
        route.add_pack(COMPLETION_CORE_PACK);
        route.add_coverage_pack("patch-coverage-completion-core");
        return;
    }

    if file.starts_with("crates/perl-lsp-ux-tests/tests/ux_scenario_") {
        route.add_surface("ux-scenario");
        route.add_pack(UX_SCENARIO_PACK);
        route.add_coverage_pack("patch-coverage-ux-scenario");
        return;
    }

    if file.starts_with(".github/workflows/")
        || file.starts_with(".ci/")
        || file.starts_with("policy/")
        || file == "scripts/ci/ci_classify.py"
        || file == "scripts/ci/test_ci_classify.py"
        || matches!(
            file,
            "xtask/tests/codecov_patch_gate_policy.rs"
                | "xtask/tests/quality_ci_wiring_policy.rs"
                | "xtask/tests/quality_gate_patch_coverage_cli_policy.rs"
        )
    {
        route.add_surface("ci-policy");
        route.add_pack(CI_POLICY_PACK);
        route.add_coverage_pack("patch-coverage-ci-policy");
        return;
    }

    if file == "scripts/ci/route-codecov-packs.py"
        || file == "scripts/ci/test_route_codecov_packs.py"
        || file == "xtask/src/tasks/ci_route.rs"
        || file == "xtask/tests/ci_route_cli.rs"
    {
        route.add_surface("ci-routing");
        route.add_pack(CI_ROUTE_PACK);
        route.add_coverage_pack("patch-coverage-ci-route");
        return;
    }

    if file == "scripts/ci/emit_ci_actuals.py" || file == "scripts/ci/test_emit_ci_actuals.py" {
        route.add_surface("ci-actuals");
        route.add_pack(CI_ACTUALS_PACK);
        route.add_coverage_pack("patch-coverage-ci-actuals");
        return;
    }

    if file == "scripts/ci/ripr_summary.py" || file == "scripts/ci/test_ripr_summary.py" {
        route.add_surface("ripr-summary");
        route.add_pack(RIPR_SUMMARY_PACK);
        route.add_coverage_pack("patch-coverage-ripr-summary");
        return;
    }

    if file == "scripts/ci/learned_estimate.py" || file == "scripts/ci/test_learned_estimate.py" {
        route.add_surface("learned-estimate");
        route.add_pack(LEARNED_ESTIMATE_PACK);
        route.add_coverage_pack("patch-coverage-learned-estimate");
        return;
    }

    if file == "scripts/ci/validate_risk_packs.py"
        || file == "scripts/ci/test_validate_risk_packs.py"
    {
        route.add_surface("risk-packs-validator");
        route.add_pack(RISK_PACKS_VALIDATOR_PACK);
        route.add_coverage_pack("patch-coverage-risk-packs-validator");
        return;
    }

    if file == "scripts/ci/validate_gate_lane_mapping.py"
        || file == "scripts/ci/test_validate_gate_lane_mapping.py"
    {
        route.add_surface("gate-lane-mapping");
        route.add_pack(GATE_LANE_MAPPING_PACK);
        route.add_coverage_pack("patch-coverage-gate-lane-mapping");
        return;
    }

    if file == "scripts/ci/validate_trust_lanes.py"
        || file == "scripts/ci/test_validate_trust_lanes.py"
    {
        route.add_surface("trust-lanes-validator");
        route.add_pack(TRUST_LANES_VALIDATOR_PACK);
        route.add_coverage_pack("patch-coverage-trust-lanes-validator");
        return;
    }

    if file == "scripts/ci/receipts-to-junit.py" || file == "scripts/ci/test_receipts_to_junit.py" {
        route.add_surface("receipts-junit");
        route.add_pack(RECEIPTS_JUNIT_PACK);
        route.add_coverage_pack("patch-coverage-receipts-junit");
        return;
    }

    if file == "scripts/ci/check_perl_lsp_rs_core_package.py"
        || file == "scripts/ci/test_check_perl_lsp_rs_core_package.py"
    {
        route.add_surface("core-package-validator");
        route.add_pack(CORE_PACKAGE_VALIDATOR_PACK);
        route.add_coverage_pack("patch-coverage-core-package-validator");
        return;
    }

    if file == "scripts/ci/aggregate_lane_history.py"
        || file == "scripts/ci/test_aggregate_lane_history.py"
    {
        route.add_surface("aggregate-lane-history");
        route.add_pack(AGGREGATE_LANE_HISTORY_PACK);
        route.add_coverage_pack("patch-coverage-aggregate-lane-history");
        return;
    }

    if file == "scripts/ci/pr_plan.py" || file == "scripts/ci/test_pr_plan.py" {
        route.add_surface("pr-plan");
        route.add_pack(PR_PLAN_PACK);
        route.add_coverage_pack("patch-coverage-pr-plan");
        return;
    }

    if file == "scripts/pr_overlap.py" || file == "scripts/tests/test_pr_overlap.py" {
        route.add_surface("pr-overlap");
        route.add_pack(PR_OVERLAP_PACK);
        route.add_coverage_pack("patch-coverage-pr-overlap");
        return;
    }

    if file == "scripts/control-plane-lock.sh" || file == "scripts/test-control-plane-lock.sh" {
        route.add_surface("control-plane-lock");
        route.add_pack(CONTROL_PLANE_LOCK_PACK);
        route.add_coverage_pack("patch-coverage-control-plane-lock");
        return;
    }

    if file == "scripts/agent-preflight.sh" || file == "scripts/test-agent-preflight.sh" {
        route.add_surface("agent-preflight");
        route.add_pack(AGENT_PREFLIGHT_PACK);
        route.add_coverage_pack("patch-coverage-agent-preflight");
        return;
    }

    if file == "scripts/ci-audit-workflows.py"
        || file == "scripts/tests/test-ci-audit-workflows-shim.py"
    {
        route.add_surface("ci-audit-workflows-shim");
        route.add_pack(CI_AUDIT_WORKFLOWS_SHIM_PACK);
        route.add_coverage_pack("patch-coverage-ci-audit-workflows-shim");
        return;
    }

    if file == "scripts/check-doc-claims.py"
        || file == "scripts/tests/test-check-doc-claims-shim.py"
    {
        route.add_surface("doc-claims-shim");
        route.add_pack(DOC_CLAIMS_SHIM_PACK);
        route.add_coverage_pack("patch-coverage-doc-claims-shim");
        return;
    }

    if file == "scripts/check_features_invariants.py"
        || file == "scripts/tests/test-check-features-invariants-shim.py"
    {
        route.add_surface("features-invariants-shim");
        route.add_pack(FEATURES_INVARIANTS_SHIM_PACK);
        route.add_coverage_pack("patch-coverage-features-invariants-shim");
        return;
    }

    if file == "scripts/debt-report.py" || file == "scripts/tests/test-debt-report-shim.py" {
        route.add_surface("debt-report-shim");
        route.add_pack(DEBT_REPORT_SHIM_PACK);
        route.add_coverage_pack("patch-coverage-debt-report-shim");
        return;
    }

    if file == "scripts/preflight.sh" || file == "scripts/tests/test-preflight-wrapper.sh" {
        route.add_surface("preflight-wrapper");
        route.add_pack(PREFLIGHT_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-preflight-wrapper");
        return;
    }

    if file == "scripts/install-githooks.sh"
        || file == "scripts/tests/test-install-githooks-wrapper.sh"
    {
        route.add_surface("install-githooks-wrapper");
        route.add_pack(INSTALL_GITHOOKS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-install-githooks-wrapper");
        return;
    }

    if file == "scripts/e2e-gate.sh" || file == "scripts/tests/test-e2e-gate-wrapper.sh" {
        route.add_surface("e2e-gate-wrapper");
        route.add_pack(E2E_GATE_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-e2e-gate-wrapper");
        return;
    }

    if file == "scripts/execute-gate.sh" || file == "scripts/tests/test-execute-gate-wrapper.sh" {
        route.add_surface("execute-gate-wrapper");
        route.add_pack(EXECUTE_GATE_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-execute-gate-wrapper");
        return;
    }

    if file == "scripts/run-gates.sh" || file == "scripts/tests/test-run-gates-wrapper.sh" {
        route.add_surface("run-gates-wrapper");
        route.add_pack(RUN_GATES_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-run-gates-wrapper");
        return;
    }

    if file == "scripts/gate-local.sh" || file == "scripts/tests/test-gate-local-wrapper.sh" {
        route.add_surface("gate-local-wrapper");
        route.add_pack(GATE_LOCAL_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-gate-local-wrapper");
        return;
    }

    if file == "scripts/list-gates.py" || file == "scripts/tests/test-list-gates-wrapper.py" {
        route.add_surface("list-gates-wrapper");
        route.add_pack(LIST_GATES_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-list-gates-wrapper");
        return;
    }

    if file == "scripts/forbid-fatal-constructs.sh"
        || file == "scripts/tests/test-forbid-fatal-constructs-wrapper.sh"
    {
        route.add_surface("forbid-fatal-constructs-wrapper");
        route.add_pack(FORBID_FATAL_CONSTRUCTS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-forbid-fatal-constructs-wrapper");
        return;
    }

    if file == "scripts/dead-code-check.sh" || file == "scripts/tests/test-dead-code-wrapper.sh" {
        route.add_surface("dead-code-wrapper");
        route.add_pack(DEAD_CODE_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-dead-code-wrapper");
        return;
    }

    if file == "scripts/check-rust-toolchain.sh"
        || file == "scripts/tests/test-check-rust-toolchain-wrapper.sh"
    {
        route.add_surface("check-toolchain-wrapper");
        route.add_pack(CHECK_TOOLCHAIN_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-check-toolchain-wrapper");
        return;
    }

    if file == "scripts/devex-doctor.sh" || file == "scripts/tests/test-devex-doctor-wrapper.sh" {
        route.add_surface("devex-doctor-wrapper");
        route.add_pack(DEVEX_DOCTOR_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-devex-doctor-wrapper");
        return;
    }

    if file == "scripts/devex-targeted-checks.sh"
        || file == "scripts/tests/test-devex-targeted-checks-wrapper.sh"
    {
        route.add_surface("devex-targeted-checks-wrapper");
        route.add_pack(DEVEX_TARGETED_CHECKS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-devex-targeted-checks-wrapper");
        return;
    }

    if file == "scripts/test-lsp-cancellation.sh"
        || file == "scripts/tests/test-lsp-cancellation-wrapper.sh"
    {
        route.add_surface("lsp-cancellation-wrapper");
        route.add_pack(LSP_CANCELLATION_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-lsp-cancellation-wrapper");
        return;
    }

    if file == "scripts/test-capped.sh" || file == "scripts/tests/test-test-capped-wrapper.sh" {
        route.add_surface("test-capped-wrapper");
        route.add_pack(TEST_CAPPED_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-test-capped-wrapper");
        return;
    }

    if file == "scripts/test-e2e-capped.sh"
        || file == "scripts/tests/test-test-e2e-capped-wrapper.sh"
    {
        route.add_surface("test-e2e-capped-wrapper");
        route.add_pack(TEST_E2E_CAPPED_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-test-e2e-capped-wrapper");
        return;
    }

    if file == "scripts/ci-cost-monitor.sh"
        || file == "scripts/tests/test-ci-cost-monitor-wrapper.sh"
    {
        route.add_surface("ci-cost-monitor-wrapper");
        route.add_pack(CI_COST_MONITOR_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-ci-cost-monitor-wrapper");
        return;
    }

    if file == "scripts/build-timing-receipt.sh"
        || file == "scripts/tests/test-build-timing-receipt-wrapper.sh"
    {
        route.add_surface("build-timing-receipt-wrapper");
        route.add_pack(BUILD_TIMING_RECEIPT_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-build-timing-receipt-wrapper");
        return;
    }

    if file == "scripts/compare-build-timing.sh"
        || file == "scripts/tests/test-compare-build-timing-wrapper.sh"
    {
        route.add_surface("compare-build-timing-wrapper");
        route.add_pack(COMPARE_BUILD_TIMING_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-compare-build-timing-wrapper");
        return;
    }

    if file == "scripts/check-coverage-baseline.sh"
        || file == "scripts/tests/test-check-coverage-baseline.sh"
    {
        route.add_surface("coverage-baseline-script");
        route.add_pack(COVERAGE_BASELINE_SCRIPT_PACK);
        route.add_coverage_pack("patch-coverage-baseline-script");
        return;
    }

    if file == "scripts/update-coverage-baseline.sh"
        || file == "scripts/tests/test-update-coverage-baseline.sh"
    {
        route.add_surface("update-coverage-baseline-script");
        route.add_pack(UPDATE_COVERAGE_BASELINE_SCRIPT_PACK);
        route.add_coverage_pack("patch-coverage-update-baseline-script");
        return;
    }

    if file == "scripts/generate-receipt.sh" || file == "scripts/tests/test-generate-receipt.sh" {
        route.add_surface("generate-receipt-script");
        route.add_pack(GENERATE_RECEIPT_SCRIPT_PACK);
        route.add_coverage_pack("patch-coverage-generate-receipt-script");
        return;
    }

    if file == "scripts/quick-receipts.sh" || file == "scripts/tests/test-quick-receipts-wrapper.sh"
    {
        route.add_surface("quick-receipts-wrapper");
        route.add_pack(QUICK_RECEIPTS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-quick-receipts-wrapper");
        return;
    }

    if file == "scripts/publish-receipts.sh"
        || file == "scripts/tests/test-publish-receipts-wrapper.sh"
    {
        route.add_surface("publish-receipts-wrapper");
        route.add_pack(PUBLISH_RECEIPTS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-publish-receipts-wrapper");
        return;
    }

    if file == "scripts/generate-badges.sh"
        || file == "scripts/tests/test-generate-badges-wrapper.sh"
    {
        route.add_surface("generate-badges-wrapper");
        route.add_pack(GENERATE_BADGES_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-generate-badges-wrapper");
        return;
    }

    if file == "scripts/ignored-test-count.sh"
        || file == "scripts/tests/test-ignored-test-count-wrapper.sh"
    {
        route.add_surface("ignored-test-count-wrapper");
        route.add_pack(IGNORED_TEST_COUNT_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-ignored-test-count-wrapper");
        return;
    }

    if file == "scripts/swarm-summary.sh" || file == "scripts/tests/test-swarm-summary-wrapper.sh" {
        route.add_surface("swarm-summary-wrapper");
        route.add_pack(SWARM_SUMMARY_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-swarm-summary-wrapper");
        return;
    }

    if file == "scripts/validate-workspace-exclusions.sh"
        || file == "scripts/tests/test-validate-workspace-exclusions-wrapper.sh"
    {
        route.add_surface("workspace-exclusions-wrapper");
        route.add_pack(WORKSPACE_EXCLUSIONS_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-workspace-exclusions-wrapper");
        return;
    }

    if file == "scripts/cleanup-worktrees.sh"
        || file == "scripts/tests/test-cleanup-worktrees-wrapper.sh"
    {
        route.add_surface("worktree-cleanup-wrapper");
        route.add_pack(WORKTREE_CLEANUP_WRAPPER_PACK);
        route.add_coverage_pack("patch-coverage-worktree-cleanup-wrapper");
        return;
    }

    if file == "scripts/cleanup-completed-worktrees.sh"
        || file == "scripts/tests/test-cleanup-completed-worktrees.sh"
    {
        route.add_surface("cleanup-completed-worktrees");
        route.add_pack(CLEANUP_COMPLETED_WORKTREES_PACK);
        route.add_coverage_pack("patch-coverage-cleanup-completed-worktrees");
        return;
    }

    if file == "scripts/clean-tmp-targets.sh" || file == "scripts/tests/test-clean-tmp-targets.sh" {
        route.add_surface("clean-tmp-targets");
        route.add_pack(CLEAN_TMP_TARGETS_PACK);
        route.add_coverage_pack("patch-coverage-clean-tmp-targets");
        return;
    }

    if matches!(
        file,
        "scripts/swarm-clean"
            | "scripts/swarm-doctor"
            | "scripts/tests/test_swarm_clean.sh"
            | "scripts/tests/test_swarm_doctor.sh"
    ) {
        route.add_surface("swarm-cleanup");
        route.add_pack(SWARM_CLEANUP_PACK);
        route.add_coverage_pack("patch-coverage-swarm-cleanup");
        return;
    }

    if file == "scripts/pre-merge-check.sh" || file == "scripts/tests/test-pre-merge-check.sh" {
        route.add_surface("pre-merge-check");
        route.add_pack(PRE_MERGE_CHECK_PACK);
        route.add_coverage_pack("patch-coverage-pre-merge-check");
        return;
    }

    if is_docs_file(file) {
        route.add_surface("docs");
        route.add_pack(DOCS_PACK);
        return;
    }

    if file.ends_with(".rs") {
        route.add_surface("rust");
        route.add_pack(GENERAL_RUST_PACK);
        route.add_coverage_pack("patch-coverage-rust-focused");
    } else {
        route.add_surface("misc");
    }
}

fn is_docs_file(file: &str) -> bool {
    file.starts_with("docs/")
        || file == "README.md"
        || file == "CHANGELOG.md"
        || file.ends_with(".md")
}

fn write_receipt(path: &Path, receipt: &CiRouteReceipt) -> Result<()> {
    let json = serde_json::to_string_pretty(receipt)?;
    write_text(path, &format!("{json}\n"))
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn render_summary(receipt_path: &Path, summary_path: &Path, receipt: &CiRouteReceipt) -> String {
    let mut markdown = String::new();
    writeln!(markdown, "# CI Route Proof Packet").ok();
    writeln!(markdown).ok();
    writeln!(markdown, "- decision: `advisory`").ok();
    writeln!(markdown, "- provider_action: `{}`", receipt.provider_action).ok();
    writeln!(markdown, "- claim_boundary: {}", receipt.claim_boundary).ok();
    writeln!(markdown, "- base: `{}`", receipt.base).ok();
    writeln!(markdown, "- head: `{}`", receipt.head).ok();
    writeln!(markdown, "- receipt: `{}`", receipt_path.display()).ok();
    writeln!(markdown, "- summary: `{}`", summary_path.display()).ok();
    writeln!(markdown, "- estimated_lem: `{}`", receipt.estimated_lem).ok();
    writeln!(markdown).ok();

    markdown_list(&mut markdown, "Changed Files", &receipt.changed_files);
    markdown_list(&mut markdown, "Changed Surfaces", &receipt.changed_surfaces);
    markdown_skips(&mut markdown, &receipt.skipped_by_policy);
    markdown_proof_packs(&mut markdown, &receipt.required_proof_packs);
    markdown_coverage_packs(&mut markdown, &receipt.coverage_proof_packs);

    writeln!(markdown, "## Refresh Command").ok();
    writeln!(markdown).ok();
    writeln!(markdown, "```bash").ok();
    writeln!(markdown, "{}", refresh_command(receipt_path, summary_path, receipt)).ok();
    writeln!(markdown, "```").ok();
    markdown
}

fn refresh_command(receipt_path: &Path, summary_path: &Path, receipt: &CiRouteReceipt) -> String {
    let mut command = format!(
        "rtk cargo xtask ci route --base {} --head {} --receipt {} --summary {}",
        shell_quote(&receipt.base),
        shell_quote(&receipt.head),
        shell_quote(&receipt_path.display().to_string()),
        shell_quote(&summary_path.display().to_string())
    );
    for file in &receipt.changed_files {
        write!(command, " --changed-file {}", shell_quote(file)).ok();
    }
    command
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '-' | '_' | '.' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn markdown_list(markdown: &mut String, heading: &str, values: &[String]) {
    writeln!(markdown, "## {heading}").ok();
    writeln!(markdown).ok();
    if values.is_empty() {
        writeln!(markdown, "- none").ok();
    } else {
        for value in values {
            writeln!(markdown, "- `{value}`").ok();
        }
    }
    writeln!(markdown).ok();
}

fn markdown_skips(markdown: &mut String, skipped_by_policy: &BTreeMap<String, String>) {
    writeln!(markdown, "## Skipped By Policy").ok();
    writeln!(markdown).ok();
    if skipped_by_policy.is_empty() {
        writeln!(markdown, "- none").ok();
    } else {
        for (pack, reason) in skipped_by_policy {
            writeln!(markdown, "- `{pack}`: {reason}").ok();
        }
    }
    writeln!(markdown).ok();
}

fn markdown_proof_packs(markdown: &mut String, proof_packs: &[ProofPackReceipt]) {
    writeln!(markdown, "## Required Proof Packs").ok();
    writeln!(markdown).ok();
    for pack in proof_packs {
        writeln!(markdown, "### `{}`", pack.id).ok();
        writeln!(markdown).ok();
        for command in &pack.commands {
            writeln!(markdown, "- `{command}`").ok();
        }
        writeln!(markdown).ok();
    }
}

fn markdown_coverage_packs(markdown: &mut String, coverage_packs: &[CoverageProofPackReceipt]) {
    writeln!(markdown, "## Coverage Proof Packs").ok();
    writeln!(markdown).ok();
    if coverage_packs.is_empty() {
        writeln!(markdown, "- none").ok();
        writeln!(markdown).ok();
        return;
    }

    for pack in coverage_packs {
        writeln!(markdown, "### `{}`", pack.id).ok();
        writeln!(markdown).ok();
        writeln!(markdown, "Files:").ok();
        for file in &pack.files {
            writeln!(markdown, "- `{file}`").ok();
        }
        writeln!(markdown).ok();
        writeln!(markdown, "Coverage filters:").ok();
        for filter in &pack.coverage_filters {
            writeln!(markdown, "- `{filter}`").ok();
        }
        writeln!(markdown).ok();
        writeln!(markdown, "Commands:").ok();
        for command in &pack.commands {
            writeln!(markdown, "- `{command}`").ok();
        }
        writeln!(markdown).ok();
    }
}

#[cfg(test)]
fn coverage_proof_pack_receipts(selector: &[String]) -> Result<Vec<CoverageProofPackReceipt>> {
    let manifest = coverage_pack_manifest()?;
    let changed_files: Vec<String> = manifest
        .pack
        .iter()
        .filter(|pack| selector.iter().any(|selected| selected == &pack.id))
        .flat_map(|pack| pack.files.iter().cloned())
        .filter(|path| is_lcov_source_path(path))
        .collect();
    let (_, _, proof_packs) = coverage_proof_pack_selection(selector, &changed_files)?;
    Ok(proof_packs)
}

fn coverage_proof_pack_selection(
    selector: &[String],
    changed_files: &[String],
) -> Result<(Vec<String>, BTreeMap<String, String>, Vec<CoverageProofPackReceipt>)> {
    let manifest = coverage_pack_manifest()?;
    let packs_by_id: BTreeMap<&str, &CoveragePack> =
        manifest.pack.iter().map(|pack| (pack.id.as_str(), pack)).collect();
    let mut selected = Vec::new();
    let mut skipped = BTreeMap::new();
    let mut proof_packs = Vec::new();
    for pack_id in selector {
        let Some(pack) = packs_by_id.get(pack_id.as_str()) else {
            bail!("coverage pack `{pack_id}` is missing from .ci/coverage-packs.toml");
        };
        if !pack.lcov {
            skipped.insert(pack_id.clone(), NON_LCOV_COVERAGE_SKIP_REASON.to_string());
            continue;
        }
        if !pack_matches_lcov_source(pack, changed_files) {
            skipped.insert(pack_id.clone(), NON_SOURCE_LCOV_COVERAGE_SKIP_REASON.to_string());
            continue;
        }
        selected.push(pack_id.clone());
        proof_packs.push(CoverageProofPackReceipt {
            id: pack.id.clone(),
            files: pack.files.clone(),
            commands: pack.commands.clone(),
            coverage_filters: pack.coverage_filters.clone(),
        });
    }
    Ok((selected, skipped, proof_packs))
}

fn pack_matches_lcov_source(pack: &CoveragePack, paths: &[String]) -> bool {
    paths.iter().any(|path| {
        is_lcov_source_path(path)
            && pack.files.iter().any(|pattern| matches_coverage_pattern(path, pattern))
    })
}

fn is_lcov_source_path(path: &str) -> bool {
    path.ends_with(".rs")
        && !path.starts_with("xtask/tests/")
        && !path.contains("/tests/")
        && (path.starts_with("xtask/src/") || path.starts_with("crates/"))
}

fn matches_coverage_pattern(path: &str, pattern: &str) -> bool {
    let normalized_pattern = pattern.replace('\\', "/");
    if let Some(suffix) = normalized_pattern.strip_prefix("*.") {
        return path.ends_with(&format!(".{suffix}"));
    }
    if normalized_pattern.ends_with('/') {
        return path.starts_with(&normalized_pattern);
    }
    path == normalized_pattern || path.starts_with(&normalized_pattern)
}

fn coverage_pack_manifest() -> Result<CoveragePackManifest> {
    parse_coverage_pack_manifest(COVERAGE_PACKS_TOML)
}

fn default_lcov() -> bool {
    true
}

fn parse_coverage_pack_manifest(contents: &str) -> Result<CoveragePackManifest> {
    let manifest: CoveragePackManifest = toml::from_str(contents)?;
    let mut ids = BTreeSet::new();
    for pack in &manifest.pack {
        if pack.id.trim().is_empty() {
            bail!("coverage pack id must not be empty");
        }
        if pack.files.is_empty() {
            bail!("coverage pack `{}` must list at least one file", pack.id);
        }
        if pack.commands.is_empty() {
            bail!("coverage pack `{}` must list at least one command", pack.id);
        }
        if pack.coverage_filters.is_empty() {
            bail!("coverage pack `{}` must list at least one coverage filter", pack.id);
        }
        if !ids.insert(pack.id.as_str()) {
            bail!("duplicate coverage pack id `{}`", pack.id);
        }
    }
    Ok(manifest)
}

#[derive(Default)]
struct RouteBuilder {
    surfaces: BTreeSet<String>,
    proof_packs: BTreeMap<&'static str, ProofPack>,
    skipped_by_policy: BTreeMap<String, String>,
    coverage_pack_selector: BTreeSet<String>,
}

impl RouteBuilder {
    fn add_surface(&mut self, surface: impl Into<String>) {
        self.surfaces.insert(surface.into());
    }

    fn add_pack(&mut self, pack: ProofPack) {
        self.proof_packs.insert(pack.id, pack);
    }

    fn skip(&mut self, pack: impl Into<String>, reason: impl Into<String>) {
        self.skipped_by_policy.entry(pack.into()).or_insert_with(|| reason.into());
    }

    fn add_coverage_pack(&mut self, pack: impl Into<String>) {
        self.coverage_pack_selector.insert(pack.into());
    }

    fn estimated_lem(&self, coverage_pack_count: usize) -> u64 {
        let pack_cost = u64::try_from(self.proof_packs.len()).unwrap_or(u64::MAX);
        let coverage_cost = u64::try_from(coverage_pack_count).unwrap_or(u64::MAX);
        2 + pack_cost.saturating_mul(3) + coverage_cost.saturating_mul(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;
    use serde_json::Value;
    use tempfile::TempDir;

    #[test]
    fn ci_route_receipt_maps_supported_editor_smoke_to_focused_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/src/tasks/supported_editor_inline_smoke.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["xtask-supported-editor-inline-smoke"]);
        assert!(proof_pack_ids(&receipt).contains(&"xtask-supported-editor-inline-smoke"));
        assert!(
            receipt
                .coverage_pack_selector
                .iter()
                .any(|pack| pack == "patch-coverage-xtask-supported-editor-inline-smoke")
        );
        assert!(receipt.coverage_proof_packs.iter().any(|pack| {
            pack.id == "patch-coverage-xtask-supported-editor-inline-smoke"
                && pack
                    .commands
                    .iter()
                    .any(|command| command.contains("supported_editor_inline_smoke"))
                && pack
                    .coverage_filters
                    .iter()
                    .any(|filter| filter == "supported_editor_inline_smoke")
        }));
        assert_eq!(
            receipt.skipped_by_policy.get("full-ux-regression").map(String::as_str),
            Some("supported-editor smoke receipt change")
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_semantic_inline_receipts_to_dashboard_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/src/tasks/semantic_inline_receipts.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["xtask-semantic-inline-receipts"]);
        assert!(proof_pack_ids(&receipt).contains(&"xtask-semantic-inline-receipts"));
        assert!(
            receipt
                .coverage_pack_selector
                .iter()
                .any(|pack| pack == "patch-coverage-xtask-semantic-inline")
        );
        assert!(receipt.coverage_proof_packs.iter().any(|pack| {
            pack.id == "patch-coverage-xtask-semantic-inline"
                && pack.commands.iter().any(|command| command.contains("semantic_inline_receipts"))
        }));
        Ok(())
    }

    #[test]
    fn ci_route_receipt_skips_coverage_for_docs_only_changes() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["docs/development/INLINE_COMPLETION_ROADMAP.md".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["docs"]);
        assert!(proof_pack_ids(&receipt).contains(&"docs-focused"));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("codecov-patch-95").map(String::as_str),
            Some("docs-only change")
        );
        Ok(())
    }

    #[test]
    fn route_receipt_maps_ci_route_files_to_focused_non_lcov_route_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["xtask/src/tasks/ci_route.rs".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-routing"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-route-receipt"));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ci-route").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn route_receipt_maps_codecov_router_script_to_focused_non_lcov_route_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/route-codecov-packs.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-routing"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-route-receipt"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ci-route-receipt"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_route_codecov_packs.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ci-route").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ci_policy_tests_to_focused_non_lcov_policy_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/tests/quality_ci_wiring_policy.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-policy"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-policy-focused"));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ci-policy").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ci_classifier_script_to_focused_policy_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/ci/ci_classify.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-policy"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-policy-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ci-policy-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python -m unittest scripts/ci/test_ci_classify.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ci-policy").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ci_actuals_script_to_focused_non_lcov_actuals_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/emit_ci_actuals.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-actuals"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-actuals-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ci-actuals-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_emit_ci_actuals.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ci-actuals").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ripr_summary_script_to_focused_non_lcov_summary_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/ci/ripr_summary.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["ripr-summary"]);
        assert!(proof_pack_ids(&receipt).contains(&"ripr-summary-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ripr-summary-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python -m unittest scripts/ci/test_ripr_summary.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-ripr-summary").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_learned_estimate_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/learned_estimate.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["learned-estimate"]);
        assert!(proof_pack_ids(&receipt).contains(&"learned-estimate-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "learned-estimate-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_learned_estimate.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-learned-estimate").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_risk_pack_validator_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/validate_risk_packs.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["risk-packs-validator"]);
        assert!(proof_pack_ids(&receipt).contains(&"risk-packs-validator-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "risk-packs-validator-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_validate_risk_packs.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-risk-packs-validator")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_gate_lane_mapping_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/validate_gate_lane_mapping.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["gate-lane-mapping"]);
        assert!(proof_pack_ids(&receipt).contains(&"gate-lane-mapping-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "gate-lane-mapping-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_validate_gate_lane_mapping.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-gate-lane-mapping").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_trust_lanes_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/validate_trust_lanes.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["trust-lanes-validator"]);
        assert!(proof_pack_ids(&receipt).contains(&"trust-lanes-validator-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "trust-lanes-validator-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_validate_trust_lanes.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-trust-lanes-validator")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_receipts_junit_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/receipts-to-junit.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["receipts-junit"]);
        assert!(proof_pack_ids(&receipt).contains(&"receipts-junit-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "receipts-junit-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_receipts_to_junit.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-receipts-junit").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_core_package_validator_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/check_perl_lsp_rs_core_package.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["core-package-validator"]);
        assert!(proof_pack_ids(&receipt).contains(&"core-package-validator-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "core-package-validator-focused"
                && pack.commands.iter().any(|command| {
                    command
                        == "python -m unittest scripts/ci/test_check_perl_lsp_rs_core_package.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-core-package-validator")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_aggregate_lane_history_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci/aggregate_lane_history.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["aggregate-lane-history"]);
        assert!(proof_pack_ids(&receipt).contains(&"aggregate-lane-history-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "aggregate-lane-history-focused"
                && pack.commands.iter().any(|command| {
                    command == "python -m unittest scripts/ci/test_aggregate_lane_history.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-aggregate-lane-history")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_pr_plan_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/ci/pr_plan.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["pr-plan"]);
        assert!(proof_pack_ids(&receipt).contains(&"pr-plan-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "pr-plan-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python -m unittest scripts/ci/test_pr_plan.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-pr-plan").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_clean_tmp_targets_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/clean-tmp-targets.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["clean-tmp-targets"]);
        assert!(proof_pack_ids(&receipt).contains(&"clean-tmp-targets-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "clean-tmp-targets-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-clean-tmp-targets.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-clean-tmp-targets").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_cleanup_completed_worktrees_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/cleanup-completed-worktrees.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["cleanup-completed-worktrees"]);
        assert!(proof_pack_ids(&receipt).contains(&"cleanup-completed-worktrees-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "cleanup-completed-worktrees-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-cleanup-completed-worktrees.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-cleanup-completed-worktrees")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_swarm_cleanup_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec![
                "scripts/swarm-clean".to_string(),
                "scripts/tests/test_swarm_doctor.sh".to_string(),
            ],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["swarm-cleanup"]);
        assert!(proof_pack_ids(&receipt).contains(&"swarm-cleanup-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "swarm-cleanup-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test_swarm_clean.sh")
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test_swarm_doctor.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-swarm-cleanup").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_pre_merge_check_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/pre-merge-check.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["pre-merge-check"]);
        assert!(proof_pack_ids(&receipt).contains(&"pre-merge-check-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "pre-merge-check-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-pre-merge-check.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-pre-merge-check").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_pr_overlap_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/pr_overlap.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["pr-overlap"]);
        assert!(proof_pack_ids(&receipt).contains(&"pr-overlap-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "pr-overlap-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python scripts/tests/test_pr_overlap.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-pr-overlap").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_control_plane_lock_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/control-plane-lock.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["control-plane-lock"]);
        assert!(proof_pack_ids(&receipt).contains(&"control-plane-lock-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "control-plane-lock-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/test-control-plane-lock.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-control-plane-lock").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_agent_preflight_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/agent-preflight.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["agent-preflight"]);
        assert!(proof_pack_ids(&receipt).contains(&"agent-preflight-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "agent-preflight-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/test-agent-preflight.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-agent-preflight").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ci_audit_workflows_shim_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ci-audit-workflows.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-audit-workflows-shim"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-audit-workflows-shim-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ci-audit-workflows-shim-focused"
                && pack.commands.iter().any(|command| {
                    command == "python scripts/tests/test-ci-audit-workflows-shim.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-ci-audit-workflows-shim")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_doc_claims_shim_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/check-doc-claims.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["doc-claims-shim"]);
        assert!(proof_pack_ids(&receipt).contains(&"doc-claims-shim-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "doc-claims-shim-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python scripts/tests/test-check-doc-claims-shim.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-doc-claims-shim").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_features_invariants_shim_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/check_features_invariants.py".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["features-invariants-shim"]);
        assert!(proof_pack_ids(&receipt).contains(&"features-invariants-shim-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "features-invariants-shim-focused"
                && pack.commands.iter().any(|command| {
                    command == "python scripts/tests/test-check-features-invariants-shim.py"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-features-invariants-shim")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_debt_report_shim_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/debt-report.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["debt-report-shim"]);
        assert!(proof_pack_ids(&receipt).contains(&"debt-report-shim-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "debt-report-shim-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python scripts/tests/test-debt-report-shim.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-debt-report-shim").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_preflight_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/preflight.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["preflight-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"preflight-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "preflight-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-preflight-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-preflight-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_install_githooks_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/install-githooks.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["install-githooks-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"install-githooks-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "install-githooks-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-install-githooks-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-install-githooks-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_e2e_gate_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/e2e-gate.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["e2e-gate-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"e2e-gate-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "e2e-gate-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-e2e-gate-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-e2e-gate-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_execute_gate_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/execute-gate.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["execute-gate-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"execute-gate-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "execute-gate-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-execute-gate-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-execute-gate-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_run_gates_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/run-gates.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["run-gates-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"run-gates-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "run-gates-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-run-gates-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-run-gates-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_gate_local_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/gate-local.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["gate-local-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"gate-local-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "gate-local-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-gate-local-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-gate-local-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_list_gates_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/list-gates.py".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["list-gates-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"list-gates-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "list-gates-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "python scripts/tests/test-list-gates-wrapper.py")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-list-gates-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_forbid_fatal_constructs_wrapper_to_focused_non_lcov_pack() -> Result<()>
    {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/forbid-fatal-constructs.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["forbid-fatal-constructs-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"forbid-fatal-constructs-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "forbid-fatal-constructs-wrapper-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-forbid-fatal-constructs-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_dead_code_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/dead-code-check.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["dead-code-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"dead-code-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "dead-code-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-dead-code-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-dead-code-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_check_toolchain_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/check-rust-toolchain.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["check-toolchain-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"check-toolchain-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "check-toolchain-wrapper-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-check-rust-toolchain-wrapper.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-check-toolchain-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_lsp_cancellation_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/test-lsp-cancellation.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["lsp-cancellation-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"lsp-cancellation-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "lsp-cancellation-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-lsp-cancellation-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-lsp-cancellation-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_test_capped_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/test-capped.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["test-capped-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"test-capped-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "test-capped-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-test-capped-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-test-capped-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_test_e2e_capped_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/test-e2e-capped.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["test-e2e-capped-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"test-e2e-capped-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "test-e2e-capped-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-test-e2e-capped-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-test-e2e-capped-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ci_cost_monitor_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/ci-cost-monitor.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["ci-cost-monitor-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-cost-monitor-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ci-cost-monitor-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-ci-cost-monitor-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-ci-cost-monitor-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_build_timing_receipt_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/build-timing-receipt.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["build-timing-receipt-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"build-timing-receipt-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "build-timing-receipt-wrapper-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-build-timing-receipt-wrapper.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-build-timing-receipt-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_compare_build_timing_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/compare-build-timing.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["compare-build-timing-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"compare-build-timing-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "compare-build-timing-wrapper-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-compare-build-timing-wrapper.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-compare-build-timing-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_coverage_baseline_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/check-coverage-baseline.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["coverage-baseline-script"]);
        assert!(proof_pack_ids(&receipt).contains(&"coverage-baseline-script-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "coverage-baseline-script-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-check-coverage-baseline.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-baseline-script").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_update_coverage_baseline_script_to_focused_non_lcov_pack() -> Result<()>
    {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/update-coverage-baseline.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["update-coverage-baseline-script"]);
        assert!(proof_pack_ids(&receipt).contains(&"update-coverage-baseline-script-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "update-coverage-baseline-script-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-update-coverage-baseline.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-update-baseline-script")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_generate_receipt_script_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/generate-receipt.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["generate-receipt-script"]);
        assert!(proof_pack_ids(&receipt).contains(&"generate-receipt-script-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "generate-receipt-script-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-generate-receipt.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-generate-receipt-script")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_quick_receipts_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/quick-receipts.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["quick-receipts-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"quick-receipts-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "quick-receipts-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-quick-receipts-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-quick-receipts-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_publish_receipts_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/publish-receipts.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["publish-receipts-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"publish-receipts-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "publish-receipts-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-publish-receipts-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-publish-receipts-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_generate_badges_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["scripts/generate-badges.sh".to_string()])?;

        assert_eq!(receipt.changed_surfaces, vec!["generate-badges-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"generate-badges-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "generate-badges-wrapper-focused"
                && pack
                    .commands
                    .iter()
                    .any(|command| command == "bash scripts/tests/test-generate-badges-wrapper.sh")
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-generate-badges-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_ignored_test_count_wrapper_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["scripts/ignored-test-count.sh".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["ignored-test-count-wrapper"]);
        assert!(proof_pack_ids(&receipt).contains(&"ignored-test-count-wrapper-focused"));
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "ignored-test-count-wrapper-focused"
                && pack.commands.iter().any(|command| {
                    command == "bash scripts/tests/test-ignored-test-count-wrapper.sh"
                })
        }));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-ignored-test-count-wrapper")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_completion_provider_to_focused_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec![
                "crates/perl-lsp-rs-core/src/providers/completion/completion/import_map/used_modules.rs"
                    .to_string(),
            ],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["completion-core"]);
        assert!(proof_pack_ids(&receipt).contains(&"completion-core"));
        assert!(
            receipt
                .coverage_pack_selector
                .iter()
                .any(|pack| pack == "patch-coverage-completion-core")
        );
        assert!(receipt.coverage_proof_packs.iter().any(|pack| {
            pack.id == "patch-coverage-completion-core"
                && pack.commands.iter().any(|command| command.contains("completion::completion"))
                && pack.coverage_filters.iter().any(|filter| filter == "completion::completion")
        }));
        Ok(())
    }

    #[test]
    fn ci_route_coverage_proof_pack_receipts_materializes_each_selected_pack() -> Result<()> {
        let selector = vec![
            "patch-coverage-xtask-semantic-inline".to_string(),
            "patch-coverage-xtask-supported-editor-inline-smoke".to_string(),
        ];
        let packs = coverage_proof_pack_receipts(&selector)?;

        let pack_ids: Vec<&str> = packs.iter().map(|pack| pack.id.as_str()).collect();
        assert_eq!(
            pack_ids,
            vec![
                "patch-coverage-xtask-semantic-inline",
                "patch-coverage-xtask-supported-editor-inline-smoke"
            ]
        );

        let semantic_pack = packs.first().ok_or_else(|| eyre!("missing semantic coverage pack"))?;
        assert_eq!(
            semantic_pack.files,
            vec![
                "xtask/src/tasks/semantic_inline_receipts.rs",
                "xtask/src/tasks/semantic_inline_next_edit.rs",
                "xtask/tests/semantic_inline_receipts_cli.rs",
                "xtask/tests/semantic_inline_next_edit_cli.rs",
            ]
        );
        assert_eq!(
            semantic_pack.coverage_filters,
            vec!["semantic_inline_receipts", "semantic_inline_next_edit"]
        );

        let supported_editor_pack =
            packs.get(1).ok_or_else(|| eyre!("missing supported-editor coverage pack"))?;
        assert_eq!(
            supported_editor_pack.files,
            vec![
                "xtask/src/tasks/supported_editor_inline_smoke.rs",
                "xtask/tests/supported_editor_inline_smoke_cli.rs",
            ]
        );
        assert_eq!(
            supported_editor_pack.coverage_filters,
            vec!["supported_editor_inline_smoke", "semantic_inline_receipts"]
        );
        assert!(supported_editor_pack.commands.iter().any(|command| {
            command
                == "cargo test -p xtask --test supported_editor_inline_smoke_cli --profile agent --locked -- --nocapture"
        }));
        Ok(())
    }

    #[test]
    fn ci_route_coverage_proof_pack_receipts_reports_unknown_selector() -> Result<()> {
        let selector = vec!["patch-coverage-missing-pack".to_string()];
        let Err(error) = coverage_proof_pack_receipts(&selector) else {
            bail!("unknown coverage selector should fail");
        };
        assert_eq!(
            error.to_string(),
            "coverage pack `patch-coverage-missing-pack` is missing from .ci/coverage-packs.toml"
        );
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_rejects_empty_pack_id() -> Result<()> {
        let Err(error) = parse_coverage_pack_manifest(
            r#"
                [[pack]]
                id = " "
                files = ["xtask/src/tasks/ci_route.rs"]
                commands = ["cargo test -p xtask ci_route"]
                coverage_filters = ["ci_route"]
            "#,
        ) else {
            bail!("empty coverage pack id should fail");
        };
        assert_eq!(error.to_string(), "coverage pack id must not be empty");
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_rejects_empty_file_list() -> Result<()> {
        let Err(error) = parse_coverage_pack_manifest(
            r#"
                [[pack]]
                id = "patch-coverage-ci-route"
                files = []
                commands = ["cargo test -p xtask ci_route"]
                coverage_filters = ["ci_route"]
            "#,
        ) else {
            bail!("coverage pack without files should fail");
        };
        assert_eq!(
            error.to_string(),
            "coverage pack `patch-coverage-ci-route` must list at least one file"
        );
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_rejects_empty_command_list() -> Result<()> {
        let Err(error) = parse_coverage_pack_manifest(
            r#"
                [[pack]]
                id = "patch-coverage-ci-route"
                files = ["xtask/src/tasks/ci_route.rs"]
                commands = []
                coverage_filters = ["ci_route"]
            "#,
        ) else {
            bail!("coverage pack without commands should fail");
        };
        assert_eq!(
            error.to_string(),
            "coverage pack `patch-coverage-ci-route` must list at least one command"
        );
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_rejects_empty_coverage_filter_list() -> Result<()> {
        let Err(error) = parse_coverage_pack_manifest(
            r#"
                [[pack]]
                id = "patch-coverage-ci-route"
                files = ["xtask/src/tasks/ci_route.rs"]
                commands = ["cargo test -p xtask ci_route"]
                coverage_filters = []
            "#,
        ) else {
            bail!("coverage pack without filters should fail");
        };
        assert_eq!(
            error.to_string(),
            "coverage pack `patch-coverage-ci-route` must list at least one coverage filter"
        );
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_rejects_duplicate_pack_id() -> Result<()> {
        let Err(error) = parse_coverage_pack_manifest(
            r#"
                [[pack]]
                id = "patch-coverage-ci-route"
                files = ["xtask/src/tasks/ci_route.rs"]
                commands = ["cargo test -p xtask ci_route"]
                coverage_filters = ["ci_route"]

                [[pack]]
                id = "patch-coverage-ci-route"
                files = ["xtask/tests/ci_route_cli.rs"]
                commands = ["cargo test -p xtask --test ci_route_cli"]
                coverage_filters = ["ci_route_cli"]
            "#,
        ) else {
            bail!("duplicate coverage pack id should fail");
        };
        assert_eq!(error.to_string(), "duplicate coverage pack id `patch-coverage-ci-route`");
        Ok(())
    }

    #[test]
    fn ci_route_coverage_pack_manifest_lists_every_route_selector() -> Result<()> {
        let manifest = coverage_pack_manifest()?;
        let manifest_ids: Vec<&str> = manifest.pack.iter().map(|pack| pack.id.as_str()).collect();

        assert_eq!(
            manifest_ids,
            vec![
                "patch-coverage-xtask-supported-editor-inline-smoke",
                "patch-coverage-xtask-semantic-inline",
                "patch-coverage-inline-provider-core",
                "patch-coverage-inline-ux-fixtures",
                "patch-coverage-xtask-inline-quality",
                "patch-coverage-completion-core",
                "patch-coverage-ux-scenario",
                "patch-coverage-ci-policy",
                "patch-coverage-ci-route",
                "patch-coverage-ci-actuals",
                "patch-coverage-ripr-summary",
                "patch-coverage-learned-estimate",
                "patch-coverage-risk-packs-validator",
                "patch-coverage-gate-lane-mapping",
                "patch-coverage-trust-lanes-validator",
                "patch-coverage-receipts-junit",
                "patch-coverage-core-package-validator",
                "patch-coverage-aggregate-lane-history",
                "patch-coverage-pr-plan",
                "patch-coverage-pr-overlap",
                "patch-coverage-control-plane-lock",
                "patch-coverage-agent-preflight",
                "patch-coverage-ci-audit-workflows-shim",
                "patch-coverage-doc-claims-shim",
                "patch-coverage-features-invariants-shim",
                "patch-coverage-debt-report-shim",
                "patch-coverage-preflight-wrapper",
                "patch-coverage-install-githooks-wrapper",
                "patch-coverage-e2e-gate-wrapper",
                "patch-coverage-execute-gate-wrapper",
                "patch-coverage-run-gates-wrapper",
                "patch-coverage-gate-local-wrapper",
                "patch-coverage-list-gates-wrapper",
                "patch-coverage-forbid-fatal-constructs-wrapper",
                "patch-coverage-dead-code-wrapper",
                "patch-coverage-check-toolchain-wrapper",
                "patch-coverage-devex-doctor-wrapper",
                "patch-coverage-devex-targeted-checks-wrapper",
                "patch-coverage-lsp-cancellation-wrapper",
                "patch-coverage-test-capped-wrapper",
                "patch-coverage-test-e2e-capped-wrapper",
                "patch-coverage-ci-cost-monitor-wrapper",
                "patch-coverage-build-timing-receipt-wrapper",
                "patch-coverage-compare-build-timing-wrapper",
                "patch-coverage-baseline-script",
                "patch-coverage-update-baseline-script",
                "patch-coverage-generate-receipt-script",
                "patch-coverage-quick-receipts-wrapper",
                "patch-coverage-publish-receipts-wrapper",
                "patch-coverage-generate-badges-wrapper",
                "patch-coverage-ignored-test-count-wrapper",
                "patch-coverage-swarm-summary-wrapper",
                "patch-coverage-workspace-exclusions-wrapper",
                "patch-coverage-worktree-cleanup-wrapper",
                "patch-coverage-cleanup-completed-worktrees",
                "patch-coverage-clean-tmp-targets",
                "patch-coverage-swarm-cleanup",
                "patch-coverage-pre-merge-check",
                "patch-coverage-rust-focused",
            ]
        );
        let route_selectors = [
            "patch-coverage-xtask-semantic-inline",
            "patch-coverage-xtask-supported-editor-inline-smoke",
            "patch-coverage-inline-provider-core",
            "patch-coverage-inline-ux-fixtures",
            "patch-coverage-xtask-inline-quality",
            "patch-coverage-completion-core",
            "patch-coverage-ux-scenario",
            "patch-coverage-ci-policy",
            "patch-coverage-ci-route",
            "patch-coverage-ci-actuals",
            "patch-coverage-ripr-summary",
            "patch-coverage-learned-estimate",
            "patch-coverage-risk-packs-validator",
            "patch-coverage-gate-lane-mapping",
            "patch-coverage-trust-lanes-validator",
            "patch-coverage-receipts-junit",
            "patch-coverage-core-package-validator",
            "patch-coverage-aggregate-lane-history",
            "patch-coverage-pr-plan",
            "patch-coverage-pr-overlap",
            "patch-coverage-control-plane-lock",
            "patch-coverage-agent-preflight",
            "patch-coverage-ci-audit-workflows-shim",
            "patch-coverage-doc-claims-shim",
            "patch-coverage-preflight-wrapper",
            "patch-coverage-install-githooks-wrapper",
            "patch-coverage-e2e-gate-wrapper",
            "patch-coverage-execute-gate-wrapper",
            "patch-coverage-run-gates-wrapper",
            "patch-coverage-gate-local-wrapper",
            "patch-coverage-list-gates-wrapper",
            "patch-coverage-forbid-fatal-constructs-wrapper",
            "patch-coverage-dead-code-wrapper",
            "patch-coverage-check-toolchain-wrapper",
            "patch-coverage-devex-doctor-wrapper",
            "patch-coverage-devex-targeted-checks-wrapper",
            "patch-coverage-lsp-cancellation-wrapper",
            "patch-coverage-test-capped-wrapper",
            "patch-coverage-test-e2e-capped-wrapper",
            "patch-coverage-ci-cost-monitor-wrapper",
            "patch-coverage-build-timing-receipt-wrapper",
            "patch-coverage-compare-build-timing-wrapper",
            "patch-coverage-baseline-script",
            "patch-coverage-update-baseline-script",
            "patch-coverage-generate-receipt-script",
            "patch-coverage-quick-receipts-wrapper",
            "patch-coverage-publish-receipts-wrapper",
            "patch-coverage-generate-badges-wrapper",
            "patch-coverage-ignored-test-count-wrapper",
            "patch-coverage-swarm-summary-wrapper",
            "patch-coverage-workspace-exclusions-wrapper",
            "patch-coverage-worktree-cleanup-wrapper",
            "patch-coverage-cleanup-completed-worktrees",
            "patch-coverage-clean-tmp-targets",
            "patch-coverage-swarm-cleanup",
            "patch-coverage-pre-merge-check",
            "patch-coverage-rust-focused",
        ];
        let changed_files = vec![
            "xtask/src/tasks/semantic_inline_receipts.rs".to_string(),
            "xtask/src/tasks/supported_editor_inline_smoke.rs".to_string(),
            "crates/perl-lsp-rs-core/src/providers/inline_completion/engine.rs".to_string(),
            "crates/perl-lsp-rs-core/tests/inline_completion_ux_fixtures.rs".to_string(),
            "xtask/src/tasks/inline_completion_quality.rs".to_string(),
            "crates/perl-lsp-rs-core/src/providers/completion/completion/import_map/used_modules.rs"
                .to_string(),
            "crates/perl-parser/src/lib.rs".to_string(),
        ];
        let (selected, skipped, proof_packs) = coverage_proof_pack_selection(
            &route_selectors.iter().map(|selector| (*selector).to_string()).collect::<Vec<_>>(),
            &changed_files,
        )?;
        assert_eq!(
            selected,
            vec![
                "patch-coverage-xtask-semantic-inline",
                "patch-coverage-xtask-supported-editor-inline-smoke",
                "patch-coverage-inline-provider-core",
                "patch-coverage-completion-core",
                "patch-coverage-rust-focused",
            ]
        );
        assert_eq!(
            skipped.get("patch-coverage-xtask-inline-quality").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ci-policy").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ci-route").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ci-actuals").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ripr-summary").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-learned-estimate").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-risk-packs-validator").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-gate-lane-mapping").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-trust-lanes-validator").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-receipts-junit").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-core-package-validator").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-aggregate-lane-history").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-pr-plan").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-pr-overlap").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-control-plane-lock").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-agent-preflight").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ci-audit-workflows-shim").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-doc-claims-shim").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-preflight-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-install-githooks-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-e2e-gate-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-list-gates-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-forbid-fatal-constructs-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-dead-code-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-check-toolchain-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-lsp-cancellation-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-test-capped-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-test-e2e-capped-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ci-cost-monitor-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-build-timing-receipt-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-compare-build-timing-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-baseline-script").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-update-baseline-script").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-generate-receipt-script").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-quick-receipts-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-publish-receipts-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-generate-badges-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ignored-test-count-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-swarm-summary-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-workspace-exclusions-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-worktree-cleanup-wrapper").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-cleanup-completed-worktrees").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-clean-tmp-targets").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-swarm-cleanup").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-pre-merge-check").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-ux-scenario").map(String::as_str),
            Some(NON_SOURCE_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            skipped.get("patch-coverage-inline-ux-fixtures").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        let inline_provider_pack = proof_packs
            .iter()
            .find(|pack| pack.id == "patch-coverage-inline-provider-core")
            .ok_or_else(|| eyre!("missing inline provider coverage pack"))?;
        assert!(
            inline_provider_pack
                .files
                .iter()
                .any(|file| { file == "crates/perl-lsp-rs-core/src/providers/inline_completion/" })
        );
        assert!(
            !inline_provider_pack
                .commands
                .iter()
                .any(|command| { command.contains("inline-completion-quality") })
        );
        assert!(
            !proof_packs.iter().any(|pack| { pack.id == "patch-coverage-xtask-inline-quality" })
        );
        let completion_core_pack = proof_packs
            .iter()
            .find(|pack| pack.id == "patch-coverage-completion-core")
            .ok_or_else(|| eyre!("missing completion core coverage pack"))?;
        assert!(
            completion_core_pack
                .files
                .iter()
                .any(|file| { file == "crates/perl-lsp-rs-core/src/providers/completion/" })
        );
        assert!(
            completion_core_pack
                .commands
                .iter()
                .any(|command| { command.contains("completion::completion") })
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_splits_inline_completion_receipt_files_by_surface() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec![
                "crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs".to_string(),
                "crates/perl-lsp-rs-core/tests/inline_completion_ux_fixtures.rs".to_string(),
                "xtask/src/tasks/inline_completion_quality.rs".to_string(),
            ],
        )?;

        assert_eq!(
            receipt.changed_surfaces,
            vec!["inline-core", "inline-ux-fixtures", "xtask-inline-completion-quality"]
        );
        assert!(proof_pack_ids(&receipt).contains(&"inline-core"));
        assert!(proof_pack_ids(&receipt).contains(&"inline-ux-fixtures"));
        assert!(proof_pack_ids(&receipt).contains(&"xtask-inline-completion-quality"));
        assert!(!proof_pack_ids(&receipt).contains(&"rust-focused"));
        assert_eq!(receipt.coverage_pack_selector, vec!["patch-coverage-inline-provider-core"]);
        assert_eq!(
            receipt.coverage_proof_packs.iter().map(|pack| pack.id.as_str()).collect::<Vec<_>>(),
            vec!["patch-coverage-inline-provider-core"]
        );
        assert_eq!(
            receipt.skipped_by_policy.get("patch-coverage-inline-ux-fixtures").map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-xtask-inline-quality")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "xtask-inline-completion-quality"
                && pack.commands.iter().any(|command| command.contains("inline-completion-quality"))
        }));
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_inline_provider_to_provider_coverage_pack_only() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["inline-core"]);
        assert_eq!(receipt.coverage_pack_selector, vec!["patch-coverage-inline-provider-core"]);
        assert!(receipt.coverage_proof_packs.iter().flat_map(|pack| pack.commands.iter()).any(
            |command| {
                command
                    == "cargo test -p perl-lsp-rs-core --lib --profile agent --locked inline_completion -- --nocapture"
            }
        ));
        assert!(
            !receipt
                .coverage_proof_packs
                .iter()
                .flat_map(|pack| pack.commands.iter())
                .any(|command| command.contains("inline-completion-quality"))
        );
        Ok(())
    }

    #[test]
    fn ci_route_receipt_maps_inline_quality_to_focused_non_lcov_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/src/tasks/inline_completion_quality.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["xtask-inline-completion-quality"]);
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-xtask-inline-quality")
                .map(String::as_str),
            Some(NON_LCOV_COVERAGE_SKIP_REASON)
        );
        assert!(receipt.required_proof_packs.iter().any(|pack| {
            pack.id == "xtask-inline-completion-quality"
                && pack.commands.iter().any(|command| command.contains("inline-completion-quality"))
        }));
        Ok(())
    }

    #[test]
    fn ci_route_receipt_skips_lcov_pack_when_only_matching_test_file_changed() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/tests/semantic_inline_receipts_cli.rs".to_string()],
        )?;

        assert_eq!(receipt.changed_surfaces, vec!["xtask-semantic-inline-receipts"]);
        assert!(proof_pack_ids(&receipt).contains(&"xtask-semantic-inline-receipts"));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert!(receipt.coverage_proof_packs.is_empty());
        assert_eq!(
            receipt
                .skipped_by_policy
                .get("patch-coverage-xtask-semantic-inline")
                .map(String::as_str),
            Some(NON_SOURCE_LCOV_COVERAGE_SKIP_REASON)
        );
        Ok(())
    }

    #[test]
    fn ci_route_summary_reports_docs_only_without_coverage_packs() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["docs/release notes.md".to_string()])?;
        let summary = render_summary(
            Path::new("target/receipts/ci route.json"),
            Path::new("target/receipts/ci route.md"),
            &receipt,
        );

        assert!(summary.contains("## Coverage Proof Packs"));
        assert!(summary.contains("- none"));
        assert!(summary.contains("`docs/release notes.md`"));
        assert!(summary.contains("`codecov-patch-95`: docs-only change"));
        assert!(
            summary.contains(
                "rtk cargo xtask ci route --base origin/main --head HEAD --receipt 'target/receipts/ci route.json' --summary 'target/receipts/ci route.md' --changed-file 'docs/release notes.md'"
            )
        );
        Ok(())
    }

    #[test]
    fn ci_route_command_writes_receipt_from_explicit_changed_files() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("ci-route.json");
        let summary_path = temp.path().join("ci-route.md");

        run(CiRouteArgs {
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
            receipt: receipt_path.clone(),
            summary: summary_path.clone(),
            changed_files: vec!["xtask\\src\\tasks\\supported_editor_inline_smoke.rs".to_string()],
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt_path)?)?;
        assert_eq!(
            value
                .get("schema_version")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing schema_version"))?,
            "ci-route.v1"
        );
        assert_eq!(
            value
                .pointer("/changed_files/0")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing changed file"))?,
            "xtask/src/tasks/supported_editor_inline_smoke.rs"
        );
        assert_eq!(
            value
                .pointer("/coverage_pack_selector/0")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing coverage pack"))?,
            "patch-coverage-xtask-supported-editor-inline-smoke"
        );
        let summary = fs::read_to_string(summary_path)?;
        assert!(summary.contains("# CI Route Proof Packet"));
        assert!(summary.contains("patch-coverage-xtask-supported-editor-inline-smoke"));
        assert!(summary.contains("supported_editor_inline_smoke"));
        assert!(summary.contains("rtk cargo xtask ci route --base origin/main --head HEAD"));
        assert!(
            summary.contains("--changed-file xtask/src/tasks/supported_editor_inline_smoke.rs")
        );
        Ok(())
    }

    fn proof_pack_ids(receipt: &CiRouteReceipt) -> Vec<&str> {
        receipt.required_proof_packs.iter().map(|pack| pack.id.as_str()).collect()
    }
}
