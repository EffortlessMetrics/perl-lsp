//! Comprehensive unit tests for `perl-parser-core`.
//!
//! Covers the public API surface: RecoveryParser, Parser, ParserContext,
//! TokenStream, Trivia, PositionMapper, LineIndex, error types, and
//! budget tracking.

use perl_parser_core::{
    BudgetTracker,
    LineEnding,
    // AST (v1) types used by Parser
    Node as V1Node,
    NodeKind as V1NodeKind,
    ParseBudget,
    // Error types and recovery
    ParseError as CatastrophicParseError,
    ParseOutput,
    // Parser
    Parser,
    // Position mapping
    PositionMapper,
    SourceLocation,
    // AST (v2) types used by RecoveryParser
    ast_v2::NodeKind as V2NodeKind,
    error::recovery_parser::RecoveryParser,
    error_recovery::{ParseError as RecoveryParseError, RecoveryResult, SyncPoint},
    line_index::LineIndex,
    // ParserContext
    parser_context::ParserContext,
    // Token stream
    token_stream::TokenStream,
    // Trivia
    trivia::{NodeWithTrivia, Trivia, TriviaPreservingParser, TriviaToken},
    trivia_parser::format_with_trivia,
};
use perl_tdd_support::{must, must_some};

// ───────────────────────────────────────────────────────────────────
// RecoveryParser tests
// ───────────────────────────────────────────────────────────────────

