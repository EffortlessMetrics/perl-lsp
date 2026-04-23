use perl_module::name::legacy_package_separator;
use perl_module::token::{contains_module_token, replace_module_token};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,8}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

fn boundary_padding_strategy() -> impl Strategy<Value = String> {
    let boundary_char = prop_oneof![
        Just(' '),
        Just('\t'),
        Just(';'),
        Just(','),
        Just('('),
        Just(')'),
        Just('['),
        Just(']'),
        Just('{'),
        Just('}'),
        Just('/'),
        Just('-'),
        Just('"'),
    ];

    proptest::collection::vec(boundary_char, 0..8).prop_map(|chars| chars.into_iter().collect())
}

fn nonempty_boundary_padding_strategy() -> impl Strategy<Value = String> {
    boundary_padding_strategy()
        .prop_filter("boundary padding must not be empty", |padding| !padding.is_empty())
}

fn module_char_strategy() -> impl Strategy<Value = char> {
    prop_oneof![
        proptest::char::range('a', 'z'),
        proptest::char::range('A', 'Z'),
        proptest::char::range('0', '9'),
        Just('_'),
        Just(':'),
    ]
}

proptest! {
    #[test]
    fn prop_replacement_occurs_for_standalone_module_tokens(
        prefix in boundary_padding_strategy(),
        suffix in boundary_padding_strategy(),
        from in module_name_strategy(),
        to in module_name_strategy(),
    ) {
        prop_assume!(from != to);

        let line = format!("{prefix}{from}{suffix}");
        let (rewritten, changed) = replace_module_token(&line, &from, &to);

        prop_assert!(changed);
        prop_assert_eq!(rewritten, format!("{prefix}{to}{suffix}"));
    }

    #[test]
    fn prop_replacement_occurs_for_legacy_module_tokens(
        prefix in boundary_padding_strategy(),
        suffix in boundary_padding_strategy(),
        from in module_name_strategy(),
        to in module_name_strategy(),
    ) {
        prop_assume!(from != to);

        let legacy_from = legacy_package_separator(&from).into_owned();
        let legacy_to = legacy_package_separator(&to).into_owned();
        let line = format!("{prefix}{legacy_from}{suffix}");
        let (rewritten, changed) = replace_module_token(&line, &legacy_from, &legacy_to);

        prop_assert!(changed);
        prop_assert_eq!(rewritten, format!("{prefix}{legacy_to}{suffix}"));
    }

    #[test]
    fn prop_replacement_does_not_trigger_on_partial_module_names(
        left in module_char_strategy(),
        right in module_char_strategy(),
        from in module_name_strategy(),
        to in module_name_strategy(),
    ) {
        prop_assume!(from != to);

        let line = format!("{left}{from}{right}");
        let (rewritten, changed) = replace_module_token(&line, &from, &to);

        prop_assert!(!changed);
        prop_assert_eq!(rewritten.as_str(), line.as_str());
        prop_assert!(!contains_module_token(&rewritten, &from));
    }

    #[test]
    fn prop_replacement_updates_every_standalone_occurrence(
        prefix in boundary_padding_strategy(),
        infix in nonempty_boundary_padding_strategy(),
        suffix in boundary_padding_strategy(),
        from in module_name_strategy(),
        to in module_name_strategy(),
    ) {
        prop_assume!(from != to);

        let line = format!("{prefix}{from}{infix}{from}{suffix}");
        let (rewritten, changed) = replace_module_token(&line, &from, &to);

        prop_assert!(changed);
        let expected = format!("{prefix}{to}{infix}{to}{suffix}");
        prop_assert_eq!(rewritten.as_str(), expected.as_str());
        prop_assert!(!contains_module_token(&rewritten, &from));
        prop_assert!(contains_module_token(&rewritten, &to));
    }

    #[test]
    fn prop_self_replacement_preserves_the_line(
        line in "(?s).{0,256}",
        module in module_name_strategy(),
    ) {
        let (rewritten, changed) = replace_module_token(&line, &module, &module);
        prop_assert_eq!(contains_module_token(&line, &module), changed);
        prop_assert_eq!(rewritten, line);
    }
}
