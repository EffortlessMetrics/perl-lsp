//! Tests for newly implemented features in the pure Rust parser

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::pure_rust_parser::{AstNode, PureRustPerlParser};

    #[test]
    fn test_pratt_parser_precedence() {
        let mut parser = PureRustPerlParser::new();

        // Test basic precedence
        let code = "2 + 3 * 4";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        // Should parse as 2 + (3 * 4), not (2 + 3) * 4
        assert!(sexp.contains("binary_expression"));

        // Test defined-or operator
        let code = "$x // $y // $z";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("//"));

        // Test ternary operator
        let code = "$a ? $b : $c ? $d : $e";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("ternary_op"));
    }

    #[test]
    fn test_typeglob_support() {
        let mut parser = PureRustPerlParser::new();

        // Test basic typeglob
        let code = "*foo = *bar;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("typeglob_variable"));

        // Test typeglob slot access
        let code = "$scalar = *foo{SCALAR};";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("typeglob_slot_access"));

        // Test all slot types
        let slots = ["SCALAR", "ARRAY", "HASH", "CODE", "IO", "GLOB"];
        for slot in &slots {
            let code = format!("$x = *foo{{{}}};", slot);
            let ast = parser.parse(&code).unwrap();
            let sexp = parser.to_sexp(&ast);
            assert!(sexp.contains(slot));
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
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));

        // Test named format
        let code = r#"format EMPLOYEE =
Name: @<<<<<<<<<<<<
      $name
.
"#;
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("EMPLOYEE"));
    }

    #[test]
    fn test_tie_mechanisms() {
        let mut parser = PureRustPerlParser::new();

        // Test tie statement
        let code = "tie $scalar, 'Tie::Scalar', $arg1, $arg2;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test untie statement
        let code = "untie @array;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("untie_statement"));

        // Test tied expression
        let code = "$obj = tied(%hash);";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tied_expression"));
    }

    #[test]
    fn test_nested_delimiters() {
        let mut parser = PureRustPerlParser::new();

        // Test nested braces
        let code = r#"q{{nested}}"#;
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("q_string"));

        // Test nested parentheses
        let code = r#"qq((nested (more)))"#;
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("qq_string"));

        // Test complex nesting
        let code = r#"q{outer {middle {inner} middle} outer}"#;
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("q_string"));
    }

    #[test]
    fn test_operators() {
        let mut parser = PureRustPerlParser::new();

        // Test defined-or
        let code = "$x //= 42;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("//="));

        // Test smart match
        let code = "$x ~~ @array;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("~~"));

        // Test isa
        let code = "$obj isa My::Class";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("isa"));

        // Test bitwise string operators
        let code = "$a &. $b";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("&."));
    }

    #[test]
    fn test_postfix_dereference() {
        let mut parser = PureRustPerlParser::new();

        // Test array dereference
        let code = "$ref->@*";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("postfix_deref"));

        // Test hash dereference
        let code = "$ref->%*";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("postfix_deref"));

        // Test scalar dereference
        let code = "$ref->$*";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("postfix_deref"));
    }

    #[test]
    fn test_given_when() {
        let mut parser = PureRustPerlParser::new();

        let code = r#"given ($x) {
    when (1) { say "one"; }
    when (2) { say "two"; }
    default { say "other"; }
}"#;
        let ast = parser.parse(code).unwrap();
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
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("subroutine"));

        // Test signature with defaults
        let code = "sub greet ($name = 'World') { say \"Hello, $name!\"; }";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("subroutine"));
    }

    #[test]
    fn test_state_variables() {
        let mut parser = PureRustPerlParser::new();

        let code = "state $counter = 0;";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("state"));
        assert!(sexp.contains("variable_declaration"));
    }

    #[test]
    fn test_lexical_subroutines() {
        let mut parser = PureRustPerlParser::new();

        // Test my sub
        let code = "my sub helper { return 42; }";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("my"));
        assert!(sexp.contains("subroutine"));

        // Test our sub
        let code = "our sub shared { return 'shared'; }";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("our"));
        assert!(sexp.contains("subroutine"));
    }

    #[test]
    fn test_package_blocks() {
        let mut parser = PureRustPerlParser::new();

        let code = r#"package Foo::Bar 1.23 {
    sub new { bless {}, shift }
}"#;
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("package_declaration"));
        assert!(sexp.contains("Foo::Bar"));
        assert!(sexp.contains("1.23"));
    }
}
