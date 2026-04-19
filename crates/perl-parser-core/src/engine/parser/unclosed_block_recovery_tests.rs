/// Tests for improved error recovery when blocks are unclosed.
///
/// These tests verify that the parser produces useful partial ASTs
/// when encountering unclosed braces, rather than cascading errors
/// that make the rest of the file unparsable.
use super::*;
use perl_tdd_support::must;

/// Unclosed sub block at EOF should produce a partial Subroutine node
/// with the statements parsed so far.
#[test]
fn test_unclosed_sub_block_at_eof() {
    let code = "sub foo { my $x = 1; ";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Should return Ok with a partial AST, not Err
    assert!(
        result.is_ok(),
        "Parser should recover from unclosed sub block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        // We should have at least one statement: the (partial) subroutine
        assert!(
            !statements.is_empty(),
            "Program should have at least one statement"
        );

        // The first statement should be a Subroutine (possibly wrapped in Error with partial)
        let has_sub = statements.iter().any(|s| {
            matches!(&s.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo"))
                || matches!(&s.kind, NodeKind::Error { partial: Some(p), .. } if matches!(&p.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo")))
        });
        assert!(
            has_sub,
            "Should have a (possibly partial) subroutine node for 'foo'. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    // Should have recorded errors about the unclosed block
    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed block"
    );
}

/// Unclosed else block should still parse the if branch correctly.
#[test]
fn test_unclosed_else_block_parses_if_branch() {
    let code = "if ($cond) { $x = 1; } else { $y = 2;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed else block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Program should have at least one statement"
        );

        // Look for an If node (possibly wrapped in Error with partial)
        let has_if = statements.iter().any(|s| {
            matches!(&s.kind, NodeKind::If { .. })
                || matches!(&s.kind, NodeKind::Error { partial: Some(p), .. } if matches!(&p.kind, NodeKind::If { .. }))
        });
        assert!(
            has_if,
            "Should have an if statement node. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed else block"
    );
}

/// Nested unclosed blocks: inner block should parse, outer should recover.
#[test]
fn test_nested_unclosed_blocks() {
    let code = "{ my $x = 1; { my $y = 2; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from nested unclosed blocks"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Program should have at least one statement"
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed block"
    );
}

/// When a second sub is unclosed, the first sub should be fully parsed.
#[test]
fn test_first_sub_clean_when_second_unclosed() {
    let code = "sub foo { } sub bar { my $x = 1;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed second sub"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        // We should have at least 2 statements
        assert!(
            statements.len() >= 2,
            "Should have at least 2 statements (two subs). Got {} statements: {:?}",
            statements.len(),
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );

        // First statement should be a clean Subroutine named "foo"
        let first_is_foo = matches!(
            &statements[0].kind,
            NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo")
        );
        assert!(
            first_is_foo,
            "First statement should be sub foo. Got: {}",
            statements[0].kind.kind_name()
        );

        // Second statement should be a Subroutine (or Error with partial Subroutine) named "bar"
        let second_is_bar = statements[1..].iter().any(|s| {
            matches!(&s.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("bar"))
                || matches!(&s.kind, NodeKind::Error { partial: Some(p), .. } if matches!(&p.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("bar")))
        });
        assert!(
            second_is_bar,
            "Should have a (possibly partial) subroutine node for 'bar'. Got: {:?}",
            statements[1..]
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed block"
    );
}

/// Clean files should still parse correctly (no regression).
#[test]
fn test_clean_file_unaffected() {
    let code = "sub foo { my $x = 1; } sub bar { my $y = 2; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Clean file should parse successfully");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 2, "Should have exactly 2 subroutines");
        assert!(matches!(
            &statements[0].kind,
            NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo")
        ));
        assert!(matches!(
            &statements[1].kind,
            NodeKind::Subroutine { name, .. } if name.as_deref() == Some("bar")
        ));
    } else {
        panic!("Expected Program node");
    }

    assert!(
        parser.errors().is_empty(),
        "Clean file should have no errors"
    );
}

/// Unclosed block inside while loop should recover.
#[test]
fn test_unclosed_while_block_at_eof() {
    let code = "while (1) { print 'hello';";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed while block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(!statements.is_empty(), "Should have at least one statement");
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed block"
    );
}

