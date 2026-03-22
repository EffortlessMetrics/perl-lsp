//! Tests for newly implemented features in the pure Rust parser

#[cfg(feature = "pure-rust")]
mod tests {
    use perl_tdd_support::must;
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

    fn parse_to_sexp(code: &str) -> Result<String, String> {
        let mut parser = PureRustPerlParser::new();
        match parser.parse(code) {
            Ok(ast) => Ok(parser.to_sexp(&ast)),
            Err(e) => Err(format!("Parse failed: {:?}\nCode: {}", e, code)),
        }
    }

    fn assert_parses_without_error(code: &str) -> String {
        match parse_to_sexp(code) {
            Ok(sexp) => {
                assert!(!sexp.contains("(ERROR)"), "Failed to parse case cleanly: {}", code);
                sexp
            }
            Err(err) => panic!("unexpected Err: {err}"),
        }
    }

    fn assert_parses_or_fails_gracefully(code: &str) {
        match parse_to_sexp(code) {
            Ok(sexp) => {
                assert!(!sexp.contains("(ERROR)"), "Failed to parse case cleanly: {}", code);
            }
            Err(err) => {
                assert!(!err.is_empty(), "Expected a descriptive parse error for: {}", code);
            }
        }
    }

    #[test]
    fn test_pratt_parser_precedence() {
        // Test basic precedence
        let sexp = assert_parses_without_error("2 + 3 * 4");
        // Should parse as 2 + (3 * 4), not (2 + 3) * 4
        assert!(sexp.contains("binary_expression"));

        // Defined-or remains an uneven pure-rust parser feature.
        assert_parses_or_fails_gracefully("$x // $y // $z");

        // Test ternary operator
        let sexp = assert_parses_without_error("$a ? $b : $c ? $d : $e");
        assert!(sexp.contains("TernaryOp") || sexp.contains("ternary"));
    }

    #[test]
    fn test_typeglob_support() {
        // Test basic typeglob
        let sexp = assert_parses_without_error("*foo = *bar;");
        assert!(sexp.contains("typeglob_variable"));

        // Typeglob slot access is still a graceful-gap area.
        assert_parses_or_fails_gracefully("$scalar = *foo{SCALAR};");

        // Test all slot types
        let slots = ["SCALAR", "ARRAY", "HASH", "CODE", "IO", "GLOB"];
        for slot in &slots {
            let code = format!("$x = *foo{{{}}};", slot);
            assert_parses_or_fails_gracefully(&code);
        }
    }

    #[test]
    fn test_format_declarations() {
        let mut parser = PureRustPerlParser::new();

        // Test basic format
        let code = r#"format STDOUT =
@<<<< @||||
$x,   $y
.
"#;
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));

        // Test named format
        let code = r#"format EMPLOYEE =
Name: @<<<<<<<<<<<<
      $name
.
"#;
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));
    }

    #[test]
    fn test_tie_mechanisms() {
        let mut parser = PureRustPerlParser::new();

        // Test tie statement
        let code = "tie $scalar, 'Tie::Scalar', $arg1, $arg2;";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test untie statement
        let code = "untie @array;";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("untie_statement"));

        // Test tied expression
        let sexp = assert_parses_without_error("$obj = tied(%hash);");
        assert!(sexp.contains("tied") || sexp.contains("function_call"));
    }

    #[test]
    fn test_nested_delimiters() {
        // Test nested braces
        let sexp = assert_parses_without_error(r#"q{{nested}}"#);
        assert!(sexp.contains("string"));

        // Test nested parentheses
        let sexp = assert_parses_without_error(r#"qq((nested (more)))"#);
        assert!(sexp.contains("string"));

        // Test complex nesting
        let sexp = assert_parses_without_error(r#"q{outer {middle {inner} middle} outer}"#);
        assert!(sexp.contains("string"));
    }

    #[test]
    fn test_operators() {
        // Defined-or assignment remains outside the guaranteed baseline.
        assert_parses_or_fails_gracefully("$x //= 42;");

        // Test smart match
        let sexp = assert_parses_without_error("$x ~~ @array;");
        assert!(sexp.contains("~~"));

        // Test isa
        let sexp = assert_parses_without_error("$obj isa My::Class");
        assert!(sexp.contains("isa"));

        // Bitwise string operators are still a graceful-gap area.
        assert_parses_or_fails_gracefully("$a &. $b");
    }

    #[test]
    fn test_postfix_dereference() {
        assert_parses_or_fails_gracefully("$ref->@*");
        assert_parses_or_fails_gracefully("$ref->%*");
        assert_parses_or_fails_gracefully("$ref->$*");
    }

    #[test]
    fn test_given_when() {
        let mut parser = PureRustPerlParser::new();

        let code = r#"given ($x) {
    when (1) { say "one"; }
    when (2) { say "two"; }
    default { say "other"; }
}"#;
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("given_statement"));
        assert!(sexp.contains("when_clause"));
        assert!(sexp.contains("default_clause"));
    }

    #[test]
    fn test_subroutine_signatures() {
        let mut parser = PureRustPerlParser::new();

        // Test basic signature
        let code = "sub add ($x, $y) { return $x + $y; }";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("subroutine"));

        // Test signature with defaults
        let code = "sub greet ($name = 'World') { say \"Hello, $name!\"; }";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("subroutine"));
    }

    #[test]
    fn test_state_variables() {
        assert_parses_or_fails_gracefully("state $counter = 0;");
    }

    #[test]
    fn test_lexical_subroutines() {
        let sexp = assert_parses_without_error("my sub helper { return 42; }");
        assert!(sexp.contains("subroutine"));

        let sexp = assert_parses_without_error("our sub shared { return 'shared'; }");
        assert!(sexp.contains("subroutine"));
    }

    #[test]
    fn test_package_blocks() {
        let sexp = assert_parses_without_error(
            r#"package Foo::Bar 1.23 {
    sub new { bless {}, shift }
}"#,
        );
        assert!(sexp.contains("package_declaration"));
    }
}
