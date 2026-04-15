use perl_module::resolution::{ModuleUriResolution, resolve_module_uri};
use std::time::Duration;

#[test]
fn resolves_workspace_module_from_file_uri_folder() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Foo").join("Bar.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Foo::Bar; 1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace)
        .map_err(|()| "failed to create workspace URI")?
        .to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("Foo"));
            assert!(uri.contains("Bar.pm"));
        }
        other => return Err(format!("expected resolved URI, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn blocks_workspace_traversal_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let escaped_dir = temp.path().join("escaped");

    std::fs::create_dir_all(&workspace)?;
    std::fs::create_dir_all(&escaped_dir)?;

    let escaped_file = escaped_dir.join("Target.pm");
    std::fs::write(&escaped_file, "package escaped::Target; 1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace)
        .map_err(|()| "failed to create workspace URI")?
        .to_string();

    let result = resolve_module_uri(
        "escaped::Target",
        &[],
        &[workspace_uri],
        &["..".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}
