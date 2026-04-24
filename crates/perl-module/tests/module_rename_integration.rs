use perl_module::path::file_path_to_module_name;
use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};

#[test]
fn rewrites_imports_from_path_derived_module_names() {
    let old_module = file_path_to_module_name("/workspace/lib/My/App.pm");
    let new_module = file_path_to_module_name("/workspace/lib/My/RenamedApp.pm");

    let source = "use My::App;\nuse parent 'My::App';\nrequire My::App;\n";
    let edits = plan_module_rename_edits(source, &old_module, &new_module);
    let rewritten = apply_module_rename_edits(source, &edits);

    let expected = "use My::RenamedApp;\nuse parent 'My::RenamedApp';\nrequire My::RenamedApp;\n";
    assert_eq!(rewritten, expected);
}

#[test]
fn does_not_rewrite_non_matching_imports() {
    let old_module = file_path_to_module_name("/workspace/lib/Foo/Bar.pm");
    let new_module = file_path_to_module_name("/workspace/lib/Foo/Baz.pm");

    let source = "use Foo::Other;\nrequire Foo::Other;\n";
    let edits = plan_module_rename_edits(source, &old_module, &new_module);

    assert!(edits.is_empty());
    assert_eq!(apply_module_rename_edits(source, &edits), source);
}
