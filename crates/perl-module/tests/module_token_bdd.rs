use perl_module::token::{contains_module_token, module_variant_pairs, replace_module_token};

#[test]
fn given_canonical_module_names_when_building_variant_pairs_then_canonical_and_legacy_pairs_exist()
{
    let variants = module_variant_pairs("My::Module", "My::Renamed");

    assert_eq!(
        variants,
        vec![
            ("My::Module".to_string(), "My::Renamed".to_string()),
            ("My'Module".to_string(), "My'Renamed".to_string()),
        ]
    );
}

#[test]
fn given_legacy_module_names_when_building_variant_pairs_then_canonical_pair_is_also_available() {
    let variants = module_variant_pairs("My'Module", "My'Renamed");

    assert_eq!(
        variants,
        vec![
            ("My::Module".to_string(), "My::Renamed".to_string()),
            ("My'Module".to_string(), "My'Renamed".to_string()),
        ]
    );
}

#[test]
fn given_standalone_module_token_when_replacing_then_target_token_is_rewritten() {
    let (rewritten, changed) =
        replace_module_token("use parent 'My::Module';", "My::Module", "My::Renamed");

    assert!(changed);
    assert_eq!(rewritten, "use parent 'My::Renamed';");
}

#[test]
fn given_partial_module_token_when_replacing_then_source_line_remains_unchanged() {
    let (rewritten, changed) =
        replace_module_token("use My::ModuleX;", "My::Module", "My::Renamed");

    assert!(!changed);
    assert_eq!(rewritten, "use My::ModuleX;");
    assert!(!contains_module_token(&rewritten, "My::Module"));
}

#[test]
fn given_legacy_partial_module_token_when_replacing_then_source_line_remains_unchanged() {
    let (rewritten, changed) = replace_module_token("use My'Module'X;", "My'Module", "My'Renamed");

    assert!(!changed);
    assert_eq!(rewritten, "use My'Module'X;");
}