/// Recovery when a new `sub` keyword is found while inside an unclosed block.
/// In Perl, nested named subs are valid, so `sub bar` inside `sub foo`'s block
/// is parsed as a nested subroutine. The recovery happens when foo's block
/// isn't closed at EOF — we still get a partial Subroutine node for foo
/// containing bar as a nested statement.
#[test]
fn test_recovery_on_sub_keyword_in_unclosed_block() {
    let code = "sub foo { my $x = 1;\nsub bar { my $y = 2; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover on encountering new sub"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        // We should have at least 1 top-level statement (foo with bar nested)
        assert!(
            !statements.is_empty(),
            "Should have at least 1 statement. Got {} statements: {:?}",
            statements.len(),
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );

        // The top-level sub foo should exist and contain bar as a nested sub
        let has_foo = statements.iter().any(|s| {
            matches!(&s.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo"))
        });
        assert!(
            has_foo,
            "Should have subroutine 'foo'. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );

        // foo's body should contain bar as a nested named sub
        if let NodeKind::Subroutine { body, .. } = &statements[0].kind {
            if let NodeKind::Block {
                statements: body_stmts,
            } = &body.kind
            {
                let has_bar = body_stmts.iter().any(|s| {
                    matches!(&s.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("bar"))
                });
                assert!(
                    has_bar,
                    "foo's body should contain nested sub bar. Got: {:?}",
                    body_stmts
                        .iter()
                        .map(|s| s.kind.kind_name())
                        .collect::<Vec<_>>()
                );
            }
        }
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed block in foo"
    );
}

/// Unclosed C-style for loop body should recover at EOF.
#[test]
fn test_unclosed_c_for_loop_body() {
    let code = "for (my $i = 0; $i < 10; $i++) { print $i;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed C-style for loop body"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement (the for loop). Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed for loop body"
    );
}

/// Unclosed foreach body should recover at EOF.
#[test]
fn test_unclosed_foreach_body() {
    let code = "foreach my $x (@list) { print $x;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed foreach body"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement (the foreach). Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed foreach body"
    );
}

/// Unclosed unless block should recover at EOF.
#[test]
fn test_unclosed_unless_block() {
    let code = "unless ($x) { do_thing();";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed unless block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement (the unless). Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed unless block"
    );
}

/// Unclosed BEGIN phase block should recover at EOF.
#[test]
fn test_unclosed_begin_phase_block() {
    let code = "BEGIN { use strict; use warnings;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed BEGIN block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(!statements.is_empty(), "Should have at least one statement");
        let has_phase = statements.iter().any(|s| {
            matches!(&s.kind, NodeKind::PhaseBlock { phase, .. } if phase == "BEGIN")
                || matches!(&s.kind, NodeKind::Error { .. })
        });
        assert!(
            has_phase,
            "Should have a PhaseBlock or Error for BEGIN. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed BEGIN block"
    );
}

/// Doubly-nested unclosed blocks (inner and outer both unclosed) should recover at EOF.
#[test]
fn test_doubly_nested_unclosed_blocks() {
    let code = "{ my $x = 1; { my $y = 2; my $z = 3;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from doubly-nested unclosed blocks"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about doubly-nested unclosed blocks"
    );
}

/// Unclosed nested block inside sub (two levels missing) should recover.
#[test]
fn test_unclosed_nested_block_inside_sub() {
    let code = "sub foo { if ($x) { my $y = 1;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed nested block inside sub"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
        let has_sub = statements.iter().any(|s| {
            matches!(&s.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo"))
                || matches!(&s.kind, NodeKind::Error { partial: Some(p), .. } if matches!(&p.kind, NodeKind::Subroutine { name, .. } if name.as_deref() == Some("foo")))
        });
        assert!(
            has_sub,
            "Should have a (possibly partial) subroutine node for 'foo'. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed nested block inside sub"
    );
}

/// Unclosed `until` loop body should recover at EOF.
/// `until` is `while !cond` — a distinct parse path from `while`.
#[test]
fn test_unclosed_until_loop_body() {
    let code = "until ($done) { process_item();";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed until loop body"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement (the until loop). Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed until loop body"
    );
}

/// Unclosed `eval { }` block should recover at EOF.
/// `eval` blocks are extremely common in Perl error-handling patterns; the LSP
/// must not cascade errors when a user is typing inside an `eval` body.
#[test]
fn test_unclosed_eval_block_at_eof() {
    let code = "eval { my $result = dangerous_call();";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed eval block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement (the eval). Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed eval block"
    );
}

/// Unclosed `do { }` block should recover at EOF.
/// `do { }` blocks are used for scoped variable lifetimes and as expression blocks.
#[test]
fn test_unclosed_do_block_at_eof() {
    let code = "my $result = do { my $tmp = compute();";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from unclosed do block"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed do block"
    );
}

/// A bare open brace at EOF (empty unclosed block) should not crash the parser.
/// This is the minimal possible unclosed-block scenario.
#[test]
fn test_bare_open_brace_at_eof() {
    let code = "{";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Parser should recover from a bare open brace at EOF"
    );
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            !statements.is_empty(),
            "Should have at least one statement. Got: {:?}",
            statements
                .iter()
                .map(|s| s.kind.kind_name())
                .collect::<Vec<_>>()
        );
    } else {
        panic!("Expected Program node");
    }

    assert!(
        !parser.errors().is_empty(),
        "Should have errors about unclosed bare block"
    );
}
