use crate::ast::Node;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: String },
    UnclosedString,
    UnclosedRegex,
    UnclosedBlock,
    MissingSemicolon,
    InvalidSyntax,
    UnclosedParenthesis,
    UnclosedBracket,
    UnclosedBrace,
    UnterminatedHeredoc,
    InvalidVariableName,
    InvalidSubroutineName,
    MissingOperator,
    MissingOperand,
    UnexpectedEof,
}

pub struct ErrorClassifier;

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorClassifier {
    pub fn new() -> Self {
        ErrorClassifier
    }

    pub fn classify(&self, error_node: &Node, source: &str) -> ParseErrorKind {
        // Get the error text if available based on location
        let error_text = {
            let start = error_node.location.start;
            let end = (start + 10).min(source.len()); // Look at next 10 chars
            if start < source.len() && end <= source.len() && start <= end {
                &source[start..end]
            } else {
                ""
            }
        };

        // Check for common patterns - check the entire source for unclosed quotes
        let quote_count = source.matches('"').count();
        let single_quote_count = source.matches('\'').count();
        
        // Check if we have unclosed quotes
        if quote_count % 2 != 0 {
            return ParseErrorKind::UnclosedString;
        }
        if single_quote_count % 2 != 0 {
            return ParseErrorKind::UnclosedString;
        }
        
        // Also check the error text itself
        if error_text.starts_with('"') && !error_text.ends_with('"') {
            return ParseErrorKind::UnclosedString;
        }
        
        if error_text.starts_with('\'') && !error_text.ends_with('\'') {
            return ParseErrorKind::UnclosedString;
        }

        if error_text.starts_with('/') && !error_text.contains("//") {
            // Could be unclosed regex
            if !error_text[1..].contains('/') {
                return ParseErrorKind::UnclosedRegex;
            }
        }

        // Check context around error
        {
            let pos = error_node.location.start;
            let line_start = source[..pos]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let line_end = source[pos..]
                .find('\n')
                .map(|i| pos + i)
                .unwrap_or(source.len());
            
            let line = &source[line_start..line_end];
            
            // Check for missing semicolon
            if !line.trim().is_empty() && !line.trim().ends_with(';') && !line.trim().ends_with('{') && !line.trim().ends_with('}') {
                // Look for common statement patterns
                if line.contains("my ") || line.contains("our ") || line.contains("local ") ||
                   line.contains("print ") || line.contains("say ") || line.contains("return ") {
                    return ParseErrorKind::MissingSemicolon;
                }
            }

            // Check for unclosed delimiters
            let open_parens = line.matches('(').count();
            let close_parens = line.matches(')').count();
            if open_parens > close_parens {
                return ParseErrorKind::UnclosedParenthesis;
            }

            let open_brackets = line.matches('[').count();
            let close_brackets = line.matches(']').count();
            if open_brackets > close_brackets {
                return ParseErrorKind::UnclosedBracket;
            }

            let open_braces = line.matches('{').count();
            let close_braces = line.matches('}').count();
            if open_braces > close_braces {
                return ParseErrorKind::UnclosedBrace;
            }
        }

        // Check if we're at EOF
        if error_node.location.start >= source.len() - 1 {
            return ParseErrorKind::UnexpectedEof;
        }

        // Default to invalid syntax
        ParseErrorKind::InvalidSyntax
    }

    pub fn get_diagnostic_message(&self, kind: &ParseErrorKind) -> String {
        match kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                format!("Expected {} but found {}", expected, found)
            }
            ParseErrorKind::UnclosedString => {
                "Unclosed string literal".to_string()
            }
            ParseErrorKind::UnclosedRegex => {
                "Unclosed regular expression".to_string()
            }
            ParseErrorKind::UnclosedBlock => {
                "Unclosed code block - missing '}'".to_string()
            }
            ParseErrorKind::MissingSemicolon => {
                "Missing semicolon at end of statement".to_string()
            }
            ParseErrorKind::InvalidSyntax => {
                "Invalid syntax".to_string()
            }
            ParseErrorKind::UnclosedParenthesis => {
                "Unclosed parenthesis - missing ')'".to_string()
            }
            ParseErrorKind::UnclosedBracket => {
                "Unclosed bracket - missing ']'".to_string()
            }
            ParseErrorKind::UnclosedBrace => {
                "Unclosed brace - missing '}'".to_string()
            }
            ParseErrorKind::UnterminatedHeredoc => {
                "Unterminated heredoc".to_string()
            }
            ParseErrorKind::InvalidVariableName => {
                "Invalid variable name".to_string()
            }
            ParseErrorKind::InvalidSubroutineName => {
                "Invalid subroutine name".to_string()
            }
            ParseErrorKind::MissingOperator => {
                "Missing operator".to_string()
            }
            ParseErrorKind::MissingOperand => {
                "Missing operand".to_string()
            }
            ParseErrorKind::UnexpectedEof => {
                "Unexpected end of file".to_string()
            }
        }
    }

    pub fn get_suggestion(&self, kind: &ParseErrorKind) -> Option<String> {
        match kind {
            ParseErrorKind::MissingSemicolon => {
                Some("Add a semicolon ';' at the end of the statement".to_string())
            }
            ParseErrorKind::UnclosedString => {
                Some("Add a closing quote to terminate the string".to_string())
            }
            ParseErrorKind::UnclosedParenthesis => {
                Some("Add a closing parenthesis ')'".to_string())
            }
            ParseErrorKind::UnclosedBracket => {
                Some("Add a closing bracket ']'".to_string())
            }
            ParseErrorKind::UnclosedBrace => {
                Some("Add a closing brace '}'".to_string())
            }
            ParseErrorKind::UnclosedRegex => {
                Some("Add a closing delimiter to terminate the regex".to_string())
            }
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, NodeKind, SourceLocation};

    #[test]
    fn test_classify_unclosed_string() {
        let classifier = ErrorClassifier::new();
        let source = r#"my $x = "hello"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap_or_else(|_| {
            Node::new(
                NodeKind::Error { message: "Parse error".to_string() },
                SourceLocation { start: 0, end: source.len() }
            )
        });
        
        // Find error nodes
        let mut errors = vec![];
        find_errors(&ast, &mut errors);
        
        if let Some(error) = errors.first() {
            let kind = classifier.classify(error, source);
            assert_eq!(kind, ParseErrorKind::UnclosedString);
        }
    }

    #[test]
    fn test_classify_missing_semicolon() {
        let classifier = ErrorClassifier::new();
        let source = "my $x = 42\nmy $y = 10";
        
        // Simulate an error node at the end of first line
        let error = Node::new(
            NodeKind::Error { message: "".to_string() },
            SourceLocation { start: 10, end: 11 }
        );
        let kind = classifier.classify(&error, source);
        assert_eq!(kind, ParseErrorKind::MissingSemicolon);
    }

    fn find_errors(node: &Node, errors: &mut Vec<Node>) {
        if matches!(&node.kind, NodeKind::Error { .. }) {
            errors.push(node.clone());
        }
        // Recursively check children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    find_errors(stmt, errors);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    find_errors(stmt, errors);
                }
            }
            _ => {} // Other node types don't have children we need to check
        }
    }
}