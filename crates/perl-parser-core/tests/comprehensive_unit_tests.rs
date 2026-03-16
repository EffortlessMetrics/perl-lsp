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
    ast_v2::MissingKind,
    ast_v2::NodeKind as V2NodeKind,
    builtin_signatures,
    builtin_signatures_phf,
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

// ───────────────────────────────────────────────────────────────────
// SourceLocation / ByteSpan tests
// ───────────────────────────────────────────────────────────────────

mod source_location_tests {
    use super::*;

    #[test]
    fn empty_span_at_position() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::empty(5);
        assert_eq!(loc.start, 5);
        assert_eq!(loc.end, 5);
        assert_eq!(loc.len(), 0);
        assert!(loc.is_empty());
        Ok(())
    }

    #[test]
    fn whole_span_covers_source() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello world";
        let loc = SourceLocation::whole(source);
        assert_eq!(loc.start, 0);
        assert_eq!(loc.end, 11);
        assert_eq!(loc.len(), 11);
        assert!(!loc.is_empty());
        Ok(())
    }

    #[test]
    fn whole_span_empty_source() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::whole("");
        assert_eq!(loc.start, 0);
        assert_eq!(loc.end, 0);
        assert!(loc.is_empty());
        Ok(())
    }

    #[test]
    fn contains_offset() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::new(5, 10);
        assert!(!loc.contains(4));
        assert!(loc.contains(5));
        assert!(loc.contains(7));
        assert!(loc.contains(9));
        // end is exclusive
        assert!(!loc.contains(10));
        Ok(())
    }

    #[test]
    fn contains_span_inner_outer() -> Result<(), Box<dyn std::error::Error>> {
        let outer = SourceLocation::new(0, 20);
        let inner = SourceLocation::new(5, 15);
        let partial = SourceLocation::new(15, 25);

        assert!(outer.contains_span(inner));
        assert!(!inner.contains_span(outer));
        assert!(!outer.contains_span(partial));
        // A span contains itself
        assert!(outer.contains_span(outer));
        Ok(())
    }

    #[test]
    fn overlaps_various() -> Result<(), Box<dyn std::error::Error>> {
        let a = SourceLocation::new(0, 10);
        let b = SourceLocation::new(5, 15);
        let c = SourceLocation::new(10, 20);

        assert!(a.overlaps(b));
        assert!(b.overlaps(a));
        // Adjacent spans do NOT overlap (half-open)
        assert!(!a.overlaps(c));
        // Empty spans at same position don't overlap
        let e1 = SourceLocation::empty(5);
        let e2 = SourceLocation::empty(5);
        assert!(!e1.overlaps(e2));
        Ok(())
    }

    #[test]
    fn intersection_overlapping() -> Result<(), Box<dyn std::error::Error>> {
        let a = SourceLocation::new(0, 10);
        let b = SourceLocation::new(5, 15);
        if let Some(inter) = a.intersection(b) {
            assert_eq!(inter.start, 5);
            assert_eq!(inter.end, 10);
        } else {
            return Err("expected intersection".into());
        }
        Ok(())
    }

    #[test]
    fn intersection_disjoint() -> Result<(), Box<dyn std::error::Error>> {
        let a = SourceLocation::new(0, 5);
        let b = SourceLocation::new(10, 15);
        assert!(a.intersection(b).is_none());
        Ok(())
    }

    #[test]
    fn intersection_adjacent() -> Result<(), Box<dyn std::error::Error>> {
        let a = SourceLocation::new(0, 5);
        let b = SourceLocation::new(5, 10);
        assert!(a.intersection(b).is_none());
        Ok(())
    }

    #[test]
    fn union_covers_both() -> Result<(), Box<dyn std::error::Error>> {
        let a = SourceLocation::new(3, 7);
        let b = SourceLocation::new(10, 20);
        let u = a.union(b);
        assert_eq!(u.start, 3);
        assert_eq!(u.end, 20);
        Ok(())
    }

    #[test]
    fn try_slice_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello world";
        let loc = SourceLocation::new(6, 11);
        if let Some(s) = loc.try_slice(source) {
            assert_eq!(s, "world");
        } else {
            return Err("expected Some slice".into());
        }
        Ok(())
    }

    #[test]
    fn try_slice_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
        let source = "short";
        let loc = SourceLocation::new(0, 100);
        assert!(loc.try_slice(source).is_none());
        Ok(())
    }

    #[test]
    fn to_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::new(3, 9);
        let range = loc.to_range();
        assert_eq!(range, 3..9);
        Ok(())
    }

    #[test]
    fn display_format() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::new(42, 100);
        assert_eq!(format!("{}", loc), "42..100");
        Ok(())
    }

    #[test]
    fn from_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let loc: SourceLocation = (5..10).into();
        assert_eq!(loc.start, 5);
        assert_eq!(loc.end, 10);
        Ok(())
    }

    #[test]
    fn from_tuple_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let loc: SourceLocation = (3, 7).into();
        assert_eq!(loc.start, 3);
        assert_eq!(loc.end, 7);
        Ok(())
    }

    #[test]
    fn into_tuple_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::new(3, 7);
        let (s, e): (usize, usize) = loc.into();
        assert_eq!(s, 3);
        assert_eq!(e, 7);
        Ok(())
    }

    #[test]
    fn into_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::new(1, 5);
        let range: std::ops::Range<usize> = loc.into();
        assert_eq!(range, 1..5);
        Ok(())
    }

    #[test]
    fn default_is_zero_span() -> Result<(), Box<dyn std::error::Error>> {
        let loc = SourceLocation::default();
        assert_eq!(loc.start, 0);
        assert_eq!(loc.end, 0);
        assert!(loc.is_empty());
        Ok(())
    }

    #[test]
    fn slice_extracts_text() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello world";
        let loc = SourceLocation::new(0, 5);
        assert_eq!(loc.slice(source), "hello");
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ParseError (catastrophic) variant and method tests
// ───────────────────────────────────────────────────────────────────

mod catastrophic_parse_error_tests {
    use super::*;

