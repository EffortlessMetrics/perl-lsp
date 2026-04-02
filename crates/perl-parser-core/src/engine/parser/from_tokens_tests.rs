//! Tests for `Parser::from_tokens` — pre-lexed token constructor.
//!
//! Verifies that parsing with pre-lexed tokens produces the same AST structure
//! as parsing directly from source. This is the key correctness property for
//! the incremental parsing pipeline.

use super::*;
use perl_tdd_support::must;

/// Lex a source string into parser `Token`s, filtering out EOF.
fn lex_to_tokens(source: &str) -> Vec<Token> {
    let mut stream = TokenStream::new(source);
    let mut tokens = Vec::new();
    loop {
        match stream.next() {
            Ok(t) if t.kind == TokenKind::Eof => break,
            Ok(t) => tokens.push(t),
            Err(_) => break,
        }
    }
    tokens
}

#[test]
fn test_from_tokens_simple_variable_assignment() {
    let source = "my $x = 42;";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST"
    );
}

#[test]
fn test_from_tokens_multiple_statements() {
    let source = "my $x = 42;\nmy $y = 99;\nprint $x + $y;";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST for multiple statements"
    );
}

#[test]
fn test_from_tokens_sub_declaration() {
    let source = "sub greet { my $name = shift; print \"Hello, $name\"; }";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST for sub declaration"
    );
}

#[test]
fn test_from_tokens_control_flow() {
    let source = "if ($x > 10) { print \"big\"; } else { print \"small\"; }";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST for control flow"
    );
}

#[test]
fn test_from_tokens_empty_input() {
    let source = "";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST for empty input"
    );
}

#[test]
fn test_from_tokens_program_statement_count() {
    let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let tokens = lex_to_tokens(source);

    let mut parser = Parser::from_tokens(tokens, source);
    let ast = must(parser.parse());

    match ast.kind {
        NodeKind::Program { ref statements } => {
            assert_eq!(statements.len(), 3, "expected 3 statements");
        }
        _ => panic!("expected Program node"),
    }
}

#[test]
fn test_from_tokens_use_strict() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;";
    let tokens = lex_to_tokens(source);

    let mut parser_from_source = Parser::new(source);
    let ast_source = must(parser_from_source.parse());

    let mut parser_from_tokens = Parser::from_tokens(tokens, source);
    let ast_tokens = must(parser_from_tokens.parse());

    assert_eq!(
        ast_source.to_sexp(),
        ast_tokens.to_sexp(),
        "from_tokens AST must match from-source AST for use strict"
    );
}

#[test]
fn test_from_tokens_does_not_require_cancellation_flag() {
    let source = "my $x = 1;";
    let tokens = lex_to_tokens(source);
    let mut parser = Parser::from_tokens(tokens, source);
    // Should succeed without cancellation flag
    let result = parser.parse();
    assert!(result.is_ok(), "from_tokens parse should succeed");
}
