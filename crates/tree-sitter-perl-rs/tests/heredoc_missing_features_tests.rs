#[cfg(all(test, feature = "pure-rust"))]
mod heredoc_missing_features_tests {
    use tree_sitter_perl::full_parser::FullPerlParser;

    #[test]
    fn test_backtick_heredoc() {
        let input = r#"my $output = <<`CMD`;
echo "Hello from shell"
date
CMD
print $output;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse backtick heredoc");
    }

    #[test]
    fn test_escaped_delimiter_heredoc() {
        let input = r#"my $literal = <<\EOF;
This has $no interpolation
Even though it looks like $variables
EOF
print $literal;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse escaped delimiter heredoc");
    }

    #[test]
    #[ignore] // Likely to cause stack overflow
    fn test_heredoc_in_array_context() {
        let input = r#"my @messages = (<<MSG1, <<MSG2, "regular string");
First message
MSG1
Second message
MSG2
print @messages;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc in array context");
    }

    #[test]
    fn test_heredoc_as_hash_value() {
        // This is valid Perl but our parser can't handle heredocs
        // in multi-line statements because we don't track statement boundaries
        let input = r#"my %config = (
    name => "Test",
    description => <<'DESC'
);
This is a long description
that spans multiple lines
DESC
print $config{description};"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc as hash value");
    }
    
    #[test]
    fn test_heredoc_in_simple_hash() {
        // Simpler case that should work
        let input = r#"my %config = (name => "Test", description => <<'DESC');
This is a long description
that spans multiple lines
DESC
print $config{description};"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc in simple hash");
    }

    #[test]
    fn test_whitespace_around_heredoc_operator() {
        let input = r#"my $spaced = << 'EOF';
Content with spaces around operator
EOF
print $spaced;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with whitespace around operator");
    }

    #[test]
    fn test_mixed_quote_heredocs() {
        let input = r#"my $single = <<'SINGLE';
No interpolation here: $var
SINGLE
my $double = <<"DOUBLE";
Interpolation works: $var
DOUBLE
my $backtick = <<`BACKTICK`;
echo "Command execution"
BACKTICK
print($single, $double, $backtick);"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse mixed quote heredocs");
    }

    #[test]
    fn test_keyword_as_terminator() {
        let input = r#"my $text = <<'if';
This uses a keyword as terminator
if
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with keyword terminator");
    }

    #[test]
    fn test_numeric_terminator() {
        let input = r#"my $data = <<'123';
Content with numeric terminator
123
print $data;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with numeric terminator");
    }

    #[test]
    fn test_heredoc_in_return_statement() {
        let input = r#"sub get_message {
    return <<'MSG';
This is the return value
MSG
}
print get_message();"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc in return statement");
    }

    #[test]
    fn test_indented_heredoc_mixed_whitespace() {
        let input = r#"my $mixed = <<~'END';
    	Mixed tabs and spaces
    	should work fine
    END
print $mixed;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse indented heredoc with mixed whitespace");
    }

    #[test]
    fn test_heredoc_with_regex_chars_in_terminator() {
        let input = r#"my $text = <<'.*?';
Content with regex chars in terminator
.*?
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with regex chars in terminator");
    }

    #[test]
    fn test_very_long_terminator() {
        let input = r#"my $text = <<'THIS_IS_A_VERY_LONG_TERMINATOR_STRING_THAT_SHOULD_STILL_WORK';
Content with very long terminator
THIS_IS_A_VERY_LONG_TERMINATOR_STRING_THAT_SHOULD_STILL_WORK
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with very long terminator");
    }

    #[test]
    fn test_heredoc_content_with_heredoc_syntax() {
        let input = r#"my $nested = <<'OUTER';
This content contains <<'FAKE'
what looks like a heredoc
but it's just content
OUTER
print $nested;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc containing heredoc-like syntax");
    }

    #[test]
    #[ignore] // Expected to fail - missing terminator
    fn test_missing_terminator() {
        let input = r#"my $incomplete = <<'EOF';
This heredoc is never closed
print "This should fail";"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_err(), "Should fail with missing heredoc terminator");
    }

    #[test]
    fn test_empty_heredoc() {
        let input = r#"my $empty = <<'EMPTY';
EMPTY
print length($empty);"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse empty heredoc");
    }

    #[test]
    fn test_heredoc_with_blank_lines_at_end() {
        let input = r#"my $text = <<'EOF';
Content
More content


EOF
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with trailing blank lines");
    }
}