    #[test]
    fn unexpected_eof_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::UnexpectedEof;
        let msg = format!("{}", err);
        assert!(msg.contains("Unexpected end of input"));
        Ok(())
    }

    #[test]
    fn unexpected_token_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::unexpected("semicolon", "comma", 42);
        let msg = format!("{}", err);
        assert!(msg.contains("semicolon"));
        assert!(msg.contains("comma"));
        assert!(msg.contains("42"));
        Ok(())
    }

    #[test]
    fn syntax_error_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::syntax("bad thing", 99);
        let msg = format!("{}", err);
        assert!(msg.contains("bad thing"));
        assert!(msg.contains("99"));
        Ok(())
    }

    #[test]
    fn lexer_error_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::LexerError { message: "bad char".to_string() };
        let msg = format!("{}", err);
        assert!(msg.contains("bad char"));
        Ok(())
    }

    #[test]
    fn recursion_limit_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::RecursionLimit;
        let msg = format!("{}", err);
        assert!(msg.contains("recursion") || msg.contains("Recursion"));
        Ok(())
    }

    #[test]
    fn invalid_number_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::InvalidNumber { literal: "0xZZ".to_string() };
        let msg = format!("{}", err);
        assert!(msg.contains("0xZZ"));
        Ok(())
    }

    #[test]
    fn invalid_string_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::InvalidString;
        let msg = format!("{}", err);
        assert!(msg.contains("string") || msg.contains("String"));
        Ok(())
    }

    #[test]
    fn unclosed_delimiter_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::UnclosedDelimiter { delimiter: '(' };
        let msg = format!("{}", err);
        assert!(msg.contains('('));
        Ok(())
    }

    #[test]
    fn invalid_regex_display() -> Result<(), Box<dyn std::error::Error>> {
        let err =
            CatastrophicParseError::InvalidRegex { message: "unterminated group".to_string() };
        let msg = format!("{}", err);
        assert!(msg.contains("unterminated group"));
        Ok(())
    }

    #[test]
    fn nesting_too_deep_display() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::NestingTooDeep { depth: 300, max_depth: 256 };
        let msg = format!("{}", err);
        assert!(msg.contains("300"));
        assert!(msg.contains("256"));
        Ok(())
    }

    #[test]
    fn location_for_positioned_errors() -> Result<(), Box<dyn std::error::Error>> {
        let err1 = CatastrophicParseError::unexpected("a", "b", 10);
        assert_eq!(err1.location(), Some(10));

        let err2 = CatastrophicParseError::syntax("msg", 20);
        assert_eq!(err2.location(), Some(20));
        Ok(())
    }

    #[test]
    fn location_none_for_unpositioned_errors() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(CatastrophicParseError::UnexpectedEof.location(), None);
        assert_eq!(CatastrophicParseError::RecursionLimit.location(), None);
        assert_eq!(CatastrophicParseError::InvalidString.location(), None);
        assert_eq!(
            CatastrophicParseError::LexerError { message: "x".to_string() }.location(),
            None
        );
        Ok(())
    }

    #[test]
    fn suggestion_for_semicolon() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::unexpected("';'", "newline", 5);
        let suggestion = err.suggestion().unwrap_or_default();
        assert!(suggestion.contains("semicolon"));
        Ok(())
    }

    #[test]
    fn suggestion_for_unclosed_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::UnclosedDelimiter { delimiter: ')' };
        let suggestion = err.suggestion().unwrap_or_default();
        assert!(suggestion.contains(')'));
        Ok(())
    }

    #[test]
    fn suggestion_none_for_generic_errors() -> Result<(), Box<dyn std::error::Error>> {
        assert!(CatastrophicParseError::UnexpectedEof.suggestion().is_none());
        assert!(CatastrophicParseError::RecursionLimit.suggestion().is_none());
        Ok(())
    }

    #[test]
    fn parse_error_equality() -> Result<(), Box<dyn std::error::Error>> {
        let e1 = CatastrophicParseError::UnexpectedEof;
        let e2 = CatastrophicParseError::UnexpectedEof;
        assert_eq!(e1, e2);

        let e3 = CatastrophicParseError::syntax("a", 1);
        let e4 = CatastrophicParseError::syntax("a", 1);
        assert_eq!(e3, e4);

        assert_ne!(e1, e3);
        Ok(())
    }

    #[test]
    fn parse_error_clone() -> Result<(), Box<dyn std::error::Error>> {
        let err = CatastrophicParseError::unexpected("x", "y", 42);
        let cloned = err.clone();
        assert_eq!(err, cloned);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ErrorContext and get_error_contexts tests
// ───────────────────────────────────────────────────────────────────

mod error_context_tests {
    use super::*;
    use perl_parser_core::error::get_error_contexts;

    #[test]
    fn error_context_single_line() -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $x = 42";
        let errors = vec![CatastrophicParseError::syntax("missing semicolon", 10)];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].line, 0);
        assert_eq!(contexts[0].source_line, "my $x = 42");
        Ok(())
    }

    #[test]
    fn error_context_multiline() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1;\nline2;\nline3;";
        // byte offset 7 is start of "line2;"
        let errors = vec![CatastrophicParseError::syntax("bad", 7)];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].line, 1);
        assert_eq!(contexts[0].source_line, "line2;");
        Ok(())
    }

    #[test]
    fn error_context_at_eof() -> Result<(), Box<dyn std::error::Error>> {
        let source = "short";
        let errors = vec![CatastrophicParseError::UnexpectedEof];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 1);
        // UnexpectedEof has no location, defaults to source.len()
        Ok(())
    }

    #[test]
    fn error_context_with_suggestion() -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $x = 1";
        let errors = vec![CatastrophicParseError::unexpected("';'", "EOF", 9)];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 1);
        // The suggestion should be present since expected contains "';'"
        let suggestion = contexts[0].suggestion.as_deref().unwrap_or("");
        assert!(suggestion.contains("semicolon"));
        Ok(())
    }

    #[test]
    fn error_context_empty_source() -> Result<(), Box<dyn std::error::Error>> {
        let source = "";
        let errors = vec![CatastrophicParseError::UnexpectedEof];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].line, 0);
        Ok(())
    }

    #[test]
    fn error_context_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
        let source = "a;\nb;\nc;";
        let errors = vec![
            CatastrophicParseError::syntax("err1", 0),
            CatastrophicParseError::syntax("err2", 3),
            CatastrophicParseError::syntax("err3", 6),
        ];
        let contexts = get_error_contexts(&errors, source);
        assert_eq!(contexts.len(), 3);
        assert_eq!(contexts[0].line, 0);
        assert_eq!(contexts[1].line, 1);
        assert_eq!(contexts[2].line, 2);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ErrorClassifier tests
// ───────────────────────────────────────────────────────────────────

mod error_classifier_tests {
    use super::*;
    use perl_parser_core::error_classifier::{ErrorClassifier, ParseErrorKind};

    #[test]
    fn classifier_default_and_new() -> Result<(), Box<dyn std::error::Error>> {
        let _c1 = ErrorClassifier::new();
        let _c2 = ErrorClassifier;
        Ok(())
    }

