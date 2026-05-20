//! Validate the workspace-symbol class promotion registry.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const POLICY_PATH: &str = "policy/workspace-symbol-classes.toml";
const POLICY_NAME: &str = "workspace-symbol-classes";
const REQUIRED_GENERATED_LABEL: &str = "[generated/framework]";
const REQUIRED_SURFACE: &str = "workspace_symbols";

const ALLOWED_STATES: &[&str] = &["partial_live", "generated_label_pilot", "blocked", "deferred"];
const ALLOWED_PROVENANCES: &[&str] =
    &["ExplicitSource", "SourceBackedGenerated", "GeneratedNoSource", "DynamicBoundary", "Unknown"];
const REQUIRED_GENERATED_BLOCKERS: &[&str] = &[
    "generated_no_source",
    "dynamic_boundary",
    "stale_fact",
    "low_confidence",
    "ambiguous_identity",
];

#[derive(Debug, Deserialize)]
struct WorkspaceSymbolClasses {
    schema_version: u32,
    policy: String,
    spec: String,
    support_tiers: String,
    provider_promotion_ledger: String,
    default_state: String,
    default_fallback: String,
    required_generated_label: String,
    receipt_sources: Vec<String>,
    #[serde(default)]
    class: Vec<WorkspaceSymbolClass>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSymbolClass {
    name: String,
    state: String,
    surface: String,
    fact_provenance: String,
    live: bool,
    requires_non_empty_query: bool,
    requires_ready_index: bool,
    requires_high_confidence: bool,
    requires_source_anchor: bool,
    requires_generated_label: bool,
    #[serde(default)]
    label: Option<String>,
    blocks: Vec<String>,
    claim_boundary: String,
}

#[derive(Debug, Deserialize)]
struct ProviderPromotionLedger {
    blocker_registry: Vec<String>,
}

#[derive(Debug)]
struct ValidationStats {
    classes: usize,
    receipts: usize,
    blockers: usize,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate(&root)?;
    println!(
        "workspace symbol class check passed: {} classes, {} receipt sources, {} allowed blockers",
        stats.classes, stats.receipts, stats.blockers
    );
    Ok(())
}

fn validate(root: &Path) -> Result<ValidationStats> {
    let policy: WorkspaceSymbolClasses = read_policy(root, POLICY_PATH)?;
    let provider_ledger =
        read_policy::<ProviderPromotionLedger>(root, &policy.provider_promotion_ledger)?;
    let mut violations = Vec::new();

    validate_policy_shape(root, &policy, &provider_ledger, &mut violations);
    validate_classes(&policy, &provider_ledger, &mut violations);

    if !violations.is_empty() {
        eprintln!("workspace symbol class policy violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("workspace symbol class check failed with {} violation(s)", violations.len());
    }

    Ok(ValidationStats {
        classes: policy.class.len(),
        receipts: policy.receipt_sources.len(),
        blockers: allowed_blockers(&provider_ledger).len(),
    })
}

fn read_policy<T: for<'de> Deserialize<'de>>(root: &Path, rel: &str) -> Result<T> {
    let text = read_text(root, rel)?;
    toml::from_str(&text).with_context(|| format!("failed to parse {rel}"))
}

fn read_text(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

fn validate_policy_shape(
    root: &Path,
    policy: &WorkspaceSymbolClasses,
    provider_ledger: &ProviderPromotionLedger,
    violations: &mut Vec<String>,
) {
    if policy.schema_version != 1 {
        violations.push(format!(
            "{POLICY_PATH}: schema_version is {}; expected 1",
            policy.schema_version
        ));
    }
    if policy.policy != POLICY_NAME {
        violations.push(format!(
            "{POLICY_PATH}: policy is {:?}; expected {:?}",
            policy.policy, POLICY_NAME
        ));
    }
    if policy.default_state != "blocked_unlisted" {
        violations.push(format!(
            "{POLICY_PATH}: default_state is {:?}; expected \"blocked_unlisted\"",
            policy.default_state
        ));
    }
    require_non_empty(POLICY_PATH, "default_fallback", &policy.default_fallback, violations);
    if policy.required_generated_label != REQUIRED_GENERATED_LABEL {
        violations.push(format!(
            "{POLICY_PATH}: required_generated_label is {:?}; expected {:?}",
            policy.required_generated_label, REQUIRED_GENERATED_LABEL
        ));
    }

    require_existing_path(root, POLICY_PATH, "spec", &policy.spec, violations);
    require_existing_path(root, POLICY_PATH, "support_tiers", &policy.support_tiers, violations);
    require_existing_path(
        root,
        POLICY_PATH,
        "provider_promotion_ledger",
        &policy.provider_promotion_ledger,
        violations,
    );

    require_non_empty_list(POLICY_PATH, "receipt_sources", &policy.receipt_sources, violations);
    for source in &policy.receipt_sources {
        require_existing_path(root, POLICY_PATH, "receipt_sources", source, violations);
    }

    let allowed = allowed_blockers(provider_ledger);
    if allowed.is_empty() {
        violations.push(format!(
            "{POLICY_PATH}: provider_promotion_ledger has no blocker_registry entries"
        ));
    }

    if policy.class.is_empty() {
        violations.push(format!("{POLICY_PATH}: class must not be empty"));
    }
}

fn validate_classes(
    policy: &WorkspaceSymbolClasses,
    provider_ledger: &ProviderPromotionLedger,
    violations: &mut Vec<String>,
) {
    let allowed_states = ALLOWED_STATES.iter().copied().collect::<BTreeSet<_>>();
    let allowed_provenances = ALLOWED_PROVENANCES.iter().copied().collect::<BTreeSet<_>>();
    let allowed_blockers = allowed_blockers(provider_ledger);
    let mut seen_names = BTreeSet::new();

    for class in &policy.class {
        let key = format!("{POLICY_PATH}: class {}", class.name);
        if !seen_names.insert(class.name.as_str()) {
            violations.push(format!("{POLICY_PATH}: duplicate class name {:?}", class.name));
        }

        require_non_empty(&key, "name", &class.name, violations);
        require_non_empty(&key, "state", &class.state, violations);
        require_non_empty(&key, "surface", &class.surface, violations);
        require_non_empty(&key, "fact_provenance", &class.fact_provenance, violations);
        require_non_empty(&key, "claim_boundary", &class.claim_boundary, violations);
        require_non_empty_list(&key, "blocks", &class.blocks, violations);

        if class.surface != REQUIRED_SURFACE {
            violations.push(format!(
                "{key} has surface {:?}; expected {:?}",
                class.surface, REQUIRED_SURFACE
            ));
        }
        if !allowed_states.contains(class.state.as_str()) {
            violations.push(format!("{key} has unsupported state {:?}", class.state));
        }
        if !allowed_provenances.contains(class.fact_provenance.as_str()) {
            violations
                .push(format!("{key} has unsupported fact_provenance {:?}", class.fact_provenance));
        }
        validate_blockers(&key, "blocks", &class.blocks, &allowed_blockers, violations);

        validate_source_backed_exact_class(class, &key, violations);
        validate_source_backed_generated_class(class, &key, policy, violations);
        validate_blocked_generated_class(class, &key, violations);
    }
}

fn validate_source_backed_exact_class(
    class: &WorkspaceSymbolClass,
    key: &str,
    violations: &mut Vec<String>,
) {
    if class.fact_provenance != "ExplicitSource" {
        return;
    }
    if !class.live {
        violations.push(format!("{key} is ExplicitSource but live is false"));
    }
    if class.requires_generated_label {
        violations.push(format!("{key} is ExplicitSource but requires_generated_label is true"));
    }
    if !class.requires_source_anchor {
        violations.push(format!("{key} is ExplicitSource but requires_source_anchor is false"));
    }
}

fn validate_source_backed_generated_class(
    class: &WorkspaceSymbolClass,
    key: &str,
    policy: &WorkspaceSymbolClasses,
    violations: &mut Vec<String>,
) {
    if class.fact_provenance != "SourceBackedGenerated" {
        return;
    }
    if !class.live {
        return;
    }
    if class.state != "generated_label_pilot" {
        violations.push(format!(
            "{key} is SourceBackedGenerated but state is {:?}; expected \"generated_label_pilot\"",
            class.state
        ));
    }
    if !class.requires_non_empty_query {
        violations
            .push(format!("{key} is SourceBackedGenerated but requires_non_empty_query is false"));
    }
    if !class.requires_ready_index {
        violations
            .push(format!("{key} is SourceBackedGenerated but requires_ready_index is false"));
    }
    if !class.requires_high_confidence {
        violations
            .push(format!("{key} is SourceBackedGenerated but requires_high_confidence is false"));
    }
    if !class.requires_source_anchor {
        violations
            .push(format!("{key} is SourceBackedGenerated but requires_source_anchor is false"));
    }
    if !class.requires_generated_label {
        violations
            .push(format!("{key} is SourceBackedGenerated but requires_generated_label is false"));
    }
    match class.label.as_deref() {
        Some(label) if label == policy.required_generated_label => {}
        Some(label) => violations.push(format!(
            "{key} label is {label:?}; expected {:?}",
            policy.required_generated_label
        )),
        None => violations.push(format!("{key} is SourceBackedGenerated but label is missing")),
    }

    let blockers = class.blocks.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in REQUIRED_GENERATED_BLOCKERS {
        if !blockers.contains(required) {
            violations.push(format!(
                "{key} is SourceBackedGenerated but blocks is missing required blocker {required:?}"
            ));
        }
    }
}

fn validate_blocked_generated_class(
    class: &WorkspaceSymbolClass,
    key: &str,
    violations: &mut Vec<String>,
) {
    if class.fact_provenance != "GeneratedNoSource" && class.fact_provenance != "DynamicBoundary" {
        return;
    }
    if class.live {
        violations.push(format!("{key} is {} but live is true", class.fact_provenance));
    }
    if class.state != "blocked" {
        violations.push(format!(
            "{key} is {} but state is {:?}; expected \"blocked\"",
            class.fact_provenance, class.state
        ));
    }
    if class.requires_source_anchor {
        violations
            .push(format!("{key} is {} but requires_source_anchor is true", class.fact_provenance));
    }
}

fn allowed_blockers(provider_ledger: &ProviderPromotionLedger) -> BTreeSet<&str> {
    provider_ledger.blocker_registry.iter().map(String::as_str).collect()
}

fn validate_blockers(
    doc: &str,
    field: &str,
    blockers: &[String],
    allowed: &BTreeSet<&str>,
    violations: &mut Vec<String>,
) {
    for blocker in blockers {
        if !allowed.contains(blocker.as_str()) {
            violations.push(format!("{doc}: {field} contains unsupported blocker {blocker:?}"));
        }
    }
}

fn require_existing_path(
    root: &Path,
    doc: &str,
    field: &str,
    rel: &str,
    violations: &mut Vec<String>,
) {
    require_non_empty(doc, field, rel, violations);
    if !rel.trim().is_empty() && !root.join(rel).exists() {
        violations.push(format!("{doc}: {field} path does not exist: {rel}"));
    }
}

fn require_non_empty(doc: &str, field: &str, value: &str, violations: &mut Vec<String>) {
    if value.trim().is_empty() {
        violations.push(format!("{doc}: field {field} must not be empty"));
    }
}

fn require_non_empty_list(doc: &str, field: &str, values: &[String], violations: &mut Vec<String>) {
    if values.is_empty() {
        violations.push(format!("{doc}: field {field} must not be empty"));
        return;
    }
    for value in values {
        if value.trim().is_empty() {
            violations.push(format!("{doc}: field {field} contains an empty item"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T>;

    fn policy() -> WorkspaceSymbolClasses {
        WorkspaceSymbolClasses {
            schema_version: 1,
            policy: POLICY_NAME.to_string(),
            spec: "docs/specs/PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md"
                .to_string(),
            support_tiers: "docs/project/status/SUPPORT_TIERS.md".to_string(),
            provider_promotion_ledger: "policy/provider-promotion-ledger.toml".to_string(),
            default_state: "blocked_unlisted".to_string(),
            default_fallback: "legacy_workspace_symbol_index".to_string(),
            required_generated_label: REQUIRED_GENERATED_LABEL.to_string(),
            receipt_sources: vec!["docs/project/status/provider_confidence_matrix.md".to_string()],
            class: Vec::new(),
        }
    }

    fn provider_ledger() -> ProviderPromotionLedger {
        ProviderPromotionLedger {
            blocker_registry: vec![
                "generated_no_source".to_string(),
                "dynamic_boundary".to_string(),
                "stale_fact".to_string(),
                "low_confidence".to_string(),
                "ambiguous_identity".to_string(),
                "fallback_policy".to_string(),
                "unsupported_fact_class".to_string(),
            ],
        }
    }

    fn live_generated_class() -> WorkspaceSymbolClass {
        WorkspaceSymbolClass {
            name: "source_backed_generated_framework_symbol".to_string(),
            state: "generated_label_pilot".to_string(),
            surface: REQUIRED_SURFACE.to_string(),
            fact_provenance: "SourceBackedGenerated".to_string(),
            live: true,
            requires_non_empty_query: true,
            requires_ready_index: true,
            requires_high_confidence: true,
            requires_source_anchor: true,
            requires_generated_label: true,
            label: Some(REQUIRED_GENERATED_LABEL.to_string()),
            blocks: REQUIRED_GENERATED_BLOCKERS
                .iter()
                .map(|blocker| (*blocker).to_string())
                .collect(),
            claim_boundary: "Source-backed generated/framework members are labeled and anchored."
                .to_string(),
        }
    }

    #[test]
    fn rejects_generated_live_class_without_label() -> TestResult {
        let policy = policy();
        let mut class = live_generated_class();
        class.requires_generated_label = false;
        class.label = None;
        let mut violations = Vec::new();

        validate_source_backed_generated_class(&class, "test", &policy, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("requires_generated_label")),
            "generated live class without required label should be rejected: {violations:?}"
        );
        assert!(
            violations.iter().any(|violation| violation.contains("label is missing")),
            "generated live class without label text should be rejected: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_generated_live_class_without_source_anchor() -> TestResult {
        let policy = policy();
        let mut class = live_generated_class();
        class.requires_source_anchor = false;
        let mut violations = Vec::new();

        validate_source_backed_generated_class(&class, "test", &policy, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("requires_source_anchor")),
            "generated live class without source anchor should be rejected: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_generated_no_source_live_class() -> TestResult {
        let class = WorkspaceSymbolClass {
            name: "generated_no_source_candidate".to_string(),
            state: "generated_label_pilot".to_string(),
            surface: REQUIRED_SURFACE.to_string(),
            fact_provenance: "GeneratedNoSource".to_string(),
            live: true,
            requires_non_empty_query: false,
            requires_ready_index: false,
            requires_high_confidence: false,
            requires_source_anchor: false,
            requires_generated_label: true,
            label: None,
            blocks: vec!["generated_no_source".to_string(), "unsupported_fact_class".to_string()],
            claim_boundary: "Generated/no-source candidates remain blocked.".to_string(),
        };
        let mut violations = Vec::new();

        validate_blocked_generated_class(&class, "test", &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("live is true")),
            "generated/no-source live class should be rejected: {violations:?}"
        );
        assert!(
            violations.iter().any(|violation| violation.contains("expected \"blocked\"")),
            "generated/no-source non-blocked class should be rejected: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_unknown_blocker_names() -> TestResult {
        let provider_ledger = provider_ledger();
        let allowed = allowed_blockers(&provider_ledger);
        let mut violations = Vec::new();

        validate_blockers(
            "test",
            "blocks",
            &["generated_no_source".to_string(), "made_up_blocker".to_string()],
            &allowed,
            &mut violations,
        );

        assert!(
            violations.iter().any(|violation| violation.contains("made_up_blocker")),
            "unknown blocker should be rejected: {violations:?}"
        );
        Ok(())
    }
}
