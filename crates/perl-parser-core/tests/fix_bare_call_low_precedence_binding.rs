mod cpan_test_helpers;
use cpan_test_helpers::parse;
use perl_parser_core::NodeKind;

fn first_program_statement(ast: &perl_parser_core::Node) -> &perl_parser_core::Node {
    match &ast.kind {
        NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "expected at least one statement");
            &statements[0]
        }
        _ => panic!("expected Program node"),
    }
}

fn last_program_statement(ast: &perl_parser_core::Node) -> &perl_parser_core::Node {
    match &ast.kind {
        NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "expected at least one statement");
            &statements[statements.len() - 1]
        }
        _ => panic!("expected Program node"),
    }
}

#[test]
fn test_bare_call_sigil_arg_stops_before_ternary() {
    let ast = parse("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    let stmt = last_program_statement(&ast);

    let initializer = match &stmt.kind {
        NodeKind::VariableDeclaration { initializer: Some(initializer), .. } => initializer,
        _ => panic!("expected variable declaration with initializer"),
    };

    let condition = match &initializer.kind {
        NodeKind::Ternary { condition, .. } => condition,
        _ => panic!("expected ternary initializer"),
    };

    match &condition.kind {
        NodeKind::FunctionCall { name, args } => {
            assert_eq!(name, "is_ready");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected ternary condition to be a function call"),
    }
}

#[test]
fn test_bare_call_sigil_arg_stops_before_word_or() {
    let ast = parse("do_thing @args or die;");
    let stmt = first_program_statement(&ast);

    let expr = match &stmt.kind {
        NodeKind::ExpressionStatement { expression } => expression,
        _ => panic!("expected expression statement"),
    };

    match &expr.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "or");
            match &left.kind {
                NodeKind::FunctionCall { name, args } => {
                    assert_eq!(name, "do_thing");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("expected left side of `or` to be function call"),
            }
        }
        _ => panic!("expected top-level binary expression"),
    }
}

#[test]
fn test_bare_call_sigil_arg_stops_before_word_and() {
    let ast = parse("do_thing $x and return;");
    let stmt = first_program_statement(&ast);

    let expr = match &stmt.kind {
        NodeKind::ExpressionStatement { expression } => expression,
        _ => panic!("expected expression statement"),
    };

    match &expr.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "and");
            match &left.kind {
                NodeKind::FunctionCall { name, args } => {
                    assert_eq!(name, "do_thing");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("expected left side of `and` to be function call"),
            }
        }
        _ => panic!("expected top-level binary expression"),
    }
}

#[test]
fn test_bare_call_sigil_arg_stops_before_defined_or() {
    let ast = parse("my $v = transform $x // $fallback;");
    let stmt = first_program_statement(&ast);

    let initializer = match &stmt.kind {
        NodeKind::VariableDeclaration { initializer: Some(initializer), .. } => initializer,
        _ => panic!("expected variable declaration with initializer"),
    };

    match &initializer.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "//");
            match &left.kind {
                NodeKind::FunctionCall { name, args } => {
                    assert_eq!(name, "transform");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("expected left side of `//` to be function call"),
            }
        }
        _ => panic!("expected defined-or binary expression"),
    }
}

#[test]
fn test_nested_bare_call_ternary_in_larger_expression() {
    let ast = parse("my $z = (is_ready $obj ? 1 : 0) + 2;");
    let stmt = first_program_statement(&ast);

    let initializer = match &stmt.kind {
        NodeKind::VariableDeclaration { initializer: Some(initializer), .. } => initializer,
        _ => panic!("expected variable declaration with initializer"),
    };

    match &initializer.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "+");
            match &left.kind {
                NodeKind::Ternary { condition, .. } => match &condition.kind {
                    NodeKind::FunctionCall { name, args } => {
                        assert_eq!(name, "is_ready");
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("expected ternary condition to be function call"),
                },
                _ => panic!("expected left side of `+` to be ternary"),
            }
        }
        _ => panic!("expected binary `+` initializer"),
    }
}
