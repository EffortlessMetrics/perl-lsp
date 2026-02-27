use perl_workspace_folder::workspace_folder_to_path;
use proptest::prelude::*;

fn plain_path_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z]{1,8}", 1..6).prop_map(|segments| segments.join("/"))
}

proptest! {
    #[test]
    fn prop_plain_paths_are_interpreted_as_filesystem_paths(folder in plain_path_strategy()) {
        let parsed = workspace_folder_to_path(&folder);
        assert!(!parsed.to_string_lossy().contains("file://"));

        let canonical = parsed.to_string_lossy().replace('\\', "/");
        assert_eq!(canonical, folder);
    }

    #[test]
    fn prop_file_uri_inputs_strip_file_scheme(folder in plain_path_strategy()) {
        let uri = format!("file://{folder}");
        let parsed = workspace_folder_to_path(&uri);
        assert!(!parsed.to_string_lossy().contains("file://"));
    }
}
