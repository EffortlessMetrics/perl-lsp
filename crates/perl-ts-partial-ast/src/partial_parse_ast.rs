//! AST nodes for partial parsing with anti-pattern support
//!
//! This module extends the standard AST with nodes that can represent
//! unparseable or problematic constructs while still maintaining a valid tree.

use perl_ts_heredoc_analysis::anti_pattern_detector::{AntiPattern, Diagnostic};
use perl_parser_pest::{AstNode, PureRustPerlParser};
use std::sync::Arc;

/// Extended AST node that can represent partial or problematic parses
#[derive(Debug, Clone)]
pub enum ExtendedAstNode {
    /// Successfully parsed standard node
    Normal(AstNode),

    /// Node with associated warnings but successful parse
    WithWarning { node: Box<AstNode>, diagnostics: Vec<Diagnostic> },

    /// Partially parsed construct with anti-pattern
    PartialParse {
        pattern: AntiPattern,
        raw_text: Arc<str>,
        parsed_fragments: Vec<ExtendedAstNode>,
        diagnostics: Vec<Diagnostic>,
    },

    /// Completely unparseable construct
    Unparseable {
        pattern: AntiPattern,
        raw_text: Arc<str>,
        reason: String,
        diagnostics: Vec<Diagnostic>,
        recovery_point: usize,
    },

    /// Placeholder for content that needs runtime evaluation
    RuntimeDependentParse {
        construct_type: String,
        static_parts: Vec<ExtendedAstNode>,
        dynamic_parts: Vec<DynamicPart>,
        diagnostics: Vec<Diagnostic>,
    },
}

#[derive(Debug, Clone)]
pub struct DynamicPart {
    pub expression: String,
    pub context: RuntimeContext,
    pub fallback_parse: Option<Box<ExtendedAstNode>>,
}

#[derive(Debug, Clone)]
pub enum RuntimeContext {
    BeginBlock,
    EvalString,
    RegexCodeBlock,
    SourceFilter(String),
    TiedHandle(String),
}

/// Parser state for recovery from anti-patterns
#[derive(Debug)]
pub struct RecoveryState {
    pub last_good_position: usize,
    pub depth: usize,
    pub active_anti_patterns: Vec<AntiPattern>,
    pub deferred_heredocs: Vec<(String, usize)>,
}

impl ExtendedAstNode {
    /// Convert to S-expression format for compatibility
    pub fn to_sexp(&self) -> String {
        match self {
            ExtendedAstNode::Normal(node) => PureRustPerlParser::node_to_sexp(node),

            ExtendedAstNode::WithWarning { node, diagnostics } => {
                format!(
                    "(with_warning {} ; {} warnings)",
                    PureRustPerlParser::node_to_sexp(node),
                    diagnostics.len()
                )
            }

            ExtendedAstNode::PartialParse { pattern, parsed_fragments, .. } => {
                let fragments =
                    parsed_fragments.iter().map(|f| f.to_sexp()).collect::<Vec<_>>().join(" ");

                format!("(partial_parse ({:?}) {})", pattern_type(pattern), fragments)
            }

            ExtendedAstNode::Unparseable { pattern, raw_text, reason, .. } => {
                format!(
                    "(unparseable ({:?}) {:?} ; reason: {})",
                    pattern_type(pattern),
                    truncate_string(raw_text, 50),
                    reason
                )
            }

            ExtendedAstNode::RuntimeDependentParse { construct_type, static_parts, .. } => {
                let parts = static_parts.iter().map(|p| p.to_sexp()).collect::<Vec<_>>().join(" ");

                format!("(runtime_dependent {} {})", construct_type, parts)
            }
        }
    }

    /// Extract diagnostics from this node and its children
    pub fn collect_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        match self {
            ExtendedAstNode::Normal(_) => {}

            ExtendedAstNode::WithWarning { diagnostics: diags, .. }
            | ExtendedAstNode::PartialParse { diagnostics: diags, .. }
            | ExtendedAstNode::Unparseable { diagnostics: diags, .. }
            | ExtendedAstNode::RuntimeDependentParse { diagnostics: diags, .. } => {
                diagnostics.extend(diags.clone());
            }
        }

        // Recursively collect from children
        match self {
            ExtendedAstNode::PartialParse { parsed_fragments, .. } => {
                for fragment in parsed_fragments {
                    diagnostics.extend(fragment.collect_diagnostics());
                }
            }
            ExtendedAstNode::RuntimeDependentParse { static_parts, dynamic_parts, .. } => {
                for part in static_parts {
                    diagnostics.extend(part.collect_diagnostics());
                }
                for dyn_part in dynamic_parts {
                    if let Some(fallback) = &dyn_part.fallback_parse {
                        diagnostics.extend(fallback.collect_diagnostics());
                    }
                }
            }
            _ => {}
        }

