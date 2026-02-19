//! Comprehensive tests for all implemented features in the pure Rust parser

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;
    use tree_sitter_perl::stateful_parser::StatefulPerlParser;

    #[test]
    fn test_operator_precedence() {
        let mut parser = PureRustPerlParser::new();

        // Test basic precedence
        let code = "2 + 3 * 4";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("binary_expression"));

        // Test exponentiation (right associative)
        let code = "2 ** 3 ** 4";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("**"));

        // Test all operator types
        let operators = vec![
            ("$a = $b", "="),
            ("$a += $b", "+="),
            ("$a || $b", "||"),
            ("$a // $b", "//"),
            ("$a && $b", "&&"),
            ("$a | $b", "|"),
            ("$a & $b", "&"),
            ("$a == $b", "=="),
            ("$a eq $b", "eq"),
            ("$a ~~ $b", "~~"),
            ("$a < $b", "<"),
            ("$a lt $b", "lt"),
            ("$a isa MyClass", "isa"),
            ("$a << $b", "<<"),
            ("$a + $b", "+"),
            ("$a . $b", "."),
            ("$a * $b", "*"),
            ("$a x $b", "x"),
            ("$a =~ /test/", "=~"),
            ("$a !~ /test/", "!~"),
        ];

        for (code, op) in operators {
            let ast = parser.parse(code).unwrap();
            let sexp = parser.to_sexp(&ast);
            assert!(
                sexp.contains(op) || sexp.contains("binary_expression"),
                "Failed to parse operator {} in code: {}",
                op,
                code
            );
        }
    }

    #[test]
    fn test_typeglob_support() {
        let mut parser = PureRustPerlParser::new();

        // Test basic typeglob
        let code = "*foo";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("typeglob_variable"));

        // Test typeglob slot access
        let slots =
            vec!["SCALAR", "ARRAY", "HASH", "CODE", "IO", "GLOB", "FORMAT", "NAME", "PACKAGE"];
        for slot in slots {
            let code = format!("*foo{{{}}}", slot);
            let ast = parser.parse(&code).unwrap();
            let sexp = parser.to_sexp(&ast);
            assert!(
                sexp.contains("typeglob_slot_access"),
                "Failed to parse typeglob slot access for {}",
                slot
            );
        }

        // Test typeglob assignment
        let code = "*new = *old";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("assignment"));
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
            let ast = parser.parse(code).unwrap();
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
            let ast = parser.parse(code).unwrap();
            let sexp = parser.to_sexp(&ast);
            assert!(sexp.contains("string"), "Failed to parse nested: {}", code);
        }

        // Test qw (word list)
        let code = "qw(one two three)";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("qw_list"));

        // Test qr (regex)
        let qr_tests = vec!["qr/pattern/", "qr{pattern}", "qr(pattern)i", "qr[pattern]x"];

        for code in qr_tests {
            let ast = parser.parse(code).unwrap();
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

        let ast = parser.parse(code).unwrap();
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));

        // Test anonymous format
        let code = r#"format =
Name: @<<<<<<<<<<<
      $name
.
write;"#;

        let ast = parser.parse(code).unwrap();
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("format_declaration"));
    }

    #[test]
    fn test_tie_untie_tied() {
        let mut parser = PureRustPerlParser::new();

        // Test tie
        let code = "tie %hash, 'Tie::Hash::Indexed'";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test tie with args
        let code = "tie @array, 'Tie::File', $filename, O_RDWR";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("tie_statement"));

        // Test untie
        let code = "untie %hash";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("untie_statement"));

        // Test tied
        let code = "tied(%hash)";
        let ast = parser.parse(code).unwrap();
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

        let ast = parser.parse(code).unwrap();
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));

        // Test quoted heredoc
        let code = r#"my $text = <<'EOF';
This is a single-quoted heredoc
No interpolation: $var @array
EOF
"#;

        let ast = parser.parse(code).unwrap();
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));

        // Test indented heredoc
        let code = r#"my $text = <<~"END";
    This is an indented heredoc
    The indentation is stripped
    END
"#;

        let ast = parser.parse(code).unwrap();
        let sexp = PureRustPerlParser::node_to_sexp(&ast);
        assert!(sexp.contains("heredoc"));
    }

    #[test]
    fn test_complex_expressions() {
        let mut parser = PureRustPerlParser::new();

        // Test ternary with precedence
        let code = "$a = $b > 5 ? $c + 10 : $d * 2";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("ternary"));

        // Test chained comparisons
        let code = "$a < $b && $b < $c";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("&&"));

        // Test postfix dereference
        let code = "$ref->@*";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("postfix_dereference"));
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
            let ast = parser.parse(code).unwrap();
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
        let mut parser = PureRustPerlParser::new();

        let code = "OUTER: { last OUTER if $done; }";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("labeled_block"));

        // Test with loops
        let code = "LOOP: while ($x) { next LOOP if $skip; }";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("while"));
    }

    #[test]
    fn test_modern_perl_features() {
        let mut parser = PureRustPerlParser::new();

        // Test state variables
        let code = "state $counter = 0";
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("state"));

        // Test given/when (if supported)
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
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("~~"));
    }
}
