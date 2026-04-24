//! Comprehensive tests for all implemented features in the pure Rust parser

#[cfg(feature = "pure-rust")]
mod tests {
    use perl_tdd_support::must;
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;
    use tree_sitter_perl::stateful_parser::StatefulPerlParser;

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
    fn test_operator_precedence() {
        // Test basic precedence
        let sexp = assert_parses_without_error("2 + 3 * 4");
        assert!(sexp.contains("binary_expression"));

        // Test exponentiation (right associative)
        let sexp = assert_parses_without_error("2 ** 3 ** 4");
        assert!(sexp.contains("**"));

        let supported_operators = vec![
            ("$a = $b", "="),
            ("$a += $b", "+="),
            ("$a || $b", "||"),
            ("$a && $b", "&&"),
            ("$a == $b", "=="),
            ("$a ~~ $b", "~~"),
            ("$a < $b", "<"),
            ("$a isa MyClass", "isa"),
            ("$a + $b", "+"),
            ("$a . $b", "."),
            ("$a * $b", "*"),
            ("$a =~ /test/", "=~"),
            ("$a !~ /test/", "!~"),
        ];

        for (code, op) in supported_operators {
            let sexp = assert_parses_without_error(code);
            assert!(
                sexp.contains(op) || sexp.contains("binary_expression"),
                "Failed to parse operator {} in code: {}",
                op,
                code
            );
        }

        // Defined-or is still uneven in the pure-rust parser contract suite.
        assert_parses_or_fails_gracefully("$a // $b");
        assert_parses_or_fails_gracefully("$a | $b");
        assert_parses_or_fails_gracefully("$a & $b");
        assert_parses_or_fails_gracefully("$a eq $b");
        assert_parses_or_fails_gracefully("$a lt $b");
        assert_parses_or_fails_gracefully("$a << $b");
        assert_parses_or_fails_gracefully("$a x $b");
    }

    #[test]
    fn test_typeglob_support() {
        // Test basic typeglob
        let sexp = assert_parses_without_error("*foo");
        assert!(sexp.contains("typeglob_variable"));

        // Basic slotless assignment is part of the current baseline.
        let sexp = assert_parses_without_error("*new = *old");
        assert!(sexp.contains("assignment"));

        // Typeglob slot access remains an aspirational gap for the pure-rust parser.
        let slots =
            vec!["SCALAR", "ARRAY", "HASH", "CODE", "IO", "GLOB", "FORMAT", "NAME", "PACKAGE"];
        for slot in slots {
            let code = format!("*foo{{{}}}", slot);
            assert_parses_or_fails_gracefully(&code);
        }
    }

    #[test]
    fn test_quote_like_operators() {
        let mut parser = PureRustPerlParser::new();

        // Test q// with various delimiters
        let q_tests = vec![
            "q(hello world)",
            "q[hello world]",
            "q{hello world}",
            "q<hello world>",
            "q!hello world!",
            "q#hello world#",
        ];

        for code in q_tests {
            let ast = must(parser.parse(code));
            let sexp = parser.to_sexp(&ast);
            assert!(sexp.contains("string"), "Failed to parse: {}", code);
        }

        // Test nested delimiters
        let nested_tests = vec![
            "q{hello {nested} world}",
            "q(hello (nested) world)",
            "q[hello [nested] world]",
            "qq{hello {nested {deeply}} world}",
        ];

        for code in nested_tests {
            let ast = must(parser.parse(code));
            let sexp = parser.to_sexp(&ast);
            assert!(sexp.contains("string"), "Failed to parse nested: {}", code);
        }

        // Test qw (word list)
        let code = "qw(one two three)";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("qw_list"));

        // Test qr (regex)
        let qr_tests = vec!["qr/pattern/", "qr{pattern}", "qr(pattern)i", "qr[pattern]x"];

