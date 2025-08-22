//! Comprehensive test suite for the Rust Perl parser

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::enhanced_parser::EnhancedPerlParser;
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

    #[test]
    fn test_basic_statements() {
        let cases = vec![
            "my $x = 42;",
            "print \"Hello, World!\";",
            "my @array = (1, 2, 3);",
            "my %hash = (a => 1, b => 2);",
            "sub foo { return 42; }",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse: {}", case);
        }
    }

    #[test]
    fn test_string_edge_cases() {
        let cases = vec![
            r#"my $x = "{$";"#,
            r#"my $y = "$";"#,
            r#"my $z = "test $x";"#,
            r#"print "$y";"#,
            r#"my $empty = "";"#,
            r#"my $single = '';"#,
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse string edge case: {}", case);
        }
    }

    #[test]
    fn test_heredocs() {
        let cases = vec![
            // Basic heredoc
            "my $x = <<EOF;\nHello\nWorld\nEOF\n",
            // Quoted heredoc
            "my $x = <<'END';\nNo $interpolation\nEND\n",
            // Indented heredoc
            "my $x = <<~INDENT;\n    This is indented\n    INDENT\n",
        ];

        let parser = EnhancedPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse heredoc: {}", case);
        }
    }

    #[test]
    fn test_quote_operators() {
        let cases = vec![
            "my $x = q/single quotes/;",
            "my $x = qq/double quotes/;",
            "my $x = qw/word list/;",
            "my $x = qr/regex/;",
            "my $x = qx/command/;",
            "my $x = q{balanced braces};",
            "my $x = qq[balanced brackets];",
            "my $x = q<balanced angles>;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse quote operator: {}", case);
        }
    }

    #[test]
    fn test_regex_operators() {
        let cases = vec![
            "if ($x =~ /pattern/) { }",
            "if ($x !~ /pattern/) { }",
            "$x =~ s/old/new/g;",
            "$x =~ tr/a-z/A-Z/;",
            "$x =~ m{pattern}i;",
            "$x =~ s{old}{new}gx;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse regex operator: {}", case);
        }
    }

    #[test]
    fn test_control_structures() {
        let cases = vec![
            "if ($x) { print; }",
            "unless ($x) { print; }",
            "while ($x) { print; }",
            "until ($x) { print; }",
            "for (my $i = 0; $i < 10; $i++) { print; }",
            "foreach my $item (@list) { print; }",
            "do { print; } while ($x);",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse control structure: {}", case);
        }
    }

    #[test]
    fn test_special_variables() {
        let cases = vec![
            "print $_;",
            "print @_;",
            "print %ENV;",
            "print $!;",
            "print $@;",
            "print $$;",
            "print $?;",
            "print $0;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse special variable: {}", case);
        }
    }

    #[test]
    fn test_identifiers_with_underscores() {
        let cases = vec![
            "my $q_ = 1;",
            "my $qq_ = 2;",
            "my $qr_ = 3;",
            "my $qw_ = 4;",
            "my $qx_ = 5;",
            "my $m_ = 6;",
            "my $s_ = 7;",
            "my $tr_ = 8;",
            "my $y_ = 9;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse identifier: {}", case);
        }
    }

    #[test]
    fn test_pod() {
        let cases = vec![
            "=head1 NAME\n\nTest\n\n=cut\n",
            "print 1;\n=pod\n\nDocumentation\n\n=cut\nprint 2;",
            "=head2 DESCRIPTION\n\nThis is a test\n\n=cut\n",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse POD: {}", case);
        }
    }

    #[test]
    fn test_complex_expressions() {
        let cases = vec![
            "my $x = 1 + 2 * 3 - 4 / 5;",
            "my $y = $a && $b || $c;",
            "my $z = $x ? $y : $z;",
            "my $w = $x // $y // $z;",
            "my $v = $x . $y . $z;",
            "my $u = $x .. $y;",
            "my $t = $x ... $y;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse complex expression: {}", case);
        }
    }

    #[test]
    fn test_list_operations() {
        let cases = vec![
            "my @sorted = sort @list;",
            "my @filtered = grep { $_ > 0 } @list;",
            "my @mapped = map { $_ * 2 } @list;",
            "my @sliced = @array[0..5];",
            "my @reversed = reverse @list;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse list operation: {}", case);
        }
    }

    #[test]
    fn test_references() {
        let cases = vec![
            "my $ref = \\$scalar;",
            "my $ref = \\@array;",
            "my $ref = \\%hash;",
            "my $ref = \\&sub;",
            "my $value = $$ref;",
            "my $value = $ref->[0];",
            "my $value = $ref->{key};",
            "my $value = $ref->();",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse reference: {}", case);
        }
    }

    #[test]
    fn test_real_world_snippets() {
        let cases = vec![
            // Common Perl idioms
            "open my $fh, '<', $filename or die $!;",
            "while (<$fh>) { chomp; print; }",
            "my $content = do { local $/; <$fh> };",
            "use strict; use warnings;",
            "package My::Module; use base 'Parent';",
            // Error handling
            "eval { dangerous_operation() }; warn $@ if $@;",
            "local $SIG{__DIE__} = sub { print STDERR @_ };",
            // One-liners
            "@ARGV = grep { -f } @ARGV;",
            "print join('\\n', sort keys %hash);",
            "my %seen; @unique = grep { !$seen{$_}++ } @list;",
        ];

        let mut parser = PureRustPerlParser::new();
        for case in cases {
            let result = parser.parse(case);
            assert!(result.is_ok(), "Failed to parse real-world snippet: {}", case);
        }
    }
}
