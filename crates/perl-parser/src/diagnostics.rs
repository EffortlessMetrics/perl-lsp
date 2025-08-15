//! Diagnostics provider for Perl code analysis
//!
//! This module provides syntax error detection, linting, and code quality checks.

use crate::ast::{Node, NodeKind};
use crate::error::ParseError;
use crate::error_classifier::ErrorClassifier;
use crate::pragma_tracker::PragmaTracker;
use crate::scope_analyzer::{IssueKind, ScopeAnalyzer};
use crate::symbol::{SymbolExtractor, SymbolKind, SymbolTable};

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// A diagnostic message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: (usize, usize),
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub related_information: Vec<RelatedInformation>,
    pub tags: Vec<DiagnosticTag>,
}

/// Related information for a diagnostic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    pub location: (usize, usize),
    pub message: String,
}

/// Tags for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

/// Diagnostics provider
pub struct DiagnosticsProvider {
    symbol_table: SymbolTable,
    _source: String,
    scope_analyzer: ScopeAnalyzer,
    error_classifier: ErrorClassifier,
}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new(ast: &Node, source: String) -> Self {
        let extractor = SymbolExtractor::new();
        let symbol_table = extractor.extract(ast);
        let scope_analyzer = ScopeAnalyzer::new();
        let error_classifier = ErrorClassifier::new();

        Self { symbol_table, _source: source, scope_analyzer, error_classifier }
    }

    /// Get all diagnostics for the document
    pub fn get_diagnostics(
        &self,
        ast: &Node,
        parse_errors: &[ParseError],
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Convert parse errors to diagnostics
        for error in parse_errors {
            diagnostics.push(self.parse_error_to_diagnostic(error));
        }

        // Build pragma map from AST
        let pragma_map = PragmaTracker::build(ast);

        // Run scope analyzer for variable issues with pragma awareness
        let scope_issues = self.scope_analyzer.analyze(ast, source, &pragma_map);
        for issue in scope_issues {
            let severity = match issue.kind {
                IssueKind::UndeclaredVariable
                | IssueKind::VariableRedeclaration
                | IssueKind::DuplicateParameter
                | IssueKind::UnquotedBareword => DiagnosticSeverity::Error,
                IssueKind::VariableShadowing
                | IssueKind::UnusedVariable
                | IssueKind::ParameterShadowsGlobal
                | IssueKind::UnusedParameter => DiagnosticSeverity::Warning,
            };

            let code = match issue.kind {
                IssueKind::UndeclaredVariable => "undeclared-variable",
                IssueKind::UnusedVariable => "unused-variable",
                IssueKind::VariableShadowing => "variable-shadowing",
                IssueKind::VariableRedeclaration => "variable-redeclaration",
                IssueKind::DuplicateParameter => "duplicate-parameter",
                IssueKind::ParameterShadowsGlobal => "parameter-shadows-global",
                IssueKind::UnusedParameter => "unused-parameter",
                IssueKind::UnquotedBareword => "unquoted-bareword",
            };

            // Calculate byte position for the line number
            let line_start = source
                .lines()
                .take(issue.line.saturating_sub(1))
                .map(|l| l.len() + 1)
                .sum::<usize>();

            diagnostics.push(Diagnostic {
                range: (line_start, line_start + issue.variable_name.len()),
                severity,
                code: Some(code.to_string()),
                message: issue.description.clone(),
                related_information: Vec::new(),
                tags: if matches!(
                    issue.kind,
                    IssueKind::UnusedVariable | IssueKind::UnusedParameter
                ) {
                    vec![DiagnosticTag::Unnecessary]
                } else {
                    Vec::new()
                },
            });
        }

        // Check for ERROR nodes in AST and classify them
        self.check_error_nodes(ast, source, &mut diagnostics);

        // Run other linting checks
        self.check_deprecated_syntax(ast, &mut diagnostics);
        self.check_strict_warnings(ast, &mut diagnostics);
        self.check_common_mistakes(ast, &mut diagnostics);

        // De-duplicate diagnostics - remove parse errors that overlap with classified errors
        self.deduplicate_diagnostics(&mut diagnostics);

        diagnostics
    }

    /// Convert a parse error to a diagnostic
    fn parse_error_to_diagnostic(&self, error: &ParseError) -> Diagnostic {
        let message = error.to_string();
        let location = match error {
            ParseError::UnexpectedToken { location, .. } => *location,
            ParseError::SyntaxError { location, .. } => *location,
            _ => 0,
        };

        Diagnostic {
            range: (location, location + 1),
            severity: DiagnosticSeverity::Error,
            code: Some("syntax-error".to_string()),
            message,
            related_information: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Check for deprecated syntax
    fn check_deprecated_syntax(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            match &n.kind {
                // Check for deprecated 'defined @array' or 'defined %hash'
                NodeKind::FunctionCall { name, args } => {
                    if name == "defined" {
                        if let Some(arg) = args.first() {
                            if let NodeKind::Variable { sigil, .. } = &arg.kind {
                                if sigil == "@" || sigil == "%" {
                                    diagnostics.push(Diagnostic {
                                        range: (n.location.start, n.location.end),
                                        severity: DiagnosticSeverity::Warning,
                                        code: Some("deprecated-defined".to_string()),
                                        message: format!(
                                            "Use of 'defined {}variable' is deprecated",
                                            sigil
                                        ),
                                        related_information: vec![RelatedInformation {
                                            location: (arg.location.start, arg.location.end),
                                            message: format!("Use 'if ({}array)' instead", sigil),
                                        }],
                                        tags: vec![DiagnosticTag::Deprecated],
                                    });
                                }
                            }
                        }
                    }
                }

                // Check for deprecated $[ variable
                NodeKind::Variable { sigil, name } => {
                    if sigil == "$" && name == "[" {
                        diagnostics.push(Diagnostic {
                            range: (n.location.start, n.location.start + 2),
                            severity: DiagnosticSeverity::Warning,
                            code: Some("deprecated-array-base".to_string()),
                            message: "Use of '$[' is deprecated and will be removed".to_string(),
                            related_information: Vec::new(),
                            tags: vec![DiagnosticTag::Deprecated],
                        });
                    }
                }

                _ => {}
            }
        });
    }

    /// Check for common strict/warnings issues
    fn check_strict_warnings(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        let mut has_strict = false;
        let mut has_warnings = false;

        // Check if 'use strict' and 'use warnings' are present
        self.walk_node(node, &mut |n| {
            if let NodeKind::Use { module, args: _ } = &n.kind {
                if module == "strict" {
                    has_strict = true;
                } else if module == "warnings" {
                    has_warnings = true;
                }
            }
        });

        // Add diagnostics if missing
        if !has_strict {
            diagnostics.push(Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Information,
                code: Some("missing-strict".to_string()),
                message: "Consider adding 'use strict;' for better error checking".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            });
        }

        if !has_warnings {
            diagnostics.push(Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Information,
                code: Some("missing-warnings".to_string()),
                message: "Consider adding 'use warnings;' for better error detection".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            });
        }
    }

    /// Check for common mistakes
    fn check_common_mistakes(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            match &n.kind {
                // Check for assignment in condition
                NodeKind::If { condition, .. } | NodeKind::While { condition, .. } => {
                    self.check_assignment_in_condition(condition, diagnostics);
                }

                // Check for == or != with undef
                NodeKind::Binary { op, left, right } => {
                    if (op == "==" || op == "!=")
                        && (self.might_be_undef(left) || self.might_be_undef(right))
                    {
                        diagnostics.push(Diagnostic {
                            range: (n.location.start, n.location.end),
                            severity: DiagnosticSeverity::Warning,
                            code: Some("numeric-undef".to_string()),
                            message: format!("Using '{}' with potentially undefined value", op),
                            related_information: vec![RelatedInformation {
                                location: (n.location.start, n.location.end),
                                message: "Consider using 'defined' check or '//' operator"
                                    .to_string(),
                            }],
                            tags: Vec::new(),
                        });
                    }
                }

                _ => {}
            }
        });
    }

    /// Check for assignment in condition (common mistake)
    fn check_assignment_in_condition(&self, condition: &Node, diagnostics: &mut Vec<Diagnostic>) {
        match &condition.kind {
            NodeKind::Binary { op, .. } if op == "=" => {
                diagnostics.push(Diagnostic {
                    range: (condition.location.start, condition.location.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some("assignment-in-condition".to_string()),
                    message: "Assignment in condition - did you mean '=='?".to_string(),
                    related_information: Vec::new(),
                    tags: Vec::new(),
                });
            }
            NodeKind::Assignment { .. } => {
                diagnostics.push(Diagnostic {
                    range: (condition.location.start, condition.location.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some("assignment-in-condition".to_string()),
                    message: "Assignment in condition - did you mean '=='?".to_string(),
                    related_information: Vec::new(),
                    tags: Vec::new(),
                });
            }
            _ => {}
        }
    }

    /// Check if a node might evaluate to undef
    fn might_be_undef(&self, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                // If variable is not defined in scope, it might be undef
                self.symbol_table.find_symbol(name, 0, SymbolKind::ScalarVariable).is_empty()
            }
            NodeKind::Undef => true,
            _ => false,
        }
    }

    /// Walk the AST and call a function for each node
    #[allow(clippy::only_used_in_recursion)]
    fn walk_node<F>(&self, node: &Node, func: &mut F)
    where
        F: FnMut(&Node),
    {
        func(node);

        // Visit children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.walk_node(stmt, func);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.walk_node(stmt, func);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.walk_node(condition, func);
                self.walk_node(then_branch, func);
                for (cond, branch) in elsif_branches {
                    self.walk_node(cond, func);
                    self.walk_node(branch, func);
                }
                if let Some(branch) = else_branch {
                    self.walk_node(branch, func);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.walk_node(condition, func);
                self.walk_node(body, func);
            }
            NodeKind::Binary { left, right, .. } => {
                self.walk_node(left, func);
                self.walk_node(right, func);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.walk_node(arg, func);
                }
            }
            _ => {} // Other nodes don't have children or are handled differently
        }
    }

    /// Check for ERROR nodes in the AST and classify them
    fn check_error_nodes(&self, node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            if let NodeKind::Error { message } = &n.kind {
                let error_kind = self.error_classifier.classify(n, source);
                let diagnostic_message = self.error_classifier.get_diagnostic_message(&error_kind);
                let suggestion = self.error_classifier.get_suggestion(&error_kind);

                let mut full_message = diagnostic_message.clone();
                if !message.is_empty() {
                    full_message.push_str(&format!(": {}", message));
                }
                if let Some(sugg) = suggestion {
                    full_message.push_str(&format!(". {}", sugg));
                }

                let start = n.location.start;
                let end = n.location.end.min(source.len());

                diagnostics.push(Diagnostic {
                    range: (start, end),
                    severity: DiagnosticSeverity::Error,
                    code: Some(format!("parse-error-{:?}", error_kind).to_lowercase()),
                    message: full_message,
                    related_information: Vec::new(),
                    tags: Vec::new(),
                });
            }
        });
    }

    /// De-duplicate diagnostics to avoid reporting the same issue twice
    fn deduplicate_diagnostics(&self, diagnostics: &mut Vec<Diagnostic>) {
        // Sort by range, severity, code, and message
        diagnostics.sort_by(|a, b| {
            a.range
                .0
                .cmp(&b.range.0)
                .then(a.range.1.cmp(&b.range.1))
                .then(a.severity.cmp(&b.severity))
                .then(a.code.cmp(&b.code))
                .then(a.message.cmp(&b.message))
        });

        // Remove only exact duplicates (same range, severity, code, and message)
        diagnostics.dedup_by(|a, b| {
            a.range == b.range
                && a.severity == b.severity
                && a.code == b.code
                && a.message == b.message
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_undefined_variable() {
        let source = r#"
            use strict;
            print $undefined_var;
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        assert!(
            diagnostics.iter().any(|d| d.code == Some("undefined-variable".to_string())
                || d.code == Some("undeclared-variable".to_string())),
            "Expected undefined/undeclared variable diagnostic, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_unused_variable() {
        let source = r#"
            my $unused = 42;
            print "Hello";
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        assert!(diagnostics.iter().any(|d| d.code == Some("unused-variable".to_string())));
    }
}
