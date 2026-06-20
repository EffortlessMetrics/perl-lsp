//! RED TDD tests for issue #1469: Static checks required on PRs.
//!
//! These tests verify that:
//! - compile_all_targets is in pr_fast tier (moved from merge_gate)
//! - clippy_full is in pr_fast tier (moved from merge_gate)
//! - unit_routed_full gate exists in pr_fast tier (new gate)
//! - unit_routed_full uses --tests (not --lib) to catch integration test runtime failures
//! - No duplicate gate definitions across tiers
//! - GitHub Actions workflow matrix includes these gates on PRs

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use perl_tdd_support::{must, must_some};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GatePolicyDoc {
    gates: Vec<PolicyGate>,
}

#[derive(Debug, Deserialize)]
struct PolicyGate {
    name: String,
    tier: String,
    #[serde(default = "default_true")]
    required: bool,
    #[serde(default)]
    command: String,
}

fn default_true() -> bool {
    true
}

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

/// TEST: compile_all_targets is in pr_fast tier (red: currently in merge_gate)
#[test]
fn gate_compile_all_targets_moved_to_pr_fast() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let gate = must_some(gates.get("compile_all_targets"));

    assert_eq!(
        gate.tier, "pr_fast",
        "compile_all_targets must be in pr_fast tier (currently in merge_gate, needs move)"
    );
    assert!(gate.required, "compile_all_targets must be required on PRs");

    Ok(())
}

/// TEST: clippy_full is in pr_fast tier (red: currently in merge_gate)
#[test]
fn gate_clippy_full_moved_to_pr_fast() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let gate = must_some(gates.get("clippy_full"));

    assert_eq!(
        gate.tier, "pr_fast",
        "clippy_full must be in pr_fast tier (currently in merge_gate, needs move)"
    );
    assert!(gate.required, "clippy_full must be required on PRs");

    Ok(())
}

/// TEST: unit_routed_full gate exists in pr_fast tier (red: doesn't exist yet)
#[test]
fn gate_unit_routed_full_added_to_pr_fast() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let gate = must_some(gates.get("unit_routed_full"));

    assert_eq!(
        gate.tier, "pr_fast",
        "unit_routed_full must be in pr_fast tier for required PR test gate"
    );
    assert!(gate.required, "unit_routed_full must be required on PRs");

    Ok(())
}

/// TEST: unit_routed_full uses --tests (not --lib) to catch integration test runtime failures.
///
/// The motivating incident for #1469 was `all_kind_names_contains_every_variant` in
/// `crates/perl-ast/tests/` — a runtime assertion failure (not a compile error). That test
/// lives in the `tests/` integration-test directory and is NOT reachable by `--lib`.
/// Using `--tests` ensures the routed gate actually executes integration tests.
#[test]
fn gate_unit_routed_full_uses_tests_flag_not_lib() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let gate = must_some(gates.get("unit_routed_full"));

    assert!(
        gate.command.contains("--tests"),
        "unit_routed_full must use --tests (not --lib) to run integration tests from tests/ \
         directories; --lib only runs lib.rs inline tests and misses runtime assertion failures \
         like the 69≠70 variant-count check that motivated #1469. \
         Current command: {}",
        gate.command
    );
    assert!(
        !gate.command.contains("--lib"),
        "unit_routed_full must NOT use --lib (use --tests to run integration tests); \
         current command: {}",
        gate.command
    );

    Ok(())
}

/// TEST: All five gates are in pr_fast tier (compile_all_targets, clippy_full, unit_routed_full,
/// fmt, check_conflict_markers)
#[test]
fn pr_fast_tier_includes_all_required_static_checks() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    for gate_name in
        ["fmt", "check_conflict_markers", "compile_all_targets", "clippy_full", "unit_routed_full"]
    {
        let gate = must_some(gates.get(gate_name));
        assert_eq!(gate.tier, "pr_fast", "{gate_name} must be in pr_fast tier (red TDD for #1469)");
        assert!(gate.required, "{gate_name} must be required on PRs");
    }

    Ok(())
}

/// TEST: No duplicate gate definitions (each gate name appears exactly once)
#[test]
fn no_duplicate_gate_definitions_across_tiers() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    // Count occurrences of each gate name
    let mut gate_counts = HashMap::new();
    for gate in parsed.gates {
        *gate_counts.entry(gate.name).or_insert(0) += 1;
    }

    // Check for duplicates (gates that appear more than once)
    for (gate_name, count) in gate_counts {
        assert_eq!(
            count, 1,
            "Gate '{gate_name}' appears {count} times (must appear exactly once to avoid tier collision)"
        );
    }

    Ok(())
}

/// TEST: compile_all_targets is NOT in merge_gate tier (it was moved to pr_fast)
#[test]
fn gate_compile_all_targets_not_in_merge_gate() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let merge_gate_gates: Vec<_> =
        parsed.gates.iter().filter(|g| g.tier == "merge_gate").map(|g| g.name.clone()).collect();

    assert!(
        !merge_gate_gates.contains(&"compile_all_targets".to_string()),
        "compile_all_targets must NOT be in merge_gate tier (moved to pr_fast, red until builder implements)"
    );

    Ok(())
}