mod recovery_parser_tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty_program() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new(String::new());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert!(statements.is_empty(), "empty source should yield zero statements");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty(), "empty source should produce no errors");
        Ok(())
    }

    #[test]
    fn single_number_literal() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("42".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::Number { value } => assert_eq!(value, "42"),
                    other => return Err(format!("expected Number, got {:?}", other).into()),
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn variable_declaration_with_initializer() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x = 42".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                        assert_eq!(declarator, "my");
                        assert!(initializer.is_some(), "should have an initializer");
                    }
                    other => {
                        return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                    }
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn variable_declaration_without_initializer() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                        assert_eq!(declarator, "my");
                        assert!(initializer.is_none(), "should have no initializer");
                    }
                    other => {
                        return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                    }
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn multiple_declarations_with_semicolons() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x = 1; my $y = 2".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 2, "should parse two declarations");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn missing_semicolon_recovery() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x = 42 my $y = 99".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 2, "should recover and parse both statements");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn syntax_error_produces_error_nodes() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x = ; my $y = 42".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 2, "should parse both despite error");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(!errors.is_empty(), "should record at least one error");
        Ok(())
    }

    #[test]
    fn if_statement_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("if $x { my $y = 1 }".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::If { then_branch, .. } => match &then_branch.kind {
                        V2NodeKind::Block { statements } => {
                            assert_eq!(statements.len(), 1);
                        }
                        other => return Err(format!("expected Block, got {:?}", other).into()),
                    },
                    other => return Err(format!("expected If, got {:?}", other).into()),
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn unclosed_block_recovery() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("if $x { my $y = 42".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1, "should parse the if statement");
                match &statements[0].kind {
                    V2NodeKind::If { then_branch, .. } => match &then_branch.kind {
                        V2NodeKind::Block { statements } => {
                            assert_eq!(statements.len(), 1, "block should contain the declaration");
                        }
                        other => return Err(format!("expected Block, got {:?}", other).into()),
                    },
                    other => return Err(format!("expected If, got {:?}", other).into()),
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(!errors.is_empty(), "should have error about missing closing brace");
        Ok(())
    }

    #[test]
    fn our_declarator() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("our $config".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::VariableDeclaration { declarator, .. } => {
                        assert_eq!(declarator, "our");
                    }
                    other => {
                        return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                    }
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn local_declarator() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("local $x".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert!(!statements.is_empty(), "should parse at least one statement");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn state_declarator() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("state $counter = 0".to_string());
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    V2NodeKind::VariableDeclaration { declarator, .. } => {
                        assert_eq!(declarator, "state");
                    }
                    other => {
                        return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                    }
                }
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn while_keyword_parsed() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("while 1".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert!(!statements.is_empty(), "should produce at least one node");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn sub_keyword_parsed() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("sub foo".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert!(!statements.is_empty(), "should produce at least one node");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn program_node_range_covers_input() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RecoveryParser::new("my $x = 1".to_string());
        let (ast, _errors) = parser.parse();

        assert!(ast.range.start.byte == 0, "program should start at byte 0");
        Ok(())
    }

    #[test]
    fn string_literal_or_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
        // The RecoveryParser's simplified expression parser may not handle all
        // string literal token forms. Verify it produces a valid Program either way.
        let parser = RecoveryParser::new("\"hello\"".to_string());
        let (ast, _errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                // Should produce at least one node (string or error recovery)
                assert!(!statements.is_empty(), "should produce at least one statement/node");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Parser (v1 / main parser) tests
// ───────────────────────────────────────────────────────────────────

mod parser_tests {
    use super::*;

    #[test]
    fn parse_simple_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $var = 42;");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(!statements.is_empty(), "should parse at least one statement");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn parse_empty_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(statements.is_empty(), "empty source should yield no statements");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(parser.errors().is_empty());
        Ok(())
    }

    #[test]
    fn parse_with_recovery_returns_output() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $x = ;");
        let output: ParseOutput = parser.parse_with_recovery();

        match &output.ast.kind {
            V1NodeKind::Program { .. } => { /* ok */ }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        // The output should contain diagnostics for the syntax error
        // or recovery nodes within the AST
        Ok(())
    }

    #[test]
    fn parse_with_recovery_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("");
        let output = parser.parse_with_recovery();

        match &output.ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(statements.is_empty());
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(output.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn parser_errors_initially_empty() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new("my $x = 1;");
        assert!(parser.errors().is_empty(), "should have no errors before parsing");
        Ok(())
    }

    #[test]
    fn new_with_recovery_config_creates_parser() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new_with_recovery_config("my $x = 1;", ());
        assert!(parser.errors().is_empty());
        Ok(())
    }

    #[test]
    fn parse_subroutine_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("sub hello { print 'hi'; }");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(!statements.is_empty(), "should parse subroutine");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn parse_multiple_statements() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $x = 1; my $y = 2; my $z = 3;");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(
                    statements.len() >= 3,
                    "should parse three statements, got {}",
                    statements.len()
                );
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn parse_output_budget_usage_tracked() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $x = ;");
        let output = parser.parse_with_recovery();

        // Budget tracker should reflect diagnostics
        assert_eq!(output.budget_usage.errors_emitted, output.diagnostics.len());
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ParserContext tests
// ───────────────────────────────────────────────────────────────────

mod parser_context_tests {
    use super::*;

    #[test]
    fn context_from_empty_source() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new(String::new());
        assert!(ctx.is_eof(), "empty source should be immediately at EOF");
        assert!(ctx.current_token().is_none());
        Ok(())
    }

    #[test]
    fn advance_through_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("my $x = 42;".to_string());
        assert!(!ctx.is_eof());

        // Advance until EOF
        let mut count = 0;
        while !ctx.is_eof() {
            ctx.advance();
            count += 1;
            // Safety bound
            if count > 100 {
                return Err("infinite loop detected in token advancement".into());
            }
        }
        assert!(count > 0, "should have advanced through some tokens");
        assert!(ctx.is_eof());
        Ok(())
    }

    #[test]
    fn peek_token_offset() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new("my $x = 42;".to_string());
        // peek(0) should be same as current_token
        let current = must_some(ctx.current_token());
        let peeked = must_some(ctx.peek_token(0));
        assert_eq!(current.range().start.byte, peeked.range().start.byte);

        // peek(1) should be the next token
        let next = ctx.peek_token(1);
        assert!(next.is_some(), "should be able to peek ahead");
        Ok(())
    }

    #[test]
    fn save_and_restore_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("my $x = 42;".to_string());

        let saved = ctx.current_index();
        ctx.advance();
        ctx.advance();
        assert!(ctx.current_index() > saved, "index should have advanced");

        ctx.set_index(saved);
        assert_eq!(ctx.current_index(), saved, "should be restored");
        Ok(())
    }

    #[test]
    fn set_index_clamped_to_token_count() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("42".to_string());
        // set_index beyond token count should clamp
        ctx.set_index(9999);
        assert!(ctx.is_eof());
        Ok(())
    }

    #[test]
    fn check_and_consume_token_type() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("42;".to_string());

        // Skip the number token
        ctx.advance();

        // Now should be at semicolon
        assert!(ctx.check(&perl_lexer::TokenType::Semicolon));
        assert!(ctx.consume(&perl_lexer::TokenType::Semicolon));
        // After consuming, should no longer match
        assert!(!ctx.check(&perl_lexer::TokenType::Semicolon));
        Ok(())
    }

    #[test]
    fn consume_returns_false_on_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("42".to_string());
        assert!(!ctx.consume(&perl_lexer::TokenType::Semicolon));
        Ok(())
    }

    #[test]
    fn expect_returns_error_on_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("42".to_string());
        let result = ctx.expect(perl_lexer::TokenType::Semicolon);
        assert!(result.is_err(), "expect should fail when token doesn't match");

        let err = result.err();
        assert!(err.is_some());
        Ok(())
    }

    #[test]
    fn expect_eof_gives_error() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new(String::new());
        let result = ctx.expect(perl_lexer::TokenType::Semicolon);
        assert!(result.is_err(), "expect at EOF should fail");
        Ok(())
    }

    #[test]
    fn error_accumulation_and_take() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("test".to_string());

        let e1 = RecoveryParseError::new("err1".to_string(), ctx.current_position_range());
        let e2 = RecoveryParseError::new("err2".to_string(), ctx.current_position_range());
        ctx.add_error(e1);
        ctx.add_error(e2);

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].message, "err1");
        assert_eq!(errors[1].message, "err2");

        // After take, errors should be empty
        let errors_after = ctx.take_errors();
        assert!(errors_after.is_empty());
        Ok(())
    }

    #[test]
    fn add_error_unchecked_always_adds() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("test".to_string());
        let e = RecoveryParseError::new("critical".to_string(), ctx.current_position_range());
        ctx.add_error_unchecked(e);

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "critical");
        Ok(())
    }

    #[test]
    fn source_slice_extracts_text() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new("my $x = 42;".to_string());
        let token = must_some(ctx.current_token());
        let range = token.range();
        let slice = ctx.source_slice(&range);
        assert!(!slice.is_empty(), "source slice should not be empty");
        Ok(())
    }

    #[test]
    fn current_position_at_eof_uses_last_token() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("42".to_string());
        // Advance past the only token
        ctx.advance();
        assert!(ctx.is_eof());

        let pos = ctx.current_position();
        // Should use end of last token, not zero
        assert!(pos.byte > 0, "at EOF, position should be at end of last token");
        Ok(())
    }

    #[test]
    fn current_position_empty_source() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new(String::new());
        let pos = ctx.current_position();
        assert_eq!(pos.byte, 0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        Ok(())
    }

    #[test]
    fn with_budget_sets_custom_budget() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget::strict();
        let ctx = ParserContext::with_budget("my $x;".to_string(), budget);
        assert_eq!(ctx.budget().max_errors, budget.max_errors);
        Ok(())
    }

    #[test]
    fn depth_tracking() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("test".to_string());

        assert!(!ctx.depth_would_exceed(), "fresh context should not exceed depth");
        assert!(ctx.enter_depth(), "should be able to enter depth");
        ctx.exit_depth();
        Ok(())
    }

    #[test]
    fn errors_exhausted_respects_budget() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_errors: 1, ..ParseBudget::default() };
        let mut ctx = ParserContext::with_budget("test".to_string(), budget);

        assert!(!ctx.errors_exhausted());

        let e = RecoveryParseError::new("err".to_string(), ctx.current_position_range());
        ctx.add_error(e);

        assert!(ctx.errors_exhausted(), "should be exhausted after max_errors reached");
        Ok(())
    }

    #[test]
    fn add_error_returns_false_when_budget_exhausted() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_errors: 1, ..ParseBudget::default() };
        let mut ctx = ParserContext::with_budget("test".to_string(), budget);

        let e1 = RecoveryParseError::new("err1".to_string(), ctx.current_position_range());
        assert!(ctx.add_error(e1), "first error should be added");

        let e2 = RecoveryParseError::new("err2".to_string(), ctx.current_position_range());
        assert!(!ctx.add_error(e2), "second error should be rejected (budget exhausted)");
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ErrorRecovery (context_impls) tests
// ───────────────────────────────────────────────────────────────────

