//! Simple recursive descent parser for Perl
//!
//! This demonstrates the token-based approach without complex parser combinators

use crate::context_lexer_simple::ContextLexer;
use crate::simple_token::Token;
use crate::token_ast::AstNode;
use std::sync::Arc;

pub struct SimpleParser<'source> {
    lexer: ContextLexer<'source>,
    current_pos: usize,
}

impl<'source> SimpleParser<'source> {
    pub fn new(input: &'source str) -> Self {
        Self { lexer: ContextLexer::new(input), current_pos: 0 }
    }

    pub fn parse(&mut self) -> Result<AstNode, String> {
        self.parse_statements()
    }

    fn parse_statements(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        let mut statements = Vec::new();

        loop {
            // Skip newlines
            while let Some(Token::Newline) = self.lexer.peek() {
                self.lexer.next();
            }

            if self.lexer.peek().is_none() {
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(AstNode {
            node_type: "program".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: statements,
        })
    }

    fn parse_statement(&mut self) -> Result<AstNode, String> {
        let _start = self.current_pos;

        match self.lexer.peek() {
            Some(Token::My) | Some(Token::Our) | Some(Token::Local) => {
                self.parse_variable_declaration()
            }
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::Sub) => self.parse_subroutine(),
            Some(Token::Return) => self.parse_return_statement(),
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                self.consume_semicolon()?;
                Ok(expr)
            }
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        let decl_type = match self.lexer.next().ok_or("Unexpected EOF".to_string())? {
            Token::My => "my",
            Token::Our => "our",
            Token::Local => "local",
            unexpected => {
                // Error: Unexpected token in variable declaration context
                // Expected one of: my, our, local (Note: 'state' keyword not supported in this parser)
                // This error occurs when the parser encounters an invalid token after parsing
                // statement-level context that requires a variable declaration keyword.
                return Err(format!(
                    "Expected variable declaration keyword (my/our/local), found {:?} at position {}",
                    unexpected, self.current_pos
                ));
            }
        };

        let var = self.parse_variable()?;

        let value = if let Some(Token::Assign) = self.lexer.peek() {
            self.lexer.next(); // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.consume_semicolon()?;

        Ok(AstNode {
            node_type: format!("{}_declaration", decl_type),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: if let Some(val) = value { vec![var, *val] } else { vec![var] },
        })
    }

    fn parse_variable(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        let token = self.lexer.next().ok_or("Unexpected EOF".to_string())?;

        match token {
            Token::ScalarVar => Ok(AstNode {
                node_type: "scalar_variable".to_string(),
                start_position: start,
                end_position: self.current_pos,
                value: Some(Arc::from("value")),
                children: vec![],
            }),
            Token::ArrayVar => Ok(AstNode {
                node_type: "array_variable".to_string(),
                start_position: start,
                end_position: self.current_pos,
                value: Some(Arc::from("value")),
                children: vec![],
            }),
            Token::HashVar => Ok(AstNode {
                node_type: "hash_variable".to_string(),
                start_position: start,
                end_position: self.current_pos,
                value: Some(Arc::from("value")),
                children: vec![],
            }),
            _ => Err(format!("Expected variable, got {:?}", token)),
        }
    }

    fn parse_expression(&mut self) -> Result<AstNode, String> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.lexer.peek() {
                Some(Token::Plus) => "+",
                Some(Token::Minus) => "-",
                _ => break,
            };

            self.lexer.next().ok_or("Unexpected EOF".to_string())?;
            let right = self.parse_multiplicative()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: left.start_position,
                end_position: right.end_position,
                value: Some(Arc::from(op)),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_primary()?;

        loop {
            let op = match self.lexer.peek() {
                Some(Token::Multiply) => "*",
                Some(Token::Divide) => "/",
                Some(Token::Modulo) => "%",
                _ => break,
            };

            self.lexer.next().ok_or("Unexpected EOF".to_string())?;
            let right = self.parse_primary()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: left.start_position,
                end_position: right.end_position,
                value: Some(Arc::from(op)),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;

        match self.lexer.peek().cloned() {
            Some(Token::IntegerLiteral) | Some(Token::FloatLiteral) => {
                self.lexer.next().ok_or("Unexpected EOF".to_string())?;
                Ok(AstNode {
                    node_type: "number".to_string(),
                    start_position: start,
                    end_position: self.current_pos,
                    value: Some(Arc::from("value")),
                    children: vec![],
                })
            }
            Some(Token::StringLiteral) => {
                self.lexer.next().ok_or("Unexpected EOF".to_string())?;
                Ok(AstNode {
                    node_type: "string".to_string(),
                    start_position: start,
                    end_position: self.current_pos,
                    value: Some(Arc::from("value")),
                    children: vec![],
                })
            }
            Some(Token::ScalarVar) | Some(Token::ArrayVar) | Some(Token::HashVar) => {
                self.parse_variable()
            }
            Some(Token::LParen) => {
                self.lexer.next().ok_or("Unexpected EOF".to_string())?; // consume (
                let expr = self.parse_expression()?;
                if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::RParen {
                    return Err("Expected )".to_string());
                }
                Ok(expr)
            }
            _ => Err(format!("Unexpected token: {:?}", self.lexer.peek())),
        }
    }

    fn parse_if_statement(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        self.lexer.next().ok_or("Unexpected EOF".to_string())?; // consume 'if'

        if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::LParen {
            return Err("Expected ( after if".to_string());
        }

        let condition = self.parse_expression()?;

        if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::RParen {
            return Err("Expected ) after condition".to_string());
        }

        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "if_statement".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: vec![condition, body],
        })
    }

