//! Native critic rule contract.
//!
//! These types define the Rust-native policy diagnostic surface that future
//! rules should target. They intentionally live beside the existing
//! subprocess-backed Perl::Critic adapter and built-in fallback so callers can
//! migrate rule-by-rule without changing runtime behavior in one large step.

use super::{CriticConfig, Severity, Violation, insertion_range};
use perl_parser_core::Node;
use perl_parser_core::position::Range;
use serde::{Deserialize, Serialize};

/// Native critic rule category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticCategory {
    /// Syntax-level policy that can be checked from tokens or AST shape.
    Syntax,
    /// Semantic policy that needs bindings, scopes, or inferred facts.
    Semantic,
    /// Workspace-aware policy that needs cross-file/module facts.
    Workspace,
    /// Style convention policy.
    Style,
    /// Maintainability or complexity policy.
    Maintainability,
    /// Security or unsafe-code policy.
    Security,
    /// Documentation or POD policy.
    Documentation,
}

/// Safety level for an optional native critic fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixSafety {
    /// Safe to apply automatically.
    Safe,
    /// Useful suggestion, but should require user confirmation.
    Suggested,
    /// Diagnostic-only guidance with no automatic edit.
    ManualOnly,
}

/// Text edit attached to a native critic fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticTextEdit {
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// Optional fix attached to a native critic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFix {
    /// Human-readable action title.
    pub title: String,
    /// How safe the edit is.
    pub safety: FixSafety,
    /// Edits needed to apply the fix.
    pub edits: Vec<CriticTextEdit>,
}

/// Related span for richer native critic diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticRelatedInformation {
    /// Related source range.
    pub range: Range,
    /// Explanation for the related span.
    pub message: String,
}

/// Native critic finding emitted by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFinding {
    /// Stable native rule ID, such as `native.variables.unused_lexical`.
    pub rule_id: String,
    /// Rule category.
    pub category: CriticCategory,
    /// Finding severity.
    pub severity: Severity,
    /// Precise source span for the finding.
    pub range: Range,
    /// Short diagnostic message.
    pub message: String,
    /// Longer explanation shown in editor/CI details.
    pub explanation: String,
    /// Suppression key accepted by inline or file-level suppression handling.
    pub suppression_key: String,
    /// Related spans, when useful.
    pub related: Vec<CriticRelatedInformation>,
    /// Optional fix.
    pub fix: Option<CriticFix>,
}

impl CriticFinding {
    /// Convert this native finding into the existing violation shape.
    ///
    /// This is the bridge that lets native rules flow through current LSP,
    /// execute-command, and diagnostic consumers while those consumers still
    /// expect `Violation` values.
    #[must_use]
    pub fn to_violation(&self, file: impl Into<String>) -> Violation {
        Violation {
            policy: self.rule_id.clone(),
            description: self.message.clone(),
            explanation: self.explanation.clone(),
            severity: self.severity,
            range: self.range,
            file: file.into(),
        }
    }
}

/// Native critic rule context.
pub struct CriticContext<'a> {
    /// Source text being analyzed.
    pub source: &'a str,
    /// Parsed AST for the source.
    pub ast: &'a Node,
    /// Critic configuration.
    pub config: &'a CriticConfig,
}

impl<'a> CriticContext<'a> {
    /// Build a native critic context.
    pub fn new(source: &'a str, ast: &'a Node, config: &'a CriticConfig) -> Self {
        Self { source, ast, config }
    }
}

/// Native critic rule interface.
pub trait CriticRule: Send + Sync {
    /// Stable native rule ID.
    fn id(&self) -> &'static str;

    /// Rule category.
    fn category(&self) -> CriticCategory;

    /// Default severity before profile/config remapping.
    fn default_severity(&self) -> Severity;

    /// Check the source and append findings.
    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>);
}

/// Registry for Rust-native critic rules.
///
/// The registry is intentionally small orchestration: it owns rule instances,
/// runs them against a shared context, and returns their findings in registry
/// order. Runtime diagnostic wiring can build on this without each caller
/// needing to know how native rules are stored or executed.
#[derive(Default)]
pub struct NativeCriticRegistry {
    rules: Vec<Box<dyn CriticRule>>,
}

