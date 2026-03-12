use perl_parser::{NodeKind, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_x_repeat_operator_binary_expression() -> TestResult {
    let mut parser = Parser::new("'x' x 3");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Binary { op, .. } = &expression.kind {
                assert_eq!(op, "x");
            } else {
                return Err("expected binary expression for x repetition operator".into());
            }
        } else {
            return Err("expected expression statement".into());
        }
    } else {
        return Err("expected program node".into());
    }

    Ok(())
}

#[test]
fn test_x_repeat_operator_precedence_between_concat_and_multiply() -> TestResult {
    let mut parser = Parser::new("'a' . 'b' x 3");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Binary { op, left, right } = &expression.kind {
                assert_eq!(op, ".");
                if let NodeKind::String { .. } = &left.kind {
                    // expected
                } else {
                    return Err("expected string literal on left of concatenation".into());
                }

                if let NodeKind::Binary { op, .. } = &right.kind {
                    assert_eq!(op, "x");
                } else {
                    return Err("expected x repetition on right of concatenation".into());
                }
            } else {
                return Err("expected top-level concatenation binary expression".into());
            }
        } else {
            return Err("expected expression statement".into());
        }
    } else {
        return Err("expected program node".into());
    }

    Ok(())
}