    fn parse_while_statement(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        self.lexer.next().ok_or("Unexpected EOF".to_string())?; // consume 'while'

        if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::LParen {
            return Err("Expected ( after while".to_string());
        }

        let condition = self.parse_expression()?;

        if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::RParen {
            return Err("Expected ) after condition".to_string());
        }

        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "while_statement".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: vec![condition, body],
        })
    }

    fn parse_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;

        if self.lexer.next().ok_or("Unexpected EOF".to_string())? != Token::LBrace {
            return Err("Expected {".to_string());
        }

        let mut statements = Vec::new();

        loop {
            // Skip newlines
            while let Some(Token::Newline) = self.lexer.peek() {
                self.lexer.next();
            }

            if let Some(Token::RBrace) = self.lexer.peek() {
                self.lexer.next();
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(AstNode {
            node_type: "block".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: statements,
        })
    }

    fn parse_subroutine(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        self.lexer.next().ok_or("Unexpected EOF".to_string())?; // consume 'sub'

        let name = if let Some(Token::Identifier) = self.lexer.peek() {
            self.lexer.next();
            Some(Arc::from("value"))
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "subroutine".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: name,
            children: vec![body],
        })
    }

    fn parse_return_statement(&mut self) -> Result<AstNode, String> {
        let start = self.current_pos;
        self.lexer.next().ok_or("Unexpected EOF".to_string())?; // consume 'return'

        let value = if !matches!(self.lexer.peek(), Some(Token::Semicolon) | None) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon()?;

        Ok(AstNode {
            node_type: "return_statement".to_string(),
            start_position: start,
            end_position: self.current_pos,
            value: None,
            children: value.map(|v| vec![v]).unwrap_or_default(),
        })
    }

    fn consume_semicolon(&mut self) -> Result<(), String> {
        match self.lexer.next().ok_or("Unexpected EOF".to_string())? {
            Token::Semicolon => Ok(()),
            Token::Newline | Token::Eof => Ok(()), // Perl allows newline as statement terminator
            token => Err(format!("Expected ; or newline, got {:?}", token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_declaration() {
        let input = "my $x = 42;";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);
        assert_eq!(ast.children[0].node_type, "my_declaration");
    }

    #[test]
    fn test_arithmetic() {
        let input = "$a + $b * $c;";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);
        assert_eq!(ast.children[0].node_type, "binary_expression");
        assert_eq!(ast.children[0].value.as_ref().map(|s| s.as_ref()), Some("+"));
    }

    #[test]
    fn test_if_statement() {
        let input = "if ($x) { return 42; }";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);
        assert_eq!(ast.children[0].node_type, "if_statement");
    }
}
