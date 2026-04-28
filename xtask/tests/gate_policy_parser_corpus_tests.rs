//! Guardrails for parser/corpus gate policy wiring.
//!
//! Ensures merge-blocking behavior follows `.ci/gate-policy.yaml` (the CI gate
//! source of truth) and that legacy registry metadata cannot accidentally make
//! CPAN/parser ratchets block ordinary PRs.

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

fn gate_section<'a>(content: &'a str, gate_name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    let marker = format!("name: {gate_name}");
    let start = content
        .find(&marker)
        .ok_or_else(|| format!("missing gate in policy: {gate_name}"))?;
    let section = &content[start..];
    let end = section[1..].find("\n  - name:").map(|i| i + 1).unwrap_or(section.len());
    Ok(&section[..end])
}

fn registry_gate_section<'a>(content: &'a str, gate_id: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    let marker = format!("id = \"{gate_id}\"");
    let start = content
        .find(&marker)
        .ok_or_else(|| format!("missing gate in registry: {gate_id}"))?;
    let section = &content[start..];
    let end = section[1..].find("\n[[gate]]").map(|i| i + 1).unwrap_or(section.len());
    Ok(&section[..end])
}

#[test]
fn pr_policy_keeps_common_corpus_required_and_cpan_non_blocking() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let content = fs::read_to_string(root.join(".ci/gate-policy.yaml"))?;

    let common = gate_section(&content, "common_corpus_clean")?;
    assert!(common.contains("tier: merge_gate"));
    assert!(common.contains("required: true"));

    let parser_ratchet = gate_section(&content, "parser_corpus_ratchet")?;
    assert!(parser_ratchet.contains("tier: merge_gate"));
    assert!(parser_ratchet.contains("required: false"));

    let cpan_ratchet = gate_section(&content, "cpan_corpus_ratchet")?;
    assert!(cpan_ratchet.contains("required: false"));
    assert!(
        !cpan_ratchet.contains("tier: pr_fast"),
        "cpan_corpus_ratchet must never be a pr_fast gate"
    );

    Ok(())
}

#[test]
fn legacy_registry_cannot_reintroduce_pr_blocking_for_corpus_ratchets() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let content = fs::read_to_string(root.join(".ci/GATE_REGISTRY.toml"))?;

    for gate_id in ["parser-corpus-ratchet", "cpan-corpus-ratchet", "parser-audit-closeout"] {
        let section = registry_gate_section(&content, gate_id)?;
        assert!(
            section.contains("blocking = false"),
            "legacy registry gate must stay non-blocking: {gate_id}\n{section}"
        );
    }

    Ok(())
}

#[test]
fn merge_gate_workflow_uses_xtask_gate_policy_runner() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let content = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;

    assert!(content.contains("Run full ci-gate with receipts"));
    assert!(content.contains("run: just gates"));
    assert!(
        !content.contains("GATE_REGISTRY.toml"),
        "merge-gate workflow should not read legacy GATE_REGISTRY.toml directly"
    );

    Ok(())
}
