use perl_quote::{
    extract_regex_parts, extract_substitution_parts, extract_substitution_parts_strict,
    extract_transliteration_parts,
};
use proptest::prelude::*;

fn simple_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just('a'),
            Just('b'),
            Just('c'),
            Just('x'),
            Just('y'),
            Just('z'),
            Just('0'),
            Just('1'),
            Just('_'),
            Just(' '),
            Just('-'),
            Just(':'),
            Just('é'),
            Just('Ω'),
        ],
        0..12,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn non_empty_body() -> impl Strategy<Value = String> {
    simple_body().prop_filter("body must not be empty", |body| !body.is_empty())
}

fn valid_modifiers() -> impl Strategy<Value = String> {
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
            Just('c')
        ],
        0..6,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn transliteration_modifiers() -> impl Strategy<Value = String> {
    prop::collection::vec(prop_oneof![Just('c'), Just('d'), Just('s'), Just('r')], 0..4)
        .prop_map(|chars| chars.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn fuzz_valid_unpaired_substitution_roundtrip(
        delimiter in prop_oneof![Just('/'), Just('!'), Just('#'), Just('~'), Just('@'), Just('%')],
        pattern in simple_body(),
        replacement in simple_body(),
        modifiers in valid_modifiers(),
    ) {
        let input = format!("s{delimiter}{pattern}{delimiter}{replacement}{delimiter}{modifiers}");

        let lenient = extract_substitution_parts(&input);
        let strict = extract_substitution_parts_strict(&input);

        prop_assert_eq!(lenient.0, pattern.clone());
        prop_assert_eq!(lenient.1, replacement.clone());
        prop_assert_eq!(lenient.2, modifiers.clone());
        prop_assert_eq!(strict.ok(), Some((pattern, replacement, modifiers)));
    }

    #[test]
    fn fuzz_valid_paired_substitution_roundtrip(
        delimiters in prop_oneof![
            Just(('{', '}')),
            Just(('(', ')')),
            Just(('[', ']')),
            Just(('<', '>')),
        ],
        replacement_delimiters in prop_oneof![
            Just(('{', '}')),
            Just(('(', ')')),
            Just(('[', ']')),
            Just(('<', '>')),
        ],
        pattern in simple_body(),
        replacement in simple_body(),
        modifiers in valid_modifiers(),
    ) {
        let (open_pat, close_pat) = delimiters;
        let (open_repl, close_repl) = replacement_delimiters;
        let input = format!("s{open_pat}{pattern}{close_pat}{open_repl}{replacement}{close_repl}{modifiers}");

        let lenient = extract_substitution_parts(&input);
        let strict = extract_substitution_parts_strict(&input);

        prop_assert_eq!(lenient.0, pattern.clone());
        prop_assert_eq!(lenient.1, replacement.clone());
        prop_assert_eq!(lenient.2, modifiers.clone());
        prop_assert_eq!(strict.ok(), Some((pattern, replacement, modifiers)));
    }

    #[test]
    fn fuzz_regex_roundtrip(
        operator in prop_oneof![Just(""), Just("m"), Just("qr")],
        delimiters in prop_oneof![
            Just(('/', '/')),
            Just(('!', '!')),
            Just(('{', '}')),
            Just(('(', ')')),
            Just(('[', ']')),
            Just(('<', '>')),
        ],
        body in simple_body(),
        modifiers in valid_modifiers(),
    ) {
        let (open, close) = delimiters;
        let input = format!("{operator}{open}{body}{close}{modifiers}");
        let (pattern, extracted_body, extracted_modifiers) = extract_regex_parts(&input);

        prop_assert_eq!(extracted_body, body.clone());
        prop_assert_eq!(extracted_modifiers, modifiers.clone());
        prop_assert_eq!(pattern, format!("{open}{body}{close}"));
    }

    #[test]
    fn fuzz_transliteration_roundtrip(
        operator in prop_oneof![Just("tr"), Just("y")],
        delimiters in prop_oneof![
            Just(('{', '}')),
            Just(('(', ')')),
            Just(('[', ']')),
            Just(('<', '>')),
        ],
        search in non_empty_body(),
        replace in non_empty_body(),
        modifiers in transliteration_modifiers(),
    ) {
        let (open, close) = delimiters;
        let input = format!("{operator}{open}{search}{close}{open}{replace}{close}{modifiers}");
        let (extracted_search, extracted_replace, extracted_modifiers) = extract_transliteration_parts(&input);

        prop_assert_eq!(extracted_search, search);
        prop_assert_eq!(extracted_replace, replace);
        prop_assert_eq!(extracted_modifiers, modifiers);
    }
}
