use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::tasks::gates::{GatePolicy, load_policy_for_inspection};
use crate::utils::project_root;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum GatePolicyProfile {
    Pr,
    Nightly,
    Release,
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    #[serde(rename = "gate", default)]
    gates: Vec<RegistryGate>,
}

#[derive(Debug, Deserialize)]
struct RegistryGate {
    id: String,
    blocking: bool,
}

pub fn check() -> Result<()> {
    let root = project_root()?;
    let policy = load_policy_for_inspection(&root.join(".ci/gate-policy.yaml"))?;
    let registry = load_registry(&root.join(".ci/GATE_REGISTRY.toml"))?;

    // CI Gate runs `cargo xtask gates` using `.ci/gate-policy.yaml`.
    // Ensure PR profile cannot be blocked by CPAN/parser ratchet wiring.
    let pr_effective = effective_required_gate_names(&policy, GatePolicyProfile::Pr)?;
    assert_required(&pr_effective, "common_corpus_clean")?;
    assert_not_required(&pr_effective, "cpan_corpus_ratchet")?;
    assert_not_required(&pr_effective, "parser_corpus_ratchet")?;

    // Keep legacy registry aligned for human readers and secondary tooling.
    assert_registry_not_blocking(&registry, "cpan-corpus-ratchet")?;
    assert_registry_not_blocking(&registry, "parser-corpus-ratchet")?;

    println!("✅ Gate policy check passed.");
    println!("   Source of truth: .ci/gate-policy.yaml (used by `cargo xtask gates`).");
    println!("   PR required includes common_corpus_clean, excludes CPAN/parser ratchets.");

    Ok(())
}

pub fn effective(profile: GatePolicyProfile) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy_for_inspection(&root.join(".ci/gate-policy.yaml"))?;
    let required = effective_required_gate_names(&policy, profile)?;
    let advisory = effective_advisory_gate_names(&policy, profile)?;

    println!("Source of truth: .ci/gate-policy.yaml");
    println!("Profile: {}", profile_label(profile));
    println!("Required gates ({}):", required.len());
    for gate in &required {
        println!("  - {gate}");
    }

    println!("Advisory gates ({}):", advisory.len());
    for gate in &advisory {
        println!("  - {gate}");
    }

    Ok(())
}

fn profile_label(profile: GatePolicyProfile) -> &'static str {
    match profile {
        GatePolicyProfile::Pr => "pr",
        GatePolicyProfile::Nightly => "nightly",
        GatePolicyProfile::Release => "release",
    }
}

fn load_registry(path: &Path) -> Result<RegistryFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read gate registry from {}", path.display()))?;
    let registry: RegistryFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse gate registry from {}", path.display()))?;
    Ok(registry)
}

fn effective_required_gate_names(
    policy: &GatePolicy,
    profile: GatePolicyProfile,
) -> Result<Vec<String>> {
    let mut names = effective_gate_names(policy, profile, true)?;
    names.sort();
    Ok(names)
}

fn effective_advisory_gate_names(
    policy: &GatePolicy,
    profile: GatePolicyProfile,
) -> Result<Vec<String>> {
    let mut names = effective_gate_names(policy, profile, false)?;
    names.sort();
    Ok(names)
}

fn effective_gate_names(
    policy: &GatePolicy,
    profile: GatePolicyProfile,
    required: bool,
) -> Result<Vec<String>> {
    let allowed_tiers = match profile {
        GatePolicyProfile::Pr => ["pr_fast", "merge_gate"].as_slice(),
        GatePolicyProfile::Nightly => ["pr_fast", "merge_gate", "nightly"].as_slice(),
        GatePolicyProfile::Release => ["release"].as_slice(),
    };

    for tier in allowed_tiers {
        if !policy.tiers.contains_key(*tier) {
            bail!("Policy missing required tier '{tier}' for profile {}", profile_label(profile));
        }
    }

    Ok(policy
        .gates
        .iter()
        .filter(|gate| allowed_tiers.contains(&gate.tier.as_str()) && gate.required == required)
        .map(|gate| gate.name.clone())
        .collect())
}

fn assert_required(required: &[String], gate_name: &str) -> Result<()> {
    if required.iter().any(|name| name == gate_name) {
        Ok(())
    } else {
        bail!("Gate '{gate_name}' must be required in PR profile")
    }
}

fn assert_not_required(required: &[String], gate_name: &str) -> Result<()> {
    if required.iter().any(|name| name == gate_name) {
        bail!("Gate '{gate_name}' must not be required in PR profile")
    } else {
        Ok(())
    }
}

fn assert_registry_not_blocking(registry: &RegistryFile, gate_id: &str) -> Result<()> {
    let by_id: HashMap<&str, bool> =
        registry.gates.iter().map(|gate| (gate.id.as_str(), gate.blocking)).collect();

    match by_id.get(gate_id) {
        Some(true) => bail!("Registry gate '{gate_id}' must be non-blocking"),
        Some(false) => Ok(()),
        None => bail!("Registry gate '{gate_id}' missing; keep registry aligned with policy"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_enforces_cpan_non_blocking_for_pr_profile() -> Result<()> {
        check()
    }

    #[test]
    fn effective_pr_marks_common_required_and_cpan_advisory() -> Result<()> {
        let root = project_root()?;
        let policy = load_policy_for_inspection(&root.join(".ci/gate-policy.yaml"))?;

        let required = effective_required_gate_names(&policy, GatePolicyProfile::Pr)?;
        let advisory = effective_advisory_gate_names(&policy, GatePolicyProfile::Pr)?;

        assert!(required.iter().any(|name| name == "common_corpus_clean"));
        assert!(!required.iter().any(|name| name == "cpan_corpus_ratchet"));
        assert!(advisory.iter().any(|name| name == "cpan_corpus_ratchet"));
        Ok(())
    }
}
