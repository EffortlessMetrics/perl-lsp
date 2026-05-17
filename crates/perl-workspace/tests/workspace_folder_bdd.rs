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
fn given_file_uri_with_remote_host_when_resolving_then_raw_uri_is_preserved() {
    let parsed = workspace_folder_to_path("file://relative/example");

    // Remote file URI hosts are intentionally not converted into local path
    // components: preserving the raw URI keeps the caller from opening a path
    // that accidentally includes a remote hostname as a normal directory.
    assert_eq!(parsed, PathBuf::from("file://relative/example"));
}
