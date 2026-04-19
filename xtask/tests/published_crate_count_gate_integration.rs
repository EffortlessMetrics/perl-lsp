//! Integration tests for the `published_crate_count` gate in `.ci/gate-policy.yaml`.
//!
//! These tests verify that the `published_crate_count` ratchet gate is properly
//! integrated into the CI gate policy:
//! - Gate exists in the `gates` list with correct configuration
//! - Gate is assigned to the `merge_gate` tier
//! - Gate is included in the `ci-gate` job_mapping
//!
//! The gate prevents regression of the published crate count after the
//! microcrate collapse (ADR-0041). See ADR-0043 and issue #4416.
//!
//! NOTE: These are RED tests - they define what correct behavior looks like.
//! They FAIL until the gate is properly integrated into gate-policy.yaml.

use serde_yaml_ng::Value;
use std::path::PathBuf;

const GATE_NAME: &str = "published_crate_count";
const EXPECTED_TIER: &str = "merge_gate";
const EXPECTED_COMMAND: &str = "cargo xtask published-crate-count";
const EXPECTED_TIMEOUT: u64 = 30;

/// Get the project root (parent of xtask crate directory).
fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

/// Load and parse the gate policy from `.ci/gate-policy.yaml`.
fn load_gate_policy_yaml() -> Value {
    let root = project_root();
    let path = root.join(".ci/gate-policy.yaml");
    let content = std::fs::read_to_string(&path)
        .expect(&format!("Failed to read {:?} - file must exist", path));
    serde_yaml_ng::from_str(&content).expect("Failed to parse gate-policy.yaml")
}

fn find_gate<'a>(gates: &'a Vec<Value>, name: &str) -> Option<&'a Value> {
    gates.iter().find(|g| {
        g.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == name)
            .unwrap_or(false)
    })
}

/// Test that `published_crate_count` gate exists in gate-policy.yaml.
///
/// This is the primary integration test - it verifies the gate is defined
/// at all. Without this, the ratchet cannot function in CI.
#[test]
fn published_crate_count_gate_exists_in_policy() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found in gate-policy.yaml")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME);

    assert!(
        gate.is_some(),
        "published_crate_count gate not found in .ci/gate-policy.yaml.\n\
         Expected gate named '{}' in the gates list.\n\
         Gate is implemented in xtask/src/tasks/count_ratchet.rs and wired\n\
         in justfile as 'ci-published-crate-count', but is missing from\n\
         gate-policy.yaml - this must be added for CI integration.",
        GATE_NAME
    );
}

/// Test that `published_crate_count` gate is in the merge_gate tier.
#[test]
fn published_crate_count_gate_is_in_merge_gate_tier() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let tier = gate.get("tier")
        .and_then(|t| t.as_str())
        .expect("gate must have tier field");

    assert_eq!(
        tier, EXPECTED_TIER,
        "published_crate_count gate must be in '{}' tier, found '{}'.\n\
         The ratchet gate should run before merge, not on every PR push.\n\
         Adjust the tier assignment in gate-policy.yaml.",
        EXPECTED_TIER, tier
    );
}

/// Test that `published_crate_count` gate has the correct command.
#[test]
fn published_crate_count_gate_has_correct_command() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let command = gate.get("command")
        .and_then(|c| c.as_str())
        .expect("gate must have command field");

    assert_eq!(
        command, EXPECTED_COMMAND,
        "published_crate_count gate command must be '{}', found '{}'.\n\
         The command must invoke the xtask subcommand directly.",
        EXPECTED_COMMAND, command
    );
}

/// Test that `published_crate_count` gate has quarantine enabled.
///
/// Until the microcrate collapse completes (~30-31 crates from current 81),
/// the gate should be in quarantine mode to avoid blocking PRs during
/// the transition period.
#[test]
fn published_crate_count_gate_has_quarantine_enabled() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let quarantine = gate.get("quarantine")
        .and_then(|q| q.as_bool())
        .unwrap_or(false); // defaults to false if not present

    assert!(
        quarantine,
        "published_crate_count gate must have quarantine: true.\n\
         Current published count is 81, target is ~30-31 (per ADR-0041).\n\
         Until collapse completes, the gate would fail every PR.\n\
         Set quarantine: true until collapse reaches target."
    );
}