/// TEST: clippy_full is NOT in merge_gate tier (it was moved to pr_fast)
#[test]
fn gate_clippy_full_not_in_merge_gate() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let merge_gate_gates: Vec<_> =
        parsed.gates.iter().filter(|g| g.tier == "merge_gate").map(|g| g.name.clone()).collect();

    assert!(
        !merge_gate_gates.contains(&"clippy_full".to_string()),
        "clippy_full must NOT be in merge_gate tier (moved to pr_fast, red until builder implements)"
    );

    Ok(())
}

/// TEST: GitHub Actions workflow matrix includes compile_all_targets in merge-gate-shards
#[test]
fn ci_workflow_includes_compile_all_targets_in_matrix() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/ci.yml")));

    // Check if compile_all_targets appears in the merge-gate-shards matrix section.
    // We search from the "merge-gate-shards:" label to the next top-level job definition
    // (lines starting with exactly two-space indent followed by a non-space char and ending
    // with ":"). A reliable delimiter is the "merge-gate:" job which immediately follows
    // the shards.
    let merge_gate_shards_start = must_some(workflow.find("merge-gate-shards:"));
    let rest = &workflow[merge_gate_shards_start..];

    // Use the "merge-gate:" aggregate job as the upper boundary of the shards section.
    // Fall back to searching the whole remaining file if not present.
    let next_job = rest.find("\nmerge-gate:").unwrap_or(rest.len());
    let shards_section = &rest[..next_job];

    assert!(
        shards_section.contains("compile_all_targets"),
        "merge-gate-shards matrix must include compile_all_targets (red until builder adds to ci.yml)"
    );

    Ok(())
}

/// TEST: GitHub Actions workflow matrix includes clippy_full in merge-gate-shards
#[test]
fn ci_workflow_includes_clippy_full_in_matrix() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/ci.yml")));

    // Check if clippy_full appears in the merge-gate-shards matrix section.
    let merge_gate_shards_start = must_some(workflow.find("merge-gate-shards:"));
    let rest = &workflow[merge_gate_shards_start..];

    // Use the "merge-gate:" aggregate job as the upper boundary.
    let next_job = rest.find("\nmerge-gate:").unwrap_or(rest.len());
    let shards_section = &rest[..next_job];

    assert!(
        shards_section.contains("clippy_full"),
        "merge-gate-shards matrix must include clippy_full (already present, verify it's not lost)"
    );

    Ok(())
}

/// TEST: unit_routed_full runs on every PR via pr-smoke (--tier pr_fast)
///
/// The gate is in tier pr_fast, so it runs in the pr-smoke job with:
///   cargo xtask gates --tier pr-fast --base origin/main
/// This properly resolves {package_args} for the rust_scoped gate.
///
/// It does NOT appear in merge-gate-shards because that shard uses --gate <name>,
/// which doesn't resolve {package_args} and triggers the guard. Instead,
/// merge-gate runs MergeGate tier which includes pr_fast gates via plan_pr_fast_gates.
#[test]
fn ci_workflow_runs_unit_routed_full_in_pr_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/ci.yml")));

    // Verify that pr-smoke job runs --tier pr-fast
    // which includes all pr_fast gates (including unit_routed_full)
    let has_pr_fast_tier = workflow.contains("gates --tier pr-fast");
    let has_base_origin_main = workflow.contains("--base origin/main");

    assert!(
        has_pr_fast_tier && has_base_origin_main,
        "pr-smoke job must run gates --tier pr-fast --base origin/main \
         to properly resolve package_args for rust_scoped gates like unit_routed_full"
    );
    assert!(
        workflow.contains("timeout-minutes: 50"),
        "pr-smoke job timeout must include the observed heavy routed integration-test path"
    );
    assert!(
        workflow.contains("PR-fast timeout policy: GitHub job 50m, outer runner watchdog 45m"),
        "pr-smoke log message must document the active watchdog policy"
    );
    assert!(
        workflow.contains(
            "timeout --signal=TERM --kill-after=60s 2700s ./target/debug/xtask gates --tier pr-fast --base origin/main --receipt"
        ),
        "pr-smoke inner watchdog must leave enough room for unit_routed_full receipt output"
    );

    // Ensure the routed shard (which was a failed attempt to run unit_routed_full)
    // has been removed
    let has_routed_shard = workflow.contains("- name: routed");
    assert!(
        !has_routed_shard,
        "The routed shard must be removed — unit_routed_full is tier pr_fast \
         and runs correctly in pr-smoke, not in merge-gate-shards"
    );

    Ok(())
}

/// TEST: pr_fast tier has at least 5 gates (the static checks + others)
#[test]
fn pr_fast_tier_has_minimum_gate_count() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let pr_fast_gates: Vec<_> =
        parsed.gates.iter().filter(|g| g.tier == "pr_fast").map(|g| g.name.clone()).collect();

    assert!(
        pr_fast_gates.len() >= 5,
        "pr_fast tier must include at least 5 gates (fmt, check_conflict_markers, compile_all_targets, clippy_full, unit_routed_full); currently has: {pr_fast_gates:?}"
    );

    Ok(())
}
