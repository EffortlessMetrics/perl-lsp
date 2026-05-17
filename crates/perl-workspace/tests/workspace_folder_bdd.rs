use perl_workspace::folder::workspace_folder_to_path;
use std::path::PathBuf;

#[test]
fn given_plain_path_when_resolving_then_path_is_returned() {
    let parsed = workspace_folder_to_path("/tmp/project");
    assert_eq!(parsed, PathBuf::from("/tmp/project"));
}

#[test]
fn given_file_uri_when_resolving_then_path_is_returned() {
    let parsed = workspace_folder_to_path("file:///tmp/workspace");
    assert!(!parsed.to_string_lossy().contains("file://"));
    assert!(parsed.to_string_lossy().contains("tmp"));
}

#[test]
fn given_file_uri_with_unresolvable_path_when_resolving_then_scheme_is_stripped() {
    let parsed = workspace_folder_to_path("file://relative/example");
    assert!(!parsed.to_string_lossy().contains("file://"));
    // On Windows, file://host/path resolves as a UNC path \\host\path;
    // on other platforms the fallback strips "file://" and returns "relative/example".
    // Either way the path must contain "example".
    assert!(parsed.to_string_lossy().contains("example"));
}