/// Test that `published_crate_count` gate has appropriate timeout.
#[test]
fn published_crate_count_gate_has_appropriate_timeout() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let timeout = gate.get("timeout_seconds")
        .and_then(|t| t.as_i64())
        .expect("gate must have timeout_seconds field");

    assert_eq!(
        timeout as u64, EXPECTED_TIMEOUT,
        "published_crate_count gate timeout must be {} seconds, found {}.\n\
         The gate should complete quickly (reads cargo metadata + file I/O).",
        EXPECTED_TIMEOUT, timeout
    );
}

/// Test that `published_crate_count` gate has required: true.
#[test]
fn published_crate_count_gate_is_required() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let required = gate.get("required")
        .and_then(|r| r.as_bool())
        .unwrap_or(true); // defaults to true

    assert!(
        required,
        "published_crate_count gate must be required: true.\n\
         A failed ratchet gate should block merge."
    );
}

/// Test that `published_crate_count` gate has ratchet-related tags.
#[test]
fn published_crate_count_gate_has_ratchet_tags() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let gate = find_gate(gates, GATE_NAME)
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    let tags = gate.get("tags")
        .and_then(|t| t.as_sequence())
        .expect("gate must have tags field");

    let tag_names: Vec<&str> = tags.iter()
        .filter_map(|t| t.as_str())
        .collect();

    assert!(
        tag_names.contains(&"ratchet"),
        "published_crate_count gate must have 'ratchet' tag.\n\
         Tags help categorize gates in receipts and dashboards.\n\
         Found tags: {:?}",
        tag_names
    );

    assert!(
        tag_names.contains(&"microcrate"),
        "published_crate_count gate must have 'microcrate' tag.\n\
         The gate guards against microcrate collapse regression.\n\
         Found tags: {:?}",
        tag_names
    );

    assert!(
        tag_names.contains(&"collapse"),
        "published_crate_count gate must have 'collapse' tag.\n\
         Marks this as related to the microcrate collapse effort.\n\
         Found tags: {:?}",
        tag_names
    );
}

/// Test that `published_crate_count` gate is in the ci-gate job_mapping.
///
/// The job_mapping defines which gates run in which CI jobs.
/// The gate must be listed in workflow_integration.job_mapping.ci-gate.gates
/// to actually run in CI.
#[test]
fn published_crate_count_gate_is_in_ci_gate_job_mapping() {
    let policy = load_gate_policy_yaml();

    let workflow_integration = policy.get("workflow_integration")
        .expect("workflow_integration section not found in gate-policy.yaml");

    let job_mapping = workflow_integration.get("job_mapping")
        .expect("job_mapping not found in workflow_integration section");

    let ci_gate = job_mapping.get("ci-gate")
        .expect("ci-gate job not found in job_mapping");

    let gates = ci_gate.get("gates")
        .expect("gates list not found in ci-gate job")
        .as_sequence()
        .expect("ci-gate.gates must be a sequence");

    let gate_names: Vec<&str> = gates
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    assert!(
        gate_names.contains(&GATE_NAME),
        "published_crate_count gate not found in workflow_integration.job_mapping.ci-gate.gates.\n\
         Found gates: {:?}\n\
         The gate must be listed in the ci-gate job mapping to run in CI.",
        gate_names
    );
}

/// Test that `nested_lock_check` gate precedes `published_crate_count` in policy.
#[test]
fn published_crate_count_gate_placement_in_merge_gate_tier() {
    let policy = load_gate_policy_yaml();

    let gates = policy.get("gates")
        .expect("gates section not found")
        .as_sequence()
        .expect("gates must be a sequence");

    let nested_lock_idx = gates.iter()
        .position(|g| {
            g.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == "nested_lock_check")
                .unwrap_or(false)
        })
        .expect("nested_lock_check gate must exist");

    let published_crate_idx = gates.iter()
        .position(|g| {
            g.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == GATE_NAME)
                .unwrap_or(false)
        })
        .expect("published_crate_count gate must exist - see test_gate_exists_in_policy");

    // published_crate_count should come AFTER nested_lock_check in the gates list
    assert!(
        published_crate_idx > nested_lock_idx,
        "published_crate_count gate should be placed after nested_lock_check gate.\n\
         Expected: nested_lock_check (index {}) < published_crate_count (index {})\n\
         Gate order in the policy should match tier organization.",
        nested_lock_idx, published_crate_idx
    );
}
