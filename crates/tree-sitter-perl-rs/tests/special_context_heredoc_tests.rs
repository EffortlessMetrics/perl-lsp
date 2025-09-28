//! Tests for heredocs in special contexts (eval and s///e)

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::context_aware_parser::{
        ContextAwareFullParser, ContextAwareHeredocParser,
    };

    #[test]
    fn test_basic_eval_heredoc() {
        let code = r#"
# Basic eval with heredoc
eval <<'EOF';
print "Hello from eval\n";
my $var = 42;
EOF

print "After eval\n";
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (_processed, declarations) = parser.parse();

        assert_eq!(declarations.len(), 1, "Should find one heredoc");
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(declarations[0].content.is_some());
    }

    #[test]
    fn test_eval_heredoc_with_interpolation() {
        let code = r#"
my $name = "World";
eval <<"CODE";
print "Hello, $name!\n";
my $result = <<'DATA';
Some data here
DATA
print $result;
CODE
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (_processed, declarations) = parser.parse();

        // Should find the outer heredoc
        assert!(declarations.iter().any(|d| d.terminator == "CODE"));

        // The inner heredoc should be detected when eval content is parsed
        let eval_content = declarations
            .iter()
            .find(|d| d.terminator == "CODE")
            .and_then(|d| d.content.as_ref())
            .unwrap();

        assert!(eval_content.contains("<<'DATA'"));
    }

    #[test]
    fn test_substitution_with_e_flag() {
        let code = r#"
# Substitution with /e flag and heredoc
my $text = "foo bar baz";
$text =~ s/foo/<<'REPLACEMENT'/e;
This replaces foo
with multiple lines
REPLACEMENT

print $text;
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (_processed, declarations) = parser.parse();

        // Should detect heredoc in s///e
        assert!(declarations
            .iter()
            .any(|d| d.terminator == "REPLACEMENT" || d.terminator == "EVAL_CONTEXT"));
    }

    #[test]
    fn test_complex_substitution_patterns() {
        let test_cases = vec![
            // Different delimiters
            r#"s/pattern/<<EOF/e
content
EOF"#,
            r#"s|pattern|<<EOF|e
content
EOF"#,
            r#"s#pattern#<<EOF#e
content
EOF"#,
            // With other flags
            r#"s/pattern/<<EOF/egi
content
EOF"#,
            // Multiple on same line
            r#"s/foo/<<'A'/e; s/bar/<<'B'/e;
foo content
A
bar content
B"#,
        ];

        for code in test_cases {
            let parser = ContextAwareHeredocParser::new(code);
            let (_, declarations) = parser.parse();
            assert!(!declarations.is_empty(), "Should find heredoc in: {}", code);
        }
    }

    #[test]
    fn test_nested_eval_contexts() {
        let code = r#"
# Nested eval contexts
eval <<'OUTER';
    my $inner = eval <<'INNER';
        print "Deeply nested\n";
        return <<'DATA';
        Some data
DATA
INNER
    print "Got: $inner\n";
OUTER
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (_processed, declarations) = parser.parse();

        // Should find at least the outer heredoc
        assert!(declarations.iter().any(|d| d.terminator == "OUTER"));
    }

    #[test]
    fn test_heredoc_in_qx_eval() {
        let code = r#"
# Backticks with eval-like behavior
my $result = `perl -e '<<EOF
print "Hello from subshell"
EOF'`;
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (processed, _declarations) = parser.parse();

        // This is a complex case - heredoc is in a subshell
        // Parser should at least not crash
        assert!(processed.contains("EOF"));
    }

    #[test]
    fn test_heredoc_in_regex_match() {
        let code = r#"
# Heredoc in regex match context (not replacement)
if ($text =~ /<<EOF/) {
    print "Found heredoc marker\n";
}
EOF
"#;

        let parser = ContextAwareHeredocParser::new(code);
        let (_processed, declarations) = parser.parse();

        // Should NOT treat this as a heredoc (it's in a regex pattern)
        assert!(declarations.is_empty() || !declarations.iter().any(|d| d.terminator == "EOF"));
    }

    #[test]
    fn test_full_parser_integration() {
        let code = r#"
eval <<'CODE';
my $x = 42;
print "Value: $x\n";
CODE

my $str = "test";
$str =~ s/test/<<END/e;
replaced
END

print $str;
"#;

        let mut parser = ContextAwareFullParser::new();
        let result = parser.parse(code);

        assert!(result.is_ok(), "Full parser should handle special contexts");

        // Verify AST contains the eval and substitution nodes
        // In a real implementation, we'd check specific AST structure
    }

    #[test]
    fn test_error_cases() {
        let error_cases = vec![
            // Unclosed eval heredoc
            r#"eval <<'EOF';
never closed"#,
            // Invalid s///e syntax
            r#"s/pattern/<<EOF/e
missing terminator"#,
            // Nested but malformed
            r#"eval <<'OUTER';
eval <<'INNER';
OUTER
# Missing INNER terminator"#,
        ];

        for code in error_cases {
            let parser = ContextAwareHeredocParser::new(code);
            let (_processed, declarations) = parser.parse();

            // Parser should handle errors gracefully
            // May have incomplete declarations but shouldn't panic
            println!("Processed error case: {} declarations found", declarations.len());
        }
    }

    #[test]
    fn test_edge_case_combinations() {
        // Test heredoc after eval on same line
        let code1 = r#"eval <<'A'; print <<'B';
eval content
A
print content
B"#;

        let parser = ContextAwareHeredocParser::new(code1);
        let (_, declarations) = parser.parse();
        assert_eq!(declarations.len(), 2, "Should find both heredocs");

        // Test heredoc in eval in substitution
        let code2 = r#"$x =~ s/foo/eval <<'E'/e;
bar
E"#;

        let parser2 = ContextAwareHeredocParser::new(code2);
        let (_, declarations2) = parser2.parse();
        assert!(!declarations2.is_empty(), "Should find heredoc in complex context");
    }
}