        diagnostics
    }

    /// Check if this node or any child contains anti-patterns
    pub fn has_anti_patterns(&self) -> bool {
        match self {
            ExtendedAstNode::Normal(_) => false,
            ExtendedAstNode::WithWarning { .. } => true,
            ExtendedAstNode::PartialParse { .. } => true,
            ExtendedAstNode::Unparseable { .. } => true,
            ExtendedAstNode::RuntimeDependentParse { .. } => true,
        }
    }

    /// Try to extract a normal AST node, ignoring warnings
    pub fn as_normal(&self) -> Option<&AstNode> {
        match self {
            ExtendedAstNode::Normal(node) => Some(node),
            ExtendedAstNode::WithWarning { node, .. } => Some(node),
            _ => None,
        }
    }
}

fn pattern_type(pattern: &AntiPattern) -> &'static str {
    match pattern {
        AntiPattern::FormatHeredoc { .. } => "format_heredoc",
        AntiPattern::BeginTimeHeredoc { .. } => "begin_heredoc",
        AntiPattern::SourceFilterHeredoc { .. } => "source_filter_heredoc",
        AntiPattern::DynamicHeredocDelimiter { .. } => "dynamic_delimiter",
        AntiPattern::RegexCodeBlockHeredoc { .. } => "regex_code_heredoc",
        AntiPattern::EvalStringHeredoc { .. } => "eval_heredoc",
        AntiPattern::TiedHandleHeredoc { .. } => "tied_handle_heredoc",
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len { s.to_string() } else { format!("{}...", &s[..max_len]) }
}

/// Builder for creating ExtendedAstNode with proper diagnostics
pub struct ExtendedAstBuilder {
    diagnostics: Vec<Diagnostic>,
}

impl Default for ExtendedAstBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedAstBuilder {
    pub fn new() -> Self {
        Self { diagnostics: Vec::new() }
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn build_normal(self, node: AstNode) -> ExtendedAstNode {
        if self.diagnostics.is_empty() {
            ExtendedAstNode::Normal(node)
        } else {
            ExtendedAstNode::WithWarning { node: Box::new(node), diagnostics: self.diagnostics }
        }
    }

    pub fn build_partial(
        self,
        pattern: AntiPattern,
        raw_text: Arc<str>,
        fragments: Vec<ExtendedAstNode>,
    ) -> ExtendedAstNode {
        ExtendedAstNode::PartialParse {
            pattern,
            raw_text,
            parsed_fragments: fragments,
            diagnostics: self.diagnostics,
        }
    }

    pub fn build_unparseable(
        self,
        pattern: AntiPattern,
        raw_text: Arc<str>,
        reason: String,
        recovery_point: usize,
    ) -> ExtendedAstNode {
        ExtendedAstNode::Unparseable {
            pattern,
            raw_text,
            reason,
            diagnostics: self.diagnostics,
            recovery_point,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_ts_heredoc_analysis::anti_pattern_detector::{Location, Severity};

    #[test]
    fn test_extended_ast_normal() {
        let node = ExtendedAstNode::Normal(AstNode::Identifier(Arc::from("test")));
        assert!(!node.has_anti_patterns());
        assert_eq!(node.to_sexp(), "(identifier test)");
    }

    #[test]
    fn test_extended_ast_with_warning() {
        let diagnostic = Diagnostic {
            severity: Severity::Warning,
            pattern: AntiPattern::FormatHeredoc {
                location: Location { line: 1, column: 1, offset: 0 },
                format_name: "REPORT".to_string(),
                heredoc_delimiter: "END".to_string(),
            },
            message: "Test warning".to_string(),
            explanation: "Test explanation".to_string(),
            suggested_fix: None,
            references: vec![],
        };

        let node = ExtendedAstNode::WithWarning {
            node: Box::new(AstNode::Identifier(Arc::from("test"))),
            diagnostics: vec![diagnostic],
        };

        assert!(node.has_anti_patterns());
        assert!(node.to_sexp().contains("with_warning"));
        assert_eq!(node.collect_diagnostics().len(), 1);
    }

    #[test]
    fn test_builder() {
        let mut builder = ExtendedAstBuilder::new();
        builder.add_diagnostic(Diagnostic {
            severity: Severity::Info,
            pattern: AntiPattern::DynamicHeredocDelimiter {
                location: Location { line: 1, column: 1, offset: 0 },
                expression: "<<$var".to_string(),
            },
            message: "Info".to_string(),
            explanation: "Test".to_string(),
            suggested_fix: None,
            references: vec![],
        });

        let node = builder.build_normal(AstNode::Number(Arc::from("42")));
        assert!(matches!(node, ExtendedAstNode::WithWarning { .. }));
    }
}
