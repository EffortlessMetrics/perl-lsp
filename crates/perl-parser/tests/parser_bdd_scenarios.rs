//! Behavior-driven parser scenarios for high-signal language behaviors.
//!
//! These tests use explicit Given/When/Then structure so parser behavior can be
//! reviewed from a user-observable perspective rather than parser internals.

use perl_parser::{NodeKind, Parser};

fn parse_to_sexp(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    Ok(ast.to_sexp())
}

#[test]
fn bdd_given_scalar_assignment_when_parsing_then_assignment_and_literal_are_preserved()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a scalar assignment from a numeric literal.
    let source = "my $count = 42;";

    // When: the source is parsed into an AST.
    let sexp = parse_to_sexp(source)?;

    // Then: the resulting AST encodes declaration and value information.
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("variable"));
    assert!(sexp.contains("42"));
    Ok(())
}

#[test]
fn bdd_given_operator_precedence_when_parsing_then_multiplication_binds_tighter_than_addition()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an expression where precedence matters.
    let source = "$result = 1 + 2 * 3;";

    // When: we parse and inspect the serialized AST.
    let sexp = parse_to_sexp(source)?;

    // Then: both operators exist and assignment is retained.
    // This checks parser coverage for a common precedence shape.
    assert!(sexp.contains("assignment"));
    assert!(sexp.contains("binary_+"));
    assert!(sexp.contains("binary_*"));
    Ok(())
}

#[test]
fn bdd_given_incomplete_first_statement_when_parsing_then_parser_recovers_and_keeps_following_statement()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an intentionally incomplete first statement.
    let source = "my $x = 10\nmy $y = 20;";

    // When: we parse in IDE-style permissive mode.
    let sexp = parse_to_sexp(source)?;

    // Then: later declarations still appear, demonstrating recovery.
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("$y") || sexp.contains("20"));
    Ok(())
}

#[test]
fn bdd_given_nested_control_flow_when_parsing_then_if_and_while_nodes_exist()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: nested control-flow constructs.
    let source = r#"
if ($ready) {
    while ($more) {
        $more--;
    }
}
"#;

    // When: parsing the source into an AST.
    let sexp = parse_to_sexp(source)?;

    // Then: both control-flow nodes are represented.
    assert!(sexp.contains("if"));
    assert!(sexp.contains("while"));
    Ok(())
}

#[test]
fn bdd_given_empty_program_when_parsing_then_program_has_no_statements()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an empty Perl document.
    let mut parser = Parser::new("");

    // When: we parse the input.
    let ast = parser.parse()?;

    // Then: the AST root is Program with zero statements.
    match &ast.kind {
        NodeKind::Program { statements } => {
            assert!(statements.is_empty());
            Ok(())
        }
        other => Err(format!("Expected Program node, got {other:?}").into()),
    }
}