    #[test]
    fn classify_unclosed_double_quote() -> Result<(), Box<dyn std::error::Error>> {
        let classifier = ErrorClassifier::new();
        let source = r#"my $x = "hello"#;
        let node = V1Node::new(
            V1NodeKind::Error {
                message: "err".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            SourceLocation::new(9, 15),
        );
        let kind = classifier.classify(&node, source);
        assert_eq!(kind, ParseErrorKind::UnclosedString);
        Ok(())
    }

    #[test]
    fn classify_missing_semicolon() -> Result<(), Box<dyn std::error::Error>> {
        let classifier = ErrorClassifier::new();
        let source = "my $x = 42\nmy $y = 10;";
        let node = V1Node::new(
            V1NodeKind::Error {
                message: "err".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            SourceLocation::new(10, 11),
        );
        let kind = classifier.classify(&node, source);
        assert_eq!(kind, ParseErrorKind::MissingSemicolon);
        Ok(())
    }

    #[test]
    fn classify_unclosed_paren() -> Result<(), Box<dyn std::error::Error>> {
        let classifier = ErrorClassifier::new();
        let source = "my $x = (1 + 2;";
        let node = V1Node::new(
            V1NodeKind::Error {
                message: "err".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            SourceLocation::new(8, 9),
        );
        let kind = classifier.classify(&node, source);
        assert_eq!(kind, ParseErrorKind::UnclosedParenthesis);
        Ok(())
    }

    #[test]
    fn diagnostic_message_all_kinds() -> Result<(), Box<dyn std::error::Error>> {
        let c = ErrorClassifier::new();
        // Just verify messages are non-empty for each kind
        let kinds = vec![
            ParseErrorKind::UnclosedString,
            ParseErrorKind::UnclosedRegex,
            ParseErrorKind::UnclosedBlock,
            ParseErrorKind::MissingSemicolon,
            ParseErrorKind::InvalidSyntax,
            ParseErrorKind::UnclosedParenthesis,
            ParseErrorKind::UnclosedBracket,
            ParseErrorKind::UnclosedBrace,
            ParseErrorKind::UnterminatedHeredoc,
            ParseErrorKind::InvalidVariableName,
            ParseErrorKind::InvalidSubroutineName,
            ParseErrorKind::MissingOperator,
            ParseErrorKind::MissingOperand,
            ParseErrorKind::UnexpectedEof,
            ParseErrorKind::UnexpectedToken {
                expected: "ident".to_string(),
                found: "number".to_string(),
            },
        ];
        for kind in &kinds {
            let msg = c.get_diagnostic_message(kind);
            assert!(!msg.is_empty(), "empty message for {:?}", kind);
        }
        Ok(())
    }

    #[test]
    fn suggestion_some_for_most_kinds() -> Result<(), Box<dyn std::error::Error>> {
        let c = ErrorClassifier::new();
        // These should all have suggestions
        let kinds_with_suggestions = vec![
            ParseErrorKind::MissingSemicolon,
            ParseErrorKind::UnclosedString,
            ParseErrorKind::UnclosedParenthesis,
            ParseErrorKind::UnclosedBracket,
            ParseErrorKind::UnclosedBrace,
            ParseErrorKind::UnclosedBlock,
            ParseErrorKind::UnclosedRegex,
            ParseErrorKind::UnterminatedHeredoc,
            ParseErrorKind::UnexpectedEof,
        ];
        for kind in &kinds_with_suggestions {
            assert!(c.get_suggestion(kind).is_some(), "no suggestion for {:?}", kind);
        }
        // InvalidSyntax should have no suggestion
        assert!(c.get_suggestion(&ParseErrorKind::InvalidSyntax).is_none());
        Ok(())
    }

    #[test]
    fn explanation_some_for_common_kinds() -> Result<(), Box<dyn std::error::Error>> {
        let c = ErrorClassifier::new();
        assert!(c.get_explanation(&ParseErrorKind::MissingSemicolon).is_some());
        assert!(c.get_explanation(&ParseErrorKind::UnclosedString).is_some());
        assert!(c.get_explanation(&ParseErrorKind::UnclosedRegex).is_some());
        assert!(c.get_explanation(&ParseErrorKind::UnterminatedHeredoc).is_some());
        assert!(c.get_explanation(&ParseErrorKind::UnclosedBlock).is_some());
        // InvalidSyntax has no explanation
        assert!(c.get_explanation(&ParseErrorKind::InvalidSyntax).is_none());
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// MissingKind tests
// ───────────────────────────────────────────────────────────────────

mod missing_kind_tests {
    use super::*;

    #[test]
    fn all_variants_exist() -> Result<(), Box<dyn std::error::Error>> {
        let variants = vec![
            MissingKind::Expression,
            MissingKind::Statement,
            MissingKind::Identifier,
            MissingKind::Block,
            MissingKind::ClosingDelimiter(')'),
            MissingKind::ClosingDelimiter('}'),
            MissingKind::ClosingDelimiter(']'),
            MissingKind::Semicolon,
            MissingKind::Condition,
            MissingKind::Argument,
            MissingKind::Operator,
        ];
        // Each is distinct
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
        let kind = MissingKind::Expression;
        let dbg = format!("{:?}", kind);
        assert!(dbg.contains("Expression"));
        Ok(())
    }

    #[test]
    fn clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
        let kind = MissingKind::Semicolon;
        let cloned = kind;
        assert_eq!(kind, cloned);
        Ok(())
    }

    #[test]
    fn closing_delimiter_variants() -> Result<(), Box<dyn std::error::Error>> {
        let paren = MissingKind::ClosingDelimiter(')');
        let brace = MissingKind::ClosingDelimiter('}');
        let bracket = MissingKind::ClosingDelimiter(']');
        assert_ne!(paren, brace);
        assert_ne!(brace, bracket);
        assert_ne!(paren, bracket);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// ParseOutput extended tests
// ───────────────────────────────────────────────────────────────────

mod parse_output_extended_tests {
    use super::*;

    fn make_empty_program() -> V1Node {
        V1Node::new(V1NodeKind::Program { statements: vec![] }, SourceLocation::new(0, 0))
    }

    #[test]
    fn finish_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
        let ast = make_empty_program();
        let errors =
            vec![CatastrophicParseError::syntax("e1", 0), CatastrophicParseError::syntax("e2", 5)];
        let mut tracker = BudgetTracker::new();
        tracker.errors_emitted = 7;
        tracker.tokens_skipped = 33;
        tracker.recoveries_attempted = 4;
        tracker.max_depth_reached = 12;
        tracker.current_depth = 2;

        let output = ParseOutput::finish(ast, errors, tracker, true);
        assert_eq!(output.error_count(), 2);
        assert!(output.has_errors());
        assert!(!output.is_ok());
        assert!(output.terminated_early);
        assert_eq!(output.budget_usage.errors_emitted, 7);
        assert_eq!(output.budget_usage.tokens_skipped, 33);
        assert_eq!(output.budget_usage.recoveries_attempted, 4);
        assert_eq!(output.budget_usage.max_depth_reached, 12);
        assert_eq!(output.budget_usage.current_depth, 2);
        Ok(())
    }

    #[test]
    fn success_output_is_clean() -> Result<(), Box<dyn std::error::Error>> {
        let output = ParseOutput::success(make_empty_program());
        assert!(output.is_ok());
        assert!(!output.has_errors());
        assert_eq!(output.error_count(), 0);
        assert!(!output.terminated_early);
        assert_eq!(output.budget_usage.errors_emitted, 0);
        assert_eq!(output.budget_usage.tokens_skipped, 0);
        Ok(())
    }

    #[test]
    fn with_errors_sets_error_count_in_tracker() -> Result<(), Box<dyn std::error::Error>> {
        let errors = vec![
            CatastrophicParseError::UnexpectedEof,
            CatastrophicParseError::RecursionLimit,
            CatastrophicParseError::InvalidString,
        ];
        let output = ParseOutput::with_errors(make_empty_program(), errors);
        assert_eq!(output.budget_usage.errors_emitted, 3);
        assert!(!output.terminated_early);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// LineEnding extended tests
// ───────────────────────────────────────────────────────────────────

mod line_ending_extended_tests {
    use super::*;

    #[test]
    fn cr_only_detection() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\rline2\rline3";
        let mapper = PositionMapper::new(source);
        let ending = mapper.line_ending();
        // CR-only should be detected
        assert!(
            ending == LineEnding::Cr || ending == LineEnding::Mixed,
            "expected Cr or Mixed, got {:?}",
            ending
        );
        Ok(())
    }

    #[test]
    fn lf_detection() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\nline2\nline3";
        let mapper = PositionMapper::new(source);
        assert_eq!(mapper.line_ending(), LineEnding::Lf);
        Ok(())
    }

    #[test]
    fn crlf_detection() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\r\nline2\r\nline3";
        let mapper = PositionMapper::new(source);
        assert_eq!(mapper.line_ending(), LineEnding::CrLf);
        Ok(())
    }

    #[test]
    fn mixed_line_ending_detection() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\nline2\r\nline3\rline4";
        let mapper = PositionMapper::new(source);
        assert_eq!(mapper.line_ending(), LineEnding::Mixed);
        Ok(())
    }

    #[test]
    fn no_newline_defaults_to_lf() -> Result<(), Box<dyn std::error::Error>> {
        let source = "single line no newline";
        let mapper = PositionMapper::new(source);
        // When there are no newlines, default should be Lf
        assert_eq!(mapper.line_ending(), LineEnding::Lf);
        Ok(())
    }

    #[test]
    fn line_ending_equality() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(LineEnding::Lf, LineEnding::Lf);
        assert_eq!(LineEnding::CrLf, LineEnding::CrLf);
        assert_eq!(LineEnding::Cr, LineEnding::Cr);
        assert_eq!(LineEnding::Mixed, LineEnding::Mixed);
        assert_ne!(LineEnding::Lf, LineEnding::CrLf);
        assert_ne!(LineEnding::Cr, LineEnding::Mixed);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// PositionMapper extended tests
// ───────────────────────────────────────────────────────────────────

mod position_mapper_extended_tests {
    use super::*;

    #[test]
    fn is_empty_true() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("");
        assert!(mapper.is_empty());
        assert_eq!(mapper.len_bytes(), 0);
        Ok(())
    }

    #[test]
    fn is_empty_false() -> Result<(), Box<dyn std::error::Error>> {
        let mapper = PositionMapper::new("a");
        assert!(!mapper.is_empty());
        assert_eq!(mapper.len_bytes(), 1);
        Ok(())
    }

    #[test]
    fn char_to_lsp_pos_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello\nworld";
        let mapper = PositionMapper::new(source);
        // char 0 = 'h' on line 0, col 0
        let pos = mapper.char_to_lsp_pos(0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
        // char 6 = 'w' on line 1, col 0
        let pos2 = mapper.char_to_lsp_pos(6);
        assert_eq!(pos2.line, 1);
        assert_eq!(pos2.character, 0);
        Ok(())
    }

    #[test]
    fn lsp_pos_to_char_and_back() -> Result<(), Box<dyn std::error::Error>> {
        let source = "abc\ndefgh";
        let mapper = PositionMapper::new(source);
        // byte 4 = 'd' => line 1, col 0
        let pos = mapper.byte_to_lsp_pos(4);
        if let Some(char_idx) = mapper.lsp_pos_to_char(pos) {
            let roundtrip = mapper.char_to_lsp_pos(char_idx);
            assert_eq!(roundtrip.line, pos.line);
            assert_eq!(roundtrip.character, pos.character);
        }
        Ok(())
    }

    #[test]
    fn out_of_bounds_byte_position() -> Result<(), Box<dyn std::error::Error>> {
        let source = "short";
        let mapper = PositionMapper::new(source);
        // byte offset beyond source length should clamp
        let pos = mapper.byte_to_lsp_pos(1000);
        // Should not crash, returns clamped position
        assert!(pos.line <= 1);
        Ok(())
    }

    #[test]
    fn apply_edit_insert() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("hello world");
        mapper.apply_edit(5, 5, " beautiful");
        assert_eq!(mapper.text(), "hello beautiful world");
        Ok(())
    }

    #[test]
    fn apply_edit_delete() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("hello beautiful world");
        mapper.apply_edit(5, 15, "");
        assert_eq!(mapper.text(), "hello world");
        Ok(())
    }

    #[test]
    fn apply_edit_replace() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("hello world");
        mapper.apply_edit(6, 11, "earth");
        assert_eq!(mapper.text(), "hello earth");
        Ok(())
    }

