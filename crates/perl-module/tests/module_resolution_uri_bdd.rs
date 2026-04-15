use perl_module::resolution::uri::{ModuleUriResolution, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn given_open_document_when_resolving_then_open_document_takes_precedence() {
    let open_doc = "file:///workspace/lib/Foo/Bar.pm".to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        std::slice::from_ref(&open_doc),
        &["file:///workspace".to_string()],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
}

#[test]
fn given_workspace_folder_when_resolving_then_workspace_file_uri_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Demo").join("Worker.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Demo::Worker; 1;")?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    let result = resolve_module_uri(
        "Demo::Worker",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.starts_with("file://"));
            assert!(uri.ends_with("Demo/Worker.pm") || uri.ends_with("Demo\\Worker.pm"));
        }
        other => return Err(format!("expected resolved URI, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn given_system_inc_disabled_when_resolving_then_system_paths_are_ignored()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let system_inc = temp.path().join("perl5");
    let module_file = system_inc.join("Only").join("InInc.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&system_inc))?;
    std::fs::write(&module_file, "package Only::InInc; 1;")?;

    let result = resolve_module_uri(
        "Only::InInc",
        &[],
        &[],
        &["lib".to_string()],
        false,
        &[PathBuf::from(&system_inc)],
        Duration::from_millis(50),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}