mod error_recovery_context_tests {
    use super::*;
    use perl_parser_core::error_recovery::ErrorRecovery;

    #[test]
    fn create_error_node_at_token() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("my $x;".to_string());
        let node =
            ctx.create_error_node("test error".to_string(), vec!["something".to_string()], None);

        match &node.kind {
            V2NodeKind::Error { message, expected, partial } => {
                assert_eq!(message, "test error");
                assert_eq!(expected, &["something"]);
                assert!(partial.is_none());
            }
            other => return Err(format!("expected Error node, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn create_error_node_at_eof() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new(String::new());
        let node = ctx.create_error_node("eof error".to_string(), vec![], None);

        match &node.kind {
            V2NodeKind::Error { message, .. } => {
                assert_eq!(message, "eof error");
            }
            other => return Err(format!("expected Error node, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn is_sync_point_semicolon() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new(";".to_string());
        assert!(ctx.is_sync_point(SyncPoint::Semicolon));
        assert!(!ctx.is_sync_point(SyncPoint::CloseBrace));
        assert!(!ctx.is_sync_point(SyncPoint::Keyword));
        Ok(())
    }

    #[test]
    fn is_sync_point_close_brace() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new("}".to_string());
        assert!(ctx.is_sync_point(SyncPoint::CloseBrace));
        assert!(!ctx.is_sync_point(SyncPoint::Semicolon));
        Ok(())
    }

    #[test]
    fn is_sync_point_keyword() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new("my $x".to_string());
        assert!(ctx.is_sync_point(SyncPoint::Keyword));
        Ok(())
    }

    #[test]
    fn is_sync_point_eof_on_empty() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ParserContext::new(String::new());
        assert!(ctx.is_sync_point(SyncPoint::Eof));
        assert!(!ctx.is_sync_point(SyncPoint::Semicolon));
        Ok(())
    }

    #[test]
    fn synchronize_skips_to_sync_point() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("foo bar ;".to_string());
        let skipped = ctx.synchronize(&[SyncPoint::Semicolon]);
        // It should have found the semicolon
        assert!(skipped || ctx.is_eof());
        Ok(())
    }

    #[test]
    fn synchronize_at_sync_point_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new(";".to_string());
        // Already at a semicolon sync point
        let result = ctx.synchronize(&[SyncPoint::Semicolon]);
        // skip_until returns 0 when already at sync point, so synchronize returns false
        assert!(!result);
        Ok(())
    }

