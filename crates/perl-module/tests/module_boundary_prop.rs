use perl_module::boundary::{
    contains_standalone_module_token, find_standalone_module_token_ranges,
};
use perl_module::name::legacy_package_separator;
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
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

    proptest::collection::vec(boundary_char, 0..4).prop_map(|chars| chars.into_iter().collect())
}

fn nonempty_boundary_padding_strategy() -> impl Strategy<Value = String> {
    boundary_padding_strategy()
        .prop_filter("boundary padding must not be empty", |padding| !padding.is_empty())
}

proptest! {
    #[test]
    fn prop_finds_exact_range_for_direct_use_lines(module in module_name_strategy()) {
        let line = format!("use {module};");
        let ranges = find_standalone_module_token_ranges(&line, &module).collect::<Vec<_>>();

        prop_assert_eq!(ranges.len(), 1);
        prop_assert_eq!(ranges[0].start, 4);
        prop_assert_eq!(ranges[0].end, 4 + module.len());
        prop_assert!(contains_standalone_module_token(&line, &module));
    }

    #[test]
    fn prop_finds_exact_range_for_direct_use_lines_with_legacy_separators(module in module_name_strategy()) {
        let legacy_module = legacy_package_separator(&module).into_owned();
        let line = format!("use {legacy_module};");
        let ranges = find_standalone_module_token_ranges(&line, &legacy_module).collect::<Vec<_>>();

        prop_assert_eq!(ranges.len(), 1);
        prop_assert_eq!(ranges[0].start, 4);
        prop_assert_eq!(ranges[0].end, 4 + legacy_module.len());
        prop_assert!(contains_standalone_module_token(&line, &legacy_module));
    }

    #[test]
    fn prop_rejects_embedded_module_name_in_larger_identifier(module in module_name_strategy()) {
        let line = format!("use {module}Suffix;");

        let ranges = find_standalone_module_token_ranges(&line, &module).collect::<Vec<_>>();
        prop_assert!(ranges.is_empty());
        prop_assert!(!contains_standalone_module_token(&line, &module));
    }

    #[test]
    fn prop_repeated_standalone_matches_are_non_overlapping_and_complete(
        module in module_name_strategy(),
        prefix in boundary_padding_strategy(),
        infix in nonempty_boundary_padding_strategy(),
        suffix in boundary_padding_strategy(),
    ) {
        let line = format!("{prefix}{module}{infix}{module}{suffix}");
        let ranges = find_standalone_module_token_ranges(&line, &module).collect::<Vec<_>>();

        prop_assert_eq!(ranges.len(), 2);
        prop_assert!(contains_standalone_module_token(&line, &module));

        let first = ranges[0];
        let second = ranges[1];
        prop_assert_eq!(&line[first.start..first.end], module.as_str());
        prop_assert_eq!(&line[second.start..second.end], module.as_str());
        prop_assert!(first.end <= second.start);
    }

    #[test]
    fn prop_contains_agrees_with_range_presence(
        line in "(?s).{0,256}",
        module_name in "[A-Za-z_][A-Za-z0-9_:'\\:]{0,24}",
    ) {
        let ranges = find_standalone_module_token_ranges(&line, &module_name).collect::<Vec<_>>();
        prop_assert_eq!(contains_standalone_module_token(&line, &module_name), !ranges.is_empty());

        let mut prev_end = 0usize;
        for range in ranges {
            prop_assert!(range.start <= range.end);
            prop_assert!(range.end <= line.len());
            prop_assert!(line.is_char_boundary(range.start));
            prop_assert!(line.is_char_boundary(range.end));
            prop_assert!(range.start >= prev_end);
            prop_assert_eq!(&line[range.start..range.end], module_name.as_str());
            prev_end = range.end;
        }
    }
}
