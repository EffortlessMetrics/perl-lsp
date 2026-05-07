// Regression tests for `pos` used as an lvalue.
//
// Root causes fixed:
// 1. `pos` was missing from the indirect-call exclusion list, so
//    `pos $s = value` was mis-parsed as an indirect method call
//    (method=pos, object=$s), which then failed on the `=`.
// 2. `parse_named_unary_statement_call` did not handle assignment
//    operators after the call node, so `pos $s = value` left `= value`
//    unparsed even after fix 1.
//
// Patterns from real CPAN code:
// - re.pm:212             `pos $s = $sav_pos - 1;`
// - Text/Balanced.pm:228  `pos $$textref = $startpos;`
// - Text/Balanced.pm:180  `pos = $posbug;`
// - Unicode/UCD.pm:869    `pos $x = 0;`

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

fn first_expression(source: &str) -> Result<Node, Box<dyn std::error::Error>> {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let NodeKind::Program { mut statements } = ast.kind else {
        return Err(format!("expected Program, got {sexp}").into());
    };
    if statements.len() != 1 {
        return Err(format!("expected one statement, got {} in {sexp}", statements.len()).into());
    }

    let statement = statements.remove(0);
    let NodeKind::ExpressionStatement { expression } = statement.kind else {
        return Err(
            format!("expected ExpressionStatement, got {}", statement.kind.kind_name()).into()
        );
    };

    Ok(*expression)
}

fn assert_pos_assignment_shape(
    source: &str,
    expected_arg_count: usize,
    expected_op: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let expression = first_expression(source)?;
    let NodeKind::Assignment { lhs, rhs: _, op } = expression.kind else {
        return Err(format!("expected Assignment, got {}", expression.kind.kind_name()).into());
    };
    assert_eq!(op, expected_op);

    let NodeKind::FunctionCall { name, args } = lhs.kind else {
        return Err(format!("expected pos() FunctionCall lhs, got {}", lhs.kind.kind_name()).into());
    };
    assert_eq!(name, "pos");
    assert_eq!(args.len(), expected_arg_count);
    Ok(())
}

#[test]
fn test_pos_lvalue_assign_scalar() -> Result<(), Box<dyn std::error::Error>> {
    // re.pm:212 and Unicode/UCD.pm:869
    assert_pos_assignment_shape("pos $s = $sav_pos - 1;", 1, "=")
}

#[test]
fn test_pos_lvalue_assign_zero() -> Result<(), Box<dyn std::error::Error>> {
    assert_pos_assignment_shape("pos $x = 0;", 1, "=")
}

#[test]
fn test_pos_lvalue_assign_deref_scalar() -> Result<(), Box<dyn std::error::Error>> {
    // Text/Balanced.pm:228, 249, 258, 268
    assert_pos_assignment_shape("pos $$textref = $startpos;", 1, "=")
}

#[test]
fn test_pos_lvalue_bare_assign() -> Result<(), Box<dyn std::error::Error>> {
    // Text/Balanced.pm:180 - pos without argument is an lvalue for $_
    assert_pos_assignment_shape("pos = $posbug;", 0, "=")
}

#[test]
fn test_pos_lvalue_augmented_assign() -> Result<(), Box<dyn std::error::Error>> {
    // Augmented assignment operators also valid (e.g. pos $s += 1)
    assert_pos_assignment_shape("pos $s += 1;", 1, "+=")
}

#[test]
fn test_pos_lvalue_in_block() {
    let source = r#"
sub import {
    while ($s =~ /(\w+)/g) {
        my $sav_pos = pos $s;
        my $count = $s =~ s/a//g;
        pos $s = $sav_pos - 1;
    }
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_pos_read_only_still_works() {
    // Read-only uses should still parse correctly
    assert_clean_parse("my $p = pos $s;");
    assert_clean_parse("if (pos $$textref) { return; }");
    assert_clean_parse("warn pos $s;");
}

#[test]
fn test_pos_no_arg_read() {
    // pos() with no arg reads pos of $_
    assert_clean_parse("my $p = pos;");
    assert_clean_parse("print pos;");
}
