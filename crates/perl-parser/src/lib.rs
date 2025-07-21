//! A modern, modular Perl parser built on perl-lexer
//!
//! This crate provides a clean, efficient parser that consumes tokens from
//! the perl-lexer crate and produces a well-structured Abstract Syntax Tree (AST).
//!
//! ## Architecture
//!
//! The parser follows a recursive descent design with operator precedence handling,
//! maintaining a clean separation from the lexing phase. This modular approach
//! enables:
//!
//! - Independent testing of parsing logic
//! - Easy integration with different lexer implementations
//! - Clear error boundaries between lexing and parsing phases
//! - Optimal performance through single-pass parsing
//!
//! ## Example
//!
//! ```rust
//! use perl_parser::Parser;
//! 
//! let code = "my $x = 42;";
//! let mut parser = Parser::new(code);
//! 
//! match parser.parse() {
//!     Ok(ast) => println!("AST: {}", ast.to_sexp()),
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! ```

pub mod ast;
pub mod error;
pub mod parser;
pub mod token_stream;

pub use ast::{Node, NodeKind, SourceLocation};
pub use error::{ParseError, ParseResult};
pub use parser::Parser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = Parser::new("my $x = 42;");
        let result = parser.parse();
        assert!(result.is_ok());
    }
}