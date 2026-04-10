use perl_quote::{
    SubstitutionError, extract_substitution_parts, extract_substitution_parts_strict,
    validate_substitution_modifiers,
};
use proptest::prelude::*;

fn modifier_candidate_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 0..64)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

fn valid_modifier_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just('g'),
            Just('i'),
            Just('m'),
            Just('s'),
            Just('x'),
            Just('o'),
            Just('e'),
            Just('r'),
            Just('a'),
            Just('d'),
            Just('l'),
            Just('u'),
            Just('n'),
            Just('p'),
            Just('c'),
        ],
        0..16,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn invalid_alpha_modifier_strategy() -> impl Strategy<Value = char> {
    prop::char::range('a', 'z').prop_filter("must not be valid substitution modifier", |c| {
        !matches!(
            c,
            'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c'
        )
    })
}

fn modifier_oracle(input: &str) -> Result<String, char> {
    let mut out = String::new();

    for c in input.chars() {
        if !c.is_ascii_alphabetic() {
            if c.is_whitespace() || c == ';' || c == '\n' || c == '\r' {
                break;
            }
            return Err(c);
        }

        if matches!(
            c,
            'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c'
        ) {
            out.push(c);
        } else {
            return Err(c);
        }
    }

    Ok(out)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_validate_substitution_modifiers_matches_oracle(modifiers in modifier_candidate_strategy()) {
        prop_assert_eq!(validate_substitution_modifiers(&modifiers), modifier_oracle(&modifiers));
    }

    #[test]
    fn prop_strict_and_lenient_agree_for_valid_modifier_sequences(
        pattern in "[A-Za-z0-9_]{0,24}",
        replacement in "[A-Za-z0-9_]{0,24}",
        modifiers in valid_modifier_strategy(),
    ) {
        let text = format!("s/{pattern}/{replacement}/{modifiers}");

        let lenient = extract_substitution_parts(&text);
        let strict = extract_substitution_parts_strict(&text);

        prop_assert_eq!(strict.ok(), Some(lenient));
    }

    #[test]
    fn prop_strict_rejects_invalid_alphabetic_modifier(
        pattern in "[A-Za-z0-9_]{0,16}",
        replacement in "[A-Za-z0-9_]{0,16}",
        valid_prefix in valid_modifier_strategy(),
        invalid in invalid_alpha_modifier_strategy(),
    ) {
        let text = format!("s/{pattern}/{replacement}/{valid_prefix}{invalid}");

        let strict = extract_substitution_parts_strict(&text);

        prop_assert_eq!(strict, Err(SubstitutionError::InvalidModifier(invalid)));
    }
}
