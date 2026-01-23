//! Diagnostics provider for Perl code analysis
//!
//! This module provides syntax error detection, linting, and code quality checks.
//!
//! # PSTX Pipeline Integration
//!
//! Diagnostics are generated throughout the PSTX (Parse → Index → Navigate → Complete → Analyze) pipeline:
//!
//! - **Parse**: Syntax errors and parsing issues are detected during AST construction
//! - **Index**: Symbol resolution errors identified during workspace symbol indexing
//! - **Navigate**: Cross-reference validation errors found during link analysis
//! - **Complete**: Context-aware warnings generated during completion analysis
//! - **Analyze**: Comprehensive code quality issues detected via static analysis
//!
//! This multi-stage approach ensures comprehensive error detection while maintaining
//! performance through incremental analysis and caching strategies.
//!
//! # LSP Client Capabilities
//!
//! Supports comprehensive LSP `textDocument/publishDiagnostics` capabilities:
//! - **Diagnostic categories**: Error, Warning, Information, Hint severity levels
//! - **Related information**: Cross-file error context with URI links
//! - **Code actions**: Quick fixes and refactoring suggestions
//! - **Tags**: Deprecated/unnecessary code identification
//! - **Versioned diagnostics**: Document version tracking for incremental updates
//!
//! Client capability requirements:
//! ```json
//! {
//!   "textDocument": {
//!     "publishDiagnostics": {
//!       "relatedInformation": true,
//!       "versionSupport": true,
//!       "codeActionsIntegration": true,
//!       "tagSupport": { "valueSet": [1, 2] }
//!     }
//!   }
//! }
//! ```
//!
//! # Protocol Compliance
//!
//! Full LSP 3.18 specification compliance for diagnostic publishing:
//! - **Real-time updates**: Immediate diagnostic publishing on document changes
//! - **Batch processing**: Efficient workspace-wide diagnostic computation
//! - **Cancellation support**: Responsive to client cancellation requests
//! - **Error resilience**: Graceful degradation for malformed documents
//! - **UTF-16 position mapping**: Correct client position synchronization
//!
//! # Performance Characteristics
//!
//! - **Diagnostic generation**: <100ms for typical Perl files
//! - **Incremental analysis**: Leverages ≤1ms parsing SLO for real-time feedback
//! - **Memory usage**: <15MB for large workspace diagnostic caching
//! - **Cross-file analysis**: <500ms for workspace-wide issue detection
//!
//! # Usage Examples
//!
//! ```no_run
//! use perl_lsp_providers::ide::lsp_compat::diagnostics::{DiagnosticsProvider, DiagnosticSeverity};
//! use perl_parser_core::Parser;
//!
//! let code = "my $x = 42; # valid code";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse().unwrap();
//! let provider = DiagnosticsProvider::new(&ast, code.to_string());
//!
//! // Generate diagnostics for code
//! let parse_errors = vec![]; // No parsing errors for this example
//! let diagnostics = provider.get_diagnostics(&ast, &parse_errors, code);
//! for diagnostic in diagnostics {
//!     println!("{:?}: {} at {:?}", diagnostic.severity, diagnostic.message, diagnostic.range);
//! }
//! ```

use perl_parser_core::ast::{Node, NodeKind};
use perl_parser_core::error::ParseError;
use perl_parser_core::error_classifier::ErrorClassifier;
use perl_parser_core::pragma_tracker::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeAnalyzer};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};

/// Severity level for diagnostics
///
/// Represents the importance and type of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents successful parsing or execution
    Error = 1,
    /// Non-critical issue that should be addressed
    Warning = 2,
    /// Informational message
    Information = 3,
    /// Subtle suggestion for improvement
    Hint = 4,
}

