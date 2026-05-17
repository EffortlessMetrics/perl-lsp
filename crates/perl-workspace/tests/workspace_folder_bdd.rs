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
fn given_file_uri_with_remote_host_when_resolving_then_remote_host_is_not_pathified() {
    let parsed = workspace_folder_to_path("file://relative/example");
    let path = parsed.to_string_lossy();

    // Remote file URI hosts must not be stripped into a path component that a caller
    // could accidentally open as a local/UNC path. Keeping the raw URI makes the
    // unresolved authority explicit while preserving the original value for diagnostics.
    assert!(path.contains("file://relative/example"));
    assert!(!path.starts_with("relative"));
}
