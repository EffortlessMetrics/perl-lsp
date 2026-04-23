use super::{built_in_quick_fix, insertion_range, QuickFix, Severity, Violation};
use perl_parser_core::Node;
use regex::Regex;
use std::sync::LazyLock;

static TWO_ARG_OPEN_PATTERN: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)\bopen\s*(?:\(|\s+)\s*[^,\n]+,\s*[^,\n\)]+\)?\s*;").ok());

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
        Self {
            policies: vec![
                Box::new(RequireUseStrict),
                Box::new(RequireUseWarnings),
                Box::new(RequireThreeArgOpen),
            ],
        }
    }
}

/// Require three-argument `open` calls.
struct RequireThreeArgOpen;

impl Policy for RequireThreeArgOpen {
    fn name(&self) -> &str {
        "InputOutput::RequireThreeArgOpen"
    }

    fn severity(&self) -> Severity {
        Severity::Brutal
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        let Some(pattern) = TWO_ARG_OPEN_PATTERN.as_ref() else {
            return Vec::new();
        };

        let Some(mat) = pattern.find(content) else {
            return Vec::new();
        };

        let line = content[..mat.start()].bytes().filter(|b| *b == b'\n').count() as u32;
        let line_start = content[..mat.start()].rfind('\n').map_or(0, |idx| idx + 1);
        let column = content[line_start..mat.start()].chars().count() as u32;

        vec![Violation {
            policy: self.name().to_string(),
            description: "Use three-argument open for safer file handling".to_string(),
            explanation: "Three-argument open avoids mode/path ambiguities and is easier to audit."
                .to_string(),
            severity: self.severity(),
            range: perl_parser_core::position::Range {
                start: perl_parser_core::position::Position { byte: mat.start(), line, column },
                end: perl_parser_core::position::Position {
                    byte: mat.start() + 4,
                    line,
                    column: column + 4,
                },
            },
            file: String::new(),
        }]
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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::{Node, NodeKind, SourceLocation};

    fn dummy_ast() -> Node {
        Node::new(NodeKind::Program { statements: Vec::new() }, SourceLocation { start: 0, end: 0 })
    }

    #[test]
    fn built_in_analyzer_reports_two_arg_open_without_perlcritic() {
        let analyzer = BuiltInAnalyzer::new();
        let content = "use strict;\nuse warnings;\nopen my $fh, $path;\n";

        let violations = analyzer.analyze(&dummy_ast(), content);

        assert!(
            violations
                .iter()
                .any(|violation| violation.policy == "InputOutput::RequireThreeArgOpen"),
            "expected built-in analyzer to flag two-argument open"
        );
    }

    #[test]
    fn built_in_analyzer_allows_three_arg_open() {
        let analyzer = BuiltInAnalyzer::new();
        let content = "use strict;\nuse warnings;\nopen my $fh, '<', $path;\n";

        let violations = analyzer.analyze(&dummy_ast(), content);

        assert!(
            !violations
                .iter()
                .any(|violation| violation.policy == "InputOutput::RequireThreeArgOpen"),
            "expected three-argument open to pass built-in policy"
        );
    }
}
