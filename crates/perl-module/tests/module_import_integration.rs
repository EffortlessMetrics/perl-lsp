use perl_module::import::{ModuleImportKind, parse_module_import_head};
use perl_module::path::file_path_to_module_name;
use perl_module::token::{contains_module_token, module_variant_pairs, replace_module_token};

fn rewrite_line_with_variant_pairs(line: &str, old_module: &str, new_module: &str) -> String {
    let mut rewritten = line.to_string();

    for (from, to) in module_variant_pairs(old_module, new_module) {
        let (candidate, changed) = replace_module_token(&rewritten, &from, &to);
        if changed {
            rewritten = candidate;
        }
    }

    rewritten
}

#[test]
fn parses_use_head_and_drives_token_rewrite_for_path_derived_module_names() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");
    let line = "use My::App;";

    let parsed = parse_module_import_head(line);
    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::Use);
        assert_eq!(head.token, old_module.as_str());
    }

    let rewritten = rewrite_line_with_variant_pairs(line, &old_module, &new_module);
    assert_eq!(rewritten, "use My::RenamedApp;");
}

#[test]
fn parses_require_head_and_drives_token_rewrite_for_path_derived_module_names() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");
    let line = "require My::App;";

    let parsed = parse_module_import_head(line);
    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::Require);
        assert_eq!(head.token, old_module.as_str());
    }

    let rewritten = rewrite_line_with_variant_pairs(line, &old_module, &new_module);
    assert_eq!(rewritten, "require My::RenamedApp;");
}

#[test]
fn parses_use_parent_head_and_allows_parent_module_token_detection() {
    let line = "use parent qw(My::App Other::Base);";
    let parsed = parse_module_import_head(line);

    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::UseParent);
        assert_eq!(head.token, "parent");
    }
    assert!(contains_module_token(line, "My::App"));
    assert!(!contains_module_token(line, "My::Missing"));
}
