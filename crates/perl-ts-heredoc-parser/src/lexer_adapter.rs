//! Lexer adapter for tree-sitter-perl
//!
//! This module provides a bridge between the Rust lexer and tree-sitter,
//! handling token preprocessing and postprocessing.

use crate::perl_lexer::{PerlLexer, TokenType};
use perl_parser_pest::AstNode;

pub struct LexerAdapter;

impl LexerAdapter {
    /// Preprocess input string for Perl parsing.
    ///
    /// Uses the context-aware `PerlLexer` to classify ambiguous slash tokens,
    /// then rewrites Division as `_DIV_`, Substitution as `_SUB_/pattern/replacement/flags`,
    /// and Transliteration as `_TRANS_/search/replace/flags` so the PEG grammar can
    /// parse them without context sensitivity.
    pub fn preprocess(input: &str) -> String {
        if input.is_empty() {
            return String::new();
        }

        // Tokenize input
        let mut lexer = PerlLexer::new(input);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            let is_eof = matches!(token.token_type, TokenType::EOF);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        // If tokenization produced nothing useful, return input unchanged
        if tokens.is_empty() {
            return input.to_string();
        }

        let mut output = String::with_capacity(input.len() + 64);
        let mut prev_end = 0;
        let mut skip_next = false;

        for (i, token) in tokens.iter().enumerate() {
            if skip_next {
                skip_next = false;
                prev_end = token.end;
                continue;
            }

            // Copy gap between tokens (whitespace not captured as tokens)
            if token.start > prev_end {
                output.push_str(&input[prev_end..token.start]);
            }

            match &token.token_type {
                TokenType::Division => {
                    // Check if next char in input is '=' (divide-assign /=)
                    if token.end < input.len()
                        && input.as_bytes()[token.end] == b'='
                        && tokens.get(i + 1).is_some_and(|next| next.start == token.end)
                    {
                        output.push_str("_DIV_=");
                        skip_next = true;
                        prev_end = tokens[i + 1].end;
                        continue;
                    }
                    output.push_str("_DIV_");
                }
                TokenType::Substitution => {
                    let text = &input[token.start..token.end];
                    if let Some((pattern, replacement, flags)) = parse_substitution_text(text) {
                        output.push_str("_SUB_/");
                        output.push_str(&escape_slashes(&pattern));
                        output.push('/');
                        output.push_str(&escape_slashes(&replacement));
                        output.push('/');
                        output.push_str(&flags);
                    } else {
                        // Fallback: emit original text
                        output.push_str(text);
                    }
                }
                TokenType::Transliteration => {
                    let text = &input[token.start..token.end];
                    if let Some((search, replace, flags)) = parse_transliteration_text(text) {
                        output.push_str("_TRANS_/");
                        output.push_str(&escape_slashes(&search));
                        output.push('/');
                        output.push_str(&escape_slashes(&replace));
                        output.push('/');
                        output.push_str(&flags);
                    } else {
                        // Fallback: emit original text
                        output.push_str(text);
                    }
                }
                TokenType::EOF => {
                    // Don't emit anything for EOF
                }
                _ => {
                    // All other tokens: emit original text from input
                    output.push_str(&input[token.start..token.end]);
                }
            }

            prev_end = token.end;
        }

        // Copy any remaining input after the last token
        if prev_end < input.len() {
            output.push_str(&input[prev_end..]);
        }

        output
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

/// Grammar rules placeholder (preprocessing markers are handled by the grammar
/// via `_DIV_`, `_SUB_`, and `_TRANS_` rules).
pub const PREPROCESSED_GRAMMAR: &str = "";

// ---------------------------------------------------------------------------
// Preprocessing helpers
// ---------------------------------------------------------------------------

/// Escape forward slashes in a string for slash-delimited form.
/// Preserves already-escaped sequences (`\/`, `\\`, etc.).
fn escape_slashes(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Already escaped — preserve as-is
            result.push('\\');
            result.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'/' {
            result.push_str("\\/");
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

/// Extract (pattern, replacement, flags) from substitution token text
/// like `s{foo}{bar}g` or `s/foo/bar/g`.
fn parse_substitution_text(text: &str) -> Option<(String, String, String)> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes[0] != b's' {
        return None;
    }
    if bytes.len() < 2 {
        return None;
    }

    let delimiter = bytes[1] as char;
    let (open, close) = paired_delimiters(delimiter);

    if open == close {
        parse_simple_three_part(bytes, 1, delimiter)
    } else {
        parse_paired_three_part(bytes, 1, open, close)
    }
}

/// Extract (search, replace, flags) from transliteration token text
/// like `tr{a-z}{A-Z}` or `y/abc/def/`.
fn parse_transliteration_text(text: &str) -> Option<(String, String, String)> {
    let bytes = text.as_bytes();
    let prefix_len = if text.starts_with("tr") {
        2
    } else if text.starts_with('y') {
        1
    } else {
        return None;
    };

    if prefix_len >= bytes.len() {
        return None;
    }

    let delimiter = bytes[prefix_len] as char;
    let (open, close) = paired_delimiters(delimiter);

    if open == close {
        parse_simple_three_part(bytes, prefix_len, delimiter)
    } else {
        parse_paired_three_part(bytes, prefix_len, open, close)
    }
}

/// Return `(open, close)` for paired delimiters; same char for simple ones.
fn paired_delimiters(ch: char) -> (char, char) {
    match ch {
        '{' => ('{', '}'),
        '[' => ('[', ']'),
        '(' => ('(', ')'),
        '<' => ('<', '>'),
        _ => (ch, ch),
    }
}

/// Parse `<delim>part1<delim>part2<delim>flags` for simple (non-paired) delimiters.
/// `start` points at the delimiter byte in `bytes`.
fn parse_simple_three_part(
    bytes: &[u8],
    start: usize,
    delimiter: char,
) -> Option<(String, String, String)> {
    let delim = delimiter as u8;

    // Skip opening delimiter
    let mut pos = start + 1;

    // Scan first part (pattern / search)
    let part1_start = pos;
    while pos < bytes.len() && bytes[pos] != delim {
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
        } else {
            pos += 1;
        }
    }
    let part1 = std::str::from_utf8(&bytes[part1_start..pos]).ok()?.to_string();

    if pos >= bytes.len() {
        return None;
    }
    pos += 1; // skip middle delimiter

    // Scan second part (replacement / replace)
    let part2_start = pos;
    while pos < bytes.len() && bytes[pos] != delim {
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
        } else {
            pos += 1;
        }
    }
    let part2 = std::str::from_utf8(&bytes[part2_start..pos]).ok()?.to_string();

    if pos < bytes.len() {
        pos += 1; // skip closing delimiter
    }

    // Rest is flags
    let flags = std::str::from_utf8(&bytes[pos..]).ok()?.to_string();

    Some((part1, part2, flags))
}

/// Parse `{part1}{part2}flags` for paired delimiters with nesting support.
/// `start` points at the opening delimiter byte in `bytes`.
fn parse_paired_three_part(
    bytes: &[u8],
    start: usize,
    open: char,
    close: char,
) -> Option<(String, String, String)> {
    let open_byte = open as u8;
    let close_byte = close as u8;

    // Skip opening delimiter
    let mut pos = start + 1;

    // Scan first part with depth tracking
    let part1_start = pos;
    let mut depth: usize = 1;
    while pos < bytes.len() && depth > 0 {
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
            continue;
        }
        if bytes[pos] == close_byte {
            depth -= 1;
            if depth == 0 {
                break;
            }
        } else if bytes[pos] == open_byte {
            depth += 1;
        }
        pos += 1;
    }
    if depth != 0 {
        return None;
    }
    let part1 = std::str::from_utf8(&bytes[part1_start..pos]).ok()?.to_string();
    pos += 1; // skip closing delimiter

