use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::utils::project_root;

#[derive(Clone, Debug, clap::Subcommand)]
pub enum GatePolicyCommand {
    /// Validate that authoritative gate-policy and legacy registry do not disagree on parser corpus blocking.
    Check,
    /// Print effective gate policy for a profile.
    Effective {
        /// Policy profile to render.
        #[arg(long, value_enum, default_value = "pr")]
        profile: GatePolicyProfile,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum GatePolicyProfile {
    /// Pull-request effective gates (pr_fast + merge_gate).
    Pr,
    /// Nightly effective gates.
    Nightly,
    /// Release effective gates.
    Release,
}

#[derive(Debug, Deserialize)]
struct GatePolicyDoc {
    gates: Vec<GatePolicyGate>,
}

#[derive(Debug, Deserialize)]
struct GatePolicyGate {
    name: String,
    tier: String,
    required: bool,
}

#[derive(Debug, Deserialize)]
struct RegistryDoc {
    gate: Vec<RegistryGate>,
}

#[derive(Debug, Deserialize)]
struct RegistryGate {
    id: String,
    blocking: bool,
}

#[derive(Debug, Clone)]
struct EffectiveGate {
    name: String,
    tier: String,
    required: bool,
}

pub fn run(command: GatePolicyCommand) -> Result<()> {
    match command {
        GatePolicyCommand::Check => check(),
        GatePolicyCommand::Effective { profile } => effective(profile),
    }
}

fn check() -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root.join(".ci/gate-policy.yaml"))?;
    let registry = load_registry(&root.join(".ci/GATE_REGISTRY.toml"))?;

    let effective = effective_gates_for_profile(&policy, &GatePolicyProfile::Pr);
    let effective_map: HashMap<&str, bool> =
        effective.iter().map(|gate| (gate.name.as_str(), gate.required)).collect();

    // Canonical PR corpus policy expectations.
    assert_gate_required(&effective_map, "common_corpus_clean", true)?;
    assert_gate_required(&effective_map, "cpan_corpus_ratchet", false)?;
    assert_gate_required(&effective_map, "parser_corpus_ratchet", false)?;

    let registry_expectations = [
        ("cpan-corpus-ratchet", false),
        ("parser-corpus-ratchet", false),
        ("parser-audit-closeout", false),
    ];

    for (id, expected_blocking) in registry_expectations {
        let gate = registry
            .gate
            .iter()
            .find(|gate| gate.id == id)
            .with_context(|| format!("missing '{id}' in .ci/GATE_REGISTRY.toml"))?;
        if gate.blocking != expected_blocking {
            bail!(
                "registry mismatch for '{id}': blocking={} expected={}",
                gate.blocking,
                expected_blocking
            );
        }
    }

    println!("✅ gate-policy check passed");
    println!("   source of truth: .ci/gate-policy.yaml (consumed by cargo xtask gates)");
    println!("   legacy registry parity: parser/corpus blocking flags aligned");
    Ok(())
}

fn effective(profile: GatePolicyProfile) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root.join(".ci/gate-policy.yaml"))?;
    let gates = effective_gates_for_profile(&policy, &profile);

    println!("profile: {}", profile_label(&profile));
    println!("source_of_truth: .ci/gate-policy.yaml");
    for gate in gates {
        let status = if gate.required { "required" } else { "advisory" };
        println!("- {} [{}] ({})", gate.name, gate.tier, status);
    }

    Ok(())
}

fn assert_gate_required(
    effective_map: &HashMap<&str, bool>,
    gate_name: &str,
    expected_required: bool,
) -> Result<()> {
    let required = effective_map
        .get(gate_name)
        .copied()
        .with_context(|| format!("missing '{gate_name}' in effective PR profile"))?;
    if required != expected_required {
        bail!(
            "effective policy mismatch for '{gate_name}': required={} expected={}",
            required,
            expected_required
        );
    }
    Ok(())
}

fn effective_gates_for_profile(
    policy: &GatePolicyDoc,
    profile: &GatePolicyProfile,
) -> Vec<EffectiveGate> {
    policy
        .gates
        .iter()
        .filter(|gate| match profile {
            GatePolicyProfile::Pr => gate.tier == "pr_fast" || gate.tier == "merge_gate",
            GatePolicyProfile::Nightly => gate.tier == "nightly",
            GatePolicyProfile::Release => gate.tier == "release",
        })
        .map(|gate| EffectiveGate {
            name: gate.name.clone(),
            tier: gate.tier.clone(),
            required: gate.required,
        })
        .collect()
}

fn profile_label(profile: &GatePolicyProfile) -> &'static str {
    match profile {
        GatePolicyProfile::Pr => "pr",
        GatePolicyProfile::Nightly => "nightly",
        GatePolicyProfile::Release => "release",
    }
}

fn load_policy(path: &Path) -> Result<GatePolicyDoc> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy file {}", path.display()))?;
    let policy = serde_yaml_ng::from_str::<GatePolicyDoc>(&content)
        .with_context(|| format!("failed to parse policy file {}", path.display()))?;
    Ok(policy)
}

fn load_registry(path: &Path) -> Result<RegistryDoc> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read registry file {}", path.display()))?;
    let registry = toml::from_str::<RegistryDoc>(&content)
        .with_context(|| format!("failed to parse registry file {}", path.display()))?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_profile_keeps_common_required_and_cpan_advisory() {
        let policy = GatePolicyDoc {
            gates: vec![
                GatePolicyGate {
                    name: "common_corpus_clean".to_string(),
                    tier: "merge_gate".to_string(),
                    required: true,
                },
                GatePolicyGate {
                    name: "cpan_corpus_ratchet".to_string(),
                    tier: "merge_gate".to_string(),
                    required: false,
                },
                GatePolicyGate {
                    name: "fmt".to_string(),
                    tier: "pr_fast".to_string(),
                    required: true,
                },
                GatePolicyGate {
                    name: "nightly_only".to_string(),
                    tier: "nightly".to_string(),
                    required: false,
                },
            ],
        };

        let effective = effective_gates_for_profile(&policy, &GatePolicyProfile::Pr);
        let by_name: HashMap<&str, bool> =
            effective.iter().map(|gate| (gate.name.as_str(), gate.required)).collect();

        assert_eq!(by_name.get("common_corpus_clean"), Some(&true));
        assert_eq!(by_name.get("cpan_corpus_ratchet"), Some(&false));
        assert_eq!(by_name.get("fmt"), Some(&true));
        assert_eq!(by_name.get("nightly_only"), None);
    }
}
