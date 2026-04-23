use super::{built_in_quick_fix, insertion_range, QuickFix, Severity, Violation};
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
    if pragma_is_enabled(content, feature) {
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

fn pragma_is_enabled(content: &str, feature: &str) -> bool {
    let mut enabled = false;
    let mut in_pod = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        if in_pod {
            if trimmed.starts_with("=cut") {
                in_pod = false;
            }
            continue;
        }

        if trimmed.starts_with("=pod")
            || trimmed.starts_with("=head")
            || trimmed.starts_with("=over")
            || trimmed.starts_with("=item")
            || trimmed.starts_with("=begin")
            || trimmed.starts_with("=for")
        {
            in_pod = true;
            continue;
        }

        if trimmed.starts_with("__DATA__") || trimmed.starts_with("__END__") {
            break;
        }

        let code_segment = line.split('#').next().unwrap_or_default();
        let code_segment = code_segment.trim();
        if code_segment.is_empty() {
            continue;
        }

        let mut parts = code_segment.split_whitespace();
        let Some(directive) = parts.next() else { continue };
        if directive != "use" && directive != "no" {
            continue;
        }

        let Some(pragma_name) = parts.next() else { continue };
        let pragma_name = pragma_name.trim_end_matches(';');
        if pragma_name != "strict" && pragma_name != "warnings" {
            continue;
        }

        if pragma_name != feature {
            continue;
        }

        enabled = directive == "use";
    }

    enabled
}

#[cfg(test)]
mod tests {
    use super::pragma_is_enabled;

    #[test]
    fn detects_enabled_pragma() {
        let content = "use strict;\nuse warnings;\nmy $x = 1;\n";
        assert!(pragma_is_enabled(content, "strict"));
        assert!(pragma_is_enabled(content, "warnings"));
    }

    #[test]
    fn ignores_comments_and_pod_mentions() {
        let content = r#"
# use strict;
=head1 NAME
This module mentions use warnings; in docs.
=cut
my $x = 1;
"#;
        assert!(!pragma_is_enabled(content, "strict"));
        assert!(!pragma_is_enabled(content, "warnings"));
    }

    #[test]
    fn respects_no_directive_over_use() {
        let content = "use strict;\nno strict;\n";
        assert!(!pragma_is_enabled(content, "strict"));
    }

    #[test]
    fn ignores_data_section() {
        let content = "my $x = 1;\n__DATA__\nuse strict;\n";
        assert!(!pragma_is_enabled(content, "strict"));
    }
}
