//! Tests for word operator precedence (or, xor, and, not)
//!
//! According to Perl documentation, word operators have very low precedence,
//! lower than assignment operators. The precedence from highest to lowest is:
//! 1. Assignment operators (=, +=, etc.)
//! 2. Comma operator (,)
//! 3. List operators (rightward)
//! 4. Word NOT (not)
//! 5. Word AND (and)
//! 6. Word OR/XOR (or, xor)

use perl_parser::Parser;

#[test]
fn test_or_lower_than_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // $a = 1 or $b = 2 should parse as ($a = 1) or ($b = 2)
    let input = "$a = 1 or $b = 2";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    // Check that 'or' is at the top level by examining the S-expression
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(binary_or"));
    assert!(sexp.contains("(assignment_assign (variable $ a) (number 1))"));
    assert!(sexp.contains("(assignment_assign (variable $ b) (number 2))"));
    Ok(())
}

#[test]
fn test_and_lower_than_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // $a = 1 and $b = 2 should parse as ($a = 1) and ($b = 2)
    let input = "$a = 1 and $b = 2";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    // Check that 'and' is at the top level by examining the S-expression
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(binary_and"));
    assert!(sexp.contains("(assignment_assign (variable $ a) (number 1))"));
    assert!(sexp.contains("(assignment_assign (variable $ b) (number 2))"));
    Ok(())
}

#[test]
fn test_not_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // $a = not 0 should parse as $a = (not 0)
    let input = "$a = not 0";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    // Check that assignment contains 'not' as its RHS
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(assignment_assign"));
    assert!(sexp.contains("(unary_not (number 0))"));
    Ok(())
}

#[test]
fn test_or_and_precedence() -> Result<(), Box<dyn std::error::Error>> {
    // $a = 1 and $b = 2 or $c = 3 should parse as (($a = 1) and ($b = 2)) or ($c = 3)
    // because 'and' has higher precedence than 'or'
    let input = "$a = 1 and $b = 2 or $c = 3";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    // The top level should be 'or'
    assert!(sexp.starts_with("(source_file (binary_or"));
    // The left side of 'or' should be an 'and' expression
    assert!(sexp.contains("(binary_and (assignment_assign"));
    Ok(())
}

#[test]
fn test_statement_with_or_modifier() -> Result<(), Box<dyn std::error::Error>> {
    // open FILE, "test.txt" or die "error" should parse correctly
    let input = "open FILE, \"test.txt\" or die \"error\"";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("(binary_or"));
    assert!(sexp.contains("(call open"));
    assert!(sexp.contains("(call die"));
    Ok(())
}

#[test]
fn test_complex_word_operators() -> Result<(), Box<dyn std::error::Error>> {
    // Test complex expression with multiple word operators
    let input = "$result = $foo = 42 and $bar = 84 or $baz = 126";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    // Should parse as: (($result = ($foo = 42)) and ($bar = 84)) or ($baz = 126)
    assert!(sexp.contains("(binary_or"));
    assert!(sexp.contains("(binary_and"));
    Ok(())
}

#[test]
fn test_not_not_double_negation() -> Result<(), Box<dyn std::error::Error>> {
    // not not $x should parse as (not (not $x))
    let input = "not not $x";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("(unary_not (unary_not"));
    Ok(())
}

#[test]
fn test_xor_precedence() -> Result<(), Box<dyn std::error::Error>> {
    // xor has same precedence as or
    let input = "$a = 1 xor $b = 2";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("(binary_xor"));
    assert!(sexp.contains("(assignment_assign (variable $ a) (number 1))"));
    assert!(sexp.contains("(assignment_assign (variable $ b) (number 2))"));
    Ok(())
}

#[test]
fn test_word_vs_symbolic_operators() -> Result<(), Box<dyn std::error::Error>> {
    // || has much higher precedence than 'or'
    // $a = 0 || $b = 1 should parse as $a = (0 || ($b = 1))
    let input = "$a = 0 || $b = 1";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    // The assignment should contain the || expression
    assert!(sexp.contains("(assignment_assign"));
    assert!(sexp.contains("(binary_||"));
    // Note: This is different from word 'or' which would be at the top level
    Ok(())
}

#[test]
fn test_list_context_with_word_operators() -> Result<(), Box<dyn std::error::Error>> {
    // In list context, comma has higher precedence than word operators
    let input = "($a = 1, $b = 2) or die";
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("(binary_or"));
    // Parenthesized list expressions are parsed as arrays
    assert!(sexp.contains("(array"));
    Ok(())
}

/// Regression: indirect-call heuristic must treat uppercase bareword + comma as regular call
/// (`print STDERR, "x"` is NOT an indirect method call on STDERR).
#[test]
fn test_print_stderr_comma_or_die() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"print STDERR, "x" or die "err""#;
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    // Top-level should be `or` — the call is on the left, die on the right
    assert!(sexp.contains("(binary_or"), "expected binary_or at top level, got: {sexp}");
    assert!(sexp.contains("(call print"), "expected (call print ...), got: {sexp}");
    Ok(())
}

/// Regression: `WordNot` must terminate indirect-object argument collection.
/// `open FILE, "x" or not $failed` should parse `not` outside the arg list.
#[test]
fn test_word_not_as_terminator() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"open FILE, "x" or not $failed"#;
    let mut parser = Parser::new(input);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    // `or` at top level, `not` on the RHS
    assert!(sexp.contains("(binary_or"), "expected binary_or, got: {sexp}");
    assert!(sexp.contains("(unary_not"), "expected unary_not on RHS, got: {sexp}");
    assert!(sexp.contains("(call open"), "expected (call open ...), got: {sexp}");
    Ok(())
}