    // Skip whitespace between paired delimiters
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
        pos += 1;
    }

    // Expect opening delimiter for second part
    if pos >= bytes.len() || bytes[pos] != open_byte {
        return None;
    }
    pos += 1; // skip opening delimiter

    // Scan second part with depth tracking
    let part2_start = pos;
    depth = 1;
    while pos < bytes.len() && depth > 0 {
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
            continue;
        }
        if bytes[pos] == close_byte {
            depth -= 1;
            if depth == 0 {
                break;
            }
        } else if bytes[pos] == open_byte {
            depth += 1;
        }
        pos += 1;
    }
    if depth != 0 {
        return None;
    }
    let part2 = std::str::from_utf8(&bytes[part2_start..pos]).ok()?.to_string();
    pos += 1; // skip closing delimiter

    // Rest is flags
    let flags = std::str::from_utf8(&bytes[pos..]).ok()?.to_string();

    Some((part1, part2, flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_division() {
        let result = LexerAdapter::preprocess("x / 2");
        assert!(result.contains("_DIV_"), "expected _DIV_ in '{}'", result);
    }

    #[test]
    fn test_preprocess_regex_passthrough() {
        // RegexMatch tokens pass through unchanged
        let result = LexerAdapter::preprocess("$x =~ /foo/");
        assert!(result.contains("/foo/"), "regex should pass through: '{}'", result);
    }

    #[test]
    fn test_preprocess_substitution_slashes() {
        let result = LexerAdapter::preprocess("s/foo/bar/g");
        assert_eq!(result, "_SUB_/foo/bar/g");
    }

    #[test]
    fn test_preprocess_substitution_braces() {
        let result = LexerAdapter::preprocess("s{foo}{bar}g");
        assert_eq!(result, "_SUB_/foo/bar/g");
    }

    #[test]
    fn test_preprocess_transliteration_slashes() {
        let result = LexerAdapter::preprocess("tr/a-z/A-Z/");
        assert_eq!(result, "_TRANS_/a-z/A-Z/");
    }

    #[test]
    fn test_preprocess_transliteration_braces() {
        let result = LexerAdapter::preprocess("tr{a-z}{A-Z}");
        assert_eq!(result, "_TRANS_/a-z/A-Z/");
    }

    #[test]
    fn test_preprocess_y_transliteration() {
        let result = LexerAdapter::preprocess("y/abc/def/");
        assert_eq!(result, "_TRANS_/abc/def/");
    }

    #[test]
    fn test_preprocess_division_and_regex() {
        let result = LexerAdapter::preprocess("1/ /abc/");
        assert!(result.contains("_DIV_"), "expected _DIV_ in '{}'", result);
        assert!(result.contains("/abc/"), "expected /abc/ in '{}'", result);
    }

    #[test]
    fn test_preprocess_empty_input() {
        assert_eq!(LexerAdapter::preprocess(""), "");
    }

    #[test]
    fn test_preprocess_no_slashes() {
        let input = "$x = 42";
        let result = LexerAdapter::preprocess(input);
        // Should contain the essential parts (whitespace may differ due to tokenizer gaps)
        assert!(result.contains("$x"), "expected $x in '{}'", result);
        assert!(result.contains("42"), "expected 42 in '{}'", result);
    }

    #[test]
    fn test_escape_slashes() {
        assert_eq!(escape_slashes("foo"), "foo");
        assert_eq!(escape_slashes("a/b"), "a\\/b");
        assert_eq!(escape_slashes("a\\/b"), "a\\/b"); // already escaped
        assert_eq!(escape_slashes("a\\\\b"), "a\\\\b"); // escaped backslash
    }

    #[test]
    fn test_parse_substitution_simple() {
        let parsed = parse_substitution_text("s/foo/bar/g");
        assert!(parsed.is_some(), "expected substitution parse for s/foo/bar/g");
        let Some((p, r, f)) = parsed else {
            return;
        };
        assert_eq!(p, "foo");
        assert_eq!(r, "bar");
        assert_eq!(f, "g");
    }

    #[test]
    fn test_parse_substitution_braces() {
        let parsed = parse_substitution_text("s{foo}{bar}g");
        assert!(parsed.is_some(), "expected substitution parse for brace delimiters");
        let Some((p, r, f)) = parsed else {
            return;
        };
        assert_eq!(p, "foo");
        assert_eq!(r, "bar");
        assert_eq!(f, "g");
    }

    #[test]
    fn test_parse_substitution_nested_braces() {
        let parsed = parse_substitution_text("s{f{o}o}{bar}");
        assert!(parsed.is_some(), "expected substitution parse for s{{f{{o}}o}}{{bar}}");
        let Some((p, r, f)) = parsed else {
            return;
        };
        assert_eq!(p, "f{o}o");
        assert_eq!(r, "bar");
        assert_eq!(f, "");
    }

    #[test]
    fn test_parse_substitution_escaped_delimiter() {
        let parsed = parse_substitution_text("s/foo\\/bar/baz/");
        assert!(parsed.is_some(), "expected substitution parse for escaped delimiter case");
        let Some((p, r, f)) = parsed else {
            return;
        };
        assert_eq!(p, "foo\\/bar");
        assert_eq!(r, "baz");
        assert_eq!(f, "");
    }

    #[test]
    fn test_parse_transliteration_simple() {
        let parsed = parse_transliteration_text("tr/a-z/A-Z/");
        assert!(parsed.is_some(), "expected transliteration parse for tr/a-z/A-Z/");
        let Some((s, r, f)) = parsed else {
            return;
        };
        assert_eq!(s, "a-z");
        assert_eq!(r, "A-Z");
        assert_eq!(f, "");
    }

    #[test]
    fn test_parse_transliteration_y() {
        let parsed = parse_transliteration_text("y/abc/def/");
        assert!(parsed.is_some(), "expected transliteration parse for y/abc/def/");
        let Some((s, r, f)) = parsed else {
            return;
        };
        assert_eq!(s, "abc");
        assert_eq!(r, "def");
        assert_eq!(f, "");
    }

    #[test]
    fn test_parse_transliteration_braces() {
        let parsed = parse_transliteration_text("tr{a-z}{A-Z}");
        assert!(parsed.is_some(), "expected transliteration parse for brace delimiters");
        let Some((s, r, f)) = parsed else {
            return;
        };
        assert_eq!(s, "a-z");
        assert_eq!(r, "A-Z");
        assert_eq!(f, "");
    }
}
