use perl_parser::Parser;

/// Helper to assert code parses successfully
fn assert_parses(code: &str) {
    use perl_tdd_support::must;
    let mut parser = Parser::new(code);
    must(parser.parse());
}

/// Helper to assert code fails to parse
#[allow(dead_code)]
fn assert_parse_fails(code: &str) {
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_err(), "Expected parse to fail but got AST:\n{:?}", result.ok());
}

#[test]
fn print_scalar_in_simple_context() {
    // Basic print $var should work
    assert_parses("print $x;");
    assert_parses("print $x");
    assert_parses("{ print $x; }");
    assert_parses("if (1) { print $x; }");
}

#[test]
fn print_scalar_after_my_inside_if() {
    let code = r#"
my $y = 10;
if (1) {
    print $y;
}
"#;
    assert_parses(code);
}

#[test]
fn print_scalar_with_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"print $x + 1;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x + 1");

    // Should parse as print($x + 1), NOT as indirect object
    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse arithmetic as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_scalar_with_string_concat() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"print $x . "s";"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x . \"s\"");

    // Should parse as print($x . "s"), NOT as indirect object
    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse string concat as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_indirect_object_still_works() {
    // These should parse as indirect object syntax
    assert_parses(r#"open($fh, '<', 'x.txt'); print $fh "hi\n";"#);
    assert_parses(r#"print STDOUT "hello";"#);
    assert_parses(r#"print STDERR "error", "\n";"#);
    assert_parses(r#"say $fh "message";"#);
}

#[test]
fn print_filehandle_then_variable_is_indirect() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure: print $fh $x; is treated as indirect object form
    let code = r#"print $fh $x;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $fh $x");

    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("indirect_call"),
        "print $fh $x should be treated as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_scalar_vs_indirect_object() {
    // print $var; should NOT be treated as indirect object
    assert_parses("print $x;");
    assert_parses("print $x, $y;");
    assert_parses("print $array[0];");
    assert_parses("print $hash{key};");

    // print $fh ... with more args should be indirect object
    assert_parses(r#"print $fh "text";"#);
    assert_parses(r#"print $fh "text", "more";"#);
}

#[test]
fn new_constructor_pattern() {
    assert_parses("new Class");
    assert_parses("new Class()");
    assert_parses("new Class('arg')");
    assert_parses("$obj = new Class;");
}

#[test]
fn statement_modifier_inside_block_if() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my @array = (1, 2, 3);
        foreach my $item (@array) {
            print "$item\n" if $item > 1;
        }
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    // We accept the statement modifier node in the output
    assert!(
        sexp.contains("statement_modifier") || sexp.contains("(if "),
        "expected statement_modifier or if in output; got: {sexp}"
    );
    Ok(())
}

#[test]
fn statement_modifier_inside_block_for() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my @arr = (1,2,3);
        print $_ for @arr;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier") || sexp.contains("(for ") || sexp.contains("(foreach "),
        "expected statement_modifier or for/foreach in output"
    );
    Ok(())
}

// Regression tests for declaration + control flow issues
#[test]
fn decl_then_if_allows_assignment() {
    let code = r#"my $x; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_call() {
    let code = r#"my $x; if (1) { foo("bar"); }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_print() {
    let code = r#"my $x; if (1) { print "hi"; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_postfix_if() {
    let code = r#"my @a=(1,2,3); if (1) { print "$_" if 1; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_foreach_allows_postfix_if() {
    let code = r#"my @a=(1,2,3); foreach my $x (@a) { print "$x\n" if $x > 1; }"#;
    assert_parses(code);
}

#[test]
fn package_then_if_allows_assignment() {
    let code = r#"package Foo; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn our_then_if_allows_assignment() {
    let code = r#"our $x; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_while_allows_assignment() {
    let code = r#"my $x; while (1) { $x = 5; last; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_foreach_allows_print() {
    let code = r#"my $x; foreach my $y (@a) { print "hi"; }"#;
    assert_parses(code);
}

#[test]
fn multiple_semicolons_in_block() {
    let code = r#"{ print "hi";; print "bye";;; }"#;
    assert_parses(code);
}

#[test]
fn empty_statements_allowed() {
    let code = r#";;; print "hi"; ;;;"#;
    assert_parses(code);
}

#[test]
fn statement_modifier_in_foreach_with_prior_decl() {
    let code = r#"
    my @array = (1, 2, 3);
    foreach my $item (@array) {
        print "$item\n" if $item > 1;
    }
    "#;
    assert_parses(code);
}

#[test]
fn complex_foreach_with_modifiers() {
    let code = r#"
    sub test {
        for my $i (1..10) {
            print "$i\n" if $i % 2;
        }
    }
    "#;
    assert_parses(code);
}

#[test]
fn statement_modifier_unless_and_while() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my $x = 0;
        print "ok\n" unless $x;
        print "loop\n" while $x < 0;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("statement_modifier"), "expected statement_modifier nodes in output");
    Ok(())
}

#[test]
fn statement_modifier_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    sub test {
        {
            my $count = 10;
            print "Count: $count\n" if $count > 5;
            last if $count == 10;
        }
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier"),
        "expected statement_modifier nodes in nested blocks"
    );
    Ok(())
}

#[test]
fn statement_modifier_with_complex_expression() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my $x = 5;
        print $x * 2, "\n" if $x > 0 && $x < 10;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier"),
        "expected statement_modifier with complex expression"
    );
    Ok(())
}
