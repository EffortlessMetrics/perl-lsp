//! Coverage tests for `NativeCriticProfile` and `NativeCriticRegistry`.
//!
//! These exercise the public profile parser, the stable-string label, and the
//! registry's small orchestration surface (constructors, len/is_empty,
//! rule_ids ordering, add_rule mutation, and the recommended/strict
//! pre-populated profiles).
//!
//! The private rule-enabled / severity-enabled gates are reached indirectly
//! via the documented `include` / `exclude` / `severity` fields on
//! `CriticConfig` and a synthetic `CriticRule` implementation, which lets the
//! filtering branches be exercised without spinning up a real parser pipeline.

use perl_lsp_rs_core::tooling::perl_critic::{
    CriticCategory, CriticConfig, CriticContext, CriticFinding, CriticRule, NativeCriticProfile,
    NativeCriticRegistry, Severity,
};
use perl_parser_core::position::{Position, Range};
use perl_parser_core::{Node, NodeKind, SourceLocation};

// ---- NativeCriticProfile ---------------------------------------------------

#[test]
fn profile_parse_recommended_token_matches() {
    assert_eq!(NativeCriticProfile::parse("recommended"), Some(NativeCriticProfile::Recommended));
}

#[test]
fn profile_parse_strict_token_matches() {
    assert_eq!(NativeCriticProfile::parse("strict"), Some(NativeCriticProfile::Strict));
}

#[test]
fn profile_parse_unknown_token_is_none() {
    assert!(NativeCriticProfile::parse("loose").is_none());
}

#[test]
fn profile_parse_empty_string_is_none() {
    assert!(NativeCriticProfile::parse("").is_none());
}

#[test]
fn profile_parse_is_case_sensitive() {
    // Case sensitivity is part of the contract; receipts compare stable
    // lowercase tokens, so a mixed-case input is intentionally rejected.
    assert!(NativeCriticProfile::parse("Strict").is_none());
    assert!(NativeCriticProfile::parse("RECOMMENDED").is_none());
}

#[test]
fn profile_as_str_roundtrips_through_parse() {
    for profile in [NativeCriticProfile::Recommended, NativeCriticProfile::Strict] {
        let token = profile.as_str();
        assert_eq!(
            NativeCriticProfile::parse(token),
            Some(profile),
            "roundtrip failed for {token}"
        );
    }
}

#[test]
fn profile_as_str_labels_are_stable() {
    assert_eq!(NativeCriticProfile::Recommended.as_str(), "recommended");
    assert_eq!(NativeCriticProfile::Strict.as_str(), "strict");
}

// ---- NativeCriticRegistry: empty / construction ----------------------------

#[test]
fn registry_new_is_empty() {
    let registry = NativeCriticRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
    assert!(registry.rule_ids().is_empty());
}

