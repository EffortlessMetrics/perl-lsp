use super::{built_in_quick_fix, insertion_range, QuickFix, Severity, Violation};
use perl_parser_core::position::Position;
use perl_parser_core::Node;
use regex::Regex;
use std::sync::LazyLock;

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

/// Require three-argument `open`.
struct RequireThreeArgOpen;

impl Policy for RequireThreeArgOpen {
    fn name(&self) -> &str {
        "InputOutput::RequireThreeArgOpen"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        static TWO_ARG_OPEN_RE: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
            Regex::new(r"(?m)^\s*open\s*(?:\(\s*)?[^,\n)]+\s*,\s*[^,\n)]+\s*(?:\)|;)")
        });

        let Ok(two_arg_open_re) = &*TWO_ARG_OPEN_RE else {
            return Vec::new();
        };

        two_arg_open_re
            .find_iter(content)
            .map(|m| {
                let start = byte_to_position(content, m.start());
                let end = byte_to_position(content, m.end());
                Violation {
                    policy: self.name().to_string(),
                    description: "Use three-argument open".to_string(),
                    explanation: "Always use three-argument open for safer file handling"
                        .to_string(),
                    severity: self.severity(),
                    range: perl_parser_core::position::Range { start, end },
                    file: String::new(),
                }
            })
            .collect()
    }
}

/// Prohibit bareword filehandles in `open`.
struct ProhibitBarewordFileHandles;

impl Policy for ProhibitBarewordFileHandles {
    fn name(&self) -> &str {
        "InputOutput::ProhibitBarewordFileHandles"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        static BAREWORD_OPEN_RE: LazyLock<Result<Regex, regex::Error>> =
            LazyLock::new(|| Regex::new(r"(?m)^\s*open\s*(?:\(\s*)?([A-Za-z_]\w*)\s*,"));

        let Ok(bareword_open_re) = &*BAREWORD_OPEN_RE else {
            return Vec::new();
        };

        bareword_open_re
            .captures_iter(content)
            .filter_map(|caps| {
                let matched = caps.get(1)?;
                let start = byte_to_position(content, matched.start());
                let end = byte_to_position(content, matched.end());
                Some(Violation {
                    policy: self.name().to_string(),
                    description: "Bareword filehandle used".to_string(),
                    explanation: "Use lexical filehandles (my $fh) instead of bareword handles"
                        .to_string(),
                    severity: self.severity(),
                    range: perl_parser_core::position::Range { start, end },
                    file: String::new(),
                })
            })
            .collect()
    }
}

impl Default for BuiltInAnalyzer {
    fn default() -> Self {
        Self {
            policies: vec![
                Box::new(RequireUseStrict),
                Box::new(RequireUseWarnings),
                Box::new(RequireThreeArgOpen),
                Box::new(ProhibitBarewordFileHandles),
            ],
        }
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

fn byte_to_position(content: &str, byte: usize) -> Position {
    let byte = byte.min(content.len());
    let mut line: u32 = 0;
    let mut column: u32 = 0;

    for ch in content[..byte].chars() {
        if ch == '\n' {
            line = line.saturating_add(1);
            column = 0;
        } else {
            column = column.saturating_add(1);
        }
    }

    Position { byte, line, column }
}

#[cfg(test)]
mod tests {
    use super::BuiltInAnalyzer;
    use perl_parser_core::Parser;

    #[test]
    fn detects_two_arg_open_without_external_perlcritic() -> Result<(), Box<dyn std::error::Error>>
    {
        let source = "use strict;\nuse warnings;\nopen($fh, $path);\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(violations.iter().any(|v| v.policy == "InputOutput::RequireThreeArgOpen"));
        Ok(())
    }

    #[test]
    fn detects_bareword_filehandle_without_external_perlcritic(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source = "use strict;\nuse warnings;\nopen(FH, '<', $path);\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(violations.iter().any(|v| v.policy == "InputOutput::ProhibitBarewordFileHandles"));
        Ok(())
    }
}
