//! Real-world heredoc edge cases from CPAN and production code

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_heredoc_in_interpolated_string() {
    // From CPAN: Template::Toolkit
    let input = r#"
my $template = "$prefix<<$end_tag
Template content here
$end_tag";
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should recover heredoc inside interpolated string");
}

#[test]
fn test_nested_heredoc_delimiters() {
    // From production code: nested variable references
    let input = r#"
my $outer = 'EOF';
my $inner = $outer;
my $doc = <<${${inner}};
Nested content
EOF
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();
    
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }
    
    // Should recover to EOF based on static analysis
    assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::HeredocStart)));
}

#[test]
fn test_heredoc_with_method_chain() {
    // From CPAN: Mojo::Template
    let input = r#"
my $content = <<$self->delimiter->value;
Dynamic content based on object state
END
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut error_count = 0;
    let mut heredoc_count = 0;
    
    while let Some(token) = lexer.next_token() {
        match &token.token_type {
            TokenType::Error(_) => error_count += 1,
            TokenType::HeredocStart => heredoc_count += 1,
            _ => {}
        }
    }
    
    assert!(heredoc_count > 0 || error_count > 0, "Should either recover or error gracefully");
}

#[test]
fn test_heredoc_with_ternary_operator() {
    // From PerlMonks discussion
    let input = r#"
my $doc = <<($debug ? 'DEBUG' : 'PROD');
Configuration data
DEBUG
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should recover heredoc with ternary expression");
}

#[test]
fn test_heredoc_with_array_element() {
    // From production logging code
    let input = r#"
my @markers = ('START', 'END', 'DATA');
my $log = <<$markers[1];
Log entry
END
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should recover heredoc with array element");
}

#[test]
fn test_heredoc_with_hash_lookup() {
    // From CPAN: Config::General
    let input = r#"
my %delims = (sql => 'SQL', xml => 'XML');
my $data = <<$delims{sql};
SELECT * FROM table;
SQL
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should recover heredoc with hash lookup");
}

#[test]
fn test_heredoc_with_special_vars() {
    // Edge case with special variables
    let input = r#"
my $doc1 = <<$_;
Default variable content
EOF

my $doc2 = <<$@;
Error variable content
ERROR
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut heredoc_count = 0;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            heredoc_count += 1;
        }
    }
    
    assert_eq!(heredoc_count, 2, "Should handle special variable heredocs");
}

#[test]
fn test_heredoc_with_concatenation() {
    // From template generation code
    let input = r#"
my $prefix = 'BEGIN_';
my $suffix = '_END';
my $doc = <<$prefix . $suffix;
Content
BEGIN_END
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut has_heredoc_or_error = false;
    
    while let Some(token) = lexer.next_token() {
        match token.token_type {
            TokenType::HeredocStart | TokenType::Error(_) => {
                has_heredoc_or_error = true;
            }
            _ => {}
        }
    }
    
    assert!(has_heredoc_or_error, "Should handle or error on concatenated delimiter");
}

#[test]
fn test_heredoc_with_function_call() {
    // From CPAN module internals
    let input = r#"
sub get_marker { 'MARKER' }
my $content = <<get_marker();
Function-determined content
MARKER
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should recover heredoc with function call");
}

#[test]
fn test_multiple_dynamic_heredocs() {
    // Complex case with multiple dynamic heredocs
    let input = r#"
my $d1 = 'EOF1';
my $d2 = 'EOF2';
my $doc = <<$d1 . "\n" . <<$d2;
First document
EOF1
Second document
EOF2
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut heredoc_count = 0;
    let mut error_count = 0;
    
    while let Some(token) = lexer.next_token() {
        match token.token_type {
            TokenType::HeredocStart => heredoc_count += 1,
            TokenType::Error(_) => error_count += 1,
            _ => {}
        }
    }
    
    // Should handle at least one heredoc or error gracefully
    assert!(heredoc_count > 0 || error_count > 0, "Should handle multiple dynamic heredocs");
}

#[test]
fn test_heredoc_with_regex_result() {
    // Edge case with regex capture
    let input = r#"
my $text = "START:DATA:END";
$text =~ /:(.+):/;
my $doc = <<$1;
Content based on regex capture
DATA
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_token = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart | TokenType::Error(_)) {
            found_token = true;
        }
    }
    
    assert!(found_token, "Should handle heredoc with regex capture variable");
}

#[test]
fn test_heredoc_in_eval() {
    // Dynamic code generation pattern
    let input = r#"
my $delimiter = 'CODE';
eval "my \$generated = <<$delimiter;
Generated content
$delimiter
";
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut token_count = 0;
    
    while let Some(_token) = lexer.next_token() {
        token_count += 1;
    }
    
    // Should at least tokenize without panic
    assert!(token_count > 0, "Should tokenize eval with heredoc");
}

#[test]
fn test_heredoc_with_package_variable() {
    // From namespace-aware code
    let input = r#"
package My::Config;
our $END_MARKER = 'END_CONFIG';

package main;
my $config = <<$My::Config::END_MARKER;
Configuration data
END_CONFIG
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut found_heredoc = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            found_heredoc = true;
        }
    }
    
    assert!(found_heredoc, "Should handle package-qualified variable heredocs");
}

#[test]
fn test_heredoc_with_tied_variable() {
    // Edge case with tied variables
    let input = r#"
tie my $magic, 'TiedScalar';
my $doc = <<$magic;
Content with tied variable delimiter
TIED_END
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut has_token = false;
    
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart | TokenType::Error(_)) {
            has_token = true;
        }
    }
    
    assert!(has_token, "Should handle tied variable heredocs gracefully");
}

#[test]
fn test_heredoc_error_recovery_message() {
    // Verify error messages are helpful
    let input = r#"
my $doc = <<$complex->expression->{that}->cannot_resolve;
Content
END
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut error_msg = None;
    
    while let Some(token) = lexer.next_token() {
        if let TokenType::Error(msg) = &token.token_type {
            error_msg = Some(msg.clone());
        }
    }
    
    if let Some(msg) = error_msg {
        assert!(msg.contains("heredoc") || msg.contains("delimiter"), 
                "Error message should be descriptive: {}", msg);
    }
}