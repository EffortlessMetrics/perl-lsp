//! Integration tests for the Pure Rust Perl Parser

use perl_tdd_support::must;
use tree_sitter_perl::PureRustPerlParser;

#[test]
fn test_basic_parsing() {
    let mut parser = PureRustPerlParser::new();
    let input = r#"my $x = 42; print "Hello, World!";"#;

    match parser.parse(input) {
        Ok(ast) => {
            // Just ensure it parses without error
            assert!(!format!("{:?}", ast).is_empty());
        }
        Err(e) => must(Err::<(), _>(format!("Failed to parse: {:?}", e))),
    }
}

#[test]
fn test_heredoc_parsing() {
    let mut parser = PureRustPerlParser::new();
    let input = r#"
my $text = <<EOF;
This is a heredoc
with multiple lines
EOF
print $text;
"#;

    match parser.parse(input) {
        Ok(ast) => {
            let debug_str = format!("{:?}", ast);
            assert!(debug_str.contains("This is a heredoc"));
        }
        Err(e) => must(Err::<(), _>(format!("Failed to parse heredoc: {:?}", e))),
    }
}

#[test]
fn test_control_flow() {
    let mut parser = PureRustPerlParser::new();
    let input = r#"
if ($x > 10) {
    print "big";
} else {
    print "small";
}

for my $i (1..10) {
    print $i;
}
"#;

    match parser.parse(input) {
        Ok(_) => {
            // Success - it parsed
        }
        Err(e) => must(Err::<(), _>(format!("Failed to parse control flow: {:?}", e))),
    }
}

#[test]
fn test_subroutines() {
    let mut parser = PureRustPerlParser::new();
    let input = r#"
sub hello {
    my ($name) = @_;
    return "Hello, $name!";
}

my $greeting = hello("World");
print $greeting;
"#;

    match parser.parse(input) {
        Ok(_) => {
            // Success - it parsed
        }
        Err(e) => must(Err::<(), _>(format!("Failed to parse subroutine: {:?}", e))),
    }
}

#[test]
fn test_regex_and_substitution() {
    let mut parser = PureRustPerlParser::new();
    let input = r#"
$text =~ s/old/new/g;
if ($text =~ /pattern/) {
    print "matched";
}
"#;

    match parser.parse(input) {
        Ok(_) => {
            // Success - it parsed
        }
        Err(e) => must(Err::<(), _>(format!("Failed to parse regex: {:?}", e))),
    }
}
