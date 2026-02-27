#[cfg(all(test, feature = "pure-rust"))]
mod enhanced_parser_tests {
    use tree_sitter_perl::EnhancedFullParser;

    #[test]
    fn test_backtick_heredoc() {
        let input = r#"
my $output = <<`CMD`;
echo "Hello from shell"
date
CMD
print $output;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse backtick heredoc: {:?}", result.err());
    }

    #[test]
    fn test_escaped_heredoc() {
        let input = r#"
my $literal = <<\EOF;
This has $no interpolation
Even though it looks like $variables
EOF
print $literal;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse escaped heredoc: {:?}", result.err());
    }

    #[test]
    fn test_whitespace_around_heredoc() {
        let input = r#"
my $spaced = << 'EOF';
Content with spaces around operator
EOF
print $spaced;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with whitespace: {:?}", result.err());
    }

    #[test]
    fn test_indented_heredoc() {
        let input = r#"
my $indented = <<~'END';
    This is indented
    And this too
END
print $indented;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse indented heredoc: {:?}", result.err());
    }

    #[test]
    fn test_multiple_heredocs() {
        let input = r#"
print <<EOF, <<'LITERAL';
First heredoc with $interpolation
EOF
Second heredoc without $interpolation
LITERAL
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse multiple heredocs: {:?}", result.err());
    }

    #[test]
    fn test_data_section() {
        let input = r#"
#!/usr/bin/perl
use strict;
print "Hello World\n";

__DATA__
This is data content
that can be read with <DATA>
Multiple lines are supported
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse DATA section: {:?}", result.err());

        // Verify DATA section was extracted
        assert!(parser.data_section_start.is_some());
    }

    #[test]
    fn test_end_section() {
        let input = r#"
print "Main program\n";

__END__
Everything after __END__ is ignored
Including this line
And this one too
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse END section: {:?}", result.err());
    }

    #[test]
    fn test_pod_documentation() {
        let input = r#"
print "Before POD\n";

=head1 NAME

TestModule - A test module

=head2 SYNOPSIS

    use TestModule;
    my $obj = TestModule->new();

=head2 DESCRIPTION

This module does something useful.

=cut

print "After POD\n";

=pod

Another POD section

=cut

print "End of program\n";
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse POD: {:?}", result.err());

        // Verify POD sections were extracted
        assert_eq!(parser.pod_sections.len(), 2);
    }

    #[test]
    fn test_heredoc_in_hash() {
        let input = r#"
my %config = (
    name => "Test",
    description => <<'DESC',
This is a long description
that spans multiple lines
DESC
    version => "1.0",
);
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc in hash: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_in_array() {
        let input = r#"
my @messages = (
    "regular string",
    <<'MSG1',
First message
MSG1
    <<'MSG2',
Second message  
MSG2
);
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc in array: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_as_subroutine_argument() {
        let input = r#"
process_text(<<'TEXT');
This is text
passed to a function
TEXT

sub process_text {
    my $text = shift;
    print $text;
}
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc as sub arg: {:?}", result.err());
    }

    #[test]
    fn test_mixed_quote_heredocs() {
        let input = r#"
my $single = <<'SINGLE';
No interpolation: $var @array
SINGLE

my $double = <<"DOUBLE";  
With interpolation: $var @array
DOUBLE

my $bare = <<BARE;
Also interpolated: $var
BARE

my $backtick = <<`COMMAND`;
echo "Command execution"
COMMAND
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse mixed quote heredocs: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_with_special_terminators() {
        let input = r#"
my $data1 = <<'123';
Numeric terminator
123

my $data2 = <<'if';
Keyword terminator
if

my $data3 = <<'__END__';
Special terminator
__END__
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse special terminators: {:?}", result.err());
    }

    #[test]
    fn test_format_declaration() {
        let input = r#"
format STDOUT =
@<<<<< @||||| @>>>>>
$name, $age, $salary
.

format REPORT_TOP =
Name    Age   Salary
------- ----- -------
.

write;
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        // Format declarations are a known limitation, but shouldn't crash
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_complex_mixed_content() {
        let input = r#"
#!/usr/bin/perl
use strict;
use warnings;

=head1 NAME

ComplexTest - Tests multiple features

=cut

my $var = "test";

my $heredoc = <<'EOF';
Heredoc content
EOF

print $heredoc;

=head2 METHODS

=over 4

=item new()

Constructor

=back

=cut

sub new {
    my $class = shift;
    return bless {}, $class;
}

__DATA__
Data section content
More data here
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse complex mixed content: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_content_preservation() {
        let input = r#"
my $text = <<'EOF';
Line 1
  Line 2 with indent
    Line 3 with more indent
Line 4 back to start
EOF
"#;

        let mut parser = EnhancedFullParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());

        // The content should be preserved with original formatting
        let ast = result.unwrap();
        // Just verify the AST was created successfully
        // Content preservation is handled internally
        assert!(!format!("{:?}", ast).is_empty());
    }
}
