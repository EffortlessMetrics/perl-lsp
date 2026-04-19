use perl_workspace::folder::workspace_folder_to_path;
use std::fs;
use tempfile::TempDir;
use url::Url;

#[test]
fn resolves_temporary_workspace_file_uri_to_actual_path() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    let workspace = temp.path().join("workspace");
    fs::create_dir(&workspace)?;

    let uri = Url::from_file_path(&workspace)
        .map_err(|_| "failed to encode workspace path")?
        .to_string();
    let parsed = workspace_folder_to_path(&uri);

    assert_eq!(parsed, workspace);
    Ok(())
}

#[test]
fn resolves_file_uri_with_space_in_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let workspace = temp.path().join("my folder");
    fs::create_dir_all(&workspace)?;

    let uri = Url::from_file_path(&workspace)
        .map_err(|_| "failed to encode workspace path")?
        .to_string();
    let parsed = workspace_folder_to_path(&uri);

    let rendered = parsed.to_string_lossy();
    let expected = workspace.to_string_lossy();
    assert_eq!(rendered, expected);
    assert!(!rendered.contains("file://"));
    Ok(())
}