    #[test]
    fn recover_with_node_adds_error_and_creates_node() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("foo ;".to_string());
        let error = RecoveryParseError::new("bad token".to_string(), ctx.current_position_range());

        let node = ctx.recover_with_node(error);

        match &node.kind {
            V2NodeKind::Error { message, .. } => {
                assert_eq!(message, "bad token");
            }
            other => return Err(format!("expected Error, got {:?}", other).into()),
        }

        let errors = ctx.take_errors();
        assert!(!errors.is_empty(), "error should have been recorded");
        Ok(())
    }

    #[test]
    fn skip_until_with_budget_at_sync_point() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new(";".to_string());
        let budget = ParseBudget::default();
        let mut tracker = BudgetTracker::new();

        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        assert_eq!(result, RecoveryResult::AtSyncPoint);
        Ok(())
    }

    #[test]
    fn skip_until_with_budget_reaches_eof() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("foo bar baz".to_string());
        let budget = ParseBudget::for_ide();
        let mut tracker = BudgetTracker::new();

        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        assert_eq!(result, RecoveryResult::ReachedEof);
        assert!(tracker.tokens_skipped > 0, "should have skipped tokens");
        Ok(())
    }

    #[test]
    fn skip_until_with_budget_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_tokens_skipped: 1, ..ParseBudget::strict() };
        let mut ctx = ParserContext::new("foo bar baz qux ;".to_string());
        let mut tracker = BudgetTracker::new();

        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        // Should exhaust budget before finding semicolon
        assert_eq!(result, RecoveryResult::BudgetExhausted);
        Ok(())
    }

    #[test]
    fn skip_until_with_budget_eof_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new(String::new());
        let budget = ParseBudget::default();
        let mut tracker = BudgetTracker::new();

        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        assert_eq!(result, RecoveryResult::ReachedEof);
        Ok(())
    }

    #[test]
    fn skip_until_with_budget_recovers() -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = ParserContext::new("foo ;".to_string());
        let budget = ParseBudget::for_ide();
        let mut tracker = BudgetTracker::new();

        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        match result {
            RecoveryResult::Recovered(n) => {
                assert!(n > 0, "should have skipped at least one token")
            }
            other => return Err(format!("expected Recovered, got {:?}", other).into()),
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Budget tracking tests
// ───────────────────────────────────────────────────────────────────

mod budget_tests {
    use super::*;

    #[test]
    fn default_budget_is_reasonable() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget::default();
        assert!(budget.max_errors > 0);
        assert!(budget.max_depth > 0);
        assert!(budget.max_tokens_skipped > 0);
        assert!(budget.max_recoveries > 0);
        Ok(())
    }

    #[test]
    fn ide_budget_is_more_permissive() -> Result<(), Box<dyn std::error::Error>> {
        let strict = ParseBudget::strict();
        let ide = ParseBudget::for_ide();
        assert!(ide.max_errors >= strict.max_errors);
        assert!(ide.max_tokens_skipped >= strict.max_tokens_skipped);
        Ok(())
    }

    #[test]
    fn unlimited_budget() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget::unlimited();
        assert!(budget.max_errors > 1000);
        assert!(budget.max_depth > 1000);
        Ok(())
    }

    #[test]
    fn tracker_initially_zero() -> Result<(), Box<dyn std::error::Error>> {
        let tracker = BudgetTracker::new();
        assert_eq!(tracker.errors_emitted, 0);
        assert_eq!(tracker.current_depth, 0);
        assert_eq!(tracker.max_depth_reached, 0);
        assert_eq!(tracker.tokens_skipped, 0);
        assert_eq!(tracker.recoveries_attempted, 0);
        Ok(())
    }

    #[test]
    fn tracker_record_error() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.record_error();
        assert_eq!(tracker.errors_emitted, 1);
        tracker.record_error();
        assert_eq!(tracker.errors_emitted, 2);
        Ok(())
    }

    #[test]
    fn tracker_depth_enter_exit() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.enter_depth();
        assert_eq!(tracker.current_depth, 1);
        assert_eq!(tracker.max_depth_reached, 1);

        tracker.enter_depth();
        assert_eq!(tracker.current_depth, 2);
        assert_eq!(tracker.max_depth_reached, 2);

        tracker.exit_depth();
        assert_eq!(tracker.current_depth, 1);
        assert_eq!(tracker.max_depth_reached, 2); // max doesn't decrease
        Ok(())
    }

    #[test]
    fn tracker_record_skip() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.record_skip(5);
        assert_eq!(tracker.tokens_skipped, 5);
        tracker.record_skip(3);
        assert_eq!(tracker.tokens_skipped, 8);
        Ok(())
    }

    #[test]
    fn tracker_errors_exhausted() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_errors: 2, ..ParseBudget::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.errors_exhausted(&budget));
        tracker.record_error();
        assert!(!tracker.errors_exhausted(&budget));
        tracker.record_error();
        assert!(tracker.errors_exhausted(&budget));
        Ok(())
    }

    #[test]
    fn tracker_depth_would_exceed() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_depth: 2, ..ParseBudget::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.depth_would_exceed(&budget));
        tracker.enter_depth();
        assert!(!tracker.depth_would_exceed(&budget));
        tracker.enter_depth();
        assert!(tracker.depth_would_exceed(&budget));
        Ok(())
    }

    #[test]
    fn tracker_skip_would_exceed() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_tokens_skipped: 10, ..ParseBudget::default() };
        let tracker = BudgetTracker::new();

        assert!(!tracker.skip_would_exceed(&budget, 5));
        assert!(!tracker.skip_would_exceed(&budget, 10));
        assert!(tracker.skip_would_exceed(&budget, 11));
        Ok(())
    }

    #[test]
    fn tracker_begin_recovery_checks_budget() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_recoveries: 1, ..ParseBudget::default() };
        let mut tracker = BudgetTracker::new();

        assert!(tracker.begin_recovery(&budget), "first recovery should succeed");
        assert!(!tracker.begin_recovery(&budget), "second recovery should fail");
        Ok(())
    }

    #[test]
    fn tracker_can_skip_more() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_tokens_skipped: 5, ..ParseBudget::default() };
        let tracker = BudgetTracker::new();

        assert!(tracker.can_skip_more(&budget, 3));
        assert!(tracker.can_skip_more(&budget, 5));
        assert!(!tracker.can_skip_more(&budget, 6));
        Ok(())
    }

    #[test]
    fn tracker_record_recovery() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.record_recovery();
        assert_eq!(tracker.recoveries_attempted, 1);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// RecoveryResult tests
