//! Comprehensive test suite for heredoc features
//!
//! This test file ensures all heredoc improvements are tested and don't regress:
//! - Multi-line statement heredocs
//! - Statement boundary tracking  
//! - Builtin list operators (print, say, warn, die)

#[cfg(all(test, feature = "pure-rust"))]
mod comprehensive_heredoc_tests {
    use tree_sitter_perl::full_parser::FullPerlParser;

    #[test]
    fn test_simple_heredoc() {
        let input = r#"my $text = <<'EOF';
Hello, World!
EOF
print $text;"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse simple heredoc");
    }

    #[test]
    fn test_multi_line_statement_heredoc() {
        // The key fix - heredoc in a multi-line hash definition
        let input = r#"my %config = (
    name => "Test",
    description => <<'DESC'
);
This is a long description
that spans multiple lines
DESC
print $config{description};"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse heredoc in multi-line statement");
    }

    #[test]
    fn test_builtin_print_without_parens() {
        // Testing the new builtin list operator support
        let input = "print $x;";
        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse print without parentheses");
    }

    #[test]
    fn test_builtin_print_multiple_args() {
        let input = "print $a, $b, $c;";
        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse print with multiple arguments");
    }

    #[test]
    fn test_builtin_say() {
        let input = r#"say "Hello, world!";"#;
        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse say statement");
    }

    #[test]
    fn test_builtin_warn() {
        let input = r#"warn "Something went wrong";"#;
        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse warn statement");
    }

    #[test]
    fn test_builtin_die() {
        let input = r#"die "Fatal error";"#;
        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse die statement");
    }

    #[test]
    fn test_print_with_heredoc() {
        // Combining builtin list operator with heredoc
        let input = r#"print <<'EOF';
Hello world
EOF"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse print with heredoc");
    }

    #[test]
    fn test_mixed_heredocs_with_print() {
        // Complex test combining all features
        let input = r#"my $single = <<'SINGLE';
No interpolation here: $var
SINGLE
my $double = <<"DOUBLE";
Interpolation works: $var
DOUBLE
print $single, $double;"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse mixed heredocs with print");
    }

    #[test]
    fn test_nested_structure_heredoc() {
        // Testing statement boundary tracking with nested structures
        let input = r#"my $result = func(
    arg1,
    func2(
        <<'EOF'
    ),
    arg3
);
content
EOF"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse heredoc in nested function call");
    }

    #[test]
    fn test_multiple_heredocs_same_line() {
        let input = r#"print(<<A, <<B, <<C);
First
A
Second
B
Third
C"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse multiple heredocs on same line");
    }

    #[test]
    fn test_indented_heredoc() {
        let input = r#"my $text = <<~'END';
    This is indented
    content
    END
print $text;"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse indented heredoc");
    }

    #[test]
    fn test_backtick_heredoc() {
        let input = r#"my $output = <<`CMD`;
echo "Hello from shell"
CMD
print $output;"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse backtick heredoc");
    }

    #[test]
    fn test_heredoc_in_array_ref() {
        let input = r#"my $ref = [
    "first",
    <<'HEREDOC',
    "third"
];
second element
HEREDOC
print $ref->[1];"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse heredoc in array reference");
    }

    #[test]
    fn test_print_filehandle_heredoc() {
        // Print to filehandle with heredoc
        let input = r#"print STDERR <<'ERROR';
An error occurred
ERROR"#;

        let mut parser = FullPerlParser::new();
        assert!(parser.parse(input).is_ok(), "Failed to parse print to filehandle with heredoc");
    }
}
