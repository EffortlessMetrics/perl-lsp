//! Integration tests for Unicode handling and heredoc parsing fixes

use tree_sitter_perl::PureRustPerlParser;

fn parse_to_sexp(input: &str) -> Result<String, String> {
    let mut parser = PureRustPerlParser::new();
    match parser.parse(input) {
        Ok(ast) => Ok(parser.to_sexp(&ast)),
        Err(err) => Err(format!("Parse failed: {err:?}\nInput: {input}")),
    }
}

fn assert_parses_without_error(input: &str) -> String {
    match parse_to_sexp(input) {
        Ok(sexp) => {
            assert!(!sexp.contains("ERROR"), "Unexpected error node for input: {input}");
            sexp
        }
        Err(err) => panic!("{err}"),
    }
}

#[test]
fn test_unicode_parsing() {
    let mut parser = PureRustPerlParser::new();

    // Test basic Unicode
    let input = r#"my $emoji = "✅";"#;
    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse Unicode emoji");

    // Test mixed Unicode
    let input2 = r#"my $text = "Hello 世界 🌍";"#;
    let result2 = parser.parse(input2);
    assert!(result2.is_ok(), "Failed to parse mixed Unicode");

    // Test Unicode in comments
    let input3 = r#"# Comment with emoji 🎯
my $x = 42;"#;
    let result3 = parser.parse(input3);
    assert!(result3.is_ok(), "Failed to parse Unicode in comments");
}

#[test]
fn test_basic_heredoc() {
    let input = r#"my $text = <<'EOF';
This is a heredoc
With multiple lines
EOF"#;

    let sexp = assert_parses_without_error(input);
    assert!(sexp.contains("(variable_declaration $text"));
}

#[test]
fn test_interpolated_heredoc() {
    let mut parser = PureRustPerlParser::new();

    let input = r#"my $name = "World";
my $text = <<EOF;
Hello, $name!
EOF"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse interpolated heredoc");
}

#[test]
fn test_indented_heredoc() {
    let input = r#"my $text = <<~'EOF';
    This is indented
    content with spaces
    EOF"#;

    let sexp = assert_parses_without_error(input);
    assert!(sexp.contains("(variable_declaration $text"));
}

#[test]
fn test_multiple_heredocs() {
    let input = r#"print <<'FIRST', <<'SECOND';
First content
FIRST
Second content  
SECOND"#;

    let sexp = assert_parses_without_error(input);
    assert!(sexp.contains("source_file"));
}

#[test]
fn test_heredoc_with_unicode() {
    let input = r#"my $text = <<'EOF';
Unicode heredoc ✅
With emojis 🎉
EOF"#;

    let sexp = assert_parses_without_error(input);
    assert!(sexp.contains("(variable_declaration $text"));
}

#[test]
fn test_complex_perl_with_all_features() {
    let mut parser = PureRustPerlParser::new();

    // Test each component separately first
    let simple_heredoc = r#"my $greeting = <<~EOF;
    Hello World!
    EOF"#;
    assert!(parser.parse(simple_heredoc).is_ok(), "Failed to parse simple indented heredoc");

    // Test regex
    let regex_test = r#"if ("test" =~ /test/) { print "ok"; }"#;
    assert!(parser.parse(regex_test).is_ok(), "Failed to parse regex");

    // Test qw
    let qw_test = r#"my @items = qw(apple banana cherry);"#;
    assert!(parser.parse(qw_test).is_ok(), "Failed to parse qw");

    // Test subroutine
    let sub_test = r#"sub test_function {
    my ($param) = @_;
    return $param * 2;
}"#;
    assert!(parser.parse(sub_test).is_ok(), "Failed to parse subroutine");

    // Now test a simpler combined version
    let combined = r#"#!/usr/bin/env perl
use strict;

my $greeting = <<EOF;
Hello World!
EOF

print $greeting;"#;

    let result = parser.parse(combined);
    assert!(result.is_ok(), "Failed to parse combined Perl code");
}

#[test]
fn test_slash_disambiguation_in_heredoc() {
    let input = r#"my $text = <<'EOF';
Path: /usr/local/bin
Division: 10 / 2
Regex: s/foo/bar/
EOF
my $x = 10 / 2;"#;

    let sexp = assert_parses_without_error(input);
    assert!(sexp.contains("(variable_declaration $text"), "Expected variable declaration");
    assert!(sexp.contains("source_file"), "Expected parsed source file");
}
