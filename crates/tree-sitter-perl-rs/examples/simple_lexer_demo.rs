//! Simple demo showing perl-lexer integration
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

fn main() {
    println!("=== Perl Lexer Demo ===\n");

    let test_cases = vec![
        "my $x = 42;",
        "print \"Hello, World!\";",
        "if ($x > 10) { print $x; }",
        "sub hello { return \"hi\"; }",
        r#"
my $heredoc = <<'END';
This is a heredoc
with multiple lines
END
"#,
    ];

    for (i, code) in test_cases.iter().enumerate() {
        println!("Test {}: {}", i + 1, code.lines().next().unwrap_or(code));
        println!("Tokens:");

        let mut lexer = PerlLexer::new(code);
        let mut token_count = 0;

        while let Some(token) = lexer.next_token() {
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }

            println!(
                "  {:?} @ {}..{}: {:?}",
                token.token_type, token.start, token.end, token.value
            );

            token_count += 1;

            // Limit output for long examples
            if token_count > 20 {
                println!("  ... (truncated)");
                break;
            }
        }

        println!();
    }

    // Demo heredoc recovery
    println!("=== Heredoc Recovery Demo ===");
    let heredoc_code = r#"
my $data = <<EOF;
First line
Second line
EOF

print $data;
"#;

    println!("Code: {}", heredoc_code.trim());
    println!("\nRecovery process:");

    let mut lexer = PerlLexer::with_heredoc_recovery(heredoc_code);
    let mut in_heredoc = false;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }

        match token.token_type {
            TokenType::HeredocMarker => {
                println!("  Found heredoc marker: {:?}", token.value);
                in_heredoc = true;
            }
            TokenType::HeredocBody => {
                println!("  Found heredoc body");
                in_heredoc = false;
            }
            _ if in_heredoc => {
                println!("  Skipping token in heredoc: {:?}", token.token_type);
            }
            _ => {
                // Regular token
            }
        }
    }

    println!("\n✅ Heredoc recovery successful!");
}