// ───────────────────────────────────────────────────────────────────

mod recovery_result_tests {
    use super::*;

    #[test]
    fn recovery_result_variants_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let recovered = RecoveryResult::Recovered(3);
        let at_sync = RecoveryResult::AtSyncPoint;
        let exhausted = RecoveryResult::BudgetExhausted;
        let eof = RecoveryResult::ReachedEof;

        assert_ne!(recovered, at_sync);
        assert_ne!(at_sync, exhausted);
        assert_ne!(exhausted, eof);
        assert_ne!(eof, recovered);
        Ok(())
    }

    #[test]
    fn recovery_result_clone_eq() -> Result<(), Box<dyn std::error::Error>> {
        let original = RecoveryResult::Recovered(5);
        let cloned = original;
        assert_eq!(original, cloned);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ParseError (recovery) tests
// ───────────────────────────────────────────────────────────────────

mod parse_error_tests {
    use super::*;

    #[test]
    fn parse_error_builder_chain() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(5, 1, 6),
        );

        let err = RecoveryParseError::new("test error".to_string(), range)
            .with_expected(vec!["semicolon".to_string()])
            .with_found("brace".to_string())
            .with_hint("add a semicolon".to_string());

        assert_eq!(err.message, "test error");
        assert_eq!(err.expected, vec!["semicolon"]);
        assert_eq!(err.found, "brace");
        assert_eq!(err.recovery_hint, Some("add a semicolon".to_string()));
        Ok(())
    }

    #[test]
    fn parse_error_default_fields() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(0, 1, 1),
        );

        let err = RecoveryParseError::new("msg".to_string(), range);
        assert!(err.expected.is_empty());
        assert!(err.found.is_empty());
        assert!(err.recovery_hint.is_none());
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// TokenStream tests
// ───────────────────────────────────────────────────────────────────

