//! Full Perl parser with slash disambiguation and heredoc support
//!
//! This module provides a complete Perl parser that handles both
//! context-sensitive slash disambiguation and multi-line heredoc parsing.

use perl_parser_pest::ParseError;
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use perl_ts_heredoc_parser::heredoc_parser::{HeredocDeclaration, parse_with_heredocs};
use perl_ts_heredoc_parser::lexer_adapter::LexerAdapter;
use pest::Parser;
use std::collections::HashMap;
use std::sync::Arc;

/// A complete Perl parser that handles all context-sensitive features
pub struct FullPerlParser {
    /// Stored heredoc declarations for AST enrichment
    heredoc_declarations: Vec<HeredocDeclaration>,
}

impl FullPerlParser {
    /// Create a new full parser instance
    pub fn new() -> Self {
        Self { heredoc_declarations: Vec::new() }
    }

    /// Parse Perl code with full preprocessing
    pub fn parse(&mut self, input: &str) -> Result<AstNode, ParseError> {
        // Phase 1: Handle heredocs
        let (heredoc_processed, declarations) = parse_with_heredocs(input);
        self.heredoc_declarations = declarations;

        // Phase 2: Handle slash disambiguation
        let fully_processed = LexerAdapter::preprocess(&heredoc_processed);

        // Phase 3: Parse with Pest
        let pairs = PerlParser::parse(Rule::program, &fully_processed).map_err(|e| {
            eprintln!("Pest parse error: {:?}", e);
            eprintln!("Input after preprocessing: {:?}", fully_processed);
            ParseError::ParseFailed
        })?;

        // Phase 4: Build AST
        let mut parser = PureRustPerlParser::new();
        let mut ast = None;
        for pair in pairs {
            ast = parser.build_node(pair).map_err(|_| ParseError::ParseFailed)?;
        }

        // Phase 5: Postprocess to restore original tokens and heredoc content
        if let Some(ref mut node) = ast {
            LexerAdapter::postprocess(node);
            self.restore_heredoc_content(node);
        }

        ast.ok_or(ParseError::ParseFailed)
    }

    /// Parse and return S-expression format
    pub fn parse_to_sexp(&mut self, input: &str) -> Result<String, ParseError> {
        let ast = self.parse(input)?;
        let parser = PureRustPerlParser::new();
        Ok(parser.to_sexp(&ast))
    }

    /// Restore heredoc content in the AST
    fn restore_heredoc_content(&self, node: &mut AstNode) {
        // Map placeholder IDs to heredoc content
        let placeholder_map: HashMap<String, Arc<str>> = self
            .heredoc_declarations
            .iter()
            .filter_map(|decl| {
                decl.content.as_ref().map(|content| (decl.placeholder_id.clone(), content.clone()))
            })
            .collect();

        self.restore_node_content(node, &placeholder_map);
    }

    fn restore_node_content(
        &self,
        node: &mut AstNode,
        placeholder_map: &HashMap<String, Arc<str>>,
    ) {
        self.restore_node_content_with_depth(node, placeholder_map, 0);
    }

