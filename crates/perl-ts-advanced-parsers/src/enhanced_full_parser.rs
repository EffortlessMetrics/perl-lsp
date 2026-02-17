//! Enhanced full Perl parser with improved heredoc and edge case handling
//!
//! This module provides a complete Perl parser that handles:
//! - All heredoc variants (backtick, escaped, indented, etc.)
//! - DATA/END sections
//! - POD documentation
//! - Format declarations
//! - Context-sensitive parsing

use perl_parser_pest::ParseError;
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use perl_ts_heredoc_parser::enhanced_heredoc_lexer::{
    HeredocDeclaration, process_with_enhanced_heredocs,
};
use perl_ts_heredoc_parser::lexer_adapter::LexerAdapter;
use pest::Parser;
use std::collections::HashMap;
use std::sync::Arc;

/// Enhanced parser with comprehensive edge case support
pub struct EnhancedFullParser {
    /// Stored heredoc declarations
    heredoc_declarations: Vec<HeredocDeclaration>,
    /// Whether we've encountered __DATA__ or __END__
    pub data_section_start: Option<usize>,
    /// POD sections found
    pub pod_sections: Vec<(usize, usize, String)>,
}

impl EnhancedFullParser {
    pub fn new() -> Self {
        Self {
            heredoc_declarations: Vec::new(),
            data_section_start: None,
            pod_sections: Vec::new(),
        }
    }

    /// Parse Perl code with comprehensive preprocessing
    pub fn parse(&mut self, input: &str) -> Result<AstNode, ParseError> {
        // Phase 1: Extract special sections (DATA/END, POD)
        let (main_code, data_content) = self.extract_special_sections(input);

        // Phase 2: Handle enhanced heredocs
        let (heredoc_processed, declarations) = process_with_enhanced_heredocs(&main_code);
        self.heredoc_declarations = declarations;

        // Phase 3: Handle slash disambiguation
        let fully_processed = LexerAdapter::preprocess(&heredoc_processed);

        // Phase 4: Parse with Pest
        let pairs = PerlParser::parse(Rule::program, &fully_processed).map_err(|e| {
            eprintln!("Enhanced parser error: {:?}", e);
            ParseError::ParseFailed
        })?;

        // Phase 5: Build AST
        let mut parser = PureRustPerlParser::new();
        let mut ast = None;
        for pair in pairs {
            ast = parser.build_node(pair).map_err(|_| ParseError::ParseFailed)?;
        }

        // Phase 6: Postprocess and enrich AST
        if let Some(ref mut node) = ast {
            LexerAdapter::postprocess(node);
            self.restore_heredoc_content(node);
            self.add_data_section(node, data_content);
        }

        ast.ok_or(ParseError::ParseFailed)
    }

    /// Extract DATA/END sections and POD from input
    fn extract_special_sections(&mut self, input: &str) -> (String, Option<String>) {
        let lines: Vec<&str> = input.lines().collect();
        let mut main_lines: Vec<&str> = Vec::new();
        let mut data_lines: Vec<&str> = Vec::new();
        let mut in_data_section = false;
        let mut in_pod = false;
        let mut pod_start = 0;
        let mut pod_content = String::new();

        for (i, line) in lines.iter().enumerate() {
            // Check for POD start
            if !in_pod
                && line.starts_with('=')
                && line.len() > 1
                && line.chars().nth(1).is_some_and(|c| c.is_alphabetic())
            {
                in_pod = true;
                pod_start = i;
                pod_content.clear();
                pod_content.push_str(line);
                pod_content.push('\n');
                continue;
            }

            // Check for POD end
            if in_pod {
                pod_content.push_str(line);
                pod_content.push('\n');
                if line.trim() == "=cut" {
                    in_pod = false;
                    self.pod_sections.push((pod_start, i, pod_content.clone()));
                }
                continue;
            }

            // Check for DATA/END section
            if !in_data_section && (line.trim() == "__DATA__" || line.trim() == "__END__") {
                in_data_section = true;
                self.data_section_start = Some(i);
                data_lines.push(line);
                continue;
            }

            if in_data_section {
                data_lines.push(line);
            } else {
                main_lines.push(line);
            }
        }

        let main_code = main_lines.join("\n");
        let data_content = if data_lines.is_empty() { None } else { Some(data_lines.join("\n")) };

        (main_code, data_content)
    }

    /// Restore heredoc content in the AST
    fn restore_heredoc_content(&self, node: &mut AstNode) {
        let placeholder_map: HashMap<String, Arc<str>> = self
            .heredoc_declarations
            .iter()
            .filter_map(|decl| {
                decl.content.as_ref().map(|content| (decl.placeholder_id.clone(), content.clone()))
            })
            .collect();

        self.restore_node_content(node, &placeholder_map);
    }