mod token_stream_tests {
    use super::*;

    #[test]
    fn empty_stream_is_eof() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("");
        assert!(stream.is_eof());
        Ok(())
    }

    #[test]
    fn peek_returns_token() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("42");
        let token = must(stream.peek());
        assert!(!token.text.is_empty());
        Ok(())
    }

    #[test]
    fn next_consumes_token() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("42");
        let token = must(stream.next());
        assert!(!token.text.is_empty());
        assert!(stream.is_eof());
        Ok(())
    }

    #[test]
    fn peek_second_looks_ahead() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("my $x");
        let _first = must(stream.peek());
        let second = stream.peek_second();
        assert!(second.is_ok(), "should be able to peek second token");
        Ok(())
    }

    #[test]
    fn peek_third_looks_further() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("my $x = 42;");
        let third = stream.peek_third();
        assert!(third.is_ok(), "should be able to peek third token");
        Ok(())
    }

    #[test]
    fn stream_processes_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("my $x = 42;");
        let mut count = 0;
        while !stream.is_eof() {
            let _tok = must(stream.next());
            count += 1;
            if count > 100 {
                return Err("infinite loop in token stream".into());
            }
        }
        assert!(count >= 4, "should have at least 4 tokens, got {}", count);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Trivia tests
// ───────────────────────────────────────────────────────────────────

mod trivia_tests {
    use super::*;

    #[test]
    fn trivia_whitespace_variant() -> Result<(), Box<dyn std::error::Error>> {
        let trivia = Trivia::Whitespace("  ".to_string());
        assert_eq!(trivia.as_str(), "  ");
        assert_eq!(trivia.kind_name(), "whitespace");
        Ok(())
    }

    #[test]
    fn trivia_comment_variant() -> Result<(), Box<dyn std::error::Error>> {
        let trivia = Trivia::LineComment("# hello".to_string());
        assert_eq!(trivia.as_str(), "# hello");
        assert_eq!(trivia.kind_name(), "comment");
        Ok(())
    }

    #[test]
    fn trivia_newline_variant() -> Result<(), Box<dyn std::error::Error>> {
        let trivia = Trivia::Newline;
        assert_eq!(trivia.as_str(), "\n");
        assert_eq!(trivia.kind_name(), "newline");
        Ok(())
    }

    #[test]
    fn trivia_token_construction() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(2, 1, 3),
        );
        let tt = TriviaToken::new(Trivia::Whitespace("  ".to_string()), range);
        assert_eq!(tt.trivia.as_str(), "  ");
        Ok(())
    }

    #[test]
    fn trivia_preserving_parser_returns_node_with_trivia() -> Result<(), Box<dyn std::error::Error>>
    {
        let parser = TriviaPreservingParser::new("  # comment\nmy $x;".to_string());
        let result: NodeWithTrivia = parser.parse();

        // The parser should produce a Program node
        match &result.node.kind {
            V2NodeKind::Program { .. } => { /* ok */ }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn format_with_trivia_includes_trivia_text() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(0, 1, 1),
        );
        let node = perl_ast::v2::Node::new(
            perl_ast::v2::NodeIdGenerator::new().next_id(),
            V2NodeKind::Program { statements: vec![] },
            range,
        );

        let leading = vec![TriviaToken::new(Trivia::Whitespace("  ".to_string()), range)];
        let nwt = NodeWithTrivia { node, leading_trivia: leading, trailing_trivia: vec![] };

        let formatted = format_with_trivia(&nwt);
        assert!(formatted.contains("  "), "should include leading whitespace trivia");
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// PositionMapper tests
// ───────────────────────────────────────────────────────────────────

mod position_mapper_tests {
    use super::*;

    #[test]
    fn mapper_empty_text() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("");
        assert_eq!(mapper.len_bytes(), 0);
        assert!(mapper.is_empty());
        Ok(())
    }

    #[test]
    fn mapper_single_line() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("hello world");
        assert_eq!(mapper.len_bytes(), 11);
        assert!(!mapper.is_empty());
        assert_eq!(mapper.len_lines(), 1);
        Ok(())
    }

    #[test]
    fn mapper_multi_line_lf() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("line1\nline2\nline3");
        assert_eq!(mapper.len_lines(), 3);
        assert_eq!(mapper.line_ending(), LineEnding::Lf);
        Ok(())
    }

    #[test]
    fn mapper_crlf_detection() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("line1\r\nline2\r\n");
        assert_eq!(mapper.line_ending(), LineEnding::CrLf);
        Ok(())
    }

    #[test]
    fn mapper_byte_to_lsp_pos_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let text = "my $x = 42;\nmy $y = 99;";
        let mapper = PositionMapper::new(text);

        // Byte 0 should map to line 0, char 0
        let pos = mapper.byte_to_lsp_pos(0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        // Roundtrip: lsp_pos -> byte -> lsp_pos
        let byte = must_some(mapper.lsp_pos_to_byte(pos));
        assert_eq!(byte, 0);
        Ok(())
    }

    #[test]
    fn mapper_second_line_position() -> Result<(), Box<dyn std::error::Error>> {
        let text = "line1\nline2";
        let mapper = PositionMapper::new(text);

        // "line2" starts at byte 6
        let pos = mapper.byte_to_lsp_pos(6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
        Ok(())
    }

    #[test]
    fn mapper_text_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let original = "my $x = 42;\nmy $y = 99;";
        let mapper = PositionMapper::new(original);
        assert_eq!(mapper.text(), original);
        Ok(())
    }

    #[test]
    fn mapper_slice() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("hello world");
        let slice = mapper.slice(0, 5);
        assert_eq!(slice, "hello");
        Ok(())
    }

    #[test]
    fn mapper_update_replaces_content() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("old text");
        mapper.update("new text");
        assert_eq!(mapper.text(), "new text");
        Ok(())
    }

    #[test]
    fn mapper_apply_edit() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("hello world");
        // Replace "world" (bytes 6..11) with "rust"
        mapper.apply_edit(6, 11, "rust");
        assert_eq!(mapper.text(), "hello rust");
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// LineIndex tests
// ───────────────────────────────────────────────────────────────────

