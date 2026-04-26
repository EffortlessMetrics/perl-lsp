use perl_lexer::{Checkpointable, PerlLexer, Token, TokenType};
use proptest::prelude::*;

fn token_signature(token: &Token) -> (String, usize, usize, String) {
    (format!("{:?}", token.token_type), token.start, token.end, token.text.to_string())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 192,
        ..ProptestConfig::default()
    })]

    #[test]
    fn lexer_emits_monotonic_valid_spans(input in ".{0,256}") {
        let mut lexer = PerlLexer::new(&input);
        let mut previous_end = 0usize;

        let max_tokens = input.len().max(1) * 3 + 32;
        for _ in 0..max_tokens {
            let Some(token) = lexer.next_token() else {
                return Ok(());
            };

            prop_assert!(token.start <= token.end);
            prop_assert!(token.end <= input.len());
            prop_assert!(token.start >= previous_end);
            prop_assert!(input.is_char_boundary(token.start));
            prop_assert!(input.is_char_boundary(token.end));

            match token.token_type {
                TokenType::EOF => {
                    prop_assert_eq!(token.start, input.len());
                    prop_assert_eq!(token.end, input.len());
                    return Ok(());
                }
                _ => {
                    prop_assert_eq!(&input[token.start..token.end], token.text.as_ref());
                    previous_end = token.end;
                }
            }
        }

        prop_assert!(false, "lexer exceeded token budget without EOF");
    }

    #[test]
    fn restoring_checkpoint_preserves_forward_progress(
        input in ".{0,200}",
        split_tokens in 0usize..40,
    ) {
        let mut lexer = PerlLexer::new(&input);

        for _ in 0..split_tokens {
            let Some(token) = lexer.next_token() else {
                return Ok(());
            };

            if matches!(token.token_type, TokenType::EOF) {
                return Ok(());
            }
        }

        let checkpoint = lexer.checkpoint();
        let checkpoint_pos = checkpoint.position;

        let max_tokens = input.len().max(1) * 3 + 32;
        for _ in 0..max_tokens {
            let Some(token) = lexer.next_token() else {
                break;
            };
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }
        }

        lexer.restore(&checkpoint);

        for _ in 0..max_tokens {
            let Some(token) = lexer.next_token() else {
                return Ok(());
            };
            prop_assert!(token.start >= checkpoint_pos);
            prop_assert!(token.end <= input.len());
            prop_assert!(token.start <= token.end);
            if matches!(token.token_type, TokenType::EOF) {
                return Ok(());
            }
        }

        prop_assert!(false, "lexer exceeded token budget after restore");
    }
    #[test]
    fn collect_tokens_matches_manual_iteration(input in ".{0,200}") {
        let mut manual_lexer = PerlLexer::new(&input);
        let mut manual = Vec::new();

        let max_tokens = input.len().max(1) * 3 + 32;
        for _ in 0..max_tokens {
            let Some(token) = manual_lexer.next_token() else {
                break;
            };
            let is_eof = matches!(token.token_type, TokenType::EOF);
            manual.push(token_signature(&token));
            if is_eof {
                break;
            }
        }

        let collected = PerlLexer::new(&input)
            .collect_tokens()
            .into_iter()
            .map(|token| token_signature(&token))
            .collect::<Vec<_>>();

        prop_assert_eq!(collected, manual);
    }
}
