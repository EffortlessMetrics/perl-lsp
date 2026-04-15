use perl_module::name::{
    legacy_package_separator, module_variant_pairs, normalize_package_separator,
};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,8}", 1..5).prop_flat_map(|segments| {
        let separator_count = segments.len().saturating_sub(1);
        let separators =
            proptest::collection::vec(prop_oneof![Just("::"), Just("'")], separator_count);

        separators.prop_map(move |separators| {
            let mut out = String::new();

            for (idx, segment) in segments.iter().enumerate() {
                if idx > 0 {
                    out.push_str(separators[idx - 1]);
                }
                out.push_str(segment);
            }

            out
        })
    })
}

proptest! {
    #[test]
    fn prop_normalization_removes_legacy_separator(module_name in module_name_strategy()) {
        let normalized = normalize_package_separator(&module_name);
        prop_assert!(!normalized.contains('\''));
    }

    #[test]
    fn prop_normalization_is_idempotent(module_name in module_name_strategy()) {
        let normalized_once = normalize_package_separator(&module_name).into_owned();
        let normalized_twice = normalize_package_separator(&normalized_once).into_owned();

        prop_assert_eq!(normalized_once, normalized_twice);
    }

    #[test]
    fn prop_legacy_projection_round_trips_to_canonical(module_name in module_name_strategy()) {
        let normalized = normalize_package_separator(&module_name).into_owned();
        let legacy = legacy_package_separator(&normalized).into_owned();
        let round_tripped = normalize_package_separator(&legacy).into_owned();

        prop_assert_eq!(round_tripped, normalized);
    }

    #[test]
    fn prop_variant_pairs_are_stable_and_canonical_first(
        old_module in module_name_strategy(),
        new_module in module_name_strategy(),
    ) {
        let pairs = module_variant_pairs(&old_module, &new_module);

        prop_assert!(!pairs.is_empty());
        prop_assert!(pairs.len() <= 2);

        let expected_canonical = (
            normalize_package_separator(&old_module).into_owned(),
            normalize_package_separator(&new_module).into_owned(),
        );

        prop_assert_eq!(pairs.first(), Some(&expected_canonical));
        prop_assert!(!pairs[0].0.contains('\''));
        prop_assert!(!pairs[0].1.contains('\''));

        if pairs.len() == 2 {
            prop_assert_ne!(&pairs[0], &pairs[1]);
        }
    }
}
