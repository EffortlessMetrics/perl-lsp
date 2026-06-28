use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

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
    timeout_seconds: Option<u64>,
    budgets: Option<GateBudgets>,
    planning: Option<GatePlanning>,
}

#[derive(Debug, Deserialize)]
struct GateBudgets {
    max_duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GatePlanning {
    role: String,
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GateRegistryDoc {
    gate: Vec<RegistryGate>,
}

#[derive(Debug, Deserialize)]
struct RegistryGate {
    id: String,
    #[serde(default)]
    blocking: bool,
}

fn default_true() -> bool {
    true
}

#[test]
fn parser_corpus_pr_policy_is_unambiguous() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let common = gates.get("common_corpus_clean").ok_or("missing common_corpus_clean gate")?;
    let parser = gates.get("parser_corpus_ratchet").ok_or("missing parser_corpus_ratchet gate")?;
    let cpan = gates.get("cpan_corpus_ratchet").ok_or("missing cpan_corpus_ratchet gate")?;

    assert_eq!(common.tier, "merge_gate");
    assert!(common.required, "common_corpus_clean must stay PR-blocking");
    assert!(
        common.timeout_seconds.unwrap_or_default() >= 240,
        "common_corpus_clean timeout must include cold CI xtask startup"
    );
    assert!(
        common.budgets.as_ref().and_then(|budget| budget.max_duration_ms).unwrap_or_default()
            >= 180_000,
        "common_corpus_clean duration budget must reflect CI startup overhead"
    );

    assert_eq!(parser.tier, "merge_gate");
    assert!(!parser.required, "parser_corpus_ratchet must be advisory in PR merge-gate profile");

    assert_eq!(cpan.tier, "merge_gate");
    assert!(!cpan.required, "cpan_corpus_ratchet must never be PR-blocking");

    Ok(())
}

#[test]
fn release_history_pr_gates_have_realistic_timeout_headroom()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    for gate_name in ["release_history", "release_history_check"] {
        let gate = gates.get(gate_name).ok_or_else(|| format!("missing {gate_name} gate"))?;
        assert_eq!(gate.tier, "pr_fast", "{gate_name} must stay in pr-fast");
        assert!(gate.required, "{gate_name} must stay PR-blocking");
        assert!(
            gate.timeout_seconds.unwrap_or_default() >= 120,
            "{gate_name} timeout must include cold or partially-cold xtask startup"
        );
        assert!(
            gate.budgets.as_ref().and_then(|budget| budget.max_duration_ms).unwrap_or_default()
                >= 90_000,
            "{gate_name} duration budget must reflect observed pr-fast runtime"
        );
    }

    Ok(())
}

#[test]
fn conflict_marker_gate_has_local_runtime_headroom() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gate = parsed
        .gates
        .into_iter()
        .find(|gate| gate.name == "check_conflict_markers")
        .ok_or("missing check_conflict_markers gate")?;

    assert_eq!(gate.tier, "pr_fast", "conflict marker scan must stay in pr-fast");
    assert!(gate.required, "conflict marker scan must stay PR-blocking");
    assert!(
        gate.timeout_seconds.unwrap_or_default() >= 120,
        "conflict marker timeout must include Windows and mounted-worktree headroom"
    );
    assert!(
        gate.budgets.as_ref().and_then(|budget| budget.max_duration_ms).unwrap_or_default()
            >= 120_000,
        "conflict marker budget must reflect observed local runtime"
    );

    Ok(())
}

#[test]
fn routed_integration_test_gate_has_cold_ci_headroom() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gate = parsed
        .gates
        .into_iter()
        .find(|gate| gate.name == "unit_routed_full")
        .ok_or("missing unit_routed_full gate")?;

    assert_eq!(gate.tier, "pr_fast", "unit_routed_full must stay in pr-fast");
    assert!(gate.required, "unit_routed_full must stay PR-blocking");
    assert_eq!(
        gate.planning.as_ref().map(|planning| planning.role.as_str()),
        Some("rust_scoped"),
        "unit_routed_full must stay routed to changed Rust packages"
    );
    assert!(
        gate.timeout_seconds.unwrap_or_default() >= 1_500,
        "unit_routed_full timeout must include cold integration-test build headroom"
    );
    assert!(
        gate.budgets.as_ref().and_then(|budget| budget.max_duration_ms).unwrap_or_default()
            >= 1_320_000,
        "unit_routed_full duration budget must reflect observed cold PR-fast runtime"
    );

    Ok(())
}

#[test]
fn inline_completion_contract_scope_stays_on_lsp_crates() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(policy_path)?;
    let parsed: GatePolicyDoc = serde_yaml_ng::from_str(&content)?;

    let gates: HashMap<_, _> =
        parsed.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();

    let contract =
        gates.get("inline_completion_contract").ok_or("missing inline_completion_contract gate")?;
    let contract_planning =
        contract.planning.as_ref().ok_or("inline_completion_contract missing planning")?;

    assert_eq!(contract.tier, "pr_fast");
    assert!(contract.required, "inline_completion_contract must stay PR-blocking");
    assert!(
        contract.timeout_seconds.unwrap_or_default() >= 600,
        "inline_completion_contract timeout must include cold CI compile headroom"
    );
    assert!(
        contract.budgets.as_ref().and_then(|budget| budget.max_duration_ms).unwrap_or_default()
            >= 540_000,
        "inline_completion_contract duration budget must reflect observed cold PR-fast runtime"
    );
    assert_eq!(contract_planning.role, "rust_package_scoped");
    assert_eq!(contract_planning.packages, vec!["perl-lsp-rs", "perl-lsp-rs-core"]);

    let quality = gates
        .get("inline_completion_quality_receipt")
        .ok_or("missing inline_completion_quality_receipt gate")?;
    let quality_planning =
        quality.planning.as_ref().ok_or("inline_completion_quality_receipt missing planning")?;

    assert_eq!(quality.tier, "pr_fast");
    assert!(quality.required, "inline_completion_quality_receipt must stay PR-blocking");
    assert_eq!(quality_planning.role, "rust_package_scoped");
    assert_eq!(quality_planning.packages, vec!["perl-lsp-rs-core", "xtask"]);

    Ok(())
}

#[test]
fn gate_registry_alignment_prevents_stale_parser_wiring() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();

    let policy: GatePolicyDoc =
        serde_yaml_ng::from_str(&fs::read_to_string(root.join(".ci/gate-policy.yaml"))?)?;
    let registry: GateRegistryDoc =
        toml::from_str(&fs::read_to_string(root.join(".ci/GATE_REGISTRY.toml"))?)?;

    let policy_by_name: HashMap<_, _> =
        policy.gates.into_iter().map(|gate| (gate.name.clone(), gate.required)).collect();
    let registry_by_id: HashMap<_, _> =
        registry.gate.into_iter().map(|gate| (gate.id.clone(), gate.blocking)).collect();

    let pairs = [
        ("parser_corpus_ratchet", "parser-corpus-ratchet"),
        ("cpan_corpus_ratchet", "cpan-corpus-ratchet"),
        ("parser_audit_closeout", "parser-audit-closeout"),
    ];

    for (policy_name, registry_id) in pairs {
        let required = policy_by_name
            .get(policy_name)
            .ok_or_else(|| format!("missing policy gate: {policy_name}"))?;
        let blocking = registry_by_id
            .get(registry_id)
            .ok_or_else(|| format!("missing registry gate: {registry_id}"))?;
        assert_eq!(
            required, blocking,
            "gate-policy and gate-registry must agree for {policy_name}/{registry_id}"
        );
    }

    Ok(())
}
