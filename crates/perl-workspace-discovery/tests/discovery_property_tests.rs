//! Property tests for workspace discovery invariants.

use perl_workspace_discovery::{discover_perl_files, is_perl_discovery_path};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;

fn extension_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pl".to_string()),
        Just("pm".to_string()),
        Just("t".to_string()),
        Just("psgi".to_string()),
        Just("xs".to_string()),
        Just("ep".to_string()),
        Just("tt".to_string()),
        Just("tt2".to_string()),
        Just("md".to_string()),
        Just("txt".to_string()),
        Just("json".to_string()),
    ]
}

proptest! {
    #[test]
    fn prop_discovery_returns_all_and_only_perl_files(
        specs in prop::collection::vec(("[a-z]{1,10}", extension_strategy()), 1..24)
    ) {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        let mut expected = HashSet::new();

        for (idx, (stem, ext)) in specs.iter().enumerate() {
            let relative = format!("src/file_{idx}_{stem}.{ext}");
            let path = root.join(relative);

            let parent = match path.parent() {
                Some(parent) => parent,
                None => {
                    prop_assert!(false, "path.parent() returned None for joined path: {:?}", path);
                    return Ok(());
                }
            };

            prop_assert!(fs::create_dir_all(parent).is_ok());
            prop_assert!(fs::write(&path, "# generated\n").is_ok());

            if matches!(ext.as_str(), "pl" | "pm" | "t" | "psgi" | "xs" | "ep" | "tt" | "tt2") {
                expected.insert(path);
            }
        }

        let result = discover_perl_files(root);
        let discovered: HashSet<_> = result.files.iter().cloned().collect();

        // No duplicates in discovery output.
        prop_assert_eq!(discovered.len(), result.files.len());

        for path in &discovered {
            prop_assert!(path.starts_with(root));
            prop_assert!(is_perl_discovery_path(path));
        }

        prop_assert_eq!(discovered, expected);
    }

    #[test]
    fn prop_discovery_never_returns_skipped_directories(stem in "[a-z]{3,12}") {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        let skipped_dirs = [".git", ".hg", ".svn", "target", "node_modules", ".cache"];

        for directory in skipped_dirs {
            let path = root.join(directory).join(format!("{stem}.pm"));
            if let Some(parent) = path.parent() {
                prop_assert!(fs::create_dir_all(parent).is_ok());
            }
            prop_assert!(fs::write(path, "# skipped\n").is_ok());
        }

        let visible = root.join(format!("lib/{stem}.pm"));
        if let Some(parent) = visible.parent() {
            prop_assert!(fs::create_dir_all(parent).is_ok());
        }
        prop_assert!(fs::write(&visible, "# visible\n").is_ok());

        let result = discover_perl_files(root);

        for path in &result.files {
            let rendered = path.to_string_lossy();
            prop_assert!(!rendered.contains("/.git/"));
            prop_assert!(!rendered.contains("/.hg/"));
            prop_assert!(!rendered.contains("/.svn/"));
            prop_assert!(!rendered.contains("/target/"));
            prop_assert!(!rendered.contains("/node_modules/"));
            prop_assert!(!rendered.contains("/.cache/"));
        }

        prop_assert!(result.files.iter().any(|path| path.ends_with(&visible)));
    }
}
