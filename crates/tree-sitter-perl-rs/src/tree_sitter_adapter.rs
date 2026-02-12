//! Tree-sitter compatibility adapter for edge case handling
//!
//! This module ensures all our advanced parsing features output
//! tree-sitter compatible AST nodes, with diagnostics kept separate.

use crate::anti_pattern_detector::{AntiPattern, Diagnostic};
use crate::partial_parse_ast::ExtendedAstNode;

/// Tree-sitter compatible node types for edge cases
#[derive(Debug, Clone)]
pub enum EdgeCaseNodeType {
    // Standard nodes
    Heredoc,
    HeredocOpener,
    HeredocBody,
    HeredocDelimiter,

    // Edge case nodes (tree-sitter compatible)
    DynamicHeredocDelimiter,
    PhaseDependendHeredoc,
    TiedHandleHeredoc,
    SourceFilteredHeredoc,
    EncodingAffectedHeredoc,

    // Error recovery nodes
    HeredocError,
    UnresolvedDelimiter,
    PartialHeredoc,
}

impl EdgeCaseNodeType {
    /// Convert to tree-sitter node type string
    pub fn as_str(&self) -> &'static str {
        match self {
            // Standard nodes match grammar.js
            EdgeCaseNodeType::Heredoc => "heredoc",
            EdgeCaseNodeType::HeredocOpener => "heredoc_opener",
            EdgeCaseNodeType::HeredocBody => "heredoc_body",
            EdgeCaseNodeType::HeredocDelimiter => "heredoc_delimiter",

            // Edge case nodes use clear naming
            EdgeCaseNodeType::DynamicHeredocDelimiter => "dynamic_heredoc_delimiter",
            EdgeCaseNodeType::PhaseDependendHeredoc => "phase_dependent_heredoc",
            EdgeCaseNodeType::TiedHandleHeredoc => "tied_handle_heredoc",
            EdgeCaseNodeType::SourceFilteredHeredoc => "source_filtered_heredoc",
            EdgeCaseNodeType::EncodingAffectedHeredoc => "encoding_affected_heredoc",

            // Error nodes follow tree-sitter convention
            EdgeCaseNodeType::HeredocError => "ERROR",
            EdgeCaseNodeType::UnresolvedDelimiter => "MISSING",
            EdgeCaseNodeType::PartialHeredoc => "ERROR",
        }
    }
}

/// Tree-sitter compatible AST with separate diagnostics
#[derive(Debug)]
pub struct TreeSitterOutput {
    /// The tree-sitter compatible AST
    pub tree: TreeSitterAST,
    /// Diagnostics kept separate from AST
    pub diagnostics: Vec<TreeSitterDiagnostic>,
    /// Optional metadata for advanced features
    pub metadata: TreeSitterMetadata,
}

#[derive(Debug)]
pub struct TreeSitterAST {
    pub root: TreeSitterNode,
}

#[derive(Debug, Clone)]
pub struct TreeSitterNode {
    pub node_type: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize), // (row, column)
    pub end_point: (usize, usize),
    pub children: Vec<TreeSitterNode>,
    pub is_error: bool,
    pub is_missing: bool,
    pub field_name: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug)]
pub struct TreeSitterDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize),
    pub end_point: (usize, usize),
    pub code: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Default)]
pub struct TreeSitterMetadata {
    pub parse_coverage: f64,
    pub recovery_points: Vec<usize>,
    pub edge_case_count: usize,
    pub phase_transitions: Vec<String>,
}

/// Adapter to convert our extended AST to tree-sitter format
pub struct TreeSitterAdapter;

impl TreeSitterAdapter {
    /// Convert ExtendedAstNode to tree-sitter compatible format
    pub fn convert_to_tree_sitter(
        extended_ast: ExtendedAstNode,
        diagnostics: Vec<Diagnostic>,
        source: &str,
    ) -> TreeSitterOutput {
        let mut ts_diagnostics = Vec::new();
        let mut metadata = TreeSitterMetadata::default();

        // Convert AST
        let root = Self::convert_node(&extended_ast, source, &mut ts_diagnostics, &mut metadata);

        // Convert diagnostics
        for diag in diagnostics {
            ts_diagnostics.push(Self::convert_diagnostic(diag));
        }

        TreeSitterOutput { tree: TreeSitterAST { root }, diagnostics: ts_diagnostics, metadata }
    }