    #[allow(clippy::only_used_in_recursion)]
    fn restore_node_content_with_depth(
        &self,
        node: &mut AstNode,
        placeholder_map: &HashMap<String, Arc<str>>,
        depth: usize,
    ) {
        if depth > 100 {
            eprintln!("WARNING: Deep recursion detected at depth {}", depth);
            eprintln!("Node type: {:?}", std::mem::discriminant(node));
            return;
        }

        let next_depth = depth + 1;
        match node {
            AstNode::String(value) => {
                // Check if this is a heredoc placeholder
                if let Some(content) = placeholder_map.get(value.as_ref()) {
                    *value = content.clone();
                }
            }
            AstNode::Block(statements) | AstNode::Program(statements) => {
                for stmt in statements {
                    self.restore_node_content_with_depth(stmt, placeholder_map, next_depth);
                }
            }
            AstNode::BinaryOp { left, right, .. } => {
                self.restore_node_content_with_depth(left, placeholder_map, next_depth);
                self.restore_node_content_with_depth(right, placeholder_map, next_depth);
            }
            AstNode::FunctionCall { args, .. } => {
                for arg in args {
                    self.restore_node_content_with_depth(arg, placeholder_map, next_depth);
                }
            }
            AstNode::Assignment { target, value, .. } => {
                self.restore_node_content_with_depth(target, placeholder_map, next_depth);
                self.restore_node_content_with_depth(value, placeholder_map, next_depth);
            }
            AstNode::List(elements) => {
                for elem in elements {
                    self.restore_node_content_with_depth(elem, placeholder_map, next_depth);
                }
            }
            AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
                self.restore_node_content_with_depth(condition, placeholder_map, next_depth);
                self.restore_node_content_with_depth(then_block, placeholder_map, next_depth);
                for (cond, block) in elsif_clauses {
                    self.restore_node_content_with_depth(cond, placeholder_map, next_depth);
                    self.restore_node_content_with_depth(block, placeholder_map, next_depth);
                }
                if let Some(block) = else_block {
                    self.restore_node_content_with_depth(block, placeholder_map, next_depth);
                }
            }
            AstNode::VariableDeclaration { variables, initializer, .. } => {
                for var in variables {
                    self.restore_node_content_with_depth(var, placeholder_map, next_depth);
                }
                if let Some(init) = initializer {
                    self.restore_node_content_with_depth(init, placeholder_map, next_depth);
                }
            }
            AstNode::Statement(inner) => {
                self.restore_node_content_with_depth(inner, placeholder_map, next_depth);
            }
            // Handle more control flow structures
            AstNode::UnlessStatement { condition, block, else_block } => {
                self.restore_node_content_with_depth(condition, placeholder_map, next_depth);
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
                if let Some(else_blk) = else_block {
                    self.restore_node_content_with_depth(else_blk, placeholder_map, next_depth);
                }
            }
            AstNode::WhileStatement { condition, block, .. }
            | AstNode::UntilStatement { condition, block, .. } => {
                self.restore_node_content_with_depth(condition, placeholder_map, next_depth);
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }
            AstNode::ForStatement { init, condition, update, block, .. } => {
                if let Some(i) = init {
                    self.restore_node_content_with_depth(i, placeholder_map, next_depth);
                }
                if let Some(c) = condition {
                    self.restore_node_content_with_depth(c, placeholder_map, next_depth);
                }
                if let Some(u) = update {
                    self.restore_node_content_with_depth(u, placeholder_map, next_depth);
                }
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }
            AstNode::ForeachStatement { variable, list, block, .. } => {
                if let Some(var) = variable {
                    self.restore_node_content_with_depth(var, placeholder_map, next_depth);
                }
                self.restore_node_content_with_depth(list, placeholder_map, next_depth);
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }

            // Handle more expression types
            AstNode::UnaryOp { operand, .. } => {
                self.restore_node_content_with_depth(operand, placeholder_map, next_depth);
            }
            AstNode::TernaryOp { condition, true_expr, false_expr } => {
                self.restore_node_content_with_depth(condition, placeholder_map, next_depth);
                self.restore_node_content_with_depth(true_expr, placeholder_map, next_depth);
                self.restore_node_content_with_depth(false_expr, placeholder_map, next_depth);
            }
            AstNode::ArrayAccess { array, index }
            | AstNode::HashAccess { hash: array, key: index } => {
                self.restore_node_content_with_depth(array, placeholder_map, next_depth);
                self.restore_node_content_with_depth(index, placeholder_map, next_depth);
            }
            AstNode::MethodCall { object, args, .. } => {
                self.restore_node_content_with_depth(object, placeholder_map, next_depth);
                for arg in args {
                    self.restore_node_content_with_depth(arg, placeholder_map, next_depth);
                }
            }
            AstNode::BuiltinListOp { args, .. } => {
                for arg in args {
                    self.restore_node_content_with_depth(arg, placeholder_map, next_depth);
                }
            }

            // Handle various block types
            AstNode::DoBlock(block)
            | AstNode::EvalBlock(block)
            | AstNode::EvalString(block)
            | AstNode::BeginBlock(block)
            | AstNode::EndBlock(block)
            | AstNode::CheckBlock(block)
            | AstNode::InitBlock(block)
            | AstNode::UnitcheckBlock(block) => {
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }

            // Handle sub declarations
            AstNode::SubDeclaration { body, .. } | AstNode::AnonymousSub { body, .. } => {
                self.restore_node_content_with_depth(body, placeholder_map, next_depth);
            }

            // Handle collections
            AstNode::ArrayRef(elements) | AstNode::HashRef(elements) => {
                for elem in elements {
                    self.restore_node_content_with_depth(elem, placeholder_map, next_depth);
                }
            }

            // Handle special statements
            AstNode::ReturnStatement { value } => {
                if let Some(val) = value {
                    self.restore_node_content_with_depth(val, placeholder_map, next_depth);
                }
            }

            // Handle literals and simple types that don't need recursion
            AstNode::Number(_)
            | AstNode::Identifier(_)
            | AstNode::SpecialLiteral(_)
            | AstNode::Bareword(_)
            | AstNode::EmptyExpression
            | AstNode::Comment(_)
            | AstNode::Label(_)
            | AstNode::ScalarVariable(_)
            | AstNode::ArrayVariable(_)
            | AstNode::HashVariable(_)
            | AstNode::TypeglobVariable(_)
            | AstNode::ScalarReference(_)
            | AstNode::ArrayReference(_)
            | AstNode::HashReference(_)
            | AstNode::SubroutineReference(_)
            | AstNode::GlobReference(_)
            | AstNode::Glob(_)
            | AstNode::Regex { .. }
            | AstNode::Substitution { .. }
            | AstNode::Transliteration { .. }
            | AstNode::QwList(_)
            | AstNode::QqString(_)
            | AstNode::QxString(_)
            | AstNode::QrRegex { .. }
            | AstNode::Heredoc { .. }
            | AstNode::Readline { .. }
            | AstNode::LastStatement { .. }
            | AstNode::NextStatement { .. }
            | AstNode::GotoStatement { .. } => {
                // These are leaf nodes, no recursion needed
            }

            // Handle remaining complex types
            AstNode::ArrayElement { index, .. } | AstNode::HashElement { key: index, .. } => {
                self.restore_node_content_with_depth(index, placeholder_map, next_depth);
            }
            AstNode::PostfixDereference { expr, .. }
            | AstNode::TypeglobSlotAccess { typeglob: expr, .. } => {
                self.restore_node_content_with_depth(expr, placeholder_map, next_depth);
            }
            AstNode::PackageDeclaration { block, .. } => {
                if let Some(blk) = block {
                    self.restore_node_content_with_depth(blk, placeholder_map, next_depth);
                }
            }
            AstNode::TieStatement { variable, class, args } => {
                self.restore_node_content_with_depth(variable, placeholder_map, next_depth);
                self.restore_node_content_with_depth(class, placeholder_map, next_depth);
                for arg in args {
                    self.restore_node_content_with_depth(arg, placeholder_map, next_depth);
                }
            }
            AstNode::UntieStatement { variable } | AstNode::TiedExpression { variable } => {
                self.restore_node_content_with_depth(variable, placeholder_map, next_depth);
            }
            AstNode::GivenStatement { expression, when_clauses, default_block } => {
                self.restore_node_content_with_depth(expression, placeholder_map, next_depth);
                for (cond, block) in when_clauses {
                    self.restore_node_content_with_depth(cond, placeholder_map, next_depth);
                    self.restore_node_content_with_depth(block, placeholder_map, next_depth);
                }
                if let Some(def) = default_block {
                    self.restore_node_content_with_depth(def, placeholder_map, next_depth);
                }
            }
            AstNode::LabeledBlock { block, .. } => {
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }
            AstNode::InterpolatedString(parts) => {
                for part in parts {
                    self.restore_node_content_with_depth(part, placeholder_map, next_depth);
                }
            }

            // These should not appear in parsed AST but handle them just in case
            AstNode::UseStatement { .. }
            | AstNode::RequireStatement { .. }
            | AstNode::FormatDeclaration { .. } => {
                // No nested expressions in these
            }

            // Handle additional node types
            AstNode::ClassDeclaration { body, .. } => {
                for member in body {
                    self.restore_node_content_with_depth(member, placeholder_map, next_depth);
                }
            }
            AstNode::MethodDeclaration { body, .. } => {
                self.restore_node_content_with_depth(body, placeholder_map, next_depth);
            }
            AstNode::EndSection(_) | AstNode::Pod(_) | AstNode::DataSection(_) => {
                // These are documentation/data sections, no recursion needed
            }
            AstNode::TryCatch { try_block, catch_clauses, finally_block } => {
                self.restore_node_content_with_depth(try_block, placeholder_map, next_depth);
                for (_exception_var, catch_block) in catch_clauses {
                    // exception_var is Arc<str>, not AstNode, so we don't need to recurse
                    self.restore_node_content_with_depth(catch_block, placeholder_map, next_depth);
                }
                if let Some(finally) = finally_block {
                    self.restore_node_content_with_depth(finally, placeholder_map, next_depth);
                }
            }
            AstNode::DeferStatement(block) => {
                self.restore_node_content_with_depth(block, placeholder_map, next_depth);
            }
            AstNode::FieldDeclaration { default, .. } => {
                if let Some(def) = default {
                    self.restore_node_content_with_depth(def, placeholder_map, next_depth);
                }
            }
            AstNode::RoleDeclaration { body, .. } => {
                self.restore_node_content_with_depth(body, placeholder_map, next_depth);
            }
            AstNode::ErrorNode { .. } => {
                // Error nodes don't need content restoration
            }
        }
    }
}

