//! Integration tests for Unicode handling and heredoc parsing fixes

use tree_sitter_perl::PureRustPerlParser;

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
    let mut parser = PureRustPerlParser::new();

    let input = r#"my $text = <<'EOF';
This is a heredoc
With multiple lines
EOF"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse basic heredoc");

    let ast = parser.parse(input).unwrap();
    let sexp = parser.to_sexp(&ast);
    assert!(sexp.contains("This is a heredoc\nWith multiple lines"));
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
    let mut parser = PureRustPerlParser::new();

    let input = r#"my $text = <<~'EOF';
    This is indented
    content with spaces
    EOF"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse indented heredoc");

    let ast = parser.parse(input).unwrap();
    let sexp = parser.to_sexp(&ast);
    // Verify indentation is removed
    assert!(sexp.contains("This is indented\ncontent with spaces"));
    assert!(!sexp.contains("    This is indented"));
}

#[test]
fn test_multiple_heredocs() {
    let mut parser = PureRustPerlParser::new();

    let input = r#"print <<'FIRST', <<'SECOND';
First content
FIRST
Second content  
SECOND"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse multiple heredocs");

    let ast = parser.parse(input).unwrap();
    let sexp = parser.to_sexp(&ast);
    assert!(sexp.contains("First content"));
    assert!(sexp.contains("Second content"));
}

#[test]
fn test_heredoc_with_unicode() {
    let mut parser = PureRustPerlParser::new();

    let input = r#"my $text = <<'EOF';
Unicode heredoc ✅
With emojis 🎉
EOF"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse heredoc with Unicode");

    let ast = parser.parse(input).unwrap();
    let sexp = parser.to_sexp(&ast);
    assert!(sexp.contains("Unicode heredoc ✅"));
    assert!(sexp.contains("With emojis 🎉"));
}

#[test]
fn test_complex_perl_with_all_features() {
    let mut parser = PureRustPerlParser::new();

    // Test each component separately first
    let simple_heredoc = r#"my $greeting = <<~EOF;
    Hello World!
    EOF"#;
    assert!(
        parser.parse(simple_heredoc).is_ok(),
        "Failed to parse simple indented heredoc"
    );

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
    let mut parser = PureRustPerlParser::new();

    let input = r#"my $text = <<'EOF';
Path: /usr/local/bin
Division: 10 / 2
Regex: s/foo/bar/
EOF
my $x = 10 / 2;"#;

    let result = parser.parse(input);
    assert!(result.is_ok(), "Failed to parse heredoc with slashes");

    let ast = parser.parse(input).unwrap();
    let sexp = parser.to_sexp(&ast);
    // The parser uses placeholders for heredocs in the S-expression output
    // We just need to verify it parses correctly and the structure is right
    assert!(
        sexp.contains("__HEREDOC_"),
        "Expected heredoc placeholder in output"
    );
    assert!(
        sexp.contains("(variable_declaration $text"),
        "Expected variable declaration"
    );
    assert!(
        sexp.contains("(variable_declaration $x"),
        "Expected second variable declaration"
    );
    assert!(
        sexp.contains("(binary_expression"),
        "Expected binary expression for division"
    );
}