impl NativeCriticRegistry {
    /// Create an empty native critic registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry from prebuilt rules.
    #[must_use]
    pub fn with_rules(rules: Vec<Box<dyn CriticRule>>) -> Self {
        Self { rules }
    }

    /// Create the default recommended native critic registry.
    ///
    /// This is the opt-in bundle for callers migrating from ad hoc built-in
    /// policy execution to the native rule contract. Keep ordering stable so
    /// diagnostics and receipts are deterministic.
    #[must_use]
    pub fn recommended() -> Self {
        Self::with_rules(vec![Box::new(RequireUseStrictRule), Box::new(RequireUseWarningsRule)])
    }

    /// Add a rule to the registry.
    pub fn add_rule(&mut self, rule: Box<dyn CriticRule>) {
        self.rules.push(rule);
    }

    /// Number of rules in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry has no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Stable IDs for registered rules, in execution order.
    #[must_use]
    pub fn rule_ids(&self) -> Vec<&'static str> {
        self.rules.iter().map(|rule| rule.id()).collect()
    }

    /// Run all registered rules and return collected findings.
    #[must_use]
    pub fn check(&self, ctx: &CriticContext<'_>) -> Vec<CriticFinding> {
        let mut findings = Vec::new();

        for rule in &self.rules {
            rule.check(ctx, &mut findings);
        }

        findings
    }
}

/// Native rule that requires a file-level `use strict;` pragma.
///
/// This is the first built-in rule expressed through the native critic
/// contract. It deliberately does not replace the existing legacy built-in
/// analyzer yet; callers can opt into it through `NativeCriticRegistry` while
/// runtime diagnostic migration remains incremental.
pub struct RequireUseStrictRule;

impl CriticRule for RequireUseStrictRule {
    fn id(&self) -> &'static str {
        "native.testing.require_use_strict"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        if has_use_statement(ctx.source, "strict") {
            return;
        }

        let range = insertion_range();
        out.push(CriticFinding {
            rule_id: self.id().to_string(),
            category: self.category(),
            severity: self.default_severity(),
            range,
            message: "Code does not use strict".to_string(),
            explanation: "Always use strict to catch common mistakes".to_string(),
            suppression_key: self.id().to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Add 'use strict'".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit { range, new_text: "use strict;\n".to_string() }],
            }),
        });
    }
}

/// Native rule that requires a file-level `use warnings;` pragma.
///
/// Like [`RequireUseStrictRule`], this is exposed through the native critic
/// contract without replacing the existing legacy built-in analyzer yet.
pub struct RequireUseWarningsRule;

impl CriticRule for RequireUseWarningsRule {
    fn id(&self) -> &'static str {
        "native.testing.require_use_warnings"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        if has_use_statement(ctx.source, "warnings") {
            return;
        }

        let range = insertion_range();
        out.push(CriticFinding {
            rule_id: self.id().to_string(),
            category: self.category(),
            severity: self.default_severity(),
            range,
            message: "Code does not use warnings".to_string(),
            explanation: "Always use warnings to catch potential issues".to_string(),
            suppression_key: self.id().to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Add 'use warnings'".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit { range, new_text: "use warnings;\n".to_string() }],
            }),
        });
    }
}

fn has_use_statement(content: &str, feature: &str) -> bool {
    content.lines().any(|line| has_use_statement_line(line, feature))
}

fn has_use_statement_line(line: &str, feature: &str) -> bool {
    let code_portion = line.split('#').next().unwrap_or_default();
    let mut tokens = code_portion.split_whitespace();
    let Some(first) = tokens.next() else {
        return false;
    };
    if first != "use" {
        return false;
    }
    let Some(module) = tokens.next() else {
        return false;
    };
    module.trim_end_matches(';') == feature
}