    #[test]
    fn slice_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        let source = "abcdefghij";
        let mapper = PositionMapper::new(source);
        assert_eq!(mapper.slice(0, 3), "abc");
        assert_eq!(mapper.slice(7, 10), "hij");
        // Entire string
        assert_eq!(mapper.slice(0, 10), "abcdefghij");
        // Empty slice
        assert_eq!(mapper.slice(5, 5), "");
        Ok(())
    }

    #[test]
    fn update_replaces_entirely() -> Result<(), Box<dyn std::error::Error>> {
        let mut mapper = PositionMapper::new("old content");
        mapper.update("new content\nwith lines");
        assert_eq!(mapper.text(), "new content\nwith lines");
        assert_eq!(mapper.len_lines(), 2);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// TokenStream extended tests
// ───────────────────────────────────────────────────────────────────

mod token_stream_extended_tests {
    use super::*;

    #[test]
    fn on_stmt_boundary_resets_peek() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("my $x; my $y;");
        // Prime the peek
        let _ = stream.peek();
        // on_stmt_boundary should invalidate cached peeks
        stream.on_stmt_boundary();
        // After reset, peek should still work
        if let Ok(token) = stream.peek() {
            // We should get a valid token (the reparsed first token)
            let _ = format!("{:?}", token.kind);
        }
        Ok(())
    }

    #[test]
    fn invalidate_peek_clears_cache() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("1 + 2");
        let _ = stream.peek();
        let _ = stream.peek_second();
        // Invalidate all cached peeks
        stream.invalidate_peek();
        // Should still work after invalidation
        if let Ok(token) = stream.peek() {
            let _ = format!("{:?}", token.kind);
        }
        Ok(())
    }

    #[test]
    fn peek_fresh_kind_on_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("");
        let kind = stream.peek_fresh_kind();
        // Should return Some(Eof) or similar
        assert!(kind.is_some());
        Ok(())
    }

    #[test]
    fn peek_fresh_kind_on_valid_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("my $x;");
        let kind = stream.peek_fresh_kind();
        assert!(kind.is_some());
        Ok(())
    }

    #[test]
    fn enter_format_mode_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TokenStream::new("format STDOUT =\nsome text\n.\n");
        stream.enter_format_mode();
        // Should still be able to get tokens
        if let Ok(token) = stream.peek() {
            let _ = format!("{:?}", token.kind);
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Trivia extended tests
// ───────────────────────────────────────────────────────────────────

mod trivia_extended_tests {
    use super::*;

    #[test]
    fn pod_comment_variant() -> Result<(), Box<dyn std::error::Error>> {
        let trivia = Trivia::PodComment("=head1 NAME\n\nMy Module\n\n=cut".to_string());
        assert_eq!(trivia.as_str(), "=head1 NAME\n\nMy Module\n\n=cut");
        assert_eq!(trivia.kind_name(), "pod");
        Ok(())
    }

    #[test]
    fn kind_name_for_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Trivia::Whitespace("  ".to_string()).kind_name(), "whitespace");
        assert_eq!(Trivia::LineComment("# comment".to_string()).kind_name(), "comment");
        assert_eq!(Trivia::PodComment("=pod".to_string()).kind_name(), "pod");
        assert_eq!(Trivia::Newline.kind_name(), "newline");
        Ok(())
    }

    #[test]
    fn trivia_as_str_newline() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Trivia::Newline.as_str(), "\n");
        Ok(())
    }

    #[test]
    fn trivia_clone_equality() -> Result<(), Box<dyn std::error::Error>> {
        let t = Trivia::Whitespace("\t".to_string());
        let cloned = t.clone();
        assert_eq!(t, cloned);
        Ok(())
    }

    #[test]
    fn trivia_different_variants_not_equal() -> Result<(), Box<dyn std::error::Error>> {
        let ws = Trivia::Whitespace(" ".to_string());
        let nl = Trivia::Newline;
        assert_ne!(ws, nl);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// BudgetTracker extended edge cases
// ───────────────────────────────────────────────────────────────────

mod budget_tracker_extended_tests {
    use super::*;

    #[test]
    fn depth_tracking_max_depth_reached() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.enter_depth();
        tracker.enter_depth();
        tracker.enter_depth();
        assert_eq!(tracker.max_depth_reached, 3);
        assert_eq!(tracker.current_depth, 3);
        tracker.exit_depth();
        assert_eq!(tracker.current_depth, 2);
        // max_depth_reached stays at 3
        assert_eq!(tracker.max_depth_reached, 3);
        Ok(())
    }

    #[test]
    fn exit_depth_at_zero_saturates() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        assert_eq!(tracker.current_depth, 0);
        tracker.exit_depth();
        // Should not underflow
        assert_eq!(tracker.current_depth, 0);
        Ok(())
    }

    #[test]
    fn record_skip_saturating_add() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.tokens_skipped = usize::MAX - 1;
        tracker.record_skip(5);
        // Should saturate at usize::MAX
        assert_eq!(tracker.tokens_skipped, usize::MAX);
        Ok(())
    }

    #[test]
    fn record_error_saturating() -> Result<(), Box<dyn std::error::Error>> {
        let mut tracker = BudgetTracker::new();
        tracker.errors_emitted = usize::MAX - 1;
        tracker.record_error();
        assert_eq!(tracker.errors_emitted, usize::MAX);
        tracker.record_error();
        // Should stay at MAX
        assert_eq!(tracker.errors_emitted, usize::MAX);
        Ok(())
    }

    #[test]
    fn begin_recovery_increments_and_returns_true() -> Result<(), Box<dyn std::error::Error>> {
        let budget = ParseBudget { max_recoveries: 3, ..ParseBudget::default() };
        let mut tracker = BudgetTracker::new();
        assert!(tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 1);
        assert!(tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 2);
        assert!(tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 3);
        // 4th attempt should fail
        assert!(!tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 3);
        Ok(())
    }

    #[test]
    fn parse_budget_for_ide_equals_default() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ParseBudget::for_ide(), ParseBudget::default());
        Ok(())
    }

    #[test]
    fn parse_budget_unlimited_values() -> Result<(), Box<dyn std::error::Error>> {
        let unlimited = ParseBudget::unlimited();
        assert_eq!(unlimited.max_errors, usize::MAX);
        assert_eq!(unlimited.max_depth, usize::MAX);
        assert_eq!(unlimited.max_tokens_skipped, usize::MAX);
        assert_eq!(unlimited.max_recoveries, usize::MAX);
        Ok(())
    }

    #[test]
    fn parse_budget_strict_values() -> Result<(), Box<dyn std::error::Error>> {
        let strict = ParseBudget::strict();
        assert_eq!(strict.max_errors, 10);
        assert_eq!(strict.max_depth, 64);
        assert_eq!(strict.max_tokens_skipped, 100);
        assert_eq!(strict.max_recoveries, 50);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// RecoveryParseError builder chain tests
// ───────────────────────────────────────────────────────────────────

mod recovery_parse_error_extended_tests {
    use super::*;
    use perl_parser_core::position::Range;

    #[test]
    fn builder_chain_all_methods() -> Result<(), Box<dyn std::error::Error>> {
        let range = Range::new(
            perl_parser_core::position::Position::new(0, 1, 1),
            perl_parser_core::position::Position::new(5, 1, 6),
        );
        let err = RecoveryParseError::new("test error".to_string(), range)
            .with_expected(vec!["semicolon".to_string(), "brace".to_string()])
            .with_found("comma".to_string())
            .with_hint("try adding a semicolon".to_string());

        assert_eq!(err.message, "test error");
        assert_eq!(err.expected.len(), 2);
        assert_eq!(err.found, "comma");
        assert_eq!(err.recovery_hint.as_deref().unwrap_or(""), "try adding a semicolon");
        Ok(())
    }

    #[test]
    fn new_sets_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let range = Range::new(
            perl_parser_core::position::Position::new(0, 1, 1),
            perl_parser_core::position::Position::new(0, 1, 1),
        );
        let err = RecoveryParseError::new("msg".to_string(), range);
        assert!(err.expected.is_empty());
        assert!(err.found.is_empty());
        assert!(err.recovery_hint.is_none());
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// RecoveryResult extended tests
// ───────────────────────────────────────────────────────────────────

mod recovery_result_extended_tests {
    use super::*;

    #[test]
    fn recovered_with_count() -> Result<(), Box<dyn std::error::Error>> {
        let r = RecoveryResult::Recovered(5);
        if let RecoveryResult::Recovered(n) = r {
            assert_eq!(n, 5);
        } else {
            return Err("expected Recovered".into());
        }
        Ok(())
    }

    #[test]
    fn all_variants_debug() -> Result<(), Box<dyn std::error::Error>> {
        let variants = vec![
            RecoveryResult::Recovered(0),
            RecoveryResult::AtSyncPoint,
            RecoveryResult::BudgetExhausted,
            RecoveryResult::ReachedEof,
        ];
        for v in &variants {
            let dbg = format!("{:?}", v);
            assert!(!dbg.is_empty());
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// SyncPoint extended tests
// ───────────────────────────────────────────────────────────────────

mod sync_point_extended_tests {
    use super::*;

    #[test]
    fn all_variants_exist() -> Result<(), Box<dyn std::error::Error>> {
        let variants =
            [SyncPoint::Semicolon, SyncPoint::CloseBrace, SyncPoint::Keyword, SyncPoint::Eof];
        // All are distinct
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
        Ok(())
    }

    #[test]
    fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
        assert!(format!("{:?}", SyncPoint::Semicolon).contains("Semicolon"));
        assert!(format!("{:?}", SyncPoint::CloseBrace).contains("CloseBrace"));
        assert!(format!("{:?}", SyncPoint::Keyword).contains("Keyword"));
        assert!(format!("{:?}", SyncPoint::Eof).contains("Eof"));
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// Builtin signatures tests
// ───────────────────────────────────────────────────────────────────

mod builtin_signatures_tests {
    use super::*;

    #[test]
    fn builtin_signatures_contains_common_functions() -> Result<(), Box<dyn std::error::Error>> {
        let sigs = builtin_signatures::create_builtin_signatures();
        // Some well-known Perl builtins should be present
        assert!(sigs.contains_key("print"), "missing 'print'");
        assert!(sigs.contains_key("push"), "missing 'push'");
        assert!(sigs.contains_key("pop"), "missing 'pop'");
        assert!(sigs.contains_key("chomp"), "missing 'chomp'");
        assert!(sigs.contains_key("open"), "missing 'open'");
        assert!(sigs.contains_key("close"), "missing 'close'");
        Ok(())
    }

    #[test]
    fn builtin_signatures_phf_contains_common_functions() -> Result<(), Box<dyn std::error::Error>>
    {
        let phf = &builtin_signatures_phf::BUILTIN_SIGS;
        assert!(phf.contains_key("print"), "missing 'print' in phf");
        assert!(phf.contains_key("push"), "missing 'push' in phf");
        assert!(phf.contains_key("chomp"), "missing 'chomp' in phf");
        Ok(())
    }

    #[test]
    fn builtin_signatures_not_empty() -> Result<(), Box<dyn std::error::Error>> {
        let sigs = builtin_signatures::create_builtin_signatures();
        assert!(sigs.len() > 50, "expected >50 builtins, got {}", sigs.len());
        Ok(())
    }

    #[test]
    fn phf_map_not_empty() -> Result<(), Box<dyn std::error::Error>> {
        let phf = &builtin_signatures_phf::BUILTIN_SIGS;
        assert!(!phf.is_empty());
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// V1 Node extended tests
// ───────────────────────────────────────────────────────────────────

mod v1_node_extended_tests {
    use super::*;

    #[test]
    fn node_with_children() -> Result<(), Box<dyn std::error::Error>> {
        let child =
            V1Node::new(V1NodeKind::Number { value: "42".to_string() }, SourceLocation::new(0, 2));
        let program = V1Node::new(
            V1NodeKind::Program { statements: vec![child.clone()] },
            SourceLocation::new(0, 2),
        );
        if let V1NodeKind::Program { statements } = &program.kind {
            assert_eq!(statements.len(), 1);
        } else {
            return Err("expected Program".into());
        }
        Ok(())
    }

    #[test]
    fn variable_declaration_node() -> Result<(), Box<dyn std::error::Error>> {
        let var = V1Node::new(
            V1NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
            SourceLocation::new(3, 5),
        );
        let decl = V1Node::new(
            V1NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var),
                attributes: vec![],
                initializer: None,
            },
            SourceLocation::new(0, 5),
        );
        if let V1NodeKind::VariableDeclaration { declarator, attributes, initializer, .. } =
            &decl.kind
        {
            assert_eq!(declarator, "my");
            assert!(attributes.is_empty());
            assert!(initializer.is_none());
        } else {
            return Err("expected VariableDeclaration".into());
        }
        Ok(())
    }

    #[test]
    fn error_node_fields() -> Result<(), Box<dyn std::error::Error>> {
        let node = V1Node::new(
            V1NodeKind::Error {
                message: "bad stuff".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            SourceLocation::new(10, 15),
        );
        if let V1NodeKind::Error { message, expected, found, partial } = &node.kind {
            assert_eq!(message, "bad stuff");
            assert!(expected.is_empty());
            assert!(found.is_none());
            assert!(partial.is_none());
        } else {
            return Err("expected Error".into());
        }
        Ok(())
    }

    #[test]
    fn to_sexp_empty_program() -> Result<(), Box<dyn std::error::Error>> {
        let node =
            V1Node::new(V1NodeKind::Program { statements: vec![] }, SourceLocation::new(0, 0));
        let sexp = node.to_sexp();
        assert!(sexp.contains("source_file"), "sexp should contain source_file: {}", sexp);
        Ok(())
    }

    #[test]
    fn to_sexp_with_number() -> Result<(), Box<dyn std::error::Error>> {
        let num =
            V1Node::new(V1NodeKind::Number { value: "99".to_string() }, SourceLocation::new(0, 2));
        let prog =
            V1Node::new(V1NodeKind::Program { statements: vec![num] }, SourceLocation::new(0, 2));
        let sexp = prog.to_sexp();
        assert!(sexp.contains("number"), "sexp should contain number: {}", sexp);
        Ok(())
    }

    #[test]
    fn node_debug_format() -> Result<(), Box<dyn std::error::Error>> {
        let node = V1Node::new(V1NodeKind::MissingExpression, SourceLocation::new(0, 0));
        let dbg = format!("{:?}", node);
        assert!(dbg.contains("MissingExpression"));
        Ok(())
    }

    #[test]
    fn node_clone_equals() -> Result<(), Box<dyn std::error::Error>> {
        let node =
            V1Node::new(V1NodeKind::Number { value: "1".to_string() }, SourceLocation::new(0, 1));
        let cloned = node.clone();
        assert_eq!(node, cloned);
        Ok(())
    }

    #[test]
    fn block_node() -> Result<(), Box<dyn std::error::Error>> {
        let stmt =
            V1Node::new(V1NodeKind::Number { value: "1".to_string() }, SourceLocation::new(1, 2));
        let block =
            V1Node::new(V1NodeKind::Block { statements: vec![stmt] }, SourceLocation::new(0, 3));
        if let V1NodeKind::Block { statements } = &block.kind {
            assert_eq!(statements.len(), 1);
        } else {
            return Err("expected Block".into());
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// DiagnosticId type alias test
// ───────────────────────────────────────────────────────────────────

mod diagnostic_id_tests {
    use perl_parser_core::DiagnosticId;

    #[test]
    fn diagnostic_id_is_u32() -> Result<(), Box<dyn std::error::Error>> {
        let id: DiagnosticId = 42;
        assert_eq!(id, 42u32);
        let id2: DiagnosticId = 0;
        assert_eq!(id2, 0u32);
        let id3: DiagnosticId = u32::MAX;
        assert_eq!(id3, u32::MAX);
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────
// LineIndex extended tests
// ───────────────────────────────────────────────────────────────────

mod line_index_extended_tests {
    use super::*;

    #[test]
    fn line_index_crlf_input() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\r\nline2\r\nline3".to_string();
        let index = LineIndex::new(source);
        let (line, _col) = index.offset_to_position(7); // start of "line2"
        assert_eq!(line, 1);
        Ok(())
    }

    #[test]
    fn line_index_unicode() -> Result<(), Box<dyn std::error::Error>> {
        let source = "héllo\nwörld".to_string();
        let index = LineIndex::new(source);
        let (line, _col) = index.offset_to_position(0);
        assert_eq!(line, 0);
        // 'h' + 'é'(2 bytes) + 'l' + 'l' + 'o' + '\n' = 7 bytes for first line
        let (line2, _col2) = index.offset_to_position(7);
        assert_eq!(line2, 1);
        Ok(())
    }

    #[test]
    fn line_index_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line1\nline2\n".to_string();
        let index = LineIndex::new(source);
        // Past the last newline
        let (line, _col) = index.offset_to_position(12);
        assert_eq!(line, 2);
        Ok(())
    }

    // ---- Wave 2B: Fat arrow as general separator ----

    #[test]
    fn wave2b_push_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @array => $value;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "push @array => $value should parse cleanly, got: {sexp}");
        assert!(sexp.contains("call push"), "should be a function call");
        Ok(())
    }

    #[test]
    fn wave2b_bless_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("bless \\%opts => $class;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "bless \\%opts => $class should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_push_fat_arrow_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @attrs => (key => $val);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "push @attrs => (key => $val) should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_push_comma_regression() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @array, $value;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "push @array, $value should still work, got: {sexp}");
        assert!(sexp.contains("call push"), "should be a function call");
        Ok(())
    }

    #[test]
    fn wave2b_indirect_call_regression() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("print $fh \"data\";");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "print $fh \"data\" should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2b_hash_fat_arrow_regression() -> Result<(), Box<dyn std::error::Error>> {
        // Hash construction should still work
        let mut parser = Parser::new("my %h = (key => 'value');");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "hash construction should still work, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2b_unshift_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("unshift @arr => $val;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "unshift @arr => $val should parse cleanly, got: {sexp}");
        assert!(sexp.contains("call unshift"), "should be a function call");
        Ok(())
    }

    #[test]
    fn wave2b_splice_mixed_comma_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        // `splice @a, 0, 1 => @replacement` — the `=>` after `1` is consumed by
        // the builtin argument loop as a separator, exactly like a comma.
        let mut parser = Parser::new("splice @a, 0, 1 => @replacement;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "splice @a, 0, 1 => @replacement should parse cleanly, got: {sexp}"
        );
        // splice uses the generic builtin path; the sexp uses ambiguous_function_call_expression
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call splice"),
            "should be a function call, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_tie_fat_arrow_in_args() -> Result<(), Box<dyn std::error::Error>> {
        // tie uses a dedicated AST handler; fat arrow must work in trailing args
        let mut parser = Parser::new("tie %hash, 'MyModule' => @args;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "tie %hash, 'MyModule' => @args should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_map_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
        // map with block then fat arrow before list
        let mut parser = Parser::new("map { $_ * 2 } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "map {{ $_ * 2 }} => @list should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_grep_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("grep { defined } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "grep {{ defined }} => @list should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_sort_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("sort { $a <=> $b } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "sort {{ $a <=> $b }} => @list should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    // ---- Wave 2B-ext: Fat arrow in postfix builtin paths ----

    #[test]
    fn wave2b_bless_hash_literal_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        // bless {} => $class  —  exercises the bless-with-LeftBrace path in postfix.rs
        let mut parser = Parser::new("bless {} => $class;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "bless {{}} => $class should parse cleanly, got: {sexp}");
        assert!(sexp.contains("call bless"), "should be a bless call, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2b_bless_hash_with_entries_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        // bless { key => 1 } => $class
        let mut parser = Parser::new("bless { key => 1 } => $class;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "bless {{ key => 1 }} => $class should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_split_regex_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        // split /,/ => @parts  —  exercises the split-with-Slash path in postfix.rs
        let mut parser = Parser::new("split /,/ => @parts;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split /,/ => @parts should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2b_unshift_fat_arrow_multiple() -> Result<(), Box<dyn std::error::Error>> {
        // unshift @arr => 1, 2, 3  —  fat arrow then commas
        let mut parser = Parser::new("unshift @arr => 1, 2, 3;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "unshift @arr => 1, 2, 3 should parse cleanly, got: {sexp}"
        );
        assert!(sexp.contains("call unshift"), "should be an unshift call, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2b_grep_block_fat_arrow_list() -> Result<(), Box<dyn std::error::Error>> {
        // grep { defined } => @list  —  exercises the sort/map/grep block path in postfix.rs
        // (when reached via postfix, e.g. inside an expression context)
        let mut parser = Parser::new("my @r = grep { defined } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "grep {{ defined }} => @list in assignment should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_bless_hash_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
        // my $obj = bless {} => $class  —  in assignment, goes through postfix.rs bless path
        let mut parser = Parser::new("my $obj = bless {} => $class;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "bless {{}} => $class in assignment should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_split_regex_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
        // my @parts = split /,/ => $str  —  in assignment, goes through postfix.rs split path
        let mut parser = Parser::new("my @parts = split /,/ => $str;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "split /,/ => $str in assignment should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_sort_block_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
        // my @s = sort { $a <=> $b } => @list  —  in assignment, goes through postfix.rs
        let mut parser = Parser::new("my @s = sort { $a <=> $b } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "sort {{ $a <=> $b }} => @list in assignment should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2b_map_block_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
        // my @r = map { $_ * 2 } => @list
        let mut parser = Parser::new("my @r = map { $_ * 2 } => @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "map {{ $_ * 2 }} => @list in assignment should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    // ---- Wave 2C: split /regex/ ----

    #[test]
    fn wave2c_split_regex() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("split /\\./, $string;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split /\\./, $string should parse cleanly, got: {sexp}");
        assert!(sexp.contains("regex"), "should contain a regex node");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("split /\\s+/, $cmd;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split /\\s+/, $cmd should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my @parts = split /::/, $module;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "my @parts = split /::/, $module should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2c_split_parens_regression() -> Result<(), Box<dyn std::error::Error>> {
        // Parenthesized form should still work
        let mut parser = Parser::new("split(/\\./, $x);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split(/\\./, $x) should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_string_regression() -> Result<(), Box<dyn std::error::Error>> {
        // split with string pattern should still work
        let mut parser = Parser::new("split ',', $csv;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split with string should still work, got: {sexp}");
        Ok(())
    }

    // ---- Wave 2C+: split /regex/ in expression contexts (not just statement start) ----

    #[test]
    fn wave2c_split_regex_in_assignment_comma_pattern() -> Result<(), Box<dyn std::error::Error>> {
        // split with a single-char regex pattern containing comma
        let mut parser = Parser::new("my @p = split /,/, $s;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "my @p = split /,/, $s should parse cleanly, got: {sexp}");
        assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_after_return() -> Result<(), Box<dyn std::error::Error>> {
        // return split /regex/, $var — split in expression context after return
        let mut parser = Parser::new("return split /\\s+/, $line;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "return split /\\s+/, $line should parse cleanly, got: {sexp}"
        );
        assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_inside_push_args() -> Result<(), Box<dyn std::error::Error>> {
        // push @r, split /;/, $v — split as argument to another builtin
        let mut parser = Parser::new("push @r, split /;/, $v;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "push @r, split /;/, $v should parse cleanly, got: {sexp}"
        );
        assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_in_for_list() -> Result<(), Box<dyn std::error::Error>> {
        // split in for loop list context
        let mut parser = Parser::new("for my $x (split /,/, $s) { }");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "for my $x (split /,/, $s) should parse cleanly, got: {sexp}"
        );
        assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_in_ternary() -> Result<(), Box<dyn std::error::Error>> {
        // split in ternary expression
        let mut parser = Parser::new("my @r = $flag ? split(/,/, $a) : split(/;/, $b);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "ternary with split should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_chained() -> Result<(), Box<dyn std::error::Error>> {
        // join of split — split as argument inside another function call
        let mut parser = Parser::new("my $x = join('-', split /\\s+/, $input);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "join('-', split /\\s+/, $input) should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_in_array_ref() -> Result<(), Box<dyn std::error::Error>> {
        // split result stored in an anonymous array ref
        let mut parser = Parser::new("my $r = [split /,/, $s];");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "[split /,/, $s] should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_conditional_or() -> Result<(), Box<dyn std::error::Error>> {
        // split in || expression
        let mut parser = Parser::new("my @r = split(/,/, $s) || die;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split(/,/, $s) || die should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_three_args() -> Result<(), Box<dyn std::error::Error>> {
        // split with limit argument
        let mut parser = Parser::new("my @p = split /,/, $s, 3;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "split /,/, $s, 3 should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2c_split_regex_no_parens_method_chain() -> Result<(), Box<dyn std::error::Error>> {
        // using scalar result of split
        let mut parser = Parser::new("my $count = scalar(split /,/, $s);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "scalar(split /,/, $s) should parse cleanly, got: {sexp}");
        Ok(())
    }

    // ---- Wave 2D: Postfix modifiers after complex expressions ----

    #[test]
    fn wave2d_push_deref_with_if() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @{$hash{key}}, $val if $cond;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "push @{{$hash{{key}}}}, $val if $cond should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2d_push_deref_simple() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @{$arr}, 1;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "push @{{$arr}}, 1 should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2d_or_assign_for() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$hash{$_} ||= '' for @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "$hash{{$_}} ||= '' for @list should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2d_simple_modifier_regression() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("print $msg unless $quiet;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "print $msg unless $quiet should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2d_do_thing_for_list() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("do_thing() for @list;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "do_thing() for @list should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2d_deref_hash_push() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("push @{$self->{items}}, $item;");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "push @{{$self->{{items}}}}, $item should parse cleanly, got: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn wave2d_complex_lvalue_while() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("chomp($line) while ($line = <STDIN>);");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        Ok(())
    }
}

// ---- Wave 2A: Package-qualified subscripts ----

// ───────────────────────────────────────────────────────────────────
// Keyword autoquoting before fat arrow (=>)
// ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used)]
mod keyword_autoquoting_tests {
    use super::*;

    /// Helper: parse source, assert no errors, return sexp string.
    fn parse_ok(src: &str) -> String {
        let mut parser = Parser::new(src);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "parse should succeed without errors for: {src}\ngot: {sexp}"
        );
        sexp
    }

    // ── Statement-level keyword autoquoting ──────────────────────

    #[test]
    fn if_before_fat_arrow_in_hash_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (if => 1, for => 2, while => 3);");
        // All three keywords should be treated as string keys
        assert!(sexp.contains("(string \"if\")"), "if should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"for\")"), "for should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"while\")"), "while should be autoquoted: {sexp}");
        Ok(())
    }

    #[test]
    fn my_and_use_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (my => \"value\", use => \"something\");");
        assert!(sexp.contains("(string \"my\")"), "my should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"use\")"), "use should be autoquoted: {sexp}");
        Ok(())
    }

    #[test]
    fn return_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (return => 1);");
        assert!(sexp.contains("(string \"return\")"), "return should be autoquoted: {sexp}");
        Ok(())
    }

    #[test]
    fn unless_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (unless => 1);");
        assert!(sexp.contains("(string \"unless\")"), "unless should be autoquoted: {sexp}");
        Ok(())
    }

    #[test]
    fn next_last_redo_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (next => 1, last => 2, redo => 3);");
        assert!(sexp.contains("(string \"next\")"), "next should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"last\")"), "last should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"redo\")"), "redo should be autoquoted: {sexp}");
        Ok(())
    }

    #[test]
    fn sub_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (sub => \\&handler);");
        assert!(sexp.contains("(string \"sub\")"), "sub should be autoquoted: {sexp}");
        Ok(())
    }

    // ── Function call arguments ─────────────────────────────────

    #[test]
    fn keyword_autoquoted_in_function_call() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("func(return => 1);");
        assert!(
            sexp.contains("(string \"return\")"),
            "return should be autoquoted in func call: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn keyword_autoquoted_in_function_call_if() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("func(if => 1);");
        assert!(sexp.contains("(string \"if\")"), "if should be autoquoted in func call: {sexp}");
        Ok(())
    }

    // ── Hash constructor with braces ────────────────────────────

    #[test]
    fn keyword_in_brace_hash_constructor() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my $h = { if => 1, for => 2 };");
        assert!(!sexp.contains("(if "), "if should NOT be parsed as control flow: {sexp}");
        Ok(())
    }

    // ── Statement-level bare keyword => value ───────────────────

    #[test]
    fn bare_if_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
        // `if => 1;` at statement level should be an expression, not an if-statement
        let sexp = parse_ok("if => 1;");
        assert!(
            sexp.contains("(string \"if\")"),
            "if should be autoquoted at statement level: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn bare_return_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("return => 1;");
        assert!(
            sexp.contains("(string \"return\")"),
            "return should be autoquoted at statement level: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn bare_for_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("for => 1;");
        assert!(
            sexp.contains("(string \"for\")"),
            "for should be autoquoted at statement level: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn multiple_keyword_pairs_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("if => 1, for => 2, while => 3;");
        assert!(sexp.contains("(string \"if\")"), "if should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"for\")"), "for should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"while\")"), "while should be autoquoted: {sexp}");
        Ok(())
    }

    // ── Normal keyword usage should still work ──────────────────

    #[test]
    fn if_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("if (1) { 2; }");
        assert!(sexp.contains("(if "), "if statement should still parse: {sexp}");
        Ok(())
    }

    #[test]
    fn while_loop_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("while (1) { 2; }");
        assert!(sexp.contains("(while "), "while loop should still parse: {sexp}");
        Ok(())
    }

    #[test]
    fn for_loop_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("for my $i (1..10) { 2; }");
        assert!(
            sexp.contains("for") || sexp.contains("(foreach "),
            "for loop should still parse: {sexp}"
        );
        Ok(())
    }

    #[test]
    fn return_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("return 42;");
        assert!(sexp.contains("(return "), "return statement should still parse: {sexp}");
        Ok(())
    }

    #[test]
    fn my_declaration_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my $x = 1;");
        assert!(sexp.contains("my_declaration"), "my declaration should still parse: {sexp}");
        Ok(())
    }

    #[test]
    fn use_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("use strict;");
        assert!(sexp.contains("(use "), "use statement should still parse: {sexp}");
        Ok(())
    }

    // ── Hash subscript should NOT autoquote ─────────────────────

    #[test]
    fn hash_subscript_unless_is_identifier() -> Result<(), Box<dyn std::error::Error>> {
        // $hash{unless} is a hash subscript, not autoquoting
        let sexp = parse_ok("$hash{unless};");
        // This should parse as a hash subscript, not trigger autoquoting logic
        assert!(!sexp.contains("ERROR"), "hash subscript with keyword should parse: {sexp}");
        Ok(())
    }

    // ── Additional keywords ─────────────────────────────────────

    #[test]
    fn eval_do_try_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (eval => 1, do => 2);");
        assert!(!sexp.contains("ERROR"), "eval/do before => should parse: {sexp}");
        Ok(())
    }

    #[test]
    fn package_class_method_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (package => 1, class => 2, method => 3);");
        assert!(!sexp.contains("ERROR"), "package/class/method before => should parse: {sexp}");
        Ok(())
    }

    #[test]
    fn begin_end_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (BEGIN => 1, END => 2);");
        assert!(!sexp.contains("ERROR"), "BEGIN/END before => should parse: {sexp}");
        Ok(())
    }

    // ── Regular identifier autoquoting still works ──────────────

    #[test]
    fn regular_bareword_autoquoted() -> Result<(), Box<dyn std::error::Error>> {
        let sexp = parse_ok("my %h = (foo => 1, bar => 2);");
        assert!(sexp.contains("(string \"foo\")"), "foo should be autoquoted: {sexp}");
        assert!(sexp.contains("(string \"bar\")"), "bar should be autoquoted: {sexp}");
        Ok(())
    }
}

mod wave2a_qualified_subscripts {
    use super::*;

    #[test]
    fn wave2a_scalar_qualified_array_subscript() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$Text::Unidecode::Char[255];");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        assert!(sexp.contains("[]"), "should have array subscript");
        Ok(())
    }

    #[test]
    fn wave2a_scalar_qualified_array_subscript_hex() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$Text::Unidecode::Char[0xff];");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        assert!(sexp.contains("[]"), "should have array subscript");
        Ok(())
    }

    #[test]
    fn wave2a_scalar_qualified_hash_subscript() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$Package::Hash{key};");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        assert!(sexp.contains("{}"), "should have hash subscript");
        Ok(())
    }

    #[test]
    fn wave2a_deep_qualified_array() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$A::B::C::D[42];");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2a_qualified_hash_string_key() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("$Config::Config{'osname'};");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2a_qualified_in_expression() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("my $val = $Pkg::data{$key};");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        Ok(())
    }

    #[test]
    fn wave2a_unqualified_regression() -> Result<(), Box<dyn std::error::Error>> {
        // Make sure normal subscripts still work
        let mut parser = Parser::new("$hash{key};");
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Bare block followed by for-loop (regression tests for is_compound_statement)
// ---------------------------------------------------------------------------
//
// A bare `{ ... }` block is a compound statement in Perl.  When it is followed
// by `for my $var (LIST) { BODY }` on the next line the parser must NOT
// interpret the `for` as a postfix statement modifier on the block.  Without
// the fix, the parser would produce:
//
//   (statement_modifier_for (block ...) my) ... ERROR
//
// because `is_compound_statement` did not include `Block`, so the postfix-for
// check fired.

#[cfg(test)]
mod bare_block_for_regression {
    use perl_parser_core::Parser;

    fn parse_clean(src: &str) -> Result<(), String> {
        let mut parser = Parser::new(src);
        let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;
        let sexp = ast.to_sexp();
        if sexp.contains("ERROR") {
            return Err(format!("expected clean parse, got ERROR nodes in: {sexp}\nsource: {src}"));
        }
        Ok(())
    }

    #[test]
    fn bare_block_then_for_my_var() {
        parse_clean("{ my $x = 1; }\nfor my $i (1..3) { print $i; }\n").unwrap();
    }

    #[test]
    fn bare_block_then_foreach_my_var() {
        parse_clean("{ my $y = 2; }\nforeach my $item (@arr) { print $item; }\n").unwrap();
    }

    #[test]
    fn bare_block_then_for_my_alias() {
        // Real-world pattern from List::SomeUtils / Module::Implementation
        parse_clean(
            r#"
{
    my $loader = build_loader_sub(
        implementations => [ 'XS', 'PP' ],
        symbols         => \@subs,
    );
    $loader->();
}

for my $alias ( keys %aliases ) {
    no strict 'refs';
    *{$alias} = __PACKAGE__->can( $aliases{$alias} );
}
"#,
        )
        .unwrap();
    }

    #[test]
    fn if_block_then_for_still_works() {
        // if/while/etc. blocks were already compound — should be unaffected
        parse_clean("if (1) { my $x = 1; }\nfor my $i (1..3) { print $i; }\n").unwrap();
    }

    #[test]
    fn nested_bare_blocks_then_for() {
        parse_clean("{ { my $x = 1; } }\nfor my $k (keys %h) { print $k; }\n").unwrap();
    }
}
