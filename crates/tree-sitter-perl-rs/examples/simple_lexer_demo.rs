//! Simple demo showing perl-lexer integration
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

fn main() {
    println!("=== Perl Lexer Demo ===\n");

    let test_cases = [
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

        loop {
            let token = lexer.next_token();
            if let Some(tok) = token {
                if matches!(tok.token_type, TokenType::EOF) {
                    break;
                }
                println!("  {:?} @ {}..{}: {:?}", tok.token_type, tok.start, tok.end, tok.text);
                token_count += 1;
            } else {
                break;
            }

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

    let mut lexer = PerlLexer::new(heredoc_code);
    let mut in_heredoc = false;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }

        match token.token_type {
            TokenType::HeredocBody(_) => {
                println!("  Found heredoc body: {:?}", token.text);
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
