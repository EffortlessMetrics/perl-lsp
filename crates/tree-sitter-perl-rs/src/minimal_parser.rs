//! Minimal parser implementation demonstrating lexer integration
//!
//! This parser shows how to build a basic AST from the Perl lexer tokens
//! without complex borrowing issues.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::perl_lexer::PerlLexer;
use std::sync::Arc;

/// Minimal Perl parser
pub struct MinimalParser;

impl MinimalParser {
    /// Parse Perl source code into an AST
    pub fn parse(source: &str) -> Node {
        let mut lexer = PerlLexer::new(source);
        let mut statements = Vec::new();
        let mut last_pos = 0;

        // Collect statements
        while let Some(token) = lexer.next_token() {
            use crate::perl_lexer::TokenType;

            // Skip whitespace and comments
            if matches!(token.token_type, TokenType::Whitespace | TokenType::Comment(_)) {
                continue;
            }

            // End of file
            if matches!(token.token_type, TokenType::EOF) {
                last_pos = token.end;
                break;
            }

            // Create simple nodes for demonstration
            let node = match &token.token_type {
                TokenType::Identifier(name) if name.as_ref() == "my" => {
                    // Variable declaration
                    let mut var_name = Arc::from("$unknown");
                    let mut value_node = None;

                    // Look for variable
                    if let Some(var_token) = lexer.next_token() {
                        if let TokenType::Identifier(name) = &var_token.token_type
                            && (name.starts_with('$')
                                || name.starts_with('@')
                                || name.starts_with('%'))
                        {
                            var_name = name.clone();
                        }

                        // Look for assignment
                        if let Some(eq_token) = lexer.next_token()
                            && matches!(eq_token.token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                        {
                            // Get value
                            if let Some(val_token) = lexer.next_token() {
                                value_node = Some(Box::new(match &val_token.token_type {
                                    TokenType::Number(n) => Node::new(
                                        NodeKind::Number { value: n.clone() },
                                        SourceLocation {
                                            start: val_token.start,
                                            end: val_token.end,
                                        },
                                    ),
                                    TokenType::StringLiteral => Node::new(
                                        NodeKind::String { value: val_token.text.clone() },
                                        SourceLocation {
                                            start: val_token.start,
                                            end: val_token.end,
                                        },
                                    ),
                                    _ => Node::new(
                                        NodeKind::Bareword { value: val_token.text.clone() },
                                        SourceLocation {
                                            start: val_token.start,
                                            end: val_token.end,
                                        },
                                    ),
                                }));
                            }
                        }
                    }

                    // Skip semicolon
                    if let Some(semi) = lexer.next_token() {
                        last_pos = semi.end;
                    }

                    if let Some(value) = value_node {
                        Node::new(
                            NodeKind::Assignment {
                                left: Box::new(Node::new(
                                    NodeKind::Variable { name: var_name },
                                    SourceLocation { start: token.start, end: token.end },
                                )),
                                op: crate::token_compat::TokenType::Equal,
                                right: value,
                            },
                            SourceLocation { start: token.start, end: last_pos },
                        )
                    } else {
                        Node::new(
                            NodeKind::VariableDeclaration {
                                declarator: Arc::from("my"),
                                variables: vec![Node::new(
                                    NodeKind::Variable { name: var_name },
                                    SourceLocation { start: token.start, end: token.end },
                                )],
                            },
                            SourceLocation { start: token.start, end: last_pos },
                        )
                    }
                }

                TokenType::Identifier(name) if name.as_ref() == "print" => {
                    // Print statement
                    let mut args = Vec::new();
                    let mut end_pos = token.end;

                    // Collect arguments
                    while let Some(arg_token) = lexer.next_token() {
                        match &arg_token.token_type {
                            TokenType::StringLiteral => {
                                args.push(Node::new(
                                    NodeKind::String { value: arg_token.text.clone() },
                                    SourceLocation { start: arg_token.start, end: arg_token.end },
                                ));
                                end_pos = arg_token.end;
                            }
                            TokenType::Semicolon => {
                                end_pos = arg_token.end;
                                break;
                            }
                            TokenType::EOF => break,
                            _ => {}
                        }
                    }

                    Node::new(
                        NodeKind::FunctionCall { name: Arc::from("print"), args },
                        SourceLocation { start: token.start, end: end_pos },
                    )
                }

                _ => {
                    // Generic node
                    Node::new(
                        NodeKind::Bareword { value: token.text.clone() },
                        SourceLocation { start: token.start, end: token.end },
                    )
                }
            };

            statements.push(node);
        }

        // Return program node
        Node::new(NodeKind::Program { statements }, SourceLocation { start: 0, end: last_pos })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_parse() {
        let source = "my $x = 42;";
        let ast = MinimalParser::parse(source);

        println!("AST: {:#?}", ast);
        println!("\nS-expression:\n{}", ast.to_sexp());

        assert!(matches!(ast.kind, NodeKind::Program { .. }));
    }

    #[test]
    fn test_print_statement() {
        let source = r#"print "Hello, world!";"#;
        let ast = MinimalParser::parse(source);

        println!("\nS-expression:\n{}", ast.to_sexp());

        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            assert!(matches!(statements[0].kind, NodeKind::FunctionCall { .. }));
        }
    }
}
