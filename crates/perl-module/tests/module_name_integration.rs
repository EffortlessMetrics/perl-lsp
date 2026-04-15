use perl_module::name::{
    legacy_package_separator, module_variant_pairs, normalize_package_separator,
};

fn apply_variant_pairs(line: &str, old_module: &str, new_module: &str) -> String {
    let mut rewritten = line.to_string();

    for (from, to) in module_variant_pairs(old_module, new_module) {
        rewritten = rewritten.replace(&from, &to);
    }

    rewritten
}

#[test]
fn canonical_and_legacy_pairs_rewrite_corresponding_import_styles() {
    let canonical_line = "use My::App;";
    let legacy_line = "use My'App;";

    let canonical_rewritten = apply_variant_pairs(canonical_line, "My::App", "My::RenamedApp");
    let legacy_rewritten = apply_variant_pairs(legacy_line, "My::App", "My::RenamedApp");

    assert_eq!(canonical_rewritten, "use My::RenamedApp;");
    assert_eq!(legacy_rewritten, "use My'RenamedApp;");
}

#[test]
fn normalize_and_legacy_helpers_form_stable_bridge_for_variant_generation() {
    let old_module = "Legacy'Pkg";
    let new_module = "Next::Pkg";

    let normalized_old = normalize_package_separator(old_module).into_owned();
    let normalized_new = normalize_package_separator(new_module).into_owned();

    let pairs = module_variant_pairs(old_module, new_module);

    assert_eq!(pairs[0], (normalized_old.clone(), normalized_new.clone()));
    assert_eq!(
        pairs[1],
        (
            legacy_package_separator(&normalized_old).into_owned(),
            legacy_package_separator(&normalized_new).into_owned(),
        )
    );
}
