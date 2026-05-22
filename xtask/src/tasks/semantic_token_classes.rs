//! Validate the semantic-token class promotion registry.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const POLICY_PATH: &str = "policy/semantic-token-classes.toml";
const POLICY_NAME: &str = "semantic-token-classes";

const REQUIRED_PROMOTION_CONDITIONS: &[&str] = &[
    "source_backed_compiler_span",
    "span_matches_exactly_one_existing_live_token",
    "emits_no_unscoped_new_output",
    "did_change_freshness_proven",
    "generated_no_source_blocked",
    "stale_dynamic_low_confidence_fallback_unmatched_blocked",
    "support_review_blocks_broad_promotion",
];

const ALLOWED_STATES: &[&str] = &["partial_live_trace", "shadow_proof", "blocked", "deferred"];
const ALLOWED_TOKEN_KINDS: &[&str] = &["function", "macro", "method", "namespace", "variable"];

const REQUIRED_TRACE_BLOCKERS: &[&str] =
    &["generated_no_source", "dynamic_boundary", "stale_fact", "low_confidence", "unmatched_span"];

#[derive(Debug, Deserialize)]
struct SemanticTokenClasses {
    schema_version: u32,
    policy: String,
    spec: String,
    support_tiers: String,
    provider_promotion_ledger: String,
    default_state: String,
    default_fallback: String,
    required_promotion_conditions: Vec<String>,
    default_blockers: Vec<String>,
    receipt_sources: Vec<String>,
    #[serde(default)]
    class: Vec<SemanticTokenClass>,
}

#[derive(Debug, Deserialize)]
struct SemanticTokenClass {
    name: String,
    state: String,
    live_token_kind: String,
    compiler_identity_prefix: String,
    emits_new_output: bool,
    requires_exact_live_token_match: bool,
    requires_edit_freshness: bool,
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
        "semantic token class check passed: {} classes, {} receipt sources, {} allowed blockers",
        stats.classes, stats.receipts, stats.blockers
    );
    Ok(())
}