    fn convert_node(
        node: &ExtendedAstNode,
        source: &str,
        diagnostics: &mut Vec<TreeSitterDiagnostic>,
        metadata: &mut TreeSitterMetadata,
    ) -> TreeSitterNode {
        match node {
            ExtendedAstNode::Normal(ast_node) => {
                // Standard nodes remain unchanged
                Self::convert_normal_node(ast_node, source)
            }

            ExtendedAstNode::WithWarning { node, diagnostics: node_diags } => {
                // Convert node normally but add diagnostics
                let ts_node = Self::convert_normal_node(node, source);

                // Add warnings to diagnostic list
                for diag in node_diags {
                    diagnostics.push(Self::convert_diagnostic(diag.clone()));
                }

                ts_node
            }

            ExtendedAstNode::PartialParse { pattern, parsed_fragments, .. } => {
                // Create error node with recoverable children
                metadata.edge_case_count += 1;

                let node_type = match pattern {
                    AntiPattern::DynamicHeredocDelimiter { .. } => {
                        EdgeCaseNodeType::DynamicHeredocDelimiter
                    }
                    AntiPattern::BeginTimeHeredoc { .. } => EdgeCaseNodeType::PhaseDependendHeredoc,
                    AntiPattern::SourceFilterHeredoc { .. } => {
                        EdgeCaseNodeType::SourceFilteredHeredoc
                    }
                    _ => EdgeCaseNodeType::PartialHeredoc,
                };

                TreeSitterNode {
                    node_type: node_type.as_str().to_string(),
                    start_byte: 0, // Would calculate from source
                    end_byte: 0,
                    start_point: (0, 0),
                    end_point: (0, 0),
                    children: parsed_fragments
                        .iter()
                        .map(|f| Self::convert_node(f, source, diagnostics, metadata))
                        .collect(),
                    is_error: true,
                    is_missing: false,
                    field_name: None,
                    text: None,
                }
            }

            ExtendedAstNode::Unparseable { raw_text, .. } => {
                // Create ERROR node
                metadata.edge_case_count += 1;

                TreeSitterNode {
                    node_type: "ERROR".to_string(),
                    start_byte: 0,
                    end_byte: raw_text.len(),
                    start_point: (0, 0),
                    end_point: (0, 0),
                    children: vec![],
                    is_error: true,
                    is_missing: false,
                    field_name: None,
                    text: Some(raw_text.to_string()),
                }
            }

            ExtendedAstNode::RuntimeDependentParse { construct_type, static_parts, .. } => {
                // Create specialized node type
                metadata.edge_case_count += 1;

                let node_type = if construct_type.contains("BEGIN") {
                    EdgeCaseNodeType::PhaseDependendHeredoc
                } else {
                    EdgeCaseNodeType::PartialHeredoc
                };

                TreeSitterNode {
                    node_type: node_type.as_str().to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    start_point: (0, 0),
                    end_point: (0, 0),
                    children: static_parts
                        .iter()
                        .map(|p| Self::convert_node(p, source, diagnostics, metadata))
                        .collect(),
                    is_error: false,  // Not an error, just needs runtime
                    is_missing: true, // Missing runtime information
                    field_name: None,
                    text: None,
                }
            }
        }
    }

    fn convert_normal_node(
        ast_node: &crate::pure_rust_parser::AstNode,
        _source: &str,
    ) -> TreeSitterNode {
        // Convert our pure Rust AST nodes to tree-sitter format
        // This would map to the actual tree-sitter node types from grammar.js
        TreeSitterNode {
            node_type: Self::get_tree_sitter_type(ast_node),
            start_byte: 0, // Would calculate from source positions
            end_byte: 0,
            start_point: (0, 0),
            end_point: (0, 0),
            children: Self::get_children(ast_node)
                .into_iter()
                .map(|child| Self::convert_normal_node(child, _source))
                .collect(),
            is_error: false,
            is_missing: false,
            field_name: None,
            text: Self::get_node_text(ast_node),
        }
    }

    fn get_tree_sitter_type(node: &crate::pure_rust_parser::AstNode) -> String {
        // Map our AST nodes to tree-sitter node types
        use crate::pure_rust_parser::AstNode;

        match node {
            AstNode::Program(_) => "source_file",
            AstNode::Statement(_) => "statement",
            AstNode::Identifier(_) => "identifier",
            AstNode::Number(_) => "number",
            AstNode::String(_) => "string",
            AstNode::List(_) => "list",
            AstNode::ScalarVariable(_) => "scalar_variable",
            AstNode::ArrayVariable(_) => "array_variable",
            AstNode::HashVariable(_) => "hash_variable",
            _ => "unknown",
        }
        .to_string()
    }

    fn get_children(
        node: &crate::pure_rust_parser::AstNode,
    ) -> Vec<&crate::pure_rust_parser::AstNode> {
        // Extract children from composite nodes
        use crate::pure_rust_parser::AstNode;

        match node {
            AstNode::Program(children) => children.iter().collect(),
            AstNode::List(children) => children.iter().collect(),
            _ => vec![],
        }
    }

