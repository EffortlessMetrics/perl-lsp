use perl_module::path::{
    file_path_to_module_name, module_name_to_path, module_path_to_name, normalize_package_separator,
};
use proptest::prelude::*;

fn module_segment_strategy() -> impl Strategy<Value = String> {
    ("[A-Za-z_]", "[A-Za-z0-9_]{0,7}").prop_map(|(head, tail)| format!("{head}{tail}"))
}

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(module_segment_strategy(), 1..=6).prop_flat_map(|segments| {
        let separator_count = segments.len().saturating_sub(1);
        let separators =
            proptest::collection::vec(prop_oneof![Just("::"), Just("'")], separator_count);

        separators.prop_map(move |seps| {
            let mut module_name = segments[0].clone();
            for (sep, segment) in seps.iter().zip(segments.iter().skip(1)) {
                module_name.push_str(sep);
                module_name.push_str(segment);
            }
            module_name
        })
    })
}

fn module_path_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(module_segment_strategy(), 1..=6).prop_flat_map(|segments| {
        let separator_count = segments.len().saturating_sub(1);
        let separators =
            proptest::collection::vec(prop_oneof![Just("/"), Just(r"\")], separator_count);

        (Just(segments), separators, prop_oneof![Just(".pm"), Just(".pl"), Just("")]).prop_map(
            |(segments, seps, ext)| {
                let mut path = segments[0].clone();
                for (sep, segment) in seps.iter().zip(segments.iter().skip(1)) {
                    path.push_str(sep);
                    path.push_str(segment);
                }
                path.push_str(ext);
                path
            },
        )
    })
}

proptest! {
    #[test]
    fn prop_module_name_path_roundtrip_preserves_normalized_name(module_name in module_name_strategy()) {
        let normalized = normalize_package_separator(&module_name).into_owned();
        let path = module_name_to_path(&module_name);
        let roundtrip = module_path_to_name(&path);

        prop_assert_eq!(roundtrip, normalized);
    }

    #[test]
    fn prop_module_path_to_name_is_separator_style_agnostic(module_path in module_path_strategy()) {
        let forward = module_path.replace('\\', "/");
        let backward = module_path.replace('/', r"\");

        let from_forward = module_path_to_name(&forward);
        let from_backward = module_path_to_name(&backward);

        prop_assert_eq!(from_forward, from_backward);
    }

    #[test]
    fn prop_file_path_to_module_name_uses_last_lib_segment(
        module_name in module_name_strategy(),
        prefix in "[a-z]{1,8}",
        outer in "[a-z]{1,8}",
    ) {
        let module_path = module_name_to_path(&module_name);
        let file_path = format!("/{prefix}/lib/{outer}/lib/{module_path}");
        let normalized = normalize_package_separator(&module_name).into_owned();

        let derived = file_path_to_module_name(&file_path);

        prop_assert_eq!(derived, normalized);
    }
}
