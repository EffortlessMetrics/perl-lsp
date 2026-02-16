//! Proof of concept token-based parser
//!
//! This demonstrates how the token-based approach would work,
//! with a simplified implementation that compiles and runs.

use chumsky::prelude::*;
use logos::Logos;
use std::sync::Arc;

/// Simplified token enum for POC
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t]+")]
pub enum Token {
    // Keywords
    #[token("my")]
    My,
    #[token("our")]
    Our,
    #[token("if")]
    If,
    #[token("print")]
    Print,
    
    // Identifiers and literals
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    
    #[regex(r"-?[0-9]+", |lex| lex.slice().to_string())]
    Number(String),
    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    String(String),
    
    // Variables
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    ScalarVar(String),
    
    // Operators
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(">")]
    Gt,
    #[token("<")]
    Lt,
    
    // Delimiters
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    
    #[regex(r"\n")]
    Newline,
    
    // Comments
    #[regex(r"#[^\n]*")]
    Comment,
}

/// Simplified AST for POC
#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Program(Vec<Ast>),
    VarDecl { scope: String, name: String, value: Option<Box<Ast>> },
    If { cond: Box<Ast>, then: Box<Ast> },
    Block(Vec<Ast>),
    Binary { op: String, left: Box<Ast>, right: Box<Ast> },
    Var(String),
    Number(String),
    String(String),
    Print(Box<Ast>),
}

/// Token parser proof of concept
pub struct TokenParserPoc;

impl TokenParserPoc {
    pub fn parse(input: &str) -> Result<Ast, Vec<Simple<Token>>> {
        let tokens: Vec<_> = Token::lexer(input)
            .filter_map(|t| t.ok())
            .collect();
        
        let parser = Self::program();
        parser.parse(tokens)
    }
    
    fn program() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        Self::statement()
            .repeated()
            .map(Ast::Program)
    }
    
    fn statement() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        choice((
            Self::var_decl(),
            Self::if_stmt(),
            Self::print_stmt(),
            Self::expr_stmt(),
        ))
    }
    
    fn var_decl() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        choice((
            just(Token::My).to("my".to_string()),
            just(Token::Our).to("our".to_string()),
        ))
        .then(filter_map(|span, token| match token {
            Token::ScalarVar(name) => Ok(name),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        }))
        .then(
            just(Token::Assign)
                .ignore_then(Self::expr())
                .or_not()
        )
        .then_ignore(just(Token::Semicolon).or_not())
        .map(|((scope, name), value)| Ast::VarDecl {
            scope,
            name,
            value: value.map(Box::new),
        })
    }
    
    fn if_stmt() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        just(Token::If)
            .ignore_then(just(Token::LeftParen).or_not())
            .ignore_then(Self::expr())
            .then_ignore(just(Token::RightParen).or_not())
            .then(Self::block())
            .map(|(cond, then)| Ast::If {
                cond: Box::new(cond),
                then: Box::new(then),
            })
    }
    
    fn print_stmt() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        just(Token::Print)
            .ignore_then(Self::expr())
            .then_ignore(just(Token::Semicolon).or_not())
            .map(|expr| Ast::Print(Box::new(expr)))
    }
    
    fn expr_stmt() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        Self::expr()
            .then_ignore(just(Token::Semicolon).or_not())
    }
    
    fn block() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        just(Token::LeftBrace)
            .ignore_then(Self::statement().repeated())
            .then_ignore(just(Token::RightBrace))
            .map(Ast::Block)
    }
    
    fn expr() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        recursive(|expr| {
            let primary = choice((
                Self::number(),
                Self::string(),
                Self::var(),
                just(Token::LeftParen)
                    .ignore_then(expr.clone())
                    .then_ignore(just(Token::RightParen)),
            ));
            
            // Simple left-associative binary operators
            let op = choice((
                just(Token::Plus).to("+"),
                just(Token::Minus).to("-"),
                just(Token::Star).to("*"),
                just(Token::Slash).to("/"),
                just(Token::Gt).to(">"),
                just(Token::Lt).to("<"),
            ));
            
            primary.clone()
                .then(op.then(primary).repeated())
                .foldl(|left, (op, right)| Ast::Binary {
                    op: op.to_string(),
                    left: Box::new(left),
                    right: Box::new(right),
                })
        })
    }
    
    fn number() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::Number(n) => Ok(Ast::Number(n)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }
    
    fn string() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::String(s) => Ok(Ast::String(s)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }
    
    fn var() -> impl Parser<Token, Ast, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::ScalarVar(name) => Ok(Ast::Var(name)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }
}

/// Convert AST to S-expression for tree-sitter compatibility
impl Ast {
    pub fn to_sexp(&self) -> String {
        match self {
            Ast::Program(stmts) => {
                let children = stmts.iter()
                    .map(|s| s.to_sexp())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(program {})", children)
            }
            Ast::VarDecl { scope, name, value } => {
                if let Some(val) = value {
                    format!("(variable_declaration ({} {}) {})", scope, name, val.to_sexp())
                } else {
                    format!("(variable_declaration ({} {}))", scope, name)
                }
            }
            Ast::If { cond, then } => {
                format!("(if_statement {} {})", cond.to_sexp(), then.to_sexp())
            }
            Ast::Block(stmts) => {
                let children = stmts.iter()
                    .map(|s| s.to_sexp())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(block {})", children)
            }
            Ast::Binary { op, left, right } => {
                format!("(binary_expression {} {} {})", op, left.to_sexp(), right.to_sexp())
            }
            Ast::Var(name) => format!("(scalar_variable {})", name),
            Ast::Number(n) => format!("(number {})", n),
            Ast::String(s) => format!("(string {})", s),
            Ast::Print(expr) => format!("(print_statement {})", expr.to_sexp()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_parsing() {
        let code = "my $x = 42;";
        let ast_res = TokenParserPoc::parse(code);
        assert!(ast_res.is_ok());
        let ast = ast_res.unwrap_or_else(|_| unreachable!());
        println!("AST: {:?}", ast);
        println!("S-expr: {}", ast.to_sexp());
        
        assert!(matches!(
            ast,
            Ast::Program(ref stmts) if stmts.len() == 1
        ));
    }
    
    #[test]
    fn test_if_statement() {
        let code = "if ($x > 0) { print $x; }";
        let ast_res = TokenParserPoc::parse(code);
        assert!(ast_res.is_ok());
        let ast = ast_res.unwrap_or_else(|_| unreachable!());
        println!("AST: {:?}", ast);
        println!("S-expr: {}", ast.to_sexp());
    }
    
    #[test]
    fn test_expressions() {
        let code = "my $y = $x + 2 * 3;";
        let ast_res = TokenParserPoc::parse(code);
        assert!(ast_res.is_ok());
        let ast = ast_res.unwrap_or_else(|_| unreachable!());
        println!("AST: {:?}", ast);
        println!("S-expr: {}", ast.to_sexp());
    }
}