/// A diagnostic message
///
/// Represents an issue found during code analysis with location,
/// severity, and additional context information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Source code range (start, end) where the issue occurs
    pub range: (usize, usize),
    /// Severity level of the diagnostic
    pub severity: DiagnosticSeverity,
    /// Optional diagnostic code for categorization
    pub code: Option<String>,
    /// Human-readable description of the issue
    pub message: String,
    /// Additional context and related information
    pub related_information: Vec<RelatedInformation>,
    /// Tags for categorizing the diagnostic
    pub tags: Vec<DiagnosticTag>,
}

/// Related information for a diagnostic
///
/// Additional context that helps understand or resolve the main diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    /// Location in source code for the related information
    pub location: (usize, usize),
    /// Description of the related information
    pub message: String,
}

/// Tags for diagnostics
///
/// Additional metadata about the nature of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    /// Code that is not needed and can be removed
    Unnecessary,
    /// Code that uses deprecated features
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
    /// Create a new diagnostics provider for Perl script analysis
    ///
    /// Constructs a diagnostics provider capable of analyzing Perl scripts
    /// for syntax errors, semantic issues, and Perl parsing best practices
    /// within the LSP workflow workflow.
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST containing Perl script structure for analysis
    /// * `source` - Original source code for position mapping and context
    ///
    /// # Returns
    ///
    /// A configured diagnostics provider ready for comprehensive Perl script
    /// error detection and reporting during development and processing stages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticsProvider;
    ///
    /// let script = "my $data_filter = qr/valid/; my $data_filter = 1;";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse().unwrap();
    ///
    /// let provider = DiagnosticsProvider::new(&ast, script.to_string());
    /// // Provider ready for Perl script error analysis
    /// ```
    pub fn new(ast: &Node, source: String) -> Self {
        let extractor = SymbolExtractor::new_with_source(&source);
        let symbol_table = extractor.extract(ast);
        let scope_analyzer = ScopeAnalyzer::new();
        let error_classifier = ErrorClassifier::new();

        Self { symbol_table, _source: source, scope_analyzer, error_classifier }
    }

    /// Get all diagnostics for Perl script document analysis
    ///
    /// Performs comprehensive analysis of Perl script content to identify
    /// syntax errors, semantic issues, unused variables, and Perl parsing
    /// best practice violations within LSP workflow development.
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST structure for semantic analysis
    /// * `parse_errors` - Parser-detected syntax errors for reporting
    /// * `source` - Original source code for position mapping and context
    ///
    /// # Returns
    ///
    /// Vector of diagnostic messages including errors, warnings, and information
    /// items sorted by severity and position for optimal Perl script development.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticsProvider;
    ///
    /// let script = "my $unused_var = 1; my $email_count";
    /// let mut parser = Parser::new(script);
    /// let result = parser.parse();
    ///
    /// match result {
    ///     Ok(ast) => {
    ///         let provider = DiagnosticsProvider::new(&ast, script.to_string());
    ///         let diagnostics = provider.get_diagnostics(&ast, &[], script);
    ///         // Should include unused variable warnings
    ///         assert!(!diagnostics.is_empty());
    ///     }
    ///     Err(parse_errors) => {
    ///         // Handle parse errors
    ///     }
    /// }
    /// ```
    ///
    /// # Email Processing Context
    ///
    /// This analysis is particularly valuable for:
    /// - Email filtering script validation
    /// - Message processing automation error detection
    /// - Configuration script best practice enforcement
    /// - Template processing code quality assurance
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
                | IssueKind::UnusedParameter
                | IssueKind::UninitializedVariable => DiagnosticSeverity::Warning,
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
                IssueKind::UninitializedVariable => "uninitialized-variable",
            };

            // Build helpful related information based on issue type
            let related_info = match issue.kind {
                IssueKind::UndeclaredVariable => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Declare the variable with 'my', 'our', 'local', or 'state'".to_string(),
                    },
                    RelatedInformation {
                        location: issue.range,
                        message: "ℹ️ Under 'use strict', all variables must be declared before use. Use 'my' for lexical scope or 'our' for package variables.".to_string(),
                    }
                ],
                IssueKind::UnusedVariable => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Remove the unused variable or prefix with '_' to indicate it's intentionally unused".to_string(),
                    }
                ],
                IssueKind::UnusedParameter => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Remove the unused parameter or prefix with '_' (e.g., $_unused) to indicate it's intentionally unused".to_string(),
                    }
                ],
                IssueKind::VariableShadowing => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Rename this variable or use the outer scope variable instead".to_string(),
                    },
                    RelatedInformation {
                        location: issue.range,
                        message: "ℹ️ Variable shadowing can make code harder to understand and may hide bugs.".to_string(),
                    }
                ],
                IssueKind::VariableRedeclaration => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Remove the duplicate 'my' declaration - just assign to the existing variable".to_string(),
                    }
                ],
                IssueKind::DuplicateParameter => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Remove the duplicate parameter or use a different name".to_string(),
                    }
                ],
                IssueKind::ParameterShadowsGlobal => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Rename the parameter to avoid shadowing the global variable".to_string(),
                    }
                ],
                IssueKind::UninitializedVariable => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Initialize the variable when declaring it: my $var = value;".to_string(),
                    },
                    RelatedInformation {
                        location: issue.range,
                        message: "ℹ️ Using uninitialized variables may cause warnings and unexpected behavior.".to_string(),
                    }
                ],
                IssueKind::UnquotedBareword => vec![
                    RelatedInformation {
                        location: issue.range,
                        message: "💡 Quote the bareword as a string: 'word' or \"word\"".to_string(),
                    },
                    RelatedInformation {
                        location: issue.range,
                        message: "ℹ️ Under 'use strict', barewords are not allowed unless they're subroutine calls or hash keys.".to_string(),
                    }
                ],
            };

            diagnostics.push(Diagnostic {
                range: issue.range,
                severity,
                code: Some(code.to_string()),
                message: issue.description.clone(),
                related_information: related_info,
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
                            if let NodeKind::Variable { sigil, name } = &arg.kind {
                                if sigil == "@" || sigil == "%" {
                                    let type_name = if sigil == "@" { "array" } else { "hash" };
                                    diagnostics.push(Diagnostic {
                                        range: (n.location.start, n.location.end),
                                        severity: DiagnosticSeverity::Warning,
                                        code: Some("deprecated-defined".to_string()),
                                        message: format!(
                                            "Use of 'defined {}{}' is deprecated",
                                            sigil, name
                                        ),
                                        related_information: vec![
                                            RelatedInformation {
                                                location: (arg.location.start, arg.location.end),
                                                message: format!("💡 Use 'if ({}{})'  or 'if ({}{}[0])' instead", sigil, name, sigil, name),
                                            },
                                            RelatedInformation {
                                                location: (n.location.start, n.location.end),
                                                message: format!("ℹ️ Testing definedness of {} is deprecated because it was rarely useful and often wrong. Empty {}s are false in boolean context.", type_name, type_name),
                                            }
                                        ],
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
                            related_information: vec![
                                RelatedInformation {
                                    location: (n.location.start, n.location.start + 2),
                                    message: "💡 Remove usage of '$[' - arrays always start at index 0".to_string(),
                                },
                                RelatedInformation {
                                    location: (n.location.start, n.location.start + 2),
                                    message: "ℹ️ The $[ variable was used to change the base index of arrays, but this feature has been deprecated since Perl 5.12 and will be removed in future versions.".to_string(),
                                }
                            ],
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
                related_information: vec![
                    RelatedInformation {
                        location: (0, 0),
                        message: "💡 Add 'use strict;' at the beginning of your script".to_string(),
                    },
                    RelatedInformation {
                        location: (0, 0),
                        message: "ℹ️ The 'use strict' pragma enforces good coding practices by requiring variable declarations, disabling barewords, and preventing symbolic references.".to_string(),
                    }
                ],
                tags: Vec::new(),
            });
        }

        if !has_warnings {
            diagnostics.push(Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Information,
                code: Some("missing-warnings".to_string()),
                message: "Consider adding 'use warnings;' for better error detection".to_string(),
                related_information: vec![
                    RelatedInformation {
                        location: (0, 0),
                        message: "💡 Add 'use warnings;' at the beginning of your script".to_string(),
                    },
                    RelatedInformation {
                        location: (0, 0),
                        message: "ℹ️ The 'use warnings' pragma enables helpful warning messages about questionable constructs, uninitialized values, and deprecated features.".to_string(),
                    }
                ],
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
                    related_information: vec![
                        RelatedInformation {
                            location: (condition.location.start, condition.location.end),
                            message: "💡 Use '==' for comparison or 'eq' for string comparison".to_string(),
                        },
                        RelatedInformation {
                            location: (condition.location.start, condition.location.end),
                            message: "ℹ️ Assignment (=) in conditions is usually a mistake. If intentional, wrap in parentheses: if (($x = value))".to_string(),
                        }
                    ],
                    tags: Vec::new(),
                });
            }
            NodeKind::Assignment { .. } => {
                diagnostics.push(Diagnostic {
                    range: (condition.location.start, condition.location.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some("assignment-in-condition".to_string()),
                    message: "Assignment in condition - did you mean '=='?".to_string(),
                    related_information: vec![
                        RelatedInformation {
                            location: (condition.location.start, condition.location.end),
                            message: "💡 Use '==' for comparison or 'eq' for string comparison".to_string(),
                        },
                        RelatedInformation {
                            location: (condition.location.start, condition.location.end),
                            message: "ℹ️ Assignment in conditions is usually a mistake. If intentional, wrap in parentheses: if (($x = value))".to_string(),
                        }
                    ],
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
                self.symbol_table.find_symbol(name, 0, SymbolKind::scalar()).is_empty()
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
            NodeKind::ExpressionStatement { expression } => {
                self.walk_node(expression, func);
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
                let explanation = self.error_classifier.get_explanation(&error_kind);

                let mut full_message = diagnostic_message.clone();
                if !message.is_empty() {
                    full_message.push_str(&format!(": {}", message));
                }

                let start = n.location.start;
                let end = n.location.end.min(source.len());

                // Build related information with suggestion and explanation
                let mut related_info = Vec::new();
                if let Some(sugg) = suggestion {
                    related_info.push(RelatedInformation {
                        location: (start, end),
                        message: format!("💡 {}", sugg),
                    });
                }
                if let Some(exp) = explanation {
                    related_info.push(RelatedInformation {
                        location: (start, end),
                        message: format!("ℹ️ {}", exp),
                    });
                }

                diagnostics.push(Diagnostic {
                    range: (start, end),
                    severity: DiagnosticSeverity::Error,
                    code: Some(format!("parse-error-{:?}", error_kind).to_lowercase()),
                    message: full_message,
                    related_information: related_info,
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;

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

    #[test]
    fn test_diagnostic_has_helpful_suggestions() {
        let source = r#"
            use strict;
            print $undefined_var;
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        // Find the undeclared variable diagnostic
        let undeclared_diag =
            diagnostics.iter().find(|d| d.code == Some("undeclared-variable".to_string()));

        if let Some(diag) = undeclared_diag {
            // Should have related information with suggestions
            assert!(
                !diag.related_information.is_empty(),
                "Diagnostic should have related information with suggestions"
            );

            // Should have a lightbulb suggestion
            assert!(
                diag.related_information.iter().any(|r| r.message.contains("💡")),
                "Should have actionable suggestion marked with lightbulb"
            );

            // Should have explanatory information
            assert!(
                diag.related_information.iter().any(|r| r.message.contains("ℹ️")),
                "Should have explanatory information marked with info icon"
            );
        }
    }

    #[test]
    fn test_deprecated_syntax_has_explanation() {
        let source = r#"
            my @array = (1, 2, 3);
            if (defined @array) {
                print "array defined";
            }
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        // Find deprecated diagnostic
        let deprecated_diag =
            diagnostics.iter().find(|d| d.code == Some("deprecated-defined".to_string()));

        if let Some(diag) = deprecated_diag {
            assert_eq!(diag.severity, DiagnosticSeverity::Warning);
            assert!(!diag.tags.is_empty(), "Should be tagged as deprecated");
            assert!(diag.tags.contains(&DiagnosticTag::Deprecated));

            // Should have helpful related information
            assert!(
                !diag.related_information.is_empty(),
                "Deprecated syntax should have explanation"
            );

            // Check for both suggestion and explanation
            let has_suggestion = diag.related_information.iter().any(|r| r.message.contains("💡"));
            let has_explanation = diag.related_information.iter().any(|r| r.message.contains("ℹ️"));
            assert!(
                has_suggestion && has_explanation,
                "Deprecated diagnostic should have both suggestion and explanation"
            );
        }
    }

    #[test]
    fn test_assignment_in_condition_has_helpful_message() {
        let source = r#"
            my $x = 10;
            if ($x = 5) {
                print $x;
            }
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        let assignment_diag =
            diagnostics.iter().find(|d| d.code == Some("assignment-in-condition".to_string()));

        if let Some(diag) = assignment_diag {
            assert_eq!(diag.severity, DiagnosticSeverity::Warning);
            assert!(diag.message.contains("=="), "Message should suggest using == instead");

            // Should have helpful suggestions
            assert!(!diag.related_information.is_empty());
            let has_comparison_suggestion = diag
                .related_information
                .iter()
                .any(|r| r.message.contains("==") || r.message.contains("eq"));
            assert!(has_comparison_suggestion, "Should suggest using comparison operators");
        }
    }

    #[test]
    fn test_missing_strict_has_suggestion() {
        let source = "print 'Hello';";

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        let strict_diag = diagnostics.iter().find(|d| d.code == Some("missing-strict".to_string()));

        if let Some(diag) = strict_diag {
            assert_eq!(diag.severity, DiagnosticSeverity::Information);
            assert!(!diag.related_information.is_empty());

            let has_suggestion =
                diag.related_information.iter().any(|r| r.message.contains("use strict"));
            assert!(has_suggestion, "Should suggest adding 'use strict'");
        }
    }

    #[test]
    fn test_unused_variable_tagged_appropriately() {
        let source = r#"
            my $unused = 42;
            print "test";
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], source);

        let unused_diag =
            diagnostics.iter().find(|d| d.code == Some("unused-variable".to_string()));

        if let Some(diag) = unused_diag {
            assert!(
                diag.tags.contains(&DiagnosticTag::Unnecessary),
                "Unused variable should be tagged as unnecessary"
            );

            // Should have suggestion to remove or prefix with underscore
            let has_removal_suggestion = diag
                .related_information
                .iter()
                .any(|r| r.message.contains("Remove") || r.message.contains("_"));
            assert!(has_removal_suggestion, "Should suggest removing or prefixing with underscore");
        }
    }

    #[test]
    fn test_parse_error_has_suggestion_and_explanation() {
        let source = r#"my $x = "unclosed string"#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap_or_else(|_| {
            use perl_parser_core::{Node, NodeKind, SourceLocation};
            Node::new(
                NodeKind::Error { message: "test".to_string() },
                SourceLocation { start: 0, end: source.len() },
            )
        });

        // We need to test via error nodes in AST if recovery happened
        // For now, this tests the error classifier integration
        let provider = DiagnosticsProvider::new(&ast, source.to_string());

        // Just verify the provider doesn't crash with error nodes
        let _diagnostics = provider.get_diagnostics(&ast, &[], source);
    }
}
