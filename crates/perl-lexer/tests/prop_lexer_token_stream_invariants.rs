use perl_lexer::PerlLexer;
use proptest::prelude::*;

fn perlish_input() -> impl Strategy<Value = String> {
    let perl_fragment = prop_oneof![
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_map(|id| format!("my ${id} = 1;\n")),
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_map(|id| format!("if (${id}) {{ print ${id}; }}\n")),
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_map(|id| format!("${id} =~ m/[a-z]+/;\n")),
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_map(|id| format!("s/{id}/x/g;\n")),
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_map(|id| format!("tr/{id}/xyz/;\n")),
        "[A-Z]{1,8}".prop_map(|label| format!("print <<{label};\n{label}\n")),
        "[[:ascii:]]{0,48}".prop_map(|s| format!("# {s}\n")),
    ];

    prop::collection::vec(perl_fragment, 1..24).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 300,
        ..ProptestConfig::default()
    })]

    #[test]
    fn token_stream_spans_are_monotonic_and_in_bounds(input in perlish_input()) {
        let mut lexer = PerlLexer::new(&input);
        let mut previous_end = 0usize;

        for _ in 0..(input.len().max(1) * 4 + 64) {
            match lexer.next_token() {
                Some(token) => {
                    prop_assert!(token.start <= token.end, "invalid span ordering: {}..{}", token.start, token.end);
                    prop_assert!(token.end <= input.len(), "token end {} exceeds input length {}", token.end, input.len());
                    prop_assert!(token.start >= previous_end, "token starts before previous token ended: {} < {}", token.start, previous_end);
                    previous_end = token.end;
                }
                None => {
                    return Ok(());
                }
            }
        }

        prop_assert!(false, "tokenization did not terminate within configured bound");
    }

    #[test]
    fn tokenization_is_deterministic_for_same_input(input in perlish_input()) {
        let mut lexer1 = PerlLexer::new(&input);
        let tokens1 = lexer1.collect_tokens();

        let mut lexer2 = PerlLexer::new(&input);
        let tokens2 = lexer2.collect_tokens();

        prop_assert_eq!(tokens1.len(), tokens2.len(), "token count mismatch");

        for (left, right) in tokens1.iter().zip(tokens2.iter()) {
            prop_assert_eq!(left.start, right.start, "start mismatch");
            prop_assert_eq!(left.end, right.end, "end mismatch");
            prop_assert_eq!(&left.text, &right.text, "text mismatch");
            prop_assert_eq!(&left.token_type, &right.token_type, "token kind mismatch");
        }
    }
}