    fn get_node_text(node: &crate::pure_rust_parser::AstNode) -> Option<String> {
        // Get text content for leaf nodes
        use crate::pure_rust_parser::AstNode;

        match node {
            AstNode::Identifier(s) => Some(s.to_string()),
            AstNode::Number(s) => Some(s.to_string()),
            AstNode::String(s) => Some(s.to_string()),
            AstNode::ScalarVariable(s) => Some(s.to_string()),
            AstNode::ArrayVariable(s) => Some(s.to_string()),
            AstNode::HashVariable(s) => Some(s.to_string()),
            _ => None,
        }
    }

    fn convert_diagnostic(diag: Diagnostic) -> TreeSitterDiagnostic {
        let location = match &diag.pattern {
            AntiPattern::FormatHeredoc { location, .. }
            | AntiPattern::BeginTimeHeredoc { location, .. }
            | AntiPattern::DynamicHeredocDelimiter { location, .. }
            | AntiPattern::SourceFilterHeredoc { location, .. }
            | AntiPattern::RegexCodeBlockHeredoc { location, .. }
            | AntiPattern::EvalStringHeredoc { location, .. }
            | AntiPattern::TiedHandleHeredoc { location, .. } => location,
        };

        TreeSitterDiagnostic {
            severity: match diag.severity {
                crate::anti_pattern_detector::Severity::Error => DiagnosticSeverity::Error,
                crate::anti_pattern_detector::Severity::Warning => DiagnosticSeverity::Warning,
                crate::anti_pattern_detector::Severity::Info => DiagnosticSeverity::Info,
            },
            message: diag.message,
            start_byte: location.offset,
            end_byte: location.offset, // Would calculate actual end
            start_point: (location.line, location.column),
            end_point: (location.line, location.column),
            code: Some(format!("PERL{:03}", Self::get_diagnostic_code(&diag.pattern))),
            source: Some("tree-sitter-perl".to_string()),
        }
    }

    fn get_diagnostic_code(pattern: &AntiPattern) -> u32 {
        match pattern {
            AntiPattern::FormatHeredoc { .. } => 101,
            AntiPattern::BeginTimeHeredoc { .. } => 102,
            AntiPattern::DynamicHeredocDelimiter { .. } => 103,
            AntiPattern::SourceFilterHeredoc { .. } => 104,
            AntiPattern::RegexCodeBlockHeredoc { .. } => 105,
            AntiPattern::EvalStringHeredoc { .. } => 106,
            AntiPattern::TiedHandleHeredoc { .. } => 107,
        }
    }
}

/// Example JSON output format
impl TreeSitterNode {
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;

        let mut obj = json!({
            "type": self.node_type,
            "startPosition": {
                "row": self.start_point.0,
                "column": self.start_point.1
            },
            "endPosition": {
                "row": self.end_point.0,
                "column": self.end_point.1
            },
            "startIndex": self.start_byte,
            "endIndex": self.end_byte,
        });

        if self.is_error {
            obj["isError"] = json!(true);
        }

        if self.is_missing {
            obj["isMissing"] = json!(true);
        }

        if !self.children.is_empty() {
            obj["children"] = json!(self.children.iter().map(|c| c.to_json()).collect::<Vec<_>>());
        }

        if let Some(ref text) = self.text {
            obj["text"] = json!(text);
        }

        obj
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partial_parse_ast::ExtendedAstNode;
    use crate::pure_rust_parser::AstNode;
    use std::sync::Arc;

    #[test]
    fn test_normal_node_conversion() {
        let ast = ExtendedAstNode::Normal(AstNode::Identifier(Arc::from("test")));

        let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "test");

        assert_eq!(output.tree.root.node_type, "identifier");
        assert!(!output.tree.root.is_error);
        assert_eq!(output.diagnostics.len(), 0);
    }

    #[test]
    fn test_error_node_conversion() {
        let ast = ExtendedAstNode::Unparseable {
            pattern: AntiPattern::DynamicHeredocDelimiter {
                location: crate::anti_pattern_detector::Location { line: 1, column: 1, offset: 0 },
                expression: "<<$var".to_string(),
            },
            raw_text: Arc::from("<<$var"),
            reason: "Dynamic delimiter".to_string(),
            diagnostics: vec![],
            recovery_point: 0,
        };

        let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "<<$var");

        assert_eq!(output.tree.root.node_type, "ERROR");
        assert!(output.tree.root.is_error);
        assert_eq!(output.metadata.edge_case_count, 1);
    }
}
