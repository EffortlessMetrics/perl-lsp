use perl_module_boundary::{contains_standalone_module_token, find_standalone_module_token_ranges};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
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
    fn prop_rejects_embedded_module_name_in_larger_identifier(module in module_name_strategy()) {
        let line = format!("use {module}Suffix;");

        let ranges = find_standalone_module_token_ranges(&line, &module).collect::<Vec<_>>();
        prop_assert!(ranges.is_empty());
        prop_assert!(!contains_standalone_module_token(&line, &module));
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
