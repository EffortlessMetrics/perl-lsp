use perl_qualified_name::{
    is_valid_identifier_part, split_qualified_name, validate_perl_qualified_name,
};
use proptest::prelude::*;

fn identifier_segment() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,8}".prop_map(|s| s.to_string())
}

fn qualified_name_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(identifier_segment(), 1..5).prop_flat_map(|segments| {
        let separator_count = segments.len().saturating_sub(1);
        let separators = prop::collection::vec(Just("::"), separator_count);

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
    fn prop_qualified_name_split_round_trips(name in qualified_name_strategy()) {
        let (package, bare) = split_qualified_name(&name);
        if let Some(idx) = name.rfind("::") {
            assert_eq!(package, Some(&name[..idx]));
            assert_eq!(bare, &name[idx + 2..]);
        } else {
            assert_eq!(package, None);
            assert_eq!(bare, name.as_str());
        }
        assert!(validate_perl_qualified_name(&name).is_ok());
    }

    #[test]
    fn prop_identifier_segment_rules_are_respected(
        segment in "[A-Za-z_][A-Za-z0-9_]{0,12}",
    ) {
        let qualified = format!("{}::{}", segment, segment);
        assert!(is_valid_identifier_part(&segment));
        assert!(validate_perl_qualified_name(&qualified).is_ok());
    }
}
