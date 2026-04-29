use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::utils::project_root;

#[derive(clap::Subcommand, Debug)]
pub enum GatePolicyCommand {
    /// Validate policy invariants and detect stale gate-registry wiring.
    Check,
    /// Show the effective gate policy for a CI profile.
    Effective {
        /// CI profile to evaluate.
        #[arg(long, value_enum, default_value = "pr")]
        profile: GateProfile,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateProfile {
    /// Pull-request profile (pr_fast + merge_gate tiers).
    Pr,
    /// Full merge-gate profile (same tier set as `pr`).
    Merge,
    /// Nightly profile (all tiers, informational depth).
    Nightly,
    /// Release profile.
    Release,
}

#[derive(Debug, Deserialize)]
struct GatePolicyDoc {
    gates: Vec<PolicyGate>,
}

#[derive(Debug, Deserialize, Clone)]
struct PolicyGate {
    name: String,
    tier: String,
    #[serde(default = "default_true")]
    required: bool,
}

#[derive(Debug, Deserialize)]
struct GateRegistryDoc {
    gate: Vec<RegistryGate>,
}

#[derive(Debug, Deserialize, Clone)]
struct RegistryGate {
    id: String,
    #[serde(default)]
    blocking: bool,
}

fn default_true() -> bool {
    true
}

pub fn run(command: GatePolicyCommand) -> Result<()> {
    match command {
        GatePolicyCommand::Check => check(),
        GatePolicyCommand::Effective { profile } => effective(profile),
    }
}

fn check() -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;
    let registry = load_registry(&root)?;

    let policy_by_name: BTreeMap<_, _> =
        policy.gates.into_iter().map(|gate| (gate.name.clone(), gate)).collect();
    let registry_by_id: BTreeMap<_, _> =
        registry.gate.into_iter().map(|gate| (gate.id.clone(), gate)).collect();

    assert_required_flag(&policy_by_name, "common_corpus_clean", true)?;
    assert_required_flag(&policy_by_name, "parser_corpus_ratchet", false)?;
    assert_required_flag(&policy_by_name, "cpan_corpus_ratchet", false)?;

    // Keep YAML policy and legacy TOML registry aligned for corpus gates.
    assert_registry_alignment(
        &policy_by_name,
        &registry_by_id,
        "parser_corpus_ratchet",
        "parser-corpus-ratchet",
    )?;
    assert_registry_alignment(
        &policy_by_name,
        &registry_by_id,
        "cpan_corpus_ratchet",
        "cpan-corpus-ratchet",
    )?;
    assert_registry_alignment(
        &policy_by_name,
        &registry_by_id,
        "parser_audit_closeout",
        "parser-audit-closeout",
    )?;

    println!("gate-policy check passed");
    println!("- common_corpus_clean: required in PR merge-gate profile");
    println!("- parser_corpus_ratchet: advisory (non-blocking)");
    println!("- cpan_corpus_ratchet: advisory (non-blocking)");
    Ok(())
}

fn effective(profile: GateProfile) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;
    let selected_tiers = profile_tiers(profile);

    println!("profile={:?}", profile);
    println!("gate | tier | required | effective_blocking");

    for gate in policy.gates.iter().filter(|gate| selected_tiers.contains(&gate.tier.as_str())) {
        println!("{} | {} | {} | {}", gate.name, gate.tier, gate.required, gate.required);
    }

    Ok(())
}

fn profile_tiers(profile: GateProfile) -> &'static [&'static str] {
    match profile {
        GateProfile::Pr | GateProfile::Merge => &["pr_fast", "merge_gate"],
        GateProfile::Nightly => &["pr_fast", "merge_gate", "nightly"],
        GateProfile::Release => &["release"],
    }
}

fn load_policy(root: &Path) -> Result<GatePolicyDoc> {
    let path = root.join(".ci/gate-policy.yaml");
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_yaml_ng::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn load_registry(root: &Path) -> Result<GateRegistryDoc> {
    let path = root.join(".ci/GATE_REGISTRY.toml");
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn assert_required_flag(
    gates: &BTreeMap<String, PolicyGate>,
    gate: &str,
    expected_required: bool,
) -> Result<()> {
    let policy_gate =
        gates.get(gate).with_context(|| format!("missing gate-policy entry '{gate}'"))?;
    if policy_gate.required != expected_required {
        bail!(
            "gate-policy mismatch for {gate}: expected required={}, got required={}",
            expected_required,
            policy_gate.required
        );
    }
    Ok(())
}

fn assert_registry_alignment(
    policy: &BTreeMap<String, PolicyGate>,
    registry: &BTreeMap<String, RegistryGate>,
    policy_gate_name: &str,
    registry_gate_id: &str,
) -> Result<()> {
    let policy_gate = policy
        .get(policy_gate_name)
        .with_context(|| format!("missing gate-policy entry '{policy_gate_name}'"))?;
    let registry_gate = registry
        .get(registry_gate_id)
        .with_context(|| format!("missing gate-registry entry '{registry_gate_id}'"))?;

    if policy_gate.required != registry_gate.blocking {
        bail!(
            "policy mismatch: .ci/gate-policy.yaml gate '{}' required={} but .ci/GATE_REGISTRY.toml gate '{}' blocking={}",
            policy_gate_name,
            policy_gate.required,
            registry_gate_id,
            registry_gate.blocking
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GateProfile, profile_tiers};

    #[test]
    fn pr_profile_contains_merge_gate_tiers() {
        let tiers = profile_tiers(GateProfile::Pr);
        assert!(tiers.contains(&"pr_fast"));
        assert!(tiers.contains(&"merge_gate"));
    }
}