fn validate(root: &Path) -> Result<ValidationStats> {
    let policy: SemanticTokenClasses = read_policy(root, POLICY_PATH)?;
    let provider_ledger =
        read_policy::<ProviderPromotionLedger>(root, &policy.provider_promotion_ledger)?;
    let mut violations = Vec::new();

    validate_policy_shape(root, &policy, &provider_ledger, &mut violations);
    validate_classes(&policy, &provider_ledger, &mut violations);

    if !violations.is_empty() {
        eprintln!("semantic token class policy violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("semantic token class check failed with {} violation(s)", violations.len());
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
    policy: &SemanticTokenClasses,
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

    require_exact_set(
        POLICY_PATH,
        "required_promotion_conditions",
        &policy.required_promotion_conditions,
        REQUIRED_PROMOTION_CONDITIONS,
        violations,
    );

    let allowed = allowed_blockers(provider_ledger);
    require_non_empty_list(POLICY_PATH, "default_blockers", &policy.default_blockers, violations);
    validate_blockers(
        POLICY_PATH,
        "default_blockers",
        &policy.default_blockers,
        &allowed,
        violations,
    );

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

    if policy.class.is_empty() {
        violations.push(format!("{POLICY_PATH}: class must not be empty"));
    }
}

fn validate_classes(
    policy: &SemanticTokenClasses,
    provider_ledger: &ProviderPromotionLedger,
    violations: &mut Vec<String>,
) {
    let allowed_states = ALLOWED_STATES.iter().copied().collect::<BTreeSet<_>>();
    let allowed_token_kinds = ALLOWED_TOKEN_KINDS.iter().copied().collect::<BTreeSet<_>>();
    let allowed_blocker_set = allowed_blockers(provider_ledger);
    let mut seen_names = BTreeSet::new();

    for class in &policy.class {
        let key = format!("{POLICY_PATH}: class {}", class.name);
        if !seen_names.insert(class.name.as_str()) {
            violations.push(format!("{POLICY_PATH}: duplicate class name {:?}", class.name));
        }

        require_non_empty(&key, "name", &class.name, violations);
        require_non_empty(&key, "state", &class.state, violations);
        require_non_empty(&key, "live_token_kind", &class.live_token_kind, violations);
        require_non_empty(
            &key,
            "compiler_identity_prefix",
            &class.compiler_identity_prefix,
            violations,
        );
        require_non_empty(&key, "claim_boundary", &class.claim_boundary, violations);
        require_non_empty_list(&key, "blocks", &class.blocks, violations);

        if !allowed_states.contains(class.state.as_str()) {
            violations.push(format!("{key} has unsupported state {:?}", class.state));
        }
        if !allowed_token_kinds.contains(class.live_token_kind.as_str()) {
            violations
                .push(format!("{key} has unsupported live_token_kind {:?}", class.live_token_kind));
        }
        validate_blockers(&key, "blocks", &class.blocks, &allowed_blocker_set, violations);

        if class.state == "partial_live_trace" {
            validate_live_trace_class(class, &key, violations);
        }
    }
}

fn validate_live_trace_class(class: &SemanticTokenClass, key: &str, violations: &mut Vec<String>) {
    if class.emits_new_output {
        violations.push(format!("{key} is partial_live_trace but emits_new_output is true"));
    }
    if !class.requires_exact_live_token_match {
        violations.push(format!(
            "{key} is partial_live_trace but requires_exact_live_token_match is false"
        ));
    }
    if !class.requires_edit_freshness {
        violations
            .push(format!("{key} is partial_live_trace but requires_edit_freshness is false"));
    }

    let blockers = class.blocks.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in REQUIRED_TRACE_BLOCKERS {
        if !blockers.contains(required) {
            violations.push(format!(
                "{key} is partial_live_trace but blocks is missing required blocker {required:?}"
            ));
        }
    }
}

fn allowed_blockers(provider_ledger: &ProviderPromotionLedger) -> BTreeSet<&str> {
    let mut blockers =
        provider_ledger.blocker_registry.iter().map(String::as_str).collect::<BTreeSet<_>>();
    blockers.insert("unmatched_span");
    blockers
}

fn require_exact_set(
    doc: &str,
    field: &str,
    actual: &[String],
    expected: &[&str],
    violations: &mut Vec<String>,
) {
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_set = expected.iter().copied().collect::<BTreeSet<_>>();

    for missing in expected_set.difference(&actual_set) {
        violations.push(format!("{doc}: {field} missing required entry {missing:?}"));
    }
    for unexpected in actual_set.difference(&expected_set) {
        violations.push(format!("{doc}: {field} contains unsupported entry {unexpected:?}"));
    }
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

    fn provider_ledger() -> ProviderPromotionLedger {
        ProviderPromotionLedger {
            blocker_registry: vec![
                "generated_no_source".to_string(),
                "dynamic_boundary".to_string(),
                "stale_fact".to_string(),
                "low_confidence".to_string(),
                "fallback_policy".to_string(),
                "unsupported_fact_class".to_string(),
            ],
        }
    }

    fn live_trace_class() -> SemanticTokenClass {
        SemanticTokenClass {
            name: "method_call".to_string(),
            state: "partial_live_trace".to_string(),
            live_token_kind: "method".to_string(),
            compiler_identity_prefix: "token:method_call:".to_string(),
            emits_new_output: false,
            requires_exact_live_token_match: true,
            requires_edit_freshness: true,
            blocks: REQUIRED_TRACE_BLOCKERS.iter().map(|blocker| (*blocker).to_string()).collect(),
            claim_boundary: "source-backed method calls match existing live tokens".to_string(),
        }
    }

    #[test]
    fn rejects_trace_class_that_emits_new_output() -> TestResult {
        let mut class = live_trace_class();
        class.emits_new_output = true;
        let mut violations = Vec::new();

        validate_live_trace_class(&class, "test", &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("emits_new_output")),
            "output-expanding trace class should be rejected: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_trace_class_without_unmatched_span_blocker() -> TestResult {
        let mut class = live_trace_class();
        class.blocks.retain(|blocker| blocker != "unmatched_span");
        let mut violations = Vec::new();

        validate_live_trace_class(&class, "test", &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("unmatched_span")),
            "missing unmatched span blocker should be rejected: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn accepts_unmatched_span_as_semantic_token_local_blocker() -> TestResult {
        let provider_ledger = provider_ledger();
        let allowed = allowed_blockers(&provider_ledger);

        assert!(allowed.contains("unmatched_span"));
        Ok(())
    }
}
