use crate::perl_lexer::{PerlLexer, TokenType};
use std::sync::Arc;

/// Adapter to use PerlLexer with Pest parser
/// This allows us to pre-tokenize the slash ambiguities
pub struct LexerAdapter;

impl LexerAdapter {
    /// Pre-process Perl source to disambiguate slashes
    /// Returns a modified source where:
    /// - Division operators become `_DIV_`
    /// - Regex matches stay as `/pattern/`
    /// - Substitutions become `_SUB_/pat/repl/`
    /// - Transliterations become `_TRANS_/src/dst/`
    pub fn preprocess(input: &str) -> String {
        let mut lexer = PerlLexer::new(input);
        let mut result = String::with_capacity(input.len() * 2);
        let mut last_end = 0;

        while let Some(token) = lexer.next_token() {
            // Add any text between tokens
            if token.start > last_end {
                result.push_str(&input[last_end..token.start]);
            }

            match token.token_type {
                TokenType::EOF => break,
                TokenType::Division => {
                    // Replace division with a unique token
                    result.push_str("_DIV_");
                }
                TokenType::Substitution => {
                    // Mark substitution with special prefix
                    result.push_str("_SUB_");
                    result.push_str(&token.text[1..]); // Skip original 's'
                }
                TokenType::Transliteration => {
                    // Mark transliteration
                    result.push_str("_TRANS_");
                    if token.text.starts_with("tr") {
                        result.push_str(&token.text[2..]); // Skip 'tr'
                    } else {
                        result.push_str(&token.text[1..]); // Skip 'y'
                    }
                }
                TokenType::QuoteRegex => {
                    // Mark qr//
                    result.push_str("_QR_");
                    result.push_str(&token.text[2..]); // Skip original 'qr'
                }
                _ => {
                    // Keep other tokens as-is
                    result.push_str(&token.text);
                }
            }

            last_end = token.end;
        }

        // Add any remaining text
        if last_end < input.len() {
            result.push_str(&input[last_end..]);
        }

        result
    }

    /// Post-process the parsed AST to restore original tokens
    pub fn postprocess(node: &mut crate::pure_rust_parser::AstNode) {
        use crate::pure_rust_parser::AstNode;

        match node {
            AstNode::BinaryOp { op, left, right } => {
                if op.as_ref() == "_DIV_" {
                    *op = Arc::from("/");
                }
                Self::postprocess(left);
                Self::postprocess(right);
            }
            AstNode::Substitution { .. } => {
                // Already handled correctly
            }
            AstNode::Transliteration { .. } => {
                // Already handled correctly
            }
            AstNode::Regex { .. } => {
                // Already handled correctly
            }
            AstNode::Block(nodes)
            | AstNode::List(nodes)
            | AstNode::ClassDeclaration { body: nodes, .. } => {
                for node in nodes {
                    Self::postprocess(node);
                }
            }
            AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
                Self::postprocess(condition);
                Self::postprocess(then_block);
                for (cond, block) in elsif_clauses {
                    Self::postprocess(cond);
                    Self::postprocess(block);
                }
                if let Some(block) = else_block {
                    Self::postprocess(block);
                }
            }
            AstNode::UnlessStatement { condition, block, else_block } => {
                Self::postprocess(condition);
                Self::postprocess(block);
                if let Some(else_b) = else_block {
                    Self::postprocess(else_b);
                }
            }
            AstNode::WhileStatement { condition, block, .. } => {
                Self::postprocess(condition);
                Self::postprocess(block);
            }
            AstNode::UntilStatement { condition, block, .. } => {
                Self::postprocess(condition);
                Self::postprocess(block);
            }
            AstNode::ForStatement { init, condition, update, block, .. } => {
                if let Some(i) = init {
                    Self::postprocess(i);
                }
                if let Some(c) = condition {
                    Self::postprocess(c);
                }
                if let Some(u) = update {
                    Self::postprocess(u);
                }
                Self::postprocess(block);
            }
            AstNode::ForeachStatement { variable, list, block, .. } => {
                if let Some(v) = variable {
                    Self::postprocess(v);
                }
                Self::postprocess(list);
                Self::postprocess(block);
            }
            AstNode::ArrayAccess { array, index }
            | AstNode::HashAccess { hash: array, key: index } => {
                Self::postprocess(array);
                Self::postprocess(index);
            }
            AstNode::Assignment { target, value, .. } => {
                Self::postprocess(target);
                Self::postprocess(value);
            }
            AstNode::FunctionCall { function, args } => {
                Self::postprocess(function);
                for arg in args {
                    Self::postprocess(arg);
                }
            }
            AstNode::UnaryOp { operand, .. } => {
                Self::postprocess(operand);
            }
            AstNode::TernaryOp { condition, true_expr, false_expr } => {
                Self::postprocess(condition);
                Self::postprocess(true_expr);
                Self::postprocess(false_expr);
            }
            AstNode::SubDeclaration { body, .. } | AstNode::AnonymousSub { body, .. } => {
                Self::postprocess(body);
            }
            AstNode::MethodCall { object, args, .. } => {
                Self::postprocess(object);
                for arg in args {
                    Self::postprocess(arg);
                }
            }
            AstNode::ReturnStatement { value } => {
                if let Some(v) = value {
                    Self::postprocess(v);
                }
            }
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
            AstNode::FieldDeclaration { default, .. } => {
                if let Some(d) = default {
                    Self::postprocess(d);
                }
            }
            _ => {
                // Other nodes don't need postprocessing
            }
        }
    }
}

/// Modified grammar rules to handle preprocessed tokens
pub const PREPROCESSED_GRAMMAR: &str = r#"
// Division operator (was /)
division_op = { "÷" }

// Substitution (was s///)
substitution_op = { "ṡ" }

// Transliteration (was tr/// or y///)
transliteration_op = { "ṫ" | "ẏ" }

// Quote regex (was qr//)
quote_regex_op = { "ǫ" }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocessing() {
        // Test division vs regex
        let input = "x / 2 =~ /foo/";
        let processed = LexerAdapter::preprocess(input);
        assert!(processed.contains("÷"));
        assert!(processed.contains("/foo/"));

        // Test substitution
        let input = "s/foo/bar/g";
        let processed = LexerAdapter::preprocess(input);
        assert!(processed.starts_with("ṡ"));

        // Test complex case
        let input = "1/ /abc/ + s{x}{y}";
        let processed = LexerAdapter::preprocess(input);
        assert!(processed.contains("1÷"));
        assert!(processed.contains("/abc/"));
        assert!(processed.contains("ṡ{x}{y}"));
    }
}