/// Build an empty AST node for tests that only exercise rule contract plumbing.
#[cfg(test)]
fn empty_program_node() -> Node {
    use perl_parser_core::{NodeKind, SourceLocation};

    Node::new(NodeKind::Program { statements: Vec::new() }, SourceLocation { start: 0, end: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::position::{Position, Range};

    struct DummyRule;

    impl CriticRule for DummyRule {
        fn id(&self) -> &'static str {
            "native.test.dummy"
        }

        fn category(&self) -> CriticCategory {
            CriticCategory::Syntax
        }

        fn default_severity(&self) -> Severity {
            Severity::Harsh
        }

        fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
            if ctx.source.contains("dummy") {
                out.push(CriticFinding {
                    rule_id: self.id().to_string(),
                    category: self.category(),
                    severity: self.default_severity(),
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 5, line: 0, column: 5 },
                    },
                    message: "dummy finding".to_string(),
                    explanation: "dummy explanation".to_string(),
                    suppression_key: self.id().to_string(),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }
    }

    struct SecondDummyRule;

    impl CriticRule for SecondDummyRule {
        fn id(&self) -> &'static str {
            "native.test.second"
        }

        fn category(&self) -> CriticCategory {
            CriticCategory::Maintainability
        }

        fn default_severity(&self) -> Severity {
            Severity::Cruel
        }

        fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
            if ctx.source.contains("second") {
                out.push(CriticFinding {
                    rule_id: self.id().to_string(),
                    category: self.category(),
                    severity: self.default_severity(),
                    range: Range {
                        start: Position { byte: 6, line: 0, column: 6 },
                        end: Position { byte: 12, line: 0, column: 12 },
                    },
                    message: "second finding".to_string(),
                    explanation: "second explanation".to_string(),
                    suppression_key: self.id().to_string(),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }
    }

    #[test]
    fn native_critic_rule_contract_emits_stable_finding_shape() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("dummy", &ast, &config);
        let mut findings = Vec::new();

        DummyRule.check(&ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
        assert_eq!(findings[0].suppression_key, "native.test.dummy");
        assert_eq!(findings[0].category, CriticCategory::Syntax);
        assert_eq!(findings[0].severity, Severity::Harsh);
    }

    #[test]
    fn native_critic_finding_serializes_agent_friendly_fields() {
        let finding = CriticFinding {
            rule_id: "native.test.fixable".to_string(),
            category: CriticCategory::Style,
            severity: Severity::Cruel,
            range: Range {
                start: Position { byte: 0, line: 0, column: 0 },
                end: Position { byte: 1, line: 0, column: 1 },
            },
            message: "style issue".to_string(),
            explanation: "style explanation".to_string(),
            suppression_key: "native.test.fixable".to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Apply style fix".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 1, line: 0, column: 1 },
                    },
                    new_text: "x".to_string(),
                }],
            }),
        };

        let value = serde_json::to_value(&finding).expect("serialize native critic finding");

        assert_eq!(value["rule_id"], "native.test.fixable");
        assert_eq!(value["category"], "style");
        assert_eq!(value["fix"]["safety"], "safe");
        assert_eq!(value["fix"]["edits"][0]["new_text"], "x");
    }

    #[test]
    fn native_critic_finding_converts_to_legacy_violation_shape() {
        let finding = CriticFinding {
            rule_id: "native.variables.unused_lexical".to_string(),
            category: CriticCategory::Semantic,
            severity: Severity::Stern,
            range: Range {
                start: Position { byte: 10, line: 1, column: 4 },
                end: Position { byte: 12, line: 1, column: 6 },
            },
            message: "unused lexical variable".to_string(),
            explanation: "remove or use the lexical variable".to_string(),
            suppression_key: "native.variables.unused_lexical".to_string(),
            related: Vec::new(),
            fix: None,
        };

        let violation = finding.to_violation("lib/App.pm");

        assert_eq!(violation.policy, "native.variables.unused_lexical");
        assert_eq!(violation.description, "unused lexical variable");
        assert_eq!(violation.explanation, "remove or use the lexical variable");
        assert_eq!(violation.severity, Severity::Stern);
        assert_eq!(violation.range, finding.range);
        assert_eq!(violation.file, "lib/App.pm");
    }

    #[test]
    fn native_critic_registry_runs_rules_in_order() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("dummy second", &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DummyRule), Box::new(SecondDummyRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
        assert_eq!(registry.rule_ids(), vec!["native.test.dummy", "native.test.second"]);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
        assert_eq!(findings[1].rule_id, "native.test.second");
    }

    #[test]
    fn native_critic_registry_can_be_extended_incrementally() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("second", &ast, &config);
        let mut registry = NativeCriticRegistry::new();

        assert!(registry.is_empty());
        registry.add_rule(Box::new(SecondDummyRule));

        let findings = registry.check(&ctx);

        assert_eq!(registry.rule_ids(), vec!["native.test.second"]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, CriticCategory::Maintainability);
    }

    #[test]
    fn native_require_use_strict_rule_emits_safe_fix_when_missing() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("my $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseStrictRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.testing.require_use_strict");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Harsh);
        assert_eq!(finding.message, "Code does not use strict");
        assert_eq!(finding.suppression_key, "native.testing.require_use_strict");

        let fix = finding.fix.as_ref().expect("missing strict should have a safe fix");
        assert_eq!(fix.title, "Add 'use strict'");
        assert_eq!(fix.safety, FixSafety::Safe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, insertion_range());
        assert_eq!(fix.edits[0].new_text, "use strict;\n");
    }

    #[test]
    fn native_require_use_strict_rule_accepts_exact_pragma_only() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let exact_ctx = CriticContext::new("use strict;\nmy $x = 1;\n", &ast, &config);
        let similar_ctx = CriticContext::new("use strictures;\nmy $x = 1;\n", &ast, &config);
        let commented_ctx = CriticContext::new("# use strict;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseStrictRule)]);

        assert!(registry.check(&exact_ctx).is_empty());
        assert_eq!(registry.check(&similar_ctx).len(), 1);
        assert_eq!(registry.check(&commented_ctx).len(), 1);
    }

    #[test]
    fn native_require_use_warnings_rule_emits_safe_fix_when_missing() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("use strict;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseWarningsRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.testing.require_use_warnings");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Harsh);
        assert_eq!(finding.message, "Code does not use warnings");
        assert_eq!(finding.suppression_key, "native.testing.require_use_warnings");

        let fix = finding.fix.as_ref().expect("missing warnings should have a safe fix");
        assert_eq!(fix.title, "Add 'use warnings'");
        assert_eq!(fix.safety, FixSafety::Safe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, insertion_range());
        assert_eq!(fix.edits[0].new_text, "use warnings;\n");
    }

    #[test]
    fn native_require_use_warnings_rule_accepts_exact_pragma_only() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let exact_ctx = CriticContext::new("use warnings;\nmy $x = 1;\n", &ast, &config);
        let similar_ctx = CriticContext::new("use warningsx;\nmy $x = 1;\n", &ast, &config);
        let commented_ctx = CriticContext::new("# use warnings;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseWarningsRule)]);

        assert!(registry.check(&exact_ctx).is_empty());
        assert_eq!(registry.check(&similar_ctx).len(), 1);
        assert_eq!(registry.check(&commented_ctx).len(), 1);
    }

    #[test]
    fn native_strict_and_warnings_rules_run_together_in_order() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("my $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![
            Box::new(RequireUseStrictRule),
            Box::new(RequireUseWarningsRule),
        ]);

        let findings = registry.check(&ctx);

        assert_eq!(
            registry.rule_ids(),
            vec!["native.testing.require_use_strict", "native.testing.require_use_warnings"]
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "native.testing.require_use_strict");
        assert_eq!(findings[1].rule_id, "native.testing.require_use_warnings");
    }

    #[test]
    fn native_recommended_registry_contains_initial_policy_bundle() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("my $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::recommended();

        let findings = registry.check(&ctx);

        assert_eq!(
            registry.rule_ids(),
            vec!["native.testing.require_use_strict", "native.testing.require_use_warnings"]
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].suppression_key, "native.testing.require_use_strict");
        assert_eq!(findings[1].suppression_key, "native.testing.require_use_warnings");
    }
}