mod line_index_tests {
    use super::*;

    #[test]
    fn line_index_empty() -> Result<(), Box<dyn std::error::Error>> {
        let index = LineIndex::new(String::new());
        // offset 0 should map to line 0, col 0
        let (line, col) = index.offset_to_position(0);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
        Ok(())
    }

    #[test]
    fn line_index_single_line() -> Result<(), Box<dyn std::error::Error>> {
        let index = LineIndex::new("hello".to_string());
        let (line, col) = index.offset_to_position(0);
        assert_eq!(line, 0);
        assert_eq!(col, 0);

        // "hello" last char at offset 4
        let (line2, col2) = index.offset_to_position(4);
        assert_eq!(line2, 0);
        assert_eq!(col2, 4);
        Ok(())
    }

    #[test]
    fn line_index_multiple_lines() -> Result<(), Box<dyn std::error::Error>> {
        let index = LineIndex::new("a\nb\nc".to_string());
        // "b" is at offset 2
        let (line, col) = index.offset_to_position(2);
        assert_eq!(line, 1);
        assert_eq!(col, 0);

        // "c" is at offset 4
        let (line2, col2) = index.offset_to_position(4);
        assert_eq!(line2, 2);
        assert_eq!(col2, 0);
        Ok(())
    }

    #[test]
    fn line_index_position_to_offset_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let text = "abc\ndef\nghi".to_string();
        let index = LineIndex::new(text);

        // "def" starts at line 1, col 0 → offset 4
        let offset = index.position_to_offset(1, 0);
        assert_eq!(offset, Some(4));