    #[allow(clippy::only_used_in_recursion)]
    fn restore_node_content(
        &self,
        node: &mut AstNode,
        placeholder_map: &HashMap<String, Arc<str>>,
    ) {
        use AstNode::*;

        match node {
            String(value) => {
                if let Some(content) = placeholder_map.get(value.as_ref()) {
                    *value = content.clone();
                }
            }
            Program(nodes) | Block(nodes) => {
                for n in nodes {
                    self.restore_node_content(n, placeholder_map);
                }
            }
            Statement(inner) => {
                self.restore_node_content(inner, placeholder_map);
            }
            BinaryOp { left, right, .. } => {
                self.restore_node_content(left, placeholder_map);
                self.restore_node_content(right, placeholder_map);
            }
            UnaryOp { operand, .. } => {
                self.restore_node_content(operand, placeholder_map);
            }
            TernaryOp { condition, true_expr, false_expr } => {
                self.restore_node_content(condition, placeholder_map);
                self.restore_node_content(true_expr, placeholder_map);
                self.restore_node_content(false_expr, placeholder_map);
            }
            Assignment { target, value, .. } => {
                self.restore_node_content(target, placeholder_map);
                self.restore_node_content(value, placeholder_map);
            }
            FunctionCall { function, args } => {
                self.restore_node_content(function, placeholder_map);
                for arg in args {
                    self.restore_node_content(arg, placeholder_map);
                }
            }
            MethodCall { object, args, .. } => {
                self.restore_node_content(object, placeholder_map);
                for arg in args {
                    self.restore_node_content(arg, placeholder_map);
                }
            }
            IfStatement { condition, then_block, elsif_clauses, else_block } => {
                self.restore_node_content(condition, placeholder_map);
                self.restore_node_content(then_block, placeholder_map);
                for (cond, block) in elsif_clauses {
                    self.restore_node_content(cond, placeholder_map);
                    self.restore_node_content(block, placeholder_map);
                }
                if let Some(block) = else_block {
                    self.restore_node_content(block, placeholder_map);
                }
            }
            WhileStatement { condition, block, .. } | UntilStatement { condition, block, .. } => {
                self.restore_node_content(condition, placeholder_map);
                self.restore_node_content(block, placeholder_map);
            }
            ForeachStatement { variable, list, block, .. } => {
                if let Some(var) = variable {
                    self.restore_node_content(var, placeholder_map);
                }
                self.restore_node_content(list, placeholder_map);
                self.restore_node_content(block, placeholder_map);
            }
            ArrayAccess { array, index } => {
                self.restore_node_content(array, placeholder_map);
                self.restore_node_content(index, placeholder_map);
            }
            HashAccess { hash, key } => {
                self.restore_node_content(hash, placeholder_map);
                self.restore_node_content(key, placeholder_map);
            }
            List(items) => {
                for item in items {
                    self.restore_node_content(item, placeholder_map);
                }
            }
            ArrayRef(items) | HashRef(items) => {
                for item in items {
                    self.restore_node_content(item, placeholder_map);
                }
            }
            _ => {} // Other node types don't contain nested content
        }
    }

    /// Add DATA section content to AST
    fn add_data_section(&self, node: &mut AstNode, data_content: Option<String>) {
        if let Some(content) = data_content {
            // For now, we'll add it as a special comment node
            // In a real implementation, we'd have a dedicated AST node type
            if let AstNode::Program(nodes) = node {
                nodes.push(AstNode::DataSection(Arc::from(content.as_str())));
            }
        }
    }

    /// Parse and return S-expression format
    pub fn parse_to_sexp(&mut self, input: &str) -> Result<String, ParseError> {
        let ast = self.parse(input)?;
        let parser = PureRustPerlParser::new();
        Ok(parser.to_sexp(&ast))
    }
}

impl Default for EnhancedFullParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_heredoc_parsing() {
        let input = r#"
my $cmd = <<`EOF`;
echo "Hello from shell"
EOF

my $text = <<\LITERAL;
No $interpolation here
LITERAL

print $cmd, $text;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_data_section_extraction() {
        let input = r#"
print "Hello\n";

__DATA__
This is data content
that can be read with <DATA>
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
        assert!(parser.data_section_start.is_some());
    }

    #[test]
    fn test_pod_extraction() {
        let input = r#"
print "Hello\n";

=head1 NAME

Test - A test module

=cut

print "After POD\n";
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
        assert!(!parser.pod_sections.is_empty());
    }
}
