use perl_module::path::canonicalize_path_long_form;
use perl_module::resolution::path::resolve_module_path;
use std::path::PathBuf;

#[test]
fn resolves_existing_module_under_include_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Demo").join("Worker.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Demo::Worker; 1;")?;

    let resolved = resolve_module_path(&workspace, "Demo::Worker", &["lib".to_string()]);

    assert_eq!(resolved, Some(canonicalize_path_long_form(&module_file)));
    Ok(())
}

#[test]
fn returns_lib_fallback_when_no_include_path_matches() {
    let workspace = PathBuf::from("/workspace");
    let resolved =
        resolve_module_path(&workspace, "Missing::Module", &["nonexistent/include".to_string()]);

    assert_eq!(
        resolved,
        Some(canonicalize_path_long_form(&workspace.join("lib").join("Missing/Module.pm"),))
    );
}
