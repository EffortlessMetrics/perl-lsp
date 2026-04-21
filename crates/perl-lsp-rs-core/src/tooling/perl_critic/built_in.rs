use super::{QuickFix, Severity, Violation, built_in_quick_fix, insertion_range};
use perl_parser_core::Node;

/// Built-in policy analyzer that works without external perlcritic
pub struct BuiltInAnalyzer {
    /// Collection of registered policy implementations
    policies: Vec<Box<dyn Policy>>,
}

/// Trait for implementing policies
pub trait Policy: Send + Sync {
    /// Returns the fully qualified policy name.
    fn name(&self) -> &str;
    /// Returns the severity level for violations of this policy.
    fn severity(&self) -> Severity;
    /// Analyzes the AST and source content, returning any violations found.
    fn analyze(&self, ast: &Node, content: &str) -> Vec<Violation>;
}

/// Require 'use strict'
struct RequireUseStrict;

impl Policy for RequireUseStrict {
    fn name(&self) -> &str {
        "TestingAndDebugging::RequireUseStrict"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        missing_use_statement_violation(
            self,
            content,
            "strict",
            "Always use strict to catch common mistakes",
        )
    }
}

/// Require 'use warnings'
struct RequireUseWarnings;

impl Policy for RequireUseWarnings {
    fn name(&self) -> &str {
        "TestingAndDebugging::RequireUseWarnings"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        missing_use_statement_violation(
            self,
            content,
            "warnings",
            "Always use warnings to catch potential issues",
        )
    }
}

impl Default for BuiltInAnalyzer {
    fn default() -> Self {
        Self { policies: vec![Box::new(RequireUseStrict), Box::new(RequireUseWarnings)] }
    }
}

impl BuiltInAnalyzer {
    /// Creates a new analyzer with default built-in policies.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze AST with built-in policies
    pub fn analyze(&self, ast: &Node, content: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for policy in &self.policies {
            violations.extend(policy.analyze(ast, content));
        }
        violations
    }

    /// Get quick fix for a violation
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        built_in_quick_fix(violation)
    }
}

fn missing_use_statement_violation(
    policy: &dyn Policy,
    content: &str,
    feature: &str,
    explanation: &str,
) -> Vec<Violation> {
    if content.contains(&format!("use {feature}")) {
        return Vec::new();
    }

    vec![Violation {
        policy: policy.name().to_string(),
        description: format!("Code does not use {feature}"),
        explanation: explanation.to_string(),
        severity: policy.severity(),
        range: insertion_range(),
        file: String::new(),
    }]
}