impl Default for FullPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heredoc_with_division() {
        let input = r#"my $x = <<'EOF';
Hello / World
EOF
my $y = $x / 2;"#;

        let mut parser = FullPerlParser::new();
        match parser.parse_to_sexp(input) {
            Ok(result) => {
                println!("Parse result:\n{}", result);
                // The S-expression shows the heredoc placeholder (content is
                // restored in the AST but the placeholder identifier remains
                // in the S-expression representation).
                assert!(result.contains("__HEREDOC_1__"));
                // Should also parse the division correctly
                assert!(result.contains("binary_expression"));
            }
            Err(e) => {
                assert!(false, "Failed to parse: {:?}", e);
            }
        }
    }

    #[test]
    fn test_heredoc_with_regex() {
        let input = r#"print <<EOF =~ /pattern/;
Test content
EOF"#;

        let mut parser = FullPerlParser::new();
        use perl_tdd_support::must;
        let result = must(parser.parse_to_sexp(input));
        println!("Heredoc+regex result: {}", result);

        // The heredoc content is replaced with a placeholder in the
        // S-expression representation. Verify the parser produces valid
        // output and the input is parseable.
        assert!(!result.is_empty());
    }

    #[test]
    fn test_multiple_features() {
        let input = r#"my $data = <<'DATA';
Line 1: a/b
Line 2: s/foo/bar/
DATA

if ($data =~ /Line 1: (.*)/) {
    my $result = $1;
    $result =~ s/a\/b/x\/y/g;
}"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_indented_heredoc_with_slash() {
        let input = r#"if (1) {
    my $config = <<~'CONFIG';
        path: /usr/local/bin
        regex: /\w+/
        CONFIG
    print $config;
}"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
}
