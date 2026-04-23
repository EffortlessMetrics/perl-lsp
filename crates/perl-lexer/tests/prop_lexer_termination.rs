use perl_lexer::PerlLexer;
use proptest::prelude::*;

fn matching_delimiter(open: char) -> char {
    match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        other => other,
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn lexer_terminates_without_panics(s in ".{0,300}") {
        // This test ensures:
        // 1. The lexer never panics (no underflows, no slice bounds errors)
        // 2. The lexer always terminates (no infinite loops)

        let mut lx = PerlLexer::new(&s);

        // Give generous upper bound for tokens (avg 3 chars per token is very conservative)
        let max_expected_tokens = s.len().max(1) * 2 + 100;

        for _ in 0..max_expected_tokens {
            match lx.next_token() {
                Some(_) => {},
                None => {
                    // Reached EOF successfully
                    return Ok(());
                }
            }
        }

        // If we consumed max_expected_tokens without hitting EOF,
        // the lexer is likely in an infinite loop
        prop_assert!(
            false,
            "Lexer failed to terminate after {} tokens on input of len={}",
            max_expected_tokens,
            s.len()
        );
    }

    #[test]
    fn lexer_handles_edge_patterns_without_panic(
        prefix in "[a-zA-Z0-9]{0,5}",
        sigil in prop::sample::select(vec!['$', '@', '%', '*', '&']),
        suffix in "[{}()\\[\\]]{0,5}"
    ) {
        // Test patterns that previously caused issues
        let patterns = vec![
            format!("{}{{{}", sigil, suffix),           // Sigil with brace
            format!("{}<<EOF", prefix),                 // Heredoc start
            format!("}}{{{}", suffix),                  // Unbalanced braces
            format!("{}s{{}}{{}}", prefix),             // Empty substitution
        ];

        for pattern in patterns {
            let mut lx = PerlLexer::new(&pattern);
            let mut count = 0;

            // Consume all tokens, ensuring no panic
            while lx.next_token().is_some() && count < 1000 {
                count += 1;
            }

            prop_assert!(count < 1000, "Possible infinite loop in pattern: {}", pattern);
        }
    }

    #[test]
    fn lexer_quote_like_constructs_terminate_without_panics(
        operator in prop::sample::select(vec!["q", "qq", "qw", "qx", "qr", "m", "s", "tr", "y"]),
        delimiter in prop::sample::select(vec!['/', '!', '#', '~', '(', '{', '[', '<']),
        lhs in ".{0,32}",
        rhs in ".{0,32}",
        modifiers in "[a-z]{0,6}",
    ) {
        let close = matching_delimiter(delimiter);

        let script = match operator {
            "s" | "tr" | "y" => {
                format!("{operator}{delimiter}{lhs}{close}{delimiter}{rhs}{close}{modifiers}")
            }
            _ => format!("{operator}{delimiter}{lhs}{close}{modifiers}"),
        };

        let mut lexer = PerlLexer::new(&script);
        let max_expected_tokens = script.len().max(1) * 2 + 100;

        for _ in 0..max_expected_tokens {
            if lexer.next_token().is_none() {
                return Ok(());
            }
        }

        prop_assert!(
            false,
            "Lexer failed to terminate for quote-like input after {} tokens: {}",
            max_expected_tokens,
            script
        );
    }
}
