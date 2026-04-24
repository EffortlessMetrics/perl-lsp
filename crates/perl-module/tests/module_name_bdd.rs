use perl_module::name::{
    legacy_package_separator, module_variant_pairs, normalize_package_separator,
};

#[test]
fn given_legacy_separator_when_normalizing_then_canonical_name_is_returned() {
    assert_eq!(normalize_package_separator("My'Module"), "My::Module");
}

#[test]
fn given_canonical_separator_when_projecting_then_legacy_name_is_returned() {
    assert_eq!(legacy_package_separator("My::Module"), "My'Module");
}

#[test]
fn given_module_rename_when_building_variant_pairs_then_canonical_and_legacy_pairs_are_returned() {
    let pairs = module_variant_pairs("My::Module", "My::Renamed");

    assert_eq!(
        pairs,
        vec![
            ("My::Module".to_string(), "My::Renamed".to_string()),
            ("My'Module".to_string(), "My'Renamed".to_string()),
        ]
    );
}

#[test]
fn given_separatorless_module_names_when_building_variant_pairs_then_pairs_are_deduplicated() {
    let pairs = module_variant_pairs("strict", "warnings");

    assert_eq!(pairs, vec![("strict".to_string(), "warnings".to_string())]);
}
