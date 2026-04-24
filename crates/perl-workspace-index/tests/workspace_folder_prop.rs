use perl_workspace::folder::{
    extract_workspace_folder_change, extract_workspace_folder_uris, root_path_to_file_uri,
    workspace_folder_to_path,
};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;
use serde_json::{Value, json};

fn plain_path_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z]{1,8}", 1..6).prop_map(|segments| segments.join("/"))
}

fn uri_entry_strategy() -> impl Strategy<Value = (Option<String>, Value)> {
    prop_oneof![
        plain_path_strategy().prop_map(|folder| {
            let uri = format!("file:///{folder}");
            (Some(uri.clone()), json!({"uri": uri, "name": "workspace"}))
        }),
        plain_path_strategy().prop_map(|folder| {
            let uri = format!("file:///{folder}");
            (Some(uri.clone()), json!({"uri": uri}))
        }),
        plain_path_strategy().prop_map(|folder| {
            let path = format!("/{folder}");
            let uri = root_path_to_file_uri(&path);
            (Some(uri), json!({"path": path}))
        }),
        any::<i64>().prop_map(|value| (None, json!({"uri": value}))),
        plain_path_strategy().prop_map(|folder| (None, json!({"name": folder}))),
        Just((None, json!(null))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_plain_paths_are_interpreted_as_filesystem_paths(folder in plain_path_strategy()) {
        let parsed = workspace_folder_to_path(&folder);
        prop_assert!(!parsed.to_string_lossy().contains("file://"));

        let canonical = parsed.to_string_lossy().replace('\\', "/");
        prop_assert_eq!(canonical, folder);
    }

    #[test]
    fn prop_file_uri_inputs_strip_file_scheme(folder in plain_path_strategy()) {
        // Use file:/// (triple slash / absolute path form) to avoid platform-specific
        // UNC-path behavior on Windows when a host component is present.
        let uri = format!("file:///{folder}");
        let parsed = workspace_folder_to_path(&uri);
        prop_assert!(!parsed.to_string_lossy().contains("file://"));
    }

    #[test]
    fn prop_extract_workspace_folder_uris_preserves_only_valid_uri_strings(
        entries in prop::collection::vec(uri_entry_strategy(), 0..16)
    ) {
        let raw_entries: Vec<Value> = entries.iter().map(|(_, value)| value.clone()).collect();
        let expected: Vec<String> = entries
            .iter()
            .filter_map(|(expected_uri, _)| expected_uri.clone())
            .collect();

        let extracted = extract_workspace_folder_uris(&raw_entries);

        prop_assert_eq!(extracted, expected);
    }

    #[test]
    fn prop_extract_workspace_folder_change_tracks_added_and_removed_independently(
        added_entries in prop::collection::vec(uri_entry_strategy(), 0..8),
        removed_entries in prop::collection::vec(uri_entry_strategy(), 0..8),
    ) {
        let added_values: Vec<Value> = added_entries.iter().map(|(_, value)| value.clone()).collect();
        let removed_values: Vec<Value> = removed_entries.iter().map(|(_, value)| value.clone()).collect();
        let expected_added: Vec<String> = added_entries
            .iter()
            .filter_map(|(expected_uri, _)| expected_uri.clone())
            .collect();
        let expected_removed: Vec<String> = removed_entries
            .iter()
            .filter_map(|(expected_uri, _)| expected_uri.clone())
            .collect();

        let change = extract_workspace_folder_change(&json!({
            "added": added_values,
            "removed": removed_values,
        }));

        prop_assert_eq!(change.added, expected_added);
        prop_assert_eq!(change.removed, expected_removed);
    }

    #[test]
    fn prop_root_path_to_file_uri_produces_file_scheme_for_plain_paths(folder in plain_path_strategy()) {
        let root_path = format!("/{folder}");
        let uri = root_path_to_file_uri(&root_path);

        prop_assert!(uri.starts_with("file://"));
        prop_assert!(uri.ends_with(&folder));
        prop_assert!(!uri.contains('\\'));
    }
}
