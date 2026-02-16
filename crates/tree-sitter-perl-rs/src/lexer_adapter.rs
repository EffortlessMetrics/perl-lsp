//! Lexer adapter for tree-sitter-perl
//!
//! This module provides a bridge between the Rust lexer and tree-sitter,
//! handling token preprocessing and postprocessing.

use crate::pure_rust_parser::AstNode;

pub struct LexerAdapter;

impl LexerAdapter {
    /// Preprocess input string for Perl parsing.
    ///
    /// The pest grammar handles division-vs-regex and substitution/transliteration
    /// disambiguation natively via PEG ordered-choice rules, so no preprocessing
    /// is needed. This method returns the input unchanged.
    pub fn preprocess(input: &str) -> String {
        input.to_string()
    }

    /// Postprocess AST to restore original tokens
    pub fn postprocess(node: &mut AstNode) {
        match node {
            AstNode::Program(nodes) | AstNode::Block(nodes) | AstNode::List(nodes) => {
                for child in nodes {
                    Self::postprocess(child);
                }
            }
            AstNode::Statement(inner)
            | AstNode::BeginBlock(inner)
            | AstNode::EndBlock(inner)
            | AstNode::CheckBlock(inner)
            | AstNode::InitBlock(inner)
            | AstNode::UnitcheckBlock(inner)
            | AstNode::DoBlock(inner)
            | AstNode::EvalBlock(inner)
            | AstNode::EvalString(inner) => {
                Self::postprocess(inner);
            }
            AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
                Self::postprocess(condition);
                Self::postprocess(then_block);
                for (cond, block) in elsif_clauses {
                    Self::postprocess(cond);
                    Self::postprocess(block);
                }
                if let Some(else_block) = else_block {
                    Self::postprocess(else_block);
                }
            }
            AstNode::UnlessStatement { condition, block, else_block } => {
                Self::postprocess(condition);
                Self::postprocess(block);
                if let Some(else_block) = else_block {
                    Self::postprocess(else_block);
                }
            }
            AstNode::WhileStatement { condition, block, .. }
            | AstNode::UntilStatement { condition, block, .. } => {
                Self::postprocess(condition);
                Self::postprocess(block);
            }
            AstNode::ForStatement { init, condition, update, block, .. } => {
                if let Some(init) = init {
                    Self::postprocess(init);
                }
                if let Some(condition) = condition {
                    Self::postprocess(condition);
                }
                if let Some(update) = update {
                    Self::postprocess(update);
                }
                Self::postprocess(block);
            }
            AstNode::ForeachStatement { variable, list, block, .. } => {
                if let Some(variable) = variable {
                    Self::postprocess(variable);
                }
                Self::postprocess(list);
                Self::postprocess(block);
            }
            AstNode::SubDeclaration { body, .. } => {
                Self::postprocess(body);
            }
            AstNode::LabeledBlock { block, .. } => {
                Self::postprocess(block);
            }
            AstNode::Assignment { target, value, .. } => {
                Self::postprocess(target);
                Self::postprocess(value);
            }
            AstNode::BinaryOp { left, right, .. } => {
                Self::postprocess(left);
                Self::postprocess(right);
            }
            AstNode::UnaryOp { operand, .. } => {
                Self::postprocess(operand);
            }
            AstNode::TernaryOp { condition, true_expr, false_expr } => {
                Self::postprocess(condition);
                Self::postprocess(true_expr);
                Self::postprocess(false_expr);
            }
            AstNode::FunctionCall { function, args }
            | AstNode::MethodCall { object: function, args, .. } => {
                Self::postprocess(function);
                for arg in args {
                    Self::postprocess(arg);
                }
            }
            AstNode::ArrayElement { index, .. } => {
                Self::postprocess(index);
            }
            AstNode::HashElement { key, .. } => {
                Self::postprocess(key);
            }
            AstNode::ArrayRef(items) | AstNode::HashRef(items) => {
                for item in items {
                    Self::postprocess(item);
                }
            }
            AstNode::VariableDeclaration { initializer: Some(init), .. } => {
                Self::postprocess(init);
            }
            AstNode::ReturnStatement { value: Some(v) } => {
                Self::postprocess(v);
            }
            AstNode::ReturnStatement { value: None } => {}
            AstNode::TryCatch { try_block, catch_clauses, finally_block } => {
                Self::postprocess(try_block);
                for (_, block) in catch_clauses {
                    Self::postprocess(block);
                }
                if let Some(block) = finally_block {
                    Self::postprocess(block);
                }
            }
            AstNode::DeferStatement(block) => {
                Self::postprocess(block);
            }
            AstNode::MethodDeclaration { body, .. } => {
                Self::postprocess(body);
            }
            AstNode::FieldDeclaration { default: Some(d), .. } => {
                Self::postprocess(d);
            }
            AstNode::FieldDeclaration { default: None, .. } => {}
            _ => {
                // Other nodes don't need postprocessing
            }
        }
    }
}

/// Grammar rules placeholder (preprocessing is now a no-op; the pest grammar
/// handles all operator disambiguation natively).
pub const PREPROCESSED_GRAMMAR: &str = "";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocessing_is_passthrough() {
        // preprocess() is now a no-op — the grammar handles disambiguation natively
        let cases = [
            "x / 2 =~ /foo/",
            "s/foo/bar/g",
            "1/ /abc/ + s{x}{y}",
            "tr/a-z/A-Z/",
            "y/abc/def/",
            "qr/pattern/i",
        ];
        for input in &cases {
            assert_eq!(LexerAdapter::preprocess(input), *input);
        }
    }
}