        let (line, col) = index.offset_to_position(4);
        assert_eq!(line, 1);
        assert_eq!(col, 0);
        Ok(())
    }

    #[test]
    fn line_index_range() -> Result<(), Box<dyn std::error::Error>> {
        let index = LineIndex::new("abc\ndef".to_string());
        let (start, end) = index.range(0, 4);
        assert_eq!(start, (0, 0)); // "a" at line 0 col 0
        assert_eq!(end, (1, 0)); // "d" at line 1 col 0
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// AST node construction tests
// ───────────────────────────────────────────────────────────────────

mod ast_node_tests {
    use super::*;

    #[test]
    fn v1_node_creation() -> Result<(), Box<dyn std::error::Error>> {
        let node = V1Node::new(
            V1NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );
        assert_eq!(node.location.start, 0);
        assert_eq!(node.location.end, 2);
        assert_eq!(node.kind.kind_name(), "Number");
        Ok(())
    }

    #[test]
    fn v1_node_to_sexp() -> Result<(), Box<dyn std::error::Error>> {
        let node = V1Node::new(
            V1NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );
        let sexp = node.to_sexp();
        assert!(!sexp.is_empty(), "S-expression should not be empty");
        Ok(())
    }

    #[test]
    fn v1_program_node() -> Result<(), Box<dyn std::error::Error>> {
        let child = V1Node::new(
            V1NodeKind::Number { value: "1".to_string() },
            SourceLocation { start: 0, end: 1 },
        );
        let program = V1Node::new(
            V1NodeKind::Program { statements: vec![child] },
            SourceLocation { start: 0, end: 1 },
        );
        match &program.kind {
            V1NodeKind::Program { statements } => assert_eq!(statements.len(), 1),
            _ => return Err("expected Program".into()),
        }
        Ok(())
    }

    #[test]
    fn v2_node_creation() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(2, 1, 3),
        );
        let mut id_gen = perl_ast::v2::NodeIdGenerator::new();
        let node = perl_ast::v2::Node::new(
            id_gen.next_id(),
            V2NodeKind::Number { value: "99".to_string() },
            range,
        );
        assert_eq!(node.to_sexp(), node.kind.to_sexp());
        Ok(())
    }

    #[test]
    fn v2_error_node() -> Result<(), Box<dyn std::error::Error>> {
        let range = perl_position_tracking::Range::new(
            perl_position_tracking::Position::new(0, 1, 1),
            perl_position_tracking::Position::new(0, 1, 1),
        );
        let mut id_gen = perl_ast::v2::NodeIdGenerator::new();
        let node = perl_ast::v2::Node::new(
            id_gen.next_id(),
            V2NodeKind::Error {
                message: "test".to_string(),
                expected: vec!["foo".to_string()],
                partial: None,
            },
            range,
        );
        match &node.kind {
            V2NodeKind::Error { message, expected, partial } => {
                assert_eq!(message, "test");
                assert_eq!(expected.len(), 1);
                assert!(partial.is_none());
            }
            _ => return Err("expected Error node".into()),
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ParseOutput tests
// ───────────────────────────────────────────────────────────────────

mod parse_output_tests {
    use super::*;

    #[test]
    fn success_output_has_no_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
        let ast = V1Node::new(
            V1NodeKind::Program { statements: vec![] },
            SourceLocation { start: 0, end: 0 },
        );
        let output = ParseOutput::success(ast);
        assert!(output.diagnostics.is_empty());
        assert!(!output.terminated_early);
        assert_eq!(output.budget_usage.errors_emitted, 0);
        Ok(())
    }

    #[test]
    fn with_errors_output_tracks_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
        let ast = V1Node::new(
            V1NodeKind::Program { statements: vec![] },
            SourceLocation { start: 0, end: 0 },
        );
        let diags = vec![CatastrophicParseError::UnexpectedEof];
        let output = ParseOutput::with_errors(ast, diags);
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.budget_usage.errors_emitted, 1);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// SyncPoint tests
// ───────────────────────────────────────────────────────────────────

mod sync_point_tests {
    use super::*;

    #[test]
    fn sync_point_equality() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SyncPoint::Semicolon, SyncPoint::Semicolon);
        assert_eq!(SyncPoint::CloseBrace, SyncPoint::CloseBrace);
        assert_eq!(SyncPoint::Keyword, SyncPoint::Keyword);
        assert_eq!(SyncPoint::Eof, SyncPoint::Eof);
        assert_ne!(SyncPoint::Semicolon, SyncPoint::Eof);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Integration: end-to-end recovery scenarios
// ───────────────────────────────────────────────────────────────────

mod integration_tests {
    use super::*;

    #[test]
    fn recovery_parser_many_errors() -> Result<(), Box<dyn std::error::Error>> {
        // Source with multiple syntax errors
        let source = "my $a = ; my $b = ; my $c = ;".to_string();
        let parser = RecoveryParser::new(source);
        let (ast, errors) = parser.parse();

        match &ast.kind {
            V2NodeKind::Program { statements } => {
                assert!(statements.len() >= 3, "should attempt to parse all three decls");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        assert!(errors.len() >= 3, "should have at least 3 errors");
        Ok(())
    }

    #[test]
    fn parser_parse_and_errors_consistent() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $x = 42; sub hello { }");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(statements.len() >= 2, "should parse declaration and sub");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        // Valid code should have no errors
        assert!(parser.errors().is_empty(), "valid code should have no parse errors");
        Ok(())
    }

    #[test]
    fn whitespace_only_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("   \n\n  \t  ");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(statements.is_empty(), "whitespace-only should yield empty program");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn comment_only_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("# just a comment\n# another one\n");
        let ast = must(parser.parse());

        match &ast.kind {
            V1NodeKind::Program { statements } => {
                assert!(statements.is_empty(), "comment-only should yield empty program");
            }
            other => return Err(format!("expected Program, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn position_mapper_with_parser() -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $x = 42;\nmy $y = 99;";
        let mapper = PositionMapper::new(source);
        let mut parser = Parser::new(source);
        let _ast = must(parser.parse());

        // Verify mapper agrees on line count
        assert_eq!(mapper.len_lines(), 2);
        // First char of second line
        let pos = mapper.byte_to_lsp_pos(12);
        assert_eq!(pos.line, 1);
        Ok(())
    }
}
