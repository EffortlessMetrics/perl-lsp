use perl_module::resolution::path::resolve_module_path;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn given_current_directory_include_path_when_resolving_then_result_is_root_relative_module_file()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?.path().to_path_buf();
    let module_file = root.join("Foo").join("Bar.pm");
    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "package Foo::Bar; 1;")?;

    let resolved = resolve_module_path(&root, "Foo::Bar", &[".".to_string()]);

    assert_eq!(resolved, Some(root.join("Foo/Bar.pm")));
    Ok(())
}

#[test]
fn given_traversal_include_path_when_resolving_then_fallback_remains_inside_workspace_root() {
    let root = PathBuf::from("/workspace");
    let resolved = resolve_module_path(&root, "Foo::Bar", &["..".to_string()]);

    assert!(resolved.is_some(), "expected a resolved path");
    let resolved = resolved.unwrap_or_default();

    assert!(resolved.starts_with(&root));
    assert_eq!(resolved, root.join("lib").join("Foo/Bar.pm"));
}
