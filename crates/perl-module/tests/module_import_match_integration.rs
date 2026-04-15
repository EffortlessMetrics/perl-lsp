use perl_module::import::parse_module_import_head;
use perl_module::import_match::line_references_module_import;
use perl_module::path::file_path_to_module_name;
use perl_module::token::{module_variant_pairs, replace_module_token};

fn rewrite_line_with_target_match(line: &str, old_module: &str, new_module: &str) -> String {
    let mut rewritten = line.to_string();

    for (from, to) in module_variant_pairs(old_module, new_module) {
        if !line_references_module_import(&rewritten, &from) {
            continue;
        }

        let (candidate, changed) = replace_module_token(&rewritten, &from, &to);
        if changed {
            rewritten = candidate;
        }
    }

    rewritten
}

#[test]
fn path_derived_module_names_drive_use_and_parent_rewrites() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");

    let use_line = rewrite_line_with_target_match("use My::App;", &old_module, &new_module);
    let parent_line =
        rewrite_line_with_target_match("use parent 'My::App';", &old_module, &new_module);
    let non_target = rewrite_line_with_target_match("use My::App::Util;", &old_module, &new_module);

    assert_eq!(use_line, "use My::RenamedApp;");
    assert_eq!(parent_line, "use parent 'My::RenamedApp';");
    assert_eq!(non_target, "use My::App::Util;");
}

#[test]
fn matched_lines_are_also_parseable_as_import_heads() {
    let line = "require Demo::Worker;";
    let module = "Demo::Worker";

    assert!(line_references_module_import(line, module));
    assert!(parse_module_import_head(line).is_some());
}
