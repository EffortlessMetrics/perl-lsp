use perl_module::path::file_path_to_module_name;
use perl_module::token::{module_variant_pairs, replace_module_token};

fn rewrite_with_variants(line: &str, old_module: &str, new_module: &str) -> String {
    let mut rewritten = line.to_string();

    for (old_variant, new_variant) in module_variant_pairs(old_module, new_module) {
        let (candidate, changed) = replace_module_token(&rewritten, &old_variant, &new_variant);
        if changed {
            rewritten = candidate;
        }
    }

    rewritten
}

#[test]
fn rewrites_canonical_import_lines_using_path_derived_module_names() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");

    let rewritten = rewrite_with_variants("use My::App;", &old_module, &new_module);
    assert_eq!(rewritten, "use My::RenamedApp;");
}

#[test]
fn rewrites_legacy_import_lines_using_path_derived_module_names() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");

    let rewritten = rewrite_with_variants("use My'App;", &old_module, &new_module);
    assert_eq!(rewritten, "use My'RenamedApp;");
}

#[test]
fn does_not_rewrite_partial_legacy_module_names_in_larger_tokens() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");

    let rewritten = rewrite_with_variants("use My'App'Child;", &old_module, &new_module);
    assert_eq!(rewritten, "use My'App'Child;");
}