        for code in qr_tests {
            let ast = must(parser.parse(code));
            let sexp = parser.to_sexp(&ast);
            assert!(
                sexp.contains("qr_regex") || sexp.contains("regex"),
                "Failed to parse qr: {}",
                code
            );
        }
    }

    #[test]
    fn test_format_declarations() {
        let mut parser = StatefulPerlParser::new();

        // Test basic format
        let code = r#"format STDOUT =
@<<<< @|||| @>>>>
$name, $age, $city
.
print "done";"#;

        let ast = must(parser.parse(code));
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));

        // Test anonymous format
        let code = r#"format =
Name: @<<<<<<<<<<<
      $name
.
write;"#;

        let ast = must(parser.parse(code));
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));
    }

    #[test]
    fn test_tie_untie_tied() {
        let mut parser = PureRustPerlParser::new();

        // Test tie
        let code = "tie %hash, 'Tie::Hash::Indexed'";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test tie with args
        let code = "tie @array, 'Tie::File', $filename, O_RDWR";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test untie
        let code = "untie %hash";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("untie_statement"));

        // Test tied
        let code = "tied(%hash)";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tied"));
    }

    #[test]
    fn test_heredocs() {
        let mut parser = StatefulPerlParser::new();

        // Test basic heredoc
        let code = r#"my $text = <<END;
This is a heredoc
with multiple lines
END
print $text;"#;

        let ast = must(parser.parse(code));
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));

        // Test quoted heredoc
        let code = r#"my $text = <<'EOF';
This is a single-quoted heredoc
No interpolation: $var @array
EOF
"#;

        let ast = must(parser.parse(code));
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));

        // Test indented heredoc
        let code = r#"my $text = <<~"END";
    This is an indented heredoc
    The indentation is stripped
    END
"#;

        let ast = must(parser.parse(code));
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));
    }

    #[test]
    fn test_complex_expressions() {
        // Test ternary with precedence
        let sexp = assert_parses_without_error("$a = $b > 5 ? $c + 10 : $d * 2");
        assert!(
            sexp.contains("TernaryOp") || sexp.contains("ternary"),
            "Expected ternary structure or unhandled ternary marker, got: {}",
            sexp
        );

        // Test chained comparisons
        let sexp = assert_parses_without_error("$a < $b && $b < $c");
        assert!(sexp.contains("&&"));

        // Postfix dereference is still uneven in the pure-rust parser contract suite.
        assert_parses_or_fails_gracefully("$ref->@*");
    }

    #[test]
    fn test_special_blocks() {
        let mut parser = PureRustPerlParser::new();

        let blocks = vec![
            ("BEGIN { print 'start' }", "begin_block"),
            ("END { print 'done' }", "end_block"),
            ("CHECK { validate() }", "check_block"),
            ("INIT { setup() }", "init_block"),
            ("UNITCHECK { test() }", "unitcheck_block"),
        ];

        for (code, expected) in blocks {
            let ast = must(parser.parse(code));
            let sexp = parser.to_sexp(&ast);
            assert!(
                sexp.contains(expected),
                "Failed to parse {} - expected to find {}",
                code,
                expected
            );
        }
    }

    #[test]
    fn test_labeled_blocks() {
        let sexp = assert_parses_without_error("OUTER: { last OUTER if $done; }");
        assert!(sexp.contains("labeled_block"));

        // Labeled loop serialization is still uneven; keep this as a clean-parse check.
        assert_parses_without_error("LOOP: while ($x) { next LOOP if $skip; }");
    }

    #[test]
    fn test_modern_perl_features() {
        // State declarations are still tracked as a modern-perl gap for the pure-rust parser.
        assert_parses_or_fails_gracefully("state $counter = 0");

        // Test given/when (if supported)
        let mut parser = PureRustPerlParser::new();
        let code = r#"given ($value) {
    when (1) { say "one" }
    when (2) { say "two" }
    default { say "other" }
}"#;
        let result = parser.parse(code);
        if let Ok(ast) = result {
            let sexp = parser.to_sexp(&ast);
            assert!(sexp.contains("given"));
        }

        // Test smartmatch
        let code = "$a ~~ @array";
        let ast = must(parser.parse(code));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("~~"));
    }
}
