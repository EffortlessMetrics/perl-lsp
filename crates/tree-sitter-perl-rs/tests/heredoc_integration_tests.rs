#[cfg(all(test, feature = "pure-rust"))]
mod heredoc_integration_tests {
    use tree_sitter_perl::full_parser::FullPerlParser;
    use tree_sitter_perl::heredoc_parser::parse_with_heredocs;

    #[test]
    fn test_basic_heredoc() {
        let input = r#"my $text = <<'EOF';
Hello, World!
This is a heredoc.
EOF
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        if let Err(ref e) = result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok(), "Failed to parse basic heredoc");

        let ast = result.unwrap();
        eprintln!("Parse succeeded, AST type: {:?}", std::mem::discriminant(&ast));

        eprintln!("Calling parse_to_sexp...");
        let sexp_result = parser.parse_to_sexp(input);
        if let Err(ref e) = sexp_result {
            eprintln!("S-expression generation failed: {:?}", e);
        }
        let sexp = sexp_result.unwrap();
        eprintln!("S-expression generated:\n{}", sexp);
        assert!(sexp.contains("Hello, World"));
    }

    #[test]
    fn test_interpolated_heredoc() {
        let input = r#"my $name = "World";
my $greeting = <<EOF;
Hello, $name!
Welcome to Perl.
EOF
print $greeting;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse interpolated heredoc");
    }

    #[test]
    fn test_multiple_heredocs() {
        let input = r#"print(<<A, <<B, <<C);
First content
A
Second content
B
Third content
C"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        if let Err(ref e) = result {
            eprintln!("Parse error: {:?}", e);

            // Debug the preprocessing stages
            let (processed, declarations) = parse_with_heredocs(input);
            eprintln!("\nHeredoc processed:\n{}", processed);
            eprintln!("\nDeclarations: {} found", declarations.len());
            for (i, decl) in declarations.iter().enumerate() {
                eprintln!("  [{}] {}: {:?}", i, decl.terminator, decl.content.as_deref());
            }
        }
        if let Err(ref e) = result {
            eprintln!("Parse failed: {:?}", e);
        }
        assert!(result.is_ok(), "Failed to parse multiple heredocs");
        eprintln!("Parse succeeded!");

        // Don't try to generate S-expression yet, just verify parsing worked
    }

    #[test]
    fn test_indented_heredoc() {
        let input = r#"if ($condition) {
    my $config = <<~'CONFIG';
        server: localhost
        port: 8080
        debug: true
        CONFIG
    print $config;
}"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse indented heredoc");
    }

    #[test]
    fn test_heredoc_in_expression() {
        let input = r#"my $result = process(<<'DATA') + calculate(42);
Input data for
processing function
DATA
print $result;"#;

        let mut parser = FullPerlParser::new();
        match parser.parse(input) {
            Ok(_) => (),
            Err(e) => must(Err::<(), _>(format!("Failed to parse heredoc in expression: {:?}", e))),
        }
    }

    #[test]
    fn test_heredoc_with_special_chars() {
        let input = r#"my $regex = <<'REGEX';
/\w+\s*=\s*"[^"]*"/
REGEX
print $regex;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with special characters");
    }

    #[test]
    fn test_heredoc_with_empty_lines() {
        let input = r#"my $text = <<'EOF';
Line 1

Line 3 (with empty line above)
EOF
print $text;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with empty lines");
    }

    #[test]
    fn test_heredoc_terminator_in_content() {
        let input = r#"my $tricky = <<'END';
This line contains END but doesn't terminate
The real END is below
END
print $tricky;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with terminator in content");
    }

    #[test]
    fn test_heredoc_preprocessing() {
        let input = r#"my $x = <<'EOF';
Hello / World
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);

        println!("Original:\n{}", input);
        println!("\nProcessed:\n{}", processed);
        println!("\nDeclarations: {:?}", declarations);

        // Check that heredoc was detected
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(!declarations[0].interpolated);
        assert_eq!(declarations[0].content.as_deref(), Some("Hello / World"));

        // Check that heredoc was replaced with placeholder
        assert!(processed.contains("__HEREDOC_1__"));

        // The processed output should NOT contain the heredoc content
        assert!(!processed.contains("Hello / World"));

        // Check the overall structure
        assert!(processed.starts_with("my $x = __HEREDOC_1__"));
    }

    #[test]
    fn test_heredoc_with_slash_disambiguation() {
        let input = r#"my $text = <<'EOF';
This contains a / slash
And a regex: /pattern/
EOF
my $result = $text =~ s/slash/SLASH/g;
print $result / 2;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse heredoc with slash disambiguation");
    }

    #[test]
    fn test_complex_heredoc_scenario() {
        let input = r#"#!/usr/bin/perl
use strict;
use warnings;

my $name = "Alice";
my $age = 30;

my $template = <<~'TEMPLATE';
    Name: $name
    Age: $age
    Status: Active
    TEMPLATE

my $sql = <<SQL;
SELECT * FROM users
WHERE name = '$name'
  AND age > $age
  AND status = 'active'
SQL

print $template;
print $sql;

my $result = process(<<'DATA', <<'CONFIG');
Raw data goes here
with multiple lines
DATA
key: value
another: value
CONFIG

print $result;"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse complex heredoc scenario");
    }

    #[test]
    fn test_heredoc_error_recovery() {
        // Test various edge cases and potential error conditions
        let test_cases = vec![
            // Empty heredoc
            r#"my $x = <<'';

"#,
            // Heredoc with numeric terminator
            r#"my $x = <<'123';
content
123"#,
            // Heredoc with special terminator
            r#"my $x = <<'!!!';
content
!!!"#,
        ];

        for (i, input) in test_cases.iter().enumerate() {
            let mut parser = FullPerlParser::new();
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed test case {}: {}", i, input);
        }
    }
}