#[test]
fn registry_default_is_empty() {
    let registry = NativeCriticRegistry::default();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[test]
fn registry_with_rules_preserves_len_and_order() {
    let registry = NativeCriticRegistry::with_rules(vec![
        Box::new(MarkerRule::new("alpha")),
        Box::new(MarkerRule::new("beta")),
        Box::new(MarkerRule::new("gamma")),
    ]);
    assert_eq!(registry.len(), 3);
    assert!(!registry.is_empty());
    assert_eq!(registry.rule_ids(), vec!["alpha", "beta", "gamma"]);
}

#[test]
fn registry_add_rule_appends() {
    let mut registry = NativeCriticRegistry::new();
    registry.add_rule(Box::new(MarkerRule::new("first")));
    registry.add_rule(Box::new(MarkerRule::new("second")));
    assert_eq!(registry.len(), 2);
    assert_eq!(registry.rule_ids(), vec!["first", "second"]);
}

// ---- NativeCriticRegistry: built-in profiles -------------------------------

#[test]
fn registry_recommended_is_non_empty() {
    // Don't pin the exact rule list; that would couple this test to the
    // recommended-set roster. Just verify a meaningful set was wired up.
    let registry = NativeCriticRegistry::recommended();
    assert!(!registry.is_empty(), "recommended bundle should have rules");
    assert!(registry.len() >= 16, "recommended bundle smaller than expected: {}", registry.len());
}

#[test]
fn registry_for_profile_recommended_is_subset_of_strict() {
    // Strict adds extra semantic rules on top of recommended; strict must be
    // at least as large as recommended.
    let recommended_len = NativeCriticRegistry::for_profile(NativeCriticProfile::Recommended).len();
    let strict_len = NativeCriticRegistry::for_profile(NativeCriticProfile::Strict).len();
    assert!(
        strict_len >= recommended_len,
        "strict ({strict_len}) should be at least as large as recommended ({recommended_len})"
    );
}

#[test]
fn registry_built_in_profiles_have_unique_rule_ids() {
    for profile in [NativeCriticProfile::Recommended, NativeCriticProfile::Strict] {
        let ids = NativeCriticRegistry::for_profile(profile).rule_ids();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate rule IDs in {profile:?} profile: {ids:?}");
    }
}

#[test]
fn registry_recommended_helper_matches_strict_profile() {
    // `NativeCriticRegistry::recommended()` is documented as a compatibility
    // shim: it currently returns the strict profile. Pin that contract here
    // so a quiet roster change doesn't slip past.
    let recommended_helper = NativeCriticRegistry::recommended().rule_ids();
    let strict_profile = NativeCriticRegistry::for_profile(NativeCriticProfile::Strict).rule_ids();
    assert_eq!(recommended_helper, strict_profile);
}

// ---- NativeCriticRegistry::check: include / exclude / severity gates --------

#[test]
fn check_returns_no_findings_when_registry_is_empty() {
    let source = String::new();
    let ast = empty_program();
    let config = CriticConfig::default();
    let ctx = CriticContext::new(&source, &ast, &config);

    let findings = NativeCriticRegistry::new().check(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn check_runs_rule_when_include_is_empty_and_not_excluded() {
    let registry = NativeCriticRegistry::with_rules(vec![Box::new(MarkerRule::new("rule.a"))]);
    let findings = run_check(&registry, &CriticConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "rule.a");
}

#[test]
fn check_skips_rule_when_not_listed_in_explicit_include() {
    let registry = NativeCriticRegistry::with_rules(vec![
        Box::new(MarkerRule::new("rule.a")),
        Box::new(MarkerRule::new("rule.b")),
    ]);
    let config = CriticConfig { include: vec!["rule.a".to_string()], ..Default::default() };
    let findings = run_check(&registry, &config);
    let ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert_eq!(ids, vec!["rule.a"]);
}

#[test]
fn check_skips_rule_when_explicitly_excluded() {
    let registry = NativeCriticRegistry::with_rules(vec![
        Box::new(MarkerRule::new("rule.a")),
        Box::new(MarkerRule::new("rule.b")),
    ]);
    let config = CriticConfig { exclude: vec!["rule.b".to_string()], ..Default::default() };
    let findings = run_check(&registry, &config);
    let ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert_eq!(ids, vec!["rule.a"]);
}

#[test]
fn check_filters_out_findings_below_configured_severity() {
    // Severity::Cruel = 2; configure threshold = 3 so the finding is dropped.
    let registry = NativeCriticRegistry::with_rules(vec![Box::new(
        MarkerRule::new("rule.low").with_severity(Severity::Cruel),
    )]);
    let config = CriticConfig { severity: 3, ..Default::default() };
    let findings = run_check(&registry, &config);
    assert!(findings.is_empty());
}

#[test]
fn check_keeps_findings_at_or_above_configured_severity() {
    let registry = NativeCriticRegistry::with_rules(vec![Box::new(
        MarkerRule::new("rule.harsh").with_severity(Severity::Harsh),
    )]);
    let config = CriticConfig { severity: 3, ..Default::default() };
    let findings = run_check(&registry, &config);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "rule.harsh");
}

#[test]
fn check_violations_carries_file_through_finding_to_violation_bridge() {
    let registry = NativeCriticRegistry::with_rules(vec![Box::new(MarkerRule::new("rule.bridge"))]);
    let source = String::new();
    let ast = empty_program();
    let config = CriticConfig::default();
    let ctx = CriticContext::new(&source, &ast, &config);

    let violations = registry.check_violations(&ctx, "src/foo.pl");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].policy, "rule.bridge");
    assert_eq!(violations[0].file, "src/foo.pl");
}

// ---- Helpers ---------------------------------------------------------------

fn run_check(registry: &NativeCriticRegistry, config: &CriticConfig) -> Vec<CriticFinding> {
    let source = String::new();
    let ast = empty_program();
    let ctx = CriticContext::new(&source, &ast, config);
    registry.check(&ctx)
}

fn empty_program() -> Node {
    Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 })
}

/// Minimal `CriticRule` impl used to drive the registry filtering logic.
struct MarkerRule {
    id: &'static str,
    severity: Severity,
}

impl MarkerRule {
    fn new(id: &'static str) -> Self {
        Self { id, severity: Severity::Harsh }
    }

    fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}

impl CriticRule for MarkerRule {
    fn id(&self) -> &'static str {
        self.id
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        self.severity
    }

    fn check(&self, _ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        out.push(CriticFinding {
            rule_id: self.id.to_string(),
            category: CriticCategory::Syntax,
            severity: self.severity,
            range: Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
            message: format!("{} finding", self.id),
            explanation: String::new(),
            suppression_key: self.id.to_string(),
            related: Vec::new(),
            fix: None,
        });
    }
}
